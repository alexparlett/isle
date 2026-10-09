# Roadmap

The build order for the rust-desktop program. Each entry is a build
packet's brief: what it delivers, what it depends on, and the decisions and
intent documents it answers to. A packet is planned in detail, with its own
PRD and phases, by its own run of feature-plan when it starts, against the
code as it is then. Intent: `docs/design/` (compositor, shell, gaming,
apps, platform). Decisions: `docs/DECISIONS.md` (B1–B79).

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

**P11 · Capture and portals.** `xdg-desktop-portal-isle` (B13), a bus name
on the compositor's service worker (B26): ScreenCast and Screenshot with
PipeWire streams from the compositor and the shell's picker, the screenshot
tool, ext-image-copy-capture; FileChooser follows in P15. A capture of a
locked screen shows the lock and a window share holds its last frame; the
portal's Inhibit joins idle's holds (from P9). Intent: platform.md,
shell.md.

**P12 · Gaming.** The async page flip patch on the Smithay fork and tearing
control tested on the RTX 5070 Ti (B6); VRR on while a full-screen app is
in front, with its setting (B30), and direct scanout for games; game
detection, Game Mode as the automatic Gaming Focus and the escape set of
chords over a game (B30); Steam, its Big Picture as a full-screen game
(B22), and nested gamescope. Game Mode as the Gaming Focus came in P8
(B64); P12 refines game detection. Night light, the colour transform on each
output (GAMMA_LUT or a shader), with Control Centre's Night Light module
and Game Mode turning it off (given here at P8's review). Intent:
gaming.md, compositor.md.

**P13 · Settings, core.** The Settings app on the app conventions: displays,
sound, network, Bluetooth, keyboard and shortcuts, appearance, desktop and
hot corners, notifications, Control Centre, Focus and Game Mode, over the
settings P8 added; the modules' "Settings…" links open them. Measures an app's memory on the GPU before
B26's default is final. A display mode change reverts unless confirmed in
the Displays pane (moved from P2). Settings ships the row index Spotlight
reads (`data/settings/index.kdl`, empty since P10) and opens at a pane and
row from `isle-settings://<pane>/<row>`; Settings › Spotlight edits the
`spotlight` section (favourites, file folders, clipboard exclusions, the
web search). Intent: apps.md.

## Milestone M2: Isle replaces v1 on the host

**P14 · Session and install.** The PKGBUILD built from git, installed with
paru and updated through pacman (B29); the compositor's greeter mode under
greetd (B26) and session start, Login Items and autostart, crash logs, the
update path, the dev loop that replaces `isle-dev-sync`; theming installed
apps from tokens as P1's `system/theming.md` maps them, with Qt apps
reaching Isle's file panels; the one-time import of v1's preferences, window places
and launch counts (B27). The package installs `/usr/lib/pam.d/isle`, and the
greeter is the lock's face with its PAM path (from P9). Scan-out of a second GPU's connectors (moved from
P2). v1's user units disabled before the host moves over: xremap.service is
wanted by graphical-session.target, which isle-session.target binds, and
would rewrite Cmd chords before Isle saw them. The supervisor stops clearing
the user manager's session variables on exit when another session owns them,
and v1's and Isle's session logs stop sharing a path (both found in P4's
host run). Ends with the host booting
into the new Isle.
Intent: platform.md, compositor.md.

## After M2: parity

**P15 · Files.** Finder-shaped Files with v1's browsing-engine lessons; the
Open and Save panels, handed FileChooser by the portal backend, and Quick
Look as a D-Bus service Spotlight calls, all in the Files process (B26).
Files owns `org.freedesktop.FileManager1`, which Spotlight's Show in Files and
Cmd+Return on a file already call (P10), and a search that Spotlight's "Show
all in Files" and Cmd+Y on a file reach. Intent: apps.md, platform.md.

**P16 · Mission Control.** The Spaces overview, hot corners (Lock Screen
and Put Display to Sleep among them, from P9, and the bottom-right corner's
Apps view, which `isle spotlight apps` opens since P10), trackpad gestures, Split
View; Stage Manager as a layout module, off by default (B25). Intent: shell.md, compositor.md.

**P17 · Widgets.** Notification Centre moved to P8 at its review. Widgets
where P1 places them, with the grid editor and the gallery (B24); the provider contract (B14), sandboxed with granted
permissions (B28); the isle-widgets registry as providers. Intent:
shell.md, platform.md.

**P18 · Controllers.** Pad detection and naming, batteries in Control
Centre, Settings › Controllers, idle held off by pad play (a hold on P9's
idle policy). Intent:
gaming.md, apps.md.

**P19 · Monitor, Keychain, Cleaner.** Intent: apps.md.

**P20 · The rest of v1.** Settings' remaining pages, session restore
with the session dialog's "Reopen windows when logging back in" and the
crash notice for apps (from P9),
calendars with Notification Centre's agenda, the providers (vault, with
Spotlight's `*` scope, VPN, snapshots, phone), the dropdown terminal and picture-in-picture, the on-screen
keyboard (on the lock too), and an audit against the inventory that nothing of v1 is left.
Intent: all.

**P21 · Cut-over.** Remove v1's tree (`shell/`, `hypr/`, `plugins/`,
`native/`, `theme/`) and its tooling; this branch becomes `main`.

## After cut-over

**P22 · HDR.** Colour management from the community Smithay fork's work
(B23): the colour-management protocol, HDR metadata to the output, SDR
content composited beside HDR, the NVIDIA colour pipeline, HDR through
nested gamescope; tested on the host's driver first. Intent:
compositor.md, gaming.md.
