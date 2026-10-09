//! Ending the session: the chooser and the confirmation on the loop, and the one engine behind
//! them and the lock's power row, which quits every app and then logs out, restarts or shuts
//! down (`p9-lock-and-prompts.md` choice 22).

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use isle_shell::{
    lock::Power,
    prompts::session::{Choice, Form, Frame, OpenApp, SessionAction, SessionSurface, Stop},
};
use smithay::{
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::Rectangle,
};
use tracing::info;

use crate::{
    focus::Focus,
    state::{Isle, State},
};

/// The countdown's start, in seconds.
const COUNTDOWN: u32 = 60;
/// How long an app that answers its pings has to quit before it counts as asking to save.
const QUIT_WAIT: Duration = Duration::from_secs(5);
const CHECK_EVERY: Duration = Duration::from_millis(250);

/// The dialog on screen.
pub struct Dialog {
    pub form: Form,
    pub since: Instant,
    pub countdown: Option<u32>,
    pub armed: bool,
}

/// The session ending: what it ends in, since when the apps were asked, and what stopped it.
pub struct Ending {
    pub choice: Choice,
    pub started: Instant,
    pub stop: Option<Stop>,
    /// Asked from the lock's power row, which names no app.
    pub from_lock: bool,
    /// Every app is gone and logind was asked.
    pub finishing: bool,
}

pub struct SessionState {
    pub surface: Rc<SessionSurface>,
    pub dialog: Option<Dialog>,
    pub ending: Option<Ending>,
    /// The lock's power row.
    pub lock_power: Power,
    tick: Option<RegistrationToken>,
    arm: Option<RegistrationToken>,
    check: Option<RegistrationToken>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            surface: SessionSurface::new(),
            dialog: None,
            ending: None,
            lock_power: Power::Row,
            tick: None,
            arm: None,
            check: None,
        }
    }
}

impl Isle {
    /// Since when a scrim has been wanted: a prompt or the session dialog shows.
    pub fn scrim_since(&self) -> Option<Instant> {
        let prompt = self.prompts.shown.as_ref().map(|(_, _, since)| *since);
        let dialog = self.session.dialog.as_ref().map(|d| d.since);
        match (prompt, dialog) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    pub fn scrimmed(&self) -> bool {
        self.scrim_since().is_some()
    }

    /// The open apps as the dialog shows them, most recently used first.
    fn open_apps(&self) -> Vec<OpenApp> {
        let size = self.shell.theme().tokens.size_app_icon as u32;
        self.apps_by_recency()
            .into_iter()
            .map(|app| OpenApp {
                name: self.app_name(&app),
                icon: self.app_icon(&app, size),
                not_responding: self
                    .wm
                    .records
                    .iter()
                    .any(|r| r.app == app && r.not_responding),
                app,
            })
            .collect()
    }

    /// The line the lock shows for a stopped or refused end, which names no app.
    pub fn lock_session_line(&self) -> Option<String> {
        let ending = self.session.ending.as_ref().filter(|e| e.from_lock)?;
        let verb = if ending.choice == Choice::Restart {
            "restart"
        } else {
            "shut down"
        };
        match ending.stop.as_ref()? {
            Stop::Refused(why) => Some(why.clone()),
            _ => Some(format!(
                "Couldn't {verb} because an app didn't quit. Unlock to see it."
            )),
        }
    }

    pub fn session_status(&self) -> String {
        let dialog = self.session.dialog.as_ref();
        let ending = self.session.ending.as_ref();
        serde_json::json!({
            "open": dialog.is_some(),
            "form": dialog.map(|d| match d.form {
                Form::Chooser(i) => format!("chooser {}", Choice::ROW[i].label()),
                Form::Confirm(c) => format!("confirm {}", c.label()),
            }),
            "countdown": dialog.and_then(|d| d.countdown),
            "armed": dialog.is_some_and(|d| d.armed),
            "ending": ending.map(|e| e.choice.label()),
            "stop": ending.and_then(|e| e.stop.as_ref()).map(|s| match s {
                Stop::Open(app) => format!("open {}", app.app),
                Stop::NotResponding(app) => format!("not responding {}", app.app),
                Stop::Refused(why) => format!("refused {why}"),
            }),
            "lock_power": format!("{:?}", self.session.lock_power),
        })
        .to_string()
    }
}

impl State {
    /// The power key and the chord: the five actions, Shut Down selected and counting down.
    pub fn open_session_chooser(&mut self) {
        self.open_session_dialog(Form::Chooser(4));
    }

