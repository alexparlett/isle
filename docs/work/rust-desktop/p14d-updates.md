# P14d · Updates

Planned 2026-11-12 and approved the same day ("lgtm", every
recommendation taken); building. Split from P14c at the user's word, built
ahead of the rest of the switch. 14d.0's evidence
(`docs/research/rust-desktop/updates-2026.md`) revised choices 6, 12 and
13 and added choice 5's target directory.

Isle on the host has no way to update anything. The catalogue lists
Control Centre's Updates module as "Not in this version of Isle."; the
island's Updates glyph has a tier but nothing feeds it; the system menu's
"Software Update…" is a disabled item; Settings › General has About and
Login Items and no Software Update. v1 did all of it (`tools/update.py`,
`Updates.qml`, `restartneeded.py` on `main`), and the host has gone
without: it has 470 repository updates waiting today.

P14d checks for updates, runs the full upgrade without a terminal, tells
the user what it is doing while they get on with something else, and
says when a restart is owed until they restart. Isle's own update is part
of the same upgrade: Isle is a package built from git (B29, B100), and a
new commit on its branch is an update like any other.

What it must get right: nothing but a full upgrade is ever installed; the
run outlives Settings, the compositor and a crash of either; one password
per run; a failure is reported with pacman's reason and never as success;
and a restart owed is said wherever the user looks until they restart.

## What it answers to

