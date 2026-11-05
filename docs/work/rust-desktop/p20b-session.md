# P20b · Session

Planned 2026-11-04; building from 2026-11-05 at the user's word.

P20 is split into seven packets (the user's choice, 2026-11-04), and P20b
goes first at the user's word. Signing in today brings back nothing but the
Login Items. An app that crashes just vanishes. The session dialog leaves out
its "Reopen windows when logging back in" row (P9 choice 22), and the
crash notice says "Isle quit unexpectedly." with nothing about the apps.
Settings has no Login Items page and no row for either restore switch.

P20b brings back what was open, on its desktops, after a sign-in or a
crash; says when an app crashes and offers to reopen it; and gives Login
Items and the two restore switches their places in Settings and the Apps
grid.

What it must get right: a restored app's windows land on the desktops they
left, never on the current one first; nothing Login Items starts is
started twice; log out records the session as it was before the apps
quit, not after; and a crash notice comes only for an app the user was
using, never for a helper process with no window.

## What it answers to

- Decisions: B8 (window places), B10 (the compositor owns state), B20
  (keep running), B27 (settings through the store), B35 (the watchdog's
  crash record), B55 (background apps), B102 (Login Items started by Isle),
  B123 (the interrupted restart).
- Intent: `platform.md` Login Items and session restore 1–6, Supervisor and
  crash records; `compositor.md` Session and robustness 3, 7 and 8;
  `apps.md` Conventions 6 and Settings 18.
- P1: `p1/structure/prompts.md` The session dialog and Crash notice;
  `p1/surfaces/prompts.md` (the checkbox row); `p1/structure/conventions.md`
  For the user 1 and What an app remembers; `p1/structure/settings.md`
  General and Desktop, the List row; `p1/structure/spotlight.md` the tile's
  Options; `p1/structure/copy.md` Crash notice and Log out.
- v1: D53 (by app and desktop; what autostart starts is not started again),
  D62 (what was open comes back after a restart); `isle-v1-inventory.md` §5
  and §12.
- Evidence read 2026-11-04: on the host, `systemd-coredump` is the core
  handler and the user reads their own records with `coredumpctl`. Each
  record's `COREDUMP_CGROUP` names the app's scope
  (`app.slice/app-…scope`), its `COREDUMP_PID` the process, and its
  `MESSAGE` the stack. The host has four user autostart entries, seven
  system ones (gnome-keyring, KDE Connect, at-spi among them), and three
  units the user enabled (`xremap`, `arch-update-tray`, `gcr-ssh-agent`)
  besides `isle-dev-sync`.

## Choices this plan makes

### Scope

1. **P20 splits into seven packets.** P20a windows (PiP, keep on top, focus by direction, popups as `xdg_popup`, P15b's
   compositor cuts); P20b session; P20c providers; P20d calendars; P20e
   system panes; P20f the on-screen keyboard; P20g P15b's remaining cuts
   and the audit against the inventory. Software Update moves to P14c with
   the update path. The roadmap's P20 entry becomes the seven, P20b first.
2. **P20b holds:** the restore record and restore at each compositor
   start; the session dialog's checkbox; Isle's apps honouring "Close
   windows when quitting an application"; the crash notice for apps with
   Reopen and Report…; the Isle notice's "Your apps were reopened.";
   Settings › General › Login Items; Desktop's two Windows rows; the Apps
   grid tile's Open at login. Not `xdg-session-management`: places by app
   already give a window its box, and the content is each app's own.

### The record

3. **A `restore` state domain, written by the compositor.** For each
   running app with a desktop entry: its id, and per window, back to front,
   the output's identity, the desktop's index on it, and whether the window
   was hidden or minimised. Then the front app, whether the next sign-in
   skips, and the checkbox as last left. The record is worked out from the
   window list on every change and written through the state writer only
   when it differs, so a title or a focus change writes nothing.
4. **Apps running without windows are noted too,** with no windows, as
   the switcher and Force Quit count them (B55): an app kept running after
   its last window (B20) comes back the same way.
5. **Left out:** recognised games and PiP (`compositor.md` Places 4), windows with a parent (dialogs, the Open and
   Save panels), untitled windows, and windows of a client with no desktop
   entry, which cannot be relaunched. Full-screen Spaces and Split View
   pairs are not kept: such a window is noted on the desktop its Space sits
   beside, and comes back as a window there at its place.
6. **The session's end freezes the record.** When the dialog's action
   starts asking apps to quit, recording stops, so log out keeps the
   session as it was. A cancelled or interrupted end ("Couldn't restart
   because {app} didn't quit.") starts it again. A shutdown not made through
   Isle leaves the record as of the last change.

### Restoring

7. **Restore runs at each compositor start, a crash's restart included.**
   That is `platform.md` 5 and P1's "Your apps were reopened."; D53's "a
   shell restart replays nothing" was v1's QML reload, not a lost session.
   `compositor.md` Session 7's "on a new session only" is corrected in
   20b.0. It never runs in the greeter, in a safe start (the supervisor's
   quick deaths, so a restored app that crashes the compositor cannot
   loop), or with `ISLE_RESTORE=0`, which isle-tty sets beside
   `ISLE_LOGIN_ITEMS=0`.
8. **After Login Items, by B102's tests.** The compositor reads the
   autostart entries once for both: an app any applicable entry starts is
   skipped whether or not it is still running, and so is an app whose
   launched scope still has processes or whose bus name is owned. The rest
   are launched through the launch path, with no files, back to front.
9. **Windows land where they were as they map.** A launched app holds its
   noted windows as pending: its next windows, in mapping order, take each
   pending output and desktop, then are hidden or minimised again, with the
   box from places as now. Pending ends when every noted window has
   mapped, or two minutes after the launch; later windows open as any new
   window does. A noted output that is absent gives the main screen, and
   the desktop index is clamped to its row.
10. **Focus ends on the front app.** Restored launches' activation tokens
    open their windows behind without taking focus, except the noted front
    app's, so the desktop that comes up is the one the user left, with the
    window in front that was in front.
11. **"Reopen apps when logging in"** is Desktop's Windows row, on by
    default (`windows.reopen-apps`). Off, nothing is noted and nothing
    restored, a crash's restart included. The dialog's checkbox shows only
    while it is on.

### The session dialog

12. **The checkbox is drawn as P1 draws it,** in both forms, "checked as
    last left". Unticked, the record's skip is set when the action
    starts, and the next start restores nothing and clears it. A crash
    before that next sign-in still restores (the skip is for a sign-in
    only).

### Isle's own apps

13. **"Close windows when quitting an application"** is Desktop's Windows
    row, off by default (P1 conventions For the user 1, (a)),
    `windows.close-windows-when-quitting`. `isle-app` reads it, and every
    Isle app that reopens its windows (Files, Settings at its pane,
    Monitor, Keychain, Cleaner) does so only while it is off. Files
    already reopens without asking; it moves onto `isle-app`'s call.
14. **A restored launch reopens windows whatever the switch says,** as
    macOS's "Reopen windows when logging back in" does: the launch path
    sets `ISLE_RESTORED=1` in a restored app's environment, and `isle-app`
    reads it at start. Other apps restore their own content their own way
    (Firefox's session, an editor's workspace).

### The crash notice

15. **Crashes come from `systemd-coredump`'s records.** The worker follows
    `journalctl --follow --output=json` for the coredump message id and the
    user's uid, from the compositor's start on. A record whose cgroup is a
    scope the launch path made, or whose pid had a window (`Processes`),
    names its app. Rejected: exit statuses from the launch path (a
    D-Bus-activated app is not Isle's child, and a scope reports no exit
    code), and the pidfds alone (a quit and a crash look the same).
16. **The notice is for the app, not its helpers.** It is posted when the
    named app has no window and no running process a second after the
    record. A browser's renderer or GPU process dumping core while its
    windows stay posts nothing. One notice per app a minute.
17. **"{app} quit unexpectedly."** with Reopen and Report…, as P1 has it,
    drawn as an Isle notification. Reopen launches the app with its
    windows' last record as pending, so its windows return to their
    desktops (choice 9). Report… writes the record's summary and stack to
    `~/.local/state/isle/crashes/apps/<app>-<time>.txt`, the last twenty
    kept, and opens it as Isle's own Report… opens the compositor's crash
    record.
18. **Isle's apps abort after their panic message.** `isle-app`'s panic hook
    prints the message and calls `abort`, so a panicking Isle app leaves a
    core and a record like any other, and no unwinding runs into Wayland
    objects freed on the way out (P19a's segfault).
19. **The Isle notice gains "Your apps were reopened."** when the start's
    restore launched at least one app.

### Login Items

20. **General › Login Items is two groups, as macOS 13's.** "Open at
    Login": the autostart entries that are apps (their id names a desktop
    entry the Apps grid shows), user and system alike, as a List row with
    + and −. + opens the app picker Focus's allowed-apps list uses and
    writes `~/.config/autostart/<id>.desktop`. − deletes a user entry and
    writes a `Hidden=true` copy over a system one. "Allow in the
    Background": every other applicable entry, and the user units the user
    enabled (the links under `~/.config/systemd/user/*.wants/`), each with
    a switch. Off writes the `Hidden=true` copy, or `systemctl --user
    disable` through the manager's D-Bus. Isle's own units are shown
    locked. An entry for another desktop is shown with its switch disabled
    and "Starts only in {desktops}." Vendor-enabled units (PipeWire and
    the like) are not listed.
21. **The list is live** while the page is shown: an inotify watch on the
    autostart folders and the user unit folder, and a change made by hand
    shows at once. "No login items" when both groups are empty.
22. **Open at login in the Apps grid's tile Options**, checked while the
    app is in Open at Login, toggling the same user entry (P10 choice 24
    left it until Login Items existed).

### Proof

23. **Headless tests** in `isle-compositor` with fake apps (test clients
    with desktop entries in a temporary data folder): a record from a
    window list (left-outs, back to front, hidden and minimised, the
    front app); no write on a title change; the freeze and its undo on a
    cancelled end; restore skipping Login Items and running apps; windows
    landing on their desktops in mapping order, the absent output, the
    clamp, the two-minute end; focus on the front app; the skip and its
    clear; no restore in a safe start. The crash watch reads its records
    from a fake source of JSON lines: a windowed app's record posts the
    notice, a helper's does not, Reopen restores its windows, one a
    minute. `isle-apps`: the Login Items groups, add, remove, hide and the
    other-desktop rule against a temporary home.
24. **Bars.** Restore launches every noted app within one second of Login
    Items being decided. A restored window is never drawn on a desktop
    other than its own (the headless test checks the first frame each
    window is in). A crash notice is posted within two seconds of the
    record.
25. **The VM walk** with real input: log out with the box ticked and
    unticked; restart through the dialog; `kill -ABRT` of the compositor
    with apps open on three desktops of two outputs; `kill -SEGV` of an
    app, then Reopen; a renderer crash in Firefox; the two Desktop rows;
    Login Items' add, remove, a system entry hidden, a unit disabled and
    back; the tile's Open at login.

## The workspace

- `isle-state`: the `Restore` domain.
- `isle-compositor`: `restore.rs` (the record, the freeze, restore at
  start, pending windows), `crashes.rs` (the journal source, matching, the
  notice, Reopen, Report…), changes in `autostart.rs` (one read for Login
  Items and restore), `launch.rs` (restored tokens, `ISLE_RESTORED`),
  `session_end.rs` (the freeze, the checkbox), `desktop_notes.rs` (the
  Isle notice's second line), `spotlight.rs` (Open at login).
- `isle-shell`: the checkbox row in `prompts::session`.
- `isle-apps`: `autostart` gains the groups, add, remove, hide and the
  user units, which read and write files only; enabling and disabling a
  unit goes through the user manager's D-Bus from Settings.
- `isle-app`: the close-windows switch, `ISLE_RESTORED`, the panic hook.
- `isle-files`, `isle-system-settings`, `isle-monitor`, `isle-keychain`,
  `isle-cleaner`: reopening through `isle-app`'s call.
- `isle-system-settings`: General › Login Items; Desktop's two rows.
- `isle-cli`: isle-tty sets `ISLE_RESTORE=0`.

## Phases

### 20b.0 · The note and the record

The roadmap's P20 entry becomes seven packets, with Software Update moved
to P14c; `compositor.md` Session 7 corrected (choice 7); `state.md`,
`progress.md`, README.

### 20b.1 · The record

The `Restore` domain; the record from the window list, its left-outs and
background apps; written only on a difference; the freeze and its undo.
Tests.

### 20b.2 · Restore at start

One autostart read for Login Items and restore; the skips; launches back
to front; pending windows by mapping order with the absent output and the
clamp; restored tokens and the front app; the safe start, the greeter and
`ISLE_RESTORE=0`; the Isle notice's line. Tests.

### 20b.3 · The session dialog

The checkbox in both forms, last left; the skip set on the action and
cleared at the next start; `windows.reopen-apps` hiding it. Tests.

### 20b.4 · Isle's apps

`isle-app`'s switch and `ISLE_RESTORED`; Files, Settings, Monitor,
Keychain and Cleaner reopening through it; the panic hook. Tests.

### 20b.5 · The crash notice

The journal source on the worker; matching by scope and by pid; the
notice after a second with one a minute; Reopen with the pending windows;
Report…'s file and its twenty. Tests on the fake source.

### 20b.6 · Login Items' engine

`isle_apps::autostart`'s groups, add, remove, hide, other desktops and the
user units; the live watch. Tests against a temporary home.

### 20b.7 · Settings and the tile

General › Login Items with both groups, the picker and the switches;
Desktop's two rows; the tile's Open at login. Tests in `pane_tests.rs`.

### 20b.8 · The bars and the walk

Choice 24's bars; choice 25's walk in the VM, the journal read after each
step.

### 20b.9 · The host and close

Package and install with `tools/package.sh`. With the user: a day's work
logged out and back in with the box ticked; once unticked; a restart; an
app killed with `SIGSEGV` and reopened; Login Items read through against
the host's entries (choice 20's evidence). Then the decisions from the next
free number, ARCHITECTURE's How a session comes back, this sheet's outcome.

## Not in P20b

`xdg-session-management` and windows' content for apps that are not
Isle's. Full-screen Spaces and Split View pairs restored as they were.
macOS's ⌥⌘Q (Quit and Keep Windows). A crash notice where
`systemd-coredump` is not the core handler, or for a process that exits
without a core. Restoring a game. Vendor-enabled user units in Login
Items.

## Depends on, and feeds

- Needs P4 (places, hide, minimise), P6 (the launch path), P7 and P9 (the
  app registry, the session dialog, Force Quit's background apps), P10 (the
  tile's menu), P13 (Settings, the List row), P14a (Login Items' engine),
  P16 (desktops per output, full-screen Spaces).
- Feeds P20a (PiP joins choice 5's left-outs) and P14c (v1's import has no restore record to carry: v1's `reopen.json` is
  Isle's own windows only).

## Done when

- `cargo test` passes headless for choice 23.
- Choice 24's bars hold.
- Choice 25's walk passes in the VM.
- The host run passes with the user.

## Risks

- An app that maps its windows in a different order from before (several
  windows of one browser) puts them on each other's desktops. The order is
  the app's; a window's title is not stable enough to match by.
- An app slow to start (Steam's update check) can pass two minutes before
  its window, which then opens on the current desktop.
- `journalctl` lags a record by its flush, and a host whose coredumps go
  elsewhere posts nothing (choice 15's rejection keeps one source).
- A restored app that brings up a sign-in or an update window first gives
  that window the first pending place.
