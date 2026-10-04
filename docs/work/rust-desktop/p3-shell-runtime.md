# P3 · Shell runtime

Planned 2026-10-04.

The roadmap's brief: the Freya embedding from the spike, moved into the
fork as a crate: surface hosting on the compositor's GL context, input,
the accessibility tree, panic boundaries per surface, compositor-driven
transitions, the Skia cache cap, live `tokens.kdl`; and a preview mode
that runs a surface in `freya-winit` with hot reload, to win back design
iteration (B7's condition).

P3 draws no surface the user asks for: the island, menus and the
wallpaper are P6's. What it must get right is what every surface from P4
onwards stands on: a surface is a Freya tree the compositor hosts,
places, animates, feeds input to and rebuilds when it panics; it costs
nothing while still or closed; it reads the design from `tokens.kdl`
live; and it can be worked on outside the compositor with hot reload.
One probe surface, built only for tests and the preview, exercises all
of it.

## What it answers to

- Decisions: B7 (Freya through the fork; compositor-driven open and
  close; the cache cap; live iteration tried early), B9 (one loop, panic
  boundaries, nothing blocking on the main thread), B10 (`tokens.kdl`
  watched and applied as state), B35 (the loop, the worker's jobs, the
  redraw states, the Test interface).
- Intent: `shell.md` Shell runtime 1–9, 11–13, 15, 16 (10 and 14 are
  P6's and P10's once more than one surface exists; P3 leaves the room)
  and its open points; acceptance bars 2, 3, 5, 11, 12, and 4 in part.
  `compositor.md`'s stacking of shell surfaces against layer-shell (rule
  13). `platform.md` Settings for the new `appearance` section.
