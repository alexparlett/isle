//! A Wayland client for the integration tests: windows of one colour, printing what they
//! receive, one event per line, to standard output, and taking commands, one per line, on
//! standard input. Lines about any window but the first start with `wN `.
//!
//! Commands, to the first window unless they start with `wN `: `title T`, `app-id A`,
//! `fullscreen`, `unfullscreen`, `maximize`, `unmaximize`, `minimize`, `min W H`, `max W H`,
//! `size W H` (draw at that size), `geometry X Y W H` (the window's geometry within a larger
//! buffer), `decoration client|server|unset`, `move-on-press`, `resize-on-press EDGE`,
//! `window` (another toplevel), `dialog` and `modal` (a toplevel whose parent is this one),
//! `destroy`, `unmap` (a null buffer), `remap` (a commit that asks to be configured again),
//! `pings ignore|answer` (answering those ignored so far), `inhibit` (hold a keyboard
//! shortcuts inhibitor, printing `inhibitor active|inactive`), `linger` (a close of the
//! first window destroys it but leaves the process running), `quit`. A close of any other
//! window destroys it.
//!
//! Keys print as `key CODE pressed`, then `sym NAME pressed` from the keymap the client was
//! sent; `keymap LAYOUT` names a keymap's first layout and `mods Control+Mod4` the
//! modifiers in force after each `modifiers` event.

use std::{
    io::{BufRead, Write},
    os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
};

