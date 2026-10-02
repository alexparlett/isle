# island-slint: notes

Slint 1.18.1 (current on crates.io, 2 Oct 2026), software renderer into a CPU
buffer, uploaded to one `GlesTexture` by dirty rectangle. The UI is
`ui/island.slint`, compiled at run time by `slint-interpreter` and reloaded
when the file changes. RTX 5070 Ti, DP-4 3440x1440 at 60 Hz, scale 1, so the
texture is 520x240.

## Read this first: the shared workspace no longer resolves

Resolved since: Freya moved into its own nested workspace, and
`cargo build --release -p island-slint` from `spikes/shell-ui` works again
(Round 2 below was built and run that way).

`cargo build --release -p island-slint` from `spikes/shell-ui` fails since
`freya/Cargo.toml` gained `freya-engine`:

```
package `skia-bindings` links to the native library `skia`, but it conflicts with a previous package which links to `skia` as well:
package `freya-skia-bindings v0.100.0-beta.1`
```

The path is `slint-interpreter` -> `i-slint-backend-winit` (a non-optional
dependency for `wasm32` only) -> `i-slint-renderer-skia` (non-optional for
iOS only) -> `skia-safe 0.153` -> `skia-bindings`, `links = "skia"`. Cargo
resolves one lockfile for every target and every member, so a dependency
that only exists on iOS still collides with Freya's fork of the same native
library. No feature choice in this crate avoids it: `slint` alone would not
pull the winit backend in, but run-time loading needs `slint-interpreter`,
and Slint's own `live-preview` feature goes through the interpreter too.
Freya's builds are blocked by this crate in the same way.

The fix is in the workspace, which I was told not to touch: give `freya` and
`slint` a lockfile each (drop them from `members`, add an `exclude`, and a
`[workspace]` table in each crate's manifest) or split the spike into two
workspaces that share `base` by path. Everything below was built from a
scratch workspace outside the repo (`base` and `slint` symlinked in, same
`[workspace.dependencies]` and release profile, the repo's `Cargo.lock` as
the starting point), so nothing in the repo was changed for it.

## How it works

- `Platform` hands out one `MinimalSoftwareWindow` for every instance the
  interpreter creates, which is also how a reload keeps the window.
- **Repaint buffer: `RepaintBufferType::ReusedBuffer`.** There is one CPU
  buffer and one GL texture and both persist, so the renderer only has to
  repaint what changed on top of last frame's pixels, and exactly those
  rectangles go to `ImportMem::update_memory`. `NewBuffer` would repaint
  and upload everything each frame; `SwappedBuffers` is for two alternating
  buffers, which this does not have.
- Pixels are `PremultipliedRgbaColor` (`R, G, B, A` bytes) so the texture is
  `Fourcc::Abgr8888`; the screenshot confirms the colours (`#4ade80` dot reads
  back as (74, 222, 128)). The base's clip shader expects premultiplied input,
  which this is.
- `update_memory` takes the whole buffer and a rectangle; smithay sets
  `UNPACK_ROW_LENGTH`/`SKIP_*`, so no per-rectangle copy is needed.
- **Drawing happens in `tick`, `render` only uploads.** `MinimalSoftwareWindow`
  only tells you it needed a redraw by calling the draw closure, and `tick`
  has to say whether the texture will change, so `tick` runs
  `update_timers_and_animations()`, then `draw_if_needed`, and returns true
  when the dirty region is non-empty. Both sit inside the base's content time.
- Clock: Rust sets an `int seconds` (seconds since local midnight) when the
  second changes; Slint derives `HH:MM:SS` and `HH:MM` from it. One property
  write per second, nothing else wakes at rest.
- Progress bar: `animation-tick()` modulo 8 s, read only while the panel's
  opacity is above zero, so Slint stops animating once it has faded out.
- Cross-fade: a `states` block with `in`/`out` transitions (180 ms rest fade,
  220 ms after 80 ms in, 120 ms out), all `ease-out`.
- Relayout: `set_layout_size` writes `layout-width`/`layout-height`; the panel
  is that size, top-centred, `clip: true`, and its layouts reflow. Checked by
  grabbing frames mid-morph: at about 400 px wide the HH:MM sits at the
  shape's right edge and the slider is shorter.
- Input: `PointerMove` divides by the scale into a `LogicalPosition`;
  `PointerButton` becomes `PointerPressed`/`Released` at the last position;
  `PointerLeave` becomes `PointerExited`. Keys map BackSpace, Delete, Return,
  Tab, arrows, Home/End and the modifiers to `slint::platform::Key`; other
  keys send their text. Releases carry no text from the base, and Ctrl turns
  letters into control codes, so both fall back to `keysym_to_utf8`.
- Focus and caret: `WindowActiveChanged(true)` is sent at init (Slint only
  blinks the caret in an active window). The blink is Slint's own timer,
  `Platform::cursor_flash_cycle`, 1000 ms per cycle by default, so 500 ms on,
  500 ms off. The field drops focus when the island collapses
  (`changed expanded`), or the caret timer would keep redrawing an invisible
  panel at rest. `focus-search()` puts the caret after the text.

## Live reload: what I chose and why

Slint 1.18 has two options:

1. The `slint` crate's `live-preview` feature (`SLINT_LIVE_PREVIEW=1` at build
   time, with `slint!`/`include_modules!`). It swaps the compiled component for
   an interpreter-backed `LiveReloadingComponent` that watches the files. It
   does not fit here: it schedules reloads with `slint::spawn_local`, which
   needs an event loop the platform runs (this platform has none; the
   compositor drives Slint by hand), and on reload it re-applies only the
   properties set from Rust, so the slider value and the typed text would
   reset. It also still depends on `slint-interpreter`.
