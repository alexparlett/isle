//! Focus as a pure model: the modes, what turns each on, and which one is in force, by
//! `structure/focus.md`'s order: Game Mode while a detected game is in front, then a mode
//! turned on by hand, then the latest automatic start (`p8-services.md` choice 12).

use std::collections::BTreeMap;

use jiff::{Timestamp, Zoned, civil::Time};
use serde::{Deserialize, Serialize};

pub const DO_NOT_DISTURB: &str = "Do Not Disturb";
pub const GAME_MODE: &str = "Game Mode";

/// A time of day when a mode turns itself on, and the days.
#[derive(Clone, Debug, PartialEq)]
pub struct Schedule {
    pub from: Time,
    pub to: Time,
    /// Monday first.
    pub days: [bool; 7],
}

impl Schedule {
    /// `HH:MM-HH:MM`, then `every-day` (the default), `weekdays`, `weekends`, or days by
    /// their first three letters joined by commas.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut words = text.split_whitespace();
        let span = words.next().ok_or("a schedule needs its times")?;
        let (from, to) = span
            .split_once('-')
            .ok_or("a schedule's times are from-to")?;
        let time = |t: &str| -> Result<Time, String> {
            let (h, m) = t.split_once(':').ok_or(format!("{t} is not HH:MM"))?;
            Time::new(
                h.parse().map_err(|_| format!("{t} is not HH:MM"))?,
                m.parse().map_err(|_| format!("{t} is not HH:MM"))?,
                0,
                0,
            )
            .map_err(|e| e.to_string())
        };
        let (from, to) = (time(from)?, time(to)?);
        if from == to {
            return Err("a schedule's times are the same".into());
        }
        let days = match words.next().unwrap_or("every-day") {
            "every-day" => [true; 7],
            "weekdays" => [true, true, true, true, true, false, false],
            "weekends" => [false, false, false, false, false, true, true],
            list => {
                let mut days = [false; 7];
                for day in list.split(',') {
                    let i = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
                        .iter()
                        .position(|d| day.eq_ignore_ascii_case(d))
                        .ok_or(format!("{day} is not a day"))?;
                    days[i] = true;
                }
                days
            }
        };
        Ok(Self { from, to, days })
    }

    /// The start of the run covering `now`, if one does; a run past midnight belongs to the
    /// day it started.
    fn started(&self, now: &Zoned) -> Option<Zoned> {
        let today = now.date();
        for day in [today, today.yesterday().ok()?] {
            if !self.days[day.weekday().to_monday_zero_offset() as usize] {
                continue;
            }
            let start = day
                .to_datetime(self.from)
                .to_zoned(now.time_zone().clone())
                .ok()?;
            let end_day = if self.to > self.from {
                day
            } else {
                day.tomorrow().ok()?
            };
            let end = end_day
                .to_datetime(self.to)
                .to_zoned(now.time_zone().clone())
                .ok()?;
            if &start <= now && now < &end {
                return Some(start);
            }
        }
        None
    }

    /// The next start or end after `now`.
    fn next_edge(&self, now: &Zoned) -> Option<Timestamp> {
        let mut best: Option<Timestamp> = None;
        let mut day = now.date().yesterday().ok()?;
        for _ in 0..9 {
            if self.days[day.weekday().to_monday_zero_offset() as usize] {
                let start = day
                    .to_datetime(self.from)
                    .to_zoned(now.time_zone().clone())
                    .ok()?;
                let end_day = if self.to > self.from {
                    day
                } else {
                    day.tomorrow().ok()?
                };
                let end = end_day
                    .to_datetime(self.to)
                    .to_zoned(now.time_zone().clone())
                    .ok()?;
                for edge in [start.timestamp(), end.timestamp()] {
                    if edge > now.timestamp() && best.is_none_or(|b| edge < b) {
                        best = Some(edge);
                    }
                }
            }
            day = day.tomorrow().ok()?;
        }
        best
    }

    /// The next start after `now`.
    fn next_start(&self, now: &Zoned) -> Option<Timestamp> {
        let mut day = now.date();
        for _ in 0..8 {
            if self.days[day.weekday().to_monday_zero_offset() as usize] {
                let start = day
                    .to_datetime(self.from)
                    .to_zoned(now.time_zone().clone())
                    .ok()?;
                if start.timestamp() > now.timestamp() {
                    return Some(start.timestamp());
                }
            }
            day = day.tomorrow().ok()?;
        }
        None
    }

    fn end(&self, started: &Zoned) -> Option<Timestamp> {
        let day = started.date();
        let end_day = if self.to > self.from {
            day
        } else {
            day.tomorrow().ok()?
        };
        Some(
            end_day
                .to_datetime(self.to)
                .to_zoned(started.time_zone().clone())
                .ok()?
                .timestamp(),
        )
    }
}

