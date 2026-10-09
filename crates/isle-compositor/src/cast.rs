//! Screen streams: PipeWire video nodes Isle produces itself, a screen, a portion of one or a
//! window, rendered on the loop with the compositor's own context (`p11-capture.md` choices
//! 11–13). PipeWire's loop is a calloop source; nothing here runs on a thread of its own.

use std::{
    cell::RefCell,
    collections::HashMap,
    io::Cursor,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
    ptr::NonNull,
    rc::Rc,
    time::{Duration, Instant},
};

use pipewire::{
    self as pw,
    context::ContextRc,
    core::CoreRc,
    main_loop::MainLoopRc,
    properties::properties,
    registry::RegistryRc,
    spa::{
        self,
        buffer::DataType,
        param::{
            ParamType,
            format::{FormatProperties, MediaSubtype, MediaType},
            format_utils::parse_format,
            video::{VideoFormat, VideoInfoRaw},
        },
        pod::{
            self, ChoiceValue, Pod, PodPropFlags, Property, PropertyFlags,
            deserialize::PodDeserializer, serialize::PodSerializer,
        },
        sys::*,
        utils::{
            Choice, ChoiceEnum, ChoiceFlags, Direction, Fraction, Rectangle as SpaRect, SpaTypes,
        },
    },
    stream::{StreamFlags, StreamListener, StreamRc, StreamState},
    sys::pw_buffer,
};
use smithay::{
    backend::{
        allocator::{
            Fourcc, Modifier,
            dmabuf::{AsDmabuf, Dmabuf},
            format::FormatSet,
            gbm::{GbmBuffer, GbmBufferFlags, GbmDevice},
        },
        drm::DrmDeviceFd,
        renderer::{
            Bind, ExportMem, Offscreen, Renderer,
            damage::OutputDamageTracker,
            element::utils::{Relocate, RelocateRenderElement},
            gles::{GlesRenderer, GlesTexture},
            sync::SyncPoint,
        },
    },
    output::Output,
    reexports::calloop::{
        self, Interest, Mode, PostAction, RegistrationToken,
        channel::{Channel, Sender},
        generic::Generic,
        timer::{TimeoutAction, Timer},
    },
    utils::{Logical, Physical, Point, Rectangle, Size, Transform},
};
use tracing::{debug, info, warn};

use crate::{
    render::IsleElement,
    state::{Isle, State},
};

const SHM_BYTES_PER_PIXEL: u32 = 4;
/// The side of the square a cursor's bitmap is drawn into, in pixels.
pub const CURSOR_SIDE: i32 = 128;
const CURSOR_META_SIZE: usize = size_of::<spa_meta_cursor>()
    + size_of::<spa_meta_bitmap>()
    + (CURSOR_SIDE * CURSOR_SIDE * 4) as usize;

/// How long a click's mark takes to fade from the recording.
const CLICK_FADE: Duration = Duration::from_millis(600);

/// What a stream shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Screen(Output),
    /// Part of a screen, in points from its top left.
    Portion(Output, Rectangle<i32, Logical>),
    Window(u64),
}

/// How the pointer reaches the consumer: left out, drawn into the frames, or sent beside
/// them as cursor metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorMode {
    Hidden,
    Embedded,
    Metadata,
}

/// Why a stream ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// The window closed.
    WindowClosed,
    /// The screen went.
    ScreenGone,
    /// The consumer or PipeWire went.
    Disconnected,
    /// Stopped from Isle's side.
    Stopped,
}

impl Ended {
    pub fn reason(self) -> &'static str {
        match self {
            Ended::WindowClosed => "the window closed",
            Ended::ScreenGone => "the screen went",
            Ended::Disconnected => "the app stopped",
            Ended::Stopped => "stopped",
        }
    }
}

/// From PipeWire's callbacks to the loop.
enum Event {
    Paused {
        id: u64,
        node: u32,
    },
    Streaming {
        id: u64,
    },
    Failed {
        id: u64,
    },
    Redraw {
        id: u64,
    },
    Serial {
        node: u32,
        serial: u64,
    },
    /// Isle ends it, for `why`.
    End {
        id: u64,
        why: Ended,
    },
    Fatal,
}

/// The connection to PipeWire, made at the first stream.
struct Connection {
    _context: ContextRc,
    core: CoreRc,
    _registry: RegistryRc,
    _listeners: (pw::core::Listener, pw::registry::Listener),
    loop_token: RegistrationToken,
    events: Sender<Event>,
    /// Node serials by node id, from the registry.
    serials: Rc<RefCell<HashMap<u32, u64>>>,
}

#[derive(Default)]
pub struct Casting {
    connection: Option<Connection>,
    pub streams: Vec<Stream>,
    next: u64,
    /// Where dmabufs for streams are allocated, and what the renderer draws into.
    pub gbm: Option<(GbmDevice<DrmDeviceFd>, FormatSet)>,
    /// Each frame's render on the loop in microseconds, while measuring.
    pub times: Vec<u64>,
    /// The pointer's image as metadata streams send it, and its serial.
    cursor: Option<CursorImage>,
    cursor_serial: u64,
}

/// What a cursor bitmap was drawn from.
#[derive(Clone, Debug, PartialEq)]
enum CursorKey {
    Named(smithay::input::pointer::CursorIcon, u32),
    Surface(smithay::reexports::wayland_server::backend::ObjectId, u32),
}

struct CursorImage {
    key: CursorKey,
    serial: u64,
    /// `None` when the backend has no image for it.
    image: Option<crate::backend::Pixels>,
    hotspot: Point<i32, Physical>,
}

/// The cursor a metadata stream sends with a buffer: where, and the bitmap's serial.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SentCursor {
    at: Point<i32, Physical>,
    serial: u64,
}

/// The negotiated buffers' kind and layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Negotiated {
    Shm { stride: i32, size: u32 },
    Dma { modifier: Modifier, planes: usize },
}

struct Inner {
    node: Option<u32>,
    streaming: bool,
    size: Size<u32, Physical>,
    alpha: bool,
    negotiated: Option<Negotiated>,
    min_interval: Duration,
    dmabufs: HashMap<i64, Dmabuf>,
    shmbufs: HashMap<i64, Shm>,
}

struct Shm {
    _fd: OwnedFd,
    map: NonNull<u8>,
    len: usize,
}

impl Drop for Shm {
    fn drop(&mut self) {
        // SAFETY: `map` is this buffer's mapping of `len` bytes, unmapped once.
        unsafe {
            libc::munmap(self.map.as_ptr().cast(), self.len);
        }
    }
}

pub struct Stream {
    pub id: u64,
    pub source: Source,
    pub cursor: CursorMode,
    /// Started by: `recording`, `portal`, `test`.
    pub owner: &'static str,
    pub started: Instant,
    stream: StreamRc,
    _listener: StreamListener<()>,
    inner: Rc<RefCell<Inner>>,
    damage: Option<OutputDamageTracker>,
    /// A texture of the stream's size, rendered into for shared-memory buffers.
    scratch: Option<GlesTexture>,
    last_queued: Option<Instant>,
    later: Option<RegistrationToken>,
    seq: u64,
    /// Buffers rendered and waiting for their fence, in order.
    rendering: Vec<(NonNull<pw_buffer>, SyncPoint)>,
    pub frames: u64,
    /// Metadata mode: where the pointer was last sent, and the bitmap's serial sent.
    cursor_sent: Option<Point<i32, Physical>>,
    bitmap_sent: u64,
}

