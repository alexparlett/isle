# Isle v1 feature inventory

What Isle v1 does, what each feature is built on, and what was learned building it. This is the evidence
for planning the Rust rewrite. It does not propose an architecture.

**Sources.** The code on `feature/rust-desktop` at `ae7fd27`, and the design documents deleted on this
branch, read from `main`: `git show main:docs/{ARCHITECTURE,DESIGN,DECISIONS,STACK,WIDGETS,AUDIT}.md`.
`Dn` means entry n in `main:docs/DECISIONS.md`. Line counts come from `wc -l` on tracked files. Where the
documents and the code disagree, the code wins, and §2 lists every disagreement found.

---

## 1. Shape at a glance

```
Hyprland 0.56 (Lua config)                       hypr/hyprland.lua + hypr/generated/*.lua (written by the shell)
 ├─ plugins: isle-windows (own), hyprbars (fork)  built by hyprpm from hyprpm.toml
 ├─ start hook → shell/scripts/isle-session       restart loop; puts qs inside the logind session scope (D17)
 │    └─ qs -p ~/.config/quickshell/isle          ONE Quickshell process: every surface, window, service
 │         └─ Isle.Files (native/files, C++/Qt6)  QML module on QML_IMPORT_PATH: file engine, disk scan, portal, app ids
 ├─ capture.sh (slurp/grim/hyprpicker)            compositor children, since layer surfaces from a qs child cannot map
xremap (systemd --user)                           per-device key remapping; config written by the shell
greetd → its own Hyprland → qs greeter            root-owned copy under /usr/local/share/isle-greeter
daemons: pipewire/wireplumber, NetworkManager, bluez, upower, power-profiles-daemon, udisks2,
         gnome-keyring + gcr ssh agent, gamemoded, xdg-desktop-portal(-hyprland, -gtk), cups, fprintd, lactd, coolercontrold
```

| area | path | lines |
|---|---|---|
| services (singletons) | `shell/services/` (63 files) | 6,360 |
| always-on and on-demand surfaces | `shell/surfaces/` | ~4,600 |
| windows | `shell/windows/` (settings 2,720, files 3,376, cleaner 2,400, monitor 561, keychain 299) | 9,356 |
| widgets | `shell/widgets/` (13 built-ins incl. the text renderer) | 1,150 |
| component kit | `shell/ui/` | 1,445 |
| helper scripts | `shell/scripts/` (40 Python and 8 shell helpers) | 4,072 |
| theme tokens, Theme singleton, icons | `shell/theme/`, `shell/assets/` | 330 |
| keymap | `shell/keymap.json` | 696 |
| native file engine | `native/files/` (C++/Qt6, 21 files) | 4,639 |
| compositor plugin, own | `plugins/isle-windows/main.cpp` | 536 |
| compositor plugin, fork | `plugins/hyprbars/` (upstream plus one Isle block) | ~1,500 |
| compositor config | `hypr/hyprland.lua` | 164 |
| app theme renderer and templates | `theme/` | 477 |
| install, update, root half, greeter, tools | `tools/` | 1,368 |
| greeter | `greeter/shell.qml` (links into `shell/`) | 121 |
| **total, excluding docs** | | **~37,600** |

Roughly 189 QML files. Every capability below is one of three patterns: a native Quickshell type (Pipewire,
Mpris, Notifications, Polkit, Pam, WlSessionLock, IdleMonitor, Screencopy, SystemTray, UPower, Bluetooth,
Networking, Greetd, Hyprland IPC); a `Process` that runs a CLI or a `shell/scripts/*.py` and parses JSON; or
the native `Isle.Files` module.

---

## 2. Where the documents and the code disagree

Read these before trusting `main:docs/` as a specification.

1. **"Nothing else imports the Hyprland module" is false.** ARCHITECTURE says the compositor is reached only
   through `services/Compositor.qml`. `Quickshell.Hyprland` is imported by eleven files: Compositor, Windows,
   Games, Displays, Keyboard, Notifications, Capture, Pip, Restore, Terminal and `surfaces/overview/Overview.qml`.
   Many more build Lua dispatcher strings or call `hyprctl` directly (Modes, Input, Access, NightLight, Lock,
   Displays, Windows, Keyboard, capture.sh, geometry.py, AboutPage). The compositor is coupled throughout.
2. **There is no focus mode.** ARCHITECTURE and DESIGN describe `Modes.current ∈ {normal, focus, game,
   bigpicture}`. `Modes.qml` implements only normal, game and bigpicture, and nothing calls `Modes.set("focus")`.
3. **The island morph is not a spring.** DESIGN says island size changes use a spring (180/22).
   `Island.qml` animates width, height and radius with `NumberAnimation` OutQuint over `Theme.morph`. The spring
   token is used only in Hyprland's window animations (`hl.curve("island", spring)`).
4. **Only volume raises an OSD.** DESIGN lists volume and brightness OSD events. Only `Audio.qml` emits
   `kind: "osd"`. The brightness keys run `brightnessctl` from the bind with no island event.
5. **There is no `input/` directory.** ARCHITECTURE describes `input/mac.yml` and `windows.yml`. The xremap
   config is generated at `~/.config/isle/xremap.yml` by `Keyboard.qml`, and the unit is `systemd/xremap.service`.
6. **Scripts that do not exist.** ARCHITECTURE names `scripts/drives.py` (the drives work was withdrawn, D66)
   and `scripts/vaults.py`. Discovery is `scripts/providers.py`.
7. **The X11 float rule.** ARCHITECTURE and D40 say only titled X11 windows float. `Input.qml` renders
   `isle-float-x11` matching every `xwayland = true` window. Menus are left alone because they are
   override-redirect, not because of the rule's match.
8. **DESIGN's "Installed apps"** says portals use the GTK file dialog, which D67 replaced. STACK still says
   the shell runs as a systemd unit, which D17 replaced.
9. **The numbering has holes and repeats.** D25, D64 and D65 are cited (sandboxed user widgets; drives as
   places) but do not exist. D75 and D76 are each used twice: plugin skew and title-keyed windows; screen
   recovery and favourite order.
10. **`playerctl` is not in `packages/shell.txt`**, although the media keys in `keymap.json` call it.

---

## 3. Compositor behaviour

### 3.1 Base config: `hypr/hyprland.lua` (164 lines)

**Environment.** XCURSOR/HYPRCURSOR size 24; Qt `wayland;xcb` with no Qt decorations; `GDK_BACKEND=wayland,x11,*`;
`QT_QPA_PLATFORMTHEME=gtk3` (D39); `ELECTRON_OZONE_PLATFORM_HINT=auto`; the two NVIDIA variables when
`/proc/driver/nvidia/version` exists; `~/.local/bin` on PATH; `SSH_AUTH_SOCK=$XDG_RUNTIME_DIR/gcr/ssh`.

**Layout and general.** dwindle with `preserve_split`. `gaps_in` 4. `gaps_out {top 48, right 8, bottom 8, left 8}`,
where the 48 clears the island; a gap is a table, not an array (D11). Border 1. `resize_on_border`. Floating snap
on, with `window_gap` and `monitor_gap` 8. Tearing off, and game mode turns it on.

**Misc.** `vrr = 1`, `focus_on_activate`, no middle-click paste, `allow_session_lock_restore = true` (D43).

**Cursor.** `no_warps` (focus never moves the pointer) and `no_hardware_cursors = false`. The second keeps the
pointer out of captures (D27), but it is linked to an open Hyprland crash in `renderHWCursorBuffer` on monitor
layout change (see the memory note on crash signatures).

**Input.** `follow_mouse = 2` (click to focus), flat acceleration, `kb_layout` gb as a placeholder that the
generated fragment overrides. `render.direct_scanout = 1`.

**Animations.** The curves are easeOutQuint, linear and a spring `island` (mass 1, stiffness 180, damping 22).
Windows use the spring with `popin 96%`, `windowsOut` is linear, layers fade with OutQuint, and workspaces fade.

**Gestures.** Three fingers up opens Mission Control and three down closes it, each running a function that
calls the shell's IPC.

**Mouse.** SUPER with the left button drags a window and SUPER with the right button resizes it.

**Submap.** `isle-record` unbinds everything while Settings records a chord, with Escape as the way out (D58, keyboard detection).

**Start hook.** It runs `hyprpm reload -n && hyprctl reload`, imports the environment into systemd and starts
`xremap.service`, then runs `isle-session`.

**Fragments loaded with pcall.** `generated/monitors.lua`, `generated/theme.lua`, `generated/input.lua`, and
`generated/binds.lua`, which is called as `binds(ipc, terminal)`. The `ipc()` helper builds
`qs -p <shell> ipc call <target> <fn> [arg]` exec dispatchers.

### 3.2 Generated config (the shell writes the compositor's config)

| fragment | writer | from | then |
|---|---|---|---|
| `hypr/generated/monitors.lua` | `services/Displays.qml` `render()` | `prefs.monitors` (mode, scale, position, transform, vrr, enabled) | `hyprctl reload` |
| `hypr/generated/input.lua` | `services/Input.qml` | `prefs.input` (layouts, variants, xkb options, repeat, sensitivity, accel, natural scroll, left-handed, touchpad), plus per-device `hl.device` lines from `Keyboard.deviceLua()`, plus the window rules `isle-float` (every Wayland window floats at 60×65% of monitor 0, centred) and `isle-float-x11` | `hyprctl reload` |
| `hypr/generated/binds.lua` | `services/Keyboard.qml` `renderHypr()` | `shell/keymap.json` and `prefs.keymapOverrides`, `prefs.terminal` | `hyprctl reload` and a restart of xremap |
| `hypr/generated/theme.lua` | `theme/render.py` (template `theme/templates/hypr-theme.lua`) | tokens and prefs: border colours, rounding, blur (size 8, passes 3, popups), the hyprbars config and buttons | `hyprctl reload` |

