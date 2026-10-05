//! What the island shows, from what its sources post: the rules of `structure/island.md`'s
//! queue, with no drawing and no clock of its own, so they are tested alone.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::{Duration, Instant},
};

/// The status glyphs, in the order they stand after the clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GlyphKind {
    Wifi,
    Bluetooth,
    Sound,
    Vpn,
    Focus,
    Updates,
    Battery,
    Inhibited,
    ConfigError,
}

impl GlyphKind {
    pub const ALL: [GlyphKind; 9] = [
        GlyphKind::Wifi,
        GlyphKind::Bluetooth,
        GlyphKind::Sound,
        GlyphKind::Vpn,
        GlyphKind::Focus,
        GlyphKind::Updates,
        GlyphKind::Battery,
        GlyphKind::Inhibited,
        GlyphKind::ConfigError,
    ];

    /// The name settings and the bus use.
    pub fn name(self) -> &'static str {
        match self {
            GlyphKind::Wifi => "wifi",
            GlyphKind::Bluetooth => "bluetooth",
            GlyphKind::Sound => "sound",
            GlyphKind::Vpn => "vpn",
            GlyphKind::Focus => "focus",
            GlyphKind::Updates => "updates",
            GlyphKind::Battery => "battery",
            GlyphKind::Inhibited => "inhibited",
            GlyphKind::ConfigError => "config-error",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }

    /// The last two show whenever their state holds: the user could not otherwise tell why
    /// chords do nothing or settings are not in force.
    pub fn tiered(self) -> bool {
        !matches!(self, GlyphKind::Inhibited | GlyphKind::ConfigError)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tier {
    Always,
    #[default]
    WhenActive,
    Never,
}

/// One glyph's state as its source posts it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Glyph {
    /// Not the normal state, so it shows at the default tier.
    pub active: bool,
    /// The Phosphor or Isle glyph drawn.
    pub symbol: String,
    /// A level drawn over the dimmed `symbol`, such as Wi-Fi's strength.
    pub level: Option<String>,
    pub fill: bool,
    pub alt: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Privacy {
    Microphone,
    Camera,
    Screen,
}

/// Live activities, highest rank first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActivityKind {
    Prompt,
    Recording,
    Sharing,
    Call,
    Timer,
    Media,
}

impl ActivityKind {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "prompt" => Self::Prompt,
            "recording" => Self::Recording,
            "sharing" => Self::Sharing,
            "call" => Self::Call,
            "timer" => Self::Timer,
            "media" => Self::Media,
            _ => return None,
        })
    }

    /// The mark it shows as when another activity leads.
    pub fn mark(self) -> &'static str {
        match self {
            Self::Prompt => "lock",
            Self::Recording => "record",
            Self::Sharing => "screencast",
            Self::Call => "phone",
            Self::Timer => "timer",
            Self::Media => "music-note",
        }
    }
}

/// A live activity: its compact face and its row in Control Centre.
#[derive(Clone, Debug, PartialEq)]
pub struct Activity {
    pub kind: ActivityKind,
    /// The face's text: an elapsed time, an app's name, a title.
    pub face: String,
    pub title: String,
    pub detail: String,
    /// Its row's buttons, by id and label.
    pub actions: Vec<(String, String)>,
    pub app: Option<String>,
    pub started: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Osd,
    Copied,
    Mode,
    Attention,
    Start,
    Ended,
    TimerDone,
}

impl EventKind {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "osd" => Self::Osd,
            "copied" => Self::Copied,
            "mode" => Self::Mode,
            "attention" => Self::Attention,
            "start" => Self::Start,
            "ended" => Self::Ended,
            "timer-done" => Self::TimerDone,
            _ => return None,
        })
    }
}

/// How an event is drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum Form {
    /// A glyph and a line of text.
    Line {
        symbol: String,
        fill: bool,
        text: String,
    },
    /// A level with the device it applies to.
    Level {
        symbol: String,
        name: String,
        level: f32,
        muted: bool,
    },
    /// An activity's row, briefly, as it starts.
    StartRow(Activity),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub kind: EventKind,
    pub form: Form,
    /// Shown on this output only, by name; else on the main one.
    pub output: Option<String>,
    /// The app it is about, for attention.
    pub app: Option<String>,
    /// Holds until the app is focused or it is clicked.
    pub urgent: bool,
}

