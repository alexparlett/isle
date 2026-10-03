# Architecture

How Isle is put together. This is the planned architecture of the
rust-desktop program: nothing in it is built yet, and each part is settled
in detail by the build packet that makes it (`docs/work/rust-desktop/
roadmap.md`). What each capability does is in `docs/design/`; why it is
built this way is in `docs/DECISIONS.md`.

## Process layout

```
isle-session                      supervisor: starts the compositor in the
 │                                logind session, restarts it, keeps crashes (B9)
 └── isle                         the compositor, one process
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
      ├── service worker          async runtime: the service crates (B11), the system
      │                           roles (B12, B28), the menu registrar (B18), the
      │                           settings D-Bus interface (B10), the portal backend
      │                           xdg-desktop-portal-isle (B13, B26), and the runner
      │                           for providers (B14)
      ├── PAM thread              lock-screen conversations, which can block (B12)
      └── watchdog thread         logs any main-loop stall over a frame's budget (B9)

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

| Store | What | Written by | Watched |
|---|---|---|---|
| `settings.kdl` | the user's choices, apps' preferences included | the compositor, or the user by hand | yes |
| `tokens.kdl` | design tokens | the user by hand; Isle never writes it | yes |
| state | window places, launch counts, session restore | the compositor | no |

v1's preferences, window places and launch counts are imported once, on
the first start that finds them (B27).

## Delivery

A PKGBUILD in this repository builds Isle from git; it is installed with
paru and updated through pacman (B29).

## Dependencies held as forks

- **Smithay**, pinned in `alexparlett/smithay` (to be created), carrying
  the async page flip patch (B6).
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
