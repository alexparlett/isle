//! The sources, read and written on the worker: the Secret Service on Keychain's own
//! connection, NetworkManager and polkit on the system bus, `~/.ssh` and the agent. The window
//! asks; the store answers each ask and publishes what is listed after every change.

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use futures_channel::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
    oneshot,
};
use futures_util::StreamExt;
use isle_network::Keeps;
use isle_secrets::{
    agent::Agent,
    kinds::{self, Kind, Owner},
    service::{self, Change, Keyring},
    ssh::{self, NewKind},
    wifi,
};
use zbus::zvariant::ObjectPath;
use zeroize::Zeroizing;

use crate::model::{self, Detail, Item, ItemId, Snapshot, Source};

/// Why an ask could not be done, in words the window shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Refused,
    Locked,
    Missing,
    Dismissed,
    /// A wrong passphrase.
    Passphrase,
    Other(String),
}

impl From<service::Error> for Failure {
    fn from(e: service::Error) -> Self {
        match e {
            service::Error::Refused => Failure::Refused,
            service::Error::Locked => Failure::Locked,
            service::Error::Missing => Failure::Missing,
            service::Error::Dismissed => Failure::Dismissed,
            other => Failure::Other(other.to_string()),
        }
    }
}

type Reply<T> = oneshot::Sender<Result<T, Failure>>;

/// A login's fields as the window edits them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoginFields {
    pub title: String,
    pub user: String,
    pub website: String,
    pub notes: String,
}

pub enum Ask {
    Reload,
    /// Asks polkit for the reveal gate; true when given.
    Authorize(oneshot::Sender<bool>),
    /// One item's secret; `None` when it has none saved.
    Reveal(ItemId, Reply<Option<Zeroizing<String>>>),
    Unlock(String),
    Lock(String),
    ChangePassword(String),
    CreateLogin {
        collection: String,
        fields: LoginFields,
        password: Zeroizing<String>,
        reply: Reply<ItemId>,
    },
    /// A login's fields, or an Apps item's title; the secret when it changed.
    Update {
        id: ItemId,
        fields: LoginFields,
        login: bool,
        secret: Option<Zeroizing<String>>,
        reply: Reply<()>,
    },
    Delete(ItemId, Reply<()>),
    AddToAgent {
        key: String,
        passphrase: Zeroizing<String>,
        reply: Reply<()>,
    },
    RemoveFromAgent(String),
    /// Moves a key's files to the Trash, taking it out of the agent first.
    TrashKey(String, Reply<Vec<isle_fs::trash::Trashed>>),
    /// Puts trashed files back.
    Untrash(Vec<isle_fs::trash::Trashed>),
    GenerateKey {
        name: String,
        kind: NewKind,
        comment: String,
        passphrase: Zeroizing<String>,
        reply: Reply<String>,
    },
}

pub enum Event {
    Snapshot(Arc<Snapshot>),
    /// The session locked or unlocked.
    Locked(bool),
}

/// The store's handle: asks go in, events come out on the receiver `start` gave.
#[derive(Clone)]
pub struct Store(UnboundedSender<Ask>);

impl Store {
    pub fn ask(&self, ask: Ask) {
        let _ = self.0.unbounded_send(ask);
    }
}

impl PartialEq for Store {
    fn eq(&self, other: &Self) -> bool {
        self.0.same_receiver(&other.0)
    }
}

pub fn start(worker: &isle_app::Worker) -> (Store, UnboundedReceiver<Event>) {
    let (asks, asked) = unbounded();
    let (events, rx) = unbounded();
    worker.spawn(run(asked, events));
    (Store(asks), rx)
}

/// Names and icons of the apps that stored secrets.
struct Names {
    index: isle_apps::Index,
    icons: isle_apps::icons::Icons,
}

impl Names {
    fn load() -> Self {
        Self {
            index: isle_apps::Index::scan(&isle_apps::Index::data_dirs()),
            icons: isle_apps::icons::Icons::papirus(true),
        }
    }

    /// The app an id names, by a few spellings browsers' storage keys use.
    fn app(&mut self, id: &str) -> Option<(String, Option<PathBuf>)> {
        let lower = id.to_lowercase();
        let tries = [
            id.to_owned(),
            format!("google-{lower}"),
            format!("{lower}-browser"),
            format!("{lower}-desktop"),
        ];
        let found = tries
            .iter()
            .map(|t| self.index.resolve(t))
            .find(|r| self.index.get(r).is_some())?;
        let entry = self.index.get(&found)?;
        let icon = entry.icon.clone().and_then(|i| self.icons.lookup(&i, 32));
        Some((entry.name.clone(), icon))
    }

