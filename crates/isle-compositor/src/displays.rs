//! A display change Settings asks for, applied at once and held until kept: the deadline,
//! the asking client leaving the bus or Revert put the configuration it replaced back
//! (`p13-settings.md` choice 19; `settings-2026.md` §6).

use std::collections::BTreeMap;

use isle_settings::{Value, schema::Output as OutputConfig};
use serde::Deserialize;
use smithay::reexports::calloop::{
    RegistrationToken,
    timer::{TimeoutAction, Timer},
};
use tracing::info;

use crate::{
    state::{Isle, State},
    worker::Event,
};

/// One display's fields a change sets; those absent stay.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Edit {
    /// `WIDTHxHEIGHT@HZ`, or empty for the preferred size at its fastest rate.
    pub mode: Option<String>,
    pub scale: Option<f64>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub transform: Option<String>,
    pub enabled: Option<bool>,
}

/// A change in force and not yet kept.
pub struct Pending {
    pub id: u64,
    /// What each changed display now has.
    pub applied: BTreeMap<String, OutputConfig>,
    deadline: Option<RegistrationToken>,
}

fn relit(a: &OutputConfig, b: &OutputConfig) -> bool {
    a.mode != b.mode || a.transform != b.transform || a.enabled != b.enabled
}

impl Isle {
    /// The configuration a display has now: a change not yet kept, else its settings.
    pub fn effective_output(&self, identity: &str) -> OutputConfig {
        self.display_change
            .as_ref()
            .and_then(|p| p.applied.get(identity))
            .or_else(|| self.settings.output.get(identity))
            .cloned()
            .unwrap_or_default()
    }
}

impl State {
    /// Every display with a screen on it, lit or turned off.
    pub fn display_infos(&self) -> Vec<isle_ipc::DisplayInfo> {
        let connected = self.connected_displays();
        let mut infos: Vec<isle_ipc::DisplayInfo> = self
            .isle
            .space
            .outputs()
            .map(|output| {
                let identity = Isle::output_identity(output);
                let config = self.isle.effective_output(&identity);
                let properties = output.physical_properties();
                let mode = output.current_mode();
                let location = self
                    .isle
                    .space
                    .output_geometry(output)
                    .map(|g| g.loc)
                    .unwrap_or_default();
                let known = connected.iter().find(|c| c.0 == identity);
                isle_ipc::DisplayInfo {
                    name: output.name(),
                    make: properties.make,
                    model: properties.model,
                    serial: properties.serial_number,
                    width: mode.map_or(0, |m| m.size.w),
                    height: mode.map_or(0, |m| m.size.h),
                    refresh: mode.map_or(0, |m| m.refresh),
                    scale: output.current_scale().fractional_scale(),
                    x: location.x,
                    y: location.y,
                    vrr_capability: output
                        .user_data()
                        .get::<crate::present::VrrCapability>()
                        .map_or("none", |c| c.name())
                        .into(),
                    vrr_setting: config.vrr.as_str().into(),
                    vrr_on: crate::redraw::OutputState::of(output).borrow().vrr,
                    enabled: true,
                    transform: config.transform.as_str().into(),
                    modes: known.map(|c| c.2.clone()).unwrap_or_else(|| {
                        mode.map(|m| isle_ipc::DisplayMode {
                            width: m.size.w,
                            height: m.size.h,
                            refresh: m.refresh,
                        })
                        .into_iter()
                        .collect()
                    }),
                    width_mm: properties.size.w,
                    height_mm: properties.size.h,
                    min_hz: known.map_or(0, |c| c.4.0),
                    max_hz: known.map_or(0, |c| c.4.1),
                }
            })
            .collect();
        for (identity, name, modes, size_mm, hz) in connected {
            if infos.iter().any(|i| i.name == name) {
                continue;
            }
            let mut words = identity.splitn(3, ' ');
            let config = self.isle.effective_output(&identity);
            infos.push(isle_ipc::DisplayInfo {
                name,
                make: words.next().unwrap_or_default().into(),
                model: words.next().unwrap_or_default().into(),
                serial: words.next().unwrap_or_default().into(),
                width: 0,
                height: 0,
                refresh: 0,
                scale: config.scale,
                x: 0,
                y: 0,
                vrr_capability: "none".into(),
                vrr_setting: config.vrr.as_str().into(),
                vrr_on: false,
                enabled: false,
                transform: config.transform.as_str().into(),
                modes,
                width_mm: size_mm.0 as i32,
                height_mm: size_mm.1 as i32,
                min_hz: hz.0,
                max_hz: hz.1,
            });
        }
        infos
    }

