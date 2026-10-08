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
    /// Notification Centre, in Control Centre's place.
    pub centre: bool,
    /// Edit mode, and the module whose menu is open in it.
    pub edit: bool,
    pub edit_menu: Option<String>,
    /// The module whose page was last left, where the keyboard returns.
    pub left_page: Option<String>,
}

impl Isle {
    /// The modules of the arrangement that have something to control here.
    pub fn panel_modules(&self) -> Vec<Module> {
        let mut out = Vec::new();
        for entry in &self.settings.control_centre.modules {
            let (id, size) = entry.split_once(':').unwrap_or((entry.as_str(), "m"));
            let size = TileSize::from_name(size).unwrap_or(TileSize::Medium);
            let Some(known) = crate::control_edit::entry(id) else {
                continue;
            };
            // A size it does not come in reads as the nearest it does.
            let size = if known.sizes.contains(&size) {
                size
            } else {
                *known.sizes.iter().rev().find(|s| s.cells().0 <= size.cells().0).unwrap_or(&known.sizes[0])
            };
            if out.iter().any(|m: &Module| m.id == id) {
                continue;
            }
            if let Some(mut module) = self.module_of(id, size) {
                module.sizes = known.sizes.to_vec();
                out.push(module);
            }
        }
        out
    }

    /// A module as it shows here, or none when its source is missing.
    pub fn module_of(&self, id: &str, size: TileSize) -> Option<Module> {
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
                    ..Default::default()
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
                    ..Default::default()
                })
            }
            "energy-mode" => self.energy_module(size),
            "batteries" => self.batteries_module(size),
            "sound" => self.sound_module(size),
            "display" => self.display_module(size),
            "wifi" => self.wifi_module(size),
            "vpn" => self.vpn_module(size),
            "bluetooth" => self.bluetooth_module(size),
            "now-playing" => self.now_playing_module(size),
            "focus" => self.focus_module(size),
            "accessibility" => self.accessibility_module(size),
            _ => None,
        }
    }

    /// Control Centre as JSON: how it is open, its modules' states and the open page's rows.
    pub fn panel_status(&self, state: &PanelState) -> serde_json::Value {
        use isle_shell::island::rows::PageRow;
        fn rows(list: &[PageRow]) -> Vec<serde_json::Value> {
            list.iter()
                .map(|row| match row {
                    PageRow::Heading(label) => serde_json::json!({ "heading": label }),
                    PageRow::Status(line) => serde_json::json!({ "status": line }),
                    PageRow::Media(card) => serde_json::json!({
                        "media": card.title, "artist": card.artist, "playing": card.playing,
                        "position": card.position,
                    }),
                    PageRow::Meter { label, level } => {
                        serde_json::json!({ "meter": label, "level": level })
                    }
                    PageRow::Choice(c) => serde_json::json!({
                        "id": c.id, "label": c.label, "detail": c.detail, "checked": c.checked,
                        "busy": c.busy, "error": c.error, "battery": c.battery,
                        "enabled": c.enabled,
                    }),
                    PageRow::Slider(r) => serde_json::json!({
                        "id": r.id, "label": r.label, "value": r.value, "muted": r.muted,
                    }),
                    PageRow::Switch { id, label, on, .. } => {
                        serde_json::json!({ "id": id, "label": label, "on": on })
                    }
                    PageRow::Group { id, label, open, rows: inner } => serde_json::json!({
                        "id": id, "label": label, "open": open, "rows": rows(inner),
                    }),
                })
                .collect()
        }
        let frame = self.panel_frame(state);
        let page = state.page.as_deref().and_then(|p| p.split('/').next());
        serde_json::json!({
            "pinned": state.pinned,
            "page": state.page,
            "centre": state.centre.then(|| {
                let c = self.centre_frame();
                serde_json::json!({
                    "groups": c.groups.iter().map(|g| serde_json::json!({
                        "app": g.app, "count": g.cards.len(), "open": g.open,
                        "ids": g.cards.iter().map(|c| c.id).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                    "earlier": c.earlier.iter().map(|g| g.cards.len()).sum::<usize>(),
                    "clear_armed": c.clear_armed,
                    "month": c.month.title,
                })
            }),
            "privacy": frame.privacy.iter().map(|(mark, app)| serde_json::json!({
                "mark": format!("{mark:?}").to_lowercase(), "app": app,
            })).collect::<Vec<_>>(),
            "modules": frame.modules.iter().map(|m| serde_json::json!({
                "id": m.id, "name": m.name, "state": m.state, "on": m.on, "glyph": m.glyph,
                "size": format!("{:?}", m.size).to_lowercase(), "slider": m.slider,
            })).collect::<Vec<_>>(),
            "edit": frame.edit.as_ref().map(|e| serde_json::json!({
                "menu": e.menu,
                "gallery": e.gallery.iter().map(|g| serde_json::json!({
                    "id": g.id, "group": g.group, "why": g.why,
                })).collect::<Vec<_>>(),
            })),
            "tray": frame.tray.iter().map(|t| serde_json::json!({
                "key": t.key, "title": t.title, "icon": t.icon, "attention": t.attention,
            })).collect::<Vec<_>>(),
            "rows": frame
                .modules
                .iter()
                .chain(frame.tray_menu.as_ref())
                .find(|m| Some(m.id.as_str()) == page)
                .and_then(|m| m.page.as_deref())
                .map(rows),
        })
    }

    /// Control Centre's content for its output.
    pub fn panel_frame(&self, state: &PanelState) -> PanelFrame {
        if state.centre {
            return PanelFrame {
                pinned: state.pinned,
                centre: Some(self.centre_frame()),
                ..Default::default()
            };
        }
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
        let modules = self.panel_modules();
        // Media's one place in the panel is Now Playing, while it is in the grid.
        let playing_module = modules.iter().any(|m| m.id == "now-playing");
        let activities = self
            .island
            .model
            .activities()
            .filter(|a| !(playing_module && a.kind == isle_shell::island::model::ActivityKind::Media))
            .cloned()
            .collect();
        let privacy: Vec<(Privacy, String)> = self.privacy_rows();
        PanelFrame {
            pinned: state.pinned,
            page: state.page.clone(),
            notices,
            activities,
            privacy,
            modules,
            tray: self.tray_icons(),
            edit: state.edit.then(|| self.edit_frame(state.edit_menu.clone())),
            focus_module: state.left_page.clone(),
            tray_menu: state.page.as_deref().and_then(|p| self.tray_menu_module(p)),
            centre: None,
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
        let (edit, edit_menu) = self
            .isle
            .island
            .panel
            .as_ref()
            .map_or((false, None), |p| (p.edit, p.edit_menu.clone()));
        self.isle.island.panel = Some(crate::control_centre::PanelState {
            output: name,
            pinned: pinned || was_pinned,
            page,
            centre: false,
            edit,
            edit_menu,
            left_page: None,
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
            self.follow_pages();
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
                // Back from an item's menu is the grid; the tray has no page of its own.
                let page = page.filter(|p| p != "tray");
                let left_page = match (&page, &panel.page) {
                    (None, Some(old)) => old.split('/').next().map(str::to_owned),
                    _ => panel.left_page.clone(),
                };
                self.isle.island.panel = Some(crate::control_centre::PanelState {
                    page,
                    left_page,
                    ..panel
                });
            }
            PanelAction::Tray { .. } | PanelAction::TrayMove { .. } => self.tray_action(action),
            PanelAction::Edit(_)
            | PanelAction::EditMenu(_)
            | PanelAction::EditRemove(_)
            | PanelAction::EditAdd(_)
            | PanelAction::EditSize { .. }
            | PanelAction::EditMove { .. } => self.edit_action(action),
            PanelAction::Switch { module, row, on } if module == "accessibility" => {
                self.accessibility_switch(&row, on)
            }
            PanelAction::Choose { ref module, .. } if module == "tray" => self.tray_action(action),
            PanelAction::Choose { module, row } if module == "keyboard-layout" => {
                if let Ok(index) = row.parse::<u32>() {
                    let keyboard = self.isle.seat.get_keyboard().unwrap();
                    keyboard.with_xkb_state(self, |mut context| {
                        context.set_layout(smithay::input::keyboard::Layout(index))
                    });
                    self.island_layout();
                }
            }
            PanelAction::Choose { module, row } if module == "centre" => self.centre_action(&row),
            PanelAction::Choose { module, row } if module == "energy-mode" => {
                self.energy_choose(row)
            }
            PanelAction::Slide { module, row, value } if module == "display" => {
                self.display_action(&row, value)
            }
            PanelAction::Toggle(ref m)
            | PanelAction::Choose { module: ref m, .. }
            | PanelAction::Expand { module: ref m, .. }
                if m == "wifi" =>
            {
                self.wifi_action(action)
            }
            PanelAction::Toggle(ref m)
            | PanelAction::Choose { module: ref m, .. }
            | PanelAction::Slide { module: ref m, .. }
                if m == "now-playing" =>
            {
                self.media_action(action)
            }
            PanelAction::Toggle(ref m) | PanelAction::Choose { module: ref m, .. }
                if m == "focus" =>
            {
                self.focus_action(action)
            }
            PanelAction::Toggle(ref m) | PanelAction::Choose { module: ref m, .. }
                if m == "bluetooth" =>
            {
                self.bluetooth_action(action)
            }
            PanelAction::Toggle(ref m) | PanelAction::Choose { module: ref m, .. }
                if m == "vpn" =>
            {
                self.vpn_action(action)
            }
            PanelAction::Toggle(ref m)
            | PanelAction::Choose { module: ref m, .. }
            | PanelAction::Slide { module: ref m, .. }
            | PanelAction::Switch { module: ref m, .. }
                if m == "sound" =>
            {
                self.sound_action(action)
            }
            PanelAction::Notice(id) => self.notice_action(&id),
            PanelAction::Activity { kind, action } if kind == "prompt" => {
                self.prompt_activity(&action)
            }
            other => info!(?other, "not built yet"),
        }
        self.follow_pages();
        self.isle.refresh_island();
    }

    /// A Control Centre action from the Test interface's JSON.
    #[cfg(feature = "test-hooks")]
    pub fn panel_test_action(&mut self, json: &str) -> Result<(), String> {
        let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let (key, args) = value
            .as_object()
            .and_then(|o| o.iter().next())
            .ok_or("one key")?;
        let s = |i: usize| args[i].as_str().unwrap_or_default().to_owned();
        let action = match key.as_str() {
            "page" => PanelAction::Page(args.as_str().map(str::to_owned)),
            "toggle" => PanelAction::Toggle(args.as_str().unwrap_or_default().into()),
            "choose" => PanelAction::Choose { module: s(0), row: s(1) },
            "expand" => PanelAction::Expand { module: s(0), row: s(1) },
            "slide" => PanelAction::Slide {
                module: s(0),
                row: s(1),
                value: args[2].as_f64().ok_or("a value")? as f32,
            },
            "tray" => PanelAction::Tray {
                key: s(0),
                press: match s(1).as_str() {
                    "primary" => isle_shell::island::panel::TrayPress::Primary,
                    "secondary" => isle_shell::island::panel::TrayPress::Secondary,
                    "middle" => isle_shell::island::panel::TrayPress::Middle,
                    _ => isle_shell::island::panel::TrayPress::Scroll(
                        args[2].as_i64().unwrap_or(1) as i32,
                    ),
                },
            },
            "edit" => PanelAction::Edit(args.as_bool().ok_or("on or off")?),
            "edit-menu" => PanelAction::EditMenu(args.as_str().map(str::to_owned)),
            "edit-remove" => PanelAction::EditRemove(args.as_str().unwrap_or_default().into()),
            "edit-add" => PanelAction::EditAdd(args.as_str().unwrap_or_default().into()),
            "edit-size" => PanelAction::EditSize {
                id: s(0),
                size: TileSize::from_name(&s(1)).ok_or("a size")?,
            },
            "edit-move" => PanelAction::EditMove {
                id: s(0),
                to: args[1].as_u64().ok_or("a place")? as usize,
            },
            "tray-move" => {
                self.tray_reorder(&s(0), args[1].as_u64().ok_or("a place")? as usize);
                return Ok(());
            }
            "switch" => PanelAction::Switch {
                module: s(0),
                row: s(1),
                on: args[2].as_bool().ok_or("on or off")?,
            },
            other => return Err(format!("no action {other}")),
        };
        self.panel_action(action);
        Ok(())
    }

    pub fn notice_action(&mut self, id: &str) {
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
