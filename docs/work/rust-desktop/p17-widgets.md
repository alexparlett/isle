# P17 · Widgets

Planned 2026-10-28. Reviewed by the user the same day: every
recommendation taken but sources, which use gitoxide (choice 22).

The roadmap's brief: widgets where P1 places them, with the grid editor and
the gallery (B24); the provider contract (B14), sandboxed with granted
permissions (B28); the isle-widgets registry as providers.

P1 designed all of it. `p1/structure/widgets.md` and
`p1/surfaces/widgets.md` fix the dashboard: its backdrop and the island over
it, the grid of 72-point cells per screen, the twelve built-ins in every
size, the seven provider templates and their states, edit mode with the soft
push, the gallery's three sections, the consent panel, grant rows, the
provider row every provider surface shares, every key, string, token and
motion. `wallpaper.md` adds the wallpaper's menu, `dock.md` the Widgets
corner, `settings.md` the Desktop and Privacy & Security rows. What is open
is how it is built, and the three points `shell.md` left for this packet:
the sandbox and the permission vocabulary, which of the registry's QML
widgets become built-ins, and interactive widgets.

Today nothing of the dashboard exists but its chord (reserved, Ctrl+Option+
Cmd+W), its tokens and the softened wallpaper the lock already draws.
Control Centre's edit mode is the nearest precedent for the grid and the
gallery. Of the built-ins' data, media, batteries, disks, desktops, session
actions, Energy Mode and notifications are already in the compositor's
services; system sampling, weather, a timer and calendars are not. There is
no sandbox, runner, manifest parser, grant store or git client.
`docs/research/rust-desktop/widgets-2026.md` has the survey and the sandbox
probe.

What P17 must get right is the trust line: a provider is someone else's
program, run with the user's files, buses and network a switch away, so
what it can reach must be exactly what the user granted and nothing Isle
itself relies on. And the dashboard is a place the user goes to and leaves:
it must open on the frame after the chord with the last data drawn, and
run nothing at all while closed.

## What it answers to

- Decisions: B14 (built-ins, extended by providers), B17, B21 (the chord
  before Cmd translation), B24, B25, B27 (writes refused while the file
  does not parse), B28 (the sandbox, granted permissions, v1's trust), B30
  (not in the escape set), B33 (plates on glass, solid over plates), B37
  and B39 (surfaces, glass is the compositor's), B53 and B57 (centre
  surfaces), B60 (services), B73 and B113 (the compositor owns a surface's
  model and keys), B94 and B97 (Settings' rows), B99 (the corner engine),
  B102 (scopes).
- Intent: `shell.md` Widgets 1–9 and its open points, Shell runtime 2, 3,
  7, 8, 11, 14 and 16, the hot corners 8; `platform.md` Providers 1–9 and
  its open point on the sandbox; `apps.md` Settings' Desktop and Privacy &
  Security panes; `compositor.md` B24's line (Show Desktop moves, the
  dashboard does not).
- Design, binding: `p1/structure/widgets.md`, `p1/surfaces/widgets.md`;
  `wallpaper.md` (the softened form, the menu), `lock.md` (the wash, the
  plates), `island.md` (the privacy form and OSDs over it, the Timer
  activity), `control-centre.md` (the modules, Now Playing, Batteries),
  `notification-centre.md` (the event rows, the month), `competition.md`,
  `dock.md` (the corner), `prompts.md` (the consent frame, the armed
  moment), `conventions.md`, `copy.md`; `structure/settings.md` and
  `surfaces/settings.md` (Desktop, Privacy & Security, the Hot Corners…
  sheet, Keyboard Shortcuts); `review/` where it names widgets.
- Evidence: `widgets-2026.md` (new), `isle-v1-inventory.md` §6 and §8,
  `design-v1-lessons.md` § Dashboard and widgets, the registry at
  `~/Development/isle-widgets`.

## Choices this plan makes

### Scope

1. **One packet, the author tools last and cuttable.** The dashboard, its
   built-ins, the runner and the gallery share one model of a widget (its
   manifest, sizes, settings and data) and one grid, so they are built
   together. Yours' author tools (New widget…, Check widget, Update
   picture, Tag release…) are used by whoever writes providers and touch
   nothing else; they are the last phase and can move to P20 if the rest
   runs long, the user's call at the time. `isle widgets check` and `run`
   on the CLI are built earlier regardless, because the tests need them.
