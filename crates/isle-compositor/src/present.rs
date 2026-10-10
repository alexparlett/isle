//! What each presented frame put on the screen's planes, kept per output for `isle displays
//! trace`.

use std::{collections::VecDeque, time::Duration};

use isle_ipc::{FrameTrace, PlaneTrace};

/// How many frames an output's trace keeps; the counters cover every frame since a reset.
const KEPT: usize = 1024;

/// What the primary plane carried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Primary {
    /// Isle drew the frame.
    Composited,
    /// A window's buffer was scanned out directly.
    Window(u64),
    /// A buffer of a surface that is no window: a layer, an X11 override-redirect window.
    Surface,
}

/// Where the pointer was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Plane,
    Drawn,
    None,
}

/// A frame's planes, known when it is queued.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Planes {
    pub primary: Primary,
    pub cursor: Cursor,
    pub overlays: usize,
    /// The frame asked to flip without waiting for vblank.
    pub asked_async: bool,
}

impl Planes {
    pub const COMPOSITED: Planes = Planes {
        primary: Primary::Composited,
        cursor: Cursor::Drawn,
        overlays: 0,
        asked_async: false,
    };
}

/// When a frame reached the screen, as the backend reports it.
#[derive(Clone, Copy, Debug)]
pub struct Presented {
    /// On the monotonic clock.
    pub time: Duration,
    pub sequence: u64,
    /// The time and sequence are the display's own, not Isle's.
    pub hardware: bool,
    /// The flip did not wait for vblank.
    pub asynchronous: bool,
}

#[derive(Default)]
pub struct Trace {
    kept: VecDeque<FrameTrace>,
    last: Option<Duration>,
    next_frame: u64,
    frames: u64,
    scanned_out: u64,
    async_flips: u64,
}

impl Trace {
    /// Notes a presented frame and how long it came after the last.
    pub fn push(&mut self, planes: Planes, presented: Presented, vrr: bool, app: impl FnOnce(u64) -> String) {
        let interval = self
            .last
            .map_or(Duration::ZERO, |last| presented.time.saturating_sub(last));
        self.last = Some(presented.time);
        self.next_frame += 1;
        self.frames += 1;
        if planes.primary != Primary::Composited {
            self.scanned_out += 1;
        }
        if presented.asynchronous {
            self.async_flips += 1;
        }
        let primary = match planes.primary {
            Primary::Composited => "composited".to_string(),
            Primary::Window(id) => format!("window {id} {}", app(id)),
            Primary::Surface => "surface".to_string(),
        };
        let cursor = match planes.cursor {
            Cursor::Plane => "plane",
            Cursor::Drawn => "drawn",
            Cursor::None => "none",
        };
        if self.kept.len() == KEPT {
            self.kept.pop_front();
        }
        self.kept.push_back(FrameTrace {
            frame: self.next_frame,
            time_us: presented.time.as_micros() as u64,
            interval_us: interval.as_micros() as u64,
            primary,
            cursor: cursor.into(),
            overlays: planes.overlays as u32,
            asynchronous: presented.asynchronous,
            asked_async: planes.asked_async,
            vrr,
            hardware: presented.hardware,
        });
    }

    /// The counters and the kept frames after `since`.
    pub fn report(&self, output: &str, since: u64) -> PlaneTrace {
        PlaneTrace {
            output: output.into(),
            frames: self.frames,
            scanned_out: self.scanned_out,
            async_flips: self.async_flips,
            recent: self.kept.iter().filter(|f| f.frame > since).cloned().collect(),
        }
    }

    /// Clears the counters; frame numbers go on.
    pub fn reset(&mut self) {
        self.kept.clear();
        self.frames = 0;
        self.scanned_out = 0;
        self.async_flips = 0;
    }
}

/// Whether the frame on an output may flip without waiting for vblank: the output allows
/// tearing, its full-screen window is in front, and that window asks for it. Smithay then
/// tears only when the frame scans that window's buffer out and nothing else changed.
pub fn may_tear(tearing_allowed: bool, front_asks: Option<bool>) -> bool {
    tearing_allowed && front_asks == Some(true)
}

/// Whether `surface`'s tree asks for its frames to be presented without vsync.
pub fn asks_to_tear(surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) -> bool {
    use smithay::{
        reexports::wayland_protocols::wp::tearing_control::v1::server::wp_tearing_control_v1::PresentationHint,
        wayland::{
            compositor::{TraversalAction, with_surface_tree_downward},
            tearing_control::TearingControlSurfaceCachedState,
        },
    };
    let mut asks = false;
    with_surface_tree_downward(
        surface,
        (),
        |_, _, _| TraversalAction::DoChildren(()),
        |_, states, _| {
            asks |= *states
                .cached_state
                .get::<TearingControlSurfaceCachedState>()
                .current()
                .presentation_hint()
                == PresentationHint::Async;
        },
        |_, _, _| true,
    );
    asks
}

impl crate::state::Isle {
    /// Whether `output`'s next frame may tear (`may_tear`).
    pub fn wants_tearing(&self, output: &smithay::output::Output) -> bool {
        let allowed = self
            .settings
            .output
            .get(&Self::output_identity(output))
            .is_some_and(|c| c.tearing);
        let asks = self
            .full_screen_front(output)
            .and_then(|id| self.wm.get(id))
            .and_then(|r| r.surface())
            .map(|surface| asks_to_tear(&surface));
        may_tear(allowed, asks)
    }
}

