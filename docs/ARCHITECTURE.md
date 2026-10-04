# Architecture

How Isle is put together. The compositor core is built (P2); the rest is
the planned architecture of the rust-desktop program, each part settled in
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
| `tokens.kdl` | `/usr/share/isle/`, `~/.config/isle/` | design tokens | the user by hand; Isle never writes it | yes |
| state | `~/.local/state/isle/<domain>.json` | window places, launch counts, session restore, crash history | the compositor and the supervisor | no |
| crash records | `~/.local/state/isle/crashes/` | the last ten | the compositor's panic hook and watchdog, then the supervisor | no |
| session log | `~/.local/state/isle/session.log`, `.old` | this session's and the last | the supervisor | no |

v1's preferences, window places and launch counts are imported once, on
the first start that finds them (B27).

## Delivery

A PKGBUILD in this repository builds Isle from git; it is installed with
paru and updated through pacman (B29).

## Dependencies held as forks

- **Smithay**, pinned by revision to upstream master (`118e34f`) until a
  patch needs the fork `alexparlett/smithay`, which will carry the async
  page flip (B6, P12 at the latest).
- **Freya**, through `alexparlett/freya` (B7), which also carries the
  compositor embedding crate made from the spike's glue. The fork must be
  at upstream v0.5.0-rc.8 or later.

## Repository

Built on `feature/rust-desktop` in this repository (B16), beside Isle v1,
which stays the daily desktop on `main` until the new one replaces it
(roadmap milestone M2) and is removed at cut-over. The Cargo workspace
is at the root: `crates/` holds `isle-compositor`, `isle-session`, `isle-cli`
(the `isle` command), `isle-ipc`, `isle-settings` with its derive and
`isle-state`; `data/` holds the shipped tokens with their schema and
contrast check, and the Phosphor subset. `spikes/shell-ui/`
is the throwaway code behind B7 and is not built on.
