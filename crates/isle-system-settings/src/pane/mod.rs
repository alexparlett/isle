//! Each pane's rows, sub-pages and sheets (`p13/rows.md`).

mod accessibility;
mod appearance;
mod apps;
mod control_centre;
mod desktop;
mod displays;
mod energy;
mod focus;
pub mod general;
mod keyboard;
mod lock_screen;
mod mouse;
mod network;
pub mod notifications;
mod privacy;
mod sound;
mod spotlight;
mod wallpaper;

use freya::prelude::*;

pub use crate::frame::Nav;
use crate::nav::Location;

/// A sheet's name ends with an ellipsis, as the button that opens it reads; anything else
/// a pane's `sub` names is a sub-page.
pub fn is_sheet(sub: &str) -> bool {
    sub.ends_with('…')
}

/// The pane's body, remade whenever its sub-page or sheet changes.
pub fn body(pane: &str, at: &Location) -> Element {
    let at = at.clone();
    let key = format!("{pane}/{}", at.sub.as_deref().unwrap_or(""));
    let body: Element = match pane {
        "wi-fi" => network::WiFi { at }.into_element(),
        "bluetooth" => network::Bluetooth { at }.into_element(),
        "network" => network::Network { at }.into_element(),
        "vpn" => network::Vpn { at }.into_element(),
        "energy" => energy::Energy { at }.into_element(),
        "general" => general::General { at }.into_element(),
        "accessibility" => accessibility::Accessibility { at }.into_element(),
        "appearance" => appearance::Appearance { at }.into_element(),
        "apps" => apps::Apps { at }.into_element(),
        "control-centre" => control_centre::ControlCentre { at }.into_element(),
        "desktop" => desktop::Desktop { at }.into_element(),
        "displays" => displays::Displays { at }.into_element(),
        "spotlight" => spotlight::Spotlight { at }.into_element(),
        "wallpaper" => wallpaper::Wallpaper { at }.into_element(),
        "notifications" => notifications::Notifications { at }.into_element(),
        "sound" => sound::Sound { at }.into_element(),
        "focus" => focus::Focus { at }.into_element(),
        "lock-screen" => lock_screen::LockScreen { at }.into_element(),
        "privacy-security" => privacy::Privacy { at }.into_element(),
        "keyboard" => keyboard::Keyboard { at }.into_element(),
        "mouse" => mouse::Mouse { at }.into_element(),
        _ => rect().into_element(),
    };
    rect()
        .key(key)
        .width(Size::fill())
        .spacing(20.)
        .child(body)
        .into_element()
}

/// Goes to a sub-page or sheet of the pane shown.
pub fn open(nav: Nav, at: &Location, sub: &str) {
    nav.go(Location {
        pane: at.pane.clone(),
        sub: Some(sub.to_owned()),
        row: None,
    });
}

/// Closes a sheet, back to the pane under it.
pub fn close(nav: Nav, at: &Location) {
    nav.replace(Location::pane(&at.pane));
}
