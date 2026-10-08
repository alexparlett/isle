//! Brightness: each screen's, set from the Display module, its page and the brightness keys,
//! shown at once and written behind, and the keyboard backlight's OSD.

use std::{collections::BTreeMap, time::Duration};

use isle_brightness::{Control, Level};
use isle_shell::island::{
    grid::TileSize,
    panel::Module,
    rows::{PageRow, SliderRow},
};
use smithay::output::Output;
use tokio::sync::watch;
use tracing::debug;

use crate::state::{Isle, State};

/// One brightness key's step: macOS's sixteen.
const STEP: f32 = 1.0 / 16.0;
/// The least time between two writes to one screen.
const WRITE_GAP: Duration = Duration::from_millis(200);

pub struct Screen {
    pub control: Control,
    /// 0 to 1: what was last read, or asked for since.
    pub level: Option<f32>,
    pub max: u32,
    writer: Option<watch::Sender<u32>>,
}

pub type Screens = BTreeMap<String, Screen>;

/// The name shown for a screen: its monitor's model, else its connector.
fn monitor_name(output: &Output) -> String {
    let model = output.physical_properties().model;
    if model.is_empty() || model == "Unknown" {
        output.name()
    } else {
        model
    }
}

impl Isle {
    /// Finds again how each screen's brightness is set, and reads it, off the loop.
    pub fn discover_brightness(&mut self) {
        let connectors: Vec<String> = self.space.outputs().map(|o| o.name()).collect();
        self.run_service(
            async move {
                Ok(tokio::task::spawn_blocking(move || {
                    isle_brightness::discover(&connectors)
                        .into_iter()
                        .map(|(connector, control)| {
                            let level = match &control {
                                Control::Ddc(bus) => isle_brightness::ddc_get(bus),
                                Control::Backlight(name) => isle_brightness::backlight_get(name),
                            };
                            (connector, control, level.ok())
                        })
                        .collect::<Vec<_>>()
                })
                .await?)
            },
            |state, found| {
                if let Ok(found) = found {
                    state.isle.brightness_found(found);
                }
            },
        );
    }

    #[cfg(feature = "test-hooks")]
    pub fn fake_screen(&mut self, connector: String, current: u32, max: u32) {
        let mut found: Vec<(String, Control, Option<Level>)> = self
            .services
            .screens
            .iter()
            .map(|(c, s)| (c.clone(), s.control.clone(), None))
            .collect();
        found.push((
            connector,
            Control::Backlight("test".into()),
            Some(Level { current, max }),
        ));
        self.brightness_found(found);
    }

    fn brightness_found(&mut self, found: Vec<(String, Control, Option<Level>)>) {
        let mut screens = Screens::new();
        for (connector, control, level) in found {
            let kept = self
                .services
                .screens
                .remove(&connector)
                .filter(|s| s.control == control);
            screens.insert(
                connector,
                Screen {
                    level: level
                        .map(Level::share)
                        .or(kept.as_ref().and_then(|s| s.level)),
                    max: level.map_or(kept.as_ref().map_or(0, |s| s.max), |l| l.max),
                    writer: kept.and_then(|s| s.writer),
                    control,
                },
            );
        }
        debug!(screens = screens.len(), "brightness found");
        self.services.screens = screens;
        self.refresh_island();
    }

    /// Asks for `connector`'s screen at `share`: shown now, written when the bus is free.
    pub fn set_brightness(&mut self, connector: &str, share: f32) {
        let share = share.clamp(0.0, 1.0);
        let (Some(runtime), Some(screen)) = (
            self.worker.clone(),
            self.services.screens.get_mut(connector),
        ) else {
            return;
        };
        if screen.max == 0 {
            return;
        }
        screen.level = Some(share);
        let raw = (share * screen.max as f32).round() as u32;
        if let Some(writer) = &screen.writer
            && writer.send(raw).is_ok()
        {
            return;
        }
        let (writer, mut wanted) = watch::channel(raw);
        screen.writer = Some(writer);
        let control = screen.control.clone();
        runtime.spawn(async move {
            loop {
                let value = *wanted.borrow_and_update();
                let control = control.clone();
                let result = match control {
                    Control::Ddc(bus) => {
                        tokio::task::spawn_blocking(move || isle_brightness::ddc_set(&bus, value))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string().into()))
                    }
                    Control::Backlight(name) => isle_brightness::backlight_set(&name, value).await,
                };
                if let Err(err) = result {
                    debug!("brightness: {err}");
                }
                tokio::time::sleep(WRITE_GAP).await;
                if wanted.changed().await.is_err() {
                    return;
                }
            }
        });
        self.refresh_island();
    }

    pub fn display_module(&self, size: TileSize) -> Option<Module> {
        if self.services.screens.is_empty() {
            return None;
        }
        let panel_output = self.island.panel.as_ref().map(|p| p.output.clone());
        let outputs: Vec<Output> = self.space.outputs().cloned().collect();
        let mut rows: Vec<PageRow> = Vec::new();
        // The screen the panel is on first.
        let mut ordered: Vec<&Output> = outputs.iter().collect();
        ordered.sort_by_key(|o| Some(o.name()) != panel_output);
        for output in ordered {
            let name = monitor_name(output);
            match self.services.screens.get(&output.name()) {
                Some(screen) if screen.level.is_some() => rows.push(PageRow::Slider(SliderRow {
                    id: output.name(),
                    label: name,
                    value: screen.level.unwrap_or(0.0),
                    enabled: true,
                    ..Default::default()
                })),
                _ => rows.push(PageRow::Status(format!(
                    "{name}: Brightness can't be set on this display"
                ))),
            }
        }
        let holders = self.idle_holders();
        if !holders.is_empty() {
            rows.push(PageRow::Status(format!(
                "Kept awake by {}",
                holders.join(", ")
            )));
        }
        let here = panel_output
            .and_then(|o| self.services.screens.get(&o))
            .or_else(|| self.services.screens.values().next());
        let level = here.and_then(|s| s.level);
        Some(Module {
            id: "display".into(),
            size,
            name: "Display".into(),
            state: String::new(),
            glyph: if level.unwrap_or(1.0) < 0.5 {
                "sun-dim"
            } else {
                "sun"
            }
            .into(),
            page: Some(rows),
            pane: "Displays".into(),
            slider: level,
            ..Default::default()
        })
    }
}

impl State {
    /// The brightness keys: the main screen's brightness by a step, with its OSD there.
    pub fn brightness_key(&mut self, up: bool) {
        let Some(output) = self.isle.main_output() else {
            return;
        };
        let connector = output.name();
        let Some(level) = self
            .isle
            .services
            .screens
            .get(&connector)
            .and_then(|s| s.level)
        else {
            return;
        };
        let level = (level + if up { STEP } else { -STEP }).clamp(0.0, 1.0);
        self.isle.set_brightness(&connector, level);
        let symbol = if level < 0.5 { "sun-dim" } else { "sun" };
        let name = monitor_name(&output);
        self.isle
            .osd_level(symbol, &name, level, false, Some(connector));
    }

    /// Display's slider and its page's rows.
    pub fn display_action(&mut self, row: &str, value: f32) {
        let connector = if row.is_empty() {
            self.isle.island.panel.as_ref().map(|p| p.output.clone())
        } else {
            Some(row.to_owned())
        };
        if let Some(connector) = connector {
            self.isle.set_brightness(&connector, value);
        }
    }
}
