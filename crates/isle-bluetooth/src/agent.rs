//! BlueZ's pairing agent: registered as the default with the KeyboardDisplay capability, so a
//! device that pairs asks here whether Settings is open or not (`lock-and-prompts-2026.md` §5).

use std::{future::Future, pin::Pin, sync::Arc};

use tokio::sync::watch;
use tracing::info;
use zbus::{Connection, interface, zvariant::OwnedObjectPath};

pub const AGENT_PATH: &str = "/io/github/alexparlett/Isle/BluetoothAgent";

/// What BlueZ asks of the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Does the device show this passkey?
    Confirm(u32),
    /// The six-digit passkey the device shows.
    Passkey,
    /// A PIN of up to sixteen characters.
    PinCode,
    /// Type this passkey on the device; `entered` digits are typed so far.
    DisplayPasskey {
        passkey: u32,
        entered: u16,
    },
    DisplayPinCode(String),
    /// May the device pair at all?
    Authorize,
    /// May the device use a service, by its UUID?
    Service(String),
}

/// The user's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Yes,
    Passkey(u32),
    PinCode(String),
    No,
}

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.bluez.Error")]
pub enum Error {
    #[zbus(error)]
    ZBus(zbus::Error),
    Rejected(String),
    Canceled(String),
}

type Handler = Arc<
    dyn Fn(String, Ask, watch::Receiver<bool>) -> Pin<Box<dyn Future<Output = Reply> + Send>>
        + Send
        + Sync,
>;

struct Agent {
    handler: Handler,
    /// BlueZ's Cancel for whatever is asked now.
    cancel: Arc<std::sync::Mutex<Option<watch::Sender<bool>>>>,
}

impl Agent {
    async fn ask(&self, device: OwnedObjectPath, ask: Ask) -> Reply {
        let (cancel, cancelled) = watch::channel(false);
        if let Some(old) = self.cancel.lock().unwrap().replace(cancel) {
            let _ = old.send(true);
        }
        info!(device = device.as_str(), ?ask, "BlueZ asks");
        (self.handler)(device.to_string(), ask, cancelled).await
    }

    fn refused() -> Error {
        Error::Rejected("The user refused".into())
    }
}

#[interface(name = "org.bluez.Agent1")]
impl Agent {
    async fn release(&self) {}

    async fn request_pin_code(&self, device: OwnedObjectPath) -> Result<String, Error> {
        match self.ask(device, Ask::PinCode).await {
            Reply::PinCode(pin) => Ok(pin),
            _ => Err(Self::refused()),
        }
    }

    async fn display_pin_code(&self, device: OwnedObjectPath, pincode: String) {
        let handler = self.handler.clone();
        let (_cancel, cancelled) = watch::channel(false);
        tokio::spawn(async move {
            handler(device.to_string(), Ask::DisplayPinCode(pincode), cancelled).await;
        });
    }

    async fn request_passkey(&self, device: OwnedObjectPath) -> Result<u32, Error> {
        match self.ask(device, Ask::Passkey).await {
            Reply::Passkey(passkey) => Ok(passkey),
            _ => Err(Self::refused()),
        }
    }

    /// Called again as each digit is typed on the device; the prompt follows.
    async fn display_passkey(&self, device: OwnedObjectPath, passkey: u32, entered: u16) {
        let handler = self.handler.clone();
        let (cancel, cancelled) = watch::channel(false);
        if entered == 0
            && let Some(old) = self.cancel.lock().unwrap().replace(cancel)
        {
            let _ = old.send(true);
        }
        tokio::spawn(async move {
            handler(
                device.to_string(),
                Ask::DisplayPasskey { passkey, entered },
                cancelled,
            )
            .await;
        });
    }

    async fn request_confirmation(
        &self,
        device: OwnedObjectPath,
        passkey: u32,
    ) -> Result<(), Error> {
        match self.ask(device, Ask::Confirm(passkey)).await {
            Reply::Yes => Ok(()),
            _ => Err(Self::refused()),
        }
    }

    async fn request_authorization(&self, device: OwnedObjectPath) -> Result<(), Error> {
        match self.ask(device, Ask::Authorize).await {
            Reply::Yes => Ok(()),
            _ => Err(Self::refused()),
        }
    }

    async fn authorize_service(&self, device: OwnedObjectPath, uuid: String) -> Result<(), Error> {
        match self.ask(device, Ask::Service(uuid)).await {
            Reply::Yes => Ok(()),
            _ => Err(Self::refused()),
        }
    }

    async fn cancel(&self) {
        if let Some(cancel) = self.cancel.lock().unwrap().take() {
            let _ = cancel.send(true);
        }
    }
}

/// Serves the agent on `conn` and makes it BlueZ's default.
pub async fn serve<F, Fut>(conn: &Connection, handler: F) -> zbus::Result<()>
where
    F: Fn(String, Ask, watch::Receiver<bool>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Reply> + Send + 'static,
{
    let handler: Handler =
        Arc::new(move |device, ask, cancelled| Box::pin(handler(device, ask, cancelled)));
    conn.object_server()
        .at(
            AGENT_PATH,
            Agent {
                handler,
                cancel: Default::default(),
            },
        )
        .await?;
    let path = zbus::zvariant::ObjectPath::try_from(AGENT_PATH)?;
    for (method, body) in [("RegisterAgent", true), ("RequestDefaultAgent", false)] {
        if body {
            conn.call_method(
                Some("org.bluez"),
                "/org/bluez",
                Some("org.bluez.AgentManager1"),
                method,
                &(&path, "KeyboardDisplay"),
            )
            .await?;
        } else {
            conn.call_method(
                Some("org.bluez"),
                "/org/bluez",
                Some("org.bluez.AgentManager1"),
                method,
                &(&path,),
            )
            .await?;
        }
    }
    info!("the Bluetooth agent is the default");
    Ok(())
}

/// A service's name for the authorise prompt, by its 16-bit UUID's assigned number.
pub fn service_name(uuid: &str) -> String {
    let short = uuid
        .get(4..8)
        .and_then(|s| u16::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    match short {
        0x110a | 0x110b | 0x110d => "Audio",
        0x110c | 0x110e | 0x110f => "Remote control",
        0x1108 | 0x111e | 0x111f | 0x1112 => "Calls",
        0x1124 | 0x1812 => "Keyboard, mouse or controller",
        0x1105 | 0x1106 => "File transfer",
        0x1115 | 0x1116 => "Network",
        0x112f => "Phone book",
        0x1132 | 0x1133 | 0x1134 => "Messages",
        _ => uuid,
    }
    .to_owned()
}
