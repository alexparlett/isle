# P19p · Compositor performance

Planned 2026-10-30. Reviewed by the user the same day: every
recommendation taken. The evidence is in
`docs/research/rust-desktop/p19p-performance.md`.

On the host, `isle-compositor` uses the most CPU and memory of any process.
After 25 minutes it was using 75% of one core and 440 MB of RSS, with a peak
of 662 MB, while the desktop was mostly idle. It runs before P19b. P19b,
P19c and P20 add more to the same process, and a game beside a desktop that
takes a core is not acceptable.

## Choices this plan makes

### What it fixes

Four causes were measured, plus cache risks found by reading. In order of
cost:

1. **The file index rescans itself.** This is about 64% of a core, and it
   grows with uptime.
2. **The redraw pass costs more than its damage.** This is about 8% of a core
   at 144 Hz, driven by clients drawing.
3. **Priority.** The whole session, and every app it starts, runs at nice 16.
   This comes from ananicy's rule for greetd.
4. **Memory.** glibc arenas with THP, filled by cause 1; Mesa mapped for
   nothing; cache risks that are not the cause today.

### The file index

- **The watch.** `files.rs` stops forwarding opens. The index watches with
  its own inotify mask:
  `CREATE | DELETE | MOVED_FROM | MOVED_TO | DELETE_SELF | MOVE_SELF`.
  It uses `isle-fs/src/watch.rs`'s rustix wrapper rather than notify, so
  `IN_OPEN` and `IN_ACCESS` cannot come back with a notify upgrade.
  Recommended. The alternative is to filter `EventKind::Access` the way
  `games.rs` does, which is smaller but leaves 14,000 wakes a second of
  filtered opens on the notify thread.
- **The sync.**
  - `FileIndex::sync` reads the one folder with `std::fs::read_dir`.
  - It uses each root's gitignore matchers, built once and rebuilt only when
    a `.gitignore` itself changes.
  - The walk with `ignore::WalkBuilder` stays for the first build and for a
    new subfolder.
- **The lock.**
  - The new folder map is built outside the write lock and swapped in.
  - Spotlight's searches no longer wait on a round.
  - The `before.cloned()` and the clone of every folder key go.

### The redraw pass

The compositor already does nothing on its own. The cost is what each
client-driven frame does.

- **One element list per frame.**
  - The list for the output is built once.
  - The glass and window-backdrop passes take the slice below their own
    region from it, instead of rebuilding the list each.
  - `window_backdrops` and `floats_over_shell` are computed once per frame.
- **Glass blurs only when its footprint is damaged.**
  - The footprint is a shape plus the blur's reach, not the surface's box.
    The island's box is 700×1440, while its glass is the pill.
  - With no damage in the footprint, the blur's damage-tracker pass is
    skipped entirely.
  - The 4×4 tint readback stays, but becomes asynchronous: read through a
    pixel buffer object a frame later, never `map_texture` on the loop.
- **Island refreshes queue a redraw only when its model changed.** The same
  `set_if_modified` result that already spares Freya decides this.
- **Per-frame allocations go:**
  - `output_identity` is cached on the output;
  - `watch_island` reuses its vectors and compares names without cloning;
  - the modal check in `window_pieces` is indexed by parent, not scanned for
    each window;
  - `update_shown` reuses its buffers;
  - `update_primary_scanout_output` walks only the output's own windows.
- **The opaque interior of a rounded window** is reported to the damage
  tracker so the wallpaper and shadows under it are not blended. This saves
  GPU time, not CPU. Cuttable.

### Priority

- **The package ships an ananicy-cpp rule**, `/etc/ananicy.d/30-isle/isle.rules`:
  - `isle-compositor` as `LowLatency_RT` (nice −12), as the stock rules
    treat niri, kwin_wayland and sway;
  - `isle-session` with an explicit `"nice": 0`.
- **The session does not depend on ananicy being installed.**
  - At start, `isle-session` sets its own nice to 0 when it inherited a higher
    one. The host's `RLIMIT_NICE` of 31 allows down to −11.
  - Before exec, `child_env.rs` sets each launched app's nice to 0. Raising
    nice is always allowed, so apps inherit neither greetd's 16 nor the
    compositor's −12.
  - Real-time scheduling through rtkit is out. niri leaves priority to
    ananicy as well.

