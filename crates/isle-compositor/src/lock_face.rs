//! The lock's face on the loop: its model from the compositor's state, the screen that has
//! it, dozing, and the minimal form after its panics (`p9-lock-and-prompts.md` choices 8, 16
//! and 17).

use std::{
    collections::VecDeque,
    rc::Rc,
    time::{Duration, Instant},
};

use isle_shell::lock::{Face, Frame, Line, LockAction, LockSurface, MinimalSurface, Note, Playing};
use smithay::{
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
};
use tracing::warn;

use crate::{focus::Focus, lock::Status, state::State};

/// The second panic of the face within this long brings in the minimal lock.
const PANIC_WINDOW: Duration = Duration::from_secs(60);

pub struct FaceState {
    pub surface: Rc<LockSurface>,
    pub minimal: Rc<MinimalSurface>,
    /// The screen that has the face, by output name.
    pub output: Option<String>,
    pub awake: bool,
    /// The face's panics in the last minute.
    panics: VecDeque<Instant>,
    /// The face panicked twice: the minimal lock stands until unlock.
    pub minimal_on: bool,
    pub shakes: u32,
    pub error_edge: bool,
    /// Notifications with ids from here on arrived while locked.
    pub first_note: u32,
    /// The check has run past `timing-check-long-ms`.
    pub check_long: bool,
    #[cfg(feature = "test-hooks")]
    pub injected: u32,
    doze_timer: Option<RegistrationToken>,
    clock_timer: Option<RegistrationToken>,
    check_timer: Option<RegistrationToken>,
}

impl FaceState {
    pub fn new() -> Self {
        Self {
            surface: LockSurface::new(),
            minimal: MinimalSurface::new(),
            output: None,
            awake: true,
            panics: VecDeque::new(),
            minimal_on: false,
            shakes: 0,
            error_edge: false,
            first_note: 0,
            check_long: false,
            #[cfg(feature = "test-hooks")]
            injected: 0,
            doze_timer: None,
            clock_timer: None,
            check_timer: None,
        }
    }

    fn name(&self) -> &'static str {
        if self.minimal_on {
            "lock-minimal"
        } else {
            "lock"
        }
    }
}

/// A layout's short name for the capsule: "English (US)" is "US".
fn short_layout(name: &str) -> String {
    if let Some((_, rest)) = name.split_once('(')
        && let Some(inside) = rest.strip_suffix(')')
        && inside.len() <= 4
    {
        return inside.to_uppercase();
    }
    name.chars().take(2).collect::<String>().to_uppercase()
}

/// The user's full name from the password database, else the login name.
fn user_name() -> (String, String) {
    unsafe {
        let entry = libc::getpwuid(libc::getuid());
        if entry.is_null() {
            return (String::new(), String::new());
        }
        let login = std::ffi::CStr::from_ptr((*entry).pw_name)
            .to_string_lossy()
            .into_owned();
        let gecos = if (*entry).pw_gecos.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr((*entry).pw_gecos)
                .to_string_lossy()
                .split(',')
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        let full = if gecos.is_empty() {
            login.clone()
        } else {
            gecos
        };
        (login, full)
    }
}

/// AccountsService's picture of `login`, else `~/.face`.
fn picture(login: &str) -> Option<std::path::PathBuf> {
    let accounts = std::path::PathBuf::from("/var/lib/AccountsService/icons").join(login);
    if accounts.is_file() {
        return Some(accounts);
    }
    let face = std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".face"))?;
    face.is_file().then_some(face)
}

impl State {
    /// The face on every screen, the keyboard to the one with the pointer.
    pub fn show_lock_face(&mut self) {
        if !self.isle.lock.locked {
            return;
        }
        let pointer_output = self.isle.active_output().map(|o| o.name());
        let face = &mut self.isle.lock.face;
        if face.output.is_none()
            || !self
                .isle
                .space
                .outputs()
                .any(|o| Some(o.name()) == face.output)
        {
            face.output = pointer_output;
        }
        face.awake = true;
        let name = self.isle.lock.face.name();
        let other = if name == "lock" {
            "lock-minimal"
        } else {
            "lock"
        };
        self.isle.shell.close(other);
        self.isle.shell.open(name);
        self.refresh_lock_face();
        self.focus_lock_face();
        self.arm_lock_clock();
        self.arm_doze();
    }

