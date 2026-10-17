//! Sound through pipewire-pulse: output and input devices, the defaults, volume and mute,
//! the streams each app plays and records, alert samples and an input's level; and, through
//! PipeWire, the cameras in use (`p8-services.md` choice 3).

pub mod camera;
pub mod pulse;
pub mod sounds;

use std::{
    ffi::{CStr, CString},
    sync::{Arc, Mutex},
    time::Duration,
};

use isle_bus::{Error, Publisher};
use pulse::Conn;
use pulseaudio::protocol::{self, Command, Prop};
use serde::Serialize;
use tokio::sync::mpsc;

/// Apps whose recording streams only measure levels, never a use of the microphone.
const METERS: [&str; 4] = [
    "io.github.alexparlett.Isle",
    "org.PulseAudio.pavucontrol",
    "org.gnome.VolumeControl",
    "org.kde.plasma.volume",
];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Device {
    pub index: u32,
    /// The server's name, which commands use.
    pub name: String,
    pub label: String,
    /// 0 to 1 of the device's normal volume; above it reads as 1.
    pub volume: f32,
    pub muted: bool,
    pub default: bool,
    pub bluetooth: bool,
    /// -1, all left, to 1, all right; 0 for a device with no left and right.
    pub balance: f32,
    channels: u8,
    /// Each channel's side, for setting the balance.
    #[serde(skip)]
    sides: Vec<Side>,
}

/// Which side a channel sounds from, for the balance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
    Middle,
}

fn sides(map: &protocol::ChannelMap) -> Vec<Side> {
    map.into_iter()
        .map(|position| {
            let name = format!("{position:?}");
            if name.contains("Left") {
                Side::Left
            } else if name.contains("Right") {
                Side::Right
            } else {
                Side::Middle
            }
        })
        .collect()
}

/// PulseAudio's balance of a volume: the quieter side over the louder.
fn balance(sides: &[Side], volume: &protocol::ChannelVolume) -> f32 {
    let average = |side| {
        let levels: Vec<f32> = sides
            .iter()
            .zip(volume.channels())
            .filter(|(s, _)| **s == side)
            .map(|(_, v)| v.as_u32() as f32)
            .collect();
        (!levels.is_empty()).then(|| levels.iter().sum::<f32>() / levels.len() as f32)
    };
    match (average(Side::Left), average(Side::Right)) {
        (Some(l), Some(r)) if l > r && l > 0.0 => -1.0 + r / l,
        (Some(l), Some(r)) if r > l && r > 0.0 => 1.0 - l / r,
        _ => 0.0,
    }
}