impl Stream {
    pub fn node(&self) -> Option<u32> {
        self.inner.borrow().node
    }

    pub fn streaming(&self) -> bool {
        self.inner.borrow().streaming
    }

    pub fn size(&self) -> Size<u32, Physical> {
        self.inner.borrow().size
    }

    pub fn kind(&self) -> &'static str {
        match self.inner.borrow().negotiated {
            Some(Negotiated::Shm { .. }) => "shm",
            Some(Negotiated::Dma { .. }) => "dmabuf",
            None => "none",
        }
    }
}

impl Casting {
    /// The cursor bitmap `State::cursor_bitmap` last drew, and its hotspot.
    pub fn cursor_image(&self) -> Option<(Option<&crate::backend::Pixels>, Point<i32, Physical>)> {
        self.cursor.as_ref().map(|c| (c.image.as_ref(), c.hotspot))
    }

    pub fn serial(&self, node: u32) -> Option<u64> {
        self.connection
            .as_ref()
            .and_then(|c| c.serials.borrow().get(&node).copied())
    }

    pub fn get(&self, id: u64) -> Option<&Stream> {
        self.streams.iter().find(|s| s.id == id)
    }
}

/// A pod serialised into `buffer`.
fn make_pod(buffer: &mut Vec<u8>, object: pod::Object) -> &Pod {
    PodSerializer::serialize(Cursor::new(&mut *buffer), &pod::Value::Object(object)).unwrap();
    Pod::from_bytes(buffer).unwrap()
}

/// One format the stream offers: `format` at `size`, with `modifiers` for a dmabuf, or none
/// for shared memory.
fn video_format(
    format: VideoFormat,
    modifiers: &[i64],
    size: Size<u32, Physical>,
    fps: u32,
) -> pod::Object {
    let mut properties = vec![
        pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
        pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
        pod::property!(FormatProperties::VideoFormat, Id, format),
        pod::property!(
            FormatProperties::VideoSize,
            Rectangle,
            SpaRect {
                width: size.w,
                height: size.h,
            }
        ),
        pod::property!(
            FormatProperties::VideoFramerate,
            Fraction,
            Fraction { num: 0, denom: 1 }
        ),
        pod::property!(
            FormatProperties::VideoMaxFramerate,
            Choice,
            Range,
            Fraction,
            Fraction { num: fps, denom: 1 },
            Fraction { num: 1, denom: 1 },
            Fraction { num: fps, denom: 1 }
        ),
    ];
    if !modifiers.is_empty() {
        let flags = if modifiers.len() > 1 {
            PropertyFlags::MANDATORY | PropertyFlags::DONT_FIXATE
        } else {
            PropertyFlags::MANDATORY
        };
        properties.push(Property {
            key: FormatProperties::VideoModifier.as_raw(),
            flags,
            value: pod::Value::Choice(ChoiceValue::Long(Choice(
                ChoiceFlags::empty(),
                ChoiceEnum::Enum {
                    default: modifiers[0],
                    alternatives: modifiers.to_vec(),
                },
            ))),
        });
    }
    pod::Object {
        type_: SpaTypes::ObjectParamFormat.as_raw(),
        id: ParamType::EnumFormat.as_raw(),
        properties,
    }
}

/// Every format offered at the start: for each colour format, LINEAR first, then each
/// modifier the renderer draws to, then shared memory.
fn offered(formats: &FormatSet, size: Size<u32, Physical>, fps: u32) -> Vec<pod::Object> {
    let mut out = Vec::new();
    for (video, fourcc) in [
        (VideoFormat::BGRx, Fourcc::Xrgb8888),
        (VideoFormat::BGRA, Fourcc::Argb8888),
    ] {
        let mut modifiers: Vec<i64> = formats
            .iter()
            .filter(|f| f.code == fourcc && f.modifier != Modifier::Invalid)
            .map(|f| u64::from(f.modifier) as i64)
            .collect();
        modifiers.sort_by_key(|m| *m != u64::from(Modifier::Linear) as i64);
        modifiers.dedup();
        if !modifiers.is_empty() {
            out.push(video_format(video, &modifiers, size, fps));
        }
        out.push(video_format(video, &[], size, fps));
    }
    out
}

/// A test allocation of `size` with the first of `modifiers` GBM takes.
fn allocate(
    gbm: &GbmDevice<DrmDeviceFd>,
    size: Size<u32, Physical>,
    fourcc: Fourcc,
    modifiers: &[i64],
) -> Result<Dmabuf, String> {
    let modifiers = modifiers.iter().map(|m| Modifier::from(*m as u64));
    let bo = gbm
        .create_buffer_object_with_modifiers2::<()>(
            size.w,
            size.h,
            fourcc,
            modifiers,
            GbmBufferFlags::RENDERING,
        )
        .map_err(|e| e.to_string())?;
    GbmBuffer::from_bo(bo, false)
        .export()
        .map_err(|e| e.to_string())
}

/// A sealed memfd of `len` bytes, mapped.
fn shm(len: usize) -> Result<Shm, String> {
    use std::ffi::CString;
    let name = CString::new("isle-cast").unwrap();
    // SAFETY: plain syscalls on a descriptor this function owns; the mapping is checked.
    unsafe {
        let fd = libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING);
        if fd < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        use std::os::fd::FromRawFd;
        let fd = OwnedFd::from_raw_fd(fd);
        if libc::ftruncate(fd.as_raw_fd(), len as libc::off_t) != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        libc::fcntl(
            fd.as_raw_fd(),
            libc::F_ADD_SEALS,
            libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW,
        );
        let map = libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd.as_raw_fd(),
            0,
        );
        if map == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Shm {
            _fd: fd,
            map: NonNull::new(map.cast()).unwrap(),
            len,
        })
    }
}