    /// The face's screen takes the keyboard.
    fn focus_lock_face(&mut self) {
        let name = self.isle.lock.face.name();
        let Some(output) = self.lock_face_output() else {
            return;
        };
        if let Some(focus) = self.isle.shell.instance_focus(name, &output) {
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.focus_keyboard(Some(Focus::Shell(focus)), serial);
        }
    }

    fn lock_face_output(&self) -> Option<Output> {
        let name = self.isle.lock.face.output.as_ref()?;
        self.isle
            .space
            .outputs()
            .find(|o| &o.name() == name)
            .cloned()
    }

    /// The face goes at unlock; its timers with it.
    pub fn hide_lock_face(&mut self) {
        self.isle.shell.close("lock");
        self.isle.shell.close("lock-minimal");
        let face = &mut self.isle.lock.face;
        face.output = None;
        face.minimal_on = false;
        face.panics.clear();
        face.shakes = 0;
        face.error_edge = false;
        face.check_long = false;
        for token in [
            face.doze_timer.take(),
            face.clock_timer.take(),
            face.check_timer.take(),
        ]
        .into_iter()
        .flatten()
        {
            self.isle.loop_handle.remove(token);
        }
    }

    /// The face's model from the compositor's state, put in force on every screen.
    pub fn refresh_lock_face(&mut self) {
        if !self.isle.lock.locked {
            return;
        }
        let face = self.lock_face_model();
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        let here = self.isle.lock.face.output.clone();
        #[cfg(feature = "test-hooks")]
        let panics = self.isle.lock.face.injected;
        #[cfg(not(feature = "test-hooks"))]
        let panics = 0;
        let frames: Vec<(String, Frame)> = outputs
            .iter()
            .map(|o| {
                let size = self.isle.space.output_geometry(o).unwrap_or_default().size;
                (
                    o.name(),
                    Frame {
                        face: face.clone(),
                        here: Some(o.name()) == here,
                        width: size.w as f32,
                        height: size.h as f32,
                        panics,
                    },
                )
            })
            .collect();
        let (surface, minimal) = (
            self.isle.lock.face.surface.clone(),
            self.isle.lock.face.minimal.clone(),
        );
        for (name, frame) in &frames {
            surface.set_model(name, frame.clone());
            minimal.set_model(name, frame.clone());
        }
        let find = |output: &Output| {
            frames
                .iter()
                .find(|(n, _)| *n == output.name())
                .map(|(_, f)| f.clone())
        };
        self.isle.shell.with_roots("lock", |output| {
            if let Some(frame) = find(output) {
                surface.set(&output.name(), frame);
            }
        });
        self.isle.shell.with_roots("lock-minimal", |output| {
            if let Some(frame) = find(output) {
                minimal.set(&output.name(), frame);
            }
        });
        self.dispatch_shell();
        self.isle.queue_redraw_all();
    }

