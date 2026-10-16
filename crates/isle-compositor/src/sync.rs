//! When a client's commit may apply: its buffer ready (an explicit sync point, else the
//! dmabuf's own fence), its fifo barrier passed, its commit time reached.

use std::{collections::HashMap, time::Duration};

use smithay::{
    wayland::dmabuf::get_dmabuf,
    desktop::utils::surface_primary_scanout_output,
    output::Output,
    reexports::{
        calloop::{
            Interest,
            timer::{TimeoutAction, Timer},
        },
        wayland_server::{Client, Resource, backend::ClientId, protocol::wl_surface::WlSurface},
    },
    utils::{Monotonic, Time},
    wayland::{
        compositor::{
            BufferAssignment, CompositorHandler, SurfaceAttributes, SurfaceData, add_blocker,
            add_pre_commit_hook, with_states, with_surface_tree_downward, TraversalAction,
        },
        commit_timing::CommitTimerBarrierStateUserData,
        drm_syncobj::{DrmSyncobjCachedState, DrmSyncobjHandler, DrmSyncobjState},
        fifo::{FifoBarrierCachedState, FifoCachedState},
    },
};
use tracing::warn;

use crate::state::{Isle, State};

impl DrmSyncobjHandler for State {
    fn drm_syncobj_state(&mut self) -> Option<&mut DrmSyncobjState> {
        self.isle.syncobj_state.as_mut()
    }
}

/// Holds each commit of `surface` until its buffer can be read, so a frame never samples
/// or scans out a buffer the client's GPU is still writing.
pub fn hold_unready_buffers(surface: &WlSurface) {
    add_pre_commit_hook::<State, _>(surface, |state, _, surface| {
        // A commit time is registered by the protocol's own hook; its wait is armed after.
        let target = surface.downgrade();
        state.isle.loop_handle.insert_idle(move |state| {
            if let Ok(surface) = target.upgrade() {
                state.arm_commit_timer(&surface);
            }
        });
        let (acquire, dmabuf, barrier) = with_states(surface, |states| {
            // The protocol's own hook, after this one, makes the barrier the request asks for.
            let barrier = states.cached_state.get::<FifoCachedState>().pending().set_barrier
                || states
                    .cached_state
                    .get::<FifoBarrierCachedState>()
                    .pending()
                    .barrier
                    .is_some();
            let acquire = states
                .cached_state
                .get::<DrmSyncobjCachedState>()
                .pending()
                .acquire_point
                .clone();
            let dmabuf = match states.cached_state.get::<SurfaceAttributes>().pending().buffer.as_ref() {
                Some(BufferAssignment::NewBuffer(buffer)) => get_dmabuf(buffer).cloned().ok(),
                _ => None,
            };
            (acquire, dmabuf, barrier)
        });
        if barrier {
            state.isle.paced.insert(surface.id(), surface.downgrade());
        }
        let Some(client) = surface.client() else {
            return;
        };
        if let Some(acquire) = acquire {
            if let Ok((blocker, source)) = acquire.generate_blocker() {
                let inserted = state.isle.loop_handle.insert_source(source, move |_, _, state| {
                    state.clear_blockers(&client);
                    Ok(())
                });
                if inserted.is_ok() {
                    add_blocker(surface, blocker);
                }
            }
            return;
        }
        let Some(dmabuf) = dmabuf else {
            return;
        };
        if let Ok((blocker, source)) = dmabuf.generate_blocker(Interest::READ) {
            let inserted = state.isle.loop_handle.insert_source(source, move |_, _, state| {
                state.clear_blockers(&client);
                Ok(())
            });
            if inserted.is_ok() {
                add_blocker(surface, blocker);
            }
        }
    });
}

impl State {
    /// Lets `client`'s commits whose blockers have cleared apply.
    pub fn clear_blockers(&mut self, client: &Client) {
        let dh = self.isle.display_handle.clone();
        self.client_compositor_state(client).blocker_cleared(self, &dh);
    }

    fn clear_all(&mut self, clients: HashMap<ClientId, Client>) {
        for client in clients.into_values() {
            self.clear_blockers(&client);
        }
    }

