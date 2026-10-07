//! `org.freedesktop.UPower` with its devices, and power-profiles-daemon.

use std::collections::HashMap;

use zbus::{
    Connection,
    object_server::SignalEmitter,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

const PATH: &str = "/org/freedesktop/UPower";
const PROFILES_PATH: &str = "/org/freedesktop/UPower/PowerProfiles";

/// A battery as UPower reports it.
#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    /// UPower's type: 2 battery, 5 mouse, 6 keyboard, 12 gaming input, 17 headset.
    pub kind: u32,
    pub power_supply: bool,
    pub model: String,
    pub percentage: f64,
    /// 1 charging, 2 discharging, 4 fully charged.
    pub state: u32,
}

impl Device {
    fn path(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from(format!("{PATH}/devices/{}", self.id)).unwrap()
    }
}

struct Manager {
    on_battery: bool,
    devices: Vec<OwnedObjectPath>,
}

#[zbus::interface(name = "org.freedesktop.UPower")]
impl Manager {
    fn enumerate_devices(&self) -> Vec<OwnedObjectPath> {
        self.devices.clone()
    }

    #[zbus(property)]
    fn on_battery(&self) -> bool {
        self.on_battery
    }

    #[zbus(signal)]
    async fn device_added(emitter: &SignalEmitter<'_>, path: OwnedObjectPath) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn device_removed(emitter: &SignalEmitter<'_>, path: OwnedObjectPath)
    -> zbus::Result<()>;
}

struct DeviceIface(Device);

#[zbus::interface(name = "org.freedesktop.UPower.Device")]
impl DeviceIface {
    #[zbus(property, name = "Type")]
    fn kind(&self) -> u32 {
        self.0.kind
    }
    #[zbus(property)]
    fn power_supply(&self) -> bool {
        self.0.power_supply
    }
    #[zbus(property)]
    fn model(&self) -> String {
        self.0.model.clone()
    }
    #[zbus(property)]
    fn percentage(&self) -> f64 {
        self.0.percentage
    }
    #[zbus(property)]
    fn state(&self) -> u32 {
        self.0.state
    }
    #[zbus(property)]
    fn is_present(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn battery_level(&self) -> u32 {
        1
    }
    #[zbus(property)]
    fn time_to_empty(&self) -> i64 {
        if self.0.state == 2 { 3600 } else { 0 }
    }
}

/// UPower on the bus at an address.
pub struct UPower {
    address: String,
    on_battery: bool,
    devices: Vec<Device>,
    conn: Option<Connection>,
}

impl UPower {
    pub async fn start(address: &str, devices: Vec<Device>) -> zbus::Result<Self> {
        let mut fake = Self {
            address: address.to_owned(),
            on_battery: false,
            devices,
            conn: None,
        };
        fake.restore().await?;
        Ok(fake)
    }

    pub async fn vanish(&mut self) {
        if let Some(conn) = self.conn.take() {
            let _ = conn.close().await;
        }
    }

    pub async fn restore(&mut self) -> zbus::Result<()> {
        let mut builder = zbus::connection::Builder::address(self.address.as_str())?.serve_at(
            PATH,
            Manager {
                on_battery: self.on_battery,
                devices: self.devices.iter().map(Device::path).collect(),
            },
        )?;
        for device in &self.devices {
            builder = builder.serve_at(device.path(), DeviceIface(device.clone()))?;
        }
        self.conn = Some(builder.name("org.freedesktop.UPower")?.build().await?);
        Ok(())
    }

    fn conn(&self) -> &Connection {
        self.conn.as_ref().expect("the fake UPower is on the bus")
    }

    pub async fn set_on_battery(&mut self, on: bool) -> zbus::Result<()> {
        self.on_battery = on;
        let iface = self
            .conn()
            .object_server()
            .interface::<_, Manager>(PATH)
            .await?;
        iface.get_mut().await.on_battery = on;
        iface
            .get()
            .await
            .on_battery_changed(iface.signal_emitter())
            .await
    }

    pub async fn set_percentage(&mut self, id: &str, percentage: f64) -> zbus::Result<()> {
        let device = self
            .devices
            .iter_mut()
            .find(|d| d.id == id)
            .expect("a device");
        device.percentage = percentage;
        let path = device.path();
        let iface = self
            .conn()
            .object_server()
            .interface::<_, DeviceIface>(path)
            .await?;
        iface.get_mut().await.0.percentage = percentage;
        iface
            .get()
            .await
            .percentage_changed(iface.signal_emitter())
            .await
    }