Dispatchers sent over IPC are Lua expressions, such as `hl.dsp.focus({ workspace = 2 })`, not the old strings (D11).
`Compositor.exec()` wraps `hl.dsp.exec_cmd` and quotes arguments for the shell, since exec goes through `sh`.

### 3.3 Window and layer rules

| rule | where | what |
|---|---|---|
| `x11-no-blur` | hyprland.lua | no blur under any XWayland window, since their own translucent shadows smear |
| `x11-popup` (tag `x11popup`) | hyprland.lua + isle-windows plugin | override-redirect X11 windows get no animation, no shadow, rounding 0, no focus and `min_size 1 1`. The plugin tags them in `window.openEarly`, because the rule engine cannot match override-redirect. The border stays, because removing it shrinks the window (D40) |
| `x11-untitled` | hyprland.lua | an untitled XWayland window gets the same look; it is a menu that turned override-redirect after mapping |
| `fix-xwayland-drags` | hyprland.lua | an X11 window with no class, no title, floating and not fullscreen gets no focus |
| `game-tearing` | hyprland.lua | `immediate` for fullscreen windows, effective only while game mode allows tearing |
| `isle-dropdown` | hyprland.lua | `class isle-dropdown`: `special:terminal`, float, 70%×50%, near the top, opacity 0.96 |
| `isle-windows` | hyprland.lua | the shell's own windows (`isle-settings|keychain|monitor|cleaner|files`, `org.quickshell`) float, centred |
| `pip` | hyprland.lua | title "Picture-in-Picture": float, pin, keep aspect, 480×270, no border. `services/Pip.qml` moves each one bottom-right, because the rule's own move is ignored on 0.56 |
| `isle-float`, `isle-float-x11` | generated input.lua | everything floats by default (§3.4) |
| layer `isle-blur` | hyprland.lua | blur plus `ignore_alpha 0.2` on namespaces `isle-(island|panel|dashboard|launcher|switcher|capture|power|overview|filechooser)` |
| layer `isle-wallpaper` | hyprland.lua | no animation |

### 3.4 Floating by default, tiling by drag: the `isle-windows` plugin

Files: `plugins/isle-windows/main.cpp` (536 lines), `surfaces/windows/SnapGhost.qml` and `services/Windows.qml`.
It is built by hyprpm from `hyprpm.toml`.

- **Everything floats.** Windows open centred and snap to edges and to each other while dragged. dwindle is the fallback layout.
- **Drag-to-edge tiling (D40, D91).** The plugin hooks `CLayoutManager::endDragTarget`, where every drag ends,
  whether it began with Super+drag, the hyprbar or the client's own title bar. A drop within 30px of an edge
  tiles the window: a side takes that half, a corner that quarter, the top the work area. Tiles keep 8px
  margins and respect `gaps_out`. The window sits inside the tile by its decoration extents. The pre-tile box
  is remembered and restored when the window is dragged away.
- **Snap preview.** While the pointer is over an edge, the plugin posts `isle-ghost` "x y w h" on the event
  socket. `Windows.qml` reads it from `Hyprland.rawEvent` and `SnapGhost` draws it. A drag never waits on a process.
- **Key and bar actions.** These go through a hyprctl command: `hyprctl isle snap <zone> | zoom | restore`.
  The Lua config cannot call a plugin dispatcher, but it can run a command. The keys are Super+arrows and the
  hyprbar's double click (`on_double_click = "hyprctl isle zoom"`).
- **Minimise.** `hyprctl isle hide [addr…]` parks the window on `special:hidden` silently, closes that
  workspace if it came up, and focuses the top window left. It also hooks the window `onUpdateState` handler,
  so a client's own minimise request, which Hyprland ignores and answers on X11 with a sticky maximise
  toggle, is consumed and handled on the next loop turn (`doLater`).
- **X11 `_NET_WM_MOVERESIZE`.** It hooks the XWM `handleClientMessage` with `#define private public` on the
  XWM headers and begins the layout's own drag. Without this, Chromium or GTK windows under XWayland cannot be
  moved or resized by their own controls.
- **Window places (D57, D91).** `hyprctl isle place clear` followed by `place <class> x y w h` is sent as a
  `hyprctl --batch` from `Windows.pushPlaces()`, with an empty table in game mode. The `window.open` hook sets
  the box of the class's sole titled floating window before the open animation starts, so the window opens at
  its place rather than sliding there.
- **Version check.** If the API hash differs, the plugin posts a notification and throws.

### 3.5 Title bars: the `hyprbars` fork

Files: `plugins/hyprbars/` (upstream at the commit pinned for this Hyprland, see `ISLE.md`) and the bars
block in `theme/templates/hypr-theme.lua`.

- **The Isle change.** There is no bar over a client that draws its own decorations. That covers clients
  asking for client-side decorations through xdg-decoration or KDE server-decoration, and for X11, Motif
  hints with decorations 0 or `_GTK_FRAME_EXTENTS`. The X11 case is read over the plugin's own XCB connection
  to XWayland once per rules update. The bar is decided again on title, floating and fullscreen events, so a
  client that turns borderless loses it (D48).
- **The bar.** It is 28px, in the window colour, with the title on the left in text-2 and buttons on the right.
  Close (danger) calls `qs ipc call windows close`, so an app the user keeps running is put away instead of closed.
  Fullscreen (ok) is `hl.dsp.window.fullscreen({mode=0})`. Minimise (warn) is `qs ipc call switcher hide`.
  Glyphs show on hover. The `titleBars` preference turns bars off.

### 3.6 Workspaces and windows (`services/Windows.qml`, 447 lines)

- **Workspaces.** Super+1..9 and Super+Ctrl+←/→ (`e±1`); Super+Shift+n moves a window. The island's dots show
  that screen's positive workspaces. The user-facing word is "desktop" (DESIGN).
- **Special workspaces.** `hidden` is both minimise and the tray for apps kept running. `terminal` holds the
  dropdown. `steam` holds Steam's Big Picture.
- **Fullscreen gets a space of its own (D48).** On every `fullscreen` and `closewindow` raw event (debounced
  150ms), at start, and on the 4s `hyprctl -j clients` poll, a window that covers its monitor and shares a
  workspace is moved to `workspace = "empty"`. That includes a floating window the monitor's size, which is a
  game's borderless fullscreen. The view follows only the focused window. The window returns when fullscreen
  ends, and the view goes back when a parked window closes.
- **Window places (D57).** The same 4s poll records the place of every class whose only titled window is
  floating, mapped, not fullscreen, not on a special workspace, at least 100px and on screen
  (`prefs.windowPlaces`). Off-screen places are never recorded.
- **Stray rescue (D57).** A floating window left outside every monitor box, usually after a KVM switch, is
  moved onto the nearest screen, clamped whole, one window per pass.
- **Recency.** The `activewindowv2` raw events keep an MRU list of 64 addresses.
- **Apps.** Windows are grouped by app id, which is resolved through desktop entries (`heuristicLookup`).
  `steam_app_<id>` maps to the library game's name and cover. A `gamescope` window is named after the first word
  of its title. The shell's own windows keep their `isle-*` ids, with a fallback to titles under `org.quickshell` (D75, D92).
- **Kept running.** For apps in `prefs.keepRunning`, every close the shell issues (Super+Q, the switcher's
  quit, the hyprbar ×) hides the window instead. Such apps appear in the tray row.
- **Raise, don't relaunch (D63).** `open(entry)` raises the existing window of a single-window app after a
  140ms delay, because a closing layer surface hands keyboard focus back and undoes an immediate focus.
  `scripts/entries.py single` lists which entries declare a single main window.
- **Closing goes by address**, never through the toplevel handle: closing a handle that is already going away
  is a protocol error that kills the shell.
- **Paste into the previous window.** `hl.dsp.send_shortcut` sends Ctrl+V, or Ctrl+Shift+V for terminals, to an address.

### 3.7 Displays (`services/Displays.qml`, 101 lines; Settings › Displays; `ui/DisplayMap.qml`)

- **Arrangement.** The map is drawn to scale. A drop snaps to the other monitors' edges and pushes out of
  overlap, then `place()` pins every monitor to an absolute `XxY` position. Mode, scale, transform, VRR and
  enabled are set per monitor, and the last enabled display cannot be turned off.
- **Primary monitor.** `prefs.primaryMonitor` or `shellFollowsFocus` decides which screen the shell's surfaces use.
- **Redetect (D56, D76).** One `sh -c` command does DPMS on, then `hl.monitor{disabled=true}` for every
  screen, then sleeps 1s, then `hyprctl reload`. It is bound to Super+Ctrl+D and exposed as IPC `displays redetect`.
  This handles a KVM that holds EDID: the connector stays up with a CRTC and is never re-probed.
  `force_renderer_reload` does not fix this. A sleeping output is not released, so the screens are woken first.
- **Stranded heal (D76).** While `Quickshell.screens` is only `FALLBACK`, `hyprctl reload` runs every 5s.
  `Hyprland.monitors` keeps entries for dropped monitors and never notifies, so it cannot be used for this check.
- **Brightness.** `services/Brightness.qml` uses `brightnessctl` for a backlight and `ddcutil` (DDC/CI) for
  monitors, with cached reads and debounced writes, since a DDC call takes about 1s.

### 3.8 Steam, gamescope, games (`services/Games.qml`, 210 lines)

