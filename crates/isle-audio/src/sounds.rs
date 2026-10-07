//! Alert sounds: a name looked up in the sound theme, or a file, decoded once, uploaded to
//! the sound server as a sample and played from there (`p8-services.md` choice 11).

use std::{
    collections::HashSet,
    ffi::CString,
    path::{Path, PathBuf},
    sync::Mutex,
};

use isle_bus::Error;
use pulseaudio::protocol::{self, Command, Prop};

/// Samples the server already holds, by name.
static UPLOADED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn data_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(home) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    {
        dirs.push(home);
    }
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.extend(system.split(':').map(PathBuf::from));
    dirs
}

/// A sound's file by its name in `theme`, then in what it inherits, then `freedesktop`.
pub fn lookup(name: &str, theme: &str) -> Option<PathBuf> {
    let mut themes = vec![theme.to_owned()];
    let mut seen = HashSet::new();
    while let Some(theme) = themes.pop() {
        if !seen.insert(theme.clone()) {
            continue;
        }
        for dir in data_dirs() {
            let root = dir.join("sounds").join(&theme);
            for sub in ["stereo", ""] {
                for ext in ["oga", "ogg", "wav"] {
                    let file = root.join(sub).join(format!("{name}.{ext}"));
                    if file.exists() {
                        return Some(file);
                    }
                }
            }
            if let Ok(index) = std::fs::read_to_string(root.join("index.theme"))
                && let Some(inherits) = index.lines().find_map(|l| l.strip_prefix("Inherits="))
            {
                themes.extend(inherits.split(',').map(|t| t.trim().to_owned()));
            }
        }
        if themes.is_empty() && theme != "freedesktop" {
            themes.push("freedesktop".into());
        }
    }
    None
}

/// Interleaved 16-bit samples, their channels and rate.
struct Pcm {
    samples: Vec<i16>,
    channels: u8,
    rate: u32,
}

fn decode(path: &Path) -> Result<Pcm, Error> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"RIFF") {
        return decode_wav(&bytes);
    }
    let mut reader = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes))?;
    let channels = reader.ident_hdr.audio_channels;
    let rate = reader.ident_hdr.audio_sample_rate;
    let mut samples = Vec::new();
    while let Some(packet) = reader.read_dec_packet_itl()? {
        samples.extend(packet);
        if samples.len() > rate as usize * channels as usize * 10 {
            return Err("an alert sound longer than ten seconds".into());
        }
    }
    Ok(Pcm {
        samples,
        channels,
        rate,
    })
}

/// A plain 16-bit PCM WAV.
fn decode_wav(bytes: &[u8]) -> Result<Pcm, Error> {
    let mut at = 12;
    let (mut channels, mut rate, mut bits) = (0u16, 0u32, 0u16);
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into()?) as usize;
        let body = bytes.get(at + 8..at + 8 + len).ok_or("a short WAV")?;
        match id {
            b"fmt " => {
                channels = u16::from_le_bytes(body[2..4].try_into()?);
                rate = u32::from_le_bytes(body[4..8].try_into()?);
                bits = u16::from_le_bytes(body[14..16].try_into()?);
            }
            b"data" if bits == 16 => {
                return Ok(Pcm {
                    samples: body
                        .chunks_exact(2)
                        .map(|c| i16::from_le_bytes([c[0], c[1]]))
                        .collect(),
                    channels: channels as u8,
                    rate,
                });
            }
            _ => {}
        }
        at += 8 + len + (len & 1);
    }
    Err("not a 16-bit PCM WAV".into())
}

/// Plays a sound file, uploading it as a sample the first time.
pub async fn play(path: &Path) -> Result<(), Error> {
    let conn = crate::shared()?;
    let name = format!("isle-{}", path.display());
    let known = UPLOADED
        .lock()
        .unwrap()
        .get_or_insert_with(HashSet::new)
        .contains(&name);
    if !known {
        let file = path.to_owned();
        let pcm = tokio::task::spawn_blocking(move || decode(&file)).await??;
        let bytes: Vec<u8> = pcm.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut props = protocol::Props::new();
        props.set(Prop::MediaRole, CString::new("event")?);
        let channel_map = if pcm.channels == 1 {
            protocol::ChannelMap::mono()
        } else {
            protocol::ChannelMap::stereo()
        };
        let reply: protocol::CreateUploadStreamReply = conn
            .reply(Command::CreateUploadStream(protocol::UploadStreamParams {
                media_name: Some(CString::new(name.clone())?),
                sample_spec: protocol::SampleSpec {
                    format: protocol::SampleFormat::S16Le,
                    channels: pcm.channels,
                    sample_rate: pcm.rate,
                },
                channel_map,
                length: bytes.len() as u32,
                props,
            }))
            .await?;
        conn.write_data(reply.channel, &bytes).await?;
        conn.ack(Command::FinishUploadStream(reply.channel)).await?;
        UPLOADED
            .lock()
            .unwrap()
            .get_or_insert_with(HashSet::new)
            .insert(name.clone());
    }
    let mut props = protocol::Props::new();
    props.set(Prop::MediaRole, CString::new("event")?);
    conn.ack(Command::PlaySample(protocol::PlaySampleParams {
        sink_index: None,
        sink_name: None,
        volume: u32::MAX,
        name: CString::new(name)?,
        props,
    }))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wav_reads_as_its_samples() {
        let mut wav = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(8000u32.to_le_bytes());
        wav.extend(16000u32.to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend(4u32.to_le_bytes());
        wav.extend([1, 0, 255, 255]);
        let pcm = decode_wav(&wav).unwrap();
        assert_eq!(
            (pcm.samples, pcm.channels, pcm.rate),
            (vec![1, -1], 1, 8000)
        );
    }

    #[test]
    fn the_freedesktop_theme_has_the_alert() {
        if Path::new("/usr/share/sounds/freedesktop").exists() {
            assert!(lookup("message-new-instant", "freedesktop").is_some());
            assert!(lookup("message-new-instant", "no-such-theme").is_some());
        }
    }
}