/// How long each kind of event shows, from the tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timings {
    pub osd: Duration,
    pub copied: Duration,
    pub mode: Duration,
    pub event: Duration,
    pub start_media: Duration,
    pub ended: Duration,
}

impl Timings {
    pub fn of(theme: &isle_tokens::Theme) -> Self {
        let t = &theme.tokens;
        let ms = |v: u32| Duration::from_millis(v.into());
        Self {
            osd: ms(t.timing_osd_ms),
            copied: ms(t.timing_copied_ms),
            mode: ms(t.timing_mode_ms),
            event: ms(t.timing_event_ms),
            start_media: ms(t.timing_start_media_ms),
            ended: ms(t.timing_ended_ms),
        }
    }

    /// How long `event` shows; `None` holds until acknowledged.
    fn duration(&self, event: &Event) -> Option<Duration> {
        match event.kind {
            EventKind::Osd => Some(self.osd),
            EventKind::Copied => Some(self.copied),
            EventKind::Mode => Some(self.mode),
            EventKind::Attention if event.urgent => None,
            EventKind::Attention => Some(self.event),
            EventKind::Start => Some(match &event.form {
                Form::StartRow(a) if a.kind == ActivityKind::Media => self.start_media,
                _ => self.event,
            }),
            EventKind::Ended => Some(self.ended),
            EventKind::TimerDone => None,
        }
    }
}

/// What an output's island shows now.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    /// Covered by a full-screen window with nothing to show: no shape at all.
    Nothing,
    /// Covered or locked: the privacy marks alone.
    Privacy(Vec<Privacy>),
    /// An OSD over a full-screen window or the lock, at the island's anchor.
    Alone(Event, Vec<Privacy>),
    /// The pill: the leader's face, other activities' marks or their count, the glyphs that
    /// show, and the privacy marks. The clock is always there.
    Pill {
        leader: Option<Activity>,
        marks: Vec<ActivityKind>,
        more: usize,
        glyphs: Vec<(GlyphKind, Glyph)>,
        privacy: Vec<Privacy>,
    },
    /// An event over the face, clock and glyphs; the privacy marks stay.
    Event(Event, Vec<Privacy>),
}

/// One output's circumstances.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Place {
    pub main: bool,
    /// A full-screen window is in front and the island has not been brought back.
    pub covered: bool,
    /// Its panel is open: events wait.
    pub panel: bool,
}

#[derive(Debug)]
pub struct Island {
    timings: Timings,
    marks_limit: usize,
    glyphs: BTreeMap<GlyphKind, Glyph>,
    tiers: BTreeMap<GlyphKind, Tier>,
    privacy: BTreeSet<Privacy>,
    activities: Vec<Activity>,
    hidden: BTreeSet<ActivityKind>,
    showing: Option<(Event, Instant)>,
    queue: VecDeque<(Event, Instant)>,
    /// Apps showing a banner, whose non-urgent attention is held back.
    banners: BTreeSet<String>,
    focused_app: Option<String>,
    locked: bool,
    /// Whether the main output's panel is open, which holds events back.
    main_panel: bool,
    main_covered: bool,
}

impl Island {
    pub fn new(timings: Timings, marks_limit: usize) -> Self {
        Self {
            timings,
            marks_limit,
            glyphs: BTreeMap::new(),
            tiers: BTreeMap::new(),
            privacy: BTreeSet::new(),
            activities: Vec::new(),
            hidden: BTreeSet::new(),
            showing: None,
            queue: VecDeque::new(),
            banners: BTreeSet::new(),
            focused_app: None,
            locked: false,
            main_panel: false,
            main_covered: false,
        }
    }

    pub fn set_timings(&mut self, timings: Timings, marks_limit: usize) {
        self.timings = timings;
        self.marks_limit = marks_limit;
    }

    pub fn set_glyph(&mut self, kind: GlyphKind, glyph: Option<Glyph>) {
        match glyph {
            Some(glyph) => self.glyphs.insert(kind, glyph),
            None => self.glyphs.remove(&kind),
        };
    }

