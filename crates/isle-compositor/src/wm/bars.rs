//! Isle's title bars on windows that let the compositor decorate them (B25), and the resize
//! band around their frames (`compositor.md` Server-side decorations).

use std::rc::Rc;

use isle_shell::chrome::{BarModel, TitleBar};
use smithay::{
    utils::{Logical, Point, Rectangle},
    wayland::{compositor::with_states, shell::xdg::SurfaceCachedState},
};

use super::Mode;
use crate::{
    grabs::Edges,
    state::{Isle, State},
};

impl Isle {
    /// Whether `id` wears Isle's bar: it asked for server-side decorations, bars are on, and
    /// it is not full screen.
    pub fn wants_bar(&self, id: u64) -> bool {
        self.wm.get(id).is_some_and(|r| {
            r.server_side
                && self.settings.windows.title_bars
                && !matches!(r.mode, Mode::FullScreen { .. })
        })
    }

    fn bar_model(&self, id: u64) -> Option<BarModel> {
        let record = self.wm.get(id)?;
        let (min, max) = record
            .surface()
            .map(|s| {
                with_states(&s, |states| {
                    let mut cached = states.cached_state.get::<SurfaceCachedState>();
                    let current = cached.current();
                    (current.min_size, current.max_size)
                })
            })
            .unwrap_or_default();
        let fixed = min.w > 0 && min == max;
        let active = self
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .and_then(|f| f.surface().cloned())
            .is_some_and(|s| record.surface().as_ref() == Some(&s));
        Some(BarModel {
            title: record.names().1,
            active,
            can_minimise: record.parent.is_none(),
            can_zoom: !fixed,
            full_screen: matches!(record.mode, Mode::FullScreen { .. }),
            option: self.bar_option == Some(id),
        })
    }

    /// The bar's place: the top of the window's frame on the output most of it is on.
    fn bar_place(&self, id: u64) -> Option<(isle_shell::OutputPlace, Rectangle<i32, Logical>)> {
        let frame = self.frame(id)?;
        let output = self.output_of(id)?;
        let geometry = self.space.output_geometry(&output)?;
        let place = isle_shell::OutputPlace {
            scale: output.current_scale().fractional_scale(),
            output,
            geometry,
        };
        let bar = Rectangle::new(
            frame.loc - geometry.loc,
            (frame.size.w, self.bar_height(id)).into(),
        );
        Some((place, bar))
    }

    /// Keeps `id`'s bar over the top of its frame.
    pub fn place_bar(&mut self, id: u64) {
        self.place_alert(id);
        if self.wm.get(id).is_none_or(|r| r.bar.is_none()) {
            return;
        }
        if let Some((place, bar)) = self.bar_place(id) {
            self.shell.place_attached(id, &place, bar);
        }
    }

    /// Gives each mapped window that wants a bar one, takes it from those that no longer do,
    /// and brings each bar's model up to date.
    pub fn sync_bars(&mut self) {
        let ids: Vec<u64> = self.wm.records.iter().map(|r| r.id).collect();
        for id in ids {
            let record = self.wm.get(id).unwrap();
            let wants = record.mapped && self.wants_bar(id);
            match (wants, record.bar.clone()) {
                (true, None) => {
                    let (Some(model), Some((place, bar))) =
                        (self.bar_model(id), self.bar_place(id))
                    else {
                        continue;
                    };
                    let def = TitleBar::new(model);
                    self.shell.attach(def.clone(), id, &place, bar);
                    self.wm.get_mut(id).unwrap().bar = Some(def);
                }
                (false, Some(_)) => {
                    self.shell.detach(id);
                    self.wm.get_mut(id).unwrap().bar = None;
                }
                (true, Some(def)) => {
                    if let Some(model) = self.bar_model(id)
                        && model != def.model()
                    {
                        self.shell.with_attached(id, || def.set(model));
                    }
                    self.place_bar(id);
                }
                (false, None) => {}
            }
        }
    }

