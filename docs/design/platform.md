# Platform

Intent document for the rust-desktop program. Nothing here is built yet.

Everything under the surfaces: where settings, state and design tokens live
and how they reach the running desktop and the installed apps; the service
crates and the daemons they talk to; the system roles Isle takes
(notifications, tray, menu registrar, polkit, keyring, network and Bluetooth
agents, lock); the portal backend; the compositor's own D-Bus API; how a
session starts, survives a crash, installs and updates on CachyOS; and the
smaller desktop services (Login Items, session restore, clipboard, emoji,
calendars, providers, Focus) with the test hooks a build needs. What the
user sees of these is `shell.md`, and how it is presented is the design
process's (P1, For the design process); the separate apps are `apps.md`;
protocols, rendering, input and Xwayland are `compositor.md`. Facts are
cited to `docs/research/rust-desktop/`; `Dn` is an entry in
`main:docs/DECISIONS.md`.

## Rests on

- B1: Isle is its own compositor, so it owns the session's D-Bus roles and
  API.
- B2: every v1 platform capability is in scope (the parity map below).
- B7: design tokens re-render Freya surfaces as state.
- B9: one main loop; D-Bus, PipeWire and other blocking work on the service
  worker; supervisor and watchdog.
- B10: the compositor owns settings; `settings.kdl`, state and `tokens.kdl`;
  live reload; app theming by templates.
- B11: services are shared zbus crates; audio through pipewire-pulse.
- B12: notification server, tray, polkit agent, keyring prompter,
  NetworkManager secret agent, own lock with PAM.
- B13: Isle's portal backend for ScreenCast, Screenshot and FileChooser; the
  shell draws the picker.
- B14: providers are programs in any language emitting JSON.
- B17: where a choice is open, the Mac answer.
- B18: the AppMenu registrar is owned from session start.
- B21: Isle's own injected keys are a separate input source and are never
  translated.
- B22: no console mode and no Steam shortcuts of Isle's own.
- B24: where banners and live activity appear is P1's; a running cast is
  shown as live activity with Stop.
- B26: the portal backend is a bus name the service worker owns and hands
  FileChooser to Files; the greeter is the compositor in a greeter mode
  under greetd.
- B27: app preferences are sections of `settings.kdl`; writes are refused
  while it does not parse; v1's prefs, window places and launch counts are
  imported once.
- B28: a Bluetooth pairing agent joins the roles; `ext-session-lock` is not
  served; providers are sandboxed with permissions the user grants.
- B29: a PKGBUILD in this repository, built from git, installed with paru,
  updated through pacman.
- B30: Focus modes as on macOS, with an automatic Gaming Focus.

## Features

### Settings

The user changes any setting in the Settings app, from the CLI or by editing
`~/.config/isle/settings.kdl` by hand, and every route applies at once
without a restart. A mistake in a hand edit is shown, and the desktop keeps
running on the last good settings.

1. The compositor is the only writer of `settings.kdl`. The Settings app,
   the shell and the `isle settings` CLI change settings through the D-Bus
   Settings interface.
2. A Set validates the value against the schema (type, range, choice, rules
   across keys) before anything changes. An invalid value returns an error
   naming the key and the reason; file and running state stay as they were.
3. A valid Set applies live, then writes the file atomically (temporary
   file, fsync, rename), keeping the user's comments, order and formatting
   of every node it did not change. Every applied change emits a Changed
   signal listing the keys; no client polls.
4. A key absent from the file has its default. A missing file means all
   defaults, and none is written until the first change.
5. The file is watched through its directory and through symlinks, so
   editors and dotfile managers that replace it by rename or link it from
   elsewhere still trigger a reload. Isle recognises its own writes by
   content and never reloads them as hand edits.
6. A hand edit that fails to parse, fails validation or names an unknown key
   is rejected whole. The last good settings stay in force and the shell
   shows the error with file, line, column and message until a good edit
   replaces it.
7. While the file holds a rejected edit, a Set is refused with that error
   and its file, line and column, so a hand edit is never overwritten (B27).
8. One Rust schema definition drives parsing, validation, the D-Bus types
   and the Settings app's controls, and is introspectable over D-Bus.
9. Each app's preferences, Isle's own apps included, are that app's section
   of `settings.kdl`, set through the same interface and rules. No app
   writes a preferences file of its own (B27).
10. The first start that finds v1's `prefs.json` imports it once:
    preferences into `settings.kdl`, its window places and launch counts
    into state. A marker in state records the import, and no later start
    repeats it. Notification and clipboard history are not imported (B27).

**v1 lessons kept.** D35: nothing writes settings before they have loaded.
D86: a file replaced by rename defeats a watch on its inode. D58: anything
rendered from settings is rendered again once settings and devices have both
loaded.

**Settled in P2.** `kdl` 6.7 parses; `knus` reads only KDL v1. A Set is
written into the file's own text at its value's span, never by printing
the document, which loses whitespace (B36). No `include`. How v1's keys map
onto the new schema is P14's import.

### State

Window places, launch counts, notification history and the rest survive
restarts without the user ever seeing a file.

1. State lives under `~/.local/state/isle/`. It is machine-written and never
   watched (B10).
2. One file per domain: window places, launch counts, session restore,
   notification records, clipboard history and pins, emoji recents, portal
   restore tokens, Focus state. A damaged file costs one domain.
3. Writes are atomic, debounced and done on the worker; none blocks the main
   loop.
4. Each file carries a schema version. An unreadable file, or one from a
   newer Isle, is moved aside with a `.bad` suffix, the domain starts empty
   and the log says so.
5. Nothing secret is written to state, logs or caches: no password, no vault
   field, no clipboard item marked secret.
6. Runtime facts (the lock marker, update progress, pending crash notice)
   live under `$XDG_RUNTIME_DIR/isle/`. Caches live under `~/.cache/isle/`
   and are safe to delete.

**v1 lessons kept.** D26: history is kept as records, not live objects. D43:
anything needed after a reboot is not kept in the runtime directory. D57: an
off-screen place is never recorded (`compositor.md`).

**Settled in P2.** One JSON file per domain with a `version` field,
written by temporary file, fsync and rename, debounced on the worker. Size
caps come with each domain's packet.

### Design tokens

The look is data. Editing a token changes every shell surface and the
installed apps live, which is how the design is tuned.

