//! MPRIS players: what each plays, its transport, the most recently playing first, and the
//! art for a track, fetched off the loop and kept in the cache.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use futures_util::{FutureExt, StreamExt};
use isle_bus::{Error, Props, Publisher};
use serde::Serialize;
use zbus::{Connection, zvariant::Value};

const PREFIX: &str = "org.mpris.MediaPlayer2.";
const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";
const ROOT: &str = "org.mpris.MediaPlayer2";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Playing,
    Paused,
    #[default]
    Stopped,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Player {
    /// Its bus name, which commands go to.
    pub bus: String,
    pub identity: String,
    pub desktop_entry: Option<String>,
    pub status: Status,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub track: Option<String>,
    /// Microseconds.
    pub length: Option<u64>,
    /// Microseconds, when last read; it runs on at `rate` while playing.
    pub position: u64,
    pub rate: f64,
    pub can_go_next: bool,
    pub can_go_previous: bool,
    pub can_seek: bool,
    pub can_raise: bool,
    pub shuffle: Option<bool>,
    /// `None`, `Track` or `Playlist`, when the player has a loop.
    pub loop_status: Option<String>,
    /// When it last started playing, as an order: higher is more recent.
    pub played: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Media {
    /// The player playing most recently first, then those paused by recency.
    pub players: Vec<Player>,
}

impl Media {
    /// The player the island and Now Playing show.
    pub fn chosen(&self) -> Option<&Player> {
        self.players.first()
    }
}

static ORDER: AtomicU64 = AtomicU64::new(1);
static PLAYED: Mutex<Option<HashMap<String, (Status, u64)>>> = Mutex::new(None);

fn text_list(props: &HashMap<String, zbus::zvariant::OwnedValue>, key: &str) -> String {
    match props.get(key).map(|v| &**v) {
        Some(Value::Str(s)) => s.to_string(),
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| match v {
                Value::Str(s) => Some(s.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}

async fn read_player(conn: &Connection, bus: &str) -> Result<Player, Error> {
    let p = Props::read(conn, bus, PATH, PLAYER).await?;
    let root = Props::read(conn, bus, PATH, ROOT).await.unwrap_or_default();
    let metadata: HashMap<String, zbus::zvariant::OwnedValue> =
        p.0.get("Metadata")
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| v.try_into().ok())
            .unwrap_or_default();
    let meta = Props(metadata.clone());
    let status = match p.str("PlaybackStatus").as_deref() {
        Some("Playing") => Status::Playing,
        Some("Paused") => Status::Paused,
        _ => Status::Stopped,
    };
    // Position is never announced; it is asked for, and some players refuse for a stream.
    let position = conn
        .call_method(
            Some(bus),
            PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(PLAYER, "Position"),
        )
        .await
        .ok()
        .and_then(|m| m.body().deserialize::<zbus::zvariant::OwnedValue>().ok())
        .and_then(|v| match &*v {
            Value::I64(n) => Some(*n),
            Value::U64(n) => i64::try_from(*n).ok(),
            _ => None,
        })
        .unwrap_or(0)
        .max(0) as u64;
    Ok(Player {
        bus: bus.to_owned(),
        identity: root
            .str("Identity")
            .unwrap_or_else(|| bus.trim_start_matches(PREFIX).to_owned()),
        desktop_entry: root.str("DesktopEntry"),
        status,
        title: meta.str("xesam:title").unwrap_or_default(),
        artist: text_list(&metadata, "xesam:artist"),
        album: meta.str("xesam:album").unwrap_or_default(),
        art_url: meta.str("mpris:artUrl").filter(|u| !u.is_empty()),
        track: meta.str("mpris:trackid"),
        length: meta
            .i64("mpris:length")
            .filter(|l| *l > 0)
            .map(|l| l as u64),
        position,
        rate: p.f64("Rate").unwrap_or(1.0),
        can_go_next: p.bool("CanGoNext").unwrap_or(false),
        can_go_previous: p.bool("CanGoPrevious").unwrap_or(false),
        can_seek: p.bool("CanSeek").unwrap_or(false),
        can_raise: root.bool("CanRaise").unwrap_or(false),
        shuffle: p.bool("Shuffle"),
        loop_status: p.str("LoopStatus"),
        played: 0,
    })
}

async fn read(conn: &Connection) -> Result<Media, Error> {
    let dbus = zbus::fdo::DBusProxy::new(conn).await?;
    let mut players = Vec::new();
    for name in dbus.list_names().await? {
        if name.starts_with(PREFIX)
            && let Ok(player) = read_player(conn, name.as_str()).await
        {
            players.push(player);
        }
    }
    let mut played = PLAYED.lock().unwrap();
    let played = played.get_or_insert_with(HashMap::new);
    for player in &mut players {
        let entry = played
            .entry(player.bus.clone())
            .or_insert((Status::Stopped, 0));
        if player.status == Status::Playing && entry.0 != Status::Playing {
            entry.1 = ORDER.fetch_add(1, Ordering::SeqCst);
        }
        entry.0 = player.status;
        player.played = entry.1;
    }
    played.retain(|bus, _| players.iter().any(|p| &p.bus == bus));
    players.sort_by(|a, b| {
        (b.status == Status::Playing)
            .cmp(&(a.status == Status::Playing))
            .then(b.played.cmp(&a.played))
    });
    Ok(Media { players })
}

/// Follows every player on the session bus.
pub async fn watch(publisher: Publisher<Media>) -> Result<(), Error> {
    let conn = isle_bus::session().await?;
    let dbus = zbus::fdo::DBusProxy::new(&conn).await?;
    let mut names = dbus.receive_name_owner_changed().await?;
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .path(PATH)?
        .build();
    let mut signals = zbus::MessageStream::for_match_rule(rule, &conn, Some(64)).await?;
    loop {
        publisher.publish(read(&conn).await?);
        tokio::select! {
            change = names.next() => {
                let Some(change) = change else { return Err("the bus ended".into()) };
                if !change.args().is_ok_and(|a| a.name().starts_with(PREFIX)) {
                    continue;
                }
            }
            signal = signals.next() => {
                if signal.is_none() {
                    return Err("the bus ended".into());
                }
            }
        }
        // A burst is read once, a moment after it starts.
        tokio::time::sleep(Duration::from_millis(30)).await;
        while let Some(Some(_)) = signals.next().now_or_never() {}
    }
}

async fn player_call<B>(bus: &str, interface: &str, method: &str, body: &B) -> Result<(), Error>
where
    B: serde::Serialize + zbus::zvariant::DynamicType,
{
    let conn = isle_bus::session().await?;
    conn.call_method(Some(bus), PATH, Some(interface), method, body)
        .await?;
    Ok(())
}

pub async fn play_pause(bus: &str) -> Result<(), Error> {
    player_call(bus, PLAYER, "PlayPause", &()).await
}

pub async fn next(bus: &str) -> Result<(), Error> {
    player_call(bus, PLAYER, "Next", &()).await
}

pub async fn previous(bus: &str) -> Result<(), Error> {
    player_call(bus, PLAYER, "Previous", &()).await
}

pub async fn raise(bus: &str) -> Result<(), Error> {
    player_call(bus, ROOT, "Raise", &()).await
}

/// Seeks the track `track` to `position` microseconds.
pub async fn set_position(bus: &str, track: &str, position: u64) -> Result<(), Error> {
    let track = zbus::zvariant::ObjectPath::try_from(track)?;
    player_call(bus, PLAYER, "SetPosition", &(track, position as i64)).await
}

pub async fn set_shuffle(bus: &str, on: bool) -> Result<(), Error> {
    player_call(
        bus,
        "org.freedesktop.DBus.Properties",
        "Set",
        &(PLAYER, "Shuffle", Value::from(on)),
    )
    .await
}

pub async fn set_loop(bus: &str, status: &str) -> Result<(), Error> {
    player_call(
        bus,
        "org.freedesktop.DBus.Properties",
        "Set",
        &(PLAYER, "LoopStatus", Value::from(status)),
    )
    .await
}

/// The most a fetched picture may be.
const ART_LIMIT: usize = 2 * 1024 * 1024;

fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("isle/art")
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(&text[i + 1..i + 3], 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A track's art as a local file: a `file://` as it is, an `https://` fetched and a `data:`
/// decoded into the cache.
pub async fn art(url: &str) -> Option<PathBuf> {
    if let Some(path) = url.strip_prefix("file://") {
        let path = PathBuf::from(percent_decode(path));
        return path.exists().then_some(path);
    }
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let file = cache_dir().join(format!("{:016x}", hasher.finish()));
    if file.exists() {
        return Some(file);
    }
    let bytes = if let Some(data) = url.strip_prefix("data:") {
        use base64::Engine;
        let (_, encoded) = data.split_once(";base64,")?;
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()?
    } else if url.starts_with("https://") || url.starts_with("http://") {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        let mut response = client.get(url).send().await.ok()?.error_for_status().ok()?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            bytes.extend_from_slice(&chunk);
            if bytes.len() > ART_LIMIT {
                return None;
            }
        }
        bytes
    } else {
        return None;
    };
    if bytes.len() > ART_LIMIT {
        return None;
    }
    std::fs::create_dir_all(cache_dir()).ok()?;
    let partial = file.with_extension("part");
    std::fs::write(&partial, bytes).ok()?;
    std::fs::rename(&partial, &file).ok()?;
    Some(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_files_url_decodes_its_escapes() {
        assert_eq!(percent_decode("/tmp/a%20b%2Bc.jpg"), "/tmp/a b+c.jpg");
    }

    #[tokio::test]
    async fn a_data_url_is_decoded_into_the_cache() {
        let dir = std::env::temp_dir().join(format!("isle-art-{}", std::process::id()));
        // SAFETY: the test owns its process's environment.
        unsafe { std::env::set_var("XDG_CACHE_HOME", &dir) };
        let path = art("data:image/png;base64,aGVsbG8=").await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        let _ = std::fs::remove_dir_all(dir);
    }
}