- **Game detection.** The mode turns on with `gdbus monitor` on `com.feralinteractive.GameMode`
  (GameRegistered/Unregistered), or with a fullscreen toplevel whose app id matches
  `steam_app_\d+|steam_proton|*.exe|gamescope|heroic|lutris` (`prefs.gameAuto`).
- **Steam's Big Picture.** It is detected as a toplevel with class `steam` or `gamescope` and a title containing
  "big picture" that is not on `special:hidden`. Steam minimises it rather than closing it. The window is parked
  on the `name:steam` workspace with `fullscreen_state internal=2 client=2`, and the original workspace is
  remembered across title flips. When it closes, the window is unparked, focus returns, and a Steam client
  window that opened on the steam workspace is moved back. The mode follows Steam both ways
  (`prefs.steamFollowsMode`, `steamDriven`).
- **Steam's X11 menus (D40).** These are X11 popups of the client's class. They are never placed or noted,
  only styled by the `x11popup` and `x11-untitled` rules. A month of misrendering was this.
- **Launching.** `steam steam://open/bigpicture`, or `gamescope -f -e -W -H -- steam -gamepadui` when `prefs.gamescope` is set.
  Heroic uses `heroic --console`. Lutris uses `lutris`. Games run headless through
  `heroic --no-gui heroic://launch/…` and `lutris:rungameid/…`. `scripts/games.py` reads the Steam, Heroic and
  Lutris libraries and art.
- **Shortcuts inside Steam.** `scripts/steamshortcuts.py` writes non-Steam shortcuts ("Isle settings",
  "Desktop") into `shortcuts.vdf` that run `scripts/isle-quick.sh`. It refuses while Steam runs, because Steam
  rewrites the file on exit. The script polls IPC so it stays alive while the quick menu is open, which keeps
  Steam handing it the pad.

### 3.9 Input devices

- **Mouse and touchpad.** These are `prefs.input` rendered into `input.lua`.
- **Keyboards.** There is one `hl.device` line per keyboard interface, with `kb_model` from `prefs.keyboardModels`
  (guessed from the vendor and an ISO or ANSI layout) and the Mac variant on Mac-profile devices and on the
  xremap virtual device (D32). `numlock_by_default` is set there too.
- **Magnifier (D50).** `hyprctl eval` sets `cursor.zoom_factor` in quarter steps to 2, then halves to 4
  (`services/Access.qml`, Super+Ctrl and =, -, 0).
- **Device lists.** `udevadm monitor --udev --subsystem-match=input` re-reads `hyprctl -j devices`.

### 3.10 Other compositor-level features

- **Dropdown terminal** (`services/Terminal.qml`). One `kitty --class isle-dropdown` lives on `special:terminal`,
  sized by dispatch after it maps, and is toggled with `workspace.toggle_special`.
- **Night light** (`services/NightLight.qml`). `hyprctl hyprsunset temperature <K>` or `identity`.
- **Idle and DPMS.** See §4 Lock and idle.
- **Session restore (D53)** (`services/Restore.qml`). The 4s client poll writes
  `~/.local/state/isle/session.json`, one entry per app and workspace, plus whether it was hidden. On the first
  shell start for a compositor instance (a marker keyed by `HYPRLAND_INSTANCE_SIGNATURE` under
  `$XDG_RUNTIME_DIR/isle`), entries not started by autostart and not already open are launched. Their next
  windows are moved to their workspaces as they map, and hidden ones are re-hidden with `hyprctl isle hide`.
- **Logout.** `hl.dsp.exit()`.

---

## 4. Always-on and on-demand surfaces

All of these are `PanelWindow`s (layer-shell) in the one Quickshell process. Only one full-screen surface is
open at a time: the dashboard, overview, launcher, capture bar and power menu close each other
(`services/Surfaces.qml`). Escape is bound transparently in binds.lua to `surfaces closeAll`.

| surface | file(s), lines | layer / namespace / keyboard | what the user sees | Hyprland dependence | system services |
|---|---|---|---|---|---|
| Wallpaper | `surfaces/wallpaper/Wallpaper.qml` 24, `ui/Backdrop.qml` | Background, `isle-wallpaper`, ignore exclusion | image from `prefs.wallpaper` or a drawn backdrop | layer rule no_anim | `scripts/wallpapers.py`; zenity picks a folder |
| Island and control panel | `surfaces/island/*` 1,033 (Island 183, PanelBody 488, EventContent 160, RestContent 74, ToggleTile, HeaderBand, PanelPage, HotZone) | Top, `isle-island`, OnDemand keyboard only during a notification (inline reply), input `mask` on the body | rest pill, hover panel, event morphs (below) | one per screen via `Variants` over `Quickshell.screens` (D49); dots from `Compositor.workspaceIdsOn`; the window stays at panel size and only the body morphs, so the compositor never re-places a resizing surface | everything the panel shows |
| Hot zone | `surfaces/island/HotZone.qml` 18 | Overlay, `isle-hotzone` | 4px top edge in game mode; a 400ms hover peeks the island for 4s | | |
| Dashboard | `surfaces/dashboard/*` 1,498 (Dashboard 482, WidgetCard 240, WidgetStore 584, WidgetForm, WidgetPreview) | Overlay, `isle-dashboard`, Exclusive | full-screen widget grid with the wallpaper dimmed and blurred; edit mode; store (§6) | blur layer rule | via widgets |
| Launcher | `surfaces/launcher/Launcher.qml` 148, `services/Launcher.qml` 361 | Overlay, `isle-launcher`, Exclusive | 640px card at 30% height; apps (ranked by `prefs.launchCounts`), shell actions with chords, settings pages, windows, vault, web fallback; prefixes `>` run, `=` qalc, `/` fd names, `?` rga or rg inside files (250ms debounce, 20 files), `.` emoji (`scripts/emoji.py`, typed through wtype 180ms after close), `:` cliphist (image thumbnails, pins, delete, paste into the previous window), `@` windows, `*` vault (Enter copies the password, Shift the username, Ctrl the TOTP) | raise-not-relaunch through Windows; paste through `send_shortcut` | qalc, fd, rga/rg, cliphist, wl-copy, wtype, xdg web |
| Switcher | `surfaces/switcher/Switcher.qml` 130, `services/Switcher.qml` 63 | Overlay, `isle-switcher`, no keyboard focus | row of 240×150 live previews, one per app (D59), count badge, backtick cycles that app's windows, Q quits the app, W closes the window, wraps at 3/4 width | **commit on modifier release** is a generated `release = true, non_consuming = true, transparent = true` bind on Super_L/R and Alt_L/R, plain and with Shift. A plain release bind is suppressed after Super+Tab | ScreencopyView bound to visibility (D55) |
| Mission Control | `surfaces/overview/Overview.qml` 363 | Overlay, `isle-overview`, Exclusive | spaces bar of live desktops plus a `+` tile; the current desktop's windows spread without overlap (rows of `ceil(sqrt(n))`, never larger than real); close with ×, W, Delete or middle click; drag to another desktop or to `+`; hidden-window tray; arrow, Tab and Enter navigation; windows animate from their real rectangles | toplevel list, `window.move({follow=false})`, close by address; gesture `hl.gesture` | ScreencopyView live per toplevel, only while visible (D19, D55) |
| Capture bar and freeze frame | `surfaces/capture/CaptureBar.qml` 108, `FreezeFrame.qml` 32, `services/Capture.qml` 121, `scripts/capture.sh` 60, `crop.py`, `geometry.py` | Overlay `isle-capture` (Exclusive); Top `isle-freeze` | region, window or screen; shot or record; countdown; cursor toggle; audio; annotate (satty); colour pick; the island shows a recording timer with Stop | capture.sh runs as a **compositor child** (`hl.dsp.exec_cmd`), because slurp and hyprpicker overlays cannot map from a qs child. The shot is frozen first: grim grabs every monitor, FreezeFrame shows the frames under slurp, and crop.py cuts the region, so the pick's own veil never reaches the shot | grim, slurp, wf-recorder (`-g` geometry, optional audio), satty, hyprpicker, wl-copy |
| Lock | `surfaces/lock/LockSurface.qml` 135, `services/Lock.qml` 131 | `WlSessionLock` (ext-session-lock) | wallpaper blurred and dimmed, 28px clock, password field on typing, shake on failure, attempt count, Caps Lock warning, fingerprint beside the password (D33) | `allow_session_lock_restore`; Caps Lock state by polling `hyprctl devices -j` every second while locked | PAM (`login` stack) through the Pam service; `fprintd-list`, `fprintd-verify` loop; `gdbus monitor --system org.freedesktop.login1` for Lock and Unlock signals |
| Idle dimmer | `surfaces/idle/Dimmer.qml` 32, `services/Idle.qml` 56 | Overlay `isle-dim` | dims at `prefs.idleDim`, locks at `idleLock`, DPMS off at `idleScreenOff` | `hl.dsp.dpms` | ext-idle-notify `IdleMonitor`, respecting inhibitors |
| Auth (polkit) | `surfaces/auth/AuthDialog.qml` 80, `services/Auth.qml` 37 | Overlay `isle-auth`, Exclusive | centred dialog: icon, message, password; the island says "Authentication required" | | Quickshell `PolkitAgent` (this process is the agent) |
| Power menu | `surfaces/power/PowerMenu.qml` 137, `services/Session.qml` 19 | Overlay `isle-power`, Exclusive | five pills: lock, sleep, restart, shut down, log out; first letter selects; a countdown before restart or shut down | `hl.dsp.exit()` | `systemctl suspend|hibernate|reboot|poweroff` |
| File chooser (portal) | `surfaces/filechooser/FileChooser.qml` 222, `native/files/filechooser.{h,cpp}` 406 | Overlay `isle-filechooser`, Exclusive | glass card up to 880×620: the app's title, a trail, search, the Files folder list, Cancel and Open; Save adds a name field and a type picker; does not close other surfaces | blur layer rule | the shell itself owns the D-Bus name `org.freedesktop.impl.portal.desktop.isle` and object `/org/freedesktop/portal/desktop`; the reply is held until accept or reject (D67); `system/portal/isle.portal` plus `portals.conf` `FileChooser=isle` |
| Notifications toast and centre | island EventContent, `ui/NotificationCard.qml` 109, `services/Notifications.qml` 206 | island | toast with actions, inline reply and image; centre as an island panel page; "Earlier" history records (D26); DND, quiet hours, fullscreen and game-mode silencing; a "while you were away" summary; per-app mute and allow lists; optional sound | `fullscreen` raw event | Quickshell `NotificationServer` (org.freedesktop.Notifications); `pw-play` sound; log at `~/.config/isle/notifications.json` (300 kept) |
| On-screen keyboard | `surfaces/keyboard/OnScreenKeyboard.qml` 196, `services/Osk.qml` 96 | Overlay `isle-keyboard`, never takes focus; Auto exclusion on the desktop, Ignore in game | console-style two-cursor pad keyboard, completion strip from `/usr/share/dict/words`, pad legend | | `wtype` queue (virtual-keyboard protocol) (D37) |
| Big Picture home | `surfaces/bigpicture/BigPictureHome.qml` 258 | Top `isle-bigpicture`, OnDemand | 10-foot rows: Continue, Library (Steam, Heroic, Lutris), Stores, System; 176×264 covers; focused tile scales to 1.06 with an accent ring; pad legend; search through the OSK | shown only when nothing is "in front" (D38) | gamepad helper |
| Big Picture quick menu | `surfaces/bigpicture/BigPictureQuick.qml` 161 | Overlay `isle-quick`, OnDemand | 480px right-edge slab: sound, output, brightness, Wi-Fi, Bluetooth and pairing, power profile, DND, keyboard, home, sleep, shut down (asks twice), desktop | | |
| Snap ghost | `surfaces/windows/SnapGhost.qml` 44 | Overlay `isle-ghost` | translucent tile target during a drag | `isle-ghost` plugin event | |

