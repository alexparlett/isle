# island-freya: notes

Freya (git `cfd97ac`, 0.5.0-rc.8) embedded in the toy compositor: `freya_core::Runner` + `Tree`
driven by hand, `RenderPipeline` drawing through Skia (Freya's `freya-skia-safe` fork, Ganesh GL)
into an FBO on Smithay's own EGL context. The FBO's colour attachment is a GL texture wrapped once
with `GlesTexture::from_raw`; the base samples it directly. No CPU copy, no upload.

## Building and running

The crate cannot live in the shared workspace (see "Problems" 1). It is its own workspace:

```
cd spikes/shell-ui/freya/island
cargo build --release          # writes ../../target/release/island-freya via .cargo/config.toml
cd ../..
./target/release/island-freya --bench
./target/release/island-freya --bench --relayout
./target/release/island-freya --shot shots/freya.png
```

`freya/Cargo.toml` is now a lib-only placeholder (`island-freya-stub`) so the outer workspace's
`members = ["freya"]` still resolves. `freya/island/Cargo.toml` is the real package and points its
bin at `freya/src/main.rs`. It has its own `Cargo.lock`.

## Bench output

Machine: RTX 5070 Ti, the window on the live Hyprland desktop at scale 1. Every variant shows swap
at ~32 ms, i.e. the window is being paced at ~30 Hz by the host compositor; that is the base, not
the content (the empty base shows the same).

### Cross-fade, run 1

```
== freya (cross-fade) ==
scale 1, texture 520x240
content init 16.8 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        389          0.80/2.12/2.56          0.92/2.30/4.15
Expanded     231          1.00/1.75/1.96          1.12/1.89/2.11
Rest          12          1.50/1.80/4.38          1.64/1.93/4.73
Idle          11          1.18/3.05/3.05          1.32/3.14/3.14
Morph     swap ms 32.32/33.11/33.89
Expanded  swap ms 32.19/32.99/33.21
Rest      swap ms 16.14/20.13/31.14
Idle      swap ms 0.05/33.26/33.26
idle frames over the last 10 s: 11
memory before content init: RSS 88.7 MB, PSS 55.1 MB
memory after content init:  RSS 105.9 MB, PSS 68.3 MB
memory at end:              RSS 113.2 MB, PSS 75.4 MB
gpu: 45MiB
```

### Cross-fade, run 2

```
== freya (cross-fade) ==
scale 1, texture 520x240
content init 18.1 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        391          0.87/2.07/4.19          1.00/2.21/4.31
Expanded     230          0.99/1.84/1.96          1.07/1.99/2.12
Rest          13          1.01/1.71/3.60          1.15/2.00/3.91
Idle          10          1.23/3.01/3.01          1.34/3.13/3.13
Morph     swap ms 32.27/33.08/34.19
Expanded  swap ms 32.13/32.95/33.44
Rest      swap ms 0.06/1.50/24.91
Idle      swap ms 0.06/33.22/33.22
idle frames over the last 10 s: 10
memory before content init: RSS 88.4 MB, PSS 54.8 MB
memory after content init:  RSS 105.8 MB, PSS 68.1 MB
memory at end:              RSS 113.3 MB, PSS 75.3 MB
gpu: 45MiB
```

### Relayout, run 1

```
== freya (relayout) ==
scale 1, texture 520x240
content init 17.2 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        385         1.27/2.15/29.62         1.39/2.31/29.81
Expanded     237          0.94/1.75/1.93          1.03/1.87/2.07
Rest           9          1.49/3.91/3.91          1.87/4.24/4.24
Idle          11          1.04/2.91/2.91          1.13/3.08/3.08
Morph     swap ms 32.01/33.03/33.28
Expanded  swap ms 32.20/32.99/34.00
Rest      swap ms 31.15/32.80/32.80
Idle      swap ms 0.06/32.93/32.93
idle frames over the last 10 s: 11
memory before content init: RSS 88.7 MB, PSS 55.1 MB
memory after content init:  RSS 105.8 MB, PSS 68.2 MB
memory at end:              RSS 178.4 MB, PSS 139.4 MB
gpu: 48MiB
```

### Relayout, run 2

```
== freya (relayout) ==
scale 1, texture 520x240
content init 17.4 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        389          1.30/2.14/4.24          1.42/2.36/4.34
Expanded     239          0.90/1.81/1.96          0.99/1.95/2.11
Rest          13          1.10/3.24/4.59          1.29/3.34/4.92
Idle          10          1.19/2.27/2.27          1.28/2.45/2.45
Morph     swap ms 31.92/32.95/33.43
Expanded  swap ms 32.26/32.92/33.31
Rest      swap ms 0.05/1.42/28.53
Idle      swap ms 0.05/32.43/32.43
idle frames over the last 10 s: 10
memory before content init: RSS 88.6 MB, PSS 54.9 MB
memory after content init:  RSS 105.6 MB, PSS 68.0 MB
memory at end:              RSS 113.6 MB, PSS 75.7 MB
gpu: 47MiB
```

### Empty base, for comparison

```
== empty (cross-fade) ==
scale 1, texture 520x240
content init 0.2 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        415          0.74/1.73/2.73          0.87/1.97/2.91
Expanded       0                       -                       -
Rest           2          0.65/0.65/0.65          2.09/2.09/2.09
Idle           1          0.73/0.73/0.73          0.90/0.90/0.90
Morph     swap ms 32.34/33.12/34.70
Rest      swap ms 24.72/24.72/24.72
Idle      swap ms 32.05/32.05/32.05
idle frames over the last 10 s: 1
memory before content init: RSS 80.9 MB, PSS 48.8 MB
memory after content init:  RSS 81.3 MB, PSS 49.0 MB
memory at end:              RSS 83.7 MB, PSS 51.2 MB
gpu: 38MiB
```

### Reading it

- Content time includes `glFinish` (the base adds it in bench mode), so it is CPU + GPU for
  Freya's diff, layout, Skia draw and flush. Morph p50 0.80-0.87 ms against the empty base's
  0.74 ms (which is mostly the `glFinish` round trip): Freya's share of a cross-fade frame is
  about 0.1-0.2 ms at p50, ~0.4 ms at p95. Relayout costs about 0.5 ms more at p50 (1.27-1.30).
- Idle: 10-11 frames in 10 s, one per clock second. Nothing else draws at rest.
- Memory over the empty base, end of run: about +30 MB RSS / +24 MB PSS (113 vs 84 MB RSS,
  75 vs 51 MB PSS). Content init alone is +17 MB RSS / +13 MB PSS (Skia context, font
  manager, fontconfig).
- GPU: 45-48 MiB against 38 MiB for the empty base, so +7-10 MiB.
- Relayout run 1 ended at 178 MB RSS and had one 29.6 ms frame; run 2 ended at 113.6 MB with a
  4.2 ms max, so it did not reproduce on the second run; I could not tell whether it was Skia's
  paragraph/glyph caches growing with the per-frame widths or an allocator artefact. Worth a
  longer relayout soak before trusting the memory number for that mode.

## Size of the work

- Glue (`src/main.rs`): 452 lines, 409 non-blank non-comment. That is the whole Freya platform
  layer: Runner/Platform/context setup (~95), the frame step (event dispatch, user events, task
  polling, diff, layout, accessibility/focus) (~65), the pointer/keyboard mapping (~55), GL/Skia
  setup and render (~110), GL state restore (~30), and the `IslandContent` plumbing.
- UI (`src/island.rs`): 400 lines, 363 code. Includes a 30-line `Delayed` animation type that
  exists only to work around an `AnimSequential` bug (Problems 3).
- Manifests: `freya/island/Cargo.toml`, `freya/island/.cargo/config.toml`, the stub
  `freya/Cargo.toml` + `freya/stub.rs`.

## Build

- Clean build of `island-freya` and all its deps, fresh `CARGO_TARGET_DIR`, warm cargo registry
  and git checkouts, 16 threads: **124 s** (249 crates). Skia is not compiled: `freya-skia-bindings`
  links a prebuilt Skia archive it downloads, so this number assumes network access at build time.
  The first build in the shared target dir took 3 m 01 s (it also fetched the Freya git repo and
  the Skia binaries).
- Release binary (`debug = 1`): 136 MB; stripped 25.0 MB. The empty base is 60 MB / 6.1 MB
  stripped, so Freya + Skia add ~19 MB stripped.

## Iteration loop

- Editing a colour in `island.rs` and rebuilding: **1.8-1.9 s** incremental (`cargo build
  --release`, only the bin crate recompiles). Restart to first frame is well under a second
  (content init is 17 ms; the rest is the winit window and EGL). Edit to pixels: about 2.5 s.
- Hot reload: Freya has a `hotreload` feature (subsecond hot-patching via `dx serve --hotpatch`),
  but it is wired only into `freya-winit` (it registers a subsecond handler and calls
  `runner.reload()` on the next poll). Embedded, it is not available out of the box; it would mean
  enabling `freya-core/hotreload`, registering the handler in the glue and calling
  `Runner::reload` + forcing a relayout, and running under the `dx` CLI. Not tried. There is no
  interpreted markup path like Slint's: a layout change is a Rust rebuild.

## Problems hit

1. **Freya and Slint cannot share the workspace.** `cargo build -p island-freya` failed at
   resolution: `freya-skia-bindings` and `skia-bindings` both declare `links = "skia"`, and
   `slint-interpreter` pulls `i-slint-backend-winit` -> `i-slint-renderer-skia` -> `skia-safe`
   through a `cfg(target_arch = "wasm32")` dependency. Cargo checks `links` across all targets and
   regardless of features, so `default-features = false` on the Slint side does not help. Found
   from the resolver error. Fixed without touching the workspace manifest by making
   `freya/island/` its own workspace (see Building). The cleaner fix is a workspace change:
   `exclude = ["freya"]` (or move the Freya crate out of `members`) and build it separately.
2. **Nothing drawn, first shot.** The texture path worked first time (a debug red rect drew in the
   right corner, so `SurfaceOrigin::TopLeft` is the correct orientation for Smithay's
   `Transform::Normal`), and the layout dump was correct, but the expanded layer's opacity stayed
   at 0. Found by printing the animated values.
3. **`AnimSequential` ends after its first step.** The hook checks `is_finished` before `advance`,
   and `AnimSequential::is_finished` only asks the current step; a leading delay step
   (`AnimNum::new(0., 0.).time(80)`) reports finished at 80 ms and the hook stops the task before
   the second step runs. Replaced with a small `Delayed` `AnimatedValue` (holds, then runs an
   `AnimNum` offset by the delay). Upstream bug, worth reporting.
4. **Emoji looked monochrome.** Turned out Noto Color Emoji's 🎧 is grey; fallback order is the
   spec's (Inter, Noto Sans, Noto Color Emoji, Noto Sans CJK JP) and works.
5. **The tick/redraw handshake.** Freya's animations and timers advance on its `RenderingTicker`
   (notified after each present) and ask for frames through `UserEvent::RequestRedraw`; there is
   no "next frame time" to report. `tick` drains handler events, platform user events, polls
   tasks (`sync_and_update`), applies mutations, re-measures layout and processes focus, looping
   up to four times so a handler that dirties state settles in the same frame; it returns true if
   the tree changed or a redraw was requested. `render` notifies the ticker. Timers
   (`async_io::Timer`) wake on the async-io thread and are only polled when the base calls `tick`
   (every 50 ms at rest), so the clock can be up to 50 ms late.
6. **Focus/keyboard needs the accessibility tree.** Keyboard events are routed to the focused node,
   which comes from `AccessibilityTree`; the glue has to run `process_updates` after layout and feed
   `Platform::focused_accessibility_id`, and turn `UserEvent::FocusAccessibilityNode` into
   `tree.accessibility_diff.request_focus`. This is plumbing freya-winit does and an embedder must
   copy.
7. **GL state handshake.** `DirectContext::reset(None)` before every draw; after the flush the
   glue restores FBO 0, VAO 0, buffer bindings, all vertex attrib arrays disabled, program 0,
   texture unit 0 with no texture or sampler, stencil/depth/cull/scissor off, blend on with
   `ONE, ONE_MINUS_SRC_ALPHA`, colour mask, pixel-store alignment and row length. No visual
   corruption seen in the shots or the bench runs. The FBO has an 8-bit stencil renderbuffer for
   Skia's clip paths.
8. **Teardown.** The base drops the backend (and the EGL context) before the content, so the glue
   calls `DirectContext::abandon()` in `Drop` to stop Skia touching a dead context. No crash on exit.

Verified interaction with a temporary in-process harness (removed): a press-and-drag on the slider
set the volume to 0.527 at x = 300 as computed, a click focused the field, typed `a b c`
then Backspace left `ab` with the caret shown, and collapsing the island unfocused the field.

### Still wrong or unverified

- The fades restart from their fixed origin if the state toggles mid-fade (each direction is its
  own `AnimNum`); with the bench's 1.5 s toggles it never shows. Clicking fast would show a jump.
- On collapse the progress bar resets to 0 during the 120 ms fade-out (`OnChange::Reset`), a
  barely visible jump.
- "Ease-out" is Freya's `Ease::Out` with `Function::Cubic`; CSS/Slint `ease-out` is a Bézier
  between quad and cubic, so the curves differ slightly from the Slint variant.
- The caret is a fixed 1.5 px bar after the text; no cursor movement, selection or IME. The field
  is a hand-built component, not Freya's `Input` (its blink is a 100 ms fade every 750 ms, not a
  500 ms blink, and it brings selection, scroll and clipboard machinery).
