//! What every Isle app stands on: its window, one instance, the desktop's theme and settings
//! followed live, and the Wayland extras winit does not bind (`apps.md` Conventions).

mod clipboard;
pub mod commands;
mod desktop;
mod instance;
mod toolbar;
mod wayland;
mod worker;

use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc};

use freya::prelude::*;
use futures_channel::mpsc::unbounded;
use futures_util::StreamExt;
use isle_ipc::{SettingsStatus, ThemeInfo};
use isle_settings::Value;
use isle_tokens::{Appearance, Mode, Source, Theme};
use isle_ui::{GlassShapes, ThemeContext};
use tracing::{info, warn};

pub use commands::{Chord, Command, Menu};
pub use desktop::{Write, from_owned};
pub use instance::Request;
pub use toolbar::ToolbarExt;
pub use worker::Worker;

/// An app's identity and window.
#[derive(Clone, Debug)]
pub struct Spec {
    /// The desktop file id, Wayland app id and bus name.
    pub id: &'static str,
    pub title: &'static str,
    pub size: (f64, f64),
    pub min_size: (f64, f64),
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
struct Blurs(bool);

/// Whether the backdrop is blurred, so it is drawn in the glass material rather than solid.
pub fn blurs() -> bool {
    use_try_consume::<Blurs>().is_some_and(|b| b.0)
}

/// The session bus the app holds its name on, when it could.
#[derive(Clone)]
struct SessionBus(Option<zbus::Connection>);

/// The session bus, for the app's own calls to the desktop.
pub fn session_bus() -> Option<zbus::Connection> {
    use_try_consume::<SessionBus>().and_then(|b| b.0)
}

/// The window's Wayland extras, for the toolbar's clicks.
#[derive(Clone)]
pub(crate) struct Chrome(pub Rc<RefCell<Option<wayland::Extras>>>);

/// The app's commands beside the standard ones; the menus follow each change.
#[derive(Clone, Copy)]
struct Table {
    app: State<Vec<Command>>,
}

/// Declares the app's own commands; call it in the app's root on every render.
pub fn use_commands(commands: Vec<Command>) {
    let mut table = use_consume::<Table>();
    if *table.app.peek() != commands {
        table.app.set(commands);
    }
}

/// The contexts `run` gives an app's tree, for testing its components headless: the worker,
/// the desktop's state as given, the shipped theme and the command table.
#[cfg(feature = "testing")]
pub fn provide_for_tests(desktop_values: BTreeMap<String, Value>) -> Desktop {
    use_provide_context(Worker::start);
    use_provide_context(|| SessionBus(None));
    use_provide_context(|| Chrome(Rc::default()));
    use_provide_context(|| TargetPlatform::MacOs);
    use_provide_context(GlassShapes::default);
    use_provide_context(|| {
        ThemeContext(State::create(Rc::new(
            Source::shipped().theme(&Appearance::default()).expect("the shipped tokens resolve"),
        )))
    });
    use_provide_context(|| Table {
        app: State::create(Vec::new()),
    });
    use_provide_context(|| Desktop {
        values: State::create(Rc::new(desktop_values)),
        status: State::create(None),
        tokens_status: State::create(None),
        requests: State::create(Vec::new()),
        commands: State::create(Vec::new()),
        writes: State::create(Vec::new()),
        refused: State::create(None),
        flight: State::create(Flight::default()),
    })
}

/// The app's commands as last declared, for tests.
#[cfg(feature = "testing")]
pub fn declared_commands() -> Vec<Command> {
    use_consume::<Table>().app.read().clone()
}

/// The path the app's menu bar is served at.
const MENU_PATH: &str = "/MenuBar/1";

/// The worker, for any component to send work to.
pub fn worker() -> Worker {
    use_consume::<Worker>()
}

enum Inbound {
    Update(desktop::Update),
    Request(Request),
    /// A menu item chosen, by its id.
    Menu(i32),
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

/// Starts the app, or hands its request to the instance already running and returns.
pub fn run<A: App + 'static>(spec: Spec, app: A) {
    let token = std::env::var("XDG_ACTIVATION_TOKEN").ok();
    // The token is this launch's alone; children never inherit it.
    unsafe { std::env::remove_var("XDG_ACTIVATION_TOKEN") };
    let first = Request::from_args(std::env::args().skip(1).collect(), token.clone());