    fn lock_face_model(&mut self) -> Face {
        let theme = self.isle.shell.theme().clone();
        let now = jiff::Zoned::now();
        let hours = crate::island::clock_24_hour(&self.isle.settings.island);
        let time = if hours {
            now.strftime("%H:%M").to_string()
        } else {
            now.strftime("%-I:%M").to_string()
        };
        let (login, user) = user_name();
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let caps = keyboard.modifier_state().caps_lock;
        let (layouts, layout) = keyboard.with_xkb_state(self, |context| {
            let xkb = context.xkb().lock().unwrap();
            (
                xkb.layouts().count(),
                xkb.layout_name(xkb.active_layout()).to_owned(),
            )
        });
        let lock = &self.isle.lock;
        let several = layouts > 1;
        let locked_out = lock
            .locked_out_until
            .is_some_and(|until| Instant::now() < until);
        let line = if locked_out {
            Line::LockedOut {
                minutes: lock
                    .locked_out_until
                    .map(|u| {
                        u.saturating_duration_since(Instant::now())
                            .as_secs()
                            .div_ceil(60)
                    })
                    .unwrap_or(1) as u32,
            }
        } else if lock.pending {
            Line::Checking {
                long: lock.face.check_long,
            }
        } else if lock.status == Status::Wrong {
            Line::Wrong {
                count: lock.failures,
                layout: several.then(|| short_layout(&layout)),
            }
        } else if lock.status == Status::Error {
            Line::Error
        } else if let Some(words) = self.isle.lock_session_line() {
            Line::Stopped(words)
        } else if caps {
            Line::CapsLock
        } else {
            Line::None
        };
        let game = self.isle.game_mode_on();
        Face {
            date: now.strftime("%A %-d %B").to_string(),
            time,
            focus: self
                .isle
                .focus
                .active
                .as_ref()
                .map(|_| self.isle.focus_symbol()),
            playing: self.lock_playing(),
            notes: self.lock_notes().0,
            more: self.lock_notes().1,
            user,
            picture: picture(&login),
            dots: lock.field.chars().count(),
            line,
            held: lock.pending || locked_out,
            locked_out,
            error_edge: lock.face.error_edge,
            shakes: lock.face.shakes,
            awake: lock.face.awake,
            layout: several.then(|| short_layout(&layout)),
            layout_marked: several && lock.status == Status::Wrong,
            battery: self
                .isle
                .services
                .power
                .state
                .system()
                .and_then(|b| b.level.map(|l| (l, b.charging))),
            power: self.isle.session.lock_power,
            solid: game || theme.appearance.reduce_transparency,
            still: game || theme.appearance.reduce_motion,
        }
    }

    /// Now Playing while the player plays, or for a while after it pauses.
    fn lock_playing(&self) -> Option<Playing> {
        let player = self.isle.chosen_player()?;
        let playing = player.status == isle_media::Status::Playing;
        if !playing && player.title.is_empty() {
            return None;
        }
        Some(Playing {
            title: if player.title.is_empty() {
                player.identity.clone()
            } else {
                player.title.clone()
            },
            artist: player.artist.clone(),
            art: player
                .art_url
                .as_ref()
                .and_then(|u| self.isle.services.art.get(u).cloned().flatten()),
            playing,
        })
    }

    /// What arrived since locking, by app, newest first, as the setting shows it, and how
    /// many apps more than fit.
    fn lock_notes(&self) -> (Vec<Note>, u32) {
        use isle_settings::schema::LockScreen;
        let shows = self.isle.settings.notifications.lock_screen;
        if shows == LockScreen::None {
            return (Vec::new(), 0);
        }
        let first = self.isle.lock.face.first_note;
        let focus = self.isle.focus_hold();
        let mut notes: Vec<Note> = Vec::new();
        for entry in self.isle.notifications.entries.iter().rev() {
            let n = &entry.n;
            if n.id < first || entry.fate == crate::notifications::Fate::Quiet || n.transient {
                continue;
            }
            let app = self.isle.notification_app(n);
            if let Some((_, allowed, critical)) = &focus
                && !allowed.contains(&app)
                && !(*critical && n.urgency == 2)
            {
                continue;
            }
            match notes.iter_mut().find(|note| note.app == app) {
                Some(note) => note.count += 1,
                None => notes.push(Note {
                    name: {
                        let name = self.isle.app_name(&app);
                        if name.is_empty() {
                            n.app_name.clone()
                        } else {
                            name
                        }
                    },
                    icon: self.isle.app_icon(&app, 32),
                    count: 1,
                    summary: (shows == LockScreen::Previews).then(|| n.summary.clone()),
                    app,
                }),
            }
        }
        let limit = self.isle.shell.theme().tokens.limit_lock_notifications as usize;
        let more = notes.len().saturating_sub(limit) as u32;
        notes.truncate(limit);
        (notes, more)
    }

    /// Any input on the lock: the face wakes and dozes again after a quiet spell.
    pub fn lock_activity(&mut self) {
        if !self.isle.lock.locked {
            return;
        }
        if !self.isle.lock.face.awake {
            self.isle.lock.face.awake = true;
            self.refresh_lock_face();
        }
        self.arm_doze();
    }

