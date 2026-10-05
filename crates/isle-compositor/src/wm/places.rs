//! Where windows open, and the boxes main windows are remembered at (`compositor.md`
//! Stacking 2–6).

use isle_state::{
    Loaded, StateDir,
    domains::{Place, Places},
};
use smithay::{
    output::Output,
    utils::{Point, Rectangle},
};
use tracing::{debug, warn};

use super::geometry::{self, Rect};
use crate::{
    state::{Isle, State},
    state_writer::Saver,
};

impl Isle {
    /// Reads the window's app and parent from its toplevel again.
    pub fn refresh_identity(&mut self, id: u64) {
        let parent = self
            .wm
            .get(id)
            .and_then(|r| r.window.toplevel())
            .and_then(|t| t.parent())
            .and_then(|p| self.wm.for_surface(&p))
            .map(|r| r.id);
        if let Some(record) = self.wm.get_mut(id) {
            record.app = record.names().0;
            record.parent = parent;
        }
    }

    pub fn load_places(&mut self, runtime: &tokio::runtime::Handle) {
        let dir = StateDir::new(StateDir::default_dir());
        let (places, loaded) = dir.load::<Places>();
        if !matches!(loaded, Loaded::Read | Loaded::Missing) {
            warn!(?loaded, "window places");
        }
        self.places = places;
        self.places_saver = Some(Saver::spawn(runtime, dir));
    }

    /// The part of `output` windows may use, in the global space.
    pub fn work_area(&self, output: &Output) -> Rect {
        let geometry = self.space.output_geometry(output).unwrap_or_default();
        let mut free = smithay::desktop::layer_map_for_output(output).non_exclusive_zone();
        free.loc += geometry.loc;
        geometry::work_area(geometry, free, self.strip())
    }

    /// The height kept clear along each output's top for the island.
    pub fn strip(&self) -> i32 {
        self.shell.theme().tokens.size_strip.round() as i32
    }

    /// The output new windows open on: the focused window's, else the pointer's.
    pub fn output_with_focus(&self) -> Option<Output> {
        let focused = self
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .and_then(|f| f.surface().cloned())
            .and_then(|s| self.wm.for_surface(&s).map(|r| r.window.clone()));
        focused
            .and_then(|w| self.space.outputs_for_element(&w).into_iter().next())
            .or_else(|| self.active_output())
    }

    /// The output whose box holds all of `rect`.
    pub fn output_holding(&self, rect: Rect) -> Option<Output> {
        self.space
            .outputs()
            .find(|o| {
                self.space
                    .output_geometry(o)
                    .is_some_and(|g| g.contains_rect(rect))
            })
            .cloned()
    }

    /// Whether `id` is its app's main window: its only titled toplevel with no parent.
    pub fn is_main(&self, id: u64) -> bool {
        let Some(record) = self.wm.get(id) else {
            return false;
        };
        let titled = |r: &super::Record| r.parent.is_none() && !r.names().1.is_empty();
        if !titled(record) || record.app.is_empty() {
            return false;
        }
        !self
            .wm
            .records
            .iter()
            .any(|r| r.id != id && r.mapped && r.app == record.app && titled(r))
    }

    /// `app`'s box on `output`, or its latest anywhere clamped onto `output`.
    fn place_for(&self, app: &str, output: &Output) -> Option<Rect> {
        let places = self.places.apps.get(app)?;
        let area = self.work_area(output);
        let place = places
            .get(&Self::output_identity(output))
            .or_else(|| places.values().max_by_key(|p| p.at))?;
        let rect = Rectangle::new(
            (area.loc.x + place.x, area.loc.y + place.y).into(),
            (place.width, place.height).into(),
        );
        Some(geometry::clamp_whole(rect, area))
    }

    /// Remembers `id`'s box if it is a main window wholly on one output.
    pub fn record_place(&mut self, id: u64) {
        if !self.is_main(id) {
            return;
        }
        let Some(record) = self.wm.get(id) else {
            return;
        };
        if record.mode != super::Mode::Floating {
            return;
        }
        let rect = self
            .space
            .element_geometry(&record.window)
            .unwrap_or(record.rect);
        let Some(output) = self.output_holding(rect) else {
            debug!(id, ?rect, "not on one output; its place is not recorded");
            return;
        };
        let area = self.work_area(&output);
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let place = Place {
            x: rect.loc.x - area.loc.x,
            y: rect.loc.y - area.loc.y,
            width: rect.size.w,
            height: rect.size.h,
            at,
        };
        self.places
            .apps
            .entry(record.app.clone())
            .or_default()
            .insert(Self::output_identity(&output), place);
        if let Some(saver) = &self.places_saver {
            saver.save(self.places.clone());
        }
    }
}

