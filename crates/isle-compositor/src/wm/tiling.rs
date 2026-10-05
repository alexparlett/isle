//! Tiling to zones, fill and the way back (`compositor.md` Snapping, Minimise and maximise).

use smithay::{
    output::Output,
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Point, Rectangle},
};

use super::{
    Mode,
    geometry::{Rect, Zone},
};
use crate::state::{Isle, State};

impl Isle {
    /// Where a minimised window shrinks to: a small box of its shape into the island on its
    /// output, or under the strip at the top centre while the island shows nothing.
    pub fn minimise_target(&self, id: u64) -> Option<Rect> {
        let frame = self.frame(id)?;
        let output = self.output_of(id)?;
        let screen = self.space.output_geometry(&output)?;
        let tokens = &self.shell.theme().tokens;
        let island = self.island_size(&output).filter(|(w, _)| *w >= 1.0);
        let w = island.map_or(tokens.size_minimise_target, |(w, _)| w.min(tokens.size_minimise_target)).round() as i32;
        let h = (w as f64 * frame.size.h as f64 / frame.size.w.max(1) as f64).round() as i32;
        let y = match island {
            Some((_, ih)) => screen.loc.y + (tokens.size_island_top + ih / 2.0).round() as i32 - h / 2,
            None => screen.loc.y + self.strip() + tokens.space_s.round() as i32,
        };
        Some(Rectangle::new(
            (screen.loc.x + (screen.size.w - w) / 2, y).into(),
            (w, h.max(1)).into(),
        ))
    }

    /// The height of the title bar the compositor draws over `id`, or 0.
    pub fn bar_height(&self, id: u64) -> i32 {
        if self.wants_bar(id) {
            isle_shell::chrome::bar_height(self.shell.theme())
        } else {
            0
        }
    }

    /// The window's frame: its box and the bar over it.
    pub fn frame(&self, id: u64) -> Option<Rect> {
        let record = self.wm.get(id)?;
        let rect = self
            .space
            .element_geometry(&record.window)
            .unwrap_or(record.rect);
        let bar = self.bar_height(id);
        Some(Rectangle::new(
            (rect.loc.x, rect.loc.y - bar).into(),
            (rect.size.w, rect.size.h + bar).into(),
        ))
    }

    /// The output most of the window is on.
    pub fn output_of(&self, id: u64) -> Option<Output> {
        let frame = self.frame(id)?;
        self.space
            .outputs()
            .filter_map(|o| {
                let overlap = self.space.output_geometry(o)?.intersection(frame)?;
                Some((o.clone(), overlap.size.w as i64 * overlap.size.h as i64))
            })
            .max_by_key(|(_, area)| *area)
            .map(|(o, _)| o)
            .or_else(|| self.output_with_focus())
    }

    pub fn tile_margin(&self) -> i32 {
        if self.settings.windows.tile_margins {
            self.shell.theme().tokens.space_m.round() as i32
        } else {
            0
        }
    }

    /// The frames of the windows shown on `output` other than `id`.
    pub fn other_frames(&self, id: u64, output: &Output) -> Vec<Rect> {
        let Some(area) = self.space.output_geometry(output) else {
            return Vec::new();
        };
        self.wm
            .records
            .iter()
            .filter(|r| r.id != id && self.wm.is_shown(r.id))
            .filter_map(|r| self.frame(r.id))
            .filter(|f| f.overlaps(area))
            .collect()
    }

    /// The frame `zone` gives `id` on `output`.
    pub fn zone_rect(&self, id: u64, zone: Zone, output: &Output) -> Option<Rect> {
        let area = self.work_area(output);
        zone.rect(area, self.tile_margin(), &self.other_frames(id, output))
    }
}

impl State {
    /// Configures `id` to fill `frame` less its bar, and moves it there.
    pub fn set_frame(&mut self, id: u64, frame: Rect) {
        self.begin_morph(id, frame);
        let bar = self.isle.bar_height(id);
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        record.configure_size((frame.size.w, (frame.size.h - bar).max(1)).into());
        self.move_window(id, Point::from((frame.loc.x, frame.loc.y + bar)));
    }

    /// Tiles `id` to `zone` on the output it is on; `Fill` fills.
    pub fn tile(&mut self, id: u64, zone: Zone) -> bool {
        let Some(output) = self.isle.output_of(id) else {
            return false;
        };
        self.tile_on(id, zone, &output)
    }