**Island details** (`DESIGN.md` § The island; `surfaces/island/*`, `services/IslandEvents.qml` 54):

- **Rest pill.** 30px, 12px from the top: desktop dots, clock, and glyphs only when they say something.
  Wi-Fi shows on Wi-Fi only. Bluetooth shows when a device is connected. Volume shows when muted. The other
  glyphs are recording, screen share (live colour), mode chip, calendar within 30 minutes, updates (download),
  restart owed (rotate, warn colour) and VPN shield.
- **Hover.** After a 120ms dwell the pill unfolds into a 420px panel. It leaves after a 250ms grace and is
  disarmed after a click opens something. The panel holds: a header band (44px) with the desktops as app icons;
  toggles (Wi-Fi, Bluetooth, Night light, Notifications (its click is DND), Game mode, VPN); the output picker with volume and brightness;
  media; tray icons, with the tray menu as a panel page; and a footer with batteries, the updates count and the
  power profile. Drill-down pages cover Wi-Fi (password prompt), Bluetooth (pairing code), outputs with
  per-app volumes, VPN providers and notifications.
- **Events.** OSD (volume), media start, notification, recording with timer and Stop, screen share with Stop
  (`pw-cli destroy` on the portal stream), drive inserted (Mount, Eject), auth pending, mode change,
  clipboard copy, calendar event start, AppImage installed, update finished or restart needed. An open panel
  is not taken over by an event: the event waits until the pointer leaves.
- **Clicks.** The dots open Mission Control. The clock or bell opens the dashboard or the centre.
- **Game mode.** The island is unmapped except while peeked through the hot zone.

---

## 5. Windows

These are `FloatingWindow`s of the same process. Each sets its own Wayland app id through
`Isle.Files`' `WindowAppId`, which swaps `QGuiApplication::desktopFileName` on `visibleChanged`, before the map,
and restores it on the next event-loop pass (D92, `ui/AppId.qml`). Without the engine, all of them are
`org.quickshell` and are told apart by title (D75).

| window | files, lines | user sees | depends on |
|---|---|---|---|
| Settings (`isle-settings`) | `windows/settings/Settings.qml` 164, 27 pages 2,556, `services/SettingsIndex.qml` (page list shared with the launcher) | sidebar groups. **Network**: Network (Wi-Fi, VPN providers), Bluetooth, Phone. **Look and feel**: Appearance (dark, light, auto by sunrise from tzdata coordinates `theme/sun.py`; accent; fonts; title bars; reduced motion), Wallpaper, Accessibility (magnifier, text scale, high contrast, OSK), Notifications. **Desktop**: Modes (game auto, tearing, gamescope, Steam follows, pad home, Steam shortcuts), Apps (session restore, autostart, user services, compositor hook, keep-running, default apps), Shortcuts (both profiles, chord recording). **Hardware**: Displays, Audio, Keyboard (per-device model and profile, Detect, layouts, XKB options), Mouse, Controllers (live pad drawing, dead zone), Printers, Storage (drives, SMART), Power, Devices (Device Manager tree). **System**: Users (AccountsService, fingerprint enroll and PAM toggles, autologin), Date and time, Region, Passwords, Calendars, Updates, Snapshots, About. Back and forward history (mouse buttons, Alt+arrows) | prefs; nearly every service; pkexec for PAM, autologin, users, cups, timedatectl, localectl |
| Monitor (`isle-monitor`) | `windows/monitor/Monitor.qml` 300, `ProcessTable.qml` 261, `services/Monitor.qml` 105, `scripts/sysinfo.py` 235 | Activity Monitor shape: CPU, Memory, Energy, Disk and Network tabs over a process tree with descendants' totals; Sensors (temperatures, fans, GPU); STOP, CONT, TERM, KILL and renice; buttons to LACT and CoolerControl | `/proc`, hwmon, `/proc/diskstats`, RAPL `energy_uj`, `nvidia-smi`, LACT socket `/run/lactd.sock`, CoolerControl REST `127.0.0.1:11987`, `nethogs -t` (setcap), `systemd-inhibit --list` |
| Keychain (`isle-keychain`) | `windows/keychain/Keychain.qml` 299, `services/KeychainService.qml` 85, `SshKeys.qml` 48, `Vault.qml` 81 | keyring items under category chips (logins, Wi-Fi, browser, apps); reveal, copy, delete; generator; SSH keys with fingerprint and agent state, add and remove from the agent, generate; one chip per password-manager provider | libsecret through `scripts/secrets.py` (python-gobject); `ssh-keygen`, `ssh-add`, `askpass.sh` with a one-shot 0600 file; gcr ssh agent; vault providers |
| Files (`isle-files`) | `windows/files/*` 3,376 (FilesWindow 1,373, FolderView 900, FileColumns 473, QuickLook, FilePreview, FileMenu, FileSheet, DragChip, DropGlow), `services/FileWindows.qml`, `services/Places.qml` 179 | Finder-shaped: any number of windows with tabs; sidebar (Recents, Favourites as user dirs plus GTK bookmarks in a user-chosen order, Locations as volumes plus Trash, saved searches); List, Columns and Grid; nested disclosure; preview pane; Quick Look on Space; search narrowed by kind and date, savable (D74); drag and drop in and out; copy, move, trash, put back, undo one step (D70, D71); rename many with `#`; pack and unpack (bsdtar); open with and set default (`scripts/openwith.py`); open in terminal; AppImage install by folder (D82); full keyboard map (AUDIT § Files) | `Isle.Files` (`Directory` model with its own inotify, `FileJobs`, `Thumbnails` in the freedesktop cache (D68), `Engine`); `inode/directory` handler `isle-files.desktop` → `scripts/isle-files` → IPC (D73); GTK bookmarks file (D69); udisks through the Disks service |
| Cleaner (`isle-cleaner`) | `windows/cleaner/*` 2,400 (Cleaner 630, Cleanup 208, Treemap 206, Overview 196, Duplicates 170, Space, ItemRow, FileList, Donut, Check, `kinds.js`, `rules.json` 582) | Broom or DaisyDisk shape: Overview (ring and bar by kind, hotspots, largest), Cleanup (Safe to clear, ticked by default, and Worth a look; a System card for root-only items), Space (squarified treemap or list), Duplicates, Large files, Untouched; a basket to the trash with one undo; disk cards with Empty trash | `Isle.Files` `DiskUsage` (threaded scan of folders only, by kind, allocated blocks, crosses same-device btrfs subvolumes, never `.snapshots` (D87)); `duplicates.cpp` (size, then BLAKE2b of the ends, then the whole file; reflinks via FIEMAP excluded (D90)); `evaluate(rules)` over the tree (D89); `scripts/syscleanup.py` through pkexec (package cache, orphans, journal, crash dumps) |
| Big Picture | surfaces, not a window (§4) | | |

Settings, Keychain and Monitor are reopened after a reload if they were open within the last two minutes
(`$XDG_RUNTIME_DIR/isle/reopen.json`, D62).

---

## 6. Widgets and the isle-widgets registry

**Files.** `services/Widgets.qml` 446; `surfaces/dashboard/*` 1,498; `ui/WidgetBase.qml`, `ui/WidgetHost.qml` 151;
`shell/widgets/*` 1,150; `scripts/widgets.py` 137 (scanner and sandbox), `widgetstore.py` 229 (sources and
install), `widgetauthor.py` 202 (new, validate, share); `main:docs/WIDGETS.md` (the author contract).