1. The full shipped set, `data/tokens/tokens.kdl`, is built into Isle and
   typed field by field (B38): colour (dark and light), radius, spacing,
   type, motion, blur, icons. `~/.config/isle/tokens.kdl` overrides only
   what it names.
2. Isle never writes `tokens.kdl`. It is watched and rejected on error
   exactly as settings are (rules 5 and 6 of Settings), and applied as state
   so every Freya surface re-renders (B10).
3. Preferences over the tokens live in `settings.kdl`: accent, appearance
   (light, dark, auto), high contrast, text scale, UI and mono fonts,
   reduced motion. They fold into the tokens before anything draws.
4. Auto appearance is light between sunrise and sunset, computed from the
   time zone's tzdata coordinates with no location service.
5. A token or preference change reaches the compositor's own decorations and
   the installed apps (next feature) in the same pass.

**v1 lessons kept.** D16: tokens have their own names, not Material roles.
D50: high contrast is the same tokens with the quiet ones raised. D52: a
font is one choice that every target follows.

**Open for its packet.** None; the token names and values are P1's.

### Theming installed apps

Apps Isle does not draw wear Isle's colours, fonts, cursor and icons, and
follow light and dark.

1. The parity floor of targets: GTK 3 (adw-gtk3 with `gtk.css` and
   `settings.ini`); GTK 4 and libadwaita (`gtk.css` and the gsettings for
   colour scheme, fonts, cursor and icons); Qt 5 and 6 through the GTK 3
   platform theme; kitty; btop; zathura; the Chromium and Electron flags
   files (`isle-v1-inventory.md` §9).
2. Templates are data shipped under `/usr/share/isle/templates/`. They are
   rendered in Rust on the worker, with no Python.
3. A change to tokens or appearance preferences re-renders every target,
   debounced, and the flags files are re-rendered when the set of installed
   desktop entries changes. Each target is told to reload by its own
   mechanism where it has one.
4. Isle writes, atomically, only files it owns or a marked block inside a
   file. A file the user wrote is left alone; a flags file with the user's
   own ozone line is not touched. `isle setup --undo` removes Isle's files
   and blocks.
5. Isle's portal backend serves the Settings portal ahead of
   xdg-desktop-portal-gtk (`Settings=isle;gtk`): colour scheme, accent
   colour, contrast and reduced motion, from the tokens and the user's
   settings (B13). xdg-desktop-portal-gtk stays installed for what it
   serves besides, and still reads the gsettings Isle writes.

**v1 lessons kept.** D39: Qt themes through the GTK 3 platform theme, one
source for both Qt versions. Chromium themes on Wayland only with
`--qt-version=6` (inventory §16). D73: removing xdg-desktop-portal-gtk would
take dark mode from every GTK app. Under the GTK 3 platform theme Qt opens
GTK's own file chooser rather than the portal's, so Qt apps would miss
Isle's Open and Save panels; P14 tests the alternatives (P1,
`system/theming.md`).

**Open for its packet.** The template engine, and targets beyond the floor
(B10).

### Service crates

Every panel, page and app shows the system's real state and changes it,
through one shared set of crates.

| crate | talks to | serves |
|---|---|---|
| network | NetworkManager (system bus) | Wi-Fi, wired, VPN connections, connectivity, the secret agent |
| bluetooth | BlueZ (system) | adapters, devices, pairing agent, device batteries |
| power | UPower; power-profiles-daemon (`org.freedesktop.UPower.PowerProfiles`) | batteries, profile, holding a profile in Game Mode |
| session | logind (system) | suspend, restart, power off, lid, PrepareForSleep, Lock and Unlock, inhibitors, idle and locked hints |
| audio | pipewire-pulse via `libpulse-binding`; PipeWire via `pipewire-rs` | volume, mute, default devices, per-app streams, capture in use |
| media | MPRIS players (session bus) | now playing, transport, art |
| disks | UDisks2 (system) | drives, mount, eject, power off, SMART |
| gamemode | `com.feralinteractive.GameMode` (session) | game detection |
| accounts | AccountsService, timedate1, locale1, hostname1 | Users, Date and time, Region, About |
| fingerprint | fprintd (system) | lock, greeter, enrolment |
| secrets | Secret Service (gnome-keyring); gcr's SSH agent socket | Keychain, the keyring vault provider |
| brightness | backlight; DDC/CI | displays |
| devices | udev | Device Manager, controllers, input device lists |
| desktop | desktop entries, icon themes, MIME defaults | the app index |
| calendars | HTTPS ICS | calendars |
| packages | pacman, paru | updates |

1. One D-Bus stack: zbus 5. No crate pulls in libdbus, so `bluer` and
   `mpris` are out (B11).
2. A crate has no UI and is used alike by the compositor's worker and by the
   separate apps, which talk to the daemons directly (B11). Each exposes
   current state and a stream of changes, and never blocks its caller.
3. A crate survives its daemon going away and coming back: it shows the
   service as unavailable, re-subscribes on the name's return and never
   panics.
4. A crate that samples (system load, storage) samples only while something
   is listening.
5. No long-running helper processes. A one-shot command is used only where
   the subject has no D-Bus or library interface.
6. A crate panic is contained on the worker and the crate restarts alone
   (B9).

**v1 lessons kept.** D17: polkit-gated calls work only from the session
scope. D72: one failing part never takes the others with it. Inventory §6:
polling counts its listeners.

**Settled in P8.** Proxies are written for what Isle calls, COSMIC's
`dbus-settings-bindings` read as a reference (B60). DDC/CI is written by
Isle (B65). Volume goes over the Pulse protocol to pipewire-pulse, through
WirePlumber's routes (B11). A daemon started on demand (UPower, UDisks2,
power-profiles-daemon) is asked for by name before Isle waits for it.

**Open for its packet.** A CUPS client for Printers. `libalpm` bindings or
pacman's CLIs.

### App index and launching

Every installed app, including an AppImage dropped in `~/Applications`, is
known by name and icon to the launcher, menu bar, switcher and session
restore.

1. The index of desktop entries and icons is built on the worker and updated
   when an application directory changes.
2. It maps Wayland app ids and X11 classes to entries, `steam_app_<id>` to
   the library game, and gamescope windows to the first word of their title.
3. An AppImage in `~/Applications` becomes an entry by reading its embedded
   desktop entry and icon without running it; removing the file removes the
   entry.
4. Apps are launched as children of the compositor, inside the session
   scope, with an activation token (`compositor.md`). `DBusActivatable`
   entries are activated over D-Bus.