    /// Restart…, Shut Down… and Log Out… in the system menu, and Shift+Cmd+Q.
    pub fn open_session_confirm(&mut self, choice: Choice) {
        self.open_session_dialog(Form::Confirm(choice));
    }

    fn open_session_dialog(&mut self, form: Form) {
        if self.isle.lock.locked || self.isle.session.dialog.is_some() {
            return;
        }
        if self.isle.session.ending.is_some() {
            return;
        }
        let Some(output) = self.isle.main_output() else {
            return;
        };
        info!(?form, "session dialog");
        self.cancel_switcher();
        self.close_force_quit();
        self.close_spotlight();
        self.close_toolbar();
        self.close_menu_bar_group();
        let theme = self.isle.shell.theme().clone();
        let screen = self.isle.space.output_geometry(&output).unwrap_or_default();
        let size = SessionSurface::size_for(&theme);
        let margin = SessionSurface::margin(&theme) as i32;
        let geometry = Rectangle::new(
            ((screen.size.w - size.w) / 2, screen.size.h / 3 - margin).into(),
            size,
        );
        self.isle.session.surface.set_size(size);
        self.isle.session.dialog = Some(Dialog {
            form,
            since: Instant::now(),
            countdown: Some(COUNTDOWN),
            armed: false,
        });
        self.put_session_frame();
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        self.isle.shell.open_placed("session", &place, geometry);
        if let Some(focus) = self.isle.shell.keyboard_taker("session") {
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            let previous = keyboard.current_focus();
            self.isle
                .shell_keyboard_return
                .push((focus.clone(), previous));
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.focus_keyboard(Some(Focus::Shell(focus)), serial);
        }
        self.dispatch_shell();
        self.arm_session_dialog();
        self.start_countdown();
        self.isle.queue_redraw_all();
    }

    /// Closes the dialog; an ending not yet past its apps is called off.
    pub fn close_session_dialog(&mut self) {
        if self.isle.session.dialog.take().is_none() {
            return;
        }
        for token in [self.isle.session.tick.take(), self.isle.session.arm.take()]
            .into_iter()
            .flatten()
        {
            self.isle.loop_handle.remove(token);
        }
        if self
            .isle
            .session
            .ending
            .as_ref()
            .is_some_and(|e| !e.finishing && !e.from_lock)
        {
            self.call_off_ending();
        }
        self.close_surface("session");
        self.isle.queue_redraw_all();
    }

    fn call_off_ending(&mut self) {
        info!("the session's end called off");
        self.isle.session.ending = None;
        if let Some(token) = self.isle.session.check.take() {
            self.isle.loop_handle.remove(token);
        }
    }

    /// Return and clicks act only after a moment, as a prompt's do.
    fn arm_session_dialog(&mut self) {
        let wait =
            Duration::from_millis(self.isle.shell.theme().tokens.timing_prompt_arm_ms as u64);
        self.isle.session.arm = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(wait), |_, _, state| {
                state.isle.session.arm = None;
                if let Some(dialog) = &mut state.isle.session.dialog {
                    dialog.armed = true;
                    state.put_session_frame();
                }
                TimeoutAction::Drop
            })
            .ok();
    }

    fn start_countdown(&mut self) {
        self.isle.session.tick = self
            .isle
            .loop_handle
            .insert_source(
                Timer::from_duration(Duration::from_secs(1)),
                |_, _, state| {
                    let Some(dialog) = &mut state.isle.session.dialog else {
                        return TimeoutAction::Drop;
                    };
                    let Some(left) = dialog
                        .countdown
                        .filter(|_| state.isle.session.ending.is_none())
                    else {
                        state.isle.session.tick = None;
                        return TimeoutAction::Drop;
                    };
                    let left = left.saturating_sub(1);
                    dialog.countdown = Some(left);
                    if left == 0 {
                        dialog.countdown = None;
                        state.isle.session.tick = None;
                        let choice = state.session_frame_choice();
                        state.session_choose(choice);
                        return TimeoutAction::Drop;
                    }
                    state.put_session_frame();
                    TimeoutAction::ToDuration(Duration::from_secs(1))
                },
            )
            .ok();
    }

    fn stop_countdown(&mut self) {
        if let Some(dialog) = &mut self.isle.session.dialog
            && dialog.countdown.take().is_some()
        {
            if let Some(token) = self.isle.session.tick.take() {
                self.isle.loop_handle.remove(token);
            }
            self.put_session_frame();
        }
    }

    fn session_frame_choice(&self) -> Choice {
        match self.isle.session.dialog.as_ref().map(|d| d.form) {
            Some(Form::Confirm(choice)) => choice,
            Some(Form::Chooser(i)) => Choice::ROW[i.min(4)],
            None => Choice::ShutDown,
        }
    }