    /// A frame of `output` was drawn: fifo barriers of surfaces mostly on it, and of
    /// surfaces shown nowhere, are passed.
    pub fn pass_fifo_barriers(&mut self, output: &Output) {
        let mut clients = HashMap::new();
        self.isle.for_each_paced(&mut |surface, states| {
            let on_here = surface_primary_scanout_output(surface, states).is_none_or(|o| &o == output);
            if !on_here {
                return;
            }
            let barrier = states
                .cached_state
                .get::<FifoBarrierCachedState>()
                .current()
                .barrier
                .take();
            if let Some(barrier) = barrier {
                barrier.signal();
                if let Some(client) = surface.client() {
                    clients.insert(client.id(), client);
                }
            }
        });
        self.clear_all(clients);
    }

    /// Applies commits timed for the frame about to be drawn, shown at `target`.
    pub fn pass_commit_timers(&mut self, target: Duration) {
        let mut clients = HashMap::new();
        self.isle.for_each_paced(&mut |surface, states| {
            if let Some(timers) = states.data_map.get::<CommitTimerBarrierStateUserData>()
                && timers.lock().unwrap().signal_until(Time::<Monotonic>::from(target))
                && let Some(client) = surface.client()
            {
                clients.insert(client.id(), client);
            }
        });
        self.clear_all(clients);
    }

    /// Wakes when `surface`'s next timed commit is due, so it applies even while nothing
    /// else draws.
    fn arm_commit_timer(&mut self, surface: &WlSurface) {
        let deadline = with_states(surface, |states| {
            states
                .data_map
                .get::<CommitTimerBarrierStateUserData>()
                .and_then(|timers| timers.lock().unwrap().next_deadline())
        });
        let Some(deadline) = deadline else {
            return;
        };
        let id = surface.id();
        self.isle.paced.insert(id.clone(), surface.downgrade());
        if let Some((armed, token)) = self.isle.commit_timers.get(&id) {
            if *armed <= deadline {
                return;
            }
            self.isle.loop_handle.remove(*token);
        }
        let now: Duration = self.isle.clock.now().into();
        let due: Duration = Time::<Monotonic>::from(deadline).into();
        let target = surface.downgrade();
        let timer = Timer::from_duration(due.saturating_sub(now));
        let key = id.clone();
        let armed = self.isle.loop_handle.insert_source(timer, move |_, _, state| {
            let _beat = state.isle.heartbeat.beat("commit-timer");
            state.isle.commit_timers.remove(&key);
            if let Ok(surface) = target.upgrade() {
                let passed = with_states(&surface, |states| {
                    states
                        .data_map
                        .get::<CommitTimerBarrierStateUserData>()
                        .is_some_and(|timers| timers.lock().unwrap().signal_until(deadline))
                });
                if passed && let Some(client) = surface.client() {
                    state.clear_blockers(&client);
                }
                state.arm_commit_timer(&surface);
            }
            TimeoutAction::Drop
        });
        match armed {
            Ok(token) => {
                self.isle.commit_timers.insert(id, (deadline, token));
            }
            Err(err) => warn!("could not arm a commit timer: {err}"),
        }
    }
}

/// How often a game on a desktop out of view is let draw: it keeps its loop, network and
/// sound going at a fraction of its GPU time.
const HIDDEN_GAME: Duration = Duration::from_millis(33);
/// How often any other window out of view is.
const HIDDEN: Duration = Duration::from_secs(1);

impl State {
    /// Frame callbacks and fifo barriers for windows that are mapped but out of view, which
    /// no output's frame reaches.
    pub fn pace_hidden_windows(&mut self) {
        let mut since_slow = Duration::ZERO;
        let timer = Timer::from_duration(HIDDEN);
        let armed = self.isle.loop_handle.insert_source(timer, move |_, _, state| {
            let _beat = state.isle.heartbeat.beat("hidden-pacer");
            let hidden: Vec<(u64, bool)> = state
                .isle
                .wm
                .records
                .iter()
                .filter(|r| r.mapped && !state.isle.wm.is_shown(r.id))
                .map(|r| (r.id, r.game))
                .collect();
            let any_game = hidden.iter().any(|(_, game)| *game);
            let step = if any_game { HIDDEN_GAME } else { HIDDEN };
            since_slow += step;
            let slow_due = since_slow >= HIDDEN || !any_game;
            if slow_due {
                since_slow = Duration::ZERO;
            }
            let due: Vec<u64> = hidden
                .into_iter()
                .filter(|(_, game)| *game || slow_due)
                .map(|(id, _)| id)
                .collect();
            state.let_hidden_draw(&due);
            TimeoutAction::ToDuration(step)
        });
        if let Err(err) = armed {
            warn!("could not pace hidden windows: {err}");
        }
    }

