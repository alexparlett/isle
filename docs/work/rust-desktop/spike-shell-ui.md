# Spike: how the shell UI is drawn

Done 2026-10-02: Freya, locked as B7.

Decides brainstorm's open "Shell UI rendering" item. Throwaway code in
`spikes/shell-ui/`; only the findings survive, into
`docs/research/rust-desktop/shell-ui-spike.md` and B7 in `docs/DECISIONS.md`.

## Question

Freya embedded on Skia on the compositor's GL context, or Slint's software
renderer into damage-tracked CPU buffers: which gives the better shell, for
the cost, inside a Smithay compositor on the RTX 5070 Ti?

## What gets built

A toy compositor on Smithay (git, pinned rev) with the winit backend, so it
runs as a window inside the current desktop. No Wayland clients: the island
is the only thing on screen, over a plain background.

- **Shared base** (`base` crate): calloop loop, winit backend, GlesRenderer,
  input from winit, a frame clock, metrics, and the compositor-owned island
  shape. The shape is a rounded rectangle whose size and corner radius are
  animated by a spring and drawn by a GLES shader; the content texture is
  clipped to it. A trait `IslandContent` is the one seam: set state, take
  input, draw into a texture, report damage.
- **`island-freya`**: Freya `Runner` + tree rendered by Skia into an
  offscreen GL texture on Smithay's EGL context, imported as a
  `GlesTexture`.
- **`island-slint`**: a custom Slint `Platform` with a software-rendered
  window drawing into a `MemoryRenderBuffer` with Slint's dirty regions.
  The `.slint` file is loaded through `slint-interpreter` and reloaded when
  it changes on disk.

The same island in both, two states:

- **Rest:** a pill with the time (ticking each second) and a status dot.
- **Expanded:** a panel with a title, a now-playing row with a progress bar
  animating continuously, a slider driven by the pointer, and a text field
  that takes keyboard input.

Click or `Space` toggles the state; the compositor springs the shape, the
content cross-fades or relayouts.

### The island, exactly

Both variants draw this, so the numbers compare like for like. Sizes are
logical pixels, multiplied by the scale the base passes in. The content
texture is 520×240 (the expanded size) with a transparent background; the
compositor draws the dark rounded fill under it and clips it to the shape.
The rest pill (220×38) is top-centred in the texture.

Font: Inter, falling back to the system's fonts (Noto Sans, Noto Color
Emoji, Noto Sans CJK). Text white at 92% opacity; secondary text white at
60%.

- **Rest layer** (opacity 1 at rest, 0 when expanded; 180 ms ease-out),
  inside the top-centred 220×38 area: an 8 px dot `#4ade80` centred 18 px
  from the left; the time `HH:MM:SS` in 15 px semibold tabular figures,
  centred; `72%` in 13 px secondary, right-aligned 16 px from the right.
- **Expanded layer** (opacity 0 to 1 over 220 ms ease-out after an 80 ms
  delay; back to 0 over 120 ms), 20 px padding, a column with 10 px gaps:
  1. a row: `Now Playing` 13 px secondary on the left, `HH:MM` 13 px
     secondary on the right;
  2. `Midnight City` 20 px semibold;
  3. `M83 · Hurry Up, We're Dreaming` 14 px secondary;
  4. a progress bar, full width, 4 px tall, fully rounded, track white 15%,
     fill white 85%, looping 0 to 1 every 8 s while expanded; below it
     `1:23` left and `4:03` right in 12 px secondary;
  5. a row: `Volume` 13 px, then a slider filling the rest: 4 px track
     (white 15%, filled part white 85%) and a 14 px white knob; press on
     the slider and drag to set 0..1, starting at 0.4;
  6. a text field, full width, 36 px tall, 10 px radius, background white
     8%, 12 px horizontal padding, placeholder `Search…` in secondary;
     a click focuses it, typed text appears, Backspace deletes, the caret
     blinks every 500 ms while focused.
- In relayout mode the content lays itself out in the size the base gives,
  top-centred in the texture, so the expanded layer reflows every frame.
- `prepare_shot` puts `SHOT_TEXT` in the field (focused) and the slider at
  0.6, for the side-by-side screenshot.

## Measurements