Recommended: ship the rule, and reset nice for the session and its apps.

### Memory

Fix the source first (cause 1), then measure again before changing the
allocator.

- **Arenas.**
  - At start, the compositor calls `mallopt(M_ARENA_MAX, 2)`. It is
    process-local and not inherited by exec.
  - It calls `malloc_trim(0)` after the work that leaves big transient
    buffers: the first index build, a wallpaper decode, closing Mission
    Control or the switcher.
  - mimalloc is considered only if RSS still grows between 30 and 60 minutes
    after that.
  - `PR_SET_THP_DISABLE` is not used: children inherit it, and games want THP.
- **Mesa.**
  - The compositor gets
    `__EGL_VENDOR_LIBRARY_FILENAMES=<the render GPU's vendor json>`, from
    the vendor of the device it opens.
  - `child_env.rs` removes it, so apps on the iGPU still find Mesa.
- **Caches:**
  - Spotlight's preview decodes at its layout size.
  - The shell's image cache age drops to five minutes.
  - One `FontMgr` is shared by the shell, its surfaces and Spotlight.
  - The switcher drops its previews on close.
  - `~/.cache/isle/notifications` is pruned to the records kept. This is
    disk, not RAM, but found here.

### Measures

Release builds gain the counters needed to keep this fixed:

- **`Displays1.Perf(output)`** returns, since a reset:
  - frames presented, redraw passes and undamaged passes;
  - element-list builds;
  - glass runs and glass skips;
  - commits per client, by app id, which says what drives the refresh;
  - p50, p95 and the longest time for each `FramePhases` step.
- **The worker's counters:** index syncs, and blocking-pool CPU time.
- **`isle perf`** prints both, and `--watch` prints them every second.

Bars, on the host's release package with the evidence's setup:

| | Bar |
|---|---|
| Idle (no client drawing) | 0 frames, 0 glass runs, 0 index syncs over 60 s; the compositor under 0.5% of a core |
| One client drawing at 144 Hz | element-list builds equal to frames; glass runs only for damage under the pill; main thread at half or less of the baseline's time per frame |
| Memory | RSS steady (within 5%) between 30 and 60 minutes; anonymous memory under 150 MB at 60 minutes idle |
| Priority | compositor at nice −12 under ananicy and 0 without it; the session and the apps it starts at nice 0 |

### Proof

Headless tests in `test-hooks` builds:
- **The index.** A temporary root holds a `.gitignore` and nested folders.
  After the first build, reading files in it causes no syncs over 2 s.
  Creating, renaming and deleting a file each cause exactly one sync, and
  the index then matches the disk.
- **Idle.** With windows mapped and nothing committing, 0 frames, 0 glass
  runs and 0 redraw passes over 2 s.
- **One list.** A window committing every frame under the island: element
  builds equal frames, and glass runs only when the window's damage reaches
  the pill's footprint.
- **The island.** A service snapshot that changes nothing queues no redraw.
- **Children.** An app launched by the compositor has nice 0 and no
  `__EGL_VENDOR_LIBRARY_FILENAMES`.

Each phase's before and after numbers go in the evidence note.

## Phases

### 19p.0 · Measures

- `Perf` and the worker's counters, and `isle perf`.
- The idle test.
- A baseline on the host with the current package: one hour, sampled every
  minute.

### 19p.1 · The file index

The inotify mask, the `read_dir` sync, cached matchers, and the swap outside
the lock, with the index test. On the host: 0 syncs an hour after login, and
the blocking pool idle.

### 19p.2 · Priority

The ananicy rule in the package, the session's nice reset and the children's
nice, with the children test.

### 19p.3 · One element list

One build per frame, shared by the glass passes; island refreshes that queue a
redraw only when the model changed; the per-frame allocations.

### 19p.4 · Glass

Footprint damage and the skip, and the asynchronous tint readback, with the
one-list test.

### 19p.5 · Memory

