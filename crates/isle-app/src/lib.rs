//! What every Isle app stands on: its window, one instance, the desktop's theme and settings
//! followed live, and the Wayland extras winit does not bind (`apps.md` Conventions).

pub mod commands;
pub mod data;
mod desktop;
mod hub;
mod instance;
pub mod portal;
mod toolbar;
mod wayland;
mod window;
mod worker;

use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc};

use freya::prelude::*;
use futures_channel::mpsc::unbounded;
use futures_util::StreamExt;
use isle_ipc::{SettingsStatus, ThemeInfo};
use isle_settings::Value;
use isle_tokens::{Appearance, Mode, Source, Theme};
use isle_ui::ThemeContext;
use tracing::{info, warn};

pub use commands::{Chord, ChordKey, Command, Menu};
pub use desktop::{Write, from_owned};
pub use instance::{Request, SERVICE};
pub use toolbar::ToolbarExt;
pub use hub::{AppHandle, Hold, Requester, WindowKey, app};
pub use window::{Drag, WindowSpec, this_surface, this_window, use_drag};
pub use worker::Worker;

/// An app's identity and window.
#[derive(Clone, Debug)]
pub struct Spec {
    /// The desktop file id, Wayland app id and bus name.
    pub id: &'static str,
    pub title: &'static str,
    pub size: (f64, f64),
    pub min_size: (f64, f64),
    /// The largest it grows to; a filled or tiled window takes this, centred.
    pub max_size: Option<(f64, f64)>,
    /// The lights' group from the window's top leading corner, in points.
    pub traffic_light_inset: (f64, f64),
    /// It has a settings window, opened by Settings… on Cmd+,.
    pub preferences: bool,
    /// The window's box the compositor blurs behind, from its top leading corner, in points:
    /// a sidebar drawn in the glass material.
    pub backdrop: Option<(f64, f64, f64, f64)>,
}

/// The desktop's state as the app last heard it, read by any component.
#[derive(Clone, Copy)]
pub struct Desktop {
    pub values: State<Rc<BTreeMap<String, Value>>>,
    pub status: State<Option<SettingsStatus>>,
    pub tokens_status: State<Option<SettingsStatus>>,
    /// The user's `tokens.kdl` as the compositor has it in force; empty for none.
    pub tokens_text: State<String>,
    /// Requests from launches and activations, oldest first, for the app to take.
    pub requests: State<Vec<Request>>,
    /// The app's own commands chosen from its menus or keys, oldest first, for it to take.
    pub commands: State<Vec<String>>,
    /// Changes to `settings.kdl` not yet sent, oldest first.
    pub writes: State<Vec<Write>>,
    /// The last change the compositor refused: its key and why.
    pub refused: State<Option<(String, String)>>,
    /// The keys with a change on its way, and the latest change of each that waits for it.
    pub flight: State<Flight>,
}

/// Changes sent and not yet answered, one a key at a time.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flight {
    pub sending: std::collections::BTreeSet<String>,
    /// The latest change to a key whose earlier change is still being sent.
    pub waiting: BTreeMap<String, Write>,
}

impl Flight {
    pub fn pending(&self, key: &str) -> bool {
        self.sending.contains(key) || self.waiting.contains_key(key)
    }
}

impl Desktop {
    pub fn get() -> Self {
        use_consume::<Desktop>()
    }

    /// A setting's value in force, if the compositor has said.
    pub fn value(&self, key: &str) -> Option<Value> {
        self.values.read().get(key).cloned()
    }

    /// Sets a key; it reads as set at once, and the compositor's answer follows.
    pub fn set(&self, key: impl Into<String>, value: Value) {
        let key = key.into();
        let mut values = self.values;
        let mut next = (**values.peek()).clone();
        next.insert(key.clone(), value.clone());
        values.set(Rc::new(next));
        self.send(Write::Set(key, value));
    }

    /// Puts a key back to its default.
    pub fn reset(&self, key: impl Into<String>) {
        self.send(Write::Reset(key.into()));
    }

