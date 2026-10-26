# Roadmap

The build order for the rust-desktop program. Each entry is a build
packet's brief: what it delivers, what it depends on, and the decisions and
intent documents it answers to. A packet is planned in detail, with its own
PRD and phases, by its own run of feature-plan when it starts, against the
code as it is then. Intent: `docs/design/` (compositor, shell, gaming,
apps, platform). Decisions: `docs/DECISIONS.md` (B1–B91).

Everything happens on `feature/rust-desktop` (B16); v1 stays the daily
desktop on `main` until milestone M2.

## Before the first packet

- **The Freya fork at rc.8.** Done 2026-10-04: the user merged upstream
  v0.5.0-rc.8 into `alexparlett/freya`.

## Milestone M1: a desktop that runs in the VM

**P1 · Design.** Done 2026-10-03: `docs/DESIGN.md`, B31–B34, working files in `p1/`. A full design process for the new Isle (B3, B17), run as
its own packet with the user deciding at each gate; it is where every
presentation question in the intent docs is answered (each doc's "For the
design process" list, and B24). Its stages:

1. **Research.** macOS as it is today (macOS 27, Apple's HIG), the
   desktops worth learning from, v1's lessons about its own surfaces, and
   the user's taste and daily use, gathered by asking.
2. **Structure.** For every surface (island, menu bar, Control
   Centre, Notification Centre and widgets, Spotlight, switcher, Mission
   Control, lock and greeter, prompts, capture, title bars and app
   toolbars, OSDs): what it is for, its content, its states, what wins
   when things compete for the same place.
3. **Directions.** Two or three distinct visual directions as mock-ups of
   the same few surfaces; the user picks one or combines.
4. **System.** Principles, materials (glass, blur, depth), typography,
   colour, spacing, radii and motion as `tokens.kdl`; the component set
   the shell and the apps share.
5. **Surfaces.** Each surface designed in the chosen direction, with its
   states and motion; prototyped live in the Freya preview mode once P3
   provides it.
6. **Review.** Critique, accessibility (contrast, keyboard, reduced
   motion) and copy; `DESIGN.md` written as the record.

Runs beside P2 and P3. Surfaces are built from P4 onwards against its
output; a surface whose design is not done is built as structure only.
Intent: shell.md, apps.md, compositor.md, gaming.md.

**P2 · Compositor core.** Done 2026-10-04: `p2-compositor.md`, B35 and B36. The Cargo workspace; the pinned Smithay fork (B6);
udev/DRM and winit backends; libinput; outputs with hotplug, scale and the
KVM lessons; xdg-shell and layer-shell; the calloop loop, service worker,
watchdog and session supervisor (B9); the settings store and its D-Bus
interface (B10) with `settings.kdl`, per-app sections and writes refused
while it does not parse (B27), and the state store; the `isle` CLI and the
test interface; the NVIDIA VRAM profile. Ends booting in the VM with a
terminal open. Intent: compositor.md, platform.md.

**P3 · Shell runtime.** Done 2026-10-04: `p3-shell-runtime.md`, B37–B39. The Freya embedding from the spike, moved into the
fork as a crate: surface hosting on the compositor's GL context, input,
the accessibility tree, panic boundaries per surface, compositor-driven
transitions, the Skia cache cap, live `tokens.kdl`; and a preview mode that
runs a surface in `freya-winit` with hot reload, to win back design
iteration (B7's condition). Depends on the fork at rc.8. Intent: shell.md.

**P4 · Windows.** Done 2026-10-05: `p4-windows.md`, B40–B43. Stacking with snapping and the ultrawide zones (B8);
app-centric switching mechanics and hide-on-last-close policy, with Cmd+Q
quitting keep-running apps (B20, B30); minimise, maximise, full-screen
Spaces, window places; server-side title bars drawn in Freya (B25); the
Xwayland window manager with v1's X11 lessons (B5). Intent: compositor.md.

**P5 · Keyboard.** Done 2026-10-05: `p5-keyboard.md`, B44–B46. Per-device profiles and the Mac preset in the
compositor, with the Alt and Super swap on PC keyboards on by default
(B30); Cmd translation with per-app exceptions, off in games and under
shortcuts-inhibit (B21); the default shortcut set. Retires xremap. Intent:
compositor.md.

**P6 · Island and menus.** Built 2026-10-06, host run pending:
`p6-island-and-menus.md`, B47–B52. The island on every screen (B18): the clock,
the status glyphs, live activity as P1 designs it (B24), and Control
Centre's panel opening from it, whose modules and tray fill in with P8.
The menu bar on demand from the top edge and Ctrl+F2, with the three menu
sources and the registrar owned from session start, the fallback app menu
and menu search. The wallpaper. Intent: shell.md, platform.md,
compositor.md.

**P7 · Switcher and launching.** Done 2026-10-07: `p7-switcher.md`, B53–B57. Cmd+Tab and Cmd+` (B20), the switcher's
cards carrying what a Dock would (B19): badges, progress, attention, and
hidden, minimised and background marks. Launching from Spotlight's Apps
grid with its favourites row is built with Spotlight in P10. Intent:
shell.md.

**P8 · Services, notifications and Control Centre.** Done 2026-10-07:
`p8-services.md`, B60–B65, B11 revised. The zbus service
crates (B11): network, Bluetooth, power, audio through pipewire-pulse,
media, disks; the notification server and tray (B12); notification
banners, OSDs and Control Centre as P1 designs them (B24); Focus modes
(B30), Game Mode among them with its chord and the profile hold;
Notification Centre's notifications, moved from P17 at P8's review, its
agenda waiting for calendars. Intent:
platform.md, shell.md.

**P9 · Lock and prompts.** Done 2026-10-09: `p9-lock-and-prompts.md`,
B66–B72, B12 revised. The compositor's own lock with PAM in a helper
process and v1's locker lessons, with no ext-session-lock served (B28); the
polkit agent, keyring prompter and NetworkManager secret agent (B12) and
the Bluetooth pairing agent (B28) with their dialogs, behind the Wi-Fi and
Bluetooth rows that fail in P8 for want of them; idle, dim and DPMS, with
idle and the lock among the holds on banners; the session's end and Force
Quit. Fingerprint waits for a reader.
Intent: shell.md, platform.md, compositor.md.

**P10 · Spotlight.** Done 2026-10-09: `p10-spotlight.md`, B73–B79. Launcher with apps, the Apps grid with its favourites
(B19), windows, settings, the front app's menu items (B18), actions and
v1's prefixes and sources. Games: the rest of `gaming.md`'s Game library on
P7's `isle-games` (Heroic, Lutris, gamescope's game, launching through the
launchers, B56), and badges on the Apps grid's tiles from `Windows1.Apps`.
Clipboard history and emoji, moved from P20 at P10's review; the vault stays
with the providers in P20.
Intent: shell.md, gaming.md, platform.md.

**P11 · Capture and portals.** Done 2026-10-09: `p11-capture.md`, B80–B85.
The screenshot tool with its freeze, toolbar, thumbnail, colour pick and
recording through `isle-record`; PipeWire streams from the compositor;
`isle-portal` with ScreenCast and its picker, Screenshot, Access, Settings and
Inhibit; ext-image-copy-capture with ext-foreign-toplevel-list; the picker a
sheet on the asking window through xdg-foreign. Brief: `xdg-desktop-portal-isle` (B13), a bus name
on the compositor's service worker (B26): ScreenCast and Screenshot with
PipeWire streams from the compositor and the shell's picker, the screenshot
tool, ext-image-copy-capture; FileChooser follows in P15. A capture of a
locked screen shows the lock and a window share holds its last frame; the
portal's Inhibit joins idle's holds (from P9). Intent: platform.md,
shell.md.

**P12 · Gaming.** Built 2026-10-10, host run pending: `p12-gaming.md`,
B86–B91. The async page flip patch on the Smithay fork and tearing
control tested on the RTX 5070 Ti (B6); VRR on while a full-screen app is
in front, with its setting (B30), and direct scanout for games; game
detection, Game Mode as the automatic Gaming Focus and the escape set of
chords over a game (B30); Steam, its Big Picture as a full-screen game
(B22), and nested gamescope. Game Mode as the Gaming Focus came in P8
(B64); P12 refines game detection. Night light, the colour transform on each
output (GAMMA_LUT or a shader), with Control Centre's Night Light module
and Game Mode turning it off (given here at P8's review). Intent:
gaming.md, compositor.md.

**P13 · Settings, core.** Built 2026-10-17, host run pending:
`p13-settings.md`, B94–B99. Every Isle app's platform in `isle-app`;
Settings' 21 panes on the gated rows, the compositor's lights over the app,
display changes held and reverted by the compositor, the blur behind a
window, the hot-corner engine, View › Show Changed Settings. Brief:
Settings › Privacy & Security lists and revokes
the permission store's `screenshot` and `screencast` entries, as `isle portal
permissions` does since P11. The Settings app on the app conventions: displays,
sound, network, Bluetooth, keyboard and shortcuts, appearance, desktop and
hot corners, notifications, Control Centre, Focus and Game Mode, over the
settings P8 added; the modules' "Settings…" links open them. Measures an app's memory on the GPU before
B26's default is final. A display mode change reverts unless confirmed in
the Displays pane (moved from P2). Settings ships the row index Spotlight
reads (`data/settings/index.kdl`, empty since P10) and opens at a pane and
row from `isle-settings://<pane>/<row>`; Settings › Spotlight edits the
`spotlight` section (favourites, file folders, clipboard exclusions, the
web search). Settings › Displays draws each output's VRR and tearing and the
`night-light` section (schedule, from and to, warmth); Focus › Game Mode or
Windows marks an app a game or never one, as `isle windows game` does since
P12. Intent: apps.md.

## Installed beside v1

**P14a · Installed, beside Hyprland.** Built 2026-10-19, host run pending:
`p14a-session.md`, B100–B103.
P14 split at the user's choice: Isle installed and chosen at sign-in before
the parity packets, the switch after them (P14b). The package (`isle-git`,
a PKGBUILD from git, B29) with the six binaries, Settings in software
(B26), the session entry, `isle.portal` and `isle-portals.conf`, Settings'
desktop entry and D-Bus service file, `/usr/lib/pam.d/isle`, the NVIDIA
profile and an i2c `uaccess` rule; the binaries carrying their data and
their helpers pinned to the running build. tuigreet as greetd's greeter
until P14b, listing Hyprland and Isle. Isle beside v1: the user's
`portals.conf` (preferring Hyprland's backend, and outranking the packaged
file) becomes `hyprland-portals.conf`; xremap.service, wanted by
graphical-session.target, runs only outside Isle; the supervisor unsets
only the session variables still holding its values; the lock marker per
logind session; Isle's logs out of the path v1 writes (both found in P4's
host run). Login Items' engine over XDG autostart, launched into the
session scope. `isle setup` with check, apply and undo, and P14a's steps.
The dev loop that replaces `isle-dev-sync` for the new Isle: the checkout's
HEAD packaged and installed. Ends with the host signing into Isle from
tuigreet, and P12's and P13's host runs made on the installed build.
Intent: platform.md, compositor.md.

## Parity

**P15 · Files.** Built 2026-10-21, host run pending: `p15-files.md`. The capture thumbnail's click opens Quick Look with Markup,
and its menu gains Show in Files and Markup; FileChooser joins `isle-portal`
(both left out in P11). Finder-shaped Files with v1's browsing-engine lessons; the
Open and Save panels, handed FileChooser by the portal backend, and Quick
Look as a D-Bus service Spotlight calls, all in the Files process (B26).
Files owns `org.freedesktop.FileManager1`, which Spotlight's Show in Files and
Cmd+Return on a file already call (P10), and a search that Spotlight's "Show
all in Files" and Cmd+Y on a file reach. Intent: apps.md, platform.md.

**P16 · Mission Control.** Planned 2026-10-26: `p16-mission-control.md`.
Mission Control on every screen with its Spaces bar, spread and tray, App
Exposé, Show Desktop, Mission Control's actions in the hot corners (the
engine, its four actions and Settings' Hot Corners… sheet came in P13,
B99) with mouse buttons as routes, Split View; Stage Manager as a layout
module, off by default (B25), last and cuttable to a packet of its own.
Trackpad gestures and Settings' Trackpad pane moved to the Trackpad packet
at P16's review, since the host has no trackpad. Intent: shell.md,
compositor.md.

**P17 · Widgets.** Notification Centre moved to P8 at its review. Widgets
where P1 places them, with the grid editor and the gallery (B24); the provider contract (B14), sandboxed with granted
permissions (B28); the isle-widgets registry as providers. Intent:
shell.md, platform.md.

**P18 · Controllers.** Pad detection and naming, batteries in Control
Centre, Settings › Controllers, idle held off by pad play (a hold on P9's
idle policy). Intent:
gaming.md, apps.md.

**P19 · Monitor, Keychain, Cleaner.** Intent: apps.md.

**P20 · The rest of v1.** Settings' remaining pages (General's Software
Update, Storage, Passwords, Date & Time, Language & Region, Login Items and
Snapshots; Users & Groups, Login Password, Calendars, Printers, Phone;
TCP/IP and DNS editing; P13's Not in P13), session restore
with the session dialog's "Reopen windows when logging back in" and the
crash notice for apps (from P9),
calendars with Notification Centre's agenda, the providers (vault, with
Spotlight's `*` scope, VPN, snapshots, phone), the dropdown terminal and picture-in-picture, the on-screen
keyboard (on the lock too), and an audit against the inventory that nothing of v1 is left.
Intent: all.

## Milestone M2: Isle replaces v1 on the host

**P14b · The switch.** The compositor's greeter mode under greetd (B26),
the lock's face with its user list and session menu, drawing the look the
user's session publishes for it (where: `session-2026.md` §1), replacing
tuigreet; greetd's PAM service as Isle's own file. The one-time import of
v1's preferences, window places and launch counts (B27). Theming installed
apps from tokens as P1's `system/theming.md` maps them (`session-2026.md`
§7 and §8), with Qt apps reaching Isle's file panels (§6). The rest of
`isle setup`: keyring and fingerprint PAM lines, autologin, groups, the
bootstrap command. The update path through paru and pacman, and restart
owed. Scan-out of a second GPU's connectors (moved from P2; §2), if a
screen is on the iGPU by then. v1's user units disabled. Ends with the host
booting into the new Isle. Intent: platform.md, compositor.md.

**P21 · Cut-over.** Remove v1's tree (`shell/`, `hypr/`, `plugins/`,
`native/`, `theme/`) and its tooling; this branch becomes `main`.

## After cut-over

**Trackpad.** Started when a trackpad is used; from P16 at its review.
libinput's swipe, pinch and hold events claimed or forwarded through
pointer-gestures; the desktop switch and Mission Control's opening held by
the fingers and committed by projected end (`mission-control-2026.md` §2,
§4); swipes between desktops with three or four fingers, App Exposé, Show
Desktop and the Apps view by pinch; Settings' Trackpad pane with the
touchpad's tap and scroll-method rows. The edge swipe for Notification
Centre stays out: libinput's gestures carry no position. Intent: shell.md
Mission Control 9, compositor.md Pointer 6–7.

**P22 · HDR.** Shots, streams and the colour pick from HDR outputs (P11's
are 8-bit sRGB). Night light shares the CRTC's colour pipeline with HDR
(P12 writes `GAMMA_LUT` alone). Colour management from the community Smithay fork's work
(B23): the colour-management protocol, HDR metadata to the output, SDR
content composited beside HDR, the NVIDIA colour pipeline, HDR through
nested gamescope; tested on the host's driver first. Intent:
compositor.md, gaming.md.
