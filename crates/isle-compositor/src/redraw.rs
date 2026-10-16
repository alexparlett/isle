//! Per-output redraw scheduling: a frame only when something changed, frame callbacks once
//! per refresh at most (niri's state machine, choice 6 in `p2-compositor.md`).

use std::{cell::RefCell, time::Duration};

use smithay::{
    backend::renderer::element::{RenderElementStates, default_primary_scanout_output_compare},
    desktop::{
        layer_map_for_output,
        utils::{
            OutputPresentationFeedback, surface_presentation_feedback_flags_from_states,
            surface_primary_scanout_output, update_surface_primary_scanout_output,
            with_surfaces_surface_tree,
        },
    },
    input::pointer::CursorImageStatus,
    output::Output,
    reexports::{
        calloop::{
            RegistrationToken,
            timer::{TimeoutAction, Timer},
        },
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
    },
    wayland::{fractional_scale::with_fractional_scale, presentation::Refresh},
};
use tracing::{info, trace, warn};

use crate::{
    present::{Planes, Presented, Trace},
    state::{Isle, State},
};

#[derive(Debug, Default)]
pub enum RedrawState {
    /// Nothing to draw.
    #[default]
    Idle,
    /// A redraw runs at the end of this loop iteration.
    Queued,
    /// A frame was submitted; the next one waits for its vblank.
    WaitingForVBlank {
        redraw_needed: bool,
    },
    /// Nothing was damaged; frame callbacks wait for the estimated vblank.
    WaitingForEstimatedVBlank(RegistrationToken),
    WaitingForEstimatedVBlankAndQueued(RegistrationToken),
}

#[derive(Default)]
pub struct OutputState {
    pub redraw: RedrawState,
    pub frames_presented: u64,
    pending_feedback: Option<OutputPresentationFeedback>,
    /// The frame waiting for its vblank was drawn by the lock's path.
    locked_pending: bool,
    /// The frame on screen was drawn by the lock's path.
    pub locked_shown: bool,
    /// The planes of the frame waiting for its vblank.
    pending_planes: Option<Planes>,
    /// The output's refresh follows its frames.
    pub vrr: bool,
    /// When its last frame reached the screen.
    pub last_shown: Option<Duration>,
    /// Only the pointer moved since the last frame, so what lies behind glass did not.
    pub pointer_only: bool,
    /// The timer sending a held pointer-only frame.
    retry: Option<smithay::reexports::calloop::RegistrationToken>,
    /// A scanned-out frame asked to tear and was flipped with vsync; said once.
    async_refused: bool,
    pub trace: Trace,
}

impl OutputState {
    pub fn cell() -> RefCell<OutputState> {
        RefCell::new(OutputState::default())
    }

    pub fn of(output: &Output) -> &RefCell<OutputState> {
        output
            .user_data()
            .get::<RefCell<OutputState>>()
            .expect("output state is added with the output")
    }
}

pub fn refresh_interval(output: &Output) -> Duration {
    output
        .current_mode()
        .filter(|m| m.refresh > 0)
        .map(|m| Duration::from_secs_f64(1000.0 / m.refresh as f64))
        .unwrap_or(Duration::from_micros(16_667))
}

impl Isle {
    pub fn queue_redraw(&mut self, output: &Output) {
        let mut state = OutputState::of(output).borrow_mut();
        state.pointer_only = false;
        state.redraw = match std::mem::take(&mut state.redraw) {
            RedrawState::Idle | RedrawState::Queued => RedrawState::Queued,
            RedrawState::WaitingForVBlank { .. } => RedrawState::WaitingForVBlank {
                redraw_needed: true,
            },
            RedrawState::WaitingForEstimatedVBlank(token)
            | RedrawState::WaitingForEstimatedVBlankAndQueued(token) => {
                RedrawState::WaitingForEstimatedVBlankAndQueued(token)
            }
        };
    }