    pub fn session_action(&mut self, action: SessionAction) {
        let armed = self.isle.session.dialog.as_ref().is_some_and(|d| d.armed);
        let ending = self.isle.session.ending.is_some();
        match action {
            // Keys still held from the chord that opened it are not the user's answer.
            SessionAction::Hold if armed => self.stop_countdown(),
            SessionAction::Hold => {}
            SessionAction::Cancel => {
                // While apps quit there is nothing to cancel until one stops it.
                let quitting = self
                    .isle
                    .session
                    .ending
                    .as_ref()
                    .is_some_and(|e| e.stop.is_none());
                if !quitting {
                    self.close_session_dialog();
                }
            }
            SessionAction::Step(step) => {
                if let Some(Dialog {
                    form: Form::Chooser(i),
                    ..
                }) = &mut self.isle.session.dialog
                    && !ending
                {
                    *i = (*i as i32 + step).clamp(0, 4) as usize;
                    self.put_session_frame();
                }
            }
            SessionAction::Choose(choice) if armed && !ending => self.session_choose(choice),
            SessionAction::Default if armed && !ending => {
                let choice = self.session_frame_choice();
                self.session_choose(choice);
            }
            SessionAction::Default => {
                // Return on Show {app} once stopped.
                if let Some(Stop::Open(app)) = self
                    .isle
                    .session
                    .ending
                    .as_ref()
                    .and_then(|e| e.stop.clone())
                {
                    self.session_action(SessionAction::ShowApp(app.app));
                }
            }
            SessionAction::Choose(_) => {}
            SessionAction::ShowApp(app) => {
                self.close_session_dialog();
                self.call_off_ending();
                self.activate_app(&app);
            }
            SessionAction::ForceQuit(app) => {
                self.force_quit_app(&app);
                if let Some(ending) = &mut self.isle.session.ending {
                    ending.stop = None;
                    ending.started = Instant::now();
                }
                self.watch_quitting();
                self.put_session_frame();
            }
        }
    }

    /// An action from the chooser or the confirmation.
    fn session_choose(&mut self, choice: Choice) {
        match choice {
            Choice::Lock => {
                self.close_session_dialog();
                self.lock("session dialog");
            }
            Choice::Sleep => {
                self.close_session_dialog();
                self.isle.suspend();
            }
            end => {
                self.stop_countdown();
                self.end_session(end, false);
            }
        }
    }

    /// The engine: every app is asked to quit as by Cmd+Q, keep-running ones included, and
    /// once they are gone the session ends in `choice`.
    pub fn end_session(&mut self, choice: Choice, from_lock: bool) {
        if self.isle.session.ending.is_some() {
            return;
        }
        info!(?choice, from_lock, "ending the session");
        let apps = self.isle.apps_by_recency();
        for app in &apps {
            self.quit_app(app);
        }
        // Apps with no window left get the polite signal alone.
        let windowless: Vec<String> = self
            .isle
            .processes
            .seen()
            .filter(|a| !apps.contains(a))
            .cloned()
            .collect();
        for app in windowless {
            for pid in self.isle.processes.pids(&app) {
                if pid > 1 && pid != std::process::id() as i32 {
                    // Safety: a signal to a process the registry tracks for this app.
                    unsafe { libc::kill(pid, libc::SIGTERM) };
                }
            }
        }
        self.isle.session.ending = Some(Ending {
            choice,
            started: Instant::now(),
            stop: None,
            from_lock,
            finishing: false,
        });
        if from_lock {
            self.isle.session.lock_power = Power::Quitting;
            self.refresh_lock_face();
        }
        self.put_session_frame();
        self.session_check();
        if self
            .isle
            .session
            .ending
            .as_ref()
            .is_some_and(|e| !e.finishing && e.stop.is_none())
        {
            self.watch_quitting();
        }
    }

