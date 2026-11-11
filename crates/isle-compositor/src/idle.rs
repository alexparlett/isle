//! Idle: one clock from the last input, one timer for the next step, what holds it off, the
//! dim, the screens' power and sleep (`p9-lock-and-prompts.md` choices 12, 13 and 15,
//! `p20h-power.md` choices 4–9).

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, atomic::AtomicU32},
    time::{Duration, Instant},
};

use smithay::{
    backend::renderer::element::{
        Kind as ElementKind,
        solid::{SolidColorBuffer, SolidColorRenderElement},
    },
    output::Output,
    reexports::{
        calloop::{
            RegistrationToken,
            timer::{TimeoutAction, Timer},
        },
        wayland_server::{Resource, protocol::wl_surface::WlSurface},
    },
    utils::Scale,
    wayland::idle_inhibit::IdleInhibitHandler,
};
use tracing::{info, warn};

use crate::{
    render::IsleElement,
    state::{Isle, State},
};

/// Idle's steps, in the order they come.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    Dim,
    Lock,
    Off,
}

impl Step {
    const ALL: [Step; 3] = [Step::Dim, Step::Lock, Step::Off];

    pub fn name(self) -> &'static str {
        match self {
            Step::Dim => "dim",
            Step::Lock => "lock",
            Step::Off => "off",
        }
    }
}

/// What a hold holds: a display hold every step, a sleep hold only sleep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Display,
    Sleep,
}

impl Kind {
    /// As logind writes it.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Display => "idle",
            Kind::Sleep => "sleep",
        }
    }
}

/// An `Inhibit` cookie: ScreenSaver's, PowerManagement's or `isle awake`'s.
pub struct Cookie {
    /// The caller's unique bus name; the cookie goes when it leaves.
    pub sender: String,
    pub pid: i32,
    /// The name the caller gave.
    pub app: String,
    /// The app its pid belongs to, when Isle knows it.
    pub app_id: Option<String>,
    pub why: String,
    pub kind: Kind,
}

/// One hold, as listed and as counted.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    pub pid: i32,
    /// The app id where known, else the name the holder gave.
    pub app: String,
    pub why: String,
    pub kind: Kind,
}

/// When each step comes, from the last input; `None` is never.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeouts {
    pub dim: Option<Duration>,
    pub lock: Option<Duration>,
    pub off: Option<Duration>,
}

impl Timeouts {
    fn of(self, step: Step) -> Option<Duration> {
        match step {
            Step::Dim => self.dim,
            Step::Lock => self.lock,
            Step::Off => self.off,
        }
    }
}

/// What idle's clock says now: the steps due that have not come yet, and when the next one
/// is due. Steps in `done` came already, or were passed over when their timeout changed.
pub fn due(
    idle: Duration,
    timeouts: Timeouts,
    done: &HashSet<Step>,
) -> (Vec<Step>, Option<Duration>) {
    let mut now = Vec::new();
    let mut next: Option<Duration> = None;
    for step in Step::ALL {
        if done.contains(&step) {
            continue;
        }
        let Some(after) = timeouts.of(step) else {
            continue;
        };
        if idle >= after {
            now.push(step);
        } else {
            let wait = after - idle;
            next = Some(next.map_or(wait, |n| n.min(wait)));
        }
    }
    (now, next)
}

/// The steps a change of timeouts passes over: those the current idle time is already past,
/// so a change never fires a step at once (v1 D18).
pub fn passed_over(idle: Duration, timeouts: Timeouts, done: &HashSet<Step>) -> HashSet<Step> {
    Step::ALL
        .into_iter()
        .filter(|s| !done.contains(s))
        .filter(|s| timeouts.of(*s).is_some_and(|after| idle >= after))
        .collect()
}

