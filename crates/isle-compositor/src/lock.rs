//! The lock: a state of the core, with its own way to the screen (`p9-lock-and-prompts.md`
//! choices 1–3).

use std::{cell::RefCell, collections::HashMap, path::PathBuf, time::Instant};

use smithay::{
    backend::renderer::{
        element::{
            Kind,
            solid::{SolidColorBuffer, SolidColorRenderElement},
        },
        gles::GlesRenderer,
    },
    input::pointer::{CursorImageStatus, MotionEvent},
    output::Output,
    utils::{SERIAL_COUNTER, Scale},
};
use tracing::{info, warn};
use zeroize::{Zeroize, Zeroizing};

use crate::{
    focus::Focus,
    render::{IsleElement, NamedCursor},
    state::{Isle, State},
    worker::Event,
};

/// What the status line says about the password (`structure/lock.md`'s order).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    None,
    Checking,
    Wrong,
    LockedOut,
    /// PAM could not check the password.
    Error,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::None => "",
            Status::Checking => "checking",
            Status::Wrong => "wrong",
            Status::LockedOut => "locked-out",
            Status::Error => "error",
        }
    }
}

/// The longest password the field takes.
const FIELD_LIMIT: usize = 512;

pub struct LockState {
    pub locked: bool,
    pub since: Option<Instant>,
    /// From the lock's request to every screen showing it.
    pub shown_after: Option<std::time::Duration>,
    /// The typed password; the face draws only its length.
    pub field: Zeroizing<String>,
    /// The field has the face's keyboard focus, so typing goes into it.
    pub field_focused: bool,
    pub status: Status,
    /// Wrong passwords since the lock came.
    pub failures: u32,
    pub locked_out_until: Option<Instant>,
    /// An attempt is with the helper.
    pub pending: bool,
    pub face: crate::lock_face::FaceState,
    /// The worker's task that owns the helper; aborting it kills the helper.
    helper: Option<(
        tokio::sync::mpsc::UnboundedSender<Zeroizing<String>>,
        tokio::task::AbortHandle,
    )>,
    /// The window that had the keyboard when the lock came, given it back at unlock.
    return_window: Option<u64>,
    /// What waits for the unlock: apps' capture requests made while locked.
    after_unlock: Vec<Box<dyn FnOnce(&mut State)>>,
    /// Each output's floor and the lock's wash, by the output's identity.
    washes: RefCell<HashMap<String, (SolidColorBuffer, SolidColorBuffer)>>,
    /// What each output's last frame was made of, by output name, for the Test interface.
    #[cfg(feature = "test-hooks")]
    pub frame_kinds: RefCell<HashMap<String, Vec<&'static str>>>,
}

impl Default for LockState {
    fn default() -> Self {
        Self {
            locked: false,
            since: None,
            shown_after: None,
            field: Default::default(),
            field_focused: true,
            status: Status::None,
            failures: 0,
            locked_out_until: None,
            pending: false,
            face: crate::lock_face::FaceState::new(),
            helper: None,
            return_window: None,
            after_unlock: Vec::new(),
            washes: Default::default(),
            #[cfg(feature = "test-hooks")]
            frame_kinds: Default::default(),
        }
    }
}

fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// Present while the session is locked, so a compositor restarted in the same logind session
/// starts locked; a new sign-in does not, and v1's `locked` is another file.
pub fn marker_path() -> PathBuf {
    let name = match std::env::var("XDG_SESSION_ID") {
        Ok(id) if !id.is_empty() => format!("locked-{id}"),
        _ => "locked".to_owned(),
    };
    runtime_dir().join("isle").join(name)
}

/// The last compositor left the session locked.
pub fn marked() -> bool {
    marker_path().exists()
}

fn set_marker(locked: bool) {
    let path = marker_path();
    let result = if locked {
        path.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, b"locked\n"))
    } else {
        match std::fs::remove_file(&path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    };
    if let Err(err) = result {
        warn!(path = %path.display(), "the lock's marker: {err}");
    }
}

