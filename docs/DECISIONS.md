# Decisions

Numbered so code comments can cite them: `// alpha on the effect: D3`.
Each entry is what was decided and why, in a few lines. Measurements and
history go here, not in comments.

**D1 · Hyprland + Quickshell.** Chosen on merit over niri, Sway, KWin
and over Astal, eww and a Rust shell: the only pairing with blur behind
layer surfaces, declarative animation, the gaming set (VRR, tearing,
direct scanout, HDR, NVIDIA) and native service types in one process.
Cost accepted: ~330 MB idle, Hyprland's release churn. Mitigation: the
compositor config is generated, and the compositor is reached through one
service file so niri stays a possible move. Full table in STACK.md.

**D2 · The island is the only persistent surface.** Top centre of the
focused monitor. Other monitors show nothing. Everything else appears on
intent and leaves on its own.

**D3 · Hover unfolds the island into the control panel.** Two hover
states were drawn and rejected: a 64px pill with sliders and buttons
inside it (too dense to read at a glance) and a set of lighter pills that
only enlarged what rest already showed (not worth a hover). The pill now
unfolds downward into a 420px panel: header band, toggles, output and
sliders, media, footer. Sliders and toggles live there and nowhere else.

**D4 · The dashboard replaces the control centre and notification
centre.** One full-screen surface on the control centre keybinding, a
12-column grid of widgets. Notifications are a widget in it. The island's
clock and bell open it too.

**D5 · Widgets are directories with a manifest and one QML file.** The
dashboard draws the card chrome; a widget draws only its content and
declares its sizes and requirements. Third-party widgets drop into
`~/.config/isle/widgets/`. Polled readouts use a command source so a
shell script is enough. Layout is a preference.

**D6 · Default widgets.** Clock, Media, Workspaces, Notifications,
Controls, System, Agents, Session, Devices, Storage. Available but off:
Network, GPU, Games, Clipboard, Timer, Updates, Weather.

**D7 · One bind set on SUPER; xremap shapes it per device.** Hyprland
never learns which keyboard sent a chord. The Mac profile turns Cmd+key
into Ctrl+key for apps except the chords the shell owns, and moves
workspace switching to Ctrl+digit and Ctrl+arrows because Cmd+digit must
reach browsers. Table in ARCHITECTURE.md.

**D8 · Material: dark glass, 78% at blur 24, hairline 8%, accent
`#7FA6FF`.** Two alternates (warm amber, opaque ink) are kept on the
canvas and are a token swap if wanted later. Game mode drops blur and
raises glass to 94%.

**D9 · Controller input is keyboard input.** The Steam Controller's
lizard mode and Steam Input both present as a keyboard and mouse, so
every controller-driven surface is keyboard-navigable with a focus ring
and no gamepad API is needed.

**D10 · Files: yazi first, Thunar for what a TUI cannot do.** Nautilus
was rejected for its indexer; Dolphin for the KDE frameworks it drags in.

**D11 · Hyprland 0.56 facts the code relies on.** Dispatchers over IPC
are Lua expressions (`hl.dsp.focus({ workspace = 2 })`), not the old
`workspace 2` strings, so `Compositor` builds Lua. Gap values are
`{ top, right, bottom, left }` tables; an array is read as zeros. The
example config and the API stubs ship in `/usr/share/hypr/`.

**D12 · Glyphs are SVG data URLs, not effect-tinted images.** The
vendored Lucide bodies live in `shell/assets/Icons.js`; `Glyph` builds
an SVG with the colour baked in and the token's alpha as `opacity`. No
shader effect, no extra Qt module, exact stroke colour. SVG rejects
Qt's `#AARRGGBB` form, which is why the alpha travels separately.

**D13 · The VM's memory numbers are not the baseline.** Under llvmpipe
the guest reports ~320 MB for Hyprland and ~440 MB for the shell,
inflated by software GL buffers. The idle budget in STACK.md is measured
on the real machine when the shell first runs there.

