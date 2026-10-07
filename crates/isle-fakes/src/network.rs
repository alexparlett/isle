//! `org.freedesktop.NetworkManager`: a wired device, a Wi-Fi device with networks in range,
//! saved connections, and VPNs, enough for what Isle reads and asks.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use zbus::{
    Connection,
    object_server::SignalEmitter,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value},
};

const NM: &str = "/org/freedesktop/NetworkManager";
const WIRED: &str = "/org/freedesktop/NetworkManager/Devices/1";
const WIFI: &str = "/org/freedesktop/NetworkManager/Devices/2";

/// A network in range.
#[derive(Clone, Debug)]
pub struct Ap {
    pub ssid: String,
    pub strength: u8,
    pub secured: bool,
}

/// A saved connection: Wi-Fi by its SSID, or a VPN.
#[derive(Clone, Debug)]
pub enum Saved {
    Wifi(String),
    Vpn(String),
}

#[derive(Default)]
struct Model {
    wifi_enabled: bool,
    wired_state: u32,
    wifi_state: u32,
    aps: Vec<Ap>,
    current: Option<usize>,
    saved: Vec<Saved>,
    /// Active connections by saved index.
    active: Vec<usize>,
    scans: u32,
}

type Shared = Arc<Mutex<Model>>;

fn op(path: String) -> OwnedObjectPath {
    OwnedObjectPath::try_from(path).unwrap()
}

fn ap_path(i: usize) -> OwnedObjectPath {
    op(format!("{NM}/AccessPoint/{i}"))
}

fn saved_path(i: usize) -> OwnedObjectPath {
    op(format!("{NM}/Settings/{i}"))
}

fn active_path(i: usize) -> OwnedObjectPath {
    op(format!("{NM}/ActiveConnection/{i}"))
}

fn index(path: &ObjectPath<'_>) -> Option<usize> {
    path.as_str().rsplit('/').next()?.parse().ok()
}

struct Manager(Shared);

#[zbus::interface(name = "org.freedesktop.NetworkManager")]
impl Manager {
    #[zbus(property)]
    fn wireless_enabled(&self) -> bool {
        self.0.lock().unwrap().wifi_enabled
    }

    #[zbus(property)]
    async fn set_wireless_enabled(&mut self, on: bool) {
        let mut m = self.0.lock().unwrap();
        m.wifi_enabled = on;
        m.wifi_state = if on { 30 } else { 20 };
        if !on {
            m.current = None;
        }
    }

    #[zbus(property)]
    fn devices(&self) -> Vec<OwnedObjectPath> {
        vec![op(WIRED.into()), op(WIFI.into())]
    }

    #[zbus(property)]
    fn active_connections(&self) -> Vec<OwnedObjectPath> {
        self.0
            .lock()
            .unwrap()
            .active
            .iter()
            .map(|i| active_path(*i))
            .collect()
    }

    async fn activate_connection(
        &mut self,
        connection: ObjectPath<'_>,
        _device: ObjectPath<'_>,
        _object: ObjectPath<'_>,
        #[zbus(object_server)] server: &zbus::ObjectServer,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let i = index(&connection).ok_or(zbus::fdo::Error::InvalidArgs("no such".into()))?;
        let saved = self.0.lock().unwrap().saved.get(i).cloned();
        match saved {
            Some(Saved::Wifi(ssid)) => {
                let mut m = self.0.lock().unwrap();
                m.current = m.aps.iter().position(|a| a.ssid == ssid);
                m.wifi_state = 100;
                let saved = m.saved.clone();
                m.active
                    .retain(|a| !matches!(saved.get(*a), Some(Saved::Wifi(_))));
                m.active.push(i);
                drop(m);
            }
            Some(Saved::Vpn(_)) => self.0.lock().unwrap().active.push(i),
            None => return Err(zbus::fdo::Error::InvalidArgs("no such".into())),
        }
        server
            .at(active_path(i), Active(i))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let _ = Self::state_changed(&emitter, 70).await;
        Ok(active_path(i))
    }