The arena cap and trims, the EGL vendor variable, and the caches. Then an
hour on the host measured against the bar. mimalloc is decided here, on those
numbers.

### 19p.6 · Opaque interiors (cuttable)

Rounded windows report their interior as opaque.

### 19p.7 · The host and close

- An hour of ordinary use on the installed package, and an hour with a game
  running, each read with `isle perf --watch` and `/proc`.
- The bars are checked.
- The record:
  - decisions recorded from B132;
  - ARCHITECTURE's How the core runs gains the redraw pass's budget and the
    index's watch;
  - the roadmap and `state.md` follow;
  - this file ends with its outcome.

## Reviewed

The user reviewed the plan on 2026-10-30 and took every recommendation:
the name P19p, the index's own inotify mask through `isle-fs`, and glibc's
arenas capped before mimalloc is considered.

## Not in P19p

- **Direct scan-out and overlay planes for ordinary windows.** The trace
  shows every frame composited. That is P22's and the gaming path's work,
  and the planes are off on NVIDIA by P2's default.
- **Frame pacing for clients that draw without damage.** Answering their
  frame callback less often is a policy change that games and video feel.
- **The NVIDIA driver's own memory.**
- **A heap profiler in the package.** heaptrack is used by hand if 19p.5's
  numbers do not explain the heap.

## Depends on, and feeds

- Needs P2 (the redraw loop, the watchdog), P10 (the file index), P14a (the
  package) and P17 (the glass).
- Feeds every packet after it: its counters and bars become the check that
  P19b, P19c and P20 run before closing.

## Done when

- `cargo test` passes headless with the index, idle, one-list, island and
  children tests.
- On the host, the bars in the table hold on the release package for an hour
  idle and an hour with a game.

## Risks

- **Missing a real change by dropping opens.** No event the index needs comes
  only as an open: creation, deletion and renames have their own events. A
  folder's watch overflow still forces a rebuild, as it does today.
- **A glass that lags its content.** If the footprint is computed too small, a
  window moving under the edge of the pill would leave a stale blur. The test
  moves a window across the edge.
- **Nice −12 starving something.** It is what niri and kwin get from the
  same rules, and it is not real-time. The bar for idle CPU keeps the
  compositor from spending it.
- **The arena cap costing throughput.** Two arenas across the worker's
  threads may contend. The index fix removes most of that allocation, and the
  hour's numbers decide.

## Outcome

Built 2026-10-30 in phases 19p.0–19p.5; 19p.7's hour on the host, part of
done, is the user's, on a package made from this branch with
`tools/package.sh`. B132–B136 record the decisions; ARCHITECTURE's How the
core runs and Spotlight's file index follow. Release numbers from the
headless tests are in the evidence note. Differences from the plan:

- **The index keeps its walk.** A folder is still read with
  `ignore::WalkBuilder` at depth one: with only names heard, a sync runs
  for a real change, and caching the matchers would buy little. Hidden
  names are not read again, and an ignore file changing builds afresh.
- **The glass already blurred by its footprint.** Each blur tracks damage
  over its shape and margin, and its tint readback was already a frame
  late. What each frame paid was binding the backdrop to find no damage:
  the blur now compares what is behind it with the last update and
  returns before binding (B133).
- **One list of the windows and below, not one list.** The frame's list is
  built after the glass, whose elements carry the blur's state, so the
  part shared is the windows and below; the few elements above the
  windows are built per glass region.
- **Per-frame allocations left as they are.** In release they are 6 µs of
  a 190 µs frame.
- **Not done, and why:**
  - the EGL vendor variable: the render device is chosen after libglvnd
    loads, and Mesa's pages are clean and shared;
  - one `FontMgr`: the extra managers are in the Freya fork, and
    Spotlight's is made once;
  - dropping the switcher's previews on close: they are GPU textures, and
    clearing them blanks the cards as the switcher fades out;
  - 19p.6, opaque interiors (cuttable): GPU time only.
- **A suite note.** `shell.rs`'s probe test, which wants six frames in
  250 ms, fails under the full suite with the host's old compositor taking
  a core; it passes alone and with six test threads.