**D14 · Widgets load through the `qs:` scheme.** Quickshell serves the
config as `qs:@/qs/...`; a component loaded by `file://` URL cannot
import `qs.services` or `qs.ui`, and nested modules (`qs.widgets`) and
relative directory imports do not resolve under that scheme. So
`WidgetBase` lives in `qs.ui` and built-ins load via `Qt.resolvedUrl`
from inside the config. A user's own QML widget in
`~/.config/isle/widgets/<id>/` is reached the same way: `install.sh`
links that directory in under the served tree as `shell/userwidgets`
(git-ignored, kept across syncs), and the card loads the widget as
`Qt.resolvedUrl("../userwidgets/<id>/Widget.qml")`, so it is inside the
scheme and imports the services exactly as a built-in does, with a
language server pointed at the config for completion. By default a user QML widget is
sandboxed (D25); `trust: true` opts one of the user's own back into this
full-reach path.

**D15 · Never restart the shell while the session is locked.** The
shell is the locker; if its process dies under an active lock, Hyprland
shows its "lockscreen app died" screen, and lock requests from a fresh
shell fail with a protocol error until `hyprctl eval
'hl.clear_crashed_lockscreen()'` runs and, in practice, the compositor
restarts. The unit's `Restart=always` is right for every other crash;
a lock-time crash is the one to engineer out, not restart through.

**D16 · The app themes are rendered in-repo, not by matugen.** matugen
derives a Material palette from an image and templates against that
palette's names; our tokens are fixed values with our own names. A
sixty-line renderer (`theme/render.py`) substitutes tokens into
templates for GTK 3 and 4 (with adw-gtk3 as the GTK 3 bridge), kitty,
yazi, btop, the portal config and the Hyprland theme fragment, then sets
the gsettings keys libadwaita reads. The shell re-runs it when the
accent changes. (Qt had a qt6ct palette here until D39.)

**D17 · The shell runs in the session scope, not a user unit.** A
systemd user service belongs to no logind session, and polkit's
implicit policies key on that: from the unit, NetworkManager refuses
Wi-Fi toggles outright, udisks demands an admin password to mount a USB
stick, and logind demands one to suspend. Started from the compositor's
start hook, the process sits in the session scope and all three are
allowed. `shell/scripts/isle-session` provides the restart the unit used
to; xremap stays a unit because it needs nothing from polkit.

**D18 · Idle monitors are recreated, not retuned.** An `IdleMonitor`
whose `timeout` changes while it is enabled never fires again, and one
created with a timeout of 0 fires at once, which is how the shell locked
itself at startup while a preference was still loading. Each idle policy
is a Loader that builds its monitor with the final timeout and rebuilds
it when the preference or game mode changes; 0 means no monitor.

**D19 · Mission Control is drawn by the shell, not a compositor plugin.**
The plugins in this space, hyprexpo, Hyprspace and hyprtasking, are
exposés of workspaces: each workspace a scaled tile with its windows in
place, in the plugin's own colours and borders, and the only one still
tracking this Hyprland (hyprtasking, pinned to 0.56.2) has no spread of
windows, no title or close under the pointer, and no hidden-window tray.
macOS's Mission Control is itself drawn from window snapshots, which is
what the shell already has through `ScreencopyView` on each toplevel, and
the surface then shares the shell's material, motion, keyboard and drag
handling with everything else. A plugin would buy animating the real
windows, which snapshots animated from their real rectangles give as
well. Gestures come from the compositor's `hl.gesture`, which runs a
function.


**D20 · The Proton sign-in is the CLI fed over stdin, not a terminal.** The
CLI reads the password and the 2FA code with `getpass`, which falls back to
stdin when it has no terminal; run under `setsid` it never has one, so the
shell's own fields (as for Wi-Fi passwords) hand it the secret directly
and the code when it asks. The alternative, opening a terminal for the
prompt, kept the password out of the shell but put a stray window in the
flow.

**D21 · The desktop runs from an installed copy, not the checkout.** With
`~/.config` linked straight into a working checkout, every edit and every
pull was live at once, and a half-finished change or a broken file took the
running desktop down with it. `tools/install.sh` now copies the checkout to
`~/.local/share/isle` and links into that; the VM mounts the checkout for
trying things first. Copy rather than clone, so uncommitted work installs
too when it is wanted, and the copy keeps its git history for Settings ›
Updates.

