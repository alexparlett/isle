//! A PipeWire video consumer for the stream tests: links itself to `--node`, takes shared
//! memory, and prints each format and frame.

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
            move |_, (), id, pod| {
                if ParamType::from_raw(id) != ParamType::Format {
                    return;
                }
                let Some(pod) = pod else { return };
                let mut info = VideoInfoRaw::new();
                if info.parse(pod).is_ok() {
                    size.set((info.size().width, info.size().height));
                    println!("format {}x{}", info.size().width, info.size().height);
                }
            }
        })
        .process({
            let size = size.clone();
            let frames = frames.clone();
            move |stream, ()| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let datas = buffer.datas_mut();
                let Some(data) = datas.first_mut() else {
                    return;
                };
                let chunk = data.chunk();
                let (offset, stride, len) = (
                    chunk.offset() as usize,
                    chunk.stride() as usize,
                    chunk.size() as usize,
                );
                if len == 0 {
                    return;
                }
                let (w, h) = size.get();
                let Some(bytes) = data.data() else {
                    return;
                };
                // BGRx: the centre pixel's blue, green and red.
                let i = offset + (h as usize / 2) * stride + (w as usize / 2) * 4;
                if i + 3 > bytes.len() {
                    return;
                }
                frames.set(frames.get() + 1);
                println!(
                    "frame {} {}x{} {:02x}{:02x}{:02x}",
                    frames.get(),
                    w,
                    h,
                    bytes[i + 2],
                    bytes[i + 1],
                    bytes[i]
                );
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
