//! The daemons and compositor state the panes follow, and the changes they ask of them.

use std::collections::BTreeSet;

use freya::prelude::*;
use futures_util::StreamExt;
use isle_bus::{Publisher, Snapshot};
use isle_ipc::{DisplayInfo, KeyboardInfo};
use tracing::warn;

/// A Control Centre module as `Island1.Modules` gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub id: String,
    pub name: String,
    pub glyph: String,
    pub sizes: Vec<String>,
}

/// An installed app as the panes list it.
#[derive(Clone, Debug, PartialEq)]
pub struct App {
    pub id: String,
    pub name: String,
    pub icon: Option<std::path::PathBuf>,
    /// Launchers list it.
    pub listed: bool,
    /// A library game, by the app id its windows carry.
    pub game: bool,
}

/// An app's remembered permission in a portal table.
#[derive(Clone, Debug, PartialEq)]
pub struct Permission {
    pub table: String,
    pub app: String,
}

/// What a pane follows; each is started the first time a pane asks for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Follow {
    Apps,
    Network,
    Bluetooth,
    Audio,
    Profiles,
    Power,
    Tray,
    Displays,
    Keyboards,
    Pointers,
    NotifyApps,
    Modules,
    Permissions,
    NightLight,
    Focus,
}

/// Something a pane asks a daemon or the compositor to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    SetWifi(bool),
    Scan(String),
    Join {
        device: String,
        ssid: String,
    },
    JoinHidden {
        device: String,
        ssid: String,
        secured: bool,
    },
    Activate(String),
    Deactivate(String),
    Forget(String),
    AutoJoin {
        saved: String,
        on: bool,
    },
    SetBluetooth {
        adapter: String,
        on: bool,
    },
    /// Looking for devices and letting them find this computer, while the pane is shown.
    Discover {
        adapter: String,
        on: bool,
    },
    BtConnect(String),
    BtDisconnect(String),
    BtPair(String),
    BtForget {
        adapter: String,
        device: String,
    },
    BtRename {
        device: String,
        name: String,
    },
    SetDefaultOutput(String),
    SetDefaultInput(String),
    /// A device's level, its balance kept.
    Volume {
        output: bool,
        device: isle_audio::Device,
        value: f32,
    },
    Mute {
        output: bool,
        device: String,
        on: bool,
    },
    Balance {
        device: isle_audio::Device,
        value: f32,
    },
    PlaySound(String),
    Profile(String),
    Configure(String),
    Keep(u64),
    Revert(u64),
    Brightness {
        display: String,
        level: f64,
    },
    Redetect,
    Identify,
    ToggleNightLight,
    EditControls,
    Revoke(Permission),
    Hostname(String),
}

/// The followed state, shared by every pane.
#[derive(Clone, Copy)]
pub struct Services {
    pub apps: State<Snapshot<Vec<App>>>,
    pub network: State<Snapshot<isle_network::State>>,
    pub bluetooth: State<Snapshot<isle_bluetooth::Bluetooth>>,
    pub audio: State<Snapshot<isle_audio::Audio>>,
    pub profiles: State<Snapshot<isle_power::Profiles>>,
    pub power: State<Snapshot<isle_power::Power>>,
    /// The tray's items now, by id and title.
    pub tray: State<Snapshot<Vec<(String, String)>>>,
    pub displays: State<Snapshot<Vec<DisplayInfo>>>,
    pub keyboards: State<Snapshot<Vec<KeyboardInfo>>>,
    pub pointers: State<Snapshot<Vec<String>>>,
    pub notify_apps: State<Snapshot<Vec<(String, String)>>>,
    pub modules: State<Snapshot<Vec<Module>>>,
    pub permissions: State<Snapshot<Vec<Permission>>>,
    /// The Focus in force, by its name.
    pub focus: State<Snapshot<Option<String>>>,
    /// Night light's state as `Displays1.NightLight` gives it.
    pub night_light: State<Snapshot<serde_json::Value>>,
    /// The display change in force and not yet kept, by its id.
    pub display_change: State<Option<u64>>,
    /// Changes asked for and not yet sent, oldest first.
    pub acts: State<Vec<Act>>,
    started: State<BTreeSet<Follow>>,
    /// Where following runs: the worker and the session bus; none in a test.
    runtime: State<Option<(isle_app::Worker, Option<zbus::Connection>)>>,
}