/// One mode as settings make it.
#[derive(Clone, Debug, PartialEq)]
pub struct Mode {
    pub name: String,
    pub symbol: String,
    /// Apps whose banners still come through.
    pub allowed: Vec<String>,
    /// Critical notifications pass.
    pub critical: bool,
    pub schedules: Vec<Schedule>,
    /// Apps whose being in front turns it on.
    pub apps: Vec<String>,
}

impl Mode {
    pub fn new(name: &str, symbol: &str) -> Self {
        Self {
            name: name.into(),
            symbol: symbol.into(),
            allowed: Vec::new(),
            critical: true,
            schedules: Vec::new(),
            apps: Vec::new(),
        }
    }
}

/// The modes and Game Mode's own rules.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Do Not Disturb and Game Mode first, then the user's.
    pub modes: Vec<Mode>,
    /// Game Mode turns on for a game in front.
    pub game_auto: bool,
    /// Games Game Mode stays off for, by app id.
    pub off_for: Vec<String>,
}

impl Config {
    pub fn mode(&self, name: &str) -> Option<&Mode> {
        self.modes.iter().find(|m| m.name == name)
    }
}

/// What the model remembers, kept across restarts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    /// A mode turned on by hand, and until when (Unix seconds); none until turned off.
    pub manual: Option<(String, Option<i64>)>,
    /// Modes turned off by hand while a schedule had them on, until that schedule's next
    /// start.
    pub suppressed: BTreeMap<String, i64>,
    /// When each automatic start began, the latest winning.
    pub started: BTreeMap<String, i64>,
    /// The mode last turned on, whose symbol the module offers.
    pub last: Option<String>,
}

/// Why the mode in force is on.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Why {
    /// By hand, until a time or until turned off.
    Manual(Option<i64>),
    /// Its schedule, until its end.
    Schedule(i64),
    /// An app in front.
    App(String),
    /// A game in front.
    Game(String),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Active {
    pub mode: String,
    pub why: Why,
}

/// What is in front: the app, and whether it is a game.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Front {
    pub app: Option<String>,
    pub game: bool,
}

/// The mode in force at `now`, updating what the model remembers of automatic starts and
/// expired manual ones.
pub fn evaluate(cfg: &Config, mem: &mut Memory, now: &Zoned, front: &Front) -> Option<Active> {
    let secs = now.timestamp().as_second();
    if let Some((_, Some(until))) = &mem.manual
        && *until <= secs
    {
        mem.manual = None;
    }
    mem.suppressed.retain(|_, until| *until > secs);

    // 1. Game Mode while a detected game is in front, unless turned off for that game.
    if cfg.game_auto
        && front.game
        && let Some(app) = &front.app
        && !cfg.off_for.contains(app)
    {
        return Some(Active {
            mode: GAME_MODE.into(),
            why: Why::Game(app.clone()),
        });
    }
    // 2. By hand.
    if let Some((mode, until)) = &mem.manual
        && cfg.mode(mode).is_some()
    {
        return Some(Active {
            mode: mode.clone(),
            why: Why::Manual(*until),
        });
    }
    // 3. The latest automatic start.
    let mut due: Vec<(i64, Active)> = Vec::new();
    for mode in &cfg.modes {
        if mem.suppressed.contains_key(&mode.name) {
            continue;
        }
        for schedule in &mode.schedules {
            if let Some(start) = schedule.started(now) {
                let end = schedule.end(&start).map_or(secs, |e| e.as_second());
                due.push((
                    start.timestamp().as_second(),
                    Active {
                        mode: mode.name.clone(),
                        why: Why::Schedule(end),
                    },
                ));
            }
        }
        if let Some(app) = front.app.as_ref().filter(|a| mode.apps.contains(a)) {
            let start = *mem
                .started
                .entry(format!("app:{}", mode.name))
                .or_insert(secs);
            due.push((
                start,
                Active {
                    mode: mode.name.clone(),
                    why: Why::App(app.clone()),
                },
            ));
        } else {
            mem.started.remove(&format!("app:{}", mode.name));
        }
    }
    due.sort_by_key(|(start, _)| *start);
    due.pop().map(|(_, active)| active)
}

