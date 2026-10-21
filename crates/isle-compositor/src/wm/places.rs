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
        let record = self.wm.get(id);
        let wayland_parent = record
            .and_then(|r| r.window.toplevel())
            .and_then(|t| t.parent())
            .and_then(|p| self.wm.for_surface(&p));
        let x11_parent = record
            .and_then(|r| r.x11())
            .and_then(|x| x.is_transient_for())
            .and_then(|parent| {
                self.wm
                    .records
                    .iter()
                    .find(|r| r.x11().is_some_and(|x| x.window_id() == parent))
            });
        let float_parent = record.and_then(|r| r.surface()).and_then(|s| self.float_parent(&s));
        let parent = wayland_parent.or(x11_parent).map(|r| r.id).or(float_parent);
        if let Some(record) = self.wm.get_mut(id) {
            record.app = self.apps.resolve(&record.names().0);
            record.parent = parent;
        }
        self.refresh_game(id);
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

    /// `app`'s box on `output`, or its latest anywhere clamped onto `output`, and the zone it
    /// was tiled in.
    fn place_for(&self, app: &str, output: &Output) -> Option<(Rect, Option<geometry::Zone>)> {
        let places = self.places.apps.get(app)?;
        let area = self.work_area(output);
        let place = places
            .get(&Self::output_identity(output))
            .or_else(|| places.values().max_by_key(|p| p.at))?;
        let rect = Rectangle::new(
            (area.loc.x + place.x, area.loc.y + place.y).into(),
            (place.width, place.height).into(),
        );
        let zone = place.zone.as_deref().and_then(geometry::Zone::from_name);
        Some((geometry::clamp_whole(rect, area), zone))
    }

    /// Remembers `id`'s box, and its zone when tiled or filled, if it is a main window wholly
    /// on one output.
    pub fn record_place(&mut self, id: u64) {
        if let Some(frame) = self.frame(id) {
            self.record_place_at(id, frame);
        }
    }

    /// As `record_place`, for a window being given `frame`, which its client has yet to take.
    pub fn record_place_at(&mut self, id: u64, frame: Rect) {
        if !self.is_main(id) {
            return;
        }
        let Some(record) = self.wm.get(id) else {
            return;
        };
        if record.game {
            return;
        }
        let (rect, zone) = match &record.mode {
            super::Mode::Floating => (frame, None),
            super::Mode::Tiled { zone, restore } => (*restore, Some(zone.name())),
            super::Mode::Filled { restore, .. } => (*restore, Some(geometry::Zone::Fill.name())),
            super::Mode::FullScreen { .. } => return,
        };
        let Some(output) = self.output_holding(frame) else {
            debug!(id, ?frame, "not on one output; its place is not recorded");
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
            zone: zone.map(str::to_owned),
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
        self.isle.refresh_identity(id);
        if matches!(
            self.isle.wm.get(id).map(|r| &r.mode),
            Some(super::Mode::Filled { .. })
        ) {
            self.plan_fill(id);
            return;
        }
        if self.isle.wm.get(id).is_some_and(|r| r.wants_full_screen) {
            self.plan_full_screen(id);
            return;
        }
        let isle = &mut self.isle;
        if !isle.is_main(id) {
            return;
        }
        // A game is never given a remembered place (`gaming.md` Games 7).
        isle.refresh_game(id);
        if isle.wm.get(id).is_some_and(|r| r.game) {
            return;
        }
        let app = isle.wm.get(id).unwrap().app.clone();
        let Some(output) = isle.output_with_focus() else {
            return;
        };
        let Some((place, zone)) = isle.place_for(&app, &output) else {
            return;
        };
        // A window last tiled reopens in its tile, its box kept for when it leaves it.
        if let Some(zone) = zone
            && let Some(rect) = isle.zone_rect(id, zone, &output)
        {
            let bar = isle.bar_height(id);
            let size = (rect.size.w, (rect.size.h - bar).max(1));
            let record = isle.wm.get_mut(id).unwrap();
            record.mode = if zone == geometry::Zone::Fill {
                super::Mode::Filled {
                    restore: place,
                    tile: None,
                }
            } else {
                super::Mode::Tiled {
                    zone,
                    restore: place,
                }
            };
            record.planned = Some(Rectangle::new((rect.loc.x, rect.loc.y + bar).into(), size.into()));
            if let Some(toplevel) = record.window.toplevel() {
                toplevel.with_pending_state(|state| state.size = Some(size.into()));
            }
            self.send_mode_states(id);
            return;
        }
        // The place is the frame's; the client is asked for the part under its bar.
        let bar = isle.bar_height(id);
        let client = Rectangle::new(
            (place.loc.x, place.loc.y + bar).into(),
            (place.size.w, (place.size.h - bar).max(1)).into(),
        );
        let record = isle.wm.get_mut(id).unwrap();
        record.planned = Some(client);
        if let Some(toplevel) = record.window.toplevel() {
            toplevel.with_pending_state(|state| state.size = Some(client.size));
        }
    }

    /// A window that asked to be maximised before it mapped opens filling the work area of
    /// the output with focus, returning to the default size centred there.
    pub fn plan_fill(&mut self, id: u64) {
        let Some(output) = self.isle.output_with_focus() else {
            return;
        };
        let area = self.isle.work_area(&output);
        let size = geometry::default_size(area);
        let restore = Rectangle::new(geometry::centred(area, size), size);
        let Some(rect) = self.isle.zone_rect(id, geometry::Zone::Fill, &output) else {
            return;
        };
        let bar = self.isle.bar_height(id);
        let record = self.isle.wm.get_mut(id).unwrap();
        record.mode = super::Mode::Filled {
            restore,
            tile: None,
        };
        record.planned = Some(Rectangle::new(
            (rect.loc.x, rect.loc.y + bar).into(),
            (rect.size.w, rect.size.h - bar).into(),
        ));
        if let Some(toplevel) = record.window.toplevel() {
            toplevel.with_pending_state(|state| {
                state.size = Some((rect.size.w, rect.size.h - bar).into())
            });
        }
        self.send_mode_states(id);
    }

    /// A window that asked for full screen before it mapped is configured to its output.
    pub fn plan_full_screen(&mut self, id: u64) {
        let Some(output) = self.isle.output_with_focus() else {
            return;
        };
        let area = self.isle.space.output_geometry(&output).unwrap_or_default();
        let record = self.isle.wm.get_mut(id).unwrap();
        record.planned = Some(area);
        if let Some(toplevel) = record.window.toplevel() {
            toplevel.with_pending_state(|state| {
                state.size = Some(area.size);
                state.states.set(smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Fullscreen);
            });
        }
    }

    /// At map, once placed: the desktop the window opens on. A dialog opens on its parent's;
    /// any other window on the current desktop of its output, or the one a full-screen
    /// desktop came from.
    pub fn assign_desktop(&mut self, id: u64) {
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        if let Some(parent) = record.parent.and_then(|p| self.isle.wm.get(p)) {
            let desktop = parent.desktop;
            self.isle.wm.get_mut(id).unwrap().desktop = desktop;
            return;
        }
        let Some(output) = self.isle.output_of(id) else {
            return;
        };
        let current = self.isle.current_desktop(&output);
        let identity = Isle::output_identity(&output);
        let row = &self.isle.desktops.rows[&identity];
        let full_screen =
            row.desktops
                .iter()
                .find(|d| d.id == current)
                .and_then(|d| match d.kind {
                    super::desktops::Kind::FullScreen { window } => Some(window),
                    _ => None,
                });
        let desktop = match full_screen.and_then(|w| self.isle.wm.get(w)) {
            Some(window) => match &window.mode {
                super::Mode::FullScreen { origin, .. } => *origin,
                _ => current,
            },
            None => current,
        };
        self.isle.wm.get_mut(id).unwrap().desktop = desktop;
        if let Some((_, index)) = self.isle.desktops.locate(desktop) {
            self.isle.desktops.rows.get_mut(&identity).unwrap().current = index;
        }
    }

    /// At map: the window's place, over its parent, or centred on the output with focus.
    pub fn place_new_window(&mut self, id: u64) -> Option<Point<i32, smithay::utils::Logical>> {
        let isle = &self.isle;
        let bar = isle.bar_height(id);
        let under_bar = |frame: Point<i32, smithay::utils::Logical>| frame + Point::from((0, bar));
        let record = isle.wm.get(id)?;
        let client = record.window.geometry().size;
        // Placement works on frames: the client and the bar over it.
        let size: smithay::utils::Size<i32, smithay::utils::Logical> = (client.w, client.h + bar).into();
        if let Some(planned) = record.planned {
            let output = isle
                .output_holding(planned)
                .or_else(|| isle.output_with_focus())?;
            let rect = Rectangle::new((planned.loc.x, planned.loc.y - bar).into(), size);
            return Some(under_bar(geometry::clamp_whole(rect, isle.work_area(&output)).loc));
        }
        // An X11 game opens where it asked, when that is on a screen.
        if record.game
            && let Some(x11) = record.x11()
        {
            let asked = x11.geometry().loc;
            if isle
                .space
                .outputs()
                .filter_map(|o| isle.space.output_geometry(o))
                .any(|g| g.contains(asked))
            {
                return Some(asked);
            }
        }
        let parent = record
            .parent
            .and_then(|p| isle.wm.get(p))
            .filter(|p| isle.wm.is_shown(p.id));
        if let Some(parent) = parent {
            let frame = isle.frame(parent.id)?;
            let output = isle
                .output_holding(frame)
                .or_else(|| isle.output_with_focus())?;
            let area = isle.work_area(&output);
            let parent_bar = isle.bar_height(parent.id);
            if record.modal && size.w < frame.size.w && size.h < frame.size.h - parent_bar {
                return Some(under_bar(geometry::under_bar(frame, parent_bar, size)));
            }
            return Some(under_bar(geometry::over_parent(frame, size, area)));
        }
        let output = isle.output_with_focus()?;
        let screen = isle.space.output_geometry(&output)?;
        if client == screen.size {
            // Sized to its output by itself: a borderless full-screen window (D48).
            return Some(screen.loc);
        }
        let area = isle.work_area(&output);
        let size = if size.w > area.size.w || size.h > area.size.h {
            let fitted = geometry::default_size(area);
            record.configure_size((fitted.w, fitted.h - bar).into());
            fitted
        } else {
            size
        };
        let taken: Vec<Point<i32, smithay::utils::Logical>> = isle
            .wm
            .records
            .iter()
            .filter(|r| r.id != id && isle.wm.is_shown(r.id) && r.app == record.app)
            .filter_map(|r| isle.frame(r.id).map(|f| f.loc))
            .collect();
        let step = isle.shell.theme().tokens.size_titlebar.round() as i32;
        Some(under_bar(geometry::cascade(
            geometry::centred(area, size),
            &taken,
            step,
            area,
            size,
        )))
    }

    /// Moves `id` to `loc`, its modal dialogs with it.
    pub fn move_window(&mut self, id: u64, loc: Point<i32, smithay::utils::Logical>) {
        self.isle.move_window(id, loc);
    }
}

