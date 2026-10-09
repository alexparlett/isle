//! A PipeWire video consumer for the stream tests: links itself to `--node`, takes shared
//! memory, and prints each format and frame, and the cursor metadata when it moves:
//! `cursor X Y`, `cursor gone`, and `bitmap WxH` with a new image.

use std::{cell::Cell, io::Cursor, rc::Rc};

use pipewire::{
    self as pw,
    context::ContextRc,
    main_loop::MainLoopRc,
    properties::properties,
    spa::{
        param::{
            ParamType,
            format::{FormatProperties, MediaSubtype, MediaType},
            video::{VideoFormat, VideoInfoRaw},
        },
        pod::{self, Pod, serialize::PodSerializer},
        utils::{Direction, Fraction, Rectangle, SpaTypes},
    },
    stream::{StreamFlags, StreamRc, StreamState},
};

fn cursor_meta_size(side: i32) -> i32 {
    use pw::spa::sys::{spa_meta_bitmap, spa_meta_cursor};
    (size_of::<spa_meta_cursor>() + size_of::<spa_meta_bitmap>()) as i32 + side * side * 4
}

/// Prints the buffer's cursor metadata where it differs from the last.
///
/// # Safety
/// `spa` is a dequeued buffer's.
unsafe fn read_cursor(spa: *mut pw::spa::sys::spa_buffer, last: &Cell<Option<(i32, i32)>>) {
    use pw::spa::sys::{SPA_META_Cursor, spa_buffer_find_meta_data, spa_meta_bitmap, spa_meta_cursor};
    // SAFETY: as the caller says.
    unsafe {
        let meta: *const spa_meta_cursor =
            spa_buffer_find_meta_data(spa, SPA_META_Cursor, size_of::<spa_meta_cursor>()).cast();
        let Some(meta) = meta.as_ref() else {
            return;
        };
        if meta.id == 0 {
            if last.take().is_some() {
                println!("cursor gone");
            }
            return;
        }
        let at = (meta.position.x, meta.position.y);
        if last.replace(Some(at)) != Some(at) {
            println!("cursor {} {}", at.0, at.1);
        }
        if meta.bitmap_offset > 0 {
            let bitmap: *const spa_meta_bitmap = (meta as *const spa_meta_cursor)
                .cast::<u8>()
                .add(meta.bitmap_offset as usize)
                .cast();
            println!("bitmap {}x{}", (*bitmap).size.width, (*bitmap).size.height);
        }
    }
}