- No modifiers are passed (the base only gives a keysym and text).
- The relayout-mode memory jump in run 1 (above).

## Verdict

The rendering half was easier than expected: Skia on Smithay's context, an FBO over a texture
that Smithay samples, `reset` before and a state restore after, and it drew correctly with the
right orientation on the first try. Per-frame cost is tiny (a fraction of a millisecond over the
empty base), idle is genuinely idle, and text quality is Skia's. The cost is everywhere else:
Freya is built around `freya-winit`, and embedding it means re-implementing its platform layer by
reading its source (runner contexts, the ticker, user events, the event measurer/executor pair,
the accessibility tree just to get keyboard focus). None of it is documented as an embedding API;
`feature_embedded.rs` covers drawing only. The component model itself is pleasant once the glue
exists (hooks, state, `use_animation`), but I hit a real bug in `freya-animation` on day one, the
API is a moving 0.5 RC on git, and a Skia `links` clash with Slint forced a separate workspace.
Iteration is a 2 s Rust rebuild plus restart, with no embedded hot reload. Good engine, young and
winit-shaped framework: viable for the shell if we are willing to own ~400 lines of platform glue
and track upstream closely.

# Round 2: the full-output launcher

`launcher-freya` is a second binary in the same crate (`freya/island/Cargo.toml` now has a lib
target plus two bins). The round-1 glue moved, unchanged in substance, into `src/embed.rs`
(`Embedded`, generic over the root component and a model provided at the root); `src/main.rs` and
`src/launcher_main.rs` are thin `IslandContent` adapters. The launcher UI is `src/launcher.rs`; the
text field is shared from `src/widgets.rs`. Same path as the island: Freya's tree, Skia Ganesh GL
into an FBO on Smithay's EGL context, the texture handed to Smithay with no copy. The launcher
renders at scale 1 whatever the window's scale, because the spec is in output pixels.

