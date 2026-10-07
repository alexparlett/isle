//! `org.bluez`: one adapter whose discovery finds a speaker, and the devices it starts with,
//! under an object manager as BlueZ serves them.

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

/// BlueZ on the bus at an address, its adapter on.
pub struct BlueZ {
    conn: Connection,
}

impl BlueZ {
    pub async fn start(address: &str, devices: Vec<Device>) -> zbus::Result<Self> {
        let mut builder = zbus::connection::Builder::address(address)?
            .serve_at("/", zbus::fdo::ObjectManager)?
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
        Ok(Self { conn })
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
