# P7 · Switcher

Planned 2026-10-07. Reviewed by the user the same day: every
recommendation taken.

The roadmap's brief: Cmd+Tab and Cmd+` (B20), the switcher's cards carrying
what a Dock would (B19): badges, progress, attention, and hidden, minimised
and background marks. Launching from Spotlight's Apps grid with its
favourites row is P10's.

P6 left Cmd+Tab and Shift+Cmd+Tab as reserved chords that only log. The
keyboard engine already holds them: `held_chord` is set on the press, Cmd
stays latched under Sticky Keys, and `check_held_chord` notices when the
chord's modifiers are let go, but nothing acts on it. Cmd+` cycles the front
app's shown windows by open order (P4). The registry knows each window's app,
pid, hidden, minimised, attention and not-responding state, and the apps in
recency order; it knows nothing of an app with no window, of urgency, of
LauncherEntry's badges and progress, or of a game's name and art. The
compositor can draw a window into an offscreen texture (`State::snapshot`) but
no shell surface can show one. P7 makes Cmd+Tab what the switcher's sheets
say, and makes the switcher the place that answers what is running now that
there is no Dock. What it must get right is the hold: every press commits on
release exactly once, to the entry selected, with no modifier left stuck and no
key leaking to the app in front (`shell.md` acceptance bar 7).

## What it answers to