    let worker = Worker::start();
    let (requests_tx, requests_rx) = unbounded();
    let (updates_tx, updates_rx) = unbounded();
    let bus = match worker.block_on(instance::claim(spec.id, first.clone(), requests_tx.clone())) {
        Ok(instance::Claim::Handed) => return,
        Ok(instance::Claim::Ours(bus)) => Some(bus),
        Err(err) => {
            warn!("not one instance, the bus refused: {err}");
            let _ = requests_tx.unbounded_send(first);
            None
        }
    };
    let (menu_tx, menu_rx) = unbounded();
    let menubar = bus.clone().map(|bus| {
        let menubar = isle_menus::export::MenuBar::new(
            Vec::new(),
            std::sync::Arc::new(move |id| {
                let _ = menu_tx.unbounded_send(id);
            }),
        );
        if let Err(err) = worker.block_on(bus.object_server().at(MENU_PATH, menubar.clone())) {
            warn!("the menus could not be served: {err}");
        }
        (bus, menubar)
    });
    if let Some(bus) = bus {
        worker.spawn(desktop::watch(bus, updates_tx));
    }
    let inbound = futures_util::stream::select(
        futures_util::stream::select(requests_rx.map(Inbound::Request), menu_rx.map(Inbound::Menu)),
        updates_rx.map(Inbound::Update),
    );