impl Isle {
    /// Started from a marker the last compositor left: locked before any output is lit.
    pub fn start_locked(&mut self) {
        info!("the session was locked when the compositor last ran; starting locked");
        self.lock.locked = true;
        self.lock.since = Some(Instant::now());
        self.lock.field_focused = true;
    }

    /// The lock as JSON, for the Test interface and the CLI.
    pub fn lock_status(&self) -> String {
        serde_json::json!({
            "locked": self.lock.locked,
            "dots": self.lock.field.chars().count(),
            "session_line": self.lock_session_line(),
            "status": self.lock.status.name(),
            "failures": self.lock.failures,
            "pending": self.lock.pending,
            "shown_ms": self.lock.shown_after.map(|d| d.as_secs_f64() * 1000.0),
            "face": self.lock.face.output,
            "minimal": self.lock.face.minimal_on,
            "awake": self.lock.face.awake,
            "field_focused": self.lock.field_focused,
            "minutes": self.lock.locked_out_until.map(|until| {
                until.saturating_duration_since(Instant::now()).as_secs().div_ceil(60)
            }),
        })
        .to_string()
    }

    fn locked_out(&self) -> bool {
        self.lock
            .locked_out_until
            .is_some_and(|until| Instant::now() < until)
    }

    /// A locked output's frame, front to back. Only these are ever drawn while locked: no
    /// window, layer surface, X11 surface or desktop shell surface is given to it.
    pub fn render_locked(
        &self,
        output: &Output,
        renderer: &mut GlesRenderer,
        cursor: Option<NamedCursor<'_>>,
        with_surfaces: bool,
        push: &mut dyn FnMut(IsleElement),
    ) {
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        let scale = Scale::from(output.current_scale().fractional_scale());
        if with_surfaces {
            let pointer = self
                .seat
                .get_pointer()
                .map(|p| p.current_location())
                .filter(|p| geometry.to_f64().contains(*p));
            // A dozing face hides the pointer.
            let pointer = pointer.filter(|_| self.lock.face.awake);
            if let (CursorImageStatus::Named(_), Some(at), Some(cursor)) =
                (&self.cursor_status, pointer, cursor)
                && let Some(element) = cursor.element(renderer, at - geometry.loc.to_f64(), scale)
            {
                push(IsleElement::Memory(element));
            }
            self.shell
                .elements_of(output, "island", &mut |e| push(IsleElement::Shell(e)));
            self.shell
                .elements(output, isle_shell::Layer::Lock, &mut |e| {
                    push(IsleElement::Shell(e))
                });
        }

        let tokens = &self.shell.theme().tokens;
        let wash = if self.shell.theme().appearance.increase_contrast {
            tokens.material_lock_wash_contrast
        } else {
            tokens.material_lock_wash
        };
        let floor = tokens.colour_window_sunken;
        let mut washes = self.lock.washes.borrow_mut();
        let (floor_buffer, wash_buffer) = washes.entry(Self::output_identity(output)).or_default();
        wash_buffer.update(geometry.size, crate::wallpaper::premultiplied(wash));
        floor_buffer.update(geometry.size, crate::wallpaper::premultiplied(floor));
        push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
            wash_buffer,
            (0, 0),
            scale,
            1.0,
            Kind::Unspecified,
        )));
        self.softened_elements(output, &mut |e| push(IsleElement::Wallpaper(e)));
        // Opaque whatever the wallpaper is doing, so nothing from before shows through.
        push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
            floor_buffer,
            (0, 0),
            scale,
            1.0,
            Kind::Unspecified,
        )));
    }

    /// While locked only the lock's own surfaces take the keyboard; while a prompt shows, no
    /// client does.
    pub fn lock_allows(&self, target: Option<&Focus>) -> bool {
        match target {
            None => true,
            Some(Focus::Shell(focus)) => {
                !self.lock.locked
                    || self
                        .shell
                        .name_of(focus)
                        .is_some_and(|name| name.starts_with("lock"))
            }
            Some(_) => !self.lock.locked && !self.scrimmed(),
        }
    }
}

