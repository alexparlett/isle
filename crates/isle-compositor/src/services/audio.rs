//! Sound: the Sound glyph, the microphone and camera marks, volume OSDs and keys, and the
//! Sound module with its page.

use std::time::{Duration, Instant};

use isle_audio::{Audio, Target, camera::Cameras};
use isle_bus::Snapshot;
use isle_shell::island::{
    grid::TileSize,
    model::{Glyph, Privacy},
    panel::Module,
    rows::{Choice, Lead, PageRow, SliderRow},
};
use tracing::debug;

use crate::{
    osd::volume_symbol,
    state::{Isle, State},
};

/// One step of the volume keys.
const STEP: f32 = 0.05;
/// The input meter's redraws, at most.
const METER_FRAME: Duration = Duration::from_millis(66);

impl State {
    pub fn audio_changed(&mut self, snapshot: Snapshot<Audio>) {
        let before = self.isle.services.audio.state.clone();
        let was = self.isle.services.audio.available;
        self.isle.services.audio = snapshot;
        if was && self.isle.services.audio.available && !self.isle.own_change() {
            self.isle.audio_osds(&before);
        }
        self.isle.services_changed();
    }

    pub fn cameras_changed(&mut self, snapshot: Snapshot<Cameras>) {
        self.isle.services.cameras = snapshot;
        self.isle.services_changed();
    }

    /// The volume, mute and microphone keys.
    pub fn audio_key(&mut self, key: crate::binds::Media) {
        use crate::binds::Media;
        let audio = &self.isle.services.audio.state;
        let target = match key {
            Media::MicMute => audio.default_input(),
            _ => audio.default_output(),
        };
        let Some(device) = target.cloned() else {
            return;
        };
        let (volume, mute) = match key {
            Media::VolumeUp => (Some((device.volume + STEP).min(1.0)), Some(false)),
            Media::VolumeDown => (Some((device.volume - STEP).max(0.0)), None),
            Media::Mute | Media::MicMute => (None, Some(!device.muted)),
            _ => return,
        };
        let target = if key == Media::MicMute {
            Target::Input(device.name.clone())
        } else {
            Target::Output(device.name.clone())
        };
        let channels = device.channels();
        self.isle.run_service(
            async move {
                if let Some(volume) = volume {
                    isle_audio::set_volume(target.clone(), channels, volume).await?;
                }
                if let Some(mute) = mute.filter(|m| *m != device.muted) {
                    isle_audio::set_mute(target, mute).await?;
                }
                Ok(())
            },
            |_, result| {
                if let Err(err) = result {
                    debug!("a volume key: {err}");
                }
            },
        );
    }

    /// A control of the Sound module or its page.
    pub fn sound_action(&mut self, action: isle_shell::island::panel::PanelAction) {
        use isle_shell::island::panel::PanelAction;
        let audio = self.isle.services.audio.state.clone();
        self.isle.expect_own_change();
        let command: Option<
            std::pin::Pin<Box<dyn Future<Output = Result<(), isle_bus::Error>> + Send>>,
        > = match action {
            PanelAction::Toggle(_) => audio.default_output().map(|d| {
                let (name, mute) = (d.name.clone(), !d.muted);
                Box::pin(isle_audio::set_mute(Target::Output(name), mute)) as _
            }),
            PanelAction::Slide { row, value, .. } if row.is_empty() => {
                audio.default_output().map(|d| {
                    Box::pin(isle_audio::set_volume(
                        Target::Output(d.name.clone()),
                        d.channels(),
                        value,
                    )) as _
                })
            }
            PanelAction::Slide { row, value, .. } if row == "input" => {
                audio.default_input().map(|d| {
                    Box::pin(isle_audio::set_volume(
                        Target::Input(d.name.clone()),
                        d.channels(),
                        value,
                    )) as _
                })
            }
            PanelAction::Slide { row, value, .. } => row
                .strip_prefix("app:")
                .and_then(|i| i.parse::<u32>().ok())
                .and_then(|i| audio.playing.iter().find(|s| s.index == i))
                .map(|s| {
                    Box::pin(isle_audio::set_volume(
                        Target::Playing(s.index),
                        s.channels(),
                        value,
                    )) as _
                }),
            PanelAction::Switch { row, on, .. } if row == "mic-mute" => audio
                .default_input()
                .map(|d| Box::pin(isle_audio::set_mute(Target::Input(d.name.clone()), on)) as _),
            PanelAction::Choose { row, .. } => {
                if let Some(name) = row.strip_prefix("out:") {
                    Some(Box::pin(isle_audio::set_default_output(name.to_owned())) as _)
                } else {
                    row.strip_prefix("in:")
                        .map(|name| Box::pin(isle_audio::set_default_input(name.to_owned())) as _)
                }
            }
            _ => None,
        };
        if let Some(command) = command {
            self.isle.run_service(command, |state, result| {
                if let Err(err) = result {
                    state
                        .isle
                        .services
                        .errors
                        .insert("sound".into(), format!("Couldn't change the sound: {err}"));
                    state.isle.refresh_island();
                }
            });
        }
    }
}

