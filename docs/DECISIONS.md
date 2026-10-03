# Decisions

The record of why Isle is built the way it is. Entries are numbered B1
onwards, the numbers they were locked under in the rust-desktop program's
brainstorm (`docs/work/rust-desktop/brainstorm.md` while that program is
open). Isle v1's decisions (Hyprland and Quickshell, D1 to D91) stay on
`main` in `docs/DECISIONS.md` and are cited as "v1 D55"; their lessons are
carried as requirements in `docs/design/`. Evidence for these entries is in
`docs/research/rust-desktop/`.

**B1 · Own compositor on Smithay, not a fork of an existing compositor.**
This is a project for fun and for a desktop built to its own design. A niri
fork would inherit its layout model, config, IPC and render pipeline.
Rejected: forking niri (locked into its choices), staying on Hyprland
(its crashes and churn are outside our control; see the inventory's
crash notes), keeping Quickshell as the shell (477 MB RSS measured on the
host, 2026-10-02; the user does not want it).

**B2 · v1 parity, re-planned (revised).** Every capability v1 has is in
scope; none is dropped. Each is re-designed against the north star (B17)
rather than ported, and the roadmap decides only the order. The inventory
(`isle-v1-inventory.md`) is the checklist and keeps v1's lessons. This
replaces the first answer, "clean slate", given before the north star was
stated.

**B3 · Fresh visual design.** v1's look (DESIGN.md, docs/canvas) is not
carried over; both are on `main` for reference.

**B4 · Docs as a feature-plan program.** Evidence in `docs/research/`,
intent docs in `docs/design/`, brainstorm and roadmap in
`docs/work/rust-desktop/`; ARCHITECTURE, DESIGN and DECISIONS are rebuilt
as the spine once decisions land. Rejected: the three-doc spine alone.

**B5 · Smithay's built-in X11 window manager for Xwayland.** Steam and most
games are X11; v1's hardest bugs were X11 behaviour (Motif hints,
`_NET_WM_MOVERESIZE`, untitled Steam popups) and owning the WM puts those in
our hands. It is what cosmic-comp, Otto and Pinnacle use. Rejected:
xwayland-satellite (open Steam popup and menu bugs, one on CachyOS + niri
in September 2026; X11 quirks out of our hands).

**B6 · Our own Smithay fork, pinned, carrying the async page flip PR.**
Smithay has not released since 0.7.0 (2025-06-24); every serious
compositor builds from git. Tearing (smithay#1948 async flip,
#1325 tearing-control) is not upstream; the gaming packet lands it as a
patch on the fork and tests it on the RTX 5070 Ti, with VRR (48–144 Hz on
the host monitor) covering the gap until then. Smithay's `AI.md`
discourages LLM-written contributions, so patches stay on the fork.
Rejected: upstream git with VRR only.

**B7 · Freya, through the `alexparlett/freya` fork, draws the shell and
the apps.** The shell's always-on surfaces are Freya trees drawn by Skia on
the compositor's own EGL context into textures the compositor composites;
the separate apps are Freya apps. Evidence: `shell-ui-spike.md`. On the GPU
Freya's cost barely grows with area (about 0.25 ms on the island, 1.2 ms
p95 animating a 3440×1440 launcher); it has colour emoji, font features,
transforms and group opacity. The fork (also under strata and cairn)
answers Freya's single-maintainer risk: fixes land there, and the spike's
platform glue (events, accessibility tree, futures, animation ticking, GL
state handover) becomes a crate in it rather than copied code. The fork
must first take upstream to v0.5.0-rc.8 (the spike's API; the fork is at
rc.4). Conditions: open and close transitions are animated by the
compositor, not the toolkit; Skia's resource cache is capped; a way back
to live iteration on surfaces (running one in `freya-winit` with its hot
reload) is tried early. Rejected: Slint's software renderer (best-in-class
live reload and stable, but one CPU core redraws a full-output animation at
10–20 ms a frame, and no transforms, group fades, colour emoji or font
features); an own scene graph on Skia (Otto's way: the toolkit becomes the
project); CPU-drawn textures with iced or Pango (the same area problem as
Slint).