pub struct IdleState {
    pub last_activity: Instant,
    /// Steps that came, or were passed over, since the last input.
    done: HashSet<Step>,
    timeouts: Option<Timeouts>,
    /// Whether the timeouts in force are the lock's.
    for_lock: bool,
    timer: Option<RegistrationToken>,
    /// When the dim began; it fades in from then.
    pub dim_started: Option<Instant>,
    pub off: bool,
    /// Surfaces holding an idle inhibitor.
    pub inhibitors: Vec<WlSurface>,
    /// ScreenSaver's, PowerManagement's and `isle awake`'s cookies.
    pub cookies: HashMap<u32, Cookie>,
    /// The next cookie, shared by every interface that hands them out.
    pub next_cookie: Arc<AtomicU32>,
    /// Keep Awake, while it is on.
    pub keep_awake: Option<crate::keep_awake::KeepAwake>,
    /// The sleep step's timer, running while the screens are off and nothing holds sleep.
    sleep_timer: Option<RegistrationToken>,
    /// logind refused the last idle sleep; none is tried again before the next input.
    sleep_refused: bool,
    /// The holds last counted, so a service's change that leaves them alone is not news.
    last_holds: Vec<Held>,
    /// Each output's dim, by name.
    dims: std::cell::RefCell<HashMap<String, SolidColorBuffer>>,
    /// Outputs that went while the screens were off, by identity, and when.
    pub gone: HashMap<String, Instant>,
    /// Outputs added (true) or removed since the loop last looked, for the screens' power.
    pub output_events: Vec<(Output, bool)>,
    /// Windows changed since the loop last looked, which may change what holds idle.
    pub windows_changed: std::cell::Cell<bool>,
}

impl Default for IdleState {
    fn default() -> Self {
        Self {
            last_activity: Instant::now(),
            done: HashSet::new(),
            timeouts: None,
            for_lock: false,
            timer: None,
            dim_started: None,
            off: false,
            inhibitors: Vec::new(),
            cookies: HashMap::new(),
            next_cookie: Arc::new(AtomicU32::new(1)),
            keep_awake: None,
            sleep_timer: None,
            sleep_refused: false,
            last_holds: Vec::new(),
            dims: Default::default(),
            gone: HashMap::new(),
            output_events: Vec::new(),
            windows_changed: Default::default(),
        }
    }
}

/// A connector back within this long of going, while the screens are off, is the monitor's
/// standby flapping, not a KVM's return.
const FLAP: Duration = Duration::from_secs(10);

impl Isle {
    /// The timeouts in force: the settings' unlocked, the tokens' once locked.
    fn idle_timeouts(&self) -> Timeouts {
        let minutes = |m: i32| (m > 0).then(|| Duration::from_secs(m as u64 * 60));
        if self.lock.locked {
            let t = &self.shell.theme().tokens;
            Timeouts {
                dim: Some(Duration::from_millis(t.timing_lock_dim_ms as u64)),
                lock: None,
                off: Some(Duration::from_millis(t.timing_lock_off_ms as u64)),
            }
        } else {
            let s = &self.settings.lock_screen;
            Timeouts {
                dim: minutes(s.dim_after),
                lock: minutes(s.lock_after),
                off: minutes(s.display_off_after),
            }
        }
    }