**D22 · No installer ISO; the VM harness is local.** The ISO build laid
Isle over CachyOS's live image and needed a first-login "finish setup"
step for what only a session can do; the one-line bootstrap does the same
job from a normal CachyOS install. The QEMU harness, guest provisioning and
screenshot tools are the maintainer's and now live under `dev/`, which git
ignores, so the repository holds the shell and its install and nothing
about how it is tested.

**D23 · A widget library with manifest-only widgets before QML ones.** The
picker only listed what was missing, and a second clock or a widget of
the user's own had no way in. The library lists everything by category,
instances get keys, and a user's widget is a manifest alone, a command
or a file with a view, drawn by one renderer in the shell's own tree so
it needs neither code nor the service access user QML still lacks (D14).
Widgets in QML come after the text ones show which services they need.
The library, and the form for a text widget, live in the dashboard's edit
mode rather than Settings: the dashboard is where they are placed.

**D24 · The dashboard is a grid editor, not a size-cycle.** Cards moved by
springing back on a bad drop and resized by a badge that stepped through a
fixed list. The editor now shows a live outline of where a drag lands,
swaps a same-size neighbour or pushes a different-size one down to make
room, resizes from the card's edges between the manifest's `min` and `max`
spans, and takes arrow keys on the focused card. Multiple pages, switched
by tabs or the number keys, hold their own layouts. All of it is QML over
the prefs layout: a compositor plugin has no view of a surface the shell
draws, so there is nothing for one to do here.

**D26 · Notification history is records, not notifications.** A live
notification is an object the server holds for the app: actions, reply, an
image. It cannot outlive the shell, so a restart used to empty the list. Each
arrival is now also logged as a plain record (app, summary, body, icon, time,
urgency) to a file, and the centre shows records that are not live under
"Earlier": readable, but with no actions, because there is nothing left to
act on. The centre lives in the island's panel rather than a surface of its
own so it is one hover away without leaving the window in use; the dashboard
widget shows the same list for when the dashboard is already up.

**D27 · Hardware cursors on, so a capture never contains the pointer.** The
compositor's default for `cursor.no_hardware_cursors` is "auto", which turns
hardware cursors off on NVIDIA and draws the pointer into each frame as
software. Screencopy then takes the frame as drawn, so every screenshot and
the frozen frame under the region picker carried the pointer, whatever grim
was told, and a `cursor.invisible` toggle around the grab applies a cursor
update too late to help. With the open driver, hardware cursors work, and
the pointer lives on its own plane that screencopy leaves out unless asked
for with `-c`. The test VM cannot show this: its virtual GPU has no cursor
plane, so it draws the pointer in software regardless.

**D28 · The widget library is a store, and grants are the user's.** A
list of names with a tick was enough while every widget was the shell's
own. Widgets written by others need what a package manager shows before
install: a picture, the author, a version, a licence, what changed, and
what it touches. The manifest carries the identity fields; the picture is
a shipped screenshot or, failing that, the widget itself drawn live, so
nothing is ever blank. Permissions are declared by the author but granted
by the user: a per-widget list in prefs narrows the declared set, the host
wrapper is built from the intersection, and a revoked scope turns that
service's calls into no-ops rather than refusing the widget, so a music
widget can still show the track after its transport has been taken away.
Sources, installing from a registry and updates build on this manifest.

**D29 · Sources are git repositories, and trust is granted twice.** A
registry could be a web service with an API; a git repository with an
index file needs no server, is forked and mirrored for free, carries its
own history, and is what widget authors already have. Screenshots and
READMEs come along in the clone, so the store shows them without a
second fetch. Installing copies (or clones a tagged repository) into a
staging directory and swaps it in, so a failed download never leaves a
half widget. Full reach is asked for by the author in the manifest and
granted by the user at install, or later in the sheet; a widget from a
source with the ask but not the grant is installed and blocked, so the
user can read it first. The user's own widgets skip the second step: they
wrote them. The consent dialog shows grants as toggles already on rather
than an all-or-nothing accept, because a music widget that may read the
player but not drive it is a reasonable thing to want.