impl Isle {
    /// Moves `id` to `loc`, its modal dialogs with it.
    pub fn move_window(&mut self, id: u64, loc: Point<i32, smithay::utils::Logical>) {
        let Some(record) = self.wm.get_mut(id) else {
            return;
        };
        let from = self
            .space
            .element_location(&record.window)
            .unwrap_or(record.rect.loc);
        let delta = loc - from;
        record.rect.loc = loc;
        let window = record.window.clone();
        if self.space.element_location(&window).is_some() {
            self.queue_redraw_for_window(&window);
            self.space.map_element(window.clone(), loc, false);
            self.queue_redraw_for_window(&window);
        }
        // An X11 client knows where it is on the root window.
        if let Some(x11) = window.x11_surface() {
            let _ = x11.configure(Rectangle::new(loc, x11.last_configure().size));
        }
        self.place_bar(id);
        if delta == Point::from((0, 0)) {
            return;
        }
        let children: Vec<(u64, Point<i32, smithay::utils::Logical>)> = self
            .wm
            .records
            .iter()
            .filter(|r| r.parent == Some(id) && r.modal && r.mapped)
            .map(|r| {
                let at = self.space.element_location(&r.window).unwrap_or(r.rect.loc);
                (r.id, at + delta)
            })
            .collect();
        for (child, at) in children {
            self.move_window(child, at);
        }
    }
}