    pub fn glyph(&self, kind: GlyphKind) -> Option<&Glyph> {
        self.glyphs.get(&kind)
    }

    pub fn set_tier(&mut self, kind: GlyphKind, tier: Tier) {
        self.tiers.insert(kind, tier);
    }

    pub fn set_privacy(&mut self, mark: Privacy, on: bool) {
        if on {
            self.privacy.insert(mark);
        } else {
            self.privacy.remove(&mark);
        }
    }

    /// Starts or updates the activity of its kind; one that starts and leads announces itself.
    pub fn set_activity(&mut self, activity: Activity, now: Instant) {
        let kind = activity.kind;
        match self.activities.iter_mut().find(|a| a.kind == kind) {
            Some(current) => *current = activity,
            None => {
                self.hidden.remove(&kind);
                self.activities.push(activity.clone());
                if self.leader().is_some_and(|l| l.kind == kind) {
                    self.post(
                        Event {
                            kind: EventKind::Start,
                            form: Form::StartRow(activity),
                            output: None,
                            app: None,
                            urgent: false,
                        },
                        now,
                    );
                }
            }
        }
    }

    pub fn end_activity(&mut self, kind: ActivityKind) {
        self.activities.retain(|a| a.kind != kind);
        self.hidden.remove(&kind);
    }

    /// Hides an activity's face until it stops and starts again.
    pub fn hide_activity(&mut self, kind: ActivityKind) {
        if !matches!(
            kind,
            ActivityKind::Prompt | ActivityKind::Recording | ActivityKind::Sharing
        ) {
            self.hidden.insert(kind);
        }
    }

    pub fn activities(&self) -> impl Iterator<Item = &Activity> {
        self.activities.iter()
    }

    /// The activity that leads: the highest rank, then the most recently started.
    pub fn leader(&self) -> Option<&Activity> {
        self.visible_activities().next()
    }

    fn visible_activities(&self) -> impl Iterator<Item = &Activity> {
        let mut shown: Vec<&Activity> = self
            .activities
            .iter()
            .filter(|a| !self.hidden.contains(&a.kind))
            .collect();
        shown.sort_by(|a, b| a.kind.cmp(&b.kind).then(b.started.cmp(&a.started)));
        shown.into_iter()
    }

    pub fn set_banner(&mut self, app: &str, showing: bool) {
        if showing {
            self.banners.insert(app.to_owned());
        } else {
            self.banners.remove(app);
        }
    }

    /// The app in front, which ends an urgent attention event for it.
    pub fn set_focused_app(&mut self, app: Option<String>) {
        if let Some(app) = &app {
            let about = |e: &Event| e.kind == EventKind::Attention && e.app.as_ref() == Some(app);
            self.queue.retain(|(e, _)| !about(e));
            if self.showing.as_ref().is_some_and(|(e, _)| about(e)) {
                self.showing = None;
            }
        }
        self.focused_app = app;
    }

    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    pub fn set_main(&mut self, panel: bool, covered: bool, now: Instant) {
        let reopened = self.main_panel && !panel;
        self.main_panel = panel;
        self.main_covered = covered;
        if reopened {
            // What waited past its own time while the panel was open is gone.
            self.advance(now);
        }
    }

    /// Takes an event in by the queue's rules.
    pub fn post(&mut self, event: Event, now: Instant) {
        if event.kind == EventKind::Attention
            && !event.urgent
            && event.app.as_ref().is_some_and(|a| self.banners.contains(a))
        {
            return;
        }
        // Over full screen and the lock, only OSDs show; the rest is dropped, not held.
        if (self.locked || self.main_covered) && event.kind != EventKind::Osd {
            return;
        }
        // An OSD answers a key pressed now: it pre-empts at once and never queues.
        if event.kind == EventKind::Osd {
            if self
                .showing
                .as_ref()
                .is_some_and(|(e, _)| e.kind != EventKind::Osd)
            {
                let (bumped, _) = self.showing.take().unwrap();
                self.queue.push_front((bumped, now));
            }
            self.showing = Some((event, now));
            return;
        }
        // Same kind replaces, in place, restarting its time.
        if let Some((showing, at)) = self.showing.as_mut().filter(|(e, _)| e.kind == event.kind) {
            *showing = event;
            *at = now;
            return;
        }
        if let Some(queued) = self.queue.iter_mut().find(|(e, _)| e.kind == event.kind) {
            *queued = (event, now);
            return;
        }
        self.queue.push_back((event, now));
        self.advance(now);
    }