**Contract.** A widget is a directory holding `widget.json` and `Widget.qml`. Root `WidgetBase` carries
`manifest`, `size`, `cols`, `rows`, `settings`, `title` and `meta`. The card draws the glass, title and meta.
Manifest fields: `id, name, glyph (Lucide), category, description, version, author{name,url}, license,
homepage, sizes ["CxR"], default, min, max, multiple, permissions, settings[{key,label,type toggle|choice|number|text}],
editor, screenshots, trust, requires, source{command|file, interval}, view`.

**Grid.** 12 columns by 8 rows (D35; older 4-row layouts are doubled once, marked by `dashboardGrid`).
Pages are `prefs.dashboardPages [{name, layout:[{key,id,x,y,w,h}]}]` and instances are keyed `clock#2`.
`Widgets.plan/place/resize` is the grid editor: a live outline, cascade pushing, edge resize within min and max,
and keyboard nudges (D24).

**Built-ins** (`shell/widgets/`). The defaults are Clock (multiple), Media, Workspaces, Notifications,
Controls, System (multiple, with its own `Settings.qml` editor for series and styles, D36), Agents (terminal
windows, i.e. coding agents, from `Windows.terminals`), Session (tray, power, profile), Devices (batteries)
and Storage. Off by default: Calendar and Weather (Open-Meteo through curl). `text/` is the renderer for
manifest-only widgets (text, number, gauge, sparkline, list).

**Polling.** Services that poll (`System`, `Storage`) count `listeners`, so nothing samples while the dashboard is closed.

**Loading.** Built-ins load by `Qt.resolvedUrl` inside the served `qs:@/` tree, which is the only way to
import `qs.*` (D14). User widgets in `~/.config/isle/widgets/<id>` are linked in as `shell/userwidgets`
by install.sh.

**Sandbox (WIDGETS.md, D25 cited but missing).** The source is scanned before load. Imports are limited to
QtQuick, Layouts, Shapes, Effects, Particles, QtQml and QtQml.Models. These are forbidden: `XMLHttpRequest`,
`Qt.createComponent`, `createQmlObject`, `openUrlExternally`, `Loader`, `WorkerScript` and `qs.*`. The widget
receives a `host` object gated by permissions: theme; audio(.write); network; media(.write); system;
notifications(.write); power; bluetooth; vpn; `fetch:<host>` (curl run by the host, 20s and 4MB caps, D31);
`notify` (notify-send); and `store` (per-instance state in prefs). The user can narrow grants
(`widgetGrants`, D28); a revoked `.write` turns calls into no-ops.

**Trust (D29).** `trust: true` gives full reach. For a widget from a source this also needs a user grant
(`widgetTrust`); without it the widget is installed but blocked.

**Store and sources (D29, D30).** Sources are git repositories with `index.json` at the root, holding a
`{name, widgets:[{id, path} | {id, repo, ref, …listing}]}` list. They are cloned shallow under
`~/.cache/isle/widget-sources/` and pulled hourly. Install copies or clones into staging and swaps in, leaving
`.isle-install.json` (source, version, ref). Updates compare semver. The UI has Installed, Yours and Browse
tabs, cards and a sheet (README, CHANGELOG, permissions as toggles, screenshots or a live preview), install
and update consent dialogs, and Sources management. Author tools are Check (validate), Picture
(`grabToImage` to `screenshots/card.png`) and Tag (git init, commit, tag `v<version>`).

**Registry.** The default source is derived from the shell's origin by swapping the repository name, giving
`github.com/alexparlett/isle-widgets` (checkout at `~/Development/isle-widgets`). Its index lists
analog-clock, system-rings, transport, battery, focus-timer and uptime. **Every registry widget is QML
written against this contract**, so a non-QML rewrite orphans the registry unless it keeps a QML (or
replacement) widget runtime.

---

## 7. Services (`shell/services/`, 63 singletons, 6,360 lines)

Singletons are lazy. `shell.qml`'s `Component.onCompleted` touches about 40 of them so their IPC handlers and
watchers exist at start (D80). The directory is one QML module, so one uncompilable file takes the whole shell
down, and services must not import `Isle.Files` (D72).

| service | lines | driven by | IPC target |
|---|---|---|---|
| Access | 30 | `hyprctl eval` cursor zoom | `access` |
| AppImages | 86 | FolderListModel on `~/Applications` → `scripts/appimages.py sync` (unsquashfs at the ELF section offset, D82) | |
| Audio | 119 | native Pipewire; `pw-mon` for streams; app volumes grouped by process; screencast detection by `xdph-streaming-N` names; `pw-cli destroy` | `privacy` |
| Auth | 37 | native PolkitAgent | |
| Bluetooth | 96 | native Bluetooth (BlueZ) plus a long-lived `bluetoothctl` agent (KeyboardDisplay) with its prompts parsed | |
| Brightness | 83 | brightnessctl, ddcutil | |
| Calendars | 104 | `scripts/calendars.py` (python-icalendar, recurring-ical-events; cache `~/.cache/isle/calendars`; every 30 minutes) (D41) | |
| Capture | 121 | capture.sh, wf-recorder, satty, hyprpicker | `capture` |
| Clipboard | 13 | `wl-paste --watch cliphist store` for text and image | |
| Compositor | 46 | Quickshell.Hyprland workspaces, monitors, dispatch | |
| DateTime | 56 | timedatectl | |
| Defaults | 72 | `scripts/defaults.py` (xdg-mime, xdg-settings) | |
| Devices | 35 | `scripts/devices.py` (udev db, lspci, sysfs, lsblk, lscpu) and `udevadm monitor` | |
| Disks | 118 | `lsblk -J`, `udisksctl monitor`, `mount`, `unmount`, `power-off`; auto-mount | |
| Displays | 101 | §3.7 | `displays` |
| FileWindows | 50 | Files window instances | |
| Fingerprint | 76 | `scripts/fingerprint.py` (fprintd-*, PAM edits through pkexec) | |
| Gamepad | 39 | `scripts/gamepad.py` (raw evdev, `BTN_GAMEPAD`; `--live` JSON; `--deadzone`; uinput origin detection, D54), with a reference-counted `listeners` | `gamepad` |
| Games | 210 | §3.8 | `bigpicture` |
| Idle | 56 | native IdleMonitor in Loaders recreated on change (D18); DPMS state re-sent when screens change (D76) | |
| Input | 80 | §3.2 | |
| IslandEvents | 54 | event queue, hold and resume | |
| IsleUpdate | 128 | `tools/update.py check|run`, detached (D78); FileView on `$XDG_RUNTIME_DIR/isle/update.json` | `isle` |
| Keyboard | 422 | keymap.json → binds.lua and xremap.yml; `hyprctl -j devices`; udevadm; `scripts/keyboards.py`, `keysyms.py` | `keyboard` |
| KeychainService | 85 | `scripts/secrets.py`, gnome-keyring-daemon | |
| Launcher | 361 | §4 | |
| Lock | 131 | §4 | `lock` |
| Media | 32 | native Mpris | |
| Modes | 63 | §10 | `modes` |
| Monitor | 105 | `scripts/sysinfo.py`; kill, renice | |
| Network | 51 | native Networking (NetworkManager) | |
| NightLight | 49 | hyprsunset | |
| Notifications | 206 | native NotificationServer | `notifications` |
| Osk | 96 | wtype queue | `osk` |
| Phones, Snapshots, Vault, Vpn, Provider, VaultProvider, VpnProvider, ItemProvider | ~420 | provider contract (§8); `scripts/providers.py` discovery | `phone`, `snapshots`, `vault`, `vpn` |
| Pip | 56 | hyprctl clients and monitors; moves PiP windows | |
| Places | 179 | `scripts/userdirs.py`, `recents.py`, GTK bookmarks | |
| Power | 43 | native UPower and power profiles | |
| Prefs | 151 | `~/.config/isle/prefs.json` through a watched FileView; writes wait for `loaded` (D35) | `prefs` |
| Printers | 36 | `scripts/printers.py`, lp, lpoptions, cancel, `pkexec cupsenable|cupsdisable` | |
| Restore | 121 | §3.10 | `restore` |
| Session | 19 | systemctl, `hl.dsp.exit` | |
| SettingsIndex | 98 | page list | |
| SshKeys | 48 | `scripts/sshkeys.py` | |
| Startup | 55 | `scripts/autostart.py` (XDG autostart), `services.py` (systemd user units), hyprland.lua start-hook lines; dex | |
| Storage | 41 | `df -B1` | |
| Surfaces | 169 | open and closed state, mutual exclusion, the reopen note (D62) | `surfaces` |
| Switcher | 63 | §4 | `switcher` |
| System | 106 | /proc, nvidia-smi, sensors (for the System widget) | |
| SystemLocale | 70 | localectl, `scripts/locales.py` | |
| Terminal | 40 | dropdown kitty | `terminal` |
| Theming | 59 | `theme/render.py`, `--flags-only`, sun.py | |
| Tray | 17 | native SystemTray (StatusNotifierItem) | |
| Updates | 174 | checkupdates and `paru|yay -Qua` hourly; `pkexec pacman -Syu --noconfirm`, or the helper `--sudo pkexec`; output parsed per package (D47, D81); `scripts/restartneeded.py` (D46, D75) | `updates` |
| Users | 61 | `scripts/users.py` (AccountsService over D-Bus; `openssl passwd -6`); greetd autologin through `pkexec scripts/autologin.sh` | `users` |
| Widgets | 446 | §6 | |
| Windows | 447 | §3.6 | `windows` |
| Xkb | 23 | `scripts/xkb.py` (xkb rules XML) | |

