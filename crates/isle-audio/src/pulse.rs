//! A connection to pipewire-pulse speaking the Pulse native protocol on tokio: commands and
//! their replies by sequence number, subscription events, and record streams' data.

use std::{
    collections::HashMap,
    ffi::CString,
    io::Cursor,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use isle_bus::Error;
use pulseaudio::protocol::{self, Command, CommandReply};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::{mpsc, oneshot},
};

/// How long a command waits for its reply.
const LIMIT: Duration = Duration::from_secs(2);

type Pending = Arc<Mutex<HashMap<u32, oneshot::Sender<Vec<u8>>>>>;
type Records = Arc<Mutex<HashMap<u32, mpsc::UnboundedSender<Vec<u8>>>>>;

/// One connection, shared by the watch and the commands.
pub struct Conn {
    version: u16,
    seq: AtomicU32,
    writer: tokio::sync::Mutex<tokio::net::unix::OwnedWriteHalf>,
    pending: Pending,
    records: Records,
}

impl Conn {
    /// Connects to the server the environment names and introduces Isle; returns the
    /// connection and its stream of subscription events.
    pub async fn connect(
        name: &str,
    ) -> Result<
        (
            Arc<Self>,
            mpsc::UnboundedReceiver<protocol::SubscriptionEvent>,
        ),
        Error,
    > {
        let path = pulseaudio::socket_path_from_env().ok_or("no Pulse server")?;
        // The protocol wants 256 bytes and pipewire-pulse checks only their length, so a
        // session with no cookie file sends zeroes.
        let cookie = pulseaudio::cookie_path_from_env()
            .and_then(|p| std::fs::read(p).ok())
            .filter(|c| c.len() == 256)
            .unwrap_or_else(|| vec![0; 256]);
        let stream = UnixStream::connect(path).await?;
        let (mut reader, writer) = stream.into_split();
        let mut conn = Self {
            version: protocol::MAX_VERSION,
            seq: AtomicU32::new(0),
            writer: tokio::sync::Mutex::new(writer),
            pending: Pending::default(),
            records: Records::default(),
        };

        // The handshake, before the reader runs.
        conn.send(Command::Auth(protocol::AuthParams {
            version: protocol::MAX_VERSION,
            supports_shm: false,
            supports_memfd: false,
            cookie,
        }))
        .await?;
        let frame = read_frame(&mut reader).await?;
        let (_, auth) = protocol::read_reply_message::<protocol::AuthReply>(
            &mut Cursor::new(frame),
            protocol::MAX_VERSION,
        )?;
        conn.version = protocol::MAX_VERSION.min(auth.version);
        let mut props = protocol::Props::new();
        props.set(protocol::Prop::ApplicationName, CString::new(name)?);
        props.set(
            protocol::Prop::ApplicationId,
            CString::new("io.github.alexparlett.Isle")?,
        );
        conn.send(Command::SetClientName(props)).await?;
        let frame = read_frame(&mut reader).await?;
        protocol::read_reply_message::<protocol::SetClientNameReply>(
            &mut Cursor::new(frame),
            conn.version,
        )?;

        let conn = Arc::new(conn);
        let (events, received) = mpsc::unbounded_channel();
        let (pending, records, version) =
            (conn.pending.clone(), conn.records.clone(), conn.version);
        tokio::spawn(async move {
            while let Ok((channel, frame)) = read_channel_frame(&mut reader).await {
                if channel != u32::MAX {
                    let payload = frame[protocol::DESCRIPTOR_SIZE..].to_vec();
                    if let Some(sink) = records.lock().unwrap().get(&channel) {
                        let _ = sink.send(payload);
                    }
                    continue;
                }
                let Some((tag, seq)) = tag_and_seq(&frame, version) else {
                    continue;
                };
                if matches!(
                    tag,
                    protocol::CommandTag::Reply | protocol::CommandTag::Error
                ) {
                    if let Some(waiting) = pending.lock().unwrap().remove(&seq) {
                        let _ = waiting.send(frame);
                    }
                    continue;
                }
                if let Ok((_, Command::SubscribeEvent(event))) =
                    protocol::read_command_message(&mut Cursor::new(&frame[..]), version)
                {
                    let _ = events.send(event);
                }
            }
            // Dropping the waiting senders fails every command in flight.
            pending.lock().unwrap().clear();
            records.lock().unwrap().clear();
        });
        Ok((conn, received))
    }