    async fn add_and_activate_connection2(
        &mut self,
        _settings: HashMap<String, HashMap<String, OwnedValue>>,
        _device: ObjectPath<'_>,
        object: ObjectPath<'_>,
        _options: HashMap<String, OwnedValue>,
        #[zbus(object_server)] server: &zbus::ObjectServer,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<(
        OwnedObjectPath,
        OwnedObjectPath,
        HashMap<String, OwnedValue>,
    )> {
        let ap = index(&object).ok_or(zbus::fdo::Error::InvalidArgs("no such".into()))?;
        let i = {
            let mut m = self.0.lock().unwrap();
            let network = m
                .aps
                .get(ap)
                .cloned()
                .ok_or(zbus::fdo::Error::InvalidArgs("no such".into()))?;
            if network.secured {
                // With no secret agent, NetworkManager has no way to ask for the password.
                return Err(zbus::fdo::Error::Failed(
                    "Secrets were required, but not provided".into(),
                ));
            }
            m.saved.push(Saved::Wifi(network.ssid.clone()));
            let i = m.saved.len() - 1;
            m.current = Some(ap);
            m.wifi_state = 100;
            m.active.push(i);
            i
        };
        let _ = server
            .at(saved_path(i), SavedIface(self.0.clone(), i))
            .await;
        let _ = server.at(active_path(i), Active(i)).await;
        let _ = Self::state_changed(&emitter, 70).await;
        Ok((saved_path(i), active_path(i), HashMap::new()))
    }

    async fn deactivate_connection(
        &mut self,
        active: ObjectPath<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        if let Some(i) = index(&active) {
            let mut m = self.0.lock().unwrap();
            m.active.retain(|a| *a != i);
            if matches!(m.saved.get(i), Some(Saved::Wifi(_))) {
                m.current = None;
                m.wifi_state = 30;
            }
        };
        let _ = Self::state_changed(&emitter, 70).await;
    }

    #[zbus(signal)]
    async fn state_changed(emitter: &SignalEmitter<'_>, state: u32) -> zbus::Result<()>;
}

struct Settings(Shared);

#[zbus::interface(name = "org.freedesktop.NetworkManager.Settings")]
impl Settings {
    fn list_connections(&self) -> Vec<OwnedObjectPath> {
        (0..self.0.lock().unwrap().saved.len())
            .map(saved_path)
            .collect()
    }
}

struct SavedIface(Shared, usize);

#[zbus::interface(name = "org.freedesktop.NetworkManager.Settings.Connection")]
impl SavedIface {
    fn get_settings(&self) -> HashMap<String, HashMap<String, OwnedValue>> {
        let saved = self.0.lock().unwrap().saved[self.1].clone();
        let value = |v: Value<'_>| OwnedValue::try_from(v).unwrap();
        let (kind, id, wireless) = match saved {
            Saved::Wifi(ssid) => (
                "802-11-wireless",
                ssid.clone(),
                Some(HashMap::from([(
                    "ssid".to_owned(),
                    value(Value::from(ssid.into_bytes())),
                )])),
            ),
            Saved::Vpn(name) => ("vpn", name, None),
        };
        let mut out = HashMap::from([(
            "connection".to_owned(),
            HashMap::from([
                ("type".to_owned(), value(Value::from(kind))),
                ("id".to_owned(), value(Value::from(id))),
            ]),
        )]);
        if let Some(wireless) = wireless {
            out.insert("802-11-wireless".to_owned(), wireless);
        }
        out
    }
}

struct Device(Shared, bool);

#[zbus::interface(name = "org.freedesktop.NetworkManager.Device")]
impl Device {
    #[zbus(property)]
    fn device_type(&self) -> u32 {
        if self.1 { 2 } else { 1 }
    }

    #[zbus(property)]
    fn state(&self) -> u32 {
        let m = self.0.lock().unwrap();
        if self.1 { m.wifi_state } else { m.wired_state }
    }

    async fn disconnect(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        if self.1 {
            let mut m = self.0.lock().unwrap();
            m.current = None;
            m.wifi_state = 30;
            let saved = m.saved.clone();
            m.active
                .retain(|a| !matches!(saved.get(*a), Some(Saved::Wifi(_))));
        };
        let _ = Manager::state_changed(&emitter, 70).await;
    }
}

struct Wireless(Shared);

#[zbus::interface(name = "org.freedesktop.NetworkManager.Device.Wireless")]
impl Wireless {
    #[zbus(property)]
    fn access_points(&self) -> Vec<OwnedObjectPath> {
        let m = self.0.lock().unwrap();
        if !m.wifi_enabled {
            return Vec::new();
        }
        (0..m.aps.len()).map(ap_path).collect()
    }

    #[zbus(property)]
    fn active_access_point(&self) -> OwnedObjectPath {
        self.0
            .lock()
            .unwrap()
            .current
            .map_or_else(|| op("/".into()), ap_path)
    }