    /// A redraw of every output for the pointer alone: one not already asked for by
    /// anything else may leave the glass as it is.
    pub fn queue_pointer_redraw_all(&mut self) {
        let outputs: Vec<_> = self.space.outputs().cloned().collect();
        for output in &outputs {
            let fresh = matches!(
                OutputState::of(output).borrow().redraw,
                RedrawState::Idle
                    | RedrawState::WaitingForVBlank { redraw_needed: false }
                    | RedrawState::WaitingForEstimatedVBlank(_)
            );
            self.queue_redraw(output);
            if fresh {
                OutputState::of(output).borrow_mut().pointer_only = true;
            }
        }
    }

    /// Sends frame callbacks to every surface shown on `output`.
    pub fn send_frame_callbacks(&self, output: &Output) {
        let time = self.clock.now();
        let throttle = Some(Duration::from_secs(1));
        for window in self.space.elements() {
            if self.space.outputs_for_element(window).contains(output) {
                window.send_frame(output, time, throttle, surface_primary_scanout_output);
            }
        }
        for window in &self.unmanaged {
            window.send_frame(output, time, throttle, |_, _| Some(output.clone()));
        }
        for layer in layer_map_for_output(output).layers() {
            layer.send_frame(output, time, throttle, surface_primary_scanout_output);
        }
        if let CursorImageStatus::Surface(surface) = &self.cursor_status {
            smithay::desktop::utils::send_frames_surface_tree(
                surface,
                output,
                time,
                throttle,
                surface_primary_scanout_output,
            );
        }
    }

    /// Also tells each surface the fractional scale of the output it is mostly on.
    pub fn update_primary_scanout_output(&self, output: &Output, states: &RenderElementStates) {
        for window in self.space.elements().chain(self.unmanaged.iter()) {
            window.with_surfaces(|surface, surface_states| {
                let primary = update_surface_primary_scanout_output(
                    surface,
                    output,
                    surface_states,
                    None,
                    states,
                    default_primary_scanout_output_compare,
                );
                if let Some(primary) = primary {
                    with_fractional_scale(surface_states, |fractional| {
                        fractional.set_preferred_scale(primary.current_scale().fractional_scale());
                    });
                }
            });
        }
        for layer in layer_map_for_output(output).layers() {
            layer.with_surfaces(|surface, surface_states| {
                update_surface_primary_scanout_output(
                    surface,
                    output,
                    surface_states,
                    None,
                    states,
                    default_primary_scanout_output_compare,
                );
            });
        }
        if let CursorImageStatus::Surface(surface) = &self.cursor_status {
            with_surfaces_surface_tree(surface, |surface, surface_states| {
                update_surface_primary_scanout_output(
                    surface,
                    output,
                    surface_states,
                    None,
                    states,
                    default_primary_scanout_output_compare,
                );
            });
        }
    }

    pub fn take_presentation_feedback(
        &self,
        output: &Output,
        states: &RenderElementStates,
    ) -> OutputPresentationFeedback {
        let mut feedback = OutputPresentationFeedback::new(output);
        let flags =
            |surface: &_, _: &_| surface_presentation_feedback_flags_from_states(surface, None, states);
        for window in self.space.elements() {
            if self.space.outputs_for_element(window).contains(output) {
                window.take_presentation_feedback(
                    &mut feedback,
                    surface_primary_scanout_output,
                    flags,
                );
            }
        }
        let screen = self.space.output_geometry(output);
        for window in &self.unmanaged {
            let geometry = window.x11_surface().map(|x| x.geometry());
            if let (Some(window_box), Some(screen)) = (geometry, screen)
                && window_box.overlaps(screen)
            {
                window.take_presentation_feedback(
                    &mut feedback,
                    |_, _| Some(output.clone()),
                    flags,
                );
            }
        }
        for layer in layer_map_for_output(output).layers() {
            layer.take_presentation_feedback(
                &mut feedback,
                surface_primary_scanout_output,
                flags,
            );
        }
        feedback
    }
}

impl State {
    /// Runs at the end of every loop iteration: draws each output with a queued redraw.
    pub fn redraw_queued(&mut self) {
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        for output in outputs {
            if matches!(
                OutputState::of(&output).borrow().redraw,
                RedrawState::Queued
            ) {
                self.redraw(&output);
            }
        }
    }

