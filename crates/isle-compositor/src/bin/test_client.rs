//! A Wayland client for the integration tests: windows of one colour, printing what they
//! receive, one event per line, to standard output, and taking commands, one per line, on
//! standard input. Lines about any window but the first start with `wN `.
//!
//! Commands, to the first window unless they start with `wN `: `title T`, `app-id A`,
//! `fullscreen`, `unfullscreen`, `maximize`, `unmaximize`, `minimize`, `min W H`, `max W H`,
//! `size W H` (draw at that size), `colour RRGGBB` (draw in it), `geometry X Y W H` (the window's geometry within a larger
//! buffer), `decoration client|server|unset`, `move-on-press`, `resize-on-press EDGE`,
//! `window` (another toplevel), `dialog` and `modal` (a toplevel whose parent is this one),
//! `destroy`, `unmap` (a null buffer), `remap` (a commit that asks to be configured again),
//! `pings ignore|answer` (answering those ignored so far), `inhibit` (hold a keyboard
//! shortcuts inhibitor, printing `inhibitor active|inactive`), `idle-inhibit` and
//! `idle-uninhibit` (hold an idle inhibitor on the window, and let it go), `idle-notify MS`
//! (an ext-idle-notify notification after MS of idle, printing `idled` and `resumed`), `linger` (a close of the
//! first window destroys it but leaves the process running), `keep-open` (every close is
//! refused, as by an app with unsaved work), `badge count=N progress=F
//! urgent=B` (a LauncherEntry Update for the app id's entry; any of the three), `notify
//! key=value...` (summary, body, icon, actions=key:Label,..., urgency, transient,
//! resident, desktop-entry, image, replaces, timeout; underscores read as spaces),
//! `close-notification ID`, `use-token TOKEN` (an activation with a token it was sent),
//! `export` (an xdg-foreign v2 handle for the window, printed as `exported HANDLE`),
//! `cursor-surface` (a 16-point magenta cursor surface, hotspot 2 3, set at each pointer
//! enter), `copy-screen` (an ext-image-copy-capture session of the screen, a frame after each
//! frame, printing `copied N`, then `copy stopped`), `copy-cursor` (a pointer cursor
//! session of the screen, printing `cursor-enter`, `cursor-position X Y`, `cursor-hotspot X
//! Y`, `cursor-leave` and `cursor copied N RRGGBBAA` with the pixel at 5, 5),
//! `tray [--name] [--item-is-menu] [--id ID]` (a status item with a pixmap icon and the test menu,
//! registered by path or by a well-known name and again whenever the watcher comes back,
//! printing `tray registered`, `tray activated`, `tray secondary`, `tray scroll N` and
//! `menu activated LABEL`), `tray-attention` (it asks for attention),
//! `drag-on-press` (the next press starts a drag of text with a magenta icon; drags print
//! `dnd enter`, `dnd drop`, and to the source `dnd finished` or `dnd cancelled`), `quit`. A
//! close of any other window destroys it.
//!
//! Keys print as `key CODE pressed`, then `sym NAME pressed` from the keymap the client was
//! sent; `keymap LAYOUT` names a keymap's first layout and `mods Control+Mod4` the
//! modifiers in force after each `modifiers` event.

use std::{
    io::Write,
    os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
};

use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_data_device, wl_data_device_manager, wl_data_offer,
        wl_data_source, wl_keyboard, wl_output, wl_pointer, wl_registry, wl_seat, wl_shm,
        wl_shm_pool, wl_surface,
    },
};
use wayland_protocols::xdg::{
    activation::v1::client::xdg_activation_v1,
    decoration::zv1::client::{zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1},
    dialog::v1::client::{xdg_dialog_v1, xdg_wm_dialog_v1},
    shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base},
};
use wayland_protocols::xdg::foreign::zv2::client::{zxdg_exported_v2, zxdg_exporter_v2};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_image_capture_source_v1, ext_output_image_capture_source_manager_v1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_cursor_session_v1, ext_image_copy_capture_frame_v1,
    ext_image_copy_capture_manager_v1, ext_image_copy_capture_session_v1,
};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1, ext_idle_notifier_v1,
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1, zwp_idle_inhibitor_v1,
};
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::{
    zwp_keyboard_shortcuts_inhibit_manager_v1, zwp_keyboard_shortcuts_inhibitor_v1,
};
use wayland_protocols_plasma::appmenu::client::{org_kde_kwin_appmenu, org_kde_kwin_appmenu_manager};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

