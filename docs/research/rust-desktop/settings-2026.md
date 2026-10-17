# Settings and the first app, 2026

Evidence for P13 (`docs/work/rust-desktop/p13-settings.md`): what macOS 26
and 27 put in the System Settings panes P13 builds, and what a standalone
Freya app on Wayland needs to follow `apps.md`'s Conventions. Read
2026-10-17. Sections are cited from the plan as §n.

## 1. macOS's panes

From Apple's Mac User Guide, both `/26/mac/26` and `/27/mac/27` pages
fetched and diffed; labels are Apple's. Control kinds follow Apple's verbs
("turn on" a switch, "click the pop-up menu" a pop-up); *inferred* where it
does not say. 27 changed nothing in a pane unless noted.

**Wi-Fi.** Wi-Fi (switch); the joined network with Details…; Personal
Hotspots; Known Networks, each with a ⋯ menu (Auto-Join, Copy Password,
Network Settings, Forget This Network); Other Networks with Other… (name,
security, password); Ask to join networks (Off, Notify, Ask); Ask to join
hotspots; Advanced… (admin rights to change networks, legacy networks, the
MAC, Known Networks again). Details… sheet: Automatically join this
network, Private Wi-Fi address, Wi-Fi address, Low data mode, Limit IP
address tracking, IP address and Router read only, Forget This Network…,
then tabs TCP/IP, DNS, WINS, 802.1X, Proxies, Hardware; Cancel and OK.

**Bluetooth.** Bluetooth (switch); My Devices (Connect or Disconnect on
hover, Forget on secondary click; an ⓘ with Rename and Forget This Device…
is in the UI but not the guide); Nearby Devices (Connect, then the pairing
prompt).

**Network.** Wi-Fi, VPN and Firewall as sub-pages; the service list with
status dots (secondary click: Duplicate, Delete, Make Inactive, Set Service
Order); ⋯ with Add Service…, Add VPN Configuration, Virtual Interfaces,
Locations. A service's page: status and its settings, Details…, Delete,
Make Inactive. Ethernet Details…: Name, Limit IP address tracking, the tabs
as Wi-Fi's, Configure IPv4 (Using DHCP, DHCP with manual address,
Manually).

**Displays.** The arrangement (drag to place; Option-drag to mirror; drag
the menu bar to make a display main; only with two or more), an Add
Display pop-up. Per display, in order: Use as (Main, Extended, Mirror for
…); Optimize for (27: mirroring only); Resolution (thumbnails Larger Text …
More Space on Retina, Show all resolutions); Brightness (slider);
Automatically adjust brightness; True Tone; Color profile; When connected
to TV; Preset; Rotation (Standard, 90°, 180°, 270°, confirmed in a dialog);
Refresh rate (fixed rates, ProMotion or Adaptive Sync where offered); High
Dynamic Range (third-party HDR10 displays). Buttons: Advanced… (show
resolutions as a list, Universal Control), Night Shift… (Option turns it
into Detect Displays). Night Shift… sheet: Schedule (Off, Custom with From
and To, Sunset to Sunrise), Turn on until tomorrow, Color temperature (Less
Warm to More Warm). An unsupported resolution reverts after 15 seconds or
on Escape; Apple documents a confirm only for rotation.

**Sound.** Sound Effects: Alert sound (pop-up, previews), Play sound
effects through, Alert volume (slider, relative to output), Play sound on
startup, Play user interface sound effects, Play feedback when volume is
changed. Output & Input as tabs: a device table; Output volume with Mute,
Balance; Input volume, Input level (a meter; opening the tab lights the
microphone indicator).

**Keyboard.** Key repeat rate (Off … Slow to Fast), Delay until repeat
(Long to Short), backlight rows, Press fn key to, Keyboard navigation,
Keyboard Shortcuts…. The Shortcuts sheet: categories down the side, a
checkbox list of shortcuts, double-click to record, ⚠ on a conflict,
Restore Defaults and Done; App Shortcuts (+ − with Application, Menu title,
Keyboard shortcut); Modifier Keys (a keyboard pop-up, then a pop-up each for
Caps Lock, Control, Option, Command and fn: another modifier, Escape or No
Action). Text Input: Input Sources with Edit… (a sheet: sources down the
side with + −, then typing switches), Text Replacements…. Dictation.

