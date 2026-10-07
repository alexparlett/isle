# Progress

## 2026-10-07

- P8 8.10: Focus. `isle-focus` is a pure model of `structure/focus.md`'s
  order: Game Mode while a detected game is in front unless turned off for
  it, then a mode turned on by hand until off or its end, then the latest
  automatic start (schedules across midnight, an app in front); a scheduled
  mode turned off stays off until its next start. Do Not Disturb and Game
  Mode are built in, the user's modes are `focus-mode` blocks. On the loop:
  one timer for the next edge, the Focus glyph, "{mode} on/off" events for
  automatic and chord changes, banners held with allowed apps and
  time-sensitive ones passing, Game Mode holding the performance profile and
  releasing it, Game Mode drawing the shell solid and still in place of P6's
  game-in-front stand-in, the summary once the last silencer ends ("While you
  played", or the one held shown again). The Do Not Disturb and Game Mode
  chords, the Focus module and page with durations, `Focus1` and `isle focus`.

- P8 8.9: Notification Centre, moved in from P17. The island's second panel:
  the clock, Ctrl+Option+Cmd+N and the banners' overflow open it, a click on
  the pill turns it into Control Centre in one morph, and no peek replaces
  it. The date, a fixed six-week month from Monday with today in the accent,
  then live notifications stacked by app with layered edges (Show All, a
  clear per app), Earlier for records, and Clear All in two steps. While it
  is open no banner shows anywhere and arrivals go into it. The agenda waits
  for calendars.

- P8 8.8: banners. `isle-shell`'s `banners::model` is pure: one card per app
  with its count, three at most newest first, the oldest temporary one
  leaving for a fourth, the overflow row, lives from the tokens' banner
  timings, time paused while hovered and resumed after a grace, arrivals
  waiting under the pointer or behind a panel, those waiting past their life
  dropped. The `banners` surface is a placed top-layer surface at the main
  screen's top right under the strip, its hit shapes the cards; cards come in
  from the trailing edge, still in Game Mode. Clicks, actions, close and the
  reply field (which takes the keyboard and gives it back) go through the
  server; a banner suppresses its app's attention event. Sounds: the app's
  file or theme name, else the alert, uploaded once to pipewire-pulse as a
  sample and played, one per burst. The `notifications` settings section and
  per-app `notification-app` rules. A banner is on screen inside 100 ms of
  Notify in the headless test. The summary after silence comes with Focus.

- P8 8.7: the notification server. `isle-notifications` serves
  `org.freedesktop.Notifications` (spec 1.3 with `inline-reply`,
  `NotificationReplied`, `ActivationToken` and `Inhibited`), owned before the
  compositor announces its socket; hints read leniently, pictures sent as
  pixels written out as PNGs and files copied into the cache at arrival.
  Each arrival is decided once on the loop (transient, muted, low, held, else
  a banner) against what holds banners; a replacement keeps its id; an app's
  close or its leaving the bus makes a record, the user's close or action
  removes it; actions send an activation token first, which brings the app
  forward. Records persist in the `notifications` state domain, 300 kept,
  transient ones never. `Notifications1` and `isle notify list|close|clear`;
  the test client notifies and prints the server's signals.

- P8 8.6: media. `isle-media` follows every MPRIS player, the most recently
  playing first, its position run on from when it was read, and fetches a
  track's art (`file://`, `data:`, HTTPS capped at 2 MB and 5 s) into the
  cache. The island's media face with the art, the Now Playing module (Large:
  art, track, transport) and its page (the player in full with its scrubber,
  shuffle and repeat, then the other players), the play, next and previous
  keys through MPRIS; every media key now goes through a service, and v1's
  `wpctl`, `brightnessctl` and `playerctl` commands are gone. Media leaves
  Now while the module is in the grid; a page whose rows hold the control
  has a plain head.

- P8 8.5: Bluetooth. `isle-bluetooth` reads BlueZ's object tree in one call
  after any of its signals: the adapter, devices paired and nearby with
  their batteries. The Bluetooth glyph while a device is connected, the
  module and its page (My devices, Nearby devices), discovery started when
  the page opens and stopped when it closes, and a row that connects,
  disconnects or pairs and connects; a device that needs a code waits for
  P9's agent.

