//! Protocols winit does not bind, bound on a queue of the app's own against winit's display
//! and the window's surface (`settings-2026.md` §2): raising the window with an activation
//! token, naming its menu's address, the compositor's lights over the toolbar
//! (`isle_titlebar_v1`) and the toolbar's clicks handed to the compositor (`gtk_shell1`).

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use tracing::warn;
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_registry::WlRegistry, wl_seat::WlSeat, wl_surface::WlSurface},
};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;
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
use isle_titlebar::{isle_titlebar_manager_v1::IsleTitlebarManagerV1, isle_titlebar_v1::IsleTitlebarV1};

pub use gtk_surface1::Gesture;

struct Globals;

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

pub struct Extras {
    connection: Connection,
    /// Kept for as long as the objects bound on it.
    _queue: EventQueue<Globals>,
    surface: WlSurface,
    activation: Option<XdgActivationV1>,
    appmenu: Option<OrgKdeKwinAppmenu>,
    gestures: Option<(GtkSurface1, WlSeat)>,
    _titlebar: Option<IsleTitlebarV1>,
}

impl Extras {
    /// Binds what the compositor offers against `window`'s surface, asking for the lights
    /// at `inset`; `None` off Wayland.
    pub fn connect(window: &freya::winit::window::Window, inset: (f64, f64)) -> Option<Self> {
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
        let titlebar = globals
            .bind::<IsleTitlebarManagerV1, _, _>(&qh, 1..=1, ())
            .ok()
            .map(|manager| {
                let titlebar = manager.get_titlebar(&surface, &qh, ());
                titlebar.set_overlay(inset.0.round() as i32, inset.1.round() as i32);
                titlebar
            });
        let _ = connection.flush();
        Some(Self {
            connection,
            _queue: queue,
            surface,
            activation,
            appmenu,
            gestures,
            _titlebar: titlebar,
        })
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