    fn arm_doze(&mut self) {
        if let Some(token) = self.isle.lock.face.doze_timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let wait = Duration::from_millis(self.isle.shell.theme().tokens.timing_doze_ms as u64);
        self.isle.lock.face.doze_timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(wait), |_, _, state| {
                state.isle.lock.face.doze_timer = None;
                // A face with something typed stays awake.
                if state.isle.lock.locked && state.isle.lock.field.is_empty() {
                    state.isle.lock.face.awake = false;
                    state.refresh_lock_face();
                }
                TimeoutAction::Drop
            })
            .ok();
    }

    /// The face's clock moves on at each minute.
    fn arm_lock_clock(&mut self) {
        if let Some(token) = self.isle.lock.face.clock_timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let now = jiff::Zoned::now();
        let into = now.second() as u64 * 1000 + now.millisecond() as u64;
        let wait = Duration::from_millis(60_000 - into.min(59_999));
        self.isle.lock.face.clock_timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(wait), |_, _, state| {
                state.isle.lock.face.clock_timer = None;
                if state.isle.lock.locked {
                    state.refresh_lock_face();
                    state.arm_lock_clock();
                }
                TimeoutAction::Drop
            })
            .ok();
    }

    /// A check under way: "Still checking" once it runs long.
    pub fn arm_check_long(&mut self) {
        if let Some(token) = self.isle.lock.face.check_timer.take() {
            self.isle.loop_handle.remove(token);
        }
        self.isle.lock.face.check_long = false;
        let wait =
            Duration::from_millis(self.isle.shell.theme().tokens.timing_check_long_ms as u64);
        self.isle.lock.face.check_timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(wait), |_, _, state| {
                state.isle.lock.face.check_timer = None;
                if state.isle.lock.pending {
                    state.isle.lock.face.check_long = true;
                    state.refresh_lock_face();
                }
                TimeoutAction::Drop
            })
            .ok();
    }

    /// A wrong password: the field shakes and takes its error edge.
    pub fn lock_wrong(&mut self) {
        self.isle.lock.face.shakes += 1;
        self.isle.lock.face.error_edge = true;
    }

    /// The pointer moved to another screen: the face goes with it, its dots kept.
    pub fn lock_pointer_moved(&mut self) {
        let Some(output) = self.isle.active_output().map(|o| o.name()) else {
            return;
        };
        self.lock_activity();
        if self.isle.lock.face.output.as_ref() != Some(&output) {
            self.isle.lock.face.output = Some(output);
            self.refresh_lock_face();
            self.focus_lock_face();
        }
    }

    pub fn lock_action(&mut self, action: LockAction) {
        match action {
            LockAction::FieldFocus(on) => self.isle.lock.field_focused = on,
            LockAction::Wake => self.lock_activity(),
            LockAction::Sleep => self.isle.suspend(),
            LockAction::Restart
            | LockAction::ShutDown
            | LockAction::PowerConfirm
            | LockAction::PowerCancel => self.lock_power(action),
            LockAction::PlayPause => self.media_key(crate::binds::Media::PlayPause),
            LockAction::Next => self.media_key(crate::binds::Media::Next),
            LockAction::Previous => self.media_key(crate::binds::Media::Previous),
            LockAction::NextLayout => {
                self.next_layout();
                self.refresh_lock_face();
            }
        }
    }

    /// The shell's panics of the face: rebuilt the first time, the minimal lock the second
    /// within a minute (choice 8).
    pub fn lock_panicked(&mut self, name: &str) {
        if !self.isle.lock.locked || name != "lock" {
            return;
        }
        let now = Instant::now();
        let face = &mut self.isle.lock.face;
        face.panics.push_back(now);
        while face
            .panics
            .front()
            .is_some_and(|t| now.duration_since(*t) > PANIC_WINDOW)
        {
            face.panics.pop_front();
        }
        if face.panics.len() >= 2 && !face.minimal_on {
            warn!("the lock's face panicked twice in a minute; the minimal lock stands in");
            face.minimal_on = true;
            self.show_lock_face();
        } else {
            self.refresh_lock_face();
            self.focus_lock_face();
        }
    }
}
