//! Other programs' status items: Isle owns `org.kde.StatusNotifierWatcher` and is the host
//! that reads each item (`p8-services.md` choice 17). Menus are `isle-menus`' DBusMenu.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::{FutureExt, StreamExt};
use isle_bus::{Error, Props, Publisher};
use serde::Serialize;
use tokio::sync::Notify;
use zbus::{Connection, message::Header, object_server::SignalEmitter, zvariant::OwnedValue};

pub const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const ITEM: &str = "org.kde.StatusNotifierItem";

/// An item's address: its bus name and object path.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Address {
    pub service: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Icon {
    /// By name, with the item's own theme folder to look in first.
    Name(String, Option<String>),
    /// Its pixels, written into the cache.
    File(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Item {
    pub address: Address,
    pub id: String,
    pub title: String,
    /// It asks for attention.
    pub attention: bool,
    pub passive: bool,
    pub icon: Option<Icon>,
    pub tooltip: String,
    /// Its DBusMenu's object path.
    pub menu: Option<String>,
    /// A click opens its menu rather than activating it.
    pub item_is_menu: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Tray {
    pub items: Vec<Item>,
}

#[derive(Default)]
struct Registry {
    items: Mutex<Vec<Address>>,
    changed: Notify,
}

struct Watcher(Arc<Registry>);

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    async fn register_status_notifier_item(
        &self,
        service: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        // An item gives its bus name, or its object path on the sender's connection.
        let address = if service.starts_with('/') {
            Address {
                service: sender,
                path: service,
            }
        } else {
            Address {
                service,
                path: "/StatusNotifierItem".into(),
            }
        };
        let name = format!("{}{}", address.service, address.path);
        {
            let mut items = self.0.items.lock().unwrap();
            if !items.contains(&address) {
                items.push(address);
            }
        }
        self.0.changed.notify_waiters();
        let _ = Self::status_notifier_item_registered(&emitter, &name).await;
    }

    fn register_status_notifier_host(&self, _service: String) {}

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.0
            .items
            .lock()
            .unwrap()
            .iter()
            .map(|a| format!("{}{}", a.service, a.path))
            .collect()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        item: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        item: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// The name has another owner.
#[derive(Debug)]
pub struct Taken;

/// The watcher, serving: what follows its items.
pub struct Host {
    registry: Arc<Registry>,
}

/// Serves the watcher on `conn` and owns its name without queueing.
pub async fn own(conn: &Connection) -> Result<Result<Host, Taken>, Error> {
    let registry = Arc::new(Registry::default());
    conn.object_server()
        .at(WATCHER_PATH, Watcher(registry.clone()))
        .await?;
    use zbus::fdo::{RequestNameFlags, RequestNameReply};
    match conn
        .request_name_with_flags(WATCHER, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => {}
        Ok(_) | Err(zbus::Error::NameTaken) => return Ok(Err(Taken)),
        Err(err) => return Err(err.into()),
    }
    if let Ok(iface) = conn
        .object_server()
        .interface::<_, Watcher>(WATCHER_PATH)
        .await
    {
        let _ = Watcher::status_notifier_host_registered(iface.signal_emitter()).await;
    }
    Ok(Ok(Host { registry }))
}

fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("isle/tray")
}

/// The largest of an item's pixmaps up to 64 square, written out as a PNG.
fn pixmap(value: &OwnedValue, key: &str) -> Option<PathBuf> {
    let pixmaps: Vec<(i32, i32, Vec<u8>)> = value.try_clone().ok()?.try_into().ok()?;
    let (w, h, argb) = pixmaps
        .into_iter()
        .filter(|(w, h, d)| *w > 0 && *h > 0 && d.len() == (*w * *h * 4) as usize)
        .max_by_key(|(w, _, _)| if *w <= 64 { *w } else { 64 - *w })?;
    // Network byte order ARGB to RGBA.
    let rgba: Vec<u8> = argb
        .chunks_exact(4)
        .flat_map(|p| [p[1], p[2], p[3], p[0]])
        .collect();
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).ok()?;
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    rgba.hash(&mut hasher);
    let path = dir.join(format!("{:016x}.png", hasher.finish()));
    if !path.exists() {
        let file = std::io::BufWriter::new(std::fs::File::create(&path).ok()?);
        let mut encoder = png::Encoder::new(file, w as u32, h as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().ok()?.write_image_data(&rgba).ok()?;
    }
    Some(path)
}

async fn read_item(conn: &Connection, address: &Address) -> Result<Item, Error> {
    let p = Props::read(conn, &address.service, &address.path, ITEM).await?;
    let status = p.str("Status").unwrap_or_else(|| "Active".into());
    let attention = status == "NeedsAttention";
    let key = format!("{}{}", address.service, address.path);
    let (name_key, pixmap_key) = if attention {
        ("AttentionIconName", "AttentionIconPixmap")
    } else {
        ("IconName", "IconPixmap")
    };
    let icon = p
        .str(name_key)
        .or_else(|| p.str("IconName"))
        .filter(|n| !n.is_empty())
        .map(|name| Icon::Name(name, p.str("IconThemePath").filter(|t| !t.is_empty())))
        .or_else(|| {
            p.0.get(pixmap_key)
                .or_else(|| p.0.get("IconPixmap"))
                .and_then(|v| pixmap(v, &key))
                .map(Icon::File)
        });
    let tooltip =
        p.0.get("ToolTip")
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| <(String, Vec<(i32, i32, Vec<u8>)>, String, String)>::try_from(v).ok())
            .map(|(_, _, title, _)| title)
            .unwrap_or_default();
    Ok(Item {
        address: address.clone(),
        id: p.str("Id").unwrap_or_default(),
        title: p.str("Title").unwrap_or_default(),
        attention,
        passive: status == "Passive",
        icon,
        tooltip,
        menu: p.str("Menu").filter(|m| m != "/"),
        item_is_menu: p.bool("ItemIsMenu").unwrap_or(false),
    })
}

impl Host {
    /// Follows the items: re-read on registration, on their owners leaving and on their
    /// signals, a burst read once.
    pub async fn follow(&self, conn: &Connection, publisher: Publisher<Tray>) -> Result<(), Error> {
        let dbus = zbus::fdo::DBusProxy::new(conn).await?;
        let mut owners = dbus.receive_name_owner_changed().await?;
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(ITEM)?
            .build();
        let mut signals = zbus::MessageStream::for_match_rule(rule, conn, Some(64)).await?;
        loop {
            // Armed before the read, so a registration during it is not missed.
            let changed = self.registry.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let addresses = self.registry.items.lock().unwrap().clone();
            let mut items = Vec::new();
            for address in &addresses {
                match read_item(conn, address).await {
                    Ok(item) => items.push(item),
                    Err(err) => tracing::debug!(?address, "a tray item: {err}"),
                }
            }
            publisher.publish(Tray { items });
            loop {
                tokio::select! {
                    _ = &mut changed => break,
                    change = owners.next() => {
                        let Some(change) = change else { return Err("the bus ended".into()) };
                        let Ok(args) = change.args() else { continue };
                        if args.new_owner().is_some() {
                            continue;
                        }
                        let gone = args.name().to_string();
                        let removed = {
                            let mut items = self.registry.items.lock().unwrap();
                            let before = items.len();
                            items.retain(|a| a.service != gone);
                            before != items.len()
                        };
                        if removed {
                            if let Ok(iface) =
                                conn.object_server().interface::<_, Watcher>(WATCHER_PATH).await
                            {
                                let _ = Watcher::status_notifier_item_unregistered(
                                    iface.signal_emitter(),
                                    &gone,
                                )
                                .await;
                            }
                            break;
                        }
                    }
                    signal = signals.next() => {
                        if signal.is_none() {
                            return Err("the bus ended".into());
                        }
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
            while let Some(Some(_)) = signals.next().now_or_never() {}
        }
    }
}

async fn call<B>(address: &Address, method: &str, body: &B) -> Result<(), Error>
where
    B: serde::Serialize + zbus::zvariant::DynamicType,
{
    let conn = isle_bus::session().await?;
    conn.call_method(
        Some(address.service.as_str()),
        address.path.as_str(),
        Some(ITEM),
        method,
        body,
    )
    .await?;
    Ok(())
}

pub async fn activate(address: &Address, x: i32, y: i32) -> Result<(), Error> {
    call(address, "Activate", &(x, y)).await
}

pub async fn secondary_activate(address: &Address, x: i32, y: i32) -> Result<(), Error> {
    call(address, "SecondaryActivate", &(x, y)).await
}

pub async fn scroll(address: &Address, delta: i32) -> Result<(), Error> {
    call(address, "Scroll", &(delta, "vertical")).await
}

/// An icon by name from the item's own theme folder.
pub fn theme_icon(name: &str, theme_path: &str) -> Option<PathBuf> {
    let root = PathBuf::from(theme_path);
    for ext in ["svg", "png"] {
        let direct = root.join(format!("{name}.{ext}"));
        if direct.exists() {
            return Some(direct);
        }
    }
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_stem().is_some_and(|s| s == name) {
                return Some(path);
            }
        }
    }
    None
}