    async fn send(&self, command: Command) -> Result<u32, Error> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        self.write(seq, &command).await?;
        Ok(seq)
    }

    async fn write(&self, seq: u32, command: &Command) -> Result<(), Error> {
        let mut buf = Vec::new();
        protocol::write_command_message(&mut buf, seq, command, self.version)?;
        self.writer.lock().await.write_all(&buf).await?;
        Ok(())
    }

    async fn roundtrip(&self, command: Command) -> Result<Vec<u8>, Error> {
        let (tx, rx) = oneshot::channel();
        // Waiting before the command goes, so the reply cannot arrive first.
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        self.pending.lock().unwrap().insert(seq, tx);
        if let Err(err) = self.write(seq, &command).await {
            self.pending.lock().unwrap().remove(&seq);
            return Err(err);
        }
        match tokio::time::timeout(LIMIT, rx).await {
            Ok(Ok(frame)) => Ok(frame),
            Ok(Err(_)) => Err("the Pulse connection ended".into()),
            Err(_) => {
                self.pending.lock().unwrap().remove(&seq);
                Err("the Pulse server did not answer".into())
            }
        }
    }

    /// A command and its reply.
    pub async fn reply<R: CommandReply>(&self, command: Command) -> Result<R, Error> {
        let frame = self.roundtrip(command).await?;
        Ok(protocol::read_reply_message::<R>(&mut Cursor::new(frame), self.version)?.1)
    }

    /// A command whose reply is an empty acknowledgement.
    pub async fn ack(&self, command: Command) -> Result<(), Error> {
        let frame = self.roundtrip(command).await?;
        protocol::read_ack_message(&mut Cursor::new(frame))?;
        Ok(())
    }

    /// Data arriving on a record stream's channel.
    pub fn record(&self, channel: u32) -> mpsc::UnboundedReceiver<Vec<u8>> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.records.lock().unwrap().insert(channel, tx);
        rx
    }

    pub fn forget_record(&self, channel: u32) {
        self.records.lock().unwrap().remove(&channel);
    }

    /// Writes `data` to an upload stream's channel.
    pub async fn write_data(&self, channel: u32, data: &[u8]) -> Result<(), Error> {
        let mut buf = Vec::new();
        for chunk in data.chunks(64 * 1024) {
            protocol::write_memblock(&mut buf, channel, chunk, 0)?;
        }
        self.writer.lock().await.write_all(&buf).await?;
        Ok(())
    }

    /// Whether the reader still runs.
    pub fn alive(&self) -> bool {
        Arc::strong_count(&self.pending) > 1
    }
}

fn tag_and_seq(frame: &[u8], version: u16) -> Option<(protocol::CommandTag, u32)> {
    let mut payload = &frame[protocol::DESCRIPTOR_SIZE..];
    let mut ts = protocol::TagStructReader::new(&mut payload, version);
    let tag = ts.read_enum().ok()?;
    let seq = ts.read_u32().ok()?;
    Some((tag, seq))
}

async fn read_frame(reader: &mut tokio::net::unix::OwnedReadHalf) -> Result<Vec<u8>, Error> {
    Ok(read_channel_frame(reader).await?.1)
}

/// One message whole, header included, with the channel it is on.
async fn read_channel_frame(
    reader: &mut tokio::net::unix::OwnedReadHalf,
) -> Result<(u32, Vec<u8>), Error> {
    let mut frame = vec![0; protocol::DESCRIPTOR_SIZE];
    reader.read_exact(&mut frame).await?;
    let desc = protocol::read_descriptor(&mut &frame[..])?;
    if desc.length as usize > protocol::MAX_MEMBLOCKQ_LENGTH {
        return Err("a Pulse message too long".into());
    }
    frame.resize(protocol::DESCRIPTOR_SIZE + desc.length as usize, 0);
    reader
        .read_exact(&mut frame[protocol::DESCRIPTOR_SIZE..])
        .await?;
    Ok((desc.channel, frame))
}