5. Default apps are read and written in `mimeapps.list`. The Files app is
   the `inode/directory` handler.
6. Launch counts are kept in state for ranking.

**v1 lessons kept.** D82: an AppImage is read, never executed, to list it.
D17. D73.

**Open for its packet.** Launching each app in its own transient systemd
scope, which helps accounting but must first be measured against D17.

**Settled in P6.** Isle launches into a transient user scope with an
activation token, spawning directly when systemd is not there (B52).

### Notifications and badges

Apps' notifications arrive, wait in the centre, carry actions and replies,
and survive a restart as history.

1. Isle owns `org.freedesktop.Notifications` from session start, served by
   hand on zbus (`ui-and-services.md` §3): Notify, CloseNotification,
   GetCapabilities, GetServerInformation, NotificationClosed, ActionInvoked
   and ActivationToken.
2. Capabilities: actions, body, a sanitised body markup, icons and images,
   persistence, inline reply, sound.
3. Each arrival is also logged as a record (app, summary, body, icon, time,
   urgency); 300 are kept. Icons and images are copied at arrival, so
   records outlive the sender and the compositor; they show without actions.
4. A banner lasts the app's expire timeout within 3 to 15 seconds. Critical
   ones stay until dismissed; low urgency never shows a banner. Silencing
   rules are Focus's.
5. Invoking an action hands the app an activation token so it can raise its
   window.
6. Badges and progress from `com.canonical.Unity.LauncherEntry` signals are
   kept per desktop entry for the switcher's cards and the Apps view's
   tiles (`macos-model.md` §3).

**v1 lessons kept.** D26. A phone's notifications arrive through KDE Connect
like any other (D51).

**Settled in P7.** LauncherEntry's Update is matched on the session bus by
interface and member from any sender; partial updates merge, and an app's
values go when its sender leaves the bus. `urgent` marks urgent attention.

**Settled in P8.** The server is owned before the socket is announced; each
arrival is decided once on the loop, and banners are one pure model (B63).

### Tray and global menu registrar

Other programs' status items sit in Control Centre's tray with their menus,
and the focused app's menus are in the on-demand menu bar wherever the app
exports them, with a designed fallback menu otherwise. The menu bar and
Control Centre themselves are `shell.md`; the Wayland and X11 halves of menu
collection are `compositor.md`.

1. Isle owns `com.canonical.AppMenu.Registrar` at
   `/com/canonical/AppMenu/Registrar` before the compositor accepts its
   first client. Qt decides once per process, and Chromium, Electron,
   LibreOffice and Firefox export only while the name has an owner
   (`macos-model.md` §1.2, §1.3).
2. It records `(window id, sender, path)` for each RegisterWindow, drops a
   sender's entries when it leaves the bus, and answers GetMenuForWindow and
   GetMenus.
3. A DBusMenu client and a GMenuModel client (`org.gtk.Menus`,
   `org.gtk.Actions`, written for Isle since no crate exists) produce one
   menu tree for the shell to draw.
4. A window's menu is found in the order `macos-model.md` §1.3 gives, ending
   with the sender's PID matched to the toplevel's client PID. The app id
   alone never identifies a menu.
5. Every call into an app is asynchronous with a 2 second limit. AboutToShow
   precedes opening a submenu; bursts of LayoutUpdated are batched.
6. A menu that fails, times out or is absent gives the fallback app menu
   (B18), never an empty bar.
7. Isle owns `org.kde.StatusNotifierWatcher` from session start and is the
   tray host. Item menus use the same DBusMenu client (`system-tray` 0.8.9
   or its like, `macos-model.md` §1.5). An item whose owner leaves the bus
   disappears.
8. A restarted registrar or tray service owns its name again, so exporters
   and items register again.

**Open for its packet.** Loading appmenu-gtk-module per launch through
`GTK_MODULES`, with exclusions (`macos-model.md` §1.2). Whether Isle offers
to set Firefox's two prefs. PID matching for Flatpak apps.

**Settled in P6, in part.** The registrar is owned before the socket is
announced, and a taken name is logged and the fallback shown (B50).
Firefox's prefs are not offered; appmenu-gtk-module waits for P10's
launching; Flatpak's PIDs for the host run.

### Polkit agent

Anything that needs an administrator asks once, in Isle's own dialog, never
in a terminal.

1. Isle registers as the authentication agent for the user's logind session
   at session start, before any client can ask.
2. It authenticates through `/run/polkit/agent-helper.socket`, because
   polkit 127's helper is not setuid on the host (`ui-and-services.md` §3).
3. Each request is one shell dialog: the action's message and icon, the
   identity (a choice when several administrators exist), the password.
   Requests queue and show one at a time; a cancel from polkit closes the
   dialog.
4. The PAM conversation runs off the main thread. The password is cleared
   from memory after use and never logged.
5. A dismissed dialog returns polkit's cancel, which callers report as a
   refusal.
6. A restarted agent registers again; a request in flight fails cleanly.

**v1 lessons kept.** D17. D47: pkexec plus Isle's dialog replaces a
terminal. D77 and D79: one authorisation for a whole privileged job.

### Keyring prompter and SSH agent

Unlocking the keyring and confirming access are Isle prompts, and the
keyring opens at sign-in.

1. Isle serves `org.gnome.keyring.SystemPrompter`
   (`org.gnome.keyring.internal.Prompter` at `/org/gnome/keyring/Prompter`),
   which gnome-keyring 50 and oo7-daemon both call (`ui-and-services.md`
   §3).
2. Password and confirmation prompts are drawn by the shell; the secret
   never touches state or logs.
3. The login keyring unlocks from the sign-in password and again at unlock
   through `pam_gnome_keyring` in the greeter's and the lock's PAM stacks. A
   fingerprint sign-in gives PAM no password, so the prompter asks once.
4. The Secret Service is gnome-keyring's; Isle is its client.
5. SSH keys go through gcr's socket-activated agent at
   `$XDG_RUNTIME_DIR/gcr/ssh`, exported as `SSH_AUTH_SOCK`.

**Open for its packet.** Whether gcr's SSH agent sends its passphrase
prompts through the SystemPrompter.

**Answered in P9.** The prompter's `sx-aes-1` exchange is Isle's own Rust,
checked against oo7's test vector and gnome-keyring 50 (B12, B71).

### Network and Bluetooth agents

