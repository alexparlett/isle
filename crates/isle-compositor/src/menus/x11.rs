//! An X11 window's menu address from its properties: Qt's `_KDE_NET_WM_APPMENU_*`, GTK's
//! `_GTK_*` and appmenu-gtk-module's `_UNITY_OBJECT_PATH`. Read over a connection of Isle's
//! own to Xwayland, on the worker, rather than through the window manager's.

use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ConnectionExt},
    rust_connection::RustConnection,
};

/// The menu `window` names on `display`, if any.
pub fn menu_of(display: u32, window: u32) -> Option<isle_menus::Source> {
    let (conn, _) = RustConnection::connect(Some(&format!(":{display}"))).ok()?;
    let text = |name: &str| -> Option<String> {
        let atom = conn
            .intern_atom(true, name.as_bytes())
            .ok()?
            .reply()
            .ok()?
            .atom;
        if atom == 0 {
            return None;
        }
        let reply = conn
            .get_property(false, window, atom, AtomEnum::ANY, 0, 1024)
            .ok()?
            .reply()
            .ok()?;
        let value = String::from_utf8(reply.value).ok()?;
        let value = value.trim_end_matches('\0').to_owned();
        (!value.is_empty()).then_some(value)
    };
    if let (Some(service), Some(path)) = (
        text("_KDE_NET_WM_APPMENU_SERVICE_NAME"),
        text("_KDE_NET_WM_APPMENU_OBJECT_PATH"),
    ) {
        return Some(isle_menus::Source::DBusMenu { service, path });
    }
    let service = text("_GTK_UNIQUE_BUS_NAME")?;
    let menubar = text("_GTK_MENUBAR_OBJECT_PATH")?;
    let _ = conn.flush();
    Some(isle_menus::Source::GMenu {
        service,
        menubar,
        application: text("_GTK_APPLICATION_OBJECT_PATH"),
        window: text("_GTK_WINDOW_OBJECT_PATH"),
        unity: text("_UNITY_OBJECT_PATH"),
    })
}
