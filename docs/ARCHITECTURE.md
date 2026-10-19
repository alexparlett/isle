# Architecture

How Isle is put together. The compositor core (P2), the shell runtime
(P3), windows (P4), keys (P5), the top of the screen (P6), switching (P7),
services with notifications and Control Centre (P8), and the first app,
Settings, on the platform every app shares (P13) are built; the rest is the planned architecture of the rust-desktop program, each part settled in
detail by the build packet that makes it (`docs/work/rust-desktop/
roadmap.md`). What each capability does is in `docs/design/`; why it is
built this way is in `docs/DECISIONS.md`.

## Process layout

```
isle-session                      supervisor: starts the compositor in the
 │                                logind session, exports the session's environment,
 │                                restarts it, keeps crashes and the log (B9, B35)
 └── isle-compositor              the compositor, one process
      ├── main thread             one calloop loop (B9)
      │    ├── Wayland server     xdg-shell, layer-shell, the protocols in compositor.md
      │    ├── input              libinput, per-device keyboard profiles, Cmd translation (B21)
      │    ├── outputs            DRM/KMS through the Smithay fork (B6), VRR, tearing
      │    ├── Xwayland WM        Smithay's X11 window manager (B5)
      │    ├── window model       stacking with snapping, per-workspace layout (B8, B20)
      │    ├── shell              Freya surfaces drawn by Skia on the compositor's
      │    │                      EGL context: island, menu bar, Spotlight,
      │    │                      switcher, Control Centre, notifications, widgets,
      │    │                      lock (B7, B24), and the server-side title bars (B25)
      │    └── renderer           Smithay's GLES renderer composites windows and surfaces
      ├── service worker          tokio runtime: the D-Bus API on
      │                           io.github.alexparlett.Isle (B35), the settings
      │                           store and its watcher (B10, B36), state writes,
      │                           the service crates (B11), the system
      │                           roles (B12, B28), the menu registrar (B18), the
      │                           settings D-Bus interface (B10), the portal backend
      │                           xdg-desktop-portal-isle (B13, B26), and the runner
      │                           for providers (B14)
      ├── watchdog thread         ends a dispatch stuck for ten seconds; the loop
      │                           itself logs any over a frame's budget (B9, B35)
      ├── isle-auth               the lock's PAM conversations in a helper process,
      │                           started on the first password and kept until
      │                           unlock (B67)
      └── isle-record             a screen recording's encoder, started per
                                  recording, reading the compositor's own stream
                                  (B82)

isle                            the command line over the D-Bus API
Apps, launched on demand        Settings, Files, Monitor, Keychain, Cleaner:
                                Freya processes on isle-app and the same component
                                library, drawn in software unless built with the
                                GPU (B26, B94); Files also serves the Open and Save
                                panels and Quick Look (B26)
Providers                       programs in any language, started by the service
                                worker, sandboxed with the permissions the user
                                granted (B14, B28)
Greeter                         the compositor in greeter mode under greetd (B26)
Xwayland                        X11 clients, managed by the compositor's WM
System daemons                  NetworkManager, BlueZ, UPower, UDisks2,
                                power-profiles-daemon, gamemoded, PipeWire with
                                pipewire-pulse and WirePlumber, polkit, the keyring,
                                greetd
```

## How the core runs

- **One loop over two halves.** `State { backend, isle }`: the backend
  (udev on hardware, winit nested in another compositor, headless for
  tests) beside everything else, so drawing can borrow both. Every
  protocol handler is implemented on `State`.
- **Frames only for change.** Each output has a redraw state (idle,
  queued, waiting for vblank, waiting for an estimated vblank). A change
  queues a redraw; queued outputs draw at the end of a loop iteration,
  after the space has refreshed which windows overlap which output. A
  frame with no damage is not presented, and clients asking for frame
  callbacks get them at the estimated vblank, so a still desktop
  presents nothing. Render elements are collected through a push closure.
- **The worker reaches the loop by jobs.** A job is a closure run on the
  loop with the state; a request waits for its answer for one second,
  then fails, so a stuck loop never hangs a client. Changes reach the
  worker as events, which it announces as D-Bus signals. Services on the
  worker run under a supervisor that restarts them with a backoff.
- **Outputs keep their windows.** A window on an output that goes moves
  whole onto the nearest one and remembers its place by the output's
  make, model and serial; it returns when that output does. With no
  output lit the session continues and the connectors are probed every
  three seconds.
- **The supervisor and the compositor.** `isle-session` starts the
  compositor beside it (or `ISLE_COMPOSITOR` in development) with a pipe
  on fd 3, on which the compositor reports `WAYLAND_DISPLAY`; the
  supervisor then exports the environment and starts
  `isle-session.target`, which binds `graphical-session.target`. After
  three quick deaths it starts the compositor with `ISLE_SAFE_START`,
  which leaves `settings.kdl` unread until `isle settings use-mine`. On a
  clean exit it unsets what it exported only while the manager still
  names its display (B101).
- **Tests drive the real thing.** The `test-hooks` feature, never in a
  package, adds the Test interface: input injected through the same
  handlers as devices, outputs added and removed, frames counted,
  screenshots, forced stalls and panics. `cargo test --workspace
  --features isle-compositor/test-hooks` runs the compositor headless on a
  private bus with a small Wayland test client. The headless backend draws
  on a GPU's render node when there is one and opens GBM there, as the
  hardware backend does, so dmabufs are tested too.

## How the shell runs

- **Surfaces are definitions the compositor hosts.** `isle-shell` keeps a
  `SurfaceDef` per kind of surface (B37) and an instance per output or on
  the main screen. Each instance is a Freya tree driven through
  `freya-embed`, the fork's crate for hosts that own their loop, GL context
  and input. One Skia context on the compositor's EGL context and one font
  collection serve every surface; a surface closing frees Skia's scratch
  layers, never its glyphs, masks or programs, whose compiled binaries the
  driver keeps on disk across sessions. Before the session's apps start, a
  surface holding one of each thing the shell draws is drawn offscreen
  once and its glass blurred, so the programs are built before any
  surface opens; each surface owns a texture and an FBO
  with a stencil buffer, drawn before the output's frame and placed as a
  render element: shell overlays over layer-shell's overlays, the top
  layer over windows and layer-shell's top, the background under all.