```
cd spikes/shell-ui/freya/island && cargo build --release
cd ../.. && ./target/release/launcher-freya --bench [--comp-anim]
./target/release/launcher-freya --shot shots/launcher-freya.png
```

The screenshot matches the spec: backdrop, the 1200×760 panel with its 1 px border, the field
holding `co`, and Discord, Code, Contacts with hues 60°, 180° and 330°. The caret was in its off
phase in the captured frame; I checked focus and the blink with a temporary log.

## Bench output

These runs were made one at a time. My first attempt overlapped with the Slint agent's benches
(two windows, and the same scratchpad file names, so one of my files held Slint's output); those
runs showed 5-17 s swap stalls and 3-4x content times and are discarded. **The round-1 island
numbers above were probably taken under the same contention**: rerun now, alone and on the new
frame-callback base, the island's content p50 is 0.26 ms, not 0.8-1.0 ms. Those reruns are at
the end of this section.


### Launcher, content animates, run 1

```
== freya (launcher, content animates) ==
scale 1, texture 3440x1440
content init 0.8 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        181         0.27/1.41/34.05         0.34/1.47/34.18
Expanded      43          0.42/0.72/0.85          0.51/0.78/0.94
Rest           2          0.25/0.25/0.25          0.59/0.59/0.59
Idle           0                       -                       -
Morph     swap ms 16.34/16.71/18.06  ~33 fps
Expanded  swap ms 0.05/0.11/15.94  ~75 fps
Rest      swap ms 31.32/31.32/31.32  ~62 fps
idle frames over the last 10 s: 0
memory before content init: RSS 88.7 MB, PSS 43.3 MB
memory after content init:  RSS 92.2 MB, PSS 46.7 MB
memory at end:              RSS 117.4 MB, PSS 67.8 MB
gpu: 146MiB
```

### Launcher, content animates, run 2

```
== freya (launcher, content animates) ==
scale 1, texture 3440x1440
content init 0.7 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        182         0.26/1.94/12.53         0.35/2.07/12.64
Expanded      42          0.44/0.78/1.91          0.56/0.91/2.03
Rest           3          0.14/0.17/0.17          0.42/0.49/0.49
Idle           0                       -                       -
Morph     swap ms 16.32/16.82/19.77  ~33 fps
Expanded  swap ms 0.07/0.11/15.72  ~115 fps
Rest      swap ms 0.65/20.49/20.49  ~143 fps
idle frames over the last 10 s: 0
memory before content init: RSS 88.7 MB, PSS 43.3 MB
memory after content init:  RSS 92.3 MB, PSS 46.7 MB
memory at end:              RSS 119.6 MB, PSS 69.9 MB
gpu: 146MiB
```

### Launcher, compositor animates (--comp-anim), run 1

```
== freya (launcher, compositor animates) ==
scale 1, texture 3440x1440
content init 0.8 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        208         0.26/0.51/15.37         0.33/0.64/15.48
Expanded      48          0.41/0.92/1.15          0.49/1.01/1.46
Rest           2          0.17/0.17/0.17          0.77/0.77/0.77
Idle           0                       -                       -
Morph     swap ms 16.38/16.70/18.11  ~33 fps
Expanded  swap ms 0.05/0.10/0.13  ~103 fps
Rest      swap ms 17.64/17.64/17.64  ~108 fps
idle frames over the last 10 s: 0
memory before content init: RSS 88.6 MB, PSS 43.3 MB
memory after content init:  RSS 92.3 MB, PSS 46.8 MB
memory at end:              RSS 109.0 MB, PSS 59.5 MB
gpu: 90MiB
```

### Launcher, compositor animates (--comp-anim), run 2

```
== freya (launcher, compositor animates) ==
scale 1, texture 3440x1440
content init 0.7 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        182         0.26/0.47/12.98         0.32/0.65/13.07
Expanded      42          0.46/0.73/0.95          0.53/0.87/1.04
Rest           3          0.13/0.17/0.17          0.39/0.43/0.43
Idle           0                       -                       -
Morph     swap ms 16.35/16.70/21.54  ~33 fps
Expanded  swap ms 0.05/0.11/15.78  ~90 fps
Rest      swap ms 0.64/25.53/25.53  ~115 fps
idle frames over the last 10 s: 0
memory before content init: RSS 88.7 MB, PSS 43.3 MB
memory after content init:  RSS 92.4 MB, PSS 46.8 MB
memory at end:              RSS 109.4 MB, PSS 59.7 MB
gpu: 90MiB
```

### Floor: island-empty --launcher --bench

```
== empty (launcher, content animates) ==
scale 1, texture 3440x1440
content init 5.4 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        182          0.25/0.28/0.96          0.32/0.39/1.08
Expanded       0                       -                       -
Rest           2          0.22/0.22/0.22          1.03/1.03/1.03
Idle           0                       -                       -
Morph     swap ms 16.41/16.73/16.86  ~33 fps
Rest      swap ms 32.56/32.56/32.56  ~60 fps
idle frames over the last 10 s: 0
memory before content init: RSS 80.8 MB, PSS 39.3 MB
memory after content init:  RSS 98.0 MB, PSS 56.4 MB
memory at end:              RSS 95.5 MB, PSS 53.7 MB
gpu: 58MiB
```

### Island rerun alone on the new base, cross-fade

```
== freya (island, cross-fade) ==
scale 1, texture 520x240
content init 18.3 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        340          0.26/0.67/2.40          0.33/0.76/2.48
Expanded     284          0.25/0.35/1.05          0.31/0.44/1.26
Rest          12          0.43/0.76/2.30          0.52/0.85/2.58
Idle          10          0.48/0.58/0.58          0.57/0.73/0.73
Morph     swap ms 16.38/16.75/18.06  ~32 fps
Expanded  swap ms 16.42/16.78/17.88  ~31 fps
Rest      swap ms 0.08/0.63/20.66  ~565 fps
Idle      swap ms 0.06/0.10/0.10  ~0 fps
idle frames over the last 10 s: 10
memory before content init: RSS 88.8 MB, PSS 43.3 MB
memory after content init:  RSS 105.8 MB, PSS 54.1 MB
memory at end:              RSS 113.3 MB, PSS 59.8 MB
gpu: 45MiB
```

### Island rerun alone on the new base, relayout

```
== freya (island, relayout) ==
scale 1, texture 520x240
content init 15.8 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        350          0.30/0.75/3.09          0.37/0.87/3.17
Expanded     270          0.25/0.31/0.44          0.31/0.45/0.55
Rest          12          0.54/0.64/2.16          0.65/0.77/2.40
Idle          10          0.59/0.71/0.71          0.73/0.88/0.88
Morph     swap ms 16.33/16.76/17.38  ~32 fps
Expanded  swap ms 16.44/16.75/17.98  ~31 fps
Rest      swap ms 0.07/0.51/30.63  ~386 fps
Idle      swap ms 0.09/1.70/1.70  ~0 fps
idle frames over the last 10 s: 10
memory before content init: RSS 88.7 MB, PSS 42.0 MB
memory after content init:  RSS 105.7 MB, PSS 52.7 MB
memory at end:              RSS 113.5 MB, PSS 61.3 MB
gpu: 48MiB
```

### Island floor rerun: island-empty --bench

```
== empty (island, cross-fade) ==
scale 1, texture 520x240
content init 0.2 ms
phase     frames  content ms p50/p95/max    frame ms p50/p95/max
Morph        402          0.26/0.30/0.51          0.32/0.48/2.14
Expanded       0                       -                       -
Rest           2          0.05/0.05/0.05          0.50/0.50/0.50
Idle           0                       -                       -
Morph     swap ms 16.38/16.74/18.00  ~32 fps
Rest      swap ms 25.92/25.92/25.92  ~76 fps
idle frames over the last 10 s: 0
memory before content init: RSS 81.0 MB, PSS 39.3 MB
memory after content init:  RSS 81.3 MB, PSS 39.5 MB
memory at end:              RSS 83.7 MB, PSS 41.8 MB
gpu: 38MiB
```

## Reading it

Content time includes the base's `glFinish`, so it covers Freya's diff and layout plus Skia's GPU
work on the 3440×1440 surface.

- **Transition frames (Morph).** The base's Morph phase is the 400 ms after each toggle. The open
  animation is 260 ms and the close 180 ms, so many of those frames are static, and Morph p50
  (0.26 ms in both modes, the same as the floor) mostly measures frames Freya did not redraw. The
  tail is the useful part:
  - Content animates: p95 1.41 / 1.94 ms, max 34 / 12.5 ms.
  - Compositor animates: p95 0.51 / 0.47 ms, max 15 / 13 ms.
  - Floor: p95 0.28 ms.

  The max in each run is most likely the first open (not separately verified): mounting the panel, shaping 24 labels and Skia
  compiling its pipelines.
- **Frames Freya actually redrew.** A temporary instrumented build (removed) timed tick + draw +
  `glFinish` for only the frames where Freya redrew:
  - Content animates: 113 frames, p50 0.43 ms, p95 1.76 ms, max 12.8 ms.
  - Compositor animates: 57 frames, p50 0.44 ms, p95 0.74 ms, max 15.5 ms.

  An animated full-screen frame, with every element faded through its own layer and the panel
  scaled, costs Freya about 1-2 ms of CPU+GPU on the 5070 Ti. With the compositor animating,
  Freya draws only the snaps, typing and the caret.
- **Typing frames (Expanded).** p50 0.40-0.46 ms, p95 0.62-0.92 ms in both modes. Every keystroke
  and caret blink repaints the whole 3440×1440 surface: Freya has no damage tracking, and
  `RenderPipeline` clears and redraws everything. It is cheap on this GPU only because Skia's
  GPU path is fast.
- **Memory over the floor** (end of run, floor RSS 95.5 MB / PSS 53.7 MB):
  - Content animates: RSS +22-24 MB, PSS +14-16 MB.
  - Compositor animates: RSS +13.5-14 MB, PSS +6 MB.
- **GPU over the floor** (floor 58 MiB):
  - Content animates: 146 MiB, **+88 MiB**.
  - Compositor animates: 90 MiB, **+32 MiB**.

  The +32 MiB is the content texture (19.8 MB), its stencil buffer (5 MB) and Skia's glyph atlas
  and buffers. The extra 56 MiB when content animates comes from Freya's per-element opacity (see
  Problems below): every faded element is a `saveLayerAlpha`, including the full-screen backdrop,
  and Skia keeps those layer textures in its resource cache afterwards. I did not try capping the
  cache (`DirectContext::set_resource_cache_limit`) or purging it when a transition ends; either
  would trade that memory back for the cost of reallocating on the next open.