    /// Every hold on idle and sleep, a listed app's sleep holds counted as display holds
    /// (`p20h-power.md` choice 16).
    pub fn idle_holds(&self) -> Vec<Held> {
        let mut out = Vec::new();
        let listed = |app: &str, also: Option<&str>| {
            self.settings.energy.display_apps.iter().any(|id| {
                id.eq_ignore_ascii_case(app) || also.is_some_and(|a| id.eq_ignore_ascii_case(a))
            })
        };
        let mut push = |pid: i32, app: String, why: String, kind: Kind, given: Option<&str>| {
            let kind = if kind == Kind::Sleep && listed(&app, given) {
                Kind::Display
            } else {
                kind
            };
            out.push(Held {
                pid,
                app,
                why,
                kind,
            });
        };
        for surface in &self.idle.inhibitors {
            if self.surface_shown(surface) {
                let record = self.wm.for_surface(surface);
                let pid = record
                    .and_then(|r| r.pid)
                    .or_else(|| {
                        let client = surface.client()?;
                        client.get_credentials(&self.display_handle).ok().map(|c| c.pid)
                    })
                    .unwrap_or(0);
                let app = record
                    .map(|r| r.app.clone())
                    .unwrap_or_else(|| "a window".into());
                push(pid, app, String::new(), Kind::Display, None);
            }
        }
        let mut cookies: Vec<_> = self.idle.cookies.iter().collect();
        cookies.sort_by_key(|(n, _)| **n);
        for (_, c) in cookies {
            let app = c.app_id.clone().unwrap_or_else(|| c.app.clone());
            push(c.pid, app, c.why.clone(), c.kind, Some(&c.app));
        }
        let mut portal: Vec<_> = self.portal_holds.iter().collect();
        portal.sort_by_key(|(h, _)| *h);
        for (_, (app, flags, why)) in portal {
            use isle_portal::inhibit::{IDLE, SUSPEND};
            let kind = if flags & IDLE != 0 {
                Kind::Display
            } else if flags & SUSPEND != 0 {
                Kind::Sleep
            } else {
                continue;
            };
            push(0, app.clone(), why.clone(), kind, None);
        }
        let blocks = &self.services.login.state.block_inhibited;
        if blocks.split(':').any(|w| w == "idle") {
            push(0, "logind".into(), String::new(), Kind::Display, None);
        } else if blocks.split(':').any(|w| w == "sleep") {
            push(0, "logind".into(), String::new(), Kind::Sleep, None);
        }
        let front = self
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .and_then(|f| f.surface().and_then(|s| self.wm.for_surface(s)));
        if let Some(record) = front
            && record.game
        {
            let pid = record.pid.unwrap_or(0);
            push(pid, record.app.clone(), "Playing a game".into(), Kind::Display, None);
        }
        if self.services.audio.available {
            for stream in &self.services.audio.state.playing {
                let app = stream.app_id.clone().unwrap_or_else(|| stream.app.clone());
                let pid = stream.pid.unwrap_or(0);
                push(pid, app, "Playing audio".into(), Kind::Sleep, Some(&stream.app));
            }
        }
        if !self.casting.streams.is_empty() || self.image_copy.streaming().next().is_some() {
            push(0, "Screen Sharing".into(), "Sharing the screen".into(), Kind::Sleep, None);
        }
        if self.idle.keep_awake.is_some() {
            let kind = if self.settings.energy.keep_awake_display {
                Kind::Display
            } else {
                Kind::Sleep
            };
            push(0, crate::keep_awake::APP.into(), String::new(), kind, None);
        }
        out
    }

    /// What holds idle's steps off, by name. Nothing holds the lock's own steps.
    pub fn idle_holders(&self) -> Vec<String> {
        if self.lock.locked {
            return Vec::new();
        }
        let mut holders: Vec<String> = self
            .idle_holds()
            .into_iter()
            .filter(|h| h.kind == Kind::Display)
            .map(|h| h.app)
            .collect();
        holders.sort();
        holders.dedup();
        holders
    }

    /// What holds the computer from sleeping by idle, by name: every hold of either kind,
    /// locked or not.
    pub fn sleep_holders(&self) -> Vec<String> {
        let mut holders: Vec<String> = self.idle_holds().into_iter().map(|h| h.app).collect();
        holders.sort();
        holders.dedup();
        holders
    }

    /// Every hold as the bus lists it, its kind written as logind does.
    pub fn listed_holds(&self) -> Vec<isle_ipc::Hold> {
        self.idle_holds()
            .into_iter()
            .map(|h| isle_ipc::Hold {
                pid: h.pid,
                app: h.app,
                what: h.kind.name().into(),
                why: h.why,
            })
            .collect()
    }

    /// A surface counts while it is shown on a lit output: its window neither hidden nor
    /// minimised, or a layer surface mapped.
    fn surface_shown(&self, surface: &WlSurface) -> bool {
        if !surface.is_alive() {
            return false;
        }
        let root = {
            let mut root = surface.clone();
            while let Some(parent) = smithay::wayland::compositor::get_parent(&root) {
                root = parent;
            }
            root
        };
        if let Some(record) = self.wm.for_surface(&root) {
            return self.wm.is_shown(record.id);
        }
        self.space.outputs().any(|o| {
            smithay::desktop::layer_map_for_output(o)
                .layer_for_surface(&root, smithay::desktop::WindowSurfaceType::TOPLEVEL)
                .is_some()
        })
    }