#[allow(missing_docs, clippy::all, non_upper_case_globals, non_camel_case_types, unused_imports)]
mod gtk_shell {
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../../data/protocols/gtk-shell.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("../../data/protocols/gtk-shell.xml");
}
use gtk_shell::{gtk_shell1, gtk_surface1};

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
    idle_inhibits: Option<zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1>,
    idle_inhibitor: Option<zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1>,
    idle_notifier: Option<ext_idle_notifier_v1::ExtIdleNotifierV1>,
    activation: Option<xdg_activation_v1::XdgActivationV1>,
    exporter: Option<zxdg_exporter_v2::ZxdgExporterV2>,
    output: Option<wl_output::WlOutput>,
    output_sources: Option<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1>,
    copy_manager: Option<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1>,
    copying: Option<Copying>,
    cursor_copying: Option<Copying>,
    pointer: Option<wl_pointer::WlPointer>,
    appmenus: Option<org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager>,
    gtk_shell: Option<gtk_shell1::GtkShell1>,
    wins: Vec<Win>,
    /// A window drawn again at every frame callback, its colour turning: a stand-in game.
    animating: Option<usize>,
    ignore_pings: bool,
    ignored_pings: Vec<u32>,
    linger: bool,
    /// A close is answered by staying open, as an app with unsaved work does.
    keep_open: bool,
    /// The next button press starts a move, or a resize from this edge.
    on_press: Option<Option<xdg_toplevel::ResizeEdge>>,
    /// The window the pointer is over, by index.
    pointer_on: Option<usize>,
    cursor_surface: bool,
    quit: bool,
    app_id: String,
    /// The first window has made its first commit.
    started: bool,
    xkb: Option<isle_keys::xkb::State>,
    data_manager: Option<wl_data_device_manager::WlDataDeviceManager>,
    data_device: Option<wl_data_device::WlDataDevice>,
    drag_on_press: bool,
    /// The clipboard as offered to this client, and the text this client offers.
    selection: Option<wl_data_offer::WlDataOffer>,
    /// A drag over this client's window.
    dnd_offer: Option<wl_data_offer::WlDataOffer>,
    copied: Option<String>,
    /// The latest serial from the keyboard, for setting the clipboard.
    key_serial: u32,
    /// The bus `badge` sends on, kept so the sender stays until the client exits.
    launcher: Option<(tokio::runtime::Runtime, zbus::Connection)>,
    /// The server's signals are being printed.
    listening: bool,
    /// The status item's bus, kept so the item stays until the client exits.
    tray: Option<(tokio::runtime::Runtime, zbus::Connection)>,
}

/// A status item for the tray's tests.
struct TrayItem {
    id: String,
    item_is_menu: bool,
    status: std::sync::Mutex<String>,
}

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl TrayItem {
    fn activate(&self, _x: i32, _y: i32) {
        say(0, "tray activated".into());
    }

    fn secondary_activate(&self, _x: i32, _y: i32) {
        say(0, "tray secondary".into());
    }

    fn scroll(&self, delta: i32, _orientation: String) {
        say(0, format!("tray scroll {delta}"));
    }

    #[zbus(property)]
    fn id(&self) -> String {
        self.id.clone()
    }

    #[zbus(property)]
    fn title(&self) -> String {
        "Test Tray".into()
    }

    #[zbus(property)]
    fn category(&self) -> String {
        "ApplicationStatus".into()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        self.status.lock().unwrap().clone()
    }

    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        // Opaque magenta, ARGB in network order.
        vec![(16, 16, [0xff, 0xff, 0x00, 0xff].repeat(16 * 16))]
    }

    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        self.item_is_menu
    }

    #[zbus(property)]
    fn menu(&self) -> zvariant::OwnedObjectPath {
        zvariant::OwnedObjectPath::try_from("/MenuBar/1").unwrap()
    }

    #[zbus(signal)]
    async fn new_status(emitter: &zbus::object_server::SignalEmitter<'_>, status: &str) -> zbus::Result<()>;
}