- P8 8.4: network. `isle-network` reads NetworkManager whole after any of
  its signals: the wire, Wi-Fi with the network joined, known networks and
  the rest in range one per name, and VPN and WireGuard connections; on the
  host it read the joined network and the unplugged wire. The Wi-Fi glyph
  (on Wi-Fi, connecting, or offline; a wire is normal) and the VPN glyph;
  the Wi-Fi module and page (Ethernet's state, known networks, "Other
  networks" folded, scanning every ten seconds only while the page shows)
  and the VPN module. A secured network NetworkManager has no password for
  says "needs a password" in its row until P9's prompt. Service tests now
  count only stalls in worker jobs, since a first panel draw in a debug build
  can stall the loop by itself.

- P8 8.3: brightness. `isle-brightness` speaks DDC/CI's brightness itself
  and finds each monitor's bus by its EDID, since the NVIDIA connectors have
  no `ddc` link; on the host it read the AOC on `/dev/i2c-4` at 92 of 100,
  as ddcutil does. Laptop panels go through logind's `SetBrightness`. The
  Display module and page (one slider per screen, the panel's first), the
  brightness keys on the main screen with their OSD there, values shown at
  once and written behind, at most one write per 200 ms; the keyboard
  backlight's OSD from UPower.

- P8 8.2: audio. `isle-audio` speaks the Pulse protocol to pipewire-pulse
  over its own async connection on the worker (the `pulseaudio` crate's
  high-level client has no subscribe or volume, so Isle drives its protocol
  layer): devices, defaults, volumes, per-app streams, recordings, an input
  meter that runs only while Sound's page shows; and watches PipeWire's graph
  for a camera node linked to an app. The Sound glyph, the microphone and
  camera marks with their apps in Now, volume and microphone OSDs raised by
  any change but Control Centre's own, the volume and mute keys through the
  service, the Sound module (Wide, with its slider) and its page. Tiles now
  switch on their glyph and open their page from the rest. Tests run a
  private PipeWire with null devices. Found: the NVIDIA EGL driver dlopens
  libdbus itself, so the rule is checked against Isle's own audio modules.

- P8 8.1: power. `isle-power` reads UPower's batteries (the machine's and
  peripherals' by kind, coarse levels included) and power-profiles-daemon's
  profiles and holds; `isle-bus` gains whole-object property reads and a
  follow loop that re-reads a small daemon after any of its signals. The
  Battery glyph shows on battery or when any battery is low; Energy Mode and
  Batteries modules with their pages; a user's choice of profile ends a
  hold. `Test1.Panel` drives Control Centre, and the island's status lists
  its modules and the open page's rows.

- P8 8.0: foundations. `isle-bus` (the buses, a daemon's name followed,
  snapshots, listeners); a private system bus in the test harness with
  `isle-fakes`' logind; services reach the loop as snapshots, a burst's last
  value once a frame (`isle services`, `Services1`); logind's
  `PrepareForSleep` re-reads the island's clock on resume. Control Centre's
  pages hold rows by kind (choices, devices, sliders, switches, headings,
  folded groups, status lines, sub-pages) and scroll past the room they have;
  the body is laid out at the shape's settled height, so a growing panel
  shows no scroll bar. `isle-ui` gains the slider, scrubber, switch, spinner
  and scroll area, drawn in the gallery.

- P8 planned: services, notifications and Control Centre. Reviewed; every
  recommendation taken: Notification Centre's notifications in P8, night
  light to P12, one packet. Research in `services-2026.md`.

- B59: the switcher shows icons, with live previews in the window row only;
  chosen by the user from both drawn in the VM. Theme SVGs without a viewBox
  now scale, so an icon drawn larger than its file fills its box.

- B58 widened: the Glass setting's default is the clear end, so the island's
  panels, menus and Control Centre are clear where the backdrop allows and
  the floor darkens them only as their text needs. The island's pill no
  longer stacks the glass tint under its own fill.

- B58: glass with no text on it is clear and never raised by the floor; the
  switcher's panel is clear and its name sits on a strip of the default
  glass. Chosen by the user from three renders.

- P7: the switcher drawn to the gate-5 board, not the sheet's drifted
  numbers: window-coloured cards with hairline edges, round marks, progress
  in the card, the name centred, titles in the window row; the sheet follows.
  An app with no id is named by its process.

- P7 closed. ARCHITECTURE has How switching works, B53–B57 are recorded,
  the intent docs' open points answered, the plan ends with its outcome.

- P7 7.6: the measure and the VM. Over twenty apps at 3440×1440 the panel
  draws at 0.5 ms p50, 1.4 ms p95; a preview from an uploaded texture costs
  0.25 ms, and changed windows are drawn again four at a time, ten times a
  second. The VM's kitty, xterm, Chromium, Qt Designer, a hidden GTK 4 app
  and a background app read as the board. Found and fixed: the test client
  never let go of its buffers.

- P7 7.5: the drag's icon is drawn, and a drag carries through Cmd+Tab.

- P7 7.4: the panel and its live previews.

- P7 7.3: the hold. A thousand scripted switches commit on release with no
  modifier left held. Found: letting Shift go ended a Shift+Cmd+Tab hold.

- P7 7.2: the model. 7.1: LauncherEntry and Steam's names and covers. 7.0:
  attention levels, background apps, one window order.

- P7 planned: the switcher. Reviewed; every recommendation taken. P6's
  host run waived.

- P6 screenshots in the VM, and what they showed: menus and the inhibit
  notice drew their glass inside their padding, leaving a ring; the About
  panel could not draw SVG icons and clipped its button; a dialog with its
  own app id named itself in the bar; Control Centre's check was the
  filled glyph; a submenu with nothing enabled stayed enabled. All fixed.

## 2026-10-06

- P6 6.8: closed. ARCHITECTURE has How the top of the screen works,
  B47–B52 are recorded, the open points of `shell.md`, `platform.md` and
  `compositor.md` answered, the plan ends with its outcome. The host run
  is the user's.

- P6 6.7: the VM. Qt 6 (Wayland and X11), Chromium 153, LibreOffice 26.8
  on GTK 3 and GtkApplication menubars on GTK 3 and 4 show their real
  menus and take activations; kitty, zenity and xterm fall back. Found and
  fixed: lookups only when the bar was built, a dialog without the app's
  menus, LibreOffice's late-filled menubar (GTK signals only subscribers),
  GTK toggles watched in the wrong group, an unblurred menu opened from
  the keyboard, the bar outliving its window, a blank name for a window
  with no app id, and swapped PC keyboards' hints naming the wrong key.

- P6 6.6: menu search and the measure. Help holds a search field over the
  app's menus, folded titles included, each result with its path; opening it
  asks the app to fill its submenus. `isle menus search`. At 3440×1440 in a
  release build the island and the bar draw at 0.8–1.1 and 0.5 ms p95 and
  the process sits at 163 MB and 92 MiB at most (`p6-island-and-menus.md` in
  research). Found and fixed: a closed tree's state written after it
  unmounted panicked on every reveal, unseen, and the backtraces took RSS to
  500 MB.

## 2026-10-05

- P6 6.5: the menu sources. `isle-menus` reads DBusMenu and GMenuModel into
  one tree and serves the registrar, owned before the socket is announced; the
  compositor serves `org_kde_kwin_appmenu` v2 and `gtk_shell1` v5 with its
  title-bar gestures, reads X11 windows' menu properties over a connection of
  its own (no fork patch needed), and finds an Electron-style registration by
  the app's process. Real menus replace the table fallback, hints map back to
  the Mac chord, the app's Window and Help items join Isle's.

- P6 6.4: the menu bar and the fallback. A push at the top edge outside the
  island's zone, Ctrl+F2 or Shift+Cmd+/ brings the bar down on that screen with
  the system menu, the app menu, File, Edit, View and the rest from labelled
  rules in the translation tables, Window and Help; titles that reach the
  island's reserve fold into ». The window menu opens from a title bar, a
  client's own and its chord, with keyboard move and resize and Keep on top.
  Isle's About panel. Menus1 and `isle menus bar|activate`. Found: Freya sizes
  an absolutely placed fill against the room an auto-sized parent had, so a
  menu's glass reached the bottom of the screen; menus now set their height.

- P6 6.3: Control Centre and the top edge. The panel unfolds from the pill
  with its notices (a rejected configuration, the safe start, shortcuts
  inhibited), activity rows, the packed grid and pages; Dark Mode and Keyboard
  layout are its first modules. Hover peeks without the keyboard, a click or
  Ctrl+F8 pins with it, Escape over a peek is not passed on, a push at the
  edge brings the island back over full screen. A window that takes the keys
  shows a notice inside its top edge. Found: a moving island that drew nothing
  visible stopped the loop's redraws, and a glass shape measured before its
  surface was sized was negative.

- P6 6.2: the island. One queue model sources post to (glyph tiers, privacy,
  ranked activities, events with replacement, OSD pre-emption, waiting and
  dropping), a surface on every output whose shape the compositor springs and
  feeds to the tree with the morph's progress, the clock waking once a minute,
  the main screen following focus. Real sources so far: shortcuts inhibited,
  a rejected configuration or safe start, the layout chord, attention, a game
  in front. The island sits in the overlay layer, so its privacy form stays
  above other programs' overlays. Minimise shrinks into it.

- P6 6.1: `isle-ui`'s first components (glyphs with Phosphor and Isle's own
  four, text styles, buttons, the level bar, the menu with its keyboard), a
  gallery surface in test builds and the preview, and in `isle-apps` entry
  actions, Exec field codes, icon theme lookup and AppStream links. Apps start
  in a systemd scope with an activation token (`isle apps launch|open`). A
  menu's width is estimated from its text: Freya cannot size a row with a
  spring in it to its content.

- P6 6.0: the wallpaper. Frost drawn by a shader, pictures decoded and fitted
  on the worker, per screen or for all, folders, the crossfade and its cut,
  nothing drawn under a full-screen window. A 5K PNG takes about 3 s to
  decode and scale in a debug build.

- P6 planned: island, menus and wallpaper. Reviewed; every
  recommendation taken, kept as one packet.

- P5 host run by the user on tty8: both keyboards and the apps behaved.
  Found and fixed: the US layout by default, and no input after a start on
  a console not in front.

- P5 closed but for the user's host run. ARCHITECTURE describes how keys are
  read; B44–B46 record the seat fed only Isle's keys, the tables as a data
  file, and the game flag.

- P5 5.6: the VM. Cmd chords reach wev and xev with modifiers that agree
  with the keys; Option types # through Xwayland on a Mac profile; GTK 4
  pastes with Cmd+V. xremap stops when the Isle session starts.

- P5 5.5: Sticky Keys and Slow Keys.

- P5 5.4: games from Steam's labels and GameMode, inhibitors and Cmd+Escape,
  Isle's own keys, recording. GameMode is session-activated, so Isle asks it
  only while it runs.

- P5 5.3: the translation tables; every shipped rule typed into an app.

- P5 5.2: the shortcut set, with clashes refused.

- P5 5.1: the engine. Num Lock had reached Freya as a held modifier and
  stopped Tab moving focus in shell surfaces.

- P5 5.0: keyboards, profiles and keymaps.

- P5 planned: keyboard. Reviewed; v1's chords revisited, games read from
  Steam's labels.

- P4 host run by the user on tty8: Chromium on Wayland and X11, VS Code,
  Steam and a full-screen game behaved. Isle no longer draws while its
  console is in the background. For P14: the supervisor's exit clears the
  shared session variables; v1 and Isle share a session log path.

- P4 follow-up: decorations by each protocol's rules, the Smithay fork with
  SHAPE in its X11 window manager, shaped windows bare and blended. xeyes in
  the VM had worn a bar over a see-through body.

- P4 done but for the user's host run. ARCHITECTURE describes how windows
  are kept; B40–B42 record the registry, attached and placed surfaces, and
  the X11 policy.

- P4 4.9: the VM. kitty, Qt 6 Designer, xterm and xeyes wear Isle's bar,
  GTK 4 keeps its own; tiling, minimising and dialogs behave. The
  supervisor never exported the session until the ready pipe was made
  close-on-exec: Xwayland held it open.

- P4 4.8: Xwayland under Smithay's window manager, restarted on its
  display; X11 windows are records, override-redirect and untitled ones are
  left where they ask.

- P4 4.7: the zoom menu from a dwell or Ctrl+Option+Z, and the pointer
  panel with Option in a drag.

- P4 4.6: title bars as attached Freya surfaces, with the lights, the
  title's layout, the resize band; 0.27 MB and 0.47 MiB a bar.

- P4 4.5: rounded corners and shadows from shaders; open, close, hide,
  minimise, tile and desktop motion, close and morph drawn from snapshots.

- P4 4.4: desktops per output and full-screen desktops.

- P4 4.3: `isle-apps`, the close path with keep-running, Cmd+Q, Cmd+H,
  Cmd+M, Cmd+`, pings.

- P4 4.2: zones, edge regions, snapping, tiling chords.

- P4 4.1: placement, places, dialogs.

- P4 4.0: the window registry.

## 2026-10-04

- P3 done. ARCHITECTURE describes how the shell runs; B37–B39 record the
  surface model, typed tokens and compositor glass; `shell.md`'s budgets
  are set.

- P3 3.9: the VM and the host. The probe draws in the guest session on
  llvmpipe and takes QEMU's keyboard and tablet. On the 5070 Ti at
  3440×1440 a full-output draw is p95 0.57 ms, 1.28 ms with 24 elements
  fading in layers; glass blurs in p95 0.75 ms. A one-hour soak held VRAM at
  105 MiB; RSS is unreadable under the host's swap, and the process's own
  memory keeps a few KB a cycle, not yet explained.

- P3 3.8: the preview. `isle-preview <surface>` with live tokens. dx
  0.7.10's hot patch fails on an edit to `isle-shell` (it replays the
  compositor, which the preview does not build); `--watch` rebuilds and
  restarts in 2.7 s.

- P3 3.7: glass. The backdrop behind each glass shape is drawn again
  through its own damage tracker and blurred with dual Kawase passes
  sized from `material-blur`; the floor reads a 4×4 reduction back a
  frame late and raises the tint until `text-2` holds 4.5:1.

- P3 3.6: transitions and the cache. Open and close are compositor
  springs on the texture with a rounded clip; a closing surface keeps its
  last frame until the exit ends. Skia's cache over a hundred opens: 12.7
  MB after the first close, 13.4 MB after the hundredth, under the 64 MiB
  cap.

- P3 3.5: tokens. `isle-tokens` types every shipped token from the file
  at build time; the user's file lies over it by path; `appearance` in
  settings. Found: with fonts loading the loop starts after the bus, so
  the test harness waits for the loop.

- P3 3.4: panic boundaries. Found in Freya: a panic in a component left
  its reactive context on a thread-local stack, so the rebuilt surface
  panicked again outside the boundary; the fork now pops it on unwind.

- P3 3.3: accessibility. Each surface has an accesskit_unix adapter on
  its own D-Bus thread, answered from the loop; `isle shell tree` dumps
  the tree. On a private session with its own at-spi bus the probe's
  window, buttons, field and rows are listed; the host's AT-SPI is off
  and turning it on would persist, so it was not used.

- P3 3.2: input and focus. The seat's focus is a client surface or a
  shell surface; a key-taking surface takes the keyboard and gives it
  back before its actions run. Found: `surface_under` held the layer map's
  lock while asking for it again, which wedged the loop on the first
  pointer motion over the probe.

- P3 3.1: hosting. Surfaces draw through one Skia context into textures
  on the compositor's context and stack against layer-shell. Found in
  the fork: Freya's `Timeout` (every `Input` and `ScrollView`) woke the
  app twice a second forever; it now sleeps until it is reset.

- P3 3.0: `freya-embed` in the fork (branch `isle/freya-embed`), rebuilt
  on `freya-web`'s frame step, with seven tests on Skia's CPU raster. One
  race found: the runner's future can finish early on dirty scopes, so
  `update` loops until it is pending and the waker is armed. bindgen 0.73
  breaks when syn 2 and 3 meet; prettyplease is pinned to match.

- P2 done. ARCHITECTURE describes the core as built; B35 records its
  shape and B36 how settings are written; the roadmap moves second-GPU
  scan-out and v1's units to P14 and mode confirmation to P13.

- P2 host smoke run: Isle lit the ultrawide on the 5070 Ti from tty8 and
  took input. Two findings: the pointer redrew only when something else
  did (fixed: motion queues a redraw whenever Isle draws the cursor), and
  xremap from the Hyprland session rewrites Cmd chords before Isle sees
  them, and would be started by isle-session.target too.

## 2026-10-03

- P2 phase 2.7: the supervisor, tested over the headless compositor: a
  core panic, `kill -9`, three quick deaths into a safe start and back,
  and a clean quit.

- P2 phase 2.6: outputs, layer-shell and DRM. The compositor runs on the
  VM's real DRM in a logind session on tty8. Found there: a GPU opened
  before the session is active cannot become DRM master, so opening waits
  for activation; a re-lit output's first frame drew no windows, because
  the space learns window-output overlap in its refresh, which ran after
  the redraw. Twelve headless tests pass.

- P2 phase 2.5: the headless backend, the Test interface's input,
  outputs, frames and screenshots, a Wayland test client and seven
  integration tests, passing twice in a row in about four seconds.

- P2 phase 2.4: the state store and the worker's debounced writer, with
  tests for a round trip, a damaged file, a newer version and a burst of
  fifty saves written once.

- P2 phase 2.3: settings. A broken hand edit keeps the last good settings
  and refuses Sets with its line and column until it is fixed; Sets keep
  the user's comments, create keyed nodes and app sections where absent,
  and write through symlinks; an editor's rename-replace is picked up.
  Two bugs found in the run and fixed: reading the file raised inotify
  access events, which re-read it every 100 ms; an output's new scale did
  not reach clients' fractional scale. The `binds` section runs a
  terminal, closes the focused window and quits.

- P2 phase 2.2: the worker, the D-Bus API and `isle`. Checked nested on
  a private bus: version, window and display lists, close (kitty asked
  first, as it should), an unknown id refused, `Windows1.Changed` on
  close, quit; a request during an injected 3 s stall failed after 1 s
  while the watchdog logged the stall; an injected service panic was
  restarted in a second with the compositor untouched and no crash
  record. The build time now comes from the binary's own file, since a
  build script only reran on git changes.

- P2 phase 2.1: the nested compositor. One calloop loop over `State {
  backend, isle }`; xdg-shell with toplevels placed centred on their
  first buffer, click to focus and raise, popups kept on the output and
  grabbed, client moves and resizes; the protocol baseline; damage-only
  rendering through niri's redraw states, with an estimated-vblank timer
  for frame callbacks. A heartbeat logs dispatches over a frame's budget,
  a watchdog aborts after ten seconds stuck, and panics write a crash
  record. Found on NVIDIA: Smithay never sets the swap interval, so every
  swap waited about 16.7 ms inside the loop; Isle sets it to 0.

- P2 phase 2.0: the workspace (`Cargo.toml`, `rust-toolchain.toml`,
  seven crates), `data/tokens/` and `data/icons/phosphor/` moved from P1,
  the board server rooted at the repository so boards find them. Smithay
  pinned upstream; the fork waits until a patch needs it. `kdl` 6.7.1
  reprints `} }` as `}}` (its parser discards the space after a children
  block, still so on main), so the settings writer will splice by span
  rather than print the document; a test checks every span in
  `tokens.kdl`.

- P2 planned (`p2-compositor.md`): the workspace at the repository root
  with seven crates and `data/`, the Smithay fork on an `isle` branch,
  and phases from a nested compositor through D-Bus, settings, state, a
  headless test backend, DRM and outputs, and the supervisor, to booting
  the VM into kitty. The user took every recommendation.

- P1 done. Stage 6 reviewed every surface for critique, accessibility and
  copy (298 findings, all applied); the contrast script grew to cover
  focus rings, badges, hover states, labels on fills and kind colours, and
  passes. The user signed off DESIGN.md at gate 6 ("lock it in for now").
  B31–B34 record Frost as data, how attention is presented, where glass
  may go, and the readings of the principles.

- P1 gate 5, wave C passed: Notification Centre and widgets, Mission
  Control and the window surfaces, Files, Monitor, Keychain and Cleaner,
  drawn on the wave C board. A colour-vision check failed stage 4's graph
  colours; a new four pass it, and Cleaner's ten kind colours were checked
  the same way. Stage 5 is complete.

- P1 gate 5, wave B passed: specs for banners, Focus, Control Centre's
  modules, the lock and prompts, Spotlight, capture, Settings and the app
  conventions, reconciled and drawn on the wave B board. The lock adds its
  own wash so its text passes; prompts are one glass frame over a scrim
  with an armed moment; sliders on glass fill with the text colour.
  About seventy tokens added; every glyph the specs name is vendored.

- P1 gate 5, wave A passed: specs for the island, menus, window chrome,
  the switcher and the wallpaper, drawn on the wave A board with a shared
  scene kit. The strip and the revealed bar are both 48 so their centres
  align; colour on the island sits in solid wells; menus are glass in the
  shell, raised in windows, solid over glass. About forty tokens added;
  the contrast check covers the island. DESIGN.md has its first surfaces.

- P1 gate 4 passed: Frost made into the system. The token file grew roles
  for every surface, eight accents, the glass range, macOS's text styles
  in Inter, the concentric corner rule, five springs, a terminal palette
  and the icon group; a contrast script checks every pair in both
  appearances and under Increase Contrast. Phosphor, Papirus and Bibata
  chosen. Research found Isle must serve the Settings portal (B13) and
  that Qt under the GTK platform theme bypasses Isle's file panels (P14).
  DESIGN.md's first draft written.

- P1 gate 2, batch 3 passed: Notification Centre unfolding from the pill
  with the date, month and agenda; the widgets dashboard as one canvas
  per screen on Ctrl+Option+Cmd+W; Mission Control with a minimised tray
  and a short slide between desktops; Files without tags or Connect to
  Server, undo as a stack; Monitor with a GPU view; Keychain's reveal
  gated once on opening. Stage 2 is complete; the intent docs follow.

- P1 gate 3: the user chose Frost. Three directions (Frost, Slate, Dusk)
  were drawn on the harness, now able to switch directions, each from its
  own `tokens.kdl` through a shared kit of components. Batch 3 of the
  structure sheets ran beside it and is on its own board.

- P1 gate 2, batch 2 passed with every recommendation accepted: banners
  top right under the island's strip; Game Mode wins over a manual Focus;
  Control Centre a four-column grid of sixteen modules, media only in Now
  Playing, Game Mode a row of the Focus page; Cmd+Tab sets a prompt
  aside; Spotlight without an assistant, Quick Look on Cmd+Y; one capture
  thumbnail with Markup in Quick Look; the greeter in the user's look;
  System Settings in 28 panes with Game Mode folded into Focus and
  Displays. The batch 1 sheets and the intent docs follow.

- P1 gate 2, batch 1 passed. The first board (menu bar always shown, Dock
  at the bottom) was redrawn when the user preferred v1's minimal desktop:
  revision 1 keeps v1's pill with the clock and active glyphs, opens
  Control Centre from it, brings the app's menus down on demand, and drops
  the Dock for Spotlight's Apps grid and the switcher. Every
  recommendation accepted; standard menu items keep Apple's title case.
  B18 and B19 revised; shell, compositor, gaming, apps and platform intent
  docs, DESIGN, ARCHITECTURE, README and the roadmap (P6 island and menus,
  P7 switcher and launching) updated.
- P1 stage 2 started: the structure template, the harness
  (`p1/harness/`, served by `serve.py`, `p1-boards` in launch.json), and
  batch 1's sheets for window chrome, the menu bar, the island, the Dock,
  the switcher and the wallpaper, with the copy and competition sheets.

- P1 stage 1: research notes on macOS 27, other desktops and v1's
  lessons written; the interview took four rounds (both appearances dark
  first, frosted glass with a slider, Mac density, one picked accent,
  restrained motion, Dock at the bottom, the island a dark pill opening
  on hover, a full-screen widget dashboard kept). The brief passed
  gate 1 ("agreed for now").

- P1 planned (`p1-design.md`). The user chose HTML mock-ups first, then
  Freya from P3's preview mode on, and working files in the repo with
  each gate's board also published as an Artifact.

## 2026-10-02

- The user pointed out that presentation (the island's priority order and
  the like) belongs to a real design process, and that coding agents are
  no longer a feature. B24 rewritten to hand presentation of attention
  surfaces to P1; P1 rewritten as a six-stage design process; coding-agent
  references removed from DECISIONS; the intent docs are being split into
  capability rules and a "For the design process" list.

- B30 refined: the Alt/Super swap is for PC-layout keyboards and is
  suspended in game windows; Mac-layout keyboards need none.

- B24–B30 folded into the intent docs, ARCHITECTURE, DESIGN and the
  roadmap. A coverage audit checked both directions and fixed what
  disagreed (Game Mode wording, the island as the home of casts and
  activities, calendars as notifications, apps' Modes rows, roadmap
  briefs; P22 moved after cut-over). Nothing committed yet.

- The user accepted all 21 proposed answers; locked as B24–B30.

- B23: HDR in scope as P22 after M2. The 21 open points from the intent
  drafts are listed in `brainstorm.md` for the user.

- Intent docs drafted for all five areas. The user dropped Isle's own
  console mode (B22); gaming, shell, compositor and platform docs and the
  roadmap updated.

- The user set the north star (macOS-style on Arch) and parity scope:
  B2 revised; B17–B21 locked (menu bar with global menu and the island in
  the notch, Dock, app-centric windows, Cmd everywhere).

- Round 4 locked B13 (own portal backend), B14 (built-in widgets plus
  providers), B15 (Isle) and B16 (this repo and branch). Brainstorm
  complete.

- Round 3 locked B10 (compositor-owned settings, KDL, live tokens), B11
  (shared zbus service crates, pulse audio) and B12 (prompt, tray and
  lock roles).

- Round 2 locked B8 (stacking with snapping, layout per workspace) and
  B9 (one loop, service worker, panic boundaries, supervisor, watchdog).

- Locked B7: Freya through the user's fork for shell and apps. Round 2 of
  the spike (a 3440×1440 launcher) decided it; final numbers rerun one at
  a time on a quiet host. The user updates the fork to rc.8.

- Spike results in `docs/research/rust-desktop/shell-ui-spike.md`: both
  variants run the island at about +0.1–0.6 ms over the empty base and
  idle correctly; Freya wins text (colour emoji, font features) and GPU
  scaling, Slint wins the iteration loop (live reload 80–96 ms, state
  kept), maturity and VRAM. Large full-output surfaces untested. The base
  now paces by frame callbacks; the host itself paces new windows at
  30 fps (es2gears shows the same).

- Spike base built in `spikes/shell-ui/base`: Smithay 118e34f, winit
  backend, island shape sprung and drawn by GLES shaders, content clipped to
  it. Empty-content floor on the host: morph frame 0.87 ms p50, 1 idle
  frame in 10 s, RSS 145 MB / PSS 81 MB, 38 MiB VRAM. Freya and Slint
  variants handed to two agents in parallel.

- Recon: inventory of v1, Smithay and gaming research, UI and services
  research, in `docs/research/rust-desktop/`.
- Round 1 locked B1–B6. Shell UI rendering compared in depth (Freya on
  Skia, own scene graph, CPU-drawn); the user chose to spike Freya against
  Slint.
- Otto checked at d308f56: the compositor draws dock, exposé, switcher,
  OSDs and window chrome; bar, islands, launcher and lock are otto-kit
  clients, styled by the compositor through `otto-surface-style-unstable-v1`.