    fn redraw(&mut self, output: &Output) {
        let _beat = self.isle.heartbeat.beat("redraw");
        // Away from the console, asleep or with the screens off, nothing is drawn; coming
        // back redraws every output.
        if !self.backend.can_draw() || self.isle.asleep || self.isle.idle.off {
            OutputState::of(output).borrow_mut().redraw = RedrawState::Idle;
            return;
        }
        let target = crate::sync::frame_target(
            OutputState::of(output).borrow().last_shown,
            self.isle.clock.now().into(),
            refresh_interval(output),
        );
        let started = std::time::Instant::now();
        self.pass_commit_timers(target);
        self.isle.step_windows(std::time::Instant::now());
        if let Some(renderer) = self.backend.renderer_opt() {
            self.isle.prepare_wallpaper(renderer, output);
        }
        self.watch_island();
        self.isle.step_island(std::time::Instant::now());
        self.step_bar();
        let stepped = started.elapsed();
        self.dispatch_shell();
        let dispatched = started.elapsed();
        let pointer_only = std::mem::take(&mut OutputState::of(output).borrow_mut().pointer_only);
        let glass = self.draw_shell(output, pointer_only);
        let shell_drawn = started.elapsed();
        let rendered = self.backend.render(&mut self.isle, output);
        let total = started.elapsed();
        let drawing = shell_drawn - dispatched;
        self.isle.frame_phases.note(
            output,
            total,
            [stepped, dispatched - stepped, drawing.saturating_sub(glass), glass, total - shell_drawn],
        );
        let rendered = match rendered {
            Ok(rendered) => rendered,
            Err(err) => {
                warn!(output = output.name(), "rendering failed: {err}");
                OutputState::of(output).borrow_mut().redraw = RedrawState::Idle;
                return;
            }
        };
        self.isle
            .update_primary_scanout_output(output, &rendered.states);
        self.backend
            .send_feedback(&self.isle, output, &rendered.states);
        self.cast_frames(output);
        self.copy_frames(output);

        if rendered.damaged {
            let feedback = self
                .isle
                .take_presentation_feedback(output, &rendered.states);
            let mut state = OutputState::of(output).borrow_mut();
            state.pending_feedback = Some(feedback);
            state.pending_planes = Some(rendered.planes);
            state.frames_presented += 1;
            state.locked_pending = self.isle.lock.locked;
            state.redraw = RedrawState::WaitingForVBlank {
                redraw_needed: false,
            };
            trace!(
                output = output.name(),
                frame = state.frames_presented,
                "presented"
            );
            drop(state);
            // A shell surface, a window or a wallpaper still moving needs the next frame too.
            if self.isle.shell.animating(output)
                || self.isle.dimming()
                || self.isle.scrim_fading()
                || self.isle.windows_moving(output)
                || self.isle.wallpapers.fading(&Isle::output_identity(output))
                || self.isle.island_moving(output)
                || self.bar_moving()
            {
                self.isle.queue_redraw(output);
            }
            if self.backend.paces_by_timer() {
                let target = output.clone();
                let timer = Timer::from_duration(refresh_interval(output));
                self.isle
                    .loop_handle
                    .insert_source(timer, move |_, _, state| {
                        let _beat = state.isle.heartbeat.beat("headless-vblank");
                        state.backend.frame_shown(&target);
                        state.on_vblank(&target, None);
                        TimeoutAction::Drop
                    })
                    .expect("the headless vblank timer");
            }
            // What set a fifo barrier is in this frame now, so the next commit may come;
            // after the state above, so the redraw that commit asks for is kept.
            self.pass_fifo_barriers(output);
            return;
        }

        // Nothing changed on screen; clients that asked for a frame get one at the
        // estimated vblank rather than at once, or they would spin.
        self.isle.send_frame_callbacks(output);
        let wait = {
            let state = OutputState::of(output).borrow();
            let shown = state.last_shown.filter(|_| !state.vrr);
            crate::present::next_vblank_in(refresh_interval(output), shown, self.isle.clock.now().into())
        };
        let timer = Timer::from_duration(wait);
        let target = output.clone();
        let token = self
            .isle
            .loop_handle
            .insert_source(timer, move |_, _, state| {
                let _beat = state.isle.heartbeat.beat("estimated-vblank");
                let mut output_state = OutputState::of(&target).borrow_mut();
                output_state.redraw = match std::mem::take(&mut output_state.redraw) {
                    RedrawState::WaitingForEstimatedVBlankAndQueued(_) => RedrawState::Queued,
                    _ => RedrawState::Idle,
                };
                TimeoutAction::Drop
            })
            .expect("the estimated vblank timer");
        OutputState::of(output).borrow_mut().redraw = RedrawState::WaitingForEstimatedVBlank(token);
        // A pointer-only frame held under VRR goes on its own once the hold is over.
        if let Some(left) = rendered.retry_in {
            let target = output.clone();
            let token = self.isle.loop_handle.insert_source(Timer::from_duration(left), move |_, _, state| {
                OutputState::of(&target).borrow_mut().retry = None;
                state.isle.queue_redraw(&target);
                TimeoutAction::Drop
            });
            let old = std::mem::replace(&mut OutputState::of(output).borrow_mut().retry, token.ok());
            if let Some(old) = old {
                self.isle.loop_handle.remove(old);
            }
        }
        self.pass_fifo_barriers(output);
        // Something still moving that drew nothing visible this frame goes on at the next.
        if self.isle.shell.animating(output)
            || self.isle.dimming()
                || self.isle.scrim_fading()
            || self.isle.windows_moving(output)
            || self.isle.wallpapers.fading(&Isle::output_identity(output))
            || self.isle.island_moving(output)
            || self.bar_moving()
        {
            self.isle.queue_redraw(output);
        }
    }