impl State {
    /// Puts a window that has its first frame on the screen: at its place, on its desktop,
    /// focused, opening.
    pub fn map_window(&mut self, id: u64) {
        let Some(window) = self.isle.window_by_id(id) else {
            return;
        };
        let size = window.geometry().size;
        self.isle.refresh_identity(id);
        self.isle.refresh_game(id);
        let Some(location) = self.place_new_window(id) else {
            return;
        };
        let record = self.isle.wm.get_mut(id).unwrap();
        let (app_id, title) = record.names();
        record.mapped = true;
        record.planned = None;
        record.rect = Rectangle::new(location, size);
        let full_screen = std::mem::take(&mut record.wants_full_screen);
        if let Some(x11) = record.x11() {
            let _ = x11.configure(Rectangle::new(location, size));
        }
        tracing::info!(id, app_id, title, ?location, size = ?size, "mapped");
        let (app, pid) = (record.app.clone(), record.pid);
        self.track_process(&app, pid);
        self.isle.refresh_game(id);
        self.read_process(id);
        self.assign_desktop(id);
        self.isle.sync_space();
        // A float leaves the keyboard with its caller.
        if self.isle.float_of(id).is_none() {
            self.set_focus(Some(&window));
        }
        if full_screen {
            self.enter_full_screen(id, false);
        } else {
            self.check_borderless(id);
        }
        if matches!(
            self.isle.wm.get(id).map(|r| &r.mode),
            Some(super::Mode::Floating | super::Mode::Tiled { .. } | super::Mode::Filled { .. })
        ) {
            self.animate_arriving(id, None);
        }
    }
}
