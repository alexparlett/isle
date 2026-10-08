//! `org.bluez`: one adapter whose discovery finds a speaker, and the devices it starts with,
//! under an object manager as BlueZ serves them.

use std::sync::{Arc, Mutex};

use zbus::{Connection, object_server::SignalEmitter};

const ADAPTER: &str = "/org/bluez/hci0";

/// A device the fake knows.
#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub paired: bool,
    pub connected: bool,
    pub battery: Option<u8>,
}

fn path(id: &str) -> String {
    format!("{ADAPTER}/dev_{id}")
}

struct Adapter {
    powered: bool,
    discovering: bool,
}

#[zbus::interface(name = "org.bluez.Adapter1")]
impl Adapter {
    #[zbus(property)]
    fn powered(&self) -> bool {
        self.powered
    }

    #[zbus(property)]
    fn set_powered(&mut self, on: bool) {
        self.powered = on;
    }

    #[zbus(property)]
    fn discovering(&self) -> bool {
        self.discovering
    }

    async fn start_discovery(
        &mut self,
        #[zbus(object_server)] server: &zbus::ObjectServer,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.discovering = true;
        let _ = self.discovering_changed(&emitter).await;
        let speaker = Device {
            id: "AA_BB_CC_00_11_22".into(),
            name: "Nearby Speaker".into(),
            icon: "audio-card".into(),
            paired: false,
            connected: false,
            battery: None,
        };
        let _ = server.at(path(&speaker.id), DeviceIface(speaker)).await;
    }

    async fn stop_discovery(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.discovering = false;
        let _ = self.discovering_changed(&emitter).await;
    }
}

struct DeviceIface(Device);

#[zbus::interface(name = "org.bluez.Device1")]
impl DeviceIface {
    #[zbus(property)]
    fn alias(&self) -> String {
        self.0.name.clone()
    }
    #[zbus(property)]
    fn icon(&self) -> String {
        self.0.icon.clone()
    }
    #[zbus(property)]
    fn paired(&self) -> bool {
        self.0.paired
    }
    #[zbus(property)]
    fn connected(&self) -> bool {
        self.0.connected
    }
    #[zbus(property)]
    fn trusted(&self) -> bool {
        self.0.paired
    }
    #[zbus(property)]
    fn set_trusted(&mut self, _on: bool) {}

    async fn connect(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.0.connected = true;
        let _ = self.connected_changed(&emitter).await;
    }

    async fn disconnect(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.0.connected = false;
        let _ = self.connected_changed(&emitter).await;
    }

    async fn pair(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.0.paired = true;
        let _ = self.paired_changed(&emitter).await;
    }
}

struct Battery(u8);

#[zbus::interface(name = "org.bluez.Battery1")]
impl Battery {
    #[zbus(property)]
    fn percentage(&self) -> u8 {
        self.0
    }
}

/// The agent registered: its owner, path, capability, and whether it is the default.
#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub sender: String,
    pub path: String,
    pub capability: String,
    pub default: bool,
}

struct AgentManager(Arc<Mutex<Option<Agent>>>);

#[zbus::interface(name = "org.bluez.AgentManager1")]
impl AgentManager {
    fn register_agent(
        &self,
        agent: zbus::zvariant::ObjectPath<'_>,
        capability: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) {
        *self.0.lock().unwrap() = Some(Agent {
            sender: header.sender().map(|s| s.to_string()).unwrap_or_default(),
            path: agent.to_string(),
            capability,
            default: false,
        });
    }

    fn request_default_agent(&self, _agent: zbus::zvariant::ObjectPath<'_>) {
        if let Some(agent) = self.0.lock().unwrap().as_mut() {
            agent.default = true;
        }
    }
}

/// BlueZ on the bus at an address, its adapter on.
pub struct BlueZ {
    conn: Connection,
    agent: Arc<Mutex<Option<Agent>>>,
}

impl BlueZ {
    pub async fn start(address: &str, devices: Vec<Device>) -> zbus::Result<Self> {
        let agent = Arc::new(Mutex::new(None));
        let mut builder = zbus::connection::Builder::address(address)?
            .serve_at("/", zbus::fdo::ObjectManager)?
            .serve_at("/org/bluez", AgentManager(agent.clone()))?
            .serve_at(
                ADAPTER,
                Adapter {
                    powered: true,
                    discovering: false,
                },
            )?;
        for device in devices {
            let p = path(&device.id);
            if let Some(level) = device.battery {
                builder = builder.serve_at(p.clone(), Battery(level))?;
            }
            builder = builder.serve_at(p, DeviceIface(device))?;
        }
        let conn = builder.name("org.bluez")?.build().await?;
        Ok(Self { conn, agent })
    }

    /// The agent registered, once one has.
    pub fn agent(&self) -> Option<Agent> {
        self.agent.lock().unwrap().clone()
    }

    /// Calls the agent's `method` about device `id` with the rest of `args`, as BlueZ does
    /// while pairing; its reply, or its error's name.
    pub async fn ask_agent<B, R>(&self, method: &str, args: B) -> Result<R, String>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
        R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
    {
        let agent = self.agent().ok_or("no agent")?;
        let proxy = zbus::Proxy::new(
            &self.conn,
            agent.sender.clone(),
            agent.path.clone(),
            "org.bluez.Agent1",
        )
        .await
        .map_err(|e| e.to_string())?;
        proxy.call(method, &args).await.map_err(|e| match e {
            zbus::Error::MethodError(name, _, _) => name.to_string(),
            other => other.to_string(),
        })
    }

    /// A device's path, for the agent's calls.
    pub fn device_path(id: &str) -> zbus::zvariant::OwnedObjectPath {
        zbus::zvariant::OwnedObjectPath::try_from(path(id)).unwrap()
    }

    async fn adapter(&self) -> zbus::object_server::InterfaceRef<Adapter> {
        self.conn
            .object_server()
            .interface::<_, Adapter>(ADAPTER)
            .await
            .unwrap()
    }

    pub async fn discovering(&self) -> bool {
        self.adapter().await.get().await.discovering
    }

    pub async fn powered(&self) -> bool {
        self.adapter().await.get().await.powered
    }

    async fn device(&self, id: &str) -> zbus::object_server::InterfaceRef<DeviceIface> {
        self.conn
            .object_server()
            .interface::<_, DeviceIface>(path(id))
            .await
            .unwrap()
    }

    pub async fn connected(&self, id: &str) -> bool {
        self.device(id).await.get().await.0.connected
    }

    pub async fn paired(&self, id: &str) -> bool {
        self.device(id).await.get().await.0.paired
    }

    pub async fn vanish(self) {
        let _ = self.conn.close().await;
    }
}
