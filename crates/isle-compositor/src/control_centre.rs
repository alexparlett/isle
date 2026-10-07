//! Control Centre: the island grown into a panel on one output, peeked by hover or pinned
//! with the keyboard, its notices and the modules built so far (`p6-island-and-menus.md`
//! choice 10).

use isle_shell::island::{
    grid::TileSize,
    model::Privacy,
    panel::{Module, Notice, PanelAction, PanelFrame},
    rows::{Choice, PageRow},
};
use smithay::output::Output;
use tracing::{info, warn};

use crate::{
    focus::Focus,
    state::{Isle, State},
};

/// Which output's island is Control Centre, and how it was opened.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelState {
    pub output: String,
    pub pinned: bool,
    pub page: Option<String>,
}

impl Isle {
    /// The modules of the arrangement that have something to control here.
    fn modules(&self) -> Vec<Module> {
        let mut out = Vec::new();
        for entry in &self.settings.control_centre.modules {
            let (id, size) = entry.split_once(':').unwrap_or((entry.as_str(), "m"));
            let size = TileSize::from_name(size).unwrap_or(TileSize::Medium);
            if let Some(module) = self.module(id, size) {
                out.push(module);
            }
        }
        out
    }

    fn module(&self, id: &str, size: TileSize) -> Option<Module> {
        match id {
            "dark-mode" => {
                let dark = self.shell.theme().appearance.mode == isle_tokens::Mode::Dark;
                Some(Module {
                    id: id.into(),
                    size,
                    name: "Dark Mode".into(),
                    state: if dark { "On".into() } else { "Off".into() },
                    glyph: "circle-half".into(),
                    on: dark,
                    toggles: true,
                    page: None,
                    page_title: None,
                    pane: "Appearance".into(),
                })
            }
            "keyboard-layout" => {
                let (layouts, active) = self.island.layouts.clone();
                Some(Module {
                    id: id.into(),
                    size,
                    name: "Keyboard".into(),
                    state: layouts.get(active).cloned().unwrap_or_default(),
                    glyph: "keyboard".into(),
                    on: false,
                    toggles: false,
                    page: Some(
                        layouts
                            .iter()
                            .enumerate()
                            .map(|(i, name)| {
                                PageRow::Choice(Choice {
                                    id: i.to_string(),
                                    label: name.clone(),
                                    checked: i == active,
                                    enabled: true,
                                    ..Default::default()
                                })
                            })
                            .collect(),
                    ),
                    page_title: None,
                    pane: "Keyboard".into(),
                })
            }
            // The rest come with their services (P8).
            _ => None,
        }
    }

    /// Control Centre's content for its output.
    pub fn panel_frame(&self, state: &PanelState) -> PanelFrame {
        let mut notices = Vec::new();
        if self.safe_start {
            notices.push(Notice::ConfigError {
                message: String::new(),
                file: String::new(),
                line: 0,
                column: 0,
                safe_start: true,
            });
        } else if let Some((file, line, column, message)) = &self.settings_error {
            notices.push(Notice::ConfigError {
                message: message.clone(),
                file: file.clone(),
                line: *line,
                column: *column,
                safe_start: false,
            });
        } else if let Some(err) = &self.tokens_error {
            notices.push(Notice::ConfigError {
                message: err.message.clone(),
                file: err.file.display().to_string(),
                line: err.line as u32,
                column: err.column as u32,
                safe_start: false,
            });
        }
        if let Some(app) = &self.island.inhibiting {
            notices.push(Notice::Inhibited {
                app: self.app_name(app),
                chord: "⌘⎋".into(),
            });
        }
        let activities = self.island.model.activities().cloned().collect();
        let privacy: Vec<(Privacy, String)> = Vec::new();
        PanelFrame {
            pinned: state.pinned,
            page: state.page.clone(),
            notices,
            activities,
            privacy,
            modules: self.modules(),
            body: 0.0,
        }
    }
}

impl State {
    /// Opens Control Centre on `output`: a peek has no keyboard, a pin takes it.
    pub fn open_control_centre(&mut self, output: &Output, pinned: bool, page: Option<String>) {
        let name = output.name();
        if let Some(open) = &self.isle.island.panel
            && open.output != name
        {
            self.close_control_centre();
        }
        // Pinned, it is the screen's one panel; a peek leaves the bar down.
        if pinned {
            self.hide_bar();
        }
        let was_pinned = self.isle.island.panel.as_ref().is_some_and(|p| p.pinned);
        // Pinning a panel already open keeps the page it shows.
        let page = page.or_else(|| self.isle.island.panel.as_ref().and_then(|p| p.page.clone()));
        self.isle.island.panel = Some(crate::control_centre::PanelState {
            output: name,
            pinned: pinned || was_pinned,
            page,
        });
        if pinned && !was_pinned {
            self.focus_island(output);
        }
        self.isle.refresh_island();
    }

    pub fn close_control_centre(&mut self) {
        if let Some(panel) = self.isle.island.panel.take() {
            if panel.pinned {
                self.unfocus_island();
            }
            self.isle.refresh_island();
        }
    }

