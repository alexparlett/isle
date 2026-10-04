# P3 runtime measurements

Run 2026-10-04 on the host (RTX 5070 Ti, driver 615.71.09) and in the
test VM (virtio-gpu, llvmpipe). Plan: `docs/work/rust-desktop/p3-shell-runtime.md`.

## How

`isle-compositor --headless` on the host draws on the 5070 Ti's render node
through EGL, which is the same GL path as the DRM backend without a window
on the desktop it runs beside. `crates/isle-compositor/tests/bench.rs` adds
a 3440×1440 output as the only one, turns on `Test1.Measure` (each surface
draw is followed by `glFinish` and timed, CPU and GPU together) and opens
the probes:

- `probe`, 480×360: text with fallbacks, buttons, a field, a scrolling list
  whose rows fade in, an animated bar, a glass strip.
- `probe-full`, the whole output, shaped like the spike's launcher: a
  backdrop, a 1200×760 panel with a field and 24 tiles, whose tiles can
  fade in a loop, each through its own Skia layer.

Release build (`opt-level 3`, debug info). Memory from
`/proc/<pid>/smaps_rollup`; VRAM from `nvidia-smi`'s process table.

```
cargo test --release -p isle-compositor --features test-hooks --test bench -- --ignored --nocapture
```

## Draw time at 3440×1440

| | draws | p50 | p95 | max |
|---|---|---|---|---|
| `probe-full` opening and typing | 15 | 0.33 ms | 0.57 ms | 3.82 ms |
| `probe-full`, 24 tiles fading | 185 | 0.83 ms | 1.28 ms | 4.34 ms |

The longest draw is the first, as Skia compiles its pipelines. Acceptance
5 asks for under 2 ms at p95; a full-output draw takes a quarter of that,
and the per-element fade the design avoids (B7) still stays under it. A
debug build is about four times slower (p95 1.39 and 4.54 ms), from
Freya's diff and layout rather than the GPU.

## Memory

| | RSS | PSS | VRAM |
|---|---|---|---|
| no surface open, 3440×1440 output | 101.3 MB | 82.4 MB | 68 MiB |
| `probe-full` open | 122.3 MB | 102.8 MB | 89 MiB |
| `probe` open, at rest | 126.2 MB | 106.5 MB | 62 MiB |
| both closed again | 126.2 MB | 106.5 MB | 61 MiB |

Skia's resource cache, capped at 64 MiB: 31.4 MB with `probe-full`'s fading
tiles, 12.7 MB after the probe closes and is purged, 13.4 MB after a hundred
opens and closes (the headless test). A closed surface gives its texture
back; what stays is Skia's glyph atlas, the font collection and the
allocator's high-water mark.

## Soak

`tests/bench.rs`'s soak, on the 3440×1440 headless output, each cycle
about 1.5 s: open the probe, type and delete three letters, start and stop
its bar (a relayout every frame), close it; open `probe-full`, fade its
tiles for half a second, close it.

- **One hour, 2,379 cycles.** VRAM stayed at 105 MiB from the first minute
  to the last. RSS cannot be read on a busy host: it went from 130 to 201
  MB in the first five minutes, then down to 64 MB as the kernel evicted
  and swapped pages (the host had 35 GB of zram swap in use).
- **The process's own memory** (anonymous pages, resident or swapped) is
  the figure that grows if Isle leaks. Over 20 minutes and 796 cycles it
  went from 34.3 to 42.9 MB, slowing toward the end; over ten minutes
  glibc's count of bytes allocated on the loop's thread (`mallinfo2`)
  went from 17.6 to 21.0 MB, in steps.
- **Each probe alone** for six minutes: 4.5 KB a cycle stays allocated with
  the small probe and 2.2 KB with the full one, while the process's own
  memory stayed flat (the growth fits in the heap already there).
- The spike's 178 MB relayout run did not recur.

Open: what keeps 2 to 5 KB a cycle. Freya's runner and tree do not grow
(two scopes and three layout nodes while the probe is closed, the text
cache steady). Skia's glyph and text-blob caches filling to their limits
would fit the size and the slowing; a heap profiler (heaptrack) should
settle it before the 24-hour soak at the end of M1. Run with
`ISLE_SOAK_SECS` and `ISLE_SOAK_ONLY=probe|full`.

## Glass

The probe's glass strip (480×80 points, with its 96-point margin a
672×272 backdrop) blurred again 76 times as windows behind it came and
went: p50 0.41 ms, p95 0.75 ms, the longest 3.79 ms, the backdrop drawn
again and every pass included, `glFinish` before and after. The plan asked
for under 1 ms.

## The VM

The guest session (`/repo/target/debug/isle-session` on tty1, a test-hooks
build) opened the probe with `isle shell open probe` over ssh, on
llvmpipe at 2560×1080. Typing through QEMU's keyboard reached the field
and a click through its tablet pressed the button (`Test1.Probe` read
back `"vm", 1`); Super+Return opened kitty behind it, and the probe's
glass blurred kitty's background. The CJK sample is drawn as boxes there,
since the guest has no Noto CJK font.

## Hot reload

`dx serve --hot-patch` (dioxus-cli 0.7.10) builds and launches
`isle-preview` with Freya's `hotreload` feature, but an edit to a surface
in `isle-shell` stops at "Missing rustc args for replay: 'isle_compositor'":
it replays every workspace crate that depends on the edited one, and the
compositor is not in the preview's build. Plain `dx serve` rebuilt on a
change without relaunching the app. `isle-preview --watch` rebuilds on a
saved change and starts the new preview in place of the old: 2.7 s from
save to window, the fixture state as it was.
