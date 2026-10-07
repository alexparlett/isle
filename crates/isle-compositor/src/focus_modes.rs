//! Focus on the loop: the model evaluated as what is in front, the time and settings change,
//! its one timer, the glyph and events, the holds it puts on banners, Game Mode's profile
//! hold, and the summary once silence ends (`p8-services.md` choices 12–14).

use std::time::Instant;

use isle_focus::{
    Active, Config, DO_NOT_DISTURB, Front, GAME_MODE, Memory, Mode, Off, Schedule, Why,
};
use isle_shell::island::{
    grid::TileSize,
    model::{Event, EventKind, Form, Glyph, GlyphKind},
    panel::{Module, PanelAction},
    rows::{Choice, Lead, PageRow},
};
use isle_state::{Domain, Loaded, StateDir};
use jiff::Zoned;
use smithay::reexports::calloop::{
    RegistrationToken,
    timer::{TimeoutAction, Timer},
};
use tracing::{info, warn};

use crate::{
    services::power::APP_ID,
    state::{Isle, State},
    state_writer::Saver,
};

/// What changed the Focus, which decides whether the island says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A schedule, an app or a game: said on the island.
    Auto,
    /// A chord: said on the island.
    Chord,
    /// Control Centre or the API: the module shows it, the island does not.
    Control,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Saved(pub Memory);

impl Domain for Saved {
    const NAME: &'static str = "focus";
    const VERSION: u32 = 1;
}

#[derive(Default)]
pub struct FocusState {
    pub mem: Memory,
    pub active: Option<Active>,
    timer: Option<RegistrationToken>,
    saver: Option<Saver<Saved>>,
    /// Isle's hold on the performance profile while Game Mode is on.
    cookie: Option<u32>,
    /// Something held banners when last looked.
    holding: bool,
    /// Game Mode held any of what was held.
    held_by_game: bool,
}

impl Isle {
    pub fn load_focus(&mut self, runtime: &tokio::runtime::Handle) {
        let dir = StateDir::new(StateDir::default_dir());
        let (saved, loaded) = dir.load::<Saved>();
        if !matches!(loaded, Loaded::Read | Loaded::Missing) {
            warn!(?loaded, "focus");
        }
        self.focus.mem = saved.0;
        self.focus.saver = Some(Saver::spawn(runtime, dir));
    }

    /// The modes from settings: Do Not Disturb and Game Mode, then the user's own.
    pub fn focus_config(&self) -> Config {
        let mut modes = isle_focus::builtin();
        for (name, rules) in &self.settings.focus_mode {
            let schedules = rules
                .schedules
                .iter()
                .filter_map(|s| {
                    Schedule::parse(s)
                        .inspect_err(|e| warn!(mode = name, "a schedule: {e}"))
                        .ok()
                })
                .collect();
            let apply = |mode: &mut Mode| {
                if let Some(symbol) = &rules.symbol {
                    mode.symbol = symbol.clone();
                }
                mode.allowed = rules.allow_apps.clone();
                mode.critical = rules.allow_critical;
                mode.schedules = schedules;
                mode.apps = rules.while_apps.clone();
            };
            match modes.iter_mut().find(|m| &m.name == name) {
                Some(mode) => apply(mode),
                None => {
                    let mut mode = Mode::new(name, "circle-dashed");
                    apply(&mut mode);
                    modes.push(mode);
                }
            }
        }
        Config {
            modes,
            game_auto: self.settings.focus.game_mode_auto,
            off_for: self.settings.focus.games_off.clone(),
        }
    }

    /// Game Mode is the Focus in force.
    pub fn game_mode_active(&self) -> bool {
        self.focus
            .active
            .as_ref()
            .is_some_and(|a| a.mode == GAME_MODE)
    }

    /// The Focus that holds banners: its name, the apps it lets through, critical's pass.
    pub fn focus_hold_now(&self) -> Option<(String, Vec<String>, bool)> {
        let active = self.focus.active.as_ref()?;
        let cfg = self.focus_config();
        let mode = cfg.mode(&active.mode)?;
        Some((mode.name.clone(), mode.allowed.clone(), mode.critical))
    }

    fn focus_glyph(&self) -> Option<Glyph> {
        let active = self.focus.active.as_ref()?;
        let symbol = self
            .focus_config()
            .mode(&active.mode)
            .map_or("moon".to_owned(), |m| m.symbol.clone());
        Some(Glyph {
            active: true,
            symbol,
            fill: true,
            alt: active.mode.clone(),
            ..Default::default()
        })
    }

