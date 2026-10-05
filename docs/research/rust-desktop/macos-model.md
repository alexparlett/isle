# The macOS model on Linux: global menu, Cmd translation, behaviour map

State as of 2026-10-02. Evidence for B17–B21 of
`docs/work/rust-desktop/brainstorm.md`: what a compositor-side menu bar must
serve to collect app menus, how a compositor should translate Cmd shortcuts,
and which macOS behaviours Isle owes a Mac user compared with what v1 has.

**Method.** Source code was read, not summarised from blogs, wherever a claim
is about what a program does. Repositories were cloned on 2026-10-02 or read
through the GitHub API on that date; each is cited at the commit read.
Statements marked *reasoning* are inferences from that code, not tested.
Everything unverified is in **OPEN** at the end and nowhere else.

| Source read | Commit / version | Date of commit |
|---|---|---|
| RedBearAK/toshy | `354076728e50` | 2026-09-18 |
| rbreaves/kinto | `4a3bfe79e257` | 2024-01-26 |
| xremap/xremap | `a8a91cbe6352` | 2026-09-27 |
| rvaiya/keyd | `f564288ac2b1` | 2026-06-01 |
| Smithay/smithay | `118e34ffc9b9` | 2026-09-30 |
| KDE/plasma-workspace | `19e67e2e34f0` | 2026-10-02 |
| vala-panel-project/vala-panel-appmenu (GitLab) | `a783b01c8b65` | 2026-09-04 |
| nongio/otto | `38e3bf392cdf` | 2026-10-02 |
| chromium/chromium, electron/electron, qt/qtbase, qt/qtwayland, GNOME/gtk, LibreOffice/core, mozilla-firefox/firefox | `main` via GitHub API | read 2026-10-02 |

---

## 1. Global menu on Linux Wayland in 2026

### 1.1 The mechanisms

There is no single standard. Five mechanisms are in use, and a menu bar that
wants coverage has to serve or read all of the first four.

