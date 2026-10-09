# P10 · Spotlight

Planned 2026-10-09. Reviewed by the user the same day: every
recommendation taken but one, every pick pasted rather than typed (choice
16); one packet; the host run optional.

The roadmap's brief: the launcher with apps, the Apps grid with its
favourites (B19), windows, settings, the front app's menu items (B18),
actions and v1's prefixes and sources. Games: the rest of `gaming.md`'s Game
library on P7's `isle-games` (Heroic, Lutris, gamescope's game, launching
through the launchers, B56), and badges on the Apps grid's tiles. Earlier
packets left P10 more: the `spotlight`, `apps` and `emoji` shortcuts
reserved in P5's table (Cmd+Space, the Launchpad key, Ctrl+Cmd+Space);
Spotlight among the centre surfaces that close the menu-bar group (B57);
Help's menu search, whose index Spotlight's menu items read (P6); one launch
path for everything Spotlight starts (B52); the switcher's raise as the way
a running app is chosen (P7); P5's injected input, through which a pick is
pasted.

P9 left a desktop that can be kept private and can ask, but cannot be
started from. Nothing launches but a chord bound to a command: Cmd+Space
logs "not built yet", there is no grid of apps, an installed game is a name
in the switcher only once it runs, and Heroic and Lutris games are not known
at all. No sum is worked out, no file found, nothing copied is kept, and no
emoji can be entered. P10 closes all of it. What it must get right is the
feel of one field: no key typed after Cmd+Space is lost, the fast sources
answer within the frame of the keystroke, a slower answer never moves the
row the user is about to press, and choosing something gives focus back
before anything is done in it.

## What it answers to