Joining a Wi-Fi network or pairing a device asks in Isle's own prompt, not
in a terminal.

1. Isle registers a NetworkManager secret agent with the AgentManager at
   session start. GetSecrets for Wi-Fi opens a shell prompt;
   CancelGetSecrets closes it. Secrets go to NetworkManager and are stored
   where it stores them; Isle keeps none.
2. Isle registers a BlueZ agent (`org.bluez.Agent1`) as the default at
   session start, with the KeyboardDisplay capability, on the service
   worker, so pairing asks with Settings closed (B28). Passkey display,
   confirmation and PIN entry are shell prompts.

**v1 lessons kept.** Inventory §7: pairing needed a live agent. D20: a
secret goes from Isle's own field straight to the tool that wants it.

**Open for its packet.** Certificate selection for 802.1X (P13).

**Answered in P9.** WPA Personal, WEP and 802.1X get the fields
NetworkManager's hints ask for (user name, password, private key password).
VPN plugins are asked in their external-UI mode and their fields drawn in
Isle's prompt; a plugin without that mode shows its own dialog (B12).

### Lock, PAM and idle

The screen locks on demand, on idle, on lid close and before sleep, unlocks
by password or fingerprint, and stays locked whatever crashes.

1. The lock is the compositor's own (B12). While locked, no client surface
   is drawn or receives input on any output. The lock surface is `shell.md`.
2. No `ext-session-lock` global is offered, so no other program can lock the
   session or hold it locked (B28).
3. Isle locks on the user's action, on the idle policy, on logind's Lock
   signal, and before sleep: it holds a logind delay inhibitor and releases
   it once the locked frame is on every output.
4. The lock marker is written to `$XDG_RUNTIME_DIR/isle/locked` before the
   first locked frame. A compositor that starts and finds it starts locked,
   and never shows an unlocked frame first.
5. The password goes through a PAM service of Isle's own, installed by the
   package, in the `isle-auth` helper process (B67).
6. Fingerprint runs beside the password through fprintd's D-Bus interface; a
   match unlocks.
7. Caps Lock state comes from the compositor's keyboard state. Locking and
   unlocking set logind's locked hint; idle sets its idle hint.
8. Idle dims, locks and turns screens off at the times in settings; 0 turns
   a step off. Timers are rebuilt when a time changes, never retuned.
9. Idle is held off by idle-inhibit surfaces, by
   `org.freedesktop.ScreenSaver` Inhibit (which Isle serves on the session
   bus), by logind idle inhibitors and by Game Mode.
10. Screens are turned on at start, and the screen-off state is sent again
    whenever the screen list changes.

**v1 lessons kept.** D15 and D43: a locker death strands the session; the
marker, relock at start and logs outside the runtime directory carry over.
D33: fingerprint beside the password, not in front of it. D18. D76.

**Answered in P9.** Isle's PAM service is auth and account only:
`system-local-login` for both, and `pam_gnome_keyring` optional for auth, so
unlock unlocks the login keyring (`data/pam/isle`, B67). Fingerprint (item 6)
waits for a reader.

### Portal backend

Screen sharing, screenshots and file dialogs from any app use Isle's picker
and Isle's file browser.

1. `isle.portal` (UseIn `Isle`) and `isle-portals.conf` ship with the
   package: FileChooser, ScreenCast and Screenshot to Isle; Settings to gtk;
   Secret to gnome-keyring; the rest to gtk. `XDG_CURRENT_DESKTOP=Isle`
   selects them, so no user file is needed.
2. The backend is the name `org.freedesktop.impl.portal.desktop.isle`, owned
   by the compositor's service worker from session start; there is no
   separate portal process (B26). A request's reply is held until the user
   accepts or cancels, and exactly one of those finishes it. The picker is
   parented to the app's window and closes no other surface.
3. ScreenCast offers outputs and windows, cursor hidden, embedded or as
   metadata, and persistence: restore tokens are kept in state.
4. The compositor produces each PipeWire stream (dmabuf where the consumer
   takes it, shared memory otherwise) and rewrites the metadata every frame
   (`smithay-and-gaming.md` §3).
5. A source is captured only while it exists and is shown. A window that
   closes or an output that goes ends its stream cleanly. While any cast
   runs it is shown as live activity with Stop, which ends the stream from
   Isle's side (`shell.md` Island).
6. Screenshot answers interactive requests through the shell's capture UI
   and PickColor from the compositor's frame.
7. FileChooser serves OpenFile, SaveFile and SaveFiles with filters,
   choices, current folder and name. SaveFiles is a folder pick with the
   names joined on. The worker hands each FileChooser request to the Files
   process, started by D-Bus activation if it is not running, which draws
   the Open or Save panel (`apps.md`); its answer completes the held reply.
   Nothing that browses files loads into the compositor (B26).
8. ext-image-copy-capture and ext-foreign-toplevel-list let standard capture
   tools work beside the portal (B13; `compositor.md`).

**v1 lessons kept.** D55: a capture against a vanishing source crashed the
compositor. D67: the reply is held, and the chooser closes nothing else.
D27: the pointer stays out of a shot unless asked for.

**Open for its packet.** The interface between the worker and Files for a
FileChooser request.

**Answered in P11.** Non-interactive screenshots are the frontend's own
check: Isle serves Access, so the permission store's `screenshot` table asks
once per app through a system prompt and remembers the answer; `isle portal
permissions` lists and revokes it until Settings › Privacy & Security (B83).
Settings is served by Isle ahead of gtk (`Settings=isle;gtk`), and FileChooser
stays gtk's until Files serves it (P15). The picker is a sheet on the asking
window, found by its xdg-foreign handle or X11 id, and centred on the main
screen when no window is named or the window goes. Restore tokens are the frontend's; Isle's
restore data names a screen by its identity and a window by its app and
title (B84). A user's own `~/.config/xdg-desktop-portal/portals.conf`
outranks the packaged `isle-portals.conf`, so the install has to see to it
(P14).

### Compositor D-Bus API and CLI

Scripts, the separate apps, keyboard tools and tests reach the desktop
through one documented interface and one command.

1. Isle owns one well-known name on the session bus. Its interfaces are
   versioned: Settings, Session (lock, log out, restart owed), Apps and
   Windows (list, focus, hide, close, quit an app, move to a space), Spaces,
   Surfaces (open, close or toggle a named shell surface), Modes, Focus,
   Displays (list, redetect), Providers, Restore, Updates and Version.
