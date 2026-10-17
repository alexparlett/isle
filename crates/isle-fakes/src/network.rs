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
    /// The secret agent's unique name, once one registers.
    agent: Option<String>,
    /// What a secured network takes.
    password: String,
    /// Saved connections deleted, by index.
    deleted: Vec<usize>,
    /// Saved connections set not to join by themselves, by index.
    manual: Vec<usize>,
    /// The settings each Update carried, by index, secrets included.
    updates: Vec<(usize, HashMap<String, HashMap<String, OwnedValue>>)>,
}

/// The settings a secured network's request carries, as NetworkManager sends them.
fn wifi_settings(ssid: &str) -> HashMap<String, HashMap<String, OwnedValue>> {
    let mut connection = HashMap::new();
    connection.insert("id".to_owned(), OwnedValue::from(zbus::zvariant::Str::from(ssid.to_owned())));
    connection.insert("uuid".to_owned(), OwnedValue::from(zbus::zvariant::Str::from("0f3e-test")));
    connection.insert("type".to_owned(), OwnedValue::from(zbus::zvariant::Str::from("802-11-wireless")));
    let mut wireless = HashMap::new();
    wireless.insert(
        "ssid".to_owned(),
        OwnedValue::try_from(Value::from(ssid.as_bytes().to_vec())).unwrap(),
    );
    let mut security = HashMap::new();
    security.insert("key-mgmt".to_owned(), OwnedValue::from(zbus::zvariant::Str::from("wpa-psk")));
    HashMap::from([
        ("connection".to_owned(), connection),
        ("802-11-wireless".to_owned(), wireless),
        ("802-11-wireless-security".to_owned(), security),
    ])
}

/// Asks the agent `agent` for a setting's secrets, as NetworkManager does.
async fn get_secrets(
    conn: &Connection,
    agent: &str,
    settings: HashMap<String, HashMap<String, OwnedValue>>,
    setting: &str,
    hints: Vec<String>,
    flags: u32,
) -> Result<HashMap<String, HashMap<String, OwnedValue>>, String> {
    let path = ObjectPath::try_from(format!("{NM}/Settings/99")).unwrap();
    let reply = conn
        .call_method(
            Some(agent),
            "/org/freedesktop/NetworkManager/SecretAgent",
            Some("org.freedesktop.NetworkManager.SecretAgent"),
            "GetSecrets",
            &(settings, path, setting, hints, flags),
        )
        .await
        .map_err(|e| e.to_string())?;
    reply.body().deserialize().map_err(|e| e.to_string())
}

struct AgentManager(Shared);

#[zbus::interface(name = "org.freedesktop.NetworkManager.AgentManager")]
impl AgentManager {
    fn register_with_capabilities(
        &self,
        _identifier: String,
        _capabilities: u32,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) {
        self.0.lock().unwrap().agent = header.sender().map(|s| s.to_string());
    }