**Appearance.** Appearance (Auto, Light, Dark as pictures); Liquid Glass
(26: Clear or Tinted; 27: a slider from clear to fully tinted); Color
(accent swatches); Text highlight color; Icon & widget style; Folder color;
Sidebar icon size; Tint window background with wallpaper colour (27's
name); Show scroll bars; Click in the scroll bar to. Reduce Transparency is
Accessibility's.

**Desktop & Dock**, the Dock's group aside. Window title bar double-click
action (Fill, Zoom, Minimize, No Action) sits in the Dock group. Desktop &
Stage Manager: Show Items, Show Desktop (Always on Wallpaper Click, Only in
Stage Manager), Stage Manager with Customize…. Widgets: Show Widgets, Dim
widgets on desktop, iPhone Widgets, Default web browser. Windows: Prefer
tabs, Ask to keep changes, Close windows when quitting an application, Drag
windows to left or right edge of screen to tile, Drag windows to menu bar
to fill screen, Hold ⌥ key while dragging windows to tile, Tiled windows
have margins. Mission Control: rearrange Spaces by recent use, switch to a
Space with the app's windows, Group windows by application, Displays have
separate Spaces, Drag windows to top of screen to enter Mission Control,
Shortcuts…, Hot Corners… (a pop-up per corner: Mission Control, Application
Windows, Desktop, Notification Center, Apps, Quick Note, screen saver,
Put Display to Sleep, Lock Screen, "-"; hold a modifier while the menu is
open to require it; Done).

**Notifications.** Show previews (Always, When Unlocked, Never); Show
Notifications when display is sleeping, when screen is locked, when
mirroring or sharing the display; Application Notifications, a list with
each app's settings summarised under its name. An app's page (redesigned in
26): Allow notifications; Alert Style as where (Desktop, Notification
Center, Lock Screen) and style (Temporary, Persistent); Allow critical
alerts; Allow time sensitive alerts; Badge application icon; Play sound for
notification; Show previews; Notification grouping.

**Menu Bar (26 and 27; Control Center in 15).** Automatically hide and show
the menu bar; Show menu bar background; Recent items. Menu Bar Controls:
Clock (Clock Options…), switches for each item, and for Focus, Display,
Sound, Now Playing a pop-up Show When Active or Always Show. Allow in the
Menu Bar: third-party items, a switch each. Reset Control Center…. Control
Center itself is edited in place (Edit Controls, a gallery).

**Focus.** The mode list with Add Focus… (provided or Custom with name,
colour, icon); Share across devices; Focus status. A mode's page: Allowed
People, Allowed Apps (Allow Some or Silence Some, + Add, Time Sensitive
Notifications), Set a Schedule (Add Schedule…: Time, Location, App), Focus
Filters, Delete Focus….

**Spotlight.** 26: Show Related Content; Reset Quick Keys…; Delete Search
History…; Help Apple Improve Search; Results from Apps (a switch per app);
Results from System (a switch per category); Clipboard Search; Search
Privacy… (a + − list of excluded folders). 27: Results from Clipboard;
Clipboard history is available in Spotlight (30 minutes, 8 hours, 7 days);
Clear Clipboard History….

**Privacy & Security.** One sub-page per kind of access (Location, Files &
Folders, Full Disk Access, Accessibility, Bluetooth, Camera, Input
Monitoring, Microphone, Screen & System Audio Recording and the rest);
apps appear once they have asked, each with a switch that revokes it; kinds
an app can be added to have + (and −) under the list. Screen & System Audio
Recording has two lists, screen and audio, and audio only.

## 2. A global menu from a Freya app

- No Rust crate is a ready DBusMenu server for a window. `dbusmenu-zbus`
  is client proxies only; `dbusmenu-glib` binds the C library. `ksni`
  0.3.6 (zbus 5) has a full `com.canonical.dbusmenu` server (GetLayout,
  Event, EventGroup, AboutToShow, LayoutUpdated, ItemsPropertiesUpdated)
  tied to its tray service; its shape is the one to follow. muda's Linux
  backend builds a GTK menubar and does nothing for a winit window; strata
  draws its own menu bar for that reason (strata `docs/PLATFORMS.md`).
