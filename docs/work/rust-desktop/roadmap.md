# Roadmap

The build order for the rust-desktop program. Each entry is a build
packet's brief: what it delivers, what it depends on, and the decisions and
intent documents it answers to. A packet is planned in detail, with its own
PRD and phases, by its own run of feature-plan when it starts, against the
code as it is then. Intent: `docs/design/` (compositor, shell, gaming,
apps, platform). Decisions: `docs/DECISIONS.md` (B1–B30).

Everything happens on `feature/rust-desktop` (B16); v1 stays the daily
desktop on `main` until milestone M2.

## Before the first packet

- **The Freya fork at rc.8.** The user merges upstream v0.5.0-rc.8 into
  `alexparlett/freya` (the fork is at rc.4; the spike's code needs nine
  API changes from rc.8). Blocks P3 and everything drawn.

## Milestone M1: a desktop that runs in the VM

**P1 · Design.** A full design process for the new Isle (B3, B17), run as
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

**P2 · Compositor core.** The Cargo workspace; the pinned Smithay fork (B6);
udev/DRM and winit backends; libinput; outputs with hotplug, scale and the
KVM lessons; xdg-shell and layer-shell; the calloop loop, service worker,
watchdog and session supervisor (B9); the settings store and its D-Bus
interface (B10) with `settings.kdl`, per-app sections and writes refused
while it does not parse (B27), and the state store; the `isle` CLI and the
test interface; the NVIDIA VRAM profile. Ends booting in the VM with a
terminal open. Intent: compositor.md, platform.md.

**P3 · Shell runtime.** The Freya embedding from the spike, moved into the
fork as a crate: surface hosting on the compositor's GL context, input,
the accessibility tree, panic boundaries per surface, compositor-driven
transitions, the Skia cache cap, live `tokens.kdl`; and a preview mode that
runs a surface in `freya-winit` with hot reload, to win back design
iteration (B7's condition). Depends on the fork at rc.8. Intent: shell.md.

**P4 · Windows.** Stacking with snapping and the ultrawide zones (B8);
app-centric switching mechanics and hide-on-last-close policy, with Cmd+Q
quitting keep-running apps (B20, B30); minimise, maximise, full-screen
Spaces, window places; server-side title bars drawn in Freya (B25); the
Xwayland window manager with v1's X11 lessons (B5). Intent: compositor.md.

**P5 · Keyboard.** Per-device profiles and the Mac preset in the
compositor, with the Alt and Super swap on PC keyboards on by default
(B30); Cmd translation with per-app exceptions, off in games and under
shortcuts-inhibit (B21); the default shortcut set. Retires xremap. Intent:
compositor.md.

**P6 · Island and menus.** The island on every screen (B18): the clock,
the status glyphs, live activity as P1 designs it (B24), and Control
Centre's panel opening from it, whose modules and tray fill in with P8.
The menu bar on demand from the top edge and Ctrl+F2, with the three menu
sources and the registrar owned from session start, the fallback app menu
and menu search. The wallpaper. Intent: shell.md, platform.md,
compositor.md.

**P7 · Switcher and launching.** Cmd+Tab and Cmd+` (B20), the switcher's
cards carrying what a Dock would (B19): badges, progress, attention, and
hidden, minimised and background marks. Launching from Spotlight's Apps
grid with its favourites row is built with Spotlight in P10. Intent:
shell.md.

**P8 · Services, notifications and Control Centre.** The zbus service
crates (B11): network, Bluetooth, power, audio through pipewire-pulse,
media, disks; the notification server and tray (B12); notification
banners, OSDs and Control Centre as P1 designs them (B24); Focus modes
(B30). Intent:
platform.md, shell.md.

**P9 · Lock and prompts.** The compositor's own lock with PAM off-thread
and v1's locker lessons, with no ext-session-lock served (B28); the
polkit agent, keyring prompter and NetworkManager secret agent (B12) and
the Bluetooth pairing agent (B28) with their dialogs; idle, dim and DPMS.
Intent: shell.md, platform.md, compositor.md.

**P10 · Spotlight.** Launcher with apps, the Apps grid with its favourites
(B19), windows, settings, the front app's menu items (B18), actions and
v1's prefixes and sources. Intent: shell.md.

**P11 · Capture and portals.** `xdg-desktop-portal-isle` (B13), a bus name
on the compositor's service worker (B26): ScreenCast and Screenshot with
PipeWire streams from the compositor and the shell's picker, the screenshot
tool, ext-image-copy-capture; FileChooser follows in P15. Intent:
platform.md, shell.md.

**P12 · Gaming.** The async page flip patch on the Smithay fork and tearing
control tested on the RTX 5070 Ti (B6); VRR on while a full-screen app is
in front, with its setting (B30), and direct scanout for games; game
detection, Game Mode as the automatic Gaming Focus and the escape set of
chords over a game (B30); Steam, its Big Picture as a full-screen game
(B22), and nested gamescope. Intent: gaming.md, compositor.md.

**P13 · Settings, core.** The Settings app on the app conventions: displays,
sound, network, Bluetooth, keyboard and shortcuts, appearance, desktop and
hot corners, Control Centre, Focus and Game Mode. Measures an app's memory on the GPU before
B26's default is final. Intent: apps.md.

## Milestone M2: Isle replaces v1 on the host

**P14 · Session and install.** The PKGBUILD built from git, installed with
paru and updated through pacman (B29); the compositor's greeter mode under
greetd (B26) and session start, Login Items and autostart, crash logs, the
update path, the dev loop that replaces `isle-dev-sync`; theming installed
apps from tokens; the one-time import of v1's preferences, window places
and launch counts (B27). Ends with the host booting into the new Isle.
Intent: platform.md, compositor.md.

## After M2: parity

**P15 · Files.** Finder-shaped Files with v1's browsing-engine lessons; the
Open and Save panels, handed FileChooser by the portal backend, and Quick
Look as a D-Bus service Spotlight calls, all in the Files process (B26).
Intent: apps.md, platform.md.

**P16 · Mission Control.** The Spaces overview, hot corners, trackpad
gestures, Split View; Stage Manager as a layout module, off by default
(B25). Intent: shell.md, compositor.md.

**P17 · Notification Centre and widgets.** Notification Centre; widgets
where P1 places them, with the grid editor and the gallery (B24); the provider contract (B14), sandboxed with granted
permissions (B28); the isle-widgets registry as providers. Intent:
shell.md, platform.md.

**P18 · Controllers.** Pad detection and naming, batteries in Control
Centre, Settings › Controllers, idle held off by pad play. Intent:
gaming.md, apps.md.

**P19 · Monitor, Keychain, Cleaner.** Intent: apps.md.

**P20 · The rest of v1.** Settings' remaining pages, session restore,
clipboard history, emoji, calendars, the providers (vault, VPN, snapshots,
phone), the dropdown terminal and picture-in-picture, the on-screen
keyboard, and an audit against the inventory that nothing of v1 is left.
Intent: all.

**P21 · Cut-over.** Remove v1's tree (`shell/`, `hypr/`, `plugins/`,
`native/`, `theme/`) and its tooling; this branch becomes `main`.

## After cut-over

**P22 · HDR.** Colour management from the community Smithay fork's work
(B23): the colour-management protocol, HDR metadata to the output, SDR
content composited beside HDR, the NVIDIA colour pipeline, HDR through
nested gamescope; tested on the host's driver first. Intent:
compositor.md, gaming.md.
