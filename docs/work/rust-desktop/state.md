# State

- Branch: `feature/rust-desktop`. v1's docs, canvas and screenshots are
  removed here and remain on `main`.
- Decisions: B1–B42 locked, the full text in `docs/DECISIONS.md` (B2
  revised to v1 parity; B17 the macOS north star). The brainstorm is
  complete.
- Shell UI spike done (`spike-shell-ui.md`; results in
  `docs/research/rust-desktop/shell-ui-spike.md`). Its code in
  `spikes/shell-ui/` is throwaway.
- Intent docs: `docs/design/` (compositor, shell, gaming, apps, platform),
  drafted, with B22–B30 folded in.
- Roadmap: `roadmap.md`, P1–P21 in milestones M1 (runs in the VM), M2
  (replaces v1 on the host) and parity, then P22 (HDR, B23) after
  cut-over.
- Spine: `docs/ARCHITECTURE.md` (planned), `docs/DECISIONS.md`,
  `docs/DESIGN.md` (placeholder until P1). README and CLAUDE.md point at
  them.
- Blocking P3 onwards: the user is merging upstream Freya v0.5.0-rc.8
  into `alexparlett/freya` (fork at rc.4).
- Coverage audit done: every decision reaches its intent docs,
  ARCHITECTURE and the roadmap, and the docs agree with DECISIONS.
- Presentation is separated from capability: each intent doc ends with a
  "For the design process" list that P1 answers (B24 rewritten). B18's
  arrangement stays a decision.
- The planning docs are committed.
- P1 planned in `p1-design.md`: six stages with a user gate each;
  mock-ups in HTML on a shared harness that reads `tokens.kdl`, moving to
  Freya once P3's preview mode exists; boards also published as private
  Artifacts.
- P1 stage 1: the three research notes are written
  (`design-macos.md`, `design-references.md`, `design-v1-lessons.md`),
  the interview is done, and `p1/brief.md` passed gate 1.
- P1 stage 2, batch 1 passed gate 2 (`p1/structure/`, board
  `p1/structure-1.html`, published as a private Artifact) after revision 1:
  at rest only the wallpaper and v1's pill; menus on demand; no Dock. B18
  and B19 are revised and the intent docs, DESIGN and the roadmap follow.
- P1 stage 2, batch 2 passed gate 2 (board `p1/structure-2.html`):
  notifications, Focus, Control Centre, lock and greeter, prompts,
  Spotlight with the Apps grid, capture, app conventions and Settings.
  Every recommendation accepted; the intent docs follow. Next: batch 3
  (parity surfaces: Notification Centre and widgets, Mission Control and
  the window model's rest, Files, Monitor, Keychain, Cleaner, the
  dropdown terminal, PiP, the on-screen keyboard, clipboard and emoji),
  which can run beside stage 3 (directions).
- P1 stage 3 passed gate 3: direction A, Frost (board
  `p1/directions-1.html`, tokens in `p1/directions/`). Batch 3 passed
  gate 2 (`p1/structure-3.html`); every recommendation accepted, the
  dashboard on Ctrl+Option+Cmd+W, no Connect to Server. Stage 2 is
  complete.
- P1 stage 4 passed gate 4 (board `p1/system-1.html`): `p1/system/`
  holds `tokens.kdl`, its `schema.md` for P3, `contrast.py` (all pass),
  `components.css`, `icons.md` (Phosphor, Papirus, Bibata) and
  `theming.md`. `docs/DESIGN.md` has its first draft. B13 now includes
  the Settings portal. - P1 stage 5, wave A passed gate 5 (board `p1/surfaces-a.html`, specs
  in `p1/surfaces/`, drawing kit `p1/surfaces/scene.js`): the island and
  its panels, menus, window chrome, the switcher, the wallpaper.
- Wave B passed gate 5 (board `p1/surfaces-b.html`): banners, Focus,
  Control Centre's modules, the lock and prompts, Spotlight, capture,
  Settings and the app conventions.
- Wave C passed gate 5 (board `p1/surfaces-c.html`); stage 5 is
  complete. The graph colours were replaced after a colour-vision check
  (`p1/system/data-colours.md`).
- P1 is done: stage 6's review (critique, accessibility, copy;
  `p1/review/`) applied, DESIGN.md signed off at gate 6, B31–B34 recorded,
  every intent doc's design list answered. The tokens, schema, contrast
  script and Phosphor subset in `p1/system/` move into the workspace when
  P2 lays it out; P3 reads the schema. Next: P2 (compositor core), planned
  by its own run; P3 waits for the Freya fork at rc.8.
