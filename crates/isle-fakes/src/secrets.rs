//! `org.freedesktop.secrets`: collections and their items, plain and encrypted sessions, lock
//! and unlock through a prompt the test completes, the signals, and a count of every secret
//! handed out.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use oo7::{Key, crypto};
use zbus::{
    Connection, ObjectServer, fdo,
    message::Header,
    object_server::SignalEmitter,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value},
};

const ROOT: &str = "/org/freedesktop/secrets";
const ENCRYPTED: &str = "dh-ietf1024-sha256-aes128-cbc-pkcs7";

/// An item as a test lays it out.
#[derive(Clone)]
pub struct ItemSpec {
    pub label: String,
    pub attributes: HashMap<String, String>,
    pub secret: Vec<u8>,
    pub content_type: String,
    pub created: u64,
    pub modified: u64,
}

impl ItemSpec {
    /// A text secret with `xdg:schema` set to `schema`.
    pub fn new(label: &str, schema: &str, attributes: &[(&str, &str)], secret: &str) -> Self {
        let mut attrs: HashMap<String, String> = attributes
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        attrs.insert("xdg:schema".into(), schema.into());
        Self {
            label: label.into(),
            attributes: attrs,
            secret: secret.as_bytes().to_vec(),
            content_type: "text/plain".into(),
            created: 1_700_000_000,
            modified: 1_700_000_000,
        }
    }
}

/// A collection as a test lays it out; `id` is the last part of its path.
#[derive(Clone)]
pub struct CollectionSpec {
    pub id: String,
    pub label: String,
    pub locked: bool,
    pub alias: Option<String>,
    pub items: Vec<ItemSpec>,
}

struct Item {
    spec: ItemSpec,
    deleted: bool,
}

struct Collection {
    spec: CollectionSpec,
    items: Vec<Item>,
}

/// What a prompt does once completed.
#[derive(Clone)]
enum Pending {
    Unlock(Vec<usize>),
}

#[derive(Default)]
struct Model {
    collections: Vec<Collection>,
    /// Each session's AES key; `None` for a plain one.
    sessions: Vec<Option<Arc<Key>>>,
    prompts: Vec<Option<Pending>>,
    /// Prompts the client has shown and the test has not completed.
    shown: Vec<usize>,
}

struct Shared {
    model: Mutex<Model>,
    fetched: AtomicU32,
    plain_only: AtomicBool,
    frozen: AtomicBool,
}

type Handle = Arc<Shared>;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn op(path: String) -> OwnedObjectPath {
    OwnedObjectPath::try_from(path).unwrap()
}

fn collection_path(id: &str) -> OwnedObjectPath {
    op(format!("{ROOT}/collection/{id}"))
}

fn item_path(id: &str, i: usize) -> OwnedObjectPath {
    op(format!("{ROOT}/collection/{id}/{i}"))
}

fn none() -> OwnedObjectPath {
    op("/".into())
}

/// The Secret Service's own errors, and the bus's.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.Secret.Error")]
enum SecretError {
    #[zbus(error)]
    ZBus(zbus::Error),
    IsLocked(String),
    NoSession(String),
}

impl From<fdo::Error> for SecretError {
    fn from(e: fdo::Error) -> Self {
        Self::ZBus(e.into())
    }
}

/// The secret as `(session, parameters, value, content type)`, encrypted for the session.
type Wire = (OwnedObjectPath, Vec<u8>, Vec<u8>, String);

async fn wait_unfrozen(shared: &Shared) {
    if shared.frozen.load(Ordering::Relaxed) {
        std::future::pending::<()>().await;
    }
}

fn session_index(path: &ObjectPath<'_>) -> Option<usize> {
    path.as_str()
        .strip_prefix(&format!("{ROOT}/session/"))?
        .parse()
        .ok()
}

fn seal(shared: &Shared, session: &ObjectPath<'_>, spec: &ItemSpec) -> Result<Wire, SecretError> {
    let key = {
        let m = shared.model.lock().unwrap();
        session_index(session)
            .and_then(|s| m.sessions.get(s).cloned())
            .ok_or_else(|| SecretError::NoSession("no such session".into()))?
    };
    shared.fetched.fetch_add(1, Ordering::Relaxed);
    let (parameters, value) = match key {
        Some(key) => {
            let iv = crypto::generate_iv()
                .map_err(|e| SecretError::from(fdo::Error::Failed(e.to_string())))?;
            let value = crypto::encrypt(&spec.secret, &key, &iv)
                .map_err(|e| SecretError::from(fdo::Error::Failed(e.to_string())))?;
            (iv, value)
        }
        None => (Vec::new(), spec.secret.clone()),
    };
    Ok((
        session.to_owned().into(),
        parameters,
        value,
        spec.content_type.clone(),
    ))
}