- Decisions: B13 (previews are the compositor's), B17, B19 (the cards carry
  the Dock's duties), B20 (apps, Cmd+`, Cmd+Q, keep-running), B21 (modifiers
  released before focus moves; Cmd+Tab and Cmd+` before translation), B30
  (Cmd+Tab in the escape set; Q quits keep-running apps), B37 (surfaces), B39
  (glass is the compositor's), B40 (the registry; hidden and minimised as two
  flags), B44 (the seat fed only Isle's keys), B47–B52 (the island's model as
  the one place sources post, the main screen, the launch path).
- Intent: `shell.md` App switcher 1–8 and acceptance bar 7, Shell runtime 10
  (one centre surface at a time); `compositor.md` Apps, hide, quit 2, 3, 7,
  8, 10, 11, Minimise 1 and 4, Shortcuts and keybinds 3, 5 and 7;
  `platform.md` Notifications and badges 6; `gaming.md` Games as apps 1, 4
  and 6, Game Mode 9, Full-screen Spaces 4.
- Left to P7 by earlier packets: the held chord's commit (P5), app recency
  and the hidden and minimised flags (P4), attention and icons, and closing
  the bar when the switcher opens (P6).
- Design, binding: `p1/structure/switcher.md`, `dock.md`, `competition.md`
  (the centre of the screen, the order of focus, Cmd+Tab's place among
  chords); `p1/surfaces/switcher.md`; `review/accessibility.md` and
  `copy.md`; DESIGN.md's switcher entry, Components (badge, progress,
  spinner), Motion. The switcher's tokens are already in
  `data/tokens/tokens.kdl`.
- Evidence: `macos-model.md` §3 (app-centric switching, LauncherEntry),
  `design-v1-lessons.md` §3.5 and §5 Switcher (hover guards, four content
  models, wrapping), `isle-v1-inventory.md` §3.6 and §4 Switcher.

## Choices this plan makes

1. **The switcher never takes the seat's focus.** While the chord is held,
   the compositor reads every key at the top of `key_from_device`, where the
   keyboard move and the top edge's Escape already pre-empt keys, and eats
   their releases through `swallowed`. The window in front keeps keyboard
   focus until the commit (`structure/switcher.md` Keyboard), so Escape
   gives a game back its focus and pointer lock untouched, Game Mode's solid
   and still forms stay right (P6 computes them from the focused window,
   which a focused shell surface would hide), and none of the bound chords
   Cmd+Q, Cmd+H or Cmd+Escape, nor Cmd translated to Ctrl for a shell
   surface, stand between the user and Q, H, W and the arrows. The surface
   takes the pointer only. Rejected: focusing the surface, which needs every
   one of those exceptions undone.
2. **Nothing moves until release.** The press arms the switcher and changes
   no focus; releasing Cmd commits to the selection. A release before
   `timing-switcher-delay-ms` commits to the second entry with nothing drawn
   and no texture captured, which is the quick tap the sheets ask for ("the
   previous app comes forward"); a second Tab inside the delay draws at
   once. Focus moves once per press, so a commit can never land on an app
   that was briefly in front.
3. **One model, pure and tested.** A module `switcher` in `isle-shell` holds
   the entries (apps with a window by recency, the front app first; then the
   background group), the selection, the window row, the delay and its
   second-Tab shortcut, and the layout: card size after shrinking to the
   floor, rows of equal count past it, the background group whole at the
   end or on its own row, the window row under the last row centred on its
   app, the panel's size, all at three quarters of the output's width. Keys
   go in; effects come out (commit to an app or a window, quit, stop, hide,
   show, close, cancel, open or leave the window row). It takes a clock from
   its caller like the island's queue, so the delay is unit-tested with a
   fake one and the loop arms one timer. While held, entries change as apps
   change: a quit app's plate stays with the spinner until it has gone, a
   closed last window moves the app to the background group, a new app
   joins at the end; the selection follows its app, never its index.
4. **Ending a hold.** The released chord commits (P5's
   `check_held_chord`, now with an effect). Return commits without
   releasing; Escape cancels and leaves quits, hides and closes done. A
   switch away from the console, the lock, the session going inactive or
   the typing keyboard being removed cancels, so no hold outlives the keys
   that made it. A window mapping or taking focus mid-hold changes nothing
   but the entries. A held Cmd+Tab under a keyboard-shortcuts inhibitor
   that is not a game stays the inhibiting client's, as P5 built.
5. **Keys while held.** Tab and Shift+Tab, ← and →, ↓ and ↑, Return and
   Escape by keysym; Q, H and W by the layout's keysym, so they follow the
   letters; ` and Shift+` by the same keysym as the `cycle-windows` chord,
   which P5 matched to the key that types it (left of Z on the UK Mac
   board, as macOS does) rather than the key above Tab the surface sheet
   names. Every other key is eaten. Over a game the chord arrives as the
   game keyboard's Super, as P5 left it, and the same keys act.
6. **Commit goes through the paths that exist.** An app: `activate_app`,
   which unhides it and raises its windows on its desktop with its most
   recent window focused, a minimised-only app's most recent window
   restored from the island. A window from the window row: `activate`. A
   target on another desktop slides there as `switch_desktop` does, cut in
   Game Mode and under Reduce Motion, where `activate` jumps today. A
   background entry relaunches through its desktop entry on P6's launch
   path, which a single-instance app answers by showing its window. Q runs
   `quit_app` (B30, keep-running apps too); on a background entry, the
   polite signal to its processes. H runs `set_app_hidden`. W runs
   `close_window`, so keep-running holds (Apps, hide, quit 8).
7. **Background apps are known by their processes.** A `processes` module
   in the compositor keeps, per app, a pidfd for each pid a window has had
   (client credentials, `_NET_WM_PID`) and the cgroup of each scope Isle
   launched it in, which `launch.rs` now records. An app with no mapped
   window and a live process is a background app; a pidfd's readiness on
   the loop, or the scope's `cgroup.events` going unpopulated, ends it.
   Apps started elsewhere are known only once they have had a window
   (`structure/switcher.md` Which apps). Xwayland, Isle's own processes and
   clients with only layer surfaces are never apps.
8. **Previews are textures the tree draws.** The tree places an
   `isle_ui::WindowPreview` for a window id and reports its size, as glass
   reports its shape. The compositor renders that window's pieces (its
   surfaces and popups, without shadow or title bar) into a texture at that
   size and scale, and hands the tree a Skia image wrapping the same GL
   texture: P3's one Skia context shares the compositor's GL context, and
   Freya's `ImageHandle` takes an `SkImage`, so the preview sits inside the
   tree's own layers with the icon and tags over it, as the sheet draws
   them, and nothing is copied through the CPU. A shown window's preview is
   re-rendered when it commits, only while the switcher is drawn; a hidden,
   minimised or other-desktop window, which P4 suspends and sends no frame
   callbacks, shows its last committed buffer and is not woken. Textures
   are dropped when the switcher closes (v1 D55). Rejected: drawing
   previews above the tree's texture, which puts them over the icon and
   tags; drawing them through Glass's mechanism, for the same reason.
9. **The surface is placed on the focused window's output.** `switcher` is
   an `Overlay`, `Placed` surface centred on the output of the focused
   window, else the pointer's, whatever `shell.main-screen` pins. The
   compositor sizes it from the model's layout; its top edge stays put and
   the window row grows it downward on the island's morph, the tree laying
   out the row's alpha by the spring's progress (B48). The tree draws the
   glass panel, plates, tags, badge, count, progress, name bands and window
   row from `surfaces/switcher.md`, solid and still in Game Mode, Reduce
   Transparency and Reduce Motion as P6's surfaces are. Hover selects only
   on pointer motion; a click commits.
10. **Centre surfaces become a group.** `Shell` gains exclusive groups:
    `MenuBar` (the bar and its menus, Control Centre, the window menu,
    About, the zoom menu) and `Centre` (the switcher now; Spotlight, Mission
    Control, the capture tool, the session dialog, Force Quit and the
    dashboard as their packets come). Opening a `Centre` member closes the
    other `Centre` member and every `MenuBar` one; opening a `MenuBar`
    member closes the others in its group, as P6 does by hand today (B49).
    P6's ad hoc closes in `control_centre.rs`, `menubar.rs` and
    `top_edge.rs` move onto the groups. Cmd+Tab closes them on the press,
    quick tap included.
11. **Attention gains urgency.** The registry's `attention` becomes a level:
    none, normal or urgent. Normal from an xdg-activation request without a
    fresh token, X11 `_NET_ACTIVE_WINDOW` from an app not in front,
    `gtk_surface1.present` (which today posts the island event but leaves
    the record unmarked) and `_NET_WM_STATE_DEMANDS_ATTENTION`; urgent from
    the X11 `WM_HINTS` urgency bit and LauncherEntry's `urgent`. The X11
    properties are read on the worker's own Xwayland connection that P6
    made for menu properties. The island's attention event carries the
    level; both clear when the app is activated.
12. **Badges and progress from LauncherEntry.** The worker matches
    `com.canonical.Unity.LauncherEntry.Update` on the session bus, keeps
    `count`, `count-visible`, `progress`, `progress-visible` and `urgent`
    per desktop entry (`application://<id>.desktop`, resolved through the
    index), merges partial updates, and drops an entry's values when its
    sender leaves the bus. They go to the loop with the rest of an app's
    state and out on `Windows1.Apps` for P10's tiles. P8's notification
    server does not change this.