    let extras: Rc<RefCell<Option<wayland::Extras>>> = Rc::default();
    let gpu = cfg!(feature = "gpu") && std::env::var_os("FREYA_RENDERER").is_some();
    let renderer = if gpu { RendererPreference::Auto } else { RendererPreference::Software };
    let root = Root {
        spec: spec.clone(),
        menubar,
        app: Rc::new(app),
        worker: worker.clone(),
        extras: extras.clone(),
        inbound: Rc::new(RefCell::new(Some(Box::pin(inbound)))),
    };
    let mut window = WindowConfig::new_app(root);
    if spec.backdrop.is_some() {
        window = window.with_background(Color::TRANSPARENT);
    }
    let window = window
        .with_app_id(spec.id)
        .with_title(spec.title)
        .with_size(spec.size.0, spec.size.1)
        .with_min_size(spec.min_size.0, spec.min_size.1)
        .with_renderer(renderer)
        .with_transparency(spec.backdrop.is_some())
        .with_window_attributes(move |attributes, _| match token {
            Some(token) => {
                use freya::winit::{
                    platform::startup_notify::WindowAttributesExtStartupNotify,
                    window::ActivationToken,
                };
                attributes.with_activation_token(ActivationToken::from_raw(token))
            }
            None => attributes,
        })
        .with_window_handle(move |window| {
            *extras.borrow_mut() =
                wayland::Extras::connect(window, spec.traffic_light_inset, spec.backdrop);
        });
    info!(id = spec.id, "starting");
    launch(LaunchConfig::new().with_window(window));
}

type Inbox = std::pin::Pin<Box<dyn futures_util::Stream<Item = Inbound>>>;

struct Root<A> {
    spec: Spec,
    menubar: Option<(zbus::Connection, isle_menus::export::MenuBar)>,
    app: Rc<A>,
    worker: Worker,
    extras: Rc<RefCell<Option<wayland::Extras>>>,
    inbound: Rc<RefCell<Option<Inbox>>>,
}

/// The theme for what the compositor said, or the shipped one when its file will not read.
fn resolve(info: &ThemeInfo) -> Rc<Theme> {
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

impl<A: App + 'static> App for Root<A> {
    fn render(&self) -> impl IntoElement {
        // Freya's editing and scrolling follow the Mac's conventions, Cmd its primary.
        use_provide_context(|| TargetPlatform::MacOs);
        use_provide_context(|| self.worker.clone());
        use_provide_context(|| SessionBus(self.menubar.as_ref().map(|(bus, _)| bus.clone())));
        use_provide_context(|| Chrome(self.extras.clone()));
        use_provide_context(|| {
            let asked = self.spec.backdrop.is_some();
            Blurs(asked && self.extras.borrow().as_ref().is_some_and(|e| e.blurs()))
        });
        use_provide_context(|| GlassShapes {
            draw_tint: true,
            ..Default::default()
        });
        let theme = use_provide_context(|| {
            ThemeContext(State::create(Rc::new(
                Source::shipped()
                    .theme(&Appearance::default())
                    .expect("the shipped tokens resolve"),
            )))
        });
        let desktop = use_provide_context(|| Desktop {
            values: State::create(Rc::default()),
            status: State::create(None),
            tokens_status: State::create(None),
            requests: State::create(Vec::new()),
            commands: State::create(Vec::new()),
            writes: State::create(Vec::new()),
            refused: State::create(None),
            flight: State::create(Flight::default()),
        });
        let table = use_provide_context(|| Table {
            app: State::create(Vec::new()),
        });
        let all = use_memo({
            let spec = self.spec.clone();
            move || {
                let mut all = commands::standard(spec.title, spec.preferences);
                all.extend(table.app.read().iter().cloned());
                all
            }
        });
        use_side_effect({
            let menubar = self.menubar.clone();
            let worker = self.worker.clone();
            let title = self.spec.title;
            move || {
                let menus = commands::menus(title, &all.read());
                if let Some((bus, menubar)) = menubar.clone() {
                    worker.spawn(async move {
                        if let Err(err) = menubar.update(&bus, MENU_PATH, menus).await {
                            warn!("the menus could not be updated: {err}");
                        }
                    });
                }
            }
        });
        use_side_effect({
            let worker = self.worker.clone();
            let bus = self.menubar.as_ref().map(|(bus, _)| bus.clone());
            move || {
                if desktop.writes.read().is_empty() {
                    return;
                }
                let mut queue = desktop.writes;
                let writes = std::mem::take(&mut *queue.write());
                let Some(bus) = bus.clone() else {
                    warn!("not on the bus, {} changes are lost", writes.len());
                    return;
                };
                let mut flight = desktop.flight;
                for write in writes {
                    let key = write.key().to_owned();
                    if flight.peek().sending.contains(&key) {
                        flight.write().waiting.insert(key, write);
                        continue;
                    }
                    flight.write().sending.insert(key.clone());
                    let job = worker.spawn(desktop::send(bus.clone(), write.clone()));
                    let (mut refused, mut queue) = (desktop.refused, desktop.writes);
                    spawn(async move {
                        if let Ok(Err(why)) = job.await {
                            warn!(?write, "refused: {why}");
                            refused.set(Some((key.clone(), why)));
                        }
                        let next = {
                            let mut f = flight.write();
                            f.sending.remove(&key);
                            f.waiting.remove(&key)
                        };
                        if let Some(next) = next {
                            queue.write().push(next);
                        }
                    });
                }
            }
        });
        let run = Run {
            platform: Platform::get(),
            window: Platform::window_id(),
            worker: self.worker.clone(),
            bus: self.menubar.as_ref().map(|(bus, _)| bus.clone()),
            app_id: self.spec.id,
            desktop,
        };
        use_hook(|| {
            if let (Some(extras), Some((bus, _))) = (self.extras.borrow().as_ref(), &self.menubar)
                && let Some(name) = bus.unique_name()
            {
                extras.set_menu(name.as_str(), MENU_PATH);
            }
            if let Some(extras) = self.extras.borrow().as_ref() {
                // The display is winit's, alive as long as the window and its app.
                let provider = unsafe { clipboard::WaylandClipboard::new(extras.display) };
                GlobalContexts::get().insert_context(freya::clipboard::Clipboard::create(Some(
                    Box::new(provider),
                )));
            }
        });
        use_hook(|| {
            let mut inbound = self.inbound.borrow_mut().take();
            let extras = self.extras.clone();
            let run = run.clone();
            spawn(async move {
                let Some(inbound) = inbound.as_mut() else {
                    return;
                };
                let (mut theme, mut desktop) = (theme, desktop);
                while let Some(message) = inbound.next().await {
                    match message {
                        Inbound::Update(desktop::Update::Theme(info)) => {
                            theme.0.set(resolve(&info));
                        }
                        Inbound::Update(desktop::Update::TokensStatus(status)) => {
                            desktop.tokens_status.set(Some(status));
                        }
                        Inbound::Update(desktop::Update::Values(values)) => {
                            desktop.take_values(values);
                        }
                        Inbound::Update(desktop::Update::Status(status)) => {
                            desktop.status.set(Some(status));
                        }
                        Inbound::Request(request) => {
                            info!(?request, "asked");
                            if let (Some(token), Some(extras)) =
                                (request.token(), extras.borrow().as_ref())
                            {
                                extras.activate(token);
                            }
                            desktop.requests.write().push(request);
                        }
                        Inbound::Menu(id) => {
                            let command = usize::try_from(id - 1)
                                .ok()
                                .and_then(|i| all.peek().get(i).cloned());
                            if let Some(command) = command.filter(|c| c.enabled) {
                                run.chosen(&command);
                            }
                        }
                    }
                }
            })
        });
        rect()
            .expanded()
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                let command = all.peek().iter().find(|c| {
                    c.enabled && !c.for_field && c.chord.is_some_and(|k| k.matches(&e.key, e.modifiers))
                }).cloned();
                if let Some(command) = command {
                    run.run(&command.id);
                }
            })
            .child(self.app.render())
    }
}