- Idle frames are 0 in both modes. The launcher is closed and unmounted at idle, so nothing ticks.

## Glue for round 2

- `src/launcher_main.rs`: 50 code lines. It is the `IslandContent` adapter and ignores the window
  scale.
- `src/main.rs` for the island shrank to 49 code lines.
- Making the glue generic (`src/embed.rs`, 461 code lines after rustfmt) was a mechanical move,
  with no new mechanisms.
- UI code:
  - `src/launcher.rs`: 211 code lines, including the 48-name list and an HSL helper.
  - `src/widgets.rs`: 79 code lines (the text field, now shared).
  - `src/island.rs`: down to 341.

## Problems in round 2

1. **Parallel benches poisoned the numbers.** The Slint agent and I were running benches at the
   same time, and we write to the same scratchpad with the same file names. Fixed by waiting
   until no other spike process (`pgrep -x`) is running before each run. My first wait loop used
   `pgrep -f` with a pattern that also matched its own shell's command line, so it waited forever.
2. **Freya's opacity is per element, not per group.** `RenderPipeline` gives each element under an
   `opacity` its own `saveLayerAlpha` over that element's bounds, with the ancestors' opacities
   multiplied in. Fading the launcher root therefore draws around 90 layers per frame (root, panel, field, rows, and three per tile) instead of
   one, and overlapping translucent elements (text over the panel fill over the backdrop) look
   slightly different mid-fade than a group fade would. It costs GPU memory (above), and it is
   why the closed launcher is unmounted rather than drawn at opacity 0.
