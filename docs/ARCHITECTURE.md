# Architecture

How the pieces fit, what drives each feature, how the two keyboards work,
and the order it gets built in. The technology choices and their
alternatives are in `STACK.md`.

## Process layout

```
Hyprland
 └── isle (Quickshell)            started by hyprland.lua's start hook through
      │                           shell/scripts/isle-session, which restarts it (D17)
      ├── services/               singletons: state + side effects
      ├── surfaces/               layer-shell: island + control panel, dashboard,
      │                           launcher, switcher, overview, capture, lock, auth, power
      ├── widgets/                dashboard widgets: one directory each, see Widgets
      ├── windows/                Settings, Monitor, Keychain, Big Picture
      └── theme/                  tokens.json → Theme singleton
 xremap                           systemd --user, reads input/<profile>.yml
 gnome-keyring-daemon, gamemoded, power-profiles-daemon, udisks2, NetworkManager,
 bluez, pipewire, wireplumber, xdg-desktop-portal-*
```

Everything the user sees is one Quickshell process, running inside the
compositor's logind session so polkit treats it as the active session. A
crash restarts it in about a second and the island reappears; nothing is
lost because the state that matters lives in the daemons, not the shell.
Its log is `$XDG_RUNTIME_DIR/isle.log`.

The compositor is reached only through `services/Compositor.qml`
(workspaces, windows, focus, monitors, keywords). Nothing else imports the
Hyprland module, so the runner-up compositor is a second implementation of
one file.

## Repository

```
shell/        the Quickshell config (~/.config/quickshell/isle links to the installed copy's)
  shell.qml
  theme/      tokens.json, Theme.qml
  ui/         the component kit
  services/   Audio, Network, Bluetooth, Notifications, Media, Power, Idle,
              Compositor, Windows, Modes, Prefs, Clipboard, Capture, Disks,
              Monitor, Keychain, Keyboard, Games
  surfaces/   one directory per surface
  widgets/    one directory per widget: widget.json + Widget.qml
  windows/    Settings, Monitor, Keychain, BigPicture
  assets/     Lucide glyphs, default wallpaper
hypr/         hyprland.lua and the fragments the shell writes (binds, monitors, modes, theme)
input/        xremap profiles: mac.yml, windows.yml
theme/        templates: gtk3, gtk4, qt6ct, kitty, yazi, btop, zathura, hypr
packages/     pacman and AUR lists
tools/        bootstrap, install, the greeter copy, icon vendoring
docs/
```

