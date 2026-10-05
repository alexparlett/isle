//! The top edge of each screen, decided in one place: the island's zone and its hover
//! guards, the push that brings the island back over a full-screen window, and later the
//! menu bar's reveal outside the zone (`p6-island-and-menus.md` choice 7).

use std::time::Duration;

use smithay::{
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Logical, Point, Rectangle},
};

use crate::state::{Isle, State};

#[derive(Default)]
pub struct TopEdge {
    /// Upward travel into the top edge since the pointer reached it.
    push: f64,
    dwell: Option<RegistrationToken>,
    grace: Option<RegistrationToken>,
    /// A click opened something elsewhere: no peek until the pointer leaves the zone.
    disarmed: bool,
    /// The island came back over full screen: a peek waits for the pointer to move.
    need_move: bool,
    /// The pointer is in the island's zone.
    inside: bool,
}

impl Isle {
    /// The island's zone on `output`, relative to it: its shape, the strip above it to the
    /// edge, and a guard each side; with Control Centre open, the panel.
    pub fn island_zone(&self, output: &Output) -> Option<Rectangle<f64, Logical>> {
        let screen = self.space.output_geometry(output)?;
        let t = &self.shell.theme().tokens;
        let (w, h) = self.island_size(output).unwrap_or((0.0, 0.0));
        let open = self
            .island
            .panel
            .as_ref()
            .is_some_and(|p| p.output == output.name());
        let guard = if open { 0.0 } else { t.size_island_guard };
        let w = w.max(t.size_island_height * 2.0) + 2.0 * guard;
        let h = t.size_island_top + h.max(t.size_island_height);
        let x = (screen.size.w as f32 - w) / 2.0;
        Some(Rectangle::new(
            (x as f64, 0.0).into(),
            (w as f64, h as f64).into(),
        ))
    }

    /// Where the pointer is, relative to the output it is on.
    fn pointer_on(&self) -> Option<(Output, Point<f64, Logical>)> {
        let output = self.active_output()?;
        let screen = self.space.output_geometry(&output)?;
        let at = self.seat.get_pointer()?.current_location();
        Some((output, at - screen.loc.to_f64()))
    }
}

impl State {
    /// The pointer moved by `delta` to where it is now: the island's zone decides.
    pub fn top_edge_motion(&mut self, delta: Point<f64, Logical>) {
        let Some((output, at)) = self.isle.pointer_on() else {
            return;
        };
        let dragging = self.isle.seat.get_pointer().is_some_and(|p| p.is_grabbed())
            || !self.isle.buttons_down.is_empty();
        let zone = self.isle.island_zone(&output);
        let inside = !dragging && zone.is_some_and(|z| z.contains(at));
        let name = output.name();
        let t = self.isle.shell.theme().tokens.clone();

        if !inside {
            self.isle.top_edge.push = 0.0;
            self.isle.top_edge.need_move = false;
            if let Some(token) = self.isle.top_edge.dwell.take() {
                self.isle.loop_handle.remove(token);
            }
            if self.isle.top_edge.inside {
                self.isle.top_edge.inside = false;
                self.isle.top_edge.disarmed = false;
                let peeked = self
                    .isle
                    .island
                    .panel
                    .as_ref()
                    .is_some_and(|p| !p.pinned && p.output == name);
                let revealed = self.isle.island.revealed.contains(&name);
                if peeked || revealed {
                    self.arm_grace(output, t.timing_hover_grace_ms);
                }
            }
            return;
        }
        if !self.isle.top_edge.inside {
            self.isle.top_edge.inside = true;
        } else if self.isle.top_edge.need_move && (delta.x != 0.0 || delta.y != 0.0) {
            self.isle.top_edge.need_move = false;
        }
        if let Some(token) = self.isle.top_edge.grace.take() {
            self.isle.loop_handle.remove(token);
        }

        // Over a full-screen window the island is away: a push at the edge brings it back,
        // a dwell over a game.
        let covered =
            self.isle.wallpaper_covered(&output) && !self.isle.island.revealed.contains(&name);
        if covered {
            if at.y <= 0.5 && delta.y < 0.0 {
                self.isle.top_edge.push += -delta.y;
            }
            let game = self.focus_is_game();
            if game {
                if self.isle.top_edge.dwell.is_none() && at.y <= 0.5 {
                    self.arm_dwell(output, t.timing_edge_dwell_ms, true);
                }
            } else if self.isle.top_edge.push >= t.size_edge_push as f64 {
                self.reveal_island(&output);
            }
            return;
        }

        let open = self
            .isle
            .island
            .panel
            .as_ref()
            .is_some_and(|p| p.output == name);
        if open || self.isle.top_edge.disarmed || self.isle.top_edge.need_move {
            return;
        }
        if self.isle.top_edge.dwell.is_none() {
            self.arm_dwell(output, t.timing_hover_dwell_ms, false);
        }
    }