    /// The second line that says how a Focus ends or why it is on.
    fn focus_reason(&self, active: &Active) -> String {
        let time = |secs: i64| {
            jiff::Timestamp::from_second(secs)
                .map(|t| {
                    t.to_zoned(jiff::tz::TimeZone::system())
                        .strftime("%H:%M")
                        .to_string()
                })
                .unwrap_or_default()
        };
        match &active.why {
            Why::Manual(None) => String::new(),
            Why::Manual(Some(until)) => {
                let left = (until - Zoned::now().timestamp().as_second()) / 60;
                if left < 60 {
                    format!("For {} min", left.max(1))
                } else {
                    format!("Until {}", time(*until))
                }
            }
            Why::Schedule(until) => format!("Until {}", time(*until)),
            Why::App(app) | Why::Game(app) => self.app_name(app),
        }
    }

    pub fn focus_status(&self) -> String {
        serde_json::json!({
            "active": self.focus.active.as_ref().map(|a| &a.mode),
            "why": self.focus.active.as_ref().map(|a| &a.why),
            "modes": self.focus_config().modes.iter().map(|m| &m.name).collect::<Vec<_>>(),
        })
        .to_string()
    }

    pub fn focus_module(&self, size: TileSize) -> Option<Module> {
        let cfg = self.focus_config();
        let active = self.focus.active.clone();
        let last = self
            .focus
            .mem
            .last
            .clone()
            .unwrap_or_else(|| DO_NOT_DISTURB.to_owned());
        let shown = active.as_ref().map_or(last.as_str(), |a| a.mode.as_str());
        let symbol = cfg
            .mode(shown)
            .map_or("moon".to_owned(), |m| m.symbol.clone());
        let page = self
            .island
            .panel
            .as_ref()
            .and_then(|p| p.page.clone())
            .filter(|p| p.starts_with("focus/"));
        let (rows, title) = match page.as_deref().and_then(|p| p.strip_prefix("focus/")) {
            Some(mode) => {
                let evening = Zoned::now().hour() < 18;
                let rows = vec![
                    PageRow::Choice(Choice {
                        id: format!("for:{mode}:60"),
                        label: "For 1 hour".into(),
                        enabled: true,
                        ..Default::default()
                    }),
                    PageRow::Choice(Choice {
                        id: format!(
                            "until:{mode}:{}",
                            if evening { "evening" } else { "morning" }
                        ),
                        label: if evening {
                            "Until this evening"
                        } else {
                            "Until tomorrow morning"
                        }
                        .into(),
                        enabled: true,
                        ..Default::default()
                    }),
                ];
                (rows, Some(mode.to_owned()))
            }
            None => {
                let rows = cfg
                    .modes
                    .iter()
                    .map(|m| {
                        let on = active.as_ref().is_some_and(|a| a.mode == m.name);
                        let game_off = m.name == GAME_MODE
                            && active
                                .as_ref()
                                .is_some_and(|a| matches!(a.why, Why::Game(_)));
                        PageRow::Choice(Choice {
                            id: m.name.clone(),
                            label: m.name.clone(),
                            lead: Lead::Glyph(m.symbol.clone()),
                            checked: on,
                            detail: active
                                .as_ref()
                                .filter(|a| a.mode == m.name)
                                .map(|a| self.focus_reason(a))
                                .filter(|r| !r.is_empty()),
                            trailing: game_off.then(|| "x".into()),
                            enabled: true,
                            ..Default::default()
                        })
                    })
                    .chain(std::iter::once(PageRow::Choice(Choice {
                        id: "durations".into(),
                        label: format!("Durations for {last}"),
                        sub_page: true,
                        enabled: true,
                        ..Default::default()
                    })))
                    .collect();
                (rows, None)
            }
        };
        let state = match &active {
            Some(a) => {
                let reason = self.focus_reason(a);
                if reason.is_empty() {
                    a.mode.clone()
                } else {
                    format!("{}, {reason}", a.mode)
                }
            }
            None => "Off".into(),
        };
        Some(Module {
            id: "focus".into(),
            size,
            name: active.as_ref().map_or("Focus".into(), |a| a.mode.clone()),
            state,
            glyph: symbol,
            on: active.is_some(),
            toggles: true,
            page: Some(rows),
            page_title: title,
            pane: "Focus".into(),
            plain_head: true,
            ..Default::default()
        })
    }
}

impl State {
    /// What is in front, for Focus: the focused window's app and whether it is a game.
    fn front(&self) -> Front {
        let record = self.focused_window().and_then(|id| self.isle.wm.get(id));
        Front {
            app: record.map(|r| r.app.clone()),
            game: record.is_some_and(|r| r.game),
        }
    }

