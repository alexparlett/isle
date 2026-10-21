//! Protocols winit does not bind, bound on a queue of the app's own against winit's display
//! and the window's surface (`settings-2026.md` §2): raising the window with an activation
//! token, naming its menu's address, the compositor's lights over the toolbar
//! (`isle_titlebar_v1`), the toolbar's clicks handed to the compositor (`gtk_shell1`) and the
//! blur behind the sidebar (`ext_background_effect_v1`).

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use tracing::warn;
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    globals::{GlobalList, GlobalListContents, registry_queue_init},
    protocol::{
        wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry::WlRegistry, wl_seat::WlSeat,
        wl_surface::WlSurface,
    },
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;
use wayland_protocols::xdg::{
    dialog::v1::client::{xdg_dialog_v1::XdgDialogV1, xdg_wm_dialog_v1::XdgWmDialogV1},
    foreign::zv2::client::{
        zxdg_exported_v2::{self, ZxdgExportedV2},
        zxdg_exporter_v2::ZxdgExporterV2,
        zxdg_imported_v2::ZxdgImportedV2,
        zxdg_importer_v2::ZxdgImporterV2,
    },
    shell::client::xdg_toplevel::XdgToplevel,
};
use wayland_protocols_plasma::appmenu::client::{
    org_kde_kwin_appmenu::OrgKdeKwinAppmenu, org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager,
};

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

#[allow(missing_docs, clippy::all, non_upper_case_globals, non_camel_case_types, unused_imports)]
mod isle_titlebar {
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../../data/protocols/isle-titlebar-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("../../data/protocols/isle-titlebar-v1.xml");
}

use gtk_shell::{gtk_shell1::GtkShell1, gtk_surface1::{self, GtkSurface1}};
use isle_titlebar::{
    isle_float_v1::{IsleFloatV1, Keyboard},
    isle_titlebar_manager_v1::IsleTitlebarManagerV1,
    isle_titlebar_v1::IsleTitlebarV1,
};

pub use gtk_surface1::Gesture;

/// The queue's state: what an export's handle event brought.
#[derive(Default)]
struct Globals {
    handle: Option<String>,
}