    fn owner(&mut self, owner: &Owner) -> (Option<String>, Option<PathBuf>) {
        match owner {
            Owner::Application(app) | Owner::Portal(app) => match self.app(app) {
                Some((name, icon)) => (Some(name), icon),
                None => (
                    Some(capitalised(app.rsplit('.').next().unwrap_or(app))),
                    None,
                ),
            },
            Owner::Known { name, id } => {
                let icon = self.app(id).and_then(|(_, i)| i);
                (Some((*name).to_owned()), icon)
            }
            Owner::Unknown => (None, None),
        }
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// The security a saved network uses, in words.
pub fn security_words(key_mgmt: &str) -> &'static str {
    match key_mgmt {
        "" => "None",
        "owe" => "Enhanced Open",
        "none" => "WEP",
        "sae" => "WPA3 Personal",
        "wpa-psk" => "WPA2 Personal",
        "wpa-eap" | "wpa-eap-suite-b-192" | "ieee8021x" => "WPA Enterprise",
        _ => "Secured",
    }
}

struct Sources {
    keyring: Option<Arc<Keyring>>,
    keyring_error: Option<service::Error>,
    can_change_password: bool,
    system: Option<zbus::Connection>,
    ssh_dir: PathBuf,
    agent: Option<Agent>,
    names: Names,
    /// The last snapshot, to find what an ask's item is.
    last: Arc<Snapshot>,
}

impl Sources {
    async fn open_keyring(&mut self) {
        if self.keyring.is_some() {
            return;
        }
        let opened = match Keyring::session_bus().await {
            Ok(conn) => Keyring::open(conn).await,
            Err(e) => Err(e),
        };
        match opened {
            Ok(k) => {
                self.can_change_password = k.can_change_password().await;
                self.keyring = Some(Arc::new(k));
                self.keyring_error = None;
            }
            Err(e) => {
                tracing::info!("no keyring: {e}");
                self.keyring_error = Some(e);
            }
        }
    }

    async fn read(&mut self) -> Snapshot {
        self.open_keyring().await;
        let mut out = Snapshot {
            loaded: true,
            ..Snapshot::default()
        };
        out.can_change_password = self.can_change_password;
        let collections = match &self.keyring {
            Some(k) => match k.list().await {
                Ok(list) => {
                    out.keyring = model::Keyring::Ready;
                    list
                }
                Err(service::Error::Missing) => {
                    self.keyring = None;
                    out.keyring = model::Keyring::Missing;
                    Vec::new()
                }
                Err(e) => {
                    out.keyring = model::Keyring::Failed(e.to_string());
                    Vec::new()
                }
            },
            None => {
                out.keyring = match &self.keyring_error {
                    Some(service::Error::Missing) | None => model::Keyring::Missing,
                    Some(e) => model::Keyring::Failed(e.to_string()),
                };
                Vec::new()
            }
        };
        let saved = match &self.system {
            Some(system) => wifi::saved(system).await.unwrap_or_default(),
            None => Vec::new(),
        };
        let networks: HashMap<&str, &isle_network::SavedWifi> =
            saved.iter().map(|w| (w.uuid.as_str(), w)).collect();

        for c in &collections {
            // gnome-keyring's session collection, unlabelled and empty, is no keyring to show.
            if c.label.is_empty() && c.items.is_empty() && c.path.as_str().ends_with("/collection/session") {
                continue;
            }
            out.sources.push(Source {
                path: c.path.to_string(),
                label: if c.label.is_empty() {
                    "Keyring".into()
                } else {
                    c.label.clone()
                },
                locked: c.locked,
                default: c.default,
            });
            for e in &c.items {
                let k = kinds::classify(&e.label, &e.attributes);
                // A Wi-Fi network's password shows on the network, not as an item.
                if k.nm.as_ref().is_some_and(|nm| {
                    k.kind == Kind::WiFi && networks.contains_key(nm.uuid.as_str())
                }) {
                    continue;
                }
                let kind = if k.kind == Kind::WiFi {
                    Kind::App
                } else {
                    k.kind
                };
                let (stored_by, icon) = self.names.owner(&k.owner);
                let second = match kind {
                    Kind::Login => k
                        .user
                        .clone()
                        .or_else(|| k.website.clone())
                        .unwrap_or_default(),
                    _ => stored_by.clone().unwrap_or_default(),
                };
                out.items.push(Item {
                    id: ItemId::Secret(e.path.to_string()),
                    kind,
                    title: if k.title.is_empty() {
                        "Untitled".into()
                    } else {
                        k.title.clone()
                    },
                    second,
                    source: out
                        .sources
                        .last()
                        .map(|s| s.label.clone())
                        .unwrap_or_default(),
                    collection: Some(c.path.to_string()),
                    detail: Detail::Secret {
                        user: k.user.clone(),
                        website: k.website.clone(),
                        notes: k.notes.clone(),
                        stored_by,
                        icon,
                        storage_key: k.storage_key,
                        created: e.created,
                        modified: e.modified,
                        attributes: e.attributes.clone(),
                    },
                });
            }
        }

        for w in &saved {
            let agent_item = (w.keeps == Keeps::Agent)
                .then(|| wifi::agent_item(&collections, w).map(|p| p.to_string()))
                .flatten();
            out.items.push(Item {
                id: ItemId::Wifi(w.path.clone()),
                kind: Kind::WiFi,
                title: w.id.clone(),
                second: security_words(&w.security).into(),
                source: "Wi-Fi".into(),
                collection: None,
                detail: Detail::Wifi {
                    saved: w.clone(),
                    agent_item,
                },
            });
        }

        let loaded = self
            .agent
            .as_ref()
            .and_then(|a| a.fingerprints().ok())
            .unwrap_or_default();
        for key in ssh::keys(&self.ssh_dir) {
            out.items.push(Item {
                id: ItemId::Ssh(key.name.clone()),
                kind: Kind::SshKey,
                title: key.name.clone(),
                second: if key.comment.is_empty() {
                    key.kind.clone()
                } else {
                    format!("{} · {}", key.kind, key.comment)
                },
                source: "SSH".into(),
                collection: None,
                detail: Detail::Ssh {
                    in_agent: loaded.contains(&key.fingerprint),
                    key,
                },
            });
        }
        out
    }