2. `slint-interpreter` with my own watcher. **Chosen.** `notify` watches the
   directory (editors save by rename, which drops a watch on the file
   itself), `poll_reload` waits for 30 ms of quiet, recompiles, creates a new
   instance on the same window, copies every public property of the old
   instance into the new one if it still exists (output properties refuse the
   write, which is right), re-focuses the field if it was focused, and forces
   one full repaint so nothing from the old tree is left in the buffer.

Measured:

- Compile and swap: 4.9 to 6.2 ms per reload (the log line).
- Save to pixels on screen: 96 ms writing the file in place, 80 ms writing a
  temporary file and renaming it (polling `grim` every 16 ms until the dot
  turned red). That is the 30 ms debounce, the base's 50 ms idle poll, and a
  frame.
- A syntax error prints the diagnostic, logs `reload failed, keeping the
  running island`, and the island keeps running; the next good save loads.
- Evidence: `shots/reload.png`. A `--shot` run, during which the file got a
  syntax error, then (in one save) the fill colour changed to blue, the
  column's spacing went from 10 px to 14 px, and a new `in property <string>
  badge` was added and shown in the header. The shot taken after the reload
  shows all three, with the state the shot had set before the reload still
  there: expanded, the slider at 0.6, `SHOT_TEXT` in the field, focused, caret
  at the end. The file was restored afterwards (diffed clean).

Reloads without a rebuild: anything in the `.slint` file: colours, layout,
new elements and components, new properties with defaults, states and
animations, globals. Needs a rebuild: anything the Rust side has to know
about: a new property Rust sets (`seconds`, `expanded`, the layout size, the
shot state), renaming one of those, a new callback Rust handles. Renaming a
property the old instance carried (`volume`) loses its value on that reload.
A reload restarts transitions from their defaults, so an expanded island
fades its panel in again (300 ms) after each save.

## Bench

Cross-fade, run 1:

```
== slint (cross-fade) ==
scale 1, texture 520x240
content init 4.7 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        385          0.97/2.15/6.67         1.13/2.35/10.66
Expanded     241          0.93/1.85/3.86          1.04/1.99/4.04
Rest           4          2.99/3.57/3.57          3.33/3.75/3.75
Idle          10          1.40/1.87/1.87          1.53/1.97/1.97
Morph     swap ms 32.12/33.01/36.36
Expanded  swap ms 32.22/32.99/33.55
Rest      swap ms 2.29/24.09/24.09
Idle      swap ms 0.06/32.80/32.80
idle frames over the last 10 s: 10
memory before content init: RSS 88.9 MB, PSS 55.3 MB
memory after content init:  RSS 98.6 MB, PSS 64.8 MB
memory at end:              RSS 114.9 MB, PSS 80.5 MB
gpu: 38MiB
slint uploads: 549 frames, 792 rects, 47319 px per frame, texture 124800 px
```

Cross-fade, run 2:

```
== slint (cross-fade) ==
scale 1, texture 520x240
content init 4.8 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        389          1.05/1.99/2.74          1.25/2.16/3.29
Expanded     237          1.23/1.92/3.14          1.32/2.09/3.28
Rest          13          1.34/2.27/3.49          1.74/2.42/3.94
Idle          10          1.69/2.51/2.51          1.81/2.68/2.68
Morph     swap ms 32.04/32.98/33.29
Expanded  swap ms 31.97/32.94/33.27
Rest      swap ms 0.06/1.77/29.16
Idle      swap ms 0.05/32.63/32.63
idle frames over the last 10 s: 10
memory before content init: RSS 88.8 MB, PSS 55.5 MB
memory after content init:  RSS 98.9 MB, PSS 65.4 MB
memory at end:              RSS 115.5 MB, PSS 81.0 MB
gpu: 38MiB
slint uploads: 558 frames, 882 rects, 49267 px per frame, texture 124800 px
```

Relayout, run 1:

```
== slint (relayout) ==
scale 1, texture 520x240
content init 5.1 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        385          1.38/2.06/3.05          1.51/2.27/3.17
Expanded     244          1.07/1.85/3.50          1.20/2.02/3.62
Rest          12          1.32/1.96/3.67          1.46/2.09/4.06
Idle          11          1.18/1.86/1.86          1.27/1.99/1.99
Morph     swap ms 31.84/32.91/33.50
Expanded  swap ms 32.13/32.89/33.40
Rest      swap ms 0.05/1.51/18.84
Idle      swap ms 0.05/32.20/32.20
idle frames over the last 10 s: 11
memory before content init: RSS 88.9 MB, PSS 55.4 MB
memory after content init:  RSS 98.6 MB, PSS 65.0 MB
memory at end:              RSS 115.3 MB, PSS 80.8 MB
gpu: 38MiB
slint uploads: 581 frames, 1326 rects, 52209 px per frame, texture 124800 px
```

Relayout, run 2:

```
== slint (relayout) ==
scale 1, texture 520x240
content init 4.8 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        385          1.34/2.18/3.01          1.46/2.34/3.61
Expanded     236          1.10/1.84/2.75          1.22/2.01/2.89
Rest          13          1.48/2.69/4.13          1.64/2.82/4.51
Idle          10          1.37/2.06/2.06          1.53/2.23/2.23
Morph     swap ms 31.90/32.96/33.54
Expanded  swap ms 32.14/32.92/33.43
Rest      swap ms 0.06/1.37/19.04
Idle      swap ms 0.05/32.56/32.56
idle frames over the last 10 s: 10
memory before content init: RSS 89.0 MB, PSS 55.6 MB
memory after content init:  RSS 98.7 MB, PSS 65.1 MB
memory at end:              RSS 115.1 MB, PSS 80.6 MB
gpu: 38MiB
slint uploads: 583 frames, 1123 rects, 43763 px per frame, texture 124800 px
```

Empty base (`island-empty --bench`, same scratch workspace and profile):

```
== empty (cross-fade) ==
scale 1, texture 520x240
content init 0.2 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        399          0.76/1.72/2.75          0.89/2.09/3.47
Expanded       0                       -                       -
Rest           2          0.98/0.98/0.98          2.09/2.09/2.09
Idle           1          0.51/0.51/0.51          0.62/0.62/0.62
Morph     swap ms 32.36/33.11/33.79
Rest      swap ms 27.63/27.63/27.63
Idle      swap ms 32.69/32.69/32.69
idle frames over the last 10 s: 1
memory before content init: RSS 81.0 MB, PSS 48.3 MB
memory after content init:  RSS 81.4 MB, PSS 48.5 MB
memory at end:              RSS 83.7 MB, PSS 50.7 MB
gpu: 38MiB
```

The last line of each Slint run is mine (printed when the content drops):
frames that uploaded anything, rectangles, and average pixels per upload
against the 124,800-pixel texture.

Reading it:

- Content time p50 is 1.0 to 1.4 ms against the empty base's 0.76 ms, which
  is mostly the `glFinish` the base adds in bench mode. Slint's own share is
  a few tenths of a millisecond per frame; p95 stays near 2 ms and the worst
  frame in four runs was 6.7 ms (the first frame of a morph, once).
- Relayout costs about 0.35 ms more p50 than cross-fade during the morph.
- A clock tick at rest repaints two digit cells, 2,394 pixels in two
  rectangles, and takes about 0.5 ms to draw (traced once with a temporary
  timer, then removed).
- Idle: 10 to 11 frames in the last 10 s, one per clock second. The empty base
  draws 1.
- Memory: content init adds about 10 MB RSS/PSS (compiling the `.slint` file
  and creating the instance), and RSS grows another 16 MB during the run
  (font collection, glyph caches). At the end: RSS 115 MB, PSS 81 MB against
  the empty base's 84 MB and 51 MB: about +31 MB RSS, +30 MB PSS. The binary
  maps 8 MB more before any content runs.
- GPU: 38 MiB, the same as the empty base: one 520x240 texture.
- Init: 4.7 to 5.1 ms, including the run-time compile.
- Swap is about 32 ms in every phase for both this and the empty base, on a
  60 Hz monitor: frames land every other vblank. That is the base on this
  desktop, not the content.

## Size of the thing

- Glue (Rust that is not the UI): `src/main.rs` 394 lines, 343 without blanks
  and comment lines. Roughly: platform and window 25, compile and reload 75,
  key mapping 35, input 30, draw and upload 70, the trait and setup 80, upload
  stats 20. Plus `Cargo.toml`, 16 lines.
- UI: `ui/island.slint` 232 lines, 209 without blanks and comments.
- Clean release build of the whole tree (base, smithay, Slint, interpreter)
  in a fresh `CARGO_TARGET_DIR` on 16 threads: 45 s. A first build while the
  Freya agent was compiling Skia took 2 min 7 s.
- Release binary (with the workspace's `debug = 1`): 195 MB; stripped 26 MB.
  The empty base: 60 MB, stripped 6.1 MB. So Slint plus its compiler adds
  about 20 MB stripped; the interpreter carries the whole Slint compiler.
- 268 crates in the normal dependency tree against the base's 131.

## Text

- `default-font-family: "Inter"` on the window picks Inter (`Inter.ttc`); the
  screenshot is Inter throughout, weights 400 and 600 both right.
- **No tabular figures.** Slint 1.18 has no font-feature property (parley is
  given family, weight, style, size, letter spacing, line height and wrap
  only), so `tnum` cannot be asked for. The clock is laid out by hand: eight
  fixed-width cells (9.4 px per digit, 4.5 px per colon). It holds still each
  second, but the spacing is mine, not the font's.
- CJK falls back to Noto Sans CJK through fontique: 東京 renders correctly.
- **No colour emoji.** The software renderer rasterises outlines only
  (`fonts/vectorfont.rs` asks swash for `Source::Outline` in `Format::Alpha`),
  and Noto Color Emoji has no outlines, so 🎧 comes out as a monochrome
  outline glyph from a fallback font, in the text colour. ✓ is fine.
- Text is grey-scale anti-aliased and crisp at 1x; compare the 12 to 13 px
  secondary text against Freya's shot side by side before judging weight.

## Problems hit

1. **Workspace `links = "skia"` conflict** with Freya (above). Found when a
   rebuild failed in resolution; traced through the registry manifests. Not
   fixable inside this crate; worked around with a scratch workspace.
2. **No way to ask the window whether it needs a redraw** without drawing, so
   drawing moved into `tick`. Found reading `minimal_software_window.rs`.
3. **Caret at the start** after setting the text from Rust for the shot: a
   text set by property does not move the cursor. Fixed in `focus-search()`
   with `set-selection-offsets` past the end (Slint clamps it). Seen in the
   first screenshot.
4. **Caret timer at rest**: the field would stay focused under an invisible
   panel and keep the blink timer redrawing. Prevented by clearing focus on
   collapse; idle shows 10 frames in 10 s, one per second.
5. **Compile time hidden from `content init`** in my first version (compiled
   in `new()`, before the base starts timing). Moved into `init`; init went
   from 0.2 to about 5 ms and the memory before/after init became honest.
6. `live-preview` looked like the obvious choice and is not usable with a
   hand-driven platform (above); found by reading `i-slint-live-preview`.

Still wrong or unverified:

- Tabular figures and colour emoji, as above: limits of Slint 1.18's software
  renderer, not something the glue can fix.
- I did not type into or drag the running island: injecting keys or moving
  the pointer on the live desktop could have hit another window. The key and
  pointer mapping is written and compiles; the text-field and slider state
  come from properties in the shot. Someone should click, drag and type once.
- A reload replays the panel's fade-in when expanded.
- The 16 MB RSS growth during the run is not broken down; font loading and
  glyph caches are the likely share, but I did not measure it.

## Verdict

Pleasant. The `.slint` file is short and reads like the spec: states,
transitions and `animation-tick()` cover every animation with no Rust, and
the software renderer's dirty tracking turns into partial texture uploads
with almost no code: a clock tick uploads 2,400 pixels, not 125,000. Live
reload is the standout: 80 to 100 ms from save to screen, with state kept,
and it took about 75 lines once I stopped trying to use the built-in feature.
The costs are real: the interpreter brings the whole Slint compiler (+20 MB
stripped binary, +30 MB PSS), there are no font features (hand-made tabular
clock) and no colour emoji in the software path, and the interpreter's
iOS-only Skia dependency makes it unable to share a lockfile with Freya. For
a shell whose text includes notification bodies with emoji, the colour emoji
gap matters more than any of the frame times, which are comfortably small.

## Round 2: the full-output launcher

`launcher-slint`, second binary in this crate: the same software renderer and
dirty-rectangle upload path on a 3440x1440 texture, the UI in
`ui/launcher.slint`, loaded at run time and live-reloaded like the island.
AMD Ryzen 7 9800X3D (AVX2, AVX-512), RTX 5070 Ti.

### What changed in the crate

- The glue moved into `src/lib.rs` (`Surface`): platform and window, compile
  and reload, input mapping, `draw()` (timers, animations, rasterise into the
  CPU buffer) and `upload()` (dirty rectangles to the texture). `src/main.rs`
  (island) and `src/launcher.rs` are thin `IslandContent` impls over it.
  `Surface::on_instance` runs on every new instance, so a reload rebinds the
  model and the callback.
- `upload()` now records, per kind of frame the content names, the draw time
  (Slint rasterising), the upload time (the `glTexSubImage2D` calls, CPU side,
  no finish) and the dirty pixels. That table is printed after the base's.
- The island is unchanged in behaviour; one cross-fade run after the refactor
  matched Round 1 (content p50 0.88 ms morph, 0.90 ms expanded, 10 idle frames).

### How the launcher is built

- **App list and filtering: on the Rust side.** The `.slint` language has no
  way to filter a model or test whether a string contains another, so the
  idiomatic split is a Rust model. Rust holds the 48 apps as `App` structs
  (`name`, `color`, the colour computed from `hsl(i x 360 / 48, 55%, 55%)` once)
  and a `VecModel` of the first 24 matches, bound to `in property <[App]> apps`.
  The `TextInput`'s `edited` callback calls `query-edited(text)`; Rust filters
  case-insensitively and calls `set_vec` only when the matches change, so a
  keystroke that keeps the same tiles does not touch the grid.
- The grid is a `for app[i] in apps` repeater placing 160x140 tiles by index
  (6 columns, 16 px gaps) in a box centred in the panel. Tile: a 64 px square
  with radius 16, the name 14 px below it with a 12 px gap, one line, elided.
- Open and close are a `states` block with `in` (260 ms ease-out) and `out`
  (180 ms ease-in) transitions on the backdrop's opacity, the panel's opacity,
  its offset (24 px to 0) and its `transform-scale` (0.96 to 1). The durations
  are `root.animated ? 260ms : 0ms`; `set_animated(false)` sets `animated`, so
  `--comp-anim` shows and hides in one frame.
- Opening clears the query and focuses the field (public `focus-search()`),
  closing drops focus (`blur-search()`), both called from Rust in
  `set_expanded`. I first reached for `changed open => {...}` in Slint, but
  change handlers run deferred, after `prepare_shot` had already set `co`, and
  would have cleared it.
- `prepare_shot`: open, query `co` (Discord, Code, Contacts), focused.
- Live reload works the same way: `shots/launcher-reload.png` is a `--shot`
  run where the file was saved at 0.8 s with a blue panel and round tiles;
  reload took 3.7 ms and kept open, the query `co`, the filtered model, the
  callback and the focus (the caret is visible).
- Scale: the spec's sizes are logical at scale 1 on the output-sized texture,
  so the launcher always tells Slint scale 1.

### Shot

`spikes/shell-ui/shots/launcher-slint.png` matches the spec: backdrop, panel,
border, field, three tiles, colours and Inter. Two things it cannot show:

- **The caret is missing from the shot.** It is there and blinking (seen in
  `grim` grabs during the bench, and in the reload shot), but the base takes
  the shot at exactly 2.0 s, which falls on the blink boundary; in four
  shots it was in its off half each time. Not a bug in the content.
- **`transform-scale` does nothing.** The software renderer has no
  transforms (`supports_transformations()` is false and `scale()` is a
  TODO in `i-slint-renderer-software` 1.18.1), so the panel fades and slides
  but does not grow from 0.96. The property is in the file because it is the
  idiomatic way and works on the FemtoVG and Skia renderers.
- Related: opacity on the panel is applied per item (the renderer multiplies
  each child's alpha), not to the group, so during the fade the panel's fill
  shows through the tiles slightly. Only visible mid-fade.

### Base issue to report (affects both variants)

Under `--bench` the base types an extra `c` the instant the launcher opens:
the typing script compares against `toggled_at` before that iteration resets
it, so `since` is the time since the previous toggle (2 s) and the first step
fires at once. The field goes `c`, `cc`, `cco`, `cc` on every open, and the
extra keystroke lands in a Morph frame. Seen in `grim` frames of the field. I
did not change the base; the numbers below include it.

### Bench

Content animates, run 1:

```
== slint (launcher, content animates) ==
scale 1, texture 3440x1440
content init 10.1 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        181       12.03/15.25/19.53       12.13/15.53/19.66
Expanded      35          1.54/2.49/2.75          1.66/2.59/2.85
Rest           3          0.81/3.84/3.84          5.65/6.95/6.95
Idle           0                       -                       -
Morph     swap ms 4.62/16.46/16.87  ~33 fps
Expanded  swap ms 0.05/0.08/0.09  ~59 fps
Rest      swap ms 5.04/23.30/23.30  ~99 fps
idle frames over the last 10 s: 0
memory before content init: RSS 89.3 MB, PSS 40.0 MB
memory after content init:  RSS 137.2 MB, PSS 87.6 MB
memory at end:              RSS 141.6 MB, PSS 91.4 MB
gpu: 58MiB
slint uploads (4953600 px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max
  closed                          1      1        2.23/2.23/2.23        0.92/0.92/0.92  4953.60/4953.60/4953.60
  closing                        48     48      9.89/10.42/10.54        2.00/3.18/3.87  4953.60/4953.60/4953.60
  open (typing, caret)           35     49        0.36/1.81/2.15        0.11/0.24/0.29     52.99/446.74/446.74
  opening                        63     63     10.51/11.66/16.20        1.90/3.15/4.04  4953.60/4953.60/4953.60
```

Content animates, run 2 (disturbed: the machine's load average was about 20
from another build, and the window stalled for 12 s; kept for completeness):

```
== slint (launcher, content animates) ==
scale 1, texture 3440x1440
content init 12.5 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        129       11.70/15.34/46.93       11.81/15.50/47.17
Expanded      23          1.31/2.88/3.25          1.40/2.98/3.35
Rest           3          0.89/6.92/6.92          1.49/7.31/7.31
Idle           0                       -                       -
Morph     swap ms 5.00/16.42/16.87  ~33 fps
Expanded  swap ms 0.06/0.11/12388.97  ~67 fps
Rest      swap ms 2.06/24.00/24.00  ~117 fps
idle frames over the last 10 s: 0
memory before content init: RSS 89.3 MB, PSS 36.5 MB
memory after content init:  RSS 133.2 MB, PSS 82.4 MB
memory at end:              RSS 145.2 MB, PSS 91.9 MB
gpu: 58MiB
slint uploads (4953600 px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max
  closed                          1      1        2.34/2.34/2.34        2.58/2.58/2.58  4953.60/4953.60/4953.60
  closing                        31     31      9.72/10.32/10.85        2.08/3.09/3.13  4953.60/4953.60/4953.60
  open (typing, caret)           23     33        0.41/1.88/2.23        0.13/0.25/0.28     52.99/446.74/446.74
  opening                        45     45     10.54/11.85/12.20        2.17/3.39/3.90  4953.60/4953.60/4953.60
```

Content animates, run 3 (load settling):

```
== slint (launcher, content animates) ==
scale 1, texture 3440x1440
content init 10.7 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        181       11.94/14.82/16.33       12.05/14.94/16.46
Expanded      35          1.32/2.91/3.11          1.46/3.01/3.20
Rest           2          4.38/4.38/4.38          5.22/5.22/5.22
Idle           0                       -                       -
Morph     swap ms 4.79/16.27/17.22  ~33 fps
Expanded  swap ms 0.06/0.13/1.96  ~59 fps
Rest      swap ms 24.84/24.84/24.84  ~75 fps
idle frames over the last 10 s: 0
memory before content init: RSS 89.0 MB, PSS 39.6 MB
memory after content init:  RSS 132.8 MB, PSS 83.1 MB
memory at end:              RSS 140.9 MB, PSS 90.6 MB
gpu: 58MiB
slint uploads (4953600 px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max
  closed                          1      1        1.84/1.84/1.84        1.74/1.74/1.74  4953.60/4953.60/4953.60
  closing                        48     48      9.80/10.51/10.66        2.07/3.09/4.03  4953.60/4953.60/4953.60
  open (typing, caret)           35     49        0.48/1.84/1.92        0.16/0.21/0.23     52.99/446.74/446.74
  opening                        63     63     10.31/11.34/11.76        1.76/2.98/3.43  4953.60/4953.60/4953.60
```

Compositor animates (`--comp-anim`), run 1:

```
== slint (launcher, compositor animates) ==
scale 1, texture 3440x1440
content init 22.4 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        182        0.86/13.43/17.20        0.96/13.52/17.33
Expanded      35          1.54/2.66/3.53          1.69/2.77/3.66
Rest           4        1.39/11.98/11.98        1.48/12.46/12.46
Idle           0                       -                       -
Morph     swap ms 15.74/16.80/17.75  ~33 fps
Expanded  swap ms 0.05/0.09/0.11  ~61 fps
Rest      swap ms 1.40/27.14/27.14  ~35 fps
idle frames over the last 10 s: 0
memory before content init: RSS 89.3 MB, PSS 40.0 MB
memory after content init:  RSS 133.1 MB, PSS 83.8 MB
memory at end:              RSS 137.2 MB, PSS 83.3 MB
gpu: 58MiB
slint uploads (4953600 px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max
  closed                          1      1        2.02/2.02/2.02        4.14/4.14/4.14  4953.60/4953.60/4953.60
  closing                        14     14        0.92/1.06/1.11        2.18/3.35/3.94  4953.60/4953.60/4953.60
  open (typing, caret)           35     49        0.41/1.70/2.30        0.14/0.22/0.30     52.99/446.74/446.74
  opening                        14     14     10.93/11.20/11.29        2.05/3.20/3.58  4953.60/4953.60/4953.60
```

Compositor animates, run 2 (same disturbance as content run 2):

```
== slint (launcher, compositor animates) ==
scale 1, texture 3440x1440
content init 19.7 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        154        0.62/33.60/52.07        0.84/33.79/52.96
Expanded      30         1.49/7.83/18.05         2.33/9.66/18.18
Rest           4        1.46/12.85/12.85        2.10/13.67/13.67
Idle           0                       -                       -
Morph     swap ms 15.79/17.97/3940.53  ~33 fps
Expanded  swap ms 0.07/1.13/1.90  ~170 fps
Rest      swap ms 5.23/7.66/7.66  ~406 fps
idle frames over the last 10 s: 0
memory before content init: RSS 89.3 MB, PSS 37.0 MB
memory after content init:  RSS 133.3 MB, PSS 80.7 MB
memory at end:              RSS 137.7 MB, PSS 87.5 MB
gpu: 58MiB
slint uploads (4953600 px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max
  closed                          1      1        7.33/7.33/7.33        4.74/4.74/4.74  4953.60/4953.60/4953.60
  closing                        13     13        0.80/2.71/4.74      4.35/10.25/12.39  4953.60/4953.60/4953.60
  open (typing, caret)           30     46       0.59/7.18/17.53        0.17/1.04/1.41     52.99/470.54/636.43
  opening                        14     14     27.76/39.69/40.50       3.34/6.75/10.58  4953.60/4953.60/4953.60
```

Compositor animates, run 3:

```
== slint (launcher, compositor animates) ==
scale 1, texture 3440x1440
content init 10.9 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        182        0.83/13.99/16.63        0.95/14.13/16.80
Expanded      35          1.45/2.53/3.09          1.57/2.69/3.21
Rest           2          4.54/4.54/4.54          4.91/4.91/4.91
Idle           0                       -                       -
Morph     swap ms 15.86/16.71/17.66  ~33 fps
Expanded  swap ms 0.05/0.09/0.10  ~60 fps
Rest      swap ms 22.79/22.79/22.79  ~79 fps
idle frames over the last 10 s: 0
memory before content init: RSS 89.1 MB, PSS 39.7 MB
memory after content init:  RSS 132.8 MB, PSS 83.1 MB
memory at end:              RSS 136.8 MB, PSS 86.6 MB
gpu: 58MiB
slint uploads (4953600 px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max
  closed                          1      1        1.91/1.91/1.91        2.08/2.08/2.08  4953.60/4953.60/4953.60
  closing                        14     14        0.65/1.01/1.07        2.33/3.17/3.28  4953.60/4953.60/4953.60
  open (typing, caret)           35     49        0.46/1.64/1.68        0.14/0.27/0.36     52.99/446.74/446.74
  opening                        14     14     10.75/11.51/11.74        2.47/3.06/3.07  4953.60/4953.60/4953.60
```

Floor (`island-empty --launcher --bench`):

```
== empty (launcher, content animates) ==
scale 1, texture 3440x1440
content init 5.9 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        182          0.76/1.48/2.05          0.90/1.62/2.15
Expanded       0                       -                       -
Rest           2          1.38/1.38/1.38          1.75/1.75/1.75
Idle           0                       -                       -
Morph     swap ms 15.82/16.51/16.66  ~33 fps
Rest      swap ms 29.91/29.91/29.91  ~63 fps
idle frames over the last 10 s: 0
memory before content init: RSS 81.0 MB, PSS 33.9 MB
memory after content init:  RSS 98.2 MB, PSS 50.9 MB
memory at end:              RSS 95.6 MB, PSS 48.3 MB
gpu: 58MiB
```

### What the numbers say

Taking the clean runs (1 and 3 of each):

- **Transition, content animates:** content p50 about 12 ms, p95 15 ms. Every
  transition frame dirties the whole 4.95 Mpx surface (the backdrop's opacity
  changes): Slint draws it in 10.3 to 10.5 ms (opening) or 9.8 ms (closing),
  and the upload of 19.8 MB takes 1.8 to 2.1 ms of CPU. That fits 60 Hz with
  4 ms to spare and does not fit 120 Hz.
- **Transition, compositor animates:** content p50 0.83 ms, because the
  content draws nothing while the compositor fades. The p95 (14 ms) is the one
  full repaint per open (10.75 ms draw + 2.3 ms upload); closing is a full
  clear (0.65 ms draw) plus a full upload (2.3 ms), which a real compositor
  could skip by not drawing a hidden surface at all.
- **Typing (Expanded):** content p50 1.3 to 1.5 ms, p95 under 3 ms. Dirty
  region per frame: 53 kpx p50 (caret and field text, about 1% of the
  surface) and 447 kpx p95 (the grid changing: about 9%). Draw 0.4 ms p50,
  1.8 ms p95; upload 0.15 ms.
- **Dirty regions by frame kind:** opening and closing (content animates):
  4,954 kpx every frame, 1 rectangle. Opening (compositor animates): 4,954
  kpx once per open. Typing: 53 kpx p50, 447 kpx p95, 1 to 2 rectangles.
  Closed: nothing; idle frames 0.
- **Memory over the floor (end of run):** RSS 137 to 145 MB against 95.6 MB,
  so +41 to +50 MB; PSS 87 to 92 MB against 48 MB, so +38 to +43 MB. Of that,
  19.8 MB is the CPU buffer itself, which only the CPU path pays; about 10 MB
  is Slint and the interpreter as in Round 1; the rest is fonts and caches.
- **GPU:** 58 MiB, the same as the floor: the one texture, nothing else.
- **Init:** 10 to 22 ms (compile, instance, and allocating and importing the
  20 MB buffer).

### Extra glue

`src/launcher.rs` is 179 lines, 151 without blanks and comments: the app list
(10), the colour (17), the filtered model (35), the trait impl and setup
(the rest). The shared `src/lib.rs` is 355 code lines (the Round 1 glue plus
about 40 lines of per-kind statistics); the island's `src/main.rs` shrank to
56. `ui/launcher.slint` is 132 code lines. Stripped `launcher-slint`: 26 MB,
the same as the island.

### How the CPU path scales to the full output: the SIMD check

Slint has no thread knob: the software renderer is single-threaded (no rayon,
no worker pool, the renderer is not `Send`) and has no explicit SIMD; its
fill and blend loops rely on the compiler vectorising them. So the knob is
`target-cpu`. Same code, same run, content animates:

```
== slint (launcher, content animates) ==
scale 1, texture 3440x1440
content init 10.2 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        182          3.89/6.79/8.02          4.01/6.92/8.17
Expanded      35          1.19/1.87/2.01          1.36/2.01/2.11
Rest           2          5.07/5.07/5.07          5.39/5.39/5.39
Idle           0                       -                       -
Morph     swap ms 12.63/16.48/16.75  ~33 fps
Expanded  swap ms 0.06/0.09/0.15  ~60 fps
Rest      swap ms 21.50/21.50/21.50  ~85 fps
idle frames over the last 10 s: 0
memory before content init: RSS 89.3 MB, PSS 39.9 MB
memory after content init:  RSS 133.3 MB, PSS 83.7 MB
memory at end:              RSS 141.8 MB, PSS 91.6 MB
gpu: 58MiB
slint uploads (4953600 px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max
  closed                          1      1        1.89/1.89/1.89        1.81/1.81/1.81  4953.60/4953.60/4953.60
  closing                        49     49        1.53/2.34/2.80        1.93/2.43/3.36  4953.60/4953.60/4953.60
  open (typing, caret)           35     49        0.16/0.30/0.31        0.14/0.22/0.28     52.99/446.74/446.74
  opening                        63     63        2.06/3.19/3.92        1.85/3.19/3.87  4953.60/4953.60/4953.60
```

That is `RUSTFLAGS="-C target-cpu=native"` (Zen 5: AVX-512). And with
`-C target-cpu=x86-64-v3` (AVX2; CachyOS's own packages target v3 or v4):

```
== slint (launcher, content animates) ==
scale 1, texture 3440x1440
content init 10.8 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        182          4.49/7.48/9.79          4.65/7.62/9.96
Expanded      35          0.93/1.65/2.17          1.03/1.79/2.38
Rest           2          3.98/3.98/3.98          4.40/4.40/4.40
Idle           0                       -                       -
Morph     swap ms 11.83/16.42/17.96  ~33 fps
Expanded  swap ms 0.06/0.09/0.10  ~59 fps
Rest      swap ms 25.32/25.32/25.32  ~72 fps
idle frames over the last 10 s: 0
memory before content init: RSS 89.2 MB, PSS 39.9 MB
memory after content init:  RSS 137.5 MB, PSS 87.8 MB
memory at end:              RSS 142.0 MB, PSS 91.7 MB
gpu: 58MiB
slint uploads (4953600 px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max
  closed                          1      1        1.87/1.87/1.87        0.99/0.99/0.99  4953.60/4953.60/4953.60
  closing                        49     49        2.34/3.29/3.76        1.98/2.89/3.13  4953.60/4953.60/4953.60
  open (typing, caret)           35     49        0.19/0.43/0.62        0.14/0.21/0.27     52.99/446.74/446.74
  opening                        63     63        2.87/3.79/4.09        1.91/3.17/4.81  4953.60/4953.60/4953.60
```

And native with `--comp-anim`:

```
== slint (launcher, compositor animates) ==
scale 1, texture 3440x1440
content init 10.5 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        182          0.73/4.78/6.81          0.84/4.89/6.95
Expanded      35          1.29/1.72/1.88          1.49/1.97/3.59
Rest           2          4.34/4.34/4.34          4.63/4.63/4.63
Idle           0                       -                       -
Morph     swap ms 15.84/16.69/17.46  ~33 fps
Expanded  swap ms 0.06/0.14/0.18  ~61 fps
Rest      swap ms 17.86/17.86/17.86  ~101 fps
idle frames over the last 10 s: 0
memory before content init: RSS 89.4 MB, PSS 40.0 MB
memory after content init:  RSS 133.4 MB, PSS 83.8 MB
memory at end:              RSS 149.8 MB, PSS 99.6 MB
gpu: 58MiB
slint uploads (4953600 px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max
  closed                          1      1        2.01/2.01/2.01        1.79/1.79/1.79  4953.60/4953.60/4953.60
  closing                        14     14        0.66/1.18/1.50        2.18/2.41/2.54  4953.60/4953.60/4953.60
  open (typing, caret)           35     49        0.17/0.36/0.55        0.13/0.22/0.36     52.99/446.74/446.74
  opening                        14     14        2.46/3.30/3.70        1.32/2.03/2.53  4953.60/4953.60/4953.60
```

| build | full-surface draw (opening) p50 | transition content p50 / p95 | typing draw p50 |
|---|---|---|---|
| default x86-64 | 10.3 ms | 12.0 / 14.8 ms | 0.48 ms |
| x86-64-v3 (AVX2) | 2.87 ms | 4.5 / 7.5 ms | 0.19 ms |
| native (AVX-512) | 2.06 ms | 3.9 / 6.8 ms | 0.16 ms |

The default build leaves a 3.5 to 5x speed-up on the table. With v3, a
full-surface repaint plus upload is about 5 ms, which fits 120 Hz with
headroom and 144 Hz just. The upload (about 2 ms CPU for 20 MB, plus the
copy over PCIe) does not change with the flag and becomes the bigger share.

Candid read: the CPU path scales linearly with dirty pixels, and Slint's
dirty tracking is good at keeping them small: typing on a full-screen launcher
costs about what the island costs. The cost is the frames where the whole
surface changes. Built for the CPU it runs on, those are 4 to 5 ms at
3440x1440, fine at 60 Hz and workable at 120 Hz; built generic, 12 ms, which
is not. Either way it is one core busy for the whole transition and 20 MB
crossing the bus per frame, where a GPU renderer would do the same in a
fraction of a millisecond. The `--comp-anim` design removes nearly all of it
(one full repaint per open), and full-output surfaces like the launcher,
overview and lock screen are exactly where the compositor should own the
transition. Two limits are not about speed: no transforms (the panel's scale
cannot be done in the content at all) and per-item opacity.
