//! System prompts on the loop: the agents' requests, one queue deciding which shows, the
//! frame on its screen with the keyboard, the scrim the compositor draws under it, and the
//! island's activity for those waiting (`p9-lock-and-prompts.md` choices 18, 19 and 21).

use std::{
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use isle_shell::prompts::{
    Frame, Prompt, PromptAction, PromptSurface, Status,
    queue::{Entry, Holds, Queue},
};
use isle_ui::Secret;
use smithay::{
    backend::renderer::element::{
        Kind,
        solid::{SolidColorBuffer, SolidColorRenderElement},
    },
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Rectangle, Scale},
};
use tokio::sync::mpsc;
use tracing::info;

use crate::{
    focus::Focus,
    render::IsleElement,
    state::{Isle, State},
    worker::{LoopTimeout, ToLoop},
};

/// The user's answer to a prompt, or its cancel.
#[derive(Clone, Debug)]
pub enum Answer {
    Values {
        values: Vec<(String, Secret)>,
        choices: Vec<(String, bool)>,
        identity: usize,
    },
    Cancelled,
}

impl Answer {
    /// A field's value by its id.
    pub fn value(&self, id: &str) -> Option<&Secret> {
        match self {
            Answer::Values { values, .. } => values.iter().find(|(k, _)| k == id).map(|(_, v)| v),
            Answer::Cancelled => None,
        }
    }

    pub fn choice(&self, id: &str) -> Option<bool> {
        match self {
            Answer::Values { choices, .. } => {
                choices.iter().find(|(k, _)| k == id).map(|(_, v)| *v)
            }
            Answer::Cancelled => None,
        }
    }
}

/// What an agent asks: the prompt's content, the asking process, and whether the answer is
/// checked before the prompt goes (a password), so it waits showing "Checking".
#[derive(Clone, Debug, Default)]
pub struct Ask {
    pub prompt: Prompt,
    /// The title when the asking app is not known; `{app}` in the prompt's title is its name.
    pub title_unknown: Option<String>,
    pub pid: Option<i32>,
    pub user_started: bool,
    pub checks: bool,
    /// What the request is about, a device by its path, which Control Centre may have asked
    /// for a moment before.
    pub subject: Option<String>,
}

struct Request {
    prompt: Prompt,
    app: Option<String>,
    checks: bool,
    replies: mpsc::UnboundedSender<Answer>,
}

pub struct PromptsState {
    pub queue: Queue,
    pub surface: Rc<PromptSurface>,
    requests: HashMap<u64, Request>,
    next: u64,
    /// The prompt on screen, the output it is on, and since when.
    pub shown: Option<(u64, String, Instant)>,
    pub armed: bool,
    arm_timer: Option<RegistrationToken>,
    scrims: std::cell::RefCell<HashMap<String, SolidColorBuffer>>,
    /// Answers the Test interface's own prompts received, as `id field=value ...`.
    #[cfg(feature = "test-hooks")]
    pub test_answers: Vec<String>,
}

impl PromptsState {
    pub fn new() -> Self {
        Self {
            queue: Queue::default(),
            surface: PromptSurface::new(),
            requests: HashMap::new(),
            next: 1,
            shown: None,
            armed: false,
            arm_timer: None,
            scrims: Default::default(),
            #[cfg(feature = "test-hooks")]
            test_answers: Vec::new(),
        }
    }
}

/// How the agents on the worker reach the queue.
#[derive(Clone)]
pub struct PromptClient(pub ToLoop);

impl PromptClient {
    /// Queues a prompt; its answers, and a cancel, arrive on the receiver.
    pub async fn ask(
        &self,
        ask: Ask,
    ) -> Result<(u64, mpsc::UnboundedReceiver<Answer>), LoopTimeout> {
        let (send, receive) = mpsc::unbounded_channel();
        let id = self
            .0
            .call(move |state| state.prompt_ask(ask, send))
            .await?;
        Ok((id, receive))
    }