**B8 · Stacking with snapping, layout as a per-workspace module.** Windows
float by default and open at their remembered place; drag or keys snap
them to zones designed for 3440×1440 (halves, thirds, a centred 60%, and
the space beside an existing window), with a preview while dragging.
Minimise and maximise are real states, including for X11 clients that ask
for them; picture-in-picture and the dropdown terminal are part of the
model, not rules. This is what v1 converged on through its `isle-windows`
plugin and float-everything rules (inventory §3.4: D40, D57, D91), built in
rather than patched onto a tiling compositor. Layout sits behind a trait
chosen per workspace, so other layouts (tiled, scrollable) can be added
later without touching the rest. Rejected: dynamic
tiling (v1 turned it off in practice), scrollable columns as the default
(good for terminals on an ultrawide, fights games, Steam and dialogs).

**B9 · One event loop, blocking work on a service worker.** The main thread
runs one calloop loop for Wayland, input, the Xwayland WM, every shell
surface's Freya runner and rendering, without locks (niri's model). D-Bus,
PipeWire, the app index, icon loading and file search run on a worker
thread with its own async runtime and reach the loop through calloop
channels; nothing that can block runs on the main thread. Separate apps
are separate processes. Containment: each shell surface's event handling
and drawing sits behind a panic boundary and is rebuilt if it panics; a
panicking service is restarted on the worker; a compositor-core panic ends
the session, so a small session supervisor restarts it and keeps the
crash; a watchdog logs any main-loop stall over a frame's budget.
Rejected: a render thread per output with shell state behind locks
(cosmic-comp's model), which pays off on several mixed-refresh outputs, not
on one main monitor.

**B10 · The compositor owns settings; KDL on disk; tokens reload live.**
The compositor is the one writer: a D-Bus settings interface serves the
Settings app, a CLI and the shell; it validates, applies live and writes
the file. The file is also watched, so hand edits apply live; a parse
error is shown in the shell and the last good config stays in force.
Three stores: `settings.kdl` (the user's choices, hand-editable, watched),
state (window places, launch counts, session restore: machine-written,
atomic, never watched) and `tokens.kdl` (design tokens: colour, radius,
motion, blur; watched, applied as state so every Freya surface re-renders,
which gives back live tuning of the look that B7 gives up for layout).
Installed apps are themed from the same tokens through templates, as v1
did; the template set is decided in its packet. KDL because nested binds,
rules and per-device blocks read best in it (niri's precedent). Open for
the config packet: which crate maps KDL to types and writes it back
preserving comments (`kdl` 6.7 is KDL v2 and format-preserving; `knus`
3.4, the maintained fork of niri's `knuffel`, derives types). Evidence:
`ui-and-services.md` §4. Rejected: file-only ownership (the app and the
watcher race and echo), TOML (sound, but KDL suits the nesting), RON
(unpleasant to hand-edit).

**B11 · Services are shared crates on zbus; audio through pipewire-pulse.**
Service logic (NetworkManager, BlueZ, UPower, logind, MPRIS, UDisks2,
power profiles, GameMode, audio) is a set of library crates used by the
compositor's service worker (B9) and by the separate apps, which talk to
the system daemons directly. What only the desktop knows (settings,
window places, mode) comes from the compositor's D-Bus interface. One
D-Bus stack, zbus 5: proxies generated with `zbus_xmlgen` or taken from
COSMIC's `dbus-settings-bindings` where its licence allows; `bluer` and
`mpris` are out (libdbus). Volume, mute and default devices go through
`libpulse-binding` over pipewire-pulse (COSMIC's and wayle's route);
`pipewire-rs` only for what Pulse cannot do, such as screencast streams.
Evidence: `ui-and-services.md` §3. Rejected: a separate desktop daemon
owning all services (one more process and protocol between apps and the
system).

**B12 · The desktop takes the system's prompt and tray roles, and locks
itself.** On the service worker, with every dialog drawn by the shell: the
notification server (`org.freedesktop.Notifications`, written by hand, no
crate serves it); the tray (`org.kde.StatusNotifierWatcher` and host); the
polkit agent through `/run/polkit/agent-helper.socket` (polkit 127's
helper is not setuid on the host); the keyring unlock prompter
(`org.gnome.keyring.SystemPrompter`, which gnome-keyring 50 and oo7-daemon
both call); the NetworkManager secret agent for Wi-Fi passwords. The lock
screen is the compositor's own, with PAM conversations off the main thread
(fprintd blocks); v1's locker-death lessons (D15, D43) carry over as
requirements.

**B13 · Our own portal backend; the shell draws the picker.**
`xdg-desktop-portal-isle` implements ScreenCast, Screenshot and
FileChooser as zbus interfaces (xdg-desktop-portal-cosmic's and Otto's
pattern); the compositor produces the PipeWire streams and the shell draws
the source picker. The compositor also implements ext-image-copy-capture
and ext-foreign-toplevel-list (both in Smithay) so standard capture tools
work. Window previews in the switcher and overview are drawn by the
compositor and need no capture protocol. v1's rule carries over: capture a
source only while it is visible (D55; capturing a disappearing output
crashed Hyprland). It also serves the Settings portal, listed ahead of
xdg-desktop-portal-gtk (`Settings=isle;gtk`): GTK's backend gives apps
the colour scheme and contrast but not the accent colour or reduced
motion, which Isle's adds (P1, `system/theming.md`). Rejected: mimicking
GNOME's D-Bus screencast API for
xdg-desktop-portal-gnome (its GTK picker, against B3), xdg-desktop-portal-
wlr (an external picker).

**B14 · Built-in widgets, extended by data providers.** Dashboard widgets
are Freya components compiled into the shell. Extension is v1's provider
contract: a provider is a program in any language that emits JSON into
built-in templates (card, list, graph and the like), sandboxed by being a
separate process. The isle-widgets registry carries providers instead of
QML widgets. Rejected: WASM plugins (needs a UI description protocol; a
large build for an uncertain need), built-in only (loses the registry).

**B15 · The name stays Isle.**

**B16 · Built in this repository on `feature/rust-desktop`.** The Rust
workspace grows here beside v1, which stays the daily desktop on `main`
until the new one reaches parity; v1's tree is removed when it does.
Rejected: a new repository.

**B17 · North star: a macOS-style desktop on Arch (CachyOS).** Where a
choice is open, the answer is the one a Mac user expects, built for this
machine, which is used mostly for gaming and coding. B18–B21 are its first consequences.

**B18 · The island at rest, the global menu on demand (revised).** At
rest each screen shows the wallpaper and the island: v1's pill at top
centre with the clock, and a status glyph only while its state is not the
normal one (Wi-Fi only on Wi-Fi, Bluetooth only with a device connected,
sound only when muted, a VPN, Focus, a restart owed, shortcuts inhibited, a
configuration error), each settable to always or never. Hovering the island
opens Control Centre from it, as v1's panel opened, with live activities
first and other programs' tray items under its modules; a click on the
clock opens Notification Centre. Windows keep a strip along the top clear
for the island. The front app's menu bar (the system menu, the app menu,
the app's menus, Window, and Help with menu search) is macOS's
automatically hidden bar: it slides down over windows when the pointer
pushes the top edge outside the island's zone, or on Ctrl+F2, carries menus
on the left and nothing on the right, and leaves with the pointer. Menu
items are also searchable from Spotlight. Menus are collected three ways,
all of which Isle serves from session start (`macos-model.md` §1):
`com.canonical.AppMenu.Registrar` with DBusMenu (Qt, Chromium, Electron,
LibreOffice and Firefox export only while the name is owned, and Qt checks
once per process); KDE's `org_kde_kwin_appmenu` Wayland protocol (Qt 6.9+
and Chromium 137+ use it unprompted; only KWin implements it today); and
GTK's `gtk_shell1` with `org.gtk.Menus`/`org.gtk.Actions` (GtkApplication
menubars in GTK 3 and 4; libadwaita apps have none). No Rust crate reads
GTK menus, so Isle writes that client; `system-tray` covers DBusMenu.
`gtk_shell1`'s menu-bar capability makes GTK hide the in-window menubar, so
it is advertised only once the reader works. Electron on Wayland
registers a wrong window id when the KDE protocol is present (fix
unmerged), so menus are also matched to windows by the D-Bus sender's PID.
Firefox exports only with two prefs set, and Steam, Wine and games export
nothing, so the fallback app menu (About, Hide, Quit, the window list) is
the common case and gets designed as such. Structure:
`docs/work/rust-desktop/p1/structure/revision-1.md`, `island.md`,
`menu-bar.md`. Rejected: a menu bar always shown, with status items,
Control Centre and the clock on the right and the island in the notch, as
drawn for P1's gate 2 (the user preferred v1's minimal desktop, and since
most Linux apps export no menus, a bar always shown would mostly show
fallback menus); the island with no app menus at all (v1). This replaces
the first answer, that permanent menu bar.

**B19 · No Dock (revised).** Nothing at rest takes its place; each of its
duties moves to a surface that opens on demand. Launching is Spotlight and
its Apps grid, which keeps a row of favourites in the user's order;
switching is Cmd+Tab, Cmd+` and Mission Control. The switcher's cards
carry what the Dock's icons did: badges and progress (LauncherEntry),
attention (also an island event), hidden and minimised marks, and apps
running in the background after quit, which Q there stops. Minimised
windows are in App Exposé and Mission Control, and minimise shrinks the
window toward the island. The bottom edge belongs to windows. The default
hot corners are bottom left Mission Control and bottom right the Apps
grid, so the pointer alone can switch and launch. Duty by duty:
`docs/work/rust-desktop/p1/structure/dock.md`. Rejected: a Dock of pinned
and running apps with indicators, badges, progress, minimised windows,
magnification and auto-hide (Otto's `otto-dock-v1` shows the client
side), as drawn for P1's gate 2, where the user preferred v1's minimal
desktop. This replaces the first answer, "a Dock".

**B20 · App-centric windows.** Cmd+Tab switches apps, Cmd+` cycles the
front app's windows; the menu bar and switcher speak in apps; Cmd+Q
quits. An app that exits when its last window closes cannot be made to
stay (GTK and Qt apps decide that themselves), so Cmd+W on an app's last
window hides it where the app allows, and v1's "keep running" list
(inventory §3.6) becomes the per-app policy for which apps get that.
Evidence: `macos-model.md` §3. Rejected: window-centric (v1).

**B21 · Cmd everywhere.** Super acts as Cmd across the system: in
Smithay's keyboard filter the compositor translates Cmd shortcuts (copy,
paste, undo, close, quit, new tab, find and the rest) into the focused
app's own shortcuts, with per-app exceptions for terminals, browsers,
IDEs and file managers; Ctrl stays Ctrl and Option is Alt. Because the
compositor knows the physical keys it needs none of Toshy's Ctrl-to-Super
swapping, and it can withhold a translated Alt so menus do not open on
it. The exception tables are written for Isle: Toshy (GPL-3.0) and Kinto
(GPL-2.0) are the record of what apps need, not text to copy; keyd and
xremap are MIT. Translation is off in game windows and while
keyboard-shortcuts-inhibit is active (VMs, remote desktops); Isle's own
injected keys are a separate source and never translated; modifiers still
held are released before focus moves mid-chord. Tiling chords avoid Fn,
which most keyboards do not report. The per-device keyboard profiles from
v1 (the Mac preset, D7, D61) move into the compositor, which sees every key
with its device, so xremap and its silent wrong-device failure (D58) go.
Open for its packet: synthetic modifier events agreeing with the mask
(Xwayland), and text-input-v3 as a "text field focused" signal for
Cmd+arrow. Evidence: `macos-model.md` §2. Rejected: Super for desktop
actions only (v1).

