//! The zoom menu, opened by a dwell on the zoom light or its chord, and the pointer panel
//! that brings the zones to the pointer while Option is held in a drag (`p1/surfaces/
//! window-chrome.md`).

use std::time::Duration;

use isle_shell::zones::{Beside, Grid, ZoneAction, ZoneModel};
use smithay::{
    output::Output,
    reexports::calloop::timer::{TimeoutAction, Timer},
    utils::{Logical, Point, Rectangle},
};

use super::{Mode, geometry::Zone};
use crate::state::{Isle, State};

/// Where the zone surfaces stand now.
#[derive(Default)]
pub struct ZoomState {
    /// The window the zoom menu is open for.
    pub menu_for: Option<u64>,
    /// The window being dragged with the pointer panel open, and the panel's origin.
    pub panel_for: Option<(u64, Point<i32, Logical>)>,
    /// The dwell under way over a zoom light: its window, and its turn, so a later one
    /// cancels it.
    dwell: Option<(u64, u64)>,
    turn: u64,
    /// A click on the zoom light disarms the dwell until the pointer leaves it.
    disarmed: Option<u64>,
}

impl Isle {
    /// What the zone grid shows for window `id` on `output`.
    pub fn zone_model(&self, id: u64, output: &Output, by_key: bool) -> ZoneModel {
        let screen = self.space.output_geometry(output).unwrap_or_default();
        let area = self.work_area(output);
        let record = self.wm.get(id);
        let current = match record.map(|r| &r.mode) {
            Some(Mode::Tiled { zone, .. }) => Some(zone.name().to_string()),
            Some(Mode::Filled { .. }) => Some("fill".to_string()),
            _ => None,
        };
        let frac = |r: Rectangle<i32, Logical>| {
            [
                (r.loc.x - area.loc.x) as f32 / area.size.w as f32,
                (r.loc.y - area.loc.y) as f32 / area.size.h as f32,
                r.size.w as f32 / area.size.w as f32,
                r.size.h as f32 / area.size.h as f32,
            ]
        };
        let beside = Zone::Beside
            .rect(area, 0, &self.other_frames(id, output))
            .map(|gap| Beside {
                windows: self
                    .other_frames(id, output)
                    .into_iter()
                    .map(frac)
                    .collect(),
                gap: frac(gap),
            });
        ZoneModel {
            aspect: screen.size.w as f32 / screen.size.h.max(1) as f32,
            beside,
            can_restore: current.is_some(),
            current,
            selected: None,
            full_screen: matches!(record.map(|r| &r.mode), Some(Mode::FullScreen { .. })),
            displays: self
                .space
                .outputs()
                .filter(|o| *o != output)
                .map(|o| o.name())
                .collect(),
            by_key,
        }
    }

    fn output_place(&self, output: &Output) -> Option<isle_shell::OutputPlace> {
        Some(isle_shell::OutputPlace {
            output: output.clone(),
            geometry: self.space.output_geometry(output)?,
            scale: output.current_scale().fractional_scale(),
        })
    }
}

impl State {
    /// Opens the zoom menu for window `id`: under its zoom light when it wears Isle's bar,
    /// else inside its top leading corner.
    pub fn open_zoom_menu(&mut self, id: u64, by_key: bool) {
        let (Some(frame), Some(output)) = (self.isle.frame(id), self.isle.output_of(id)) else {
            return;
        };
        let Some(place) = self.isle.output_place(&output) else {
            return;
        };
        let model = self.isle.zone_model(id, &output, by_key);
        let theme = self.isle.shell.theme().clone();
        let size = isle_shell::zones::menu_size(&model, &theme);
        let t = &theme.tokens;
        let at = if self.isle.bar_height(id) > 0 {
            Point::from((
                frame.loc.x + (t.space_l + 2.0 * (t.size_traffic_light + t.space_s)) as i32,
                frame.loc.y + self.isle.bar_height(id) + t.space_xs as i32,
            ))
        } else {
            Point::from((
                frame.loc.x + t.space_l as i32,
                frame.loc.y + t.space_xs as i32,
            ))
        };
        let rect = super::geometry::clamp_whole(Rectangle::new(at, size), place.geometry);
        self.isle.zoom.menu_for = Some(id);
        let menu = self.isle.zoom_menu.clone();
        menu.set_model(model.clone());
        self.isle.shell.with_root("zoom-menu", || menu.set(model));
        self.isle.shell.open_placed(
            "zoom-menu",
            &place,
            Rectangle::new(rect.loc - place.geometry.loc, rect.size),
        );
        if by_key {
            self.open_surface("zoom-menu");
        } else {
            self.dispatch_shell();
        }
    }

