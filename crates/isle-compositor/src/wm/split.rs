//! Split View: two windows sharing a full-screen desktop with a divider between them
//! (`compositor.md` Split View; P16 choices 22 to 25). Each pane's window is full screen
//! with its origin, so every full-screen rule holds for it; it is told it is tiled, not full
//! screen, and keeps its own chrome.

use std::time::{Duration, Instant};

use isle_shell::mission::model::{DIVIDER, Mode as MissionMode};
use smithay::{
    backend::renderer::element::{
        Kind as ElementKind,
        solid::{SolidColorBuffer, SolidColorRenderElement},
    },
    output::Output,
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Point, Scale},
};
use tracing::info;

use super::{Mode, desktops::Kind, geometry::Rect};
use crate::{
    render::IsleElement,
    state::{Isle, State},
};

/// Twice this apart, two presses on the divider are a double click.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// A divider being dragged: its output, and where the press began.
#[derive(Clone, Debug)]
pub struct DividerDrag {
    pub output: Output,
}

#[derive(Default)]
pub struct SplitState {
    pub drag: Option<DividerDrag>,
    last_press: Option<Instant>,
    /// The divider and grip drawn per output.
    buffers:
        std::cell::RefCell<std::collections::HashMap<String, (SolidColorBuffer, SolidColorBuffer)>>,
}

/// The two panes' frames on `screen` with the left one `fraction` of the width beside the
/// divider, each held to its window's smallest width.
pub fn panes(screen: Rect, fraction: f64, min: (i32, i32)) -> (Rect, Rect) {
    let room = screen.size.w - DIVIDER as i32;
    let left = ((room as f64 * fraction).round() as i32)
        .max(min.0)
        .min(room - min.1)
        .clamp(1, room - 1);
    let right = room - left;
    (
        Rect::new(screen.loc, (left, screen.size.h).into()),
        Rect::new(
            (screen.loc.x + left + DIVIDER as i32, screen.loc.y).into(),
            (right, screen.size.h).into(),
        ),
    )
}

impl Isle {
    /// The Split View `output` shows: its desktop, panes left then right (0 while one is
    /// being chosen) and the divider's place as a share of the width.
    pub fn split_shown(&self, output: &Output) -> Option<(u64, u64, u64, f64)> {
        let row = self.desktops.rows.get(&Self::output_identity(output))?;
        let desk = row.desktops.get(row.current)?;
        match desk.kind {
            Kind::Split {
                left,
                right,
                fraction,
            } => Some((desk.id, left, right, fraction)),
            _ => None,
        }
    }

    /// The divider's box in the global space, when `output` shows a pair.
    pub fn divider_rect(&self, output: &Output) -> Option<Rect> {
        let (_, left, right, fraction) = self.split_shown(output)?;
        if left == 0 || right == 0 {
            return None;
        }
        let screen = self.space.output_geometry(output)?;
        let (l, _) = panes(
            screen,
            fraction,
            (self.size_bounds(left).0.w, self.size_bounds(right).0.w),
        );
        Some(Rect::new(
            (l.loc.x + l.size.w, screen.loc.y).into(),
            (DIVIDER as i32, screen.size.h).into(),
        ))
    }

    /// The divider and its grip, front to back.
    pub fn divider_elements(&self, output: &Output, push: &mut dyn FnMut(IsleElement)) {
        let (Some(rect), Some(geometry)) = (
            self.divider_rect(output),
            self.space.output_geometry(output),
        ) else {
            return;
        };
        let t = &self.shell.theme().tokens;
        let mut buffers = self.split.buffers.borrow_mut();
        let (bar, grip) = buffers.entry(output.name()).or_default();
        let grip_h = t.space_xxxl as i32;
        bar.update(
            rect.size,
            crate::wallpaper::premultiplied(t.colour_window_sunken),
        );
        let dragging = self
            .split
            .drag
            .as_ref()
            .is_some_and(|d| &d.output == output);
        grip.update(
            (4, grip_h),
            crate::wallpaper::premultiplied(if dragging {
                t.colour_text_2
            } else {
                t.colour_hairline_strong
            }),
        );
        let scale = Scale::from(output.current_scale().fractional_scale());
        let at = rect.loc - geometry.loc;
        let grip_at = Point::<i32, Logical>::from((
            at.x + (rect.size.w - 4) / 2,
            at.y + (rect.size.h - grip_h) / 2,
        ));
        push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
            grip,
            grip_at.to_physical_precise_round(scale),
            scale,
            1.0,
            ElementKind::Unspecified,
        )));
        push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
            bar,
            at.to_physical_precise_round(scale),
            scale,
            1.0,
            ElementKind::Unspecified,
        )));
    }
}