    pub fn tile_on(&mut self, id: u64, zone: Zone, output: &Output) -> bool {
        let Some(rect) = self.isle.zone_rect(id, zone, output) else {
            return false;
        };
        let Some(frame) = self.isle.frame(id) else {
            return false;
        };
        let Some(record) = self.isle.wm.get_mut(id) else {
            return false;
        };
        let restore = match &record.mode {
            Mode::Floating => frame,
            Mode::Tiled { restore, .. } | Mode::Filled { restore } => *restore,
            Mode::FullScreen { .. } => return false,
        };
        record.mode = if zone == Zone::Fill {
            Mode::Filled { restore }
        } else {
            Mode::Tiled { zone, restore }
        };
        self.send_mode_states(id);
        self.set_frame(id, rect);
        self.isle.notify(crate::worker::Event::WindowsChanged);
        true
    }

    /// The chord for `zone` on the focused window: again on the same family moves along its
    /// cycle.
    pub fn tile_by_chord(&mut self, zone: Zone) {
        let Some(id) = self.focused_window() else {
            return;
        };
        let current = match self.isle.wm.get(id).map(|r| &r.mode) {
            Some(Mode::Tiled { zone, .. }) => Some(*zone),
            _ => None,
        };
        let mut next = zone;
        if let Some(current) = current {
            let mut z = zone;
            for _ in 0..3 {
                if z == current {
                    next = current.next_in_cycle();
                    break;
                }
                z = z.next_in_cycle();
            }
        }
        self.tile(id, next);
    }

    /// Fills `id`, or returns it to its box when it is filled.
    pub fn toggle_fill(&mut self, id: u64) {
        if matches!(
            self.isle.wm.get(id).map(|r| &r.mode),
            Some(Mode::Filled { .. })
        ) {
            self.restore_box(id);
        } else {
            self.tile(id, Zone::Fill);
        }
    }

    /// Returns a tiled or filled window to the box it had before.
    pub fn restore_box(&mut self, id: u64) -> bool {
        let Some(record) = self.isle.wm.get_mut(id) else {
            return false;
        };
        let restore = match record.mode {
            Mode::Tiled { restore, .. } | Mode::Filled { restore } => restore,
            Mode::Floating | Mode::FullScreen { .. } => return false,
        };
        record.mode = Mode::Floating;
        self.send_mode_states(id);
        self.set_frame(id, restore);
        self.isle.notify(crate::worker::Event::WindowsChanged);
        true
    }

    /// Leaves a tile or fill without moving: a resize, or a drag that unsnaps.
    pub fn float(&mut self, id: u64) {
        if let Some(record) = self.isle.wm.get_mut(id)
            && matches!(record.mode, Mode::Tiled { .. } | Mode::Filled { .. })
        {
            record.mode = Mode::Floating;
            self.send_mode_states(id);
        }
    }

    /// Tells the client its mode: maximised when filled, tiled on every edge in a zone.
    pub fn send_mode_states(&self, id: u64) {
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        if let Some(x11) = record.x11() {
            let _ = x11.set_maximized(matches!(record.mode, Mode::Filled { .. }));
            return;
        }
        let Some(toplevel) = record.window.toplevel() else {
            return;
        };
        let (filled, tiled) = match record.mode {
            Mode::Filled { .. } => (true, false),
            Mode::Tiled { .. } => (false, true),
            Mode::Floating | Mode::FullScreen { .. } => (false, false),
        };
        toplevel.with_pending_state(|state| {
            let mut set = |s, on| {
                if on {
                    state.states.set(s);
                } else {
                    state.states.unset(s);
                }
            };
            set(xdg_toplevel::State::Maximized, filled);
            for edge in [
                xdg_toplevel::State::TiledLeft,
                xdg_toplevel::State::TiledRight,
                xdg_toplevel::State::TiledTop,
                xdg_toplevel::State::TiledBottom,
            ] {
                set(edge, tiled || filled);
            }
        });
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
    }

