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
}

impl Planes {
    pub const COMPOSITED: Planes = Planes {
        primary: Primary::Composited,
        cursor: Cursor::Drawn,
        overlays: 0,
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

#[cfg(test)]
mod tests {
    use super::*;

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
