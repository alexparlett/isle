# P8 · Services, notifications and Control Centre

Planned 2026-10-07. Reviewed by the user the same day: every
recommendation taken, Notification Centre's notifications moved in from
P17, night light given to P12, P8 kept as one packet.

The roadmap's brief: the zbus service crates (B11): network, Bluetooth,
power, audio through pipewire-pulse, media, disks; the notification server
and tray (B12); notification banners, OSDs and Control Centre as P1 designs
them (B24); Focus modes (B30). The review added Notification Centre's
notifications from P17's brief.

P7 left a desktop that switches apps but knows nothing of the machine under
it. The island's model takes Wi-Fi, Bluetooth, sound, VPN, battery and Focus
glyphs, privacy marks, live activities and level OSDs, but only the
inhibited and configuration-error glyphs have sources, and the Focus glyph
stands in for Game Mode with the game-in-front flag. Control Centre has its
frame, two modules (Dark Mode, Keyboard layout) and pages that are flat
lists of checked rows; `isle-ui` has no slider, switch, scroll area or
spinner. The media keys still spawn v1's `wpctl`, `brightnessctl` and
`playerctl`. Nothing owns `org.freedesktop.Notifications` or
`org.kde.StatusNotifierWatcher`, the compositor holds no system-bus
connection, and the test harness has no system bus. P8 connects the desktop
to the system: every glyph, mark, OSD and module from its real service,
notifications that arrive, wait and are acted on, the tray, and Focus
deciding what interrupts. What it must get right is that the system's
daemons can come and go, stall or lie without the loop ever waiting on
them, and that what is shown is always the system's true state, a control
falling back to it when an action fails.

## What it answers to

- Decisions: B9 (blocking work on the worker), B10 (one writer), B11 (shared
  zbus crates, audio through pipewire-pulse; revised by choice 3), B12 (the
  notification server and tray roles), B17, B18 (glyphs only when not
  normal; the tray in Control Centre), B24 and B32 (banners top right,
  OSDs on the island, the island never shows a notification), B27 (no v1
  history imported), B30 (Focus modes, the Gaming Focus), B33 and B58
  (glass and its floor), B37 (surfaces), B47 (the island's model is where
  sources post), B49 (one top-layer panel), B51 (the main screen), B52 (the
  launch path), B57 (centre surfaces close the menu-bar ones).
- Intent: `platform.md` Service crates 1–6, Notifications and badges 1–5,
  Tray and global menu registrar 7–8, Focus 1–6, acceptance bars 3 (in
  part), 5, 6, 10 and 11 (in part); `shell.md` Island 2–4, 6, 10, Control
  Centre 1–10, Notification Centre 4–8 and 10, OSDs 1–4, acceptance bars 2,
  3, 5, 9, 11, 14; `gaming.md` Game Mode 1, 2, 3 (the profile), 6, 9, 10;
  `compositor.md` Shortcuts (media keys, Do Not Disturb and Game Mode
  chords).
- Left to P8 by earlier packets: every service glyph, privacy mark, level
  OSD and the media activity (P6 choice 5); every Control Centre module but
  two, the tray, edit mode and the gallery (P6 choice 10); the Focus glyph's
  stand-in (P6 choice 22); the `do-not-disturb` chord (P5); the media keys'
  commands (P5); the logind `PrepareForSleep` watch P6 planned for the clock
  and did not build.
- Design, binding: `p1/structure/control-centre.md`, `notifications.md`,
  `focus.md`, `island.md` (glyphs, privacy, media, OSDs, Now, the tray),
  `competition.md`; `p1/surfaces/control-centre.md`, `notifications.md`,
  `focus.md`, `island.md`; `review/accessibility.md` and `copy.md`;
  DESIGN.md's Control Centre, banners and Focus entries, Components
  (switch, slider, progress, spinner, badge, banner).
- Evidence: `services-2026.md` (new), `ui-and-services.md` §3,
  `macos-model.md`, `design-v1-lessons.md` §3.5 and §8,
  `isle-v1-inventory.md` §2.4, §3.7, §4, §6, §7, §10, §15, §16.

## Choices this plan makes