    /// Applies `edits` now and holds them until kept; a second change before the first is
    /// kept replaces it and keeps its way back. Returns the change's id.
    pub fn configure_displays(&mut self, edits: BTreeMap<String, Edit>) -> Result<u64, String> {
        let known: Vec<String> = self
            .display_infos()
            .into_iter()
            .map(|i| format!("{} {} {}", i.make, i.model, i.serial))
            .collect();
        let mut applied = self
            .isle
            .display_change
            .take()
            .map(|p| {
                if let Some(token) = p.deadline {
                    self.isle.loop_handle.remove(token);
                }
                p.applied
            })
            .unwrap_or_default();
        let before: BTreeMap<String, OutputConfig> = known
            .iter()
            .map(|id| (id.clone(), self.isle.effective_output(id)))
            .collect();
        for (identity, edit) in &edits {
            let Some(current) = before.get(identity) else {
                return Err(format!("no display is {identity}"));
            };
            let mut next = current.clone();
            if let Some(mode) = &edit.mode {
                next.mode = Some(mode.clone()).filter(|m| !m.is_empty());
            }
            if let Some(scale) = edit.scale {
                if !(0.5..=4.0).contains(&scale) {
                    return Err("a scale is from 0.5 to 4".into());
                }
                next.scale = scale;
            }
            next.x = edit.x.or(next.x);
            next.y = edit.y.or(next.y);
            if let Some(word) = &edit.transform {
                next.transform = isle_settings::Field::from_value(Value::Str(word.clone()))
                    .map_err(|_| format!("{word} is not a rotation"))?;
            }
            if let Some(enabled) = edit.enabled {
                next.enabled = enabled;
            }
            applied.insert(identity.clone(), next);
        }
        let on = known
            .iter()
            .filter(|id| applied.get(*id).map_or(before[*id].enabled, |c| c.enabled))
            .count();
        if on == 0 {
            return Err("the last display that is on cannot be turned off".into());
        }

        self.isle.next_display_change += 1;
        let id = self.isle.next_display_change;
        let ms = self.isle.shell.theme().tokens.timing_display_revert_ms;
        let deadline = self
            .isle
            .loop_handle
            .insert_source(
                Timer::from_duration(std::time::Duration::from_millis(ms as u64)),
                move |_, _, state| {
                    info!(id, "a display change was not kept in time");
                    state.revert_displays(id);
                    TimeoutAction::Drop
                },
            )
            .ok();
        self.isle.display_change = Some(Pending {
            id,
            applied,
            deadline,
        });
        info!(id, ?edits, "displays changed until kept");
        self.apply_displays(&before);
        Ok(id)
    }

    /// The keys that make change `id` the settings, for the worker to write; the change
    /// stays in force until `kept`.
    pub fn display_writes(&self, id: u64) -> Option<Vec<(String, Option<Value>)>> {
        let pending = self.isle.display_change.as_ref().filter(|p| p.id == id)?;
        let mut writes = Vec::new();
        for (identity, config) in &pending.applied {
            let key = |field: &str| format!("output[{identity}].{field}");
            let saved = self.isle.settings.output.get(identity).cloned().unwrap_or_default();
            if config.mode != saved.mode {
                writes.push((key("mode"), config.mode.clone().map(Value::Str)));
            }
            if config.scale != saved.scale {
                writes.push((key("scale"), Some(Value::Float(config.scale))));
            }
            if config.x != saved.x || config.y != saved.y {
                writes.push((key("x"), config.x.map(|v| Value::Int(v.into()))));
                writes.push((key("y"), config.y.map(|v| Value::Int(v.into()))));
            }
            if config.transform != saved.transform {
                writes.push((key("transform"), Some(Value::Str(config.transform.as_str().into()))));
            }
            if config.enabled != saved.enabled {
                writes.push((key("enabled"), Some(Value::Bool(config.enabled))));
            }
        }
        Some(writes)
    }

    /// Change `id` is in the settings now; nothing is held for it.
    pub fn displays_kept(&mut self, id: u64) {
        if let Some(pending) = self.isle.display_change.take_if(|p| p.id == id) {
            if let Some(token) = pending.deadline {
                self.isle.loop_handle.remove(token);
            }
            info!(id, "display change kept");
            self.isle.notify(Event::DisplaysChanged);
        }
    }

    /// Puts back what change `id` replaced; false when it is not the change in force.
    pub fn revert_displays(&mut self, id: u64) -> bool {
        let Some(pending) = self.isle.display_change.take_if(|p| p.id == id) else {
            return false;
        };
        if let Some(token) = pending.deadline {
            self.isle.loop_handle.remove(token);
        }
        info!(id, "display change reverted");
        self.apply_displays(&pending.applied);
        true
    }

    /// Lights a display again from its effective configuration.
    fn relight_display(&mut self, identity: &str) {
        if matches!(self.backend, crate::backend::Backend::Headless(_)) {
            self.relight_headless(identity);
        } else {
            self.relight(identity);
        }
    }

    /// Brings each display from `before` to its effective configuration: relit for a mode,
    /// rotation or turning on or off, placed and scaled for the rest.
    fn apply_displays(&mut self, before: &BTreeMap<String, OutputConfig>) {
        for (identity, was) in before {
            let now = self.isle.effective_output(identity);
            if relit(was, &now) {
                self.relight_display(identity);
            }
        }
        let outputs: Vec<_> = self.isle.space.outputs().cloned().collect();
        for output in outputs {
            self.isle.apply_output_settings(&output);
        }
        self.isle.notify(Event::DisplaysChanged);
    }
}
