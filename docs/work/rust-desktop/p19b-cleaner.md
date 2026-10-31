# P19b · Cleaner

Planned 2026-10-29. Reviewed by the user on 2026-10-31: every
recommendation taken. The second of P19's three packets (the user's
choice, 2026-10-29), after P19a Monitor and before P19c Keychain.

`apps.md` Cleaner 1–12 says what Cleaner gives: what Broom and DaisyDisk
give a Mac. That means where the space went by kind, what can go safely,
duplicates, large and untouched files, and a basket that goes to the Trash
in one job with one undo. D87–D90 are v1's record of how. P1 answered
every presentation question: `p1/structure/cleaner.md` (binding) and
`p1/surfaces/cleaner.md` with its Wave C board. Its tokens, the ten kind
colours among them, are in `tokens.kdl`. What is open is how it is built,
and `apps.md`'s one point: the rules file's format.

Today nothing scans. `isle-fs` (B104) has the jobs, the Trash with put
back, undo by steps, and mounts, but its only size code is a plain
recursive measure. v1's scan, kinds and duplicate finder were C++ in
`native/files/` and its rules were `shell/windows/cleaner/rules.json`,
ten groups. P19a gives a shared table, `use_shown()` and the pattern of a
pkexec helper under Isle's own policy.
`docs/research/rust-desktop/apps-2026.md` §8–9 has the evidence:
- the host's `/` and `/home` are btrfs subvolumes of one device, mounted
  `noatime` and `compress=zstd:3`, `/home` 86% full;
- `/.snapshots` exists for `@`;
- `st_blocks` reports uncompressed bytes;
- `cp` reflinks by default;
- the pacman cache is 14 GB and `~/.cache` 24 GB.

What P19b must get right is trust. Cleaner is the one Isle app whose
purpose is removing things. Nothing it finds may leave the disk except
through the basket, the user seeing the total first, and with one undo.
And it must not make the user wait: a scan of the home on a warm cache
in under a second, as D87 measured v1's.

## What it answers to

- Decisions: B2, B9, B17, B19, B26, B27, B94 (`isle-app`), B104 and B106
  (`isle-fs`, jobs that lose nothing and undo), B107, B110 (Quick Look),
  B119 (work in flight); D70, D87–D90.
- Intent: `apps.md` Cleaner 1–12, its v1 lessons and open point, Files 9,
  Conventions 1–25, bars 1–4, 7, 8, 10 and 11, the Parity map's Cleaner
  rows.
- Design, binding: `p1/structure/cleaner.md`, `p1/surfaces/cleaner.md` and
  its board, `structure/conventions.md` and `surfaces/conventions.md` (the
  inspector, tables, sidebar rows), `surfaces/window-chrome.md` (alerts),
  `structure/files.md` (Empty Trash's alert), `surfaces/prompts.md` (the
  polkit frame), `system/data-colours.md` (the kinds).
- Evidence: `apps-2026.md` §8–9, `isle-v1-inventory.md` §5 Cleaner, v1's
  `native/files/{diskusage,duplicates,kinds}.cpp`,
  `shell/windows/cleaner/rules.json` and `scripts/syscleanup.py` on `main`.

## Choices this plan makes

### Scope

1. **One packet. The drag and drop is cuttable** (19b.10): dragging rows
   and tiles out as files, and dropping a folder in to scan it.
2. **Settings › General › Storage stays in P20.** The scan, the kinds and
   the rules are a crate of their own (choice 4), so P20's Storage page
   reads the same figures and Recommendations without a second scan
   engine.
3. **No gate.** P1 drew every view, state and motion. Three things P1 did
   not write are added to `p1/structure/cleaner.md` at 19b.0 as copy:
   - Duplicates' "Comparing files…" while content is read (choice 14);
   - Untouched's labels on a `noatime` disk (choice 13);
   - "Some of this is shared with other files." (choice 15).

### The scan

4. **A new crate, `isle-scan`,** beside `isle-fs`, not inside it. It
   holds the walk, the kinds, the rules and the duplicate finder, with no
   UI. Files does not need it; Cleaner and P20's Storage page do. It uses
   `isle-fs`'s mounts.