---

## 8. Providers: password managers, VPNs, snapshots, phones (D42, D44, D45, D51)

A provider is a folder holding `provider.json {id, name, note, script}` and a script run as
`python3 <script> <command>`. The shell's own live in `shell/{vaults,vpns,snapshots,phones}/*.json`, and the
user's in `~/.config/isle/<kind>/<name>/provider.json`.

The commands are:

- **`status`** returns `{installed, ready, state, actions:[{id,label,input,terminal,primary,on,options,value}], error, active?}`.
- **`list`** returns vault items (titles only, never secrets), VPN choices, or snapshot and phone items, each with their own actions.
- **`field <shareId> <id> <field>`** is vault only and returns a single secret on demand.
- **`action <id>`** takes its input on stdin.

The shell renders the state sentence and the actions; it knows no tool's notions. The shipped providers are
Proton Pass (`protonpass.py` over pass-cli), gnome-keyring as a vault (`keyringvault.py`), Proton VPN
(`protonvpn.py`), NetworkManager VPN (`nmvpn.py`), Snapper (`snapper.py`, `ALLOW_USERS` set once through
pkexec), Timeshift (opens its window) and KDE Connect (`kdeconnect.py`). The Proton sign-in feeds the CLI over
stdin under setsid (D20). This contract is language-neutral, so it carries straight over to a rewrite.

---

## 9. Theming (`shell/theme/tokens.json`, `Theme.qml`, `theme/render.py`, `theme/templates/*`)

- **Tokens.** `tokens.json` holds colour (dark, plus a `light` override set), radius, space, font, motion,
  island, blur and icon. `Theme.qml` exposes them as typed properties and folds in the preferences
  (`accent`, `fontUi`, `fontMono`, `textScale`, `highContrast`, `theme`) and `motion = 0` in game mode or with
  reduced motion.
- **Themed apps.** `theme/render.py` (237 lines) substitutes tokens into templates and writes:
  - GTK 3 and 4 `gtk.css` and `settings.ini`
  - kitty `isle.conf` (with `include` in kitty.conf, then a reload)
  - btop `themes/isle.theme` (and `color_theme` set)
  - zathura `isle`
  - `xdg-desktop-portal/portals.conf` (FileChooser → isle, Settings → gtk, Secret → gnome-keyring)
  - `hypr/generated/theme.lua`
  - gsettings for libadwaita (colour scheme, theme, cursor, fonts)
- **Qt.** Qt 5 and 6 are themed through `QT_QPA_PLATFORMTHEME=gtk3` from the GTK theme (D39). qt6ct was removed.
- **Chromium and Electron flags files.** `render.py` scans the launchers on PATH for the `*-flags.conf` names
  they read and writes an Isle block to each: `--enable-features=UseOzonePlatform,NativeNotifications
  --ozone-platform=wayland --qt-version=6`. It re-runs when the set of desktop entries changes
  (`--flags-only`) and leaves a file alone if the user wrote their own ozone line.
- **Auto light and dark.** Auto is light between sunrise and sunset, computed from the timezone's tzdata coordinates (`theme/sun.py`).
- **Icons, cursor, fonts.** Papirus-Dark, Bibata Modern Classic 24, Inter and JetBrains Mono. Lucide glyphs are
  vendored as SVG bodies in `shell/assets/Icons.js` by `tools/vendor-icons.sh`; `Glyph` bakes the colour into an
  SVG data URL with the alpha as opacity, because SVG rejects `#AARRGGBB` (D12).
- **matugen was rejected (D16)** because fixed tokens with their own names do not map onto Material roles.

---

## 10. Modes (`services/Modes.qml`, 63 lines)

There are three states: `normal`, `game` and `bigpicture`. Focus mode is documented but not built (§2).

**Entering game or Big Picture:**

- Night light off, remembering whether it was on.
- Power profile to performance, remembering the previous one.
- `hyprctl eval hl.config({animations off, blur off, shadow off, gaps 0, border 0, allow_tearing = prefs.gameTearing, vrr = 1})`.
- `Theme.motion = 0`.
- The island is unmapped, with the hot zone left to peek it.
- Notifications are silenced and counted.
- Idle monitors are torn down (inhibited).
- Window places are handed to the plugin as an empty table.

**Leaving:** `hyprctl reload` restores the whole compositor profile from the files. Night light and the
power profile are restored, and "N notifications while you played" is shown.

**Triggers.** Game mode is automatic (§3.8), or toggled with Super+Shift+G or from the panel. Big Picture is
toggled with Super+Shift+B, follows Steam, or opens with a Guide hold when `prefs.padHome` is set.

**Pad bridge (D38).** While a mode is on and the front window's class is in `prefs.padBridgeApps` (Heroic,
Lutris, Spotify), pad presses become wtype keys. Games are never bridged. A Guide tap opens the quick menu, a
Guide hold opens home, and Select held opens the keyboard.

---

## 11. Keyboards (dual layout through a KVM)

**Rule (D7).** Hyprland sees one bind set, always on SUPER. xremap, per device, decides which physical chords
reach Hyprland as SUPER and which reach the app as CTRL.

**`shell/keymap.json` (696 lines)** is the single source. It has 50 `actions` `{id, label, hypr, win, mac,
ipc|lua|cmd, range, macRemap, macHypr, opts}`, 10 `media` keys (wpctl, brightnessctl, playerctl; `locked`,
some repeating), `macTerminalIds`, and `profiles.mac {text, control, terminal}`, the macOS modifier meanings as
xremap chords (D60). `prefs.keyTranslations` and `prefs.keymapOverrides` override entries.

**Rendering (`Keyboard.qml`).**

- **Hypr side.** One `hl.bind` per chord, with a duplicate-chord warning. Range binds are emitted as Lua loops.
  The release-commit binds for the switcher are generated here, as are a transparent Escape bound to
  `closeAll` and the media keys with `locked = true`.
- **xremap side.** On Mac-profile devices, Cmd+key becomes Ctrl+key except for chords the shell owns. The
  terminal preset applies in kitty and other terminal ids. Text-navigation and selection chords are
  translated (D58).
- **Profile per device.** It is auto from the name (Keychron, Apple, Magic Keyboard) or set by hand. With no
  Mac keyboard the file holds an empty keymap. It must never fall back to a stand-in device name, because a
  wrong name silently disables the whole remap (D58).
- **Symbols are resolved against XKB, never assumed (D61).** `scripts/keysyms.py` compiles keymaps with
  `xkbcli` for the keyboard's own layout (the pressed side, with the Mac variant) and the compositor's (the
  emit side). A symbol with no unshifted key drops the rule.
- **Re-rendering.** The config is rendered again whenever the set of Mac keyboards changes, because the first
  render happens before the device query answers and before prefs load (D58).

**Mac keyboard rules (D58).** Shell chords move off Cmd where the app needs Cmd: workspaces go to Ctrl+1..9
and Ctrl+←/→, overview to Ctrl+↑, and close-front, settings and terminal to Cmd+Option+W, comma and Return.

**Verification by measurement (D59).** `tools/keycheck.py` creates a uinput keyboard with a name the profile
matches, presses chords, and reads xremap's virtual device. Run it in the VM.

**Detection and hardware.** Settings › Keyboard Detect asks for two keys (the ISO key, and the key left of the
space bar) while the compositor sits in the `isle-record` submap. `scripts/keyboards.py` reads
`/proc/bus/input/devices` and udev to guess the hardware.

**Configurators.** `system/udev/70-isle-hidraw.rules` gives the user hidraw, STM32 DFU (0483:df11) and
Keychron (3434) for WebHID and WebUSB configurators. `scripts/webapp.sh` installs a Keychron Launcher web-app entry.

**Default chords** (`hypr` column; Mac equivalents in `main:docs/ARCHITECTURE.md` § Keymap):