/// The channel volumes for `level` at `balance`: the louder side at the level, the other
/// lowered by the balance.
fn balanced(sides: &[Side], level: f32, balance: f32) -> protocol::ChannelVolume {
    let balance = balance.clamp(-1.0, 1.0);
    let (left, right) = if balance < 0.0 {
        (level, level * (1.0 + balance))
    } else {
        (level * (1.0 - balance), level)
    };
    let mut volume = protocol::ChannelVolume::empty();
    for side in sides {
        let value = match side {
            Side::Left => left,
            Side::Right => right,
            Side::Middle => level,
        };
        let raw = (value.clamp(0.0, 1.0) * protocol::Volume::NORM.as_u32() as f32).round() as u32;
        volume.push(protocol::Volume::from_u32_clamped(raw));
    }
    volume
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Stream {
    pub index: u32,
    /// The app as it names itself.
    pub app: String,
    pub app_id: Option<String>,
    pub binary: Option<String>,
    pub pid: Option<i32>,
    pub icon: Option<String>,
    pub volume: f32,
    pub muted: bool,
    /// The device it plays to or records from.
    pub device: u32,
    channels: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Audio {
    pub outputs: Vec<Device>,
    pub inputs: Vec<Device>,
    /// Streams playing, corked ones left out.
    pub playing: Vec<Stream>,
    /// Streams recording from a microphone, meters and monitors left out.
    pub recording: Vec<Stream>,
}

impl Audio {
    pub fn default_output(&self) -> Option<&Device> {
        self.outputs.iter().find(|d| d.default)
    }

    pub fn default_input(&self) -> Option<&Device> {
        self.inputs.iter().find(|d| d.default)
    }
}

fn text(c: &CStr) -> String {
    c.to_string_lossy().into_owned()
}

fn prop(props: &protocol::Props, prop: Prop) -> Option<String> {
    let bytes = props.get(prop)?;
    let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    Some(String::from_utf8_lossy(bytes).into_owned()).filter(|s| !s.is_empty())
}

fn level(volume: &protocol::ChannelVolume) -> f32 {
    let channels = volume.channels();
    if channels.is_empty() {
        return 0.0;
    }
    let loudest = channels.iter().map(|v| v.as_u32()).max().unwrap_or(0);
    (loudest as f32 / protocol::Volume::NORM.as_u32() as f32).clamp(0.0, 1.0)
}

fn stream(
    index: u32,
    props: &protocol::Props,
    volume: &protocol::ChannelVolume,
    muted: bool,
    device: u32,
) -> Stream {
    Stream {
        index,
        app: prop(props, Prop::ApplicationName)
            .or_else(|| prop(props, Prop::MediaName))
            .unwrap_or_else(|| "Unknown".into()),
        app_id: prop(props, Prop::ApplicationId),
        binary: prop(props, Prop::ApplicationProcessBinary),
        pid: prop(props, Prop::ApplicationProcessId).and_then(|p| p.parse().ok()),
        icon: prop(props, Prop::ApplicationIconName),
        volume: level(volume),
        muted,
        device,
        channels: volume.channels().len() as u8,
    }
}

async fn read(conn: &Conn) -> Result<Audio, Error> {
    let server: protocol::ServerInfo = conn.reply(Command::GetServerInfo).await?;
    let default_sink = server.default_sink_name.as_deref().map(text);
    let default_source = server.default_source_name.as_deref().map(text);
    let sinks: protocol::SinkInfoList = conn.reply(Command::GetSinkInfoList).await?;
    let sources: protocol::SourceInfoList = conn.reply(Command::GetSourceInfoList).await?;
    let inputs: protocol::SinkInputInfoList = conn.reply(Command::GetSinkInputInfoList).await?;
    let outputs: protocol::SourceOutputInfoList =
        conn.reply(Command::GetSourceOutputInfoList).await?;

    let device = |index,
                  name: &CStr,
                  description: Option<&CStr>,
                  props: &protocol::Props,
                  volume: &protocol::ChannelVolume,
                  map: &protocol::ChannelMap,
                  muted,
                  default: &Option<String>| {
        let name = text(name);
        let sides = sides(map);
        Device {
            index,
            label: description.map(text).unwrap_or_else(|| name.clone()),
            default: default.as_deref() == Some(name.as_str()),
            name,
            volume: level(volume),
            muted,
            bluetooth: prop(props, Prop::DeviceBus).as_deref() == Some("bluetooth"),
            balance: balance(&sides, volume),
            channels: volume.channels().len() as u8,
            sides,
        }
    };
    let monitors: Vec<u32> = sources
        .iter()
        .filter(|s| s.monitor_of_sink_index.is_some())
        .map(|s| s.index)
        .collect();
    Ok(Audio {
        outputs: sinks
            .iter()
            .map(|s| {
                device(
                    s.index,
                    &s.name,
                    s.description.as_deref(),
                    &s.props,
                    &s.cvolume,
                    &s.channel_map,
                    s.muted,
                    &default_sink,
                )
            })
            .collect(),
        inputs: sources
            .iter()
            .filter(|s| s.monitor_of_sink_index.is_none())
            .map(|s| {
                device(
                    s.index,
                    &s.name,
                    s.description.as_deref(),
                    &s.props,
                    &s.cvolume,
                    &s.channel_map,
                    s.muted,
                    &default_source,
                )
            })
            .collect(),
        playing: inputs
            .iter()
            .filter(|i| !i.corked)
            .map(|i| stream(i.index, &i.props, &i.cvolume, i.muted, i.sink_index))
            .collect(),
        recording: outputs
            .iter()
            .filter(|o| !o.corked && !monitors.contains(&o.source_index))
            .filter(|o| {
                let id = prop(&o.props, Prop::ApplicationId);
                !METERS.iter().any(|m| Some(*m) == id.as_deref())
            })
            .map(|o| stream(o.index, &o.props, &o.cvolume, o.muted, o.source_index))
            .collect(),
    })
}

static SHARED: Mutex<Option<Arc<Conn>>> = Mutex::new(None);

fn shared() -> Result<Arc<Conn>, Error> {
    SHARED
        .lock()
        .unwrap()
        .clone()
        .filter(|c| c.alive())
        .ok_or_else(|| "no sound server".into())
}

/// Follows the sound server, connecting again when it goes and returns.
pub async fn watch(publisher: Publisher<Audio>) -> Result<(), Error> {
    loop {
        // A server that accepts and never answers, as a socket-activated one can while it
        // starts, is asked again.
        let connected = tokio::time::timeout(Duration::from_secs(5), Conn::connect("Isle")).await;
        let (conn, mut events) = match connected {
            Ok(Ok(connected)) => connected,
            _ => {
                publisher.unavailable();
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        *SHARED.lock().unwrap() = Some(conn.clone());
        let mask = protocol::SubscriptionMask::SINK
            | protocol::SubscriptionMask::SOURCE
            | protocol::SubscriptionMask::SINK_INPUT
            | protocol::SubscriptionMask::SOURCE_OUTPUT
            | protocol::SubscriptionMask::SERVER;
        let mut result = conn.ack(Command::Subscribe(mask)).await;
        while result.is_ok() {
            result = read(&conn).await.map(|audio| publisher.publish(audio));
            if events.recv().await.is_none() {
                break;
            }
            while events.try_recv().is_ok() {}
        }
        *SHARED.lock().unwrap() = None;
        publisher.unavailable();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// What a volume or mute applies to.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Output(String),
    Input(String),
    Playing(u32),
    Recording(u32),
}

fn cvolume(channels: u8, value: f32) -> protocol::ChannelVolume {
    let raw = (value.clamp(0.0, 1.0) * protocol::Volume::NORM.as_u32() as f32).round() as u32;
    let mut volume = protocol::ChannelVolume::empty();
    for _ in 0..channels.max(1) {
        volume.push(protocol::Volume::from_u32_clamped(raw));
    }
    volume
}

/// Sets `target`'s volume, 0 to 1, on every channel; `channels` is how many it has.
pub async fn set_volume(target: Target, channels: u8, value: f32) -> Result<(), Error> {
    let conn = shared()?;
    let volume = cvolume(channels, value);
    let command = match target {
        Target::Output(name) => Command::SetSinkVolume(protocol::SetDeviceVolumeParams {
            device_index: None,
            device_name: Some(CString::new(name)?),
            volume,
        }),
        Target::Input(name) => Command::SetSourceVolume(protocol::SetDeviceVolumeParams {
            device_index: None,
            device_name: Some(CString::new(name)?),
            volume,
        }),
        Target::Playing(index) => {
            Command::SetSinkInputVolume(protocol::SetStreamVolumeParams { index, volume })
        }
        Target::Recording(index) => {
            Command::SetSourceOutputVolume(protocol::SetStreamVolumeParams { index, volume })
        }
    };
    conn.ack(command).await
}

/// Sets a device's balance, -1 all left to 1 all right, at its level now.
pub async fn set_balance(device: &Device, output: bool, value: f32) -> Result<(), Error> {
    let conn = shared()?;
    let volume = balanced(&device.sides, device.volume, value);
    let params = protocol::SetDeviceVolumeParams {
        device_index: None,
        device_name: Some(CString::new(device.name.clone())?),
        volume,
    };
    conn.ack(if output {
        Command::SetSinkVolume(params)
    } else {
        Command::SetSourceVolume(params)
    })
    .await
}

pub async fn set_mute(target: Target, mute: bool) -> Result<(), Error> {
    let conn = shared()?;
    let command = match target {
        Target::Output(name) => Command::SetSinkMute(protocol::SetDeviceMuteParams {
            device_index: None,
            device_name: Some(CString::new(name)?),
            mute,
        }),
        Target::Input(name) => Command::SetSourceMute(protocol::SetDeviceMuteParams {
            device_index: None,
            device_name: Some(CString::new(name)?),
            mute,
        }),
        Target::Playing(index) => {
            Command::SetSinkInputMute(protocol::SetStreamMuteParams { index, mute })
        }
        Target::Recording(index) => {
            Command::SetSourceOutputMute(protocol::SetStreamMuteParams { index, mute })
        }
    };
    conn.ack(command).await
}

pub async fn set_default_output(name: String) -> Result<(), Error> {
    shared()?
        .ack(Command::SetDefaultSink(CString::new(name)?))
        .await
}

pub async fn set_default_input(name: String) -> Result<(), Error> {
    shared()?
        .ack(Command::SetDefaultSource(CString::new(name)?))
        .await
}

impl Device {
    pub fn channels(&self) -> u8 {
        self.channels
    }
}

impl Stream {
    pub fn channels(&self) -> u8 {
        self.channels
    }
}

/// An input's level, read while held: its peaks, 0 to 1, a few dozen times a second.
pub struct Meter {
    pub peaks: mpsc::UnboundedReceiver<f32>,
    conn: Arc<Conn>,
    channel: u32,
}

/// Starts measuring the input `source`; measuring stops when the meter is dropped.
pub async fn meter(source: &str) -> Result<Meter, Error> {
    let conn = shared()?;
    let mut props = protocol::Props::new();
    props.set(
        Prop::ApplicationId,
        CString::new("io.github.alexparlett.Isle")?,
    );
    props.set(Prop::MediaName, CString::new("Input level")?);
    let reply: protocol::CreateRecordStreamReply = conn
        .reply(Command::CreateRecordStream(protocol::RecordStreamParams {
            sample_spec: protocol::SampleSpec {
                format: protocol::SampleFormat::Float32Le,
                channels: 1,
                sample_rate: 25,
            },
            channel_map: protocol::ChannelMap::mono(),
            source_name: Some(CString::new(source)?),
            buffer_attr: protocol::stream::BufferAttr {
                fragment_size: 4,
                ..Default::default()
            },
            flags: protocol::stream::StreamFlags {
                peak_detect: true,
                adjust_latency: true,
                no_move: true,
                ..Default::default()
            },
            props,
            ..Default::default()
        }))
        .await?;
    let mut data = conn.record(reply.channel);
    let (peaks, received) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(chunk) = data.recv().await {
            for sample in chunk.chunks_exact(4) {
                let value = f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]);
                if peaks.send(value.abs().clamp(0.0, 1.0)).is_err() {
                    return;
                }
            }
        }
    });
    Ok(Meter {
        peaks: received,
        conn,
        channel: reply.channel,
    })
}

impl Drop for Meter {
    fn drop(&mut self) {
        let (conn, channel) = (self.conn.clone(), self.channel);
        conn.forget_record(channel);
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let _ = conn.ack(Command::DeleteRecordStream(channel)).await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_reads_and_sets_as_pulseaudio_does() {
        let stereo = [Side::Left, Side::Right];
        let centred = balanced(&stereo, 0.8, 0.0);
        assert_eq!(balance(&stereo, &centred), 0.0);
        let left = balanced(&stereo, 0.8, -0.5);
        assert!((balance(&stereo, &left) + 0.5).abs() < 0.01);
        assert!((level(&left) - 0.8).abs() < 0.001, "the louder side keeps the level");
        assert_eq!(balance(&[Side::Middle], &balanced(&[Side::Middle], 0.5, 0.7)), 0.0);
        let surround = sides(&protocol::ChannelMap::stereo());
        assert_eq!(surround, stereo);
    }

    #[test]
    fn a_volume_reads_as_its_loudest_channel_capped_at_normal() {
        let mut v = protocol::ChannelVolume::empty();
        v.push(protocol::Volume::from_u32_clamped(
            protocol::Volume::NORM.as_u32() / 2,
        ));
        v.push(protocol::Volume::from_u32_clamped(
            protocol::Volume::NORM.as_u32() / 4,
        ));
        assert_eq!(level(&v), 0.5);
        let loud = cvolume(2, 1.0);
        assert_eq!(level(&loud), 1.0);
        assert_eq!(loud.channels().len(), 2);
    }
}
