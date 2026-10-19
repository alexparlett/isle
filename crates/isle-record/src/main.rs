//! Isle's screen recorder: reads one of the compositor's PipeWire video nodes, and the
//! computer's sound and a microphone when asked, and writes H.264 and AAC into a fragmented
//! MP4 (`p11-capture.md` choices 14–15): its index leads the file, so players start at once,
//! and each frame is a fragment flushed as it goes, so a recording cut short still plays.
//!
//! `isle-record --node <id> --out <path> [--fps <n>] [--sink <name>] [--mic <name>]`; it
//! stops on `stop` or the end of its input, and prints `ready`, `recording <w>x<h> <encoder>`,
//! `done <path>` or `error <why>`.

use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{BufRead, Cursor},
    path::PathBuf,
    rc::Rc,
    time::Instant,
};

use ffmpeg_next as ffmpeg;
use pipewire::{
    self as pw,
    context::ContextRc,
    core::CoreRc,
    main_loop::MainLoopRc,
    properties::properties,
    spa::{
        param::{
            ParamType,
            audio::{AudioFormat, AudioInfoRaw},
            format::{FormatProperties, MediaSubtype, MediaType},
            video::{VideoFormat, VideoInfoRaw},
        },
        pod::{self, Pod, serialize::PodSerializer},
        utils::{Direction, Fraction, Rectangle, SpaTypes},
    },
    stream::{StreamFlags, StreamRc},
};

const RATE: u32 = 48_000;
const CHANNELS: usize = 2;

struct Args {
    node: u32,
    out: PathBuf,
    fps: u32,
    sink: Option<String>,
    mic: Option<String>,
}

fn args() -> Result<Args, String> {
    let all: Vec<String> = std::env::args().collect();
    let get = |name: &str| {
        all.iter()
            .position(|a| a == name)
            .and_then(|i| all.get(i + 1))
            .cloned()
    };
    Ok(Args {
        node: get("--node")
            .and_then(|n| n.parse().ok())
            .ok_or("--node <id>")?,
        out: get("--out").map(PathBuf::from).ok_or("--out <path>")?,
        fps: get("--fps")
            .and_then(|n| n.parse().ok())
            .unwrap_or(60)
            .clamp(1, 240),
        sink: get("--sink").filter(|s| !s.is_empty()),
        mic: get("--mic").filter(|s| !s.is_empty()),
    })
}

/// The encoders, open once the video's size is known.
struct Encoding {
    out: ffmpeg::format::context::Output,
    video: ffmpeg::encoder::Video,
    video_index: usize,
    scaler: ffmpeg::software::scaling::Context,
    source: ffmpeg::frame::Video,
    scaled: ffmpeg::frame::Video,
    audio: Option<AudioEncoding>,
    started: Instant,
}

struct AudioEncoding {
    encoder: ffmpeg::encoder::Audio,
    index: usize,
    /// Interleaved stereo samples waiting for a whole encoder frame.
    pending: VecDeque<f32>,
    written: i64,
}

struct Recorder {
    path: PathBuf,
    fps: u32,
    size: (u32, u32),
    alpha: bool,
    encoding: Option<Encoding>,
    /// The computer's sound and the microphone, each as interleaved stereo, mixed when both
    /// have arrived.
    sound: VecDeque<f32>,
    voice: VecDeque<f32>,
    has_sound: bool,
    has_voice: bool,
    failed: Option<String>,
}

fn video_format() -> pod::Object {
    pod::Object {
        type_: SpaTypes::ObjectParamFormat.as_raw(),
        id: ParamType::EnumFormat.as_raw(),
        properties: vec![
            pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
            pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
            pod::property!(
                FormatProperties::VideoFormat,
                Choice,
                Enum,
                Id,
                VideoFormat::BGRx,
                VideoFormat::BGRx,
                VideoFormat::BGRA
            ),
            pod::property!(
                FormatProperties::VideoSize,
                Choice,
                Range,
                Rectangle,
                Rectangle {
                    width: 1920,
                    height: 1080
                },
                Rectangle {
                    width: 2,
                    height: 2
                },
                Rectangle {
                    width: 8192,
                    height: 8192
                }
            ),
            pod::property!(
                FormatProperties::VideoFramerate,
                Choice,
                Range,
                Fraction,
                Fraction { num: 60, denom: 1 },
                Fraction { num: 0, denom: 1 },
                Fraction { num: 240, denom: 1 }
            ),
        ],
    }
}