    /// The answer was wrong: the prompt asks again with `message`, its secrets cleared.
    pub fn wrong(&self, id: u64, message: String) {
        self.0.post(move |state| state.prompt_wrong(id, message));
    }

    /// A message from the requester on the way, or a status back to answerable.
    pub fn status(&self, id: u64, status: Status) {
        self.0.post(move |state| state.prompt_status(id, status));
    }

    /// Changes the prompt's content in place: a pairing code's digits typed.
    pub fn update(&self, id: u64, f: impl FnOnce(&mut Prompt) + Send + 'static) {
        self.0.post(move |state| {
            if let Some(request) = state.isle.prompts.requests.get_mut(&id) {
                f(&mut request.prompt);
                state.refresh_prompts();
            }
        });
    }

    /// Answered, or the requester cancelled: the prompt goes, silently.
    pub fn done(&self, id: u64) {
        self.0.post(move |state| state.prompt_done(id));
    }
}

impl Isle {
    /// The prompt on screen, by id.
    pub fn prompt_showing(&self) -> Option<u64> {
        self.prompts.shown.as_ref().map(|(id, _, _)| *id)
    }

    /// The scrim under a prompt on `output`: every screen dimmed while one shows.
    pub fn scrim_elements(&self, output: &Output, push: &mut dyn FnMut(IsleElement)) {
        let Some((_, _, since)) = &self.prompts.shown else {
            return;
        };
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        let t = &self.shell.theme().tokens;
        let alpha = if self.shell.animated() {
            let fade = Duration::from_millis(t.motion_fade_ms as u64)
                .as_secs_f32()
                .max(1e-3);
            (since.elapsed().as_secs_f32() / fade).min(1.0)
        } else {
            1.0
        };
        let mut scrims = self.prompts.scrims.borrow_mut();
        let buffer = scrims.entry(output.name()).or_default();
        buffer.update(
            geometry.size,
            crate::wallpaper::premultiplied(t.colour_scrim),
        );
        let scale = Scale::from(output.current_scale().fractional_scale());
        push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
            buffer,
            (0, 0),
            scale,
            alpha,
            Kind::Unspecified,
        )));
    }

    /// The scrim still fades in.
    pub fn scrim_fading(&self) -> bool {
        let fade = Duration::from_millis(self.shell.theme().tokens.motion_fade_ms as u64);
        self.prompts
            .shown
            .as_ref()
            .is_some_and(|(_, _, since)| since.elapsed() < fade)
    }

    fn prompt_holds(&self) -> Holds {
        let front = self
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .and_then(|f| f.surface().and_then(|s| self.wm.for_surface(s)));
        let full_screen = front.filter(|r| matches!(r.mode, crate::wm::Mode::FullScreen { .. }));
        Holds {
            locked: self.lock.locked,
            game_in_front: full_screen.is_some_and(|r| r.game),
            full_screen_app: full_screen.map(|r| r.app.clone()),
        }
    }

    pub fn prompts_status(&self) -> String {
        let holds = self.prompt_holds();
        #[allow(unused_mut)]
        let mut status = serde_json::json!({
            "showing": self.prompt_showing(),
            "armed": self.prompts.armed,
            "waiting": self.prompts.queue.waiting(&holds).iter()
                .map(|(id, why)| serde_json::json!({"id": id, "why": format!("{why:?}").to_lowercase()}))
                .collect::<Vec<_>>(),
            "title": self.prompt_showing()
                .and_then(|id| self.prompts.requests.get(&id))
                .map(|r| r.prompt.title.clone()),
            "status": self.prompt_showing()
                .and_then(|id| self.prompts.requests.get(&id))
                .map(|r| format!("{:?}", r.prompt.status)),
        });
        #[cfg(feature = "test-hooks")]
        {
            status["answers"] = serde_json::json!(self.prompts.test_answers);
        }
        status.to_string()
    }
}

