//! Keep Awake: the user's own hold, for a while or until turned off (`p20h-power.md` choices
//! 10 and 11).

use std::time::{Duration, Instant};

use isle_shell::island::{
    grid::TileSize,
    panel::{Module, PanelAction},
    rows::{Choice, PageRow},
};
use smithay::reexports::calloop::{
    RegistrationToken,
    timer::{TimeoutAction, Timer},
};
use tracing::info;

use crate::state::{Isle, State};

/// The hold's name wherever holds are listed.
pub const APP: &str = "Keep Awake";

/// How long Keep Awake's page offers, in hours; none is until turned off.
const FOR: [Option<u64>; 4] = [None, Some(1), Some(2), Some(5)];

pub struct KeepAwake {
    /// The hours chosen; none is until turned off.
    pub hours: Option<u64>,
    /// When it ends by itself.
    pub until: Option<Instant>,
    timer: Option<RegistrationToken>,
}

impl State {
    /// Keep Awake on for `hours`, or until turned off; on already, it starts again.
    pub fn keep_awake(&mut self, hours: Option<u64>) {
        self.keep_awake_clear();
        let timer = hours.and_then(|h| {
            let wait = Duration::from_secs(h * 3600);
            self.isle
                .loop_handle
                .insert_source(Timer::from_duration(wait), |_, _, state| {
                    if let Some(on) = &mut state.isle.idle.keep_awake {
                        on.timer = None;
                    }
                    info!("keep awake's time is up");
                    state.keep_awake_off();
                    TimeoutAction::Drop
                })
                .ok()
        });
        let until = hours.map(|h| Instant::now() + Duration::from_secs(h * 3600));
        info!(?hours, "keep awake on");
        self.isle.idle.keep_awake = Some(KeepAwake {
            hours,
            until,
            timer,
        });
        self.keep_awake_changed();
    }

    pub fn keep_awake_off(&mut self) {
        if self.isle.idle.keep_awake.is_none() {
            return;
        }
        self.keep_awake_clear();
        info!("keep awake off");
        self.keep_awake_changed();
    }

    /// What Keep Awake's module and page asked for.
    pub fn keep_awake_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::Toggle(_) => {
                if self.isle.idle.keep_awake.is_some() {
                    self.keep_awake_off();
                } else {
                    self.keep_awake(None);
                }
            }
            PanelAction::Choose { row, .. } => {
                let hours = row.parse::<u64>().ok();
                self.keep_awake(hours);
            }
            PanelAction::Switch { row, on, .. } if row == "display" => {
                self.isle
                    .write_setting("energy.keep-awake-display", isle_settings::Value::Bool(on));
            }
            other => info!(?other, "keep awake: not a control"),
        }
    }

    fn keep_awake_clear(&mut self) {
        if let Some(token) = self.isle.idle.keep_awake.take().and_then(|k| k.timer) {
            self.isle.loop_handle.remove(token);
        }
    }

    /// Keep Awake turned on or off, or its kind changed: the holds, the glyph and its page.
    pub fn keep_awake_changed(&mut self) {
        self.idle_holds_changed();
        self.isle.keep_awake_glyph();
        self.isle.refresh_island();
    }
}

impl Isle {
    /// The island's glyph while Keep Awake is on: the cup filled while it holds the display,
    /// drawn in outline while it holds only the computer.
    pub fn keep_awake_glyph(&mut self) {
        use isle_shell::island::model::{Glyph, GlyphKind};
        let display = self.settings.energy.keep_awake_display;
        let glyph = self.idle.keep_awake.as_ref().map(|_| Glyph {
            active: true,
            symbol: "isle-coffee".into(),
            fill: display,
            alt: if display {
                "Keep Awake: the display stays on".into()
            } else {
                "Keep Awake: the computer stays on".into()
            },
            ..Default::default()
        });
        self.island.model.set_glyph(GlyphKind::KeepAwake, glyph);
    }

    /// Keep Awake's module: a switch, and a page with how long and whether the display stays
    /// on.
    pub fn keep_awake_module(&self, size: TileSize) -> Option<Module> {
        let on = self.idle.keep_awake.as_ref();
        let display = self.settings.energy.keep_awake_display;
        let state = match on {
            None => "Off".to_owned(),
            Some(KeepAwake { until: None, .. }) => "On".to_owned(),
            Some(KeepAwake {
                until: Some(until), ..
            }) => {
                let left = until.saturating_duration_since(Instant::now());
                let at = jiff::Zoned::now()
                    .checked_add(jiff::SignedDuration::try_from(left).unwrap_or_default())
                    .unwrap_or_else(|_| jiff::Zoned::now());
                let format = if crate::island::clock_24_hour(&self.settings.island) {
                    "%H:%M"
                } else {
                    "%-I:%M %p"
                };
                format!("Until {}", at.strftime(format))
            }
        };
        let mut page: Vec<PageRow> = FOR
            .iter()
            .map(|hours| {
                PageRow::Choice(Choice {
                    id: hours.map_or_else(|| "on".to_owned(), |h| h.to_string()),
                    label: match hours {
                        None => "Until Turned Off".to_owned(),
                        Some(1) => "For 1 Hour".to_owned(),
                        Some(h) => format!("For {h} Hours"),
                    },
                    checked: on.is_some_and(|k| k.hours == *hours),
                    enabled: true,
                    ..Default::default()
                })
            })
            .collect();
        page.push(PageRow::Switch {
            id: "display".into(),
            label: "Keep the Display On".into(),
            on: display,
            enabled: true,
        });
        page.push(PageRow::Status(if display {
            "The display stays on and the session stays unlocked.".into()
        } else {
            "The computer stays on; the display dims, locks and turns off as usual.".into()
        }));
        Some(Module {
            id: "keep-awake".into(),
            size,
            name: APP.into(),
            state,
            glyph: "isle-coffee".into(),
            on: on.is_some(),
            toggles: true,
            page: Some(page),
            page_title: None,
            pane: "Energy".into(),
            ..Default::default()
        })
    }

    pub fn keep_awake_status(&self) -> serde_json::Value {
        match &self.idle.keep_awake {
            None => serde_json::Value::Null,
            Some(on) => serde_json::json!({
                "display": self.settings.energy.keep_awake_display,
                "left_ms": on.until.map(|u| u.saturating_duration_since(Instant::now()).as_millis() as u64),
            }),
        }
    }
}