5. **Hand-rolled on `rustix`, not `dua-core`.**
   - **The walk:** a pool of workers taking one folder each from a
     work-stealing queue, one per core up to eight. Each lists its folder
     with `getdents64` and asks `statx` for type, blocks, device, inode,
     links, mode and times, never following a link.
   - **Why not `dua-core`:** D87 needs things inside the walk that
     `dua-core`'s walker does not offer: a kind inherited down the tree,
     crossing mounts by the mount table rather than `st_dev` (btrfs
     subvolumes each have their own), never entering `.snapshots`, and
     keeping folders rather than files.
6. **What a scan keeps** (D87):
   - **Folders:** each folder's own bytes and files, its totals, and its
     bytes by kind, as a tree of nodes with names interned.
   - **Files:** the 500 largest; the 500 largest untouched for more than
     six months (choice 13); and the duplicate candidates (choice 14).
   - **Sizes:** allocated blocks. A file with several names counts once,
     at the first met, by device and inode.
   - **Not reached:** folders the user cannot read are counted apart,
     and named in Overview.
7. **Roots and mounts.**
   - **Roots.** All disks is every mounted disk-backed filesystem from
     the mount table, under one root node. A root another root's walk
     already reaches is dropped (D90): the host's `/` reaches `/home`.
   - **Crossing.** A walk crosses into a mount of the same device and
     filesystem, and into nothing else: no other disk, pseudo filesystem
     or network mount.
   - **Never entered:** `.snapshots`, mounted or not, and `/proc`, `/sys`,
     `/dev` and `/run` by type.
8. **Progress is a share of used space,** as P1 draws it: bytes scanned
   over the roots' used bytes from `statvfs`. A compressed btrfs counts
   more than `df` says is used, so the share stops at 99% until the walk
   ends.
   - **Stop** keeps what was found, marked partial.
   - **Closing the window** cancels the scan (`apps.md` Cleaner 11).
   - **Not shown:** no scan starts while the window is not shown
     (`use_shown()`, P19a); one already running carries on.
9. **Views fill as the scan goes.** The scan publishes its tree at most
   once a `timing-scan-layout-ms` (1 s), and the window redraws from that.
   Space's files in a folder are read with `statx` as the folder opens,
   since the scan keeps no files but the lists in choice 6.

### Kinds

10. **Ten fixed kinds, folder names first** (D88). This is v1's
    `kinds.cpp` tables ported: folder names such as `steamapps`, `Games`,
    `node_modules`, `.cache`, `Videos`, each giving their kind to
    everything under them, a nearer folder winning. A file's ending is
    used only where no folder has said. A file has one kind everywhere,
    and the tables are tested by path.

### Rules

11. **The rules file is KDL** (`apps.md` Cleaner Open), as the settings
    are: comments, one parser in the tree, a `version` node.
    - **Two files:** the shipped `data/cleaner/rules.kdl`, and
      `~/.config/isle/cleaner/rules.kdl` read beside it. A user rule of
      the same id replaces a shipped one, and `disable "<id>"` drops one.
      A user file that does not parse is skipped with its error in the
      session log, and the shipped rules stand.
    - **A rule** has an id, the app's name, a kind, `safe` or not, and a
      reason for Worth a look, plus what it matches:
      - paths with `*` and `**`;
      - a file that must sit `beside` it, for build folders;
      - `except` names;
      - an `unless` path with the matched name, for Steam leftovers
        without their `appmanifest_<id>.acf`;
      - `older-than` for files in one folder.
    - **Evaluation.** Rules are matched against the scan's tree in
      memory (D89). A named rule wins over the general one where both
      match. Steam's paths are expanded to every library `isle-games`
      reads from `libraryfolders.vdf`, other disks included.
12. **The shipped rules: v1's ten groups, recut to P1's cards.**
    - **Safe to clear:**
      - the general rule: every entry in `~/.cache` and each
        `~/.var/app/*/cache`, each named for the app that made it;
      - package downloads: npm, pnpm, Bun, Cargo's registry and git
        checkouts, Gradle, Maven, NuGet, Yarn;
      - Electron caches in `~/.config/*`.
    - **Worth a look:**
      - build folders while their project file sits beside them (P1
        draws `target/` there, where v1 had them safe);
      - games no longer installed, and Proton and Wine versions;
      - shader caches (the general rule's exception: a game stutters
        while they rebuild);
      - AI models (the other exception);
      - agent worktrees;
      - old downloads;
      - virtual machines.
    - **Never listed:** a clipboard history.