impl Services {
    /// Followed for real: each source starts when first asked for.
    pub fn live(worker: isle_app::Worker, bus: Option<zbus::Connection>) -> Self {
        Self::new(Some((worker, bus)))
    }

    /// Nothing followed: a test sets the state and reads the acts.
    #[cfg(test)]
    pub fn still() -> Self {
        Self::new(None)
    }

    fn new(runtime: Option<(isle_app::Worker, Option<zbus::Connection>)>) -> Self {
        Self {
            apps: State::create(Snapshot::default()),
            network: State::create(Snapshot::default()),
            bluetooth: State::create(Snapshot::default()),
            audio: State::create(Snapshot::default()),
            profiles: State::create(Snapshot::default()),
            power: State::create(Snapshot::default()),
            tray: State::create(Snapshot::default()),
            displays: State::create(Snapshot::default()),
            keyboards: State::create(Snapshot::default()),
            pointers: State::create(Snapshot::default()),
            notify_apps: State::create(Snapshot::default()),
            modules: State::create(Snapshot::default()),
            permissions: State::create(Snapshot::default()),
            focus: State::create(Snapshot::default()),
            night_light: State::create(Snapshot::default()),
            display_change: State::create(None),
            acts: State::create(Vec::new()),
            started: State::create(BTreeSet::new()),
            runtime: State::create(runtime),
        }
    }

    pub fn get() -> Self {
        use_consume::<Services>()
    }

    pub fn act(&self, act: Act) {
        let mut acts = self.acts;
        acts.write().push(act);
    }

    /// Starts following `what` if nothing has yet.
    pub fn want(&self, what: Follow) {
        let Some((worker, bus)) = self.runtime.peek().clone() else {
            return;
        };
        if self.started.peek().contains(&what) {
            return;
        }
        let mut started = self.started;
        started.write().insert(what);
        match what {
            Follow::Apps => {
                let (publisher, rx) = isle_bus::snapshot::<Vec<App>>();
                worker.spawn(async move { publisher.publish(scan_apps()) });
                forward(rx, self.apps);
            }
            Follow::Network => system(&worker, self.network, isle_network::watch),
            Follow::Bluetooth => system(&worker, self.bluetooth, isle_bluetooth::watch),
            Follow::Audio => system(&worker, self.audio, isle_audio::watch),
            Follow::Profiles => system(&worker, self.profiles, isle_power::watch_profiles),
            Follow::Power => system(&worker, self.power, isle_power::watch_batteries),
            Follow::Tray => session(&worker, bus, self.tray, tray),
            Follow::Displays => session(&worker, bus, self.displays, displays),
            Follow::Keyboards => session(&worker, bus, self.keyboards, keyboards),
            Follow::Pointers => session(&worker, bus, self.pointers, pointers),
            Follow::NotifyApps => session(&worker, bus, self.notify_apps, notify_apps),
            Follow::Modules => session(&worker, bus, self.modules, modules),
            Follow::Permissions => session(&worker, bus, self.permissions, permissions),
            Follow::NightLight => session(&worker, bus, self.night_light, night_light),
            Follow::Focus => session(&worker, bus, self.focus, focus),
        }
    }
}