2. Every state change has a signal; nothing needs to poll.
3. The interface is served on the worker, which forwards to the main loop
   over calloop channels and replies when the loop answers, with a time
   limit (B9).
4. Nothing on this API reads screen contents or synthesises input.
   Screenshots go through the portal; input injection exists only in test
   builds.
5. The separate apps learn what only the desktop knows from this API and
   talk to system daemons through the service crates (B11). They read no
   file of the compositor's.
6. `isle` is the CLI over the whole API: one subcommand per method, `--json`
   output, exit codes that mean failure.

**v1 lessons kept.** Inventory §13: the IPC was the test surface and every
hook went through one CLI. D80: a handler must exist from start, not on
first use.

**Settled in P2.** `io.github.alexparlett.Isle`, one object per
interface family under `/io/github/alexparlett/Isle/`, each interface
versioned (`…Isle.Settings1`) (B35).

### Session start and greeter

Signing in at the greeter gives a desktop whose apps can mount drives, join
Wi-Fi and suspend without an admin password.

1. greetd runs Isle's greeter: the compositor started in a greeter mode,
   reusing the lock screen's surface with a user list; there is no separate
   greeter binary (B26). The greeter authenticates through greetd's PAM
   conversation and starts `isle-session` as the user's session. A
   `wayland-sessions` entry lets other display managers start it too.
2. The greeter is installed root-owned by the package, runs as greetd's
   user and lists users from AccountsService. It draws with the shipped
   tokens and the choices the user's session publishes for it (the
   wallpaper, light or dark, the accent, the keyboard layouts and
   per-device profiles, the main screen), so signing in and the lock are
   one face; it never reads the user's token file, so a broken hand edit
   cannot reach it. Before any session has published them it uses shipped
   defaults.
3. Fingerprint sign-in is opt-in. With it on, the greeter creates the greetd
   session as it shows, so the reader listens while a password is typed.
   Autologin is greetd's initial session, set through `isle setup`.
4. The compositor, its worker and every app it launches run in the logind
   session scope, never as systemd user units.
5. Start order: supervisor, compositor, the D-Bus names (Notifications,
   StatusNotifierWatcher, Registrar, polkit, prompter, secret agents,
   Bluetooth agent, portal), then the environment is exported, then
   `graphical-session.target` starts, then Login Items, then session
   restore.
6. The environment, pushed into the systemd user manager and the D-Bus
   activation environment, and removed at logout:
   `XDG_CURRENT_DESKTOP=Isle`, `XDG_SESSION_TYPE=wayland`,
   `WAYLAND_DISPLAY`, `DISPLAY`, `QT_QPA_PLATFORM=wayland;xcb`,
   `QT_QPA_PLATFORMTHEME=gtk3`, `GDK_BACKEND=wayland,x11,*`,
   `ELECTRON_OZONE_PLATFORM_HINT=auto`, `SSH_AUTH_SOCK`, cursor theme and
   size from tokens, v1's NVIDIA variables when
   `/proc/driver/nvidia/version` exists, `~/.local/bin` on `PATH` (inventory
   §3.1).

**v1 lessons kept.** D17. D34: greetd holds one PAM conversation per
session. D39.

**Open for its packet.** The greetd IPC client. Where the session writes
the greeter's choices so greetd's user can read them.

### Supervisor and crash records

A compositor crash costs the open apps but never the session, and the next
start says what happened.

1. `isle-session` starts the compositor as its child and is the only thing
   that restarts it.
2. A clean exit (log out) stops `graphical-session.target`, clears the
   exported environment and ends the session.
3. On a crash it restarts the compositor after a second, backing off after
   repeated quick deaths and never giving up.
4. Each crash leaves a record in `~/.local/state/isle/crashes/`: time,
   build, panic message and backtrace (written by the compositor's panic
   hook), the last log lines and recent watchdog stalls. The next start
   posts a notification saying Isle quit unexpectedly, with Report…, which
   opens the record. The last ten are kept.
5. After three crashes within a minute of start, the next start ignores the
   user's `settings.kdl` and `tokens.kdl` and says so: the island shows its
   configuration-error glyph, whose notice row offers Use my settings,
   which applies them live, and a banner says the same once.
6. Logs go to the user journal and to `~/.local/state/isle/session.log`, the
   previous session's kept beside it.
7. The watchdog logs every main-loop stall over a frame's budget with what
   was dispatching. A shell surface rebuilt after a panic, or a service
   restarted on the worker, is logged and costs no restart (B9).

**v1 lessons kept.** D43: back off rather than stop; logs where a reboot
cannot empty them.

### Packaging and setup

Isle installs from a package, runs from what is installed, and leaves
nothing behind when removed.

1. Isle ships as a PKGBUILD in this repository that builds from the
   repository's git source, installed with paru and from then on updated
   through pacman (B29). Its packages hold binaries in `/usr/bin`, shared
   data in `/usr/share/isle`, the session entry, portal files, D-Bus service
   files, Isle's PAM service, udev rules the apps need, and an NVIDIA
   application profile setting `GLVidHeapReuseRatio` to 0 for the
   compositor's process name (`smithay-and-gaming.md` §2.1,
   `ui-and-services.md` §1.4).
2. Dependencies are the packages'. AUR-only tools are optional dependencies.
3. What a package cannot do is `isle setup`: greetd's config, PAM lines for
   the keyring and fingerprint, enabling greetd and `gcr-ssh-agent.socket`,
   groups. Each step is guarded. `isle setup --check` answers as the user
   and asks nothing when nothing is due; the rest is one authorisation,
   through sudo with a terminal and pkexec without.
4. `isle setup --undo` reverses its own edits. Removing the packages and
   undoing setup leaves no system file of Isle's; the user's files stay.
5. The desktop never runs from a checkout. The host runs the installed
   packages, and no sync copies a checkout into the live desktop.
6. A fresh CachyOS install becomes Isle with one bootstrap command that
   builds and installs the packages with paru and runs setup.
7. The package version names the git commit it was built from, and
   `isle version` reports the same commit.

**v1 lessons kept.** D21: a checkout run live takes the desktop down with a
half-finished change. D22: no installer image. D77, D79: one authorisation.
D83: what Isle calls arrives with Isle.

**Open for its packet.** The split into packages.

### Updates

The system and Isle update from Settings without a terminal, and a needed
restart is said out loud.

