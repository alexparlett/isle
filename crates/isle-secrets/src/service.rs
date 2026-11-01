//! The Secret Service: its collections and their items listed without secrets, one secret
//! fetched for one item, and the writes Keychain makes.

use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::{Stream, StreamExt, future::join_all};
use oo7::{Key, crypto};
use zbus::{
    Connection, MatchRule, MessageStream,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value},
};
use zeroize::Zeroizing;

use crate::Revealed;

const BUS: &str = "org.freedesktop.secrets";
const ROOT: &str = "/org/freedesktop/secrets";
const ENCRYPTED: &str = "dh-ietf1024-sha256-aes128-cbc-pkcs7";

/// How long a call may take before the service counts as not answering.
pub const ANSWER: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Nothing owns the Secret Service's name.
    Missing,
    /// The service did not answer in time.
    Frozen,
    Locked,
    /// The user dismissed the service's prompt.
    Dismissed,
    /// An authorization check said no.
    Refused,
    Failed(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => write!(f, "no keyring service is running"),
            Self::Frozen => write!(f, "the keyring service is not answering"),
            Self::Locked => write!(f, "the keyring is locked"),
            Self::Dismissed => write!(f, "the prompt was dismissed"),
            Self::Refused => write!(f, "not authorized"),
            Self::Failed(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<zbus::Error> for Error {
    fn from(e: zbus::Error) -> Self {
        match &e {
            zbus::Error::MethodError(name, _, _) => match name.as_str() {
                "org.freedesktop.DBus.Error.ServiceUnknown"
                | "org.freedesktop.DBus.Error.NameHasNoOwner" => Self::Missing,
                "org.freedesktop.DBus.Error.NoReply" | "org.freedesktop.DBus.Error.Timeout" => {
                    Self::Frozen
                }
                "org.freedesktop.Secret.Error.IsLocked" => Self::Locked,
                _ => Self::Failed(e.to_string()),
            },
            zbus::Error::FDO(f) => match f.as_ref() {
                zbus::fdo::Error::ServiceUnknown(_) | zbus::fdo::Error::NameHasNoOwner(_) => {
                    Self::Missing
                }
                zbus::fdo::Error::NoReply(_) | zbus::fdo::Error::Timeout(_) => Self::Frozen,
                zbus::fdo::Error::Failed(m)
                    if m.starts_with("org.freedesktop.Secret.Error.IsLocked") =>
                {
                    Self::Locked
                }
                _ => Self::Failed(e.to_string()),
            },
            zbus::Error::InputOutput(io) if io.kind() == std::io::ErrorKind::TimedOut => {
                Self::Frozen
            }
            _ => Self::Failed(e.to_string()),
        }
    }
}

impl From<zbus::fdo::Error> for Error {
    fn from(e: zbus::fdo::Error) -> Self {
        zbus::Error::from(e).into()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

async fn answered<T>(f: impl Future<Output = zbus::Result<T>>) -> Result<T> {
    match tokio::time::timeout(ANSWER, f).await {
        Ok(r) => r.map_err(Error::from),
        Err(_) => Err(Error::Frozen),
    }
}

/// An item as listed: what the service gives without its secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: OwnedObjectPath,
    pub collection: OwnedObjectPath,
    pub label: String,
    pub attributes: HashMap<String, String>,
    /// Seconds since the epoch.
    pub created: u64,
    pub modified: u64,
}

/// A collection as listed, with its items while unlocked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub path: OwnedObjectPath,
    pub label: String,
    pub locked: bool,
    /// Whether this is the collection the `default` alias names.
    pub default: bool,
    pub items: Vec<Entry>,
}

/// What a write changes on an item; `None` leaves that part as it is.
#[derive(Default)]
pub struct Change {
    pub label: Option<String>,
    pub attributes: Option<HashMap<String, String>>,
    pub secret: Option<Zeroizing<String>>,
}

/// A session with the Secret Service.
pub struct Keyring {
    conn: Connection,
    session: OwnedObjectPath,
    aes: Option<Arc<Key>>,
}

impl std::fmt::Debug for Keyring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keyring")
            .field("session", &self.session.as_str())
            .field("encrypted", &self.aes.is_some())
            .finish()
    }
}

fn prop<T: TryFrom<OwnedValue>>(props: &HashMap<String, OwnedValue>, name: &str) -> Option<T> {
    props.get(name).cloned().and_then(|v| T::try_from(v).ok())
}