impl Isle {
    fn cast_connect(&mut self) -> Result<(), String> {
        if self.casting.connection.is_some() {
            return Ok(());
        }
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(pw::init);
        let main_loop = MainLoopRc::new(None).map_err(|e| e.to_string())?;
        // Without the realtime module and D-Bus support, nothing in the compositor loads
        // libdbus, and nothing asks the Realtime portal, which Isle may itself be serving.
        let context = ContextRc::new(
            &main_loop,
            Some(properties! {
                "module.rt" => "false",
                "support.dbus" => "false",
            }),
        )
        .map_err(|e| e.to_string())?;
        let core = context.connect_rc(None).map_err(|e| e.to_string())?;
        let (events, channel): (Sender<Event>, Channel<Event>) = calloop::channel::channel();
        let fatal = events.clone();
        let core_listener = core
            .add_listener_local()
            .error(move |id, _seq, res, message| {
                warn!(id, res, message, "PipeWire");
                if id == pw::core::PW_ID_CORE && res == -libc::EPIPE {
                    let _ = fatal.send(Event::Fatal);
                }
            })
            .register();
        let registry = core.get_registry_rc().map_err(|e| e.to_string())?;
        let serials: Rc<RefCell<HashMap<u32, u64>>> = Rc::default();
        let found = events.clone();
        let registry_listener = registry
            .add_listener_local()
            .global(move |global| {
                if global.type_ != pw::types::ObjectType::Node {
                    return;
                }
                let serial = global
                    .props
                    .and_then(|p| p.get("object.serial"))
                    .and_then(|s| s.parse().ok());
                if let Some(serial) = serial {
                    let _ = found.send(Event::Serial {
                        node: global.id,
                        serial,
                    });
                }
            })
            .register();

        struct LoopFd(MainLoopRc);
        impl AsFd for LoopFd {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.0.loop_().fd()
            }
        }
        let loop_token = self
            .loop_handle
            .insert_source(
                Generic::new(LoopFd(main_loop), Interest::READ, Mode::Level),
                |_, fd, _| {
                    fd.0.loop_().iterate(pw::loop_::Timeout::None);
                    Ok(PostAction::Continue)
                },
            )
            .map_err(|e| e.to_string())?;
        self.loop_handle
            .insert_source(channel, |event, _, state| {
                if let calloop::channel::Event::Msg(event) = event {
                    state.cast_event(event);
                }
            })
            .map_err(|e| e.to_string())?;
        self.casting.connection = Some(Connection {
            _context: context,
            core,
            _registry: registry,
            _listeners: (core_listener, registry_listener),
            loop_token,
            events,
            serials,
        });
        Ok(())
    }

    /// Ends every stream of `output`'s screen or portions, as the loop next runs.
    pub fn end_casts_of_output(&self, output: &Output) {
        let Some(c) = &self.casting.connection else {
            return;
        };
        for s in &self.casting.streams {
            if matches!(&s.source, Source::Screen(o) | Source::Portion(o, _) if o == output) {
                let _ = c.events.send(Event::End {
                    id: s.id,
                    why: Ended::ScreenGone,
                });
            }
        }
    }

    /// The size a source streams at, in pixels.
    pub fn source_size(&self, source: &Source) -> Option<Size<u32, Physical>> {
        let size = match source {
            Source::Screen(output) => output.current_mode()?.size,
            Source::Portion(output, rect) => rect
                .size
                .to_f64()
                .to_physical(output.current_scale().fractional_scale())
                .to_i32_round(),
            Source::Window(id) => {
                let frame = self.frame(*id)?;
                let scale = self.output_of(*id)?.current_scale().fractional_scale();
                frame.size.to_f64().to_physical(scale).to_i32_round()
            }
        };
        // Encoders take even sizes.
        Some(((size.w.max(2) & !1) as u32, (size.h.max(2) & !1) as u32).into())
    }
}