impl State {
    /// Takes an agent's request into the queue.
    pub fn prompt_ask(&mut self, ask: Ask, replies: mpsc::UnboundedSender<Answer>) -> u64 {
        let id = self.isle.prompts.next;
        self.isle.prompts.next += 1;
        let app = ask.pid.and_then(|pid| self.isle.processes.app_of(pid));
        let mut prompt = ask.prompt;
        prompt.id = id;
        if let Some(fallback) = ask.title_unknown {
            let name = app.as_ref().map(|a| self.isle.app_name(a)).filter(|n| !n.is_empty());
            prompt.title = match name {
                Some(name) => prompt.title.replace("{app}", &name),
                None => fallback,
            };
        }
        if prompt.icon.is_none()
            && prompt.glyph.is_none()
            && let Some(app) = &app
        {
            prompt.icon = self.isle.app_icon(app, 48);
        }
        // `pkexec` in the front terminal, or a device Control Centre is pairing, is the user's
        // own request.
        let front = self.isle.front_app();
        let user_started = ask.user_started
            || (app.is_some() && app == front)
            || ask
                .subject
                .as_ref()
                .is_some_and(|s| self.isle.services.connecting.contains(s));
        info!(id, title = prompt.title, ?app, user_started, "prompt");
        let showing = self.isle.prompt_showing();
        self.isle.prompts.queue.push(
            Entry {
                id,
                app: app.clone(),
                user_started,
            },
            showing,
        );
        self.isle.prompts.requests.insert(
            id,
            Request {
                prompt,
                app,
                checks: ask.checks,
                replies,
            },
        );
        self.refresh_prompts();
        id
    }

    pub fn prompt_action(&mut self, action: PromptAction) {
        match action {
            PromptAction::Answer {
                id,
                values,
                choices,
                identity,
            } => {
                if !self.isle.prompts.armed || self.isle.prompt_showing() != Some(id) {
                    return;
                }
                let Some(request) = self.isle.prompts.requests.get_mut(&id) else {
                    return;
                };
                #[cfg(feature = "test-hooks")]
                {
                    let fields: Vec<String> = values
                        .iter()
                        .map(|(k, v)| format!("{k}={}", v.expose()))
                        .chain(choices.iter().map(|(k, v)| format!("{k}={v}")))
                        .collect();
                    self.isle
                        .prompts
                        .test_answers
                        .push(format!("{id} {}", fields.join(" ")));
                }
                let _ = request.replies.send(Answer::Values {
                    values,
                    choices,
                    identity,
                });
                if request.checks {
                    request.prompt.status = Status::Checking;
                    self.refresh_prompts();
                } else {
                    self.prompt_done(id);
                }
            }
            PromptAction::Cancel(id) => {
                if let Some(request) = self.isle.prompts.requests.get(&id) {
                    let _ = request.replies.send(Answer::Cancelled);
                }
                self.prompt_done(id);
            }
        }
    }

    pub fn prompt_wrong(&mut self, id: u64, message: String) {
        if let Some(request) = self.isle.prompts.requests.get_mut(&id) {
            request.prompt.status = Status::Warning(message);
            request.prompt.wrongs += 1;
            self.refresh_prompts();
        }
    }

    pub fn prompt_status(&mut self, id: u64, status: Status) {
        if let Some(request) = self.isle.prompts.requests.get_mut(&id) {
            request.prompt.status = status;
            self.refresh_prompts();
        }
    }

    pub fn prompt_done(&mut self, id: u64) {
        self.isle.prompts.queue.remove(id);
        self.isle.prompts.requests.remove(&id);
        self.refresh_prompts();
    }

    /// Cmd+Tab over a prompt: it waits in the island, and the switch goes on.
    pub fn set_prompt_aside(&mut self) {
        if let Some(id) = self.isle.prompt_showing() {
            info!(id, "prompt set aside");
            self.isle.prompts.queue.set_aside(id);
            self.refresh_prompts();
        }
    }