    fn watch_quitting(&mut self) {
        if self.isle.session.check.is_some() {
            return;
        }
        self.isle.session.check = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(CHECK_EVERY), |_, _, state| {
                state.session_check();
                let waiting = state
                    .isle
                    .session
                    .ending
                    .as_ref()
                    .is_some_and(|e| !e.finishing && e.stop.is_none());
                if waiting {
                    TimeoutAction::ToDuration(CHECK_EVERY)
                } else {
                    state.isle.session.check = None;
                    TimeoutAction::Drop
                }
            })
            .ok();
    }

    /// Every app gone ends the session; one that missed its ping, or is still open after the
    /// wait, stops it.
    fn session_check(&mut self) {
        let Some(ending) = &self.isle.session.ending else {
            return;
        };
        if ending.finishing || ending.stop.is_some() {
            return;
        }
        let (choice, started) = (ending.choice, ending.started);
        let open = self.isle.open_apps();
        let stop = if open.is_empty() {
            None
        } else if let Some(app) = open.iter().find(|a| a.not_responding) {
            Some(Stop::NotResponding(app.clone()))
        } else if started.elapsed() >= QUIT_WAIT {
            Some(Stop::Open(open[0].clone()))
        } else {
            self.put_session_frame();
            return;
        };
        match stop {
            Some(stop) => self.session_stopped(stop),
            None => self.finish_session(choice),
        }
    }

    fn session_stopped(&mut self, stop: Stop) {
        info!(?stop, "the session's end stopped");
        let Some(ending) = &mut self.isle.session.ending else {
            return;
        };
        ending.stop = Some(stop);
        let (choice, from_lock) = (ending.choice, ending.from_lock);
        if from_lock {
            self.isle.session.lock_power = Power::Row;
            self.refresh_lock_face();
            return;
        }
        // Log out with no confirmation opens the dialog to say what stopped it.
        if self.isle.session.dialog.is_none() {
            let ending = self.isle.session.ending.take();
            self.open_session_confirm(choice);
            self.isle.session.ending = ending;
            self.stop_countdown();
        }
        self.put_session_frame();
    }

    fn finish_session(&mut self, choice: Choice) {
        if let Some(ending) = &mut self.isle.session.ending {
            ending.finishing = true;
        }
        info!(?choice, "every app quit");
        match choice {
            Choice::LogOut => self.isle.loop_signal.stop(),
            Choice::Restart | Choice::ShutDown => {
                let restart = choice == Choice::Restart;
                self.isle.run_service(
                    async move {
                        Ok::<_, isle_bus::Error>(crate::services::login::end(restart).await)
                    },
                    |state, result| {
                        let refused = match result {
                            Ok(Ok(())) => return,
                            Ok(Err(why)) | Err(why) => why,
                        };
                        if let Some(ending) = &mut state.isle.session.ending {
                            ending.finishing = false;
                        }
                        state.session_stopped(Stop::Refused(refused));
                    },
                );
            }
            Choice::Lock | Choice::Sleep => {}
        }
    }

    /// The lock's power row: Restart and Shut Down ask once in place.
    pub fn lock_power(&mut self, action: isle_shell::lock::LockAction) {
        use isle_shell::lock::LockAction;
        let power = &mut self.isle.session.lock_power;
        match (action, *power) {
            (LockAction::Restart, Power::Row) => *power = Power::Confirm { restart: true },
            (LockAction::ShutDown, Power::Row) => *power = Power::Confirm { restart: false },
            (LockAction::PowerCancel, Power::Confirm { .. }) => *power = Power::Row,
            (LockAction::PowerConfirm, Power::Confirm { restart }) => {
                let choice = if restart {
                    Choice::Restart
                } else {
                    Choice::ShutDown
                };
                self.isle.session.ending = None;
                self.end_session(choice, true);
                return;
            }
            _ => return,
        }
        self.refresh_lock_face();
    }

    /// Unlocked after an app stopped the lock's restart: that app comes to the front with its
    /// question.
    pub fn session_unlocked(&mut self) {
        self.isle.session.lock_power = Power::Row;
        let Some(ending) = self.isle.session.ending.take_if(|e| e.from_lock) else {
            return;
        };
        if let Some(token) = self.isle.session.check.take() {
            self.isle.loop_handle.remove(token);
        }
        if let Some(Stop::Open(app) | Stop::NotResponding(app)) = ending.stop {
            self.activate_app(&app.app);
        }
    }

    fn put_session_frame(&mut self) {
        let Some(dialog) = &self.isle.session.dialog else {
            return;
        };
        let ending = self.isle.session.ending.as_ref();
        let theme = self.isle.shell.theme().clone();
        let frame = Frame {
            form: dialog.form,
            apps: self.isle.open_apps(),
            countdown: dialog.countdown,
            quitting: ending.map(|e| e.choice),
            stop: ending.and_then(|e| e.stop.clone()),
            armed: dialog.armed,
            solid: self.isle.game_mode_on()
                || self.focus_is_game()
                || theme.appearance.reduce_transparency,
        };
        let surface = self.isle.session.surface.clone();
        surface.set_model(frame.clone());
        self.isle.shell.with_root("session", || surface.set(frame));
        self.dispatch_shell();
    }
}
