# P2 · Compositor core

Planned 2026-10-03. The roadmap's brief: the Cargo workspace; the pinned
Smithay fork (B6); udev/DRM and winit backends; libinput; outputs with
hotplug, scale and the KVM lessons; xdg-shell and layer-shell; the calloop
loop, service worker, watchdog and session supervisor (B9); the settings
store and its D-Bus interface (B10) with `settings.kdl`, per-app sections
and writes refused while it does not parse (B27), and the state store; the
`isle` CLI and the test interface; the NVIDIA VRAM profile. Ends booting in
the VM with a terminal open.

P2 is the floor every later packet builds on. It draws no shell (P3), places
no window beyond centring it (P4), runs no Xwayland (P4) and translates no
keys (P5). What it must get right is the shape: one loop that never blocks,
a worker that owns D-Bus, one writer of settings, outputs that survive
anything, and a test surface that lets every later packet be checked by
script.

## What it answers to

- Decisions: B6 (the fork), B9 (loop, worker, panic boundaries, supervisor,
  watchdog), B10 (settings, state, tokens as data), B26 (the portal and
  greeter live here later, so the worker and the binary leave room), B27
  (per-app sections, refused writes), B28 (layer-shell served, no
  ext-session-lock).
- Intent: `compositor.md` (Outputs; Pointer, touchpad and gestures 1–2, 4;
  Keyboards 1; Session and robustness 1–3, 5, 6; Wire protocols, the rows
  below), `platform.md` (Settings 1–9, State, Compositor D-Bus API and CLI,
  Supervisor and crash records 1–4, 6, 7, Test hooks and VM workflow).