    /// The island's Show and Cancel on a waiting prompt.
    pub fn prompt_activity(&mut self, action: &str) {
        let holds = self.isle.prompt_holds();
        let Some((id, _)) = self.isle.prompts.queue.waiting(&holds).first().copied() else {
            return;
        };
        match action {
            "show" => {
                self.isle.prompts.queue.bring_back(id);
                self.refresh_prompts();
            }
            "cancel" => self.prompt_action(PromptAction::Cancel(id)),
            _ => {}
        }
    }

    /// Puts the queue's answer on screen: the frame on its screen with the keyboard, armed
    /// after a moment; or no frame, and the island's activity for what waits.
    pub fn refresh_prompts(&mut self) {
        let holds = self.isle.prompt_holds();
        let showing = self.isle.prompts.queue.showing(&holds);
        let before = self.isle.prompt_showing();
        if showing != before {
            if before.is_some() {
                self.isle.prompts.surface.forget();
                self.isle.prompts.shown = None;
                self.isle.prompts.armed = false;
                self.close_surface("prompt");
            }
            if let Some(id) = showing {
                self.show_prompt(id);
            }
        }
        if let Some(id) = showing {
            self.put_prompt_frame(id);
        }
        self.prompt_island(&holds);
        self.isle.queue_redraw_all();
    }

    fn show_prompt(&mut self, id: u64) {
        let app = self
            .isle
            .prompts
            .requests
            .get(&id)
            .and_then(|r| r.app.clone());
        // The asker's screen, else the main one.
        let output = app
            .and_then(|app| {
                let record = self
                    .isle
                    .wm
                    .records
                    .iter()
                    .find(|r| r.app == app && r.mapped)?;
                self.isle.output_of(record.id)
            })
            .or_else(|| self.isle.main_output());
        let Some(output) = output else {
            return;
        };
        let theme = self.isle.shell.theme().clone();
        let screen = self.isle.space.output_geometry(&output).unwrap_or_default();
        let size = PromptSurface::size_for(&theme);
        let margin = PromptSurface::margin(&theme) as i32;
        let geometry = Rectangle::new(
            ((screen.size.w - size.w) / 2, screen.size.h / 5 - margin).into(),
            size,
        );
        self.isle.prompts.surface.set_size(size);
        self.isle.prompts.shown = Some((id, output.name(), Instant::now()));
        self.isle.prompts.armed = false;
        self.put_prompt_frame(id);
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        self.isle.shell.open_placed("prompt", &place, geometry);
        if let Some(focus) = self.isle.shell.keyboard_taker("prompt") {
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            let previous = keyboard.current_focus();
            self.isle
                .shell_keyboard_return
                .push((focus.clone(), previous));
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.focus_keyboard(Some(Focus::Shell(focus)), serial);
        }
        self.dispatch_shell();
        self.arm_prompt();
    }

