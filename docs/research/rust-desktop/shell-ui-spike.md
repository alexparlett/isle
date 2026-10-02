# Shell UI spike: Freya on Skia versus Slint's software renderer

Run 2026-10-02 on the host (RTX 5070 Ti, driver 615.71.09, DP-4 3440×1440
at 60 Hz with VRR, scale 1). Question and island spec:
`docs/work/rust-desktop/spike-shell-ui.md`. Code: `spikes/shell-ui/`
(base, `freya/`, `slint/`); full outputs in `freya/NOTES.md` and
`slint/NOTES.md`; screenshots in `spikes/shell-ui/shots/` (git-ignored).

## Setup

A toy compositor on Smithay 118e34f with the winit backend, running as a
window on the host. The compositor springs the island's size and radius and
draws its fill with GLES shaders; the content texture (520×240) is clipped
to the shape. Both variants draw the same island (clock, track, looping
progress bar, slider, text field) behind the same `IslandContent` trait.

- **Freya** (git cfd97ac, 0.5.0-rc): `freya_core::Runner` + tree, drawn by
  Skia (via `freya_engine`) on Smithay's own EGL context into an FBO whose
  texture Smithay samples directly. Skia `reset` before each draw, Smithay's
  GL state restored after.
- **Slint** 1.18.1: custom `Platform`, `MinimalSoftwareWindow` with a
  reused buffer, dirty rectangles uploaded with `ImportMem::update_memory`.
  The `.slint` file is loaded through `slint-interpreter` and reloaded by a
  `notify` watcher, carrying public properties and focus across.

## Numbers

Final runs 2026-10-02 18:30–18:40, one at a time, each started with the
1-minute load under 2 (`spikes/shell-ui` binaries after the base's frame
pacing and typing-script fixes). Earlier agent runs overlapped each other
and a build, and one Freya result file held Slint's output; they are
superseded by these. Content time includes a `glFinish`, so it is CPU plus
GPU for the content. Milliseconds p50 / p95 / max.

### Island (520×240)

| | Empty base | Freya | Slint |
|---|---|---|---|
| Morph | 0.25 / 0.36 / 0.61 | 0.25 / 0.71 / 2.30 | 0.28 / 0.60 / 2.05 |
| Expanded (progress animating) | — | 0.24 / 0.34 / 0.66 | 0.27 / 0.43 / 1.95 |
| Idle frames in 10 s (clock) | 0 | 10 | 9 |
| RSS / PSS at end | 83.9 / 41.9 MB | 113.5 / 61.4 MB | 116.5 / 70.8 MB |
| GPU memory | 38 MiB | 45 MiB | 38 MiB |

### Launcher (3440×1440)

Morph is the 400 ms after a toggle; with `--comp-anim` most of those
frames are the compositor's alone, so the p95 is the content's one full
repaint. Expanded is typing and the caret.

| | Morph | Expanded | RSS / PSS | GPU |
|---|---|---|---|---|
| Empty base | 0.24 / 0.29 / 1.02 | — | 95.8 / 53.8 MB | 58 MiB |
| Freya, content animates | 0.26 / 1.17 / 10.26 | 0.41 / 0.76 / 0.99 | 116.6 / 67.0 MB | 146 MiB |
| Freya, compositor animates | 0.25 / 0.45 / 15.77 | 0.44 / 0.87 / 0.95 | 110.0 / 60.4 MB | 90 MiB |
| Slint, content animates | 10.77 / 20.18 / 30.92 | 2.31 / 4.76 / 12.17 | 137.3 / 92.3 MB | 58 MiB |
| Slint, compositor animates | 0.26 / 13.27 / 15.39 | 2.39 / 3.90 / 4.29 | 137.5 / 92.7 MB | 58 MiB |
| Slint AVX2, content animates | 4.59 / 6.04 / 7.13 | 1.00 / 1.88 / 1.95 | 139.0 / 94.2 MB | 58 MiB |
| Slint AVX2, compositor animates | 0.25 / 5.02 / 7.04 | 0.92 / 1.81 / 1.82 | 142.7 / 97.8 MB | 58 MiB |