- Evidence: `smithay-and-gaming.md` §0 (the host's two GPUs), §1.1–1.3
  (Smithay master, its churn and protocol coverage), §2.1 (NVIDIA
  workarounds), §4.1–4.2 and §4.6 (niri's redraw loop and push-based render
  lists, cosmic-comp's NVIDIA quirks); `ui-and-services.md` §4 (KDL crates,
  reload patterns); `shell-ui-spike.md` (the spike base's frame pacing).

## Choices this plan makes

The user confirmed 1, 2 and the fork's creation at review (2026-10-03); the
rest are recommendations the plan carries.

1. **Binaries.** `isle-session` (the supervisor), `isle-compositor` (the
   compositor; fifteen characters, so `pkill -x` and the NVIDIA profile's
   process name both match it whole) and `isle` (the CLI, as `platform.md`
   names it). ARCHITECTURE's process tree is renamed to match.
2. **D-Bus name.** `io.github.alexparlett.Isle`, one object per interface
   family under `/io/github/alexparlett/Isle/` (`Settings`, `Session`,
   `Displays`, `Windows`, `Version`, and `Test` in test builds), each
   interface versioned (`io.github.alexparlett.Isle.Settings1`). Isle owns
   no domain; a reverse-DNS name on the GitHub account is the convention
   Flatpak uses.
3. **KDL.** The `kdl` crate (6.7, KDL v2, format-preserving) reads and
   writes `settings.kdl`; `tokens.kdl` is already KDL v2, so one parser
   serves both. `knus` derives types but is KDL v1 (to confirm in phase 0)
   and cannot write back with comments kept. The typed layer is Isle's own:
   a derive macro (`isle-settings-derive`) generates parsing, validation,
   the D-Bus value types and an introspectable schema from one Rust
   definition (`platform.md` Settings 8). No `include` in P2.
4. **State format.** One JSON file per domain (serde), each with a
   `version` field, written atomically from the worker. State is never
   hand-edited, so it needs no comments, and JSON keeps it inspectable.
5. **Worker runtime.** A tokio multi-thread runtime with two workers on its
   own thread; zbus 5 on its tokio feature. Requests reach the loop over a
   calloop channel and are answered through a oneshot with a one-second
   limit, so a busy loop returns an error rather than hanging a client.
6. **Redraw loop.** niri's per-output state machine (idle, queued, waiting
   for vblank, waiting for an estimated vblank): a change queues a redraw,
   redraws run at the end of a dispatch, and frame callbacks are throttled
   by a timer at the estimated vblank when nothing is damaged, so a static
   desktop presents nothing. Render elements are collected by a push
   closure from the start (niri PR #3113). Timing never relies on DRM
   presentation timestamps or early commit returns (NVIDIA, §2.1).
7. **Watchdog.** The loop stamps a heartbeat (time and the source it is
   dispatching) before each dispatch; a thread checks it every few
   milliseconds and logs any stall over the fastest output's frame budget
   with that source. A stall of ten seconds writes a crash record with the
   main thread's backtrace and aborts, so the supervisor starts a fresh
   session; `debug { watchdog-abort false }` turns that off for debugging.
8. **Decorations until P4.** xdg-decoration and KDE server-decoration are
   served preferring client-side, so clients draw their own frames until
   P4's Freya title bars exist; P4 flips the preference.
9. **NVIDIA defaults.** Overlay planes off on NVIDIA (cosmic-comp's quirk,
   untested on Blackwell); the render GPU is Smithay's `primary_gpu()`
   (`boot_vga`, the NVIDIA card on the host) with `render-device` as an
   override; the application profile ships as
   `data/nvidia/50-isle.json` (`GLVidHeapReuseRatio` 0 for
   `isle-compositor`). P14 installs it; P2 measures it.

## The workspace

At the repository root, beside v1's tree, which it does not touch.

```
Cargo.toml                    the workspace; excludes spikes/
rust-toolchain.toml           pinned stable
crates/
  isle-compositor/            lib + bin: loop, backends, outputs, input,
                              xdg and layer shell, render, worker, watchdog
  isle-session/               bin: the supervisor
  isle-cli/                   bin `isle`
  isle-ipc/                   the D-Bus interfaces and zbus proxies, shared by
                              the compositor, the CLI, Isle's apps and tests
  isle-settings/              schema, KDL read and write, validation, the
                              watch rules; Isle's apps' sections live here
  isle-settings-derive/       the schema derive macro
  isle-state/                 per-domain state files
data/
  tokens/                     tokens.kdl, schema.md, contrast.py, from p1/system/
  icons/phosphor/             the vendored subset, from p1/system/icons/
  nvidia/50-isle.json         the application profile
```

P1's boards read `data/tokens/tokens.kdl` after the move;
`p1/harness/serve.py` serves from the repository root so their relative
paths still resolve. DESIGN.md, `schema.md` and the intent docs point at
the new path. P3 adds `isle-shell` and its token reader; later packets add
crates beside these.

Smithay comes from `alexparlett/smithay`, branch `isle`, cut from master at
the newest revision that builds the spike base unchanged or with the
churn `smithay-and-gaming.md` §1.1 lists, pinned by `rev`. Upgrades are
their own commits. Patches (the async flip, P12) land on that branch.

## Phases

Each phase ends in a check run by script or in the VM, and lands as its own
commits. Nothing later starts on an unchecked phase.

### 2.0 · Ground

- The Smithay fork created with `gh` (asking first) and pinned; the workspace, crates as empty skeletons, `cargo build` and
  `cargo clippy` clean.
- P1's system files moved into `data/`; the boards still render from
  `serve.py` (checked in the browser pane); references updated.
- Confirm `knus`'s KDL version and `kdl` 6.7's round-trip on `tokens.kdl`
  (a test that parses and re-emits it byte for byte).

**Check:** the workspace builds; the round-trip test passes; one board
renders with the moved tokens.

### 2.1 · A nested compositor

- `State { backend, isle }` on one calloop loop; the winit backend; the
  Wayland display; seat with keyboard and pointer; xdg-shell with every
  toplevel centred on the output, click to focus and raise, popups with
  their positioner constrained to the output.
- GlesRenderer with damage tracking, the redraw loop (choice 6),
  cursor-shape and client cursors.
- The protocol baseline: core, xdg-shell, xdg-decoration and KDE
  server-decoration (choice 8), xdg-activation, xdg-output,
  linux-dmabuf v4, viewporter, fractional-scale, presentation-time,
  single-pixel-buffer, cursor-shape, data device and primary selection.
- tracing to stderr; the watchdog thread (choice 7); a panic hook that
  writes the crash record (format fixed here, used by 2.7).

**Check:** on the host, nested under Hyprland: kitty, foot, Firefox,
a GTK 4 and a Qt 6 app map, take keys and pointer, open menus and close;
weston-simple-egl animates. A static desktop presents no frames over ten
seconds (counted in the log). An injected stall is logged with its source.

### 2.2 · Worker, D-Bus and the CLI

- The service worker thread (choice 5) with the bus name (choice 2); a
  service that panics is restarted there and logged.
- Interfaces: `Version` (commit, build time, features), `Session` (quit),
  `Windows` (list with id, app id, title, output; focus; close),
  `Displays` (list), each with its change signals; nothing polls.
- `isle`: one subcommand per method, `--json`, exit codes that mean
  failure; it talks only through `isle-ipc`'s proxies.

**Check:** `isle version --json` reports the running build; `isle windows
list` sees kitty; `isle windows close` closes it; a request while the loop
is held by an injected stall returns the time-limit error within a second.

### 2.3 · Settings

- `isle-settings` with the derive macro (choice 3). P2's schema: outputs by
  identity (mode, scale, position, transform, enabled, and the VRR mode
  stored with its three values, of which P12 implements the full-screen
  one); `render-device`; keyboard layouts, variants, options, repeat and
  Num Lock; pointer and touchpad settings per device identity (acceleration
  profile, speed, natural scroll, left-handed, tap, scroll method); a
  minimal bind table (launch a terminal, close the window, quit Isle,
  switch VT) that P5 replaces with the shortcut set; `debug`. An `app`
  section mechanism with one test app's schema.
- The interface: Get, Set (validated, applied live, then written
  atomically keeping comments, order and formatting of untouched nodes),
  Changed with the keys, Schema (introspection), Status (the standing
  error with file, line, column and message).
- The watcher: through the directory and symlinks, own writes recognised
  by content; a rejected hand edit keeps the last good settings and is
  reported on Status and in the log (the island's glyph is P6's); a Set
  while one stands is refused with that error (B27).
- Every P2 key applies live: an output's scale or mode, keyboard layout,
  pointer speed.
- `isle settings get|set|status|schema`.

**Check:** unit tests for parse, validate, write-back preserving comments,
symlinked and renamed files, own-write suppression; `platform.md`
acceptance 7 run as a script; changing an output's scale through `isle
settings set` rescales kitty without a restart.

### 2.4 · State

- `isle-state`: one JSON file per domain under `~/.local/state/isle/`,
  schema-versioned; debounced atomic writes on the worker; an unreadable
  or newer file moved aside as `.bad`, the domain started empty, the log
  saying so. Runtime facts under `$XDG_RUNTIME_DIR/isle/`.
- P2's domains: the supervisor's crash history (2.7) and the v1 import
  marker, which P14 uses. Window places and the rest come with their
  packets.

**Check:** unit tests for round trip, a damaged file, a newer version, and
a write burst coalesced into one write.

### 2.5 · Headless backend and the Test interface

- A headless backend: GlesRenderer on a surfaceless EGL display on a render
  node, with outputs the Test interface adds and removes; the same loop
  and the same output code as the DRM backend.
- The `Test` interface behind the `test-hooks` cargo feature, never in a
  package: inject pointer and keys as a physical device; render an output
  to PNG; dump the window and output state; count presented frames; force a
  main-loop stall, a service panic and a core panic.
- An integration harness: `cargo test` starts the compositor headless on a
  private bus (`dbus-daemon --session` per test), runs real clients
  (`weston-simple-shm`, foot) and asserts through `isle-ipc`.

**Check:** the suites from 2.1–2.4 run headless in `cargo test`, including
idle frames, the stall time limit and a settings hand edit.

### 2.6 · DRM, libinput and outputs

- The udev backend: libseat through logind, VT switching, `DrmOutputManager`
  with `DrmCompositor` per output, GBM allocation, primary-plane scanout on,
  overlay planes off on NVIDIA (choice 9), the cursor plane with drawing as
  the fallback and cursor planes reassigned on every output change.
- libinput with device settings applied live by identity; keyboard layouts
  and repeat from settings.
- Outputs by EDID make, model and serial, restored with their settings;
  the last enabled output cannot be disabled; a mode change reverts unless
  confirmed within a timeout (`isle displays confirm`).
- The KVM lessons: windows on a vanished output move to a remaining one,
  clamped whole, and go back when it returns; with no output lit the
  session runs headless and re-probes connectors every few seconds;
  `isle displays redetect` wakes every output, releases it and probes
  again as one step that cannot stop half way; the DPMS state is applied
  again whenever the set of outputs changes. A floating window outside
  every output is moved onto the nearest.
- Layer-shell (B28) with exclusive zones honoured in the work area.

**Check:** headless tests for output removal, return, none lit and
redetect, with windows asserted on-screen throughout. In the VM, on tty2
by hand first: the compositor takes the seat, draws, kitty runs, VT
switching to tty1 and back works, scale changes live.

### 2.7 · The supervisor

- `isle-session`: started from the login shell or a session entry, inside
  the logind session scope; starts the compositor as its child and is the
  only thing that restarts it; a development override (`ISLE_COMPOSITOR`)
  names the binary, and the installed default never points at a checkout.
- Exports the session environment (`platform.md` Session start 6, the
  NVIDIA variables when `/proc/driver/nvidia/version` exists) to the
  systemd user manager and D-Bus activation once the compositor's socket
  is up, starts `graphical-session.target`, and on a clean exit stops it
  and clears the environment.
- On a crash: restart after a second, backing off after repeated quick
  deaths and never giving up; a crash record in
  `~/.local/state/isle/crashes/` (time, build, panic and backtrace from the
  hook, the last log lines, recent stalls), the last ten kept; after three
  crashes within a minute of start the next start ignores the user's
  `settings.kdl` and says so on Status (the island's notice is P6's).
- Logs to the user journal and to `~/.local/state/isle/session.log`, the
  previous session's kept beside it.

**Check:** headless: a forced core panic restarts the compositor within
two seconds with a record written; four forced panics give a safe start;
`kill -9` restarts it. Greeter mode, Login Items and the lock-after-crash
rule come with P14 and P9.

### 2.8 · The VM, and the host

- The guest boots the new session: the maintainer's local tooling in
  `dev/` (git-ignored, never staged) gains a switch so tty1's autologin
  runs `/repo/target/debug/isle-session` with the override instead of
  Hyprland; v1's guest setup stays one switch away.
- The bind opens kitty; `dev/test-vm.sh shot` shows it; `isle version`
  over ssh matches the build.
- On the host, nested: compositor VRAM with and without the NVIDIA profile
  (niri's reference is about 100 MiB with it), recorded in
  `docs/research/rust-desktop/`.
- On the host, on a spare VT with the user present: the
  compositor lights the ultrawide on the 5070 Ti, kitty runs, switching
  back to the daily Hyprland session works. This retires the NVIDIA DRM
  risk ten packets before M2 needs it.

**Check:** the VM screenshot with kitty open on the new compositor; the
VRAM figures; the host smoke result.

### 2.9 · Close

- ARCHITECTURE: the process tree with the binary names, the bus name, the
  workspace layout, the worker and redraw loop as built.
- The intent docs' "Open for its packet" items P2 settles, answered where
  they stand (the KDL crate, `include`, the state format, the bus name,
  the watchdog's long hang). DECISIONS gets an entry for any choice above
  that later packets must keep.
- `state.md`, `progress.md`, the roadmap's P2 entry marked done.

## Not in P2

The shell and tokens reading (P3); stacking, snapping, places, title bars,
Spaces and Xwayland (P4); keyboard profiles, Cmd translation and the
shortcut set (P5); lock, idle and DPMS timeouts (P9); explicit sync, VRR's
full-screen mode, tearing, direct scanout of games and the gaming globals
(P12); greeter mode, the PKGBUILD, installing the NVIDIA profile and the v1
import (P14). The remaining wire protocols arrive with the packet that
needs them.

## Depends on, and feeds

- Needs nothing built. The Freya fork at rc.8 is not needed here.
- P3 takes the worker, the loop's surface hooks, the GLES context and the
  token files' new home. P4 takes the window list, the headless harness
  and the output model. P5 takes the bind table's place. Every packet takes
  the Test interface and the CLI.

## Done when

- The VM boots from tty1 into `isle-session` and the compositor on DRM,
  and kitty opens on the bind and takes typing.
- Nested on the host, kitty, foot, Firefox, a GTK 4 and a Qt 6 app run.
- `cargo test` passes headless: idle presents no frames; output removal,
  return, none lit and redetect keep every window on-screen; a settings
  hand edit with an error keeps the last good settings, refuses Sets and
  keeps comments on a good Set; a core panic restarts with a record; an
  injected stall is logged and a D-Bus request during it times out.
- Every P2 setting applies without a restart.
- The NVIDIA VRAM figures are recorded.

## Risks

- **Smithay master churn.** `delegate_dispatch2!` and a wayland-rs rework
  are mid-flight. Pinning on the fork contains it; an upgrade is a
  deliberate commit with the test suite as its check.
- **Software GL in the VM.** The guest has plain virtio-gpu, so EGL runs on
  llvmpipe through GBM. Hyprland works there, so the path exists; if
  Smithay's GBM path fails on it, the guest gets `virtio-gpu-gl` and loses
  QEMU's screendump, which `dev/test-vm.sh` documents.
- **The VM cannot exercise the host's hardware.** No KVM switch, no
  NVIDIA, no cursor plane, no real hotplug. The headless tests carry the
  output logic; the host smoke (2.8) and P14 carry the rest.
- **NVIDIA DRM on Blackwell.** No report of a Smithay compositor on a 5070
  Ti was found. The host smoke finds out early; if it fails, P12's scope
  grows and the fix starts there.
- **Settings write-back.** Keeping formatting through `kdl`'s document
  model while the schema is typed is the fiddliest code here; the
  round-trip tests in 2.0 and 2.3 are written before the writer.