- `org_kde_kwin_appmenu_manager` v2: `create(id, wl_surface)`, then
  `set_address(service, path)` once the object is exported. Qt does it in
  qtbase (`qwaylandappmenu.cpp`), exporting `/MenuBar/N` and giving the
  connection's unique name. Client bindings: `wayland-protocols-plasma`
  `appmenu`. Isle serves the global and the registrar already
  (`menus/protocols.rs`, `isle-menus/src/registrar.rs`).
- winit 0.30.13 gives the display and surface pointers through
  raw-window-handle 0.6. `wayland_backend::client::Backend::from_foreign_display`
  (feature `client_system`) and `ObjectId::from_ptr(WlSurface::interface(), ptr)`
  let an app bind any global on its own event queue against the window's
  surface, without destroying the borrowed proxy. Freya reaches the window
  through `WindowConfig::with_window_handle` and `Platform::with_window`.

## 3. One instance, raised properly

- The desktop-entry D-Bus spec: the bus name is the desktop file id, the
  object path the id with dots as slashes and `-` as `_`;
  `org.freedesktop.Application` has `Activate(a{sv})`, `Open(as, a{sv})`
  and `ActivateAction(s, av, a{sv})`; platform data carries
  `activation-token` and `desktop-startup-id`. `DBusActivatable=true`
  tells launchers to call it rather than run Exec; the service file is
  `dbus-1/services/<id>.service`. GApplication requests its name with
  `DO_NOT_QUEUE` and, finding it taken, forwards to the running instance
  and exits. Isle's `launch.rs` already calls Activate, Open and
  ActivateAction with both platform-data keys for such entries.
- winit 0.30.13: `read_token_from_env`, `with_activation_token` (activates
  at creation) and `request_activation_token`. `focus_window()` is empty on
  Wayland in 0.30.13 and 0.31's beta, so a running window cannot be raised
  with a token handed to it. Either a winit patch (it already holds the
  `xdg_activation_v1` global) or the app binds `xdg_activation_v1` itself
  through §2's route. Freya wraps none of these.

## 4. Client-side decorations

- `with_decorations(false)` creates the toplevel asking xdg-decoration for
  client mode and asks again if told server mode. Isle grants a client's
  request when title bars are on (`handlers/mod.rs`).
- `drag_window()` and `drag_resize_window()` send `xdg_toplevel.move` and
  `resize` with the last button serial, so they must be called while the
  button is down. Freya's `window_drag()` starts on the first move after a
  press and toggles maximise on a double press, which an Isle app must
  replace with `windows.titlebar-action`. `freya-borderless-plugin` gives
  resize bands. `show_window_menu` exists.
- winit sets the window geometry to the whole surface, so a client-drawn
  shadow counts as window. Isle draws window shadows itself.

## 5. ext-background-effect-v1

- In wayland-protocols staging since 1.45 (2025), still staging at 1.49;
  Rust `wayland-protocols` 0.32.13 `ext::background_effect::v1` under
  `staging`. Smithay's server side merged 2026-02-04 (#1860); niri, Mutter
  and Quickshell support it.
- The client binds the manager, waits for `capabilities` (blur = 1), sends
  `get_background_effect(surface)` and `set_blur_region(region)`, surface
  local and double-buffered on the next commit; a null region removes it.
- Isle's code does not serve it (no use in `crates/`); one headless run
  logged the global, which the phase that serves it confirms or corrects.

## 6. Who owns a display change's revert

- Mutter owns it. `ApplyMonitorsConfig` takes VERIFY, TEMPORARY or
  PERSISTENT; PERSISTENT starts mutter's own 20-second timer and asks
  gnome-shell (the compositor process) to show the confirm; `monitors.xml`
  is written only on confirm. gnome-control-center is a caller: if it dies
  mid-countdown the change still reverts (`meta-monitor-manager.c`,
  `windowManager.js`).
- KDE's kscreen KCM runs a 15-second QML timer itself, and KWin stores the
  new configuration when it applies it. If System Settings dies during the
  countdown nothing reverts and the change is kept (`kcm/ui/main.qml`,
  `kwin/src/workspace.cpp`).
