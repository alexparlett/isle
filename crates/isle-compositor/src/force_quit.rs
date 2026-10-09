//! Force Quit on the loop: the panel of running apps with its sheet, and the alert hung on a
//! frozen window when close is pressed (`p9-lock-and-prompts.md` choice 23).

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    time::{Duration, Instant},
};

use isle_shell::{
    alert::{AlertAction, NotRespondingAlert},
    force_quit::{ForceQuitAction, ForceQuitSurface, Frame, Row},
};
use smithay::{
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Logical, Point, Rectangle},
};
use tracing::info;

use crate::{
    focus::Focus,
    state::{Isle, State},
};

/// An alert's key among attached surfaces: its window's id with this bit set, apart from the
/// window's bar.
const ALERT: u64 = 1 << 62;
const REFRESH_EVERY: Duration = Duration::from_millis(500);
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

pub fn alert_key(id: u64) -> u64 {
    id | ALERT
}

/// The window an attached key's alert hangs on.
pub fn alert_window(key: u64) -> Option<u64> {
    (key & ALERT != 0).then_some(key & !ALERT)
}

pub struct ForceQuitState {
    pub surface: Rc<ForceQuitSurface>,
    pub open: bool,
    selected: Vec<String>,
    asking: bool,
    /// Apps whose kill was refused.
    failed: HashSet<String>,
    last_press: Option<(String, Instant)>,
    refresh: Option<RegistrationToken>,
    /// The alerts hung on frozen windows, by window.
    pub alerts: HashMap<u64, Rc<NotRespondingAlert>>,
}

impl ForceQuitState {
    pub fn new() -> Self {
        Self {
            surface: ForceQuitSurface::new(),
            open: false,
            selected: Vec::new(),
            asking: false,
            failed: HashSet::new(),
            last_press: None,
            refresh: None,
            alerts: HashMap::new(),
        }
    }
}

impl Isle {
    /// Every running app: those with windows by recency, then those running without one.
    fn force_quit_rows(&self) -> Vec<Row> {
        let mut apps = self.apps_by_recency();
        for app in self.processes.seen() {
            if !apps.contains(app) {
                apps.push(app.clone());
            }
        }
        let size = self.shell.theme().tokens.size_app_icon_small as u32;
        apps.into_iter()
            .map(|app| Row {
                name: self.app_name(&app),
                icon: self.app_icon(&app, size),
                not_responding: self
                    .wm
                    .records
                    .iter()
                    .any(|r| r.app == app && r.not_responding),
                failed: self.force_quit.failed.contains(&app),
                app,
            })
            .collect()
    }