    /// Works out the Focus in force and acts on a change.
    pub fn focus_evaluate(&mut self, origin: Origin) {
        let cfg = self.isle.focus_config();
        let front = self.front();
        let now = Zoned::now();
        let before = self.isle.focus.active.clone();
        let active = isle_focus::evaluate(&cfg, &mut self.isle.focus.mem, &now, &front);
        let changed = before.as_ref().map(|a| &a.mode) != active.as_ref().map(|a| &a.mode);
        self.isle.focus.active = active.clone();
        if changed {
            info!(?active, ?origin, "focus");
            self.focus_changed(before, origin);
        }
        self.arm_focus_timer(&cfg, &now);
        if let Some(saver) = &self.isle.focus.saver {
            saver.save(Saved(self.isle.focus.mem.clone()));
        }
        self.silence_check();
    }

    fn focus_changed(&mut self, before: Option<Active>, origin: Origin) {
        let glyph = self.isle.focus_glyph();
        self.isle.island.model.set_glyph(GlyphKind::Focus, glyph);
        let after = self.isle.focus.active.clone();
        if origin != Origin::Control {
            let (name, on) = match (&after, &before) {
                (Some(a), _) => (a.mode.clone(), true),
                (None, Some(b)) => (b.mode.clone(), false),
                (None, None) => return,
            };
            let symbol = self
                .isle
                .focus_config()
                .mode(&name)
                .map_or("moon".to_owned(), |m| m.symbol.clone());
            self.isle.island.model.post(
                Event {
                    kind: EventKind::Mode,
                    form: Form::Line {
                        symbol,
                        fill: on,
                        text: format!("{name} {}", if on { "on" } else { "off" }),
                    },
                    output: None,
                    app: None,
                    urgent: false,
                },
                Instant::now(),
            );
        }
        // Game Mode holds the performance profile, and gives it back on leaving.
        let game = self.isle.game_mode_active();
        let was_game = before.as_ref().is_some_and(|b| b.mode == GAME_MODE);
        if game && !was_game {
            self.isle.run_service(
                isle_power::hold_profile("performance", "Game Mode", APP_ID),
                |state, cookie| {
                    if let Ok(cookie) = cookie {
                        state.isle.focus.cookie = Some(cookie);
                    }
                },
            );
        } else if was_game
            && !game
            && let Some(cookie) = self.isle.focus.cookie.take()
        {
            self.isle
                .run_service(isle_power::release_profile(cookie), |_, _| {});
        }
        self.isle.notify(crate::worker::Event::FocusChanged);
        self.isle.refresh_island();
    }