    fn item(&self, id: &ItemId) -> Option<Item> {
        self.last.items.iter().find(|i| &i.id == id).cloned()
    }

    async fn reveal(&self, id: &ItemId) -> Result<Option<Zeroizing<String>>, Failure> {
        match id {
            ItemId::Secret(path) => {
                let keyring = self.keyring.as_ref().ok_or(Failure::Missing)?;
                let path = ObjectPath::try_from(path.as_str())
                    .map_err(|e| Failure::Other(e.to_string()))?;
                Ok(Some(keyring.fetch(&path).await?.text()))
            }
            ItemId::Wifi(_) => {
                let Some(Item {
                    detail: Detail::Wifi { saved, agent_item },
                    ..
                }) = self.item(id)
                else {
                    return Err(Failure::Other("the network is gone".into()));
                };
                let system = self.system.as_ref().ok_or(Failure::Missing)?;
                let item =
                    agent_item.and_then(|p| zbus::zvariant::OwnedObjectPath::try_from(p).ok());
                Ok(
                    wifi::password(system, self.keyring.as_deref(), item.as_ref(), &saved)
                        .await?
                        .map(|r| r.text()),
                )
            }
            ItemId::Ssh(_) => Ok(None),
        }
    }

    async fn update(
        &self,
        id: &ItemId,
        fields: LoginFields,
        login: bool,
        secret: Option<Zeroizing<String>>,
    ) -> Result<(), Failure> {
        let (
            ItemId::Secret(path),
            Some(Item {
                detail: Detail::Secret { attributes, .. },
                ..
            }),
        ) = (id, self.item(id))
        else {
            return Err(Failure::Other("only a keyring item is edited".into()));
        };
        let keyring = self.keyring.as_ref().ok_or(Failure::Missing)?;
        let path =
            ObjectPath::try_from(path.as_str()).map_err(|e| Failure::Other(e.to_string()))?;
        let mut change = Change {
            label: Some(fields.title.clone()),
            secret,
            ..Change::default()
        };
        // An Apps item's attributes are what its app looks it up by; only a login's change.
        if login {
            let mut next = attributes.clone();
            let user_key = ["user", "username", "login", "account"]
                .into_iter()
                .find(|k| attributes.contains_key(*k))
                .unwrap_or("user");
            next.insert(user_key.into(), fields.user.clone());
            let site_key = ["server", "url", "uri", "origin", "host", "domain"]
                .into_iter()
                .find(|k| attributes.contains_key(*k));
            match site_key {
                Some("server") | None => {
                    let fresh = kinds::login_attributes(&fields.user, &fields.website, "");
                    for k in ["server", "protocol", "port"] {
                        match fresh.get(k) {
                            Some(v) => next.insert(k.into(), v.clone()),
                            None => next.remove(k),
                        };
                    }
                }
                Some(k) => {
                    next.insert(k.into(), fields.website.clone());
                }
            }
            if fields.notes.is_empty() {
                next.remove(kinds::NOTES);
            } else {
                next.insert(kinds::NOTES.into(), fields.notes.clone());
            }
            if next != attributes {
                change.attributes = Some(next);
            }
        }
        keyring.update(&path, change).await.map_err(Failure::from)
    }
}

async fn run(mut asked: UnboundedReceiver<Ask>, events: UnboundedSender<Event>) {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut sources = Sources {
        keyring: None,
        keyring_error: None,
        can_change_password: false,
        system: zbus::Connection::system().await.ok(),
        ssh_dir: home.join(".ssh"),
        agent: Agent::find(),
        names: Names::load(),
        last: Arc::default(),
    };
    let publish = |sources: &mut Sources, snap: Snapshot| {
        let snap = Arc::new(snap);
        sources.last = snap.clone();
        let _ = events.unbounded_send(Event::Snapshot(snap));
    };
    let first = sources.read().await;
    publish(&mut sources, first);

    let (changed_tx, mut changed) = unbounded::<()>();
    let watch_changes = |keyring: &Arc<Keyring>, tx: UnboundedSender<()>| {
        let conn = keyring.connection().clone();
        tokio::spawn(async move {
            let Ok(stream) = service::changes(&conn).await else {
                return;
            };
            let mut stream = std::pin::pin!(stream);
            while stream.next().await.is_some() {
                if tx.unbounded_send(()).is_err() {
                    return;
                }
            }
        });
    };
    let mut watching = false;
    if let Some(k) = &sources.keyring {
        watch_changes(k, changed_tx.clone());
        watching = true;
    }
    if let Ok(bus) = zbus::Connection::session().await {
        let events = events.clone();
        tokio::spawn(async move {
            let Ok(session) = isle_ipc::session::SessionProxy::new(&bus).await else {
                return;
            };
            let Ok(mut changes) = session.receive_lock_changed().await else {
                return;
            };
            while changes.next().await.is_some() {
                let locked = session.locked().await.unwrap_or(false);
                if events.unbounded_send(Event::Locked(locked)).is_err() {
                    return;
                }
            }
        });
    }

    loop {
        let ask = tokio::select! {
            ask = asked.next() => match ask {
                Some(ask) => ask,
                None => return,
            },
            Some(()) = changed.next() => {
                // Signals come in bursts; one read after the burst.
                tokio::time::sleep(Duration::from_millis(50)).await;
                while changed.try_recv().is_ok() {}
                Ask::Reload
            }
        };
        let mut reload = false;
        match ask {
            Ask::Reload => reload = true,
            Ask::Authorize(reply) => {
                let allowed = match &sources.system {
                    Some(system) => isle_secrets::gate::ask(system).await.unwrap_or_else(|e| {
                        tracing::warn!("the reveal gate could not ask: {e}");
                        false
                    }),
                    None => false,
                };
                let _ = reply.send(allowed);
            }
            Ask::Reveal(id, reply) => {
                let _ = reply.send(sources.reveal(&id).await);
            }
            Ask::Unlock(path) | Ask::Lock(path) | Ask::ChangePassword(path)
                if sources.keyring.is_none() =>
            {
                tracing::warn!("no keyring to act on {path}");
            }
            // A prompt waits on the user, so it waits apart from the asks behind it.
            Ask::Unlock(path) => {
                if let Some(k) = sources.keyring.clone() {
                    let tx = changed_tx.clone();
                    tokio::spawn(async move {
                        if let Ok(p) = ObjectPath::try_from(path.as_str())
                            && let Err(e) = k.unlock(&p).await
                        {
                            tracing::info!("unlock: {e}");
                        }
                        let _ = tx.unbounded_send(());
                    });
                }
            }
            Ask::Lock(path) => {
                if let (Some(k), Ok(p)) = (&sources.keyring, ObjectPath::try_from(path.as_str())) {
                    let _ = k.lock(&p).await;
                }
                reload = true;
            }
            Ask::ChangePassword(path) => {
                if let Some(k) = sources.keyring.clone() {
                    tokio::spawn(async move {
                        if let Ok(p) = ObjectPath::try_from(path.as_str()) {
                            let _ = k.change_password(&p).await;
                        }
                    });
                }
            }
            Ask::CreateLogin {
                collection,
                fields,
                password,
                reply,
            } => {
                let result = async {
                    let k = sources.keyring.as_ref().ok_or(Failure::Missing)?;
                    let path = ObjectPath::try_from(collection.as_str())
                        .map_err(|e| Failure::Other(e.to_string()))?;
                    let attributes =
                        kinds::login_attributes(&fields.user, &fields.website, &fields.notes);
                    let title = if fields.title.is_empty() {
                        fields.website.clone()
                    } else {
                        fields.title.clone()
                    };
                    let item = k.create(&path, &title, &attributes, &password).await?;
                    Ok(ItemId::Secret(item.to_string()))
                }
                .await;
                let _ = reply.send(result);
                reload = true;
            }
            Ask::Update {
                id,
                fields,
                login,
                secret,
                reply,
            } => {
                let _ = reply.send(sources.update(&id, fields, login, secret).await);
                reload = true;
            }
            Ask::Delete(id, reply) => {
                let result = match &id {
                    ItemId::Secret(path) => {
                        match (&sources.keyring, ObjectPath::try_from(path.as_str())) {
                            (Some(k), Ok(p)) => k.delete(&p).await.map_err(Failure::from),
                            _ => Err(Failure::Missing),
                        }
                    }
                    _ => Err(Failure::Other("only a keyring item is deleted here".into())),
                };
                let _ = reply.send(result);
                reload = true;
            }
            Ask::AddToAgent {
                key,
                passphrase,
                reply,
            } => {
                let result = match (sources.item(&ItemId::Ssh(key)), &sources.agent) {
                    (
                        Some(Item {
                            detail: Detail::Ssh { key, .. },
                            ..
                        }),
                        Some(agent),
                    ) => {
                        let (agent, passphrase) = (agent.clone(), passphrase);
                        tokio::task::spawn_blocking(move || match ssh::decrypt(&key, &passphrase) {
                            Ok(private) => agent
                                .add(&private)
                                .map_err(|e| Failure::Other(e.to_string())),
                            Err(ssh::DecryptError::WrongPassphrase) => Err(Failure::Passphrase),
                            Err(ssh::DecryptError::Unreadable) => {
                                Err(Failure::Other("the key can't be read".into()))
                            }
                        })
                        .await
                        .unwrap_or_else(|e| Err(Failure::Other(e.to_string())))
                    }
                    (_, None) => Err(Failure::Missing),
                    _ => Err(Failure::Other("the key is gone".into())),
                };
                let _ = reply.send(result);
                reload = true;
            }
            Ask::RemoveFromAgent(key) => {
                if let (
                    Some(Item {
                        detail: Detail::Ssh { key, .. },
                        ..
                    }),
                    Some(agent),
                ) = (sources.item(&ItemId::Ssh(key)), &sources.agent)
                    && let Ok(public) = ssh_key::PublicKey::from_openssh(&key.public_key)
                    && let Err(e) = agent.remove(&public)
                {
                    tracing::warn!("removing a key from the agent: {e}");
                }
                reload = true;
            }
            Ask::TrashKey(name, reply) => {
                let result = match sources.item(&ItemId::Ssh(name)) {
                    Some(Item { detail: Detail::Ssh { key, in_agent }, .. }) => {
                        if in_agent
                            && let Some(agent) = &sources.agent
                            && let Ok(public) = ssh_key::PublicKey::from_openssh(&key.public_key)
                        {
                            let _ = agent.remove(&public);
                        }
                        let mut trashed = Vec::new();
                        let mut failed = None;
                        for path in key.private.iter().chain(key.public.iter()) {
                            match isle_fs::trash::trash(path) {
                                Ok(t) => trashed.push(t),
                                Err(e) => failed = Some(e.to_string()),
                            }
                        }
                        match failed {
                            Some(why) if trashed.is_empty() => Err(Failure::Other(why)),
                            _ => Ok(trashed),
                        }
                    }
                    _ => Err(Failure::Other("the key is gone".into())),
                };
                let _ = reply.send(result);
                reload = true;
            }
            Ask::Untrash(items) => {
                for item in &items {
                    if let Err(e) = isle_fs::trash::put_back(item) {
                        tracing::warn!("putting a key back: {e}");
                    }
                }
                reload = true;
            }
            Ask::GenerateKey {
                name,
                kind,
                comment,
                passphrase,
                reply,
            } => {
                let dir = sources.ssh_dir.clone();
                let result = tokio::task::spawn_blocking(move || {
                    ssh::generate(&dir, &name, kind, &comment, &passphrase)
                        .map(|k| k.name)
                        .map_err(|e| Failure::Other(e.to_string()))
                })
                .await
                .unwrap_or_else(|e| Err(Failure::Other(e.to_string())));
                let _ = reply.send(result);
                reload = true;
            }
        }
        if !watching && let Some(k) = &sources.keyring {
            watch_changes(k, changed_tx.clone());
            watching = true;
        }
        if reload {
            let snap = sources.read().await;
            publish(&mut sources, snap);
        }
    }
}