impl Keyring {
    /// Opens a session on `conn`, encrypted unless the service offers only `plain`.
    pub async fn open(conn: Connection) -> Result<Self> {
        let private = Key::generate_private_key().map_err(|e| Error::Failed(e.to_string()))?;
        let public =
            Key::generate_public_key(&private).map_err(|e| Error::Failed(e.to_string()))?;
        let encrypted = answered(conn.call_method(
            Some(BUS),
            ROOT,
            Some("org.freedesktop.Secret.Service"),
            "OpenSession",
            &(ENCRYPTED, Value::from(public)),
        ))
        .await;
        let (aes, session) = match encrypted {
            Ok(reply) => {
                let (output, session): (OwnedValue, OwnedObjectPath) =
                    reply.body().deserialize().map_err(zbus::Error::from)?;
                let server = Key::try_from(output).map_err(|e| Error::Failed(e.to_string()))?;
                let aes = Key::generate_aes_key(&private, &server)
                    .map_err(|e| Error::Failed(e.to_string()))?;
                (Some(Arc::new(aes)), session)
            }
            Err(Error::Failed(_)) => {
                let reply = answered(conn.call_method(
                    Some(BUS),
                    ROOT,
                    Some("org.freedesktop.Secret.Service"),
                    "OpenSession",
                    &("plain", Value::from("")),
                ))
                .await?;
                let (_, session): (OwnedValue, OwnedObjectPath) =
                    reply.body().deserialize().map_err(zbus::Error::from)?;
                (None, session)
            }
            Err(e) => return Err(e),
        };
        Ok(Self { conn, session, aes })
    }

    /// A connection of Keychain's own to the session bus.
    pub async fn session_bus() -> Result<Connection> {
        Ok(zbus::connection::Builder::session()?
            .method_timeout(ANSWER)
            .build()
            .await?)
    }