    fn reveal_island(&mut self, output: &Output) {
        let name = output.name();
        if !self.isle.island.revealed.contains(&name) {
            self.isle.island.revealed.push(name);
            self.isle.top_edge.push = 0.0;
            self.isle.top_edge.need_move = true;
            self.isle.refresh_island();
        }
    }

    /// After `ms` in the zone: a peek, or over a game the island back.
    fn arm_dwell(&mut self, output: Output, ms: u32, reveal: bool) {
        let timer = Timer::from_duration(Duration::from_millis(ms.into()));
        let token = self
            .isle
            .loop_handle
            .insert_source(timer, move |_, _, state| {
                state.isle.top_edge.dwell = None;
                let still = state.isle.pointer_on().is_some_and(|(o, at)| {
                    o == output && state.isle.island_zone(&o).is_some_and(|z| z.contains(at))
                });
                if still {
                    if reveal {
                        state.reveal_island(&output);
                    } else {
                        state.open_control_centre(&output, false, None);
                    }
                }
                TimeoutAction::Drop
            });
        self.isle.top_edge.dwell = token.ok();
    }

    /// After `ms` out of the zone: a peek closes, the island goes back over full screen.
    fn arm_grace(&mut self, output: Output, ms: u32) {
        if let Some(token) = self.isle.top_edge.grace.take() {
            self.isle.loop_handle.remove(token);
        }
        let timer = Timer::from_duration(Duration::from_millis(ms.into()));
        let token = self
            .isle
            .loop_handle
            .insert_source(timer, move |_, _, state| {
                state.isle.top_edge.grace = None;
                if state.isle.top_edge.inside {
                    return TimeoutAction::Drop;
                }
                let name = output.name();
                if state
                    .isle
                    .island
                    .panel
                    .as_ref()
                    .is_some_and(|p| !p.pinned && p.output == name)
                {
                    state.close_control_centre();
                }
                if state.isle.island.panel.is_none() {
                    state.isle.island.revealed.retain(|o| o != &name);
                    state.isle.refresh_island();
                }
                TimeoutAction::Drop
            });
        self.isle.top_edge.grace = token.ok();
    }

    /// A press anywhere: one outside a pinned Control Centre closes it.
    pub fn top_edge_press(&mut self) {
        let Some(panel) = self.isle.island.panel.clone() else {
            return;
        };
        let on_island = matches!(
            self.isle.seat.get_pointer().and_then(|p| p.current_focus()),
            Some(crate::focus::Focus::Shell(f)) if self.isle.shell.name_of(&f) == Some("island")
        );
        if !on_island {
            self.close_control_centre();
            if !panel.pinned {
                self.isle.top_edge.disarmed = true;
            }
        }
    }

    /// Escape while a peek is open and the pointer rests in the zone closes the peek and is
    /// not passed to the window.
    pub fn top_edge_escape(&mut self) -> bool {
        let peeked = self.isle.island.panel.as_ref().is_some_and(|p| !p.pinned);
        if peeked && self.isle.top_edge.inside {
            self.close_control_centre();
            self.isle.top_edge.disarmed = true;
            return true;
        }
        false
    }
}