/// When the mode in force could next change by itself: a schedule's edge, a manual end.
pub fn next_change(cfg: &Config, mem: &Memory, now: &Zoned) -> Option<Timestamp> {
    let mut next: Option<Timestamp> = mem
        .manual
        .as_ref()
        .and_then(|(_, until)| *until)
        .and_then(|u| Timestamp::from_second(u).ok());
    for until in mem.suppressed.values() {
        if let Ok(t) = Timestamp::from_second(*until) {
            next = Some(next.map_or(t, |n| n.min(t)));
        }
    }
    for mode in &cfg.modes {
        for schedule in &mode.schedules {
            if let Some(edge) = schedule.next_edge(now) {
                next = Some(next.map_or(edge, |n| n.min(edge)));
            }
        }
    }
    next
}

/// Turns `mode` on by hand, until `until` (Unix seconds) or until turned off.
pub fn turn_on(mem: &mut Memory, mode: &str, until: Option<i64>) {
    mem.manual = Some((mode.to_owned(), until));
    mem.suppressed.remove(mode);
    mem.last = Some(mode.to_owned());
}

/// What turning a mode off by hand asks of settings: a game to keep Game Mode off for.
#[derive(Clone, Debug, PartialEq)]
pub enum Off {
    Done,
    /// Game Mode turned off over this game: it joins "Off for these games".
    KeepOffFor(String),
}

/// Turns the mode in force off by hand.
pub fn turn_off(cfg: &Config, mem: &mut Memory, active: &Active, now: &Zoned) -> Off {
    match &active.why {
        Why::Game(game) => return Off::KeepOffFor(game.clone()),
        Why::Manual(_) => mem.manual = None,
        Why::Schedule(_) | Why::App(_) => {
            // Off until that schedule next starts; an app's, until it leaves the front.
            let until = cfg
                .mode(&active.mode)
                .and_then(|m| m.schedules.iter().filter_map(|s| s.next_start(now)).min())
                .map_or(now.timestamp().as_second() + 86_400, |t| t.as_second());
            mem.suppressed.insert(active.mode.clone(), until);
        }
    }
    Off::Done
}