impl Isle {
    /// OSDs for what changed between `before` and now on the default devices.
    fn audio_osds(&mut self, before: &Audio) {
        let now = self.services.audio.state.clone();
        if let (Some(old), Some(new)) = (before.default_output(), now.default_output())
            && old.name == new.name
            && (old.volume != new.volume || old.muted != new.muted)
        {
            self.osd_level(
                volume_symbol(new.volume, new.muted),
                &new.label,
                new.volume,
                new.muted,
                None,
            );
        }
        if let (Some(old), Some(new)) = (before.default_input(), now.default_input())
            && old.name == new.name
            && old.muted != new.muted
        {
            if new.muted {
                self.osd_line("microphone-slash", "Muted");
            } else {
                self.osd_line("microphone", &new.label);
            }
        }
    }

    /// The Sound glyph: shown while the output is muted.
    pub fn sound_glyph(&self) -> Option<Glyph> {
        let audio = &self.services.audio;
        let output = audio.state.default_output()?;
        (audio.available && output.muted).then(|| Glyph {
            active: true,
            symbol: "speaker-slash".into(),
            alt: "Sound muted".into(),
            ..Default::default()
        })
    }

    /// The privacy marks from the microphone and cameras in use.
    pub fn audio_privacy(&mut self) {
        let microphone =
            self.services.audio.available && !self.services.audio.state.recording.is_empty();
        let camera =
            self.services.cameras.available && !self.services.cameras.state.in_use.is_empty();
        self.island
            .model
            .set_privacy(Privacy::Microphone, microphone);
        self.island.model.set_privacy(Privacy::Camera, camera);
    }

    /// Who is using the microphone and the camera, for Control Centre's Now.
    pub fn privacy_rows(&self) -> Vec<(Privacy, String)> {
        let mut rows: Vec<(Privacy, String)> = Vec::new();
        if self.services.audio.available {
            for stream in &self.services.audio.state.recording {
                let name = self.stream_app(stream.pid, &stream.app).0;
                if !rows
                    .iter()
                    .any(|(p, n)| *p == Privacy::Microphone && *n == name)
                {
                    rows.push((Privacy::Microphone, name));
                }
            }
        }
        if self.services.cameras.available {
            for user in &self.services.cameras.state.in_use {
                let name = self.stream_app(user.pid, &user.app).0;
                if !rows
                    .iter()
                    .any(|(p, n)| *p == Privacy::Camera && *n == name)
                {
                    rows.push((Privacy::Camera, name));
                }
            }
        }
        rows
    }

    /// The app a process belongs to, by its pid or an ancestor's, as a name and an icon;
    /// else the name the stream gave.
    pub fn stream_app(
        &self,
        pid: Option<i32>,
        fallback: &str,
    ) -> (String, Option<std::path::PathBuf>) {
        let mut pid = pid;
        for _ in 0..6 {
            let Some(p) = pid else { break };
            let app = self
                .wm
                .records
                .iter()
                .find(|r| r.pid == Some(p))
                .map(|r| r.app.clone())
                .or_else(|| self.processes.app_of(p));
            if let Some(app) = app {
                let name = self.app_name(&app);
                let icon = self.app_icon(&app, 20);
                return (
                    if name.is_empty() {
                        fallback.to_owned()
                    } else {
                        name
                    },
                    icon,
                );
            }
            pid = parent_of(p);
        }
        (fallback.to_owned(), None)
    }

