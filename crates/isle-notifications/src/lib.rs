//! `org.freedesktop.Notifications`, spec 1.3 with KDE's inline reply and `Inhibited`: each
//! Notify parsed, its picture copied into the cache, and handed to whoever decides what
//! becomes of it; the signals that decision leads to are sent back through the server
//! (`p8-services.md` choices 6, 9, 10).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use futures_util::StreamExt;
use isle_bus::Error;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, warn};
use zbus::{
    Connection,
    message::Header,
    object_server::SignalEmitter,
    zvariant::{OwnedValue, Structure, Value},
};

pub const NAME: &str = "org.freedesktop.Notifications";
pub const PATH: &str = "/org/freedesktop/Notifications";

/// A notification as it arrived, ready to show.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    /// An icon by name, when `app_icon` names one.
    pub app_icon: Option<String>,
    pub summary: String,
    pub body: String,
    /// Key and label, in the order sent; `default` is the click, `inline-reply` the reply.
    pub actions: Vec<(String, String)>,
    /// 0 low, 1 normal, 2 critical.
    pub urgency: u8,
    pub category: Option<String>,
    pub desktop_entry: Option<String>,
    pub transient: bool,
    pub resident: bool,
    /// The picture, copied into the cache at arrival.
    pub image: Option<PathBuf>,
    pub sound_file: Option<String>,
    pub sound_name: Option<String>,
    pub suppress_sound: bool,
    /// A progress from 0 to 100.
    pub value: Option<u8>,
    /// Milliseconds; -1 leaves it to the server, 0 means never.
    pub expire: i32,
    /// The sender's unique bus name.
    pub sender: String,
    pub pid: Option<i32>,
    /// The id it replaces, when it does.
    pub replaces: Option<u32>,
    /// Unix seconds of its arrival.
    pub at: u64,
}

impl Notification {
    pub fn inline_reply(&self) -> bool {
        self.actions.iter().any(|(k, _)| k == "inline-reply")
    }
}

/// What the server hands on.
#[derive(Clone, Debug)]
pub enum Incoming {
    Notify(Box<Notification>),
    /// The app's CloseNotification.
    Close(u32),
    /// A sender left the bus.
    SenderGone(String),
}

/// Why a notification closed (spec's NotificationClosed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Expired = 1,
    Dismissed = 2,
    ClosedByCall = 3,
}

/// What the server sends for the desktop.
#[derive(Clone, Debug)]
pub enum Outgoing {
    Closed(u32, Reason),
    /// An action invoked, after its activation token when there is one.
    Action(u32, String, Option<String>),
    Replied(u32, String),
    Inhibited(bool),
}

struct Server {
    next: Arc<AtomicU32>,
    deliver: Arc<dyn Fn(Incoming) + Send + Sync>,
    cache: PathBuf,
    inhibited: bool,
}

const CAPABILITIES: [&str; 8] = [
    "actions",
    "body",
    "body-markup",
    "icon-static",
    "persistence",
    "inline-reply",
    "sound",
    "x-kde-display-appname",
];

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] conn: &Connection,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id != 0 {
            replaces_id
        } else {
            self.next.fetch_add(1, Ordering::SeqCst)
        };
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        let pid = match header.sender() {
            Some(name) => zbus::fdo::DBusProxy::new(conn)
                .await
                .ok()
                .map(|dbus| async move {
                    dbus.get_connection_unix_process_id(name.clone().into())
                        .await
                        .ok()
                }),
            None => None,
        };
        let pid = match pid {
            Some(fut) => fut.await.map(|p| p as i32),
            None => None,
        };
        let notification = parse(
            id,
            Fields {
                app_name,
                replaces_id,
                app_icon,
                summary,
                body,
                actions,
                hints,
                expire_timeout,
            },
            sender,
            pid,
            &self.cache,
        );
        (self.deliver)(Incoming::Notify(Box::new(notification)));
        id
    }

    fn close_notification(&self, id: u32) {
        (self.deliver)(Incoming::Close(id));
    }

    fn get_capabilities(&self) -> Vec<&'static str> {
        CAPABILITIES.to_vec()
    }

    fn get_server_information(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        ("Isle", "Isle", env!("CARGO_PKG_VERSION"), "1.3")
    }

    #[zbus(property)]
    fn inhibited(&self) -> bool {
        self.inhibited
    }

    #[zbus(signal)]
    async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn action_invoked(emitter: &SignalEmitter<'_>, id: u32, key: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn activation_token(
        emitter: &SignalEmitter<'_>,
        id: u32,
        token: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn notification_replied(
        emitter: &SignalEmitter<'_>,
        id: u32,
        text: &str,
    ) -> zbus::Result<()>;
}

struct Fields {
    app_name: String,
    replaces_id: u32,
    app_icon: String,
    summary: String,
    body: String,
    actions: Vec<String>,
    hints: HashMap<String, OwnedValue>,
    expire_timeout: i32,
}

fn hint_str(hints: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    match hints.get(key).map(|v| &**v) {
        Some(Value::Str(s)) => Some(s.to_string()).filter(|s| !s.is_empty()),
        _ => None,
    }
}

