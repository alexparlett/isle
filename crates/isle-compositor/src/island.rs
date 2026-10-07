//! The island on the loop: its model, which sources post to, each output's shape on its
//! springs, the clock, and the main screen (`p6-island-and-menus.md` choices 4–6, 9).

use std::{
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use isle_settings::schema::{ClockHours, GlyphTier};
use isle_shell::{
    island::{
        Frame, IslandSurface,
        model::{Glyph, GlyphKind, Island, Place, Shown, Tier, Timings},
        surface::{clock_width, target_size},
    },
    transition::Motion,
};
use smithay::{
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
};

use crate::state::{Isle, State};

/// One output's shape: what it shows, what it leaves, and its size on springs.
struct Morph {
    shown: Shown,
    before: Option<Shown>,
    panel: Option<isle_shell::island::panel::PanelFrame>,
    /// The panel was open when this morph began.
    panel_before: bool,
    width: Motion,
    height: Motion,
    /// The size the spring started from, for the share done.
    from: (f32, f32),
    last: Instant,
}

impl Morph {
    fn target(&self) -> (f32, f32) {
        (self.width.target() as f32, self.height.target() as f32)
    }

    fn settled(&self) -> bool {
        self.width.settled() && self.height.settled()
    }

    /// The share of the size change done, on whichever side changes more.
    fn progress(&self) -> f32 {
        let (tw, th) = self.target();
        let (dw, dh) = ((tw - self.from.0).abs(), (th - self.from.1).abs());
        let (now, total, from) = if dw >= dh {
            (self.width.value() as f32, tw - self.from.0, self.from.0)
        } else {
            (self.height.value() as f32, th - self.from.1, self.from.1)
        };
        if total.abs() < 0.5 {
            1.0
        } else {
            ((now - from) / total).clamp(0.0, 1.0)
        }
    }
}

pub struct IslandState {
    pub model: Island,
    pub surface: Rc<IslandSurface>,
    pub notice: Rc<isle_shell::island::notice::InhibitNotice>,
    notice_timer: Option<RegistrationToken>,
    morphs: HashMap<String, Morph>,
    clock: String,
    clock_width: f32,
    timer: Option<RegistrationToken>,
    /// The output events and faces show on, by name.
    pub main: Option<String>,
    /// Outputs where the pointer brought the island back over a full-screen window.
    pub revealed: Vec<String>,
    /// The sources as last taken in, by [`State::watch_island`].
    seen: Option<Seen>,
    /// Control Centre, open on one output.
    pub panel: Option<crate::control_centre::PanelState>,
    /// The app holding the keys, by id.
    pub inhibiting: Option<String>,
    /// The seat keymap's layouts by name, and the one in use.
    pub layouts: (Vec<String>, usize),
    /// Resumes from sleep that re-read the clock.
    resumes: u64,
}

type Seen = (Option<String>, Vec<bool>, String, bool, Option<String>);

impl IslandState {
    pub fn new(theme: &isle_tokens::Theme) -> Self {
        Self {
            model: Island::new(Timings::of(theme), theme.tokens.limit_island_marks as usize),
            surface: IslandSurface::new(),
            notice: isle_shell::island::notice::InhibitNotice::new(),
            notice_timer: None,
            morphs: HashMap::new(),
            clock: String::new(),
            clock_width: 0.0,
            timer: None,
            main: None,
            revealed: Vec::new(),
            seen: None,
            panel: None,
            inhibiting: None,
            layouts: (Vec::new(), 0),
            resumes: 0,
        }
    }
}

/// Whether the locale writes times in 24 hours: its time format has `%H`, `%T` or `%R`.
fn locale_24_hour() -> bool {
    // Safety: setlocale and nl_langinfo are called from the loop alone; the string is
    // copied before anything else could change the locale.
    unsafe {
        libc::setlocale(libc::LC_TIME, c"".as_ptr());
        let format = libc::nl_langinfo(libc::T_FMT);
        if format.is_null() {
            return true;
        }
        let format = std::ffi::CStr::from_ptr(format)
            .to_string_lossy()
            .into_owned();
        format.is_empty() || ["%H", "%T", "%R", "%k"].iter().any(|c| format.contains(c))
    }
}

/// The clock's strftime format, and the widest string it can make.
fn clock_format(settings: &isle_settings::schema::Island) -> (String, String) {
    let hours = match settings.clock_24_hour {
        ClockHours::On => true,
        ClockHours::Off => false,
        ClockHours::Auto => locale_24_hour(),
    };
    let mut format = String::new();
    let mut widest = String::new();
    if settings.clock_weekday {
        format.push_str("%a ");
        widest.push_str("Wed ");
    }
    if settings.clock_date {
        format.push_str("%-d %b ");
        widest.push_str("28 May ");
    }
    if hours {
        format.push_str("%H:%M");
        widest.push_str("88:88");
    } else {
        format.push_str("%-I:%M");
        widest.push_str("88:88");
    }
    if settings.clock_seconds {
        format.push_str(":%S");
        widest.push_str(":88");
    }
    if !hours {
        format.push_str(" %p");
        widest.push_str(" PM");
    }
    (format, widest)
}

fn tier(t: GlyphTier) -> Tier {
    match t {
        GlyphTier::Always => Tier::Always,
        GlyphTier::WhenActive => Tier::WhenActive,
        GlyphTier::Never => Tier::Never,
    }
}

impl Isle {
    /// The main screen: the one settings name, else the focused window's, else the
    /// pointer's, else the first.
    pub fn main_output(&self) -> Option<Output> {
        if let Some(identity) = &self.settings.shell.main_screen
            && let Some(output) = self
                .space
                .outputs()
                .find(|o| &Self::output_identity(o) == identity)
        {
            return Some(output.clone());
        }
        self.output_with_focus()
            .or_else(|| self.active_output())
            .or_else(|| self.space.outputs().next().cloned())
    }

    /// The clock's text now, and the settings' tiers, into the model.
    /// After sleep: the clock read again and its timer armed from now.
    pub fn island_resumed(&mut self) {
        self.island.resumes += 1;
        self.island_settings();
        self.refresh_island();
    }

    pub fn island_settings(&mut self) {
        let (format, widest) = clock_format(&self.settings.island);
        let now = jiff::Zoned::now();
        self.island.clock = now.strftime(&format).to_string();
        self.island.clock_width = clock_width(self.shell.theme(), &widest);
        let g = &self.settings.island_glyphs;
        for (kind, t) in [
            (GlyphKind::Wifi, g.wifi),
            (GlyphKind::Bluetooth, g.bluetooth),
            (GlyphKind::Sound, g.sound),
            (GlyphKind::Vpn, g.vpn),
            (GlyphKind::Focus, g.focus),
            (GlyphKind::Updates, g.updates),
            (GlyphKind::Battery, g.battery),
        ] {
            self.island.model.set_tier(kind, tier(t));
        }
        let theme = self.shell.theme().clone();
        self.island.model.set_timings(
            Timings::of(&theme),
            theme.tokens.limit_island_marks as usize,
        );
    }

    /// Works out what every output's island shows and starts its springs toward it.
    pub fn refresh_island(&mut self) {
        let now = Instant::now();
        self.island.model.tick(now);
        let theme = self.shell.theme().clone();
        let animated = self.shell.animated() && !theme.appearance.reduce_motion;
        let main = self.main_output().map(|o| o.name());
        self.island.main = main.clone();
        let outputs: Vec<Output> = self.space.outputs().cloned().collect();
        let main_covered = outputs
            .iter()
            .find(|o| Some(o.name()) == main)
            .is_some_and(|o| self.island_covered(o));
        let panel = self.island.panel.clone();
        let main_panel = panel.as_ref().is_some_and(|p| Some(&p.output) == main.as_ref());
        self.island.model.set_main(main_panel, main_covered, now);
        for output in &outputs {
            let name = output.name();
            let open = panel.as_ref().filter(|p| p.output == name);
            let place = Place {
                main: Some(&name) == main.as_ref(),
                covered: self.island_covered(output) && open.is_none(),
                panel: open.is_some(),
            };
            let shown = self.island.model.view(&name, place);
            let mut panel_frame = open.map(|p| self.panel_frame(p));
            let room = self.space.output_geometry(output).map_or(800.0, |g| {
                g.size.h as f32 - theme.tokens.size_strip - theme.tokens.space_xxxl
            });
            let target = target_size(
                &shown,
                panel_frame.as_ref(),
                &theme,
                self.island.clock_width,
                room,
            );
            if let Some(frame) = panel_frame.as_mut() {
                frame.body = target.1 - theme.tokens.size_island_height;
            }
            let morph = self
                .island
                .morphs
                .entry(name.clone())
                .or_insert_with(|| Morph {
                    shown: Shown::Nothing,
                    before: None,
                    width: Motion::at(0.0),
                    height: Motion::at(0.0),
                    from: (0.0, 0.0),
                    last: now,
                    panel: None,
                    panel_before: false,
                });
            let panel_changed = morph.panel.is_some() != panel_frame.is_some();
            if panel_changed {
                morph.panel_before = morph.panel.is_some();
            }
            let content_changed = morph.panel != panel_frame;
            morph.panel = panel_frame;
            if content_changed && !panel_changed {
                morph.last = now;
            }
            if morph.shown != shown || morph.target() != target || panel_changed {
                let same_form = std::mem::discriminant(&morph.shown)
                    == std::mem::discriminant(&shown)
                    && matches!(shown, Shown::Pill { .. } | Shown::Privacy(_));
                morph.before =
                    (!same_form && morph.shown != Shown::Nothing).then(|| morph.shown.clone());
                morph.from = (morph.width.value() as f32, morph.height.value() as f32);
                let growing = target.0 * target.1 >= morph.from.0 * morph.from.1;
                let spring = if growing {
                    theme.spring_morph()
                } else {
                    theme.spring_exit()
                };
                if animated && morph.shown != Shown::Nothing {
                    morph.width.spring_to(target.0 as f64, spring);
                    morph.height.spring_to(target.1 as f64, spring);
                } else {
                    morph.width = Motion::at(target.0 as f64);
                    morph.height = Motion::at(target.1 as f64);
                    morph.before = None;
                }
                morph.shown = shown;
                morph.last = now;
            }
        }
        self.island
            .morphs
            .retain(|name, _| outputs.iter().any(|o| &o.name() == name));
        self.push_island_frames();
        self.arm_island_timer();
        if !self.banners.model.idle() || self.shell.is_open("banners") {
            self.refresh_banners();
        }
        for output in &outputs {
            self.queue_redraw(output);
        }
    }

    /// The app of the window with keyboard focus.
    pub fn focused_app_name(&self) -> Option<String> {
        self.seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .and_then(|f| {
                f.surface()
                    .and_then(|s| self.wm.for_surface(s))
                    .map(|r| r.app.clone())
            })
    }

    /// Whether `output` shows a full-screen window that hides the island.
    fn island_covered(&self, output: &Output) -> bool {
        let identity = Self::output_identity(output);
        let full_screen = self.desktops.rows.get(&identity).is_some_and(|row| {
            row.desktops
                .get(row.current)
                .is_some_and(|d| matches!(d.kind, crate::wm::desktops::Kind::FullScreen { .. }))
        });
        full_screen && !self.island.revealed.contains(&output.name())
    }

    /// Moves every shape on to `now`; true while any still moves.
    pub fn step_island(&mut self, now: Instant) -> bool {
        let mut moving = false;
        for morph in self.island.morphs.values_mut() {
            if morph.settled() {
                if morph.before.is_some() {
                    morph.before = None;
                    moving = true;
                }
                continue;
            }
            let dt = now.duration_since(morph.last).as_secs_f64().min(0.05);
            morph.last = now;
            morph.width.step(now, dt);
            morph.height.step(now, dt);
            moving = true;
        }
        if moving {
            self.push_island_frames();
        }
        moving || self.island.morphs.values().any(|m| !m.settled())
    }

    /// The island's shape on `output` now, in points.
    pub fn island_size(&self, output: &Output) -> Option<(f32, f32)> {
        self.island
            .morphs
            .get(&output.name())
            .map(|m| (m.width.value() as f32, m.height.value() as f32))
    }

    pub fn island_moving(&self, output: &Output) -> bool {
        self.island
            .morphs
            .get(&output.name())
            .is_some_and(|m| !m.settled() || m.before.is_some())
    }

    fn push_island_frames(&mut self) {
        let theme = self.shell.theme().clone();
        let solid = theme.appearance.reduce_transparency || self.island_game_mode();
        let mut frames = HashMap::new();
        for (name, morph) in &self.island.morphs {
            frames.insert(
                name.clone(),
                Frame {
                    shown: morph.shown.clone(),
                    before: morph.before.clone(),
                    p: morph.progress(),
                    width: morph.width.value().max(0.0) as f32,
                    height: morph.height.value().max(0.0) as f32,
                    clock: self.island.clock.clone(),
                    clock_width: self.island.clock_width,
                    solid,
                    panel: morph.panel.clone(),
                    panel_before: morph.panel_before && !morph.settled(),
                },
            );
        }
        let surface = self.island.surface.clone();
        for (name, frame) in &frames {
            surface.set_model(name, frame.clone());
        }
        self.shell.with_roots("island", |output| {
            if let Some(frame) = frames.get(&output.name()) {
                surface.set(&output.name(), frame.clone());
            }
        });
        // An island with nothing to show adds nothing to the frame, so a full-screen game
        // can be scanned out directly.
        let outputs: Vec<Output> = self.space.outputs().cloned().collect();
        for output in outputs {
            let empty = frames.get(&output.name()).is_none_or(|f| {
                f.width < 1.0 && f.before.is_none() && matches!(f.shown, Shown::Nothing)
            });
            if self.shell.is_open_on("island", &output) == empty {
                self.shell.set_open_on("island", &output, !empty);
            }
        }
    }

    /// Game Mode: over a game in front, solid and still.
    pub fn island_game_mode(&self) -> bool {
        self.seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .and_then(|f| {
                f.surface()
                    .and_then(|s| self.wm.for_surface(s))
                    .map(|r| r.game)
            })
            .unwrap_or(false)
    }

    /// One timer for the island's next change by itself: an event's end or the clock's next
    /// minute (or second, with seconds shown).
    fn arm_island_timer(&mut self) {
        if let Some(token) = self.island.timer.take() {
            self.loop_handle.remove(token);
        }
        let now = Instant::now();
        let step = if self.settings.island.clock_seconds {
            1
        } else {
            60
        };
        let wall = jiff::Timestamp::now();
        let into = wall.as_second().rem_euclid(step) as f64 + wall.subsec_nanosecond() as f64 / 1e9;
        let to_tick = Duration::from_secs_f64(step as f64 - into + 0.001);
        let next = self
            .island
            .model
            .next_change()
            .map(|at| at.saturating_duration_since(now))
            .map_or(to_tick, |d| d.min(to_tick));
        let token = self
            .loop_handle
            .insert_source(Timer::from_duration(next), |_, _, state| {
                let _beat = state.isle.heartbeat.beat("island");
                state.isle.island.timer = None;
                state.isle.island_settings();
                state.isle.refresh_island();
                TimeoutAction::Drop
            })
            .ok();
        self.island.timer = token;
    }
}

impl State {
    /// The glyphs whose state the compositor itself knows: shortcuts inhibited, a rejected
    /// configuration, and the Gaming Focus standing in while a game is in front.
    pub fn island_sources(&mut self, settings_error: Option<Option<(String, u32, u32, String)>>) {
        let app = self.inhibiting_app();
        self.isle.island.inhibiting = (!app.is_empty()).then(|| app.clone());
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        self.isle.island.layouts = keyboard.with_xkb_state(self, |context| {
            let xkb = context.xkb().lock().unwrap();
            let names = xkb.layouts().map(|l| xkb.layout_name(l).to_owned()).collect();
            (names, xkb.active_layout().0 as usize)
        });
        let inhibited = (!app.is_empty()).then(|| self.isle.app_name(&app));
        self.isle.island.model.set_glyph(
            GlyphKind::Inhibited,
            inhibited.map(|app| Glyph {
                active: true,
                symbol: "isle-keyboard-slash".into(),
                alt: format!("{app} is receiving all keyboard shortcuts"),
                ..Default::default()
            }),
        );
        if let Some(error) = settings_error {
            self.isle.settings_error = error;
        }
        let error = self.isle.settings_error.is_some()
            || self.isle.tokens_error.is_some()
            || self.isle.safe_start;
        self.isle.island.model.set_glyph(
            GlyphKind::ConfigError,
            error.then(|| Glyph {
                active: true,
                symbol: "warning-circle".into(),
                fill: true,
                alt: "Configuration error".into(),
                ..Default::default()
            }),
        );
        let game = self.focus_is_game();
        self.isle.island.model.set_glyph(
            GlyphKind::Focus,
            game.then(|| Glyph {
                active: true,
                symbol: "game-controller".into(),
                fill: true,
                alt: "Game Mode".into(),
                ..Default::default()
            }),
        );
    }

    /// A window asked for focus it could not take: its record is marked and the island told.
    pub fn want_attention(&mut self, id: u64, urgent: bool) {
        let Some(record) = self.isle.wm.get_mut(id) else {
            return;
        };
        record.attention = true;
        record.urgent |= urgent;
        let app = record.app.clone();
        self.isle.notify(crate::worker::Event::WindowsChanged);
        self.island_attention(app, urgent);
    }

    /// An app wants attention: the island's event naming it.
    pub fn island_attention(&mut self, app: String, urgent: bool) {
        let name = self.isle.app_name(&app);
        self.isle.island.model.post(
            isle_shell::island::model::Event {
                kind: isle_shell::island::model::EventKind::Attention,
                form: isle_shell::island::model::Form::Line {
                    symbol: "app-window".into(),
                    fill: false,
                    text: format!("{name} needs your attention"),
                },
                output: None,
                app: Some(app),
                urgent,
            },
            Instant::now(),
        );
        self.isle.refresh_island();
    }

    /// The layout changed by its chord: an OSD with its name.
    pub fn island_layout(&mut self) {
        self.island_sources(None);
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let name = keyboard.with_xkb_state(self, |context| {
            let xkb = context.xkb().lock().unwrap();
            let layout = xkb.active_layout();
            xkb.layout_name(layout).to_owned()
        });
        self.isle.island.model.post(
            isle_shell::island::model::Event {
                kind: isle_shell::island::model::EventKind::Osd,
                form: isle_shell::island::model::Form::Line {
                    symbol: "keyboard".into(),
                    fill: false,
                    text: name,
                },
                output: None,
                app: None,
                urgent: false,
            },
            Instant::now(),
        );
        self.isle.refresh_island();
    }

    /// Takes the island's sources in when any changed: the main screen, what covers each
    /// screen, who holds the keys, a game in front. Cheap, so every redraw asks.
    pub fn watch_island(&mut self) {
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        let seen = (
            self.isle.main_output().map(|o| o.name()),
            outputs
                .iter()
                .map(|o| self.isle.island_covered(o))
                .collect::<Vec<_>>(),
            self.inhibiting_app(),
            self.focus_is_game(),
            self.isle.focused_app_name(),
        );
        let gained = !seen.2.is_empty()
            && self.isle.island.seen.as_ref().is_none_or(|s| s.2 != seen.2);
        if gained {
            self.show_inhibit_notice(&seen.2);
        } else if seen.2.is_empty() && self.isle.shell.is_open("inhibit-notice") {
            self.close_surface("inhibit-notice");
        }
        if self.isle.island.seen.as_ref() != Some(&seen) {
            if self.isle.island.seen.as_ref().is_some_and(|s| s.4 != seen.4) {
                self.refresh_bar();
            }
            self.isle.island.model.set_focused_app(seen.4.clone());
            self.isle.island.seen = Some(seen);
            self.island_sources(None);
            self.isle.refresh_island();
        }
    }

    /// Once each time a window gains the keys: a notice inside its top edge naming the chord
    /// that takes them back.
    fn show_inhibit_notice(&mut self, app: &str) {
        let Some(id) = self.focused_window() else {
            return;
        };
        let (Some(frame), Some(output)) = (self.isle.frame(id), self.isle.output_of(id)) else {
            return;
        };
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let theme = self.isle.shell.theme().clone();
        let name = self.isle.app_name(app);
        let chord = "⌘⎋".to_owned();
        let size = isle_shell::island::notice::InhibitNotice::size(&theme, &name, &chord);
        let full_screen = matches!(
            self.isle.wm.get(id).map(|r| &r.mode),
            Some(crate::wm::Mode::FullScreen { .. })
        );
        let top = if full_screen {
            screen.loc.y + theme.tokens.size_strip as i32
        } else {
            frame.loc.y + self.isle.bar_height(id) + theme.tokens.space_m as i32
        };
        let x = frame.loc.x + (frame.size.w - size.w) / 2;
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        let notice = self.isle.island.notice.clone();
        self.isle.shell.with_root("inhibit-notice", || notice.set(name.clone(), chord.clone()));
        notice.set(name, chord);
        self.isle.shell.open_placed(
            "inhibit-notice",
            &place,
            smithay::utils::Rectangle::new((x - screen.loc.x, top - screen.loc.y).into(), size),
        );
        if let Some(token) = self.isle.island.notice_timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let hold = Duration::from_millis(theme.tokens.timing_event_ms.into());
        self.isle.island.notice_timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(hold), |_, _, state| {
                state.isle.island.notice_timer = None;
                state.close_surface("inhibit-notice");
                TimeoutAction::Drop
            })
            .ok();
        self.dispatch_shell();
    }

    /// What a click on an output's island asked for.
    pub fn island_action(
        &mut self,
        focus: Option<isle_shell::ShellFocus>,
        action: isle_shell::island::IslandAction,
    ) {
        use isle_shell::island::IslandAction;
        match action {
            IslandAction::Pin => {
                if let Some(output) = focus.as_ref().and_then(|f| self.isle.shell.output_of(f)) {
                    self.open_control_centre(&output, true, None);
                }
            }
            IslandAction::Clock => {
                if let Some(output) = focus.as_ref().and_then(|f| self.isle.shell.output_of(f)) {
                    self.open_centre_on(&output);
                }
            }
            IslandAction::Glyph(kind) => {
                if let Some(output) = focus.as_ref().and_then(|f| self.isle.shell.output_of(f)) {
                    let page = match kind {
                        isle_shell::island::model::GlyphKind::Inhibited
                        | isle_shell::island::model::GlyphKind::ConfigError => None,
                        isle_shell::island::model::GlyphKind::Battery => Some("batteries".into()),
                        other => Some(other.name().to_owned()),
                    };
                    self.open_control_centre(&output, true, page);
                }
            }
            IslandAction::Panel(action) => self.panel_action(action),
            IslandAction::Close => {
                let page = self.isle.island.panel.as_ref().and_then(|p| p.page.clone());
                if let Some(page) = page {
                    let parent = isle_shell::island::panel::parent_page(&page);
                    self.panel_action(isle_shell::island::panel::PanelAction::Page(parent));
                } else {
                    self.close_control_centre();
                }
            }
            IslandAction::Event => {
                // Attention's app comes forward; a held event is acknowledged.
                let app = self.isle.island.model.showing().and_then(|e| e.app.clone());
                self.isle.island.model.dismiss(Instant::now());
                if let Some(app) = app {
                    self.activate_app(&app);
                }
                self.isle.refresh_island();
            }
        }
    }

    /// Takes the island's settings and sources in and redraws what changed.
    pub fn sync_island(&mut self) {
        self.isle.island_settings();
        self.island_sources(None);
        self.isle.refresh_island();
    }

    /// The configuration was rejected, or a rejected one fixed.
    pub fn island_settings_status(&mut self, error: Option<(String, u32, u32, String)>) {
        self.island_sources(Some(error));
        self.isle.refresh_island();
    }

    /// Posts to the island from JSON, as a service will: `{"glyph": {...}}`,
    /// `{"privacy": {...}}`, `{"activity": {...}}`, `{"end": "media"}`, `{"event": {...}}`.
    pub fn island_post(&mut self, json: &str) -> Result<(), String> {
        use isle_shell::island::model::{Activity, ActivityKind, Event, EventKind, Form, Privacy};
        let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let text = |v: &serde_json::Value, key: &str| {
            v.get(key).and_then(|s| s.as_str()).unwrap_or("").to_owned()
        };
        let now = Instant::now();
        let model = &mut self.isle.island.model;
        if let Some(g) = value.get("glyph") {
            let kind = GlyphKind::from_name(&text(g, "kind")).ok_or("no such glyph")?;
            let glyph = g.get("active").map(|_| Glyph {
                active: g["active"].as_bool().unwrap_or(false),
                symbol: text(g, "symbol"),
                level: g.get("level").and_then(|l| l.as_str()).map(str::to_owned),
                fill: g.get("fill").and_then(|f| f.as_bool()).unwrap_or(false),
                alt: text(g, "alt"),
            });
            model.set_glyph(kind, glyph);
        } else if let Some(p) = value.get("privacy") {
            let mark = match text(p, "mark").as_str() {
                "microphone" => Privacy::Microphone,
                "camera" => Privacy::Camera,
                "screen" => Privacy::Screen,
                _ => return Err("no such privacy mark".into()),
            };
            model.set_privacy(mark, p.get("on").and_then(|o| o.as_bool()).unwrap_or(true));
        } else if let Some(a) = value.get("activity") {
            let kind = ActivityKind::from_name(&text(a, "kind")).ok_or("no such activity")?;
            model.set_activity(
                Activity {
                    kind,
                    face: text(a, "face"),
                    title: text(a, "title"),
                    detail: text(a, "detail"),
                    actions: Vec::new(),
                    app: a.get("app").and_then(|x| x.as_str()).map(str::to_owned),
                    art: None,
                    started: now,
                },
                now,
            );
        } else if let Some(kind) = value.get("end").and_then(|e| e.as_str()) {
            model.end_activity(ActivityKind::from_name(kind).ok_or("no such activity")?);
        } else if let Some(e) = value.get("event") {
            let kind = EventKind::from_name(&text(e, "kind")).ok_or("no such event")?;
            let form = match e.get("level").and_then(|l| l.as_f64()) {
                Some(level) => Form::Level {
                    symbol: text(e, "symbol"),
                    name: text(e, "name"),
                    level: level as f32,
                    muted: e.get("muted").and_then(|m| m.as_bool()).unwrap_or(false),
                },
                None => Form::Line {
                    symbol: text(e, "symbol"),
                    fill: e.get("fill").and_then(|f| f.as_bool()).unwrap_or(false),
                    text: text(e, "text"),
                },
            };
            model.post(
                Event {
                    kind,
                    form,
                    output: e.get("output").and_then(|o| o.as_str()).map(str::to_owned),
                    app: e.get("app").and_then(|o| o.as_str()).map(str::to_owned),
                    urgent: e.get("urgent").and_then(|u| u.as_bool()).unwrap_or(false),
                },
                now,
            );
        } else {
            return Err("nothing to post".into());
        }
        self.isle.refresh_island();
        Ok(())
    }
}