- P2 planned in `p2-compositor.md`: nine choices made provisionally
  (binary and bus names, `kdl` with an own derive, JSON state, tokio on
  the worker, niri's redraw loop, the watchdog, client-side decorations
  until P4, NVIDIA defaults) and ten phases from the workspace to the VM
  boot and a host smoke test. Reviewed: the user took every
  recommendation (`isle-compositor`, `io.github.alexparlett.Isle`, the
  fork made with `gh`, the host smoke in P2).
- P2 phase 2.0 done: the Cargo workspace at the root (seven crates under
  `crates/`), P1's tokens, schema, contrast script and Phosphor subset in
  `data/`, the boards reading them there. Smithay pinned upstream at
  `118e34f`; the user deferred the fork. `knus` is KDL v1 only; `kdl`'s
  printer drops text, so settings writes splice by span.
- P2 phase 2.1 done in part: `isle-compositor` runs nested on the host
  (winit on the 5070 Ti) with the protocol baseline, niri's redraw loop,
  the watchdog and crash records. kitty, Chromium, VS Code and zathura
  map; a static desktop presents no frames. Not yet checked: typing and
  clicking in clients (by hand, or scripted once 2.5's Test interface
  exists), and foot, Firefox, a GTK 4 and a Qt 6 app, which the host
  does not have.
- P2 phase 2.2 done: the worker thread (tokio, two workers) runs the
  D-Bus service under a restarting supervisor and reaches the loop by
  jobs with a one-second limit. `io.github.alexparlett.Isle` serves
  Version1, Session1, Windows1 and Displays1, and Test1 in `test-hooks`
  builds; `isle` covers them with `--json`, exit 1 on failure and 69 when
  Isle is not running. Checked on a private bus.
- P2 phase 2.3 done: `isle-settings` with `#[derive(Section)]` and
  `#[derive(Choice)]`, the P2 schema (keyboard, outputs and pointers by
  identity, render device, a three-chord bind table, debug, one example
  app section), and a store that splices Sets into the file's own text.
  The worker owns the store and watches the file with inotify through its
  directory and a symlink's target; the loop applies keyboard, output
  scale, position and the watchdog live. `Settings1` and `isle settings
  get|set|status|schema`. `platform.md` acceptance 7 passes nested.
- P2 phase 2.4 done: `isle-state` keeps one versioned JSON file per
  domain under `~/.local/state/isle/`, moving a damaged or newer file
  aside as `.bad`; the worker's `Saver` writes the last value of a burst
  after half a second. Domains so far: the supervisor's crash history and
  the v1 import marker.
- P2 phase 2.5 done: `isle-compositor --headless` draws each output into
  a texture on an EGL device with no surface, a timer standing in for
  vblank. `Test1` injects keys, pointer and buttons through the same
  handlers devices use, adds and removes outputs, counts frames and
  writes screenshots. `isle-test-client` is a one-colour shm window that
  prints what it receives. `cargo test --workspace --features
  isle-compositor/test-hooks` runs seven headless tests, each on its own
  bus and runtime directory: mapping, input, idle frames, the stall
  timeout, a settings hand edit, a screenshot, outputs.
- P2 phase 2.6 done: windows follow an output away and back by its
  identity, the session survives no output, layer-shell with exclusive
  zones, and the udev backend (libseat, DRM output managers, libinput,
  themed cursors, VT switching, re-probing, `isle displays redetect`).
  Checked in the VM on tty8 (llvmpipe): kitty draws and takes typing,
  switching to tty1 and back redraws, scale changes live, redetect
  relights with kitty in place. Second-GPU scan-out moved to P14 and
  mode confirm-or-revert to P13.
- P2 phase 2.7 done: `isle-session` starts the compositor (beside it, or
  `ISLE_COMPOSITOR`), learns its socket through an inherited pipe,
  exports the session environment and starts `isle-session.target`
  (`data/systemd/`; graphical-session.target refuses a manual start),
  keeps `session.log` and the old one, adds the exit and log tail to
  crash records and keeps ten, restarts with a doubling backoff, and
  after three quick deaths starts on the defaults until `isle settings
  use-mine`. Four supervisor tests.