13. **Games are named from Steam's library in P7.** A new crate,
    `isle-games`, reads Steam's library folders (`libraryfolders.vdf`), app
    manifests (the `name`) and the library cache (a portrait cover, in the
    current per-app folder layout and the older flat one), on the worker,
    re-read when those files change. A `steam_app_<id>` window takes the
    game's name and cover in the switcher, the menu bar and the island's
    events (`gaming.md` Games as apps 1). Heroic, Lutris, naming a
    gamescope window from the game under it, and launching through the
    launchers are the rest of `gaming.md`'s Game library and go to P10,
    where Spotlight lists games, on this crate.
14. **One window order.** The window row and Cmd+` share one order: the
    app's toplevels (dialogs excluded) in the order they opened. Cmd+`
    walks the shown ones and skips minimised ones, as now; the row lists
    them all, minimised and hidden marked, every desktop included, and the
    count beside the icon is its length.
15. **A drag survives the switch.** The drag icon, which nothing draws
    today, is drawn above every layer, overlay included, at the pointer.
    Cmd+Tab during a drag commits and the drag carries on into the new
    front window.

## The workspace

```
crates/
  isle-games/              Steam library folders, manifests, covers (new)
  isle-apps/               badges.rs: LauncherEntry's properties and merging
  isle-ui/                 WindowPreview, the badge, the progress bar, tags
  isle-shell/src/
    switcher/              the model (entries, selection, layout, keys), the
                           tree
    lib.rs                 exclusive groups
  isle-compositor/src/
    switcher.rs            the hold, the key filter, commits, previews, the
                           surface's placement and morph
    processes.rs           pidfds, launched scopes' cgroups, background apps
    wm/apps.rs             attention levels, one window order, stop
    launch.rs              the scope's unit and cgroup recorded
    xwayland.rs            urgency and demands-attention
  isle-ipc/                AppInfo extended; Windows1.StopApp; Test1's
                           switcher readout
  isle-cli/                isle apps list's new columns, isle apps stop
```