    pub fn encrypted(&self) -> bool {
        self.aes.is_some()
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    async fn properties(
        &self,
        path: &ObjectPath<'_>,
        interface: &str,
    ) -> Result<HashMap<String, OwnedValue>> {
        let reply = answered(self.conn.call_method(
            Some(BUS),
            path,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(interface,),
        ))
        .await?;
        Ok(reply.body().deserialize().map_err(zbus::Error::from)?)
    }

    async fn entry(&self, collection: &OwnedObjectPath, path: OwnedObjectPath) -> Result<Entry> {
        let props = self
            .properties(&path, "org.freedesktop.Secret.Item")
            .await?;
        Ok(Entry {
            collection: collection.clone(),
            label: prop(&props, "Label").unwrap_or_default(),
            attributes: prop(&props, "Attributes").unwrap_or_default(),
            created: prop(&props, "Created").unwrap_or_default(),
            modified: prop(&props, "Modified").unwrap_or_default(),
            path,
        })
    }

    /// The item at `path` in `collection`, as listed.
    pub async fn item(&self, collection: &OwnedObjectPath, path: OwnedObjectPath) -> Result<Entry> {
        self.entry(collection, path).await
    }

    async fn source(
        &self,
        path: OwnedObjectPath,
        default: Option<&OwnedObjectPath>,
    ) -> Result<Source> {
        let props = self
            .properties(&path, "org.freedesktop.Secret.Collection")
            .await?;
        let locked = prop(&props, "Locked").unwrap_or(true);
        let paths: Vec<OwnedObjectPath> = prop(&props, "Items").unwrap_or_default();
        let items = join_all(paths.into_iter().map(|p| self.entry(&path, p)))
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect();
        Ok(Source {
            default: default == Some(&path),
            label: prop(&props, "Label").unwrap_or_default(),
            locked,
            items,
            path,
        })
    }

    /// Every collection and its items, without a secret.
    pub async fn list(&self) -> Result<Vec<Source>> {
        let props = self
            .properties(
                &ObjectPath::from_static_str_unchecked(ROOT),
                "org.freedesktop.Secret.Service",
            )
            .await?;
        let paths: Vec<OwnedObjectPath> = prop(&props, "Collections").unwrap_or_default();
        let default = self.alias("default").await.ok().flatten();
        let sources = join_all(paths.into_iter().map(|p| self.source(p, default.as_ref()))).await;
        sources.into_iter().collect()
    }

    async fn alias(&self, name: &str) -> Result<Option<OwnedObjectPath>> {
        let reply = answered(self.conn.call_method(
            Some(BUS),
            ROOT,
            Some("org.freedesktop.Secret.Service"),
            "ReadAlias",
            &(name,),
        ))
        .await?;
        let path: OwnedObjectPath = reply.body().deserialize().map_err(zbus::Error::from)?;
        Ok((path.as_str() != "/").then_some(path))
    }

    /// The secret of one item. The only place a secret is read.
    pub async fn fetch(&self, item: &ObjectPath<'_>) -> Result<Revealed> {
        let reply = answered(self.conn.call_method(
            Some(BUS),
            item,
            Some("org.freedesktop.Secret.Item"),
            "GetSecret",
            &(&self.session,),
        ))
        .await?;
        let (_, parameters, value, content_type): (OwnedObjectPath, Vec<u8>, Vec<u8>, String) =
            reply.body().deserialize().map_err(zbus::Error::from)?;
        let value = Zeroizing::new(value);
        let bytes = match &self.aes {
            Some(key) => crypto::decrypt(&*value, key, &parameters)
                .map_err(|_| Error::Failed("the secret could not be decrypted".into()))?,
            None => value,
        };
        Ok(Revealed::new(bytes, &content_type))
    }

    fn wire(&self, secret: &str) -> Result<(OwnedObjectPath, Vec<u8>, Vec<u8>, &'static str)> {
        Ok(match &self.aes {
            Some(key) => {
                let iv = crypto::generate_iv().map_err(|e| Error::Failed(e.to_string()))?;
                let value = crypto::encrypt(secret.as_bytes(), key, &iv)
                    .map_err(|e| Error::Failed(e.to_string()))?;
                (self.session.clone(), iv, value, "text/plain")
            }
            None => (
                self.session.clone(),
                Vec::new(),
                secret.as_bytes().to_vec(),
                "text/plain",
            ),
        })
    }

    /// Creates an item in `collection`; its path.
    pub async fn create(
        &self,
        collection: &ObjectPath<'_>,
        label: &str,
        attributes: &HashMap<String, String>,
        secret: &str,
    ) -> Result<OwnedObjectPath> {
        let properties: HashMap<&str, Value<'_>> = HashMap::from([
            ("org.freedesktop.Secret.Item.Label", Value::from(label)),
            (
                "org.freedesktop.Secret.Item.Attributes",
                Value::from(attributes.clone()),
            ),
        ]);
        let mut wire = self.wire(secret)?;
        let reply = answered(self.conn.call_method(
            Some(BUS),
            collection,
            Some("org.freedesktop.Secret.Collection"),
            "CreateItem",
            &(properties, &wire, false),
        ))
        .await;
        zeroize::Zeroize::zeroize(&mut wire.2);
        let (item, prompt): (OwnedObjectPath, OwnedObjectPath) =
            reply?.body().deserialize().map_err(zbus::Error::from)?;
        if prompt.as_str() != "/" {
            self.prompt(prompt).await?;
        }
        Ok(item)
    }

    /// Writes what `change` holds to an item.
    pub async fn update(&self, item: &ObjectPath<'_>, change: Change) -> Result<()> {
        if let Some(label) = change.label {
            self.set(item, "Label", Value::from(label)).await?;
        }
        if let Some(attributes) = change.attributes {
            self.set(item, "Attributes", Value::from(attributes))
                .await?;
        }
        if let Some(secret) = change.secret {
            let mut wire = self.wire(&secret)?;
            let r = answered(self.conn.call_method(
                Some(BUS),
                item,
                Some("org.freedesktop.Secret.Item"),
                "SetSecret",
                &(&wire,),
            ))
            .await;
            zeroize::Zeroize::zeroize(&mut wire.2);
            r?;
        }
        Ok(())
    }

    async fn set(&self, item: &ObjectPath<'_>, name: &str, value: Value<'_>) -> Result<()> {
        answered(self.conn.call_method(
            Some(BUS),
            item,
            Some("org.freedesktop.DBus.Properties"),
            "Set",
            &("org.freedesktop.Secret.Item", name, value),
        ))
        .await?;
        Ok(())
    }

    pub async fn delete(&self, item: &ObjectPath<'_>) -> Result<()> {
        let reply = answered(self.conn.call_method(
            Some(BUS),
            item,
            Some("org.freedesktop.Secret.Item"),
            "Delete",
            &(),
        ))
        .await?;
        let prompt: OwnedObjectPath = reply.body().deserialize().map_err(zbus::Error::from)?;
        if prompt.as_str() != "/" {
            self.prompt(prompt).await?;
        }
        Ok(())
    }

    /// Unlocks a collection, through the service's prompt when it asks.
    pub async fn unlock(&self, collection: &ObjectPath<'_>) -> Result<()> {
        self.lock_or_unlock("Unlock", collection).await
    }

    pub async fn lock(&self, collection: &ObjectPath<'_>) -> Result<()> {
        self.lock_or_unlock("Lock", collection).await
    }

    async fn lock_or_unlock(&self, method: &str, collection: &ObjectPath<'_>) -> Result<()> {
        let reply = answered(self.conn.call_method(
            Some(BUS),
            ROOT,
            Some("org.freedesktop.Secret.Service"),
            method,
            &(vec![collection],),
        ))
        .await?;
        let (_, prompt): (Vec<OwnedObjectPath>, OwnedObjectPath) =
            reply.body().deserialize().map_err(zbus::Error::from)?;
        if prompt.as_str() != "/" {
            self.prompt(prompt).await?;
        }
        Ok(())
    }

    /// Shows a prompt and waits, without limit, for the user to complete it.
    async fn prompt(&self, prompt: OwnedObjectPath) -> Result<()> {
        let rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender(BUS)?
            .path(prompt.clone())?
            .interface("org.freedesktop.Secret.Prompt")?
            .member("Completed")?
            .build();
        let mut stream = MessageStream::for_match_rule(rule, &self.conn, None).await?;
        answered(self.conn.call_method(
            Some(BUS),
            &prompt,
            Some("org.freedesktop.Secret.Prompt"),
            "Prompt",
            &("",),
        ))
        .await?;
        let message = stream
            .next()
            .await
            .ok_or_else(|| Error::Failed("the service left".into()))??;
        let (dismissed, _): (bool, OwnedValue) =
            message.body().deserialize().map_err(zbus::Error::from)?;
        if dismissed {
            Err(Error::Dismissed)
        } else {
            Ok(())
        }
    }

    /// Whether the service offers gnome-keyring's change-password call.
    pub async fn can_change_password(&self) -> bool {
        let Ok(reply) = answered(self.conn.call_method(
            Some(BUS),
            ROOT,
            Some("org.freedesktop.DBus.Introspectable"),
            "Introspect",
            &(),
        ))
        .await
        else {
            return false;
        };
        reply.body().deserialize::<String>().is_ok_and(|xml| {
            xml.contains("org.gnome.keyring.InternalUnsupportedGuiltRiddenInterface")
        })
    }

    /// Asks the service to change a collection's password through its own prompt.
    pub async fn change_password(&self, collection: &ObjectPath<'_>) -> Result<()> {
        let reply = answered(self.conn.call_method(
            Some(BUS),
            ROOT,
            Some("org.gnome.keyring.InternalUnsupportedGuiltRiddenInterface"),
            "ChangeWithPrompt",
            &(collection,),
        ))
        .await?;
        let prompt: OwnedObjectPath = reply.body().deserialize().map_err(zbus::Error::from)?;
        if prompt.as_str() != "/" {
            self.prompt(prompt).await?;
        }
        Ok(())
    }
}

/// What changed in the Secret Service, as its signals say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Changed {
    Item {
        collection: OwnedObjectPath,
        item: OwnedObjectPath,
        gone: bool,
    },
    Collection(OwnedObjectPath),
}

/// The service's change signals on `conn`.
pub async fn changes(conn: &Connection) -> Result<impl Stream<Item = Changed> + use<>> {
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender(BUS)?
        .path_namespace(ROOT)?
        .build();
    let stream = MessageStream::for_match_rule(rule, conn, None).await?;
    Ok(stream.filter_map(|m| async move {
        let m = m.ok()?;
        let header = m.header();
        let member = header.member()?.as_str().to_owned();
        let at: OwnedObjectPath = header.path()?.to_owned().into();
        match member.as_str() {
            "ItemCreated" | "ItemChanged" | "ItemDeleted" => {
                let item: OwnedObjectPath = m.body().deserialize().ok()?;
                Some(Changed::Item {
                    collection: at,
                    item,
                    gone: member == "ItemDeleted",
                })
            }
            "CollectionCreated" | "CollectionChanged" | "CollectionDeleted" => {
                Some(Changed::Collection(m.body().deserialize().ok()?))
            }
            _ => None,
        }
    }))
}