    /// Return, Space and clicks answer only after a moment, so a key meant for a window
    /// cannot (`surfaces/prompts.md`, The armed moment).
    fn arm_prompt(&mut self) {
        if let Some(token) = self.isle.prompts.arm_timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let wait =
            Duration::from_millis(self.isle.shell.theme().tokens.timing_prompt_arm_ms as u64);
        self.isle.prompts.arm_timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(wait), |_, _, state| {
                state.isle.prompts.arm_timer = None;
                if state.isle.prompt_showing().is_some() {
                    state.isle.prompts.armed = true;
                    if let Some(id) = state.isle.prompt_showing() {
                        state.put_prompt_frame(id);
                    }
                }
                TimeoutAction::Drop
            })
            .ok();
    }

    fn put_prompt_frame(&mut self, id: u64) {
        let Some(request) = self.isle.prompts.requests.get(&id) else {
            return;
        };
        let holds = self.isle.prompt_holds();
        let game = self.isle.game_mode_on();
        let windowed_game = self.focus_is_game();
        let over_panel = self.isle.island.panel.is_some();
        let theme = self.isle.shell.theme().clone();
        let frame = Frame {
            prompt: Some(request.prompt.clone()),
            armed: self.isle.prompts.armed,
            more: self.isle.prompts.queue.more(&holds),
            solid: game || windowed_game || over_panel || theme.appearance.reduce_transparency,
            still: game || theme.appearance.reduce_motion,
        };
        let surface = self.isle.prompts.surface.clone();
        surface.set_model(frame.clone());
        self.isle.shell.with_root("prompt", || surface.set(frame));
        self.dispatch_shell();
    }

    /// A prompt waiting is the island's activity, with Show and Cancel.
    fn prompt_island(&mut self, holds: &Holds) {
        use isle_shell::island::model::{Activity, ActivityKind};
        let waiting = self.isle.prompts.queue.waiting(holds);
        let first = waiting
            .first()
            .and_then(|(id, _)| self.isle.prompts.requests.get(id));
        match (first, self.isle.prompt_showing()) {
            (Some(request), None) => {
                let title = request.prompt.title.clone();
                self.isle.island.model.set_activity(
                    Activity {
                        kind: ActivityKind::Prompt,
                        face: request.app.clone().unwrap_or_default(),
                        title,
                        detail: request.prompt.message.clone(),
                        actions: vec![
                            ("show".into(), "Show".into()),
                            ("cancel".into(), "Cancel".into()),
                        ],
                        app: request.app.clone(),
                        art: None,
                        started: Instant::now(),
                    },
                    Instant::now(),
                );
            }
            _ => self.isle.island.model.end_activity(ActivityKind::Prompt),
        }
        self.isle.refresh_island();
    }
}

/// A prompt from the Test interface's JSON.
#[cfg(feature = "test-hooks")]
pub fn test_ask(json: &str) -> Result<Ask, String> {
    use isle_shell::prompts::{ChoiceSpec, FieldSpec, Kind as PromptKind};
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let text = |k: &str| v[k].as_str().unwrap_or_default().to_owned();
    let kind = match v["kind"].as_str().unwrap_or("polkit") {
        "keyring" => PromptKind::Keyring,
        "network" => PromptKind::Network,
        "vpn" => PromptKind::Vpn,
        "bluetooth" => PromptKind::Bluetooth,
        _ => PromptKind::Polkit,
    };
    let fields = v["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| FieldSpec {
            id: f["id"].as_str().unwrap_or_default().into(),
            label: f["label"].as_str().unwrap_or_default().into(),
            secret: f["secret"].as_bool().unwrap_or(false),
            show: f["show"].as_bool().unwrap_or(false),
            mono: false,
        })
        .collect();
    let choices = v["choices"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| ChoiceSpec {
            id: c["id"].as_str().unwrap_or_default().into(),
            label: c["label"].as_str().unwrap_or_default().into(),
            checked: c["checked"].as_bool().unwrap_or(false),
        })
        .collect();
    Ok(Ask {
        prompt: Prompt {
            kind,
            title: text("title"),
            message: text("message"),
            fields,
            choices,
            details: v["details"].as_str().map(str::to_owned),
            code: v["code"].as_str().map(str::to_owned),
            typed: v["typed"].as_u64().map(|n| n as usize),
            cancel: v["cancel"].as_str().unwrap_or("Cancel").into(),
            action: v["action"].as_str().map(str::to_owned),
            confirm: v["confirm"].as_bool().unwrap_or(false),
            ..Default::default()
        },
        title_unknown: None,
        subject: None,
        pid: v["pid"].as_i64().map(|p| p as i32),
        user_started: v["user_started"].as_bool().unwrap_or(false),
        checks: v["checks"].as_bool().unwrap_or(false),
    })
}