    fn let_hidden_draw(&mut self, ids: &[u64]) {
        let Some(output) = self.isle.space.outputs().next().cloned() else {
            return;
        };
        let time = self.isle.clock.now();
        let mut clients = HashMap::new();
        for id in ids {
            let Some(record) = self.isle.wm.get(*id) else {
                continue;
            };
            record
                .window
                .send_frame(&output, time, None, |_, _| Some(output.clone()));
            let Some(root) = record.surface() else {
                continue;
            };
            with_surface_tree_downward(
                &root,
                (),
                |_, _, _| TraversalAction::DoChildren(()),
                |surface, states, _| {
                    let barrier = states
                        .cached_state
                        .get::<FifoBarrierCachedState>()
                        .current()
                        .barrier
                        .take();
                    if let Some(barrier) = barrier {
                        barrier.signal();
                        if let Some(client) = surface.client() {
                            clients.insert(client.id(), client);
                        }
                    }
                },
                |_, _, _| true,
            );
        }
        self.clear_all(clients);
    }
}

impl Isle {
    /// Serves linux-drm-syncobj on `device` when it can wait on sync points.
    pub fn serve_syncobj(&mut self, device: smithay::backend::drm::DrmDeviceFd) {
        use smithay::wayland::drm_syncobj::supports_syncobj_eventfd;
        if self.syncobj_state.is_some() {
            return;
        }
        if !supports_syncobj_eventfd(&device) {
            tracing::info!("no explicit sync: the device cannot wait on sync points");
            return;
        }
        self.syncobj_state = Some(DrmSyncobjState::new::<State>(&self.display_handle, device));
    }

    /// Every surface that may be shown on `output`, and every window's shown nowhere.
    /// Each surface with a fifo barrier or a timed commit outstanding, dropping those gone
    /// or with neither left.
    fn for_each_paced(&mut self, f: &mut dyn FnMut(&WlSurface, &SurfaceData)) {
        self.paced.retain(|_, weak| {
            let Ok(surface) = weak.upgrade() else {
                return false;
            };
            with_states(&surface, |states| {
                f(&surface, states);
                // A barrier set by a commit still held is in the pending state until it applies.
                let mut barriers = states.cached_state.get::<FifoBarrierCachedState>();
                let barrier = barriers.current().barrier.is_some()
                    || barriers
                        .pending()
                        .barrier
                        .as_ref()
                        .is_some_and(|b| !b.is_signaled());
                let timed = states
                    .data_map
                    .get::<CommitTimerBarrierStateUserData>()
                    .is_some_and(|timers| timers.lock().unwrap().next_deadline().is_some());
                barrier || timed
            })
        });
    }
}

/// The commit time a frame shown at `target` covers.
pub fn frame_target(last_shown: Option<Duration>, now: Duration, interval: Duration) -> Duration {
    match last_shown {
        Some(last) if last + interval > now => last + interval,
        _ => now + interval,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::wayland::commit_timing::Timestamp;

    #[test]
    fn the_next_frame_follows_the_last_or_starts_from_now() {
        let ms = Duration::from_millis;
        assert_eq!(frame_target(Some(ms(100)), ms(105), ms(16)), ms(116));
        assert_eq!(frame_target(Some(ms(100)), ms(140), ms(16)), ms(156));
        assert_eq!(frame_target(None, ms(10), ms(16)), ms(26));
    }

    #[test]
    fn timestamps_round_trip_the_clock() {
        let at = Duration::from_millis(1234);
        let stamp = Timestamp::from(Time::<Monotonic>::from(at));
        let back: Duration = Time::<Monotonic>::from(stamp).into();
        assert_eq!(back, at);
    }
}