- P2 phase 2.8, VM part done: the guest's tty1 autologin runs
  `/repo/target/debug/isle-session` when `~/.isle-rust-session` exists
  (guest-side switch in `~/.bash_profile`; remove the file for Hyprland);
  `isle-session.target` and `graphical-session.target` come up, Super+Return
  opens kitty, typing works, `isle version` over ssh matches. Nested VRAM
  recorded (`docs/research/rust-desktop/p2-vram.md`).
- P2 host smoke run done (2026-10-04, tty8, run by the user): Isle opened
  the 5070 Ti (card2, overlay planes off), lit the AOC on DP-4 at
  3440x1440 at its preferred 60 Hz, took the Keychron and the pointer, no
  stalls or errors, about 6 MiB VRAM. The pointer jumped until pointer
  motion queued a redraw for Isle's own cursor (fixed). Cmd+Return opened
  nothing because v1's xremap, still running from the Hyprland session,
  grabs the Mac keyboard and sends Cmd+Return as Ctrl+Return. xremap.service
  is WantedBy graphical-session.target, so isle-session.target would start
  it in a real Isle session too: P5 retires xremap, and P14 must disable
  v1's units before the host moves over.
- P2 is done (2026-10-04): ARCHITECTURE has How the core runs, B35 and
  B36 are recorded, the intent docs' open points P2 settled are answered,
  and `p2-compositor.md` ends with its outcome. Next: P3, the shell
  runtime, once the Freya fork is at rc.8; P4 (windows) and P5 (keyboard)
  need only P2 and can start before it.
- P3 planned in `p3-shell-runtime.md` (2026-10-04): `freya-embed` in the
  fork, `isle-shell`, `isle-tokens`, `isle-ui` and `isle-preview` here;
  one Skia context, waking instead of ticking, panic boundaries, tokens
  generated from the shipped file, blur and the glass floor, AT-SPI, the
  preview under `dx`. The user took every recommendation. Phases 3.0–3.10.
- P3 3.0–3.3 done (2026-10-04): `freya-embed` on the fork's
  `isle/freya-embed` branch,
  `isle-shell` hosting surfaces with input, focus return and AT-SPI, the
  probe surface in test builds, `Shell1` and `isle shell`.
- P3 3.4–3.7 done (2026-10-04): panic boundaries with rebuild and the
  five-a-minute guard, `isle-tokens` and the `appearance` settings live in
  every surface, compositor transitions with a rounded clip, glass blur
  and its contrast floor.
- P3 is done (2026-10-04): ARCHITECTURE has How the shell runs; B37 (the
  surface model), B38 (tokens typed from the shipped file) and B39 (glass
  is the compositor's) are recorded; `shell.md`'s open points answered and
  its budgets set (250 MB RSS, 200 MiB VRAM; a soak margin of 10% or 20
  MB); measurements in `docs/research/rust-desktop/p3-runtime.md`. The
  preview is `isle-preview <surface>`, with `--watch` in place of dx's hot
  patch, which fails across workspace crates. Open: the soak keeps 2 to 5
  KB a cycle on the loop's thread (VRAM flat); a heap profiler settles it
  before the 24-hour soak. The fork's changes merged as
  alexparlett/freya#10 (2026-10-05) and Isle pins that merge on the fork's
  `main`. Next: P4 (windows), P5 (keyboard) or P6
  (island and menus), all now unblocked.
- P4 planned in `p4-windows.md` (2026-10-05): one window registry with
  Smithay's `Space` as its shown part, desktops per output, places, zones,
  hide and quit, title bars as attached Freya surfaces, the zoom menu,
  Xwayland. The user took every recommendation. Phases 4.0–4.10.
- P4 4.0–4.9 done (2026-10-05): the window registry, placement and
  places, zones and tiling, apps with hide, close and quit, desktops and
  full-screen desktops, rounded corners, shadows and window motion, title
  bars as attached surfaces, the zoom menu and pointer panel, Xwayland.
  Checked by the workspace's 118 tests, 38 of them headless window and X11 tests, and in the VM with kitty, GTK 4, Qt 6,
  xterm and xeyes, where the ready pipe held open by Xwayland was found
  and fixed. Waiting on the user: the host run on tty8 (Firefox, Chromium,
  VS Code, Steam's client and menus, a full-screen game).
- P4 is done but for that host run: ARCHITECTURE has How windows are kept,
  B40–B42 are recorded, `compositor.md`'s open points are answered and
  `p4-windows.md` ends with its outcome. Next: P5 (keyboard) or P6 (island
  and menus).