1. **A crate per daemon family, each with one shape.** New crates:
   `isle-network` (NetworkManager), `isle-bluetooth` (BlueZ), `isle-power`
   (UPower and power-profiles-daemon), `isle-audio` (devices, streams,
   volumes, sounds, the camera), `isle-brightness` (DDC/CI and the
   backlight), `isle-media` (MPRIS), `isle-disks` (UDisks2),
   `isle-notifications` (the server), `isle-tray` (the watcher and host)
   and `isle-focus` (the pure model of choice 12). Each service crate
   exposes a snapshot of the daemon's state behind a watch channel, a
   handle whose commands are async and return the daemon's error, and an
   `available` flag; it never blocks its caller, never panics on a
   daemon's answer, and follows the daemon's name, re-subscribing when it
   returns (`platform.md` Service crates 2, 3). A small `isle-bus` crate
   holds what they share: the name watch, the snapshot channel, a listener
   count for anything that samples, and the system-bus connection. Proxies
   are generated with `zbus_xmlgen` from the daemons' introspection on the
   host, trimmed to what Isle calls and kept in each crate; COSMIC's
   `dbus-settings-bindings` (MPL-2.0, git only) is read as a reference, not
   depended on. The separate apps (P13's Settings) take the crates they
   need. Rejected: one `isle-services` crate with features (the audio
   crate's PipeWire FFI would sit in every build of it), a git dependency
   on COSMIC's bindings (a third pinned repository for generated code).
2. **Services reach the loop as snapshots, coalesced.** Each crate runs
   under the worker's `supervise`. A change posts one job carrying the new
   snapshot, and a burst (signal strength, a stream's volume while dragged)
   posts the last value once per frame at most. The loop keeps
   `Isle::services` with every snapshot, and the island's glyphs and
   marks, Control Centre's frame and the OSDs are derived from it there, in
   one place, so a glyph and its module never disagree. Commands go from
   the loop to a service's handle on the worker and come back as a new
   snapshot or an error for the row they came from. A service that has not
   answered shows its last known state, controls disabled; one that is gone
   reads "Not available" (`structure/control-centre.md` States).
3. **Audio over the Pulse protocol in Rust; PipeWire only for the camera.**
   B11 chose `libpulse-binding` for Pulse's volume semantics, but Arch's
   libpulse links libdbus (`services-2026.md`), the second D-Bus stack B11
   rules out. `isle-audio` speaks the Pulse native protocol to
   pipewire-pulse through the pure-Rust `pulseaudio` crate, on the worker's
   runtime with no thread of its own: sinks, sources, sink inputs, source
   outputs, default devices, volume and mute, subscribed, so volumes go
   through WirePlumber's routes exactly as pavucontrol's do. Per-app streams
   are grouped by app through `application.process.id` and the registry's
   pids (B55), else `application.name`; Isle's own meter and other apps'
   peak meters are skipped. The microphone mark is any source output that
   is not a monitor or a meter. The camera mark comes from a PipeWire
   registry listener (`pipewire-rs`, `module.rt` off so nothing loads
   libdbus) on one thread, marking a node with `media.role` Camera while it
   runs, with GNOME's short clear delay; apps that open `/dev/video*`
   directly are not seen, as on every desktop. This revises B11's audio
   route. Rejected: `libpulse-binding` (libdbus), PipeWire for volumes
   (setting a node's `channelVolumes` is not what WirePlumber's routes do;
   `ui-and-services.md` OPEN).
4. **OSDs come from the change, not the key.** A level OSD is raised when
   the default output's or input's volume or mute, a monitor's brightness or
   the keyboard backlight changes, whoever changed it (`shell.md` OSDs 1):
   the keys, an app, `pactl`, a headset's buttons. The service tags what the
   loop asked for, so a change made from Control Centre raises none
   (`structure/island.md` Pointer). The first snapshot and a change of the
   default device raise none. The volume and mute keys step the default
   output by 5% (Shift by 1%), up to 100%; the brightness keys step the monitor of the main screen
   (B51), whose OSD shows on that monitor. The media keys stop spawning v1's
   commands.
5. **Brightness is DDC/CI on this desktop, written by Isle.** The host has
   no backlight. `isle-brightness` speaks DDC/CI's brightness feature
   (VCP 0x10) over `/dev/i2c-N` itself, a few hundred lines with
   `ddcutil`'s retry and sleep rules as reference, since the Rust DDC crates
   are unmaintained and `ddcutil-service` is a second daemon. Each output
   is matched to its bus by the connector's `ddc` link in sysfs, else by
   reading the EDID at 0x50 and comparing it with the output's identity.
   Reads are cached, writes debounced and serialised per bus, and the
   slider shows the requested value at once (`shell.md` Control Centre 6);
   a monitor that answers no DDC says so in its row. A laptop panel goes
   through logind's `SetBrightness`, and a keyboard backlight through
   UPower's `KbdBacklight`. Access is ddcutil's `uaccess` rule today; P14's
   package ships its own. Rejected: `ddcutil-service` (a daemon Isle would
   start), `ddc-hi` (unmaintained since 2021).
6. **The notification server is owned before the socket is announced.**
   `isle-notifications` serves `org.freedesktop.Notifications` on the
   worker, owning the name before the compositor announces its socket, as
   the registrar is (B50): Notify, CloseNotification, GetCapabilities,
   GetServerInformation, NotificationClosed, ActionInvoked,
   ActivationToken, and KDE's `Inhibited` property with
   `NotificationReplied`. Capabilities: `actions`, `body`, `body-markup`
   (bold, italic, underline; links as text), `icon-static`,
   `persistence`, `inline-reply`, `sound`. IDs count up from 1 and are
   never reused in a session. The server parses hints, resolves the app
   (`desktop-entry`, else the sender's pid through the registry, else
   `app_name`), decodes `image-data` and reads `image-path` on the worker,
   copies the picture into `~/.cache/isle/notifications/`, and hands the
   loop a notification ready to show, so a banner never waits on a file or
   a decode (acceptance bar 9). What happens to it (choice 7) is the
   loop's, where Focus, full screen and the panels are known.
7. **One decision per arrival, and one pure model of banners.** On the
   loop each Notify is decided once, in `structure/notifications.md`'s
   order: transient, muted app, low urgency, held, else a banner.
   `isle-shell`'s new `banners::model` is pure and takes a clock, as the
   island's does (B47): one banner per app with its count and newest
   content, three at most newest first, the oldest temporary one leaving
   early for a fourth, the overflow row, lifetimes clamped to 3–15 s,
   persistent ones, the pause while hovered, arrivals waiting while the
   pointer is on the stack, while a menu-bar surface or Control Centre is
   open on the main screen or an overlay covers it, and the summary once the
   last silencer ends. It answers what the stack shows and when that next
   changes; the loop arms one timer. A banner from an app suppresses that
   app's non-urgent attention event (`Island::set_banner`, already built).
8. **Banners are one placed surface.** `banners` is a top-layer, placed
   surface at the top right of the main screen, under the strip, sized to
   the stack, closed when no banner shows (B37). The tree lays the cards
   out and moves them (arrival from the trailing edge, the stack's moves,
   the swipe), reporting each card's glass shape every frame as the island
   does; Game Mode, Reduce Motion and a full-screen game draw them solid and
   still. Hover and clicks are the surface's; it takes the keyboard only
   while a reply field is open, at the level of a menu-bar surface, and
   gives it back before Send runs (B37). Its hit shapes are the cards, so
   the strip of screen beside a short stack stays the window's.
9. **Records are state; live ones are the server's.** Every arrival that is
   not transient is kept: live while the server holds it (its actions,
   reply and picture usable), a record once it is not and the user has not
   dealt with it (the sender left the bus, the app closed it, a restart).
   Closing or acting removes both. The `notifications` state domain keeps
   the last 300, written by the worker's Saver; pictures stay in the cache,
   so losing the cache costs only thumbnails. A record's click opens its
   app through the launch path (B52). Records start empty (B27).
10. **Actions bring the app forward.** Invoking an action or the default
    sends `ActivationToken` with a fresh xdg-activation token for the
    app, then `ActionInvoked`, so its window may take focus; a resident
    notification stays. A reply sends `NotificationReplied`. An action on a
    sender that has gone turns the notification into a record.
11. **Sounds through the same Pulse connection.** A banner's sound is the
    app's `sound-file`, else its `sound-name` in the sound theme, else the
    alert sound chosen in settings, unless suppressed, muted or held;
    resolved by Isle's own small theme lookup, decoded once (`lewton` for
    Ogg Vorbis, WAV by hand), uploaded as a Pulse sample and played by name,
    with `media.role` Notification so WirePlumber can duck it. One sound for a
    burst within half a second. Nothing held, muted or low makes a sound.
12. **Focus is a pure model in `isle-focus`.** Modes (Do Not Disturb, Game
    Mode, the user's own), each with allowed apps, whether time-sensitive
    notifications pass, and triggers: times (from, to, days) and an app in
    front; Game Mode's game in front and its "Off for these games" list.
    The winner is `structure/focus.md`'s order: Game Mode while a detected
    game is in front (P5's flag, B46), then a mode turned on by hand until
    turned off or its duration ends, then the latest automatic start; a
    scheduled mode turned off by hand stays off until its next start; Game
    Mode turned off over a game stays off for that game. It takes a zoned
    clock (`jiff`) and answers the active mode, why, until when, and when
    that next changes; the loop arms one timer, and `PrepareForSleep` and a
    clock change re-evaluate it. The `focus` settings section holds the
    modes as keyed blocks; the manual state is the `focus` state domain.
    `Focus1` and `isle focus` read and set it with a signal. "Until the end
    of this event" waits for calendars (P20).
13. **Game Mode is the Gaming Focus from P8.** The P6 stand-in goes: the
    Focus glyph, the island's events, the module and the shell's solid and
    still form follow Game Mode being on, whatever turned it on, so a
    manual Game Mode with no game draws the shell solid too (`structure/
    focus.md` effects). On entering, Isle holds the performance profile with
    `HoldProfile` and releases the cookie on leaving; a profile the user
    picks meanwhile ends the hold and stands (`gaming.md` Game Mode 3, 4).
    The `game-mode` chord, which P5 left to P12, is built here beside
    `do-not-disturb` (Ctrl+Option+Cmd+G and +D, desktop chords given up
    over a game). P12 keeps game detection's refinements, VRR, direct
    scanout and idle; night light's part waits for night light (choice 19).
14. **What holds banners.** A Focus (its allowed apps and, if allowed,
    critical ones pass); a full-screen window on the main screen when no
    other screen is free of one, else banners move to the free screen; the
    user away, which in P8 is sleep (logind's `PrepareForSleep`) and in P9
    the idle dim and the lock; a capture of a screen, an input the model
    takes now and P11 feeds. Each is an input to the model with its own
    reason, and the summary's title is "While you played" if Game Mode held
    any. The `Inhibited` property is true while anything holds.
15. **Control Centre's pages become typed.** `panel.rs`'s `PageRow` grows
    into rows of kinds: a choice (a network with signal and security, an
    output, a player, a mode with its durations), a device with battery and
    connect state, a slider with its name column, a switch row, a section
    heading, a collapsed group, a status line, an error in its row, and a
    link to a sub-page. Module bodies gain sliders (Sound, Display), the
    media card and device lists at Large. The panel scrolls between the
    pill's row and the footer past its maximum height, and a page under its
    head. Each module's builder in `control_centre.rs` reads
    `Isle::services`; each action goes to its service and the row shows
    "Connecting", then the result, then the true state.
16. **Components join `isle-ui` as they are first needed.** The slider (on
    glass it fills with `colour-text`, `surfaces/control-centre.md`
    Choices 1), the scrubber, the switch, the progress bar's arc for
    Updates later, the spinner (which P7's quitting mark then uses), the
    scroll area, the reply field and the banner card, each in the gallery
    and `isle-preview`.
17. **The tray is Isle's own watcher and host.** `isle-tray` owns
    `org.kde.StatusNotifierWatcher` before the socket is announced and
    is the host. Items registered by unique name and path or by well-known
    name are kept until their owner leaves the bus; title, icon (by name
    through the icon theme and the item's `IconThemePath`, else its
    pixmaps), attention icon and status, tooltip. A click activates or
    opens its menu page as the item asks; secondary click its menu page;
    middle click SecondaryActivate; scroll passes on. Menus go through
    `isle-menus`' DBusMenu client (B50), drawn as panel pages. The order is a
    `control-centre` setting written by Cmd+drag. XEmbed-only programs (Wine
    and Proton) show nothing; `xembedsniproxy` bridges them for a user who
    wants it. Rejected: `system-tray` as host (its DBusMenu proxy is the one
    B50 rejected, and it would own the watcher on its own terms).
18. **Modules come with their sources.** P8 builds Wi-Fi, Bluetooth, Now
    Playing, Focus, VPN (NetworkManager's VPN and WireGuard connections),
    Sound, Display, Energy Mode, Batteries, and Accessibility (Increase
    contrast, Reduce motion and Reduce transparency; Zoom and the on-screen
    keyboard rows when they exist), with Dark Mode and Keyboard layout from
    P6. Night Light, Updates, Screenshot, Phone and VPN providers are
    absent until their packets, listed in the gallery as unavailable with
    why, as the sheet draws a device that is missing. The `control-centre`
    default goes back to the design's arrangement. Edit mode and the gallery
    are built here, writing the arrangement through the store.
19. **Night light is not P8's.** Its module is in the default arrangement,
    but night light is a colour transform on each output (`compositor.md`
    Outputs 8, its open point GAMMA_LUT or a shader) that no packet holds.
    It goes to P12 beside VRR and tearing, which are the same kind of DRM
    work on the host, and its Displays settings to P13; until then its
    module is absent and Game Mode leaves night light alone.
20. **Disks raise the desktop's own notifications.** `isle-disks` watches
    UDisks2 for a removable filesystem appearing; the notification is
    "{volume} connected" from Files with Open (mounting first, then the
    `inode/directory` handler through B52), Mount and Eject. Nothing
    mounts by itself. The other desktop notifications whose sources exist
    post the same way: the rejected hand edit (once per broken state), and
    the supervisor's crash record ("Isle quit unexpectedly", Report…).
21. **The test harness gains a system bus and fake daemons.** Each test
    starts a second private `dbus-daemon` and sets
    `DBUS_SYSTEM_BUS_ADDRESS`. A test-only crate, `isle-fakes`, serves
    NetworkManager, BlueZ, UPower, power-profiles-daemon, UDisks2, logind,
    an MPRIS player and an SNI item from the shapes the proxies were
    generated from, each able to vanish and return. Audio runs against a
    private PipeWire, pipewire-pulse and WirePlumber started in the test's
    runtime directory with a null sink and a null source, skipped where
    PipeWire is not installed. The test client gains `notify` (Notify with
    hints and actions, printing ActionInvoked, ActivationToken and
    NotificationReplied), `tray` (an item with a menu) and `play` (a stream
    on the null sink).

22. **Notification Centre's notifications are built here.** The review
    moved them from P17 so a missed banner can be found before Isle
    replaces v1. The centre is the island's second panel, unfolding from
    the pill under the clock as Control Centre does from the rest of it
    (`structure/island.md`, B49): one of the two open at a time across
    screens, one morph between them, the pill taking no hover while it is
    open. It holds the date and a fixed six-week month with today in the
    accent, then the live notifications and records stacked by app with
    their layered edges, Earlier for records, Clear per notification and
    per app, and Clear all in two steps (`structure/notification-centre.md`,
    `surfaces/notification-centre.md`). The clock opens it, ending P6
    choice 11's stand-in, and the `notification-centre` chord, which P5
    left to P17, is built here. The agenda waits for calendars (P20) and
    keeps its place empty, never a gap that moves the list.

## The workspace

```
crates/
  isle-bus/              name watch, snapshot channel, listeners, system bus (new)
  isle-network/          NetworkManager: devices, Wi-Fi, connections, VPN (new)
  isle-bluetooth/        BlueZ: adapters, devices, discovery, batteries (new)
  isle-power/            UPower batteries and keyboard backlight; profiles, holds (new)
  isle-audio/            Pulse protocol: devices, streams, volumes, samples;
                         PipeWire camera watch (new)
  isle-brightness/       DDC/CI over i2c-dev, logind backlight (new)
  isle-media/            MPRIS players, position, art (new)
  isle-disks/            UDisks2 drives and filesystems (new)
  isle-notifications/    org.freedesktop.Notifications, hints, pictures, records (new)
  isle-tray/             StatusNotifierWatcher and host (new)
  isle-focus/            the Focus model (new)
  isle-fakes/            fake daemons for tests (new, test only)
  isle-ui/               slider, scrubber, switch, spinner, scroll area, reply
                         field, banner card
  isle-shell/src/
    banners/             the model, the surface
    island/centre.rs     Notification Centre's panel
    island/panel.rs      typed rows, sliders, module bodies, scrolling, edit mode,
                         the gallery, the tray
  isle-settings/         `notifications`, `focus`, `sound`, `control-centre` additions
  isle-compositor/src/
    services.rs          snapshots, coalescing, commands, glyphs and marks
    control_centre.rs    every module's builder and action
    notifications.rs     the decision, banners, records, tokens
    focus_modes.rs       Focus on the loop: timer, triggers, holds, Game Mode
    osd.rs               level OSDs from changes
    binds.rs             media keys through the services
  isle-ipc/              Notifications1 readout, Focus1, Services1, Test1 additions
  isle-cli/              isle focus, isle notify list|close|clear, isle services
```

`Services1` lists each service's availability and snapshot as JSON, for
`isle services` and tests. `Test1` gains the banners model's state, a clock
override for banners and Focus, and the OSDs raised.

## Phases

Each phase ends in a check run by script or in the VM and lands as its own
commits.

### 8.0 · Foundations

- `isle-bus`; the system bus in the harness; `isle-fakes`' skeleton with
  one fake that vanishes and returns; `Isle::services` and the coalescing
  path (choices 1, 2, 21).
- logind on the system bus: `PrepareForSleep` re-arms the island's clock.
- Panel rows by kind, scrolling, and the slider, switch, scrubber, spinner
  and scroll area in `isle-ui`, with gallery and preview pages (choices 15,
  16).

**Check:** headless: a fake daemon's change reaches `isle services` and a
burst arrives once per frame; the fake vanishing marks it unavailable and
its return re-subscribes, with no stall logged; a resume from sleep re-arms
the clock; unit tests for row layout and scrolling; the components'
screenshots against `p1/surfaces-b.html`'s crops.

### 8.1 · Power

- `isle-power`: system batteries, peripherals by kind, `BatteryLevel`
  devices, the keyboard backlight; profiles and holds.
- The Battery glyph; the Energy Mode and Batteries modules and pages.

**Check:** headless against the fakes: a laptop battery on battery shows the
glyph, a low mouse battery shows it, a desktop with none shows nothing and
no module; Energy Mode switches the fake's profile; a hold shows "High Power
while Game Mode is on" and a user's choice ends it.

### 8.2 · Audio

- `isle-audio` (choice 3): devices, defaults, volume and mute, per-app
  streams, the input level meter sampling only while the Sound page shows,
  the microphone and camera marks with their apps in Now.
- The Sound glyph, module and page; level OSDs from changes (choice 4); the
  volume and mute keys through the service.

**Check:** headless with a private PipeWire: `pactl` changing the null
sink's volume raises one OSD; the slider raises none; the mute key mutes and
shows the glyph; a stream from the test client appears under its app with
its own slider; a recording stream shows the microphone mark and its app; a
fake camera node running shows the camera mark; PipeWire killed and
restarted under load logs no stall and the module returns (acceptance bar
6). No libdbus in the process (`/proc/<pid>/maps`).

### 8.3 · Brightness

- `isle-brightness` (choice 5): DDC/CI, buses matched to outputs, cached
  reads, debounced writes; logind's backlight; the keyboard backlight.
- The Display module and page; the brightness keys and OSDs on the main
  screen's monitor.

**Check:** unit tests of the DDC/CI framing and checksum against captured
exchanges; headless with a fake i2c device and a fake logind; in the VM, a
monitor with no DDC says so. The host's AOC is the real check, by the
user's choice of a host run.

### 8.4 · Network

- `isle-network`: devices, wired state, Wi-Fi on and off, access points
  while the page is open, known networks, connecting, VPN and WireGuard
  connections.
- The Wi-Fi and VPN glyphs, modules and pages. A secured network
  NetworkManager has no secret for fails in its row with "needs a password"
  until P9's secret agent opens the prompt.

**Check:** headless against the fake: the glyph by `structure/island.md`'s
rule (wired is normal), a scan only while the page is open, connecting a
known network, a failure shown in its row and the switch back to the true
state; NetworkManager stopped and started logs no stall. In the VM, a real
NetworkManager with its wired connection and a WireGuard connection.

### 8.5 · Bluetooth

- `isle-bluetooth`: adapters, power, devices with batteries, discovery only
  while the page is open, connect, disconnect, pair (a device that needs a
  code waits for P9's agent).
- The Bluetooth glyph, module and page; device batteries in Batteries.

**Check:** headless against the fake: discovery starts on the page's open and
stops on its close, a connected device shows the glyph and its battery,
BlueZ restarting re-subscribes.

### 8.6 · Media

- `isle-media`: players, the most recently playing, position by `Seeked`
  and rate, art from files, `data:` and HTTPS (fetched on the worker,
  capped at 2 MB and 5 s, cached by URL).
- The Now Playing module and page; the media activity's face and start
  event on the island; the transport keys through MPRIS.

**Check:** headless against a fake player: play and pause from the module
and the key, the activity's start event, the scrubber seeking, a second
player taking the face when it plays, a player leaving the bus.

### 8.7 · The notification server

- `isle-notifications` (choices 6, 9, 10): the interface, hints, pictures,
  records in state, Inhibited, tokens, replies; the decision on the loop
  (choice 7's order); `isle notify`.

**Check:** headless with the test client: capabilities and server
information; replacement keeps the id; a transient notification leaves no
record; low urgency makes no banner; closing emits NotificationClosed with
its reason; an action emits ActivationToken then ActionInvoked and the
token focuses the client's window; the client leaving turns its
notifications into records that survive a restart; killing the service
brings the name back within a second (acceptance bar 5).

### 8.8 · Banners

- `banners::model` and the surface (choices 7, 8): grouping, the stack,
  overflow, hover, swipe, reply, the secondary menu (mute for an hour,
  mute), persistent and critical, waiting for panels and overlays, the
  summary; sounds (choice 11).
- The `notifications` settings section: per-app Allow, banner style, sound,
  badges, over full screen; the global holds and the alert sound.

**Check:** unit tests with a fake clock for every row of the sheet's States;
headless: a banner on screen within 100 ms of Notify (acceptance bar 9),
screenshots against `p1/surfaces-b.html`'s banner crops, a reply typed and
received, a click raising the app, the banner waiting while Control Centre
is open, the attention event suppressed while the app's banner shows.

### 8.9 · Notification Centre

- The centre as the island's other panel (choice 22): the clock and the
  `notification-centre` chord open it, notifications stacked by app,
  Earlier, Clear and Clear all, the card's centre form, the date and the
  month; arrivals going into it while it is open.

**Check:** headless: the clock opens it and Control Centre closes; a click
on the pill turns it into Control Centre in one morph; a banner's arrival
while it is open lands in it and no banner shows; the summary's Show and
the chord open it with the newest focused; a record's click launches its
app; Clear all takes two steps; screenshots against `p1/surfaces-c.html`'s
crops.

### 8.10 · Focus

- `isle-focus` and the loop's side (choices 12–14): modes, schedules, the
  app trigger, manual with durations, Game Mode from the game flag, the
  profile hold, the chords, `Focus1` and `isle focus`.
- The Focus glyph, events and module with its page; banners held and
  summarised; the shell's solid form following Game Mode.

**Check:** unit tests of the winner rules with a zoned fake clock (a
schedule across midnight, a manual mode over a schedule, Game Mode over a
manual mode and giving it back, off for a game); headless: Do Not Disturb by
chord holds a banner and the summary follows its end, a test game holds
banners as "While you played", a critical one passing when allowed.

### 8.11 · Disks and the desktop's notifications

- `isle-disks` and the drive notification; the rejected hand edit's and the
  crash record's notifications (choice 20).

**Check:** headless against the fake: a removable filesystem's notification,
Mount and Eject through it; a broken `settings.kdl` notifies once until
fixed. In the VM, a `udisksctl loop-setup` image mounts without a password
(acceptance bar 3, in part).

### 8.12 · The tray

- `isle-tray` (choice 17): the watcher before the socket, the host, icons,
  attention, menus as pages, the order by Cmd+drag.

**Check:** headless: the test client's item appears with its icon, its menu
page activates an item, it goes when its owner leaves; the watcher killed
comes back within a second and the item registers again. In the VM: a Qt
app's tray icon, `nm-applet --indicator` and Discord or Steam if installed.

### 8.13 · Control Centre, completed

- Accessibility and VPN modules, the unavailable modules in the gallery,
  the default arrangement, edit mode and the gallery (choice 18); the
  keyboard map of `structure/control-centre.md` through every page; volume,
  brightness and media keys moving the open panel's controls.

**Check:** headless: edit mode moves, resizes, removes and adds modules and
the setting is written; a keyboard-only walk reaches every module, page,
row and tray item (acceptance bar 14 for this surface); screenshots of the
default arrangement and every page against `p1/surfaces-b.html`.

### 8.14 · The measure, the VM and the host

- Release build at 3440 × 1440: Control Centre's draw on every page and
  banners under 2 ms p95; a banner's latency from Notify; the shell inside
  its budgets with every service live; nothing sampled while hidden
  (acceptance bar 10's helper count and `shell.md` bar 2's frames at rest).
- The VM: real NetworkManager, BlueZ (no adapter), UPower, PipeWire,
  Chromium's and Firefox's notifications with actions, a tray item, a
  player. The host run is optional, as for P6 and P7, and is where DDC and
  Bluetooth are real.

**Check:** the measures in `docs/research/rust-desktop/p8-services.md`; the
VM's screenshots.

### 8.15 · Close

- ARCHITECTURE: how services reach the shell (crates, snapshots, commands),
  how notifications are decided and shown, how Focus decides.
- DECISIONS for what later packets keep: B11's audio route revised; the
  service crates' shape; OSDs from changes; the notification server's
  ownership and the banner model; Focus's winner rules and Game Mode as a
  Focus; DDC/CI written by Isle.
- The intent docs' open points (`platform.md` Service crates and Focus,
  `gaming.md` Game Mode); the roadmap's P8, P9 (the secret and pairing
  agents' rows), P12 (night light), P13, P17 (Notification Centre moved)
  and P20 (the agenda) entries; `state.md`,
  `progress.md`.

## Not in P8

Notification Centre's agenda (P20, with calendars) and the widgets
dashboard (P17); the Wi-Fi, VPN and Bluetooth prompts, the polkit agent, the lock's notification
form, idle and the ScreenSaver interface (P9); the screen privacy mark,
recording and sharing activities and the capture hold's source (P11); night
light and Game Mode's part in it, VRR, idle held by a game (P12); Settings'
Sound, Network, Bluetooth, Notifications, Focus and Control Centre panes,
which the modules' "Settings…" links open (P13); Updates and the restart
owed (P14); VPN and phone providers, calendars and "Until the end of this
event", the Accessibility module's Zoom and keyboard rows, timers and calls
(P20); a camera opened outside PipeWire; XEmbed tray icons.

## Depends on, and feeds

- Needs P3 (surfaces, glass, springs), P5 (the shortcut table, the game
  flag), P6 (the island's model, Control Centre's frame, the top edge, the
  launch path), P7 (the registry's pids for apps of streams and senders,
  attention suppression).
- P9 registers the secret and pairing agents behind the rows that fail
  today, adds idle and the lock as "away" to the holds, and draws
  notifications on the lock. P11 feeds the capture hold and the screen
  mark. P12 builds night light with its module and Game Mode's part in it.
  P13 draws Settings' panes over the new sections. P17 keeps the widgets
  and the dashboard; P20's calendars fill the centre's agenda and raise
  event notifications through the server.

## Done when

- `cargo test` passes headless: every service against its fake, audio
  against a private PipeWire, the server, the banners model and surface,
  Focus's rules, Notification Centre, the tray, Control Centre's modules,
  pages and edit mode.
- In the VM, the island's glyphs and Control Centre show the guest's real
  network, sound and power, and Chromium's notifications arrive as banners
  whose actions reach the page.

## Risks

- **The `pulseaudio` crate is young.** Its high-level client lacks volume
  and subscribe, so Isle uses its protocol layer. If pipewire-pulse
  disagrees with it somewhere, the fallback is `libpulse-binding` on a
  thread of its own, taking libdbus with it, which would be recorded as an
  exception to B11.
- **DDC on NVIDIA.** The proprietary driver's i2c buses may lack the sysfs
  `ddc` link, leaving the EDID match; some monitors need ddcutil's longer
  sleeps. Measured on the host.
- **Banner latency.** Opening a closed surface and mounting its tree on the
  first arrival may approach 100 ms; if it does, the surface stays mounted
  with no texture for a minute after its last banner.
- **Focus's settings shape.** Modes are keyed blocks inside a section, which
  the settings derive may not support yet; if not, the derive gains nested
  keyed blocks rather than the modes moving to a data file, since Settings
  writes them.
- **Fakes drifting from daemons.** The fakes are written from the same
  introspection as the proxies, and the VM runs the real daemons.

## Reviewed

The user took every recommendation on 2026-10-07: Notification Centre's
notifications in P8 (choice 22, phase 8.9); B11's audio route revised as
choice 3, with `libpulse-binding` on its own thread as the recorded
fallback; night light to P12 (choice 19); one packet.