    /// The showing event was acted on: a timer's end acknowledged, attention clicked.
    pub fn dismiss(&mut self, now: Instant) {
        self.showing = None;
        self.advance(now);
    }

    /// Moves time on: ends what has shown its time, shows what waits.
    pub fn tick(&mut self, now: Instant) {
        if let Some((event, at)) = &self.showing
            && let Some(length) = self.timings.duration(event)
            && now.duration_since(*at) >= length
        {
            self.showing = None;
        }
        self.advance(now);
    }

    fn expire(&mut self, now: Instant) {
        let timings = self.timings;
        let focused = self.focused_app.clone();
        self.queue
            .retain(|(event, arrived)| match timings.duration(event) {
                Some(length) => now.duration_since(*arrived) < length,
                // A held event stays while its app is not in front.
                None => event.app.is_none() || event.app != focused,
            });
    }

    fn advance(&mut self, now: Instant) {
        self.expire(now);
        if self.showing.is_some() || self.main_panel {
            return;
        }
        if let Some((event, _)) = self.queue.pop_front() {
            self.showing = Some((event, now));
        }
    }

    /// When the island next changes by itself: an event's end or a queued one's drop.
    pub fn next_change(&self) -> Option<Instant> {
        let ends = self
            .showing
            .iter()
            .chain(self.queue.iter())
            .filter_map(|(e, at)| self.timings.duration(e).map(|d| *at + d));
        ends.min()
    }

    pub fn showing(&self) -> Option<&Event> {
        self.showing.as_ref().map(|(e, _)| e)
    }

    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    fn shown_glyphs(&self) -> Vec<(GlyphKind, Glyph)> {
        GlyphKind::ALL
            .into_iter()
            .filter_map(|kind| {
                let glyph = self.glyphs.get(&kind)?;
                let tier = if kind.tiered() {
                    self.tiers.get(&kind).copied().unwrap_or_default()
                } else {
                    Tier::WhenActive
                };
                let shows = match tier {
                    Tier::Always => true,
                    Tier::WhenActive => glyph.active,
                    Tier::Never => false,
                };
                shows.then(|| (kind, glyph.clone()))
            })
            .collect()
    }