impl Dispatch<ZxdgExportedV2, ()> for Globals {
    fn event(state: &mut Self, _: &ZxdgExportedV2, event: zxdg_exported_v2::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let zxdg_exported_v2::Event::Handle { handle } = event {
            state.handle = Some(handle);
        }
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for Globals {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

wayland_client::delegate_noop!(Globals: ignore XdgActivationV1);
wayland_client::delegate_noop!(Globals: ignore OrgKdeKwinAppmenuManager);
wayland_client::delegate_noop!(Globals: ignore OrgKdeKwinAppmenu);
wayland_client::delegate_noop!(Globals: ignore WlSeat);
wayland_client::delegate_noop!(Globals: ignore GtkShell1);
wayland_client::delegate_noop!(Globals: ignore GtkSurface1);
wayland_client::delegate_noop!(Globals: ignore IsleTitlebarManagerV1);
wayland_client::delegate_noop!(Globals: ignore IsleTitlebarV1);
wayland_client::delegate_noop!(Globals: ignore IsleFloatV1);
wayland_client::delegate_noop!(Globals: ignore ExtBackgroundEffectManagerV1);
wayland_client::delegate_noop!(Globals: ignore ExtBackgroundEffectSurfaceV1);
wayland_client::delegate_noop!(Globals: ignore WlCompositor);
wayland_client::delegate_noop!(Globals: ignore WlRegion);
wayland_client::delegate_noop!(Globals: ignore ZxdgImporterV2);
wayland_client::delegate_noop!(Globals: ignore ZxdgExporterV2);
wayland_client::delegate_noop!(Globals: ignore ZxdgImportedV2);
wayland_client::delegate_noop!(Globals: ignore XdgWmDialogV1);
wayland_client::delegate_noop!(Globals: ignore XdgDialogV1);
wayland_client::delegate_noop!(Globals: ignore XdgToplevel);

pub struct Extras {
    /// winit's `wl_display`, for the data device's own connection.
    pub display: *mut std::ffi::c_void,
    connection: Connection,
    /// Kept for as long as the objects bound on it.
    queue: std::cell::RefCell<EventQueue<Globals>>,
    globals: GlobalList,
    /// The window exported for other apps' dialogs, and its handle.
    exported: std::cell::RefCell<Option<(ZxdgExportedV2, String)>>,
    surface: WlSurface,
    activation: Option<XdgActivationV1>,
    appmenu: Option<OrgKdeKwinAppmenu>,
    gestures: Option<(GtkSurface1, WlSeat)>,
    _titlebar: Option<IsleTitlebarV1>,
    _float: Option<IsleFloatV1>,
    /// Another app's window this one is the modal child of, kept while it is.
    _parent: Option<(ZxdgImportedV2, Option<XdgDialogV1>)>,
    backdrop: Option<(ExtBackgroundEffectSurfaceV1, WlCompositor, QueueHandle<Globals>)>,
}

/// winit's `xdg_toplevel` for the window.
fn toplevel(window: &freya::winit::window::Window, connection: &Connection) -> Option<XdgToplevel> {
    use freya::winit::platform::wayland::WindowExtWayland;
    let ptr = window.xdg_toplevel()?;
    let id = unsafe { ObjectId::from_ptr(XdgToplevel::interface(), ptr.as_ptr().cast()) }.ok()?;
    XdgToplevel::from_id(connection, id).ok()
}

impl Extras {
    /// Binds what the compositor offers against `window`'s surface, asking for the lights
    /// at their inset when given and the blur behind `backdrop`; `None` off Wayland.
    pub fn connect(
        window: &freya::winit::window::Window,
        lights: Option<(f64, f64)>,
        backdrop: Option<(f64, f64, f64, f64)>,
        parent: Option<&str>,
        float: Option<&crate::window::Float>,
    ) -> Option<Self> {
        let display = match window.display_handle().ok()?.as_raw() {
            RawDisplayHandle::Wayland(handle) => handle.display.as_ptr(),
            _ => return None,
        };
        let surface = match window.window_handle().ok()?.as_raw() {
            RawWindowHandle::Wayland(handle) => handle.surface.as_ptr(),
            _ => return None,
        };
        // The display and surface are winit's and outlive the window this is kept beside.
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let connection = Connection::from_backend(backend);
        let (globals, queue) = match registry_queue_init::<Globals>(&connection) {
            Ok(found) => found,
            Err(err) => {
                warn!("the compositor's globals could not be listed: {err}");
                return None;
            }
        };
        let qh = queue.handle();
        let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.cast()) }.ok()?;
        let surface = WlSurface::from_id(&connection, id).ok()?;
        let activation = globals.bind::<XdgActivationV1, _, _>(&qh, 1..=1, ()).ok();
        let appmenu = globals
            .bind::<OrgKdeKwinAppmenuManager, _, _>(&qh, 1..=2, ())
            .ok()
            .map(|manager| manager.create(&surface, &qh, ()));
        let gestures = globals
            .bind::<GtkShell1, _, _>(&qh, 5..=5, ())
            .ok()
            .zip(globals.bind::<WlSeat, _, _>(&qh, 1..=9, ()).ok())
            .map(|(shell, seat)| (shell.get_gtk_surface(&surface, &qh, ()), seat));
        let manager = globals.bind::<IsleTitlebarManagerV1, _, _>(&qh, 1..=2, ()).ok();
        let titlebar = lights.zip(manager.as_ref()).map(|(inset, manager)| {
            let titlebar = manager.get_titlebar(&surface, &qh, ());
            titlebar.set_overlay(inset.0.round() as i32, inset.1.round() as i32);
            titlebar
        });
        // Before the first commit, so the compositor never gives the window the keyboard.
        let float = float.zip(manager.as_ref().filter(|m| m.version() >= 2)).map(|(want, manager)| {
            let float = manager.get_float(&surface, &qh, ());
            float.set_float_over(want.over.clone());
            float.set_keyboard(if want.on_request { Keyboard::OnRequest } else { Keyboard::None });
            if let Some((x, y, w, h)) = want.from {
                float.open_from(x, y, w, h);
            }
            float
        });
        let effect = globals
            .bind::<ExtBackgroundEffectManagerV1, _, _>(&qh, 1..=1, ())
            .ok()
            .zip(globals.bind::<WlCompositor, _, _>(&qh, 1..=6, ()).ok())
            .map(|(manager, compositor)| (manager.get_background_effect(&surface, &qh, ()), compositor, qh.clone()));
        // Before the first buffer, so the compositor places the window as its parent's sheet.
        let parent = parent.and_then(|handle| {
            let importer = globals.bind::<ZxdgImporterV2, _, _>(&qh, 1..=1, ()).ok()?;
            let imported = importer.import_toplevel(handle.to_owned(), &qh, ());
            imported.set_parent_of(&surface);
            let dialog = toplevel(window, &connection).and_then(|toplevel| {
                let wm = globals.bind::<XdgWmDialogV1, _, _>(&qh, 1..=1, ()).ok()?;
                let dialog = wm.get_xdg_dialog(&toplevel, &qh, ());
                dialog.set_modal();
                Some(dialog)
            });
            Some((imported, dialog))
        });
        let _ = connection.flush();
        let extras = Self {
            display: display.cast(),
            connection,
            queue: std::cell::RefCell::new(queue),
            globals,
            exported: std::cell::RefCell::default(),
            surface,
            activation,
            appmenu,
            gestures,
            _titlebar: titlebar,
            _float: float,
            _parent: parent,
            backdrop: effect,
        };
        extras.set_backdrop(backdrop);
        Some(extras)
    }

    /// The window's xdg-foreign handle, for another app's dialog on it, as a portal's
    /// `parent_window` names it; exported once and kept.
    pub fn export(&self) -> Option<String> {
        if let Some((_, handle)) = self.exported.borrow().as_ref() {
            return Some(handle.clone());
        }
        let mut queue = self.queue.borrow_mut();
        let qh = queue.handle();
        let exporter = self.globals.bind::<ZxdgExporterV2, _, _>(&qh, 1..=1, ()).ok()?;
        let exported = exporter.export_toplevel(&self.surface, &qh, ());
        let mut state = Globals::default();
        queue.roundtrip(&mut state).ok()?;
        let handle = state.handle?;
        *self.exported.borrow_mut() = Some((exported, handle.clone()));
        Some(handle)
    }

    /// The window's surface, as the data device's events name it.
    pub fn surface_id(&self) -> u32 {
        self.surface.id().protocol_id()
    }

    /// Whether the compositor blurs behind the window.
    pub fn blurs(&self) -> bool {
        self.backdrop.is_some()
    }

    /// Asks for the blur behind `rect`, surface-local, from the next commit; none removes it.
    /// The rest of the window is declared opaque, so the compositor still skips what it hides.
    pub fn set_backdrop(&self, rect: Option<(f64, f64, f64, f64)>) {
        if let (Some((x, y, w, h)), Some((_, compositor, qh))) = (rect, &self.backdrop) {
            // Larger than any window; the compositor cuts a region to its surface.
            const ALL: i32 = 1 << 15;
            let opaque = compositor.create_region(qh, ());
            opaque.add(0, 0, ALL, ALL);
            opaque.subtract(x.round() as i32, y.round() as i32, w.round() as i32, h.round() as i32);
            self.surface.set_opaque_region(Some(&opaque));
            opaque.destroy();
        }
        let Some((effect, compositor, qh)) = &self.backdrop else {
            return;
        };
        match rect {
            Some((x, y, w, h)) => {
                let region = compositor.create_region(qh, ());
                region.add(x.round() as i32, y.round() as i32, w.round() as i32, h.round() as i32);
                effect.set_blur_region(Some(&region));
                region.destroy();
            }
            None => effect.set_blur_region(None),
        }
        let _ = self.connection.flush();
    }

    /// Raises the window with a token another launch handed over.
    pub fn activate(&self, token: &str) {
        if let Some(activation) = &self.activation {
            activation.activate(token.to_owned(), &self.surface);
            let _ = self.connection.flush();
        }
    }

    /// Hands a click on the toolbar to the compositor, which does what the user set for it.
    pub fn gesture(&self, gesture: Gesture) {
        if let Some((surface, seat)) = &self.gestures {
            surface.titlebar_gesture(0, seat, gesture);
            let _ = self.connection.flush();
        }
    }

    /// Tells the compositor where the window's menu is exported (`apps.md` Conventions 8).
    pub fn set_menu(&self, service: &str, path: &str) {
        if let Some(appmenu) = &self.appmenu {
            appmenu.set_address(service.to_owned(), path.to_owned());
            let _ = self.connection.flush();
        }
    }
}