impl State {
    /// Locks the session: the marker first, then every frame from the lock's path.
    pub fn lock(&mut self, why: &'static str) {
        if self.isle.lock.locked {
            return;
        }
        set_marker(true);
        info!(why, "locked");
        self.isle.lock.return_window = self.focused_window();
        for output in self.isle.space.outputs() {
            crate::redraw::OutputState::of(output).borrow_mut().locked_shown = false;
        }
        self.isle.lock.locked = true;
        self.portal_monitor_state(None);
        self.isle.lock.since = Some(Instant::now());
        self.isle.lock.shown_after = None;
        self.isle.lock.field_focused = true;
        self.isle.lock.face.first_note = self.isle.notifications.next_id;
        self.take_input_back();
        self.isle.island.model.set_locked(true);
        self.isle.refresh_island();
        self.show_lock_face();
        self.isle.queue_redraw_all();
        self.lock_changed();
    }

    /// A compositor started locked shows the face once its outputs are lit.
    pub fn locked_at_start(&mut self) {
        if self.isle.lock.locked {
            self.isle.island.model.set_locked(true);
            self.isle.refresh_island();
            self.show_lock_face();
        }
    }

    /// Ends the lock and gives the keyboard back to the window that had it.
    pub fn unlock(&mut self, why: &'static str) {
        if !self.isle.lock.locked {
            return;
        }
        info!(why, "unlocked");
        self.isle.lock.locked = false;
        self.portal_monitor_state(None);
        self.isle.lock.since = None;
        set_marker(false);
        self.end_auth();
        self.session_unlocked();
        self.hide_lock_face();
        self.isle.island.model.set_locked(false);
        self.isle.refresh_island();
        let window = self
            .isle
            .lock
            .return_window
            .take()
            .filter(|id| self.isle.wm.is_shown(*id))
            .and_then(|id| self.isle.window_by_id(id));
        match window {
            Some(window) => self.set_focus(Some(&window)),
            None => self.focus_next(),
        }
        // The pointer meets whatever is under it again.
        let pointer = self.isle.seat.get_pointer().unwrap();
        let location = pointer.current_location();
        let under = self.isle.surface_under(location);
        pointer.motion(
            self,
            under,
            &MotionEvent {
                location,
                serial: SERIAL_COUNTER.next_serial(),
                time: smithay::backend::input::InputTime::now(),
            },
        );
        pointer.frame(self);
        self.isle.queue_redraw_all();
        self.lock_changed();
        for then in std::mem::take(&mut self.isle.lock.after_unlock) {
            then(self);
        }
    }

    /// Runs `then` once the screen is unlocked: now, when it is not locked.
    pub fn when_unlocked(&mut self, then: impl FnOnce(&mut State) + 'static) {
        if self.isle.lock.locked {
            self.isle.lock.after_unlock.push(Box::new(then));
        } else {
            then(self);
        }
    }

    /// Everything a client or a desktop surface holds is taken back: the centre surface and
    /// the menu-bar group close, grabs end, and neither keyboard nor pointer is a client's.
    fn take_input_back(&mut self) {
        self.close_pick();
        self.share_cancel();
        self.close_toolbar();
        self.cancel_capture_countdown();
        self.dismiss_thumbnail();
        self.close_session_dialog();
        self.close_force_quit();
        self.close_spotlight();
        self.cancel_switcher();
        self.close_menu_bar_group();
        if self.isle.recording.is_some() {
            self.cancel_recording();
        }
        let serial = SERIAL_COUNTER.next_serial();
        let time = smithay::backend::input::InputTime::now();
        let pointer = self.isle.seat.get_pointer().unwrap();
        if pointer.is_grabbed() {
            pointer.unset_grab(self, serial, time);
        }
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        if keyboard.is_grabbed() {
            keyboard.unset_grab(self);
        }
        self.focus_keyboard(None, serial);
        self.isle.cursor_status = CursorImageStatus::default_named();
        let location = pointer.current_location();
        let under = self.isle.surface_under(location);
        pointer.motion(
            self,
            under,
            &MotionEvent {
                location,
                serial,
                time,
            },
        );
        pointer.frame(self);
    }