/// The built-in modes, with the user's own after them.
pub fn builtin() -> Vec<Mode> {
    vec![
        Mode::new(DO_NOT_DISTURB, "moon"),
        Mode::new(GAME_MODE, "game-controller"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Zoned {
        format!("{text}[Europe/London]").parse().unwrap()
    }

    fn config() -> Config {
        let mut dnd = Mode::new(DO_NOT_DISTURB, "moon");
        dnd.schedules.push(Schedule::parse("22:00-07:00").unwrap());
        let mut reading = Mode::new("Reading", "book");
        reading
            .schedules
            .push(Schedule::parse("21:00-23:00 weekdays").unwrap());
        reading.apps.push("org.gnome.Evince".into());
        Config {
            modes: vec![dnd, Mode::new(GAME_MODE, "game-controller"), reading],
            game_auto: true,
            off_for: vec![],
        }
    }

    #[test]
    fn schedules_parse_and_refuse_equal_times() {
        let s = Schedule::parse("22:00-07:00 weekdays").unwrap();
        assert_eq!(s.days, [true, true, true, true, true, false, false]);
        assert!(Schedule::parse("09:00-09:00").is_err());
        assert!(Schedule::parse("9-5").is_err());
        assert_eq!(
            Schedule::parse("08:00-09:00 sat,sun").unwrap().days[5],
            true
        );
    }

    #[test]
    fn a_schedule_across_midnight_holds_until_morning() {
        let cfg = config();
        let mut mem = Memory::default();
        // Wednesday 7 October 2026, 02:00: Tuesday night's run.
        let active = evaluate(&cfg, &mut mem, &at("2026-10-07T02:00"), &Front::default()).unwrap();
        assert_eq!(active.mode, DO_NOT_DISTURB);
        assert_eq!(
            evaluate(&cfg, &mut mem, &at("2026-10-07T08:00"), &Front::default()),
            None
        );
    }

    #[test]
    fn the_latest_automatic_start_wins_and_by_hand_beats_both() {
        let cfg = config();
        let mut mem = Memory::default();
        // 22:30 on a weekday: Reading started at 21:00, Do Not Disturb at 22:00.
        let now = at("2026-10-07T22:30");
        assert_eq!(
            evaluate(&cfg, &mut mem, &now, &Front::default())
                .unwrap()
                .mode,
            DO_NOT_DISTURB
        );
        turn_on(&mut mem, "Reading", None);
        let active = evaluate(&cfg, &mut mem, &now, &Front::default()).unwrap();
        assert_eq!(
            (active.mode.as_str(), active.why),
            ("Reading", Why::Manual(None))
        );
    }

    #[test]
    fn game_mode_takes_over_and_gives_the_manual_mode_back() {
        let cfg = config();
        let mut mem = Memory::default();
        let now = at("2026-10-07T15:00");
        turn_on(&mut mem, DO_NOT_DISTURB, None);
        let game = Front {
            app: Some("steam_app_440".into()),
            game: true,
        };
        assert_eq!(
            evaluate(&cfg, &mut mem, &now, &game).unwrap().mode,
            GAME_MODE
        );
        assert_eq!(
            evaluate(&cfg, &mut mem, &now, &Front::default())
                .unwrap()
                .mode,
            DO_NOT_DISTURB
        );
    }

    #[test]
    fn game_mode_turned_off_over_a_game_stays_off_for_it() {
        let mut cfg = config();
        let mut mem = Memory::default();
        let now = at("2026-10-07T15:00");
        let game = Front {
            app: Some("steam_app_440".into()),
            game: true,
        };
        let active = evaluate(&cfg, &mut mem, &now, &game).unwrap();
        assert_eq!(
            turn_off(&cfg, &mut mem, &active, &now),
            Off::KeepOffFor("steam_app_440".into())
        );
        cfg.off_for.push("steam_app_440".into());
        assert_eq!(evaluate(&cfg, &mut mem, &now, &game), None);
    }

    #[test]
    fn a_scheduled_mode_turned_off_stays_off_until_its_next_start() {
        let cfg = config();
        let mut mem = Memory::default();
        let night = at("2026-10-07T23:30");
        let active = evaluate(&cfg, &mut mem, &night, &Front::default()).unwrap();
        turn_off(&cfg, &mut mem, &active, &night);
        assert_eq!(evaluate(&cfg, &mut mem, &night, &Front::default()), None);
        let next_night = at("2026-10-08T22:10");
        assert_eq!(
            evaluate(&cfg, &mut mem, &next_night, &Front::default())
                .unwrap()
                .mode,
            DO_NOT_DISTURB
        );
    }

    #[test]
    fn an_app_in_front_turns_its_mode_on_and_a_manual_end_passes() {
        let cfg = config();
        let mut mem = Memory::default();
        let now = at("2026-10-07T15:00");
        let evince = Front {
            app: Some("org.gnome.Evince".into()),
            game: false,
        };
        let active = evaluate(&cfg, &mut mem, &now, &evince).unwrap();
        assert_eq!(active.why, Why::App("org.gnome.Evince".into()));
        turn_on(
            &mut mem,
            DO_NOT_DISTURB,
            Some(now.timestamp().as_second() + 60),
        );
        assert_eq!(
            evaluate(&cfg, &mut mem, &now, &Front::default())
                .unwrap()
                .mode,
            DO_NOT_DISTURB
        );
        let later = at("2026-10-07T15:02");
        assert_eq!(evaluate(&cfg, &mut mem, &later, &Front::default()), None);
    }

    #[test]
    fn the_next_change_is_the_soonest_edge() {
        let cfg = config();
        let mem = Memory::default();
        let next = next_change(&cfg, &mem, &at("2026-10-07T15:00")).unwrap();
        assert_eq!(next, at("2026-10-07T21:00").timestamp());
    }
}