    pub fn force_quit_status(&self) -> String {
        serde_json::json!({
            "open": self.force_quit.open,
            "rows": self.force_quit_rows().iter().map(|r| serde_json::json!({
                "app": r.app,
                "not_responding": r.not_responding,
                "failed": r.failed,
            })).collect::<Vec<_>>(),
            "selected": self.force_quit.selected,
            "asking": self.force_quit.asking,
            "alerts": self.force_quit.alerts.keys().collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Where `id`'s alert hangs: centred under its bar, as an offset from its frame, and its
    /// size.
    fn alert_box(&self, id: u64) -> Option<Rectangle<i32, Logical>> {
        let frame = self.frame(id)?;
        let theme = self.shell.theme();
        let size = NotRespondingAlert::size_for(theme);
        let below = if self.wm.get(id)?.bar.is_some() {
            self.bar_height(id)
        } else {
            0
        };
        Some(Rectangle::new(
            ((frame.size.w - size.w) / 2, below).into(),
            size,
        ))
    }

    /// The alert on `id` drawn at the frame's `frame_at`, physical on the output being drawn.
    pub fn alert_element(
        &self,
        id: u64,
        frame_at: Point<i32, Logical>,
        scale: f64,
        alpha: f32,
    ) -> Option<isle_shell::ShellElement> {
        if !self.force_quit.alerts.contains_key(&id) {
            return None;
        }
        let at = frame_at + self.alert_box(id)?.loc;
        self.shell
            .attached_element(alert_key(id), at.to_f64().to_physical(scale), alpha)
    }

    /// The alert under `pos` on `id`, and its top left.
    pub fn alert_at(
        &self,
        id: u64,
        pos: Point<f64, Logical>,
    ) -> Option<(Focus, Point<f64, Logical>)> {
        if !self.force_quit.alerts.contains_key(&id) {
            return None;
        }
        let frame = self.frame(id)?;
        let place = self.alert_box(id)?;
        let at = frame.loc + place.loc;
        Rectangle::new(at, place.size)
            .to_f64()
            .contains(pos)
            .then(|| self.shell.attached_focus(alert_key(id)))
            .flatten()
            .map(|focus| (Focus::Shell(focus), at.to_f64()))
    }

    /// Keeps `id`'s alert where its window is.
    pub fn place_alert(&mut self, id: u64) {
        if !self.force_quit.alerts.contains_key(&id) {
            return;
        }
        let (Some(frame), Some(place), Some(output)) =
            (self.frame(id), self.alert_box(id), self.output_of(id))
        else {
            return;
        };
        let Some(geometry) = self.space.output_geometry(&output) else {
            return;
        };
        let at = frame.loc + place.loc - geometry.loc;
        let output_place = isle_shell::OutputPlace {
            scale: output.current_scale().fractional_scale(),
            output,
            geometry,
        };
        self.shell
            .place_attached(alert_key(id), &output_place, Rectangle::new(at, place.size));
    }
}

impl State {
    /// Cmd+Option+Escape and Force Quit… in the system menu.
    pub fn open_force_quit(&mut self) {
        if self.isle.lock.locked || self.isle.force_quit.open {
            return;
        }
        let Some(output) = self.isle.main_output() else {
            return;
        };
        self.cancel_switcher();
        self.close_session_dialog();
        self.close_spotlight();
        self.close_toolbar();
        self.close_menu_bar_group();
        // Each window is asked again, so "Not responding" is current.
        let ids: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| r.mapped)
            .map(|r| r.id)
            .collect();
        for id in ids {
            self.ping(id);
        }
        let rows = self.isle.force_quit_rows();
        // A frozen app first, else the front one.
        let front = self.isle.front_app();
        let first = rows
            .iter()
            .find(|r| r.not_responding)
            .or_else(|| rows.iter().find(|r| Some(&r.app) == front.as_ref()))
            .or(rows.first())
            .map(|r| r.app.clone());
        let fq = &mut self.isle.force_quit;
        fq.open = true;
        fq.asking = false;
        fq.failed.clear();
        fq.selected = first.into_iter().collect();
        info!(selected = ?fq.selected, "force quit");
        let theme = self.isle.shell.theme().clone();
        let screen = self.isle.space.output_geometry(&output).unwrap_or_default();
        let size = ForceQuitSurface::size_for(&theme);
        let geometry = Rectangle::new(
            ((screen.size.w - size.w) / 2, (screen.size.h - size.h) / 2).into(),
            size,
        );
        self.isle.force_quit.surface.set_size(size);
        self.put_force_quit_frame();
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        self.isle.shell.open_placed("force-quit", &place, geometry);
        if let Some(focus) = self.isle.shell.keyboard_taker("force-quit") {
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            let previous = keyboard.current_focus();
            self.isle
                .shell_keyboard_return
                .push((focus.clone(), previous));
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.focus_keyboard(Some(Focus::Shell(focus)), serial);
        }
        self.dispatch_shell();
        // The list follows apps quitting and freezing while it is open.
        self.isle.force_quit.refresh = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(REFRESH_EVERY), |_, _, state| {
                if !state.isle.force_quit.open {
                    state.isle.force_quit.refresh = None;
                    return TimeoutAction::Drop;
                }
                state.put_force_quit_frame();
                TimeoutAction::ToDuration(REFRESH_EVERY)
            })
            .ok();
        self.isle.queue_redraw_all();
    }

    pub fn close_force_quit(&mut self) {
        if !std::mem::take(&mut self.isle.force_quit.open) {
            return;
        }
        if let Some(token) = self.isle.force_quit.refresh.take() {
            self.isle.loop_handle.remove(token);
        }
        self.isle.force_quit.asking = false;
        self.close_surface("force-quit");
    }

    pub fn force_quit_action(&mut self, action: ForceQuitAction) {
        let rows: Vec<String> = self
            .isle
            .force_quit_rows()
            .into_iter()
            .map(|r| r.app)
            .collect();
        let fq = &mut self.isle.force_quit;
        match action {
            ForceQuitAction::Close => return self.close_force_quit(),
            _ if fq.asking => match action {
                ForceQuitAction::Cancel => fq.asking = false,
                ForceQuitAction::Confirm => {
                    fq.asking = false;
                    let apps = fq.selected.clone();
                    for app in apps {
                        if !self.force_quit_app(&app) {
                            self.isle.force_quit.failed.insert(app);
                        }
                    }
                }
                _ => return,
            },
            ForceQuitAction::Select(app) => {
                let now = Instant::now();
                let double = fq
                    .last_press
                    .as_ref()
                    .is_some_and(|(a, at)| *a == app && now - *at < DOUBLE_CLICK);
                fq.last_press = Some((app.clone(), now));
                let mods = self.held_mods();
                let fq = &mut self.isle.force_quit;
                if mods.cmd {
                    match fq.selected.iter().position(|a| *a == app) {
                        Some(at) => {
                            fq.selected.remove(at);
                        }
                        None => fq.selected.push(app),
                    }
                } else if mods.shift
                    && let Some(anchor) = fq.selected.first()
                    && let (Some(from), Some(to)) = (
                        rows.iter().position(|a| a == anchor),
                        rows.iter().position(|a| *a == app),
                    )
                {
                    let (lo, hi) = (from.min(to), from.max(to));
                    let anchor = anchor.clone();
                    fq.selected = std::iter::once(anchor.clone())
                        .chain(rows[lo..=hi].iter().filter(|a| **a != anchor).cloned())
                        .collect();
                } else {
                    fq.selected = vec![app];
                    if double {
                        fq.asking = true;
                    }
                }
            }
            ForceQuitAction::Step { step, extend } => {
                if rows.is_empty() {
                    return;
                }
                let at = fq
                    .selected
                    .last()
                    .and_then(|a| rows.iter().position(|r| r == a));
                let next = match at {
                    Some(at) => (at as i32 + step).clamp(0, rows.len() as i32 - 1) as usize,
                    None => 0,
                };
                let app = rows[next].clone();
                if extend {
                    fq.selected.retain(|a| *a != app);
                    fq.selected.push(app);
                } else {
                    fq.selected = vec![app];
                }
            }
            ForceQuitAction::Ask => {
                if !fq.selected.is_empty() {
                    fq.asking = true;
                }
            }
            ForceQuitAction::Confirm | ForceQuitAction::Cancel => return,
        }
        self.put_force_quit_frame();
    }

    fn put_force_quit_frame(&mut self) {
        let rows = self.isle.force_quit_rows();
        let fq = &mut self.isle.force_quit;
        // The selection stays on apps still running.
        fq.selected.retain(|a| rows.iter().any(|r| &r.app == a));
        let frame = Frame {
            rows,
            selected: fq.selected.clone(),
            asking: fq.asking,
            solid: self.isle.game_mode_on()
                || self.isle.shell.theme().appearance.reduce_transparency,
        };
        let surface = self.isle.force_quit.surface.clone();
        surface.set_model(frame.clone());
        self.isle
            .shell
            .with_root("force-quit", || surface.set(frame));
        self.dispatch_shell();
    }

    /// Close on a window whose app is not responding: the alert, rather than a request it
    /// will not answer.
    pub fn show_alert(&mut self, id: u64) {
        let Some(app) = self.isle.wm.get(id).map(|r| r.app.clone()) else {
            return;
        };
        if !self.isle.force_quit.alerts.contains_key(&id) {
            info!(id, app, "not responding: the alert");
            let def = NotRespondingAlert::new(self.isle.app_name(&app));
            let (Some(frame), Some(place), Some(output)) = (
                self.isle.frame(id),
                self.isle.alert_box(id),
                self.isle.output_of(id),
            ) else {
                return;
            };
            let Some(geometry) = self.isle.space.output_geometry(&output) else {
                return;
            };
            let output_place = isle_shell::OutputPlace {
                scale: output.current_scale().fractional_scale(),
                output,
                geometry,
            };
            let at = frame.loc + place.loc - geometry.loc;
            self.isle.shell.attach(
                def.clone(),
                alert_key(id),
                &output_place,
                Rectangle::new(at, place.size),
            );
            self.isle.force_quit.alerts.insert(id, def);
        }
        if let Some(focus) = self.isle.shell.attached_focus(alert_key(id)) {
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            let previous = keyboard.current_focus();
            if previous.as_ref() != Some(&Focus::Shell(focus.clone())) {
                self.isle
                    .shell_keyboard_return
                    .push((focus.clone(), previous));
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                self.focus_keyboard(Some(Focus::Shell(focus)), serial);
            }
        }
        self.dispatch_shell();
        self.isle.queue_redraw_all();
    }

    /// Takes `id`'s alert down, the keyboard back where it was.
    pub fn close_alert(&mut self, id: u64) {
        if self.isle.force_quit.alerts.remove(&id).is_none() {
            return;
        }
        let focus = self.isle.shell.attached_focus(alert_key(id));
        self.isle.shell.detach(alert_key(id));
        if let Some(focus) = focus
            && let Some(index) = self
                .isle
                .shell_keyboard_return
                .iter()
                .position(|(f, _)| *f == focus)
        {
            let (_, previous) = self.isle.shell_keyboard_return.remove(index);
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            if keyboard.current_focus() == Some(Focus::Shell(focus)) {
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                match previous.filter(smithay::utils::IsAlive::alive) {
                    Some(previous) => self.focus_keyboard(Some(previous), serial),
                    None => self.focus_next(),
                }
            }
        }
        self.isle.queue_redraw_all();
    }

    pub fn alert_action(&mut self, id: u64, action: AlertAction) {
        match action {
            AlertAction::Wait => self.close_alert(id),
            AlertAction::ForceQuit => {
                let app = self.isle.wm.get(id).map(|r| r.app.clone());
                self.close_alert(id);
                if let Some(app) = app {
                    self.force_quit_app(&app);
                }
            }
        }
    }

    /// The app answered again: its windows' alerts go.
    pub fn alerts_answered(&mut self, ids: &[u64]) {
        for id in ids {
            self.close_alert(*id);
        }
    }
}