    /// Ctrl+F8: Control Centre pinned on the focused output, or closed again.
    pub fn control_centre_chord(&mut self) {
        if self.isle.island.panel.as_ref().is_some_and(|p| p.pinned) {
            self.close_control_centre();
        } else if let Some(output) = self
            .isle
            .output_with_focus()
            .or_else(|| self.isle.active_output())
        {
            self.open_control_centre(&output, true, None);
        }
    }

    fn focus_island(&mut self, output: &Output) {
        let Some(focus) = self.isle.shell.instance_focus("island", output) else {
            return;
        };
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let previous = keyboard.current_focus();
        if previous == Some(Focus::Shell(focus.clone())) {
            return;
        }
        self.isle
            .shell_keyboard_return
            .push((focus.clone(), previous));
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        self.focus_keyboard(Some(Focus::Shell(focus)), serial);
    }

    /// Gives the keyboard back to what had it before the panel was pinned.
    fn unfocus_island(&mut self) {
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let Some(Focus::Shell(focus)) = keyboard.current_focus() else {
            return;
        };
        if self.isle.shell.name_of(&focus) != Some("island") {
            return;
        }
        match self
            .isle
            .shell_keyboard_return
            .iter()
            .position(|(f, _)| *f == focus)
            .map(|i| self.isle.shell_keyboard_return.remove(i).1)
        {
            Some(Some(previous)) if smithay::utils::IsAlive::alive(&previous) => {
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                self.focus_keyboard(Some(previous), serial);
            }
            _ => self.focus_next(),
        }
    }

    /// What a control in Control Centre asked for.
    pub fn panel_action(&mut self, action: PanelAction) {
        let Some(panel) = self.isle.island.panel.clone() else {
            return;
        };
        match action {
            PanelAction::Toggle(id) if id == "dark-mode" => {
                let dark = self.isle.shell.theme().appearance.mode == isle_tokens::Mode::Dark;
                self.isle.write_setting(
                    "appearance.mode",
                    isle_settings::Value::Str(if dark { "light" } else { "dark" }.into()),
                );
            }
            PanelAction::Page(page) => {
                self.isle.island.panel = Some(crate::control_centre::PanelState { page, ..panel });
            }
            PanelAction::Choose { module, row } if module == "keyboard-layout" => {
                if let Ok(index) = row.parse::<u32>() {
                    let keyboard = self.isle.seat.get_keyboard().unwrap();
                    keyboard.with_xkb_state(self, |mut context| {
                        context.set_layout(smithay::input::keyboard::Layout(index))
                    });
                    self.island_layout();
                }
            }
            PanelAction::Notice(id) => self.notice_action(&id),
            other => info!(?other, "not built yet"),
        }
        self.isle.refresh_island();
    }

    fn notice_action(&mut self, id: &str) {
        match id {
            "take-back" => {
                self.close_control_centre();
                self.toggle_keys_back();
            }
            "use-mine" => {
                let (Some(store), Some(runtime), Some(to_loop)) = (
                    self.isle.settings_store.clone(),
                    self.isle.worker.clone(),
                    self.isle.to_loop.clone(),
                ) else {
                    return;
                };
                let events = self.isle.events.clone();
                runtime.spawn(async move {
                    let mut store = store.lock().await;
                    if let isle_settings::Reload::Applied(keys) = store.use_file() {
                        crate::settings::publish(&to_loop, &events, store.current().clone(), keys);
                    }
                    to_loop.post(|state| {
                        state.isle.safe_start = false;
                        state.island_settings_status(None);
                    });
                });
            }
            "editor" | "files" => {
                let file = self
                    .isle
                    .settings_error
                    .as_ref()
                    .map(|(file, ..)| file.clone())
                    .or_else(|| {
                        self.isle
                            .tokens_error
                            .as_ref()
                            .map(|e| e.file.display().to_string())
                    });
                let Some(file) = file else {
                    return;
                };
                self.close_control_centre();
                let target = if id == "files" {
                    std::path::Path::new(&file)
                        .parent()
                        .map(|p| p.display().to_string())
                        .unwrap_or(file)
                } else {
                    file
                };
                self.launch(crate::launch::Launch::Open(target));
            }
            other => warn!(other, "no such notice action"),
        }
    }
}

impl Isle {
    /// Writes a setting from the loop, through the store on the worker.
    pub fn write_setting(&self, key: &str, value: isle_settings::Value) {
        let (Some(store), Some(runtime), Some(to_loop)) = (
            self.settings_store.clone(),
            self.worker.clone(),
            self.to_loop.clone(),
        ) else {
            return;
        };
        let events = self.events.clone();
        let key = key.to_owned();
        runtime.spawn(async move {
            let mut store = store.lock().await;
            match store.set(&key, value) {
                Ok(keys) => {
                    crate::settings::publish(&to_loop, &events, store.current().clone(), keys)
                }
                Err(err) => warn!(key, "could not set: {err}"),
            }
        });
    }
}