    /// Moves the focused window to the next display (`step` 1) or the previous (-1), in its
    /// zone there or at the same place relative to the work area.
    pub fn move_to_display(&mut self, step: i32) {
        let Some(id) = self.focused_window() else {
            return;
        };
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        let Some(from) = self.isle.output_of(id) else {
            return;
        };
        let Some(index) = outputs.iter().position(|o| *o == from) else {
            return;
        };
        if outputs.len() < 2 {
            return;
        }
        let to = outputs[(index as i32 + step).rem_euclid(outputs.len() as i32) as usize].clone();
        match self.isle.wm.get(id).map(|r| r.mode.clone()) {
            Some(Mode::Tiled { zone, .. }) => {
                self.tile_on(id, zone, &to);
            }
            Some(Mode::Filled { .. }) => {
                self.tile_on(id, Zone::Fill, &to);
            }
            Some(Mode::FullScreen { .. }) | None => {}
            _ => {
                let (Some(frame), a, b) = (
                    self.isle.frame(id),
                    self.isle.work_area(&from),
                    self.isle.work_area(&to),
                ) else {
                    return;
                };
                let moved: Rectangle<i32, Logical> = Rectangle::new(
                    (
                        b.loc.x + (frame.loc.x - a.loc.x) * b.size.w / a.size.w.max(1),
                        b.loc.y + (frame.loc.y - a.loc.y) * b.size.h / a.size.h.max(1),
                    )
                        .into(),
                    frame.size,
                );
                let fitted = super::geometry::clamp_whole(moved, b);
                self.set_frame(id, fitted);
                let bar = self.isle.bar_height(id);
                if let Some(record) = self.isle.wm.get_mut(id) {
                    record.record_at_size = Some((fitted.size.w, fitted.size.h - bar).into());
                }
            }
        }
    }
}

/// The zone a dragged window would tile to, drawn under it.
pub struct ZonePreview {
    pub output: Output,
    pub zone: Zone,
    /// In the global space.
    pub rect: Rect,
    /// The dragged window it is drawn under.
    pub under: u64,
    pub buffer: smithay::backend::renderer::element::solid::SolidColorBuffer,
}

impl Isle {
    /// Where a dragged frame lands: its edges snapped to the other windows' and the work
    /// area's, and its top kept out of the strip.
    pub fn snapped(
        &self,
        id: u64,
        wanted: Rect,
        pointer: Point<f64, Logical>,
    ) -> Point<i32, Logical> {
        let Some(output) = self
            .space
            .output_under(pointer)
            .next()
            .cloned()
            .or_else(|| self.output_of(id))
        else {
            return wanted.loc;
        };
        let area = self.work_area(&output);
        let others = self.other_frames(id, &output);
        let xs: Vec<i32> = others
            .iter()
            .flat_map(|f| [f.loc.x, f.loc.x + f.size.w])
            .chain([area.loc.x, area.loc.x + area.size.w])
            .collect();
        let ys: Vec<i32> = others
            .iter()
            .flat_map(|f| [f.loc.y, f.loc.y + f.size.h])
            .chain([area.loc.y, area.loc.y + area.size.h])
            .collect();
        let mut at = super::geometry::snap(wanted, &xs, &ys);
        at.y = at.y.max(area.loc.y);
        at
    }

    /// Shows, moves or clears the zone preview for a drag of `id` with the pointer at
    /// `pointer`.
    pub fn update_zone_preview(&mut self, id: u64, pointer: Point<f64, Logical>) {
        let target = self
            .space
            .output_under(pointer)
            .next()
            .cloned()
            .and_then(|output| {
                let geometry = self.space.output_geometry(&output)?;
                let leads_on = |dx: f64, dy: f64| {
                    let beyond: Point<f64, Logical> = (pointer.x + dx, pointer.y + dy).into();
                    self.space.output_under(beyond).next().is_some()
                };
                let reach = super::geometry::EDGE_REGION + 1.0;
                let open = [
                    !leads_on(-reach, 0.0),
                    !leads_on(reach, 0.0),
                    !leads_on(0.0, -reach),
                    !leads_on(0.0, reach),
                ];
                let zone = super::geometry::region_zone(geometry, pointer, open)?;
                let rect = self.zone_rect(id, zone, &output)?;
                Some((output, zone, rect))
            });
        self.set_zone_preview(id, target);
    }

    /// Shows `target`'s zone under window `id`, or clears the preview.
    pub fn set_zone_preview(&mut self, id: u64, target: Option<(Output, Zone, Rect)>) {
        let changed = match (&self.zone_preview, &target) {
            (None, None) => false,
            (Some(p), Some((o, z, r))) => p.output != *o || p.zone != *z || p.rect != *r,
            _ => true,
        };
        if !changed {
            return;
        }
        if let Some(old) = self.zone_preview.take() {
            self.queue_redraw(&old.output);
        }
        if let Some((output, zone, rect)) = target {
            let colour = self.shell.theme().tokens.colour_fill_strong;
            let a = colour.a;
            let premultiplied = [
                colour.r as f32 / 255.0 * a,
                colour.g as f32 / 255.0 * a,
                colour.b as f32 / 255.0 * a,
                a,
            ];
            self.queue_redraw(&output);
            self.zone_preview = Some(ZonePreview {
                output,
                zone,
                rect,
                under: id,
                buffer: smithay::backend::renderer::element::solid::SolidColorBuffer::new(
                    rect.size,
                    premultiplied,
                ),
            });
        }
    }
}