    /// Deletes a keyed entry and everything in it.
    pub fn remove(&self, entry: impl Into<String>) {
        let entry = entry.into();
        let mut values = self.values;
        let prefix = format!("{entry}.");
        let next = values
            .peek()
            .iter()
            .filter(|(k, _)| !k.starts_with(&prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        values.set(Rc::new(next));
        self.send(Write::Remove(entry));
    }

    fn send(&self, write: Write) {
        let (mut writes, mut refused) = (self.writes, self.refused);
        if refused.peek().as_ref().is_some_and(|(k, _)| k == write.key()) {
            refused.set(None);
        }
        // A key changed again before the last change left supersedes it, as a slider dragged.
        let mut queue = writes.write();
        queue.retain(|w| w.key() != write.key());
        queue.push(write);
    }

    /// The compositor's values, with the app's own for the keys whose change is still on its
    /// way: an answer to an earlier change must not undo a later one.
    pub fn take_values(&self, mut values: BTreeMap<String, Value>) {
        let flight = self.flight.peek();
        let queued = self.writes.peek();
        let mine = self.values.peek();
        for key in mine.keys().chain(values.clone().keys()) {
            if flight.pending(key) || queued.iter().any(|w| w.key() == key) {
                match mine.get(key) {
                    Some(v) => values.insert(key.clone(), v.clone()),
                    None => values.remove(key),
                };
            }
        }
        drop((flight, queued, mine));
        let mut state = self.values;
        state.set(Rc::new(values));
    }
}

/// The compositor blurs behind the window's backdrop.
#[derive(Clone, Copy)]
pub(crate) struct Blurs(pub bool);

/// Whether the backdrop is blurred, so it is drawn in the glass material rather than solid.
pub fn blurs() -> bool {
    use_try_consume::<Blurs>().is_some_and(|b| b.0)
}

/// The session bus the app holds its name on, when it could.
#[derive(Clone)]
pub(crate) struct SessionBus(pub Option<zbus::Connection>);

/// The session bus, for the app's own calls to the desktop.
pub fn session_bus() -> Option<zbus::Connection> {
    use_try_consume::<SessionBus>().and_then(|b| b.0)
}

/// The window's Wayland extras, for the toolbar's clicks.
#[derive(Clone)]
pub(crate) struct Chrome(pub Rc<RefCell<Option<wayland::Extras>>>);

/// The app's commands beside the standard ones; the menus follow each change.
#[derive(Clone, Copy)]
pub(crate) struct Table {
    pub app: State<Vec<Command>>,
}

/// Declares the app's own commands; call it in the app's root on every render.
pub fn use_commands(commands: Vec<Command>) {
    let mut table = use_consume::<Table>();
    if *table.app.peek() != commands {
        table.app.set(commands);
    }
}

fn shipped_theme() -> Rc<Theme> {
    Rc::new(
        Source::shipped()
            .theme(&Appearance::default())
            .expect("the shipped tokens resolve"),
    )
}

/// The contexts `run` gives a window's tree, for testing its components headless: the worker,
/// the desktop's state as given, the shipped theme, the command table, and an app with no
/// event loop whose one window is this tree.
#[cfg(feature = "testing")]
pub fn provide_for_tests(desktop_values: BTreeMap<String, Value>) -> Desktop {
    let worker = use_provide_context(Worker::start);
    use_provide_context(|| SessionBus(None));
    use_provide_context(|| Chrome(Rc::default()));
    use_provide_context(|| TargetPlatform::MacOs);
    use_provide_context(isle_ui::GlassShapes::default);
    let theme = use_provide_context(|| ThemeContext(State::create(shipped_theme())));
    use_provide_context(|| Table {
        app: State::create(Vec::new()),
    });
    let desktop = use_provide_context(|| Desktop {
        values: State::create(Rc::new(desktop_values)),
        status: State::create(None),
        tokens_status: State::create(None),
        tokens_text: State::create(String::new()),
        requests: State::create(Vec::new()),
        commands: State::create(Vec::new()),
        writes: State::create(Vec::new()),
        refused: State::create(None),
        flight: State::create(Flight::default()),
    });
    use_provide_context(|| {
        let spec = Spec {
            id: "io.github.alexparlett.Isle.Test",
            title: "Test",
            size: (800.0, 600.0),
            min_size: (400.0, 300.0),
            max_size: None,
            traffic_light_inset: (0.0, 0.0),
            preferences: false,
            backdrop: None,
        };
        AppHandle::new(spec, worker.clone(), None, desktop, theme).0
    });
    use_provide_context(|| WindowKey(1));
    use_provide_context(|| window::Drag {
        over: State::create(None),
        dropped: State::create(Vec::new()),
        ended: State::create(0),
    });
    desktop
}

/// The app's commands as last declared, for tests.
#[cfg(feature = "testing")]
pub fn declared_commands() -> Vec<Command> {
    use_consume::<Table>().app.read().clone()
}

/// The worker, for any component to send work to.
pub fn worker() -> Worker {
    use_consume::<Worker>()
}

/// Logs to standard error, plain unless it is a terminal: warnings, and `crate`'s and
/// `isle-app`'s own lines from info, unless `RUST_LOG` says otherwise.
pub fn init_logging(krate: &str) {
    use std::io::IsTerminal;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("warn,isle_app=info,{krate}=info").into());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .init();
}

/// What an app does above its windows.
pub trait Delegate: 'static {
    /// Once, before the first request: serve the app's own objects on its bus.
    fn started(&self, app: &AppHandle) {
        let _ = app;
    }

    /// A launch, an activation or an action from another process: open or raise windows.
    fn request(&self, app: &AppHandle, request: Request);

    /// Cmd+Q. True quits now; an app with work in flight asks first and ends itself later
    /// (`apps.md` Conventions 5).
    fn quit(&self, app: &AppHandle) -> bool {
        let _ = app;
        true
    }
}

/// Starts an app with one window, its main, drawing `app`; or hands its request to the
/// instance already running and returns.
pub fn run<A: App + Clone + 'static>(spec: Spec, app: A) {
    run_with(spec.clone(), Single { spec, app })
}