impl State {
    /// Starts a stream of `source`; its node appears once PipeWire has it.
    pub fn start_cast(
        &mut self,
        source: Source,
        cursor: CursorMode,
        owner: &'static str,
    ) -> Result<u64, String> {
        self.isle.cast_connect()?;
        let size = self.isle.source_size(&source).ok_or("no such source")?;
        let fps = match &source {
            Source::Screen(o) | Source::Portion(o, _) => o
                .current_mode()
                .map_or(60, |m| (m.refresh as u32).div_ceil(1000)),
            Source::Window(id) => self
                .isle
                .output_of(*id)
                .and_then(|o| o.current_mode())
                .map_or(60, |m| (m.refresh as u32).div_ceil(1000)),
        }
        .clamp(1, 240);
        let connection = self.isle.casting.connection.as_ref().unwrap();
        let id = self.isle.casting.next;
        self.isle.casting.next += 1;
        let stream = StreamRc::new(
            connection.core.clone(),
            "isle-screen",
            properties! {
                "media.class" => "Video/Source",
                "media.name" => "Isle screen",
                "node.name" => format!("isle-cast-{id}"),
            },
        )
        .map_err(|e| e.to_string())?;
        let inner = Rc::new(RefCell::new(Inner {
            node: None,
            streaming: false,
            size,
            alpha: false,
            negotiated: None,
            min_interval: Duration::from_micros(1_000_000 / fps as u64),
            dmabufs: HashMap::new(),
            shmbufs: HashMap::new(),
        }));
        let events = connection.events.clone();
        let gbm = self.isle.casting.gbm.clone();
        let formats = gbm.as_ref().map(|(_, f)| f.clone()).unwrap_or_default();

        let listener = stream
            .add_local_listener_with_user_data(())
            .state_changed({
                let inner = inner.clone();
                let events = events.clone();
                move |stream, (), old, new| {
                    debug!(id, ?old, ?new, "stream");
                    let mut inner = inner.borrow_mut();
                    match new {
                        StreamState::Paused => {
                            if inner.node.is_none() {
                                let node = stream.node_id();
                                inner.node = Some(node);
                                let _ = events.send(Event::Paused { id, node });
                            }
                            inner.streaming = false;
                        }
                        StreamState::Streaming => {
                            inner.streaming = true;
                            let _ = events.send(Event::Streaming { id });
                        }
                        StreamState::Error(_) => {
                            inner.streaming = false;
                            let _ = events.send(Event::Failed { id });
                        }
                        StreamState::Unconnected | StreamState::Connecting => {}
                    }
                }
            })
            .param_changed({
                let inner = inner.clone();
                let events = events.clone();
                let gbm = gbm.clone();
                move |stream, (), param, pod| {
                    if ParamType::from_raw(param) != ParamType::Format {
                        return;
                    }
                    let Some(pod) = pod else { return };
                    let Ok((MediaType::Video, MediaSubtype::Raw)) = parse_format(pod) else {
                        return;
                    };
                    let mut info = VideoInfoRaw::new();
                    if info.parse(pod).is_err() {
                        return;
                    }
                    let mut inner = inner.borrow_mut();
                    let size: Size<u32, Physical> = (info.size().width, info.size().height).into();
                    if size != inner.size {
                        debug!(id, ?size, "a format of another size; waiting");
                        return;
                    }
                    inner.alpha = info.format() == VideoFormat::BGRA;
                    let rate = info.max_framerate();
                    if rate.num > 0 {
                        inner.min_interval =
                            Duration::from_micros(1_000_000 * rate.denom as u64 / rate.num as u64);
                    }
                    let fourcc = if inner.alpha {
                        Fourcc::Argb8888
                    } else {
                        Fourcc::Xrgb8888
                    };
                    let modifier = pod.as_object().ok().and_then(|o| {
                        o.find_prop(spa::utils::Id(FormatProperties::VideoModifier.0))
                    });
                    let mut params_buf: Vec<Vec<u8>> = Vec::new();
                    let negotiated = match (modifier, &gbm) {
                        (Some(prop), Some((gbm, _)))
                            if prop.flags().contains(PodPropFlags::DONT_FIXATE) =>
                        {
                            // Choose one by a test allocation and offer it alone.
                            let Ok((_, choice)) = PodDeserializer::deserialize_from::<Choice<i64>>(
                                prop.value().as_bytes(),
                            ) else {
                                let _ = events.send(Event::Failed { id });
                                return;
                            };
                            let ChoiceEnum::Enum { alternatives, .. } = choice.1 else {
                                let _ = events.send(Event::Failed { id });
                                return;
                            };
                            let chosen = alternatives
                                .iter()
                                .find(|m| allocate(gbm, size, fourcc, &[**m]).is_ok())
                                .copied();
                            let Some(chosen) = chosen else {
                                warn!(id, "no modifier allocates");
                                let _ = events.send(Event::Failed { id });
                                return;
                            };
                            let video = info.format();
                            let mut b = Vec::new();
                            let fixed = make_pod(
                                &mut b,
                                video_format(
                                    video,
                                    &[chosen],
                                    size,
                                    (1_000_000 / inner.min_interval.as_micros().max(1)) as u32,
                                ),
                            );
                            let mut list = vec![fixed];
                            if let Err(err) = stream.update_params(&mut list) {
                                warn!(id, "params: {err}");
                            }
                            return;
                        }
                        (Some(_), Some((gbm, _))) => {
                            match allocate(gbm, size, fourcc, &[info.modifier() as i64]) {
                                Ok(dmabuf) => Negotiated::Dma {
                                    modifier: Modifier::from(info.modifier()),
                                    planes: dmabuf.num_planes(),
                                },
                                Err(err) => {
                                    warn!(id, "the chosen modifier does not allocate: {err}");
                                    let _ = events.send(Event::Failed { id });
                                    return;
                                }
                            }
                        }
                        _ => Negotiated::Shm {
                            stride: (size.w * SHM_BYTES_PER_PIXEL) as i32,
                            size: size.w * size.h * SHM_BYTES_PER_PIXEL,
                        },
                    };
                    inner.negotiated = Some(negotiated);
                    let (blocks, data_type) = match negotiated {
                        Negotiated::Dma { planes, .. } => (planes as i32, DataType::DmaBuf),
                        Negotiated::Shm { .. } => (1, DataType::MemFd),
                    };
                    let buffers = pod::object!(
                        SpaTypes::ObjectParamBuffers,
                        ParamType::Buffers,
                        Property::new(
                            SPA_PARAM_BUFFERS_buffers,
                            pod::Value::Choice(ChoiceValue::Int(Choice(
                                ChoiceFlags::empty(),
                                ChoiceEnum::Range {
                                    default: 8,
                                    min: 2,
                                    max: 16
                                }
                            ))),
                        ),
                        Property::new(SPA_PARAM_BUFFERS_blocks, pod::Value::Int(blocks)),
                        Property::new(
                            SPA_PARAM_BUFFERS_dataType,
                            pod::Value::Choice(ChoiceValue::Int(Choice(
                                ChoiceFlags::empty(),
                                ChoiceEnum::Flags {
                                    default: 1 << data_type.as_raw(),
                                    flags: vec![1 << data_type.as_raw()],
                                },
                            ))),
                        ),
                    );
                    let header = pod::object!(
                        SpaTypes::ObjectParamMeta,
                        ParamType::Meta,
                        Property::new(
                            SPA_PARAM_META_type,
                            pod::Value::Id(spa::utils::Id(SPA_META_Header))
                        ),
                        Property::new(
                            SPA_PARAM_META_size,
                            pod::Value::Int(size_of::<spa_meta_header>() as i32)
                        ),
                    );
                    let cursor_meta = pod::object!(
                        SpaTypes::ObjectParamMeta,
                        ParamType::Meta,
                        Property::new(
                            SPA_PARAM_META_type,
                            pod::Value::Id(spa::utils::Id(SPA_META_Cursor))
                        ),
                        Property::new(SPA_PARAM_META_size, pod::Value::Int(CURSOR_META_SIZE as i32)),
                    );
                    params_buf.resize(3, Vec::new());
                    let [a, b, c] = &mut params_buf[..] else {
                        unreachable!()
                    };
                    let mut params = vec![make_pod(a, buffers), make_pod(b, header)];
                    if cursor == CursorMode::Metadata {
                        params.push(make_pod(c, cursor_meta));
                    }
                    if let Err(err) = stream.update_params(&mut params) {
                        warn!(id, "params: {err}");
                    }
                }
            })
            .add_buffer({
                let inner = inner.clone();
                let events = events.clone();
                let gbm = gbm.clone();
                move |_stream, (), buffer| {
                    let mut inner = inner.borrow_mut();
                    // SAFETY: PipeWire hands a live buffer with its datas for the negotiated
                    // layout; each is filled once here.
                    let result =
                        unsafe { add_buffer(&mut inner, gbm.as_ref().map(|(g, _)| g), buffer) };
                    if let Err(err) = result {
                        warn!(id, "a buffer: {err}");
                        let _ = events.send(Event::Failed { id });
                    } else {
                        let _ = events.send(Event::Redraw { id });
                    }
                }
            })
            .remove_buffer({
                let inner = inner.clone();
                move |_stream, (), buffer| {
                    // SAFETY: the buffer is still live while PipeWire removes it.
                    unsafe {
                        let data = (*(*buffer).buffer).datas;
                        let fd = (*data).fd;
                        let mut inner = inner.borrow_mut();
                        inner.dmabufs.remove(&fd);
                        inner.shmbufs.remove(&fd);
                    }
                }
            })
            .register()
            .map_err(|e| e.to_string())?;

        let objects = offered(&formats, size, fps);
        let mut bufs: Vec<Vec<u8>> = vec![Vec::new(); objects.len()];
        let mut params: Vec<&Pod> = objects
            .into_iter()
            .zip(bufs.iter_mut())
            .map(|(o, b)| make_pod(b, o))
            .collect();
        stream
            .connect(
                Direction::Output,
                None,
                StreamFlags::DRIVER | StreamFlags::ALLOC_BUFFERS,
                &mut params,
            )
            .map_err(|e| e.to_string())?;
        let what = match &source {
            Source::Screen(o) => format!("screen {}", o.name()),
            Source::Portion(o, r) => format!("portion of {} {r:?}", o.name()),
            Source::Window(w) => format!("window {w}"),
        };
        info!(id, what, ?size, owner, "a stream");
        self.isle.casting.streams.push(Stream {
            id,
            source,
            cursor,
            owner,
            started: Instant::now(),
            stream,
            _listener: listener,
            inner,
            damage: None,
            scratch: None,
            last_queued: None,
            later: None,
            seq: 0,
            rendering: Vec::new(),
            frames: 0,
            cursor_sent: None,
            bitmap_sent: 0,
        });
        self.casting_changed();
        Ok(id)
    }

    /// Ends stream `id` from Isle's side, saying why to its owner.
    pub fn end_cast(&mut self, id: u64, why: Ended) {
        let Some(index) = self.isle.casting.streams.iter().position(|s| s.id == id) else {
            return;
        };
        let stream = self.isle.casting.streams.remove(index);
        if let Some(token) = stream.later {
            self.isle.loop_handle.remove(token);
        }
        let _ = stream.stream.disconnect();
        info!(id, why = why.reason(), "a stream ended");
        self.cast_ended(id, stream.owner, why);
        self.casting_changed();
    }

    /// Owners hear of a stream's end; the portal and the recorder add theirs.
    fn cast_ended(&mut self, id: u64, owner: &'static str, why: Ended) {
        match owner {
            "recording" => self.recording_stream_ended(id, why),
            "portal" => self.share_stream_ended(id, why),
            _ => {}
        }
    }

    /// Any stream holds banners and shows the screen mark.
    fn casting_changed(&mut self) {
        use isle_shell::island::model::Privacy;
        let any = !self.isle.casting.streams.is_empty()
            || self.isle.image_copy.streaming().next().is_some();
        self.isle.island.model.set_privacy(Privacy::Screen, any);
        self.isle.refresh_island();
        self.isle.refresh_banners();
    }