3. **Focus on open.** The field must be focused whenever the launcher is open. When the panel
   only mounted at t > 0, the focus request arrived before the node existed and was dropped. It
   now mounts on `open || t > 0`, and a `use_side_effect` on `open` requests focus or unfocus.
   With `set_animated(false)` the animation's `OnChange::Finish` snaps it to the end, so one
   component covers both modes.
4. The grid is left-aligned in the panel (6×160 + 5×16 = 1040 px in 1136 px of content width);
   the spec does not say. Tile names are 14 px primary, centred, one line, ellipsised.

## How the GPU path scales to the full output

Well. Going from a 520×240 texture to 3440×1440 (40× the pixels) moved Freya's redraw cost from
about 0.25 ms to about 0.4 ms for a typing frame, and to about 1.8 ms at p95 for an animated
full-screen fade with around 90 layers. Skia on the GPU barely notices the area.

The costs that do scale are memory and the lack of damage:
- Every change, including a caret blink, repaints and recomposites the whole 20 MB texture.
- Group fades allocate full-size layers, which Skia's cache keeps.

On this GPU none of that is a frame-time problem. It is GPU memory (+88 MiB while content
animates) and power on a mostly static screen. Compositor-driven open and close is the cheap
option: GPU drops to +32 MiB and transition frames to about 0.5 ms at p95. A real shell would
want that, a capped or purged Skia cache, and ideally a damage rectangle from Freya, which it
does not produce today.