fn unseal(shared: &Shared, secret: &Wire) -> Result<Vec<u8>, SecretError> {
    let key = {
        let m = shared.model.lock().unwrap();
        session_index(&secret.0)
            .and_then(|s| m.sessions.get(s).cloned())
            .ok_or_else(|| SecretError::NoSession("no such session".into()))?
    };
    match key {
        Some(key) => crypto::decrypt(&secret.2, &key, &secret.1)
            .map(|v| v.to_vec())
            .map_err(|e| SecretError::from(fdo::Error::Failed(e.to_string()))),
        None => Ok(secret.2.clone()),
    }
}

fn matches(attrs: &HashMap<String, String>, want: &HashMap<String, String>) -> bool {
    want.iter().all(|(k, v)| attrs.get(k) == Some(v))
}

/// The collection index a collection or item path names, and the item index if an item.
fn locate(m: &Model, path: &str) -> Option<(usize, Option<usize>)> {
    let rest = path.strip_prefix(&format!("{ROOT}/collection/"))?;
    let (id, item) = match rest.split_once('/') {
        Some((id, i)) => (id, Some(i.parse().ok()?)),
        None => (rest, None),
    };
    let c = m.collections.iter().position(|c| c.spec.id == id)?;
    Some((c, item))
}

struct Service(Handle);