impl State {
    /// Before a toplevel's first configure: a main window with a place is asked for its size.
    pub fn plan_window(&mut self, id: u64) {
        let isle = &mut self.isle;
        isle.refresh_identity(id);
        if !isle.is_main(id) {
            return;
        }
        let app = isle.wm.get(id).unwrap().app.clone();
        let Some(output) = isle.output_with_focus() else {
            return;
        };
        let Some(place) = isle.place_for(&app, &output) else {
            return;
        };
        let record = isle.wm.get_mut(id).unwrap();
        record.planned = Some(place);
        if let Some(toplevel) = record.window.toplevel() {
            toplevel.with_pending_state(|state| state.size = Some(place.size));
        }
    }

    /// At map: the window's place, over its parent, or centred on the output with focus.
    pub fn place_new_window(&mut self, id: u64) -> Option<Point<i32, smithay::utils::Logical>> {
        let isle = &self.isle;
        let record = isle.wm.get(id)?;
        let size = record.window.geometry().size;
        if let Some(planned) = record.planned {
            let output = isle
                .output_holding(planned)
                .or_else(|| isle.output_with_focus())?;
            let rect = Rectangle::new(planned.loc, size);
            return Some(geometry::clamp_whole(rect, isle.work_area(&output)).loc);
        }
        let parent = record
            .parent
            .and_then(|p| isle.wm.get(p))
            .filter(|p| p.shown());
        if let Some(parent) = parent {
            let frame = isle
                .space
                .element_geometry(&parent.window)
                .unwrap_or(parent.rect);
            let output = isle
                .output_holding(frame)
                .or_else(|| isle.output_with_focus())?;
            let area = isle.work_area(&output);
            if record.modal && size.w < frame.size.w && size.h < frame.size.h {
                return Some(geometry::under_bar(frame, 0, size));
            }
            return Some(geometry::over_parent(frame, size, area));
        }
        let output = isle.output_with_focus()?;
        let area = isle.work_area(&output);
        let size = if size.w > area.size.w || size.h > area.size.h {
            let fitted = geometry::default_size(area);
            record.configure_size(fitted);
            fitted
        } else {
            size
        };
        let taken: Vec<Point<i32, smithay::utils::Logical>> = isle
            .wm
            .records
            .iter()
            .filter(|r| r.id != id && r.shown() && r.app == record.app)
            .filter_map(|r| isle.space.element_location(&r.window))
            .collect();
        let step = isle.shell.theme().tokens.size_titlebar.round() as i32;
        Some(geometry::cascade(
            geometry::centred(area, size),
            &taken,
            step,
            area,
            size,
        ))
    }

    /// Moves `id` to `loc`, its modal dialogs with it.
    pub fn move_window(&mut self, id: u64, loc: Point<i32, smithay::utils::Logical>) {
        let Some(record) = self.isle.wm.get_mut(id) else {
            return;
        };
        let from = self
            .isle
            .space
            .element_location(&record.window)
            .unwrap_or(record.rect.loc);
        let delta = loc - from;
        record.rect.loc = loc;
        let window = record.window.clone();
        if self.isle.space.element_location(&window).is_some() {
            self.isle.queue_redraw_for_window(&window);
            self.isle.space.map_element(window.clone(), loc, false);
            self.isle.queue_redraw_for_window(&window);
        }
        if delta == Point::from((0, 0)) {
            return;
        }
        let children: Vec<(u64, Point<i32, smithay::utils::Logical>)> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| r.parent == Some(id) && r.modal && r.mapped)
            .map(|r| {
                let at = self
                    .isle
                    .space
                    .element_location(&r.window)
                    .unwrap_or(r.rect.loc);
                (r.id, at + delta)
            })
            .collect();
        for (child, at) in children {
            self.move_window(child, at);
        }
    }
}