    fn cast_event(&mut self, event: Event) {
        match event {
            Event::Paused { id, node } => {
                debug!(id, node, "a stream's node");
                if self.isle.casting.get(id).is_some_and(|s| s.owner == "recording") {
                    self.recording_node(id, node);
                }
            }
            Event::End { id, why } => self.end_cast(id, why),
            Event::Streaming { id } | Event::Redraw { id } => self.cast_redraw_source(id),
            Event::Failed { id } => self.end_cast(id, Ended::Disconnected),
            Event::Serial { node, serial } => {
                if let Some(c) = self.isle.casting.connection.as_ref() {
                    c.serials.borrow_mut().insert(node, serial);
                }
            }
            Event::Fatal => {
                let ids: Vec<u64> = self.isle.casting.streams.iter().map(|s| s.id).collect();
                for id in ids {
                    self.end_cast(id, Ended::Disconnected);
                }
                if let Some(c) = self.isle.casting.connection.take() {
                    self.isle.loop_handle.remove(c.loop_token);
                }
            }
        }
    }

    /// Asks for a frame of the output stream `id` shows on.
    fn cast_redraw_source(&mut self, id: u64) {
        let output = self.isle.casting.get(id).and_then(|s| match &s.source {
            Source::Screen(o) | Source::Portion(o, _) => Some(o.clone()),
            Source::Window(w) => self.isle.output_of(*w),
        });
        if let Some(output) = output {
            // A first frame even with no damage.
            if let Some(s) = self.isle.casting.streams.iter_mut().find(|s| s.id == id) {
                s.damage = None;
            }
            self.isle.queue_redraw(&output);
        }
    }

    /// After `output`'s frame: each stream of it renders and queues a buffer, if its source
    /// is shown, it has damage and it is not too soon.
    pub fn cast_frames(&mut self, output: &Output) {
        if self.isle.casting.streams.is_empty() {
            return;
        }
        let ids: Vec<u64> = self
            .isle
            .casting
            .streams
            .iter()
            .filter(|s| match &s.source {
                Source::Screen(o) | Source::Portion(o, _) => o == output,
                Source::Window(w) => self.isle.output_of(*w).as_ref() == Some(output),
            })
            .map(|s| s.id)
            .collect();
        for id in ids {
            self.cast_frame(id);
        }
    }

    fn cast_frame(&mut self, id: u64) {
        let Some(index) = self.isle.casting.streams.iter().position(|s| s.id == id) else {
            return;
        };
        let stream = &self.isle.casting.streams[index];
        if !stream.streaming() || stream.inner.borrow().negotiated.is_none() {
            return;
        }
        let source = stream.source.clone();
        // A window streams only while it is shown, and not while locked: the consumer holds
        // its last frame.
        if let Source::Window(w) = source
            && (!self.isle.wm.is_shown(w) || self.isle.lock.locked)
        {
            return;
        }
        let now = Instant::now();
        let min = stream.inner.borrow().min_interval;
        if let Some(last) = stream.last_queued
            && now - last + Duration::from_micros(100) < min
        {
            // Too soon: once the interval is up, the output is drawn again.
            if stream.later.is_none() {
                let wait = min - (now - last);
                let token = self
                    .isle
                    .loop_handle
                    .insert_source(Timer::from_duration(wait), move |_, _, state| {
                        if let Some(s) = state.isle.casting.streams.iter_mut().find(|s| s.id == id)
                        {
                            s.later = None;
                        }
                        state.cast_redraw_source(id);
                        TimeoutAction::Drop
                    })
                    .ok();
                self.isle.casting.streams[index].later = token;
            }
            return;
        }
        let size = stream.size();
        if self.isle.source_size(&source) != Some(size) {
            // The source changed size: the stream renegotiates with the next start.
            debug!(id, "the source changed size");
            return;
        }
        let cursor_mode = stream.cursor;
        let recording = stream.owner == "recording";
        let cursor = (cursor_mode == CursorMode::Metadata)
            .then(|| self.cursor_for(&source))
            .flatten();
        let began = Instant::now();
        let (elements, scale) =
            match self.cast_elements(
                &source,
                cursor_mode == CursorMode::Embedded,
                recording,
            ) {
                Some(x) => x,
                None => return,
            };
        let stream = &mut self.isle.casting.streams[index];
        let psize: Size<i32, Physical> = (size.w as i32, size.h as i32).into();
        let tracker = stream
            .damage
            .get_or_insert_with(|| OutputDamageTracker::new(psize, scale, Transform::Normal));
        let damaged = match tracker.damage_output(1, &elements) {
            Ok((damage, _)) => damage.is_some(),
            Err(_) => true,
        };
        let new_bitmap = cursor.filter(|c| c.serial != stream.bitmap_sent);
        let moved = cursor_mode == CursorMode::Metadata
            && (cursor.map(|c| c.at) != stream.cursor_sent || new_bitmap.is_some());
        if !damaged && !moved {
            return;
        }
        // SAFETY: a dequeued buffer is ours until it is queued back.
        let Some(buffer) = NonNull::new(unsafe { stream.stream.dequeue_raw_buffer() }) else {
            debug!(id, "no free buffer");
            return;
        };
        let negotiated = stream.inner.borrow().negotiated.unwrap();
        if cursor_mode == CursorMode::Metadata {
            let image = self.isle.casting.cursor.as_ref();
            let hotspot = image.map(|c| c.hotspot).unwrap_or_default();
            let bitmap = new_bitmap.and(image).and_then(|c| c.image.as_ref());
            // SAFETY: as above.
            unsafe { write_cursor(buffer, cursor.map(|c| c.at), hotspot, bitmap) };
            let stream = &mut self.isle.casting.streams[index];
            stream.cursor_sent = cursor.map(|c| c.at);
            if bitmap.is_some()
                && let Some(c) = cursor
            {
                stream.bitmap_sent = c.serial;
            }
        }
        let stream = &mut self.isle.casting.streams[index];
        if !damaged {
            stream.seq += 1;
            // SAFETY: as above; the pointer moved over a still frame.
            unsafe { mark_cursor_only(buffer, stream.seq) };
            stream.last_queued = Some(now);
            self.queue_after(id, buffer, SyncPoint::signaled());
            return;
        }
        let Some(renderer) = self.backend.renderer_opt() else {
            // SAFETY: as above.
            unsafe { return_unused(&self.isle.casting.streams[index].stream, buffer) };
            return;
        };
        let stream = &mut self.isle.casting.streams[index];
        let result = unsafe {
            render_into(
                renderer, stream, buffer, negotiated, psize, scale, &elements,
            )
        };
        match result {
            Ok(sync) => {
                stream.seq += 1;
                // SAFETY: as above.
                unsafe { mark_good(buffer, stream.seq, negotiated) };
                stream.frames += 1;
                stream.last_queued = Some(now);
                self.queue_after(id, buffer, sync);
                if self.isle.shell.measuring() {
                    let micros = began.elapsed().as_micros() as u64;
                    self.isle.casting.times.push(micros);
                }
            }
            Err(err) => {
                warn!(id, "rendering a stream's frame: {err}");
                // SAFETY: the buffer goes back marked as not to be shown.
                unsafe { return_unused(&stream.stream, buffer) };
            }
        }
    }