    pub fn close_zoom_menu(&mut self) {
        if let Some(id) = self.isle.zoom.menu_for.take() {
            self.isle.set_zone_preview(id, None);
        }
        if self.isle.shell.is_open("zoom-menu") {
            self.close_surface("zoom-menu");
        }
    }

    /// Ctrl+Option+Z: the zoom menu for the focused window, by keyboard.
    pub fn zoom_menu_by_key(&mut self) {
        if let Some(id) = self.focused_window() {
            self.open_zoom_menu(id, true);
        }
    }

    /// What the zoom menu asked.
    pub fn zone_action(&mut self, action: ZoneAction) {
        tracing::debug!(?action, menu_for = ?self.isle.zoom.menu_for, "zoom menu");
        let Some(id) = self.isle.zoom.menu_for else {
            return;
        };
        match action {
            ZoneAction::Preview(zone) => {
                let target = zone.as_deref().and_then(Zone::from_name).and_then(|zone| {
                    let output = self.isle.output_of(id)?;
                    let rect = self.isle.zone_rect(id, zone, &output)?;
                    Some((output, zone, rect))
                });
                self.isle.set_zone_preview(id, target);
            }
            ZoneAction::Choose(zone) => {
                self.close_zoom_menu();
                if let Some(zone) = Zone::from_name(&zone) {
                    self.tile(id, zone);
                }
            }
            ZoneAction::Restore => {
                self.close_zoom_menu();
                self.restore_box(id);
            }
            ZoneAction::FullScreen => {
                self.close_zoom_menu();
                self.toggle_full_screen(id);
            }
            ZoneAction::Display(index) => {
                self.close_zoom_menu();
                let Some(from) = self.isle.output_of(id) else {
                    return;
                };
                let others: Vec<Output> = self
                    .isle
                    .space
                    .outputs()
                    .filter(|o| **o != from)
                    .cloned()
                    .collect();
                if let Some(to) = others.get(index) {
                    let window = self.isle.window_by_id(id);
                    self.set_focus(window.as_ref());
                    let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
                    let a = outputs.iter().position(|o| *o == from).unwrap_or(0) as i32;
                    let b = outputs.iter().position(|o| o == to).unwrap_or(0) as i32;
                    self.move_to_display(b - a);
                }
            }
        }
    }