    fn unregister(&self) {
        self.0.lock().unwrap().agent = None;
    }
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
        #[zbus(connection)] conn: &Connection,
        _options: HashMap<String, OwnedValue>,
        #[zbus(object_server)] server: &zbus::ObjectServer,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<(
        OwnedObjectPath,
        OwnedObjectPath,
        HashMap<String, OwnedValue>,
    )> {
        let ap = index(&object).ok_or(zbus::fdo::Error::InvalidArgs("no such".into()))?;
        // A secured network's password comes from the agent, asked again once if wrong.
        let (secured, agent, ssid, password) = {
            let m = self.0.lock().unwrap();
            let network = m.aps.get(ap).cloned();
            (
                network.as_ref().is_some_and(|n| n.secured),
                m.agent.clone(),
                network.map(|n| n.ssid).unwrap_or_default(),
                m.password.clone(),
            )
        };
        let mut joined = !secured;
        if let (true, Some(agent)) = (secured, agent) {
            for flags in [0x1 | 0x4, 0x1 | 0x2 | 0x4] {
                let secrets = get_secrets(
                    conn,
                    &agent,
                    wifi_settings(&ssid),
                    "802-11-wireless-security",
                    Vec::new(),
                    flags,
                )
                .await;
                let given = secrets
                    .ok()
                    .and_then(|s| s.get("802-11-wireless-security")?.get("psk").cloned())
                    .and_then(|v| String::try_from(v).ok());
                match given {
                    Some(given) if given == password => {
                        joined = true;
                        break;
                    }
                    Some(_) => continue,
                    None => break,
                }
            }
        }
        let i = {
            let mut m = self.0.lock().unwrap();
            let network = m
                .aps
                .get(ap)
                .cloned()
                .ok_or(zbus::fdo::Error::InvalidArgs("no such".into()))?;
            if !joined {
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
        let m = self.0.lock().unwrap();
        (0..m.saved.len())
            .filter(|i| !m.deleted.contains(i))
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
        let manual = self.0.lock().unwrap().manual.contains(&self.1);
        let mut out = HashMap::from([(
            "connection".to_owned(),
            HashMap::from([
                ("type".to_owned(), value(Value::from(kind))),
                ("id".to_owned(), value(Value::from(id))),
                ("autoconnect".to_owned(), value(Value::from(!manual))),
            ]),
        )]);
        if let Some(wireless) = wireless {
            out.insert("802-11-wireless".to_owned(), wireless);
            out.insert(
                "802-11-wireless-security".to_owned(),
                HashMap::from([("key-mgmt".to_owned(), value(Value::from("wpa-psk")))]),
            );
        }
        out
    }

    fn get_secrets(&self, setting: String) -> HashMap<String, HashMap<String, OwnedValue>> {
        let password = self.0.lock().unwrap().password.clone();
        HashMap::from([(
            setting,
            HashMap::from([("psk".to_owned(), OwnedValue::try_from(Value::from(password)).unwrap())]),
        )])
    }

    fn update(&mut self, settings: HashMap<String, HashMap<String, OwnedValue>>) {
        let mut m = self.0.lock().unwrap();
        let auto = settings
            .get("connection")
            .and_then(|c| c.get("autoconnect"))
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or(true);
        m.manual.retain(|i| *i != self.1);
        if !auto {
            m.manual.push(self.1);
        }
        m.updates.push((self.1, settings));
    }

    async fn delete(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.0.lock().unwrap().deleted.push(self.1);
        let _ = Manager::state_changed(&emitter, 70).await;
    }
}

struct Ip4Config;

#[zbus::interface(name = "org.freedesktop.NetworkManager.IP4Config")]
impl Ip4Config {
    #[zbus(property)]
    fn address_data(&self) -> Vec<HashMap<String, OwnedValue>> {
        vec![HashMap::from([
            ("address".to_owned(), OwnedValue::try_from(Value::from("192.168.1.20")).unwrap()),
            ("prefix".to_owned(), OwnedValue::from(24u32)),
        ])]
    }
    #[zbus(property)]
    fn gateway(&self) -> String {
        "192.168.1.1".into()
    }
    #[zbus(property)]
    fn nameserver_data(&self) -> Vec<HashMap<String, OwnedValue>> {
        vec![HashMap::from([(
            "address".to_owned(),
            OwnedValue::try_from(Value::from("192.168.1.1")).unwrap(),
        )])]
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

    #[zbus(property)]
    fn ip4_config(&self) -> OwnedObjectPath {
        op(format!("{NM}/IP4Config/1"))
    }

    #[zbus(property)]
    fn hw_address(&self) -> String {
        if self.1 { "aa:bb:cc:dd:ee:02" } else { "aa:bb:cc:dd:ee:01" }.into()
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
            agent: None,
            password: "password".into(),
            deleted: Vec::new(),
            manual: Vec::new(),
            updates: Vec::new(),
        }));
        let m = model.lock().unwrap();
        let mut builder = zbus::connection::Builder::address(address)?
            .serve_at(NM, Manager(model.clone()))?
            .serve_at(format!("{NM}/Settings"), Settings(model.clone()))?
            .serve_at(WIRED, Device(model.clone(), false))?
            .serve_at(WIFI, Device(model.clone(), true))?
            .serve_at(WIFI, Wireless(model.clone()))?
            .serve_at(format!("{NM}/AgentManager"), AgentManager(model.clone()))?
            .serve_at(format!("{NM}/IP4Config/1"), Ip4Config)?;
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

    /// Whether a secret agent has registered.
    pub fn agent_registered(&self) -> bool {
        self.model.lock().unwrap().agent.is_some()
    }

    /// What secured networks take; `password` until set.
    pub fn set_password(&self, password: &str) {
        self.model.lock().unwrap().password = password.to_owned();
    }

    /// Asks the agent for a setting's secrets in a connection of `settings`, as
    /// NetworkManager does; its reply, or its error's text.
    pub async fn ask_agent(
        &self,
        settings: HashMap<String, HashMap<String, OwnedValue>>,
        setting: &str,
        hints: Vec<String>,
        flags: u32,
    ) -> Result<HashMap<String, HashMap<String, OwnedValue>>, String> {
        let agent = self
            .model
            .lock()
            .unwrap()
            .agent
            .clone()
            .ok_or("no agent")?;
        get_secrets(&self.conn, &agent, settings, setting, hints, flags).await
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

    /// Whether saved connection `i` is deleted.
    pub fn deleted(&self, i: usize) -> bool {
        self.model.lock().unwrap().deleted.contains(&i)
    }

    /// The last settings saved connection `i` was updated with.
    pub fn updated(&self, i: usize) -> Option<HashMap<String, HashMap<String, OwnedValue>>> {
        let m = self.model.lock().unwrap();
        m.updates.iter().rev().find(|(at, _)| *at == i).map(|(_, s)| s.clone())
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