| Mechanism | What it is | Who owns which end |
|---|---|---|
| **`com.canonical.AppMenu.Registrar`** (session bus, object `/com/canonical/AppMenu/Registrar`) | Methods `RegisterWindow(u windowId, o menuObjectPath)`, `UnregisterWindow(u)`, `GetMenuForWindow(u) → (s service, o path)`, `GetMenus`; signals `WindowRegistered`, `WindowUnregistered`. The window id is an X11 window id; the service is taken from the caller's connection. ([interface XML in plasma-workspace](https://github.com/KDE/plasma-workspace/blob/19e67e2e34f09a1cbb4ccf87b5bf6f8faa049aec/appmenu/com.canonical.AppMenu.Registrar.xml)) | The desktop serves it. **Its presence on the bus is also the switch**: Qt, Chromium, Electron, LibreOffice and Firefox only export a menu when this name has an owner (§1.2). |
| **`com.canonical.dbusmenu`** | The menu itself, exported by the app: `GetLayout`, `GetGroupProperties`, `GetProperty`, `Event`, `EventGroup`, `AboutToShow`, `AboutToShowGroup`; signals `LayoutUpdated`, `ItemsPropertiesUpdated`, `ItemActivationRequested`. ([XML in vala-panel-appmenu](https://gitlab.com/vala-panel-project/vala-panel-appmenu/-/blob/a783b01c8b653349843fac9bbd075dac52cdc9de/subprojects/appmenu-glib-translator/com.canonical.dbusmenu.xml)) | App serves, desktop reads. Same protocol as tray menus (SNI), so one client serves both. |
| **`org_kde_kwin_appmenu_manager`** (Wayland, KDE, version 2, MIT) | `create(id, wl_surface)` then `org_kde_kwin_appmenu.set_address(service_name, object_path)`: ties a surface to its DBusMenu. ([wayland.app](https://wayland.app/protocols/kde-appmenu); [XML](https://github.com/KDE/plasma-wayland-protocols/blob/master/src/protocols/appmenu.xml)) | Compositor serves. As of 2026-10-02 the wayland.app support matrix lists **only KWin 6.7** (version 2); Mutter 51, Sway 1.11, Hyprland 0.52.1, niri 26.04, COSMIC 1.0.0~beta.8, Wayfire, Mir, labwc and the rest are marked unsupported. Bindings ship in the `wayland-protocols-plasma` crate 0.3.12 (2026-03-31, `src/lib.rs` has `pub mod appmenu`). |
| **`gtk_shell1` / `gtk_surface1.set_dbus_properties`** (Wayland, GTK private) plus **`org.gtk.Menus` / `org.gtk.Actions`** (D-Bus) | GTK tells the compositor `application_id, app_menu_path, menubar_path, window_object_path, application_object_path, unique_bus_name`; the menu is a GMenuModel exported over `org.gtk.Menus` and activated through `org.gtk.Actions`. The compositor advertises `capabilities` `global_app_menu` (1) and `global_menu_bar` (2). ([gtk-shell.xml, gtk main](https://github.com/GNOME/gtk/blob/main/gdk/wayland/protocol/gtk-shell.xml)) | Compositor serves `gtk_shell1`; the desktop reads GMenuModel. Implemented by Mutter 51 (v7), Muffin (v6), phoc and Wayfire (v3); not by KWin, Sway, Hyprland, niri or COSMIC ([wayland.app gtk-shell](https://wayland.app/protocols/gtk-shell), read 2026-10-02). No Rust crate carries `gtk-shell.xml`. |
| **X11 window properties** (XWayland apps) | `_GTK_UNIQUE_BUS_NAME`, `_GTK_MENUBAR_OBJECT_PATH`, `_GTK_APPLICATION_OBJECT_PATH`, `_GTK_WINDOW_OBJECT_PATH`, `_GTK_APPLICATION_ID` (GTK and LibreOffice set these, [gtkframe.cxx L592–600](https://github.com/LibreOffice/core/blob/master/vcl/unx/gtk3/gtkframe.cxx)); `_KDE_NET_WM_APPMENU_SERVICE_NAME` / `_OBJECT_PATH` set by KDE's Qt platform plugin ([KDE bug 424485](https://bugs.kde.org/show_bug.cgi?id=424485)). | The compositor's own X11 window manager reads them (B5 makes Isle that WM). |

Two further routes exist but are not deployed:

- **`dbus_annotation`**, a cross-desktop Wayland protocol to associate surfaces
  with D-Bus objects. The 2020 proposal ([wayland-protocols MR 52](https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/52))
  is still open (last activity 2026-03-01). A new proposal by Nicolas Fella
  (KDE), [MR 493](https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/493),
  opened 2026-03-01, is labelled "Needs acks, Needs review" (updated
  2026-08-24) with one ack (KDE) and draft implementations in KWin (MR 8883,
  draft), Qt (change 717474) and GTK (MR 9780, a draft that uses it for
  accessibility, not menus). Firefox's pref comment points at this protocol
  (§1.2). Read through the GitLab API, 2026-10-02.
- **AT-SPI.** [noctalia-appmenu](https://github.com/yolo-labz/noctalia-appmenu)
  (Apache-2.0, a Rust sidecar for niri) reads the focused app's menubar out of
  its accessibility tree instead of DBusMenu. Its README lists Qt6, GTK3,
  GTK4 and XWayland Qt5/GTK as working, Electron/Chromium only with
  `--force-accessibility`, and Firefox and libcosmic as not working.

### 1.2 Per toolkit

"Works" means the app exports a menu that a desktop can find for its window
without the user installing anything, given a registrar on the bus and the
named Wayland protocol in the compositor.

| Toolkit / app | Native Wayland | X11 / XWayland | Needs | Source |
|---|---|---|---|---|
| **Qt 6.9 and later** | **Works** via `org_kde_kwin_appmenu`. | Works via `RegisterWindow`. | A registrar on the bus **before the app starts**: `QGenericUnixTheme::isDBusGlobalMenuAvailable()` checks once and caches the answer in a function-local static. Any theme derived from `QGenericUnixTheme` creates the DBus menubar: generic, GNOME, GTK3 (v1's `QT_QPA_PLATFORMTHEME=gtk3`, D39), KDE, and the portal theme forwarding to its base. | [qdbusmenubar.cpp `registerMenuBar`](https://github.com/qt/qtbase/blob/dev/src/gui/platform/unix/dbusmenu/qdbusmenubar.cpp): registers `/MenuBar/<n>` on the session bus, calls `RegisterWindow` only when the platform is `xcb`, then `registerDBusMenuForWindow`, which on Wayland is `QWaylandAppMenu`. [qgenericunixtheme.cpp L244–257](https://github.com/qt/qtbase/blob/dev/src/gui/platform/unix/qgenericunixtheme.cpp), [qgnometheme.cpp L215](https://github.com/qt/qtbase/blob/dev/src/gui/platform/unix/qgnometheme.cpp). The Wayland half landed in qtwayland `aabc0f44` (2024-10-29, "Implement support for attaching dbus menu bar to a window ... uses the KDE specific protocol"); `src/client/qwaylandappmenu.cpp` exists at tag v6.9.0 and not at v6.8.0. The host has qt6-base 6.11.2. |
| **Qt 6 before 6.9, Qt 5** | Not through Qt itself (`RegisterWindow` is xcb-only). See OPEN for the KDE platform theme route. | Works. | Run the app under XWayland, or the KDE platform theme. | As above. |
| **GTK 3, `GtkApplication` with a menubar** | **Works** if the compositor implements `gtk_shell1` and advertises `global_menu_bar`; GTK then hides the in-window menubar (setting `gtk-shell-shows-menubar` follows the capability). | Works via `_GTK_*` properties. | `gtk_shell1`, a GMenuModel reader. | [gtk-3-24 `gdkscreen-wayland.c` L1086–1092](https://github.com/GNOME/gtk/blob/gtk-3-24/gdk/wayland/gdkscreen-wayland.c) |
| **GTK 3, plain `GtkMenuBar`** (most older GTK3 apps) | Needs **appmenu-gtk-module** in `GTK_MODULES`, its gsetting `org.appmenu.gtk-module run-on-wayland` set true (the default from 25.04, `global-menu-2026.md`), **and** `gtk_shell1` in the compositor: the module converts the menubar to GMenuModel and hands the paths over with `gdk_wayland_window_set_dbus_properties_libgtk_only`. Its TODO still lists "Wayland support without GTK compositor". | Works via the module and `_GTK_*` properties. | The module, the gsetting, `gtk_shell1`. Plasma's alternative, [appmenu-gtk-module-wayland](https://github.com/guiodic/appmenu-gtk-module-wayland) (AUR 26.08.03, LGPL-3.0), targets KWin. | [vala-panel-appmenu `subprojects/appmenu-gtk-module/src/{platform.c,support.c,hijack.c}`, `TODO`](https://gitlab.com/vala-panel-project/vala-panel-appmenu/-/tree/a783b01c8b653349843fac9bbd075dac52cdc9de/subprojects/appmenu-gtk-module) |
| **GTK 4 / libadwaita** | `GtkApplication` windows still call `gdk_wayland_toplevel_set_dbus_properties` on realize, so an app that sets `gtk_application_set_menubar` **works** through `gtk_shell1` + `org.gtk.Menus`, and `GtkApplicationWindow` drops its in-window bar when the shell shows it. **Most libadwaita apps have no menubar**: their menus live in a `GtkMenuButton` and are not exported. GTK 4 removed module loading, so there is no appmenu-gtk-module for it. Workarounds: build a menu from the app's `org.gtk.Actions` (the GNOME extension below), an `LD_PRELOAD` that promotes the header-bar menu model to the menubar ([unity-gtk4-menu](https://github.com/Ubuntu-Unity-LifeSupport/unity-gtk4-menu)), or AT-SPI. | Same as Wayland, through `_GTK_*`. | `gtk_shell1`, GMenuModel reader; a fallback menu for the rest. | [gtkapplication-wayland.c L130–145](https://github.com/GNOME/gtk/blob/main/gtk/gtkapplication-wayland.c), [gtkapplicationwindow.c L213–274](https://github.com/GNOME/gtk/blob/main/gtk/gtkapplicationwindow.c), [gdksettings-wayland.c L623–627](https://github.com/GNOME/gtk/blob/main/gdk/wayland/gdksettings-wayland.c) |
| **Chromium 137+** (Chrome, Brave, Vivaldi, Edge) | **Works** via `org_kde_kwin_appmenu`, created automatically when the compositor advertises the manager (`supports_global_application_menus = connection_->org_kde_kwin_appmenu_manager() != nullptr`), registered once the registrar name has an owner. No flag. | Works via `RegisterWindow`. | Registrar owner + `org_kde_kwin_appmenu`. | Commit [`9c30fb3`](https://github.com/chromium/chromium/commit/9c30fb37950c6a0a7ab2875f38ca3953e27963ae) "Provide dbus appmenu information on Wayland" (2025-04-07; chromiumdash: first in 137.0.7115.0); [dbus_appmenu_registrar.cc](https://github.com/chromium/chromium/blob/main/chrome/browser/ui/views/frame/dbus_appmenu_registrar.cc); [ozone_platform_wayland.cc L437](https://github.com/chromium/chromium/blob/main/ui/ozone/platform/wayland/ozone_platform_wayland.cc) |
| **Electron** (VS Code with the native title bar, which is not its default: by default VS Code exports nothing, `global-menu-2026.md`; Discord, Slack, Heroic) | **Broken, and actively harmful.** Electron has its own `GlobalMenuBarX11`. When the compositor advertises `org_kde_kwin_appmenu` and the registrar exists, Electron removes the in-window menu, exports DBusMenu and calls `RegisterWindow` with the Wayland widget id as if it were an X window id; no desktop can match it, so **the menu appears nowhere**. Issue [#34335](https://github.com/electron/electron/issues/34335) (opened 2022-05-23) is open; fix PR [#54629](https://github.com/electron/electron/pull/54629) "show global application menus on Wayland" was opened 2026-10-02 and is not merged. | Works via `RegisterWindow`. | On Wayland, see the PID fallback in §1.3. | [native_window_views.cc `SetMenu`](https://github.com/electron/electron/blob/main/shell/browser/native_window_views.cc), [global_menu_bar_registrar_x11.cc](https://github.com/electron/electron/blob/main/shell/browser/ui/views/global_menu_bar_registrar_x11.cc) |
| **Firefox, Thunderbird** | **Off by default.** `widget.gtk.global-menu.enabled` and `widget.gtk.global-menu.wayland.enabled` both default to `false`; with both on, Firefox uses `org_kde_kwin_appmenu` on Wayland and a registrar proxy. The Wayland pref's comment says it waits for the dbus_annotation protocol. Firefox loads libdbusmenu at run time (NixOS had to add `libdbusmenu-gtk3`, [nixpkgs PR 567009](https://github.com/NixOS/nixpkgs/pull/567009)). | Same prefs, `RegisterWindow`. | User prefs; registrar + `org_kde_kwin_appmenu`. | [StaticPrefList.yaml L20905–20917](https://github.com/mozilla-firefox/firefox/blob/main/modules/libpref/init/StaticPrefList.yaml), [NativeMenuGtk.cpp L770–837](https://github.com/mozilla-firefox/firefox/blob/main/widget/gtk/NativeMenuGtk.cpp); landed in [bug 1883184](https://bugzilla.mozilla.org/show_bug.cgi?id=1883184) (Firefox 125 branch) |
| **LibreOffice, GTK3 VCL** (the default on GTK desktops) | **Works** via GMenuModel + `gtk_shell1` (`gdk_wayland_window_set_dbus_properties_libgtk_only`), switched on only while `com.canonical.AppMenu.Registrar` has an owner (`EnsureAppMenuWatch`). | Works via `_GTK_*`. | Registrar owner, `gtk_shell1`, GMenuModel reader. | [gtkframe.cxx L580–680](https://github.com/LibreOffice/core/blob/master/vcl/unx/gtk3/gtkframe.cxx) |
| **LibreOffice, Qt6/KF VCL** | Through Qt (row 1). Reported working on COSMIC by cosmic-macos-applets. | Works. | As Qt. | [cosmic-macos-applets README](https://github.com/cmjcode/cosmic-macos-applets) |
| **JetBrains IDEs** | The built-in global menu (`linux.native.menu`) registers an X11 window id (from a search summary of `GlobalMenuLinux.java`, [git.jetbrains.org](https://git.jetbrains.org/?a=commitdiff&h=4f839ae92017e3b618390a2d8d83eecf1f972b16&p=idea%2Fcommunity.git); the source was not read). Since 2026.1 the IDEs default to native Wayland (WLToolkit) ([JetBrains blog, 2026-02](https://blog.jetbrains.com/platform/2026/02/wayland-by-default-in-2026-1-eap/)); the third-party plugin [Global Menu and Decoration](https://plugins.jetbrains.com/plugin/25530-global-menu-and-decoration) (GPL-3.0, 1.4.1 on 2026-07-21, IDE 262+) exports the menu with the KDE appmenu protocol and does not hide the in-IDE menu. | Built-in works. | Plugin on Wayland, or run under XWayland. | Plugin API `plugins.jetbrains.com/api/plugins/25530`, read 2026-10-02 |
| **Java Swing** (other) | No. | **jayatana**, an agent shipped in vala-panel-appmenu (`-Djayatana=enabled`). | jayatana; XWayland. | vala-panel-appmenu README |
| **Steam client** | n/a (X11 app). | No export found; Steam draws its own menu row. See OPEN. | Fallback menu. | |
| **Wine / Proton** | n/a. | **No.** A code search of `wine-mirror/wine` for `dbusmenu` returns nothing (2026-10-02); Win32 menus are drawn in the window. Games have no menus anyway. | Fallback menu. | GitHub code search |
| **Tauri (muda)** | No; menus are GTK window-local. A feature request for DBusMenu was opened 2026-10-01 ([tauri #16191](https://github.com/tauri-apps/tauri/issues/16191)). | No. | Fallback. | |

A known hazard: writing `gtk-modules="appmenu-gtk-module"` into
`~/.config/gtk-3.0/settings.ini` made a hybrid Chromium+Qt6+GTK3 app
(ChatGPT desktop) SIGSEGV about 1.7 s after launch on Plasma 6.6.6 / Ubuntu
26.04.1; plain GTK3 apps were fine ([macos-tahoe-liquid-kde #81](https://github.com/lestercorderomurillo/macos-tahoe-liquid-kde/issues/81)).
If Isle loads the module, it should do so through `GTK_MODULES` per launch
rather than globally, and be able to exclude an app.

### 1.3 What Isle's compositor has to implement

On the service worker (B9) unless marked "loop".

1. **Own `com.canonical.AppMenu.Registrar`** at `/com/canonical/AppMenu/Registrar`
   from session start, before any app starts. Qt caches the check per
   process, so an app started before the name exists never exports for its
   lifetime. Record `(window id, sender unique name, path)` per call and drop
   entries when the sender leaves the bus (`NameOwnerChanged`).
2. **`org_kde_kwin_appmenu_manager` v2** (loop). Store `(service, path)` on
   the toplevel that owns the surface. Covers Qt 6.9+, Chromium 137+, Firefox
   (with prefs) and the JetBrains plugin.
3. **`gtk_shell1` with `set_dbus_properties`** (loop), advertising
   `global_menu_bar`. Covers GTK3 `GtkApplication`, GTK4 apps with a
   menubar, LibreOffice GTK3, and GTK3 apps through appmenu-gtk-module.
   Advertising the capability makes GTK hide its own bar, so a missed or
   failed read leaves the app with no menu: only advertise once the reader
   works. Pick the version deliberately: v5 adds `titlebar_gesture`, v7
   `set_a11y_properties` (see OPEN). No crate carries the XML; generate
   bindings with `wayland-scanner` macros from GTK's file.
4. **A GMenuModel client** in zbus: `org.gtk.Menus` (`Start`, `End`,
   `Changed`) and `org.gtk.Actions` (`List`, `Describe`, `DescribeAll`,
   `Activate`, `SetState`, `Changed`), translated into the same internal menu
   tree as DBusMenu. No Rust crate exists. References: KDE's
   [gmenu-dbusmenu-proxy](https://github.com/KDE/plasma-workspace/tree/19e67e2e34f09a1cbb4ccf87b5bf6f8faa049aec/gmenu-dbusmenu-proxy)
   (C++, refuses to run off X11: `main.cpp` L28) and vala-panel-appmenu's
   `appmenu-glib-translator` (C).
5. **X11 sources in the XWM** (loop): map `RegisterWindow` ids to X11
   windows, and read the `_GTK_*` and `_KDE_NET_WM_APPMENU_*` properties.
6. **A PID fallback.** For a registration whose window id matches nothing
   (Electron on Wayland today), or a Qt app that registered `/MenuBar/<n>`
   without a protocol call, resolve the sender's PID with
   `GetConnectionUnixProcessID` and match the focused toplevel's client PID
   (Smithay gives the `wl_client` credentials). cosmic-macos-applets does the
   PID match against `/MenuBar/<n>` objects. Flatpak and other PID
   namespaces break this (OPEN). Otto's bar matches on "service name
   contains app_id" ([otto-bar `appmenu.rs` L67–83](https://github.com/nongio/otto/blob/38e3bf392cdf004b2bd550d41666ac192a76fe51/components/otto-bar/src/appmenu.rs)),
   which cannot match a unique name such as `:1.42` and is not a model.
7. **B18's fallback menu** for everything else: app name, About, Hide,
   Quit, the window list, plus Window and Help menus the desktop owns.
8. **Every call into an app is asynchronous with a timeout.**
   cosmic-macos-applets uses 2 s. Call `AboutToShow` before opening a
   submenu, re-fetch on `LayoutUpdated`, batch bursts of signals (Otto waits
   100 ms after the first, [specs/topbar.md](https://github.com/nongio/otto/blob/38e3bf392cdf004b2bd550d41666ac192a76fe51/specs/topbar.md)).
9. **Menu search.** With the trees in hand, Spotlight can search the front
   app's menu items, which macOS Tahoe's Spotlight does
   ([MacMost](https://macmost.com/using-spotlight-to-access-the-menu-bar.html))
   and Unity's HUD did.
10. **Shortcut hints.** DBusMenu items carry their shortcut as Ctrl chords.
    The menu bar should show them as the Mac chord the user presses, using
    the inverse of the Cmd translation table (§2).

**Window to menu mapping, in order:** Wayland toplevel → the appmenu or
gtk_surface address set on its surface; X11 window → registrar entry by XID,
then `_GTK_*` / `_KDE_NET_WM_APPMENU_*` properties; either → registrar entry
or `/MenuBar/<n>` object owned by a connection with the toplevel's PID;
otherwise the fallback menu. `app_id` alone never identifies a menu; it is
only what names the app in the bar.

### 1.4 How existing implementations do it

| Implementation | Registrar | Wayland protocol | GMenuModel | Mapping | Notes |
|---|---|---|---|---|---|
| **KDE Plasma** (applet + kded `appmenu`) | kded module `MenuImporter` owns it ([menuimporter.cpp](https://github.com/KDE/plasma-workspace/blob/19e67e2e34f09a1cbb4ccf87b5bf6f8faa049aec/appmenu/menuimporter.cpp)) | KWin serves `org_kde_kwin_appmenu` | X11 only, through `gmenu-dbusmenu-proxy` | KWin exposes service/path per window to the task manager model the applet reads | The reference implementation; on Wayland, GTK3 apps need appmenu-gtk-module-wayland. |
| **vala-panel-appmenu** (Xfce, MATE, Vala Panel; Budgie planned) | `subprojects/registrar` | none | yes (`appmenu-glib-translator`) | X11 (libwnck) | Ships appmenu-gtk-module and jayatana. GitHub mirror updated 2026-09-04. |
| **GNOME extensions** | [felixefelip/gnome-global-menu](https://github.com/felixefelip/gnome-global-menu) serves it | Needs his [Mutter fork](https://github.com/felixefelip/mutter) with `org_kde_kwin_appmenu` for Chromium/Electron | Reads `org.gtk.Menus` paths that Mutter exposes; builds menus from `org.gtk.Actions` for libadwaita apps | XID, then PID | Targets GNOME Shell 50. [orbit-global-menu](https://github.com/Unmade760/orbit-global-menu) (GPL-3.0) reads `org.gtk.Menus` and ships keyboard-shortcut menus for 171 apps that export nothing. |
| **COSMIC** | Not in COSMIC. Community [cosmic-macos-applets](https://github.com/cmjcode/cosmic-macos-applets) (GPL-3.0-only) serves it | none | no | Wayland: PID of the focused window against `/MenuBar/<n>`; X11: XID | cosmic-comp [#2876](https://github.com/pop-os/cosmic-comp/issues/2876) (2026-09-20) asks for a locally integrated menu; no maintainer reply. |
| **Otto** (Smithay, Skia) | `otto-bar` serves it in zbus (MIT) | none | no | Heuristic on bus name | Bar is a separate layer-shell client. |
| **noctalia-appmenu** (niri) | none | none | no | niri IPC focus + AT-SPI tree | Apache-2.0, Rust. |
| **Heaven** (Cuarzo, Louvre) | none: its own D-Bus protocol | Compositor hands the client a token over Wayland that the client presents over D-Bus | no | Token | LGPL-2.1, 0.1.0; needs toolkits to adopt it ([repo](https://github.com/CuarzoSoftware/Heaven)). |
| **Mir** | — | Issue [#3545](https://github.com/canonical/mir/issues/3545) (2024-08-10, open) asks for the KDE protocol | — | — | |
| **waybar** | none | none | no | — | Uses libdbusmenu only for tray menus (`include/modules/sni/item.hpp`). |

### 1.5 Rust crates

| Crate | Version, date | Licence | Use |
|---|---|---|---|
| [`system-tray`](https://github.com/jakestanger/system-tray) | 0.8.9, 2026-08-29 | MIT | Async SNI **and DBusMenu client** on zbus 5 + tokio (ironbar's). The closest fit for reading menus; check that its GTK feature is optional. |
| `dbusmenu-zbus` | 0.1.0, 2024-11-15 | MIT OR Apache-2.0 | zbus interface definitions only; unmaintained. |
| [`ksni`](https://github.com/iovxw/ksni) | 0.3.6, 2026-07-15 | Unlicense | DBusMenu **server** side (for Isle's own status items, not reading). |
| `dbusmenu-glib`, `dbusmenu-gtk3` (+ `-sys`) | 0.1.0, 2023-04-05 | — | libdbusmenu FFI; pulls GLib, against B11. |
| `appmenu-glib-translator(-sys)` | 0.1.0, 2026-01-12 | — | FFI to vala-panel's GMenuModel translator; GLib. |
| `wayland-protocols-plasma` | 0.3.12, 2026-03-31 | MIT | `org_kde_kwin_appmenu` bindings. |
| GMenuModel (`org.gtk.Menus`) | none | | Write it (§1.3 item 4). |

Crate data from the crates.io API, 2026-10-02.

---

## 2. Cmd-key translation

### 2.1 Precedents

| Project | How it works | App awareness | Licence | State |
|---|---|---|---|---|
| **[Toshy](https://github.com/RedBearAK/toshy)** | Python keymapper (`xwaykeyz`, vendored) grabs evdev and re-emits through uinput. Config `default-toshy-config/toshy_config.py`, 6,229 lines. | Window class and title from X11, KWin script, GNOME extension, COSMIC protocol, `zwlr_foreign_toplevel_manager_v1` (Hyprland, niri, sway, labwc, Wayfire, river) | GPL-3.0 | Active; last commit 2026-09-18. The most complete table. |
| **[Kinto](https://github.com/rbreaves/kinto)** | xkeysnail, X11 only. Toshy's ancestor (same table shape). | `WM_CLASS` | GPL-2.0 | Last commit 2024-01-26; README calls it "at a state of completion". |
| **[xremap](https://github.com/xremap/xremap)** | Rust, evdev → uinput, YAML. | `--desktop` gnome, x11, hypr, kde, wlroots, niri, cosmic, pantheon, socket (README L505–511) | MIT | Active (2026-09-27). Ships no macOS config; v1 generated one (inventory §11). |
| **[keyd](https://github.com/rvaiya/keyd)** | C daemon, layers. [`examples/macos.conf`](https://github.com/rvaiya/keyd/blob/f564288ac2b19d2305a5b39023c474805ff8fce5/examples/macos.conf): a `meta_mac:C` layer (Cmd inherits Ctrl), `c = C-insert`, `v = S-insert`, `x = S-delete`, `left = home`, Cmd+Tab via `swapm(app_switch_state, M-tab)`. | `keyd-application-mapper`: "At the moment X, Sway and Gnome are supported." | MIT | Last commit 2026-06-01. |
| **[gnome-macos-remap-wayland](https://github.com/petrstepanov/gnome-macos-remap-wayland)** | xremap config (111 lines): swaps Ctrl and Meta globally, then per-app fixes for Nautilus, GNOME Terminal, Console, Ptyxis, Eclipse. | xremap's GNOME extension | none stated | 2025-11-30 |
| **[macos-keybindings-extension](https://github.com/kadir-gunel/macos-keybindings-extension)** | xremap + GNOME extension | GNOME | NOASSERTION | 2026-10-01 |
| **v1** | xremap per device; `shell/keymap.json` `profiles.mac` {`text`, `control`, `terminal`}, `macTerminalIds` [kitty, foot, footclient, Alacritty, com.mitchellh.ghostty, org.wezfurlong.wezterm] | xremap-hypr | own | Inventory §11, D58–D61. |

None of them is a compositor. All of them sit below it and fake a keyboard,
which is why they need suspend timeouts, pointer monitors and per-desktop
window-context plugins. B21 puts the same table inside the compositor, which
sees the device, the focused surface and the modifier state directly.

### 2.2 The model they share

Kinto and Toshy decide **modifiers first, then chords** ([Toshy L2375–2520](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L2375)):

- On an Apple keyboard in a **GUI app**, physical Cmd becomes Right Ctrl
  ("RC" in every table) and physical Ctrl becomes Super.
- In a **terminal**, Cmd still becomes Right Ctrl, and Ctrl stays Ctrl.
- Option is Alt in both.
- Then keymaps rewrite the exceptions, written as `RC-<key>` → output chord.

So Cmd+key becomes Ctrl+key everywhere by default, and the tables list only
where an app differs. The left/right split matters: Right Ctrl is "Cmd",
Left Ctrl is "the real Ctrl". Terminal keymaps bind `LC-` and `RC-`
separately.

B21 differs on one point: "Ctrl stays Ctrl" in every app. Toshy moves Ctrl
to Super in GUI apps so that Mac Ctrl chords (Ctrl+A/E emacs motion, which
macOS text fields honour) can be told apart from Cmd chords and rewritten.
v1 kept Ctrl as Ctrl and translated the emacs chords explicitly
(`profiles.mac.control`, passed through untouched in terminals). In a
compositor the distinction comes for free: the translator knows which
physical key is down, so it never has to repurpose a modifier to remember it.

### 2.3 The tables to start from

Toshy at `354076728e50` ([file](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py)); line numbers link into that file.

**App-class lists** (regexes, case-folded):

| List | Lines | Contents |
|---|---|---|
| `terminals` | [L484–536](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L484-L536) | 50 entries: alacritty, kitty, `.*ghostty.*`, foot (`org.codeberg.dnkl.foot.desktop`), wezterm, konsole, gnome-terminal, `.*Ptyxis.*`, Warp, tabby, xterm, … |
| `term_sigint_preventer_lst` | L544 | Ptyxis, konsole |
| `vscodes` | [L551](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L551) | code, code-oss, cursor, vscodium |
| `sublimes` | L565 | |
| `remotes` | [L592](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L592) | AnyDesk, Boxes, Remmina, qemu, virt-manager, VirtualBox, xfreerdp, Citrix: no remapping at all |
| `browsers_chrome`, `browsers_firefox`, `browsers_all` | [L654–704](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L654) | Chromium family, Firefox family incl. zen, LibreWolf, floorp; plus Discord, Epiphany |
| `thunderbirds` | L712 | incl. title patterns for compose windows |
| `filemanagers` | [L733](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L733) | 16, incl. Nautilus, Dolphin, Thunar, COSMIC Files |
| `dialogs_Escape_lod`, `dialogs_CloseWin_lod` | L756, L785 | dialogs that take Escape or Alt+F4 for Cmd+W |
| `keyboards_Apple` | L833 | device-name patterns for the Mac profile |

**Keymaps:**

| Keymap | Lines | What it does |
|---|---|---|
| General GUI | [L6097](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L6097) | Cmd+Tab → Alt+Tab held (`bind`), Cmd+` → Alt+`, Cmd+Q → Alt+F4, Cmd+Space → launcher, Cmd+H blocked unless a desktop override exists, Cmd+←/→ → Home/End, Cmd+↑/↓ → Ctrl+Home/End (with Shift for selection), Cmd+Backspace → Ctrl+Shift+Backspace, Option+Delete → Ctrl+Delete, Super+a/e/b/f/n/p/k/d (the physical Ctrl) → emacs motion |
| GenGUI overrides per desktop | L5709–6090 | Cmd+H, Cmd+Space and minimise per desktop (GNOME, KDE, COSMIC, Hyprland, sway, …) |
| Wordwise (not VS Code) | [L4863](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L4863) | Option+←/→ → Ctrl+←/→ (with Shift) |
| General Terminals | [L5606](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L5606) | Cmd+C/V/T/N/W/F/S/L/D → Ctrl+Shift+same; Cmd+= → Ctrl+Shift+=; Cmd+. → Ctrl+C; Option+Backspace → Ctrl+W; Option+Delete → Esc d; Cmd+Backspace → Ctrl+U; Cmd+Delete → Ctrl+K; Cmd+Shift+←/→ → tab nav; Ctrl+Tab → Ctrl+PgDn |
| Per-terminal overrides | L5372–5480 | Alacritty, Contour, COSMIC Terminal, Ghostty (Cmd+D → split), kitty (Cmd+K → Ctrl+L), Konsole, Terminology, Wave, Xfce, plus per-desktop GenTerms overrides L5487–5600 |
| SIGINT preventer | L5361 | Cmd+C → Ctrl+Insert in Ptyxis and Konsole, so a copy chord can never become Ctrl+C |
| Browsers | L4605–4722 | Firefox: Cmd+1..9 → Alt+1..9, Cmd+Shift+N → private window, Cmd+, opens `about:preferences` by typing it; all browsers: Cmd+Q → Ctrl+Q, Cmd+Option+I/J → devtools, Cmd+Option+←/→ → tab nav |
| VS Code | [L4930](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L4930) | Cmd+C → Ctrl+Insert (copies in the editor and its terminal), Cmd+G → F3, Cmd+Option+F → Ctrl+H, Option+arrows prefixed with Alt+F19 to defeat the Alt menu |
| JetBrains | [L4766](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L4766) | class `^jetbrains-(?!.*toolbox).*$`; Cmd+0..9 → Alt+0..9 tool windows, Cmd+, → Ctrl+Alt+S, Cmd+; → project structure, Cmd+G → F3, Ctrl+G → Alt+J |
| File managers | [L4531](https://github.com/RedBearAK/toshy/blob/354076728e5009e40fb6aa84a658e0e01b5e8faf/default-toshy-config/toshy_config.py#L4531), per-FM L4233–4477 | Cmd+I → Alt+Enter (Get Info), Cmd+Shift+. → Ctrl+H (hidden files), Cmd+[ / ] → back/forward, per-FM Cmd+↑/↓, Cmd+Backspace → Delete, Enter → rename |
| XDG file dialogs | L4487–4530 | Finder keys inside GTK/Qt open/save dialogs |
| Cmd+W dialog fixes | L5180–5222 | Escape or Alt+F4 for windows with no Ctrl+W |
| Tab navigation fixes | L5223–5290 | apps wanting Ctrl+Tab or Ctrl+Alt+PgUp/PgDn instead of Ctrl+PgUp/PgDn |
| Option special characters | L3686, L3860 | Option+letter → macOS ABC / US characters, with dead keys L2650–3680 |
| Timeouts | L46, L1220–1250 | global `suspend = 0`; 1 s for Firefox and VS Code |

Kinto at `4a3bfe79e257` ([linux/kinto.py](https://github.com/rbreaves/kinto/blob/4a3bfe79e2578dd85cb6ff2ebc5505f758c64ab6/linux/kinto.py)), 846 lines: terminals L10, VS Code L44, remotes L53, browsers L75, Chromes L94, modmaps L124 (GUI) and L159 (terminals), JetBrains L196, file managers L290–405, browsers L447–467, terminal overrides L530 and L758–785, VS Code L646, Sublime L689. It is the smaller, older form of Toshy's table.

v1's [`shell/keymap.json`](../../../shell/keymap.json) `profiles.mac` is already the distilled
version of this for Isle: `text` (Cmd and Option motion), `control` (emacs
Ctrl chords) and `terminal` (Cmd+C/V/T/W/N/F/A/K → Ctrl+Shift+, Ctrl chords
passed through), with shell chords derived from `actions[].mac` (D60).

**Licences.** Toshy is GPL-3.0 and Kinto GPL-2.0: their config files are code,
so do not copy them into Isle's tree. Use them as the record of which apps
need what, and write Isle's table in its own format (KDL, B10). xremap and
keyd are MIT; snippets may be reused with the notice.

### 2.4 Edge cases they document

| Case | What macOS does | What the precedents do | For Isle |
|---|---|---|---|
| **Cmd+Tab, Cmd+`** | App switcher; cycle the front app's windows | Translate to the desktop's Alt+Tab with Alt held while Cmd is held (`bind`) | Compositor action (B20); never reaches an app. v1 needed a non-consuming transparent release bind to commit on release (§16). |
| **Cmd+Q** | Quit the app | Alt+F4 generally, Ctrl+Q in browsers | Compositor action: close every toplevel of the app (B20), or the app's Ctrl+Q where the table says it has one. |
| **Cmd+W** | Close the window or tab | Ctrl+W; Escape or Alt+F4 for dialogs without it; Ctrl+Shift+W in terminals | Translate to Ctrl+W per table; for dialogs, `xdg_toplevel.close` directly. |
| **Cmd+H, Cmd+Option+H, Cmd+M** | Hide app, hide others, minimise | Per-desktop minimise chords; blocked where none exists, since Ctrl+H is Replace in editors | Compositor actions; never forward. |
| **Cmd+Space** | Spotlight | The desktop's launcher | Compositor action. Ctrl+Space (macOS input source) stays Ctrl. |
| **Cmd+,** | App settings | Ctrl+, mostly; JetBrains Ctrl+Alt+S; Firefox has none, Toshy types `about:preferences` | Per-app table; fall back to the app menu's Settings item from DBusMenu. |
| **Cmd+.** | Cancel | Escape in GUI apps, Ctrl+C in terminals | Same. |
| **Cmd+C in terminals** | Copy | Ctrl+Shift+C; Ctrl+Insert in Ptyxis/Konsole; Ctrl+Insert in VS Code so its integrated terminal copies rather than interrupts | Terminal table from v1 plus Toshy's; for apps with an embedded terminal (VS Code, JetBrains) prefer Ctrl+Insert / Shift+Insert. kitty binds Shift+Insert to paste the primary selection (OPEN), so terminals keep Ctrl+Shift+V. |
| **Option word motion** | Option+←/→ by word, Option+Backspace deletes a word | Ctrl+←/→, Ctrl+Backspace; Ctrl+W in terminals; Esc d for forward delete | Same, from v1's `text` profile. |
| **Cmd+arrows, Cmd+Backspace** | Line/document start and end; delete to line start | Home/End, Ctrl+Home/End; Shift+Home then Backspace, or Ctrl+U in terminals | Same. Text field vs page: in browsers Cmd+← means Back outside a text field; the precedents cannot tell the two apart (see §2.5 on text-input-v3). |
| **Alt alone opens menus** | — | Firefox and VS Code focus their menu bar when Alt is pressed and released. Toshy withholds modifiers for 1 s in those apps (`suspend`) and prefixes Alt+F19 to Option chords in VS Code; Firefox users set `ui.key.menuAccessKeyFocuses = false` | A compositor need not send the Alt press at all for a chord it translates (§2.5), which removes the problem rather than timing around it. |
| **Option special characters** | Option+letter types é, ß, … | Toshy's OptSpecialChars keymaps and dead keys; since 2026-06-15 a Level3 Shift on left Option by default | Use the keyboard's XKB Mac variant (v1 D32, D61) for Level 3 rather than a character table, so it stays layout-correct. Decide whether Option acts as Alt (app shortcuts) or Level 3 (characters) per device. |
| **Remote desktops and VMs** | — | `remotes` list: no remapping | Honour `zwp_keyboard_shortcuts_inhibit_v1` (Smithay has it) and pass everything untranslated while it is active; keep a class list as a fallback. |
| **Games** | — | Not handled specially; Toshy suggests a zero global `suspend` "best for games" | Pass through untranslated for game windows: v1's game detection (§3.8: GameMode registration, `steam_app_\d+`, gamescope, `*.exe`, fullscreen) and game mode itself. Ctrl must reach games as Ctrl (crouch), which rules out Toshy's GUI modmap there. |
| **Tab navigation** | Cmd+Shift+[ / ] | Ctrl+PgUp/PgDn, with per-app exceptions | Table. |
| **Cmd+1…9** | Tab n in browsers; tool window in JetBrains | Alt+n in Firefox and JetBrains, Ctrl+n elsewhere | Table. Conflicts with B21 desktop shortcuts are settled by v1's rule: shell chords move off Cmd where apps need Cmd (D58). |

### 2.5 How a compositor should do it

**Where.** Smithay's `KeyboardHandle::input` runs a filter on every key with
the current `ModifiersState` and keysym before forwarding; the filter returns
`Forward` or `Intercept` ([keyboard/mod.rs L1026–1100](https://github.com/Smithay/smithay/blob/118e34ffc9b99854a2230c4805e1f37dc9029edb/src/input/keyboard/mod.rs#L1026)).
`input_intercept` plus `input_forward` split the two for deferred decisions
(L1115–1185). A `KeyboardGrab` receives `KeyboardInnerHandle::input(keycode,
state, modifiers: Option<ModifiersState>, …)`, which sends a key and an
arbitrary modifiers event to the focus (L1540–1570). Translation therefore
lives in the input filter, ahead of desktop bindings, keyed by:

1. the **device** the event came from (the Mac profile is per device, B21; v1's silent wrong-device failure, D58, disappears because the compositor sees the libinput device directly);
2. the **focused toplevel's class**: `xdg_toplevel.app_id`, or `WM_CLASS` from Isle's own XWM for X11 windows (B5), plus the title for dialog rules;
3. **game mode and shortcuts-inhibit**, which turn translation off.

**Modifier consistency.** Smithay keeps one XKB state per seat, built from the
physical keys (`key_input`, L305). A translated chord must present the
client with a self-consistent story: the key events it receives and the
modifier mask must agree. *Reasoning:* for Cmd+C → Ctrl+C the cleanest form is
what uinput remappers do, expressed as compositor-side events: withhold the
Super press until the next key decides (a desktop shortcut, a translated
chord, or a lone tap), then send synthetic Control_L press, `modifiers`
(Ctrl), C press; on C release, C release; on Cmd release, Control_L release
and `modifiers` (none). Clients that track modifiers from `wl_keyboard.modifiers`
(most Wayland toolkits) and clients that also watch modifier keycodes
(Xwayland, Chromium's Alt-menu heuristic) then see the same thing. Track the
synthetic state separately from the seat's XKB state so it can be unwound.

**Focus changes mid-chord.** Smithay's `set_focus` announces the
seat's forwarded pressed keys and modifier state to the new focus
(`focus_enter_state`, L1590). If focus moves while Cmd is held (Cmd+Tab
does exactly this), the old client must get the synthetic releases before
`leave`, and the new client must be entered with the state *its* table
implies. Otherwise modifiers stick: the classic failure of uinput remappers,
which Toshy works around with an emergency-eject hotkey and a pointer
monitor (xwaykeyz README L32–35, L448).

**Key repeat.** `wl_keyboard` repeat is client-side (`repeat_info`), and
Xwayland repeats on its own; a single translated key held down repeats
naturally as long as its press is forwarded and its release held back. A
chord translated to a sequence (Cmd+Backspace → Shift+Home, Backspace) is a
one-shot macro and will not repeat; xwaykeyz states the same split (README
L29: held single combos ride compositor repeat).

**Xwayland.** X11 apps get keys through Xwayland, which is an ordinary
Wayland client, so translation applies unchanged; only the class comes from
the XWM. *Reasoning:* sending synthetic modifier key events rather than only
a rewritten mask matters most here, because the X server derives its own
modifier state from key events.

**Games reading raw input.** Games on Wayland or Xwayland read keys from the
compositor, not from `/dev/input`; there is no raw path for them to bypass a
compositor translation. The guarantee Isle owes them is to translate nothing
in a game window. Super chords that are desktop actions should stay off in
game mode except a small escape set, as v1's game mode did.

**Text fields.** No precedent can tell a text field from the rest of an app.
A compositor sees `zwp_text_input_v3` enable/disable from clients that
support input methods (Smithay implements it), which approximates "a text
field has focus" (OPEN: reliability per toolkit; Chromium needs its Wayland
IME flag). This could let Cmd+← mean Home in a field and Back elsewhere, as on
macOS.

**Virtual keyboards.** Smithay attributes events to a `KeyboardSource`
(physical, or auxiliary for `zwp_virtual_keyboard_v1` and libei, L208–231).
Isle's own injected keys (v1's paste-into-previous-window, OSK, emoji typing)
should enter as an auxiliary source and bypass translation.

**Verification.** v1 measured its remap with a uinput device (`tools/keycheck.py`,
D59). The compositor equivalent is a headless test that feeds key events
through the filter with a fake focus class and asserts the events and
modifier masks the client would receive, including focus changes mid-chord.

---

## 3. macOS behaviours against v1

The macOS reference is macOS 27 "Golden Gate", released 2026-09-14
([AppleInsider, 2026-09-09](https://appleinsider.com/articles/26/09/09/macos-27-golden-gate-will-be-available-on-september-14)).
Option names come from Apple's Desktop & Dock settings page for macOS 27
([support.apple.com](https://support.apple.com/guide/mac-help/change-desktop-dock-settings-mchlp1119/mac),
read 2026-10-02). "Inv." is `isle-v1-inventory.md`.

| macOS behaviour | v1 today | New Isle needs | What Linux makes hard |
|---|---|---|---|
| **Menu bar**: Apple menu, bold app name, app menus (File, Edit, View, Window, Help with menu search), status items, clock | None; the island pill is the only top surface (Inv. §4 Island) | B18: §1 above; Apple-menu equivalent (About, Settings, Force Quit, Sleep, Restart, Shut Down, Lock, Log Out); Help-menu search over DBusMenu | Per-toolkit export (§1.2): libadwaita, Electron on Wayland, Firefox by default, Steam and games export nothing |
| **Control Centre**: menu-bar module with Wi-Fi, Bluetooth, AirDrop, Focus, display, sound, now playing; modules pinnable to the bar | Island hover panel: toggles, outputs, volume, brightness, media, tray, batteries (Inv. §4 Island details) | A menu-bar Control Centre, with modules that can also sit in the bar as status items | Nothing beyond B11's services |
| **Notification Centre and widgets**: click the clock; history plus widgets; widgets on the desktop ("Show Widgets", "Dim widgets on desktop"); extra-large desktop widgets and live widgets in macOS 27 ([MacRumors, 2026-09-17](https://www.macrumors.com/2026/09/17/new-features-changes-macos-27/)) | Notification centre as an island page; DND, quiet hours, per-app rules; full-screen widget dashboard (Inv. §4, §6) | A right-edge Notification Centre from the clock; widgets in it and on the desktop (B14 providers) | No equivalent of iPhone widgets |
| **Spotlight**: Cmd+Space; Cmd+1 Apps, Cmd+2 Files, Cmd+3 Actions, Cmd+4 Clipboard (Tahoe, [MacRumors](https://www.macrumors.com/how-to/do-more-with-spotlight-in-macos-tahoe/)); menu-item search; "Search or ask" with Siri AI in macOS 27 | Launcher on Super+Space with prefixes: run, calculator, file names, file contents, emoji, clipboard, windows, vault, web (Inv. §4 Launcher) | Same launcher on Cmd+Space with Mac quick keys; menu search; an assistant slot is optional | No system file index like Spotlight's; v1 used fd and rga on demand (see OPEN on indexers) |
| **Launchpad / Apps**: Launchpad removed in Tahoe, replaced by Spotlight's Apps grid without folders ([How-To Geek](https://www.howtogeek.com/replace-launchpad-in-macos-tahoe-here-are-your-options/)) | Apps ranked in the launcher (Inv. §4) | An Apps grid inside the launcher | None |
| **Dock**: size, magnification, position, minimise animation (Genie or Scale), "Minimize windows into application icon", auto-hide, "Show indicators for open applications", "Show suggested and recent apps", badges and progress; macOS 27 marks apps still running in the background after quit | None; minimise parks windows on `special:hidden` (Inv. §3.4, §3.6) | B19, including the genie/scale animations drawn by the compositor and a minimised-windows area | Badges and progress come from `com.canonical.Unity.LauncherEntry` signals; Electron 44 restored Linux support for them ([electron #52895](https://github.com/electron/electron/pull/52895)); coverage of other apps is uneven |
| **App-centric switching**: Cmd+Tab apps, Cmd+` windows, Cmd+H hide, Cmd+Option+H hide others, Cmd+Q quit vs Cmd+W close, "Close windows when quitting an application", "When switching to an application, switch to a Space with open windows" ([shortcuts](https://support.apple.com/en-us/102650)) | Switcher groups by app (D59), backtick cycles, Q quits, W closes; keep-running list hides instead of closing (Inv. §3.6, §4 Switcher) | B20 | **An app that exits when its last window closes cannot be kept alive by the compositor.** Most GTK and Qt apps quit on last-window close; Isle can only hide instead of close (v1's keep-running) or accept the exit. "App" identity comes from app_id/WM_CLASS through desktop entries (v1's heuristicLookup); Quit has no protocol, so it is "close every toplevel" or the app's own Ctrl+Q |
| **Mission Control**: Ctrl+↑, three-finger swipe up, drag a window to the top edge; App Exposé Ctrl+↓; "Group windows by application" ([support](https://support.apple.com/guide/mac-help/view-open-windows-spaces-mission-control-mh35798/mac)) | Overview with a spaces bar, spread windows, drag between desktops, live previews only while visible (Inv. §4, D19, D55) | Same, plus App Exposé, grouping by app, drag-to-top entry | None; previews are the compositor's own textures (B13) |
| **Spaces and full-screen Spaces**: Ctrl+←/→, "Automatically rearrange Spaces based on most recent use", "Displays have separate Spaces"; full-screen apps get their own Space | Desktops on Super+1..9, Super+Ctrl+←/→; a fullscreen window, borderless games included, is moved to its own workspace (Inv. §3.6, D48) | Same model with Mac chords (Ctrl+←/→, Ctrl+n) and full-screen Spaces in the Spaces bar | Ctrl+←/→ collides with word motion in Linux apps once Option is translated to Ctrl; the compositor must take Ctrl+←/→ only from the physical Ctrl key (§2.2) |
| **Split View**: two full-screen apps sharing a Space | None | A two-window full-screen Space with a divider | None |
| **Window tiling**: drag to left/right edge, drag to menu bar to fill, hold Option while dragging, "Tiled windows have margins"; Fn+Ctrl+F/C/←/→/↑/↓/R and multi-window arrangements with Shift and Option ([support](https://support.apple.com/guide/mac-help/mac-window-tiling-icons-keyboard-shortcuts-mchl9674d0b0/mac)); green-button menu | Drag-to-edge halves, quarters, top = fill; Super+arrows; snap ghost (Inv. §3.4, D40, D91) | B8's zones (ultrawide thirds, centred 60%) with the Mac gestures and a green-button menu | **Fn is invisible on most keyboards**: Keychron and most third-party boards handle Fn in firmware. On Apple keyboards hid-apple maps the Fn usage to `KEY_FN` (see OPEN). Isle needs a Fn-free chord for tiling |
| **Stage Manager**: recent apps strip; "Show recent apps", "Show windows from an application" | None | A per-workspace layout (B8's trait) | None |
| **Hot corners** (with optional modifier) | Only the 4 px game-mode hot zone that peeks the island (Inv. §4 Hot zone) | Four configurable corners: Mission Control, App windows, Desktop, Notification Centre, Lock, display sleep | Corner detection across monitors with a KVM (D56, D76) |
| **Trackpad gestures**: three-finger swipe up/down, three/four-finger swipe between Spaces, pinch for desktop, two-finger swipe from the right edge for Notification Centre | Three fingers up/down for Mission Control (Inv. §3.1) | Swipe and pinch through Smithay's pointer-gestures | libinput gesture events carry no finger positions, so an edge-start swipe needs the compositor to read touch positions some other way (OPEN). The host is a desktop: mouse-driven equivalents matter more |
| **Quick Look**: Space in Finder, Open/Save panels and Spotlight | Quick Look in Files on Space (Inv. §5 Files) | A Quick Look service shared by Files, the file chooser (B13) and Spotlight | Previewers per format |
| **Screenshots**: Shift+Cmd+3 screen, Shift+Cmd+4 region (Space for a window, Option without shadow), Shift+Cmd+5 toolbar with recording, timer, pointer; floating thumbnail; add Ctrl to copy to the clipboard; saves to the Desktop as "Screen Shot [date] at [time]" ([support](https://support.apple.com/en-us/102646)) | Capture bar: region, window, screen, record, countdown, cursor, audio, annotate, colour pick; freeze before pick; Print, Super+Shift+S, Super+Ctrl+4 (Inv. §4 Capture, §11) | The exact Mac chords, the Space toggle to window mode, the thumbnail, the Ctrl-to-clipboard rule; capture in the compositor (B13) | None |
| **Finder conventions**: Cmd+↑ parent, Cmd+↓ open, Cmd+Delete to Trash, Enter renames, Shift+Cmd+G Go to Folder, Shift+Cmd+. hidden files, Cmd+I Get Info, Option+Cmd+V move | Files is Finder-shaped: tabs, sidebar, List/Columns/Grid, Quick Look, saved searches, undo (Inv. §5) | Keep; key map from §2's Finder tables; Get Info | Tags have no shared standard across Linux file managers |
| **System Settings**: sidebar with search, account at the top | Sidebar groups with back/forward; page index shared with the launcher (Inv. §5 Settings) | Mac ordering and search; the compositor owns settings (B10) | None |
| **Lock and login**: lock screen with clock, password, Touch ID; Ctrl+Cmd+Q locks; login window with user list | Session lock (ext-session-lock), fingerprint beside the password, greeter on greetd with its own compositor (Inv. §4 Lock, §12, D15, D33, D34, D43) | Compositor-owned lock (B12) and a greeter mode of the same compositor | Locker death strands the session (D15, D43): owning the lock in the compositor removes the separate locker |
| **Touch ID**: unlock, sudo, app authorisation, keychain | fprintd on lock and greeter; optional sudo PAM (Inv. §12 fingerprint, D33) | Fingerprint in the lock, polkit dialog and keyring prompter (B12) | *Reasoning:* the login keyring is unlocked from the password PAM sees; a fingerprint login gives it none, so the keyring prompt follows. No secure enclave |
| **Continuity**: Handoff, Universal Clipboard, AirDrop, iPhone Mirroring, calls and messages | KDE Connect phone provider (Inv. §8) | KDE Connect through the provider contract | iPhone has no equivalent; Android through KDE Connect only |
| **Focus / Do Not Disturb**: modes, schedules, allowed apps and people | DND, quiet hours, fullscreen and game silencing, per-app mute and allow lists (Inv. §4 Notifications); no focus mode built (Inv. §2.2) | Named Focus modes with schedules; game mode as one of them (Inv. §10) | None |
| **Login Items & Extensions**: open at login, allow in background | Settings › Apps: XDG autostart, user services, compositor hook, session restore (Inv. §5, §3.10, D53) | One Login Items list over XDG autostart and systemd user units | Background-running apps have no shared signal (macOS 27 shows them in the Dock) |
| **Force Quit**: Cmd+Option+Esc | Monitor on Ctrl+Shift+Esc (Inv. §5 Monitor) | A Force Quit dialog listing apps | None |
| **Window controls**: traffic lights on the left; "Window title bar double-click action" | hyprbars on the right: close, fullscreen, minimise; no bar over CSD clients (Inv. §3.5, D48) | Server-side bars with left traffic lights | CSD apps (GTK4/libadwaita) draw their own buttons; GTK's `gtk-decoration-layout` setting can move them left but not restyle them |
| **Emoji & Symbols**: Ctrl+Cmd+Space | Super+. emoji in the launcher (Inv. §4, §11) | Ctrl+Cmd+Space | None |
| **Picture in Picture** | Rule for "Picture-in-Picture" titled windows, kept bottom right (Inv. §3.3) | Part of the window model (B8) | Detection by title |
| **Zoom (accessibility)** | Magnifier on Super+Ctrl+=/-/0 (Inv. §3.9) | Keep | None |

**Notes.**

- **The menu bar is the biggest gap and the least in Isle's control.** Isle
  can serve every protocol in §1.3 and still show only a fallback menu for
  libadwaita apps, Electron on Wayland until PR #54629 ships, Firefox unless
  the user flips two prefs, Steam and every game. B18's fallback menu is
  therefore the common case, not the exception.
- **Quit semantics need a decision**, since B20's "closing the last window
  does not quit" cannot be imposed on apps that exit by themselves. v1's
  hide-instead-of-close is the only lever.
- **Fn is gone.** macOS's tiling chords use Fn (Globe), which Linux mostly
  never sees. Choose Fn-free tiling chords and say so in Settings.
- **Ctrl means two things on a Mac** (Spaces and Mission Control at desktop
  level, emacs motion in text). The compositor can keep both because it
  knows which physical key is down; a uinput remapper could not.
- **v1 already holds most of the content** (Overview, capture, Files,
  notifications, lock, fingerprint, providers); the new work is the menu
  bar, the Dock, app-centric semantics, Split View, Stage Manager, hot
  corners and the Mac chords throughout.

---

## OPEN

- **Qt 5 and Qt 6 before 6.9 on Wayland through the KDE platform theme**
  (plasma-integration): believed to export through KDE's own protocol code;
  not read this session.
- **Electron**: which release will carry PR #54629, and whether the PID
  fallback in §1.3 recovers today's Electron menus in practice.
- **JetBrains built-in global menu under WLToolkit** (2026.1+): whether it
  does anything on Wayland without the third-party plugin.
- **Steam client**: no evidence either way that it exports a menu.
- **Budgie** (Wayland on labwc): current global menu status not checked.
- **GTK and `gtk_shell1` v5**: whether GTK 4 hands title-bar double, middle
  and right clicks to the compositor (`titlebar_gesture`) when the
  compositor advertises v5, and therefore must implement them. Whether GTK 4
  apps without a menubar export an empty `menubar_path` model.
- **Flatpak and PID matching**: whether `GetConnectionUnixProcessID` and the
  Wayland client PID agree for sandboxed apps.
- **Firefox's runtime libdbusmenu dependency** is taken from a nixpkgs PR
  title, not from Firefox's source.
- **appmenu-gtk-module crash** with hybrid Chromium apps is one report on one
  distribution.
- **Apple Fn**: hid-apple maps the Fn usage to `KEY_FN` and tracks `fn_on`
  (`drivers/hid/hid-apple.c`, `apple_input_mapping` and `hidinput_apple_event`,
  Linux master read 2026-10-02); whether the `KEY_FN` event itself reaches
  userspace was not verified. Keychron boards in Mac mode were not tested.
- **Xwayland modifier handling**: the claim that synthetic modifier key events
  are needed in addition to the `modifiers` event is reasoning; verify in the
  VM with an X11 app.
- **text-input-v3 as a "text field focused" signal**: reliability per
  toolkit not measured.
- **kitty's Shift+Insert** binding (primary selection) is from memory, not
  read this session.
- **Edge-swipe gestures**: whether libinput or Smithay exposes enough to
  detect a swipe starting at a screen edge.
- **File indexing for Spotlight**: no comparison of indexers (localsearch,
  plocate, an own index) was made.
- **macOS 27 details** beyond the Apple support pages (Desktop & Dock
  options, tiling shortcuts, keyboard shortcuts, screenshots, Mission
  Control) come from MacRumors and AppleInsider summaries; the Dock's
  background-process indicator and live widgets were not seen first-hand.