    /// What the island on an output in `place` shows. `output` names it, for an event aimed
    /// at one output.
    pub fn view(&self, output: &str, place: Place) -> Shown {
        let privacy: Vec<Privacy> = self.privacy.iter().copied().collect();
        let event = self
            .showing
            .as_ref()
            .map(|(e, _)| e)
            .filter(|e| match &e.output {
                Some(target) => target == output,
                None => place.main,
            });
        if self.locked || place.covered {
            return match event.filter(|e| e.kind == EventKind::Osd) {
                Some(osd) => Shown::Alone(osd.clone(), privacy),
                None if privacy.is_empty() => Shown::Nothing,
                None => Shown::Privacy(privacy),
            };
        }
        // An OSD while the panel is open shows inside the panel; other events wait.
        if !place.panel
            && let Some(event) = event
        {
            return Shown::Event(event.clone(), privacy);
        }
        let (leader, marks, more) = if place.main {
            let mut visible = self.visible_activities();
            let leader = visible.next().cloned();
            let rest: Vec<ActivityKind> = visible.map(|a| a.kind).collect();
            if rest.len() > self.marks_limit {
                (leader, Vec::new(), rest.len())
            } else {
                (leader, rest, 0)
            }
        } else {
            (None, Vec::new(), 0)
        };
        Shown::Pill {
            leader,
            marks,
            more,
            glyphs: self.shown_glyphs(),
            privacy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timings() -> Timings {
        let ms = Duration::from_millis;
        Timings {
            osd: ms(1500),
            copied: ms(2000),
            mode: ms(2000),
            event: ms(3000),
            start_media: ms(4000),
            ended: ms(5000),
        }
    }

    fn island() -> Island {
        Island::new(timings(), 2)
    }

    fn line(kind: EventKind, text: &str) -> Event {
        Event {
            kind,
            form: Form::Line {
                symbol: "check".into(),
                fill: false,
                text: text.into(),
            },
            output: None,
            app: None,
            urgent: false,
        }
    }

    fn activity(kind: ActivityKind, started: Instant) -> Activity {
        Activity {
            kind,
            face: format!("{kind:?}"),
            title: String::new(),
            detail: String::new(),
            actions: Vec::new(),
            app: None,
            started,
        }
    }

    const MAIN: Place = Place {
        main: true,
        covered: false,
        panel: false,
    };

    fn text(shown: &Shown) -> Option<String> {
        match shown {
            Shown::Event(
                Event {
                    form: Form::Line { text, .. },
                    ..
                },
                _,
            ) => Some(text.clone()),
            Shown::Alone(
                Event {
                    form: Form::Line { text, .. },
                    ..
                },
                _,
            ) => Some(text.clone()),
            _ => None,
        }
    }

    #[test]
    fn glyphs_show_only_when_active_unless_tiered_otherwise() {
        let mut island = island();
        let glyph = |active| Glyph {
            active,
            symbol: "wifi-high".into(),
            ..Default::default()
        };
        island.set_glyph(GlyphKind::Wifi, Some(glyph(false)));
        island.set_glyph(GlyphKind::Sound, Some(glyph(true)));
        island.set_glyph(GlyphKind::Inhibited, Some(glyph(true)));
        let kinds = |island: &Island| match island.view("A", MAIN) {
            Shown::Pill { glyphs, .. } => glyphs.into_iter().map(|(k, _)| k).collect::<Vec<_>>(),
            other => panic!("{other:?}"),
        };
        assert_eq!(kinds(&island), [GlyphKind::Sound, GlyphKind::Inhibited]);
        island.set_tier(GlyphKind::Wifi, Tier::Always);
        island.set_tier(GlyphKind::Sound, Tier::Never);
        island.set_tier(GlyphKind::Inhibited, Tier::Never);
        assert_eq!(
            kinds(&island),
            [GlyphKind::Wifi, GlyphKind::Inhibited],
            "never is not offered for the untiered glyphs"
        );
    }

    #[test]
    fn events_queue_one_at_a_time_and_drop_after_waiting_their_length() {
        let t0 = Instant::now();
        let mut island = island();
        island.post(line(EventKind::Copied, "Copied"), t0);
        island.post(line(EventKind::Mode, "Do Not Disturb on"), t0);
        assert_eq!(text(&island.view("A", MAIN)).as_deref(), Some("Copied"));
        island.tick(t0 + Duration::from_millis(2000));
        assert_eq!(
            text(&island.view("A", MAIN)).as_deref(),
            None,
            "the mode change waited its own two seconds and is dropped"
        );
        assert_eq!(island.queued(), 0);
    }

    #[test]
    fn the_same_kind_replaces_in_place_and_restarts_its_time() {
        let t0 = Instant::now();
        let mut island = island();
        island.post(line(EventKind::Copied, "Copied"), t0);
        island.post(
            line(EventKind::Copied, "Colour copied"),
            t0 + Duration::from_millis(1500),
        );
        island.tick(t0 + Duration::from_millis(3000));
        assert_eq!(
            text(&island.view("A", MAIN)).as_deref(),
            Some("Colour copied")
        );
        assert_eq!(island.queued(), 0);
    }

    #[test]
    fn an_osd_pre_empts_and_what_it_bumped_comes_back() {
        let t0 = Instant::now();
        let mut island = island();
        island.post(line(EventKind::Ended, "Recording stopped"), t0);
        island.post(
            line(EventKind::Osd, "Volume"),
            t0 + Duration::from_millis(100),
        );
        assert_eq!(text(&island.view("A", MAIN)).as_deref(), Some("Volume"));
        island.tick(t0 + Duration::from_millis(1700));
        assert_eq!(
            text(&island.view("A", MAIN)).as_deref(),
            Some("Recording stopped")
        );
    }

    #[test]
    fn events_wait_while_the_panel_is_open_but_an_osd_shows() {
        let t0 = Instant::now();
        let mut island = island();
        island.set_main(true, false, t0);
        island.post(line(EventKind::Copied, "Copied"), t0);
        let panel = Place {
            panel: true,
            ..MAIN
        };
        assert!(matches!(island.view("A", panel), Shown::Pill { .. }));
        island.set_main(false, false, t0 + Duration::from_millis(500));
        assert_eq!(text(&island.view("A", MAIN)).as_deref(), Some("Copied"));

        island.set_main(true, false, t0);
        island.post(line(EventKind::Osd, "Volume"), t0);
        assert!(island.showing().is_some_and(|e| e.kind == EventKind::Osd));
    }

    #[test]
    fn over_full_screen_only_privacy_and_osds_show() {
        let t0 = Instant::now();
        let mut island = island();
        let covered = Place {
            covered: true,
            ..MAIN
        };
        assert_eq!(island.view("A", covered), Shown::Nothing);
        island.set_privacy(Privacy::Microphone, true);
        assert_eq!(
            island.view("A", covered),
            Shown::Privacy(vec![Privacy::Microphone])
        );
        island.set_main(false, true, t0);
        island.post(line(EventKind::Copied, "Copied"), t0);
        assert_eq!(island.queued(), 0, "dropped, not held");
        island.post(line(EventKind::Osd, "Volume"), t0);
        assert!(matches!(island.view("A", covered), Shown::Alone(..)));
    }

    #[test]
    fn activities_rank_and_past_the_limit_the_rest_become_a_count() {
        let t0 = Instant::now();
        let mut island = island();
        island.set_activity(activity(ActivityKind::Media, t0), t0);
        assert!(
            matches!(
                island.showing(),
                Some(Event {
                    kind: EventKind::Start,
                    ..
                })
            ),
            "a new leader announces itself"
        );
        island.set_activity(activity(ActivityKind::Timer, t0), t0);
        island.set_activity(activity(ActivityKind::Recording, t0), t0);
        island.set_activity(activity(ActivityKind::Call, t0), t0);
        island.tick(t0 + Duration::from_secs(60));
        match island.view("A", MAIN) {
            Shown::Pill {
                leader,
                marks,
                more,
                ..
            } => {
                assert_eq!(leader.unwrap().kind, ActivityKind::Recording);
                assert!(marks.is_empty());
                assert_eq!(more, 3);
            }
            other => panic!("{other:?}"),
        }
        island.end_activity(ActivityKind::Call);
        match island.view("A", MAIN) {
            Shown::Pill { marks, .. } => {
                assert_eq!(marks, [ActivityKind::Timer, ActivityKind::Media])
            }
            other => panic!("{other:?}"),
        }
        let second = Place {
            main: false,
            ..MAIN
        };
        assert!(matches!(
            island.view("B", second),
            Shown::Pill { leader: None, .. }
        ));
    }

    #[test]
    fn an_event_aimed_at_an_output_shows_there_alone() {
        let t0 = Instant::now();
        let mut island = island();
        let mut osd = line(EventKind::Osd, "Brightness");
        osd.output = Some("B".into());
        island.post(osd, t0);
        assert!(matches!(island.view("A", MAIN), Shown::Pill { .. }));
        let second = Place {
            main: false,
            ..MAIN
        };
        assert_eq!(
            text(&island.view("B", second)).as_deref(),
            Some("Brightness")
        );
    }

    #[test]
    fn attention_waits_on_a_banner_and_urgent_attention_holds_until_focus() {
        let t0 = Instant::now();
        let mut island = island();
        let attention = |urgent| Event {
            app: Some("chat".into()),
            urgent,
            ..line(EventKind::Attention, "chat needs your attention")
        };
        island.set_banner("chat", true);
        island.post(attention(false), t0);
        assert!(island.showing().is_none(), "the banner speaks for it");
        island.post(attention(true), t0);
        island.tick(t0 + Duration::from_secs(600));
        assert!(island.showing().is_some(), "urgent holds");
        island.set_focused_app(Some("chat".into()));
        assert!(island.showing().is_none());
    }
}
