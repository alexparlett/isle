//! The Wayland halves of the global menu: `org_kde_kwin_appmenu` (Qt 6.9+, Chromium 137+)
//! and GTK's `gtk_shell1` with `set_dbus_properties`, each storing a toplevel's menu
//! address on its record; and `gtk_shell1`'s title-bar gestures (choices 16 and 17).

use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
    protocol::wl_surface::WlSurface,
};
use wayland_protocols_plasma::appmenu::server::{
    org_kde_kwin_appmenu::{self, OrgKdeKwinAppmenu},
    org_kde_kwin_appmenu_manager::{self, OrgKdeKwinAppmenuManager},
};

use crate::state::State;

#[allow(
    missing_docs,
    clippy::all,
    non_upper_case_globals,
    non_camel_case_types,
    unused_imports
)]
pub mod gtk_shell {
    use smithay::reexports::wayland_server;
    use smithay::reexports::wayland_server::protocol::*;

    pub mod __interfaces {
        use smithay::reexports::wayland_server::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../../data/protocols/gtk-shell.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_server_code!("../../data/protocols/gtk-shell.xml");
}

use gtk_shell::{
    gtk_shell1::{self, GtkShell1},
    gtk_surface1::{self, GtkSurface1},
};

/// The version of `gtk_shell1` served: title-bar gestures, nothing later.
pub const GTK_SHELL_VERSION: u32 = 5;

pub fn create_globals(dh: &DisplayHandle) {
    dh.create_global::<State, OrgKdeKwinAppmenuManager, ()>(2, ());
    dh.create_global::<State, GtkShell1, ()>(GTK_SHELL_VERSION, ());
}

impl GlobalDispatch<OrgKdeKwinAppmenuManager, ()> for State {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<OrgKdeKwinAppmenuManager>,
        _data: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        init.init(resource, ());
    }
}

impl Dispatch<OrgKdeKwinAppmenuManager, ()> for State {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &OrgKdeKwinAppmenuManager,
        request: org_kde_kwin_appmenu_manager::Request,
        _data: &(),
        _dh: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        if let org_kde_kwin_appmenu_manager::Request::Create { id, surface } = request {
            init.init(id, surface);
        }
    }
}

impl Dispatch<OrgKdeKwinAppmenu, WlSurface> for State {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &OrgKdeKwinAppmenu,
        request: org_kde_kwin_appmenu::Request,
        surface: &WlSurface,
        _dh: &DisplayHandle,
        _init: &mut DataInit<'_, Self>,
    ) {
        if let org_kde_kwin_appmenu::Request::SetAddress {
            service_name,
            object_path,
        } = request
        {
            state.set_menu_address(
                surface,
                isle_menus::Source::DBusMenu {
                    service: service_name,
                    path: object_path,
                },
            );
        }
    }
}

impl GlobalDispatch<GtkShell1, ()> for State {
    fn bind(
        state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<GtkShell1>,
        _data: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        let shell = init.init(resource, ());
        // GTK hides its own menubar on this capability; only offer it while the reader works.
        if state.isle.settings.debug.gtk_global_menu {
            shell.capabilities(gtk_shell1::Capability::GlobalMenuBar as u32);
        }
    }
}

impl Dispatch<GtkShell1, ()> for State {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &GtkShell1,
        request: gtk_shell1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        if let gtk_shell1::Request::GetGtkSurface {
            gtk_surface,
            surface,
        } = request
        {
            init.init(gtk_surface, surface);
        }
    }
}

impl Dispatch<GtkSurface1, WlSurface> for State {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &GtkSurface1,
        request: gtk_surface1::Request,
        surface: &WlSurface,
        _dh: &DisplayHandle,
        _init: &mut DataInit<'_, Self>,
    ) {
        match request {
            gtk_surface1::Request::SetDbusProperties {
                menubar_path,
                window_object_path,
                application_object_path,
                unique_bus_name,
                ..
            } => {
                if let (Some(service), Some(menubar)) = (unique_bus_name, menubar_path) {
                    state.set_menu_address(
                        surface,
                        isle_menus::Source::GMenu {
                            service,
                            menubar,
                            application: application_object_path,
                            window: window_object_path,
                            unity: None,
                        },
                    );
                }
            }
            gtk_surface1::Request::TitlebarGesture { gesture, .. } => {
                let Some(id) = state.isle.wm.for_surface(surface).map(|r| r.id) else {
                    return;
                };
                match gesture.into_result() {
                    Ok(gtk_surface1::Gesture::DoubleClick) => {
                        state.bar_action(id, isle_shell::WindowAction::TitlebarAction)
                    }
                    Ok(gtk_surface1::Gesture::RightClick) => {
                        let at = state
                            .isle
                            .seat
                            .get_pointer()
                            .map(|p| p.current_location().to_i32_round());
                        state.open_window_menu(id, at, false);
                    }
                    Ok(gtk_surface1::Gesture::MiddleClick) => {}
                    Err(_) => resource.post_error(
                        gtk_surface1::Error::InvalidGesture as u32,
                        "no such gesture",
                    ),
                }
            }
            gtk_surface1::Request::Present { .. } | gtk_surface1::Request::RequestFocus { .. } => {
                let id = state.isle.wm.for_surface(surface).map(|r| r.id);
                if let Some(id) = id.filter(|id| {
                    state.isle.wm.get(*id).map(|r| r.app.clone()) != state.isle.front_app()
                }) {
                    state.want_attention(id, false);
                }
            }
            _ => {}
        }
    }
}
