//! Night light: every screen warmed on a schedule through its CRTC's gamma ramp, after
//! blending, so scanout stays on and captures are never tinted (`compositor.md` Outputs 8).

use std::time::{Duration, Instant};

use isle_settings::schema::{NightLight as Settings, NightSchedule};
use jiff::{Timestamp, Zoned, civil, tz::TimeZone};
use tracing::info;

/// No warmth at all.
pub const NEUTRAL: f64 = 6500.0;
/// A toggle, or Game Mode letting go, fades this fast.
const QUICK: Duration = Duration::from_secs(1);
/// A schedule's edge fades this slowly.
const SLOW: Duration = Duration::from_secs(60);

/// The Planckian locus in CIE 1931 xy at `kelvin`, from Kim et al.'s cubic fit (1667 K to
/// 25000 K).
fn planckian_xy(kelvin: f64) -> (f64, f64) {
    let t = kelvin.clamp(1667.0, 25000.0);
    let (t1, t2, t3) = (1e3 / t, 1e6 / (t * t), 1e9 / (t * t * t));
    let x = if t <= 4000.0 {
        -0.2661239 * t3 - 0.2343589 * t2 + 0.8776956 * t1 + 0.179910
    } else {
        -3.0258469 * t3 + 2.1070379 * t2 + 0.2226347 * t1 + 0.240390
    };
    let y = if t <= 2222.0 {
        -1.1063814 * x * x * x - 1.34811020 * x * x + 2.18555832 * x - 0.20219683
    } else if t <= 4000.0 {
        -0.9549476 * x * x * x - 1.37418593 * x * x + 2.09137015 * x - 0.16748867
    } else {
        3.0817580 * x * x * x - 5.87338670 * x * x + 3.75112997 * x - 0.37001483
    };
    (x, y)
}

/// Linear sRGB of the white at `kelvin`, brightest channel 1.
fn white(kelvin: f64) -> [f64; 3] {
    let (x, y) = planckian_xy(kelvin);
    let (big_x, big_y, big_z) = (x / y, 1.0, (1.0 - x - y) / y);
    let rgb = [
        3.2404542 * big_x - 1.5371385 * big_y - 0.4985314 * big_z,
        -0.9692660 * big_x + 1.8760108 * big_y + 0.0415560 * big_z,
        0.0556434 * big_x - 0.2040259 * big_y + 1.0572252 * big_z,
    ];
    let max = rgb.iter().cloned().fold(f64::MIN, f64::max);
    rgb.map(|c| (c / max).max(0.0))
}

/// Per-channel gains in linear light that move 6500 K's white to `kelvin`'s; none at 6500 K
/// and above.
pub fn gains(kelvin: f64) -> [f64; 3] {
    if kelvin >= NEUTRAL {
        return [1.0; 3];
    }
    let (warm, neutral) = (white(kelvin), white(NEUTRAL));
    let raw = [0, 1, 2].map(|i| warm[i] / neutral[i]);
    let max = raw.iter().cloned().fold(f64::MIN, f64::max);
    raw.map(|g| (g / max).clamp(0.0, 1.0))
}