AVX2 is `RUSTFLAGS="-C target-cpu=x86-64-v3"`. The Slint agent measured a
full-surface draw at 10.3 ms default, 2.9 ms AVX2, 2.1 ms native
(AVX-512), plus about 2 ms to upload 20 MB whatever the flags. Slint's
software renderer is single-threaded with no thread knob.

Freya's extra GPU memory when content animates is Skia layers: Freya fades
each element in its own layer (around 90 a frame, one full-screen) and
Skia caches them. Capping or purging Skia's resource cache should recover
most of it; not tried. Freya repaints the whole surface for any change
(no damage tracking), which the GPU absorbs: a caret blink at 3440×1440
costs 0.4 ms.

### Build and iteration

| | Freya | Slint |
|---|---|---|
| Release binary, stripped | 25 MB | 26 MB (carries the Slint compiler) |
| Clean build, fresh target | 124 s, 249 crates; prebuilt Skia download | 45 s |
| Glue (not UI) | 461 lines (`embed.rs`) + 50 per surface | 343 lines, shared by both binaries |
| UI | island 341 + launcher 211 + widgets 79, Rust | island 209 + launcher `.slint` |
| Edit-to-screen | 1.8–1.9 s rebuild + restart | 80–96 ms live (launcher reload 3.7 ms), state kept |

Frame pacing: every variant, and `es2gears_wayland` as a control, runs at
about 30 fps as a window on this host: Hyprland on the 60 Hz VRR output
paces new windows at every other vblank. It is the host, not the spike,
and the DRM backend owns its own pacing.

## Text

- Both use Inter and fall back to Noto Sans CJK for 東京.
- **Freya:** colour emoji through Skia (Noto Color Emoji's 🎧 is itself
  grey, which looked monochrome at first).
- **Slint:** the software renderer draws glyph outlines only, so colour
  emoji come out as monochrome outlines. Slint 1.18 cannot request font
  features: the clock's tabular figures were faked with eight fixed-width
  cells.

## What building each was like

- **Freya:** rendering was right first time and cheap. The cost is around
  400 lines of undocumented platform glue copied from `freya-winit`
  (events, the accessibility tree that keyboard input goes through,
  futures polling, animation ticking); a Freya bug found at once
  (`AnimSequential` stops after its first step); Freya's hot reload works
  only under `freya-winit`, so every layout change is a Rust rebuild.
  Leftovers: a fade toggled mid-way restarts from its start value; the
  progress bar drops to 0 during collapse; the text field has no cursor
  movement, selection or IME.
- **Slint:** the `.slint` file reads like the spec; dirty tracking makes
  uploads small (a clock tick repaints 2,394 of 124,800 pixels in about
  0.5 ms); live reload is excellent and survives syntax errors. Quirk: the
  window learns it needs a redraw only by drawing, so it draws in `tick`.
  Slint's own `live-preview` feature needs an event loop the custom
  platform lacks and would lose state, hence the interpreter.
- **Together:** Freya's and Slint's Skia bindings both declare
  `links = "skia"` (Slint's through a wasm/iOS-only dependency chain that
  Cargo still resolves), so they cannot share a lockfile. The Freya crate
  has its own workspace in `spikes/shell-ui/freya/island/`.

## Round 2 findings beyond the numbers

- **Slint's software renderer has no transforms**: the launcher's panel
  fades and slides but cannot scale from 0.96. Opacity applies per element,
  so mid-fade the panel's fill shows through the tiles.
- **Slint cannot filter a model in `.slint`** (no string contains); the
  launcher filters in Rust and replaces the model when matches change.
- **Freya** needed the panel mounted as soon as the launcher opens, or the
  field's focus request was dropped.

## Not covered

- Input by hand on the live desktop (the agents drove input in-process).
- Blur, accessibility export, multiple outputs, the DRM backend, a long
  soak (Freya once ended a relayout run at 178 MB RSS; not reproduced).