#[zbus::interface(name = "org.freedesktop.Secret.Service")]
impl Service {
    async fn open_session(
        &self,
        algorithm: String,
        input: OwnedValue,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<(OwnedValue, OwnedObjectPath)> {
        wait_unfrozen(&self.0).await;
        let (output, key) = match algorithm.as_str() {
            "plain" => (OwnedValue::from(zbus::zvariant::Str::from("")), None),
            ENCRYPTED if !self.0.plain_only.load(Ordering::Relaxed) => {
                let client =
                    Key::try_from(input).map_err(|e| fdo::Error::InvalidArgs(e.to_string()))?;
                let private =
                    Key::generate_private_key().map_err(|e| fdo::Error::Failed(e.to_string()))?;
                let public = Key::generate_public_key(&private)
                    .map_err(|e| fdo::Error::Failed(e.to_string()))?;
                let aes = Key::generate_aes_key(&private, &client)
                    .map_err(|e| fdo::Error::Failed(e.to_string()))?;
                (OwnedValue::from(public), Some(Arc::new(aes)))
            }
            _ => return Err(fdo::Error::NotSupported(format!("algorithm {algorithm}"))),
        };
        let index = {
            let mut m = self.0.model.lock().unwrap();
            m.sessions.push(key);
            m.sessions.len() - 1
        };
        let path = op(format!("{ROOT}/session/{index}"));
        server.at(&path, Session).await?;
        Ok((output, path))
    }

    async fn search_items(
        &self,
        attributes: HashMap<String, String>,
    ) -> (Vec<OwnedObjectPath>, Vec<OwnedObjectPath>) {
        wait_unfrozen(&self.0).await;
        let m = self.0.model.lock().unwrap();
        let mut unlocked = Vec::new();
        for c in &m.collections {
            if c.spec.locked {
                continue;
            }
            for (i, item) in c.items.iter().enumerate() {
                if !item.deleted && matches(&item.spec.attributes, &attributes) {
                    unlocked.push(item_path(&c.spec.id, i));
                }
            }
        }
        (unlocked, Vec::new())
    }

    async fn unlock(
        &self,
        objects: Vec<OwnedObjectPath>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<(Vec<OwnedObjectPath>, OwnedObjectPath)> {
        wait_unfrozen(&self.0).await;
        let (done, prompt) = {
            let mut m = self.0.model.lock().unwrap();
            let mut done = Vec::new();
            let mut waiting = Vec::new();
            for path in &objects {
                let Some((c, _)) = locate(&m, path.as_str()) else {
                    continue;
                };
                if m.collections[c].spec.locked {
                    if !waiting.contains(&c) {
                        waiting.push(c);
                    }
                } else {
                    done.push(path.clone());
                }
            }
            let prompt = (!waiting.is_empty()).then(|| {
                m.prompts.push(Some(Pending::Unlock(waiting.clone())));
                m.prompts.len() - 1
            });
            (done, prompt)
        };
        let Some(prompt) = prompt else {
            return Ok((done, none()));
        };
        let path = op(format!("{ROOT}/prompt/{prompt}"));
        server
            .at(
                &path,
                Prompt {
                    shared: self.0.clone(),
                    index: prompt,
                },
            )
            .await?;
        Ok((done, path))
    }

    async fn lock(
        &self,
        objects: Vec<OwnedObjectPath>,
        #[zbus(connection)] conn: &Connection,
    ) -> fdo::Result<(Vec<OwnedObjectPath>, OwnedObjectPath)> {
        wait_unfrozen(&self.0).await;
        let mut changed = Vec::new();
        {
            let mut m = self.0.model.lock().unwrap();
            for path in &objects {
                if let Some((c, _)) = locate(&m, path.as_str())
                    && !m.collections[c].spec.locked
                {
                    m.collections[c].spec.locked = true;
                    changed.push(collection_path(&m.collections[c].spec.id));
                }
            }
        }
        for path in &changed {
            let emitter = SignalEmitter::new(conn, ROOT)?;
            Service::collection_changed(&emitter, path.as_ref()).await?;
        }
        Ok((objects, none()))
    }

    async fn get_secrets(
        &self,
        items: Vec<OwnedObjectPath>,
        session: OwnedObjectPath,
    ) -> Result<HashMap<OwnedObjectPath, Wire>, SecretError> {
        wait_unfrozen(&self.0).await;
        let mut out = HashMap::new();
        for path in items {
            let spec = {
                let m = self.0.model.lock().unwrap();
                match locate(&m, path.as_str()) {
                    Some((c, Some(i))) if !m.collections[c].spec.locked => m.collections[c]
                        .items
                        .get(i)
                        .filter(|it| !it.deleted)
                        .map(|it| it.spec.clone()),
                    _ => None,
                }
            };
            if let Some(spec) = spec {
                out.insert(path, seal(&self.0, &session, &spec)?);
            }
        }
        Ok(out)
    }

    fn read_alias(&self, name: String) -> OwnedObjectPath {
        let m = self.0.model.lock().unwrap();
        m.collections
            .iter()
            .find(|c| c.spec.alias.as_deref() == Some(name.as_str()))
            .map_or_else(none, |c| collection_path(&c.spec.id))
    }

    #[zbus(property)]
    async fn collections(&self) -> Vec<OwnedObjectPath> {
        wait_unfrozen(&self.0).await;
        let m = self.0.model.lock().unwrap();
        m.collections
            .iter()
            .map(|c| collection_path(&c.spec.id))
            .collect()
    }

    #[zbus(signal)]
    async fn collection_changed(
        emitter: &SignalEmitter<'_>,
        collection: ObjectPath<'_>,
    ) -> zbus::Result<()>;
}

struct Session;

#[zbus::interface(name = "org.freedesktop.Secret.Session")]
impl Session {
    async fn close(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<()> {
        if let Some(path) = header.path() {
            server.remove::<Self, _>(path.to_owned()).await?;
        }
        Ok(())
    }
}

struct Prompt {
    shared: Handle,
    index: usize,
}

#[zbus::interface(name = "org.freedesktop.Secret.Prompt")]
impl Prompt {
    fn prompt(&self, _window_id: String) {
        self.shared.model.lock().unwrap().shown.push(self.index);
    }

    async fn dismiss(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> fdo::Result<()> {
        self.shared
            .model
            .lock()
            .unwrap()
            .shown
            .retain(|p| *p != self.index);
        Prompt::completed(&emitter, true, Value::from(Vec::<OwnedObjectPath>::new())).await?;
        Ok(())
    }

    #[zbus(signal)]
    async fn completed(
        emitter: &SignalEmitter<'_>,
        dismissed: bool,
        result: Value<'_>,
    ) -> zbus::Result<()>;
}

struct CollectionIface(Handle, String);

impl CollectionIface {
    fn index(&self, m: &Model) -> Option<usize> {
        m.collections.iter().position(|c| c.spec.id == self.1)
    }
}

#[zbus::interface(name = "org.freedesktop.Secret.Collection")]
impl CollectionIface {
    #[zbus(property)]
    async fn items(&self) -> Vec<OwnedObjectPath> {
        wait_unfrozen(&self.0).await;
        let m = self.0.model.lock().unwrap();
        let Some(c) = self.index(&m) else {
            return Vec::new();
        };
        let c = &m.collections[c];
        if c.spec.locked {
            return Vec::new();
        }
        c.items
            .iter()
            .enumerate()
            .filter(|(_, it)| !it.deleted)
            .map(|(i, _)| item_path(&c.spec.id, i))
            .collect()
    }

    #[zbus(property)]
    fn label(&self) -> String {
        let m = self.0.model.lock().unwrap();
        self.index(&m)
            .map(|c| m.collections[c].spec.label.clone())
            .unwrap_or_default()
    }

    #[zbus(property)]
    fn locked(&self) -> bool {
        let m = self.0.model.lock().unwrap();
        self.index(&m).is_none_or(|c| m.collections[c].spec.locked)
    }

    #[zbus(property)]
    fn created(&self) -> u64 {
        1_700_000_000
    }

    #[zbus(property)]
    fn modified(&self) -> u64 {
        1_700_000_000
    }

    async fn search_items(&self, attributes: HashMap<String, String>) -> Vec<OwnedObjectPath> {
        wait_unfrozen(&self.0).await;
        let m = self.0.model.lock().unwrap();
        let Some(c) = self.index(&m) else {
            return Vec::new();
        };
        let c = &m.collections[c];
        if c.spec.locked {
            return Vec::new();
        }
        c.items
            .iter()
            .enumerate()
            .filter(|(_, it)| !it.deleted && matches(&it.spec.attributes, &attributes))
            .map(|(i, _)| item_path(&c.spec.id, i))
            .collect()
    }

    async fn create_item(
        &self,
        properties: HashMap<String, OwnedValue>,
        secret: Wire,
        replace: bool,
        #[zbus(object_server)] server: &ObjectServer,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(OwnedObjectPath, OwnedObjectPath), SecretError> {
        wait_unfrozen(&self.0).await;
        let label = properties
            .get("org.freedesktop.Secret.Item.Label")
            .and_then(|v| String::try_from(v.clone()).ok())
            .unwrap_or_default();
        let attributes: HashMap<String, String> = properties
            .get("org.freedesktop.Secret.Item.Attributes")
            .and_then(|v| HashMap::try_from(v.clone()).ok())
            .unwrap_or_default();
        let value = unseal(&self.0, &secret)?;
        let (path, created) = {
            let mut m = self.0.model.lock().unwrap();
            let c = self
                .index(&m)
                .ok_or_else(|| fdo::Error::UnknownObject(self.1.clone()))?;
            if m.collections[c].spec.locked {
                return Err(SecretError::IsLocked("locked".into()));
            }
            let t = now();
            let existing = replace
                .then(|| {
                    m.collections[c]
                        .items
                        .iter()
                        .position(|it| !it.deleted && it.spec.attributes == attributes)
                })
                .flatten();
            let spec = ItemSpec {
                label,
                attributes,
                secret: value,
                content_type: secret.3.clone(),
                created: t,
                modified: t,
            };
            match existing {
                Some(i) => {
                    let it = &mut m.collections[c].items[i];
                    it.spec = ItemSpec {
                        created: it.spec.created,
                        ..spec
                    };
                    (item_path(&self.1, i), false)
                }
                None => {
                    m.collections[c].items.push(Item {
                        spec,
                        deleted: false,
                    });
                    (item_path(&self.1, m.collections[c].items.len() - 1), true)
                }
            }
        };
        if created {
            let i: usize = path.as_str().rsplit('/').next().unwrap().parse().unwrap();
            server
                .at(&path, ItemIface(self.0.clone(), self.1.clone(), i))
                .await?;
            Self::item_created(&emitter, path.as_ref()).await?;
        } else {
            Self::item_changed(&emitter, path.as_ref()).await?;
        }
        Ok((path, none()))
    }

    #[zbus(signal)]
    async fn item_created(emitter: &SignalEmitter<'_>, item: ObjectPath<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn item_deleted(emitter: &SignalEmitter<'_>, item: ObjectPath<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn item_changed(emitter: &SignalEmitter<'_>, item: ObjectPath<'_>) -> zbus::Result<()>;
}

struct ItemIface(Handle, String, usize);

impl ItemIface {
    fn with<T>(&self, f: impl FnOnce(&mut Collection, &mut Item) -> T) -> fdo::Result<T> {
        let mut m = self.0.model.lock().unwrap();
        let c = m
            .collections
            .iter()
            .position(|c| c.spec.id == self.1)
            .ok_or_else(|| fdo::Error::UnknownObject(self.1.clone()))?;
        let col = &mut m.collections[c];
        let mut item = std::mem::replace(
            col.items
                .get_mut(self.2)
                .ok_or_else(|| fdo::Error::UnknownObject(self.1.clone()))?,
            Item {
                spec: ItemSpec::new("", "", &[], ""),
                deleted: true,
            },
        );
        if item.deleted {
            col.items[self.2] = item;
            return Err(fdo::Error::UnknownObject(self.1.clone()));
        }
        let out = f(col, &mut item);
        col.items[self.2] = item;
        Ok(out)
    }

    async fn changed(&self, conn: &Connection) -> zbus::Result<()> {
        let emitter = SignalEmitter::new(conn, collection_path(&self.1))?;
        CollectionIface::item_changed(&emitter, item_path(&self.1, self.2).as_ref()).await
    }
}

#[zbus::interface(name = "org.freedesktop.Secret.Item")]
impl ItemIface {
    #[zbus(property)]
    fn locked(&self) -> fdo::Result<bool> {
        self.with(|c, _| c.spec.locked)
    }

    #[zbus(property)]
    async fn label(&self) -> fdo::Result<String> {
        wait_unfrozen(&self.0).await;
        self.with(|_, it| it.spec.label.clone())
    }

    #[zbus(property)]
    async fn set_label(
        &mut self,
        label: String,
        #[zbus(connection)] conn: &Connection,
    ) -> fdo::Result<()> {
        self.with(|_, it| {
            it.spec.label = label;
            it.spec.modified = now();
        })?;
        self.changed(conn).await?;
        Ok(())
    }

    #[zbus(property)]
    async fn attributes(&self) -> fdo::Result<HashMap<String, String>> {
        wait_unfrozen(&self.0).await;
        self.with(|_, it| it.spec.attributes.clone())
    }

    #[zbus(property)]
    async fn set_attributes(
        &mut self,
        attributes: HashMap<String, String>,
        #[zbus(connection)] conn: &Connection,
    ) -> fdo::Result<()> {
        self.with(|_, it| {
            it.spec.attributes = attributes;
            it.spec.modified = now();
        })?;
        self.changed(conn).await?;
        Ok(())
    }

    #[zbus(property)]
    fn created(&self) -> fdo::Result<u64> {
        self.with(|_, it| it.spec.created)
    }

    #[zbus(property)]
    fn modified(&self) -> fdo::Result<u64> {
        self.with(|_, it| it.spec.modified)
    }

    async fn get_secret(&self, session: OwnedObjectPath) -> Result<Wire, SecretError> {
        wait_unfrozen(&self.0).await;
        let (locked, spec) = self.with(|c, it| (c.spec.locked, it.spec.clone()))?;
        if locked {
            return Err(SecretError::IsLocked("locked".into()));
        }
        seal(&self.0, &session, &spec)
    }

    async fn set_secret(
        &mut self,
        secret: Wire,
        #[zbus(connection)] conn: &Connection,
    ) -> Result<(), SecretError> {
        let value = unseal(&self.0, &secret)?;
        self.with(|_, it| {
            it.spec.secret = value;
            it.spec.content_type = secret.3.clone();
            it.spec.modified = now();
        })?;
        self.changed(conn).await?;
        Ok(())
    }

    async fn delete(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<OwnedObjectPath> {
        wait_unfrozen(&self.0).await;
        self.with(|_, it| it.deleted = true)?;
        let path = item_path(&self.1, self.2);
        let emitter = SignalEmitter::new(conn, collection_path(&self.1))?;
        CollectionIface::item_deleted(&emitter, path.as_ref()).await?;
        let _ = server.remove::<Self, _>(&path).await;
        Ok(none())
    }
}

/// A Secret Service on the bus at an address.
pub struct SecretService {
    shared: Handle,
    conn: Connection,
}

impl SecretService {
    pub async fn start(address: &str, collections: Vec<CollectionSpec>) -> zbus::Result<Self> {
        let shared: Handle = Arc::new(Shared {
            model: Mutex::new(Model {
                collections: collections
                    .iter()
                    .map(|spec| Collection {
                        spec: spec.clone(),
                        items: spec
                            .items
                            .iter()
                            .map(|s| Item {
                                spec: s.clone(),
                                deleted: false,
                            })
                            .collect(),
                    })
                    .collect(),
                ..Model::default()
            }),
            fetched: AtomicU32::new(0),
            plain_only: AtomicBool::new(false),
            frozen: AtomicBool::new(false),
        });
        let mut builder =
            zbus::connection::Builder::address(address)?.serve_at(ROOT, Service(shared.clone()))?;
        for c in &collections {
            builder = builder.serve_at(
                collection_path(&c.id),
                CollectionIface(shared.clone(), c.id.clone()),
            )?;
            if let Some(alias) = &c.alias {
                builder = builder.serve_at(
                    format!("{ROOT}/aliases/{alias}"),
                    CollectionIface(shared.clone(), c.id.clone()),
                )?;
            }
            for i in 0..c.items.len() {
                builder = builder.serve_at(
                    item_path(&c.id, i),
                    ItemIface(shared.clone(), c.id.clone(), i),
                )?;
            }
        }
        let conn = builder.name("org.freedesktop.secrets")?.build().await?;
        Ok(Self { shared, conn })
    }

    /// Secrets handed out by `GetSecret` and `GetSecrets`, one per item.
    pub fn fetched(&self) -> u32 {
        self.shared.fetched.load(Ordering::Relaxed)
    }

    /// Refuses the encrypted algorithm, as a service that offers only `plain` does.
    pub fn plain_only(&self) {
        self.shared.plain_only.store(true, Ordering::Relaxed);
    }

    /// Stops answering every call that reads the keyring, as a hung daemon does.
    pub fn freeze(&self) {
        self.shared.frozen.store(true, Ordering::Relaxed);
    }

    /// Prompts a client has shown and the test has not completed.
    pub fn prompts_shown(&self) -> usize {
        self.shared.model.lock().unwrap().shown.len()
    }

    /// Completes the oldest prompt shown: unlocks what it asked for, or dismisses it.
    pub async fn complete_prompt(&self, unlock: bool) -> zbus::Result<()> {
        let (index, paths) = {
            let mut m = self.shared.model.lock().unwrap();
            if m.shown.is_empty() {
                return Err(zbus::Error::Failure("no prompt shown".into()));
            }
            let index = m.shown.remove(0);
            let pending = m.prompts[index].take();
            let mut paths = Vec::new();
            if unlock && let Some(Pending::Unlock(cs)) = &pending {
                for c in cs {
                    m.collections[*c].spec.locked = false;
                    paths.push(collection_path(&m.collections[*c].spec.id));
                }
            }
            (index, paths)
        };
        let emitter = SignalEmitter::new(&self.conn, format!("{ROOT}/prompt/{index}"))?;
        Prompt::completed(&emitter, !unlock, Value::from(paths.clone())).await?;
        let root = SignalEmitter::new(&self.conn, ROOT)?;
        for p in &paths {
            Service::collection_changed(&root, p.as_ref()).await?;
        }
        Ok(())
    }

    /// The secret item `i` of collection `id` now holds.
    pub fn secret(&self, id: &str, i: usize) -> Option<Vec<u8>> {
        let m = self.shared.model.lock().unwrap();
        let c = m.collections.iter().find(|c| c.spec.id == id)?;
        c.items
            .get(i)
            .filter(|it| !it.deleted)
            .map(|it| it.spec.secret.clone())
    }

    /// Item `i` of collection `id` as it is now, unless deleted.
    pub fn item(&self, id: &str, i: usize) -> Option<ItemSpec> {
        let m = self.shared.model.lock().unwrap();
        let c = m.collections.iter().find(|c| c.spec.id == id)?;
        c.items
            .get(i)
            .filter(|it| !it.deleted)
            .map(|it| it.spec.clone())
    }

    /// Items collection `id` holds, deleted ones included.
    pub fn len(&self, id: &str) -> usize {
        let m = self.shared.model.lock().unwrap();
        m.collections
            .iter()
            .find(|c| c.spec.id == id)
            .map_or(0, |c| c.items.len())
    }

    /// Changes an item's label as another client would, with the signal.
    pub async fn relabel(&self, id: &str, i: usize, label: &str) -> zbus::Result<()> {
        {
            let mut m = self.shared.model.lock().unwrap();
            let c = m.collections.iter_mut().find(|c| c.spec.id == id).unwrap();
            c.items[i].spec.label = label.into();
        }
        let emitter = SignalEmitter::new(&self.conn, collection_path(id))?;
        CollectionIface::item_changed(&emitter, item_path(id, i).as_ref()).await
    }

    pub async fn vanish(self) {
        let _ = self.conn.close().await;
    }
}
