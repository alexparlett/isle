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


**B31 · Frost, as data.** Isle's look is the Frost direction (P1 gate 3):
macOS 27's frosted glass in neutral cool greys, Inter, soft depth, medium
corners, critically damped motion, dark first with light fully designed.
It lives entirely in `tokens.kdl` with its schema
(`docs/work/rust-desktop/p1/system/`), checked by a contrast script that
must pass in both appearances and under Increase Contrast before a change
lands. Symbols are Phosphor, app icons Papirus, the cursor Bibata.
Rejected: Slate (solid, hairline-drawn, Geist) and Dusk (deeper glass,
warm, Figtree), drawn beside it at gate 3.

**B32 · Attention, presented.** This answers B24. The island carries live
activity in one queue (a waiting prompt, recording and sharing, a call, a
timer, media), privacy marks always and outside the queue, and every OSD;
it never shows a notification. Banners sit at the top right under the
island's strip, three at most. Notification Centre unfolds from the
island's clock with the date, month and agenda and no widgets. Widgets
live on one full-screen dashboard per screen (Ctrl+Option+Cmd+W), never on
the desktop at rest. Rejected: OSDs as macOS's capsule under a Sound item
(there is no status area to hang it from), banners at top centre (they
would cover the window in use and fight the island), widgets on the
desktop (against calm at rest).

**B33 · Glass, and where it may go.** Glass is only on the shell's
floating surfaces: the island's panels, the revealed menu bar and the
shell's menus, banners, Spotlight, the switcher and Mission Control's
bars. Windows are opaque; menus inside an app's window are its raised
surface; menus over a glass panel are solid, so glass never sits on glass.
Colour on the translucent island sits in solid wells. Every glass surface
has a solid form for Reduce Transparency and Game Mode, and the compositor
keeps a contrast floor under text-bearing glass at every Glass setting.

**B34 · Readings of the principles.** Recorded so later packets keep them:
Control Centre's modules fill with the accent when on, as "active" (P6);
title bars use their own roles rather than an app's body colour, which a
third-party app does not report; control edges and unfocused selections
sit at the Mac's contrast by default and reach 3:1 under Increase
Contrast, a deliberate departure from WCAG 1.4.11 (P1, P3); the text size
goes to 1.5× and the compositor's Zoom serves beyond.

**B35 · The compositor core's shape.** Built in P2 and kept by later
packets. Three binaries: `isle-session` (the supervisor), `isle-compositor`
(fifteen characters, so `pkill -x` and the NVIDIA application profile
match it whole) and `isle` (the CLI). One bus name,
`io.github.alexparlett.Isle`, with an object per interface family and
versioned interfaces; Isle owns no domain, and a reverse-DNS name on the
GitHub account is Flatpak's convention. The worker is a tokio runtime on its
own thread; it reaches the loop only by jobs, which fail after a second,
and services on it restart with a backoff. Frames are drawn only for
change, in niri's per-output redraw states, with frame callbacks at the
estimated vblank. The watchdog ends a dispatch stuck for ten seconds by
aborting the main thread, so the core dump holds its stack and the
supervisor starts a fresh session. State is one versioned JSON file per
domain. The supervisor exports the session through `isle-session.target`,
since `graphical-session.target` refuses a manual start. Clients draw their
own frames until P4's title bars. Rejected: one multi-call `isle` binary
(the CLI would link Smithay), `org.isle.Desktop` (a domain Isle does not
own), logging a hang without ending it (a hung desktop is no use, and the
abort keeps the evidence).