    /// Whether idle may sleep now: the screens off, the switch off, display off not Never,
    /// no refusal since the last input, and nothing holding sleep.
    fn sleep_wanted(&self) -> bool {
        self.idle.off
            && !self.settings.energy.prevent_sleep
            && self.settings.lock_screen.display_off_after > 0
            && !self.idle.sleep_refused
            && !self.asleep
            && !self.sleep_waiting
            && self.sleep_holders().is_empty()
    }

    pub fn dimmed(&self) -> bool {
        self.idle.dim_started.is_some()
    }

    /// The dim's elements over everything on `output`, front first.
    pub fn dim_elements(&self, output: &Output, push: &mut dyn FnMut(IsleElement)) {
        let Some(start) = self.idle.dim_started else {
            return;
        };
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        let t = &self.shell.theme().tokens;
        let fade = Duration::from_millis(t.motion_dim_ms as u64)
            .as_secs_f32()
            .max(1e-3);
        let alpha = (start.elapsed().as_secs_f32() / fade).min(1.0);
        let mut dims = self.idle.dims.borrow_mut();
        let buffer = dims.entry(output.name()).or_default();
        buffer.update(geometry.size, crate::wallpaper::premultiplied(t.colour_dim));
        let scale = Scale::from(output.current_scale().fractional_scale());
        push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
            buffer,
            (0, 0),
            scale,
            alpha,
            ElementKind::Unspecified,
        )));
    }

    /// The dim still fades in.
    pub fn dimming(&self) -> bool {
        let fade = Duration::from_millis(self.shell.theme().tokens.motion_dim_ms as u64);
        self.idle.dim_started.is_some_and(|s| s.elapsed() < fade)
    }

    pub fn idle_status(&self) -> String {
        serde_json::json!({
            "idle_ms": self.idle.last_activity.elapsed().as_millis() as u64,
            "dimmed": self.dimmed(),
            "off": self.idle.off,
            "done": self.idle.done.iter().map(|s| s.name()).collect::<Vec<_>>(),
            "holders": self.idle_holders(),
            "sleep_holders": self.sleep_holders(),
            "sleep_armed": self.idle.sleep_timer.is_some(),
            "keep_awake": self.keep_awake_status(),
        })
        .to_string()
    }
}

impl State {
    /// Any input: idle starts again. Returns whether the input should be dropped, since it
    /// woke screens that were off and lands on nothing seen.
    pub fn idle_activity(&mut self) -> bool {
        let woke = self.isle.idle.off;
        self.isle.idle.last_activity = Instant::now();
        self.isle.idle.done.clear();
        self.isle.idle.sleep_refused = false;
        self.isle
            .idle_notifier_state
            .notify_activity(&self.isle.seat);
        let was_dimmed = self.isle.dimmed();
        if was_dimmed || woke {
            self.isle.idle.dim_started = None;
            if woke {
                self.screens_on();
            }
            self.isle.queue_redraw_all();
            self.idle_hint(false);
            self.silence_check();
        }
        self.arm_idle();
        woke
    }