/// Starts an app whose windows its delegate opens; or hands its request to the instance
/// already running and returns.
pub fn run_with(spec: Spec, delegate: impl Delegate) {
    let token = std::env::var("XDG_ACTIVATION_TOKEN").ok();
    // The token is this launch's alone; children never inherit it.
    unsafe { std::env::remove_var("XDG_ACTIVATION_TOKEN") };
    let first = Request::from_args(std::env::args().skip(1).collect(), token);

    let worker = Worker::start();
    let (requests_tx, requests_rx) = unbounded();
    let bus = match worker.block_on(instance::claim(spec.id, first.clone(), requests_tx.clone())) {
        Ok(instance::Claim::Handed) => return,
        Ok(instance::Claim::Ours(bus)) => Some(bus),
        Err(err) => {
            warn!("not one instance, the bus refused: {err}");
            let _ = requests_tx.unbounded_send(first);
            None
        }
    };
    let desktop = Desktop {
        values: State::create_global(Rc::default()),
        status: State::create_global(None),
        tokens_status: State::create_global(None),
        tokens_text: State::create_global(String::new()),
        requests: State::create_global(Vec::new()),
        commands: State::create_global(Vec::new()),
        writes: State::create_global(Vec::new()),
        refused: State::create_global(None),
        flight: State::create_global(Flight::default()),
    };
    let theme = ThemeContext(State::create_global(shipped_theme()));
    let (app, inbox) = AppHandle::new(spec.clone(), worker.clone(), bus.clone(), desktop, theme);
    let (updates_tx, updates_rx) = unbounded();
    if let Some(bus) = bus {
        worker.spawn(desktop::watch(bus, updates_tx));
    }
    let forward = app.0.inbox.clone();
    worker.spawn(async move {
        let mut from = futures_util::stream::select(
            requests_rx.map(hub::Inbound::Request),
            updates_rx.map(hub::Inbound::Update),
        );
        while let Some(message) = from.next().await {
            if forward.unbounded_send(message).is_err() {
                return;
            }
        }
    });
    let delegate: Rc<dyn Delegate> = Rc::new(delegate);
    info!(id = spec.id, "starting");
    launch(
        LaunchConfig::new()
            .with_exit_on_close(false)
            .with_future(move |proxy| app.serve(proxy, delegate, inbox)),
    );
}

/// An app of one window: a request raises it, or opens it again once closed, and is left in
/// `Desktop::requests` for the window to take.
struct Single<A> {
    spec: Spec,
    app: A,
}

impl<A: App + Clone + 'static> Delegate for Single<A> {
    fn request(&self, app: &AppHandle, request: Request) {
        let token = request.token().map(str::to_owned);
        let mut requests = app.desktop().requests;
        requests.write().push(request);
        match app.windows().first() {
            Some(&window) => {
                if let Some(token) = token {
                    app.activate(window, &token);
                }
            }
            None => {
                app.open(WindowSpec::main(&self.spec).token(token), self.app.clone());
            }
        }
    }
}

impl WindowSpec {
    /// The window an app of one window opens.
    pub fn main(spec: &Spec) -> Self {
        Self {
            title: spec.title.to_owned(),
            size: spec.size,
            min_size: spec.min_size,
            max_size: spec.max_size,
            lights: Some(spec.traffic_light_inset),
            backdrop: spec.backdrop,
            token: None,
            parent: None,
        }
    }
}

/// The theme for what the compositor said, or the shipped one when its file will not read.
pub(crate) fn resolve(info: &ThemeInfo) -> Rc<Theme> {
    thread_local! {
        /// The tokens last read, by their text: a glass or accent change keeps the text.
        static READ: RefCell<Option<(String, Source)>> = const { RefCell::new(None) };
    }
    let source = READ.with_borrow_mut(|read| {
        if let Some((text, source)) = read.as_ref()
            && *text == info.tokens
        {
            return source.clone();
        }
        let source = if info.tokens.is_empty() {
            Source::shipped()
        } else {
            match Source::with_user(&info.tokens, Path::new("tokens.kdl")) {
                Ok((source, _)) => source,
                Err(err) => {
                    warn!("the compositor's tokens do not read here: {err}");
                    Source::shipped()
                }
            }
        };
        *read = Some((info.tokens.clone(), source.clone()));
        source
    });
    let appearance = Appearance {
        mode: if info.mode == "light" { Mode::Light } else { Mode::Dark },
        accent: info.accent.clone(),
        glass: info.glass as f32,
        text_scale: info.text_scale as f32,
        increase_contrast: info.increase_contrast,
        reduce_motion: info.reduce_motion,
        reduce_transparency: info.reduce_transparency,
    };
    Rc::new(source.theme(&appearance).unwrap_or_else(|err| {
        warn!("the tokens do not resolve, the shipped ones stand: {err}");
        Source::shipped()
            .theme(&appearance)
            .expect("the shipped tokens resolve")
    }))
}