    /// Draws the shell surfaces on `output` whose trees changed, before its frame.
    /// Compiles the window and shell programs before anything is drawn with them.
    pub fn warm_up(&mut self) {
        let scale = self
            .isle
            .space
            .outputs()
            .next()
            .map_or(1.0, |o| o.current_scale().fractional_scale());
        let Some(renderer) = self.backend.renderer_opt() else {
            return;
        };
        if self.isle.window_programs.is_none() {
            match crate::wm::draw::Programs::compile(renderer) {
                Ok(programs) => self.isle.window_programs = Some(programs),
                Err(err) => warn!("no window corners or shadows: {err}"),
            }
        }
        let took = self.isle.shell.warm_up(renderer, scale);
        info!(ms = took.as_millis() as u64, "shell programs compiled");
    }

    /// Draws the shell surfaces that changed, then what lies behind their glass, unless
    /// only the pointer moved and no surface drew. Returns how long the glass took.
    pub fn draw_shell(&mut self, output: &Output, pointer_only: bool) -> Duration {
        let Some(renderer) = self.backend.renderer_opt() else {
            return Duration::ZERO;
        };
        if self.isle.window_programs.is_none() {
            match crate::wm::draw::Programs::compile(renderer) {
                Ok(programs) => self.isle.window_programs = Some(programs),
                Err(err) => warn!("no window corners or shadows: {err}"),
            }
        }
        let drew = self.isle.shell.draw(renderer, output);
        if pointer_only && !drew {
            return Duration::ZERO;
        }
        let started = std::time::Instant::now();
        let regions = self.isle.shell.glass_regions(output);
        self.isle.shell.begin_glass(output);
        let size = output
            .current_mode()
            .map(|m| m.size)
            .unwrap_or_default();
        for region in regions {
            let mut below = Vec::new();
            self.isle
                .render_elements_below(renderer, output, region.layer, &mut |e| below.push(e));
            self.isle.shell.prepare_glass(renderer, &region, size, below);
        }
        self.isle.shell.end_glass(output);
        started.elapsed()
    }

