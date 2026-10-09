# State

- Branch: `feature/rust-desktop`. v1's docs, canvas and screenshots are
  removed here and remain on `main`.
- Decisions: B1–B72 locked, the full text in `docs/DECISIONS.md` (B2
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
  and fixed.
- P4 is done (2026-10-05), its host run on tty8 passed by the user: ARCHITECTURE has How windows are kept,
  B40–B42 are recorded, `compositor.md`'s open points are answered and
  `p4-windows.md` ends with its outcome. Next: P5 (keyboard) or P6 (island
  and menus).
- Smithay fork made (2026-10-05): `alexparlett/smithay`, its `isle` branch
  at upstream `118e34f`; the SHAPE patch merged into it as
  alexparlett/smithay#1 and Isle pins that merge (`03d4920`). Isle's patches stay
  in the fork, not upstream (B43). Decorations now follow each protocol
  (B43): xdg- and KDE decoration negotiation honoured, clients told to draw
  their own with bars off, X11 by window type, Motif title bit,
  `_GTK_FRAME_EXTENTS` and SHAPE. xeyes in the VM draws as its eyes alone.
- P4 host run (2026-10-05): Chromium (Wayland and X11), VS Code, Steam and a
  full-screen game behaved on the 5070 Ti. Fixed: Isle drew while its console
  was in the background. Left for P14: the supervisor's exit clears the
  session variables from the user manager, which a session running beside it
  shares, and v1 and Isle share `~/.local/state/isle/session.log`.

- P5 planned in `p5-keyboard.md` (2026-10-05): `isle-keys` as the engine,
  the seat fed only what Isle emits, per-keyboard profiles and keymaps,
  withheld Cmd and Option, the shortcut set replacing `binds`, translation
  tables as a shipped data file, games, inhibitors, injection, recording,
  Sticky and Slow Keys, xremap retired. The user took every
  recommendation, with v1's chords revisited rather than carried and the
  game flag read from Steam's own labels. Phases 5.0–5.7.
- P5 5.0–5.6 done (2026-10-05): `isle-keys` (profiles, keymaps, chords,
  the shortcut table, translation tables, the engine); keyboards by name
  with Mac and PC profiles, the seat's keymap following the typing
  keyboard; Cmd and Option withheld and translated, one focus path; the
  `shortcuts` section with clash refusal, reserved chords, media keys,
  held chords; `data/keys/translation.kdl`; games, inhibitors with
  Cmd+Escape, Isle's own keys, recording; Sticky and Slow Keys; xremap
  conflicted away. Checked headless and in the VM (wev, xev, kitty, GTK 4).
- P5 is done (2026-10-05), its host run on tty8 passed by the user: ARCHITECTURE has How
  keys are read, B44–B46 are recorded, `compositor.md`'s and `gaming.md`'s
  open points answered, `p5-keyboard.md` ends with its outcome. Steam's
  `STEAM_GAME` X11 label is read through alexparlett/smithay#2, merged, and
  Isle pins that merge (`764ad1b`). Next: P6 (island and menus).
- P5 host run (2026-10-05): both Keychron receivers type `gb(mac)`, Cmd
  chords behave in kitty, Chromium, VS Code, GTK 4 and Qt 6, a game keeps
  Ctrl. Fixed on the way: the layout defaults to the system's, and input is
  opened when a session started on another console comes to the front.

- P6 planned in `p6-island-and-menus.md` (2026-10-05), reviewed: the user
  took every recommendation and kept it one packet. It covers
  the wallpaper as a compositor element, the island as one shape the
  compositor springs over a tree fed the spring's progress, one queue model
  sources post to, the top edge decided in one place, Control Centre's frame
  with Dark Mode and Keyboard layout, `isle-ui`'s first components, the bar
  as one surface per output, `isle-menus` with the registrar before the
  first client, `org_kde_kwin_appmenu`, `gtk_shell1` v5 with title-bar
  gestures, the fallback from labelled table rules, search, the window
  menu, a launch path. Research in `global-menu-2026.md`. Phases 6.0–6.8.
- P6 6.0–6.7 done (2026-10-06): the wallpaper, `isle-ui`'s first
  components and the launch path, the island and its queue, Control
  Centre and the top edge, the menu bar and its fallback, the four menu
  sources with the registrar, Help's search, the measures (release:
  island and bar under 2 ms p95, 163 MB and 92 MiB at most). Checked in
  the VM with Qt 6, Chromium, LibreOffice and GTK 3 and 4 apps.
- P6 is closed (2026-10-06) but for the host run on tty8, the user's:
  ARCHITECTURE has How the top of the screen works, B47–B52 are recorded,
  the intent docs' open points answered, `p6-island-and-menus.md` ends
  with its outcome. Next: P6's host run, then P7 (switcher).