| action | chord |
|---|---|
| launcher | Super+Space |
| switcher | Super+Tab, Alt+Tab (release commits) |
| cycle the app's windows | Alt+` |
| overview | Super+Ctrl+↑ |
| desktops | Super+n, Super+Ctrl+←/→ |
| move window to desktop n | Super+Shift+n |
| snap | Super+arrows (↓ restores) |
| close | Super+Q, Alt+F4 |
| close front window | Super+W |
| minimise | Super+M |
| fullscreen | Super+Shift+F |
| pin | Super+Alt+P |
| terminal | Super+Return |
| dropdown terminal | Super+` |
| files | Super+E |
| clipboard | Super+V |
| emoji | Super+. |
| capture | Print, Shift+Print, Super+Shift+S, Super+Ctrl+4, Ctrl+Print |
| record | Super+Shift+R |
| colour pick | Super+Shift+P |
| dashboard | Super+A, Super+N |
| settings | Super+, , Super+I |
| monitor | Ctrl+Shift+Esc |
| power menu | Super+X, Super+Esc |
| lock | Super+L |
| do not disturb | Super+Shift+D |
| game mode | Super+Shift+G |
| Big Picture | Super+Shift+B |
| on-screen keyboard | Super+Shift+K |
| zoom | Super+Ctrl+=, -, 0 |
| redetect displays | Super+Ctrl+D |
| log out | Super+Shift+Q |

---

## 12. Install, session, update

| piece | file | does |
|---|---|---|
| bootstrap | `tools/bootstrap.sh` 123 | from a fresh CachyOS: packages, paru and AUR, install.sh, PAM for the keyring, system services, groups; `--greeter`, `--no-bars` (hyprpm build) |
| install | `tools/install.sh` 195 | rsyncs the checkout to `~/.local/share/isle`, keeping `hypr/generated`, built plugins, `qml/` and userwidgets (D21). Builds `native/files` with cmake and ninja into `$ISLE_HOME/qml`. Links `~/.config/quickshell/isle` and `~/.config/hypr/hyprland.lua`, and `shell/userwidgets`. Renders themes. Writes desktop entries (`isle-files.desktop`, which is made the `inode/directory` handler; Keychron Launcher). Enables `gcr-ssh-agent.socket` and `xremap.service`. Runs the root half once if it is due. Restarts xdg-desktop-portal if the portal file changed. Kills `qs` whenever a session is up, because rename-replaced files defeat Quickshell's inotify reload and a native module is never reloaded (D84, D86). `--pam` adds `pam_gnome_keyring` to greetd and login |
| root half | `tools/isle-root.py` 294 | one sudo or pkexec for: missing packages from `shell.txt` (`pacman -T`, then `pacman -Si` to drop the unresolvable, no `-y`, D83); nethogs setcap; the portal file; the hidraw rule; the greeter copy; the hyprpm plugin rebuild. `--due` answers as the user. For hyprpm it forks to the user with `tools/pkexec-as-sudo/sudo` on PATH and serves hyprpm's eight escalations over a 0600 socket, matched against allowed shapes (D79) |
| update | `tools/update.py` 267 | `check` fetches and counts commits behind. `run` pulls fast-forward, runs install.sh, and writes progress to `$XDG_RUNTIME_DIR/isle/update.json`. It is started with `execDetached(["setsid","--fork",…])` because a reload kills child Processes (D78). It is Python because bash would execute the seam of a file pulled mid-run. Exit codes decide failure. Plugins are hashed against the Hyprland headers in `~/.cache/isle/plugins-built` |
| tidy | `tools/tidy.sh` 66 | lists and removes what an earlier Isle installed that this one no longer uses |
| greeter | `tools/install-greeter.sh` 46, `greeter/shell.qml` 121 | root-owned copy at `/usr/local/share/isle-greeter`; `/etc/greetd/config.toml` runs `start-hyprland -- -c /etc/greetd/hyprland.lua`, which starts `qs -p …/greeter`; Quickshell Greetd; fingerprint-first when `pam_fprintd` is in greetd's stack (D34) |
| fingerprint | `tools/fingerprint.sh` 56 | fprintd and drivers (the goodix53x5 AUR driver for 5335, 5385, 5395), enroll, optional sudo PAM (D33) |
| session loop | `shell/scripts/isle-session` 48 | started from the compositor start hook, so it is in the logind session scope (D17). Sets `QT_QPA_PLATFORM=wayland` and `QML_IMPORT_PATH=$ISLE_ROOT/qml`. One per session via `$XDG_RUNTIME_DIR/isle.pid`. Restarts `qs` after 1s, and after six quick deaths every 30s rather than giving up, so a locked session gets its locker back (D43). Stops when the Hyprland socket is gone. Logs to `~/.local/state/isle/session.log`, keeping the previous log as `.1`, with a link at `$XDG_RUNTIME_DIR/isle.log` |
| restart owed | `shell/scripts/restartneeded.py` 77 | the running kernel's module tree is gone; the NVIDIA module version is not nvidia-utils'; or a compositor plugin's mapped inode in `/proc/<Hyprland>/maps` is not the file on disk (D46, D75). `hyprpm reload` segfaults Hyprland when a plugin holds function hooks |

**State and file map (for migration).**

- `~/.config/isle/`: `prefs.json` (every preference, about 80 keys), `notifications.json`, `xremap.yml`,
  `widgets/`, `pins/` (clipboard pins), `cleaner.json`, `{vaults,vpns,snapshots,phones}/`.
- `~/.local/state/isle/`: `session.json` and `session.log`.
- `~/.cache/isle/`: `widget-sources/`, `calendars/`, `plugins-built`.
- `$XDG_RUNTIME_DIR/isle/`: `locked`, `reopen.json`, `update.json`, the restore marker.
- `$XDG_RUNTIME_DIR/`: `isle.pid`, `isle.log`, `isle-freeze/`, `isle-capture.log`, `isle-sysinfo.json`.
- Elsewhere: `~/.config/gtk-3.0/bookmarks` (shared with GTK, D69) and `~/.cache/thumbnails` (freedesktop, D68).

---

## 13. Test hooks and VM tooling assumptions

- **IPC is the test surface.** `qs -p ~/.config/quickshell/isle ipc call <target> <fn> [arg]`. The targets are
  island (`text`, `pin`, `notifications`, `clear`, `dnd`, `demo osd|media|text`), surfaces, switcher, windows,
  lock (`locked` is used by the dev sync so it never restarts a locked shell), modes, bigpicture, capture,
  displays, keyboard, notifications, osk, gamepad, terminal, access, privacy, prefs, restore, updates, isle,
  users, vault, vpn, snapshots and phone. Keybinds, hyprbars buttons, `isle-files`, `isle-quick.sh` and
  capture.sh all reach the shell through this same CLI.
- **Local harness (`dev/`, git-ignored, D22).**
  - `test-vm.sh` runs a CachyOS QEMU guest with the repo at `/repo` over 9p, which has no inotify, so the
    shell must be killed to pick up changes. `shot` takes a screenshot through QMP.
  - `guest.sh` runs a command with the session environment.
  - `vm-qmp.py` sends move, click, drag, type and key input. Move first, then click, for menus.
  - `install-guest.sh`, `provision-guest.sh` and `guest-baseline.sh` set up the guest.
  - `live.sh` syncs on save.
  - `fake-gamepad.py` is a uinput pad. `seed-games.py` and `testreg/` support Big Picture and widget-registry tests.
- **Guest rules.** Restart the shell with `pkill -x qs` (the memory note says `quickshell` in some setups) and
  check its elapsed time before trusting anything. Polkit-gated actions must run inside the session through
  `hl.dsp.exec_cmd`. Never restart the shell while the session is locked (D15).
- **Headless checks.** `QT_QPA_PLATFORM=offscreen /usr/lib/qt6/bin/qml -I ~/.local/share/isle/qml file.qml`
  checks the engine headlessly.
- **What the VM cannot show.** It has no hardware cursor plane (D27), uses llvmpipe so its memory figures are
  not a baseline (D13), and has no USB or hotplug. Removable media is tested with `udisksctl loop-setup`.
- **Crash reports.** `~/.cache/hyprland/hyprlandCrashReport*.txt`. The known signatures are Screenshare init
  on a vanishing output (fixed, D55), `renderHWCursorBuffer` on layout change (open) and `doLaterLock` (open,
  suspected in the overnight lock deaths).

---

## 14. Visual design language (carry-over candidate)

**Principles.** Nothing at rest except the wallpaper and the island. One material. Surfaces grow from the
island rather than popping in. The accent is a signal, not a colour scheme. Installed apps wear the same skin.

**Colour, dark** (`tokens.json` `color`):

| token | value |
|---|---|
| ink | `#0A0B0D` |
| glass | `#121316` at 0.78 with blur, 0.94 without |
| raised | `#1B1C21` |
| pressed | `#26272D` |
| hairline | text at 0.08, strong 0.14 |
| text | `#F2F2F3` |
| text-2 | text at 0.62 |
| text-3 | text at 0.38 |
| accent | `#7FA6FF`, user-settable |
| on-accent | `#0A0B0D` |
| ok | `#6FCF97` |
| warn | `#F2C94C` |
| danger | `#FF6B6B` |
| live | `#FF453A` |

Light mode overrides these: ink `#F4F4F5`, glass white at 0.72, text `#16171A`, hairline 0.10, accent
`#3F6FD6`. High contrast lifts text-2 to 0.88, text-3 to 0.70, hairlines to 0.22 and 0.36, and glass to 0.97.

**Geometry.** A 4pt grid with spacing 4, 8, 12, 16, 24 and 32. Radii: pill full, panel 20, card 14, control 10,
chip 8. Hairlines are always 1px. There is one shadow, `0 12px 40px rgba(0,0,0,.45)`, for floating surfaces.

**Glass and blur.** The compositor blurs layers behind the glass (size 8, 3 passes, ignore alpha 0.2, brightness
0.9, vibrancy 0.15, noise 0.01). Windows use the same material without blur, so menus and cards there use the
raised tone. In game mode blur is off and glass goes to 94%.

**Type.** Inter at 11, 12, 13 (body, weight 500), 15, 20 and 28, in weights 400, 500 and 600. JetBrains Mono
for numbers and code. Tabular figures for anything that changes. Sentence case everywhere, with no uppercase
tracking labels. A text-scale factor applies to every size.

**Motion.** OutQuint to enter and InQuint to exit, with exits faster. Durations are quick 140ms, move 220ms and
morph 360ms. A spring (180/22) is specified for the island, but only Hyprland windows use it. Enter is a fade
with scale 0.96 → 1 from the anchor; exit is a fade only. Reduced motion and game mode set every duration to 0.

**Icons.** Lucide at stroke 1.5 on the 24 grid, scale 1.15, with glyph boxes of 10, 14, 16 and 17–20, rasterised
at 2×. App icons are Papirus-Dark at 20, 32, 48 and 96. The cursor is Bibata Modern Classic 24.

**Components** (`shell/ui/`, 1,445 lines): Glass, Glyph, Label, Toggle (36×20), Slider (4px track, 16px knob),
Segmented, Field, ListRow (40px), Card, Button (text, raised, accent, danger), Dropdown, NumberField, Scrollbar,
Spinner, Sparkline (Shapes, not Canvas), TrayIcons, TrayMenu, SettingsPage, SettingsGroup, SettingsRow,
ProviderGroups, DisplayMap, PadLegend, NotificationCard and WidgetHost. Every control has hover, pressed, focus
and disabled states. The focus ring is 2px accent at 2px offset, for keyboard and controller only (D9).