    /// A key pressed in the field; whether the field took it.
    pub fn lock_key(
        &mut self,
        sym: smithay::input::keyboard::Keysym,
        mods: &smithay::input::keyboard::ModifiersState,
    ) -> bool {
        use smithay::input::keyboard::keysyms as k;
        let held = self.isle.lock.pending || self.isle.locked_out();
        // The power row's confirm has the keys while it shows.
        if matches!(
            self.isle.session.lock_power,
            isle_shell::lock::Power::Confirm { .. }
        ) {
            use isle_shell::lock::LockAction;
            match sym.raw() {
                k::KEY_Return | k::KEY_KP_Enter => self.lock_power(LockAction::PowerConfirm),
                k::KEY_Escape => self.lock_power(LockAction::PowerCancel),
                _ => {}
            }
            return true;
        }
        match sym.raw() {
            k::KEY_Return | k::KEY_KP_Enter => self.submit_password(),
            k::KEY_BackSpace if !held => {
                self.isle.lock.field.pop();
                self.field_changed();
            }
            k::KEY_Escape if !held => {
                self.isle.lock.field.zeroize();
                self.field_changed();
            }
            k::KEY_BackSpace | k::KEY_Escape => {}
            _ => {
                let Some(c) = sym.key_char().filter(|c| !c.is_control()) else {
                    return false;
                };
                if mods.ctrl || mods.alt || mods.logo {
                    return false;
                }
                if !held && self.isle.lock.field.len() < FIELD_LIMIT {
                    self.isle.lock.face.error_edge = false;
                    self.isle.lock.field.push(c);
                    self.field_changed();
                }
            }
        }
        true
    }

    fn field_changed(&mut self) {
        self.refresh_lock_face();
    }

    /// Return in the field: the password goes to the helper, off the loop.
    fn submit_password(&mut self) {
        let lock = &mut self.isle.lock;
        if lock.pending || lock.field.is_empty() || self.isle.locked_out() {
            return;
        }
        let password = Zeroizing::new(self.isle.lock.field.as_str().to_owned());
        if self.isle.lock.helper.is_none() {
            self.isle.lock.helper = self.start_auth();
        }
        let Some((attempts, _)) = &self.isle.lock.helper else {
            self.auth_outcome(Err("the worker is not running".into()));
            return;
        };
        if attempts.send(password).is_err() {
            self.isle.lock.helper = None;
            self.auth_outcome(Err("the helper's task ended".into()));
            return;
        }
        self.isle.lock.pending = true;
        self.isle.lock.status = Status::Checking;
        self.arm_check_long();
        self.field_changed();
    }

    /// The task on the worker that starts `isle-auth` and hands it each attempt.
    fn start_auth(
        &mut self,
    ) -> Option<(
        tokio::sync::mpsc::UnboundedSender<Zeroizing<String>>,
        tokio::task::AbortHandle,
    )> {
        let (runtime, to_loop) = (self.isle.worker.clone()?, self.isle.to_loop.clone()?);
        let helper_file = crate::helpers::auth();
        let (program, found) = (helper_file.program(), helper_file.found.clone());
        #[cfg_attr(not(feature = "test-hooks"), allow(unused_mut))]
        let mut env: Vec<(String, String)> = Vec::new();
        #[cfg(feature = "test-hooks")]
        if let Ok(dir) = std::env::var("ISLE_PAM_CONFDIR") {
            env.push(("ISLE_PAM_CONFDIR".into(), dir));
        }
        let (send, mut attempts) = tokio::sync::mpsc::unbounded_channel::<Zeroizing<String>>();
        let task = runtime.spawn(async move {
            let mut helper: Option<isle_auth::Helper> = None;
            while let Some(password) = attempts.recv().await {
                if helper.is_none() {
                    let env: Vec<(&str, &str)> =
                        env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                    match isle_auth::Helper::spawn(&program, &env) {
                        Ok(started) => helper = Some(started),
                        Err(err) => {
                            warn!(program = %found.display(), "isle-auth did not start: {err}")
                        }
                    }
                }
                let outcome = match helper.as_mut() {
                    Some(helper) => helper.attempt(&password).await.map_err(|e| e.to_string()),
                    None => Err("isle-auth did not start".into()),
                };
                drop(password);
                if outcome.is_err() {
                    helper = None;
                }
                to_loop.post(move |state| state.auth_outcome(outcome));
            }
        });
        Some((send, task.abort_handle()))
    }