2. **No gate.** P1's sheets are complete for every surface P17 draws. The
   few rows they name without laying out (Settings › Desktop's Widgets
   group: "Edit widgets…" and "Reset to default", the Providers page's
   order) are built in P1's terms and listed in the outcome. A gap found in
   building gets the smallest fix in P1's terms.
3. **Built-ins whose data comes later are built now, with their later
   parts marked.** Calendar is built with its "No calendars" state and
   Notification Centre's rows, its data from P20's calendars; until then
   "Add Calendar…" is left out, since Settings › Calendars does not exist,
   and the widget stays in the default layout as P1 put it. System's click
   to Monitor waits for P19; until then its body opens nothing. Neither
   holds P17 open.
4. **The registry's six QML widgets become built-ins or parts of them**
   (P1's proposal, `structure/widgets.md` answer 10): analog-clock the
   Clock's face, system-rings a System style, transport the Media widget,
   battery the Devices widget, focus-timer the Timer, uptime a System
   series. The registry carries providers only (B14).

### The dashboard

5. **The compositor owns the dashboard's model and keys**, as B73 for
   Spotlight and B113 for Mission Control: a pure model in
   `isle-compositor/src/dashboard/` (the layout per screen, edit mode, the
   selection, the drag, undo) with the surface drawing a frame of it. Keys
   reach it from the compositor's mode chain, so E, the arrows and the edit
   keys act on the frame after the press whatever the surface is doing.
   The grid's arithmetic (packing, the soft push, resizing within sizes,
   nudges) is a pure module with its own tests, as Mission Control's
   layout was.
6. **The backdrop is the compositor's pass, as the lock's.** One surface,
   `dashboard`, on the Overlay layer, Placed on one output, taking the
   keyboard. The compositor draws, front to back: the island's privacy form
   and OSDs, the dashboard surface, `material-lock-wash`, the softened
   wallpaper (`softened_elements`) and a floor, following `render_locked`.
   The windows beneath are not drawn on that output while it is open, so
   twenty windows cost nothing behind it.
7. **One overlay at a time in one place.** The dashboard is the seventh
   opener that must close the others by hand. P17 moves the rule into one
   function, `close_overlays(except)`, which Spotlight, Mission Control,
   the switcher, Force Quit, the session dialog, the capture toolbar and
   the dashboard call, with each opener's tests unchanged.
8. **Data drawn first, then refreshed.** Each widget's last data is kept in
   a state domain (`widgets`) and in memory, drawn in the opening frame;
   fetches and samples start on that frame and stop on closing. Nothing
   samples, fetches or runs while closed but a running timer (choice 15).
9. **The layout and each widget's settings are in `settings.kdl`**, as P1
   says, so a hand edit, Reset and the store's refusals apply to them:
   `dashboard-output["<identity>"] { widgets "clock#1 11 6 2 2" … }` keyed
   by the monitor's identity as `wallpaper-output`, and
   `widget["clock#1"] { … }` for an instance's settings. A provider's
   settings are known only from its manifest, so `isle-settings` gains an
   open section: a keyed map whose values are any `Value`, checked against
   the manifest when the dashboard reads them; the built-ins declare theirs
   in manifests too (`data/widgets/`), so one form and one section serve
   both (changed in 17.1 from typed sections per built-in), with a value that fails shown as its default and written
   back only by an edit. Last data, grants' history and source pulls are
   state, not settings.
10. **A panic rebuilds the surface open**, in edit mode if it was
    (`structure/widgets.md` Error). Per-widget containment is not built: a
    provider cannot panic the shell, since its output is data parsed on
    the worker, and a built-in's panic is a bug in Isle. The model lives
    in the compositor, so the rebuild loses nothing.
11. **The wallpaper's menu** is built here (`wallpaper.md` Pointer): a
    secondary click on the wallpaper opens "Wallpaper Settings…" and "Edit
    widgets" as a compositor menu at the pointer, the same component as the
    title bars' menus.

### The built-ins

12. **`isle-sysinfo`, a sampler shared with Monitor (P19).** CPU per core
    and total from `/proc/stat`, memory from `/proc/meminfo`, network from
    `/proc/net/dev`, uptime from `/proc/uptime`, temperatures from hwmon,
    the GPU through NVML loaded at run time (`nvml-wrapper`, no link to
    the driver; absent, no GPU series) and sysfs on amdgpu and i915. It
    samples every two seconds while something listens and keeps a minute
    of history for graphs. Monitor builds on it rather than a second one.