    fn arm_focus_timer(&mut self, cfg: &Config, now: &Zoned) {
        if let Some(token) = self.isle.focus.timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let Some(next) = isle_focus::next_change(cfg, &self.isle.focus.mem, now) else {
            return;
        };
        let wait = next.duration_since(now.timestamp());
        let wait = std::time::Duration::try_from(wait).unwrap_or_default()
            + std::time::Duration::from_millis(5);
        self.isle.focus.timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(wait), |_, _, state| {
                state.isle.focus.timer = None;
                state.focus_evaluate(Origin::Auto);
                TimeoutAction::Drop
            })
            .ok();
    }

    /// Turns `mode` on by hand for `minutes`, or until turned off; or the mode in force off.
    pub fn focus_set(&mut self, mode: Option<&str>, minutes: Option<i64>, origin: Origin) {
        match mode {
            Some(mode) => {
                let until = minutes.map(|m| Zoned::now().timestamp().as_second() + m * 60);
                isle_focus::turn_on(&mut self.isle.focus.mem, mode, until);
            }
            None => {
                let cfg = self.isle.focus_config();
                if let Some(active) = self.isle.focus.active.clone()
                    && let Off::KeepOffFor(game) =
                        isle_focus::turn_off(&cfg, &mut self.isle.focus.mem, &active, &Zoned::now())
                {
                    let mut games = self.isle.settings.focus.games_off.clone();
                    games.push(game);
                    self.isle
                        .write_setting("focus.games-off", isle_settings::Value::List(games));
                    // In force before the write comes back.
                    self.isle
                        .settings
                        .focus
                        .games_off
                        .push(self.front().app.unwrap_or_default());
                }
            }
        }
        self.focus_evaluate(origin);
    }

    /// A Focus chord: the mode on, or off when it is the one in force.
    pub fn focus_chord(&mut self, mode: &str) {
        let on = self
            .isle
            .focus
            .active
            .as_ref()
            .is_some_and(|a| a.mode == mode);
        self.focus_set((!on).then_some(mode), None, Origin::Chord);
    }

    pub fn focus_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::Toggle(_) => {
                let mode = self
                    .isle
                    .focus
                    .mem
                    .last
                    .clone()
                    .unwrap_or_else(|| DO_NOT_DISTURB.to_owned());
                let on = self.isle.focus.active.is_some();
                self.focus_set((!on).then_some(mode.as_str()), None, Origin::Control);
            }
            PanelAction::Choose { row, .. } => {
                if row == "durations" {
                    let last = self
                        .isle
                        .focus
                        .mem
                        .last
                        .clone()
                        .unwrap_or_else(|| DO_NOT_DISTURB.to_owned());
                    self.panel_action(PanelAction::Page(Some(format!("focus/{last}"))));
                    return;
                }
                if let Some(rest) = row.strip_prefix("for:") {
                    if let Some((mode, minutes)) = rest.rsplit_once(':') {
                        let minutes = minutes.parse().ok();
                        self.focus_set(Some(mode), minutes, Origin::Control);
                    }
                    self.panel_action(PanelAction::Page(Some("focus".into())));
                    return;
                }
                if let Some(rest) = row.strip_prefix("until:") {
                    if let Some((mode, when)) = rest.rsplit_once(':') {
                        let now = Zoned::now();
                        let target = if when == "evening" {
                            now.date().to_datetime(jiff::civil::time(18, 0, 0, 0))
                        } else {
                            now.date()
                                .tomorrow()
                                .unwrap_or(now.date())
                                .to_datetime(jiff::civil::time(7, 0, 0, 0))
                        };
                        let minutes = target
                            .to_zoned(now.time_zone().clone())
                            .map(|t| (t.timestamp().as_second() - now.timestamp().as_second()) / 60)
                            .ok();
                        self.focus_set(Some(mode), minutes, Origin::Control);
                    }
                    self.panel_action(PanelAction::Page(Some("focus".into())));
                    return;
                }
                let on = self
                    .isle
                    .focus
                    .active
                    .as_ref()
                    .is_some_and(|a| a.mode == row);
                self.focus_set((!on).then_some(row.as_str()), None, Origin::Control);
            }
            _ => {}
        }
    }

    /// Silence has ended: one held notification shows itself, more are summarised.
    pub fn silence_check(&mut self) {
        let holds = self.isle.holds();
        let holding = holds.any();
        if holding {
            if holds.game {
                self.isle.focus.held_by_game = true;
            }
            self.isle.focus.holding = true;
            return;
        }
        if !std::mem::take(&mut self.isle.focus.holding) {
            return;
        }
        let game = std::mem::take(&mut self.isle.focus.held_by_game);
        let held: Vec<u32> = std::mem::take(&mut self.isle.notifications.held)
            .into_iter()
            .filter(|id| {
                self.isle
                    .notifications
                    .entries
                    .iter()
                    .any(|e| e.n.id == *id && e.live && e.n.urgency != 2)
            })
            .collect();
        match held.as_slice() {
            [] => {}
            [one] => self.banner_for(*one, crate::notifications::Fate::Banner),
            many => self.summary_banner(many, game),
        }
    }

    fn summary_banner(&mut self, held: &[u32], game: bool) {
        let mut apps: Vec<String> = Vec::new();
        for id in held {
            if let Some(entry) = self
                .isle
                .notifications
                .entries
                .iter()
                .find(|e| e.n.id == *id)
            {
                let app = self.isle.notification_app(&entry.n);
                let name = self.isle.app_name(&app);
                let name = if name.is_empty() {
                    entry.n.app_name.clone()
                } else {
                    name
                };
                if !apps.contains(&name) {
                    apps.push(name);
                }
            }
        }
        let body = match apps.as_slice() {
            [a] => format!("{} notifications from {a}", held.len()),
            [a, b] => format!("{} notifications from {a} and {b}", held.len()),
            [a, b, rest @ ..] => format!(
                "{} notifications from {a}, {b} and {} more",
                held.len(),
                rest.len()
            ),
            [] => return,
        };
        let card = isle_shell::banners::model::Card {
            id: 0,
            app: "isle.summary".into(),
            app_name: "Isle".into(),
            glyph: Some("bell".into()),
            summary: if game {
                "While you played"
            } else {
                "While you were away"
            }
            .into(),
            body,
            actions: vec![("show".into(), "Show".into())],
            count: 1,
            time: "Now".into(),
            ..Default::default()
        };
        let life = isle_shell::banners::model::Timing::of(self.isle.shell.theme()).default;
        self.isle
            .banners
            .model
            .arrive(card, Some(life), Instant::now());
        self.isle.refresh_banners();
    }
}