- The user waived P6's host run (2026-10-07).
- P7 planned in `p7-switcher.md` (2026-10-07): the switcher takes no seat
  focus and reads keys while Cmd is held, commits only on release, one pure
  model, previews as compositor textures the tree draws through the shared
  Skia context, background apps by their processes, exclusive surface
  groups, attention levels, LauncherEntry, Steam names and covers in
  `isle-games`. The user took every recommendation. Phases 7.0–7.7.
- P7 is done (2026-10-07): the switcher takes no focus and commits on
  release, its model is pure, previews are compositor drawings copied into
  Skia images, background apps are known by their processes, LauncherEntry
  gives badges, progress and urgency, `isle-games` names Steam's games, and
  the switcher closes the menu-bar surfaces. ARCHITECTURE has How switching
  works, B53–B57 are recorded, the intent docs' open points answered,
  `p7-switcher.md` ends with its outcome; measures in
  `docs/research/rust-desktop/p7-switcher.md`. Checked headless and in the VM;
  no host run (the user's choice). Next: P8 (services, notifications and
  Control Centre).
- P8 planned in `p8-services.md` (2026-10-07): a crate per daemon family
  with one shape, snapshots coalesced onto the loop, audio over the Pulse
  protocol in pure Rust (libpulse links libdbus; B11 revised at close),
  DDC/CI written by Isle, OSDs from changes, the notification server owned
  before the socket, a pure banner model, Focus as a pure model with Game
  Mode as a mode, the tray Isle's own, Control Centre's modules, pages and
  edit mode, fake daemons on a private system bus. The user took every
  recommendation: Notification Centre's notifications moved in from P17,
  night light to P12, one packet. Phases 8.0–8.15.
- P8 8.0 done (2026-10-07): `isle-bus`, the harness's system bus and
  `isle-fakes`, snapshots coalesced onto the loop, the resume clock, typed
  page rows with scrolling, and the slider, scrubber, switch, spinner and
  scroll area. Next: 8.1 (power).
- P8 is done (2026-10-07): the service crates publish whole snapshots that
  reach the loop once a frame (B60); audio over the Pulse protocol in Rust,
  the camera from PipeWire's graph (B11 revised); DDC/CI written by Isle
  (B65); OSDs from changes (B62); Isle the notification server with one
  decision per arrival, a pure banner model and Notification Centre as the
  island's second panel (B63); Focus as one pure function with Game Mode a
  Focus (B64); the tray Isle's own watcher and host (B61); Control Centre's
  modules, pages, edit mode, gallery and keyboard. ARCHITECTURE has How
  services reach the shell, How notifications are decided and shown and How
  Focus decides; `p8-services.md` ends with its outcome; measures and the
  VM in `docs/research/rust-desktop/p8-services.md`. No host run. Next: P9
  (lock and prompts).
- P9 planned in `p9-lock-and-prompts.md` (2026-10-08), reviewed choice by
  choice: the lock is the core's state with its own render path and a
  runtime marker; PAM runs in an `isle-auth` helper process; the compositor
  holds the typed password; idle is one policy and one timer with
  ScreenSaver served; prompts are one queue over a compositor scrim with
  the polkit, keyring, NetworkManager and Bluetooth agents behind them; one
  engine ends the session; Force Quit. Every recommendation taken but two:
  no fingerprint in P9, and logind's `Unlock` honoured. Phases 9.0–9.14.
- P9 is done (2026-10-09): the lock as the core's state with its own render
  path and the runtime marker (B66); PAM in the `isle-auth` helper (B67); the
  minimal lock at the second panic (B68); idle as one policy with its holds
  (B69); prompts as one queue over a compositor scrim (B70), with the
  polkit, keyring, NetworkManager (VPN included) and Bluetooth agents keeping
  no secrets (B12 revised, B71); one engine ending the session, Force Quit
  and the not-responding alert (B72). ARCHITECTURE has How the lock holds,
  How idle decides, How prompts are asked and How the session ends;
  `p9-lock-and-prompts.md` ends with its outcome; measures and the VM in
  `docs/research/rust-desktop/p9-lock-and-prompts.md`. Not checked: the
  right password in the VM (no test password), a video holding idle (a
  logind idle inhibitor stood in), DPMS and the keyring on the host. No host
  run. Next: P10 (Spotlight).
- P10 planned in `p10-spotlight.md` (2026-10-09), reviewed: the user took
  every recommendation but one, every pick pasted rather than typed. A placed
  surface that takes the keyboard with keys buffered from the chord, one
  pure model, fast sources on the loop and slow ones on the worker in a new
  `isle-search`, ranking by tier then decaying use, Isle's own file index,
  `fend-core`, a generated emoji table, clipboard history moved in from P20,
  games from every launcher and root, the vault left to P20. Research in
  `docs/research/rust-desktop/spotlight-2026.md`. Phases 10.0–10.14. Next:
  10.0.
