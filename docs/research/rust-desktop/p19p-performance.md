# P19p · Compositor performance: the evidence

Measured 2026-10-30 on the host. The installed package was build `bcc6f41`
(release), launched by greetd. The desktop was mostly idle: one output, DP-4,
3440×1440 at 144 Hz with VRR on demand; Vivaldi, Claude and Proton Mail open;
the wallpaper a solid colour. Everything was read from `/proc` and the
compositor's own D-Bus counters (`Displays1.Trace`, `Shell1.List`). `perf`,
`heaptrack` and `strace` are not installed, and `ptrace_scope` is 1.

## CPU

| Uptime | Main thread | Four tokio blocking threads | Total |
|---|---|---|---|
| 10 min | 7–10% | about 2% each, waking 0.8 times a second | about 18% |
| 25 min | 8% | about 16% each | about 75% of one core |

- **The blocking threads are the file index rescanning itself.**
  - `files.rs` forwards every notify event, `Access` included. notify 8.2's
    inotify mask has `IN_OPEN`.
  - `FileIndex::sync` walks the folder with `ignore::WalkBuilder`. The walk
    opens the folder and each of its subfolders (walkdir pushes every child
    directory, even at `max_depth`). It also opens every ancestor's
    `.gitignore`.
  - Each of those opens is a new event on a watched folder, so a folder with a
    subfolder rescans itself every `SETTLE` (300 ms), and so does every
    ancestor up to the root.
  - The set of folders in the loop only grows: any process reading in an
    indexed folder adds that folder and its ancestors.
- **The index's own threads.** Its notify thread woke about 14,000 times a
  second. The index holds 7,153 watches and 86,556 entries. The write lock is
  held for each whole round, which blocks Spotlight's file searches.
- **The main thread composites every refresh.**
  - 1,443 frames in 10 s, all of them `composited`, with the cursor on its
    plane.
  - It wakes about 234 times a second.
  - The island drew 0 times in those 10 s, but its glass was blurred 490 times
    (about 49 a second): the windows under it changed.
  - The frames are driven by client commits. Nothing in the compositor loops on
    its own: every animation predicate settles, and no timer is short while
    idle.
- **Where a redraw pass goes** (`redraw.rs` `redraw()`):
  - The element list is built 1 + (glass regions) + (window backdrops) times.
  - `blur.update` runs an `OutputDamageTracker` pass each frame. When anything
    under the glass changed, it runs the Kawase passes and a synchronous
    readback (`map_texture`) of a 4×4 copy.
  - Each pass also does O(windows) scans that allocate: `watch_island`,
    `output_identity`'s `format!`, `window_pieces`' O(n²) modal check,
    `update_shown`, `update_primary_scanout_output` and `sampled_buffers`.
  - `refresh_island` queues a redraw on every output on every call, and it has
    about 60 callers driven by services.
- **Slow frames.** 24 frames over the refresh budget in 12 minutes, logged by
  `FramePhases`:
  - glass: average 6.8 ms, longest 54.7 ms;
  - render: average 4.9 ms, longest 10.6 ms;
  - shell draw: average 1.0 ms, longest 8.6 ms.
  - The watchdog logged loop stalls of 7–11 ms from `redraw` and `libinput`.

## Priority

The compositor runs at nice 16, and so does every app it starts (Claude and
Electron show NI 16).
- ananicy-cpp's stock `greetd.rules` gives `greetd` the `Launcher` type.
  `isle-session` inherits it, and so does everything below.
- The same file gives niri, kwin_wayland and sway `LowLatency_RT`. There is no
  rule for `isle-compositor`.

## Memory

| Uptime | RSS | Peak (VmHWM) |
|---|---|---|
| 10 min | 361 MB | 562 MB |
| 19 min | 467 MB | 662 MB |
| 25 min | 440 MB | 662 MB |

- **Anonymous memory is mostly glibc arenas inflated by THP.**
  - 257 MB is anonymous, of which 155–164 MB is AnonHugePages. THP is
    `always`.
  - Seven glibc arenas hold about 180 MB, and `[heap]` holds 62 MB.
  - No global allocator, `mallopt` or `MALLOC_ARENA_MAX` is set.
  - `malloc_trim` runs only after a still capture.
- **The arenas are fed by the index loop.** It allocates on whichever blocking
  thread is free: a new map per folder, a clone of the old one, and a clone of
  every folder key whenever a subfolder appears.
- **Mesa is mapped but never used.** About 29 MB of file-backed RSS is
  `libgallium` and `libLLVM`. libglvnd loads every EGL vendor; the iGPU is
  never opened. `isle-session` sets only `__GLX_VENDOR_LIBRARY_NAME`, which EGL
  ignores.
- **Cache risks that are not the cause today:**
  - Freya's `ImageViewer` keeps both the decoded and the encoded image for an
    hour, per surface tree.
  - Spotlight's preview decodes at the image's natural size (a 24 MP photo is
    about 96 MB).
  - `FontMgr::default()` makes a new fontconfig manager per call.
  - The switcher keeps its preview images after it closes.
  - `~/.cache/isle/notifications` holds 562 PNGs (150 MB on disk) while only
    300 records are kept.
- **Bounded, and checked:**
  - `isle_sysinfo::Sampler`, which runs only while a System widget shows;
  - the clipboard, notification records and frame traces;
  - per-window caches;
  - the wallpaper, whose CPU pixels are dropped after upload;
  - shell render targets, which exist only while open.
- **The NVIDIA driver's own share:** 36 MB of `/dev/nvidiactl` mappings and
  about 40 MB of its libraries. This is not Isle's to change.

## After, headless (release, 2026-10-30)

One window drawing at 60 Hz, clear of the island's pill, on the headless
backend (EGL on the 5070 Ti, the client's buffer in shared memory).

| | Before | After |
|---|---|---|
| Element lists of the windows and below per frame | 2 | 1 |
| Frame p50 / p95 | 190 / 372 µs | 180 / 313 µs |
| Glass step p50 | 88 µs | 84 µs |

- **The first list of a frame pays for the buffer upload.** Building the
  windows and below costs about 57 µs, and most of that is uploading the
  client's shared-memory buffer. A second list cost little, so sharing it
  saves less here than it would with real clients.
- **The host would not pay for the upload.** Chromium and Electron hand
  over GPU buffers (dmabufs), so there is nothing to upload.
- **Binding the backdrop.** A blur update with nothing to draw bound its
  framebuffer for about 17 µs. It now returns before binding.
- **Idle:**
  - no redraw passes, glass runs, index syncs or commits over 2 s;
  - an island post that changes nothing draws nothing.
- **The index.** Reading, writing and listing an indexed folder cause no
  sync. A create, a rename and a delete cause one each.
