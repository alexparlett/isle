# Architecture

How Isle is put together. The compositor core (P2), the shell runtime
(P3) and windows (P4) are built; the rest is the planned architecture of the rust-desktop program, each part settled in
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
      ├── PAM thread              lock-screen conversations, which can block (B12)
      └── watchdog thread         ends a dispatch stuck for ten seconds; the loop
                                  itself logs any over a frame's budget (B9, B35)

isle                            the command line over the D-Bus API
Apps, launched on demand        Settings, Files, Monitor, Keychain, Cleaner:
                                Freya processes on the same component library,
                                drawn on the GPU (B7, B26); Files also serves the Open and Save panels and Quick
                                Look (B26)
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
  which leaves `settings.kdl` unread until `isle settings use-mine`.
- **Tests drive the real thing.** The `test-hooks` feature, never in a
  package, adds the Test interface: input injected through the same
  handlers as devices, outputs added and removed, frames counted,
  screenshots, forced stalls and panics. `cargo test --workspace
  --features isle-compositor/test-hooks` runs the compositor headless on a
  private bus with a small Wayland test client.

## How the shell runs

- **Surfaces are definitions the compositor hosts.** `isle-shell` keeps a
  `SurfaceDef` per kind of surface (B37) and an instance per output or on
  the main screen. Each instance is a Freya tree driven through
  `freya-embed`, the fork's crate for hosts that own their loop, GL context
  and input. One Skia context on the compositor's EGL context and one font
  collection serve every surface; each surface owns a texture and an FBO
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
  accesskit adapter on its own D-Bus thread. A surface that takes keys
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
  Steam's library folders, manifests and covers on the worker.
- **Centre surfaces close the menu-bar surfaces** (B57).

## Rules the layout enforces

- Nothing that can block runs on the main thread. Services, D-Bus, file
  and icon work, search and PAM run elsewhere and reach the loop through
  calloop channels (B9).
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
| state | `~/.local/state/isle/<domain>.json` | window places (`places`), desktops per output (`desktops`), launch counts, session restore, crash history | the compositor and the supervisor | no |
| crash records | `~/.local/state/isle/crashes/` | the last ten | the compositor's panic hook and watchdog, then the supervisor | no |
| session log | `~/.local/state/isle/session.log`, `.old` | this session's and the last | the supervisor | no |

v1's preferences, window places and launch counts are imported once, on
the first start that finds them (B27).

## Delivery

A PKGBUILD in this repository builds Isle from git; it is installed with
paru and updated through pacman (B29).

## Dependencies held as forks

- **Smithay**, through `alexparlett/smithay` (B6), pinned by revision on
  its `isle` branch: upstream master at `118e34f` with Isle's patches, each
  merged by pull request in the fork, none offered upstream (B43). So far two:
  the X11 window manager tracks windows' bounding shape (SHAPE) and reads
  Steam's `STEAM_GAME` label (alexparlett/smithay#2). The async
  page flip joins it by P12. Its `master` follows upstream; moving Isle to a
  newer upstream is a rebase of `isle` and a revision bump.
- **Freya**, through `alexparlett/freya` (B7), pinned by revision of its
  `main`, which carries `freya-embed` (the embedding crate made from the
  spike's glue), an idle `Timeout` that sleeps, and thread-local contexts
  restored on unwind (alexparlett/freya#10). A git-ignored
  `.cargo/config.toml` may patch it to a local checkout while working on
  the fork.

## Repository

Built on `feature/rust-desktop` in this repository (B16), beside Isle v1,
which stays the daily desktop on `main` until the new one replaces it
(roadmap milestone M2) and is removed at cut-over. The Cargo workspace
is at the root: `crates/` holds `isle-compositor`, `isle-session`, `isle-cli`
(the `isle` command), `isle-ipc`, `isle-settings` with its derive,
`isle-state`, `isle-apps` (desktop entries, app resolution, icons and LauncherEntry's
badges), `isle-games` (launchers' libraries: names and covers), `isle-keys` (keyboard
profiles, chords, the shortcut set's table, translation tables and the engine),
`isle-menus` (the menu tree, its DBusMenu and GMenuModel clients and the registrar),
`isle-shell` (the surface host), `isle-tokens`, `isle-ui` (the
Freya side shared with Isle's apps) and `isle-preview`; `data/` holds the shipped tokens with their schema and
contrast check, the translation tables, the Phosphor subset with Isle's own glyphs, and
the vendored `gtk-shell.xml`. `spikes/shell-ui/`
is the throwaway code behind B7 and is not built on.