**D30 · Author tools live in the store, not in an editor.** A widget is a
folder, and every editor already edits folders; what none of them has is
the shell's reading of one: whether the manifest is whole, whether the
QML stays inside the sandbox, what the listing will look like. So the
store's sheet for one of the user's own carries exactly that (Check,
Picture, Tag) and leaves the writing to the editor the user has. Picture
grabs the live preview rather than the screen, so it needs no compositor
round trip, never contains the pointer, and gives the same image on every
machine. Tag stops at a local tag: pushing to a remote and asking a
registry to list it are the author's own moves, said in words on the
sheet and in the guide.

**D31 · The host fetches for a widget; the widget never touches the
network.** Most widgets people want are windows onto a service: weather, a
calendar, a build queue. The sandbox forbids `XMLHttpRequest`, and lifting
that would hand every widget the whole network. Instead the manifest names
hosts, one per `fetch:<host>` permission, the store shows them as pills
the user can switch off, and the host performs the GET with curl on the
widget's behalf, bounded in time and size. A widget's own state goes the
same way: `store` keeps it in prefs per instance rather than letting the
widget write files. Notifications are a scope of their own (`notify`)
because posting one is a different act from reading the count.

**D32 · A Mac keyboard's symbols follow its profile, not the layouts
list.** The layouts list is the languages the user types; the Macintosh
variant is where a Mac keyboard prints its symbols. Putting the variant
in the list would move @ for every keyboard, so a Mac-profile device gets
the variant on its own device line instead, and the list stays plain.

**D33 · Fingerprints beside the password, not in front of it.** Putting
`pam_fprintd` in the lock's PAM stack makes PAM wait on the reader before
it will take a password, so a user with wet hands stares at a dead field.
The lock runs fprintd's own verify in parallel and treats a match as the
unlock, the way hyprlock does over D-Bus; the password path stays exactly
as it was. The driver for the Goodix 27c6:5042 is the community HTK32
driver with the id added: same sensor, same CDC-data interface and
endpoints, three other ids in its table. It turned out to be a different
firmware (`GF5288_HT_APP`, no security layer, no touch events, its own
frame layout) and after a patch that got it enrolling, matching stayed
poor because the frame layout was never fully decoded. That work was
dropped in favour of a reader libfprint already supports; the script keeps
only the ids the community driver covers. Building it is a script, not the
installer, because it pulls opencv and a compiler and most machines have
no reader.

**D34 · Fingerprint sign-in goes through PAM, and the greeter starts the
session before the password.** The lock screen can run fprintd beside
its password field because it owns both. The greeter cannot: greetd runs
one PAM conversation per session, and `pam_fprintd` in that stack holds
the conversation while the reader listens. So with sign-in enabled the
greeter creates the session the moment it shows, displays what PAM says
("Place your finger…"), and keeps a typed password until PAM asks for it,
which is after the reader gives up (eight seconds, one try). That wait is
the cost of the feature and is said on the toggle, which is why it is
opt-in and separate from the lock screen, where a finger costs nothing.
sudo is the plain `pam_fprintd` line at the top of its stack. Both edits
run as root through pkexec, so the shell's own auth dialog asks.

**D35 · Eight rows, not four.** With four rows the smallest card was a
quarter of the screen tall, which is far more than a row of tray icons
or two device batteries need. Doubling the rows halves the unit; every
saved layout and manifest is doubled once so nothing moves, and the
widgets that were mostly air get a one-row size as their default, and
Agents and Notifications take a `min`/`max` range rather than a fixed
list, since how much of the screen they deserve depends on the day. The
first cut of the migration wrote before the prefs file had loaded and
reset it to defaults; every write now waits for `Prefs.loaded`.

**D36 · A widget may bring its own settings pane.** The manifest's
`settings` list covers a toggle, a choice, a number and a text; the
System widget needs a list of series with a colour each, which no row
of that kind can express. Rather than grow the generic form a type at a
time, a manifest may name an `editor` file that the gear loads in place
of the rows, with the values and a setter. The generic form stays small,
and a widget that needs more writes exactly the form it needs; the
editor is scanned like `Widget.qml`, so the sandbox holds.

