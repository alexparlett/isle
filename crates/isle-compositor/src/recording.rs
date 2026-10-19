//! A screen recording: a stream of the chosen source read by the `isle-record` helper,
//! which encodes it (`p11-capture.md` choices 14–16). One at a time; the island shows it
//! with its elapsed time and Stop.

use std::{
    io::{BufRead, BufReader, Write},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    time::{Duration, Instant},
};

use smithay::{
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
};
use tracing::{info, warn};

use crate::{
    backend::Pixels,
    capture::{Destination, Kind, Target, destination},
    cast::{CursorMode, Ended, Source},
    desktop_notes::Note,
    state::State,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Waiting for the stream's node.
    Starting,
    Running,
    Stopping,
}

pub struct Recording {
    pub stream: u64,
    pub source: Source,
    pub path: PathBuf,
    pub started: Instant,
    phase: Phase,
    output: Output,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    /// The source's first frame, made small for the thumbnail.
    first: Option<Pixels>,
    /// Why it stopped, when its source went.
    ended: Option<Ended>,
    tick: Option<RegistrationToken>,
}

/// Whether a recorder is installed, for the toolbar's recording modes.
pub fn available() -> bool {
    crate::helpers::record().available()
}

/// `0:05`, `1:02:03`.
fn elapsed(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

impl State {
    pub fn recording(&self) -> bool {
        self.isle.recording_now.is_some()
    }

    /// A press while recording with Show mouse clicks on: marked at the pointer in the
    /// recording alone.
    pub fn mark_click(&mut self) {
        if !self.isle.settings.capture.show_clicks || self.isle.recording_now.is_none() {
            return;
        }
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        self.isle.clicks.push((at, Instant::now()));
        self.isle.queue_redraw_all();
    }

    /// Starts recording `source`, unless one is running.
    pub fn start_recording(&mut self, source: Source) {
        if self.isle.recording_now.is_some() || self.isle.lock.locked {
            return;
        }
        let output = match &source {
            Source::Screen(o) | Source::Portion(o, _) => Some(o.clone()),
            Source::Window(w) => self.isle.output_of(*w),
        };
        let Some(output) = output.or_else(|| self.isle.main_output()) else {
            return;
        };
        let folder = match destination(&self.isle.settings.capture.save_to, Kind::Recording) {
            Destination::Folder(folder) => folder,
            Destination::Clipboard => return,
        };
        if let Err(err) = std::fs::create_dir_all(&folder) {
            self.recording_failed(&err.to_string());
            return;
        }
        let at = jiff::Zoned::now();
        let mut n = 0;
        let path = loop {
            let path = folder.join(format!(
                "{}.mp4",
                crate::capture::file_stem(Kind::Recording, &at, n)
            ));
            if !path.exists() {
                break path;
            }
            n += 1;
        };
        let target = match &source {
            Source::Screen(o) => Target::Screen(o.clone()),
            Source::Portion(o, r) => Target::Region(o.clone(), *r),
            Source::Window(id) => Target::Window {
                id: *id,
                shadow: false,
            },
        };
        let first = self
            .capture_pixels(&target, false)
            .ok()
            .map(|p| crate::capture::thumbnail_of(&p));
        let cursor = if self.isle.settings.capture.show_pointer {
            CursorMode::Embedded
        } else {
            CursorMode::Hidden
        };
        let stream = match self.start_cast(source.clone(), cursor, "recording") {
            Ok(id) => id,
            Err(err) => {
                self.recording_failed(&err);
                return;
            }
        };
        info!(path = %path.display(), "recording");
        self.isle.recording_now = Some(Recording {
            stream,
            source,
            path,
            started: Instant::now(),
            phase: Phase::Starting,
            output,
            child: None,
            stdin: None,
            first,
            ended: None,
            tick: None,
        });
        self.recording_activity();
    }

    /// The stream's node is ready: the helper starts reading it.
    pub fn recording_node(&mut self, stream: u64, node: u32) {
        let sink = self
            .isle
            .settings
            .capture
            .system_audio
            .then(|| {
                self.isle
                    .services
                    .audio
                    .state
                    .default_output()
                    .map(|d| d.name.clone())
            })
            .flatten();
        let mic = self.isle.settings.capture.microphone.clone();
        let Some(recording) = self.isle.recording_now.as_mut() else {
            return;
        };
        if recording.stream != stream || recording.phase != Phase::Starting {
            return;
        }
        let fps = recording
            .output
            .current_mode()
            .map_or(60, |m| (m.refresh as u32).div_ceil(1000).min(60));
        let mut command = Command::new(crate::helpers::record().program());
        command.arg0("isle-record");
        command
            .args(["--node", &node.to_string(), "--fps", &fps.to_string()])
            .envs(crate::child_env::vars())
            .arg("--out")
            .arg(&recording.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if let Some(sink) = sink {
            command.args(["--sink", &sink]);
        }
        if let Some(mic) = mic {
            command.args(["--mic", &mic]);
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                let why = err.to_string();
                self.end_cast(stream, Ended::Stopped);
                self.isle.recording_now = None;
                self.recording_failed(&why);
                return;
            }
        };
        recording.stdin = child.stdin.take();
        let stdout = child.stdout.take();
        recording.child = Some(child);
        recording.phase = Phase::Running;
        recording.started = Instant::now();
        if let (Some(stdout), Some(to_loop)) = (stdout, self.isle.to_loop.clone()) {
            std::thread::Builder::new()
                .name("isle-record".into())
                .spawn(move || {
                    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                        let line = line.trim().to_owned();
                        to_loop.post(move |state| state.recorder_said(&line));
                    }
                    to_loop.post(|state| state.recorder_exited());
                })
                .ok();
        }
        self.arm_recording_tick();
        self.recording_activity();
    }

    fn recorder_said(&mut self, line: &str) {
        if let Some(why) = line.strip_prefix("error ") {
            warn!(why, "the recorder");
            let started = self
                .isle
                .recording_now
                .as_ref()
                .is_some_and(|r| r.path.exists());
            if !started {
                self.recording_failed(why);
            }
        } else {
            info!(line, "the recorder");
        }
    }

    /// The helper ended: the stream goes, and the thumbnail shows the file once there is one.
    fn recorder_exited(&mut self) {
        let Some(mut recording) = self.isle.recording_now.take() else {
            return;
        };
        if let Some(token) = recording.tick.take() {
            self.isle.loop_handle.remove(token);
        }
        if let Some(mut child) = recording.child.take() {
            let _ = child.wait();
        }
        if self.isle.casting.get(recording.stream).is_some() {
            self.end_cast(recording.stream, Ended::Stopped);
        }
        self.end_recording_activity();
        if let Some(why) = recording.ended {
            self.island_recording_ended(why);
        }
        if recording.path.exists()
            && let Some(first) = recording.first.take()
            && self.isle.settings.capture.floating_thumbnail
            && !self.game_in_front()
        {
            self.show_thumbnail(recording.path.clone(), &recording.output, first);
        }
        info!(path = %recording.path.display(), "recording kept");
    }

    /// Stop: the helper finishes the file and exits.
    pub fn stop_recording(&mut self) -> bool {
        let Some(recording) = self.isle.recording_now.as_mut() else {
            return false;
        };
        match recording.phase {
            Phase::Starting => {
                let stream = recording.stream;
                self.isle.recording_now = None;
                self.end_cast(stream, Ended::Stopped);
                self.end_recording_activity();
            }
            Phase::Running => {
                recording.phase = Phase::Stopping;
                if let Some(mut stdin) = recording.stdin.take() {
                    let _ = writeln!(stdin, "stop");
                }
            }
            Phase::Stopping => {}
        }
        true
    }

    /// The recorded stream ended on its own: the window closed or the screen went.
    pub fn recording_stream_ended(&mut self, stream: u64, why: Ended) {
        let Some(recording) = self.isle.recording_now.as_mut() else {
            return;
        };
        if recording.stream != stream || why == Ended::Stopped {
            return;
        }
        recording.ended = Some(why);
        self.stop_recording();
    }

    fn arm_recording_tick(&mut self) {
        let token = self
            .isle
            .loop_handle
            .insert_source(
                Timer::from_duration(Duration::from_secs(1)),
                |_, _, state| {
                    if state.isle.recording_now.is_none() {
                        return TimeoutAction::Drop;
                    }
                    state.recording_activity();
                    TimeoutAction::ToDuration(Duration::from_secs(1))
                },
            )
            .ok();
        if let Some(recording) = self.isle.recording_now.as_mut() {
            recording.tick = token;
        }
    }

    /// The island's Recording activity: the elapsed time, what is recorded, and Stop.
    fn recording_activity(&mut self) {
        use isle_shell::island::model::{Activity, ActivityKind};
        let Some(recording) = &self.isle.recording_now else {
            return;
        };
        let what = match &recording.source {
            Source::Screen(_) => "Entire screen".to_owned(),
            Source::Portion(..) => "Selected portion".to_owned(),
            Source::Window(id) => self
                .isle
                .wm
                .get(*id)
                .map(|r| self.isle.app_name(&r.app))
                .unwrap_or_else(|| "Window".into()),
        };
        let audio = self.isle.settings.capture.system_audio
            || self.isle.settings.capture.microphone.is_some();
        let activity = Activity {
            kind: ActivityKind::Recording,
            face: elapsed(recording.started.elapsed()),
            title: "Recording".into(),
            detail: if audio {
                format!("{what}, with sound")
            } else {
                what
            },
            actions: vec![("stop".into(), "Stop".into())],
            app: None,
            art: None,
            started: recording.started,
        };
        self.isle
            .island
            .model
            .set_activity(activity, Instant::now());
        self.isle.refresh_island();
    }

    fn end_recording_activity(&mut self) {
        self.isle
            .island
            .model
            .end_activity(isle_shell::island::model::ActivityKind::Recording);
        self.isle.refresh_island();
    }

    fn island_recording_ended(&mut self, why: Ended) {
        use isle_shell::island::model::{Event, EventKind, Form};
        self.isle.island.model.post(
            Event {
                kind: EventKind::Ended,
                form: Form::Line {
                    symbol: "record".into(),
                    fill: false,
                    text: format!("Recording stopped: {}", why.reason()),
                },
                output: None,
                app: None,
                urgent: false,
            },
            Instant::now(),
        );
        self.isle.refresh_island();
    }

    fn recording_failed(&mut self, why: &str) {
        self.post_desktop(
            "",
            "Screen Recording",
            format!("Couldn't start recording. {why}"),
            String::new(),
            Vec::new(),
            Note::Capture,
        );
    }

    /// The recording as JSON, for the Test interface.
    #[cfg(feature = "test-hooks")]
    pub fn recording_status(&self) -> serde_json::Value {
        match &self.isle.recording_now {
            None => serde_json::Value::Null,
            Some(r) => serde_json::json!({
                "stream": r.stream,
                "path": r.path,
                "phase": format!("{:?}", r.phase),
                "pid": r.child.as_ref().map(|c| c.id()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_face_counts_minutes_then_hours() {
        assert_eq!(elapsed(Duration::from_secs(5)), "0:05");
        assert_eq!(elapsed(Duration::from_secs(62)), "1:02");
        assert_eq!(elapsed(Duration::from_secs(3723)), "1:02:03");
    }
}