    /// The helper's answer for the attempt in flight.
    pub fn auth_outcome(&mut self, outcome: Result<isle_auth::Outcome, String>) {
        if !self.isle.lock.locked {
            return;
        }
        let lock = &mut self.isle.lock;
        lock.pending = false;
        lock.field.zeroize();
        match outcome {
            Ok(isle_auth::Outcome {
                result: isle_auth::Result::Ok,
                ..
            }) => {
                self.unlock("password");
                return;
            }
            Ok(
                outcome @ isle_auth::Outcome {
                    result: isle_auth::Result::Fail(_),
                    ..
                },
            ) => match outcome.lockout() {
                Some(minutes) => {
                    info!(minutes, "the account is locked out");
                    lock.status = Status::LockedOut;
                    lock.locked_out_until =
                        Some(Instant::now() + std::time::Duration::from_secs(minutes as u64 * 60));
                }
                None => {
                    lock.failures += 1;
                    lock.status = Status::Wrong;
                    info!(failures = lock.failures, "a wrong password");
                    self.lock_wrong();
                }
            },
            Ok(isle_auth::Outcome {
                result: isle_auth::Result::Broken(why),
                ..
            })
            | Err(why) => {
                warn!("the password could not be checked: {why}");
                lock.status = Status::Error;
            }
        }
        self.field_changed();
    }

    /// The lock is over: the helper is killed, an attempt in flight with it, and the field
    /// and its count cleared.
    fn end_auth(&mut self) {
        let lock = &mut self.isle.lock;
        if let Some((_, task)) = lock.helper.take() {
            task.abort();
        }
        lock.field.zeroize();
        lock.pending = false;
        lock.status = Status::None;
        lock.failures = 0;
        lock.locked_out_until = None;
    }

    fn lock_changed(&mut self) {
        self.silence_check();
        self.arm_idle();
        self.refresh_prompts();
        self.isle.notify(Event::LockChanged);
        let locked = self.isle.lock.locked;
        self.isle.run_service(
            crate::services::login::set_hint(Some(locked), None),
            |_, result| {
                if let Err(err) = result {
                    tracing::debug!("logind's locked hint: {err}");
                }
            },
        );
    }

    /// The power key: the lock's face wakes; otherwise the session chooser opens.
    pub fn power_key(&mut self) {
        if self.isle.lock.locked {
            self.lock_activity();
            return;
        }
        self.open_session_chooser();
    }

    /// logind announced sleep: lock if the setting asks, and let sleep go on once every lit
    /// screen shows the lock (choice 10).
    pub fn before_sleep(&mut self) {
        if let Some(gate) = &self.isle.may_sleep {
            gate.send_replace(false);
        }
        if self.isle.settings.lock_screen.lock_on_sleep {
            self.lock("sleep");
        }
        self.isle.sleep_waiting = true;
        self.isle.queue_redraw_all();
        self.sleep_ready();
    }

    /// Sleep goes on once every screen's frame on show is the lock's, or at once when the
    /// lock is not wanted; drawing stops until resume.
    pub fn sleep_ready(&mut self) {
        if !self.isle.sleep_waiting {
            return;
        }
        let shown = !self.isle.lock.locked
            || self
                .isle
                .space
                .outputs()
                .all(|o| crate::redraw::OutputState::of(o).borrow().locked_shown);
        if !shown {
            return;
        }
        self.isle.sleep_waiting = false;
        self.isle.asleep = true;
        info!("ready for sleep");
        if let Some(gate) = &self.isle.may_sleep {
            gate.send_replace(true);
        }
    }

    /// Back from sleep: drawing resumes, and the face wakes.
    pub fn after_sleep(&mut self) {
        self.isle.sleep_waiting = false;
        self.isle.asleep = false;
        if let Some(gate) = &self.isle.may_sleep {
            gate.send_replace(false);
        }
        self.isle.queue_redraw_all();
        self.lock_activity();
    }
}