13. **Weather fetched by the host** (v1 D31): Open-Meteo's forecast on the
    worker through `reqwest` (already in the tree), no key, cached in the
    `widgets` domain, refetched while shown when older than fifteen
    minutes. The place defaults to the time zone's city with its tzdata
    coordinates (`isle-sun`); another place is chosen through Open-Meteo's
    geocoding in the widget's settings.
14. **Storage lists every mounted volume** with used and free: the
    system's own filesystems (`/`, and `/home` where it is another disk)
    and each volume UDisks has mounted, `statvfs` read on the worker when
    the widget is shown; a filesystem mounted twice, or as btrfs
    subvolumes, counts once. `isle-disks` is unchanged (changed in 17.5
    from adding the system volume there, which every other reader of it
    would then have shown).
15. **A timer service in the compositor.** A timer's label, duration,
    state and end are kept in the `widgets` domain, so a running timer
    survives a shell rebuild and a compositor restart; it posts the
    island's Timer activity and its timer-end event, which until now only
    `Test1` could. The widget is its control; the island shows it with the
    dashboard closed.
16. **Controls draws Control Centre's own tiles.** The panel's `Tile` and
    its actions move to a shared module in `isle-shell`, so the widget and
    the panel draw one component with one state and send the same
    actions, never a second set (`control-centre.md` Shared places).

### Providers

17. **`isle-providers`: one runner for every kind.** The manifest, the
    sandbox, the runner and the grants live in one crate on the worker,
    serving widget providers now and P20's vaults, VPNs, snapshots and
    phones later (`platform.md` Providers 9). A widget provider is a
    folder: `widget.json` and its program. It runs on an interval (once,
    its stdout one JSON object) or streaming (newline-delimited objects,
    each replacing the last), receives its instance's settings and size as
    JSON on stdin, and is started when its widget is shown, stopped when
    it is not, and restarted with a doubling backoff if it dies, left
    stopped after five deaths in a minute with its last error shown.
18. **The sandbox is bubblewrap, with two proxies** (B28 settled;
    `widgets-2026.md` §1). Landlock cannot express hosts, bus names or
    commands. Each run is `bwrap --unshare-all --die-with-parent
    --new-session --clearenv` with `/usr` read only, `/usr/bin` holding
    only the interpreter and the commands granted, the provider's folder
    read only, its own state folder writable, a private `/tmp`, and:
    - **files**: each granted path bound read only or read and write;
      `~/.config/isle`, `~/.local/state/isle` and every provider folder are
      never bound writable, whatever is declared;
    - **buses**: `xdg-dbus-proxy` with `--filter --talk=<name>` for each
      granted name, its socket bound in as the session (or system) bus;
    - **network**: none but loopback. The runner serves a filtering HTTP
      and CONNECT proxy on a Unix socket bound in, allowing granted hosts
      on 80 and 443; `isle-provider-init`, the sandbox's first process,
      relays `127.0.0.1` to it and sets the proxy variables. A program
      that ignores them has no network, as with a revoked grant;
    - **limits**: a transient user scope per provider with a memory cap,
      the call's time limit and output cap, the process group killed past
      them (`platform.md` Providers 4).
19. **The permission vocabulary is the five grant rows' words.** In the
    manifest: `connect` (hosts), `read` and `change` (paths, `~` allowed),
    `bus` and `system-bus` (names), `run` (command names). Each maps to one
    row in P1's words ("Connect to api.open-meteo.com", "Read ~/Documents",
    "Read and change ~/Notes", "Use org.kde.kdeconnect", "Run nmcli") and
    one sandbox rule above. v1's service permissions (`audio`, `media`,
    `system`, `power` and the like) have no counterpart: a provider reaches
    a service through its bus name, and what Isle's own services show is
    for the built-ins. v1's `fetch:<host>` becomes `connect`, `notify`
    becomes `bus: org.freedesktop.Notifications`, `store` is every
    provider's own state folder.