`AppInfo` gains `background`, `minimised_only`, `attention` as a level,
`badge`, `progress`, `pids`, `icon` and its recency rank. `Test1` gains
`Switcher()`, the model's state as JSON (entries, selection, window row,
drawn or not), and a clock override for its delay. The test client gains
`activate` (an activation request without a fresh token), `--launcher-entry
count=N,progress=F,urgent` (emits Update over zbus) and `background` (closes
its windows and stays running; `linger` covers part of this today); the X11
test client gains `--urgent` and `--demands-attention`. A fake Steam root
under the test's data directory carries one manifest and one cover.

## Phases

Each phase ends in a check run by script or in the VM and lands as its own
commits.

### 7.0 · Apps, completed

- Attention levels from every source (choice 11), urgency and
  demands-attention on X11, `gtk_surface1.present` marking the record.
- `processes`: pidfds, launched scopes' cgroups, background apps,
  `Windows1.StopApp` and `isle apps stop`.
- One window order (choice 14); `AppInfo` extended; `isle apps list` shows
  background, minimised-only, attention and the window count.

**Check:** headless: an activation without a token marks normal attention
and the X11 urgency hint urgent, both cleared by activating; a client that
closes its window and keeps running is a background app until it exits, and
StopApp ends it; a launched app's scope ends its background entry when
emptied; Cmd+` and the order agree.

### 7.1 · Badges, progress and game names

- LauncherEntry on the worker (choice 12), kept per desktop entry, dropped
  with its sender.
- `isle-games` with Steam's manifests and covers (choice 13); names in the
  switcher's data, the menu bar and the island's events.

**Check:** headless: the test client's Update shows as a badge and progress
on its app, a partial update merges, its exit clears them, `urgent` marks
urgent attention; a `steam_app_<id>` test window takes the fake library's
name and cover; unit tests for the VDF and manifest readers against a real
Steam folder's files copied into fixtures.

### 7.2 · The model

- The `switcher` module: entries, selection, keys to effects, the delay,
  entries changing while held, layout for one app to forty on 1920 and
  3440 wide outputs, the background group, the window row (choice 3).

**Check:** unit tests with a fake clock: the quick tap's commit target, a
second Tab inside the delay, Shift+Cmd+Tab from the end, wrapping, the
selection following its app through a quit and a new app, Q on a background
entry, the layouts the sheet gives (six apps at 1668 × 258; fourteen a row
at the floor on the ultrawide, seven on 1920; fifteen as eight and seven).

### 7.3 · The hold

- The `switcher` and `switcher-back` actions, the key filter (choices 1
  and 5), commits and cancels (choices 4 and 6), the desktop slide on
  commit, exclusive groups with P6's closes moved onto them (choice 10).

**Check:** headless: a quick Cmd+Tab between two test clients moves focus
once and no frame shows the switcher; Q, H and W act on the selected app
and never reach the focused client; Escape leaves focus where it was;
1,000 scripted switches with random holds, some with a window taking focus
mid-chord, each commit on release to the expected app and leave the seat
and both clients with no modifier held (acceptance bar 7); a VT switch and
keyboard removal mid-hold cancel; over a full-screen test game Cmd+Tab
leaves its Space and Escape keeps its pointer lock; Cmd+Tab closes the bar
and Control Centre; P6's top-edge and panel tests still pass.

### 7.4 · The surface and previews

- `isle_ui::WindowPreview` and the texture path (choice 8), the badge,
  progress and tag components.
- The tree from `surfaces/switcher.md`: plates, the name band and its
  state word, background group, window row on the morph, many apps, Game
  Mode, Reduce Motion and Transparency, Increase Contrast, the largest text
  size, accessible names and the selection's announcement; hover and click.
- `isle-preview switcher` with fake entries.

**Check:** headless screenshots against the board's crops (`p1/surfaces-a.html`,
the switcher's frames 1, 3, 5, 8 and 11) within a tolerance; a preview's
pixels match its client's colour; a client recolouring while held updates
its preview; a hidden client's preview is its last buffer and it receives
no frame callback; textures are freed on close (VRAM by `isle shell list`);
the accessibility tree names each plate with its states.

### 7.5 · Drag

- The drag icon drawn above every layer; Cmd+Tab during a drag (choice 15).

**Check:** headless: a test client drags with an icon, Cmd+Tab commits to a
second client, and the drop lands in it.

### 7.6 · The measure, the VM and the host

- Release build at 3440 × 1440 with twenty apps held: the switcher's draw
  and preview updates under 2 ms p95, its textures' VRAM, the shell inside
  its budgets (250 MB, 200 MiB).
- The VM: kitty, a GTK 4 and a Qt 6 app, Chromium, an X11 app; a hidden,
  a minimised-only and a background app; two outputs.
- The host run on tty8 is optional: the user waived P6's and builds on
  without one.

**Check:** the measures in `docs/research/rust-desktop/p7-switcher.md`; the
VM's screenshots.

### 7.7 · Close

- ARCHITECTURE: how switching works (the hold and key filter, the model,
  previews as shared textures, background processes, LauncherEntry, the
  groups).
- DECISIONS for what later packets must keep: the switcher takes no seat
  focus; previews are compositor textures the tree draws; background apps by
  their processes; centre and menu-bar groups; games named from
  `isle-games`.
- `shell.md`'s, `compositor.md`'s and `gaming.md`'s open points; the
  roadmap's P7 and P10 entries (the Game library's rest); `state.md`,
  `progress.md`.

## Not in P7

Force Quit and its dialog (P9); a system prompt set aside by Cmd+Tab (P9);
Spotlight, the Apps grid, its favourites and badges on its tiles (P10);
Heroic, Lutris, gamescope naming and launching games (P10, choice 13);
Mission Control and App Exposé (P16); Game Mode proper, which replaces the
game-in-front flag (P12); Settings › Apps for the keep-running list (P13); a
quit's timeout before log out shows it holding up (P14); protected content,
which no protocol Isle serves yet asks for.

## Depends on, and feeds

- Needs P3 (surfaces, the shared Skia context, glass, springs), P4 (the
  registry, recency, hide, quit, minimise, desktops, snapshots), P5 (the held
  chord, the escape set, inhibitors, the game flag) and P6 (the island's
  attention event and morph, icons, the launch path, the main screen, the
  menu bar's name).
- P9 adds Force Quit to the `Centre` group, lists background apps in it, and
  sets a prompt aside on Cmd+Tab. P10 joins the `Centre` group, shows badges
  on its tiles from `Windows1.Apps`, and extends `isle-games`. P12 replaces
  the game flag that makes the switcher solid and still. P16 draws Mission
  Control's previews through the same texture path.

## Done when

- `cargo test` passes headless: attention levels, background apps,
  LauncherEntry, Steam names, the model's keys and layouts, the hold with
  its 1,000 switches, the groups, previews, the drag.
- In the VM, the switcher reads as the board with real apps, and a quick
  Cmd+Tab draws nothing.

## Risks

- **Skia and GL state.** Wrapping a compositor texture in a Skia image
  needs Skia's cached GL state reset after the compositor draws, which P3
  already does for the tree's own texture; if a borrowed texture misbehaves,
  the preview is copied into a Skia-owned texture on the GPU instead, still
  without the CPU.
- **Preview cost.** Twenty live previews re-rendered on every commit of a
  video or a game could reach the 2 ms budget; previews then update at most
  once per frame each, and a full-screen game's at half rate.
- **Processes that hand off.** A launcher script that execs, or a
  single-instance app whose second launch exits, must not leave a phantom
  background app; the scope's cgroup covers launched apps, and a pid seen
  only before its first window is never tracked.
- **LauncherEntry's reach.** Few apps send it (Electron 44 and later, a few
  GTK apps); an app that sends none simply has no badge, as the sheet says.
- **Steam's cache layout.** Steam moved its library cache to per-app folders
  in 2024 and may move it again; the reader takes both layouts and falls
  back to the app's icon, never failing the card.

## Outcome

Phases 7.0 to 7.6 passed their checks headless and in the VM, with these
differences from the plan:

- **X11 urgency** came from Smithay's own `WM_HINTS` and demands-attention
  requests, not the worker's Xwayland connection.
- **Previews are copied, not borrowed.** The compositor's texture is copied
  on the GPU into an image Skia owns, so it can be freed at once (the plan's
  fallback, taken first). Windows that draw while held are drawn again at most
  four at a time, ten times a second: twenty shm clients redrawing at once
  made a pass of 45 to 68 ms, nearly all of it uploading their buffers.
- **Exclusive groups are compositor functions** (B57), not `Shell` groups,
  and P6's own pairings stay: Control Centre and the bar are states of
  surfaces that never close.
- **Shift alone does not end a hold**; the held chord's Shift is ignored at
  release.
- **The window row opens in place** with no spring; the panel fades in place
  with its own fade-only transition, still in Game Mode.
- **The quitting mark** is Phosphor's `circle-notch`, still: Isle has no
  spinner component yet.
- **Drawn first from the sheet, then to the board.** The first build took
  `surfaces/switcher.md`'s numbers (272-point plates, a `colour-fill` well,
  progress along the plate's foot), which had drifted from the gate-5 board
  the user approved; the user saw it did not look like the design. The panel
  now follows the board: 256-point plates 8 apart holding a 240 × 144 card in
  the window's colour with a hairline edge, the icon off its lower edge,
  22-point round marks, progress inside the card, the name centred under the
  row and each window's title under it in the window row. The sheet was
  brought to the board.
- **Clear glass** (B58). The panel read as an opaque slab: the floor raised
  its whole shape to about 0.82 over light windows for its one line of text.
  The panel is now `glass-clear` and never raised, and the name sits on a
  capsule of the default glass the floor still governs.
- **An app with no id** is named by its process in the switcher as in the
  menu bar (Chromium's first-run dialog had no name).
- **The ` key** is the one that types it, as P5 made Cmd+`, not the key above
  Tab the surface sheet named.

Tests: `cargo test --workspace --features isle-compositor/test-hooks`, with
fifteen headless tests for P7, and one screenshot by eye (`apps.rs`, `switcher.rs`: attention and
urgency, background apps, LauncherEntry, Steam names, the quick tap, the
hold and its keys, the window row, a thousand switches, unplugging, Control
Centre, a full-screen game, the panel's previews and their updates, a drag),
unit tests for the model, LauncherEntry, the VDF reader and the Steam
library, and the bench `switcher_cost`. Measures and the VM in
`docs/research/rust-desktop/p7-switcher.md`.

Left open: the shell test `a_panic_anywhere_in_the_probe_rebuilds_it_with_its_model`
takes about 2.1 s of its 3 s limit, before P7 as after, and fails now and
then when the workspace runs at once.