**Sizes.** Island 30px at rest, panel 420 wide with a 44px header. Launcher 640. Switcher cards 240×150.
Big Picture tiles 176×264 with radius 20. Quick menu 480 with 56px rows. File chooser at most 880×620.
Files rows 30px, grid cells 116px, toolbar 56px. Hyprbars 28px.

**Copy rules.** "Desktop", never "workspace", in the UI. A page subtitle is one sentence. No instructions under switches.

---

## 15. External dependencies

**Packages from `packages/shell.txt`:**

- **Compositor and shell:** hyprland, quickshell, qt6-declarative, cmake, ninja, gcc, xdg-desktop-portal-hyprland,
  xdg-desktop-portal-gtk (kept for the Settings portal, D73), kitty, polkit, xdg-utils, xdg-user-dirs, dex,
  zenity, glib2, python, python-gobject, python-pillow.
- **Sound, network, devices, power:** pipewire, pipewire-pulse, wireplumber, networkmanager, bluez, bluez-utils,
  upower, power-profiles-daemon, udisks2, brightnessctl, ddcutil, gnome-disk-utility, hyprsunset, accountsservice.
- **Keyring and SSH:** gnome-keyring, libsecret, gcr-4, openssh, openssl.
- **Launcher, clipboard, capture:** fd, libqalculate, cliphist, wl-clipboard, grim, slurp, wf-recorder, satty,
  hyprpicker, wtype, words.
- **Files:** mousepad, libarchive (bsdtar), squashfs-tools, fuse2.
- **Monitor and games:** btop, nethogs, lact, gamemode, lib32-gamemode, gamescope.
- **Printing and updates:** cups, pacman-contrib.
- **Look:** inter-font, ttf-jetbrains-mono, papirus-icon-theme, adw-gtk-theme, qt6-wayland, qt5-wayland.
- **AUR and extra:** xremap-hypr-bin, bibata-cursor-theme, coolercontrol, proton-vpn-cli, ripgrep-all,
  unicode-emoji, python-icalendar, python-recurring-ical-events, rsync, zathura, zathura-pdf-mupdf.
- **Optional:** nvidia-utils, steam, heroic-games-launcher-bin, lutris.

**Used but not listed:** playerctl (media keys), libnotify `notify-send`, curl, ripgrep `rg` (fallback),
`xkbcli` (libxkbcommon tools), fprintd, kdeconnect, snapper, timeshift, pass-cli, paru or yay, `gh` (a gh
sign-in row for https remotes), greetd, hyprpm (ships with hyprland).

**Daemons and buses the shell talks to:**

| bus or daemon | interface used | how |
|---|---|---|
| D-Bus session | `org.freedesktop.Notifications` | serves it (Quickshell) |
| D-Bus session | StatusNotifierWatcher and Items | tray (Quickshell) |
| D-Bus session | MPRIS | native |
| D-Bus session | `com.feralinteractive.GameMode` | `gdbus monitor` |
| D-Bus session | Secret Service (gnome-keyring) | through libsecret in Python |
| D-Bus session | `org.freedesktop.impl.portal.desktop.isle` | **serves** FileChooser |
| D-Bus session | KDE Connect | through its CLI and properties |
| D-Bus system | `org.freedesktop.login1` | Lock and Unlock monitor; systemctl and loginctl |
| D-Bus system | NetworkManager, BlueZ, UPower, power-profiles | native Quickshell |
| D-Bus system | udisks2 | udisksctl, and gdbus for SmartUpdate |
| D-Bus system | AccountsService | users.py |
| D-Bus system | polkit | serves the authentication agent |
| D-Bus system | fprintd | through its CLIs |
| D-Bus system | snapperd | snapper CLI |
| D-Bus system | timedate1, locale1 | timedatectl, localectl |
| PipeWire | native nodes, `pw-mon`, `pw-cli`, `pw-play`, `wpctl` | |
| Hyprland | IPC socket (Quickshell.Hyprland), event socket (raw events incl. `isle-ghost`), `hyprctl` incl. plugin command `isle` | |
| Wayland protocols | wlr-layer-shell, ext-session-lock, ext-idle-notify, hyprland-toplevel-export / screencopy, foreign-toplevel, virtual-keyboard (via wtype), xdg-decoration (read by hyprbars) | |
| Other | LACT socket `/run/lactd.sock`; CoolerControl REST `:11987`; greetd socket (greeter) | |

---

## 16. Hard-won lessons a rewrite must not lose (index)

These are grouped by subject. Most stay true whatever the compositor, because they are facts about Wayland,
X11 clients, NVIDIA, KVMs, Steam, polkit and Arch.

- **Session and polkit.**
  - D17: the shell must live in the logind session scope; from a systemd user unit, NetworkManager, udisks and logind refuse or demand admin.
  - D77, D79: one authorisation per install; hyprpm refuses root, runs `sudo -k` and escalates eight times.
  - D47: pkexec plus pacman's own lines replace a terminal.
  - D83: no partial upgrades.
- **Lock.**
  - D15, D43: a locker death strands the session. Keep a `locked` marker, relock at start, allow lock restore, back off rather than stop the restart loop, and log to a persistent directory.
  - D33, D34: fingerprint runs beside the password on the lock, and through PAM at the greeter.
- **Idle and DPMS.**
  - D18: an IdleMonitor with a changed timeout never fires; one with a timeout of 0 fires at once.
  - D76: re-send the DPMS state when the screen list changes; wake outputs before releasing them.
- **Monitors and KVM.**
  - D56: release outputs and reload; a renderer reload does not help.
  - D76: watch the real screen list, not the compositor's monitor list, which keeps stale entries; heal FALLBACK every 5s.
  - D57: never record or restore an off-screen place; rescue stray windows.
- **Capture.**
  - D55: capture a window only while its preview is visible; a capture against a vanishing output crashes Hyprland.
  - D27: hardware cursors keep the pointer out of screenshots, at the cost of an open NVIDIA crash.
  - Freeze before the pick, so the picker's veil never reaches the shot.
- **Windows and X11.**
  - D40: place only an app's sole titled window; untitled X11 windows are menus.
  - The plugin answers `_NET_WM_MOVERESIZE` and client minimise requests.
  - D91: placement must happen at map, before the open animation.
  - D48: fullscreen, including borderless, gets its own workspace.
  - Close by address, never through a dying toplevel handle.
  - D63: raise after a layer surface releases the keyboard.
- **App identity.** D92, D75: one process means one app id; per-window ids need a per-window override set before map.
- **Keyboards.** D58, D59, D60, D61: render after the devices and prefs load; never use a fallback device name; resolve symbols through XKB on both sides; derive Mac chords from actions; verify by measurement (keycheck).
- **Switcher.** A plain release bind is suppressed after Super+Tab, so commit needs a non-consuming, transparent release bind (`Keyboard.qml`).
- **Updates and reloads.**
  - D78: an update must not be a child of the UI process it rewrites.
  - D84, D86: rename-replaced files defeat inotify reloads, and native modules never reload, so always restart.
  - D46, D75: say a restart is owed after kernel, NVIDIA or plugin skew; `hyprpm reload` segfaults with hooks.
- **Module boundaries.**
  - D72: a crash-prone dependency must not sit in a module everything imports.
  - The memory note: name collisions in one import scope take the whole shell down.
  - D66: Rust-side work faults into a restartable shell, not into the compositor.
- **Files.**
  - D68: use the shared thumbnail cache and validate it from PNG text chunks.
  - D70, D71: trash per volume; ordering never loses the old file; undo a partial job.
  - D69: bookmarks are GTK's.
  - D87, D88, D89, D90: Cleaner scan and rules semantics.
- **Steam.** D38: whoever is fullscreen owns the pad. Steam rewrites `shortcuts.vdf` on exit. Big Picture is minimised rather than closed. Steam Input presents virtual pads (D54).
- **Theming.** D39: Qt themes through the GTK3 platform theme. Chromium needs `--qt-version=6` for theming on Wayland. D16: tokens are not Material.
- **Third-party code.** D29, D31: git-repo registries, consent per permission, the host fetches on a widget's behalf. D42: secrets are fetched per field, never cached.

---

## 17. What is Hyprland-specific vs. portable (inventory only)

| tied to Hyprland or Quickshell | portable as is (data, scripts, contracts) |
|---|---|
| `hyprland.lua` and the four generated fragments; Lua dispatcher strings across about 15 files; the `hyprctl isle` plugin command; the `isle-ghost` event; the hyprbars fork; the X11 hooks in isle-windows; `allow_session_lock_restore`; `hl.gesture`; submaps; `cursor.zoom_factor`; hyprsunset; hyprpm and its root dance; restartneeded's plugin-inode check; the binds.lua generator; `xremap-hypr` (per-app remaps through the Hyprland backend) | `keymap.json` (actions, profiles); `tokens.json`; the provider contract and its 7 scripts; `rules.json` for the Cleaner; the widget manifest schema and registry index format; 40 Python helper scripts that print JSON (games, sysinfo, devices, calendars, emoji, appimages, keysyms, keyboards, users, secrets, sshkeys, defaults, autostart, services, steamshortcuts, gamepad, fingerprint, syscleanup, …); theme templates for GTK, kitty, btop, zathura and portals; `native/files` logic (C++/Qt, so its algorithms port but its types do not); prefs.json keys; the file-state map in §12 |
| everything in `shell/` is QML on Quickshell types; the widget sandbox and registry widgets are QML source | |