    /// Metadata mode: the pointer's place in `source`'s frame while it is over it and shown,
    /// and the serial of the bitmap of its image now, drawn again when the image changes.
    fn cursor_for(&mut self, source: &Source) -> Option<SentCursor> {
        let pointer = self.isle.seat.get_pointer()?.current_location();
        let (area, output) = match source {
            Source::Screen(o) => (self.isle.space.output_geometry(o)?, o.clone()),
            Source::Portion(o, r) => {
                let g = self.isle.space.output_geometry(o)?;
                (Rectangle::new(g.loc + r.loc, r.size), o.clone())
            }
            Source::Window(id) => (self.isle.frame(*id)?, self.isle.output_of(*id)?),
        };
        if !area.to_f64().contains(pointer) {
            return None;
        }
        let serial = self.cursor_bitmap(output.current_scale().integer_scale().max(1) as u32)?;
        let at = (pointer - area.loc.to_f64())
            .to_physical(output.current_scale().fractional_scale())
            .to_i32_round();
        Some(SentCursor { at, serial })
    }

    /// The pointer's image now at an integer `scale`, drawn again when it changes: its
    /// serial, or `None` while it is hidden.
    pub fn cursor_bitmap(&mut self, scale: u32) -> Option<u64> {
        use smithay::{input::pointer::CursorImageStatus, reexports::wayland_server::Resource};
        let key = match &self.isle.cursor_status {
            CursorImageStatus::Named(icon) => CursorKey::Named(*icon, scale),
            CursorImageStatus::Surface(s) => CursorKey::Surface(s.id(), scale),
            CursorImageStatus::Hidden => return None,
        };
        if self.isle.casting.cursor.as_ref().map(|c| &c.key) != Some(&key) {
            let drawn = self
                .backend
                .cursor_image(&self.isle, scale, CURSOR_SIDE)
                .unwrap_or_else(|err| {
                    warn!("a cursor bitmap: {err}");
                    None
                });
            self.isle.casting.cursor_serial += 1;
            let (image, hotspot) = match drawn {
                Some((pixels, hotspot)) => (Some(pixels), hotspot),
                None => (None, Point::default()),
            };
            self.isle.casting.cursor = Some(CursorImage {
                key,
                serial: self.isle.casting.cursor_serial,
                image,
                hotspot,
            });
        }
        self.isle.casting.cursor.as_ref().map(|c| c.serial)
    }

    /// The marks of recent presses within `area` (global), placed relative to its origin,
    /// front to back; while any fade, the area's output is drawn again.
    fn click_marks(&mut self, area: Rectangle<i32, Logical>, scale: f64) -> Vec<IsleElement> {
        self.isle.clicks.retain(|(_, at)| at.elapsed() < CLICK_FADE);
        let Some(programs) = self.isle.window_programs.as_ref() else {
            return Vec::new();
        };
        let t = &self.isle.shell.theme().tokens;
        let mut marks = Vec::new();
        for (at, when) in &self.isle.clicks {
            if !area.to_f64().contains(*at) {
                continue;
            }
            let k = when.elapsed().as_secs_f32() / CLICK_FADE.as_secs_f32();
            // Grows from three fifths of its size as it fades.
            let side = (t.size_capture_click * (0.6 + 0.4 * k)).round() as i32;
            let centre = (*at - area.loc.to_f64()).to_i32_round::<i32>();
            let rect = Rectangle::new(
                (centre.x - side / 2, centre.y - side / 2).into(),
                (side, side).into(),
            );
            let disc = crate::wm::draw::ShadowElement::rounded(
                programs.shadow.clone(),
                rect,
                side as f32 / 2.0,
                t.colour_capture_line_shadow,
                None,
            )
            .at(rect.loc, (0, 0).into(), 1.0 - k);
            marks.push(IsleElement::Window(crate::wm::draw::unplaced(
                crate::wm::draw::WindowPiece::Shadow(disc),
                scale,
            )));
        }
        if !marks.is_empty() {
            self.isle.queue_redraw_all();
        }
        marks
    }

    /// The source's elements placed at the stream's origin, and its scale; with `clicks`,
    /// recent presses marked.
    fn cast_elements(
        &mut self,
        source: &Source,
        embed_pointer: bool,
        clicks: bool,
    ) -> Option<(Vec<CastElement>, f64)> {
        match source {
            Source::Screen(output) | Source::Portion(output, _) => {
                let scale = output.current_scale().fractional_scale();
                let origin = match source {
                    Source::Portion(_, rect) => rect.loc,
                    _ => (0, 0).into(),
                };
                let mut elements = Vec::new();
                if clicks && let Some(screen) = self.isle.space.output_geometry(output) {
                    elements.extend(self.click_marks(screen, scale));
                }
                self.backend
                    .capture_elements(&self.isle, output, embed_pointer, &mut |e| {
                        elements.push(e)
                    })?;
                let shift = origin.to_f64().to_physical(scale).to_i32_round::<i32>();
                let placed = elements
                    .into_iter()
                    .map(|e| {
                        RelocateRenderElement::from_element(
                            e,
                            (-shift.x, -shift.y),
                            Relocate::Relative,
                        )
                    })
                    .collect();
                Some((placed, scale))
            }
            Source::Window(id) => {
                let frame = self.isle.frame(*id)?;
                let scale = self.isle.output_of(*id)?.current_scale().fractional_scale();
                let marks = if clicks {
                    self.click_marks(frame, scale)
                } else {
                    Vec::new()
                };
                let renderer = self.backend.renderer_opt()?;
                let mut pieces = Vec::new();
                self.isle
                    .window_pieces(renderer, *id, frame.loc, scale, false, 1.0, &mut |p| {
                        pieces.push(p)
                    });
                let marks = marks
                    .into_iter()
                    .map(|e| RelocateRenderElement::from_element(e, (0, 0), Relocate::Relative));
                let placed = marks
                    .chain(pieces.into_iter().map(|p| {
                        RelocateRenderElement::from_element(
                            IsleElement::Window(crate::wm::draw::place(
                                p,
                                Rectangle::from_size(frame.size),
                                Rectangle::from_size(frame.size).to_f64(),
                                scale,
                            )),
                            (0, 0),
                            Relocate::Relative,
                        )
                    }))
                    .collect();
                Some((placed, scale))
            }
        }
    }

    /// Queues `buffer` once its rendering is done, keeping the stream's buffers in order.
    fn queue_after(&mut self, id: u64, buffer: NonNull<pw_buffer>, sync: SyncPoint) {
        let Some(stream) = self.isle.casting.streams.iter_mut().find(|s| s.id == id) else {
            return;
        };
        let fence = sync.export();
        stream.rendering.push((
            buffer,
            if fence.is_some() {
                sync
            } else {
                SyncPoint::signaled()
            },
        ));
        match fence {
            None => self.queue_done(id),
            Some(fd) => {
                let _ = self.isle.loop_handle.insert_source(
                    Generic::new(fd, Interest::READ, Mode::OneShot),
                    move |_, _, state| {
                        state.queue_done(id);
                        Ok(PostAction::Remove)
                    },
                );
            }
        }
    }