- **Waking, not ticking.** A tree's waker pings a calloop source; the loop
  then runs that tree's update and queues a redraw of its output if it
  changed. Freya's animations advance on the output's vblank, so they stop
  when it does. A closed surface has no content mounted and no texture.
- **Input through the seat.** The seat's focus is a client surface or a
  shell surface, so pointer, wheel and keys reach surfaces through the
  same handlers as windows; Freya's accessibility tree carries keyboard
  focus and Tab, and each surface's tree is on AT-SPI through an
  accesskit adapter on its own D-Bus thread; its nodes are built only
  while a screen reader listens, focus always. A surface that takes keys
  takes the keyboard on open and gives it back on close before its
  actions (close, a key) run.
- **Transitions and glass are drawn by the compositor.** Open and close
  are springs from the motion tokens on the texture, clipped to the
  surface's corner radius (B7). Behind each glass shape the compositor
  draws the desktop again into a texture through its own damage tracker,
  blurs it with dual Kawase passes, and draws it tinted under the surface;
  the tint rises to hold `text-2` at 4.5:1 (B39).
- **Panic boundaries.** Every call into a tree is caught; the compositor's
  panic hook knows a surface's panic from the core's by a thread-local,
  logs it and writes no crash record. The tree is rebuilt from its
  definition with the same id; the sixth panic in a minute turns the
  surface off.
- **Tokens as context.** The worker watches `~/.config/isle/tokens.kdl`;
  the loop lays it over the built-in shipped tokens, resolves a theme for
  the `appearance` settings and hands it to every tree through `isle-ui`'s
  `ThemeContext` (B38).
- **Tests and the preview.** `Shell1` lists, opens, closes, restarts and
  dumps surfaces, and reports tokens and Skia's cache; `Test1` arms the
  probe surface's panics and measures draws. `isle-preview <surface>` runs a
  surface in its own window with the same contexts and live tokens;
  `--watch` rebuilds and restarts it on a saved change.

## How windows are kept

- **A registry, and the `Space` as its view.** `wm` holds a record for
  every toplevel, Wayland or X11, from its creation to its destruction: id,
  app, PID, parent, desktop, mode (floating, tiled, filled, full screen)
  and the minimised and hidden flags (B40). Smithay's `Space` holds only
  the windows shown now, in the registry's stacking order; a window that
  leaves it is sent `suspended` and gets no frame callbacks. Everything the
  switcher, Mission Control and the CLI read comes from the registry.
- **Frames.** A window's frame is its geometry and, when it wears one,
  Isle's bar. Placement, places, zones, snapping and shadows work on
  frames; the client is configured with the frame less the bar. Geometry
  that does not touch Smithay (zones, regions, snapping, cascade) is plain
  functions in `wm::geometry`, unit-tested at 3440×1440 and 1920×1080.
- **Apps.** `isle-apps` reads desktop entries on the worker and hands the
  loop an index; a window's app id or X11 class resolves to an entry or
  stays its own app. The close path, quit and hide act on apps.
- **Desktops.** Each output has a row by its identity; a full-screen window
  gets a desktop right of its origin. Windows on an output that goes join
  the current desktop of the one they move to and return to their own.