- So the timer belongs in the compositor: apply as temporary, keep the
  previous configuration, persist only on Keep, revert on the deadline or
  when the asking client goes.

## 7. Measuring an app's memory on this host

RTX 5070 Ti, NVIDIA open module 615.71.09, kernel 7.2.8.

- PSS from `/proc/<pid>/smaps_rollup`, with `SwapPss` (the host swaps) and
  `Pss_Anon` and `Pss_File` apart.
- `nvidia-smi --query-compute-apps` lists only compute and C+G processes;
  an OpenGL Freya window is G and absent (a Vulkan one showed as C+G).
  `nvidia-smi pmon -c 1 -s m` (an `fb` column per pid) and NVML's
  `running_graphics_processes` list graphics processes without root.
- DRM fdinfo has no `drm-*` keys on NVIDIA (allocations go through the
  `/dev/nvidia*` fds), and `/proc/driver/nvidia` has nothing per process.

## 8. Freya's software renderer

- The fork's base (0.5.0-rc.8) has one: `freya-winit/src/drivers/software.rs`,
  Skia raster into softbuffer 0.4.8's buffer (wl_shm on Wayland), a full
  frame per present. Chosen by `WindowConfig::with_renderer(RendererPreference::Software)`
  or `FREYA_RENDERER=software`; Linux otherwise tries Vulkan, then GL, then
  software. A build with `default-features = false, features = ["winit"]`
  leaves the GPU crates out (upstream #2081).
- One debug sample each, `isle-preview gallery` on a headless Isle, idle
  30 s:

| Renderer | PSS (RSS) | Pss_Anon | VRAM | NVIDIA lib mappings |
|---|---|---|---|---|
| Vulkan | 141.6 MB (223 MB) | 54.1 MB | 31 MiB | 83 |
| OpenGL | 131.9 MB (212 MB) | 85.0 MB | 26 MiB | 72 |
| Software | 55.0 MB (101 MB) | 17.6 MB | 0 | 30 (built with `gpu`) |

- Against `apps.md` bar 2 (80 MB PSS) only the software path passes. Its
  cost moves to the compositor, which uploads the shm buffer, and to the
  app's CPU per repaint. Debug build, one sample, not the Settings window:
  P13's measure is the one that counts.

## Sources

Apple Mac User Guide, 26 and 27 (support.apple.com/guide/mac-help/…):
Wi-Fi mh11935, mchl339ee00e; Bluetooth blth8111, blth1004; Network
mchlee7b367f, mh11939; Displays mh40768, mchl86d72b76, mchlb5f905a1,
mh11534, mchl97bc676d, mchl9d976e85; Sound mchl9777ee30, mchlp2207,
mchlp2256, mchlp2567; Keyboard kbdm162, mchl0311bdb4, mchl84525d76,
mchlp2262, mchlp2271, mchlp2864, mchlp1011; Appearance mchlp1225; Desktop &
Dock mchlp1119, mchl118087b0, mchlp3000; Notifications mh40583,
mchl205da693; Menu Bar mchlad96d366; Focus mchlff5da36d, mchl613dc43f;
Spotlight mchl54d95e8a, mchl3e00eae9, mchl1bb43b84; Privacy & Security
mchl211c911f, mchld6aa7d23; What's new in 27 apd07d671600.
ksni https://github.com/iovxw/ksni; dbusmenu-zbus https://docs.rs/dbusmenu-zbus;
kde-appmenu https://wayland.app/protocols/kde-appmenu; qtbase
`src/plugins/platforms/wayland/qwaylandplatformservices.cpp`;
desktop-entry D-Bus https://specifications.freedesktop.org/desktop-entry/latest/dbus.html;
glib `gio/gapplicationimpl-dbus.c`; ext-background-effect
https://phoronix.com/news/Wayland-Protocols-1.45; mutter
`src/backends/meta-monitor-manager.c`; kwin `src/workspace.cpp`; kscreen
`kcm/ui/main.qml`; DRM usage stats https://docs.kernel.org/gpu/drm-usage-stats.html;
winit 0.30.13 and the Freya fork at `a8d7a03`, read locally.