    fn request_scan(&mut self, _options: HashMap<String, OwnedValue>) {
        self.0.lock().unwrap().scans += 1;
    }
}

struct AccessPoint(Shared, usize);

#[zbus::interface(name = "org.freedesktop.NetworkManager.AccessPoint")]
impl AccessPoint {
    #[zbus(property)]
    fn ssid(&self) -> Vec<u8> {
        self.0.lock().unwrap().aps[self.1].ssid.clone().into_bytes()
    }
    #[zbus(property)]
    fn strength(&self) -> u8 {
        self.0.lock().unwrap().aps[self.1].strength
    }
    #[zbus(property)]
    fn flags(&self) -> u32 {
        u32::from(self.0.lock().unwrap().aps[self.1].secured)
    }
    #[zbus(property)]
    fn rsn_flags(&self) -> u32 {
        if self.0.lock().unwrap().aps[self.1].secured {
            0x188
        } else {
            0
        }
    }
    #[zbus(property)]
    fn wpa_flags(&self) -> u32 {
        0
    }
}

struct Active(usize);

#[zbus::interface(name = "org.freedesktop.NetworkManager.Connection.Active")]
impl Active {
    #[zbus(property)]
    fn state(&self) -> u32 {
        2
    }
    #[zbus(property)]
    fn connection(&self) -> OwnedObjectPath {
        saved_path(self.0)
    }
}

/// NetworkManager on the bus at an address.
pub struct NetworkManager {
    model: Shared,
    conn: Connection,
}

impl NetworkManager {
    /// Wi-Fi on and joined to the first known network in range, if any; a wire unplugged.
    pub async fn start(address: &str, aps: Vec<Ap>, saved: Vec<Saved>) -> zbus::Result<Self> {
        let current = aps.iter().position(|a| {
            saved
                .iter()
                .any(|s| matches!(s, Saved::Wifi(ssid) if *ssid == a.ssid))
        });
        let active: Vec<usize> = current
            .and_then(|c| {
                saved
                    .iter()
                    .position(|s| matches!(s, Saved::Wifi(ssid) if *ssid == aps[c].ssid))
            })
            .into_iter()
            .collect();
        let model: Shared = Arc::new(Mutex::new(Model {
            wifi_enabled: true,
            wired_state: 20,
            wifi_state: if current.is_some() { 100 } else { 30 },
            aps,
            current,
            saved,
            active,
            scans: 0,
        }));
        let m = model.lock().unwrap();
        let mut builder = zbus::connection::Builder::address(address)?
            .serve_at(NM, Manager(model.clone()))?
            .serve_at(format!("{NM}/Settings"), Settings(model.clone()))?
            .serve_at(WIRED, Device(model.clone(), false))?
            .serve_at(WIFI, Device(model.clone(), true))?
            .serve_at(WIFI, Wireless(model.clone()))?;
        for i in 0..m.aps.len() {
            builder = builder.serve_at(ap_path(i), AccessPoint(model.clone(), i))?;
        }
        for i in 0..m.saved.len() {
            builder = builder.serve_at(saved_path(i), SavedIface(model.clone(), i))?;
        }
        for i in &m.active {
            builder = builder.serve_at(active_path(*i), Active(*i))?;
        }
        drop(m);
        let conn = builder
            .name("org.freedesktop.NetworkManager")?
            .build()
            .await?;
        Ok(Self { model, conn })
    }

    /// Announces a change, as NetworkManager's signals do, so readers look again.
    pub async fn changed(&self) -> zbus::Result<()> {
        let iface = self
            .conn
            .object_server()
            .interface::<_, Manager>(NM)
            .await?;
        Manager::state_changed(iface.signal_emitter(), 70).await
    }

    pub fn scans(&self) -> u32 {
        self.model.lock().unwrap().scans
    }

    pub fn current(&self) -> Option<String> {
        let m = self.model.lock().unwrap();
        m.current.map(|c| m.aps[c].ssid.clone())
    }

    pub fn wifi_enabled(&self) -> bool {
        self.model.lock().unwrap().wifi_enabled
    }

    pub fn vpn_active(&self) -> bool {
        let m = self.model.lock().unwrap();
        m.active
            .iter()
            .any(|a| matches!(m.saved.get(*a), Some(Saved::Vpn(_))))
    }

    pub async fn plug_cable(&self) -> zbus::Result<()> {
        self.model.lock().unwrap().wired_state = 100;
        self.changed().await
    }

    pub async fn vanish(self) {
        let _ = self.conn.close().await;
    }
}