A `--bench` mode toggles the state every 1.5 s for 30 s, then idles 10 s,
and prints:

- per-frame CPU time of content update + draw (+ upload for Slint), and of
  the whole frame: p50, p95, max, during the morph, while expanded and
  animating, and at idle;
- idle CPU (frames drawn per second at idle: should be 0 with nothing
  changing except the clock);
- RSS and Pss (`/proc/self/smaps_rollup`) at start, after bench, and the
  delta over an empty base build;
- GPU memory (`nvidia-smi`, per process) for each variant and the empty base;
- binary size, clean build time.

Plus the qualitative read:

- lines of glue each variant needed beyond the base;
- text quality at 1x (screenshot side by side);
- the iteration loop: how fast a change to the island's layout shows up
  (Slint live reload; Freya rebuild or hot-patch);
- what broke and how hard it was to find.

`--relayout` makes the content relayout at the animated size every frame
instead of cross-fading: the worst case for each.

## Out of scope

Blur, Wayland clients, the DRM backend, accessibility export, multiple
outputs. All of these are the same for both variants or come later.

## Done when

Both variants run the same island and the bench, the numbers and the
qualitative read are in `docs/research/rust-desktop/shell-ui-spike.md`,
and the user has picked one; brainstorm records it as B7 with the loser as
the rejected alternative.

## Round 2: a full-output surface

The island is small; the launcher, overview and lock screen cover the
whole output. Round 2 draws a launcher at the host's 3440×1440 to see how
each renderer copes when the dirty area is the whole screen.

The base's launcher mode (`run_launcher`, `--size WxH`) gives the content a
3440×1440 texture, draws it letterboxed into the window, toggles open and
closed every 2 s for 30 s under `--bench`, and while open types `c`, then
`o`, then Backspace (at 600, 900 and 1200 ms after opening). Frames within
400 ms of a toggle count as the transition (`Morph`); frames while open are
`Expanded` (the typing lands here). Two modes:

- **content animates** (default): the toolkit animates the open and close
  itself, so every transition frame repaints the whole surface. The worst
  case.
- **`--comp-anim`**: the compositor fades and slides the texture; the base
  calls `set_animated(false)` and the content shows and hides instantly.
  The mitigation a design could choose.

### The launcher, exactly

Logical pixels at scale 1 on a 3440×1440 texture, transparent where
nothing is drawn. Font and text colours as the island.

- **Backdrop:** the whole surface black at 45%.
- **Panel:** centred, 1200×760, radius 28, fill `rgba(20,20,26,0.92)`,
  1 px border white 10%, 32 px padding, a column with 24 px gap:
  1. a search field 48 px tall, radius 14, white 8%, 18 px text, 16 px
     horizontal padding, placeholder `Search apps`; focused whenever the
     launcher is open, typed text appears, Backspace deletes, caret blinks
     every 500 ms;
  2. a grid of tiles, 6 columns, 16 px gaps, showing the first 24 of the
     app list whose names contain the query (case-insensitive). A tile is
     160×140: a 64 px rounded square (radius 16) filled with
     `hsl(i × 360 / 48, 55%, 55%)` for the app's index `i` in the full
     list, centred at the top, and the name in 14 px below it, centred,
     one line, elided.
- **App list** (48, in this order): Firefox, Files, Terminal, Settings,
  Steam, Heroic, Lutris, Spotify, Discord, Signal, Thunderbird, Calendar,
  Calculator, Clocks, Weather, Maps, Photos, Camera, Music, Videos,
  Podcasts, Books, Notes, Text Editor, Code, Zed, Blender, GIMP, Inkscape,
  Krita, Kdenlive, OBS Studio, Audacity, Ardour, Bottles, ProtonUp,
  MangoHud, Monitor, Disks, Keychain, Cleaner, Archive, Fonts, Characters,
  Contacts, Remote Desktop, Torrent, Backups.
- **Open:** backdrop opacity 0 to 1 and panel opacity 0 to 1, panel scale
  0.96 to 1 and offset 24 px down to 0, over 260 ms ease-out. **Close:**
  the reverse over 180 ms ease-in. With `set_animated(false)` both are
  instant.
- `prepare_shot`: open, query `co`, field focused.