13. **Untouched on a `noatime` disk.** The host mounts both subvolumes
    `noatime`, so a file's access time is when it was made, and "Not
    opened in" would be false.
    - **The time used:** the later of access and modified time, as a
      file's last use.
    - **The labels:** Untouched reads "Not opened in" only where every
      root is mounted with `relatime` or `strictatime`. Otherwise it
      reads "Not changed in", and the Last opened column reads "Last
      changed".
    - **Rejected:** remounting with `relatime`, which on btrfs writes
      metadata for reads and grows every snapshot.

### Duplicates

14. **Duplicates as D90.**
    - **Candidates:** files of a megabyte or more, not executable, outside
      hidden folders and git working trees, and claimed by no rule or
      by the Games, Apps and system, Developer or Caches and logs kinds.
      They are gathered during the walk.
    - **Comparing:** grouped by size, the same device and inode skipped;
      then xxh3 over the first and last 16 KiB; then BLAKE3 over the whole
      file, only where the ends agree.
    - **When:** it runs after each scan on idle I/O priority (`ioprio`
      idle class), so a game loading wins. Duplicates shows "Comparing
      files…" in its content until it is done.
15. **What sharing means for space.** FIEMAP's shared flag is read for
    duplicate copies, as D90 has it: a copy sharing every extent is
    marked "Shares storage" and frees nothing. It is read too for any
    file put in the basket that is a megabyte or more.
    - **The basket's frees figure** leaves out shared extents, and says
      "Some of this is shared with other files." when any were.
    - **Folders** in the basket are summed from the tree without that
      read, since reading extents for every file is a second walk.

### Removing

16. **Nothing leaves except through the basket.** No rule, view or
    shortcut removes anything; only Move to Trash and Empty Trash… do,
    plus the System card's actions, each through polkit.
    - **One job** through `isle_fs::jobs::run` with Trash work (Files 9,
      B106), run on the worker with Cleaner's own small runner. Files'
      private job queue is not lifted; Cleaner runs one job at a time.
    - **The basket** holds paths, a folder taking the place of anything
      under it already there.
    - **Before moving,** each item is checked again: it still exists, is
      the same device and inode the scan saw, and is not now a link.
      An item that fails is listed with the job's failures, not moved.
17. **Undo and the Trash.**
    - **Undo:** ⌘Z puts the last move back through its undo steps. An
      item moved can also be put back from Files, since the Trash is
      shared.
    - **Empty Trash…** asks with Files' alert, Empty Trash the default
      (B138), and
      runs `EmptyTrash` over the scope's disks' Trashes.
    - **The process stays** while a job runs, with progress on the
      switcher card (Conventions 4, B119).
18. **The system's share through one helper.**
    `/usr/lib/isle/isle-clean-helper`, run by pkexec, has four fixed
    subcommands and takes no paths:
    - `package-cache`: `paccache -rk2`, then `-ruk0`;
    - `orphans`: `pacman -Rns` of `pacman -Qdtq`'s list, read again by the
      helper rather than passed in;
    - `journal`: `journalctl --vacuum-size` to v1's keep;
    - `coredumps`: removing `/var/lib/systemd/coredump`'s files.
    Each is its own action in `io.github.alexparlett.Isle.Cleaner.policy`,
    at `auth_admin_keep`, with a message saying what goes. pkexec's
    default is a bare `auth_admin` naming the binary. The sizes are read
    without root (`paccache -d`, `pacman -Qi`, `journalctl --disk-usage`,
    `stat`), and a refused authorisation changes nothing.

### The window