    /// What lies under `pos` of the decorated window `id`: its bar, or the resize band that
    /// straddles its frame's edge.
    pub fn bar_or_edge_at(&self, id: u64, pos: Point<f64, Logical>) -> Option<crate::focus::Focus> {
        let record = self.wm.get(id)?;
        if !self.wants_bar(id) {
            return None;
        }
        let frame = self.frame(id)?.to_f64();
        let band = self.shell.theme().tokens.size_resize_band as f64 / 2.0;
        let corner = band * 4.0;
        let outer = Rectangle::new(
            (frame.loc.x - band, frame.loc.y - band).into(),
            (frame.size.w + 2.0 * band, frame.size.h + 2.0 * band).into(),
        );
        if !outer.contains(pos) {
            return None;
        }
        let (x0, y0) = (frame.loc.x, frame.loc.y);
        let (x1, y1) = (x0 + frame.size.w, y0 + frame.size.h);
        let near = |p: f64, edge: f64, reach: f64| (p - edge).abs() <= reach;
        let mut edges = 0;
        let left = near(pos.x, x0, band);
        let right = near(pos.x, x1, band);
        let top = near(pos.y, y0, band);
        let bottom = near(pos.y, y1, band);
        if left || (top || bottom) && near(pos.x, x0, corner) {
            edges |= Edges::LEFT;
        }
        if right || (top || bottom) && near(pos.x, x1, corner) {
            edges |= Edges::RIGHT;
        }
        if top || (left || right) && near(pos.y, y0, corner) {
            edges |= Edges::TOP;
        }
        if bottom || (left || right) && near(pos.y, y1, corner) {
            edges |= Edges::BOTTOM;
        }
        if edges != 0 {
            return Some(crate::focus::Focus::Edge { window: id, edges });
        }
        let bar = self.bar_height(id) as f64;
        if record.bar.is_some() && pos.y < y0 + bar {
            return self
                .shell
                .attached_focus(id)
                .map(crate::focus::Focus::Shell);
        }
        None
    }
}

impl State {
    /// Option pressed or let go: the bar under the pointer shows zoom's plus.
    pub fn update_bar_option(&mut self) {
        let alt = self.held_mods().option;
        let under = self
            .isle
            .seat
            .get_pointer()
            .and_then(|p| p.current_focus())
            .and_then(|f| match f {
                crate::focus::Focus::Shell(s) => self.isle.shell.attached_key(&s),
                _ => None,
            });
        let wanted = under.filter(|_| alt);
        if wanted != self.isle.bar_option {
            self.isle.bar_option = wanted;
            self.isle.sync_bars();
        }
    }

    /// What a bar asked of its window.
    pub fn bar_action(&mut self, id: u64, action: isle_shell::WindowAction) {
        tracing::debug!(id, ?action, "bar");
        use isle_shell::WindowAction as A;
        let app = self.isle.wm.get(id).map(|r| r.app.clone());
        match action {
            A::Close => {
                self.close_window(id);
            }
            A::CloseApp => {
                let ids: Vec<u64> = self
                    .isle
                    .wm
                    .records
                    .iter()
                    .filter(|r| Some(&r.app) == app.as_ref() && r.mapped)
                    .map(|r| r.id)
                    .collect();
                for id in ids {
                    self.close_window(id);
                }
            }
            A::Minimise => {
                self.minimise(id);
            }
            A::MinimiseApp => {
                let window = self.isle.window_by_id(id);
                self.set_focus(window.as_ref());
                self.minimise_app();
            }
            A::Zoom => {
                self.disarm_zoom_dwell(id);
                self.toggle_full_screen(id);
            }
            A::Fill => {
                self.disarm_zoom_dwell(id);
                self.toggle_fill(id);
            }
            A::TitlebarAction => {
                use isle_settings::schema::TitlebarAction;
                match self.isle.settings.windows.titlebar_action {
                    TitlebarAction::Fill => self.toggle_fill(id),
                    TitlebarAction::Minimise => {
                        self.minimise(id);
                    }
                    TitlebarAction::None => {}
                }
            }
            A::Move { raise } => {
                if raise {
                    let window = self.isle.window_by_id(id);
                    self.set_focus(window.as_ref());
                }
                self.begin_bar_move(id);
            }
            A::Menu => {
                let at = self.isle.seat.get_pointer().map(|p| p.current_location().to_i32_round());
                self.open_window_menu(id, at, false);
            }
        }
    }

    /// A press on a bar: moves the window while the button stays down.
    fn begin_bar_move(&mut self, id: u64) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        if !self.isle.buttons_down.contains(&0x110) {
            return;
        }
        let Some(window) = self.isle.window_by_id(id) else {
            return;
        };
        let start_data = smithay::input::pointer::GrabStartData {
            focus: None,
            button: 0x110,
            location: pointer.current_location(),
        };
        if let Some(grab) = crate::grabs::MoveGrab::new(self, start_data, window) {
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            pointer.set_grab(self, grab, serial, smithay::input::pointer::Focus::Clear);
        }
    }

    /// A press on a window's resize band starts a resize from those edges.
    pub fn begin_edge_resize(&mut self, id: u64, edges: u32, button: u32) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        let Some(window) = self.isle.window_by_id(id) else {
            return;
        };
        let Some(rect) = self.isle.space.element_geometry(&window) else {
            return;
        };
        self.float(id);
        let start_data = smithay::input::pointer::GrabStartData {
            focus: None,
            button,
            location: pointer.current_location(),
        };
        let grab = crate::grabs::ResizeGrab::start(start_data, window, Edges::new(edges), rect);
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        pointer.set_grab(self, grab, serial, smithay::input::pointer::Focus::Clear);
    }
}

/// Rc so the shell and the record share one definition.
pub type Bar = Rc<TitleBar>;