    /// The timer for the next step, from the clock and the timeouts in force; held, none.
    pub fn arm_idle(&mut self) {
        if let Some(token) = self.isle.idle.timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let held = !self.isle.idle_holders().is_empty();
        self.isle.idle_notifier_state.set_is_inhibited(held);
        let timeouts = self.isle.idle_timeouts();
        if self.isle.idle.for_lock != self.isle.lock.locked {
            // Locking or unlocking starts the period again under the other timeouts; a dim
            // already on stays.
            self.isle.idle.for_lock = self.isle.lock.locked;
            self.isle.idle.last_activity = Instant::now();
            self.isle.idle.done.clear();
            if self.isle.dimmed() {
                self.isle.idle.done.insert(Step::Dim);
            }
        } else if self.isle.idle.timeouts.is_some_and(|t| t != timeouts) {
            let idle = self.isle.idle.last_activity.elapsed();
            let passed = passed_over(idle, timeouts, &self.isle.idle.done);
            self.isle.idle.done.extend(passed);
        }
        self.isle.idle.timeouts = Some(timeouts);
        if held {
            return;
        }
        let idle = self.isle.idle.last_activity.elapsed();
        let (now, next) = due(idle, timeouts, &self.isle.idle.done);
        for step in now {
            self.idle_step(step);
        }
        let Some(wait) = next else {
            return;
        };
        self.isle.idle.timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(wait), |_, _, state| {
                state.isle.idle.timer = None;
                state.arm_idle();
                TimeoutAction::Drop
            })
            .ok();
    }

    fn idle_step(&mut self, step: Step) {
        self.isle.idle.done.insert(step);
        info!(step = step.name(), "idle");
        match step {
            Step::Dim => {
                self.isle.idle.dim_started = Some(Instant::now());
                self.isle.queue_redraw_all();
                self.idle_hint(true);
                self.silence_check();
            }
            Step::Lock => self.lock("idle"),
            Step::Off => self.screens_off(),
        }
    }

    /// What holds idle changed: a hold that ends counts as input, so a video that stops
    /// leaves the whole timeout before the screen dims.
    pub fn idle_holds_changed(&mut self) {
        self.isle.idle.last_holds = self.isle.idle_holds();
        self.isle.notify(crate::worker::Event::HoldsChanged);
        let held = !self.isle.idle_holders().is_empty();
        if !held && self.isle.idle_notifier_state.is_inhibited() {
            self.isle.idle.last_activity = Instant::now();
            self.isle.idle.done.clear();
        }
        self.arm_idle();
        self.arm_sleep();
    }

    /// A service changed that may hold sleep (sound, a cast): news only if the holds did.
    pub fn holds_maybe_changed(&mut self) {
        if self.isle.idle_holds() != self.isle.idle.last_holds {
            self.idle_holds_changed();
        }
    }

    /// The sleep step's timer: running while the screens are off, sleep is allowed and
    /// nothing holds it; a timer already running keeps its start (choice 8).
    pub fn arm_sleep(&mut self) {
        if !self.isle.sleep_wanted() {
            if let Some(token) = self.isle.idle.sleep_timer.take() {
                self.isle.loop_handle.remove(token);
            }
            return;
        }
        if self.isle.idle.sleep_timer.is_some() {
            return;
        }
        let after = self.isle.shell.theme().tokens.timing_sleep_after_off_ms as u64;
        self.isle.idle.sleep_timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(Duration::from_millis(after)), |_, _, state| {
                state.isle.idle.sleep_timer = None;
                state.idle_sleep();
                TimeoutAction::Drop
            })
            .ok();
    }

    /// Idle's last step: sleep through logind, which announces it so the lock comes first.
    fn idle_sleep(&mut self) {
        if !self.isle.sleep_wanted() {
            return;
        }
        info!(step = "sleep", "idle");
        self.isle.run_service(crate::services::login::suspend(), |state, result| {
            if let Err(err) = result {
                warn!("logind refused idle sleep: {err}");
                state.isle.idle.sleep_refused = true;
            }
        });
    }

    fn idle_hint(&mut self, idle: bool) {
        self.isle.run_service(
            crate::services::login::set_hint(None, Some(idle)),
            |_, result| {
                if let Err(err) = result {
                    tracing::debug!("logind's idle hint: {err}");
                }
            },
        );
    }

    /// Every screen off: the outputs disabled and drawing stopped.
    pub fn screens_off(&mut self) {
        if self.isle.idle.off {
            return;
        }
        info!("screens off");
        self.isle.idle.off = true;
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        for output in &outputs {
            self.backend.set_power(output, false);
        }
        self.silence_check();
        self.sleep_ready();
        self.arm_sleep();
    }

    fn screens_on(&mut self) {
        info!("screens on");
        self.isle.idle.off = false;
        self.arm_sleep();
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        for output in &outputs {
            self.backend.set_power(output, true);
            self.isle.queue_redraw(output);
        }
    }

    /// An output came: one back after a real absence wakes the screens as a KVM's return
    /// would (D76); a quick return while off is standby flapping, and it stays off.
    pub fn idle_output_added(&mut self, output: &Output) {
        let identity = Isle::output_identity(output);
        let flapped = self
            .isle
            .idle
            .gone
            .remove(&identity)
            .is_some_and(|at| at.elapsed() < FLAP);
        if self.isle.idle.off {
            if flapped {
                self.backend.set_power(output, false);
            } else {
                self.idle_activity();
            }
        }
    }

    pub fn idle_output_removed(&mut self, output: &Output) {
        if self.isle.idle.off {
            self.isle
                .idle
                .gone
                .insert(Isle::output_identity(output), Instant::now());
        }
    }

    /// After each dispatch: outputs that came or went, and windows that changed what holds.
    pub fn idle_after_dispatch(&mut self) {
        for (output, added) in std::mem::take(&mut self.isle.idle.output_events) {
            if added {
                self.idle_output_added(&output);
            } else {
                self.idle_output_removed(&output);
            }
        }
        if self.isle.idle.windows_changed.take() {
            self.idle_holds_changed();
            self.refresh_prompts();
        }
    }

    /// `isle displays sleep`: off now, as idle's last step.
    pub fn displays_sleep(&mut self) {
        self.isle.idle.done.extend(Step::ALL);
        self.screens_off();
    }

    /// Test hook: the sleep step's timer fires now, if it runs.
    #[cfg(feature = "test-hooks")]
    pub fn sleep_now(&mut self) -> bool {
        let Some(token) = self.isle.idle.sleep_timer.take() else {
            return false;
        };
        self.isle.loop_handle.remove(token);
        self.idle_sleep();
        true
    }

    /// Test hook: the idle clock moved on by `by`.
    #[cfg(feature = "test-hooks")]
    pub fn idle_advance(&mut self, by: Duration) {
        self.isle.idle.last_activity -= by;
        self.arm_idle();
    }
}