    fn queue_done(&mut self, id: u64) {
        let Some(stream) = self.isle.casting.streams.iter_mut().find(|s| s.id == id) else {
            return;
        };
        let done = stream
            .rendering
            .iter()
            .position(|(_, sync)| !sync.is_reached())
            .unwrap_or(stream.rendering.len());
        for (buffer, _) in stream.rendering.drain(..done) {
            // SAFETY: a buffer dequeued from this stream and rendered, queued once.
            unsafe { stream.stream.queue_raw_buffer(buffer.as_ptr()) };
        }
    }

    /// A window closed: its streams end.
    pub fn cast_window_closed(&mut self, window: u64) {
        let ids: Vec<u64> = self
            .isle
            .casting
            .streams
            .iter()
            .filter(|s| s.source == Source::Window(window))
            .map(|s| s.id)
            .collect();
        for id in ids {
            self.end_cast(id, Ended::WindowClosed);
        }
    }

    /// The streams as JSON, for the Test interface and `isle capture streams`.
    pub fn casts_status(&self) -> serde_json::Value {
        let streams: Vec<serde_json::Value> = self
            .isle
            .casting
            .streams
            .iter()
            .map(|s| {
                let node = s.node();
                serde_json::json!({
                    "id": s.id,
                    "owner": s.owner,
                    "node": node,
                    "serial": node.and_then(|n| self.isle.casting.serial(n)),
                    "streaming": s.streaming(),
                    "kind": s.kind(),
                    "size": [s.size().w, s.size().h],
                    "frames": s.frames,
                    "times": self.isle.casting.times.len(),
                    "source": match &s.source {
                        Source::Screen(o) => serde_json::json!({"screen": o.name()}),
                        Source::Portion(o, r) => serde_json::json!({"portion": o.name(), "rect": [r.loc.x, r.loc.y, r.size.w, r.size.h]}),
                        Source::Window(w) => serde_json::json!({"window": w}),
                    },
                })
            })
            .collect();
        serde_json::json!(streams)
    }
}

#[cfg(feature = "test-hooks")]
impl State {
    pub fn test_cast_start(&mut self, spec: &str) -> Result<u64, String> {
        let spec: serde_json::Value = serde_json::from_str(spec).map_err(|e| e.to_string())?;
        let output = |name: &str| {
            self.isle
                .space
                .outputs()
                .find(|o| o.name() == name)
                .cloned()
                .ok_or_else(|| format!("no output {name}"))
        };
        let int = |k: &str| spec[k].as_i64().unwrap_or_default() as i32;
        let source = if let Some(name) = spec["screen"].as_str() {
            Source::Screen(output(name)?)
        } else if let Some(name) = spec["portion"].as_str() {
            Source::Portion(
                output(name)?,
                Rectangle::new((int("x"), int("y")).into(), (int("w"), int("h")).into()),
            )
        } else if let Some(id) = spec["window"].as_u64() {
            Source::Window(id)
        } else {
            return Err("no source".into());
        };
        let cursor = match spec["cursor"].as_str() {
            Some("embedded") => CursorMode::Embedded,
            Some("metadata") => CursorMode::Metadata,
            _ => CursorMode::Hidden,
        };
        self.start_cast(source, cursor, "test")
    }

    pub fn test_record(&mut self, spec: &str) -> Result<(), String> {
        let id = self.test_cast_start(spec)?;
        let source = self.isle.casting.get(id).map(|s| s.source.clone()).ok_or("no stream")?;
        self.end_cast(id, Ended::Stopped);
        self.start_recording(source);
        if self.recording() { Ok(()) } else { Err("not recording".into()) }
    }
}

impl State {
    /// Every stream, share and recording, and the portal backend, as JSON.
    pub fn capture_status(&self) -> serde_json::Value {
        let shares: serde_json::Map<String, serde_json::Value> = self
            .isle
            .share
            .sessions
            .iter()
            .map(|(path, s)| (path.clone(), serde_json::json!({"app": s.app, "streams": s.streams})))
            .collect();
        serde_json::json!({
            "portal": self.isle.portal.is_some(),
            "streams": self.casts_status(),
            "shares": shares,
            "recording": self.isle.recording_now.as_ref().map(|r| r.path.display().to_string()),
        })
    }
}

/// An element of a stream's frame, moved to the stream's origin.
pub type CastElement = RelocateRenderElement<IsleElement>;

/// Fills `buffer`'s datas for the negotiated layout.
unsafe fn add_buffer(
    inner: &mut Inner,
    gbm: Option<&GbmDevice<DrmDeviceFd>>,
    buffer: *mut pw_buffer,
) -> Result<(), String> {
    let negotiated = inner.negotiated.ok_or("not negotiated")?;
    // SAFETY: the caller's buffer, with one data per block negotiated.
    unsafe {
        let spa = (*buffer).buffer;
        match negotiated {
            Negotiated::Dma { modifier, planes } => {
                let gbm = gbm.ok_or("no GBM device")?;
                let fourcc = if inner.alpha {
                    Fourcc::Argb8888
                } else {
                    Fourcc::Xrgb8888
                };
                let dmabuf = allocate(gbm, inner.size, fourcc, &[u64::from(modifier) as i64])?;
                if dmabuf.num_planes() != planes || (*spa).n_datas as usize != planes {
                    return Err("planes changed".into());
                }
                for (i, (fd, (stride, offset))) in dmabuf
                    .handles()
                    .zip(dmabuf.strides().zip(dmabuf.offsets()))
                    .enumerate()
                {
                    let data = (*spa).datas.add(i);
                    (*data).type_ = DataType::DmaBuf.as_raw();
                    (*data).maxsize = 1;
                    (*data).fd = fd.as_raw_fd() as i64;
                    (*data).flags = SPA_DATA_FLAG_READWRITE;
                    (*(*data).chunk).stride = stride as i32;
                    (*(*data).chunk).offset = offset;
                }
                let fd = (*(*spa).datas).fd;
                inner.dmabufs.insert(fd, dmabuf);
            }
            Negotiated::Shm { stride, size } => {
                let shm = shm(size as usize)?;
                let data = (*spa).datas;
                (*data).type_ = DataType::MemFd.as_raw();
                (*data).maxsize = size;
                (*data).fd = shm._fd.as_raw_fd() as i64;
                (*data).flags = SPA_DATA_FLAG_READWRITE;
                (*(*data).chunk).stride = stride;
                (*(*data).chunk).offset = 0;
                inner.shmbufs.insert((*data).fd, shm);
            }
        }
    }
    Ok(())
}