**B36 · Settings are written into the file's own text.** A Set changes
`settings.kdl` by splicing the new value's text at the old value's span,
adding a missing node inside its block at the block's indent; `kdl` 6.7
only parses. Its printer drops whitespace on a round trip (kdl-rs #178,
#179, #193; found in P2), and `knus` reads only KDL v1, which `tokens.kdl`
is not. Every byte not changed stays as the user typed it, and the result
is parsed again before it is written. Rejected: printing the parsed
document, a derive-based KDL crate.

**B37 · A shell surface is a definition hosted by the compositor.** Built
in P3 and kept by every surface after it. A surface is a `SurfaceDef` in
`isle-shell`: a name, a layer (below windows; over windows and
layer-shell's top layer; over everything a client can show), a scope (one
instance per output, or one on the main screen), whether it takes the
keyboard, its box for an output, and a tree that holds the state which
must outlive it. Each instance is a Freya tree driven through the fork's
`freya-embed`, drawn by one shared Skia context into a texture on the
compositor's context; its waker pings the loop, so nothing runs on a
timer. A closed surface mounts no content and holds no texture. Open and
close are compositor springs on the texture, and a closing surface keeps
its last frame until its exit ends. A surface that takes keys takes the
keyboard and gives it back before its actions run. Every call into a
tree sits behind a panic boundary that rebuilds it with the same id; the
sixth panic in a minute turns it off until `isle shell restart`. Rejected:
shell surfaces as layer-shell clients (B7), a Runner kept across a panic
(Freya's cells may be left borrowed), surfaces ticked on a timer (the
spike's 50 ms poll broke idle).

**B38 · Tokens are typed from the shipped file.** `isle-tokens`' build
script types every token of `data/tokens/tokens.kdl` as a field, by the
units rules in `schema.md`, so no token is declared twice and a shipped
token cannot be missing. The user's `tokens.kdl` lies over the shipped file
by path; a token the shipped file lacks is ignored with a warning, and a
file that does not parse, has a value of the wrong kind or a newer version
is refused with its place while the last good tokens stay. The loop
resolves a theme for the `appearance` settings and hands it to every tree
as context; Isle's apps read the same crate. Rejected: a hand-written token
struct (written twice, drifting from the file), tokens looked up by string
(errors only when drawn).

**B39 · Glass is the compositor's.** A surface draws no glass: `isle-ui`'s
`Glass` reports its box and the compositor draws behind it what lies
behind the surface's layer again, through a damage tracker of its own so
the blur runs only when that changed; dual Kawase passes as wide as
`material-blur`, saturation, the tint and the rounded clip, in one shader.
The glass floor reads a reduction of the blur back a frame late and moves
the tint toward `glass-tinted` until `text-2` holds 4.5:1. The preview,
with no compositor, draws the plain tint. Rejected: reading the frame
being drawn back behind the glass (it ties glass to the damage of the
frame), blurring in Skia per surface (each surface would draw the desktop
behind it).

**B40 · One window registry; Smithay's `Space` is its view.** Built in P4
and kept by every packet that reads windows. `isle-compositor`'s `wm`
keeps a record for every toplevel, Wayland or X11, from creation to
destruction: its id, app (resolved from desktop entries by `isle-apps`),
PID, parent, desktop, mode and boxes. A mode is floating, tiled (a zone
and the box it came from), filled (maximised) or full screen (on a desktop
of its own, with its origin); minimised and hidden are two flags beside it,
so a window minimised inside a hidden app comes back minimised. The
`Space` holds only what is shown now, in the registry's stacking order;
everything that places a window works on its frame, the bar included.
Each output has a row of desktops by its identity. A main window's frame
is remembered per app and output identity and applied in its first
configure. Rejected: Smithay's `Space` as the model (it forgets what is
not mapped), one parked state for minimise and hide (a hidden app's
minimised windows would come back), places keyed by output set (an
ultrawide alone and with a second screen would keep two places for one
habit).

**B41 · Title bars and menus by windows are shell surfaces.** Built in
P4. B37 gains two scopes: attached, an instance the compositor makes,
places and removes for a window and draws in that window's place in the
stack (the title bar, a Freya tree per decorated window, 0.27 MB and 0.47
MiB each); and placed, one instance opened at a box the compositor gives,
in its layer (the zoom menu, the pointer panel). A layout both draw from
and hit-test with is a plain function, so the compositor reads the zone
grid during a drag, when the pointer is grabbed and the tree sees nothing.
Rejected: one tree drawing every bar into an atlas (the cost of a tree
per bar was measured and is small), bars drawn by a shader (text and
accessibility are Freya's).

**B42 · Isle's X11 policy.** Built in P4 on Smithay's window manager (B5).
Xwayland starts with the compositor, its display reported with
`WAYLAND_DISPLAY` and kept across restarts, each after a doubling pause.
Override-redirect and untitled X11 windows are left alone: drawn where
they ask, over windows, never placed, focused, decorated, shadowed or
animated (D40). Every other X11 window is a record like a Wayland one;
Motif hints decide its bar, its state requests act as their Wayland twins
(a minimise request is only ever a minimise), `_NET_WM_MOVERESIZE` starts
Isle's own grabs, `_NET_ACTIVE_WINDOW` is granted to the app in front and
otherwise marks attention. X11 clients draw at scale 1. Rejected: the
untitled rule only for override-redirect windows (Steam's menus are
untitled normal windows), granting every activation request (games steal
focus).

**B43 · Who draws a window's frame follows each protocol.** Built in P4.
On Wayland a client that negotiates (xdg-decoration, KDE server-decoration)
gets what it asks for, server side when it asks for nothing, while Isle
draws bars; with bars off every such client is told to draw its own, and a
client that never negotiates draws its own, as the protocol says. An X11
window wears Isle's bar when its EWMH type is normal, dialog or utility,
its Motif hints ask for a title (ALL means all but the bits listed), it sets
no `_GTK_FRAME_EXTENTS`, it has no bounding shape and it is not full screen.
A shaped window gets no bar, shadow or corner clip and is drawn blended,
since Xwayland leaves its buffer transparent outside the shape while the
surface claims to be opaque. Smithay's window manager had no SHAPE support;
Isle's Smithay fork adds it and keeps it, and Isle's patches are not offered
upstream, whose contribution policy does not accept this work. Rejected: a
second X connection beside Smithay's to ask about shapes (it races the
window manager on window lifetimes), telling clients server side with bars
off (they and Isle would both draw nothing).

**B44 · The seat sees only what Isle sends.** Built in P5 and kept by every
packet that touches keys. Physical keys go to `isle-keys`' engine, which
decides what the focused client is sent; only those keys are fed to
Smithay's keyboard, so its modifier state, and every client's, is built
from them. Cmd and Option are withheld until the next key or click decides
them; a lone tap sends nothing. A desktop chord matches the physical keys
before anything is sent. A translated chord is sent with exactly its
modifiers and the held ones are put back after. Keyboard focus moves only
through one function, which releases what the old focus holds first.
Checked with `xev`: X11 state agrees with the keys sent, so no synthetic
modifier events are needed. Rejected: Smithay's filter forwarding the
physical keys with a rewritten modifier mask (Xwayland derives state from
key events), a uinput remapper below the compositor (xremap's wrong-device
and stuck-modifier failures, v1 D58).

**B45 · Translation tables are a shipped data file.** Built in P5.
`data/keys/translation.kdl` holds shared rule sets, groups of apps and
single apps, each a Mac chord and what is sent instead, and the app's own
chord for the desktop's close and quit, which Cmd+W and Cmd+Q send to a
main window in its place. The user's file lies over it by scope and chord,
read live, refused with its place while the last good tables stay, as
`tokens.kdl` is (B38). `settings.kdl` keeps only whether to translate and
which apps not to. The tables are written for Isle; Toshy and Kinto (GPL)
are a record of what apps need, not a source. Rejected: tables in
`settings.kdl` (nested rule lists the settings derive is not shaped for),
copying Toshy's config.

**B46 · A game is a flag on the window's record.** Built in P5, refined by
P12. A window is a game when Steam labels it (`STEAM_GAME` on X11 windows,
read by the fork's window manager; a `steam_app_` or gamescope class;
`SteamGameId` in its process), Feral GameMode has registered its process,
its X11 class is a Windows `.exe`, or `windows.games` names it, unless
`windows.not-games` does. Over a game keys go raw, unswapped, and only the
escape set acts, through any inhibitor the game holds. GameMode is asked
only while it runs, since asking would start it. Neither KWin nor CachyOS
detects games for the desktop. Rejected: full screen as a sign (a video is
not a game; B30 follows games windowed or not), ananicy's process-name list
for now (distro data, names only; P12 may use it as a fallback).

**B47 · The island's model is the one place sources post.** Built in P6
and kept by every packet that shows something on the island (P8, P9, P11,
P17). `isle-shell`'s `island::model` is pure: glyph states with their
tiers, privacy marks, live activities ranked one per kind, and events with
their forms, durations, replacement and pre-emption by OSDs, waiting while
a panel is open and dropped past their time. It takes a clock and answers
what each output's island shows and when that next changes; the loop arms
one timer for that. A source never draws on the island or holds a timer
of its own. Rejected: each source owning a piece of the pill (two sources
would race for the space), a tick to re-evaluate the queue (acceptance
bar 2).

**B48 · The island's shape is the compositor's.** Built in P6. The
compositor springs the island's width and height, draws its fill, shadow
and glass and clips the tree to it; the tree draws content only and reads
the morph's progress, fading the outgoing form by 1 − 2p and the incoming
by 2p − 1, so the pill's row, in both, stays put. The texture is the larger
form's size while a spring runs. Measured at about eighty draws an open and
close, inside 2 ms at p95. Rejected: a crossfade of snapshots (the shared
row would vanish at the midpoint), springing the shape inside the tree
(every frame would lay out the panel at a new size).

**B49 · One top-layer panel at a time.** Built in P6. Control Centre is the
island grown into a panel on one output, and the bar's menus are in the
bar's surface. Revealing the bar closes Control Centre and pinning Control
Centre hides the bar; a peek leaves a revealed bar alone. Opening either on
another output closes it where it was. The island's events wait while its
panel is open. The bar's surface covers
its output while revealed, about 40 MiB of VRAM at 3440×1440, given back on
hiding. Rejected: a placed surface per menu (Help's search and moving
between titles need one layout), Control Centre as a surface of its own
(the morph from the pill is one shape).

**B50 · Menus are one tree from four sources, the registrar owned first.**
Built in P6. `isle-menus` reads DBusMenu (Qt, Chromium, Electron) and
GMenuModel (GTK) into one tree on the worker, with a 2 s limit on every
call. A window's menu is found by its surface's own address
(`org_kde_kwin_appmenu` v2, `gtk_shell1` v5), then for X11 the registrar's
entry by window id and the window's KDE and GTK properties, then a
registration from the window's process (Electron on Wayland), else the
fallback built from the desktop entry and the translation tables' labels;
the app id never selects a menu. A window is looked up as it takes focus
and again when a registration arrives after it maps; a dialog with no
menus shows its parent's, else its app's. A source is watched while the
app's menus are empty, as LibreOffice's are until its document is up, and
a GMenu watch keeps its subscription, since GTK signals only subscribers.
Items are found again by label path, since Electron renumbers its tree. Isle owns
`com.canonical.AppMenu.Registrar` without queueing before it announces
its socket, because Qt asks once per process whether it has an owner.
`gtk_shell1`'s title-bar gesture is served with the capability, so a
libadwaita header bar's double click and right click keep working.
Rejected: `system-tray`'s DBusMenu proxy (it replaces the whole tree on a
submenu's update and stops watching after a timeout), version 4 of
`gtk_shell1` (no window menu from header bars), `xdg_dbus_annotation_v1`
(unmerged, no exporter).

**B51 · The main screen follows focus.** Built in P6. Events, faces and
Isle's panels show on the focused window's output, else the pointer's;
`shell.main-screen` pins it to an output by identity. Every output's island
keeps its clock, glyphs and privacy marks. Rejected: the primary output
(Isle has none; the ultrawide and a laptop panel trade places).

**B52 · One launch path.** Built in P6 and used by Spotlight (P10) and
every later packet that starts something. `launch` takes an entry, an
entry action or a link (the default handler from `mimeapps.list`), expands
`Exec`'s field codes, and starts it in a transient systemd user scope named
by the xdg convention with an xdg-activation token, so the new window takes
focus and the app is its own unit; without systemd it spawns directly.
Rejected: spawning as a child of the compositor (an app would die with it
and share its cgroup), `gtk-launch` (no token, no scope).

**B53 · The switcher reads keys; it never takes focus.** Built in P7. While
Cmd+Tab's chord is held the compositor takes the switcher's keys before the
keyboard engine and eats the rest, and the window in front keeps the seat's
keyboard until the commit on release. A game keeps its pointer lock and its
solid, still Game Mode; none of the bound Cmd chords or Cmd's translation
for a focused shell surface stand in the way; focus moves once per press.
Shift only picks the direction. Rejected: a shell surface with the keyboard,
which needs each of those undone.

**B54 · Previews are compositor drawings that trees draw.** Built in P7, and
the path Mission Control (P16) takes. The compositor draws a window's pieces
into a texture at the size the tree shows it, and the shell copies it on the
GPU into an image Skia owns, so a tree can put marks and icons over it and
the image outlives the texture. Previews exist only while a surface shows
them (v1 D55); parked windows show their last buffer and are not woken;
windows that draw while shown are drawn again a few at a time. Rejected:
drawing previews over the tree's texture (over its marks), borrowing the
compositor's texture into Skia (it could be freed under a draw).

**B55 · Background apps are known by their processes.** Built in P7. The
compositor watches every pid a window has had with a pidfd, and the cgroup of
every scope it launched an app in; an app that has had a window, has none
now and still runs is running in the background, listed last in the
switcher, where Q sends the polite signal. A pid is its last window's app's.
An app started elsewhere is known only once it has had a window. Rejected:
the tray as the only sign (macOS's answer, `switcher.md` For the user 3).

**B56 · Games are named from their launchers' libraries in one crate.**
Built in P7 for Steam: `isle-games` reads `libraryfolders.vdf`, the app
manifests' names and the library cache's portrait cover in both of Steam's
layouts, on the worker, again when the manifests change, and writes none of
it. A `steam_app_<id>` window takes the game's name and cover wherever an app
is named. Heroic, Lutris, gamescope and launching through the launchers join
it in P10.

**B57 · Centre surfaces close the menu-bar surfaces.** Built in P7. Opening
the switcher, and later Spotlight, Mission Control, the capture tool, the
session dialog, Force Quit and the dashboard, closes the bar and its menus,
Control Centre, the window menu, About and the zoom menu, and the other centre
surface. The rule is a compositor function, not the shell's: Control Centre
and the bar are states of surfaces that never close. Rejected: exclusive
groups in `Shell`, which would need those states to become surfaces.

**B58 · Glass is as clear as its text allows.** Taken after P7, when the
user found the switcher, then the island, menus and Control Centre, neither
glassy nor frosted. The Glass setting's default is its clear end
(`glass-clear`); the floor (B39) still raises glass under text toward
`glass-tinted` only as far as secondary text needs to hold 4.5:1, so glass
is clear over a dark or quiet backdrop and darkens over a light one. Glass
with no text on it is never raised; text on such glass sits on a strip of
glass drawn over it, which the floor governs: the switcher's panel and its
name. The island's pill is its own `colour-island` over the blur, with the
glass's tint coming in only as it opens into Control Centre, as the board
draws it; the two tints had stacked to about 0.94. Increase Contrast and
Reduce Transparency still take glass to their forms. Rejected: relaxing the
floor to the Mac's levels (it revises DESIGN.md's contrast rule), a
separate clear material per surface.