1. Updates are checked hourly against a temporary copy of the sync database,
   plus paru's foreign updates. A new commit on Isle's git source counts as
   an update to Isle (B29).
2. An update is `pacman -Syu` through pkexec, with paru building as the
   user, Isle's package among what it builds, and handing pacman to pkexec.
   Isle has no updater of its own (B29). Every question takes its default;
   progress per package comes from pacman's own lines.
3. The run is no child of the compositor or of any app. It runs in its own
   transient user unit, writes its progress atomically under the runtime
   directory, and survives a compositor restart. Exit codes decide failure;
   a line that reads like a complaint is kept as the reason only if the step
   fails.
4. Nothing installs a package without the full upgrade.
5. A restart is owed when the running kernel's module tree is gone, when the
   loaded NVIDIA module is not the installed version, or when a running Isle
   binary has been replaced on disk. Isle says so and keeps saying it; it
   never restarts the compositor itself for an update.
6. A finished run clears half a minute after its recorded finish; a stopped
   run keeps its reason until the next.

**v1 lessons kept.** D46, D47, D78, D81, D83. D84 and D86: a process running
replaced code must be restarted.

### Login Items and session restore

One list says what starts at sign-in, and signing in, or recovering from a
crash, brings back the apps that were open, on their spaces.

1. The list is the XDG autostart entries, system and user, and the enabled
   systemd user units; Isle's own units are locked.
2. Adding an app writes `~/.config/autostart/<id>.desktop`. Turning off a
   system entry writes a user copy with `Hidden=true`. OnlyShowIn and
   NotShowIn are honoured with `Isle` as the desktop name; entries for other
   desktops show as never running here.
3. Login Items start at every compositor start, since their processes end
   with a compositor that crashed. User units follow
   `graphical-session.target`.
4. Isle notes each open app (by desktop entry), its space and whether it was
   hidden, whenever the window list changes.
5. At each compositor start, after Login Items, it launches the noted apps
   not started by them and not already open. Their next windows go to their
   spaces; hidden ones are hidden again. Window places are
   `compositor.md`'s.
6. A setting turns restore off, and the log-out confirmation offers to skip
   it for the next sign-in.

**v1 lessons kept.** D53: by app and space, not by window; what autostart
starts is not started again. D62: what was open comes back after a restart.

**Open for its packet.** systemd's XDG autostart generator or Isle's own
launcher.

### Clipboard history, emoji and symbols

Everything copied is a pick away, pins stay and secrets are never kept;
Ctrl+Cmd+Space opens a searchable character picker that types into the field
in use.

1. The compositor sees every selection change. Text and images are kept
   newest first, capped by count and by size per item, in state. Pins are
   kept apart and never evicted.
2. An offer carrying `x-kde-passwordManagerHint` set to `secret`, or from an
   app on the exclusion list, is not recorded. A vault copy carries that
   hint.
3. A pick sets the selection and pastes into the window that had focus, with
   the app's paste chord from the translation tables (B79).
4. Items can be deleted one at a time or all at once. The primary selection
   is not recorded.
5. Emoji come from unicode-emoji's `emoji-test.txt` with names, groups and
   subgroups; symbols (arrows, mathematics, currency, punctuation, Greek,
   box drawing and the rest) come by their Unicode names.
6. Search matches the start of a name, a word start, anywhere in the name,
   or the group. Recent picks lead an empty query and rank higher; they are
   kept in state.
7. A pick is pasted into the focused field after the picker closes; Shift
   copies instead (B79).

**Settled in P10.** 200 unpinned items or 30 days, 8 MB an item at most;
images persist across a reboot (B78). A character is pasted, never typed,
so a client needs no text-input-v3, only a paste chord (B79).

### Calendars

Subscribed calendars show their events in Notification Centre's agenda, in
the Calendar widget and as notifications.

1. Calendars are subscriptions by link (webcal or https) or by file, any
   number, each named and coloured, and read-only.
2. Each is fetched on the worker every half hour and on change. The last
   good copy is cached under `~/.cache/isle/calendars/` and used when a
   fetch fails.
3. Events, repeats included, are expanded from 60 days back to 400 ahead, in
   local time, by a maintained recurrence library.
4. An event's start raises a notification; upcoming events are served to
   Notification Centre's agenda and the shell's Calendar widget (`shell.md`).

**v1 lessons kept.** D41: a shared ICS link is the one way into Proton
Calendar; recurrence is where hand-written parsers fail.

### Providers

A password manager, VPN, snapshot tool or phone bridge plugs in as a folder,
and Isle shows its state and actions without knowing the tool.

1. A provider is a folder holding `provider.json` (`id`, `name`, `note`, and
   `script` or `exec`). Shipped ones live under
   `/usr/share/isle/providers/<kind>/`, the user's under
   `~/.config/isle/<kind>/`; the kinds are `vaults`, `vpns`, `snapshots` and
   `phones`.
2. The commands and JSON answers are v1's contract unchanged: `status`,
   `list`, `field` (vaults only) and `action` with input on stdin; a
   non-zero exit with the reason on stderr (`isle-v1-inventory.md` §8).
3. `script` runs under python3; `exec` runs any program directly.
4. Each call runs on the worker as a child process in the session scope,
   with a time limit and an output cap. A provider that overruns is killed
   with its process group and reported as its error.
5. Each call runs sandboxed (bubblewrap or Landlock) with only the
   permissions its `provider.json` declares and the user has granted:
   network hosts, paths and bus names. Declared permissions are granted
   when a provider is added and each can be revoked; a revoked permission is
   denied inside the sandbox and the provider still runs (B28; v1 D28, D29, D31).
   The grants are in one place, Settings › Privacy & Security › Providers,
   as switches labelled in words ("Connect to api.open-meteo.com"); everywhere
   else they are read only, with a way there.
6. Secrets are fetched one field at a time when wanted, never cached and
   never logged.
7. An action marked `terminal` opens in the user's terminal. A sign-in feeds
   the tool over stdin under setsid.
8. A new or removed folder is noticed without a restart; Rescan stays on the
   API.
9. The shell renders the state sentence and the actions. Widget data
   providers (B14) run through the same runner; their templates and store
   are `shell.md`'s.
10. Shipped at parity: Proton Pass, gnome-keyring as a vault, Proton VPN,
    NetworkManager VPN, Snapper, Timeshift, KDE Connect.

**v1 lessons kept.** D42, D44, D45 (Snapper reads as the user once allowed),
D51, D20. D31: the host fetches for a widget; the widget never touches the
network.