20. **Providers are display only.** A provider draws one of the seven
    templates and acts on nothing; a click on its body does nothing and its
    menu holds Try Again, Edit…, sizes and Remove. Interactive providers
    would need the actions of the provider contract (`status`'s actions)
    drawn on a widget and a way to run them without a window; macOS stops
    at buttons and toggles that run one intent (`widgets-2026.md` §5).
    That is a later packet's if a provider wants it; the built-ins are
    interactive as P1 drew them.
21. **Grants are settings, refused at the edges.** Grants, source trust and
    sources are in `settings.kdl` (`provider["<id>"] { granted … }`,
    `widget-source["<url>"] { allowed #true }`), so Settings › Privacy &
    Security › Providers is the one place they change and a hand edit is
    the user's. A grant naming a path the runner never binds (choice 18)
    is shown as Not allowed with the reason.

### The gallery and sources

22. **Sources through gitoxide** (`gix` 0.90, the user's choice at review)
    on the worker's blocking pool, with its reqwest and rustls transport,
    the HTTP stack already in the tree: a bare shallow clone (depth 1)
    under `~/.cache/isle/widget-sources/`, fetched hourly and on opening
    Browse, with a time limit; a failed fetch keeps the last good index
    (`structure/widgets.md` Error). Nothing is checked out: the index and a
    widget's folder are read from the fetched commit's tree, and installing
    writes that folder's blobs into staging under
    `~/.local/share/isle/widgets/` and swaps it in whole, leaving
    `.isle-install.json` (source, version, commit); updates compare
    semver. No `git` process and no package dependency on it. Yours' Tag
    release… (17.14) uses `gix` too: init, a tree built from the folder,
    a commit and a tag.
23. **The registry's index for Isle 2 sits beside v1's.** v1 is the daily
    desktop until M2 and its Browse reads `index.json`; changing that file
    would show providers v1 cannot load. Isle reads `providers.json` at the
    registry's root (`{name, format: 2, widgets: [{id, path}]}`), and
    `index.json` and the QML widgets stay untouched until P21 removes
    them. Three providers prove the contract and the trust line, each
    exercising a different grant:
    - **Moon**: the phase and its illumination as a Number, no
      permissions, interval;
    - **Hacker News**: the top stories as a List, `connect`
      hacker-news.firebaseio.com, interval;
    - **Repositories**: the git state of each repository under a folder as
      a List, `read` that folder and `run` git, interval.
    Changes to the registry are made in `~/Development/isle-widgets` and
    pushed by the user.
24. **The user's own providers** live in `~/.config/isle/widgets/<id>/`
    (v1's place), noticed by inotify without a restart (`platform.md`
    Providers 8). Their declared permissions start ungranted (`shell.md`
    Widgets 8).

### Proof

25. **Tested headless, walked in the VM, used on the host.** Headless: the
    grid model (packing, the push from the pre-drag layout, refusal at the
    edge, resizing, nudges, undo) by unit tests; the dashboard opened by
    every route, its keys and drags through `Test1.dashboard()`, the island
    over it, the one-overlay rule; each built-in's frame at each size; the
    runner against fake providers: interval, stream, death and backoff,
    time limit, output cap, a size and settings on stdin; the sandbox
    denying each ungranted file, bus, host and command and allowing each
    granted one; install, update with a new permission, a source turned
    off, a pull that fails. Nothing running while closed: no child
    process, no sample, no fetch. The VM: the walk of
    `structure/widgets.md`'s Keyboard, Pointer, States and Copy sections by
    real input (P15b's lesson: screenshots missed what a walk finds), the
    three registry providers installed from a local clone. The host, the
    user's: a day with the dashboard on the ultrawide and the 1080p screen.
26. **Budgets.** The dashboard's content draw within bar 5 (≤2 ms p95) with
    the default layout and a full canvas; open to first frame on the frame
    after the chord; the shell within its 250 MB RSS with the dashboard
    open; a provider's start (bwrap, the proxies, the program) measured,
    with the proxies shared across a provider's runs rather than started
    per call.

## The workspace

```
crates/
  isle-providers/         the manifest, templates, sandbox, runner, grants,
                          the network proxy (new)
  isle-provider-init/     the sandbox's first process: relay, child (new)
  isle-sysinfo/           the sampler (new; Monitor's in P19)
  isle-compositor/src/
    dashboard/            the model, the grid, keys, drag, undo (new)
    dashboard/sources.rs  gix sources, staging, versions (new)
    timer.rs              the timer service (new)
    weather.rs            Open-Meteo on the worker (new)
    overlays.rs           close_overlays (new)
    render.rs             the dashboard's pass
    wallpaper/menu.rs     the wallpaper's menu (new)
    hot_corners.rs        Widgets
    binds.rs              the dashboard action
    dbus.rs               Shell1, Widgets1, Test1
  isle-shell/src/
    dashboard/            the surface, widgets, gallery, panels (new)
    island/tile.rs        the shared module tile (from panel.rs)
  isle-ui/                the grant row, the provider row
  isle-disks/             the system volume, used and free
  isle-settings/          open sections; dashboard, widget, provider,
                          widget-source; Corner::Widgets
  isle-state/             widgets: last data, timers, pulls
  isle-system-settings/   Desktop's Widgets group, Privacy & Security ›
                          Providers, the corner, the shortcut row
  isle-cli/               isle widgets
data/
  settings/index.kdl      the new rows
  spotlight/actions.kdl   Widgets, Edit widgets
  widgets/                the built-ins' manifests and pictures
pkg/arch/PKGBUILD         bubblewrap, xdg-dbus-proxy
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 17.0 · The note and the record

`docs/research/rust-desktop/widgets-2026.md` (written with this plan); the
roadmap's P17 entry; `shell.md`'s and `platform.md`'s open points answered
by choices 4, 18, 19 and 20.

### 17.1 · Settings and state

`isle-settings`' open section; `dashboard-output`, `widget`, `provider`,
`widget-source`; `Corner::Widgets`; the `widgets` state domain. Store tests
for the open section's splicing and refusals.

### 17.2 · The grid

The pure grid: cells per screen from P1's extent, packing, the default
layout, adding at the free place nearest the top centre, the soft push
from the pre-drag layout cascading in the drag's direction, refusal at the
edge, resizing within sizes, Cmd and Option+Cmd nudges, undo and redo.
Unit tests from P1's board scenes 5 and 7.

### 17.3 · The dashboard opens

`close_overlays`; the surface and its pass (choice 6); the routes: the
chord, `isle widgets open`, `Shell1`, the Widgets corner, Spotlight's
Widgets and Edit widgets, the wallpaper's menu; open and close motion and
its Reduce Motion and Game Mode forms; focus taken and given back; the
island's privacy form and OSDs over it; locking closes it. Built with two
widgets (Clock, Session) to prove the frame.

### 17.4 · Edit mode

Cells, badges, handles, select, drag with the ghost and the push, resize,
remove, keys, the settings popover from `settings.md`'s control types,
undo; edits written as they are made.

### 17.5 · Built-ins on data that exists

Clock with its faces and zones, Media, Devices, Storage (with `isle-disks`'
system volume and figures), Desktops, Session, Notifications, Controls
(the shared tile); the one-line forms, empty strips and "{n} more"; last
data drawn first.

### 17.6 · Built-ins on new data

`isle-sysinfo` and System with its editor; Weather on the worker; the timer
service, the Timer widget and the island's Timer activity; Calendar with
its empty state (choice 3).

### 17.7 · The runner and the sandbox

`isle-providers`: the manifest and its checks, the templates' JSON,
interval and stream, start on shown and stop on hidden, backoff, limits;
the sandbox with its two proxies and `isle-provider-init`; `isle widgets
check` and `isle widgets run`. Tests for every grant allowed and denied.

### 17.8 · Providers' widgets

The seven templates and their states (waiting, stale, failing, not
allowed, not installed); the user's folder watched; Try Again.

### 17.9 · The gallery

The sidebar, search, Installed with its groups and live pictures, adding by
click and by drag, "Added", "No room", Details.

### 17.10 · Sources and installing

Sources through `gix`, `providers.json`, Browse, staging and swap, updates by
version and their count, the consent panel with its armed moment for
install, new permissions and one's own, Sources with its switches and a
failed pull.

### 17.11 · Grants

Settings › Privacy & Security › Providers with a page per provider; the
grant rows read only with "Change…" in Details; Allow… from a widget; a
revoked grant denied in the sandbox while the provider runs.

### 17.12 · The registry

`providers.json` and Moon, Hacker News and Repositories in
`~/Development/isle-widgets`, each with its README, CHANGELOG and picture;
`index.json` and the QML widgets left for v1. Pushed by the user.

### 17.13 · Settings and the record

Settings › Desktop's Widgets group (Edit widgets…, Reset to default), the
Hot Corners… sheet's Widgets, the dashboard's row in Keyboard Shortcuts,
`index.kdl`'s rows, the package's dependencies; ARCHITECTURE's How widgets
and providers run.

### 17.14 · Yours (cuttable)

New widget… from a template opened in the user's editor, Check widget,
Update picture (the widget's rectangle shot to `screenshots/card.png`),
Tag release… (through `gix`: init, commit, tag `v<version>`), Show in
Files.

### 17.15 · Measures and the walk

Bar 5 with the default layout and a full canvas on a 3440×1440 headless
output; the provider's start; RSS with the dashboard open; nothing running
closed. The walk of the sheet's Keyboard, Pointer, States and Copy in the
VM by real input, written in `docs/research/rust-desktop/p17-widgets.md`,
its gaps built.

### 17.16 · The host and close

The host run, the user's, on the installed package; decisions recorded;
the intent docs' open points answered; the outcome.

## Not in P17

Interactive providers (choice 20). Calendars' data and "Add Calendar…"
(P20). System's click to Monitor (P19). P20's provider kinds (vaults,
VPNs, snapshots, phones) on the runner. Widgets anywhere but the dashboard
(P1: not in Notification Centre, not on the desktop). Pages (P1 declined).
A QML or WASM widget runtime (B14). v1's layouts and grants imported
(P14b's import, if at all: v1's 12 × 8 grid and service permissions do not
map). Removing the registry's QML widgets (P21).

## Depends on, and feeds

- Needs P3 (surfaces, transitions, glass, panic boundaries), P5 (the
  shortcut table), P6 (the island's privacy form, Control Centre's
  modules), P8 (services, notifications, media, batteries, disks), P9 (the
  lock, prompts' frame and armed moment, the session's end), P10
  (Spotlight's actions), P13 (Settings' panes, the corner engine,
  `isle-app`), P14a (the package, scopes), P15 (Show in Files), P16 (the
  corner list, Desktops' data).
- Feeds P19 (Monitor on `isle-sysinfo`), P20 (the provider kinds on
  `isle-providers` and the Providers page; calendars into Calendar and
  "Add Calendar…"), P21 (the registry's v1 index and QML widgets removed).

## Done when

- `cargo test` passes headless with the grid, dashboard, built-in, runner,
  sandbox and store tests, and every existing overlay test unchanged under
  `close_overlays`.
- A provider reaches exactly what it was granted: each ungranted file,
  bus name, host and command is denied in a test, and Isle's own config
  and state are never writable from one.
- Nothing runs or samples while the dashboard is closed but a running
  timer.
- The dashboard stays within bar 5 with a full canvas on the ultrawide.
- The walk of `structure/widgets.md` passes in the VM by real input, with
  the three registry providers installed from a source.
- Every Widgets row of `shell.md`'s Parity map works.
- On the host, the user opens the dashboard by its chord on both screens,
  edits a layout, installs a provider from the registry and revokes one of
  its grants, on the installed package.

## Risks

- **The network proxy.** Programs that ignore the proxy variables get no
  network; most runtimes honour them, but a provider author will hit one
  that does not. The author contract says so, and `isle widgets run`
  prints the denied connection so it is found at once, not as a silent
  failure.
- **bubblewrap's start per call.** An interval provider runs once per
  interval; bwrap, the bus proxy and the relay add a few milliseconds
  each. 17.15 measures; the proxies stay up for a provider's whole showing
  rather than per call.
- **The open settings section.** A free-form map in a typed store is new;
  if splicing arbitrary keys proves fragile, instance settings move to a
  state domain and only the layout stays in `settings.kdl`.
- **NVML in the compositor's worker.** A driver library loaded into the
  session's most important process; if it misbehaves, `isle-sysinfo`'s GPU
  part moves to a helper process, as `isle-thumbnail` did for decoders.
- **The size.** Twelve built-ins, a runner, a sandbox, a store and a
  gallery. Yours is cuttable (choice 1); if more is needed, Browse and
  sources (17.10) can follow in their own packet with the user's own
  providers still working.

## Reviewed

The user reviewed the plan on 2026-10-28 and took every recommendation
but one: sources are fetched and read through gitoxide (`gix`) rather
than the `git` CLI (choice 22), with nothing checked out and no package
dependency on `git`.

## Outcome