use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_shm,
        wl_shm_pool, wl_surface,
    },
};
use wayland_protocols::xdg::{
    activation::v1::client::xdg_activation_v1,
    decoration::zv1::client::{zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1},
    dialog::v1::client::{xdg_dialog_v1, xdg_wm_dialog_v1},
    shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base},
};
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::{
    zwp_keyboard_shortcuts_inhibit_manager_v1, zwp_keyboard_shortcuts_inhibitor_v1,
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

struct Win {
    surface: wl_surface::WlSurface,
    xdg: Option<(xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel)>,
    decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    /// The size the next configure asks for, or the size the window chose.
    size: (i32, i32),
    /// Where the window's geometry sits in its buffer, and the buffer's size, when it draws
    /// more than its geometry.
    inset: Option<(i32, i32, i32, i32)>,
    colour: [u8; 4],
    gone: bool,
}

struct Client {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    seat: Option<wl_seat::WlSeat>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    decorations: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    dialogs: Option<xdg_wm_dialog_v1::XdgWmDialogV1>,
    inhibits: Option<zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1>,
    activation: Option<xdg_activation_v1::XdgActivationV1>,
    wins: Vec<Win>,
    ignore_pings: bool,
    ignored_pings: Vec<u32>,
    linger: bool,
    /// The next button press starts a move, or a resize from this edge.
    on_press: Option<Option<xdg_toplevel::ResizeEdge>>,
    /// The window the pointer is over, by index.
    pointer_on: Option<usize>,
    quit: bool,
    app_id: String,
    /// The first window has made its first commit.
    started: bool,
    xkb: Option<isle_keys::xkb::State>,
}

fn say(index: usize, line: String) {
    let mut out = std::io::stdout().lock();
    let _ = if index == 0 {
        writeln!(out, "{line}")
    } else {
        writeln!(out, "w{index} {line}")
    };
    let _ = out.flush();
}

impl Client {
    fn draw(&mut self, index: usize, qh: &QueueHandle<Self>) {
        let win = &self.wins[index];
        let (w, h) = match win.inset {
            Some((_, _, bw, bh)) => (bw, bh),
            None => win.size,
        };
        let stride = w * 4;
        let len = (stride * h) as usize;
        let fd = unsafe { libc::memfd_create(c"isle-test-client".as_ptr(), libc::MFD_CLOEXEC) };
        assert!(fd >= 0, "memfd_create");
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        file.set_len(len as u64).unwrap();
        let [r, g, b, a] = win.colour;
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
        if let (Some((x, y, _, _)), Some((xdg, _))) = (win.inset, &win.xdg) {
            xdg.set_window_geometry(x, y, win.size.0, win.size.1);
        }
        win.surface.attach(Some(&buffer), 0, 0);
        win.surface.damage_buffer(0, 0, w, h);
        win.surface.commit();
    }

    fn index_of_surface(&self, surface: &wl_surface::WlSurface) -> Option<usize> {
        self.wins.iter().position(|w| &w.surface == surface)
    }

    fn add_toplevel(
        &mut self,
        qh: &QueueHandle<Self>,
        app_id: &str,
        title: &str,
        size: (i32, i32),
        colour: [u8; 4],
        parent: Option<usize>,
    ) -> usize {
        let index = self.wins.len();
        let surface = self.compositor.as_ref().unwrap().create_surface(qh, ());
        let xdg_surface = self
            .wm_base
            .as_ref()
            .unwrap()
            .get_xdg_surface(&surface, qh, index);
        let toplevel = xdg_surface.get_toplevel(qh, index);
        toplevel.set_app_id(app_id.to_owned());
        toplevel.set_title(title.to_owned());
        if let Some(parent) = parent.and_then(|p| self.wins[p].xdg.as_ref()) {
            toplevel.set_parent(Some(&parent.1));
        }
        self.wins.push(Win {
            surface,
            xdg: Some((xdg_surface, toplevel)),
            decoration: None,
            size,
            inset: None,
            colour,
            gone: false,
        });
        index
    }

    fn command(&mut self, line: &str, qh: &QueueHandle<Self>) {
        let (index, line) = match line.strip_prefix('w').and_then(|l| l.split_once(' ')) {
            Some((n, rest)) if n.parse::<usize>().is_ok() => (n.parse().unwrap(), rest),
            _ => (0, line),
        };
        let mut words = line.split_whitespace();
        let Some(verb) = words.next() else {
            return;
        };
        let rest: Vec<&str> = words.collect();
        let num = |i: usize| rest.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
        if index >= self.wins.len() || self.wins[index].gone {
            return;
        }
        let toplevel = self.wins[index].xdg.as_ref().map(|(_, t)| t.clone());
        match (verb, toplevel) {
            ("title", Some(t)) => t.set_title(rest.join(" ")),
            ("app-id", Some(t)) => t.set_app_id(rest.join(" ")),
            ("fullscreen", Some(t)) => t.set_fullscreen(None),
            ("unfullscreen", Some(t)) => t.unset_fullscreen(),
            ("maximize", Some(t)) => t.set_maximized(),
            ("unmaximize", Some(t)) => t.unset_maximized(),
            ("minimize", Some(t)) => t.set_minimized(),
            ("min", Some(t)) => t.set_min_size(num(0), num(1)),
            ("max", Some(t)) => t.set_max_size(num(0), num(1)),
            ("size", _) => {
                self.wins[index].size = (num(0), num(1));
                self.draw(index, qh);
                return;
            }
            ("geometry", _) => {
                let (x, y, w, h) = (num(0), num(1), num(2), num(3));
                self.wins[index].size = (w, h);
                self.wins[index].inset = Some((x, y, w + 2 * x, h + 2 * y));
                self.draw(index, qh);
                return;
            }
            ("decoration", Some(t)) => {
                let Some(manager) = self.decorations.as_ref() else {
                    say(index, "no decoration manager".into());
                    return;
                };
                let win = &mut self.wins[index];
                let decoration = win
                    .decoration
                    .get_or_insert_with(|| manager.get_toplevel_decoration(&t, qh, index));
                match rest.first().copied() {
                    Some("client") => {
                        decoration.set_mode(zxdg_toplevel_decoration_v1::Mode::ClientSide)
                    }
                    Some("server") => {
                        decoration.set_mode(zxdg_toplevel_decoration_v1::Mode::ServerSide)
                    }
                    _ => decoration.unset_mode(),
                }
            }
            ("move-on-press", _) => self.on_press = Some(None),
            ("resize-on-press", _) => {
                use xdg_toplevel::ResizeEdge as E;
                let edge = match rest.first().copied() {
                    Some("top") => E::Top,
                    Some("bottom") => E::Bottom,
                    Some("left") => E::Left,
                    Some("right") => E::Right,
                    Some("top-left") => E::TopLeft,
                    Some("top-right") => E::TopRight,
                    Some("bottom-left") => E::BottomLeft,
                    _ => E::BottomRight,
                };
                self.on_press = Some(Some(edge));
            }
            ("window" | "dialog" | "modal", _) => {
                let parent = (verb != "window").then_some(index);
                let colour = self.wins[index].colour;
                let title = format!("{verb} {}", self.wins.len());
                let app_id = self.app_id.clone();
                let new = self.add_toplevel(qh, &app_id, &title, (300, 200), colour, parent);
                if verb == "modal"
                    && let (Some(dialogs), Some((_, t))) = (&self.dialogs, &self.wins[new].xdg)
                {
                    let dialog = dialogs.get_xdg_dialog(t, qh, ());
                    dialog.set_modal();
                }
                self.wins[new].surface.commit();
                say(new, "created".into());
            }
            ("destroy", _) => {
                let win = &mut self.wins[index];
                if let Some((surface, toplevel)) = win.xdg.take() {
                    toplevel.destroy();
                    surface.destroy();
                }
                win.surface.destroy();
                win.gone = true;
                say(index, "destroyed".into());
            }
            ("unmap", _) => {
                self.wins[index].surface.attach(None, 0, 0);
            }
            // An unmapped toplevel has lost its app id and title with the rest of its state.
            ("remap", Some(t)) => {
                t.set_app_id(self.app_id.clone());
                t.set_title(format!("remapped {index}"));
            }
            ("pings", _) => {
                self.ignore_pings = rest.first() == Some(&"ignore");
                if !self.ignore_pings
                    && let Some(base) = &self.wm_base
                {
                    for serial in self.ignored_pings.drain(..) {
                        base.pong(serial);
                    }
                }
            }
            ("linger", _) => self.linger = true,
            ("inhibit", _) => {
                if let (Some(manager), Some(seat)) = (&self.inhibits, &self.seat) {
                    manager.inhibit_shortcuts(&self.wins[index].surface, seat, qh, ());
                }
            }
            ("quit", _) => self.quit = true,
            _ => say(index, format!("unknown command {verb}")),
        }
        if let Some(win) = self.wins.get(index)
            && !win.gone
            && self.started
        {
            win.surface.commit();
        }
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
                "xdg_wm_base" => state.wm_base = Some(registry.bind(name, 6, qh, ())),
                "wl_seat" => state.seat = Some(registry.bind(name, 5, qh, ())),
                "zwlr_layer_shell_v1" => state.layer_shell = Some(registry.bind(name, 4, qh, ())),
                "zxdg_decoration_manager_v1" => {
                    state.decorations = Some(registry.bind(name, 1, qh, ()))
                }
                "xdg_wm_dialog_v1" => state.dialogs = Some(registry.bind(name, 1, qh, ())),
                "zwp_keyboard_shortcuts_inhibit_manager_v1" => {
                    state.inhibits = Some(registry.bind(name, 1, qh, ()))
                }
                "xdg_activation_v1" => state.activation = Some(registry.bind(name, 1, qh, ())),
                _ => {}
            }
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for Client {
    fn event(
        state: &mut Self,
        base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            if state.ignore_pings {
                say(0, "ping ignored".into());
                state.ignored_pings.push(serial);
            } else {
                base.pong(serial);
            }
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, usize> for Client {
    fn event(
        state: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        index: &usize,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            let (w, h) = state.wins[*index].size;
            say(*index, format!("configured {w} {h}"));
            state.draw(*index, qh);
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, usize> for Client {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                if width > 0 && height > 0 {
                    state.wins[*index].size = (width, height);
                }
                let names: Vec<&str> = states
                    .chunks_exact(4)
                    .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .map(|s| match xdg_toplevel::State::try_from(s) {
                        Ok(xdg_toplevel::State::Maximized) => "maximized",
                        Ok(xdg_toplevel::State::Fullscreen) => "fullscreen",
                        Ok(xdg_toplevel::State::Resizing) => "resizing",
                        Ok(xdg_toplevel::State::Activated) => "activated",
                        Ok(xdg_toplevel::State::TiledLeft) => "tiled-left",
                        Ok(xdg_toplevel::State::TiledRight) => "tiled-right",
                        Ok(xdg_toplevel::State::TiledTop) => "tiled-top",
                        Ok(xdg_toplevel::State::TiledBottom) => "tiled-bottom",
                        Ok(xdg_toplevel::State::Suspended) => "suspended",
                        _ => "other",
                    })
                    .collect();
                say(*index, format!("states {}", names.join(",")));
            }
            xdg_toplevel::Event::Close => {
                say(*index, "closed".into());
                if *index == 0 && !state.linger {
                    state.quit = true;
                } else {
                    let win = &mut state.wins[*index];
                    if let Some((surface, toplevel)) = win.xdg.take() {
                        toplevel.destroy();
                        surface.destroy();
                    }
                    win.surface.destroy();
                    win.gone = true;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, usize> for Client {
    fn event(
        _: &mut Self,
        _: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_toplevel_decoration_v1::Event::Configure {
            mode: WEnum::Value(mode),
        } = event
        {
            let mode = match mode {
                zxdg_toplevel_decoration_v1::Mode::ServerSide => "server",
                _ => "client",
            };
            say(*index, format!("decoration {mode}"));
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
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { surface, .. } => say(
                state.index_of_surface(&surface).unwrap_or(0),
                "keyboard-enter".into(),
            ),
            wl_keyboard::Event::Leave { surface, .. } => say(
                state.index_of_surface(&surface).unwrap_or(0),
                "keyboard-leave".into(),
            ),
            wl_keyboard::Event::Keymap { format, fd, size } => {
                use isle_keys::xkb;
                if format != WEnum::Value(wl_keyboard::KeymapFormat::XkbV1) {
                    return;
                }
                let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
                // Safety: the compositor shares the keymap read-only at this size.
                let keymap = unsafe {
                    xkb::Keymap::new_from_fd(
                        &context,
                        fd,
                        size as usize,
                        xkb::KEYMAP_FORMAT_TEXT_V1,
                        xkb::KEYMAP_COMPILE_NO_FLAGS,
                    )
                };
                if let Ok(Some(keymap)) = keymap {
                    say(0, format!("keymap {}", keymap.layout_get_name(0)));
                    state.xkb = Some(xkb::State::new(&keymap));
                }
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                use isle_keys::xkb;
                if let Some(xkb_state) = state.xkb.as_mut() {
                    xkb_state.update_mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
                    let names: Vec<&str> =
                        ["Shift", "Lock", "Control", "Mod1", "Mod2", "Mod4", "Mod5"]
                            .into_iter()
                            .filter(|m| xkb_state.mod_name_is_active(m, xkb::STATE_MODS_EFFECTIVE))
                            .collect();
                    say(
                        0,
                        format!(
                            "mods {}",
                            if names.is_empty() {
                                "none".into()
                            } else {
                                names.join("+")
                            }
                        ),
                    );
                }
            }
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(s),
                ..
            } => {
                let pressed = if s == wl_keyboard::KeyState::Pressed {
                    "pressed"
                } else {
                    "released"
                };
                say(0, format!("key {key} {pressed}"));
                if let Some(xkb_state) = state.xkb.as_ref() {
                    let sym = xkb_state.key_get_one_sym((key + 8).into());
                    say(
                        0,
                        format!("sym {} {pressed}", isle_keys::xkb::keysym_get_name(sym)),
                    );
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for Client {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                state.pointer_on = state.index_of_surface(&surface);
                say(
                    state.pointer_on.unwrap_or(0),
                    format!("pointer-enter {surface_x:.0} {surface_y:.0}"),
                );
            }
            wl_pointer::Event::Leave { .. } => state.pointer_on = None,
            wl_pointer::Event::Button {
                serial,
                button,
                state: WEnum::Value(s),
                ..
            } => {
                let pressed = s == wl_pointer::ButtonState::Pressed;
                let index = state.pointer_on.unwrap_or(0);
                say(
                    index,
                    format!(
                        "button {button} {}",
                        if pressed { "pressed" } else { "released" }
                    ),
                );
                if pressed
                    && let Some(grab) = state.on_press.take()
                    && let (Some(seat), Some((_, toplevel))) = (
                        &state.seat,
                        state.wins.get(index).and_then(|w| w.xdg.as_ref()),
                    )
                {
                    match grab {
                        None => toplevel._move(seat, serial),
                        Some(edge) => toplevel.resize(seat, serial, edge),
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1, ()> for Client {
    fn event(
        _: &mut Self,
        _: &zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1,
        event: zwp_keyboard_shortcuts_inhibitor_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwp_keyboard_shortcuts_inhibitor_v1::Event::Active => say(0, "inhibitor active".into()),
            zwp_keyboard_shortcuts_inhibitor_v1::Event::Inactive => say(0, "inhibitor inactive".into()),
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
                let win = &mut state.wins[0];
                if width > 0 {
                    win.size.0 = width as i32;
                }
                if height > 0 {
                    win.size.1 = height as i32;
                }
                say(0, format!("configured {} {}", win.size.0, win.size.1));
                state.draw(0, qh);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                say(0, "closed".into());
                state.quit = true;
            }
            _ => {}
        }
    }
}

ignore!(
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
    xdg_wm_dialog_v1::XdgWmDialogV1,
    xdg_dialog_v1::XdgDialogV1,
    zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
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
    let size = arg("--size", "400x300")
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
        decorations: None,
        dialogs: None,
        inhibits: None,
        activation: None,
        wins: Vec::new(),
        ignore_pings: false,
        ignored_pings: Vec::new(),
        linger: false,
        on_press: None,
        pointer_on: None,
        quit: false,
        app_id: app_id.clone(),
        started: false,
        xkb: None,
    };
    queue.roundtrip(&mut client).unwrap();

    if args.iter().any(|a| a == "--layer") {
        use zwlr_layer_shell_v1::Layer;
        use zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity};
        let surface = client.compositor.as_ref().unwrap().create_surface(&qh, ());
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
        let (lw, lh) = (size.0 as u32, size.1 as u32);
        if anchor.contains(Anchor::Left | Anchor::Right) {
            layer_surface.set_size(0, lh);
        } else {
            layer_surface.set_size(lw, 0);
        }
        layer_surface.set_exclusive_zone(arg("--exclusive", "0").parse().unwrap());
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        surface.commit();
        client.wins.push(Win {
            surface,
            xdg: None,
            decoration: None,
            size,
            inset: None,
            colour,
            gone: false,
        });
    } else {
        // The commands given as arguments come before the first commit.
        client.add_toplevel(&qh, &app_id, &title, size, colour, None);
        for command in args
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "--do")
            .filter_map(|(i, _)| args.get(i + 1))
        {
            client.command(command, &qh);
        }
        client.wins[0].surface.commit();
        // Started by a launcher, the window asks for focus with the token it was given.
        if let (Ok(token), Some(activation)) =
            (std::env::var("XDG_ACTIVATION_TOKEN"), client.activation.as_ref())
        {
            activation.activate(token.clone(), &client.wins[0].surface);
            say(0, format!("activated with {token}"));
        }
    }
    client.started = true;

    let stdin = std::io::stdin();
    let mut stdin_open = true;
    let mut lines = stdin.lock();
    while !client.quit {
        if connection.flush().is_err() {
            break;
        }
        let Some(guard) = queue.prepare_read() else {
            if queue.dispatch_pending(&mut client).is_err() {
                break;
            }
            continue;
        };
        let mut fds = [
            libc::pollfd {
                fd: guard.connection_fd().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: if stdin_open { 0 } else { -1 },
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        if unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) } < 0 {
            continue;
        }
        if fds[0].revents & libc::POLLIN != 0 {
            if guard.read().is_err() {
                break;
            }
        } else {
            drop(guard);
        }
        if queue.dispatch_pending(&mut client).is_err() {
            break;
        }
        if fds[1].revents & (libc::POLLIN | libc::POLLHUP) != 0 {
            let mut line = String::new();
            match lines.read_line(&mut line) {
                Ok(0) | Err(_) => stdin_open = false,
                Ok(_) => client.command(line.trim(), &qh),
            }
        }
    }
}

impl Dispatch<xdg_activation_v1::XdgActivationV1, ()> for Client {
    fn event(
        _: &mut Self,
        _: &xdg_activation_v1::XdgActivationV1,
        _: xdg_activation_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