impl State {
    /// The zoom menu, the Window menu, a chord or `Desktops1.Split`: `id` full screen on one
    /// side, the chooser in the other half.
    pub fn begin_split(&mut self, id: u64, left: bool) -> bool {
        let Some(record) = self.isle.wm.get(id) else {
            return false;
        };
        if record.parent.is_some()
            || !record.mapped
            || matches!(record.mode, Mode::FullScreen { .. })
        {
            return false;
        }
        let Some(output) = self.isle.output_of(id) else {
            return false;
        };
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return false;
        };
        let half = (screen.size.w - DIVIDER as i32) / 2;
        if self.isle.size_bounds(id).0.w > half {
            return false;
        }
        let origin = record.desktop;
        let candidates: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| {
                r.id != id
                    && r.parent.is_none()
                    && r.desktop == origin
                    && r.mapped
                    && !r.minimised
                    && !r.hidden
            })
            .map(|r| r.id)
            .collect();
        let (l, r) = if left { (id, 0) } else { (0, id) };
        if !self.make_split_desktop(id, &output, l, r) {
            return false;
        }
        info!(id, left, "split view begun");
        self.open_mission(MissionMode::Chooser {
            first: id,
            left,
            candidates,
        });
        true
    }

    /// A desktop right of `id`'s origin holding `left` and `right`, `id` moved onto it and
    /// shown.
    fn make_split_desktop(&mut self, id: u64, output: &Output, left: u64, right: u64) -> bool {
        let Some(frame) = self.isle.frame(id) else {
            return false;
        };
        let Some(record) = self.isle.wm.get(id) else {
            return false;
        };
        let identity = Isle::output_identity(output);
        let origin = record.desktop;
        let previous = Box::new(record.mode.clone());
        self.slide_out(output, 1, Some(id));
        let desktop = self.isle.desktops.new_desktop(Kind::Split {
            left,
            right,
            fraction: 0.5,
        });
        let desktop_id = desktop.id;
        let row = self.isle.desktops.ensure_row(&identity);
        let at = row
            .desktops
            .iter()
            .position(|d| d.id == origin)
            .map_or(row.desktops.len(), |i| i + 1);
        row.desktops.insert(at, desktop);
        row.current = at;
        let record = self.isle.wm.get_mut(id).unwrap();
        record.mode = Mode::FullScreen {
            origin,
            restore: frame,
            previous,
            borderless: false,
        };
        record.desktop = desktop_id;
        for child in self.dialogs_of(id) {
            self.isle.wm.get_mut(child).unwrap().desktop = desktop_id;
        }
        self.isle.sync_space();
        self.configure_panes(output);
        self.after_switch(output);
        true
    }

    /// The window chosen for the free half joins the first.
    pub fn choose_split(&mut self, first: u64, other: u64) -> bool {
        let Some(output) = self.isle.output_of(first) else {
            return false;
        };
        let Some((desk, left, right, _)) = self.isle.split_shown(&output) else {
            return false;
        };
        let (Some(frame), Some(record)) = (self.isle.frame(other), self.isle.wm.get(other)) else {
            return false;
        };
        let origin = record.desktop;
        let previous = Box::new(record.mode.clone());
        let record = self.isle.wm.get_mut(other).unwrap();
        record.mode = Mode::FullScreen {
            origin,
            restore: frame,
            previous,
            borderless: false,
        };
        record.desktop = desk;
        for child in self.dialogs_of(other) {
            self.isle.wm.get_mut(child).unwrap().desktop = desk;
        }
        let (l, r) = if left == 0 {
            (other, right)
        } else {
            (left, other)
        };
        self.set_panes(&output, l, r, None);
        self.isle.sync_space();
        self.configure_panes(&output);
        let window = self.isle.window_by_id(other);
        self.set_focus(window.as_ref());
        info!(first, other, "split view paired");
        true
    }

    /// Mission Control's drop of `window` on full-screen desktop `index` of `output`: they
    /// share it, `window` on the given side.
    pub fn split_with(&mut self, window: u64, output: &Output, index: usize, left: bool) -> bool {
        let identity = Isle::output_identity(output);
        let Some(desk) = self
            .isle
            .desktops
            .rows
            .get(&identity)
            .and_then(|r| r.desktops.get(index))
            .cloned()
        else {
            return false;
        };
        let Kind::FullScreen { window: other } = desk.kind else {
            return false;
        };
        let (Some(frame), Some(record)) = (self.isle.frame(window), self.isle.wm.get(window))
        else {
            return false;
        };
        if record.parent.is_some() || matches!(record.mode, Mode::FullScreen { .. }) {
            return false;
        }
        let origin = match self.isle.wm.get(other).map(|r| &r.mode) {
            Some(Mode::FullScreen { origin, .. }) => *origin,
            _ => record.desktop,
        };
        let previous = Box::new(record.mode.clone());
        let record = self.isle.wm.get_mut(window).unwrap();
        record.mode = Mode::FullScreen {
            origin,
            restore: frame,
            previous,
            borderless: false,
        };
        record.desktop = desk.id;
        let (l, r) = if left {
            (window, other)
        } else {
            (other, window)
        };
        if let Some(row) = self.isle.desktops.rows.get_mut(&identity) {
            row.desktops[index].kind = Kind::Split {
                left: l,
                right: r,
                fraction: 0.5,
            };
        }
        self.isle.sync_space();
        self.configure_panes_of(output, desk.id);
        self.isle.notify(crate::worker::Event::DesktopsChanged);
        info!(window, other, "split view by a drop");
        true
    }

    /// Sets the panes of `output`'s split desktop, and the divider when given.
    fn set_panes(&mut self, output: &Output, left: u64, right: u64, fraction: Option<f64>) {
        let identity = Isle::output_identity(output);
        let Some(row) = self.isle.desktops.rows.get_mut(&identity) else {
            return;
        };
        let current = row.current;
        if let Some(Kind::Split {
            left: l,
            right: r,
            fraction: f,
        }) = row.desktops.get_mut(current).map(|d| &mut d.kind)
        {
            *l = left;
            *r = right;
            if let Some(fraction) = fraction {
                *f = fraction;
            }
        }
    }

    /// Each pane of the split desktop `output` shows, sized and told it is tiled.
    pub fn configure_panes(&mut self, output: &Output) {
        let Some((desk, ..)) = self.isle.split_shown(output) else {
            return;
        };
        self.configure_panes_of(output, desk);
    }

    fn configure_panes_of(&mut self, output: &Output, desk: u64) {
        let identity = Isle::output_identity(output);
        let Some(Kind::Split {
            left,
            right,
            fraction,
        }) = self
            .isle
            .desktops
            .rows
            .get(&identity)
            .and_then(|r| r.desktops.iter().find(|d| d.id == desk))
            .map(|d| d.kind.clone())
        else {
            return;
        };
        let Some(screen) = self.isle.space.output_geometry(output) else {
            return;
        };
        let min = |id: u64| {
            if id == 0 {
                0
            } else {
                self.isle.size_bounds(id).0.w
            }
        };
        let (l, r) = if left == 0 || right == 0 {
            // While one is chosen the first keeps its half.
            let half = (screen.size.w - DIVIDER as i32) / 2;
            panes(
                screen,
                half as f64 / (screen.size.w - DIVIDER as i32) as f64,
                (0, 0),
            )
        } else {
            panes(screen, fraction, (min(left), min(right)))
        };
        for (id, rect) in [(left, l), (right, r)] {
            if id == 0 {
                continue;
            }
            self.begin_morph(id, rect);
            self.send_pane_states(id, true);
            if let Some(record) = self.isle.wm.get(id) {
                record.configure_size(rect.size);
            }
            self.isle.move_window(id, rect.loc);
        }
    }

    /// A pane is tiled on every edge and not full screen; out of a pane, neither.
    pub(crate) fn send_pane_states(&self, id: u64, pane: bool) {
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        if let Some(x11) = record.x11() {
            let _ = x11.set_fullscreen(false);
            return;
        }
        let Some(toplevel) = record.window.toplevel() else {
            return;
        };
        toplevel.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Fullscreen);
            for edge in [
                xdg_toplevel::State::TiledLeft,
                xdg_toplevel::State::TiledRight,
                xdg_toplevel::State::TiledTop,
                xdg_toplevel::State::TiledBottom,
            ] {
                if pane {
                    state.states.set(edge);
                } else {
                    state.states.unset(edge);
                }
            }
        });
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
    }

    /// The chooser closed with nothing chosen: the first window back to its box and the view
    /// to its origin.
    pub fn cancel_split(&mut self, first: u64) {
        info!(first, "split view cancelled");
        self.leave_full_screen(first);
        self.send_pane_states(first, false);
        self.send_mode_states(first);
    }

    /// One pane left full screen or closed: the other keeps the desktop, full screen alone.
    /// Returns false when `desk` is no Split View.
    pub fn split_lost(&mut self, desk: u64, gone: u64) -> bool {
        let Some((identity, index)) = self
            .isle
            .desktops
            .locate(desk)
            .map(|(i, n)| (i.to_owned(), n))
        else {
            return false;
        };
        let Some(Kind::Split { left, right, .. }) = self.isle.desktops.rows[&identity]
            .desktops
            .get(index)
            .map(|d| d.kind.clone())
        else {
            return false;
        };
        let other = if left == gone { right } else { left };
        let row = self.isle.desktops.rows.get_mut(&identity).unwrap();
        if other == 0 {
            return false;
        }
        row.desktops[index].kind = Kind::FullScreen { window: other };
        let Some(output) = self.isle.output_of_desktop(desk) else {
            return true;
        };
        if let Some(screen) = self.isle.space.output_geometry(&output) {
            self.send_pane_states(other, false);
            self.begin_morph(other, screen);
            if let Some(record) = self.isle.wm.get(other) {
                record.configure_size(screen.size);
            }
            self.isle.move_window(other, screen.loc);
            self.send_full_screen(other, true);
        }
        self.isle.notify(crate::worker::Event::DesktopsChanged);
        true
    }

    /// The panes change sides.
    pub fn swap_split(&mut self, output: &Output) {
        let Some((_, left, right, fraction)) = self.isle.split_shown(output) else {
            return;
        };
        self.set_panes(output, right, left, Some(1.0 - fraction));
        self.configure_panes(output);
    }

    /// `id`'s pane is chosen again: it goes back to its origin and the chooser offers its
    /// half.
    pub fn replace_pane(&mut self, id: u64) {
        let Some(output) = self.isle.output_of(id) else {
            return;
        };
        let Some((desk, left, right, _)) = self.isle.split_shown(&output) else {
            return;
        };
        let stays = if left == id { right } else { left };
        let origin = match self.isle.wm.get(stays).map(|r| &r.mode) {
            Some(Mode::FullScreen { origin, .. }) => *origin,
            _ => return,
        };
        // The replaced window goes home; the one staying keeps its side.
        let (
            Some(Mode::FullScreen {
                restore, previous, ..
            }),
            Some(_),
        ) = (
            self.isle.wm.get(id).map(|r| r.mode.clone()),
            self.isle.wm.get(stays),
        )
        else {
            return;
        };
        let record = self.isle.wm.get_mut(id).unwrap();
        record.mode = *previous;
        record.desktop = origin;
        self.send_pane_states(id, false);
        self.send_mode_states(id);
        self.set_frame(id, restore);
        let (l, r) = if left == id { (0, stays) } else { (stays, 0) };
        self.set_panes(&output, l, r, Some(0.5));
        self.isle.sync_space();
        let candidates: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| {
                r.parent.is_none() && r.desktop == origin && r.mapped && !r.minimised && !r.hidden
            })
            .map(|r| r.id)
            .collect();
        let _ = desk;
        self.open_mission(MissionMode::Chooser {
            first: stays,
            left: l == stays,
            candidates,
        });
    }

    /// A press on the divider starts its drag; a second soon after puts it back at half.
    pub fn divider_press(&mut self, at: Point<f64, Logical>) -> bool {
        let Some(output) = self
            .isle
            .space
            .outputs()
            .find(|o| {
                self.isle
                    .space
                    .output_geometry(o)
                    .is_some_and(|g| g.to_f64().contains(at))
            })
            .cloned()
        else {
            return false;
        };
        let Some(rect) = self.isle.divider_rect(&output) else {
            return false;
        };
        let hit = self.isle.shell.theme().tokens.size_hit_minimum as f64;
        let centre = rect.loc.x as f64 + rect.size.w as f64 / 2.0;
        if (at.x - centre).abs() > hit / 2.0 {
            return false;
        }
        let now = Instant::now();
        if self
            .isle
            .split
            .last_press
            .is_some_and(|t| now.duration_since(t) < DOUBLE_CLICK)
        {
            self.isle.split.last_press = None;
            if let Some((_, l, r, _)) = self.isle.split_shown(&output) {
                self.set_panes(&output, l, r, Some(0.5));
                self.configure_panes(&output);
            }
            return true;
        }
        self.isle.split.last_press = Some(now);
        self.isle.split.drag = Some(DividerDrag { output });
        true
    }

    /// The divider follows the pointer while dragged.
    pub fn divider_drag_to(&mut self, at: Point<f64, Logical>) -> bool {
        let Some(drag) = self.isle.split.drag.clone() else {
            return false;
        };
        let Some(screen) = self.isle.space.output_geometry(&drag.output) else {
            return true;
        };
        let room = (screen.size.w - DIVIDER as i32) as f64;
        let fraction =
            ((at.x - screen.loc.x as f64 - DIVIDER as f64 / 2.0) / room).clamp(0.05, 0.95);
        if let Some((_, l, r, _)) = self.isle.split_shown(&drag.output) {
            self.set_panes(&drag.output, l, r, Some(fraction));
            self.configure_panes(&drag.output);
        }
        let pointer = self.isle.seat.get_pointer().unwrap();
        pointer.motion(
            self,
            None,
            &smithay::input::pointer::MotionEvent {
                location: at,
                serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                time: smithay::backend::input::InputTime::now(),
            },
        );
        pointer.frame(self);
        self.isle.queue_redraw(&drag.output);
        true
    }

    pub fn divider_release(&mut self) -> bool {
        self.isle.split.drag.take().is_some()
    }

    /// The window menu's Resize: the divider by keys, a step at a time.
    pub fn divider_step(&mut self, output: &Output, step: f64) {
        if let Some((_, l, r, fraction)) = self.isle.split_shown(output) {
            self.set_panes(output, l, r, Some((fraction + step).clamp(0.05, 0.95)));
            self.configure_panes(output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panes_split_the_width_beside_the_divider_and_keep_their_minimums() {
        let screen = Rect::new((0, 0).into(), (3440, 1440).into());
        let (l, r) = panes(screen, 0.5, (0, 0));
        assert_eq!(l.size.w + r.size.w + 8, 3440);
        assert_eq!(r.loc.x, l.size.w + 8);
        let (l, _) = panes(screen, 0.1, (900, 0));
        assert_eq!(l.size.w, 900);
        let (_, r) = panes(screen, 0.9, (0, 900));
        assert_eq!(r.size.w, 900);
    }
}