/// Every app with a desktop entry, A to Z, with its icon.
fn scan_apps() -> Vec<App> {
    let index = isle_apps::Index::scan(&isle_apps::Index::data_dirs());
    let mut icons = isle_apps::icons::Icons::papirus(true);
    let mut apps: Vec<App> = index
        .entries()
        .iter()
        .filter(|e| !e.missing)
        .map(|e| App {
            id: e.id.clone(),
            name: e.name.clone(),
            icon: e.icon.as_deref().and_then(|i| icons.lookup(i, 32)),
            listed: e.listed(),
            game: false,
        })
        .collect();
    let library = isle_games::Library::read(&isle_games::Roots::find());
    for game in library.games() {
        if let isle_games::GameId::Steam(id) = &game.id {
            apps.push(App {
                id: format!("steam_app_{id}"),
                name: game.name.clone(),
                icon: None,
                listed: false,
                game: true,
            });
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

impl Services {
    /// An app's name and icon by its id, as far as the index knows it.
    pub fn app(&self, id: &str) -> Option<App> {
        self.apps.read().state.iter().find(|a| a.id == id).cloned()
    }
}

/// Follows a source while the component is shown.
pub fn use_follow(what: &[Follow]) -> Services {
    let services = Services::get();
    let what = what.to_vec();
    use_hook(move || {
        for w in what {
            services.want(w);
        }
    });
    services
}

/// Copies a snapshot channel into a state the panes read.
fn forward<T>(mut rx: tokio::sync::watch::Receiver<Snapshot<T>>, mut into: State<Snapshot<T>>)
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    spawn(async move {
        loop {
            let snapshot = rx.borrow_and_update().clone();
            if *into.peek() != snapshot {
                into.set(snapshot);
            }
            if rx.changed().await.is_err() {
                return;
            }
        }
    });
}

/// A system daemon's service crate watching on the worker.
fn system<T, F, Fut>(worker: &isle_app::Worker, into: State<Snapshot<T>>, watch: F)
where
    T: Clone + Default + PartialEq + Send + Sync + 'static,
    F: Fn(Publisher<T>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), isle_bus::Error>> + Send + 'static,
{
    let (publisher, rx) = isle_bus::snapshot::<T>();
    worker.spawn(async move {
        loop {
            if let Err(err) = watch(publisher.clone()).await {
                warn!("a daemon is not followed: {err}");
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
    forward(rx, into);
}

type Read<T> = fn(
    zbus::Connection,
    Publisher<T>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>>;

/// A compositor source on the session bus, read again on each change.
fn session<T>(
    worker: &isle_app::Worker,
    bus: Option<zbus::Connection>,
    into: State<Snapshot<T>>,
    read: Read<T>,
) where
    T: Clone + Default + PartialEq + Send + Sync + 'static,
{
    let Some(bus) = bus else {
        return;
    };
    let (publisher, rx) = isle_bus::snapshot::<T>();
    worker.spawn(async move {
        loop {
            if let Err(err) = read(bus.clone(), publisher.clone()).await {
                warn!("the compositor's state is not followed: {err}");
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    forward(rx, into);
}

/// Reads with `read`, then again each time `changed` fires.
async fn each_change<T, S, R, Fut>(
    publisher: &Publisher<T>,
    mut changed: S,
    read: R,
) -> zbus::Result<()>
where
    T: Clone + PartialEq,
    S: futures_util::Stream + Unpin,
    R: Fn() -> Fut,
    Fut: std::future::Future<Output = zbus::Result<T>>,
{
    loop {
        publisher.publish(read().await?);
        if changed.next().await.is_none() {
            return Ok(());
        }
    }
}

fn displays(
    bus: zbus::Connection,
    publisher: Publisher<Vec<DisplayInfo>>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>> {
    Box::pin(async move {
        let proxy = isle_ipc::displays::DisplaysProxy::new(&bus).await?;
        let changed = proxy.receive_changed().await?;
        each_change(&publisher, changed, || proxy.list()).await
    })
}

fn night_light(
    bus: zbus::Connection,
    publisher: Publisher<serde_json::Value>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>> {
    Box::pin(async move {
        let proxy = isle_ipc::displays::DisplaysProxy::new(&bus).await?;
        let changed = proxy.receive_changed().await?;
        each_change(&publisher, changed, || async {
            let json = proxy.night_light().await?;
            Ok(serde_json::from_str(&json).unwrap_or_default())
        })
        .await
    })
}

fn focus(
    bus: zbus::Connection,
    publisher: Publisher<Option<String>>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>> {
    Box::pin(async move {
        let proxy = isle_ipc::focus::FocusProxy::new(&bus).await?;
        let changed = proxy.receive_changed().await?;
        each_change(&publisher, changed, || async {
            let status: serde_json::Value =
                serde_json::from_str(&proxy.status().await?).unwrap_or_default();
            Ok(status["active"].as_str().map(str::to_owned))
        })
        .await
    })
}

fn keyboards(
    bus: zbus::Connection,
    publisher: Publisher<Vec<KeyboardInfo>>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>> {
    Box::pin(async move {
        let proxy = isle_ipc::keyboard::KeyboardProxy::new(&bus).await?;
        let changed = proxy.receive_changed().await?;
        each_change(&publisher, changed, || proxy.list()).await
    })
}

fn pointers(
    bus: zbus::Connection,
    publisher: Publisher<Vec<String>>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>> {
    Box::pin(async move {
        let proxy = isle_ipc::keyboard::KeyboardProxy::new(&bus).await?;
        let changed = proxy.receive_changed().await?;
        each_change(&publisher, changed, || proxy.pointers()).await
    })
}

fn tray(
    bus: zbus::Connection,
    publisher: Publisher<Vec<(String, String)>>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>> {
    Box::pin(async move {
        let proxy = isle_ipc::services::ServicesProxy::new(&bus).await?;
        loop {
            let status: serde_json::Value =
                serde_json::from_str(&proxy.status().await?).unwrap_or_default();
            let items = status["tray"]["state"]["items"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|i| {
                            (
                                i["id"].as_str().unwrap_or_default().to_owned(),
                                i["title"].as_str().unwrap_or_default().to_owned(),
                            )
                        })
                        .filter(|(id, _)| !id.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            publisher.publish(items);
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    })
}

fn notify_apps(
    bus: zbus::Connection,
    publisher: Publisher<Vec<(String, String)>>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>> {
    Box::pin(async move {
        let proxy = isle_ipc::notifications::NotificationsProxy::new(&bus).await?;
        let changed = proxy.receive_changed().await?;
        each_change(&publisher, changed, || proxy.apps()).await
    })
}

fn modules(
    bus: zbus::Connection,
    publisher: Publisher<Vec<Module>>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>> {
    Box::pin(async move {
        let proxy = isle_ipc::island::IslandProxy::new(&bus).await?;
        let list = proxy.modules().await?;
        publisher.publish(
            list.into_iter()
                .map(|(id, name, glyph, _, sizes, _)| Module {
                    id,
                    name,
                    glyph,
                    sizes,
                })
                .collect(),
        );
        std::future::pending().await
    })
}

const PERMISSIONS: &str = "org.freedesktop.impl.portal.PermissionStore";
const PERMISSIONS_PATH: &str = "/org/freedesktop/impl/portal/PermissionStore";
const TABLES: [&str; 2] = ["screencast", "screenshot"];

fn permissions(
    bus: zbus::Connection,
    publisher: Publisher<Vec<Permission>>,
) -> futures_util::future::BoxFuture<'static, zbus::Result<()>> {
    Box::pin(async move {
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(PERMISSIONS)?
            .member("Changed")?
            .build();
        let changed = zbus::MessageStream::for_match_rule(rule, &bus, None).await?;
        each_change(&publisher, changed, || read_permissions(&bus)).await
    })
}

async fn read_permissions(bus: &zbus::Connection) -> zbus::Result<Vec<Permission>> {
    let mut all = BTreeSet::new();
    for table in TABLES {
        let ids: Vec<String> = match bus
            .call_method(
                Some(PERMISSIONS),
                PERMISSIONS_PATH,
                Some(PERMISSIONS),
                "List",
                &(table,),
            )
            .await
        {
            Ok(reply) => reply.body().deserialize()?,
            Err(_) => continue,
        };
        for id in ids {
            let (apps, _): (
                std::collections::HashMap<String, Vec<String>>,
                zvariant::OwnedValue,
            ) = bus
                .call_method(
                    Some(PERMISSIONS),
                    PERMISSIONS_PATH,
                    Some(PERMISSIONS),
                    "Lookup",
                    &(table, id.as_str()),
                )
                .await?
                .body()
                .deserialize()?;
            for (app, grants) in apps {
                if grants.iter().any(|g| g == "yes") && !app.is_empty() {
                    all.insert((table.to_owned(), app));
                }
            }
        }
    }
    Ok(all
        .into_iter()
        .map(|(table, app)| Permission { table, app })
        .collect())
}

/// Carries out an act on the worker; a display change answers with its id.
pub async fn run(bus: Option<zbus::Connection>, act: Act) -> Result<Option<u64>, String> {
    if let Act::Configure(changes) = &act {
        let bus = bus.ok_or_else(|| "not on the bus".to_owned())?;
        let proxy = isle_ipc::displays::DisplaysProxy::new(&bus)
            .await
            .map_err(|e| e.to_string())?;
        return proxy
            .configure(changes)
            .await
            .map(Some)
            .map_err(|e| e.to_string());
    }
    act_on(bus, act).await.map(|()| None)
}

async fn act_on(bus: Option<zbus::Connection>, act: Act) -> Result<(), String> {
    use isle_audio::Target;
    let err = |e: &dyn std::fmt::Display| e.to_string();
    let session = || bus.clone().ok_or_else(|| "not on the bus".to_owned());
    match act {
        Act::SetWifi(on) => isle_network::set_wifi(on).await.map_err(|e| err(&e)),
        Act::Scan(device) => isle_network::scan(&device).await.map_err(|e| err(&e)),
        Act::Join { device, ssid } => {
            let network = isle_network::Network {
                path: String::new(),
                ssid,
                strength: 0,
                secured: false,
                known: None,
            };
            isle_network::join(&device, &network)
                .await
                .map_err(|e| err(&e))
        }
        Act::JoinHidden {
            device,
            ssid,
            secured,
        } => isle_network::join_hidden(&device, &ssid, secured)
            .await
            .map_err(|e| err(&e)),
        Act::Activate(saved) => isle_network::activate(&saved).await.map_err(|e| err(&e)),
        Act::Deactivate(active) => isle_network::deactivate(&active).await.map_err(|e| err(&e)),
        Act::Forget(saved) => isle_network::forget(&saved).await.map_err(|e| err(&e)),
        Act::AutoJoin { saved, on } => isle_network::set_auto_join(&saved, on)
            .await
            .map_err(|e| err(&e)),
        Act::SetBluetooth { adapter, on } => isle_bluetooth::set_powered(&adapter, on)
            .await
            .map_err(|e| err(&e)),
        Act::Discover { adapter, on } => {
            let _ = isle_bluetooth::set_discoverable(&adapter, on).await;
            if on {
                isle_bluetooth::start_discovery(&adapter).await
            } else {
                isle_bluetooth::stop_discovery(&adapter).await
            }
            .map_err(|e| err(&e))
        }
        Act::BtConnect(device) => isle_bluetooth::connect(&device).await.map_err(|e| err(&e)),
        Act::BtDisconnect(device) => isle_bluetooth::disconnect(&device)
            .await
            .map_err(|e| err(&e)),
        Act::BtPair(device) => isle_bluetooth::pair(&device).await.map_err(|e| err(&e)),
        Act::BtForget { adapter, device } => isle_bluetooth::forget(&adapter, &device)
            .await
            .map_err(|e| err(&e)),
        Act::BtRename { device, name } => isle_bluetooth::rename(&device, &name)
            .await
            .map_err(|e| err(&e)),
        Act::SetDefaultOutput(name) => isle_audio::set_default_output(name)
            .await
            .map_err(|e| err(&e)),
        Act::SetDefaultInput(name) => isle_audio::set_default_input(name)
            .await
            .map_err(|e| err(&e)),
        Act::Volume {
            output,
            mut device,
            value,
        } => {
            device.volume = value;
            let balance = device.balance;
            isle_audio::set_balance(&device, output, balance)
                .await
                .map_err(|e| err(&e))
        }
        Act::Mute { output, device, on } => {
            let target = if output {
                Target::Output(device)
            } else {
                Target::Input(device)
            };
            isle_audio::set_mute(target, on).await.map_err(|e| err(&e))
        }
        Act::Balance { device, value } => isle_audio::set_balance(&device, true, value)
            .await
            .map_err(|e| err(&e)),
        Act::PlaySound(name) => match isle_audio::sounds::lookup(&name, "freedesktop") {
            Some(path) => isle_audio::sounds::play(&path).await.map_err(|e| err(&e)),
            None => Err(format!("no sound is called {name}")),
        },
        Act::Profile(profile) => isle_power::set_profile(&profile).await.map_err(|e| err(&e)),
        Act::Configure(_) => Ok(()),
        Act::Keep(id) => {
            let proxy = isle_ipc::displays::DisplaysProxy::new(&session()?)
                .await
                .map_err(|e| err(&e))?;
            proxy.keep(id).await.map(|_| ()).map_err(|e| err(&e))
        }
        Act::Revert(id) => {
            let proxy = isle_ipc::displays::DisplaysProxy::new(&session()?)
                .await
                .map_err(|e| err(&e))?;
            proxy.revert(id).await.map(|_| ()).map_err(|e| err(&e))
        }
        Act::Brightness { display, level } => {
            let proxy = isle_ipc::displays::DisplaysProxy::new(&session()?)
                .await
                .map_err(|e| err(&e))?;
            proxy
                .set_brightness(&display, level)
                .await
                .map(|_| ())
                .map_err(|e| err(&e))
        }
        Act::Redetect => {
            let proxy = isle_ipc::displays::DisplaysProxy::new(&session()?)
                .await
                .map_err(|e| err(&e))?;
            proxy.redetect().await.map_err(|e| err(&e))
        }
        Act::Identify => {
            let proxy = isle_ipc::displays::DisplaysProxy::new(&session()?)
                .await
                .map_err(|e| err(&e))?;
            proxy.identify().await.map_err(|e| err(&e))
        }
        Act::ToggleNightLight => {
            let proxy = isle_ipc::displays::DisplaysProxy::new(&session()?)
                .await
                .map_err(|e| err(&e))?;
            proxy
                .toggle_night_light()
                .await
                .map(|_| ())
                .map_err(|e| err(&e))
        }
        Act::EditControls => {
            let proxy = isle_ipc::island::IslandProxy::new(&session()?)
                .await
                .map_err(|e| err(&e))?;
            proxy.edit_controls().await.map_err(|e| err(&e))
        }
        Act::Revoke(Permission { table, app }) => {
            let bus = session()?;
            let ids: Vec<String> = bus
                .call_method(
                    Some(PERMISSIONS),
                    PERMISSIONS_PATH,
                    Some(PERMISSIONS),
                    "List",
                    &(table.as_str(),),
                )
                .await
                .map_err(|e| err(&e))?
                .body()
                .deserialize()
                .map_err(|e| err(&e))?;
            for id in ids {
                let _ = bus
                    .call_method(
                        Some(PERMISSIONS),
                        PERMISSIONS_PATH,
                        Some(PERMISSIONS),
                        "DeletePermission",
                        &(table.as_str(), id.as_str(), app.as_str()),
                    )
                    .await;
            }
            Ok(())
        }
        Act::Hostname(name) => {
            let system = zbus::Connection::system().await.map_err(|e| err(&e))?;
            system
                .call_method(
                    Some("org.freedesktop.hostname1"),
                    "/org/freedesktop/hostname1",
                    Some("org.freedesktop.hostname1"),
                    "SetStaticHostname",
                    &(name.as_str(), true),
                )
                .await
                .map(|_| ())
                .map_err(|e| err(&e))
        }
    }
}