- **Drawing and motion.** Each window's surfaces are drawn through a texture
  program that clips them to the frame's corners, over a shadow drawn by a
  pixel shader per token layer. Motion is drawn by the compositor on the
  window's elements; what outlives a window's buffers or its place (a close,
  a hide, a minimise, a tile's held content, the desktop slide) is drawn
  from a snapshot taken into a texture as it begins.
- **Bars and menus.** A decorated window's bar is an attached shell surface
  (B41), a Freya tree per window drawn in the window's place in the stack;
  the zoom menu and the pointer panel are placed surfaces opened at a box.
- **Xwayland.** Smithay's window manager runs Isle's X11 policy (B42); the
  X server starts with the compositor and again if it dies, on the same
  display.

## How keys are read

- **From device to client.** A key arrives with its keyboard. The keyboard's
  profile (Mac or PC, guessed from its name or set in `keyboard-device`)
  gives its keymap, its layouts with XKB's `mac` variant on a Mac keyboard,
  and on a PC keyboard trades left Alt and left Super so Cmd sits beside
  the space bar. The seat has one keymap, the last typing keyboard's,
  switched only while no key is held.
- **The engine.** `isle-keys` takes the physical keys and decides what the
  focused client is sent (B44): Cmd and Option are withheld until the next
  key or click decides them, a desktop chord from the shortcut set runs its
  action, a translation rule or the general rule (Cmd as Ctrl, Option as a
  Mac keyboard's third level or as Alt) is sent with exactly its modifiers,
  which are put back as the held keys imply when it ends. The seat is fed
  only these keys, so the modifiers a client sees always agree with the
  keys it was sent, Xwayland included.
- **Context.** Translation, withholding and the swap are off for a game
  (B46), for a window holding the keys (keyboard-shortcuts-inhibit or an
  Xwayland grab, until Cmd+Escape) and for an untranslated app; over a game
  only the escape set acts, over a window holding the keys only Cmd+Escape.
- **Focus.** Every change of keyboard focus goes through one function that
  releases what the old focus holds and sends the new one the modifiers
  still meant, so no modifier sticks across Cmd+Tab, Cmd+W or a dialog.
- **The shortcut set and the tables.** The set is the `shortcuts` section,
  a field per action, refused whole when a chord is bound twice. The
  translation tables are a shipped data file with the user's over it
  (B45). Isle's own keys (paste, emoji) are aimed at a window by id and
  sent as that app's own chord or typed through a keymap made for them.

## How the top of the screen works

- **The wallpaper is a render element.** Each output draws its background
  first: Frost as a shader from the tokens, a colour, or a picture decoded
  and fitted on the worker and uploaded once, crossfaded on a change. It is
  no shell surface, and nothing is drawn under a full-screen window.
- **The island's model is one queue** (B47). `isle-shell`'s `island::model`
  holds glyph states, privacy marks, activities and events and answers what
  each output's island shows and when that next changes, from a clock its
  caller gives; the loop arms one timer for that change. Sources post to it
  on the loop (`island_post`, `Island1`, `Test1.Island`); the clock wakes it
  once a minute.
- **The island's shape is the compositor's** (B48). `island` is an overlay
  surface on every output. The compositor springs its width and height,
  draws its fill, shadow and glass, and hands the tree the morph's progress,
  by which the tree fades the outgoing form and brings in the incoming one;
  the tree draws content only. Control Centre is the island grown into a
  panel on one output; the bar and a pinned panel never show together (B49).
- **The top edge is decided in one place.** `top_edge` owns the island's zone
  (its shape, the strip above it and a guard each side), the hover dwell and
  grace, the push that reveals the bar outside the zone or the island over a
  full-screen window, and the dwell that replaces the push over a game.
- **The bar is one surface per output, opened on one.** `menu-bar` covers its
  output while revealed so the band, its open menus and Help's search are
  laid out together; its hit shapes say where it takes the pointer, and a
  press outside an open menu closes it and goes no further. The compositor
  builds the titles from the front app's tree and Isle's own parts, keeps a
  command per item id, and runs the chosen one.
- **Menus are one tree from four sources** (B50). `isle-menus` reads DBusMenu
  and GMenuModel into one tree on the worker and owns the registrar before
  the socket is announced. A window's menu is found by its surface's own
  address (`org_kde_kwin_appmenu`, `gtk_shell1`), then for X11 the
  registrar by window id and the window's properties, then a registration
  from the window's process, else the fallback from the desktop entry and
  the translation tables' labels. The lookup runs as a window takes focus
  and again when a registration arrives; a dialog with none shows its
  parent's. A source is watched for changes, an empty one too, until the
  app fills it. Items are found again by label path; hints are the Mac
  chord P5's tables map the app's shortcut back to.
- **The main screen follows focus** (B51): the focused window's output, else
  the pointer's, unless `shell.main-screen` names one. Events and Isle's
  About panel show there; every output keeps its clock and glyphs.
- **Launching** (B52). `launch` starts an entry, an entry action or a link in
  a transient systemd scope with an activation token, so the new window
  takes focus; it falls back to a direct spawn without systemd.

## How switching works

- **The switcher never takes the seat's focus** (B53). Cmd+Tab opens
  `isle-shell`'s `switcher::model` over the apps as they are; while Cmd is
  held, `key_from_device` reads Tab, the arrows, `, Q, H, W, Return and
  Escape ahead of the engine and eats every other key, so the window in front
  keeps the keyboard, its game state and its pointer lock. Letting Cmd go
  commits once, through the paths every other activation takes; Shift alone
  does not end the hold; a console switch or the typing keyboard's removal
  cancels it. Nothing moves before the commit, and a release before
  `timing-switcher-delay-ms` commits with nothing drawn.
- **The model is pure.** Entries, the selection that follows its app as apps
  come and go, the window row, keys to effects, and the layout (shrinking to
  the floor, rows of equal count, the background group) are unit-tested with
  a fake clock; the loop arms one timer for the delay.
- **Previews are the compositor's drawing** (B54). The panel is a placed
  overlay surface on the focused window's output, a row of app icons (B59).
  For each card of the window row the compositor draws the window's pieces
  at card size into a texture and Skia
  copies it on the GPU into an image it owns, which the tree draws among its
  own layers. A window that commits while held is drawn again, a few at a
  time, ten times a second; a parked window shows its last buffer and is not
  woken. Everything goes when the surface closes.
- **Apps are known by their processes too** (B55). Each pid a window has had
  is watched by a pidfd, and each app Isle launched by its scope's cgroup; an
  app that has had a window, has none now and still runs is a background
  app, listed last and stopped by Q.
- **Badges, progress and urgency** come from LauncherEntry's Update signals,
  kept per app and dropped when the sender leaves the bus. Attention is
  urgent from the X11 urgency hint or LauncherEntry, normal from a stale
  activation, `_NET_ACTIVE_WINDOW`, demands-attention or `gtk_surface1`.
- **Games are named from their launcher's library** (B56): `isle-games` reads
  Steam's, Heroic's and Lutris's libraries from their native and Flatpak
  folders on the worker; a gamescope window Steam started is the game it
  hosts.
- **Centre surfaces close the menu-bar surfaces** (B57).

## How services reach the shell

- **A service is a crate that publishes snapshots** (B11, B60). Each of
  `isle-power`, `isle-audio`, `isle-brightness`, `isle-network`,
  `isle-bluetooth`, `isle-media`, `isle-disks` and `isle-tray` has a
  `watch` that reads its daemon whole into one state value and publishes it
  through `isle-bus`'s `Publisher` as a `Snapshot { available, state }`;
  after any signal under the daemon's paths it reads again, a burst read
  once. Commands are plain async functions on the crate. The same crates
  serve Isle's apps, which talk to the daemons themselves.
- **On the worker, under the supervisor.** `services::start` runs each
  watch under `worker::supervise` and carries its snapshots to the loop at
  most once a frame; `State::*_changed` takes each in, and the island's
  glyphs and privacy marks, Control Centre's modules and pages and the OSDs
  read the last one. A daemon that goes makes its snapshot unavailable; its
  name coming back starts the read again.
- **Commands go out and their results come back on the loop.**
  `Isle::run_service` runs a command on the worker and calls back on the
  loop, where a failure is shown in the row it came from until the next
  action there.
- **Work that only a page needs runs while it shows.** The input level
  meter, the Wi-Fi scan, Bluetooth discovery and the media clock start when
  their page opens and stop when it closes (`follow_pages`).
- **OSDs come from changes, not from keys** (B62). A level's change from
  anywhere raises one OSD; a change Control Centre made itself is marked
  for a moment beforehand and raises none.
- **Sound, brightness and the tray have Isle's own clients.** Audio speaks
  the Pulse protocol to pipewire-pulse in pure Rust on the worker, and
  reads the camera's users from PipeWire's graph (B11). Brightness is
  DDC/CI written by Isle over `i2c-dev`, matched to outputs by EDID, and
  the backlight through logind (B65). The tray's watcher is owned before the
  socket is announced, and items' menus go through `isle-menus`' DBusMenu
  client as panel pages (B61).

## How notifications are decided and shown

- **Isle is the notification server** (B63). `isle-notifications` owns
  `org.freedesktop.Notifications` before the socket is announced and hands
  each arrival to the loop. There `decide` gives it one fate against the
  holds in force (a Focus with its allowed apps, a game or full-screen
  window in front, the user away, the screen being shared) and the app's
  settings: a banner, held for the summary, quiet into Notification Centre,
  or a transient one shown or dropped. Records live in the `notifications`
  state domain; ids are never reused across restarts, and the desktop's own
  notifications take ids from `0x8000_0000`.
- **Banners are a pure model and one placed surface** (B63). `isle-shell`'s
  `banners::model` keeps one card per app with its count, three at most,
  their lives from the tokens' timings, paused under the pointer; the
  `banners` surface draws it at the top right of the main screen.
  Notification Centre is the island's second panel, beside Control Centre.

## How Focus decides

- **One pure function** (B64). `isle-focus` evaluates, from the settings,
  the memory of what the user turned on or off, the app in front and the
  time, which Focus is in force: Game Mode while a detected game is in
  front unless turned off for it, then one turned on by hand until it is
  turned off or ends, then the latest automatic start. The loop evaluates
  on each change of those and arms one timer for the next schedule edge.
- **Game Mode is a Focus.** While it holds, banners wait, the performance
  profile is held through power-profiles-daemon, and the shell draws solid
  and still. When the last silencer ends, what waited is summarised in one
  banner.

## How the lock holds

- **The lock is a state of the core** (B66). `State::lock` writes the
  marker `$XDG_RUNTIME_DIR/isle/locked` first, takes input back (centre
  surfaces, menus, grabs, focus) and from then every output's frame comes
  from `render_locked`: the softened wallpaper (made on the worker at a
  quarter of the size), the lock's wash, the lock layer's surfaces, the
  island and the dim, over an opaque floor. No client is drawn or takes
  input; the focus guard refuses any target but the lock's surfaces. A
  compositor that starts with the marker present starts locked.
- **The password is the compositor's.** Keys go to `lock_key`, which
  holds the typed text in a zeroizing buffer; the face draws its length.
  Return sends it to the `isle-auth` helper (B67), whose `ok`, `fail`,
  `info`, `error` and `broken` lines come back to the loop.
- **The face is a surface that may fail.** `lock_face` shows the face on
  the screen with the pointer and keeps the rest drawn by the locked path;
  its second panic within a minute replaces it with the minimal lock for
  the rest of the lock (B68).
- **logind is told and listened to.** The locked hint follows the lock;
  `Lock` and `Unlock` on the session lock and unlock it; the sleep delay
  inhibitor is given back once every lit screen has presented a locked
  frame, and drawing stops until resume; the power key's block inhibitor
  makes the key Isle's.

## How idle decides

- **One policy, one timer** (B69). `idle.rs`'s pure `due` gives the next
  step from the settings (dim, lock, display off) and the time since the
  last input; `passed_over` keeps a step the user has already been through
  from firing again. The loop arms one timer for the next step.
- **Holds stop the clock**: an idle-inhibit on a shown surface, a
  ScreenSaver cookie (served at both of its paths), logind's `idle` block
  and a game in front. While held nothing steps; the count starts again
  when the last hold ends.
- **The dim is the compositor's**, a solid element over everything; display
  off goes through `DrmCompositor::clear`, the first frame after waking is
  composited, and a connector that flaps within ten seconds of going dark
  is not taken as unplugged. ext-idle-notify tells clients the same idle.

## How prompts are asked

- **The agents live on the worker** (B12): the polkit agent, the Gcr
  system prompter for the keyring, NetworkManager's secret agent (VPN
  plugins in external-UI mode included) and BlueZ's agent as the default.
  Each turns its daemon's question into an `Ask` through a `PromptClient`
  and waits for an `Answer`; none keeps a secret past its reply (B71).
- **One queue on the loop** (B70). `prompts::queue` decides which prompt
  shows: the user's own requests first, never ahead of the one on screen;
  a prompt waits behind the lock, behind a full-screen window or game
  unless that app asked, and once Cmd+Tab set it aside, as the island's
  activity with Show and Cancel. The asking app is the nearest process
  with a window above the asking pid.
- **The scrim is the compositor's.** While a prompt or the session dialog
  shows, every screen gets the scrim above windows and every surface but
  the island; the frame is a `Prompt` layer surface that takes the keyboard,
  and no client may take focus. Return and clicks answer only once armed,
  after `timing-prompt-arm-ms`.

## How the session ends

- **One engine** (B72). `end_session` asks every app to quit as Cmd+Q
  does, background ones signalled, then checks every quarter second: all
  gone, it logs out (the loop stops, which ends the session) or calls
  logind's `Reboot` or `PowerOff`; an app that missed its ping stops it as
  not responding, one still open after the wait stops it as asking to save,
  and another program's block inhibitor on shutdown stops it named.
- **It serves the chooser, the confirmation, log out with no confirmation
  and the lock's power row.** The session dialog is a `Prompt` layer
  surface over the scrim, in the centre group; on the lock the power row
  confirms in place and names no app.
- **Force Quit is a centre surface, the alert an attached one.** The panel
  lists the registry's running apps, pinging each window as it opens;
  `force_quit_app` sends `SIGKILL` to every process of the app. Close on a
  window that missed its ping hangs the not-responding alert on it, keyed
  apart from its bar.

## How Spotlight searches

- **The compositor owns Spotlight's model** (B73). `isle-shell`'s pure
  `Spotlight` holds the field, the scope, the groups and the selection; the
  compositor makes it at the press of Cmd+Space and gives it Spotlight's
  keys before the keyboard engine while Spotlight holds the keyboard, with
  text from the seat's layout, its own chords and repeat. A key typed with
  the chord reaches the model whether or not the surface has drawn. The
  surface draws a frame the compositor builds from the model; the pointer
  sends actions back.
- **Sources are `isle-search`'s, one trait** (B74). Each holds what it
  searches, handed over when Spotlight opens: apps and games, actions with
  their chords, windows, the front app's menu items from the menu bar's own
  titles, Settings rows, clipboard history, emoji, Run and the web. These
  answer from memory in the keystroke's frame on the loop. Files, contents
  and the calculator are slow: they run on the worker, checking the newest
  query's generation, and an answer to an older query is dropped. The model
  lets a slow answer land only below a selection the user moved.
- **Ranking is a tier, then use, then the name** (B75). A name that starts
  with the query, then a word start, initials, inside, then nucleo's fuzzy
  score; within a tier, use that halves every 30 days with a week's bonus
  (`usage`), a shell action ahead of an app, the match's score, the name.
  Help's menu search ranks the same way.
- **File names come from Isle's own index** (B76): a walk of home and
  `spotlight.files-include` with the `ignore` crate, without hidden folders,
  what `.gitignore` files exclude and `spotlight.files-exclude`, kept live by
  one inotify watch per folder and walked again on an overflow. Contents
  are searched on demand with ripgrep's engine, plain text only.
- **Games start through their launchers' entries** (B77): the launcher's
  desktop entry given the game's link, so a game has a token and a scope as
  an app does.

## How the clipboard is kept

- **History is the compositor's** (B78). A clipboard copy from a Wayland
  client, or through Xwayland from an X11 one, is read off the loop (text,
  an image, a file list) once Smithay has made it current, and kept under
  `~/.local/state/isle/clipboard/`. An offer marked
  `x-kde-passwordManagerHint`, or made while an excluded app is in front,
  is not kept. No data-control protocol is served.
- **Isle offers selections of its own.** A selection's owner is either
  Xwayland, read through Isle's X11 window manager, or Isle, whose bytes it
  writes itself; every selection the Wayland side sets is handed to the
  window manager so X11 clients read it.
- **Every pick is pasted** (B79). An emoji, a symbol or a clipboard item
  becomes Isle's selection and the app is sent its paste chord through P5's
  tables; for an emoji the clipboard as it was comes back once the pick has
  been read, or after a second.
- **A drag out of Spotlight is Isle's.** A file or clipboard row dragged
  past Spotlight's edge starts a `DnDGrab` whose source writes the row's
  types to the target.

## How a capture is rendered

- **Every capture is the compositor's own render** (B80). A still of a
  screen is what that output shows, through the same element list, less the
  idle dim, the pointer unless asked, and the capture tool's own surfaces
  left out by name (the readout, the toolbar, the loupe, the thumbnail, the
  share picker); a locked screen therefore gives the lock. A region is that
  render cut; a window is its own pieces at its output's scale, refused while
  locked. Streams and ext-image-copy-capture frames are the same renders;
  the share sheet, drawn with its window, is left out with them.
- **The freeze holds textures, not clients.** Shift+Cmd+4 renders each
  screen, each shown window and each open placed shell surface (Spotlight, a
  menu, a banner) into textures of their own and draws the frozen frames in
  place of the scene, with the veil, the cut, the line and the lifted piece
  as compositor elements; clients run on unaware. Rounded lines and handles
  are the window shadow's program at no blur. The pick reads keys and the
  pointer without taking focus; in window mode a camera surface stands in
  for the pointer.
- **Pixels leave the loop at once.** The readback is on the loop; PNG
  encoding, the file and the thumbnail's small copy are on the worker, and
  the allocator is trimmed after, so a still's tens of megabytes are not
  kept.

## How streams are produced

- **PipeWire runs inside the loop** (B81). Its loop's descriptor is a
  calloop source; the context has the realtime module and D-Bus support
  off, so nothing in the compositor loads libdbus or asks the Realtime
  portal. Each stream drives itself and offers dmabufs by the modifiers the
  renderer draws to, LINEAR first and one fixed by a test allocation, then
  shared memory.
- **Frames come with damage.** After each output's frame, each stream of it
  renders when its damage tracker has damage and the negotiated rate allows,
  and a dmabuf is queued once its fence signals. A window streams only while
  it is shown; a window closing or a screen going ends its streams.
- **The pointer is left out, drawn in, or sent beside.** In the metadata
  mode each buffer carries the pointer's place in the frame and, when its
  image changes, a bitmap the renderer draws from the named cursor or the
  client's cursor surface; the pointer moving over a still frame queues a
  buffer with no picture. A recording with Show mouse clicks draws a fading
  disc at each press into its own frames only.
- **Every stream shows.** Any stream sets the screen privacy mark and holds
  banners; a recording is the island's Recording activity, a portal share
  its Sharing row, each with Stop.
- **Recording is a helper** (B82): the compositor streams the source and
  `isle-record` reads it, with the output's monitor and the microphone, into
  a fragmented MP4 whose every frame is flushed.

## How the portal backend answers

- **Isle's own backend on the worker** (B83). `isle-portal` holds the impl
  interfaces as zbus, with the Request object each held reply exports and
  the Session objects; the compositor serves them on a connection of its
  own and takes `org.freedesktop.impl.portal.desktop.isle` before it reports
  ready, then has systemd try-restart the frontend so a running one reads
  Isle's versions. Only the owner of `org.freedesktop.portal.Desktop` may
  call it.
- **What each interface answers with.** ScreenCast: the restore data, the
  same app's second ask, or the share picker, then a stream per source
  (B84). The picker is a sheet on the window `parent_window` names, found
  through xdg-foreign v2 or the X11 window id, over that window dimmed, and
  holds only that window's keyboard; with no window found it is centred on
  the main screen. Its Portion of screen tab puts it away for the toolbar
  in a Close-and-Choose form. Screenshot: the toolbar for an interactive
  request, after the timer when one is set, the front window or every
  screen for one the frontend allowed. Access: a P9 prompt.
  Settings: the appearance namespace from the theme. Inhibit: idle's holds
  and the session's end.
- **The standard protocols beside it** (B85): ext-foreign-toplevel-list,
  ext-image-capture-source and ext-image-copy-capture into shared memory or
  a dmabuf on the renderer's device, the still's texture blitted into it,
  and pointer cursor sessions that follow the pointer with its image; no
  wlr-screencopy. A streaming session is listed in the Sharing row by its
  client's app, with Stop.
- **Asks made while locked wait.** ScreenCast, Screenshot and PickColor
  asks are held until the unlock unless the app gives up first.

## How a frame reaches the screen

- **Clients start after the GPU** (B92). On the console the render GPU
  opens only when the session is active; linux-dmabuf and syncobj are
  served then, and only then is Xwayland started and the supervisor told
  Isle is ready, so the session's apps start. An Xwayland started earlier
  binds no linux-dmabuf, runs without glamor and draws every X11 window in
  shared memory, which can never be scanned out. A dmabuf a client did not
  name a device for is taken as the render node's. A render GPU that fails
  to open or goes away ends Isle for the supervisor to start again; an
  output that goes is withdrawn from clients at once and destroyed later.
- **Smithay assigns the planes; Isle proves it** (B86). A full-screen
  window is drawn bare, so `DrmCompositor` can put its buffer on the
  primary plane; each surface gets dmabuf feedback with a scanout tranche
  of the primary plane's formats Isle can also render, so a game allocates
  what the plane takes, X11 windows Isle leaves alone included; feedback
  goes after the frame's primary scanout outputs are known. A bare window
  is a scanout candidate; a clipped one never, since a plane cannot clip.
  The pointer's image is resampled at a fractional scale to exactly the
  pixels it covers, so the cursor plane takes it. Overlay planes stay off
  on NVIDIA. Each output keeps a plane trace (`isle displays trace`): what
  the primary and cursor planes carried, whether the flip was async, VRR,
  the interval; a full-screen frame composited logs why.
- **A buffer is given back after the GPU has read it.** Under explicit
  sync a buffer's release is signalled when Isle drops it, so a composited
  frame holds the client buffers its elements sampled, popups and the
  pointer's surface included, until its vblank, or until the screen goes
  off or the session leaves; a scanned-out buffer is held by the plane
  until it is replaced.
- **Presentation is the display's.** A vblank's hardware time and sequence
  go to wp-presentation with the hardware flags, `Variable` refresh under
  VRR; layers and X11 popups get feedback too. linux-drm-syncobj where the
  device waits on sync points, wp-fifo, wp-commit-timing and
  wp-content-type are served; a commit's buffer is held until readable, a
  fifo barrier passed once its frame is drawn, a timed commit applied for
  the frame that covers it or at its deadline. A frame visits only the
  surfaces with a barrier or a timed commit outstanding.
- **VRR is decided per frame** (B87) from the output's setting, its
  connector (on demand, only at a modeset, or not at all) and whether it
  shows a full-screen desktop, focused or not. Under VRR a frame that
  would move only the pointer waits up to the panel's slowest refresh for
  the game's next frame, but only while the game is drawing; an idle
  game's pointer moves at once. A refused change is not asked for again
  until the wanted state changes.
- **Tearing only for a full-screen game that asks, where allowed** (B88):
  such a frame is rendered async with `ASYNC_SCANOUT_ONLY`, so Smithay
  flips it without vsync only when nothing but the primary plane changed
  and that plane holds the game's buffer. With the pointer shown the frame
  goes with vsync, as an async flip leaves the cursor plane alone.
- **Input goes before the frame.** The loop flushes clients before it
  draws as well as after, so a key taken in a frame's iteration does not
  wait for the frame. A frame asked for by the pointer alone, with no grab
  or drag, skips gathering what lies behind the shell's glass unless a
  shell surface drew.
- **Glass keeps its blur.** A glass shape that moves keeps its blur's
  textures, and a new one takes those of a shape of its size gone this
  frame, as a menu remounted for the next title does; the blur's luminance
  is read back a frame after it is copied, so the loop never waits on it.
- **Windows out of view are paced** (B89): a hidden game's frame
  callbacks and fifo barriers every 33 ms, any other hidden window's once
  a second.

## How night light is applied

- **On the CRTC's gamma ramp, never the frame** (B91). Each lit screen's
  `GAMMA_LUT`, else its legacy ramp, is written outside `DrmCompositor`'s
  commits, which never touch it, so scanout stays on, the pointer is warmed
  with the rest, and captures, which never pass the ramp, are untinted. The
  ramp found is put back when the session leaves the console or ends and
  given again on return; a screen with neither is left as it is.
- **The warmth is a blackbody white point** in linear light, 6500 K being
  none. The schedule (Sunset to Sunrise from `isle-sun`'s reading of the
  time zone, Custom, or Off), Control Centre's tap until the schedule's
  next change, and Game Mode's hold decide it; changes fade, but Game Mode
  turns it off at once. Appearance Auto follows the same sun.

## How an app is built

- **On `isle-app`** (B94). An app gives a `Spec` (its reverse-DNS id, which
  is also its desktop file id, Wayland app id and bus name, its window's
  size, the lights' inset, its backdrop) and its root component;
  `isle_app::run` does the rest. A launch first claims the bus name with
  `org.freedesktop.Application`: when another instance holds it, the
  launch's URIs and activation token are handed over and the process ends.
- **The window is `freya-winit`'s,** in software by default, into
  softbuffer's shm buffers, premultiplied ARGB for a window with a backdrop
  and opaque otherwise, and on the GPU only in a `gpu` build with
  `FREYA_RENDERER` (B26). Freya draws the whole window each frame and
  presents it whole. What winit does not bind is bound on a second event queue against
  winit's own display and the window's surface: `xdg_activation_v1` to
  raise the window, `org_kde_kwin_appmenu` to name its exported menu,
  `gtk_shell1` to hand the toolbar's double, right and middle clicks to the
  compositor, `isle_titlebar_v1` to ask for the lights over the content at
  the app's inset (B95), and `ext_background_effect_v1` for the blur
  behind the backdrop, the rest of the window declared opaque (B98).
- **The desktop is read from the compositor and followed:** the theme from
  `Shell1.Theme()` and `ThemeChanged`, every setting from `Settings1.Values()`
  and `Changed`, and whether `settings.kdl` and `tokens.kdl` are in force.
  An app writes through `Settings1` (Set, Reset, Remove); the value reads as
  set at once and a refusal is kept against its key for the row to show.
  One change a key is in flight at a time, a later one waiting in its place,
  and while a key's change is on its way the compositor's echo does not
  overwrite it, so a dragged slider sends only as fast as it is answered; a
  burst of change signals is read once.
- **One command table** declares every command with its menu, group and
  chord; the menu bar is exported from it as DBusMenu at `/MenuBar/1`, and
  its chords are matched in the window, Cmd as `Modifiers::META`. Isle's
  own apps are never Cmd-translated, so Super arrives as itself.
- **D-Bus and the daemons run on the app's worker,** a tokio runtime on a
  thread of its own; the UI thread awaits its results and never blocks.
- Menus, pop-ups, popovers and sheets draw inside the window (B94).

## How Settings writes

- **A pane is a component over two sources:** the desktop's values, for
  rows bound to `settings.kdl` keys, and the daemons' state, which the
  panes follow only once one asks (NetworkManager, BlueZ, PipeWire and
  power-profiles-daemon through the service crates; displays, keyboards,
  pointers, Control Centre's modules, Focus and the tray from the
  compositor; the portal's permissions; the app index). Their copies into
  the UI live as long as the app. A change to a daemon is an act carried
  out on the worker (B97).
- **A display change is the compositor's** (B96). Settings sends
  `Displays1.Configure` and shows "Keep these display settings?" counting
  down; the compositor applies the change at once, holds what it replaced,
  writes the change only on `Keep`, and puts it back at its own deadline,
  on `Revert`, or when the asking client leaves the bus. Scale and place
  write at once.
- **The panes are the record of rows:** each row's id, label, keywords,
  the key it shows and the sheet or sub-page it is on. Search, the
  Spotlight index (`data/settings/index.kdl`, generated by
  `isle-settings --write-index`) and Show Changed Settings read it; a link
  `isle-settings://<pane>/<row>` opens the row's sheet, rings the row and
  scrolls to it.
- **Show Changed Settings** compares the values with the schema's defaults
  in the app; Reset to Default is `Settings1.Reset` of the row's key.

## How Isle is installed

- **One package** (B100). `pkg/arch/PKGBUILD` builds `isle-git` from
  GitHub, or from the checkout's HEAD through `tools/package.sh`, which
  keeps its target directory in `~/.cache/isle/pkg` and installs with
  `pacman -U`. Two cargo builds: the compositor, supervisor, CLI and
  helpers together, Settings alone. The compositor's Skia, with GL, is
  built from the crate's sources with the system's `gn` and `ninja`;
  Settings' is the prebuilt archive in `source=()`. A test crate,
  `isle-package`, checks that the PKGBUILD and the tree agree and that no
  shipped code reads the build tree.
- **What it installs.** The six binaries; the session entry; the user
  target; the portal backend file and `isle-portals.conf`; Settings'
  desktop entry and D-Bus service; `/usr/lib/pam.d/isle`; the NVIDIA
  profile in `/etc` (the driver reads no `/usr/share` directory); an i2c
  `uaccess` rule and `i2c-dev` for DDC/CI.
- **Beside v1** (B101). tuigreet chooses the session. Each desktop's
  portals file chooses from the one backend file; xremap carries a
  condition on `XDG_CURRENT_DESKTOP`; the lock marker is named for the
  logind session; the logs are apart.
- **Login Items** (B102). `isle_apps::autostart` reads the entries; the
  compositor waits on the worker until the user manager names its display
  and `graphical-session.target` is active, then launches the items not
  still running. isle-tty sets `ISLE_LOGIN_ITEMS=0`.
- **`isle setup`** (B103). Steps with check, apply and undo and their
  record; the root half is `isle setup --as-root`, run through sudo or
  pkexec.

## Rules the layout enforces

- Anything new the shell draws, a material, a shadow token, an effect or a
  component in `isle-ui`, goes into the warm-up scene
  (`isle-shell/src/warm.rs`) in the change that adds it; what the scene
  leaves out compiles its programs on its first open, a dropped frame.
  `bench first_menu_draw` shows the first draws.
- Nothing that can block runs on the main thread. Services, D-Bus, file
  and icon work and search run elsewhere and reach the loop through
  calloop channels; PAM runs in the `isle-auth` process (B9, B67).
- A shell surface's event handling and drawing run behind a panic
  boundary; a panicking surface is rebuilt, a panicking service restarted.
  A panic in the compositor core ends the session and the supervisor
  starts a new one (B9).
- The compositor is the one writer of settings. Apps, the shell and the
  CLI change settings through its D-Bus interface, each app's preferences
  as its own section of `settings.kdl`; hand edits are picked up by a
  watcher, and while the file does not parse every write is refused
  (B10, B27).
- Apps read system state through the shared service crates and desktop
  state (settings, window places, the current mode) through the
  compositor's D-Bus interface; there is no separate desktop daemon (B11).
- One GPU context: the shell draws with Skia on the compositor's own EGL
  context, so it adds no per-process driver cost (B7; measured in
  `docs/research/rust-desktop/shell-ui-spike.md`).
- Open and close transitions of shell surfaces are animated by the
  compositor (B7).

## Data on disk

| Store | Where | What | Written by | Watched |
|---|---|---|---|---|
| `settings.kdl` | `~/.config/isle/` | the user's choices, apps' preferences included | the compositor, into the file's own text (B36), or the user by hand | yes |
| `tokens.kdl` | built into Isle; `~/.config/isle/` over it | design tokens | the user by hand; Isle never writes it | the user's file |
| `translation.kdl` | built into Isle; `~/.config/isle/` over it | Cmd translation tables (B45) | the user by hand; Isle never writes it | the user's file |
| state | `~/.local/state/isle/<domain>.json` | window places (`places`), desktops per output (`desktops`), the capture toolbar's mode, place and portion (`capture`), notifications and their records (`notifications`), what the user turned on or off in Focus (`focus`), Spotlight's use (`usage`) and Run's history (`commands`), Settings' pane last shown (`settings-app`), session restore, crash history | the compositor and the supervisor | no |
| clipboard history | `~/.local/state/isle/clipboard/` | `index.json` and each copy's types as files, mode 0600 | the worker | no |
| caches | `~/.cache/isle/` | media art (`art`), notifications' images (`notifications`), tray items' pixmaps (`tray`), shots for apps through the portal (`portal`, a day) | the worker | no |
| shots and recordings | `$XDG_PICTURES_DIR/Screenshots`, `$XDG_VIDEOS_DIR/Recordings` | by macOS's names | the worker, `isle-record` | no |
| crash records | `~/.local/state/isle/crashes/` | the last ten | the compositor's panic hook and watchdog, then the supervisor | no |
| session log | `~/.local/state/isle/logs/session.log`, `.old`; the user journal as `isle-compositor` | this session's and the last | the supervisor | no |
| setup record | `/var/lib/isle/setup.json`, `~/.local/state/isle/setup.json` | what `isle setup` changed, for `--undo` (B103) | `isle setup` | no |

v1's preferences, window places and launch counts are imported once, on
the first start that finds them (B27).

## Delivery

A PKGBUILD in this repository builds Isle from git; it is installed with
paru and updated through pacman (B29). How it is built and installed is
How Isle is installed.

## Dependencies held as forks

- **Smithay**, through `alexparlett/smithay` (B6), pinned by revision on
  its `isle` branch: upstream master at `118e34f` with Isle's patches, each
  merged by pull request in the fork, none offered upstream (B43). So far
  three: the X11 window manager tracks windows' bounding shape (SHAPE) and
  reads Steam's `STEAM_GAME` label (alexparlett/smithay#2), and an
  xdg-foreign export is looked up by its handle (alexparlett/smithay#3), which
  the share picker uses to hang on the asking window; DRM async page flips,
  Smithay #1948 rebased (alexparlett/smithay#4), and wp-tearing-control
  (alexparlett/smithay#5); and `FrameFlags::ASYNC_SCANOUT_ONLY`, which keeps
  a composited frame on vsync, pinned by its branch's commit until its pull
  request is merged. Its `master` follows upstream; moving Isle to a
  newer upstream is a rebase of `isle` and a revision bump.
- **Freya**, through `alexparlett/freya` (B7), pinned by revision of its
  `main`, which carries `freya-embed` (the embedding crate made from the
  spike's glue), an idle `Timeout` that sleeps, and thread-local contexts
  restored on unwind (alexparlett/freya#10), and accessibility nodes built
  only while a screen reader listens with the software driver's alpha, on
  softbuffer's unreleased `AlphaMode` pinned by revision
  (alexparlett/freya#11, B98). A git-ignored
  `.cargo/config.toml` may patch it to a local checkout while working on
  the fork.

## Repository

Built on `feature/rust-desktop` in this repository (B16), beside Isle v1,
which stays the daily desktop on `main` until the new one replaces it
(roadmap milestone M2) and is removed at cut-over. The Cargo workspace
is at the root: `crates/` holds `isle-compositor`, `isle-session`, `isle-cli`
(the `isle` command), `isle-ipc`, `isle-settings` with its derive,
`isle-state`, `isle-bus` (what the service crates share: the buses, a daemon's name
followed, snapshots and listeners), the service crates `isle-power`, `isle-audio`,
`isle-brightness`, `isle-network`, `isle-bluetooth`, `isle-media`, `isle-disks` and
`isle-tray`, `isle-notifications` (the notification server), `isle-focus` (which Focus
is in force), `isle-fakes` (fake system daemons for the tests), `isle-apps` (desktop entries, app resolution, icons and LauncherEntry's
badges), `isle-games` (launchers' libraries: names, covers and launch links, and
which processes descend from them), `isle-sun` (sunrise and sunset from the
time zone's coordinates), `isle-search` (Spotlight's
sources, the matcher, ranking and the file index), `isle-keys` (keyboard
profiles, chords, the shortcut set's table, translation tables and the engine),
`isle-menus` (the menu tree, its DBusMenu and GMenuModel clients and the registrar),
`isle-portal` (the portal backend's interfaces), `isle-record` (the recorder),
`isle-shell` (the surface host), `isle-tokens`, `isle-ui` (the
Freya side shared with Isle's apps) and `isle-preview`; `data/` holds the shipped tokens with their schema and
contrast check, the translation tables, the Phosphor subset with Isle's own glyphs,
Spotlight's actions, the portal's description and preferences, the emoji table with its generator and the Settings row index, and
the vendored `gtk-shell.xml`. `spikes/shell-ui/`
is the throwaway code behind B7 and is not built on.