- Design: DESIGN.md Materials (glass, blur 48, saturation 1.8, the glass
  floor "until P3 can sample cheaply"), Motion (five springs, Reduce
  Motion as a fade, Game Mode as none), `data/tokens/schema.md` (the
  token file's shape, written in P1 for this packet).
- Evidence: `shell-ui-spike.md` and `spikes/shell-ui/freya/NOTES.md` (the
  glue, the GL handover, per-element layers, the 178 MB relayout run,
  focus needing the accessibility tree), `ui-and-services.md` (the open
  Runner question). The fork's `freya-web` crate is an embedding written
  since the spike on the same `freya_core::integration` API, and is the
  model for the new crate's frame step.

## Choices this plan makes

The user took every recommendation at review (2026-10-04), blur and the
glass floor in P3 and the AT-SPI export included.

1. **Where the glue lives.** A crate `freya-embed` in the fork holds
   everything about driving Freya without winit: the `Runner`, `Tree`,
   accessibility tree, the frame step, events in Freya's own terms, the
   Skia GL target over an FBO the caller owns, and the GL state handover
   as a hook. It knows nothing of Smithay. `isle-shell` in this
   workspace holds the Smithay side: textures, render elements, keysym and
   keycode mapping, the surface registry, placement, transitions, blur and
   the panic boundaries. The fork crate can then be tested with Skia's
   CPU raster backend in the fork's CI, and Isle's apps never see it.
2. **Depending on the fork.** By git revision, as Smithay is. Work on the
   crate happens on a branch of `alexparlett/freya` merged by pull
   request, as the fork's earlier changes were; while iterating, a
   git-ignored `.cargo/config.toml` patches the dependency to
   `~/Development/freya`. A revision bump is its own commit.
3. **One Skia context.** Every surface draws through one `DirectContext`
   on the compositor's EGL context and one font collection, each surface
   owning only its FBO, texture and stencil buffer. The cache is capped
   (starting at 64 MiB, set from measurement in 3.6) and purged of
   unlocked resources when a surface closes.
4. **Waking, not ticking.** Each surface's `Runner::handle_events` is
   polled with a waker that pings a calloop source, so a timer, a task or
   a state change wakes the loop and nothing runs at rest; the spike's
   50 ms poll goes. Freya's `RenderingTicker` is notified after the
   output that shows the surface presents, so its animations advance at
   the output's rate and stop when it does.
5. **The surface model.** A surface has a name, a layer (below windows,
   above windows, or above everything including layer-shell overlays,
   which is where prompts and the lock go: rule 13), a scope (one
   instance per output, or one on the main screen), a model and a root
   component. The model is the state that outlives the tree (rule 3): it
   is held by the compositor and given to the tree as a root context, so
   a rebuilt tree reads it back. A closed surface keeps its `Runner` but
   mounts no content, holds no texture and fires no timers (rule 7); it
   reopens in one frame, so the keys typed as Spotlight opens are not
   lost.
6. **Focus.** Keyboard and pointer targets become enums over a window, a
   layer surface and a shell surface, as anvil and niri do. A shell
   surface that takes keys remembers the target it took them from and
   gives it back as it closes, before any action it started reaches that
   window (rule 11). No input method in the shell in P3.
7. **Accessibility.** The tree is built for every surface (keyboard focus
   goes through it) and exported to AT-SPI with `accesskit_unix`, one
   adapter per surface, on the worker's runtime. It is dumped as JSON over
   D-Bus for tests.
8. **Panic boundaries.** Every call into a surface (event, frame step,
   draw) runs under `catch_unwind`. A thread-local mark tells the panic
   hook it is inside a boundary, so it logs the surface's name and
   backtrace instead of writing a core crash record. The surface is
   dropped, the GL state restored, and the tree rebuilt from its model in
   the next frame. Five panics in a minute disable the surface until
   `isle shell restart <name>`, logged and shown on `Shell1.Status`. The
   lock screen's own threshold for the minimal lock is P9's.
9. **Tokens.** A crate `isle-tokens` reads `tokens.kdl`. Its typed struct
   is generated at build time from the shipped `data/tokens/tokens.kdl`,
   so every shipped token is typed and none is written twice; units follow
   `schema.md` (colours, points, `-ms` durations, plain numbers, springs,
   shadow strings parsed into lists). The user's `~/.config/isle/
   tokens.kdl`, when present, is merged over the shipped file by path, so
   it holds only what it changes. The worker watches both; a broken or
   newer-versioned file keeps the last good tokens and reports on
   `Shell1.Status` and the log (the island's glyph is P6's).
10. **Appearance.** A new `appearance` section in `settings.kdl`: mode
    (`dark`, `light`, `auto`), accent, glass position, text scale,
    Increase Contrast, Reduce Motion, Reduce Transparency. The runtime
    resolves tokens for these into the context every surface reads, so a
    change to either file re-renders every surface. `auto` follows dark
    until the Settings packet (P13) gives it the sun.
11. **`isle-ui`.** The Freya-side crate shared by the shell and Isle's
    apps: the token context and its hooks, and the primitives P3 needs
    (glass, the keyboard-only focus ring). The component library grows in
    it from P4 on. Apps read the same files and watch them themselves.
12. **Transitions.** The compositor draws each surface's texture with an
    opacity, a scale about an anchor, an offset and a rounded clip, each
    driven by a spring from the motion tokens. Reduce Motion turns a
    spring into `reduced-fade-ms` in place; a runtime switch draws
    without transitions, which P12's Game Mode turns on. Freya animates
    only what changes inside a surface.
13. **Blur and the glass floor in P3.** A surface declares its glass
    shapes (rounded rectangles from `isle-ui`'s glass primitive, reported
    from layout). The compositor blurs what lies behind each (dual
    Kawase at reduced resolution, saturation applied in the same pass),
    re-blurring only when the backdrop under it is damaged, and draws the
    surface over it. The same pass reduces the blurred region to a mean
    luminance read back a frame late, from which the tint moves toward
    `glass-tinted` until secondary text holds 4.5:1 (DESIGN.md, the glass
    floor). Reduce Transparency and the no-transitions switch use the
    solid material and skip the pass. Every glass surface from P6 on
    needs this, and its cost on NVIDIA belongs with the render runtime.
14. **Preview mode.** `isle-preview`, a `freya-winit` binary, runs any
    surface by name with fixture models and the live tokens, under `dx
    serve --hotpatch` (dioxus-cli, installed with cargo) for code changes.
    Token edits apply live with or without `dx`. If hot patching does not
    work on this setup, the fallback is a rebuild-and-restart loop that
    keeps the fixture state, and the plan records which.
15. **The test surface.** `Shell1` on the bus: list surfaces (name,
    output, open, size, panics), open, close, restart, the accessibility
    tree, Status. `Test1` gains a forced surface panic. `isle shell
    list|open|close|restart|tree|status`. Shell surfaces appear in
    screenshots like everything else.

## The workspace

```
crates/
  isle-shell/       the surface host: registry, placement, textures,
                    input routing, transitions, blur, panic boundaries,
                    the probe surface (test-hooks)
  isle-tokens/      tokens.kdl: generated struct, merge, resolve, validate
  isle-ui/          Freya side shared with apps: token context, hooks,
                    glass, focus ring
  isle-preview/     bin: a surface in freya-winit with hot reload
alexparlett/freya:
  crates/freya-embed/   Runner, Tree, accessibility, frame step, events,
                        the Skia GL target and the GL handover hook
```

`isle-compositor` depends on `isle-shell`; nothing else in the core
does. Freya and Skia enter the compositor's build here: the clean build
grows by Skia's prebuilt download and about 250 crates (spike: 124 s).

## Phases

Each phase ends in a check run by script or in the VM and lands as its
own commits, in the fork where the work is the fork's.

### 3.0 · The fork crate

- `freya-embed` in the fork from the spike's `embed.rs`, rebuilt on
  `freya-web`'s frame step: contexts provided at the root, events
  measured and executed, mutations applied, layout, accessibility, the
  focused node, the cursor icon. The waker of choice 4 is the caller's.
  Shared GPU and font state (choice 3) is one object many surfaces draw
  through.
- Fork tests on Skia's CPU raster: a tree mounts, lays out, takes a click
  and a key, requests focus, wakes its waker from a timer, and draws.
- Isle depends on it by revision; `isle-shell`, `isle-tokens`, `isle-ui`
  as skeletons; clean build time and binary size recorded.

**Check:** the fork's tests pass in its CI; the workspace builds and
passes clippy with Freya in it.

### 3.1 · Hosting

- `isle-shell`'s registry on `Isle`; a surface's texture and FBO on the
  compositor's context at the output's scale, drawn in the redraw pass
  before elements are collected, with the GL handover around Skia.
- Surfaces as render elements in their layer's place: above everything,
  above windows, below windows; `surface_under` and the layer-shell order
  follow.
- The probe surface: text in Inter with a fallback and an emoji, a button,
  a field, a scrolling list, an animated bar, a glass panel.
- Waking (choice 4); a closed surface releases its texture; an output's
  scale change resizes the texture and re-lays out.

**Check:** headless: the probe drawn into a screenshot at scale 1 and 2
with the expected pixels; with the probe open and still, no frames over
ten seconds; with its bar animating, frames every vblank; closed, no
wakeups (traced).

### 3.2 · Input and focus

- Pointer motion, buttons, scroll, enter and leave into the surface under
  the pointer; the tree's cursor icon becomes the compositor's cursor.
- Keyboard: keysyms to Freya's keys, evdev codes to its codes, modifiers;
  focus targets as enums (choice 6); a key-taking surface gets the
  keyboard and returns it on close before any action it started.
- Keys pressed while a surface opens reach it in order.

**Check:** headless through `Test1`: a click presses the probe's button,
typing fills its field, scroll moves its list, closing it returns focus
to the test client, which then receives a key the surface sent it.

### 3.3 · Accessibility

- `accesskit_unix` adapters per surface (choice 7); keyboard navigation
  with Tab and the arrow keys through the tree; the focus ring on keyboard
  input only.
- `Shell1` and `isle shell tree` dump it.

**Check:** headless: the tree lists the probe's controls with roles and
names; Tab walks them. Nested on the host: the probe is visible on the
accessibility bus (`busctl --address` on the a11y bus).

### 3.4 · Panic boundaries

- Choice 8: `catch_unwind` around every call, the hook's mark, rebuild
  from the model, the five-a-minute guard, `isle shell restart`.
- `Test1.PanicSurface(name, where)` panics in an event handler, a frame
  step or a draw.

**Check:** headless: each kind of panic rebuilds the probe within a
second with its field's text kept by the model, the test client keeps
drawing and taking input, no crash record is written; the sixth panic in
a minute disables it, `isle shell restart` brings it back (acceptance 11).

### 3.5 · Tokens and appearance

- `isle-tokens` (choice 9) with its generated struct, the merge, units,
  version and error rules; a test that every token in the shipped file
  is typed and resolves for both appearances and with Increase Contrast.
- The `appearance` settings section (choice 10); the worker watches both
  token files; the loop resolves and hands the result to every surface's
  context; `isle-ui`'s hooks.
- The P1 boards keep reading `data/tokens/tokens.kdl` unchanged.

**Check:** headless: a saved change to a colour token reaches the probe
within a second (screenshot); a broken file keeps the last good tokens
and reports its line on `Shell1.Status`; switching `appearance mode`
redraws in the other appearance (acceptance 12).

### 3.6 · Transitions and the cache

- The springs of choice 12 on the texture element; open, close and a size
  morph on the probe; Reduce Motion and the no-transitions switch.
- The Skia cache cap and purge on close (choice 3); its usage on
  `Shell1`. The cap set from a run that opens and closes the probe with
  per-element fades a hundred times.

**Check:** headless: a transition's frames are compositor frames only
(the probe draws once per open); Skia's resource usage after a hundred
open-close cycles stays under the cap and returns to its first-cycle
level.

### 3.7 · Blur and glass

- Choice 13: glass shapes reported from layout, the blur pass with its
  cache, saturation, the solid fallback, the luminance read-back and the
  tint it drives.

**Check:** headless: the probe's glass over a test client of known
colours shows the blurred colours in the screenshot; with nothing behind
it changing, the blur does not rerun (counted); over white and over
black the tint moves so secondary text holds 4.5:1. On the host: the
probe's glass costs under 1 ms a frame at 3440×1440 (GPU timer query).

### 3.8 · Preview mode

- `isle-preview <surface>` (choice 14), with fixture models per surface
  and live tokens; dioxus-cli installed; the hot patch tried and the
  outcome recorded.

**Check:** on the host, a change to the probe's layout shows in the
preview without a restart (or, on the fallback, within the rebuild with
its state kept); a token edit shows live.

### 3.9 · The VM and the host

- The VM: a `test-hooks` build of the session opens the probe with `isle
  shell open probe` over ssh; screenshot; typing and clicking by
  `dev/vm-qmp.py`.
- The host, nested: content draw at 3440×1440 (p95 under 2 ms,
  acceptance 5), RSS and VRAM with the probe at rest and after a
  one-hour scripted open, close, type and relayout soak, which also
  revisits the spike's 178 MB relayout run.
- The budgets set from these and written into `shell.md`: the RSS of the
  compositor with the always-on surfaces at rest (acceptance 3, under
  v1's 458 MB) and the soak margin (acceptance 4). The 24-hour soak
  waits for real surfaces, at the end of M1.

**Check:** the VM screenshot with the probe and kitty; the numbers
recorded in `docs/research/rust-desktop/p3-runtime.md`.

### 3.10 · Close

- ARCHITECTURE: how the shell runs, the new crates, the fork crate.
- DECISIONS for the choices later packets must keep (the surface model,
  tokens generated from the shipped file, glass in the compositor).
- `shell.md`'s open points answered; DESIGN.md's glass floor note
  updated; `state.md`, `progress.md`, the roadmap's P3 entry.

## Not in P3

Every user-facing surface (P6 onwards); one surface closing another and
the main screen (with P6 and P10, on the registry P3 builds); the
minimal lock and its threshold (P9); Game Mode turning transitions off
(P12; the switch exists); `auto` appearance by the sun (P13); the
24-hour soak (end of M1); theming installed apps from tokens (P14); an
input method in shell fields.

## Depends on, and feeds

- Needs P2 and the fork at rc.8 (done 2026-10-04).
- P4 draws its title bars as surfaces of a window-attached kind on this
  host. P6 builds the island, menus and wallpaper on the registry,
  transitions, glass and tokens. P13's Settings app and every Isle app
  take `isle-ui` and `isle-tokens`. Every packet takes `Shell1` and the
  preview.

## Done when

- The probe surface draws in the compositor, nested, headless and in the
  VM, from the shipped tokens, and takes pointer and keyboard input with
  focus returned on close.
- `cargo test` passes headless: idle and closed surfaces cost no frames or
  wakeups; a panic anywhere in a surface rebuilds it within a second with
  its model kept; a token edit reaches it within a second and a broken
  file keeps the last good tokens; Skia's cache stays under its cap over
  repeated opens; glass blurs and holds its contrast floor.
- The accessibility tree is on the a11y bus and dumped by `isle shell
  tree`.
- A surface runs in the preview with token edits live and code changes
  hot patched or the fallback recorded.
- The budgets are measured, set and written into `shell.md`.

## Risks

- **Freya's API is a moving RC.** The fork pins it; `freya-embed` sits
  on `freya_core::integration`, which `freya-web` also uses, so upstream
  changes to it show in both.
- **Skia and Smithay on one context.** The spike's handover held for a
  toy; real windows, dmabuf imports and the cursor plane add GL state.
  Every draw is followed by the full restore, and the headless tests
  composite windows and surfaces together.
- **Unwinding through Freya.** A panic can leave its `RefCell`s borrowed;
  the boundary drops the whole runner and tree rather than reusing them.
  A panic inside Skia's C++ is an abort, not an unwind, and ends the
  session like any core crash.
- **Hot patching.** Subsecond needs `dx` and a matching linker setup; it
  may not work for a workspace binary. The fallback is recorded rather
  than fought.
- **Blur on NVIDIA.** Reading back luminance must never stall the loop;
  it uses a fence and is read a frame late, or it is dropped and the
  floor tint fixed until it can be.
- **Build time.** Skia's prebuilt archive is downloaded at build time,
  which the PKGBUILD (P14) must allow for.