The desktop runs from an installed copy at `~/.local/share/isle`.
`tools/install.sh` run from a checkout elsewhere copies it there (rsync,
keeping the copy's `hypr/generated` and built plugins) and links
`~/.config` into the copy; run from the copy, as Settings › Updates and
bootstrap do, it installs in place. The test VM (local tooling under `dev/`)
mounts the checkout itself at `/repo`, so a change is tried there before
it is installed (D21).

## Services

Every capability and what drives it. "native" is a Quickshell type; no
extra process.

| capability | driven by |
|---|---|
| workspaces, windows, focus, fullscreen, monitors | native Hyprland IPC and Wayland toplevel management |
| audio: sinks, sources, streams, volume, mute, default device | native Pipewire |
| network: wifi list, connect, ethernet, connectivity | native Networking (NetworkManager) |
| bluetooth: adapters, devices, pair, connect, battery | native Bluetooth (BlueZ) |
| notifications: server, actions, inline reply, images, DND | native Notifications. `Notifications` keeps every one until dismissed and logs each arrival as a record to `~/.config/isle/notifications.json` (300 kept), so the history survives a restart as "Earlier" entries without actions. One `NotificationCard` draws them everywhere: the island's centre (a panel page, from the bell, the tile or the summary), the dashboard widget, and the toast. A toast is silenced by do not disturb, quiet hours, a fullscreen window (the compositor's `fullscreen` event), or game mode; critical ones and apps in `dndAllow` toast anyway, low-urgency ones never do; what was silenced is counted and, when silence lifts, summarised with a way to the centre. A toast lasts as long as the app's `expireTimeout` asks, within 3 to 15 s; an optional sound plays through pw-play |
| media: players, art, transport, seek | native Mpris |
| battery and power source (controllers, headsets) | native UPower |
| tray | native SystemTray; shown in the dashboard's Session widget |
| lock | native WlSessionLock + Pam |
| idle: dim, lock, screen off | native IdleNotify |
| polkit agent | native Polkit |
| thumbnails for switcher and overview | native Screencopy |
| global shortcuts from the shell | native Hyprland GlobalShortcut |
| greeter | native Greetd. It runs as the `greeter` user before any login, so it cannot live in a home: `tools/install-greeter.sh` keeps a root-owned copy of `greeter/` and `shell/` under `/usr/local/share/isle-greeter` and points greetd at it; `install.sh` refreshes it after a pull |
| clipboard history | `wl-paste --watch cliphist store`; `cliphist list` / `decode` |
| capture | grim / slurp / wf-recorder / satty / hyprpicker, run as compositor children by `capture.sh` since slurp's and hyprpicker's overlays cannot map from a Quickshell child. A shot is frozen first: grim grabs every monitor before the pick, `FreezeFrame` shows those frames under slurp, and `crop.py` cuts the region out of them, so nothing after the key press and never the pick's own veil reaches the shot |
| removable media | `Disks` mounts a filesystem the moment it appears while `prefs.autoMount` is on (the default) and announces it; off, the announcement carries a Mount button |
| disks | `udisksctl monitor` and `lsblk` for the list; `udisksctl mount` / `unmount` / `power-off` from the session scope |
| keychain | The Secret portal is routed to gnome-keyring in `portals.conf`, so Chromium-based browsers and Electron apps keep the key for their saved logins in the same keyring (a 64-byte item per app id). libsecret through `shell/scripts/secrets.py` (list, reveal, delete, store, lock); PAM unlocks it at login with `tools/install.sh --pam`. Categories come from the item's schema and attributes; the generator is `python3 -c` over `secrets` |
| ssh keys | `shell/scripts/sshkeys.py` over `~/.ssh` (`ssh-keygen -lf` for type and fingerprint, `-y -P ""` to tell a locked key) and `ssh-add -l` / `ssh-add` / `ssh-add -d` against `SSH_AUTH_SOCK`. The agent is gcr's socket-activated wrapper at `$XDG_RUNTIME_DIR/gcr/ssh`, exported by `hyprland.lua` and enabled by install; it preloads `~/.ssh` and prompts for passphrases on use. A passphrase given in the Keychain reaches `ssh-add` through `askpass.sh` reading a one-shot 0600 file. |
| bluetooth pairing | `bluetoothctl` kept open as the agent (`agent KeyboardDisplay`, `default-agent`); its prompts are parsed and answered on stdin. The code shows in the island, the panel and Settings |
| drives | `udisksctl dump` through `shell/scripts/drives.py`: drives, block devices and the SMART summary udisks keeps (ATA and NVMe); `SmartUpdate` over gdbus to refresh |
| default apps | `xdg-mime query default` / `xdg-mime default` and `xdg-settings` for the browser, through `shell/scripts/defaults.py`; candidates from the `MimeType` lines of every desktop entry |
| startup | Settings › Apps shows the three ways a session starts things. Apps: XDG autostart through `scripts/autostart.py` (a user file with `Hidden=true` switches a system entry off; entries naming another desktop are folded as never running here). Services: the enabled systemd user units through `scripts/services.py` (`enable --now` / `disable --now`), with the shell's own locked. The compositor: the `hl.exec_cmd` lines of `hyprland.lua`'s start hook, read from the file |
| gamepad | `shell/scripts/gamepad.py` reads every evdev node with `BTN_GAMEPAD` (uaccess through the joystick udev rule) and prints presses; it runs only while Big Picture is up |
| shell updates | `IsleUpdate` fetches the checkout's origin hourly and lists the commits behind; Update pulls fast-forward in a terminal and runs `install.sh`, and Quickshell reloads from the changed files. An SSH remote the host does not know gets a row pointing at the key in Keychain; an https remote gets a gh sign-in. Settings › Updates puts it above the package list; the island's download glyph and the panel footer count cover both |
| title bars | the hyprbars plugin from this repo's `plugins/hyprbars` (upstream at the commit pinned for this Hyprland, plus one change: no bar when the client asked for its own decorations, through xdg-decoration, KDE's server-decoration or X11 Motif hints), built by hyprpm from `hyprpm.toml` at the root in bootstrap and loaded from hyprland.lua's start hook (then `hyprctl reload` so the theme fragment sees it). `theme/templates/hypr-theme.lua` sets its colours, font and padding from the tokens and adds three buttons right to left: close, fullscreen, minimise to the hidden stack (`switcher hide`). Off with the `titleBars` preference (Settings › Appearance); `titleBarsExcept` lists the window classes that draw their own controls, rendered as one anchored regex in a Lua long string into a `hyprbars:no_bar` rule |
| picture-in-picture | a window rule floats, pins and sizes a browser's PiP window without a border; the Pip service moves each new one to the bottom-right of its monitor, since the rule's own move is ignored on 0.56 |
| window tiling | The `plugins/isle-windows` compositor plugin. A drag in progress is the layout's own drag controller holding a target in move mode; the plugin hooks `CLayoutManager::endDragTarget`, where every drag ends whether it began with Super+drag, the hyprbar or the client's own title bar, and tiles the window to a half, quarter or the work area from the pointer's edge, the window sitting inside the tile by its decoration extents so a title bar clears the island. While the pointer is over an edge it posts the target box to the shell (`windows ghost`) and `SnapGhost` draws it. Keys and the bar's double click go through the plugin's hyprctl command, `hyprctl isle snap <zone>`, `zoom`, `restore`: the Lua config cannot call a plugin's dispatcher, but it can run a command. The same plugin answers `_NET_WM_MOVERESIZE`, which an X11 client sends when its own title bar is dragged or its edge pulled and which this Hyprland's XWM ignores, by beginning the layout's drag the way the compositor does for xdg_toplevel.move
| minimise | The compositor has none. The hyprbar's button and the switcher's Super+M run `hidewindow.py`, which parks the window on the `hidden` special workspace silently, closes that workspace if it came up, and focuses the top window left. A client's own minimise button is a request the compositor's state handler ignores (and, for an X11 client, answers with its sticky maximise toggle); the plugin hooks that handler, consumes the request and runs the same script. The switcher lists hidden windows and restores one onto the current workspace, focused and on top
| display arrangement | `DisplayMap` in `qs.ui` draws the compositor's monitors to scale; a drop snaps the moved one to the others' edges and pushes it out of any overlap, then `Displays.place` pins every monitor's position (`XxY`) and re-renders the fragment. Pending updates show as a glyph in the island's rest pill and a count in the panel footer |
| keymap overrides | `keymapOverrides` in prefs, action id to chord; the Keyboard service renders them into the same bind fragment and xremap config, then `hyprctl reload`. While a chord is recorded the compositor sits in the `isle-record` submap so Super chords reach the window |
| system stats | `/proc/stat`, `/proc/meminfo`, hwmon, `nvidia-smi --query-gpu`, LACT socket, CoolerControl REST on localhost:11987; every process from `/proc/<pid>` with CPU and disk I/O deltas; per-process network from `nethogs -t`, which needs `cap_net_admin,cap_net_raw` (install sets them). The Monitor's strips add the CPU split from `/proc/stat`, `/proc/meminfo`, whole-disk throughput from `/proc/diskstats`, RAPL package power when `energy_uj` is readable, battery draw, hwmon fans (`fan*_input`, `pwm*`), and `systemd-inhibit --list` for what holds off sleep |
| game detection | gamemode D-Bus `ClientCount`; fullscreen window whose parent is Steam, Heroic or Lutris |
| power profiles | `powerprofilesctl` |
| wallpaper | Settings › Wallpaper: `scripts/wallpapers.py` lists the images under Pictures, `~/.local/share/wallpapers`, the system's wallpaper folders and any folder added; a click sets `prefs.wallpaper`, and the `Wallpaper` surface's `Backdrop` follows it |
| devices | `scripts/devices.py` reads udev's database, lspci, sysfs, lsblk and lscpu into a tree of categories named the way Device Manager names them (Processors, Memory, Display adapters, Monitors, Sound controllers, Keyboards, Mice, Game controllers, Network adapters, Disk drives, Storage controllers, USB controllers and devices, Bluetooth, Cameras, Printers, System devices), each device with its properties: manufacturer, model, driver, location, hardware ids, serial. The `Devices` service runs it at start and again after a burst of `udevadm monitor` events; the Settings page adds the shell's own knowledge (monitors, audio endpoints, Bluetooth peers, batteries) and shows the tree beside a properties pane |
| brightness | `Brightness`: the backlight through brightnessctl on a laptop, else DDC/CI through ddcutil, surfaced as a slider on the Displays page |
| clipboard paste | A pick in the launcher's clipboard mode copies, waits for the launcher to go, then has the compositor send the paste chord to the window that had focus (`Windows.pasteCommand`): Ctrl+V, or Ctrl+Shift+V for a terminal by app id |
| Mission Control | `surfaces/overview/Overview.qml`, one layer surface on the shell screen. Windows come from `Windows.groups` (the compositor's toplevel list, refreshed on entry for positions and sizes); each is a live `ScreencopyView` of its toplevel, with a placeholder when the capture has no content. The spread is computed in QML from the real rectangles; thumbnails animate between the real and stage rectangles. Drops move windows with `window.move({ follow = false })`; closes go through the compositor by address (`Windows.closeWindow`), never the toplevel handle, which is a protocol error on a handle already going away. The three-finger swipe is `hl.gesture` in `hyprland.lua`, the island's workspace dots open it too |
| vpn | `Vpn`: Proton VPN through `protonvpn` (status, connect by country, disconnect, kill switch and NetShield from `config`), and NetworkManager's VPN and WireGuard connections through `nmcli`. Sign-in runs the CLI under `setsid` and feeds the password, then the 2FA code when its stderr asks, over stdin (D20); `ip monitor link` and `nmcli monitor` trigger the refreshes. The panel tile, its VPN page, the launcher's Connect/Disconnect VPN and Settings › Network all drive the one service |
| shortcuts in the launcher | `Launcher.chord(id)` puts the keyboard in use's chord beside a shell result that has a keymap id; every other keymap action is a "Shortcut" result run the same way the bind runs it (`qs ipc call`, `exec_cmd`, or the Lua dispatched), so the two never drift |
| shell windows | Settings, Keychain and Monitor are `FloatingWindow`s of the shell process (app id `org.quickshell`); `Surfaces.show(name)` opens one, or focuses it through the compositor by title when it is already open |
| keyboard configurators | `system/udev/70-isle-hidraw.rules` gives the logged-in user the hidraw nodes (what the VIA package ships), so Keychron Launcher, VIA and Vial reach the keyboard over WebHID; `tools/install.sh` installs it and a Keychron Launcher entry that opens the site as its own window through `shell/scripts/webapp.sh` in the first Chromium-family browser |
| night light | `hyprctl hyprsunset` |
| brightness | ddcutil for desktop monitors, brightnessctl for laptops |
| session: sleep, restart, shut down, log out | `systemctl`, `loginctl`, `hyprctl dispatch exit` |
| keyboard profile | writes `input/<profile>.yml`, restarts xremap |
| keyboard hardware | `scripts/keyboards.py` reads `/proc/bus/input/devices` (a keyboard is a device with the letter keys), udev for bus and vendor, and the key bitmap for a size guess, which a USB keyboard's descriptor usually defeats by claiming every key; `Keyboard.hardware` joins it to the compositor's device list by slug. The XKB model per keyboard is `prefs.keyboardModels`, guessed from the vendor (Apple) and the first layout (ISO or ANSI) until chosen; `Input.render` writes one `hl.device` line per interface with `kb_model`, and `numlock_by_default` |
| compositor settings | writes `hypr/generated/*.lua`, `hyprctl reload` |
| preferences | `~/.config/isle/prefs.json`, watched |

A missing daemon means the feature is absent, not broken: no toggle, no
glyph, and Settings names the package.

## Widgets

A widget is a directory holding a manifest and a component:

```
widgets/system/
  widget.json     { "id": "system", "name": "System", "glyph": "cpu",
                    "sizes": ["3x2", "6x2"], "default": "3x2", "requires": [],
                    "settings": [{ "key": "temps", "label": "Temperatures",
                                   "type": "toggle", "default": true }] }
  Widget.qml      WidgetBase { title: "System"; meta: "..."; /* content */ }
```

Rules of the contract:

- The root is `WidgetBase` from `qs.ui`. It carries `manifest`, `size`
  (`"3x2"`), `cols`, `rows`, `settings`, and the `title` and `meta` the
  card shows. The card draws the glass, title and meta; the widget draws
  only its content, filling its parent.
- `requires` names services or binaries (`"Pipewire"`, `"lact"`). A
  missing requirement hides the widget and tells the picker why.
- Widgets read the same service singletons the rest of the shell uses;
  none starts its own daemon. Services that poll (`System`, `Storage`)
  count `listeners`: a widget increments on completion and decrements on
  destruction, so nothing is sampled while the dashboard is closed.
- Built-in widgets live in `shell/widgets/<id>/` and load through the
  config's own URL scheme, which is what lets them import `qs.*`.
  Widgets in `~/.config/isle/widgets/<id>/` are the user's: a manifest
  with a `source` (a `command` run with sh, or a `file`, every
  `interval` seconds) and a `view` of text, number, gauge, sparkline or
  list is drawn by the built-in renderer in `shell/widgets/text/`, so it
  needs no code; one with its own `Widget.qml` is
  sandboxed: it is refused if it imports `qs.*` without `trust: true`,
  and is handed a permission-gated `host` wrapper instead of the services
  (D25). `trust: true` runs it with the shell's full reach through the
  `shell/userwidgets` symlink (D14). The library in the dashboard's edit mode
  writes and deletes the manifest kind: New text widget at its foot, edit
  and delete on hover over one of Yours.
- A manifest carries `category` (Shell, System, Hardware, Media; the
  user's are Yours), `description`, `sizes`, `default`, and `multiple`
  when more than one instance makes sense (Clock). The library in the
  dashboard's edit mode lists them by category with a search.
- `settings` declares what the card's gear offers in edit mode: `toggle`,
  `choice` (with `options: [[value, label]]`) or `number` (`min`, `max`).
  Values live in prefs under `widgetSettings.<id>` and reach the widget as
  `settings.<key>`, with the manifest default filled in.
- Pages are `dashboardPages: [{name, layout}]` in prefs with
  `dashboardPage` the shown one; the older single `dashboard` becomes the
  first page's layout. A layout is `[{key, id, x, y, w, h}]` on a
  12-column 4-row grid; the first page empty means the default layout.
  `key` names the instance (`clock`, then `clock#2`), `id` the widget;
  per-instance settings are stored under the key. Entries from before
  instances have no key and take their id. Unknown ids are kept but not
  drawn, so an uninstalled widget's slot survives.
- The grid editor is the `Widgets` service: `plan(key, x, y)` returns the
  layout with the card moved, swapping a same-size neighbour or pushing a
  different-size one down, or null; `place` commits it, `resize` changes a
  span within the manifest's `min`/`max` (the extremes of `sizes` when
  unset). The dashboard shows a live outline of the plan while dragging.

## Test hooks

The island exposes an IPC target for review and scripted verification:

```
qs -p ~/.config/quickshell/isle ipc call island demo osd|media|text
qs -p ~/.config/quickshell/isle ipc call island text "Copied to clipboard"
sudo dev/fake-gamepad.py <<< $'right\na'            # a virtual pad for Big Picture
dev/vm-qmp.py drag 1210 600 1505 540               # drag in the guest
qs -p ~/.config/quickshell/isle ipc call island pin true|false
qs -p ~/.config/quickshell/isle ipc call island clear
```

The VM harness is local tooling under `dev/`, which git ignores: it is
the maintainer's, not part of the shell. `dev/guest.sh '<command>'` runs a
command in the VM with the session's environment; `dev/vm-qmp.py move X Y`
drives the pointer.

The VM itself: `dev/test-vm.sh` runs a CachyOS guest under QEMU with the
repo mounted at `/repo` and the config linked to it, so a host edit is
live in the guest. `dev/install-guest.sh` puts CachyOS on the guest's
disk without the graphical installer, `dev/provision-guest.sh` makes it
the shell's target, `dev/guest-baseline.sh` makes it photographable,
and `dev/test-vm.sh shot` takes a screenshot through QMP. `dev/live.sh`
syncs `shell/` into the guest on save for the headed loop. Restart the
shell with `pkill -x qs` in the guest; the compositor's start hook
relaunches it.

## Modes

One state: `Modes.current ∈ {normal, focus, game, bigpicture}`. Entering a
mode applies a profile; leaving restores the previous one.

| | island | notifications | compositor | daemons |
|---|---|---|---|---|
| **normal** | full | shown | animations, blur, gaps | balanced profile |
| **focus** | clock + timer | DND; urgent and allowlisted apps still shown | as normal | — |
| **game** | hidden; 4px top-edge hot zone peeks it | queued; count shown on exit | animations off, blur off, gaps 0, VRR on, tearing allowed for the game window | gamemode, performance profile, night light off, idle inhibited |
| **bigpicture** | hidden | queued | as game | as game; the Big Picture surface is the home screen |

Game mode enters when gamemoded reports a client or a fullscreen window
belongs to a game launcher, and leaves when that ends. Focus and Big
Picture are manual. Every mode is a toggle in the control panel and a
keybinding.

## Keyboards

Two physical layouts are plugged in at once through the KVM. One rule:

**Hyprland sees one bind set, always on `SUPER`. xremap, per device,
decides which physical chords reach Hyprland as `SUPER` and which reach
the app as `CTRL`.**

- The Windows keyboard (Corsair K95) passes through. Win is Super.
- The Mac keyboard (Keychron Q6 Max, matched by USB id) gets the `mac`
  profile: Cmd+key → Ctrl+key for apps, except the chords the shell owns,
  which stay Super. Option+arrows → Ctrl+arrows; Cmd+arrows → Home/End.
  In kitty, Cmd+C/V/T/W/N → Ctrl+Shift+C/V/T/W/N, so Cmd+C copies rather
  than interrupts.
- Cmd+1..9 and Cmd+Left/Right must reach apps, so the mac profile moves
  those shell actions to Ctrl+1..9 and Ctrl+Left/Right, which is what
  macOS uses for Spaces. Ctrl+Up is overview, as Mission Control.

### Keymap

`Super` is Win or Cmd. Where the profiles differ, both are shown.

| action | Windows | Mac |
|---|---|---|
| launcher | Win+Space | Cmd+Space |
| app switcher (hold) | Alt+Tab, Win+Tab | Cmd+Tab |
| cycle windows of this app | Alt+` | Cmd+` |
| workspace n / prev / next | Win+n, Win+Left/Right | Ctrl+n, Ctrl+Left/Right |
| move window to workspace n | Win+Shift+n | Ctrl+Shift+n |
| focus window ←↓↑→ | Win+Alt+arrows | Cmd+Option+arrows |
| close window | Alt+F4, Win+Q | Cmd+Q |
| minimise (to hidden) | Win+M | Cmd+M |
| fullscreen | Win+Shift+F | Cmd+Shift+F |
| float | Win+Shift+Space | Cmd+Shift+Space |
| terminal | Win+Return | Cmd+Return |
| dropdown terminal | Win+` | Cmd+Option+` |
| files | Win+E | Cmd+Shift+E |
| clipboard | Win+V | Cmd+Shift+V |
| files, GUI (Thunar) | Win+Alt+E | Cmd+Option+E |
| move window left/right/up/down (swap in the tiling) | Win+Shift+Arrows | Cmd+Option+Shift+Arrows |
| pin window on every workspace (floats it first) | Win+Alt+P | Cmd+Option+P |
| capture region / screen / toolbar | Win+Shift+S | Cmd+Shift+4 / 3 / 5 |
| record | Win+Shift+R | Cmd+Shift+R |
| colour picker | Win+Shift+P | Cmd+Shift+P |
| dashboard | Win+A, Win+N | Cmd+Shift+C |
| settings | Win+I, Win+, | Cmd+, |
| monitor | Ctrl+Shift+Esc | Cmd+Shift+M |
| power menu | Win+X, Win+Esc | Cmd+Esc |
| lock | Win+L | Cmd+Ctrl+Q |
| do not disturb | Win+Shift+D | Cmd+Shift+D |
| game mode | Win+Shift+G | Cmd+Shift+G |
| big picture | Win+Shift+B | Cmd+Shift+B |
| volume, brightness, media | the keyboard's own keys | the keyboard's own keys |

`shell/keymap.json` is the one source. The Keyboard service renders it
into `hypr/generated/binds.lua`, which `hyprland.lua` loads, and into
`~/.config/isle/xremap.yml`, which `xremap.service` watches. Settings →
Keyboard shows the table and assigns each keyboard a profile; "auto"
picks Mac for Keychron and Apple names. Rebinding in Settings is still
to come; edit the JSON for now.

## Controller

The Steam Controller is a keyboard and mouse when Steam is not running
(lizard mode: pad and dpad are arrows, A is Enter, B is Escape, triggers
click) and Steam Input takes over when it is. So every surface a
controller should drive is fully keyboard-navigable with a visible focus
ring: Big Picture, the launcher, the switcher, the power menu, the auth
dialog. Big Picture also reads the pad directly (see gamepad in the
services table), so a controller without lizard mode still drives it:
d-pad and left stick move, A launches, B returns to the desktop.
Controller battery comes from UPower and shows in the control panel and
the Devices widget. `dev/fake-gamepad.py` is a uinput pad for the VM.

## Theming installed apps

`shell/theme/tokens.json` is the source. `theme/apply` runs matugen with
the templates in `theme/` and writes:

- `~/.config/gtk-3.0/gtk.css`, `~/.config/gtk-4.0/gtk.css`
- `~/.config/qt6ct/colors/isle.conf` and `qt6ct.conf` (Fusion, Inter)
- `~/.config/kitty/theme.conf`
- `~/.config/yazi/theme.toml`, `~/.config/btop/themes/isle.theme`
- `hypr/generated/theme.lua` (border colours, rounding, blur)

Changing the accent or light/dark in Settings re-runs it. GTK and Qt
apps pick up the CSS live, kitty reloads on signal, Hyprland reloads its
fragment.

## Build order

Each slice ships on its own and is verified in the VM before the next.

| # | slice | delivers |
|---|---|---|
| 0 | skeleton | Quickshell entry, tokens, component kit, wallpaper, island at rest, systemd unit, Hyprland Lua base, install script, memory baseline measured |
| 1 | island | hover state, workspaces, clock, status glyphs, event morphs |
| 2 | notifications | server, island toast, DND |
| 3 | control panel + dashboard | hover unfold, toggles, sliders, output picker, network and bluetooth drill-downs, media, OSD in the island; the widget grid with the ten default widgets |
| 4 | launcher | apps, run, calculator, files, clipboard mode, power actions, every keyboard shortcut with its chord |
| 5 | switcher | app switcher, same-app cycle, minimise to hidden, overview |
| 6 | capture | region / window / screen, screenshot, record, annotate, picker |
| 7 | session | lock, idle, power menu, auth dialog, keyring unlock, greeter |
| 8 | keyboards + settings | xremap profiles, Settings window |
| 9 | app theming | matugen templates, icons, cursor, portals |
| 10 | modes | focus, game |
| 11 | disks + keychain | mount / eject in the island and the Storage widget; Keychain window |
| 12 | monitor | Monitor window; LACT and CoolerControl |
| 13 | big picture | 10-foot launcher over Steam, Heroic, Lutris; gamescope option |