**D37 · The on-screen keyboard is a shell surface typing through
wtype.** wvkbd works, but it is another window with its own look, no
controller, and no room for the shell's conventions. A surface of the
shell's own draws in the shell's material and takes the pad through the
same helper Big Picture uses. Speaking the virtual keyboard protocol
from QML is not possible and a Python client would be a second input
stack; `wtype` already speaks it, so every key is one short run, queued
so fast taps land in order. The console keyboards (Steam Deck, the
PlayStation, the Xbox) set the pad conventions: two cursors on two
sticks with the triggers pressing, a legend of the buttons, a strip of
completions; those are borrowed rather than invented.

**D38 · Isle is the console's system layer, not its store.** The first
Big Picture was a tile grid that launched Steam's gamepad UI, a launcher
for a launcher, and it read the pad raw while Steam Input wanted the same
pad. Steam, Heroic and Lutris each keep their own libraries, updates and
cloud saves; reimplementing that in the shell would never catch up.
So the rule is that whoever is fullscreen owns the pad: Steam's UI and
its games get Steam Input untouched, the shell's home shows only when
nothing else is in front, and the mode follows Steam's own Big Picture in
both directions. What the shell adds is what none of them has on a Linux
desktop: the system quick menu, the pad turned into keys for launchers
without support (Heroic's console mode, Lutris), the on-screen keyboard,
notifications held and summarised, and two tiles inside Steam's UI that
reach the shell's menu and the way out. The console conventions (rows,
a Guide tap for the menu and a hold for home, a legend) are borrowed.

**D39 · Qt themes through the GTK bridge, not qt6ct.** `QT_QPA_PLATFORMTHEME`
names one theme plugin, and qt6ct answers only for Qt 6, so Qt 5 apps
were left with Fusion's light defaults. The GTK platform theme exists for
both versions and derives its palette from the GTK theme Isle already
renders: measured, it yields the same window, text and accent colours as
the qt6ct palette did, and the right font, which the qt6ct template had
been getting wrong. One source for every toolkit, so the qt6ct templates
and package go.

**D40 · A window's place is its app's main window only.** The Windows
service put every new window of a class at the class's remembered place
a moment after it mapped. Steam's menus are X11 popup windows of the
client's class, so each menu was resized and moved to the main window's
place, and the recorder then noted the menu's own geometry as Steam's:
weeks of misrendering read as a compositor or driver fault while the
compositor, its plugins, the driver and the rules were all cleared one by
one. A place is the size and position of an app's main window, which is
its only titled window; an untitled window is a menu or an overlay and a
second titled one a dialog, and neither is noted nor placed. The X11 float
rule likewise takes titled windows only, leaving a popup to the
compositor's own X11 placement.

**D41 · Calendars are subscriptions, not a Proton connector.** Proton
Calendar has no CalDAV and Proton Bridge carries no calendars, so no
client syncs it live; what it gives is Share via link, a read-only ICS
address. That is the same thing Google's private iCal address, Outlook's
published calendar and any hosted .ics are, so Isle takes subscriptions
by link, any number, each with a name and a colour, and treats Proton as
one of them. Read-only, and as fresh as the half-hour poll: a shared link
has nothing to push. Parsing and repeats come from python-icalendar and
python-recurring-ical-events rather than a parser of our own, since
recurrence rules are where hand-written parsers go wrong.

**D43 · A lock outlives the shell.** The dev sync restarted the shell
while the session was locked; the locker died with it, Hyprland showed
its dead-lock screen and refused every new locker, and the screens,
turned off by idle, never came back either, since the fresh shell's idle
monitor had seen no wake. Three things, each general: the Lock service
keeps a marker in the runtime directory while locked and a starting shell
that finds it locks at once; the compositor is allowed to hand a dead
lock to that fresh locker (`allow_session_lock_restore`, safe because the
shell relocks in the same breath); and the Idle service turns the screens
on at start. The sync itself now waits while the session is locked or a
screen is off, asking `lock locked` over IPC.