- Decisions: B17, B18 (menu items in Spotlight), B19 (the Apps grid and its
  favourites, badges on tiles), B20 (raising apps, not windows), B21 (picks
  pasted with the app's chord through Isle's own source), B26 (Quick Look in Files),
  B27 (favourites in `settings.kdl`), B30 (Spotlight is not in the escape
  set), B37 (surfaces, keyboard handover), B44 (one focus path), B47 (the
  island's "Copied"), B51 (the main screen), B52 (the launch path), B55
  (running apps), B56 (games in one crate), B57 (centre surfaces), B60 (the
  worker's shape).
- Intent: `shell.md` Spotlight 1–13 and its open point, No Dock 1, 3, 5,
  acceptance bar 6; `platform.md` App index and launching 1–6 (AppImages,
  `DBusActivatable`), Clipboard history, emoji and symbols 1–7 and its open
  points, Notifications and badges 6; `gaming.md` Game library 1–3 and its
  open point, Games as apps; `apps.md` Settings 4 (the row index's format).
- Left to P10 by earlier packets: listed above; `isle-state`'s launch counts
  ("Later packets add theirs"); the shell's text fields, which have no
  clipboard (`clipboard: None` in the embed).
- Design, binding: `p1/structure/spotlight.md`, `competition.md` (Keys,
  Keyboard focus, the one-at-a-time surfaces), `island.md` (Copied),
  `dock.md` (the tile's menu, launch feedback); `p1/surfaces/spotlight.md`,
  `menus.md`; `review/accessibility.md` and `copy.md`; DESIGN.md's Spotlight
  entry.
- Evidence: `spotlight-2026.md` (new; the § numbers below are its sections
  unless another note is named), `isle-v1-inventory.md` §3.8 and §4
  Launcher, `design-v1-lessons.md` §5 Launcher, v1's
  `shell/services/Launcher.qml` and `shell/scripts/games.py`.

## Choices this plan makes

1. **Spotlight is a placed overlay surface that takes the keyboard.** It
   has a text field, so it follows Force Quit's pattern (B37), not the
   switcher's keyless one (B53): `open_placed` on the main screen (B51),
   keyboard focus through the one focus path with the previous holder kept
   for return. A prompt takes focus over it and gives it back; locking
   closes it; Cmd+Tab closes it and switches.
2. **Keys are buffered from the chord, not from the field.** From the press
   of Cmd+Space the compositor keeps every key event the engine would have
   sent to the focused client, and replays them into the surface once its
   field reports focus (acceptance bar 6). The tree is built on open, not
   kept parked: Spotlight draws and runs nothing while closed (shell runtime
   7). If 10.13 measures the first frame later than 100 ms after the chord,
   the tree is kept built and hidden instead, which the surface model
   already allows.
3. **One pure model.** `isle-shell/src/spotlight/model.rs` holds the query,
   the scope, the groups, the selection and the Apps view's grid, and turns
   keys and source answers into effects, as the switcher's and the banners'
   models do. Every rule the sheet states lives there and is tested there:
   the fixed group order, the top hit, the calculation row placed before it
   is filled, a slow answer that lands only below the selected row, Return
   before the fast sources answer acting on the new query's top hit, the
   scope token and Backspace, the last general query kept and a scoped one
   discarded.
4. **Fast sources answer on the loop, slow ones on the worker.** Apps,
   games, actions, Settings rows, windows and menu items are a few thousand
   strings held in memory; they are matched in the keystroke's own frame on
   the loop, well under a millisecond. Files, contents and the calculator
   run on the worker; each answer carries the query's generation and an
   answer to an older query is dropped. Every source is one trait in a new
   crate, `isle-search`, with no compositor in it, so each is tested alone.
5. **Ranking is a tier, then use, then the name.** A match's tier comes
   first: the name starts with the query, then a word in it does, then the
   initials (`vsc` for Visual Studio Code), then the query is inside it,
   then a fuzzy match. Within a tier, use decides: a count that halves
   every 30 days plus a bonus for the last week, kept per result in a new
   `usage` state domain (apps and games by id, actions, Settings rows,
   emoji). Then the name. A shell action wins a tie with an app (v1
   `97bcbf0`). The top hit is the best fast result by that order. v1's
   counts never decayed, so an app used heavily a year ago still led.
6. **Matching: our tiers, `nucleo-matcher` inside the fuzzy tier.** §3.
   The tier test is ours and cheap; nucleo scores only the fuzzy tier and
   gives the matched indices. `frizbee` is faster and maintained but its
   speed matters for millions of names, which Isle never matches in one
   list, and its typo tolerance puts near misses among real matches in a
   list that promises a stable top hit. Help's menu search moves onto the
   same matcher, so the bar and Spotlight agree.
7. **The app index grows in `isle-apps`, not by a new crate.** The hand
   parser gains `GenericName`, `Keywords`, `Categories`, the localised
   `Name` and `Keywords` for the session's language, `Hidden` masking a
   lower entry, `OnlyShowIn` and `NotShowIn` against `XDG_CURRENT_DESKTOP=Isle`,
   `DBusActivatable`, `SingleMainWindow`, and Flatpak's export directories.
   AppImages in `~/Applications` become entries from their embedded desktop
   entry and icon without running them (`platform.md` App index 3, v1 D82).
   `freedesktop-desktop-entry` (§4) is rejected: the parser is small, tested
   and already Isle's, and the crate follows COSMIC's needs.
8. **`DBusActivatable` entries are activated over the bus.** The launch path
   calls `org.freedesktop.Application.Activate` or `ActivateAction` with
   the activation token in `platform-data`, falling back to `Exec`
   (`platform.md` App index 4). Today they start by `Exec`, which works but
   skips the app's own single-instance path.
9. **Choosing a running app raises it, always.** `activate_app(app) ||
   launch(app)`: an app with windows comes forward, as the switcher's
   commit does; one hidden is shown; one running with no window is
   launched again, which a single-instance app answers with a window. This
   keeps B20's app as the unit. v1 revised D63 (`d49d7ae`) to launch a new
   copy of multi-window apps such as file managers; on a Mac that is Cmd+N
   after the raise, and Isle follows the Mac.
10. **Games join `isle-games` from every launcher and every install.**
    Steam, Heroic (`store_cache`'s Epic, GOG and Amazon libraries and its
    sideloaded apps, installed ones only) and Lutris (`pga.db` read only,
    through `rusqlite` on the system's SQLite) are read from both their
    native and Flatpak roots (`~/.var/app/…`), all of them, where v1 took
    the first root found (§9; `gaming.md`'s open point). Steam's covers
    follow its per-game cache folders and their hashed subfolders, and a
    cover the user set in Steam's `userdata/…/config/grid` wins. Heroic's
    covers come from its own image cache or icons; Isle fetches nothing.
    A game is launched through the launch path as its launcher's entry with
    the game's URL as the argument (`steam steam://rungameid/<id>`,
    `heroic heroic://launch?appName=…&runner=…`, `lutris
    lutris:rungameid/<id>`), so it gets a token and a scope like any app
    (B52, `gaming.md` Game library 2). A desktop entry a launcher wrote for
    a library game folds into the game by the URL in its `Exec`.
11. **A running game is raised where its window names it.** `steam_app_<id>`
    and gamescope's title (B56) name the game, so choosing it raises it.
    Heroic's and Lutris's games run under the launcher's process with their
    own app ids, which Isle cannot tie to the library entry; choosing one
    asks the launcher, which refuses a second copy itself.
12. **File names come from Isle's own index.** §1. A walk of home with the
    `ignore` crate, skipping hidden folders and what `.gitignore` files
    exclude, is about 84,000 files and 7,000 folders here and takes 17 ms
    warm; it is held as a parent-pointer tree of a few megabytes, kept live
    with one inotify watch per folder and rewalked when a watch overflows.
    The `spotlight.files` section adds and excludes folders. plocate is
    rejected: its database is readable only through its setgid binary, it
    is a day old at best, and last night's update failed on this machine.
    localsearch is rejected: a GNOME stack the host does not have, which
    skips Git repositories. Recent files for the empty Files scope and the
    one-app scope come from `recently-used.xbel`.
13. **Contents are searched on demand.** In the Files scope with Contents
    on, ripgrep's engine as a library (`grep-searcher` over the index's
    files) runs on the worker 250 ms after the last key, stops at twenty
    files and is cancelled by the next keystroke (v1's numbers, §2). Plain
    text only: v1 reached PDFs and office files through `rga` when
    installed, which P10 leaves out. No content index.
14. **The calculator is `fend-core`, offline.** §5. It has no dependencies,
    is interruptible, and makes no network call of its own; units and
    arithmetic work, currency answers that it needs rates, until a later
    packet adds an opt-in fetch. Evaluated on the worker with an interrupt
    after 100 ms. The general search shows the calculation row when the
    query starts with a digit, a bracket, a minus or a known function, or
    reads as `<number> <unit> to|in <unit>`. v1's `qalc` respawned on every
    keystroke; libqalculate has no maintained Rust binding.
15. **Emoji and symbols are a generated table shipped as data.** A script
    in `data/emoji/` builds `emoji.tsv` from the `unicode-emoji` package's
    `emoji-test.txt` (names, groups, skin-tone variants), CLDR's keyword
    annotations, and UnicodeData's names for the symbol blocks v1 offered;
    the output is checked in and regenerated when Unicode moves. Glyphs the
    colour emoji font cannot draw are left out at load. Recents live in the
    `usage` domain. The `emojis` crate has Unicode 17 but no keywords.
16. **Every pick is pasted, never typed.** A clipboard item, an emoji or a
    symbol reaches the window through one path: a selection Isle owns with
    the pick's types, then the app's paste chord through P5's tables
    (Ctrl+Shift+V in a terminal). Once the app has read Isle's offer, or
    after a timeout, the previous selection is put back, except for a
    clipboard pick, which is meant to become the clipboard; Isle's own offer
    is never recorded in the history. P5's `type_into` (wtype's keymap
    trick, §10) is not used: Xwayland applies a changed keymap late, so X11
    windows get the wrong character or none, and Wayland apps would each
    have to cope with a keymap swapped mid-stream; one path that works in
    every window is worth a clipboard briefly borrowed. This answers
    `platform.md`'s open point on a client with no text-input-v3, and
    revises `shell.md` Spotlight 8 and `platform.md` Clipboard 3 and 7,
    which say typed, at close. Isle serves no input method in P10.
17. **Clipboard history comes into P10 from P20.** The roadmap left it to
    P20, but Cmd+4 is one of Spotlight's four quick keys, the paste path is
    the emoji's, and v1's history is in daily use, so the host would lose it
    for the months between M2 and P20. The compositor keeps it from
    Smithay's selection hook: each new clipboard selection's text and image
    types are read through a pipe on the worker, never on the loop. Not
    recorded: an offer carrying `x-kde-passwordManagerHint: secret`, an app
    on `spotlight.clipboard.exclude` (KeePassXC and Bitwarden by default),
    the primary selection. Kept under `~/.local/state/isle/clipboard/`, mode
    0600: 200 items or 30 days, whichever comes first, items over 8 MB not
    kept, images as files that survive a reboot, pins never evicted
    (`platform.md`'s open points). A pick sets a selection Isle owns with
    the item's types and sends the paste chord. `ext-data-control` is not
    served: Isle's history is the reason for it, and it would hand every
    copy to any client that binds it. The shell's own text fields get the
    clipboard, so Cmd+V works in Spotlight's field.
18. **The vault waits for the providers.** The `*` scope needs the provider
    contract (`platform.md` Providers) and Keychain to unlock in, both in
    P19 and P20. Until then `*` is a character in the query. rbw is the
    first provider to write then (§8).
19. **The Settings source is built; its rows come with Settings.** P10
    defines the row index (`data/settings/index.kdl`: pane, row id, label,
    keywords) and the source that reads it; the file ships empty, and the
    group appears once it has rows and Settings' entry is installed (P13).
    Choosing a row starts Settings at it (`apps.md` Settings 5). Reading the
    settings schema's doc comments instead is rejected: they describe keys,
    not the rows a user sees.
20. **Actions are the built shortcuts and the system's commands.** Every
    entry in P5's table whose packet is built, with its label and chord,
    and the commands that have no chord (Lock Screen, Sleep, Restart…, Shut
    Down…, Log Out, Force Quit…, Dark Mode, each Focus, Game Mode, Do Not
    Disturb), with synonyms in `data/spotlight/actions.kdl` ("shut down":
    power off, turn off). An action shows its chord, never a chord as a
    result (v1 `7b66ce5`). Apps' desktop-entry actions and LauncherEntry
    quicklists, read from the quicklist's DBusMenu with `isle-menus`, join
    the Actions group and the tile's menu.
21. **Menu items read the bar's index.** The front app's tree, crawled as
    Help's search crawls it, matched with the shared matcher, uncapped,
    each with its path and hint; choosing one acts without revealing the
    bar (`menu-bar.md`). The front app is the one in front when Spotlight
    opened.
22. **Run and the web go through the launch path.** `>` runs the text with
    `sh -c` in a transient scope (a new `Launch::Command`), Cmd+Return in
    the user's terminal through `xdg-terminal-exec`; the last 50 commands
    are kept in the `spotlight` state domain. The web row opens the
    `spotlight.web-search` template, DuckDuckGo by default as in v1, as a
    link.
23. **Favourites live in `settings.kdl`.** `spotlight { favourites
    "org.mozilla.firefox.desktop" … }`, written through the store as the
    tile's menu, a drag or Option+Cmd+arrows changes them (B27), so a hand
    edit shows at once. Games are favourites by a game id
    (`steam:570`).
24. **The tile's menu offers only what is built.** Open, the entry's
    actions and quicklist, favourites, Keep running after last window (P4's
    `keep_running`), Assign to (P4's places), Show in {launcher}
    (`steam://nav/games/details/<id>` and the others' pages), Quit while
    running. Open at login waits for Login Items (P14); Show in Files waits
    for an owner of `org.freedesktop.FileManager1` (Files, P15) and shows
    when one is on the bus, as do "Show all in Files", Cmd+Return on a file
    and Quick Look on Cmd+Y. Cmd+Y on a clipboard item is Spotlight's own
    preview and is built.
25. **Long lists are virtual.** The emoji grid has some 3,800 cells and the
    clipboard up to 200 rows with thumbnails; `isle-ui` gains a list and a
    grid that build only the rows in view, on Freya's virtual scroll view,
    with group headers and fixed row heights, used by Spotlight and later
    by Files and Settings.
26. **Drag out is the compositor's.** Dragging a file or clipboard row out
    starts a drag whose source is Isle (Smithay's server-side drag), with
    the file's URI or the item's types; Spotlight hides when the drag leaves
    it and closes after the drop. Dragging tiles within the Apps view is the
    tree's own.
27. **Opening and its places.** Cmd+Space toggles; Ctrl+Cmd+Space opens the
    Emoji scope; the Launchpad key opens the Apps view. The bottom-right
    hot corner is P16's, which calls the same open. Over a game or under
    shortcuts-inhibit the chords are the window's, as P5's table already
    has them. Spotlight joins the centre surfaces both ways (B57): it closes
    the switcher, Force Quit, the session dialog and the menu-bar group,
    and they close it.
28. **The CLI and tests.** `isle spotlight [open|apps|emoji|close]` and
    `isle search <query> [--scope …] --json`, which runs the sources without
    the surface, for scripts and the tests. Test1 gains `spotlight` (the
    model as JSON) and `spotlight_act`, as for the switcher.

## The workspace

```
crates/
  isle-search/           sources as one trait, the tiered matcher, ranking and
                         use, the file index, contents, the calculator, emoji,
                         actions' synonyms, the Settings index (new)
  isle-apps/             the parser's new keys, Flatpak dirs, AppImages,
                         DBusActivatable, quicklists
  isle-games/            Heroic, Lutris, Flatpak roots, Steam's cache folders and
                         grid overrides, launch URLs, entry folding
  isle-state/            `usage`, `spotlight`, the clipboard's index
  isle-settings/         `spotlight` (favourites, files, clipboard, web search)
  isle-ui/               virtual list and grid, tile, result row, scope token,
                         segmented control
  isle-shell/src/
    spotlight/           the model, the surface, the Apps view, emoji grid,
                         clipboard preview, menus
  isle-compositor/src/
    spotlight.rs         open and close, key buffering, the centre group, effects
    clipboard.rs         the selection hook, history, picks, the paste path
    launch.rs            Command, DBusActivatable, games through launchers
    dnd.rs               the server-side drag
  isle-ipc/              Shell1 or Spotlight additions, Test1 additions
  isle-cli/              isle spotlight, isle search
data/
  emoji/                 the generator and emoji.kdl
  spotlight/actions.kdl  actions' synonyms
  settings/index.kdl     the Settings row index (empty)
```

## Phases

Each phase ends in a check run by script or in the VM and lands as its own
commits.

### 10.0 · `isle-search` and the matcher

- The `Source` trait (fast or slow, a query and generation in, rows out),
  the row type, the tiered matcher on `nucleo-matcher`, ranking with use,
  the `usage` domain. Help's search moves onto the matcher.

**Check:** unit tests for the tiers, initials, use and its decay, ties.

### 10.1 · The app index

- The parser's new keys and their locale, `Hidden` masking, `OnlyShowIn`,
  Flatpak export dirs, AppImages read without running, `SingleMainWindow`;
  `DBusActivatable` activation and `Launch::Command` in the launch path;
  categories mapped to the Apps view's names.

**Check:** headless tests with entries under the test's data dirs; an
AppImage fixture; activation against a test service.

### 10.2 · Games from every launcher

- Heroic and Lutris readers, every root including Flatpak's, Steam's cache
  folders and grid overrides, launch through the launch path, folding of
  launchers' desktop entries, Show in {launcher}, gamescope's game.

**Check:** fixtures for each launcher's files; on the host's real libraries
read only (`isle search --scope apps`).

### 10.3 · The model

- Query, scopes, groups, the top hit, the calculation row, selection rules,
  the Apps view's grid and favourites, the one-app scope, effects.

**Check:** model tests for every rule in choice 3, and arrivals in every
order.

### 10.4 · The surface and the field

- `isle-ui`'s virtual list and grid, the result row, the scope token and
  quick keys; the surface on the main screen; opening, key buffering and
  replay, focus return, the centre group both ways; the shell's clipboard.

**Check:** a headless test types "fire" within a millisecond of Cmd+Space
and finds all four letters in the field; Cmd+Tab, a prompt and the lock
over it.

### 10.5 · Fast sources

- Apps and games, actions with synonyms, windows, menu items, the Settings
  index; choosing each: raise or launch, the action, the window, the menu
  item; the "Copied" event.

**Check:** headless: an app launched and raised the second time, a window
raised on another desktop, a test client's menu item activated.

### 10.6 · The Apps view

- The grid, favourites and their writes, the category filter, typing in
  it, badges, the tile's menu, drags within it.

**Check:** headless: favourites added, moved and removed land in
`settings.kdl`; a test client's badge on its tile.

### 10.7 · Files and contents

- The index with its watches and overflow rewalk, `spotlight.files`, recent
  files, the Files scope and Names / Contents, contents on demand, opening
  a file by its default handler; the FileManager1 items when an owner is on
  the bus.

**Check:** a fixture tree: create, rename and delete seen within a second;
a contents match cancelled by the next keystroke.

### 10.8 · The calculator, run and the web

- `fend-core` on the worker with its interrupt, the cheap test, copying the
  answer; run and its history; the web template.

**Check:** model and source tests; a run command's scope seen in
`systemctl --user`.

### 10.9 · Emoji

- The generator and `emoji.kdl`, the grid, variants, recents; the paste
  path (Isle's selection, the app's chord, the previous selection put back
  once read).

**Check:** headless: an emoji and a ZWJ sequence pasted into the test client
and into the X11 test client, with the clipboard as it was afterwards.

### 10.10 · Clipboard history

- The selection hook, reading on the worker, the store and its caps, the
  hint and the exclusion list, pins, delete and clear, the preview, picks
  and the paste path.

**Check:** headless: copies from the test client recorded, a hinted copy
not; a pick pasted into a second client; the store's caps.

### 10.11 · Dragging out

- The server-side drag of a file or clipboard item, hiding on leave,
  closing on drop.

**Check:** headless: a file dropped onto the test client arrives as its
URI.

### 10.12 · The CLI and the one-app scope

- `isle spotlight`, `isle search`; Tab into an app's windows, actions,
  quicklist and recent files.

**Check:** CLI tests; the one-app scope in the model tests.

### 10.13 · The measure, the VM and the host

- Release build at 3440 × 1440: a keystroke to its results' frame for the
  fast sources, the surface's first frame after the chord, the list and
  the Apps view under 2 ms p95, the file index's memory and walk time,
  nothing drawn or timed while closed, the shell inside its budgets.
- The VM: launching kitty, GTK 4 and Qt 6 apps, a raise, a calculation, a
  file and a contents match, an emoji into kitty and xterm, a clipboard
  pick, the Apps view. The host run, on tty8, is where the real libraries,
  Steam's covers and the Launchpad key are seen; it is the user's choice.

**Check:** the measures in `docs/research/rust-desktop/p10-spotlight.md`;
the VM's screenshots.

### 10.14 · Close

- ARCHITECTURE: how Spotlight searches (the sources, fast on the loop and
  slow on the worker, the tiered ranking), how the clipboard is kept, how
  picks reach a window.
- DECISIONS for what later packets keep: sources in `isle-search`; ranking
  by tier then use; Isle's own file index; the clipboard in the compositor
  with no data-control; every pick pasted; games launched through
  their launchers' entries.
- The intent docs' open points; the roadmap's P10 entry, P13 (the Settings
  index's rows), P15 (Show in Files, Quick Look, Show all in Files), P16
  (the hot corner), P19 and P20 (the vault, clipboard history moved out);
  `state.md`, `progress.md`.

## Not in P10

The vault and the `*` scope (choice 18, with the providers in P20 and
Keychain in P19); the Settings index's rows and Settings itself (P13); Quick
Look, Show in Files and "Show all in Files" until Files owns FileManager1
(P15); the bottom-right hot corner (P16); Open at login (P14); currency
rates in the calculator; PDF and office contents; a content index; an input
method or text-input-v3; `ext-data-control`; an assistant row (gate 2);
importing v1's launch counts and pins (P14's import).

## Depends on, and feeds

- Needs P3 (surfaces, keyboard handover, springs), P4 (the registry,
  activation, places, `keep_running`), P5 (the shortcut table, injection
  and the tables' paste chords, the game flag), P6 (the launch path, the
  menu index and Help's search, the island's events), P7 (`isle-games`,
  processes, the centre group, badges), P8 (`isle-bus`, Focus and Game
  Mode as actions), P9 (the prompt and lock over Spotlight, the session's
  actions).
- P13 fills the Settings index. P14 imports v1's counts and pins and adds
  Open at login. P15 owns FileManager1 and Quick Look. P16 adds the hot
  corner. P20 adds the vault through the providers.

## Done when

- `cargo test` passes headless: the matcher and ranking, every source
  against fixtures, the model's rules, keys buffered from the chord, the
  Apps view's favourites, the clipboard's hint and caps, pasting
  into Wayland and X11 test clients, the drag.
- In the VM, Cmd+Space then "kit" and Return opens kitty, and again raises
  it; a sum, a file, an emoji and a clipboard pick work; the Apps view
  shows every installed app with its category.

## Risks

- **The first frame after the chord.** Building the tree on open may take
  longer than a frame; keys are buffered either way, but a late panel feels
  slow. The fallback is the parked tree (choice 2).
- **Pasting where paste is not Ctrl+V.** An app whose paste chord the tables
  do not know gets the wrong one, and a terminal may warn about a paste;
  the VM checks kitty, GTK 4, Qt 6, Chromium and xterm, and a miss becomes
  a row in the translation tables.
- **Reading the clipboard.** A client can offer a type and never write it,
  or write gigabytes. Reads run on the worker with a timeout and the 8 MB
  cap, and the source app's offer is never held open past them.
- **Lutris's database while Lutris writes.** Opened read only with a busy
  timeout; a locked read keeps the last library.
- **Heroic's art.** Its covers are URLs; the image cache's naming is read in
  10.2, and a game without a cached cover shows its launcher's icon.
- **inotify on a large home.** A user who adds a big folder can pass the
  watch limit; the index then falls back to a rewalk every few minutes for
  that folder and says so in `isle search --status`.

## Reviewed

The user reviewed the plan on 2026-10-09 and took every recommendation but
one: choice 16 had typed emoji and symbols into Wayland windows through
P5's keymap trick and pasted only into X11 ones; every pick is now pasted
and nothing is injected. Clipboard history moves into P10 from P20, the
vault stays with the providers, and P10 stays one packet with the host run
optional, as for P6 to P9.

## Outcome

Built 2026-10-09 in phases 10.0–10.14, as planned, with these differences:

- **The compositor owns the model and its keys** (B73), where choice 2 had
  a field in the tree with keys buffered until it took focus: the model is
  made at the chord's press, so a key typed with it is never held anywhere.
- **The emoji table is tab-separated, `data/emoji/emoji.tsv`**, not KDL:
  parsing 273 KB of KDL cost about 600 ms of CPU in debug builds. No CLDR
  keywords: the host has no annotations file, and the generator takes one
  when given.
- **The font's emoji coverage is checked on the loop** the first time the
  Emoji scope searches, through the font manager the shell holds; on the
  worker it held the loop's text layout up behind fontconfig's lock.
- **Every pick is pasted** (choice 16 as revised at the review, B79), the
  clipboard read before and put back after; an app with no paste chord
  (xterm by default) takes no pick.
- **Menus in Spotlight are drawn in its own tree** and take the keys while
  open; the clipboard's Clear asks in a menu rather than an alert sheet.
- **File names match a lower-cased copy first**, which took a search of
  84,000 names from 60 ms to 2 ms; initials no longer find files.
- **Launches start in the home folder**, as from a terminal.
- **Drags out carry no icon yet.**
- **What waits for Files is left out rather than hidden**: the tile menu has
  no Show in Files and the Files scope no "Show all in Files" row; Cmd+Y on a
  file does nothing. Cmd+Return on a file asks FileManager1 already.

Found and fixed on the way: the Pulse client sent an empty cookie when there
is no cookie file (audio failed to connect); a Wayland client could not read
an X11 app's clipboard, and an X11 app could not read the Wayland side's or
Isle's; tests scanned every font for want of fontconfig's cache since they
got a HOME of their own, about 53 MB and a burst of CPU per test compositor.

The measures and the VM are in `docs/research/rust-desktop/p10-spotlight.md`:
the fast sources answer a keystroke in 0.3 ms at p95, every Spotlight draw is
under 2 ms at p95, nothing draws while closed, and the shell stays inside its
budgets. The VM launched and raised kitty, pasted an emoji into it, kept a
copy, found a file written over ssh and answered `isle search`. Not checked:
the host's real game libraries in the grid, the Launchpad key, contents
search and a drag in the guest (both headless). B73–B79 record the
decisions.