/// Renders `elements` into `buffer`: drawn straight into a dmabuf, or into the stream's
/// texture and copied into shared memory.
unsafe fn render_into(
    renderer: &mut GlesRenderer,
    stream: &mut Stream,
    buffer: NonNull<pw_buffer>,
    negotiated: Negotiated,
    size: Size<i32, Physical>,
    scale: f64,
    elements: &[CastElement],
) -> Result<SyncPoint, String> {
    // SAFETY: the caller's dequeued buffer.
    let fd = unsafe { (*(*(*buffer.as_ptr()).buffer).datas).fd };
    let full = Rectangle::from_size(size);
    match negotiated {
        Negotiated::Dma { .. } => {
            let mut dmabuf = stream
                .inner
                .borrow()
                .dmabufs
                .get(&fd)
                .cloned()
                .ok_or("no such dmabuf")?;
            let mut target = renderer.bind(&mut dmabuf).map_err(|e| e.to_string())?;
            let mut frame = renderer
                .render(&mut target, size, Transform::Normal)
                .map_err(|e| e.to_string())?;
            use smithay::backend::renderer::{Color32F, Frame};
            frame
                .clear(Color32F::from(crate::backend::CLEAR), &[full])
                .map_err(|e| e.to_string())?;
            smithay::backend::renderer::utils::draw_render_elements(
                &mut frame,
                scale,
                elements,
                &[full],
            )
            .map_err(|e| e.to_string())?;
            frame.finish().map_err(|e| e.to_string())
        }
        Negotiated::Shm { size: len, .. } => {
            let texture = match &mut stream.scratch {
                Some(t) => t,
                slot => {
                    let buffer_size = size.to_logical(1).to_buffer(1, Transform::Normal);
                    *slot = Some(
                        Offscreen::<GlesTexture>::create_buffer(
                            renderer,
                            Fourcc::Abgr8888,
                            buffer_size,
                        )
                        .map_err(|e| e.to_string())?,
                    );
                    slot.as_mut().unwrap()
                }
            };
            let mapping = {
                let mut target = renderer.bind(texture).map_err(|e| e.to_string())?;
                let mut frame = renderer
                    .render(&mut target, size, Transform::Normal)
                    .map_err(|e| e.to_string())?;
                use smithay::backend::renderer::{Color32F, Frame};
                frame
                    .clear(Color32F::from(crate::backend::CLEAR), &[full])
                    .map_err(|e| e.to_string())?;
                smithay::backend::renderer::utils::draw_render_elements(
                    &mut frame,
                    scale,
                    elements,
                    &[full],
                )
                .map_err(|e| e.to_string())?;
                let _ = frame.finish().map_err(|e| e.to_string())?;
                // BGRx in memory is the renderer's Argb8888.
                renderer
                    .copy_framebuffer(
                        &target,
                        Rectangle::from_size((size.w, size.h).into()),
                        Fourcc::Argb8888,
                    )
                    .map_err(|e| e.to_string())?
            };
            let bytes = renderer.map_texture(&mapping).map_err(|e| e.to_string())?;
            let inner = stream.inner.borrow();
            let shm = inner.shmbufs.get(&fd).ok_or("no such memfd")?;
            let n = bytes.len().min(shm.len).min(len as usize);
            // SAFETY: the mapping is `shm.len` bytes, written while the buffer is ours.
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), shm.map.as_ptr(), n) };
            Ok(SyncPoint::signaled())
        }
    }
}

/// The header's sequence, and the chunk sized for its kind, the buffer good to show.
unsafe fn mark_good(buffer: NonNull<pw_buffer>, seq: u64, negotiated: Negotiated) {
    // SAFETY: the caller's buffer.
    unsafe {
        let spa = (*buffer.as_ptr()).buffer;
        let chunk = (*(*spa).datas).chunk;
        (*chunk).size = match negotiated {
            // Consumers ignore a dmabuf's size; OBS takes 0 for no frame.
            Negotiated::Dma { .. } => 1,
            Negotiated::Shm { size, .. } => size,
        };
        (*chunk).flags = SPA_CHUNK_FLAG_NONE as i32;
        let header: *mut spa_meta_header =
            spa_buffer_find_meta_data(spa, SPA_META_Header, size_of::<spa_meta_header>()).cast();
        if let Some(header) = header.as_mut() {
            header.flags = 0;
            header.seq = seq;
            header.pts = -1;
        }
    }
}

/// A buffer with no new picture: its chunk empty, its header sequenced, for a consumer to
/// read the cursor from.
unsafe fn mark_cursor_only(buffer: NonNull<pw_buffer>, seq: u64) {
    // SAFETY: the caller's buffer.
    unsafe {
        let spa = (*buffer.as_ptr()).buffer;
        let chunk = (*(*spa).datas).chunk;
        (*chunk).size = 0;
        (*chunk).flags = SPA_CHUNK_FLAG_NONE as i32;
        let header: *mut spa_meta_header =
            spa_buffer_find_meta_data(spa, SPA_META_Header, size_of::<spa_meta_header>()).cast();
        if let Some(header) = header.as_mut() {
            header.flags = 0;
            header.seq = seq;
            header.pts = -1;
        }
    }
}

/// The buffer's cursor metadata: the pointer at `at` in the frame, or none there; with
/// `bitmap`, its image after the cursor's header.
unsafe fn write_cursor(
    buffer: NonNull<pw_buffer>,
    at: Option<Point<i32, Physical>>,
    hotspot: Point<i32, Physical>,
    bitmap: Option<&crate::backend::Pixels>,
) {
    // SAFETY: the caller's buffer; its cursor meta is CURSOR_META_SIZE long when found at
    // that size.
    unsafe {
        let spa = (*buffer.as_ptr()).buffer;
        let meta: *mut spa_meta_cursor =
            spa_buffer_find_meta_data(spa, SPA_META_Cursor, size_of::<spa_meta_cursor>()).cast();
        let Some(meta) = meta.as_mut() else {
            return;
        };
        let Some(at) = at else {
            meta.id = 0;
            return;
        };
        meta.id = 1;
        meta.flags = 0;
        meta.position = spa_point { x: at.x, y: at.y };
        meta.hotspot = spa_point {
            x: hotspot.x,
            y: hotspot.y,
        };
        meta.bitmap_offset = 0;
        let Some(pixels) = bitmap else {
            return;
        };
        if spa_buffer_find_meta_data(spa, SPA_META_Cursor, CURSOR_META_SIZE).is_null()
            || pixels.rgba.len() > (CURSOR_SIDE * CURSOR_SIDE * 4) as usize
        {
            return;
        }
        meta.bitmap_offset = size_of::<spa_meta_cursor>() as u32;
        let base = (meta as *mut spa_meta_cursor).cast::<u8>();
        let bitmap = base.add(meta.bitmap_offset as usize).cast::<spa_meta_bitmap>();
        (*bitmap).format = SPA_VIDEO_FORMAT_RGBA;
        (*bitmap).size = spa_rectangle {
            width: pixels.width,
            height: pixels.height,
        };
        (*bitmap).stride = (pixels.width * 4) as i32;
        (*bitmap).offset = size_of::<spa_meta_bitmap>() as u32;
        std::ptr::copy_nonoverlapping(
            pixels.rgba.as_ptr(),
            bitmap.cast::<u8>().add((*bitmap).offset as usize),
            pixels.rgba.len(),
        );
    }
}

/// Gives a buffer back unshown.
unsafe fn return_unused(stream: &StreamRc, buffer: NonNull<pw_buffer>) {
    // SAFETY: the caller's dequeued buffer, queued once.
    unsafe {
        let spa = (*buffer.as_ptr()).buffer;
        let chunk = (*(*spa).datas).chunk;
        (*chunk).size = 0;
        (*chunk).flags = SPA_CHUNK_FLAG_CORRUPTED as i32;
        let header: *mut spa_meta_header =
            spa_buffer_find_meta_data(spa, SPA_META_Header, size_of::<spa_meta_header>()).cast();
        if let Some(header) = header.as_mut() {
            header.flags = SPA_META_HEADER_FLAG_CORRUPTED;
        }
        stream.queue_raw_buffer(buffer.as_ptr());
    }
}