/// What an output's connector can do about VRR.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VrrCapability {
    None,
    /// Only with a modeset, so never on demand.
    AtModeset,
    OnDemand,
}

impl VrrCapability {
    pub fn name(self) -> &'static str {
        match self {
            VrrCapability::None => "none",
            VrrCapability::AtModeset => "at-modeset",
            VrrCapability::OnDemand => "on-demand",
        }
    }
}

/// Whether an output's refresh should follow its frames now (B30).
pub fn wants_vrr(setting: isle_settings::schema::Vrr, capability: VrrCapability, full_screen_in_front: bool) -> bool {
    use isle_settings::schema::Vrr;
    match (capability, setting) {
        (VrrCapability::None, _) | (_, Vrr::Off) => false,
        (_, Vrr::Always) => true,
        (VrrCapability::AtModeset, Vrr::Fullscreen) => false,
        (VrrCapability::OnDemand, Vrr::Fullscreen) => full_screen_in_front,
    }
}

/// The longest a frame that would move only the pointer is held under VRR, waiting to ride
/// with the next frame of the window in front: the panel's slowest refresh, with KWin's
/// margin, and never past 30 Hz.
pub fn pointer_hold(min_refresh_hz: Option<u32>) -> Duration {
    let floor = Duration::from_micros(33_333);
    match min_refresh_hz {
        Some(hz) if hz > 0 => Duration::from_secs_f64(1.0 / (hz as f64 + 2.0)).min(floor),
        _ => floor,
    }
}

/// How long a pointer-only frame may still wait, or `None` when it must go now.
pub fn hold_left(vrr: bool, last_shown: Option<Duration>, now: Duration, hold: Duration) -> Option<Duration> {
    let last = last_shown.filter(|_| vrr)?;
    (last + hold).checked_sub(now).filter(|left| !left.is_zero())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tearing_needs_the_setting_and_the_front_windows_ask() {
        assert!(may_tear(true, Some(true)));
        assert!(!may_tear(false, Some(true)));
        assert!(!may_tear(true, Some(false)));
        // No full-screen window in front.
        assert!(!may_tear(true, None));
    }

    #[test]
    fn vrr_by_setting_capability_and_front() {
        use isle_settings::schema::Vrr;
        use VrrCapability::*;
        assert!(wants_vrr(Vrr::Fullscreen, OnDemand, true));
        assert!(!wants_vrr(Vrr::Fullscreen, OnDemand, false));
        assert!(wants_vrr(Vrr::Always, OnDemand, false));
        assert!(!wants_vrr(Vrr::Off, OnDemand, true));
        // HDMI on amdgpu, say: only Always, set at the next modeset.
        assert!(!wants_vrr(Vrr::Fullscreen, AtModeset, true));
        assert!(wants_vrr(Vrr::Always, AtModeset, false));
        assert!(!wants_vrr(Vrr::Always, None, true));
    }

    #[test]
    fn the_pointer_waits_at_most_the_panels_slowest_frame() {
        let ms = Duration::from_millis;
        assert_eq!(pointer_hold(Some(48)), Duration::from_millis(20));
        assert_eq!(pointer_hold(Some(20)), Duration::from_micros(33_333));
        assert_eq!(pointer_hold(None), Duration::from_micros(33_333));
        assert_eq!(hold_left(true, Some(ms(100)), ms(105), ms(20)), Some(ms(15)));
        assert_eq!(hold_left(true, Some(ms(100)), ms(125), ms(20)), None);
        assert_eq!(hold_left(false, Some(ms(100)), ms(105), ms(20)), None);
        assert_eq!(hold_left(true, None, ms(105), ms(20)), None);
    }

    fn at(ms: u64, asynchronous: bool) -> Presented {
        Presented {
            time: Duration::from_millis(ms),
            sequence: ms,
            hardware: true,
            asynchronous,
        }
    }

    #[test]
    fn counts_and_intervals() {
        let mut trace = Trace::default();
        let game = Planes {
            primary: Primary::Window(7),
            cursor: Cursor::Plane,
            overlays: 0,
            asked_async: true,
        };
        trace.push(Planes::COMPOSITED, at(10, false), false, |_| unreachable!());
        trace.push(game, at(17, true), true, |id| format!("app{id}"));
        let report = trace.report("DP-1", 0);
        assert_eq!((report.frames, report.scanned_out, report.async_flips), (2, 1, 1));
        assert_eq!(report.recent[1].primary, "window 7 app7");
        assert_eq!(report.recent[1].interval_us, 7000);
        assert_eq!(trace.report("DP-1", 1).recent.len(), 1);
        trace.reset();
        trace.push(game, at(24, false), true, |_| String::new());
        let report = trace.report("DP-1", 0);
        assert_eq!((report.frames, report.recent[0].frame), (1, 3));
    }

    #[test]
    fn keeps_the_last_frames() {
        let mut trace = Trace::default();
        for ms in 0..(KEPT as u64 + 10) {
            trace.push(Planes::COMPOSITED, at(ms, false), false, |_| String::new());
        }
        let report = trace.report("DP-1", 0);
        assert_eq!(report.recent.len(), KEPT);
        assert_eq!(report.frames, KEPT as u64 + 10);
        assert_eq!(report.recent[0].frame, 11);
    }
}