/// The frame's line, its centre pixel in BGRx, or `None` for a buffer with no picture.
///
/// # Safety
/// `spa` is a dequeued, mapped buffer's.
unsafe fn frame_line(
    spa: *mut pw::spa::sys::spa_buffer,
    (w, h): (u32, u32),
    frames: &Cell<u64>,
) -> Option<String> {
    // SAFETY: as the caller says.
    unsafe {
        let data = (*spa).datas.as_ref()?;
        let chunk = data.chunk.as_ref()?;
        let (offset, stride, len) = (chunk.offset as usize, chunk.stride as usize, chunk.size as usize);
        if len == 0 || data.data.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(data.data.cast::<u8>(), data.maxsize as usize);
        let i = offset + (h as usize / 2) * stride + (w as usize / 2) * 4;
        if i + 3 > bytes.len() {
            return None;
        }
        frames.set(frames.get() + 1);
        Some(format!(
            "frame {} {}x{} {:02x}{:02x}{:02x}",
            frames.get(),
            w,
            h,
            bytes[i + 2],
            bytes[i + 1],
            bytes[i]
        ))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let node: u32 = args
        .iter()
        .position(|a| a == "--node")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok())
        .expect("--node <id>");
    pw::init();
    let main_loop = MainLoopRc::new(None).unwrap();
    let context = ContextRc::new(
        &main_loop,
        Some(properties! { "module.rt" => "false", "support.dbus" => "false" }),
    )
    .unwrap();
    let core = context.connect_rc(None).unwrap();
    let _errors = core
        .add_listener_local()
        .error(|id, _, res, message| println!("error {id} {res} {message}"))
        .register();
    let stream = StreamRc::new(
        core.clone(),
        "isle-test-consumer",
        properties! { "media.class" => "Stream/Input/Video", "node.name" => "isle-test-consumer" },
    )
    .unwrap();
    let size = Rc::new(Cell::new((0u32, 0u32)));
    let frames = Rc::new(Cell::new(0u64));
    let cursor_at = Rc::new(Cell::new(None::<(i32, i32)>));
    let link: Rc<std::cell::RefCell<Option<pw::link::Link>>> = Rc::default();

    // With no session manager, the consumer links itself port to port once both show.
    let ports: Rc<std::cell::RefCell<(Option<u32>, Option<u32>)>> = Rc::default();
    let me: Rc<Cell<Option<u32>>> = Rc::default();
    let registry = core.get_registry_rc().unwrap();
    let _registry = registry
        .add_listener_local()
        .global({
            let core = core.clone();
            let link = link.clone();
            let ports = ports.clone();
            let me = me.clone();
            move |global| {
                if global.type_ != pw::types::ObjectType::Port {
                    return;
                }
                let Some(props) = global.props else { return };
                let owner: Option<u32> = props.get("node.id").and_then(|n| n.parse().ok());
                let direction = props.get("port.direction");
                let mut p = ports.borrow_mut();
                if owner == Some(node) && direction == Some("out") {
                    p.0 = Some(global.id);
                }
                if owner.is_some() && owner == me.get() && direction == Some("in") {
                    p.1 = Some(global.id);
                }
                if let (Some(out), Some(input)) = *p
                    && link.borrow().is_none()
                {
                    let made = core
                        .create_object::<pw::link::Link>(
                            "link-factory",
                            &properties! {
                                "link.output.node" => node.to_string(),
                                "link.output.port" => out.to_string(),
                                "link.input.node" => me.get().unwrap().to_string(),
                                "link.input.port" => input.to_string(),
                                "object.linger" => "false",
                            },
                        )
                        .expect("a link");
                    *link.borrow_mut() = Some(made);
                    println!("linked");
                }
            }
        })
        .register();

    let _listener = stream
        .add_local_listener_with_user_data(())
        .state_changed({
            let me = me.clone();
            move |stream, (), _, new| {
                println!("state {new:?}");
                if let StreamState::Paused = new {
                    me.set(Some(stream.node_id()));
                }
            }
        })
        .param_changed({
            let size = size.clone();
            move |stream, (), id, pod| {
                if ParamType::from_raw(id) != ParamType::Format {
                    return;
                }
                let Some(pod) = pod else { return };
                let mut info = VideoInfoRaw::new();
                if info.parse(pod).is_ok() {
                    size.set((info.size().width, info.size().height));
                    println!("format {}x{}", info.size().width, info.size().height);
                    // The cursor's metadata, up to a 256-pixel bitmap, as OBS asks.
                    let cursor = pod::Value::Object(pod::Object {
                        type_: SpaTypes::ObjectParamMeta.as_raw(),
                        id: ParamType::Meta.as_raw(),
                        properties: vec![
                            pod::Property::new(
                                pw::spa::sys::SPA_PARAM_META_type,
                                pod::Value::Id(pw::spa::utils::Id(pw::spa::sys::SPA_META_Cursor)),
                            ),
                            pod::Property::new(
                                pw::spa::sys::SPA_PARAM_META_size,
                                pod::Value::Choice(pod::ChoiceValue::Int(pw::spa::utils::Choice(
                                    pw::spa::utils::ChoiceFlags::empty(),
                                    pw::spa::utils::ChoiceEnum::Range {
                                        default: cursor_meta_size(64),
                                        min: cursor_meta_size(1),
                                        max: cursor_meta_size(256),
                                    },
                                ))),
                            ),
                        ],
                    });
                    let mut bytes = Vec::new();
                    PodSerializer::serialize(Cursor::new(&mut bytes), &cursor).unwrap();
                    let mut params = [Pod::from_bytes(&bytes).unwrap()];
                    let _ = stream.update_params(&mut params);
                }
            }
        })
        .process({
            let size = size.clone();
            let frames = frames.clone();
            let cursor_at = cursor_at.clone();
            move |stream, ()| {
                // SAFETY: a dequeued buffer is ours until it is queued back, below.
                unsafe {
                    let raw = stream.dequeue_raw_buffer();
                    if raw.is_null() {
                        return;
                    }
                    let spa = (*raw).buffer;
                    read_cursor(spa, &cursor_at);
                    if let Some(line) = frame_line(spa, size.get(), &frames) {
                        println!("{line}");
                    }
                    stream.queue_raw_buffer(raw);
                }
            }
        })
        .register()
        .unwrap();

    let format = pod::Object {
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
                    width: 1,
                    height: 1
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
    };
    let mut bytes = Vec::new();
    PodSerializer::serialize(Cursor::new(&mut bytes), &pod::Value::Object(format)).unwrap();
    let mut params = [Pod::from_bytes(&bytes).unwrap()];
    stream
        .connect(
            Direction::Input,
            None,
            StreamFlags::MAP_BUFFERS,
            &mut params,
        )
        .unwrap();
    println!("ready");
    main_loop.run();
}