/// Carries out commands, from a menu or a key.
#[derive(Clone)]
struct Run {
    platform: Platform,
    window: freya::winit::window::WindowId,
    worker: Worker,
    bus: Option<zbus::Connection>,
    app_id: &'static str,
    desktop: Desktop,
}

impl Run {
    /// A menu item: one with a chord arrives as its key press, as the key itself would.
    fn chosen(&self, command: &Command) {
        match command.chord {
            Some(chord) => {
                let key = Key::Character(chord.key.to_string());
                let modifiers = chord.modifiers();
                let _ = self.platform.post_callback(move |window, ctx: &mut RendererContext| {
                    ctx.send_key_press(Some(window), key, Code::Unidentified, modifiers);
                });
            }
            None => self.run(&command.id),
        }
    }

    fn run(&self, id: &str) {
        info!(id, "command");
        let window = self.window;
        match id {
            "quit" => {
                let _ = self.platform.post_callback(|_, ctx: &mut RendererContext| ctx.exit());
            }
            "close" => self.platform.close_window(window),
            "minimise" => self.platform.with_window(window, |w| w.set_minimized(true)),
            "zoom" => self.platform.with_window(window, |w| w.set_maximized(!w.is_maximized())),
            "hide" | "hide-others" | "show-all" => {
                let Some(bus) = self.bus.clone() else { return };
                let (me, id) = (self.app_id, id.to_owned());
                self.worker.spawn(async move {
                    let Ok(windows) = isle_ipc::windows::WindowsProxy::new(&bus).await else {
                        return;
                    };
                    let apps = windows.apps().await.unwrap_or_default();
                    for app in apps {
                        let mine = app.app == me;
                        let hide = match id.as_str() {
                            "hide" => mine.then_some(true),
                            "hide-others" => (!mine).then_some(true),
                            _ => Some(false),
                        };
                        if let Some(hide) = hide {
                            let _ = windows.set_app_hidden(&app.app, hide).await;
                        }
                    }
                });
            }
            other => {
                let mut commands = self.desktop.commands;
                commands.write().push(other.to_owned());
            }
        }
    }
}