**Open for its packet.** Whether the shipped providers stay Python.
Bubblewrap or Landlock, and the permission names.

### Focus

Do Not Disturb and named Focus modes decide what interrupts, by hand, on a
schedule or by situation.

1. Focus works as on macOS: Do Not Disturb plus any number of named modes.
   Each has allowed apps, whether critical notifications pass, and an
   optional schedule of days and times (B30).
2. Gaming Focus is one of the modes and turns itself on while any detected
   game (`gaming.md`, Games) is in front, windowed or full screen, and off
   when none is. It wins over any other Focus, one turned on by hand
   included, and gives that Focus back when it ends. Its allowed apps are
   edited like any mode's (B30).
3. A full-screen window (unless another screen is free of one) and sharing
   or recording a screen each hold banners unless the user turns that off;
   sharing one window does not. So does the user being away (idle dim,
   sleep, lock).
4. While silenced, banners and sounds are held back and notifications still
   reach the centre. What was silenced is counted and summarised when
   silence ends.
5. Per-app mute and allow lists apply outside any mode.
6. The current Focus is on the D-Bus API with a signal.

**v1 lessons kept.** Inventory §4: DND, quiet hours, fullscreen and game
silencing and per-app lists; §10: the summary after a game. Inventory §2: a
focus mode was documented and never built.

**Settled in P8.** Game Mode while a detected game is in front wins, then
a Focus turned on by hand, then the latest automatic start; a scheduled one
turned off by hand stays off until its next start (B64).

### Test hooks and VM workflow

Every feature here can be checked by a script, in a VM or headless, before
it reaches the host.

1. The D-Bus API and `isle` CLI are the test surface; every v1 IPC target
   has an equivalent (parity map).
2. Test builds, behind a cargo feature never enabled in the package, add a
   Test interface: inject pointer and keys as a physical device, dump
   surface trees and service state, render an output or surface to PNG,
   force a service panic or a main-loop stall.
3. `isle version` reports the running build's commit and build time, so a
   check confirms the fresh build is running before it reads anything.
4. The compositor runs headless for automated tests: key translation through
   the input filter with a fake focus (`macos-model.md` §2.5), settings and
   token validation, service crates against fake daemons on a private bus.
5. During development the compositor runs nested in a window on the host, as
   the spike did, and in a full DRM session in the VM. In the VM the
   repository is at `/repo`. The guest's supervisor takes a development
   override naming the compositor binary to run; the installed default never
   points at a checkout. `dev/` stays the maintainer's and git-ignored.
6. A surface-only check rebuilds the surface through the Test interface;
   restarting the compositor ends the guest's apps.
7. VM limits stand: llvmpipe memory figures are no baseline, there is no
   cursor plane and no USB hotplug; removable media is tested with
   `udisksctl loop-setup`. The virtual gamepad and the seeded game library
   stay fixtures (inventory §13).

**v1 lessons kept.** D13, D22, D27, D59 (verify by measurement). The rule
that a stale process serves stale answers, so the running build is checked
first.

## For the design process

What this document leaves to P1, per feature; "proposal" is the answer it
gave before.

- **Settings**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/island.md`, `settings.md`); recorded in DESIGN.md.
- **Design tokens**: Answered in P1 (`data/tokens/tokens.kdl` and
  `schema.md`; B31); recorded in DESIGN.md.
- **Notifications and badges**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/notifications.md`, `dock.md`,
  `switcher.md`, `spotlight.md`); recorded in DESIGN.md.
- **Polkit agent, keyring prompter, network and Bluetooth agents**: Answered
  in P1 (`docs/work/rust-desktop/p1/structure/prompts.md`); recorded in
  DESIGN.md at gate 6.
- **Portal backend**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/capture.md`, `island.md`); recorded
  in DESIGN.md at gate 6.
- **Calendars**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/island.md`); recorded in DESIGN.md.
- **Providers**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/widgets.md`); recorded in DESIGN.md
  at gate 6.
- **Supervisor and crash records**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/prompts.md`); recorded in DESIGN.md
  at gate 6.
