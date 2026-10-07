//! An MPRIS player on the session bus, with one track and a transport.

use std::collections::HashMap;

use zbus::{
    Connection,
    object_server::SignalEmitter,
    zvariant::{ObjectPath, OwnedValue, Value},
};

const PATH: &str = "/org/mpris/MediaPlayer2";
const TRACK: &str = "/org/isle/track/1";

struct Root {
    identity: String,
    raised: u32,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    #[zbus(property)]
    fn identity(&self) -> String {
        self.identity.clone()
    }
    #[zbus(property)]
    fn can_raise(&self) -> bool {
        true
    }
    fn raise(&mut self) {
        self.raised += 1;
    }
}

struct Player {
    status: String,
    title: String,
    position: i64,
    nexts: u32,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.status.clone()
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let value = |v: Value<'_>| OwnedValue::try_from(v).unwrap();
        HashMap::from([
            ("xesam:title".into(), value(Value::from(self.title.clone()))),
            (
                "xesam:artist".into(),
                value(Value::from(vec!["Test Artist"])),
            ),
            ("mpris:length".into(), value(Value::from(180_000_000_i64))),
            (
                "mpris:trackid".into(),
                value(Value::from(ObjectPath::try_from(TRACK).unwrap())),
            ),
        ])
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        self.position
    }
    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_seek(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    async fn play_pause(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.status = if self.status == "Playing" {
            "Paused"
        } else {
            "Playing"
        }
        .into();
        let _ = self.playback_status_changed(&emitter).await;
    }

    async fn next(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.nexts += 1;
        self.title = format!("Track {}", self.nexts + 1);
        let _ = self.metadata_changed(&emitter).await;
    }

    async fn previous(&mut self) {}

    async fn set_position(
        &mut self,
        _track: ObjectPath<'_>,
        position: i64,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.position = position;
        let _ = Self::seeked(&emitter, position).await;
    }

    #[zbus(signal)]
    async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;
}

/// A player named `org.mpris.MediaPlayer2.<name>`.
pub struct Mpris {
    conn: Connection,
}

impl Mpris {
    pub async fn start(address: &str, name: &str, playing: bool) -> zbus::Result<Self> {
        let conn = zbus::connection::Builder::address(address)?
            .serve_at(
                PATH,
                Root {
                    identity: name.into(),
                    raised: 0,
                },
            )?
            .serve_at(
                PATH,
                Player {
                    status: if playing { "Playing" } else { "Paused" }.into(),
                    title: "Track 1".into(),
                    position: 0,
                    nexts: 0,
                },
            )?
            .name(format!("org.mpris.MediaPlayer2.{name}"))?
            .build()
            .await?;
        Ok(Self { conn })
    }

    async fn player(&self) -> zbus::object_server::InterfaceRef<Player> {
        self.conn
            .object_server()
            .interface::<_, Player>(PATH)
            .await
            .unwrap()
    }

    pub async fn status(&self) -> String {
        self.player().await.get().await.status.clone()
    }

    pub async fn position(&self) -> i64 {
        self.player().await.get().await.position
    }

    pub async fn nexts(&self) -> u32 {
        self.player().await.get().await.nexts
    }

    /// Plays or pauses, as the player's own controls would.
    pub async fn play_pause(&self) {
        let iface = self.player().await;
        let mut player = iface.get_mut().await;
        player.status = if player.status == "Playing" {
            "Paused"
        } else {
            "Playing"
        }
        .into();
        let _ = player.playback_status_changed(iface.signal_emitter()).await;
    }

    pub async fn vanish(self) {
        let _ = self.conn.close().await;
    }
}