    pub async fn add(&mut self, device: Device) -> zbus::Result<()> {
        let path = device.path();
        self.conn()
            .object_server()
            .at(path.clone(), DeviceIface(device.clone()))
            .await?;
        self.devices.push(device);
        let iface = self
            .conn()
            .object_server()
            .interface::<_, Manager>(PATH)
            .await?;
        iface.get_mut().await.devices.push(path.clone());
        Manager::device_added(iface.signal_emitter(), path).await
    }
}

struct Profiles {
    active: String,
    holds: Vec<(u32, String, String, String)>,
    before: String,
    next: u32,
}

impl Profiles {
    fn maps(rows: Vec<Vec<(&str, String)>>) -> Vec<HashMap<String, OwnedValue>> {
        rows.into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(k, v)| (k.to_owned(), OwnedValue::try_from(Value::from(v)).unwrap()))
                    .collect()
            })
            .collect()
    }
}

#[zbus::interface(name = "org.freedesktop.UPower.PowerProfiles")]
impl Profiles {
    #[zbus(property)]
    fn active_profile(&self) -> String {
        self.active.clone()
    }

    #[zbus(property)]
    async fn set_active_profile(
        &mut self,
        profile: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.active = profile;
        // A choice of the user's ends every hold.
        let released: Vec<u32> = self.holds.drain(..).map(|h| h.0).collect();
        for cookie in released {
            let _ = Self::profile_released(&emitter, cookie).await;
        }
        let _ = self.active_profile_holds_changed(&emitter).await;
    }

    #[zbus(property)]
    fn profiles(&self) -> Vec<HashMap<String, OwnedValue>> {
        Self::maps(
            ["power-saver", "balanced", "performance"]
                .iter()
                .map(|p| vec![("Profile", p.to_string()), ("Driver", "placeholder".into())])
                .collect(),
        )
    }

    #[zbus(property)]
    fn active_profile_holds(&self) -> Vec<HashMap<String, OwnedValue>> {
        Self::maps(
            self.holds
                .iter()
                .map(|(_, profile, reason, app)| {
                    vec![
                        ("Profile", profile.clone()),
                        ("Reason", reason.clone()),
                        ("ApplicationId", app.clone()),
                    ]
                })
                .collect(),
        )
    }

    async fn hold_profile(
        &mut self,
        profile: String,
        reason: String,
        app: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> u32 {
        self.next += 1;
        if self.holds.is_empty() {
            self.before = self.active.clone();
        }
        self.holds.push((self.next, profile.clone(), reason, app));
        self.active = profile;
        let _ = self.active_profile_changed(&emitter).await;
        let _ = self.active_profile_holds_changed(&emitter).await;
        self.next
    }

    async fn release_profile(
        &mut self,
        cookie: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.holds.retain(|h| h.0 != cookie);
        if self.holds.is_empty() {
            self.active = self.before.clone();
        }
        let _ = self.active_profile_changed(&emitter).await;
        let _ = self.active_profile_holds_changed(&emitter).await;
    }

    #[zbus(signal)]
    async fn profile_released(emitter: &SignalEmitter<'_>, cookie: u32) -> zbus::Result<()>;
}

/// power-profiles-daemon on the bus at an address, starting on `balanced`.
pub struct PowerProfiles {
    conn: Connection,
}

impl PowerProfiles {
    pub async fn start(address: &str) -> zbus::Result<Self> {
        let conn = zbus::connection::Builder::address(address)?
            .serve_at(
                PROFILES_PATH,
                Profiles {
                    active: "balanced".into(),
                    holds: Vec::new(),
                    before: "balanced".into(),
                    next: 0,
                },
            )?
            .name("org.freedesktop.UPower.PowerProfiles")?
            .build()
            .await?;
        Ok(Self { conn })
    }

    pub async fn active(&self) -> String {
        let iface = self
            .conn
            .object_server()
            .interface::<_, Profiles>(PROFILES_PATH)
            .await
            .unwrap();
        iface.get().await.active.clone()
    }

    pub async fn holds(&self) -> usize {
        let iface = self
            .conn
            .object_server()
            .interface::<_, Profiles>(PROFILES_PATH)
            .await
            .unwrap();
        iface.get().await.holds.len()
    }
}