- **Updates**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/island.md`, `menu-bar.md`); recorded in
  DESIGN.md at gate 6.
- **Login Items and session restore**: Answered in P1
  (`docs/work/rust-desktop/p1/structure/prompts.md`); recorded in DESIGN.md
  at gate 6.
- **Focus**: Answered in P1 (`docs/work/rust-desktop/p1/structure/island.md`,
  `focus.md`, `notifications.md`); recorded in DESIGN.md.

## Acceptance bars

1. Every row of the parity map works on the host or in the VM, shown by a
   script against the CLI or the Test interface.
2. A Qt 6.9 app, Chromium and LibreOffice started by Login Items show their
   menus: the registrar is owned before the first client.
3. From a terminal in the session, `pkexec true` shows Isle's dialog;
   toggling Wi-Fi, mounting a USB stick and suspending ask for no password.
4. `kill -9` of the compositor while locked: the supervisor restarts it
   locked, no unlocked frame is shown, and the crash record exists.
5. Killing each served role (notifications, tray watcher, registrar, polkit
   agent, prompter, secret agents, Bluetooth agent, portal) brings it back
   within a second with its name owned again, and no main-loop stall is
   logged.
6. Stopping and starting NetworkManager, BlueZ, UPower and PipeWire under
   load logs no main-loop stall and leaves every panel correct afterwards.
7. A hand edit with a syntax error leaves the desktop on its last good
   settings and shows the error within a second; fixing it clears the error.
   A Set while the error stands is refused and the file stays as typed. A
   Set through D-Bus keeps the file's comments.
8. Changing the accent re-colours a GTK 3, GTK 4, Qt 5, Qt 6, kitty and
   Chromium window without restarting them where the toolkit supports it.
9. Firefox and Chromium share a window and a screen through Isle's picker;
   closing the shared window ends the stream without a crash; Stop on its
   live activity ends it.
10. An idle session runs no helper process that Isle started, other than a
    provider during its call.
11. No secret appears in the logs, state or caches after a session that
    unlocks the keyring, joins a Wi-Fi network, copies a vault password and
    authenticates through polkit.
12. On a fresh CachyOS install, the bootstrap command gives a working
    greeter and session; `isle setup --undo` and package removal leave no
    system file of Isle's.
13. A new commit on Isle's git source shows as an update; running it from
    Settings rebuilds and installs Isle's packages, leaves the session
    running and shows a restart owed.
14. A provider whose network permission is revoked cannot open a socket, and
    still reports its state.
15. A Firefox file dialog opens the Open panel as a window of the Files
    process, with Files not running beforehand.

## Parity map

| v1 capability | v1 mechanism | lands in |
|---|---|---|
| preferences (about 80 keys) and the `prefs` IPC target | `prefs.json` through a watched FileView, `qs ipc` | Settings (imported once, B27), `isle settings` |
| theme tokens, light and dark, sunrise auto, contrast, text scale, fonts | `tokens.json`, `Theme.qml`, `sun.py` | Design tokens |
| GTK 3 and 4, libadwaita, Qt, kitty, btop, zathura, flags files, Settings portal | `theme/render.py` and templates, `QT_QPA_PLATFORMTHEME=gtk3`, xdg-desktop-portal-gtk | Theming installed apps; Qt's variable in Session start |
| audio, network, Bluetooth, power, media, disks, brightness, game detection | Quickshell types, `pw-mon`, `wpctl`, `udisksctl`, brightnessctl, ddcutil, `gdbus monitor` | Service crates |
| users, date and time, region, devices, printers | Python scripts, `timedatectl`, `localectl`, `lp` | Service crates; pages in `apps.md` |
| desktop entries, default apps, AppImages | `entries.py`, `defaults.py`, `appimages.py` | App index and launching |
| notification server, history, toasts | Quickshell NotificationServer, `notifications.json` | Notifications and badges; history not imported (B27) |
| DND, quiet hours, fullscreen and game silencing, per-app lists | `Notifications.qml` | Focus; game silencing is Gaming Focus (B30) |
| tray | Quickshell SystemTray | Tray and global menu registrar |
| polkit agent | Quickshell PolkitAgent | Polkit agent |
| keyring unlock at sign-in and lock | `pam_gnome_keyring` via `install.sh --pam` | Keyring prompter and SSH agent, `isle setup` |
| keyring items, SSH keys | `secrets.py`, `sshkeys.py`, gcr agent | Service crates; Keychain in `apps.md` |
| Wi-Fi password prompt, Bluetooth pairing codes | Quickshell Networking, `bluetoothctl` agent | Network and Bluetooth agents (B28) |
| lock, fingerprint on lock, logind Lock signal, Caps Lock | Quickshell Pam, `fprintd-verify`, `gdbus monitor`, `hyprctl devices` | Lock, PAM and idle |
| idle dim, lock, screens off, inhibitors | IdleMonitor, `hl.dsp.dpms` | Lock, PAM and idle; DPMS in `compositor.md` |
| file chooser; screen share indicator and Stop | shell-owned portal name; `xdph-streaming` names, `pw-cli destroy` | Portal backend; Open and Save panels in Files, `apps.md` (B26) |
| screenshots, recording, colour pick | grim, slurp, wf-recorder, hyprpicker, satty | Portal backend; capture bar in `shell.md`; frames in `compositor.md` |
| clipboard history, pins, paste into the previous window | cliphist, wl-paste, `send_shortcut` | Clipboard history; history not imported (B27) |
| emoji and symbols | `emoji.py`, wtype | Emoji and symbols |
| calendars | `calendars.py` | Calendars |
| password managers, VPNs, snapshots, phones | `providers.py` and seven provider scripts | Providers, sandboxed (B28) |
| widget data and host fetch | QML widgets, curl | Providers runner, sandboxed with grants (B28); widgets in `shell.md` |
| session restore, reopening Isle's windows, autostart, user services, start hook | `Restore.qml`, `reopen.json`, `autostart.py`, `services.py`, `hyprland.lua` | Login Items and session restore |
| session loop, session log, crash reports | `isle-session`, Hyprland crash report files | Supervisor and crash records |
| greeter, fingerprint sign-in, autologin | greetd with Hyprland and `qs`, `autologin.sh` | Session start and greeter (greeter mode, B26), `isle setup` |
| bootstrap, install, root half, tidy, hidraw rule, dev sync | `bootstrap.sh`, `install.sh`, `isle-root.py`, `tidy.sh`, `70-isle-hidraw.rules`, `isle-dev-sync` | Packaging and setup (PKGBUILD and paru, B29) |
| system updates, Isle's own update, restart owed | checkupdates, paru or yay, pkexec pacman, `update.py`, `restartneeded.py` | Updates (through pacman, B29) |
| IPC targets (island, surfaces, switcher, windows, lock, modes, capture, displays, osk, restore, updates and the rest) | `qs ipc call` | D-Bus API and CLI |
| dev harness, headless checks, keycheck | `dev/`, offscreen `qml`, `keycheck.py` | Test hooks and VM workflow |
| key remapping and the keymap | xremap, `keymap.json` | `compositor.md` (B21) |
| night light, magnifier, display redetect, compositor theme fragment | hyprsunset, cursor zoom, `hyprctl reload`, `hypr/generated/theme.lua` | `compositor.md`; redetect on the D-Bus API |
| window places and launch counts | `prefs.windowPlaces`, `prefs.launchCounts` | State (imported once, B27) |
| Big Picture's Steam shortcuts | `steamshortcuts.py`, `isle-quick.sh` | Dropped (B22) |

## Not here

- Surfaces: the island, menu bar, Control Centre, Notification Centre,
  launcher, switcher, lock and greeter drawing, pickers and dialogs:
  `shell.md`. How anything is presented: P1, the design process.
- The separate apps (Settings, Files, Keychain, Monitor, Cleaner) and their
  pages: `apps.md`.
- Wayland protocols (layer-shell served for other programs, B28), Xwayland,
  rendering, input and Cmd translation, capture frame production, outputs
  and DPMS, window places: `compositor.md`.
- The Open and Save panels and Quick Look: Files, `apps.md` (B26).
- A console mode, Steam shortcuts and Guide-button handling of Isle's own
  (B22); Steam's Big Picture is `gaming.md`'s.
- A Secret Service of Isle's own; gnome-keyring or oo7-daemon serves it.
- RemoteDesktop and InputCapture portals, and emulating GNOME's D-Bus APIs
  for xdg-desktop-portal-gnome (B13).
- Writing calendar events, and CalDAV.
- macOS's "allow in background" for Login Items; Linux has no shared signal
  for it (`macos-model.md` §3).
- Packages for distributions other than Arch and CachyOS.
