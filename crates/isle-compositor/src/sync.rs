//! When a client's commit may apply: its buffer ready (an explicit sync point, else the
//! dmabuf's own fence), its fifo barrier passed, its commit time reached.

use std::{collections::HashMap, time::Duration};

use smithay::{
    wayland::dmabuf::get_dmabuf,
    desktop::{layer_map_for_output, utils::surface_primary_scanout_output},
    input::pointer::CursorImageStatus,
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
        fifo::FifoBarrierCachedState,
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
        let (acquire, dmabuf) = with_states(surface, |states| {
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
            (acquire, dmabuf)
        });
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
        self.isle.for_each_surface(output, &mut |surface, states| {
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

    /// Applies commits timed for the frame about to be drawn on `output`, shown at `target`.
    pub fn pass_commit_timers(&mut self, output: &Output, target: Duration) {
        let mut clients = HashMap::new();
        self.isle.for_each_surface(output, &mut |surface, states| {
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
    fn for_each_surface(&self, output: &Output, f: &mut dyn FnMut(&WlSurface, &SurfaceData)) {
        let mut visit = |root: &WlSurface| {
            with_surface_tree_downward(
                root,
                (),
                |_, _, _| TraversalAction::DoChildren(()),
                |surface, states, _| f(surface, states),
                |_, _, _| true,
            );
        };
        for record in &self.wm.records {
            if let Some(surface) = record.surface() {
                visit(&surface);
            }
        }
        for window in &self.unmanaged {
            use smithay::wayland::seat::WaylandFocus;
            if let Some(surface) = window.wl_surface() {
                visit(&surface);
            }
        }
        for layer in layer_map_for_output(output).layers() {
            visit(layer.wl_surface());
        }
        if let CursorImageStatus::Surface(surface) = &self.cursor_status {
            visit(surface);
        }
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