**B22 · No console mode of Isle's own.** Isle's Big Picture mode is
dropped: its home, the quick menu and Guide-button handling, the pad
bridge to keys, and Isle's entries inside Steam (`shortcuts.vdf`). Steam's
Big Picture is the console experience; Isle handles it as a full-screen
game (gaming.md) and keeps controller naming, battery and the Controllers
page. An exception to B2's parity, by the user's choice.

**B23 · HDR is in scope, after Isle replaces v1.** The host monitor (AOC
U34G2G4R3) takes HDR10 at 10 bpc, with a reported maximum content
luminance of about 302 cd/m² (`smithay-and-gaming.md`). Smithay has no
colour management (PR #1143, draft since 2023); a community fork
(`dividebysandwich/smithay`, `ds-hdr-master`) carries it with a matching
niri branch and is the starting point. The HDR packet serves the Wayland
colour-management protocol, sends HDR metadata to the output, composites
SDR windows and the shell correctly beside HDR content, uses the NVIDIA
DRM colour pipeline (driver 610+) where it works, and lets nested
gamescope pass HDR through. NVIDIA HDR crashed KDE on a 5070 Ti with
driver 595, so the packet tests on the host's driver first.

**B24 · How attention is presented is the design process's call.** What
the island shows, where banners appear, the order of anything that
competes for the same place, and where v1's dashboard widgets live are
decided by the design-language packet (P1), not here. The intent docs say
only what must be possible: live activity is shown, privacy indicators
always show, notifications can be read later, widgets can be placed and
edited (v1's grid editor and gallery, v1 D24, D28–D30, D35, are kept as
capabilities). The brainstorm's proposals go to P1 as starting points:
the island for live activities and OSDs, banners top right, widgets in
Notification Centre and on the desktop with a "Show widgets" chord.

**B25 · Window chrome.** Isle's own apps draw a unified toolbar with the
traffic lights. The compositor draws Mac-style server-side title bars, in
Freya, for apps that accept server-side decorations. Stage Manager comes
later as a layout module (B8), off by default.

**B26 · Where things run.** The portal backend (B13) is a bus name owned by
the compositor's service worker, as v1 owned its FileChooser (v1 D67); it
hands FileChooser to Files. The Open and Save panels and Quick Look run in
the Files process, Quick Look as a D-Bus service that Spotlight also
calls, so nothing file-browsing loads into the compositor. The greeter is
the compositor in a greeter mode under greetd, reusing the lock screen.
Apps render on the GPU by default; the Settings packet measures their
memory before that is final.

**B27 · Settings in practice.** App preferences are per-app sections of
`settings.kdl`, written through the compositor (B10's one writer). A write
that arrives while `settings.kdl` does not parse is refused with a pointer
to the parse error, so a hand edit is never overwritten. v1's prefs,
window places and launch counts are imported once; notification and
clipboard history are not.

**B28 · Roles and protocols, completed.** A Bluetooth pairing agent
(`org.bluez.Agent1`) joins B12's roles. `ext-session-lock` is not served:
the lock is Isle's. Layer-shell is served for other programs' overlays and
tools. Providers (B14) run sandboxed (bubblewrap or Landlock) with
declared permissions the user grants, carrying v1's trust model (v1
D28–D31).

**B29 · Delivery.** Isle ships as a PKGBUILD in this repository that builds
from git, installed with paru and updated through pacman.

**B30 · Keys and games, completed.** Cmd+Q quits, keep-running apps
included. On a PC-layout keyboard, a per-device setting swaps Alt and Super
so Cmd sits beside the space bar; it is on by default and suspended while a
game window has focus, so games get the keys as labelled. A Mac-layout
keyboard needs no swap: Cmd is Super and Option is Alt everywhere, games
included, as on macOS. While a game is in
front a fixed escape set still works (Cmd+Tab, Mission Control, lock,
screenshot) and every other chord goes to the game. Focus modes work as on
macOS, with a Gaming Focus that turns on for any detected game in front,
windowed or not. VRR is on while a full-screen app is in front, with a
setting to change it.

