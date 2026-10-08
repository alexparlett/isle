//! NetworkManager's secret agent: NetworkManager asks it for what a connection lacks, a
//! Wi-Fi password, an 802.1X sign-in, a VPN's secrets, and it answers from the keyring or the
//! user (`lock-and-prompts-2026.md` §4).

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use tokio::sync::watch;
use tracing::{debug, info};
use zbus::{
    Connection, interface,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

pub const AGENT_PATH: &str = "/org/freedesktop/NetworkManager/SecretAgent";
const IDENTIFIER: &str = "io.github.alexparlett.Isle";
/// The agent can take a VPN's hints, so two-factor prompts reach it.
const VPN_HINTS: u32 = 0x1;

pub const ALLOW_INTERACTION: u32 = 0x1;
pub const REQUEST_NEW: u32 = 0x2;
pub const USER_REQUESTED: u32 = 0x4;

/// A setting's items by key, as NetworkManager sends and takes them.
pub type Settings = HashMap<String, HashMap<String, OwnedValue>>;

/// What NetworkManager asks for.
#[derive(Debug, Default)]
pub struct Request {
    pub connection: Settings,
    pub path: String,
    pub setting: String,
    pub hints: Vec<String>,
    pub flags: u32,
}

impl Request {
    fn text(&self, setting: &str, key: &str) -> Option<String> {
        let value = self.connection.get(setting)?.get(key)?;
        String::try_from(value.try_clone().ok()?).ok()
    }

    pub fn id(&self) -> String {
        self.text("connection", "id").unwrap_or_default()
    }

    pub fn uuid(&self) -> String {
        self.text("connection", "uuid").unwrap_or_default()
    }

    /// The network's name, from its SSID's bytes.
    pub fn ssid(&self) -> Option<String> {
        let value = self.connection.get("802-11-wireless")?.get("ssid")?;
        let bytes = Vec::<u8>::try_from(value.try_clone().ok()?).ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn key_mgmt(&self) -> Option<String> {
        self.text("802-11-wireless-security", "key-mgmt")
    }

    pub fn eap(&self) -> Vec<String> {
        self.connection
            .get("802-1x")
            .and_then(|s| s.get("eap"))
            .and_then(|v| Vec::<String>::try_from(v.try_clone().ok()?).ok())
            .unwrap_or_default()
    }

    /// The VPN's plugin service and its data.
    pub fn vpn(&self) -> Option<(String, HashMap<String, String>)> {
        let vpn = self.connection.get("vpn")?;
        let service = String::try_from(vpn.get("service-type")?.try_clone().ok()?).ok()?;
        let data = vpn
            .get("data")
            .and_then(|v| HashMap::<String, String>::try_from(v.try_clone().ok()?).ok())
            .unwrap_or_default();
        Some((service, data))
    }

    pub fn interactive(&self) -> bool {
        self.flags & ALLOW_INTERACTION != 0
    }
}

/// The agent's answer.
pub enum Answer {
    Secrets(HashMap<String, HashMap<String, Value<'static>>>),
    /// Nothing to give without asking, or nothing asked.
    NoSecrets,
    UserCanceled,
}

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.NetworkManager.SecretAgent")]
pub enum Error {
    #[zbus(error)]
    ZBus(zbus::Error),
    NoSecrets(String),
    UserCanceled(String),
    AgentCanceled(String),
}

type Handler = Arc<
    dyn Fn(Request, watch::Receiver<bool>) -> Pin<Box<dyn Future<Output = Answer> + Send>>
        + Send
        + Sync,
>;

type Saver = Arc<dyn Fn(Settings) + Send + Sync>;

struct Agent {
    handler: Handler,
    save: Saver,
    /// Requests in flight, by connection path and setting.
    cancels: Arc<Mutex<HashMap<(String, String), watch::Sender<bool>>>>,
}

#[interface(name = "org.freedesktop.NetworkManager.SecretAgent")]
impl Agent {
    async fn get_secrets(
        &self,
        connection: Settings,
        connection_path: OwnedObjectPath,
        setting_name: String,
        hints: Vec<String>,
        flags: u32,
    ) -> Result<HashMap<String, HashMap<String, Value<'static>>>, Error> {
        let key = (connection_path.to_string(), setting_name.clone());
        let (cancel, cancelled) = watch::channel(false);
        // A second request for the same setting replaces the first.
        if let Some(old) = self.cancels.lock().unwrap().insert(key.clone(), cancel) {
            let _ = old.send(true);
        }
        info!(
            setting = setting_name,
            flags, "NetworkManager asks for secrets"
        );
        let request = Request {
            connection,
            path: connection_path.to_string(),
            setting: setting_name,
            hints,
            flags,
        };
        let answer = (self.handler)(request, cancelled).await;
        self.cancels.lock().unwrap().remove(&key);
        match answer {
            Answer::Secrets(secrets) => Ok(secrets),
            Answer::NoSecrets => Err(Error::NoSecrets("No secrets to give".into())),
            Answer::UserCanceled => Err(Error::UserCanceled("The user cancelled".into())),
        }
    }

    async fn cancel_get_secrets(&self, connection_path: OwnedObjectPath, setting_name: String) {
        let key = (connection_path.to_string(), setting_name);
        if let Some(cancel) = self.cancels.lock().unwrap().remove(&key) {
            let _ = cancel.send(true);
        }
    }

    /// Secrets the agent owns are kept in the user's keyring, as GNOME's tools keep them;
    /// Isle keeps none itself.
    async fn save_secrets(&self, connection: Settings, _connection_path: OwnedObjectPath) {
        (self.save)(connection);
    }

    async fn delete_secrets(&self, _connection: Settings, _connection_path: OwnedObjectPath) {}
}

/// Serves the agent on `conn` and registers it with NetworkManager; `save` keeps the
/// agent-owned secrets of a connection NetworkManager hands back.
pub async fn serve<F, Fut>(
    conn: &Connection,
    handler: F,
    save: impl Fn(Settings) + Send + Sync + 'static,
) -> zbus::Result<()>
where
    F: Fn(Request, watch::Receiver<bool>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Answer> + Send + 'static,
{
    let handler: Handler =
        Arc::new(move |request, cancelled| Box::pin(handler(request, cancelled)));
    conn.object_server()
        .at(
            AGENT_PATH,
            Agent {
                handler,
                save: Arc::new(save),
                cancels: Default::default(),
            },
        )
        .await?;
    conn.call_method(
        Some("org.freedesktop.NetworkManager"),
        "/org/freedesktop/NetworkManager/AgentManager",
        Some("org.freedesktop.NetworkManager.AgentManager"),
        "RegisterWithCapabilities",
        &(IDENTIFIER, VPN_HINTS),
    )
    .await?;
    debug!("the secret agent is registered");
    Ok(())
}

/// One setting's secrets for a reply.
pub fn secrets(setting: &str, items: Vec<(&str, Value<'static>)>) -> Answer {
    let items: HashMap<String, Value<'static>> =
        items.into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
    Answer::Secrets(HashMap::from([(setting.to_owned(), items)]))
}

/// The keyring's schema and attributes for NetworkManager's secrets, as nm-applet and GNOME
/// Shell keep them.
pub mod keyring {
    use std::collections::HashMap;

    use zbus::{
        Connection,
        zvariant::{OwnedObjectPath, OwnedValue, Value},
    };

    const SECRETS: &str = "org.freedesktop.secrets";
    const SERVICE: &str = "/org/freedesktop/secrets";
    const SCHEMA: &str = "org.freedesktop.NetworkManager.Connection";

    /// A call to the keyring that never starts one: it runs from login, and an agent must
    /// not launch it.
    async fn call<B, R>(
        bus: &Connection,
        path: &str,
        interface: &str,
        method: &str,
        body: &B,
    ) -> zbus::Result<R>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
        R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
    {
        let proxy = zbus::Proxy::new(bus, SECRETS, path.to_owned(), interface.to_owned()).await?;
        proxy
            .call_with_flags(method, zbus::proxy::MethodFlags::NoAutoStart.into(), body)
            .await?
            .ok_or_else(|| zbus::Error::Failure("the keyring did not answer".into()))
    }

    async fn plain_session(bus: &Connection) -> zbus::Result<OwnedObjectPath> {
        let (_, session): (OwnedValue, OwnedObjectPath) = call(
            bus,
            SERVICE,
            "org.freedesktop.Secret.Service",
            "OpenSession",
            &("plain", Value::from("")),
        )
        .await?;
        Ok(session)
    }

    /// The secrets the keyring holds for a connection's setting, by key; none when it is
    /// locked or has none.
    pub async fn lookup(uuid: &str, setting: &str) -> zbus::Result<HashMap<String, String>> {
        let bus = Connection::session().await?;
        let attributes = HashMap::from([("connection-uuid", uuid), ("setting-name", setting)]);
        let (unlocked, _): (Vec<OwnedObjectPath>, Vec<OwnedObjectPath>) = call(
            &bus,
            SERVICE,
            "org.freedesktop.Secret.Service",
            "SearchItems",
            &(attributes,),
        )
        .await?;
        if unlocked.is_empty() {
            return Ok(HashMap::new());
        }
        let session = plain_session(&bus).await?;
        let secrets: HashMap<OwnedObjectPath, (OwnedObjectPath, Vec<u8>, Vec<u8>, String)> = call(
            &bus,
            SERVICE,
            "org.freedesktop.Secret.Service",
            "GetSecrets",
            &(&unlocked, &session),
        )
        .await?;
        let mut found = HashMap::new();
        for (item, (_, _, value, _)) in secrets {
            let proxy =
                zbus::Proxy::new(&bus, SECRETS, item, "org.freedesktop.Secret.Item").await?;
            let attributes: HashMap<String, String> = proxy.get_property("Attributes").await?;
            if let Some(key) = attributes.get("setting-key") {
                found.insert(key.clone(), String::from_utf8_lossy(&value).into_owned());
            }
        }
        let _ = bus
            .call_method(
                Some(SECRETS),
                session.as_str(),
                Some("org.freedesktop.Secret.Session"),
                "Close",
                &(),
            )
            .await;
        Ok(found)
    }

    /// Keeps a setting's secret in the default keyring, replacing what was kept.
    pub async fn save(
        id: &str,
        uuid: &str,
        setting: &str,
        key: &str,
        secret: &str,
    ) -> zbus::Result<()> {
        let bus = Connection::session().await?;
        let session = plain_session(&bus).await?;
        let attributes = HashMap::from([
            ("connection-uuid", uuid.to_owned()),
            ("setting-name", setting.to_owned()),
            ("setting-key", key.to_owned()),
            ("xdg:schema", SCHEMA.to_owned()),
        ]);
        let mut properties: HashMap<&str, Value<'_>> = HashMap::new();
        properties.insert(
            "org.freedesktop.Secret.Item.Label",
            Value::from(format!("Network secret for {id}/{setting}/{key}")),
        );
        properties.insert(
            "org.freedesktop.Secret.Item.Attributes",
            Value::from(attributes),
        );
        let value = (
            &session,
            Vec::<u8>::new(),
            secret.as_bytes().to_vec(),
            "text/plain",
        );
        let _: (OwnedObjectPath, OwnedObjectPath) = call(
            &bus,
            "/org/freedesktop/secrets/aliases/default",
            "org.freedesktop.Secret.Collection",
            "CreateItem",
            &(properties, value, true),
        )
        .await?;
        let _ = bus
            .call_method(
                Some(SECRETS),
                session.as_str(),
                Some("org.freedesktop.Secret.Session"),
                "Close",
                &(),
            )
            .await;
        Ok(())
    }
}