impl IdleInhibitHandler for State {
    fn inhibit(&mut self, surface: WlSurface) {
        self.isle.idle.inhibitors.push(surface);
        self.idle_holds_changed();
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        self.isle.idle.inhibitors.retain(|s| s != &surface);
        self.idle_holds_changed();
    }
}

impl smithay::wayland::idle_notify::IdleNotifierHandler for State {
    fn idle_notifier_state(
        &mut self,
    ) -> &mut smithay::wayland::idle_notify::IdleNotifierState<Self> {
        &mut self.isle.idle_notifier_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: Duration = Duration::from_secs(60);

    fn timeouts(dim: u64, lock: u64, off: u64) -> Timeouts {
        let m = |n: u64| (n > 0).then(|| MIN * n as u32);
        Timeouts {
            dim: m(dim),
            lock: m(lock),
            off: m(off),
        }
    }

    #[test]
    fn steps_come_in_order_each_once() {
        let t = timeouts(5, 10, 15);
        let mut done = HashSet::new();
        assert_eq!(due(MIN * 4, t, &done), (vec![], Some(MIN)));
        assert_eq!(due(MIN * 5, t, &done), (vec![Step::Dim], Some(MIN * 5)));
        done.insert(Step::Dim);
        assert_eq!(due(MIN * 12, t, &done), (vec![Step::Lock], Some(MIN * 3)));
        done.insert(Step::Lock);
        assert_eq!(due(MIN * 15, t, &done), (vec![Step::Off], None));
    }

    #[test]
    fn zero_is_never_and_never_fires_at_once() {
        let t = timeouts(0, 0, 0);
        assert_eq!(due(Duration::ZERO, t, &HashSet::new()), (vec![], None));
        assert_eq!(due(MIN * 600, t, &HashSet::new()), (vec![], None));
    }

    #[test]
    fn a_timeout_lowered_below_the_idle_time_waits_for_the_next_idle_period() {
        let before = timeouts(5, 10, 15);
        let mut done = HashSet::new();
        // Seven minutes idle, dimmed; the lock is moved to three minutes.
        done.insert(Step::Dim);
        let after = timeouts(5, 3, 15);
        let passed = passed_over(MIN * 7, after, &done);
        assert_eq!(passed, HashSet::from([Step::Lock]));
        done.extend(passed);
        assert_eq!(due(MIN * 7, after, &done), (vec![], Some(MIN * 8)));
        let _ = before;
    }

    #[test]
    fn a_timeout_raised_counts_from_the_current_idle_time() {
        let t = timeouts(5, 20, 30);
        let done = HashSet::from([Step::Dim]);
        // Ten minutes idle: the lock is ten minutes on.
        assert_eq!(due(MIN * 10, t, &done), (vec![], Some(MIN * 10)));
    }
}