- Decisions: B11 (service crates on the worker), B17 (macOS's model),
  B27 (settings through the store), B29 (built from git, installed with
  paru, updated through pacman), B100 (one package named by its commit,
  helpers held open).
- Intent: `platform.md` Updates 1–6 and bar 13; `apps.md` Settings 12 and
  its open point (where the periodic check runs once Settings exits);
  `shell.md` Menu bar 2, Island 10, Control Centre 2, Notifications 8.
- P1: `p1/structure/control-centre.md` and `p1/surfaces/control-centre.md`
  (the Updates module, its page and arc); `p1/structure/island.md` and
  `p1/surfaces/island.md` (the glyph's two forms, the restart reasons);
  `p1/structure/menu-bar.md` and `p1/surfaces/menus.md` (the count, the
  restart line); `p1/structure/notifications.md` ("Update installed" with
  Restart…); `p1/structure/copy.md` (the strings). Settings' Software
  Update page is named in P1 and never drawn: a board here (14d.1).
- Earlier packets: P13 (Settings and its sub-pages), P14a (`isle-git`,
  `isle setup`), P19b (Cleaner's helper under pkexec, the policy form).
- v1: D46 (a restart owed is said), D47 and D81 (updates without a
  terminal), D78 (the run outlives its window), D83 (no partial
  upgrades), D84 and D86 (replaced code must be restarted).
- Evidence: `docs/research/rust-desktop/session-2026.md` §5 (paru's
  `--devel`, `--gendb`, PKGBUILD repositories); 14d.0 adds
  `docs/research/rust-desktop/updates-2026.md`.

## Choices this plan makes

### Scope

1. **P14c splits again: P14d is updates,** built now. P14c keeps the v1
   import, theming, the rest of `isle setup`, the second GPU and v1's
   units. The roadmap's P14c entry loses the update path.
2. **P14d holds:** the check, the run, restart owed, Control Centre's
   Updates module, the island's glyph, the system menu's count and restart
   line, the notifications, Settings › General › Software Update,
   `isle update`, and the `isle setup` step that makes Isle's own package
   updatable through paru.
3. **Left out:** installing or removing single packages (D83); Arch news
   and `.pacnew` files; mirror ranking; flatpak and firmware (fwupd);
   rolling back. Cleaner keeps the package cache and orphans (P19b).

### Isle's own package

4. **Isle is a paru PKGBUILD repository.** `isle-git` is not in the AUR,
   so `paru -Qua` never sees it (the host's paru knows only
   `appimagetool-bin` as a devel package today). `isle setup` gains an
   `updates` step that adds to `/etc/paru.conf`:

   ```ini
   [isle]
   Url = https://github.com/alexparlett/isle
   Path = pkg/arch
   GenerateSrcinfo
   SkipReview
   ```

   and runs `paru --gendb` as the user so the installed commit is
   recorded. paru then counts a new commit on the PKGBUILD's branch as an
   update and rebuilds it inside `paru -Syu`, from Settings or a terminal
   alike. The step is undone by removing the section. `/etc/paru.conf` is
   edited rather than a user file because paru reads only one config and
   a user file would hide CachyOS's `Devel` and `Provides`. Rejected:
   Isle building or installing itself (a second updater beside paru,
   against B29).
5. **The branch is the PKGBUILD's.** `feature/rust-desktop` until P21,
   which moves the PKGBUILD's source to `main`; nothing in Isle names
   the branch. The PKGBUILD builds into `$startdir/target` unless
   `ISLE_TARGET_DIR` says otherwise: makepkg under paru removes `$srcdir`
   before every build, and a cargo target beside the PKGBUILD survives,
   so an update rebuilds only what changed.

### The check

6. **The compositor checks, on the worker** (answers `apps.md`'s open
   point). Five minutes after the session starts, then hourly, on Check
   Now, and after every run: `checkupdates` (pacman's databases synced
   into a temporary copy, so the system's are never partly synced, D83)
   and `paru -Qua` (the AUR, devel packages included). Each line is
   `name from -> to`; the source is Repositories or AUR. paru never lists
   a PKGBUILD repository's devel package there, so Isle's own update is
   found as paru will find it in the run: `isle-git`'s commit in
   `~/.local/state/paru/devel.toml` against `git ls-remote` of its URL and
   branch. No check runs during a run, or while `/var/lib/pacman/db.lck`
   exists.
7. **One setting:** "Check for updates automatically", on by default
   (`updates.check`). Off, only Check Now and the post-run check look.
   Rejected: downloading in the background (pacman's cache is root's;
   a download is half an upgrade).
8. **A failed check keeps the last good list** and shows "Couldn't check
   for updates" with the reason (no network, a mirror's error) in place
   of "Checked {time}"; the next hourly check tries again.

### The run

9. **One command:** `paru -Syu --noconfirm --skipreview --noprogressbar
   --sudo pkexec`, run as the user. paru syncs and upgrades the
   repositories through `pkexec pacman`, then builds the AUR's and Isle's
   updates as the user and installs them through `pkexec pacman -U`.
   Every question takes its default. With paru missing, the command is
   `pkexec pacman -Syu --noconfirm --noprogressbar` and the AUR and Isle
   are not checked.
10. **One password per run.** paru calls pkexec two to four times in one
    run. Isle ships a polkit action for `/usr/bin/pacman`
    (`io.github.alexparlett.Isle.Updates.upgrade`, `auth_admin_keep`,
    "Software Update wants to install updates."), so the first prompt
    covers the rest of the run: polkit keeps an authorisation for the
    process that asked, and every call comes from the same paru. The
    cost: any `pkexec pacman` on the system now keeps its authorisation
    for five minutes for its caller, not for anything else. 14d.0 proves
    the single prompt; if it fails, the fallback is `--sudo` pointing at a
    small Isle shim that holds one pkexec'd root process for the run.
11. **The run is a transient user unit,** `isle-update.service`, started
    through systemd's `StartTransientUnit` with `isle update run` as its
    command. It is no child of the compositor or Settings, outlives both,
    and its whole output is in the journal under its unit. A second
    Install while the unit is active is refused. 14d.0 proves pkexec from
    a user unit reaches the compositor's polkit agent (polkit 128 falls
    back to the user's display session for a process outside one).
12. **Progress is a file.** `isle update run` reads paru's and pacman's
    lines and rewrites `$XDG_RUNTIME_DIR/isle/update.json` whole on every
    change (write and rename): the phase, each package's state (waiting,
    downloading, building, installing, done, removed), `{n} of {total}`
    counted from pacman's `upgrading X...` lines against the transactions'
    `Packages (N)`, `Pkgbuilds (N)` and `Aur (N)` lines (through a pipe
    pacman prints no counters), the last 200 lines, the start and
    finish times, the exit code and the reason. The compositor watches the
    file and publishes it; a compositor that restarts mid-run reads the
    file and carries on showing it. paru runs under `LC_ALL=C.UTF-8`, since
    pacman's lines are translated.
13. **Exit codes decide.** 0 is done; pkexec's 126 and 127, or pkexec's
    "Request dismissed" or "Not authorized" line in a run that fails (paru
    exits 1 when pkexec is refused), are "Permission was not given"; any
    other code is a failure whose reason is the last
    `error:` line pacman or paru printed, else "Stopped with code {n}".
    A line that reads like a complaint is never a reason on a run that
    exits 0. After any run the package states are settled against the
    next check: a package still listed was not updated, whatever the
    lines said.
14. **Stop, until pacman starts.** While paru is downloading, syncing or
    building, Stop ends the unit and the run reads "Stopped". While a
    pacman transaction holds `db.lck`, Stop is disabled with "Can't stop
    while packages are being installed." Rejected: no Stop at all (an
    Isle rebuild takes minutes and is safe to interrupt); Stop at any time
    (a half transaction is the one thing an updater must never leave).
15. **A finished run clears half a minute after its finish;** a failed or
    stopped run keeps its reason until the next run or Check Now
    (`platform.md` Updates 6).
16. **Isle's packages are never asked to restart Isle.** The running
    compositor keeps its build, its helpers held open (B100); the next
    sign-in runs the new one.

### Restart owed

17. **Three reasons, read by the compositor** at start, after each run and
    hourly, from paths under a root for the tests:
    - the running kernel's `/usr/lib/modules/$(uname -r)` is gone: "The
      kernel was updated";
    - `/sys/module/nvidia/version` differs from the installed
      `nvidia-utils` (read from pacman's local database, no process): "The
      graphics driver was updated";
    - `/proc/self/exe` of the compositor or of `isle-session` reads
      " (deleted)": "Isle was updated".
    The state is computed, never stored, so it holds until the restart and
    a manual `pacman -Syu` in a terminal raises it too.
18. **Restart… is the session dialog's restart** for every reason, as P1
    draws it, though "Isle was updated" alone needs only a new sign-in.
    Isle never restarts itself (`platform.md` Updates 5).

### Where it shows

19. **Control Centre's Updates module** as P1 draws it: the badge with the
    count, the arc while installing, the restart-owed form; its page with
    "Up to date" and Check now, the counts by source with "Install
    updates…" (Settings' page), "Installing {n} of {total}", the reasons
    and Restart…. Its `missing` mark goes.
20. **The island's glyph:** `arrow-circle-up` while a run is going,
    `arrows-clockwise` while a restart is owed; waiting updates are not a
    glyph (P1).
21. **The system menu:** "Software Update…" enabled, opening General ›
    Software Update, with the count as a quiet badge in the trailing
    column (`MenuItem` gains a badge); "Restart to finish updating…" in
    the same group while a restart is owed, opening the restart
    confirmation.
22. **Notifications,** from "Software Update": "Update installed" with
    Restart… when a restart is owed, posted when a run ends well and
    Settings is not in front; *added here:* "Couldn't install updates"
    with the reason and "Show Details…", opening the page, for a run that
    fails or is refused. Nothing is posted for updates becoming available:
    the badge and count say it.
23. **Settings › General › Software Update,** drawn at the board, its
    states: up to date with the last check and Check Now; checking;
    updates waiting with the counts by source, "Update Now" and "More
    Info…" (a sheet listing every package with from → to, by source,
    searchable since the host has 470); running with "Installing {n} of
    {total}", the phase, a bar, Stop, and "Show Details" (the last lines
    in a monospaced box); finished; failed or stopped with the reason,
    Show Details and Try Again; restart owed with the reasons and
    Restart…. Under the card, "Check for updates automatically". General
    lists Software Update second, after About. Settings' first open is on
    General (P1).
24. **`isle update`:** `check` (prints the list), `install` (starts the
    run, `--wait` follows it), `status` (the progress as JSON), `stop`.
    `Updates1` on the session bus carries the same: `Check`, `Install`,
    `Stop`, and the list, progress and restart-owed as properties with
    change signals.

### Proof

25. **Headless tests:** `isle-updates` parses recorded `checkupdates`,
    `paru -Qua` and full-run output (14d.0 records them in the VM),
    including a devel package, an AUR build, a failed transaction, a
    refused pkexec and pacman's lock; the progress file is never seen
    half-written; restart owed against fake trees for each reason. In
    `isle-compositor`: the check's schedule, no check during a run, the
    file watched across a compositor restart, the notifications posted
    and their actions, the module's states, the menu's badge and restart
    line, a second Install refused, Stop refused under `db.lck`. A fake
    `paru` (`ISLE_PARU`, as `ISLE_PROCESS_HELPER`) replays recorded runs.
26. **Bars.** The check never blocks the main loop. The progress shown
    is no more than a second behind the file. Killing the compositor
    mid-run leaves the run going and the new compositor showing it.
27. **The VM walk** with real input: a real upgrade of the guest's
    packages from Settings with one password, Settings quit and reopened
    mid-run; a commit to the guest's Isle source found by Check Now and
    rebuilt inside the run; restart owed from the kernel; a refused
    password; Stop during Isle's build; the module, glyph, menu and
    notifications in every state; the journal read after each step.

## The workspace

- `isle-updates` (new): the check's parsers, the run's line reader and
  progress model, `update.json`, restart owed under a root.
- `isle-cli`: `isle update` and its `run` used by the unit; the `updates`
  setup step.
- `isle-compositor`: `updates.rs` (the schedule, the file watch, the
  unit, restart owed); `control_centre.rs` and `control_edit.rs` the
  module; `island.rs` the glyph; `menubar.rs` the item and the line;
  `desktop_notes.rs` two notes; `dbus.rs` `Updates1`.
- `isle-ipc`: `Updates1`.
- `isle-ui`: `MenuItem` gains a badge.
- `isle-shell`: the module's page and the menu badge as the board draws
  them.
- `isle-settings`: `updates.check`.
- `isle-system-settings`: General › Software Update and its sheet.
- `isle-fakes`: the fake `paru` and `pkexec` replaying recordings.
- `data/polkit`: `io.github.alexparlett.Isle.Updates.policy`;
  `pkg/arch/PKGBUILD` installs it.

## Phases

### 14d.0 · The evidence

Done 2026-11-12: every point held but `paru -Qua` finding a PKGBUILD
repository's devel package, which it never does. In the VM, recorded in
`docs/research/rust-desktop/updates-2026.md`:
pkexec from a transient user unit reaching Isle's polkit agent; one
prompt across paru's calls with the action of choice 10; paru's
PKGBUILD repository from a `file://` URL with `GenerateSrcinfo`
finding a new commit; whether paru keeps `isle-git`'s `src/target` so a
rebuild is incremental, and how long one takes; real output of
`checkupdates`, `paru -Qua` and a full run kept as test fixtures; what
pacman prints when `db.lck` is held.

### 14d.1 · The board

Settings' Software Update page in each state and its More Info… sheet,
the system menu's badge and restart line, the failure notification;
drawn from the tokens and published for the user's choice.

### 14d.2 · `isle-updates`

The parsers, the progress model and file, restart owed. Tests.

### 14d.3 · The service

The schedule on the worker, the unit, the file watch, `Updates1`,
`isle update`. Tests.

### 14d.4 · The desktop's surfaces

The module and its page, the glyph, the menu's badge and line, the two
notifications. Tests.

### 14d.5 · Settings

General › Software Update and the sheet as the board draws them, the
automatic-check row.

### 14d.6 · Setup and the package

The `updates` step (the paru section and `--gendb`, `--check`, `--undo`),
the policy file, the PKGBUILD. Tests against a fake root.

### 14d.7 · The bars and the walk

Choice 26's bars; choice 27's walk in the VM.

### 14d.8 · The host and close

Package and install. With the user: `isle setup` for the `updates` step;
the host's waiting updates (470 today, the kernel and NVIDIA among them)
installed from Settings with one password, Settings closed mid-run; restart
owed shown and the restart taken; Isle's own commit found and rebuilt.
Then the decisions from the next free number, ARCHITECTURE's How an update
runs, `platform.md` Updates answered, this sheet's outcome.

## Not in P14d

Single-package installs and removal. Arch news and `.pacnew` handling.
Mirrors. Flatpak and firmware. Downgrades and rollback (P20c's snapshot
providers). Background downloads.

## Depends on, and feeds

- Needs P8 (Control Centre's grid, the worker's services), P9 (the polkit
  agent), P13 (Settings' sub-pages), P14a (`isle-git`, `isle setup`), P19b
  (the helper and policy form).
- Feeds P14c (setup's remaining steps beside `updates`), P20g (the audit's
  Updates line), P21 (the PKGBUILD's branch).

## Done when

- `cargo test` passes headless for choice 25.
- Choice 26's bars hold.
- Choice 27's walk passes in the VM.
- The host run passes with the user.

## Risks

- A partial host upgrade is the worst outcome. The run is pacman's own
  `-Syu`, Stop is refused inside a transaction, and the first host run is
  watched with the user, with a terminal and `pacman` at hand.
- The host's first run is large (the kernel, NVIDIA, Mesa) and sets a
  restart owed by two reasons at once; the compositor must stay up on the
  old NVIDIA module until the restart, as `isle-session`'s skew check
  already assumes.
- paru's output is not an interface. The parser takes only pacman's
  `(n/total)` and `::` lines and paru's `==>` lines, and falls back to the
  phase "Updating" with the last line when a line is unknown.
- An Isle build is long and heavy; a game started mid-run competes with
  cargo. ananicy's rules already cover cargo and rustc on CachyOS.