impl Isle {
    /// Each output's island as JSON: what it shows, its shape's size and target, the clock.
    pub fn island_status(&self) -> String {
        use isle_shell::island::model::{Form, Shown};
        let outputs: Vec<serde_json::Value> = self
            .space
            .outputs()
            .map(|o| {
                let name = o.name();
                let morph = self.island.morphs.get(&name);
                let (form, text, glyphs) = match morph.map(|m| &m.shown) {
                    None | Some(Shown::Nothing) => ("nothing", String::new(), Vec::new()),
                    Some(Shown::Privacy(_)) => ("privacy", String::new(), Vec::new()),
                    Some(Shown::Pill { leader, glyphs, .. }) => (
                        "pill",
                        leader.as_ref().map(|l| l.face.clone()).unwrap_or_default(),
                        glyphs.iter().map(|(k, _)| k.name()).collect(),
                    ),
                    Some(Shown::Event(e, _)) | Some(Shown::Alone(e, _)) => (
                        if matches!(morph.map(|m| &m.shown), Some(Shown::Alone(..))) {
                            "alone"
                        } else {
                            "event"
                        },
                        match &e.form {
                            Form::Line { text, .. } => text.clone(),
                            Form::Level { name, .. } => name.clone(),
                            Form::StartRow(a) => a.title.clone(),
                        },
                        Vec::new(),
                    ),
                };
                let privacy = match morph.map(|m| &m.shown) {
                    Some(Shown::Pill { privacy, .. })
                    | Some(Shown::Event(_, privacy))
                    | Some(Shown::Alone(_, privacy))
                    | Some(Shown::Privacy(privacy)) => privacy.len(),
                    _ => 0,
                };
                serde_json::json!({
                    "output": name,
                    "main": self.island.main.as_ref() == Some(&name),
                    "form": form,
                    "text": text,
                    "glyphs": glyphs,
                    "privacy": privacy,
                    "width": morph.map_or(0.0, |m| m.width.value()),
                    "height": morph.map_or(0.0, |m| m.height.value()),
                    "target": morph.map(|m| m.target()),
                    "open": self.shell.is_open_on("island", o),
                    "panel": self.island.panel.as_ref().filter(|p| p.output == name).map(|p| {
                        self.panel_status(p)
                    }),
                })
            })
            .collect();
        serde_json::json!({
            "clock": self.island.clock,
            "resumes": self.island.resumes,
            "outputs": outputs,
        })
        .to_string()
    }
}