    /// The backend's vblank for `output`: the submitted frame is on screen, at the
    /// display's own time when the backend knows it.
    pub fn on_vblank(&mut self, output: &Output, presented: Option<Presented>) {
        let presented = presented.unwrap_or_else(|| Presented {
            time: self.isle.clock.now().into(),
            sequence: 0,
            hardware: false,
            asynchronous: false,
        });
        let (feedback, vrr) = {
            let mut state = OutputState::of(output).borrow_mut();
            match std::mem::take(&mut state.redraw) {
                RedrawState::WaitingForVBlank { redraw_needed } => {
                    state.redraw = if redraw_needed {
                        RedrawState::Queued
                    } else {
                        RedrawState::Idle
                    };
                    state.locked_shown = state.locked_pending;
                }
                other => {
                    // A vblank nobody waited for: the host asked for a redraw.
                    state.redraw = other;
                    drop(state);
                    self.isle.queue_redraw(output);
                    return;
                }
            }
            state.last_shown = Some(presented.time);
            if let Some(planes) = state.pending_planes.take() {
                let scanned_out = matches!(planes.primary, crate::present::Primary::Window(_));
                if planes.asked_async && scanned_out && presented.hardware && !presented.asynchronous && !state.async_refused {
                    state.async_refused = true;
                    tracing::info!(output = output.name(), "an async flip was refused; presenting with vsync");
                }
                let vrr = state.vrr;
                state.trace.push(planes, presented, vrr, |id| {
                    self.isle.wm.get(id).map(|r| r.app.clone()).unwrap_or_default()
                });
            }
            (state.pending_feedback.take(), state.vrr)
        };
        self.isle.shell.presented(output);
        if self.isle.lock.locked
            && self.isle.lock.shown_after.is_none()
            && self
                .isle
                .space
                .outputs()
                .all(|o| OutputState::of(o).borrow().locked_shown)
        {
            self.isle.lock.shown_after = self.isle.lock.since.map(|s| s.elapsed());
        }
        if self.isle.sleep_waiting {
            self.sleep_ready();
        }
        if let Some(mut feedback) = feedback {
            let interval = refresh_interval(output);
            let refresh = if vrr {
                Refresh::Variable(interval)
            } else {
                Refresh::Fixed(interval)
            };
            let mut kind = wp_presentation_feedback::Kind::empty();
            if !presented.asynchronous {
                kind |= wp_presentation_feedback::Kind::Vsync;
            }
            if presented.hardware {
                kind |= wp_presentation_feedback::Kind::HwClock
                    | wp_presentation_feedback::Kind::HwCompletion;
            }
            feedback.presented::<_, smithay::utils::Monotonic>(
                presented.time,
                refresh,
                presented.sequence,
                kind,
            );
        }
        self.isle.send_frame_callbacks(output);
    }
}

/// Frames that took longer than their refresh, by where the time went, said at most once a
/// second.
#[derive(Default)]
pub struct FramePhases {
    last_said: Option<std::time::Instant>,
    slow: u32,
}

impl FramePhases {
    /// `shell` is the shell's own drawing; `glass` drawing what lies behind its glass,
    /// client textures included.
    fn note(&mut self, output: &Output, total: Duration, [steps, dispatch, shell, glass, render]: [Duration; 5]) {
        if total <= refresh_interval(output) {
            return;
        }
        self.slow += 1;
        let now = std::time::Instant::now();
        if self.last_said.is_some_and(|t| now.duration_since(t) < Duration::from_secs(1)) {
            return;
        }
        self.last_said = Some(now);
        let ms = |d: Duration| (d.as_secs_f64() * 1e4).round() / 10.0;
        warn!(
            output = output.name(),
            total_ms = ms(total),
            steps_ms = ms(steps),
            shell_dispatch_ms = ms(dispatch),
            shell_draw_ms = ms(shell),
            glass_ms = ms(glass),
            render_ms = ms(render),
            slow_since_last = std::mem::take(&mut self.slow),
            "a frame over its refresh"
        );
    }
}