fn decode(v: f64) -> f64 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn encode(v: f64) -> f64 {
    if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

/// A gamma ramp of `size` entries a channel, red, then green, then blue.
pub fn ramp(gains: [f64; 3], size: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(size * 3);
    for gain in gains {
        for i in 0..size {
            let v = i as f64 / (size.max(2) - 1) as f64;
            out.push((encode(decode(v) * gain) * 65535.0).round() as u16);
        }
    }
    out
}

/// `HH:MM` as minutes past midnight.
fn minutes(text: &str) -> Option<i64> {
    let (h, m) = text.split_once(':')?;
    let (h, m): (i64, i64) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    ((0..24).contains(&h) && (0..60).contains(&m)).then_some(h * 60 + m)
}

/// The spans, in Unix seconds, the schedule has night light on around `now`.
pub fn spans(settings: &Settings, now: &Zoned, place: Option<(f64, f64)>) -> Vec<(i64, i64)> {
    let tz = now.time_zone().clone();
    let day = |offset: i64| now.date().checked_add(jiff::Span::new().days(offset)).ok();
    let at = |date: civil::Date, mins: i64| {
        date.to_datetime(civil::time((mins / 60) as i8, (mins % 60) as i8, 0, 0))
            .to_zoned(tz.clone())
            .ok()
            .map(|z| z.timestamp().as_second())
    };
    let mut out = Vec::new();
    match settings.schedule {
        NightSchedule::Off => {}
        NightSchedule::Custom => {
            let (Some(from), Some(to)) = (minutes(&settings.from), minutes(&settings.to)) else {
                return out;
            };
            for offset in -1..=1 {
                let Some(date) = day(offset) else { continue };
                let end_date = if to <= from { date.tomorrow().ok() } else { Some(date) };
                if let (Some(start), Some(end)) = (at(date, from), end_date.and_then(|d| at(d, to))) {
                    out.push((start, end));
                }
            }
        }
        NightSchedule::SunsetToSunrise => {
            // Without the zone's place, the evening and the morning stand in.
            let noon = |date: civil::Date| at(date, 12 * 60);
            for offset in -1..=1 {
                let (Some(date), Some(next)) = (day(offset), day(offset + 1)) else {
                    continue;
                };
                let set = match (place, noon(date)) {
                    (Some((lat, lon)), Some(n)) => match isle_sun::sun(n, lat, lon) {
                        isle_sun::Sun::RisesAndSets { set, .. } => Some(set),
                        isle_sun::Sun::Down => at(date, 0),
                        isle_sun::Sun::Up => None,
                    },
                    _ => at(date, 19 * 60),
                };
                let rise = match (place, noon(next)) {
                    (Some((lat, lon)), Some(n)) => match isle_sun::sun(n, lat, lon) {
                        isle_sun::Sun::RisesAndSets { rise, .. } => Some(rise),
                        isle_sun::Sun::Down => at(next, 24 * 60 - 1),
                        isle_sun::Sun::Up => at(next, 0),
                    },
                    _ => at(next, 7 * 60),
                };
                if let (Some(set), Some(rise)) = (set, rise)
                    && rise > set
                {
                    out.push((set, rise));
                }
            }
        }
    }
    out
}

/// Whether the spans have it on at `now`, and when that next changes.
pub fn scheduled(spans: &[(i64, i64)], now: i64) -> (bool, Option<i64>) {
    if let Some(&(_, end)) = spans.iter().find(|(start, end)| *start <= now && now < *end) {
        return (true, Some(end));
    }
    (false, spans.iter().map(|(start, _)| *start).filter(|s| *s > now).min())
}

/// A warmth moving to another.
#[derive(Clone, Copy, Debug)]
struct Fade {
    from: f64,
    to: f64,
    start: Instant,
    length: Duration,
}

impl Fade {
    fn at(&self, now: Instant) -> f64 {
        let t = (now.duration_since(self.start).as_secs_f64() / self.length.as_secs_f64()).min(1.0);
        self.from + (self.to - self.from) * t
    }
}

#[derive(Default)]
pub struct NightLight {
    /// A tap in Control Centre: on or off until the schedule's next change.
    pub manual: Option<(bool, Option<i64>)>,
    /// Game Mode turned it off; leaving gives it back.
    pub held_by_game: bool,
    /// The warmth on the screens now, in kelvin.
    pub kelvin: f64,
    fade: Option<Fade>,
    /// When the schedule next changes.
    pub next_change: Option<i64>,
    pub scheduled_on: bool,
}

impl NightLight {
    /// Whether it is on, the hold aside.
    pub fn wanted(&self) -> bool {
        self.manual.map_or(self.scheduled_on, |(on, _)| on)
    }

    pub fn on(&self) -> bool {
        self.wanted() && !self.held_by_game
    }

    fn target(&self, settings: &Settings) -> f64 {
        if self.on() { settings.warmth as f64 } else { NEUTRAL }
    }

    /// The warmth now, a fade stepped on; whether one is still running.
    pub fn step(&mut self, now: Instant) -> bool {
        let Some(fade) = self.fade else {
            return false;
        };
        self.kelvin = fade.at(now);
        if now.duration_since(fade.start) >= fade.length {
            self.fade = None;
            return false;
        }
        true
    }

    /// Moves toward what is wanted now, over `length`, or at once.
    fn retarget(&mut self, settings: &Settings, length: Option<Duration>) {
        let to = self.target(settings);
        if self.kelvin == 0.0 {
            self.kelvin = NEUTRAL;
        }
        match length {
            Some(length) if (to - self.kelvin).abs() > 1.0 => {
                self.fade = Some(Fade {
                    from: self.kelvin,
                    to,
                    start: Instant::now(),
                    length,
                });
            }
            _ => {
                self.kelvin = to;
                self.fade = None;
            }
        }
    }

    /// The schedule looked at again; a change fades slowly unless `first`.
    pub fn reschedule(&mut self, settings: &Settings, now: &Zoned, place: Option<(f64, f64)>, first: bool) {
        let at = now.timestamp().as_second();
        let (on, next) = scheduled(&spans(settings, now, place), at);
        if let Some((_, Some(until))) = self.manual
            && at >= until
        {
            self.manual = None;
        }
        let was = self.on();
        self.scheduled_on = on;
        self.next_change = next;
        if first {
            self.retarget(settings, None);
        } else if was != self.on() || self.fade.is_none() {
            self.retarget(settings, Some(SLOW));
        }
    }

    /// A tap: the opposite of what shows, until the schedule's next change.
    pub fn toggle(&mut self, settings: &Settings) {
        let on = !self.on();
        if on {
            // A user turning it on during Game Mode wins for this game.
            self.held_by_game = false;
        }
        let until = (settings.schedule != NightSchedule::Off).then_some(self.next_change).flatten();
        self.manual = Some((on, until));
        info!(on, ?until, "night light");
        self.retarget(settings, Some(QUICK));
    }

    /// Game Mode came on (`gaming.md` Game Mode 3): off at once, no fade.
    pub fn hold_for_game(&mut self, settings: &Settings) {
        if self.on() {
            self.held_by_game = true;
            self.retarget(settings, None);
        }
    }

    /// Game Mode went: what it changed comes back, with a fade.
    pub fn release_game(&mut self, settings: &Settings) {
        if std::mem::take(&mut self.held_by_game) {
            self.retarget(settings, Some(QUICK));
        }
    }

    /// The warmth setting changed.
    pub fn settings_changed(&mut self, settings: &Settings) {
        self.retarget(settings, Some(QUICK));
    }

    /// What the module's second line says.
    pub fn state_line(&self) -> String {
        if self.held_by_game {
            return "Off during Game Mode".into();
        }
        let until = self.manual.map_or(self.next_change, |(_, until)| until);
        let word = if self.on() { "On" } else { "Off" };
        match until.and_then(|s| Timestamp::from_second(s).ok()) {
            Some(t) => format!("{word} until {}", t.to_zoned(TimeZone::system()).strftime("%H:%M")),
            None => word.into(),
        }
    }
}

/// The system's time zone's place on the map, from tzdata, read once.
pub fn place() -> Option<(f64, f64)> {
    static PLACE: std::sync::OnceLock<Option<(f64, f64)>> = std::sync::OnceLock::new();
    *PLACE.get_or_init(|| isle_sun::coordinates(TimeZone::system().iana_name()?))
}

/// Whether the sun is down where the time zone is: what `appearance.mode` Auto follows.
pub fn sun_down(now: &Zoned) -> bool {
    let schedule = Settings {
        schedule: NightSchedule::SunsetToSunrise,
        from: String::new(),
        to: String::new(),
        warmth: 0,
    };
    scheduled(&spans(&schedule, now, place()), now.timestamp().as_second()).0
}

/// When the sun next rises or sets there.
fn next_sun_change(now: &Zoned) -> Option<i64> {
    let schedule = Settings {
        schedule: NightSchedule::SunsetToSunrise,
        from: String::new(),
        to: String::new(),
        warmth: 0,
    };
    scheduled(&spans(&schedule, now, place()), now.timestamp().as_second()).1
}

/// The longest the schedule goes unchecked, for a clock or zone that changes under it.
const RECHECK: Duration = Duration::from_secs(600);
/// How often a fade steps.
const FADE_STEP: Duration = Duration::from_millis(50);

impl crate::state::State {
    /// Night light at start: on or off as the schedule says, at once.
    pub fn start_night_light(&mut self) {
        let settings = self.isle.settings.night_light.clone();
        self.isle
            .night_light
            .reschedule(&settings, &Zoned::now(), place(), true);
        self.apply_night_light(true);
        self.arm_night_light();
    }

    /// Looks at the schedule again at its next change, and the sun's for Auto appearance.
    fn arm_night_light(&mut self) {
        use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
        if let Some(token) = self.isle.night_timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let now = Zoned::now();
        let at = now.timestamp().as_second();
        let next = [self.isle.night_light.next_change, next_sun_change(&now)]
            .into_iter()
            .flatten()
            .map(|t| Duration::from_secs((t - at).max(1) as u64))
            .min()
            .unwrap_or(RECHECK)
            .min(RECHECK);
        self.isle.night_timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(next), |_, _, state| {
                let _beat = state.isle.heartbeat.beat("night-light");
                state.isle.night_timer = None;
                let settings = state.isle.settings.night_light.clone();
                state
                    .isle
                    .night_light
                    .reschedule(&settings, &Zoned::now(), place(), false);
                if state.isle.settings.appearance.mode == isle_settings::schema::AppearanceMode::Auto {
                    state.isle.refresh_theme();
                }
                state.night_light_changed();
                TimeoutAction::Drop
            })
            .ok();
    }

    /// Something changed what night light wants: the screens follow, fading where asked.
    pub fn night_light_changed(&mut self) {
        self.apply_night_light(false);
        self.arm_night_light();
        self.isle.refresh_island();
    }

    /// Puts the warmth on the screens, stepping a fade on its own timer.
    fn apply_night_light(&mut self, force: bool) {
        use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
        let fading = self.isle.night_light.step(Instant::now());
        let kelvin = self.isle.night_light.kelvin;
        if force || (kelvin - self.isle.night_applied).abs() >= 10.0 || (!fading && kelvin != self.isle.night_applied) {
            self.backend.set_warmth(kelvin);
            self.isle.night_applied = kelvin;
        }
        if fading && !self.isle.night_fading {
            self.isle.night_fading = true;
            let _ = self.isle.loop_handle.insert_source(Timer::from_duration(FADE_STEP), |_, _, state| {
                let _beat = state.isle.heartbeat.beat("night-light-fade");
                let fading = state.isle.night_light.step(Instant::now());
                let kelvin = state.isle.night_light.kelvin;
                if (kelvin - state.isle.night_applied).abs() >= 10.0 || !fading {
                    state.backend.set_warmth(kelvin);
                    state.isle.night_applied = kelvin;
                }
                if fading {
                    TimeoutAction::ToDuration(FADE_STEP)
                } else {
                    state.isle.night_fading = false;
                    TimeoutAction::Drop
                }
            });
        }
    }

    /// Control Centre's tap.
    pub fn toggle_night_light(&mut self) {
        let settings = self.isle.settings.night_light.clone();
        self.isle.night_light.toggle(&settings);
        self.night_light_changed();
    }

    /// Game Mode came or went.
    pub fn night_light_for_game(&mut self, game: bool) {
        let settings = self.isle.settings.night_light.clone();
        if game {
            self.isle.night_light.hold_for_game(&settings);
        } else {
            self.isle.night_light.release_game(&settings);
        }
        self.night_light_changed();
    }

    /// The `night-light` settings changed.
    pub fn night_light_settings_changed(&mut self) {
        let settings = self.isle.settings.night_light.clone();
        self.isle
            .night_light
            .reschedule(&settings, &Zoned::now(), place(), false);
        self.isle.night_light.settings_changed(&settings);
        self.night_light_changed();
    }

    /// What night light is doing, as JSON (`isle night-light`).
    pub fn night_light_status(&self) -> String {
        let light = &self.isle.night_light;
        serde_json::json!({
            "on": light.on(),
            "scheduled": light.scheduled_on,
            "manual": light.manual.map(|(on, _)| on),
            "held-by-game": light.held_by_game,
            "kelvin": light.kelvin.round(),
            "state": light.state_line(),
            "schedule": self.isle.settings.night_light.schedule.as_str(),
            "warmth": self.isle.settings.night_light.warmth,
            "outputs": self.backend.gamma_paths(),
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_is_untouched_and_warm_loses_blue() {
        assert_eq!(gains(6500.0), [1.0, 1.0, 1.0]);
        assert_eq!(gains(9000.0), [1.0, 1.0, 1.0]);
        let warm = gains(3400.0);
        assert!((warm[0] - 1.0).abs() < 1e-9, "{warm:?}");
        assert!(warm[2] < warm[1] && warm[1] < 1.0, "{warm:?}");
        let warmer = gains(2700.0);
        assert!(warmer[2] < warm[2] && warmer[1] < warm[1]);
        // Within a few hundredths of Thies's blackbody table at 3400 K, decoded to linear
        // light: green 0.56, blue 0.24.
        assert!((warm[1] - 0.56).abs() < 0.04 && (warm[2] - 0.24).abs() < 0.04, "{warm:?}");
    }

    #[test]
    fn the_ramp_is_identity_when_neutral() {
        let ramp = ramp([1.0; 3], 256);
        assert_eq!(ramp.len(), 768);
        assert_eq!((ramp[0], ramp[255], ramp[511], ramp[767]), (0, 65535, 65535, 65535));
        assert_eq!(ramp[128], ((128.0 / 255.0) * 65535.0_f64).round() as u16);
    }

    fn settings(schedule: NightSchedule, from: &str, to: &str) -> Settings {
        Settings {
            schedule,
            from: from.into(),
            to: to.into(),
            warmth: 3400,
        }
    }

    fn zoned(text: &str) -> Zoned {
        text.parse().unwrap()
    }

    #[test]
    fn a_custom_schedule_across_midnight() {
        let s = settings(NightSchedule::Custom, "22:00", "07:00");
        let now = zoned("2026-10-10T23:30[Europe/London]");
        let (on, next) = scheduled(&spans(&s, &now, None), now.timestamp().as_second());
        assert!(on);
        assert_eq!(next, Some(zoned("2026-10-11T07:00[Europe/London]").timestamp().as_second()));
        let now = zoned("2026-10-10T12:00[Europe/London]");
        let (on, next) = scheduled(&spans(&s, &now, None), now.timestamp().as_second());
        assert!(!on);
        assert_eq!(next, Some(zoned("2026-10-10T22:00[Europe/London]").timestamp().as_second()));
    }

    #[test]
    fn sunset_to_sunrise_in_london() {
        let s = settings(NightSchedule::SunsetToSunrise, "", "");
        let london = Some((51.5083, -0.1253));
        let evening = zoned("2026-10-10T21:00[Europe/London]");
        let (on, next) = scheduled(&spans(&s, &evening, london), evening.timestamp().as_second());
        assert!(on);
        let sunrise = Timestamp::from_second(next.unwrap()).unwrap().to_zoned(TimeZone::get("Europe/London").unwrap());
        assert_eq!(sunrise.date(), civil::date(2026, 10, 11));
        assert!((6..=8).contains(&sunrise.hour()), "{sunrise}");
        let afternoon = zoned("2026-10-10T14:00[Europe/London]");
        let (on, next) = scheduled(&spans(&s, &afternoon, london), afternoon.timestamp().as_second());
        assert!(!on);
        let sunset = Timestamp::from_second(next.unwrap()).unwrap().to_zoned(TimeZone::get("Europe/London").unwrap());
        assert!((17..=19).contains(&sunset.hour()), "{sunset}");
    }

    #[test]
    fn a_tap_holds_until_the_next_change_and_game_mode_wins_until_the_user_does() {
        let s = settings(NightSchedule::Custom, "22:00", "07:00");
        let mut light = NightLight::default();
        let now = zoned("2026-10-10T23:30[Europe/London]");
        light.reschedule(&s, &now, None, true);
        assert!(light.on());
        assert_eq!(light.kelvin, 3400.0);

        light.hold_for_game(&s);
        assert!(!light.on());
        assert_eq!(light.kelvin, NEUTRAL, "off at once");
        light.toggle(&s);
        assert!(light.on() && !light.held_by_game, "the user wins");

        light.toggle(&s);
        assert!(!light.on());
        let morning = zoned("2026-10-11T07:30[Europe/London]");
        light.reschedule(&s, &morning, None, false);
        assert!(light.manual.is_none(), "the tap lasts until the schedule's next change");
        assert!(!light.on());
    }
}
