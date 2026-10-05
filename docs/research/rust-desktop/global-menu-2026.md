# The global menu, rechecked for P6

Read 2026-10-05 against source, to plan P6. It checks `macos-model.md` §1
(researched 2026-10-02), which mostly holds, and adds what P6 needs to
build the menu sources. Package versions are the host's.

## The sources

**DBusMenu through the registrar.** On Wayland a client calls
`org_kde_kwin_appmenu_manager.create(surface)` then
`set_address(service, path)`; the protocol is at v2, which adds only
`release`. On X11 the app calls `RegisterWindow(xid, path)` and the service
is the sender. Qt's xcb plugin also sets `_KDE_NET_WM_APPMENU_SERVICE_NAME`
and `_KDE_NET_WM_APPMENU_OBJECT_PATH` itself (`qxcbintegration.cpp`), so
the window manager can find a Qt X11 menu from properties alone; since
late 2024 plasma-integration leaves this to Qt. Apps send
`ItemActivationRequested(id, timestamp)` when Alt and a mnemonic are
pressed, asking the desktop to open that menu; Plasma's kded module
listens for it ([appmenu.cpp](https://github.com/KDE/plasma-workspace/blob/master/appmenu/appmenu.cpp)).

**GTK's `org.gtk.Menus` and `org.gtk.Actions`.** GTK 3
(`gdk_wayland_window_set_dbus_properties_libgtk_only`) and GTK 4
(`gdk_wayland_toplevel_set_dbus_properties`) pass the app id, app-menu,
menubar, window and application paths and the unique bus name through
`gtk_surface1.set_dbus_properties` at realize, once `gtk_shell1` is bound;
GTK hides its own menubar when `capabilities` carries `global_menu_bar`.
`gtk-shell.xml` on GTK's main branch is at v7. From v5 GTK 4 hands
double, right and middle clicks on a header bar to the compositor
(`gdk_wayland_toplevel_titlebar_gesture`, `gtkwindowhandle.c`): a
compositor that binds v5 and ignores the request breaks double-click to
zoom. On X11 the same paths are `_GTK_*` properties.

- Action prefixes: `app.` resolves to the application path, `win.` to the
  window path, and `unity.` (appmenu-gtk-module's) to `_UNITY_OBJECT_PATH`
  on X11, or on Wayland to the menubar path, where the module exports the
  model and its actions together. KDE's
  [gmenu-dbusmenu-proxy](https://github.com/KDE/plasma-workspace/blob/master/gmenu-dbusmenu-proxy/window.cpp)
  resolves them the same way.
- Lazy menus: an item's `submenu-action`, a stateful boolean, is set true
  before its submenu opens and false after; LibreOffice fills menus this
  way.
- Wire: `Start(au) → a(uuaa{sv})`, paired with `End(au)`;
  `Changed(a(uuuuaa{sv}))`; links `:section` and `:submenu` as `(uu)`;
  attributes `label`, `action`, `target`, `accel` (`<Primary>c`), `icon`,
  `hidden-when`, `x-canonical-accel`. Actions: `DescribeAll → a{s(bgav)}`,
  `Activate(s, av, a{sv})`, `SetState`, `Changed(as, a{sb}, a{sv},
  a{s(bgav)})`.
- appmenu-gtk-module (`extra/appmenu-gtk-module` 25.04, GTK 3 only) works
  on Wayland with `gtk_shell1`; its `run-on-wayland` setting now defaults
  to true, which corrects §1.2. Its user unit is wanted only by Xfce, MATE
  and GNOME session targets, so nothing loads it under Isle by itself.

**`xdg_dbus_annotation_v1`** ([wayland-protocols MR 493](https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/493))
is still open ("Needs acks", updated 2026-08-24), implemented in KWin, Qt
and GTK. niri waits on it for accessibility, not menus. Not a menu source
yet.

## The apps

| App | Exports |
|---|---|
| Chromium 153 | Yes, through `org_kde_kwin_appmenu`; it watches the registrar name and registers again when the owner changes |
| VS Code 1.138 | Nothing by default: `titleBarStyle` is `custom` on every system and no native menu is installed off macOS ([window.ts](https://github.com/microsoft/vscode/blob/main/src/vs/platform/window/common/window.ts)). With `native` it takes Electron's broken path. This corrects §1.2 |
| Electron apps | Broken on Wayland; [PR #54629](https://github.com/electron/electron/pull/54629) is still unmerged. Electron re-checks the registrar on every `SetMenu`. `ELECTRON_FORCE_WINDOW_MENU_BAR=1` keeps the menu in the window |
| Firefox | Off unless the two prefs are set; not installed on the host |
| kitty 0.49 | None (a Cocoa menu only) |
| GTK 4, libadwaita (4.22.5, 1.9.4) | Only apps that call `gtk_application_set_menubar`, which few do |
| Qt 6.11 widget apps with a `QMenuBar` | Yes. Qt Quick apps without one: none |
| Qt 5 | None on Wayland (no plasma-integration for Qt 5 on the host) |
| Steam | None; its menu row is web UI. Its tray item has a DBusMenu |
| LibreOffice | GTK 3 VCL through `gtk_shell1`; Qt 6 VCL through DBusMenu. Not installed |
| JetBrains IDEs | Built-in menu on X11 only; the Global Menu plugin on Wayland. Not installed |
| Lutris | None |

## Serving the protocols

- Smithay (the fork at `764ad1b`) has neither protocol.
  [`wayland-protocols-plasma`](https://crates.io/crates/wayland-protocols-plasma)
  0.3.12 has a `server` feature with `appmenu` at v2, on the wayland-server
  and wayland-backend versions Smithay uses.
- `gtk_shell1` has no wayland-rs crate; bindings are generated with
  `wayland_scanner` from GTK's XML (LGPL; the XML is the interface, not
  code taken into Isle).
- [srdwm](https://github.com/srdusr/srdwm) (MIT, Smithay) serves both,
  advertising `gtk_shell1` v7 with a `titlebar_gesture` that does nothing:
  the trap above. niri's [PR #2794](https://github.com/niri-wm/niri/pull/2794)
  served the KDE protocol and was closed unmerged on 2026-04-16, its author
  noting that clients hide their own menus once it is advertised.
  cosmic-comp has nothing ([#2876](https://github.com/pop-os/cosmic-comp/issues/2876)).
  KWin is still the only major compositor serving it.
- `X11Surface` exposes fixed properties only; `_GTK_*`,
  `_UNITY_OBJECT_PATH` and `_KDE_NET_WM_APPMENU_*` with their
  PropertyNotify updates need a fork patch as `STEAM_GAME` did.

## Clients in Rust

- [`system-tray`](https://github.com/JakeStanger/system-tray) 0.8.9 (zbus 5)
  reads a status item's DBusMenu: one `GetLayout(0, -1, [])`,
  `LayoutUpdated` debounced 50 ms then `GetLayout(parent, -1)`,
  `ItemsPropertiesUpdated` applied as diffs. Its proxy is private, it
  watches only status items, a `LayoutUpdated` for a non-zero parent
  replaces the whole tree with the subtree, and the first timeout ends its
  watch loop. Its property parsing (`shortcut`, `toggle-type`,
  `icon-data`, `disposition`) is a reference; the proxy is Isle's own.
- [cosmic-macos-applets](https://github.com/cmjcode/cosmic-macos-applets)
  (GPL-3.0, read, not copied) sends `AboutToShow(0)` before `GetLayout`,
  and `AboutToShow` with `Event("opened")` before a submenu, each limited to
  2 s; its tests run a private `dbus-daemon`, as Isle's do.
- No Rust GMenuModel client exists.

**Pitfalls, from the exporters' source.**

1. Submenus fill differently: Qt on `AboutToShow` (and `Event "closed"` is
   its hide), always answering false; Chromium on `MenuWillShow`, also
   false; Firefox only on `Event "opened"`, needing a `"closed"` after. So
   send both `AboutToShow` and `opened`, and fetch again whatever the
   answer.
2. Electron rebuilds the tree with new ids on every `SetMenu`: an item is
   found again by its label path, not kept by id.
3. Shortcuts come in three forms: Chromium's keysym names
   (`[["Control","c"]]`), Qt's portable text (`C`, `Del`, `PgDown`, `plus`)
   in its own modifier order, GTK's `<Primary>c`. Each inner array of a
   DBusMenu shortcut is one chord of a sequence.
4. `icon-data` is PNG bytes.
5. `toggle-state` −1 is mixed; consecutive radio items are one group.
6. Properties at their default are omitted and must be filled in.
7. Labels mark mnemonics with `_`, a literal underscore with `__`.

## Searching menus

macOS searches the tree the app holds in its own process. Unity 8's HUD
([DBusMenuCollector.cpp](https://github.com/unity8-team/hud/blob/master/service/DBusMenuCollector.cpp))
opened every enabled submenu with `aboutToShow`, skipped disabled items
and separators, stopped at 50 menus and closed them all again; Unity 7
ranked by use. Plasma 6's global menu has no search; plasma-hud and
kde-appmenu-hud read `GetLayout(0, -1, ["label", "children-display"])` and
fill empty submenus with `AboutToShow` and `GetLayout(id)`. Since a crawl
runs app code (Firefox's handlers, LibreOffice building menus), it should
run when search opens, capped by depth and count, close what it opened,
and find a chosen item again by its path before activating it.

## Owning the registrar

Qt caches `NameHasOwner` in a function-local static
(`QGenericUnixTheme::isDBusGlobalMenuAvailable`), and `NameHasOwner` does
not activate a service, so the name must be owned before any Qt app
starts. Chromium and LibreOffice watch the name; Electron re-checks it on
every `SetMenu`. Plasma's kded registrar claims the name only when the
Global Menu applet registers `org.kde.kappmenuview`, without queueing or
replacement, so it cannot take it from an owner. vala-panel's registrar is
D-Bus activatable, so an unowned name and any call to it would start it;
it is not installed here. Request the name with `DoNotQueue` and without
`AllowReplacement` before the first client; an `Exists` reply is a
conflict to report.

## Environment

Qt 6.9+ and Chromium need nothing beyond the registrar and the protocol;
every Qt theme derived from `QGenericUnixTheme` exports (generic, gtk3,
which the host sets, qt6ct, kde). GTK 3 menubars need `GTK_MODULES=
appmenu-gtk-module` per launch, `UBUNTU_MENUPROXY` unset or true. Firefox
needs its prefs and `libdbusmenu-gtk3`, which is installed. The host sets
`ELECTRON_OZONE_PLATFORM_HINT=auto`, so Electron apps run on Wayland.
