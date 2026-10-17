//! What every Isle app stands on: its window, one instance, the desktop's theme and settings
//! followed live, and the Wayland extras winit does not bind (`apps.md` Conventions).

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

pub use desktop::from_owned;
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
}

/// The desktop's state as the app last heard it, read by any component.
#[derive(Clone, Copy)]
pub struct Desktop {
    pub values: State<Rc<BTreeMap<String, Value>>>,
    pub status: State<Option<SettingsStatus>>,
    pub tokens_status: State<Option<SettingsStatus>>,
    /// Requests from launches and activations, oldest first, for the app to take.
    pub requests: State<Vec<Request>>,
}

impl Desktop {
    pub fn get() -> Self {
        use_consume::<Desktop>()
    }

    /// A setting's value in force, if the compositor has said.
    pub fn value(&self, key: &str) -> Option<Value> {
        self.values.read().get(key).cloned()
    }
}

/// The window's Wayland extras, for the toolbar's clicks.
#[derive(Clone)]
pub(crate) struct Chrome(pub Rc<RefCell<Option<wayland::Extras>>>);

/// The worker, for any component to send work to.
pub fn worker() -> Worker {
    use_consume::<Worker>()
}

enum Inbound {
    Update(desktop::Update),
    Request(Request),
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
    if let Some(bus) = bus {
        worker.spawn(desktop::watch(bus, updates_tx));
    }
    let inbound = futures_util::stream::select(
        requests_rx.map(Inbound::Request),
        updates_rx.map(Inbound::Update),
    );

    let extras: Rc<RefCell<Option<wayland::Extras>>> = Rc::default();
    let renderer = if std::env::var_os("FREYA_RENDERER").is_some() {
        RendererPreference::Auto
    } else {
        RendererPreference::Software
    };
    let root = Root {
        app: Rc::new(app),
        worker: worker.clone(),
        extras: extras.clone(),
        inbound: Rc::new(RefCell::new(Some(Box::pin(inbound)))),
    };
    let window = WindowConfig::new_app(root)
        .with_app_id(spec.id)
        .with_title(spec.title)
        .with_size(spec.size.0, spec.size.1)
        .with_min_size(spec.min_size.0, spec.min_size.1)
        .with_renderer(renderer)
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
            *extras.borrow_mut() = wayland::Extras::connect(window, spec.traffic_light_inset);
        });
    info!(id = spec.id, "starting");
    launch(LaunchConfig::new().with_window(window));
}

type Inbox = std::pin::Pin<Box<dyn futures_util::Stream<Item = Inbound>>>;

struct Root<A> {
    app: Rc<A>,
    worker: Worker,
    extras: Rc<RefCell<Option<wayland::Extras>>>,
    inbound: Rc<RefCell<Option<Inbox>>>,
}

/// The theme for what the compositor said, or the shipped one when its file will not read.
fn resolve(info: &ThemeInfo) -> Rc<Theme> {
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
        use_provide_context(|| self.worker.clone());
        use_provide_context(|| Chrome(self.extras.clone()));
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
        });
        use_hook(|| {
            let mut inbound = self.inbound.borrow_mut().take();
            let extras = self.extras.clone();
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
                            desktop.values.set(Rc::new(values));
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
                    }
                }
            })
        });
        self.app.render()
    }
}