fn audio_format() -> pod::Object {
    let mut info = AudioInfoRaw::new();
    info.set_format(AudioFormat::F32LE);
    info.set_rate(RATE);
    info.set_channels(CHANNELS as u32);
    let mut position = [0; pw::spa::param::audio::MAX_CHANNELS];
    position[0] = pw::spa::sys::SPA_AUDIO_CHANNEL_FL;
    position[1] = pw::spa::sys::SPA_AUDIO_CHANNEL_FR;
    info.set_position(position);
    pod::Object {
        type_: SpaTypes::ObjectParamFormat.as_raw(),
        id: ParamType::EnumFormat.as_raw(),
        properties: info.into(),
    }
}

fn serialise(object: pod::Object) -> Vec<u8> {
    let mut bytes = Vec::new();
    PodSerializer::serialize(Cursor::new(&mut bytes), &pod::Value::Object(object)).unwrap();
    bytes
}

impl Recorder {
    /// Opens the file and the encoders at the first frame: NVENC when it opens, else x264.
    fn open(&mut self) -> Result<(), String> {
        let (w, h) = self.size;
        let mut out = ffmpeg::format::output_as(&self.path, "mp4").map_err(|e| e.to_string())?;
        let global = out
            .format()
            .flags()
            .contains(ffmpeg::format::Flags::GLOBAL_HEADER);
        let mut opened = None;
        for (name, pixel) in [
            ("h264_nvenc", ffmpeg::format::Pixel::NV12),
            ("libx264", ffmpeg::format::Pixel::YUV420P),
        ] {
            let Some(codec) = ffmpeg::encoder::find_by_name(name) else {
                continue;
            };
            let mut encoder = ffmpeg::codec::context::Context::new_with_codec(codec)
                .encoder()
                .video()
                .map_err(|e| e.to_string())?;
            encoder.set_width(w);
            encoder.set_height(h);
            encoder.set_format(pixel);
            encoder.set_time_base((1, 1000));
            encoder.set_frame_rate(Some((self.fps as i32, 1)));
            encoder.set_gop(self.fps * 2);
            if global {
                encoder.set_flags(ffmpeg::codec::Flags::GLOBAL_HEADER);
            }
            let mut options = ffmpeg::Dictionary::new();
            if name == "h264_nvenc" {
                options.set("preset", "p4");
                options.set("rc", "vbr");
                options.set("cq", "23");
            } else {
                options.set("preset", "veryfast");
                options.set("crf", "20");
            }
            options.set("profile", "high");
            match encoder.open_with(options) {
                Ok(encoder) => {
                    opened = Some((name, pixel, encoder));
                    break;
                }
                Err(err) => eprintln!("{name}: {err}"),
            }
        }
        let (name, pixel, video) = opened.ok_or("no H.264 encoder opens")?;
        let video_index = {
            let mut stream = out
                .add_stream(ffmpeg::encoder::find_by_name(name))
                .map_err(|e| e.to_string())?;
            stream.set_parameters(&video);
            stream.set_time_base((1, 1000));
            stream.index()
        };
        let audio = if self.has_sound || self.has_voice {
            let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::AAC).ok_or("no AAC encoder")?;
            let mut encoder = ffmpeg::codec::context::Context::new_with_codec(codec)
                .encoder()
                .audio()
                .map_err(|e| e.to_string())?;
            encoder.set_rate(RATE as i32);
            encoder.set_channel_layout(ffmpeg::ChannelLayout::STEREO);
            encoder.set_format(ffmpeg::format::Sample::F32(
                ffmpeg::format::sample::Type::Planar,
            ));
            encoder.set_bit_rate(160_000);
            encoder.set_time_base((1, RATE as i32));
            if global {
                encoder.set_flags(ffmpeg::codec::Flags::GLOBAL_HEADER);
            }
            let encoder = encoder.open_as(codec).map_err(|e| e.to_string())?;
            let mut stream = out.add_stream(codec).map_err(|e| e.to_string())?;
            stream.set_parameters(&encoder);
            stream.set_time_base((1, RATE as i32));
            Some(AudioEncoding {
                encoder,
                index: stream.index(),
                pending: VecDeque::new(),
                written: 0,
            })
        } else {
            None
        };
        let mut options = ffmpeg::Dictionary::new();
        // A fragment per frame, each flushed: a recording cut short still plays up to its
        // last frame, however rarely a still screen sends one.
        options.set("movflags", "frag_every_frame+empty_moov+default_base_moof");
        options.set("flush_packets", "1");
        out.write_header_with(options).map_err(|e| e.to_string())?;
        let input = if self.alpha {
            ffmpeg::format::Pixel::BGRA
        } else {
            ffmpeg::format::Pixel::BGRZ
        };
        let scaler = ffmpeg::software::scaling::Context::get(
            input,
            w,
            h,
            pixel,
            w,
            h,
            ffmpeg::software::scaling::Flags::BILINEAR,
        )
        .map_err(|e| e.to_string())?;
        println!("recording {w}x{h} {name}");
        self.encoding = Some(Encoding {
            out,
            video,
            video_index,
            scaler,
            source: ffmpeg::frame::Video::new(input, w, h),
            scaled: ffmpeg::frame::Video::new(pixel, w, h),
            audio,
            started: Instant::now(),
        });
        Ok(())
    }

    /// One frame of `stride`-byte rows.
    fn frame(&mut self, bytes: &[u8], stride: usize) -> Result<(), String> {
        if self.encoding.is_none() {
            self.open()?;
        }
        let e = self.encoding.as_mut().unwrap();
        let (w, h) = self.size;
        let row = w as usize * 4;
        let line = e.source.stride(0);
        let data = e.source.data_mut(0);
        for y in 0..h as usize {
            let from = y * stride;
            if from + row > bytes.len() {
                break;
            }
            data[y * line..y * line + row].copy_from_slice(&bytes[from..from + row]);
        }
        e.scaler
            .run(&e.source, &mut e.scaled)
            .map_err(|err| err.to_string())?;
        e.scaled
            .set_pts(Some(e.started.elapsed().as_millis() as i64));
        e.video
            .send_frame(&e.scaled)
            .map_err(|err| err.to_string())?;
        drain_video(e)
    }

    /// Mixes what both sources have and encodes whole AAC frames.
    fn sound(&mut self) -> Result<(), String> {
        let Some(e) = self.encoding.as_mut() else {
            // Sound before the first frame is not kept.
            self.sound.clear();
            self.voice.clear();
            return Ok(());
        };
        let Some(audio) = e.audio.as_mut() else {
            return Ok(());
        };
        let n = match (self.has_sound, self.has_voice) {
            (true, true) => self.sound.len().min(self.voice.len()),
            (true, false) => self.sound.len(),
            (false, true) => self.voice.len(),
            (false, false) => 0,
        };
        for _ in 0..n {
            let a = if self.has_sound {
                self.sound.pop_front().unwrap_or(0.0)
            } else {
                0.0
            };
            let b = if self.has_voice {
                self.voice.pop_front().unwrap_or(0.0)
            } else {
                0.0
            };
            audio.pending.push_back((a + b).clamp(-1.0, 1.0));
        }
        let size = audio.encoder.frame_size().max(1024) as usize;
        while audio.pending.len() >= size * CHANNELS {
            let mut frame = ffmpeg::frame::Audio::new(
                ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar),
                size,
                ffmpeg::ChannelLayout::STEREO,
            );
            frame.set_rate(RATE);
            for i in 0..size {
                let l = audio.pending.pop_front().unwrap();
                let r = audio.pending.pop_front().unwrap();
                frame.plane_mut::<f32>(0)[i] = l;
                frame.plane_mut::<f32>(1)[i] = r;
            }
            frame.set_pts(Some(audio.written));
            audio.written += size as i64;
            audio
                .encoder
                .send_frame(&frame)
                .map_err(|err| err.to_string())?;
            let mut packet = ffmpeg::Packet::empty();
            while audio.encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream(audio.index);
                let tb = e.out.stream(audio.index).unwrap().time_base();
                packet.rescale_ts((1, RATE as i32), tb);
                packet
                    .write_interleaved(&mut e.out)
                    .map_err(|err| err.to_string())?;
            }
        }
        Ok(())
    }

    /// Flushes the encoders and closes the file.
    fn finish(&mut self) -> Result<bool, String> {
        let Some(mut e) = self.encoding.take() else {
            return Ok(false);
        };
        e.video.send_eof().map_err(|err| err.to_string())?;
        drain_video(&mut e)?;
        if let Some(audio) = e.audio.as_mut() {
            audio.encoder.send_eof().map_err(|err| err.to_string())?;
            let mut packet = ffmpeg::Packet::empty();
            while audio.encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream(audio.index);
                let tb = e.out.stream(audio.index).unwrap().time_base();
                packet.rescale_ts((1, RATE as i32), tb);
                packet
                    .write_interleaved(&mut e.out)
                    .map_err(|err| err.to_string())?;
            }
        }
        e.out.write_trailer().map_err(|err| err.to_string())?;
        Ok(true)
    }
}