/// Serves the test status item and registers it, again whenever the watcher comes back.
fn tray(args: &[&str]) -> (tokio::runtime::Runtime, zbus::Connection) {
    let by_name = args.contains(&"--name");
    let item_is_menu = args.contains(&"--item-is-menu");
    let id = args
        .iter()
        .position(|a| *a == "--id")
        .and_then(|i| args.get(i + 1))
        .map_or("test-tray", |v| v)
        .to_owned();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let conn = runtime.block_on(async {
        let conn = zbus::Connection::session().await.unwrap();
        let on_activate: isle_menus::exporter::OnActivate =
            std::sync::Arc::new(|label| say(0, format!("menu activated {label}")));
        let server = conn.object_server();
        server
            .at("/MenuBar/1", isle_menus::exporter::DBusMenuExport::new(on_activate))
            .await
            .unwrap();
        server
            .at(
                "/StatusNotifierItem",
                TrayItem {
                    id,
                    item_is_menu,
                    status: std::sync::Mutex::new("Active".into()),
                },
            )
            .await
            .unwrap();
        let service = if by_name {
            let name = format!("org.kde.StatusNotifierItem-{}-1", std::process::id());
            conn.request_name(name.as_str()).await.unwrap();
            name
        } else {
            "/StatusNotifierItem".to_owned()
        };
        let watched = conn.clone();
        tokio::spawn(async move {
            use futures_util::StreamExt;
            let dbus = zbus::fdo::DBusProxy::new(&watched).await.unwrap();
            let mut owners = dbus
                .receive_name_owner_changed_with_args(&[(0, "org.kde.StatusNotifierWatcher")])
                .await
                .unwrap();
            let register = || async {
                let registered = watched
                    .call_method(
                        Some("org.kde.StatusNotifierWatcher"),
                        "/StatusNotifierWatcher",
                        Some("org.kde.StatusNotifierWatcher"),
                        "RegisterStatusNotifierItem",
                        &(service.as_str(),),
                    )
                    .await;
                if registered.is_ok() {
                    say(0, "tray registered".into());
                }
            };
            register().await;
            while let Some(change) = owners.next().await {
                if change.args().is_ok_and(|a| a.new_owner().is_some()) {
                    register().await;
                }
            }
        });
        conn
    });
    (runtime, conn)
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
        if self.animating == Some(index) {
            win.surface.frame(qh, index);
        }
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

    /// Reads the clipboard's text off the event loop and prints it.
    fn paste(&mut self, conn: &Connection) {
        let Some(offer) = &self.selection else {
            say(0, "pasted nothing".into());
            return;
        };
        let mut fds = [0; 2];
        // Safety: room for both ends.
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) };
        // Safety: both ends are new and owned here.
        let (read, write) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        offer.receive("text/plain;charset=utf-8".into(), write.as_fd());
        drop(write);
        let _ = conn.flush();
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = std::io::Read::read_to_string(&mut std::fs::File::from(read), &mut text);
            say(0, format!("pasted {text}"));
        });
    }

    /// Offers `text` as the clipboard, marked a secret as password managers mark theirs.
    fn copy(&mut self, text: String, secret: bool, qh: &QueueHandle<Self>) {
        let (Some(manager), Some(device)) = (&self.data_manager, &self.data_device) else {
            return;
        };
        let source = manager.create_data_source(qh, ());
        source.offer("text/plain;charset=utf-8".into());
        source.offer("text/plain".into());
        if secret {
            source.offer("x-kde-passwordManagerHint".into());
        }
        device.set_selection(Some(&source), self.key_serial);
        self.copied = Some(text);
        say(0, "copied".into());
    }

    /// A drag of text from window `index` with a 32 by 32 magenta icon.
    fn start_drag(&mut self, index: usize, serial: u32, qh: &QueueHandle<Self>) {
        let (Some(manager), Some(device), Some(compositor), Some(shm)) =
            (&self.data_manager, &self.data_device, &self.compositor, &self.shm)
        else {
            return;
        };
        let source = manager.create_data_source(qh, ());
        source.offer("text/plain".into());
        source.set_actions(wl_data_device_manager::DndAction::Copy);
        let icon = compositor.create_surface(qh, ());
        let len = 32 * 32 * 4;
        let fd = unsafe { libc::memfd_create(c"isle-test-icon".as_ptr(), libc::MFD_CLOEXEC) };
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        file.set_len(len as u64).unwrap();
        let data: Vec<u8> = [0xff, 0x00, 0xff, 0xff].repeat(32 * 32);
        std::os::unix::fs::FileExt::write_all_at(&file, &data, 0).unwrap();
        let fd: OwnedFd = file.into();
        let pool = shm.create_pool(fd.as_fd(), len, qh, ());
        let buffer = pool.create_buffer(0, 32, 32, 128, wl_shm::Format::Argb8888, qh, ());
        pool.destroy();
        icon.attach(Some(&buffer), 0, 0);
        icon.commit();
        device.start_drag(Some(&source), &self.wins[index].surface, Some(&icon), serial);
        say(index, "drag started".into());
    }

    fn badge(&mut self, rest: &[&str]) {
        let (runtime, conn) = self.launcher.get_or_insert_with(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let conn = runtime.block_on(zbus::Connection::session()).unwrap();
            (runtime, conn)
        });
        let mut properties: std::collections::HashMap<&str, zvariant::Value> = Default::default();
        for word in rest {
            match word.split_once('=') {
                Some(("count", n)) => {
                    properties.insert("count", zvariant::Value::I64(n.parse().unwrap()));
                    properties.insert("count-visible", zvariant::Value::Bool(true));
                }
                Some(("progress", p)) => {
                    properties.insert("progress", zvariant::Value::F64(p.parse().unwrap()));
                    properties.insert("progress-visible", zvariant::Value::Bool(true));
                }
                Some(("urgent", b)) => {
                    properties.insert("urgent", zvariant::Value::Bool(b == "true"));
                }
                _ => {}
            }
        }
        let uri = format!("application://{}.desktop", self.app_id);
        runtime
            .block_on(conn.emit_signal(
                None::<()>,
                "/isle/test/launcher",
                "com.canonical.Unity.LauncherEntry",
                "Update",
                &(uri, properties),
            ))
            .unwrap();
        say(0, "badge sent".into());
    }

    /// The session bus on a runtime of the client's own, made on first use.
    fn bus(&mut self) -> &(tokio::runtime::Runtime, zbus::Connection) {
        self.launcher.get_or_insert_with(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let conn = runtime.block_on(zbus::Connection::session()).unwrap();
            (runtime, conn)
        })
    }

    /// Sends a notification from `key=value` words, printing `notified ID`; the first also
    /// starts printing the server's signals: `closed ID REASON`, `action ID KEY`,
    /// `token ID TOKEN`, `replied ID TEXT`.
    fn notify(&mut self, rest: &[&str]) {
        if !self.listening {
            self.listening = true;
            std::thread::spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async {
                    use futures_util::StreamExt;
                    let conn = zbus::Connection::session().await.unwrap();
                    let rule = zbus::MatchRule::builder()
                        .msg_type(zbus::message::Type::Signal)
                        .interface("org.freedesktop.Notifications")
                        .unwrap()
                        .build();
                    let mut signals = zbus::MessageStream::for_match_rule(rule, &conn, None)
                        .await
                        .unwrap();
                    while let Some(Ok(message)) = signals.next().await {
                        let header = message.header();
                        let member = header.member().map(|m| m.to_string()).unwrap_or_default();
                        let body = message.body();
                        let line = match member.as_str() {
                            "NotificationClosed" => body
                                .deserialize::<(u32, u32)>()
                                .map(|(id, r)| format!("closed {id} {r}")),
                            "ActionInvoked" => body
                                .deserialize::<(u32, String)>()
                                .map(|(id, k)| format!("action {id} {k}")),
                            "ActivationToken" => body
                                .deserialize::<(u32, String)>()
                                .map(|(id, t)| format!("token {id} {t}")),
                            "NotificationReplied" => body
                                .deserialize::<(u32, String)>()
                                .map(|(id, t)| format!("replied {id} {t}")),
                            _ => continue,
                        };
                        if let Ok(line) = line {
                            say(0, line);
                        }
                    }
                });
            });
            // The match is in place before the first notification goes.
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        let mut summary = "Test".to_owned();
        let mut body = String::new();
        let mut actions: Vec<String> = Vec::new();
        let mut hints: std::collections::HashMap<&str, zvariant::Value> = Default::default();
        let mut replaces = 0u32;
        let mut timeout = -1i32;
        let mut icon = String::new();
        for word in rest {
            let (key, value) = word.split_once('=').unwrap_or((word, ""));
            let value = value.replace('_', " ");
            match key {
                "summary" => summary = value,
                "body" => body = value,
                "icon" => icon = value,
                "actions" => {
                    for pair in value.split(',') {
                        let (k, l) = pair.split_once(':').unwrap_or((pair, pair));
                        actions.extend([k.to_owned(), l.to_owned()]);
                    }
                }
                "urgency" => {
                    hints.insert("urgency", zvariant::Value::U8(value.parse().unwrap()));
                }
                "transient" => {
                    hints.insert("transient", zvariant::Value::Bool(true));
                }
                "resident" => {
                    hints.insert("resident", zvariant::Value::Bool(true));
                }
                "desktop-entry" => {
                    hints.insert("desktop-entry", zvariant::Value::from(value));
                }
                "image" => {
                    hints.insert("image-path", zvariant::Value::from(value));
                }
                "replaces" => replaces = value.parse().unwrap(),
                "timeout" => timeout = value.parse().unwrap(),
                _ => {}
            }
        }
        let (runtime, conn) = self.bus();
        let reply = runtime.block_on(conn.call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &(
                "Test Client",
                replaces,
                icon,
                summary,
                body,
                actions,
                hints,
                timeout,
            ),
        ));
        match reply.and_then(|m| m.body().deserialize::<u32>()) {
            Ok(id) => say(0, format!("notified {id}")),
            Err(err) => say(0, format!("notify failed {err}")),
        }
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
            ("animate", _) => {
                self.animating = if self.animating.is_some() { None } else { Some(index) };
                self.draw(index, qh);
                return;
            }
            ("colour", _) => {
                let hex = rest.first().and_then(|h| u32::from_str_radix(h, 16).ok()).unwrap_or(0);
                self.wins[index].colour = [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 0xff];
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
            ("drag-on-press", _) => self.drag_on_press = true,
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
            ("copy", _) => self.copy(rest.join(" "), false, qh),
            ("copy-secret", _) => self.copy(rest.join(" "), true, qh),
            ("badge", _) => self.badge(&rest),
            ("notify", _) => self.notify(&rest),
            ("tray", _) => self.tray = Some(tray(&rest)),
            ("tray-attention", _) => {
                if let Some((runtime, conn)) = &self.tray {
                    runtime.block_on(async {
                        let iface = conn
                            .object_server()
                            .interface::<_, TrayItem>("/StatusNotifierItem")
                            .await
                            .unwrap();
                        *iface.get().await.status.lock().unwrap() = "NeedsAttention".into();
                        TrayItem::new_status(iface.signal_emitter(), "NeedsAttention")
                            .await
                            .unwrap();
                    });
                }
            }
            ("close-notification", _) => {
                let id: u32 = rest[0].parse().unwrap();
                let (runtime, conn) = self.bus();
                let _ = runtime.block_on(conn.call_method(
                    Some("org.freedesktop.Notifications"),
                    "/org/freedesktop/Notifications",
                    Some("org.freedesktop.Notifications"),
                    "CloseNotification",
                    &(id,),
                ));
            }
            ("cursor-surface", _) => {
                self.cursor_surface = true;
                return;
            }
            ("copy-screen", _) => {
                if let (Some(sources), Some(manager), Some(output)) =
                    (&self.output_sources, &self.copy_manager, &self.output)
                {
                    let source = sources.create_source(output, qh, ());
                    let session = manager.create_session(
                        &source,
                        ext_image_copy_capture_manager_v1::Options::empty(),
                        qh,
                        "screen",
                    );
                    self.copying = Some(Copying::new(session));
                }
            }
            ("copy-cursor", _) => {
                if let (Some(sources), Some(manager), Some(output), Some(pointer)) =
                    (&self.output_sources, &self.copy_manager, &self.output, &self.pointer)
                {
                    let source = sources.create_source(output, qh, ());
                    let cursor = manager.create_pointer_cursor_session(&source, pointer, qh, ());
                    let session = cursor.get_capture_session(qh, "cursor");
                    self.cursor_copying = Some(Copying::new(session));
                }
            }
            ("export", _) => {
                if let Some(exporter) = &self.exporter {
                    exporter.export_toplevel(&self.wins[index].surface, qh, index);
                }
            }
            ("use-token", _) => {
                if let Some(activation) = &self.activation {
                    activation.activate(rest[0].to_owned(), &self.wins[index].surface);
                    say(index, "used token".into());
                }
            }
            ("inhibit", _) => {
                if let (Some(manager), Some(seat)) = (&self.inhibits, &self.seat) {
                    manager.inhibit_shortcuts(&self.wins[index].surface, seat, qh, ());
                }
            }
            ("keep-open", _) => self.keep_open = true,
            ("idle-inhibit", _) => {
                if let Some(manager) = &self.idle_inhibits {
                    self.idle_inhibitor =
                        Some(manager.create_inhibitor(&self.wins[index].surface, qh, ()));
                }
            }
            ("idle-uninhibit", _) => {
                if let Some(inhibitor) = self.idle_inhibitor.take() {
                    inhibitor.destroy();
                }
            }
            ("idle-notify", _) => {
                let ms = rest.first().and_then(|ms| ms.parse::<u32>().ok());
                if let (Some(notifier), Some(seat), Some(ms)) =
                    (&self.idle_notifier, &self.seat, ms)
                {
                    notifier.get_idle_notification(ms, seat, qh, ());
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

impl Dispatch<wl_callback::WlCallback, usize> for Client {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        index: &usize,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event
            && state.animating == Some(*index)
            && let Some(win) = state.wins.get_mut(*index)
        {
            win.colour = [win.colour[1], win.colour[2], win.colour[0].wrapping_add(7), 0xff];
            state.draw(*index, qh);
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
                "wl_data_device_manager" => {
                    state.data_manager = Some(registry.bind(name, 3, qh, ()))
                }
                "zwlr_layer_shell_v1" => state.layer_shell = Some(registry.bind(name, 4, qh, ())),
                "zxdg_decoration_manager_v1" => {
                    state.decorations = Some(registry.bind(name, 1, qh, ()))
                }
                "xdg_wm_dialog_v1" => state.dialogs = Some(registry.bind(name, 1, qh, ())),
                "zwp_keyboard_shortcuts_inhibit_manager_v1" => {
                    state.inhibits = Some(registry.bind(name, 1, qh, ()))
                }
                "xdg_activation_v1" => state.activation = Some(registry.bind(name, 1, qh, ())),
                "zxdg_exporter_v2" => state.exporter = Some(registry.bind(name, 1, qh, ())),
                "wl_output" if state.output.is_none() => {
                    state.output = Some(registry.bind(name, 1, qh, ()))
                }
                "ext_output_image_capture_source_manager_v1" => {
                    state.output_sources = Some(registry.bind(name, 1, qh, ()))
                }
                "ext_image_copy_capture_manager_v1" => {
                    state.copy_manager = Some(registry.bind(name, 1, qh, ()))
                }
                "zwp_idle_inhibit_manager_v1" => {
                    state.idle_inhibits = Some(registry.bind(name, 1, qh, ()))
                }
                "ext_idle_notifier_v1" => state.idle_notifier = Some(registry.bind(name, 1, qh, ())),
                "org_kde_kwin_appmenu_manager" => state.appmenus = Some(registry.bind(name, 2, qh, ())),
                "gtk_shell1" => state.gtk_shell = Some(registry.bind(name, 5, qh, ())),
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
                if state.keep_open {
                    // As an app with unsaved work: it stays and asks.
                } else if *index == 0 && !state.linger {
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
        state: &mut Self,
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
                state.pointer = Some(seat.get_pointer(qh, ()));
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
        conn: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { surface, serial, .. } => {
                state.key_serial = serial;
                say(state.index_of_surface(&surface).unwrap_or(0), "keyboard-enter".into())
            }
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
                serial,
                state: WEnum::Value(s),
                ..
            } => {
                state.key_serial = serial;
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
                    // Ctrl+V reads the clipboard's text and prints it.
                    let ctrl = xkb_state.mod_name_is_active("Control", isle_keys::xkb::STATE_MODS_EFFECTIVE);
                    if ctrl && pressed == "pressed" && sym.raw() == isle_keys::xkb::keysyms::KEY_v {
                        state.paste(conn);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for Client {
    fn event(
        state: &mut Self,
        pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                if state.cursor_surface
                    && let (Some(compositor), Some(shm)) = (&state.compositor, &state.shm)
                {
                    let cursor = compositor.create_surface(qh, ());
                    let (w, h) = (16, 16);
                    let len = (w * h * 4) as usize;
                    let fd = unsafe { libc::memfd_create(c"isle-test-cursor".as_ptr(), libc::MFD_CLOEXEC) };
                    let file = unsafe { std::fs::File::from_raw_fd(fd) };
                    let pixel = [0xffu8, 0x00, 0xff, 0xff];
                    std::os::unix::fs::FileExt::write_all_at(&file, &pixel.repeat(w as usize * h as usize), 0).unwrap();
                    let fd: OwnedFd = file.into();
                    let pool = shm.create_pool(fd.as_fd(), len as i32, qh, ());
                    let buffer = pool.create_buffer(0, w, h, w * 4, wl_shm::Format::Argb8888, qh, ());
                    pool.destroy();
                    cursor.attach(Some(&buffer), 0, 0);
                    cursor.damage_buffer(0, 0, w, h);
                    cursor.commit();
                    pointer.set_cursor(serial, Some(&cursor), 2, 3);
                }
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
                if pressed && std::mem::take(&mut state.drag_on_press) {
                    state.start_drag(index, serial, qh);
                }
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

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, ()> for Client {
    fn event(
        _: &mut Self,
        _: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => say(0, "idled".into()),
            ext_idle_notification_v1::Event::Resumed => say(0, "resumed".into()),
            _ => {}
        }
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for Client {
    fn event(
        state: &mut Self,
        _: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        _: &(),
        conn: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_device::Event::Enter { serial, id: Some(offer), .. } => {
                offer.accept(serial, Some("text/plain".into()));
                offer.set_actions(
                    wl_data_device_manager::DndAction::Copy,
                    wl_data_device_manager::DndAction::Copy,
                );
                state.dnd_offer = Some(offer);
                say(0, "dnd enter".into());
            }
            wl_data_device::Event::Drop => {
                say(0, "dnd drop".into());
                // What was dropped is read as a list of links and printed.
                if let Some(offer) = state.dnd_offer.take() {
                    let mut fds = [0; 2];
                    // Safety: room for both ends.
                    unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) };
                    // Safety: both ends are new and owned here.
                    let (read, write) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
                    offer.receive("text/uri-list".into(), write.as_fd());
                    drop(write);
                    offer.finish();
                    let _ = conn.flush();
                    std::thread::spawn(move || {
                        let mut text = String::new();
                        let _ = std::io::Read::read_to_string(&mut std::fs::File::from(read), &mut text);
                        say(0, format!("dropped {}", text.trim()));
                    });
                }
            }
            wl_data_device::Event::Selection { id } => state.selection = id,
            _ => {}
        }
    }

    wayland_client::event_created_child!(Client, wl_data_device::WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (wl_data_offer::WlDataOffer, ()),
    ]);
}

impl Dispatch<wl_data_source::WlDataSource, ()> for Client {
    fn event(
        state: &mut Self,
        _: &wl_data_source::WlDataSource,
        event: wl_data_source::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_source::Event::Send { fd, .. } => {
                let text = state.copied.clone().unwrap_or_else(|| "dragged".into());
                let _ = std::fs::File::from(fd).write_all(text.as_bytes());
            }
            wl_data_source::Event::DndFinished => say(0, "dnd finished".into()),
            wl_data_source::Event::Cancelled => say(0, "dnd cancelled".into()),
            _ => {}
        }
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for Client {
    fn event(
        _: &mut Self,
        buffer: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // A buffer drawn over is let go once the compositor is done with it.
        if let wl_buffer::Event::Release = event {
            buffer.destroy();
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

/// An ext-image-copy-capture session taking frame after frame, named by what it copies:
/// `screen` or `cursor`.
struct Copying {
    session: ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
    size: (i32, i32),
    buffer: Option<(wl_buffer::WlBuffer, std::fs::File)>,
    frames: u32,
}

impl Copying {
    fn new(session: ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1) -> Self {
        Self { session, size: (0, 0), buffer: None, frames: 0 }
    }

    fn capture(&self, kind: &'static str, qh: &QueueHandle<Client>) {
        let Some((buffer, _)) = &self.buffer else {
            return;
        };
        let frame = self.session.create_frame(qh, kind);
        frame.attach_buffer(buffer);
        frame.damage_buffer(0, 0, self.size.0, self.size.1);
        frame.capture();
    }

    /// The copied pixel at `(x, y)` as RRGGBBAA.
    fn pixel(&self, x: i32, y: i32) -> String {
        let Some((_, file)) = &self.buffer else {
            return String::new();
        };
        let mut px = [0u8; 4];
        let at = ((y * self.size.0 + x) * 4) as u64;
        let _ = std::os::unix::fs::FileExt::read_exact_at(file, &mut px, at);
        format!("{:02x}{:02x}{:02x}{:02x}", px[2], px[1], px[0], px[3])
    }
}

impl Client {
    fn copying(&mut self, kind: &str) -> Option<&mut Copying> {
        match kind {
            "cursor" => self.cursor_copying.as_mut(),
            _ => self.copying.as_mut(),
        }
    }
}

impl Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, &'static str> for Client {
    fn event(
        state: &mut Self,
        _: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        kind: &&'static str,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use ext_image_copy_capture_session_v1::Event;
        let shm = state.shm.clone();
        let kind = *kind;
        let Some(copying) = state.copying(kind) else {
            return;
        };
        match event {
            Event::BufferSize { width, height } => copying.size = (width as i32, height as i32),
            Event::Done if copying.buffer.is_none() => {
                let (w, h) = copying.size;
                let len = (w * h * 4) as usize;
                let fd = unsafe { libc::memfd_create(c"isle-test-copy".as_ptr(), libc::MFD_CLOEXEC) };
                assert!(fd >= 0, "memfd_create");
                let file = unsafe { std::fs::File::from_raw_fd(fd) };
                file.set_len(len as u64).unwrap();
                let pool = shm.unwrap().create_pool(file.as_fd(), len as i32, qh, ());
                let buffer = pool.create_buffer(0, w, h, w * 4, wl_shm::Format::Argb8888, qh, ());
                pool.destroy();
                copying.buffer = Some((buffer, file));
                copying.capture(kind, qh);
            }
            Event::Stopped if kind == "cursor" => say(0, "cursor copy stopped".into()),
            Event::Stopped => say(0, "copy stopped".into()),
            _ => {}
        }
    }
}

impl Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, &'static str> for Client {
    fn event(
        state: &mut Self,
        frame: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        kind: &&'static str,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use ext_image_copy_capture_frame_v1::Event;
        let kind = *kind;
        let Some(copying) = state.copying(kind) else {
            return;
        };
        match event {
            Event::Ready => {
                frame.destroy();
                copying.frames += 1;
                if kind == "cursor" {
                    say(0, format!("cursor copied {} {}", copying.frames, copying.pixel(5, 5)));
                } else {
                    say(0, format!("copied {}", copying.frames));
                }
                copying.capture(kind, qh);
            }
            Event::Failed { .. } => frame.destroy(),
            _ => {}
        }
    }
}

impl Dispatch<ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1, ()> for Client {
    fn event(
        _: &mut Self,
        _: &ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
        event: ext_image_copy_capture_cursor_session_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use ext_image_copy_capture_cursor_session_v1::Event;
        match event {
            Event::Enter => say(0, "cursor-enter".into()),
            Event::Leave => say(0, "cursor-leave".into()),
            Event::Position { x, y } => say(0, format!("cursor-position {x} {y}")),
            Event::Hotspot { x, y } => say(0, format!("cursor-hotspot {x} {y}")),
            _ => {}
        }
    }
}

impl Dispatch<zxdg_exported_v2::ZxdgExportedV2, usize> for Client {
    fn event(
        _: &mut Self,
        _: &zxdg_exported_v2::ZxdgExportedV2,
        event: zxdg_exported_v2::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_exported_v2::Event::Handle { handle } = event {
            say(*index, format!("exported {handle}"));
        }
    }
}

ignore!(
    zxdg_exporter_v2::ZxdgExporterV2,
    wl_output::WlOutput,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    wl_data_device_manager::WlDataDeviceManager,
    wl_data_offer::WlDataOffer,
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
    xdg_wm_dialog_v1::XdgWmDialogV1,
    xdg_dialog_v1::XdgDialogV1,
    zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
    zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
    ext_idle_notifier_v1::ExtIdleNotifierV1,
    wl_compositor::WlCompositor,
    wl_shm::WlShm,
    wl_shm_pool::WlShmPool,
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
        idle_inhibits: None,
        keep_open: false,
        idle_inhibitor: None,
        idle_notifier: None,
        activation: None,
        exporter: None,
        output: None,
        output_sources: None,
        copy_manager: None,
        copying: None,
        cursor_copying: None,
        pointer: None,
        appmenus: None,
        gtk_shell: None,
        wins: Vec::new(),
        animating: None,
        ignore_pings: false,
        ignored_pings: Vec::new(),
        linger: false,
        on_press: None,
        pointer_on: None,
        cursor_surface: false,
        quit: false,
        data_manager: None,
        data_device: None,
        drag_on_press: false,
        selection: None,
        dnd_offer: None,
        copied: None,
        key_serial: 0,
        launcher: None,
        tray: None,
        listening: false,
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
        if let (Some(manager), Some(seat)) = (&client.data_manager, &client.seat) {
            client.data_device = Some(manager.get_data_device(seat, &qh, ()));
        }
        // Menus exported as an app would, announced the way the flags say.
        let menus = menus(&args, &client, &qh);
        std::mem::forget(menus);
        // Started by a launcher, the window asks for focus with the token it was given.
        if let (Ok(token), Some(activation)) =
            (std::env::var("XDG_ACTIVATION_TOKEN"), client.activation.as_ref())
        {
            activation.activate(token.clone(), &client.wins[0].surface);
            say(0, format!("activated with {token}"));
        }
    }
    client.started = true;

    let mut stdin_open = true;
    // Raw reads, so every line that arrived together is run, not one per wake.
    let mut pending: Vec<u8> = Vec::new();
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
            let mut buf = [0u8; 4096];
            let n = unsafe { libc::read(0, buf.as_mut_ptr().cast(), buf.len()) };
            if n <= 0 {
                stdin_open = false;
            } else {
                pending.extend_from_slice(&buf[..n as usize]);
            }
            while let Some(at) = pending.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = pending.drain(..=at).collect();
                client.command(String::from_utf8_lossy(&line).trim(), &qh);
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

/// Exports menus for `--appmenu` (DBusMenu through the KDE protocol), `--gtk-menu`
/// (GMenuModel through gtk_shell1) and `--register ID` (DBusMenu through the registrar by
/// a window id nothing matches; `--gtk-menu-late` leaves the menubar empty for a second
/// and a half, then fills it and says so; with `--register-late` a second and a half after start, as
/// an app that registers after its window maps); each item chosen prints `menu activated
/// LABEL`.
fn menus(args: &[String], client: &Client, qh: &QueueHandle<Client>) -> Option<(tokio::runtime::Runtime, zbus::Connection)> {
    let wanted = |f: &str| args.iter().any(|a| a == f);
    let register = args
        .iter()
        .position(|a| a == "--register")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<u32>().ok());
    if !wanted("--appmenu") && !wanted("--gtk-menu") && register.is_none() {
        return None;
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let on_activate: isle_menus::exporter::OnActivate = std::sync::Arc::new(|label| say(0, format!("menu activated {label}")));
    let conn = runtime.block_on(async {
        let conn = zbus::Connection::session().await.unwrap();
        let server = conn.object_server();
        server
            .at("/MenuBar/1", isle_menus::exporter::DBusMenuExport::new(on_activate.clone()))
            .await
            .unwrap();
        let gmenu = if wanted("--gtk-menu-late") {
            isle_menus::exporter::GMenuExport::empty()
        } else {
            isle_menus::exporter::GMenuExport::new()
        };
        let filled = gmenu.filled.clone();
        server.at("/org/test/menus/menubar", gmenu).await.unwrap();
        if wanted("--gtk-menu-late") {
            let conn = conn.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                filled.store(true, std::sync::atomic::Ordering::Release);
                let emitter = zbus::object_server::SignalEmitter::new(&conn, "/org/test/menus/menubar").unwrap();
                isle_menus::exporter::GMenuExport::changed(&emitter, vec![(0, 0, 0, 0, Vec::new())])
                    .await
                    .unwrap();
            });
        }
        server
            .at("/org/test", isle_menus::exporter::ActionsExport::app(on_activate.clone()))
            .await
            .unwrap();
        server
            .at("/org/test/window/1", isle_menus::exporter::ActionsExport::window(on_activate))
            .await
            .unwrap();
        if let Some(id) = register {
            let conn = conn.clone();
            let late = wanted("--register-late");
            let registered = async move {
                if late {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                }
                conn.call_method(
                    Some("com.canonical.AppMenu.Registrar"),
                    "/com/canonical/AppMenu/Registrar",
                    Some("com.canonical.AppMenu.Registrar"),
                    "RegisterWindow",
                    &(id, zvariant::ObjectPath::try_from("/MenuBar/1").unwrap()),
                )
                .await
                .unwrap();
            };
            if late {
                tokio::spawn(registered);
            } else {
                registered.await;
            }
        }
        conn
    });
    let unique = conn.unique_name().unwrap().to_string();
    let surface = &client.wins[0].surface;
    if wanted("--appmenu") {
        let appmenu = client.appmenus.as_ref().expect("org_kde_kwin_appmenu_manager").create(surface, qh, ());
        appmenu.set_address(unique.clone(), "/MenuBar/1".into());
    }
    if wanted("--gtk-menu") {
        let gtk = client.gtk_shell.as_ref().expect("gtk_shell1").get_gtk_surface(surface, qh, ());
        gtk.set_dbus_properties(
            Some(client.app_id.clone()),
            None,
            Some("/org/test/menus/menubar".into()),
            Some("/org/test/window/1".into()),
            Some("/org/test".into()),
            Some(unique),
        );
    }
    say(0, "menus exported".into());
    Some((runtime, conn))
}

impl Dispatch<org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager, ()> for Client {
    fn event(
        _: &mut Self,
        _: &org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager,
        _: org_kde_kwin_appmenu_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<org_kde_kwin_appmenu::OrgKdeKwinAppmenu, ()> for Client {
    fn event(
        _: &mut Self,
        _: &org_kde_kwin_appmenu::OrgKdeKwinAppmenu,
        _: org_kde_kwin_appmenu::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<gtk_shell1::GtkShell1, ()> for Client {
    fn event(
        _: &mut Self,
        _: &gtk_shell1::GtkShell1,
        event: gtk_shell1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let gtk_shell1::Event::Capabilities { capabilities } = event;
        say(0, format!("gtk capabilities {capabilities}"));
    }
}

impl Dispatch<gtk_surface1::GtkSurface1, ()> for Client {
    fn event(
        _: &mut Self,
        _: &gtk_surface1::GtkSurface1,
        _: gtk_surface1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