    /// After the pointer moved: arms or disarms the dwell over a zoom light.
    pub fn check_zoom_dwell(&mut self) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        let at = pointer.current_location();
        let over = self
            .isle
            .space
            .elements()
            .rev()
            .map(super::window_id)
            .find(|id| {
                self.isle
                    .frame(*id)
                    .is_some_and(|f| f.to_f64().contains(at))
            })
            .filter(|id| self.isle.over_zoom_light(*id, at));
        if self.isle.zoom.disarmed.is_some() && self.isle.zoom.disarmed != over {
            self.isle.zoom.disarmed = None;
        }
        let Some(id) = over.filter(|id| {
            self.isle.buttons_down.is_empty()
                && self.isle.zoom.disarmed != Some(*id)
                && self.isle.zoom.menu_for != Some(*id)
        }) else {
            self.isle.zoom.dwell = None;
            return;
        };
        if self.isle.zoom.dwell.is_some_and(|(w, _)| w == id) {
            return;
        }
        self.isle.zoom.turn += 1;
        let turn = self.isle.zoom.turn;
        self.isle.zoom.dwell = Some((id, turn));
        let wait = self.isle.shell.theme().tokens.timing_zoom_dwell_ms;
        let _ = self.isle.loop_handle.insert_source(
            Timer::from_duration(Duration::from_millis(wait as u64)),
            move |_, _, state| {
                if state.isle.zoom.dwell == Some((id, turn)) {
                    state.isle.zoom.dwell = None;
                    state.open_zoom_menu(id, false);
                }
                TimeoutAction::Drop
            },
        );
    }

    /// A click on a zoom light: no dwell opens the menu until the pointer leaves it.
    pub fn disarm_zoom_dwell(&mut self, id: u64) {
        self.isle.zoom.disarmed = Some(id);
        self.isle.zoom.dwell = None;
    }

    /// During a drag of `id` with the pointer at `at`: Option shows the pointer panel beside
    /// the pointer, and a zone under the pointer in it is the one the drop tiles to.
    /// Returns whether the panel decides the preview.
    pub fn drag_panel(&mut self, id: u64, at: Point<f64, Logical>) -> bool {
        let alt = self.held_mods().option;
        if !alt {
            self.close_zone_panel();
            return false;
        }
        let Some(output) = self.isle.space.output_under(at).next().cloned() else {
            return false;
        };
        let theme = self.isle.shell.theme().clone();
        let mut model = self.isle.zone_model(id, &output, false);
        if self.isle.zoom.panel_for.is_none_or(|(w, _)| w != id) {
            let Some(place) = self.isle.output_place(&output) else {
                return false;
            };
            let size = isle_shell::zones::panel_size(&model, &theme);
            let gap = theme.tokens.space_xxl as i32;
            let y = at.y as i32 - size.h / 2;
            let mut x = at.x as i32 + gap;
            if x + size.w > place.geometry.loc.x + place.geometry.size.w {
                x = at.x as i32 - gap - size.w;
            }
            let rect =
                super::geometry::clamp_whole(Rectangle::new((x, y).into(), size), place.geometry);
            self.isle.zoom.panel_for = Some((id, rect.loc));
            let panel = self.isle.zone_panel.clone();
            panel.set_model(model.clone());
            self.isle.shell.open_placed(
                "zone-panel",
                &place,
                Rectangle::new(rect.loc - place.geometry.loc, rect.size),
            );
        }
        let origin = self.isle.zoom.panel_for.unwrap().1;
        let grid = Grid::of(&model, &theme);
        let (gx, gy) = isle_shell::zones::panel_grid_origin(&theme);
        let local = (
            (at.x - origin.x as f64) as f32 - gx,
            (at.y - origin.y as f64) as f32 - gy,
        );
        let zone = grid.zone_at(local).map(str::to_string);
        model.selected = zone.clone();
        let panel = self.isle.zone_panel.clone();
        if panel.model() != model {
            self.isle.shell.with_root("zone-panel", || panel.set(model));
        }
        let target = zone.as_deref().and_then(Zone::from_name).and_then(|zone| {
            let rect = self.isle.zone_rect(id, zone, &output)?;
            Some((output.clone(), zone, rect))
        });
        self.isle.set_zone_preview(id, target);
        true
    }

    /// Runs inside the drag's pointer grab, so it touches neither the pointer nor the
    /// keyboard focus: the panel takes neither.
    pub fn close_zone_panel(&mut self) {
        if self.isle.zoom.panel_for.take().is_some() {
            self.isle.shell.close("zone-panel");
            self.isle.queue_redraw_all();
        }
    }
}

impl Isle {
    /// Whether `at` is on the zoom light's hit area of `id`'s bar.
    fn over_zoom_light(&self, id: u64, at: Point<f64, Logical>) -> bool {
        let bar = self.bar_height(id) as f64;
        if bar == 0.0 || self.wm.get(id).is_none_or(|r| r.bar.is_none()) {
            return false;
        }
        let Some(frame) = self.frame(id) else {
            return false;
        };
        let x = at.x - frame.loc.x as f64;
        let y = at.y - frame.loc.y as f64;
        (52.0..72.0).contains(&x) && (0.0..bar).contains(&y)
    }
}