fn drain_video(e: &mut Encoding) -> Result<(), String> {
    let mut packet = ffmpeg::Packet::empty();
    while e.video.receive_packet(&mut packet).is_ok() {
        packet.set_stream(e.video_index);
        let tb = e.out.stream(e.video_index).unwrap().time_base();
        packet.rescale_ts((1, 1000), tb);
        packet
            .write_interleaved(&mut e.out)
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

/// With no session manager to do it, a stream links itself: `from`'s output ports to
/// `to`'s input ports, by channel where they have one.
struct Linker {
    core: CoreRc,
    /// Ports by node: (port id, direction, channel, monitor).
    ports: Vec<(u32, u32, String, String, bool)>,
    /// Node ids by name.
    names: Vec<(String, u32)>,
    wanted: Vec<Wanted>,
    links: Vec<pw::link::Link>,
}

/// A link to make once both ends have ports: from a node by id or name, to a node by id.
struct Wanted {
    from: From,
    to: Rc<std::cell::Cell<Option<u32>>>,
    /// Take the sink's monitor ports.
    monitor: bool,
    done: bool,
}

enum From {
    Id(u32),
    Name(String),
}

impl Linker {
    fn try_links(&mut self) {
        for i in 0..self.wanted.len() {
            if self.wanted[i].done {
                continue;
            }
            let from = match &self.wanted[i].from {
                From::Id(id) => Some(*id),
                From::Name(name) => self
                    .names
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, id)| *id),
            };
            let (Some(from), Some(to)) = (from, self.wanted[i].to.get()) else {
                continue;
            };
            let monitor = self.wanted[i].monitor;
            // A sink's monitor ports, or a source's own; a virtual source has only monitors.
            let outs_of = |mon: bool| -> Vec<(u32, String)> {
                self.ports
                    .iter()
                    .filter(|(_, node, dir, _, m)| *node == from && dir == "out" && *m == mon)
                    .map(|(id, _, _, ch, _)| (*id, ch.clone()))
                    .collect()
            };
            let mut outs = outs_of(monitor);
            if outs.is_empty() && !monitor {
                outs = outs_of(true);
            }
            let ins: Vec<(u32, String)> = self
                .ports
                .iter()
                .filter(|(_, node, dir, _, _)| *node == to && dir == "in")
                .map(|(id, _, _, ch, _)| (*id, ch.clone()))
                .collect();
            if outs.is_empty() || ins.is_empty() {
                continue;
            }
            for (input, channel) in &ins {
                // By channel, a mono output to every input, else in order.
                let out = outs
                    .iter()
                    .find(|(_, c)| c == channel && !c.is_empty())
                    .or_else(|| (outs.len() == 1).then(|| &outs[0]))
                    .or_else(|| outs.get(ins.iter().position(|(i, _)| i == input).unwrap_or(0)));
                let Some((output, _)) = out else { continue };
                match self.core.create_object::<pw::link::Link>(
                    "link-factory",
                    &properties! {
                        "link.output.node" => from.to_string(),
                        "link.output.port" => output.to_string(),
                        "link.input.node" => to.to_string(),
                        "link.input.port" => input.to_string(),
                        "object.linger" => "false",
                    },
                ) {
                    Ok(link) => self.links.push(link),
                    Err(err) => eprintln!("a link: {err}"),
                }
            }
            self.wanted[i].done = true;
        }
    }
}

