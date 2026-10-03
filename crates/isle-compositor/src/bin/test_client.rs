//! A Wayland client for the integration tests: one window of one colour, printing what
//! it receives, one event per line, to standard output.

use std::{
    io::Write,
    os::fd::{AsFd, FromRawFd, OwnedFd},
};

use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_shm,
        wl_shm_pool, wl_surface,
    },
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

struct Client {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    seat: Option<wl_seat::WlSeat>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    surface: Option<wl_surface::WlSurface>,
    size: (i32, i32),
    colour: [u8; 4],
    closed: bool,
}

fn say(line: String) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

impl Client {
    fn draw(&mut self, qh: &QueueHandle<Self>) {
        let (w, h) = self.size;
        let stride = w * 4;
        let len = (stride * h) as usize;
        let fd = unsafe { libc::memfd_create(c"isle-test-client".as_ptr(), libc::MFD_CLOEXEC) };
        assert!(fd >= 0, "memfd_create");
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        file.set_len(len as u64).unwrap();
        let [r, g, b, a] = self.colour;
        let pixel = [b, g, r, a];
        let mut data = Vec::with_capacity(len);
        for _ in 0..(w * h) {
            data.extend_from_slice(&pixel);
        }
        std::os::unix::fs::FileExt::write_all_at(&file, &data, 0).unwrap();
        let fd: OwnedFd = file.into();
        let shm = self.shm.as_ref().unwrap();
        let pool = shm.create_pool(fd.as_fd(), len as i32, qh, ());
        let buffer = pool.create_buffer(0, w, h, stride, wl_shm::Format::Argb8888, qh, ());
        pool.destroy();
        let surface = self.surface.as_ref().unwrap();
        surface.attach(Some(&buffer), 0, 0);
        surface.damage_buffer(0, 0, w, h);
        surface.commit();
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for Client {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => state.compositor = Some(registry.bind(name, 4, qh, ())),
                "wl_shm" => state.shm = Some(registry.bind(name, 1, qh, ())),
                "xdg_wm_base" => state.wm_base = Some(registry.bind(name, 1, qh, ())),
                "wl_seat" => state.seat = Some(registry.bind(name, 5, qh, ())),
                "zwlr_layer_shell_v1" => state.layer_shell = Some(registry.bind(name, 4, qh, ())),
                _ => {}
            }
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for Client {
    fn event(
        _: &mut Self,
        base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for Client {
    fn event(
        state: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            say(format!("configured {} {}", state.size.0, state.size.1));
            state.draw(qh);
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for Client {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } if width > 0 && height > 0 => {
                state.size = (width, height);
            }
            xdg_toplevel::Event::Close => {
                say("closed".into());
                state.closed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for Client {
    fn event(
        _: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(caps),
        } = event
        {
            if caps.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
            if caps.contains(wl_seat::Capability::Pointer) {
                seat.get_pointer(qh, ());
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for Client {
    fn event(
        _: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { .. } => say("keyboard-enter".into()),
            wl_keyboard::Event::Leave { .. } => say("keyboard-leave".into()),
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(s),
                ..
            } => say(format!(
                "key {key} {}",
                if s == wl_keyboard::KeyState::Pressed {
                    "pressed"
                } else {
                    "released"
                }
            )),
            _ => {}
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for Client {
    fn event(
        _: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface_x,
                surface_y,
                ..
            } => say(format!("pointer-enter {surface_x:.0} {surface_y:.0}")),
            wl_pointer::Event::Button {
                button,
                state: WEnum::Value(s),
                ..
            } => say(format!(
                "button {button} {}",
                if s == wl_pointer::ButtonState::Pressed {
                    "pressed"
                } else {
                    "released"
                }
            )),
            _ => {}
        }
    }
}

macro_rules! ignore {
    ($($t:ty),*) => {$(
        impl Dispatch<$t, ()> for Client {
            fn event(_: &mut Self, _: &$t, _: <$t as wayland_client::Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
        }
    )*};
}
impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for Client {
    fn event(
        state: &mut Self,
        layer: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                layer.ack_configure(serial);
                if width > 0 {
                    state.size.0 = width as i32;
                }
                if height > 0 {
                    state.size.1 = height as i32;
                }
                say(format!("configured {} {}", state.size.0, state.size.1));
                state.draw(qh);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                say("closed".into());
                state.closed = true;
            }
            _ => {}
        }
    }
}

ignore!(
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    wl_compositor::WlCompositor,
    wl_shm::WlShm,
    wl_shm_pool::WlShmPool,
    wl_buffer::WlBuffer,
    wl_surface::WlSurface
);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str, default: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1).cloned())
            .unwrap_or_else(|| default.to_owned())
    };
    let app_id = arg("--app-id", "isle.test.Client");
    let title = arg("--title", "test client");
    let (w, h) = arg("--size", "400x300")
        .split_once('x')
        .map(|(w, h)| (w.parse().unwrap(), h.parse().unwrap()))
        .unwrap();
    let hex = u32::from_str_radix(&arg("--colour", "ff3366"), 16).unwrap();
    let colour = [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 0xff];

    let connection = Connection::connect_to_env().expect("a Wayland display");
    let mut queue = connection.new_event_queue();
    let qh = queue.handle();
    connection.display().get_registry(&qh, ());
    let mut client = Client {
        compositor: None,
        shm: None,
        wm_base: None,
        seat: None,
        layer_shell: None,
        surface: None,
        size: (w, h),
        colour,
        closed: false,
    };
    queue.roundtrip(&mut client).unwrap();

    let surface = client.compositor.as_ref().unwrap().create_surface(&qh, ());
    if args.iter().any(|a| a == "--layer") {
        use zwlr_layer_shell_v1::Layer;
        use zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity};
        let layer = match arg("--layer", "top").as_str() {
            "background" => Layer::Background,
            "bottom" => Layer::Bottom,
            "overlay" => Layer::Overlay,
            _ => Layer::Top,
        };
        let shell = client.layer_shell.as_ref().expect("zwlr_layer_shell_v1");
        let layer_surface = shell.get_layer_surface(&surface, None, layer, app_id, &qh, ());
        let anchor = match arg("--anchor", "top").as_str() {
            "bottom" => Anchor::Bottom | Anchor::Left | Anchor::Right,
            "left" => Anchor::Left | Anchor::Top | Anchor::Bottom,
            "right" => Anchor::Right | Anchor::Top | Anchor::Bottom,
            _ => Anchor::Top | Anchor::Left | Anchor::Right,
        };
        layer_surface.set_anchor(anchor);
        let (lw, lh) = (client.size.0 as u32, client.size.1 as u32);
        if anchor.contains(Anchor::Left | Anchor::Right) {
            layer_surface.set_size(0, lh);
        } else {
            layer_surface.set_size(lw, 0);
        }
        layer_surface.set_exclusive_zone(arg("--exclusive", "0").parse().unwrap());
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
    } else {
        let xdg_surface = client
            .wm_base
            .as_ref()
            .unwrap()
            .get_xdg_surface(&surface, &qh, ());
        let toplevel = xdg_surface.get_toplevel(&qh, ());
        toplevel.set_app_id(app_id);
        toplevel.set_title(title);
    }
    surface.commit();
    client.surface = Some(surface);

    while !client.closed {
        if queue.blocking_dispatch(&mut client).is_err() {
            break;
        }
    }
}