19. **`isle-cleaner` on `isle_app::run`,** one window as P1 draws it:
    - the sidebar of views with the disks and the Trash at its foot;
    - the toolbar with back and forward over views and Space's folders;
    - the scope pop-up: All disks, each disk, recent folders, Choose
      folder… through the FileChooser portal (Files' panel, B108);
    - the six views, and the basket as the inspector;
    - the command table holds P1's Keyboard.
20. **Tables are P19a's `isle_ui::table`**: Large files, Untouched,
    Duplicates' groups and Space's list. The kind bar, the legend, the
    kind tile and the treemap are Cleaner's own.
    - **The treemap** is squarified, with children under `size-tile-min`
      merged into "{n} smaller items".
    - **Opening a folder** zooms from its tile as P1's Motion has it,
      each child's rectangle mapped through the growth.
21. **The disks at the sidebar's foot** read `statvfs` for each disk in
    the scope, through the dashboard's `read_volumes`. That code moves
    from the compositor into `isle-disks`, so the Storage widget and
    Cleaner read one list.
22. **What Cleaner remembers** is the last view, Space's Map or List,
    Untouched's period, recent folders and each table's columns. It is
    kept in Cleaner's `isle-state` domain. Each run scans afresh: no
    scan is kept (P1).
23. **Ties.**
    - **Quick Look** through `QuickLook1` on Files' name (B110), and
      **Show in Files** through `FileManager1.ShowItems`.
    - **Copy path** copies the path.
    - **Drag and drop** goes through `isle-app`'s data device: rows and
      tiles drag out as files, and a folder dropped on the window scans
      it (cuttable, choice 1).
    - **Opening Cleaner:** the desktop entry, service file and package;
      Spotlight finds it by its entry.

### Proof

24. **Tests on real trees.** `isle-scan`'s tests build trees in a
    temporary folder: hard links, sparse files, unreadable folders, a
    link loop, a fake `.snapshots`, and project files beside build
    folders. The scan's totals must equal `du --apparent-size=no -l`'s
    over the same tree, links once.
    - **Mounts and reflinks** are tested in the VM on a btrfs loop image
      with a subvolume, a snapshot folder and reflinked copies. A test
      skips where it cannot mount.
25. **A test that nothing moves.** A tree is scanned, its rules run, its
    duplicates compared and its basket filled, and every file's content
    and inode is unchanged until Move to Trash. After Undo, they are
    unchanged again.
26. **`isle-scan` has an example** that scans a path and prints its totals,
    kinds, the top of Cleanup and the cost, for the host's checks.
27. **Bars** beside `apps.md`'s:
    - **Scan speed (bar 10):** the host's home, warm, in under a second,
      as D87; All disks is reported.
    - **Memory:** the tree for All disks on the host under 64 MB,
      reported with the window's PSS after a scan.
    - **Drawing:** during a scan, a frame of Overview or the map stays
      under 8 ms at p95 in software (B26).

## The workspace

```
crates/
  isle-scan/              walk, kinds, rules, duplicates (new)
  isle-cleaner/           the app: views, treemap, basket, the System
                          card (new)
  isle-clean-helper/      the pkexec helper (new)
  isle-disks/             volumes, moved from the compositor's dashboard
  isle-compositor/src/
    dashboard.rs          reads volumes from isle-disks
data/
  cleaner/rules.kdl       the shipped rules
  apps/                   Cleaner's desktop entry and service file
  polkit/                 io.github.alexparlett.Isle.Cleaner.policy
pkg/arch/PKGBUILD         the app, the helper, the rules, the policy
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 19b.0 · The note and the record

`apps-2026.md` §8–9, with the host's `noatime` mounts added; the
roadmap's P19b entry; `apps.md`'s open point answered by choice 11; the
three lines into `p1/structure/cleaner.md` (choice 3).

### 19b.1 · The walk

`isle-scan`'s walk (choices 5–9): the pool, blocks, links once, roots
dropped, mounts crossed, `.snapshots` and pseudo filesystems never
entered, unreadable folders, progress, Stop and cancel, the kept lists.
It is tested against `du` on temporary trees, and on a btrfs loop image
in the VM.

### 19b.2 · Kinds

v1's tables ported (choice 10), tested by path: a PNG in a game, a cache in
a game, a video in Downloads.

### 19b.3 · Rules

The KDL format, the shipped rules (choice 12), the user's file, evaluation
over the tree, Steam's libraries through `isle-games`. Each rule is tested
on a tree that has it and one that does not.

### 19b.4 · Duplicates and sharing

Candidates, the three stages, idle I/O and FIEMAP (choices 14–15),
reflinks tested in the VM.

### 19b.5 · The system's share

`isle-clean-helper` and its policy (choice 18). The sizes are read without
root. The helper refuses any argument beyond its subcommand. It runs in
the VM against real pacman, the journal and a crash dump.

### 19b.6 · The window, Overview and Cleanup

`isle-cleaner`: the toolbar with the scope and the progress bar, the
sidebar with the disks (`read_volumes` moved), history, Overview's bar,
legend and groups, and Cleanup's three cards with Safe to clear in the
basket after a scan.

### 19b.7 · Space

The treemap, tiles, hover and selection, the list with disclosure, the
path, and the zoom and its reverse.

### 19b.8 · Duplicates, Large files, Untouched

The tables; Duplicates' groups, "Shares storage" and "Select all but the
newest"; Untouched's period and its labels by mount.

### 19b.9 · The basket and the Trash

The basket's forms, the check before moving, the job and its failures,
Undo, Empty Trash…, the process held while moving. Choice 25's test.

### 19b.10 · Ties and drag and drop

Quick Look, Show in Files, Copy path, Choose folder…, recent folders; the
desktop entry, service file and package; dragging out and dropping in
(cuttable).

### 19b.11 · Motion, states and accessibility

P1's Motion under springs, Reduce Motion and Game Mode; every row of
States; Increase Contrast, the largest text size, focus, the narrow
window.

### 19b.12 · Measures and the walk

The bars on a release build (choice 27; `apps.md`'s in the VM where they do
not need the host). Walked in the VM by real input, as
`verify-by-interaction` asks:
- a scan, Stop and Rescan;
- each view, and the scope changed;
- ticks across views landing in one basket;
- Move to Trash, Undo, put back from Files, Empty Trash…;
- a System action refused and allowed;
- a folder opened and closed in Space by keys;
- ⌘Q while moving.
The journal is read after.

### 19b.13 · The host and close

The host run on the installed package:
- bar 10 on the real home, and the memory figure;
- every Cleanup row on the real home read through with the user before
  anything is moved, Worth a look above all;
- the System card's sizes against `paccache` and `du`;
- a small basket moved and undone.
Then the record:
- decisions recorded from P19a's last;
- ARCHITECTURE gains How Cleaner scans;
- `apps.md`'s open point closed;
- the roadmap and `state.md` follow;
- this file ends with its outcome.

## Reviewed

The user reviewed the plan on 2026-10-31 and took every recommendation.

## Not in P19b

- Settings › General › Storage and its Recommendations (P20, on
  `isle-scan`).
- Replacing a duplicate with a clone (`FIDEDUPERANGE`), which P1 did not
  design.
- Compressed sizes on disk (`compsize`'s figures need root).
- A scheduled or automatic cleanup: Cleaner removes only what the user
  puts in the basket.
- Snapshots: snapper's are P20's provider.

## Depends on, and feeds

- Needs P15 and P15b (`isle-fs`, the Trash, FileChooser, Quick Look,
  `FileManager1`), P9 (the polkit agent), P10 (`isle-games`' Steam
  libraries), P14a (the package), P17 (the dashboard's volumes), P19a (the
  table, `use_shown()`, the helper pattern).
- Feeds P20's Storage page, and P19c, which takes the policy pattern
  again.

## Done when

- `cargo test` passes headless with the walk's, kinds', rules' and
  duplicates' tests and choice 25's; the mount and reflink tests pass in
  the VM.
- In the VM, Cleaner is walked by real input through every row of the
  sheet's Keyboard, Pointer and States, and P1's board's scenes can be
  reproduced.
- On the host, the home scans in under a second warm, Cleanup's rows are
  read through with the user, and a basket moves and undoes.

## Risks

- **Rules on a real home.** A wrong rule removes something wanted. Every
  rule has a test tree, Worth a look is never ticked, everything goes to
  the Trash first, and the host run reads the rows through with the user
  before anything moves.
- **Frees on btrfs.** Snapshots of `@` and reflinked copies mean the Trash
  emptied may free less than the basket says. The frees figure leaves out
  shared extents it read; folders are not read that way, and `/`'s
  snapshots are not counted. The risk is said, not hidden: P1's "once the
  Trash is emptied" already hedges it, and the decision records it.
- **A walk of All disks** reads a second NVMe and Steam libraries cold the
  first time. The bar is the warm home; the cold All disks is reported,
  not held to a number.
- **A treemap in software.** Thousands of tiles re-laid each second could
  miss bar 27. Tiles under `size-tile-min` merge, re-layout runs at most
  once a second, and the measures decide the merge threshold.
- **`noatime` everywhere.** Untouched is weaker on the host than on a Mac:
  "Not changed in" is the honest reading, and the view is no help for
  files only read.