fn main() {
    // Started through a descriptor, the kernel names the process by its number.
    unsafe { libc::prctl(libc::PR_SET_NAME, c"isle-record".as_ptr(), 0, 0, 0) };
    let args = match args() {
        Ok(args) => args,
        Err(err) => {
            println!("error {err}");
            std::process::exit(2);
        }
    };
    if let Err(err) = ffmpeg::init() {
        println!("error {err}");
        std::process::exit(1);
    }
    pw::init();
    let main_loop = MainLoopRc::new(None).unwrap();
    // As in the compositor: no realtime module, no D-Bus.
    let context = ContextRc::new(
        &main_loop,
        Some(properties! { "module.rt" => "false", "support.dbus" => "false" }),
    )
    .unwrap();
    let core = match context.connect_rc(None) {
        Ok(core) => core,
        Err(err) => {
            println!("error {err}");
            std::process::exit(1);
        }
    };
    let recorder = Rc::new(RefCell::new(Recorder {
        path: args.out.clone(),
        fps: args.fps,
        size: (0, 0),
        alpha: false,
        encoding: None,
        sound: VecDeque::new(),
        voice: VecDeque::new(),
        has_sound: args.sink.is_some(),
        has_voice: args.mic.is_some(),
        failed: None,
    }));
    let linker = Rc::new(RefCell::new(Linker {
        core: core.clone(),
        ports: Vec::new(),
        names: Vec::new(),
        wanted: Vec::new(),
        links: Vec::new(),
    }));

    let registry = core.get_registry_rc().unwrap();
    let _registry = registry
        .add_listener_local()
        .global({
            let linker = linker.clone();
            move |global| {
                let Some(props) = global.props else { return };
                let mut l = linker.borrow_mut();
                match global.type_ {
                    pw::types::ObjectType::Node => {
                        if let Some(name) = props.get("node.name") {
                            l.names.push((name.to_owned(), global.id));
                        }
                    }
                    pw::types::ObjectType::Port => {
                        let Some(node) = props.get("node.id").and_then(|n| n.parse().ok()) else {
                            return;
                        };
                        l.ports.push((
                            global.id,
                            node,
                            props.get("port.direction").unwrap_or_default().to_owned(),
                            props.get("audio.channel").unwrap_or_default().to_owned(),
                            props.get("port.monitor") == Some("true"),
                        ));
                    }
                    _ => return,
                }
                l.try_links();
            }
        })
        .register();

    let quit = main_loop.clone();
    let fail = {
        let recorder = recorder.clone();
        move |why: String| {
            recorder.borrow_mut().failed.get_or_insert(why);
            quit.quit();
        }
    };

    // The video.
    let video = StreamRc::new(
        core.clone(),
        "isle-record",
        properties! { "media.class" => "Stream/Input/Video", "node.name" => "isle-record-video" },
    )
    .unwrap();
    let video_node: Rc<std::cell::Cell<Option<u32>>> = Rc::default();
    linker.borrow_mut().wanted.push(Wanted {
        from: From::Id(args.node),
        to: video_node.clone(),
        monitor: false,
        done: false,
    });
    let _video = video
        .add_local_listener_with_user_data(())
        .state_changed({
            let node = video_node.clone();
            let linker = linker.clone();
            let fail = fail.clone();
            move |stream, (), _, new| match new {
                pw::stream::StreamState::Paused => {
                    node.set(Some(stream.node_id()));
                    linker.borrow_mut().try_links();
                }
                pw::stream::StreamState::Error(err) => fail(err),
                _ => {}
            }
        })
        .param_changed({
            let recorder = recorder.clone();
            move |_, (), id, pod| {
                if ParamType::from_raw(id) != ParamType::Format {
                    return;
                }
                let Some(pod) = pod else { return };
                let mut info = VideoInfoRaw::new();
                if info.parse(pod).is_ok() {
                    let mut r = recorder.borrow_mut();
                    // A size change after the first frame is not followed.
                    if r.encoding.is_none() {
                        r.size = (info.size().width & !1, info.size().height & !1);
                        r.alpha = info.format() == VideoFormat::BGRA;
                    }
                }
            }
        })
        .process({
            let recorder = recorder.clone();
            let fail = fail.clone();
            move |stream, ()| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let datas = buffer.datas_mut();
                let Some(data) = datas.first_mut() else {
                    return;
                };
                let chunk = data.chunk();
                let (offset, stride, size) = (
                    chunk.offset() as usize,
                    chunk.stride() as usize,
                    chunk.size() as usize,
                );
                if size == 0 || stride == 0 {
                    return;
                }
                let Some(bytes) = data.data() else { return };
                let bytes = &bytes[offset.min(bytes.len())..];
                let result = recorder.borrow_mut().frame(bytes, stride);
                if let Err(err) = result {
                    fail(err);
                }
            }
        })
        .register()
        .unwrap();
    let format = serialise(video_format());
    video
        .connect(
            Direction::Input,
            None,
            StreamFlags::MAP_BUFFERS,
            &mut [Pod::from_bytes(&format).unwrap()],
        )
        .unwrap();

    // The sound: the default output's monitor and the microphone, each its own stream.
    let mut sound_streams = Vec::new();
    for (name, monitor) in [(args.sink.clone(), true), (args.mic.clone(), false)] {
        let Some(name) = name else { continue };
        let stream = StreamRc::new(
            core.clone(),
            "isle-record",
            properties! {
                "media.class" => "Stream/Input/Audio",
                "node.name" => if monitor { "isle-record-sound" } else { "isle-record-voice" },
                "stream.capture.sink" => if monitor { "true" } else { "false" },
                // With no session manager to configure them, the stream's ports configure
                // themselves.
                "adapter.auto-port-config" => "{ mode = dsp monitor = false position = preserve }",
            },
        )
        .unwrap();
        let node: Rc<std::cell::Cell<Option<u32>>> = Rc::default();
        linker.borrow_mut().wanted.push(Wanted {
            from: From::Name(name),
            to: node.clone(),
            monitor,
            done: false,
        });
        let listener = stream
            .add_local_listener_with_user_data(())
            .state_changed({
                let node = node.clone();
                let linker = linker.clone();
                move |stream, (), _, new| {
                    if let pw::stream::StreamState::Paused = new {
                        node.set(Some(stream.node_id()));
                        linker.borrow_mut().try_links();
                    }
                }
            })
            .process({
                let recorder = recorder.clone();
                let fail = fail.clone();
                move |stream, ()| {
                    let Some(mut buffer) = stream.dequeue_buffer() else {
                        return;
                    };
                    let datas = buffer.datas_mut();
                    let Some(data) = datas.first_mut() else {
                        return;
                    };
                    let size = data.chunk().size() as usize;
                    let Some(bytes) = data.data() else { return };
                    let samples = bytes[..size.min(bytes.len())]
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
                    let mut r = recorder.borrow_mut();
                    if monitor {
                        r.sound.extend(samples);
                    } else {
                        r.voice.extend(samples);
                    }
                    let result = r.sound();
                    drop(r);
                    if let Err(err) = result {
                        fail(err);
                    }
                }
            })
            .register()
            .unwrap();
        let format = serialise(audio_format());
        stream
            .connect(
                Direction::Input,
                None,
                StreamFlags::MAP_BUFFERS,
                &mut [Pod::from_bytes(&format).unwrap()],
            )
            .unwrap();
        sound_streams.push((stream, listener));
    }

    // `stop`, or the end of the input, ends the recording.
    let (stop_tx, stop_rx) = pw::channel::channel::<()>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            match line {
                Ok(l) if l.trim() == "stop" => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = stop_tx.send(());
    });
    let quit = main_loop.clone();
    let _stop = stop_rx.attach(main_loop.loop_(), move |()| quit.quit());
    println!("ready");
    main_loop.run();

    let mut r = recorder.borrow_mut();
    if let Some(err) = r.failed.take() {
        let _ = r.finish();
        println!("error {err}");
        std::process::exit(1);
    }
    match r.finish() {
        Ok(true) => println!("done {}", args.out.display()),
        Ok(false) => {
            println!("error no frame arrived");
            std::process::exit(1);
        }
        Err(err) => {
            println!("error {err}");
            std::process::exit(1);
        }
    }
}