    pub fn sound_module(&self, size: TileSize) -> Option<Module> {
        let audio = &self.services.audio;
        if !audio.available && audio.state.outputs.is_empty() {
            return None;
        }
        let a = &audio.state;
        let output = a.default_output();
        let (volume, muted) = output.map_or((0.0, false), |o| (o.volume, o.muted));
        let state = if !audio.available {
            "Not available".to_owned()
        } else if muted {
            "Muted".to_owned()
        } else {
            output.map(|o| o.label.clone()).unwrap_or_default()
        };
        let mut rows = vec![PageRow::Heading("Output".into())];
        rows.extend(a.outputs.iter().map(|o| {
            PageRow::Choice(Choice {
                id: format!("out:{}", o.name),
                label: o.label.clone(),
                lead: Lead::Glyph(
                    if o.bluetooth {
                        "headphones"
                    } else {
                        "speaker-hifi"
                    }
                    .into(),
                ),
                checked: o.default,
                enabled: audio.available,
                ..Default::default()
            })
        }));
        rows.push(PageRow::Heading("Apps".into()));
        if a.playing.is_empty() {
            rows.push(PageRow::Status("No apps playing sound".into()));
        }
        rows.extend(a.playing.iter().map(|s| {
            let (name, icon) = self.stream_app(s.pid, &s.app);
            PageRow::Slider(SliderRow {
                id: format!("app:{}", s.index),
                label: name,
                lead: icon.map_or(Lead::Glyph("app-window".into()), Lead::Icon),
                value: s.volume,
                muted: s.muted,
                enabled: audio.available,
            })
        }));
        rows.push(PageRow::Heading("Input".into()));
        rows.extend(a.inputs.iter().map(|i| {
            PageRow::Choice(Choice {
                id: format!("in:{}", i.name),
                label: i.label.clone(),
                lead: Lead::Glyph("microphone".into()),
                checked: i.default,
                enabled: audio.available,
                ..Default::default()
            })
        }));
        if let Some(input) = a.default_input() {
            rows.push(PageRow::Slider(SliderRow {
                id: "input".into(),
                label: "Input volume".into(),
                value: input.volume,
                muted: input.muted,
                enabled: audio.available,
                ..Default::default()
            }));
            rows.push(PageRow::Meter {
                label: "Input level".into(),
                level: self.services.input_level,
            });
            rows.push(PageRow::Switch {
                id: "mic-mute".into(),
                label: "Mute microphone".into(),
                on: input.muted,
                enabled: audio.available,
            });
        }
        if let Some(error) = self.services.errors.get("sound") {
            rows.push(PageRow::Status(error.clone()));
        }
        Some(Module {
            id: "sound".into(),
            size,
            name: "Sound".into(),
            state,
            glyph: volume_symbol(volume, muted).into(),
            on: false,
            toggles: true,
            page: Some(rows),
            pane: "Sound".into(),
            slider: Some(volume),
            muted,
            ..Default::default()
        })
    }
}

impl State {
    /// Measures the input while Sound's page shows, and only then.
    pub fn follow_meter(&mut self) {
        let isle = &mut self.isle;
        let wanted = isle
            .island
            .panel
            .as_ref()
            .and_then(|p| p.page.as_deref())
            .is_some_and(|p| p.split('/').next() == Some("sound"))
            && isle.services.audio.available;
        let source = isle
            .services
            .audio
            .state
            .default_input()
            .map(|i| i.name.clone());
        match (
            wanted.then_some(source).flatten(),
            isle.services.meter.is_some(),
        ) {
            (Some(source), false) => {
                let (Some(runtime), Some(to_loop)) = (&isle.worker, &isle.to_loop) else {
                    return;
                };
                let to_loop = to_loop.clone();
                isle.services.meter = Some(runtime.spawn(async move {
                    let Ok(mut meter) = isle_audio::meter(&source).await else {
                        return;
                    };
                    let mut last = Instant::now();
                    let mut peak: f32 = 0.0;
                    while let Some(level) = meter.peaks.recv().await {
                        peak = peak.max(level);
                        if last.elapsed() >= METER_FRAME {
                            let shown = std::mem::take(&mut peak);
                            to_loop.post(move |state| {
                                state.isle.services.input_level = shown;
                                state.isle.refresh_island();
                            });
                            last = Instant::now();
                        }
                    }
                }));
            }
            (None, true) => {
                if let Some(task) = isle.services.meter.take() {
                    task.abort();
                }
                isle.services.input_level = 0.0;
            }
            _ => {}
        }
    }
}

/// A process's parent, from `/proc`.
fn parent_of(pid: i32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command may hold spaces and parentheses; the fields after its last ')' are plain.
    let rest = &stat[stat.rfind(')')? + 2..];
    rest.split(' ').nth(1)?.parse().ok().filter(|p| *p > 1)
}