fn hint_bool(hints: &HashMap<String, OwnedValue>, key: &str) -> bool {
    match hints.get(key).map(|v| &**v) {
        Some(Value::Bool(b)) => *b,
        Some(Value::U8(n)) => *n != 0,
        Some(Value::I32(n)) => *n != 0,
        _ => false,
    }
}

fn hint_int(hints: &HashMap<String, OwnedValue>, key: &str) -> Option<i64> {
    match hints.get(key).map(|v| &**v) {
        Some(Value::U8(n)) => Some((*n).into()),
        Some(Value::I32(n)) => Some((*n).into()),
        Some(Value::U32(n)) => Some((*n).into()),
        Some(Value::I64(n)) => Some(*n),
        _ => None,
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn parse(id: u32, f: Fields, sender: String, pid: Option<i32>, cache: &Path) -> Notification {
    let hints = &f.hints;
    let actions = f
        .actions
        .chunks(2)
        .filter_map(|pair| match pair {
            [key, label] => Some((key.clone(), label.clone())),
            _ => None,
        })
        .collect();
    // A file given as the icon is a picture, not an icon.
    let icon_file = (f.app_icon.starts_with('/') || f.app_icon.starts_with("file://"))
        .then(|| f.app_icon.trim_start_matches("file://").to_owned());
    let image = ["image-data", "image_data", "icon_data"]
        .iter()
        .find_map(|key| hints.get(*key))
        .and_then(|data| save_image_data(data, &cache.join(format!("{id}.png"))))
        .or_else(|| {
            ["image-path", "image_path"]
                .iter()
                .find_map(|key| hint_str(hints, key))
                .or(icon_file.clone())
                .map(|p| p.trim_start_matches("file://").to_owned())
                .filter(|p| p.starts_with('/'))
                .and_then(|p| copy_image(Path::new(&p), cache, id))
        });
    Notification {
        id,
        app_name: hint_str(hints, "x-kde-display-appname").unwrap_or(f.app_name),
        app_icon: Some(f.app_icon).filter(|i| !i.is_empty() && icon_file.is_none()),
        summary: f.summary,
        body: f.body,
        actions,
        urgency: hint_int(hints, "urgency").unwrap_or(1).clamp(0, 2) as u8,
        category: hint_str(hints, "category"),
        desktop_entry: hint_str(hints, "desktop-entry")
            .map(|d| d.trim_end_matches(".desktop").to_owned()),
        transient: hint_bool(hints, "transient"),
        resident: hint_bool(hints, "resident"),
        image,
        sound_file: hint_str(hints, "sound-file"),
        sound_name: hint_str(hints, "sound-name"),
        suppress_sound: hint_bool(hints, "suppress-sound"),
        value: hint_int(hints, "value").map(|v| v.clamp(0, 100) as u8),
        expire: f.expire_timeout,
        sender,
        pid: pid.or_else(|| hint_int(hints, "sender-pid").map(|p| p as i32)),
        replaces: (f.replaces_id != 0).then_some(f.replaces_id),
        at: unix_now(),
    }
}

/// A picture sent as pixels, written out as a PNG.
fn save_image_data(value: &OwnedValue, to: &Path) -> Option<PathBuf> {
    let structure = Structure::try_from(value.try_clone().ok()?).ok()?;
    let fields = structure.fields();
    let int = |i: usize| match fields.get(i)? {
        Value::I32(n) => Some(*n),
        _ => None,
    };
    let (width, height, stride) = (int(0)?, int(1)?, int(2)?);
    let alpha = matches!(fields.get(3)?, Value::Bool(true));
    let channels = int(5)?;
    let bytes = <Vec<u8>>::try_from(fields.get(6)?.try_clone().ok()?).ok()?;
    if width <= 0 || height <= 0 || width > 4096 || height > 4096 {
        return None;
    }
    let (w, h, stride, channels) = (
        width as usize,
        height as usize,
        stride as usize,
        channels as usize,
    );
    let mut rgba = Vec::with_capacity(w * h * 4);
    for row in 0..h {
        for col in 0..w {
            let at = row * stride + col * channels;
            let px = bytes.get(at..at + channels)?;
            rgba.extend_from_slice(&[px[0], px[1], px[2], if alpha { px[3] } else { 255 }]);
        }
    }
    std::fs::create_dir_all(to.parent()?).ok()?;
    let file = std::io::BufWriter::new(std::fs::File::create(to).ok()?);
    let mut encoder = png::Encoder::new(file, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().ok()?.write_image_data(&rgba).ok()?;
    Some(to.to_owned())
}

/// A picture on disk, copied so it outlives its sender.
fn copy_image(from: &Path, cache: &Path, id: u32) -> Option<PathBuf> {
    let extension = from.extension().and_then(|e| e.to_str()).unwrap_or("img");
    let to = cache.join(format!("{id}.{extension}"));
    std::fs::create_dir_all(cache).ok()?;
    std::fs::copy(from, &to).ok()?;
    Some(to)
}

/// Where pictures are kept: `~/.cache/isle/notifications`.
pub fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("isle/notifications")
}

/// The name has another owner.
#[derive(Debug)]
pub struct Taken;

/// The server running on a connection: what the desktop sends back through it.
#[derive(Clone)]
pub struct Handle {
    pub outgoing: mpsc::UnboundedSender<Outgoing>,
}

/// Serves the interface on `conn` and owns the name, without queueing behind another owner.
/// `first_id` continues numbering past ids already given out, across a restart.
pub async fn serve(
    conn: &Connection,
    first_id: u32,
    deliver: impl Fn(Incoming) + Send + Sync + 'static,
) -> Result<Result<Handle, Taken>, Error> {
    let deliver: Arc<dyn Fn(Incoming) + Send + Sync> = Arc::new(deliver);
    conn.object_server()
        .at(
            PATH,
            Server {
                next: Arc::new(AtomicU32::new(first_id.max(1))),
                deliver: deliver.clone(),
                cache: cache_dir(),
                inhibited: false,
            },
        )
        .await?;
    use zbus::fdo::{RequestNameFlags, RequestNameReply};
    let reply = conn
        .request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
        .await;
    match reply {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => {}
        Ok(_) | Err(zbus::Error::NameTaken) => {
            conn.object_server().remove::<Server, _>(PATH).await?;
            return Ok(Err(Taken));
        }
        Err(err) => return Err(err.into()),
    }

    // Senders leaving the bus turn their notifications into records.
    let dbus = zbus::fdo::DBusProxy::new(conn).await?;
    let mut owners = dbus.receive_name_owner_changed().await?;
    tokio::spawn({
        let deliver = deliver.clone();
        async move {
            while let Some(change) = owners.next().await {
                let Ok(args) = change.args() else { continue };
                if args.new_owner().is_none() && args.name().starts_with(':') {
                    deliver(Incoming::SenderGone(args.name().to_string()));
                }
            }
        }
    });

    let (outgoing, mut sends) = mpsc::unbounded_channel::<Outgoing>();
    let conn = conn.clone();
    tokio::spawn(async move {
        let Ok(iface) = conn.object_server().interface::<_, Server>(PATH).await else {
            return;
        };
        while let Some(send) = sends.recv().await {
            let emitter = iface.signal_emitter();
            let result = match send {
                Outgoing::Closed(id, reason) => {
                    Server::notification_closed(emitter, id, reason as u32).await
                }
                Outgoing::Action(id, key, token) => {
                    if let Some(token) = token {
                        let _ = Server::activation_token(emitter, id, &token).await;
                    }
                    Server::action_invoked(emitter, id, &key).await
                }
                Outgoing::Replied(id, text) => {
                    Server::notification_replied(emitter, id, &text).await
                }
                Outgoing::Inhibited(on) => {
                    let mut server = iface.get_mut().await;
                    if server.inhibited == on {
                        continue;
                    }
                    server.inhibited = on;
                    server.inhibited_changed(emitter).await
                }
            };
            if let Err(err) = result {
                debug!("a notification signal: {err}");
            }
        }
        warn!("the notification server's sender ended");
    });
    Ok(Ok(Handle { outgoing }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(hints: Vec<(&str, Value<'static>)>) -> Fields {
        Fields {
            app_name: "Mail".into(),
            replaces_id: 0,
            app_icon: "mail-unread".into(),
            summary: "Hello".into(),
            body: "World".into(),
            actions: vec![
                "default".into(),
                "Open".into(),
                "reply".into(),
                "Reply".into(),
            ],
            hints: hints
                .into_iter()
                .map(|(k, v)| (k.to_owned(), OwnedValue::try_from(v).unwrap()))
                .collect(),
            expire_timeout: -1,
        }
    }

    #[test]
    fn hints_are_read_leniently() {
        let n = parse(
            7,
            fields(vec![
                ("urgency", Value::U8(2)),
                ("desktop-entry", Value::from("org.gnome.Evolution.desktop")),
                ("transient", Value::Bool(true)),
                ("value", Value::I32(140)),
                ("x-kde-display-appname", Value::from("Evolution")),
            ]),
            ":1.4".into(),
            None,
            Path::new("/nonexistent"),
        );
        assert_eq!(n.urgency, 2);
        assert_eq!(n.desktop_entry.as_deref(), Some("org.gnome.Evolution"));
        assert!(n.transient);
        assert_eq!(n.value, Some(100));
        assert_eq!(n.app_name, "Evolution");
        assert_eq!(n.app_icon.as_deref(), Some("mail-unread"));
        assert_eq!(n.actions.len(), 2);
    }

    #[test]
    fn pixels_become_a_png_in_the_cache() {
        let dir = std::env::temp_dir().join(format!("isle-notify-{}", std::process::id()));
        let data: Vec<u8> = [255, 0, 0].repeat(4);
        let image = Value::from(Structure::from((2i32, 2i32, 6i32, false, 8i32, 3i32, data)));
        let n = parse(
            3,
            fields(vec![("image-data", image)]),
            String::new(),
            None,
            &dir,
        );
        let path = n.image.unwrap();
        assert!(path.ends_with("3.png") && path.exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
