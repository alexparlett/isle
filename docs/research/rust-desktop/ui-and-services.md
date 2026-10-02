# UI rendering, client toolkits and services for a Rust desktop

Research date: 2026-10-02. Every version below was read from crates.io's API,
a GitHub/GitLab release or tag, or the project's source at the commit noted, on
that date. Anything that could not be checked is in **OPEN** at the end, not
guessed. "Verified in source" means the repository was cloned and the code read;
"stated by" means a document or third party says it and it was not reproduced.

Host context, measured 2026-10-02 on the maintainer's machine: CachyOS, kernel
7.2.8, RTX 5070 Ti (GB203) on NVIDIA 615.71.09, plus an AMD Granite Ridge iGPU;
Hyprland; polkit 127, PipeWire 1.6.9, WirePlumber 0.5.18, power-profiles-daemon
0.30, gamemode 1.8.2, gnome-keyring 50.0. The current Isle (`qs`) owns
`org.freedesktop.Notifications`, `org.kde.StatusNotifierWatcher` and
`org.freedesktop.impl.portal.desktop.isle` on the session bus.

---

## 0. Findings most likely to shape the architecture

1. **No Rust compositor draws its shell with a GPU UI toolkit on the
   compositor's own GL context, except Otto.** cosmic-comp rasterises iced on
   the CPU (tiny-skia) into a `MemoryRenderBuffer`; niri rasterises Pango/Cairo
   on the CPU into a `TextureBuffer`. Otto (nongio/otto, v1.5.0, 2026-09-26)
   wraps Smithay's GLES framebuffer in a Skia `DirectContext` and draws a
   retained scene graph (lay-rs, Taffy layout, blur, animation) into it. Otto is
   the nearest existing match to the planned design.
2. **Two viable in-compositor paths:** (a) CPU raster to an uploaded texture,
   which is simple and proven, costs CPU and an upload per animated frame, and
   uses `w*h*4` bytes per element per scale; (b) GPU drawing on the same EGL
   context: Skia GL, femtovg via glow, or egui_glow, all through Smithay's
   `GlowRenderer::with_context` or the raw GLES context. The GPU path must save
   and restore GL state, because Smithay says its GL state is an
   implementation detail.
3. **Freya can now be embedded.** Its main branch, 0.5.0-rc.8 (2026-09-29), ships
   `examples/feature_embedded.rs` (added 2026-06-17). It drives
   `freya_core::Runner` + `Tree` + `RenderPipeline` onto any Skia `Canvas`,
   without winit. Pointed at a Skia GL surface on Smithay's FBO, the way Otto
   does it, this gives a declarative in-compositor UI with HarfBuzz/SkParagraph
   text, `backdrop_blur`, and Freya's animation hooks. Freya pins its own Skia
   fork (`freya-skia-safe =0.100.0-beta.1`), so the compositor must use Skia
   through Freya's re-export, never a second `skia-safe`.
4. **Slint cannot target Smithay's FBO through its public GL API.** In 1.18.1
   `FemtoVGRenderer` draws to `RenderTarget::Screen` (FBO 0), verified in
   source. Its `SoftwareRenderer` (dirty-region partial rendering) into a
   `MemoryRenderBuffer` works. `FemtoVGWGPURenderer` and `SkiaWGPU30Renderer`
   render into wgpu textures, which a GLES compositor does not have.
5. **wgpu inside a Smithay GLES compositor is unproven.** Smithay has no wgpu or
   Vulkan renderer upstream (issue #928, open since 2023). `wgpu-hal` 30 has
   `gles::Adapter::new_external` to wrap a current GL context, which would let
   iced_wgpu, vello or glyphon run there. No compositor was found doing it.
6. **On NVIDIA every GPU-using process pays a fixed driver cost.** Measured on
   this host: `qs` maps about 82 MB RSS of NVIDIA driver memory, 32.9 MB of it a
   private `/dev/nvidiactl` mapping, and Hyprland about 72 MB. `nvidia-smi`
   shows `qs` at 83 MiB and Hyprland at 133 MiB of VRAM. Every separate GPU
   client app (Settings, Files and the rest) pays this again, so a CPU-rendered
   app can be far smaller (Slint software renderer about 30 MB against about
   135 MB for egui on the GPU, as reported by one author). The niri wiki's
   `GLVidHeapReuseRatio=0` application profile is needed for the compositor
   process on NVIDIA.
7. **Blur and glass belong to the compositor, and the protocol exists.**
   `ext-background-effect-v1` is in wayland-protocols staging. Smithay master
   has `wayland::background_effect`, and niri v26.04 (2026-04-25) ships
   mainline blur through it (dual-pass downsample/upsample on `GlesRenderer`,
   verified in source). Any client toolkit gets glass by asking for it, with no
   toolkit-side blur.
8. **Services: zbus 5.19 throughout, but bluer and mpris pull in libdbus.**
   bluer 0.17.4 is built on the `dbus` crate, and mpris 2.1 also uses `dbus`.
   COSMIC avoids both with its own zbus bindings (`pop-os/dbus-settings-bindings`:
   networkmanager, bluez, upower, mpris2, and more, git only). For one D-Bus
   stack, generate zbus proxies (`zbus_xmlgen` 5.4.1) or vendor those bindings.
9. **Notifications, tray, polkit and the keyring prompter are hand-written zbus
   servers.** notify-rust's `server` feature is commented out. SNI host crates
   are small (`system-tray` 0.8.9 is the most used). The polkit agent is pure
   Rust in cosmic-osd, which speaks to `/run/polkit/agent-helper.socket`. That
   socket is required here: polkit 127's `polkit-agent-helper-1` is not setuid,
   and the socket unit is active. A keychain UI must also serve
   `org.gnome.keyring.SystemPrompter`, because both gnome-keyring and oo7-daemon
   call it for unlock prompts.
10. **Config: niri polls the config's mtime every 500 ms and reloads KDL
    (knuffel 3.2.0, unmaintained since 2023). COSMIC uses per-key RON files
    watched with `notify`.** For UI layout without a restart, the only
    production mechanism is Slint's runtime interpreter. iced 0.14 and Freya
    hot-patch code with `subsecond`, but only in development.

---

## 1. Drawing UI inside the compositor

### 1.1 Smithay facts that constrain every option

| Fact | Evidence |
|---|---|
| Latest crates.io release is **smithay 0.7.0 (2025-06-24)**. niri and cosmic-comp both depend on git master, which is active: commits 2026-09-27 to 2026-09-29. | crates.io API; `gh api repos/Smithay/smithay/commits` |
| Renderers upstream are GLES (`GlesRenderer`), `GlowRenderer` (GLES plus a `glow::Context`) and Pixman. There is **no Vulkan or wgpu renderer upstream**. Issue #928 "WGPU backend support" has been open since 2023-03-10 and was last active 2026-02-05. | https://github.com/Smithay/smithay/issues/928 |
| `GlowRenderer::with_context(|gl: &Arc<glow::Context>| ...)` runs custom GL on the compositor's context. The docs warn that GL state is an implementation detail and that you must reset everything you change. | https://docs.rs/smithay/latest/smithay/backend/renderer/glow/struct.GlowRenderer.html (0.7.0) |
| `EGLContext::new_shared` exists for a second context in the same share group. | Smithay source search, `src/backend/egl/context.rs` |
| CPU pixels enter the scene as `MemoryRenderBuffer` + `MemoryRenderBufferRenderElement` (used by cosmic-comp), or `TextureBuffer::from_memory` (used by niri). | cosmic-comp and niri sources (below) |
| Smithay master has `src/wayland/background_effect/`, the server side of `ext-background-effect-v1`. | Smithay code search; niri `src/handlers/background_effect.rs` |
| Smithay-egui (`smithay-egui` 0.2.0 on crates.io, repo pushed 2026-08-28) renders egui inside Smithay, and its README calls itself "mainly intended for debug interfaces". | https://github.com/Smithay/smithay-egui |

### 1.2 What existing compositors do

**cosmic-comp** (pop-os/cosmic-comp @ 3d55cba, 2026-10-01; tags `epoch-1.9.0`)

- `src/utils/iced/mod.rs` (1264 lines) defines `IcedElement<P: Program>`.
  Rendering imports `iced_tiny_skia`, which is iced's **CPU** renderer. It keeps
  `buffers: HashMap<OrderedFloat<f64>, (MemoryRenderBuffer, Option<(Vec<Layer>, Color)>)>`,
  one CPU buffer per output scale. It diffs iced layer stacks with
  `iced_tiny_skia::graphics::damage::diff` / `damage::group`, rasterises only
  the damaged rects into a `tiny_skia::PixmapMut` over the buffer, and emits a
  `MemoryRenderBufferRenderElement`. A `foreground(pixels, damage, scale)` hook
  lets a program draw raw tiny-skia on top. Verified in source.
- It implements Smithay's `PointerTarget`, `KeyboardTarget`, `TouchTarget` and
  `TabletToolTarget` for the element, so iced receives compositor input
  directly.
- Users of the element: the window stack tab bar (`shell/element/stack.rs`),
  resize and swap indicators, window menus (`shell/grabs/menu`), zoom UI and
  stack hover. COSMIC's panel, launcher, OSD, notifications and lock screen are
  **separate layer-shell clients** (cosmic-panel, cosmic-osd,
  cosmic-notifications, cosmic-greeter), not compositor UI.
- Text goes through cosmic-text 0.19 (HarfRust shaping, swash rasterisation,
  colour emoji, custom fallback lists modelled on Chromium and Firefox).
  https://github.com/pop-os/cosmic-text
- Dependencies: `iced_tiny_skia`/`iced_graphics`/`libcosmic` from git, `tiny-skia = "0.11"`.

**niri** (niri-wm/niri @ ed22699, 2026-10-01; latest release v26.04, 2026-04-25)

- `src/ui/`: `hotkey_overlay.rs`, `exit_confirm_dialog.rs`, `screenshot_ui.rs`,
  `config_error_notification.rs`, `mru.rs` (the recent-windows switcher) and
  `screen_transition.rs`. The text UIs all use **pangocairo**: they create a
  `cairo::ImageSurface` (ARGB32), lay out with Pango (`pango` 0.21.5 with
  `v1_44`, `pangocairo` 0.21.5) and upload it with `TextureBuffer::from_memory`.
  Verified in source. Animation is niri's own: the texture is static and niri
  animates its position and opacity. Pango gives full shaping, fontconfig
  fallback and colour emoji. The project author's announcement: https://mastodon.online/@YaLTeR/111786887237923737
- niri's overview is not a UI toolkit. It is the normal window render path at a
  smaller scale.
- `src/render_helpers/` includes `blur.rs` (a `Blur` program with
  `BlurOptions { passes, offset }` over `GlesTexture`s), `background_effect.rs`,
  `xray.rs`, `shadow.rs`, `shader_element.rs` and `offscreen.rs`: custom GLSL on
  `GlesRenderer`. The v26.04 release notes put blur in mainline, driven by
  `ext-background-effect-v1` or by config. https://github.com/niri-wm/niri/releases/tag/v26.04

**Otto** (nongio/otto, pushed 2026-10-02, v1.5.0 2026-09-26, 99 stars)

- "built from scratch in Rust on Smithay, with a Skia renderer and the lay-rs
  scene graph" (README). `src/skia_renderer.rs` defines a `SkiaRenderer` that
  holds a `GlesRenderer` plus a `skia::gpu::DirectContext` and wraps the
  current EGL framebuffer as a Skia surface (`docs/developer/rendering.md`). It
  uses a Smithay fork (`git = "https://github.com/nongio/smithay"`) and has an
  experimental Vulkan path (PR #225).
- The UI (windows, dock, exposé, app switcher, shadows, blur, animations) is one
  retained `lay-rs` tree with Taffy layout, handed to Smithay as a single
  `SceneElement`. Subtrees can be promoted to DRM planes.
  `laye-rs` is pinned at git tag v1.18.1 and uses skia-safe =0.93 and taffy 0.5.
- The split is a hybrid. The compositor draws the dock, exposé, switcher and
  server-side decorations; the bar, islands, launcher, lock, greeter and polkit
  UI are **clients** built on `otto-kit`. otto-kit's rule is that a component
  must be drawable from a bare `&Canvas`, so the compositor and the clients
  share widgets (`docs/developer/otto-kit.md`).
- The compositor implements AT-SPI pieces itself: accesskit 0.24 /
  accesskit_unix 0.22, and `org.freedesktop.a11y.KeyboardMonitor`
  (`docs/developer/accessibility.md`).
- Maturity: one main author, young; it has AGENTS.md and CLAUDE.md files.
  https://github.com/nongio/otto, https://github.com/nongio/layers

**Pinnacle** (pinnacle-comp/pinnacle, pushed 2026-06-27): Smithay compositor,
configured in Lua or Rust. Its widget system "Snowcap" is a **separate
layer-shell client** on smithay-client-toolkit 0.20 + iced 0.14 (wgpu and
tiny-skia). https://github.com/pinnacle-comp/pinnacle

### 1.3 Option matrix: drawing into a Smithay GLES frame

| Approach | How it reaches the GlesRenderer | Text | Animation | Memory | Maturity (2026-10-02) |
|---|---|---|---|---|---|
| **iced on tiny-skia** (cosmic-comp's way) | CPU raster into `MemoryRenderBuffer`, damage-diffed per layer; upload on change | cosmic-text (HarfRust, swash, fallback, colour emoji) | iced 0.14 `Animation` API; each animated frame is a CPU raster and upload | `w*h*4` B per element per scale (for example, 1920x1080 is 8.3 MB), plus the GL texture | Production in COSMIC for chrome. iced crates are 0.14.0 (2025-12-07); master is 0.15.0-dev. libcosmic is git only. |
| **Pango/Cairo** (niri's way) | CPU `ImageSurface` into `TextureBuffer` | Pango: best-in-class shaping, fallback, emoji | None built in; the compositor moves the static texture | as above | Production in niri for static overlays. Not a widget toolkit. |
| **Skia on the compositor EGL context** (Otto's way) | `DirectContext` on Smithay's EGL context; wrap the bound FBO as a Skia `Surface` | SkParagraph + HarfBuzz + ICU; fontconfig fallback; colour emoji | Whatever you build (lay-rs provides it) | Skia's GPU resource cache plus binary size (prebuilt Skia download at build) | Works in Otto (needs a forked Smithay). skia-safe 0.153.3 (2026-09-04, pre-release tag), tracking Skia m153. |
| **Freya embedded** | `freya_core::Runner` + `Tree::measure_layout` + `RenderPipeline { canvas, .. }.render()` onto any Skia Canvas (the example uses a raster surface; a GL surface on Smithay's FBO is the Otto pattern) | as Skia | `freya-animation` hooks; the app is ticked by the host | Skia as above. One report: an empty Freya app is about 30–60 MB on Windows (discussion #1153, 2025) | Example only, 0.5.0-rc. Freya stable is 0.4.3 (2026-08-30); 0.4.0 was 2026-07-16. Single main maintainer. |
| **femtovg** directly | `femtovg::renderer::OpenGl` on a `glow::Context` (`GlowRenderer::with_context`); `OpenGl::set_screen_target(Some(fbo))` points "screen" at Smithay's FBO (verified, 0.27.0) | Optional `textlayout` feature with rustybuzz, or swash; no system fallback of its own | DIY | small | 0.27.0 (2026-08-31). Unreleased: layers, blend modes, multi-pass Gaussian blur filters. |
| **Slint SoftwareRenderer** | `SoftwareRenderer::render(buffer, stride)` returns dirty regions; feed `MemoryRenderBuffer` damage | parley + fontique layout (since 1.14.0, 2025-10-21), swash glyphs (since 1.16.0) | Slint `animate` bindings, declarative | about 30 MB whole app as a client (one report, below) | 1.18.1 (2026-09-21), stable, commercial backing |
| **Slint FemtoVGRenderer** via `OpenGLInterface` | Renders to `RenderTarget::Screen` = FBO 0; 1.18.1 never calls femtovg's `set_screen_target`, so Smithay's FBO is unreachable through the public API (verified in `i-slint-renderer-femtovg-1.18.1`) | as Slint | as Slint | — | Not usable without patching |
| **Slint `FemtoVGWGPURenderer` / `SkiaWGPU30Renderer`** | Render into a caller's wgpu texture | as Slint | as Slint | wgpu device | `unstable-wgpu-30` feature; needs wgpu in the compositor |
| **egui** (`smithay-egui`, egui_glow) | glow painter on the compositor context | egui 0.34 moved to skrifa + vello_cpu rasterisation; 0.35 uses HarfRust for kerning and ligatures; bundled fonts by default | Immediate mode: re-runs the UI every animated frame | small | 0.36.2 (2026-09-08). smithay-egui says it is for debug UIs. |
| **vello_cpu / vello_gpu** | vello_cpu: CPU raster into a buffer. vello_gpu (renamed from vello_hybrid, 0.1.0 2026-08-26): wgpu or a browser WebGL2 backend. The compute `vello` (0.10.0) is "experimental" | via parley 0.11 / glifo | DIY | — | vello_cpu 0.2.0 is "the most mature choice" per the README; the GPU variants are not yet primary |
| **cosmic-text + taffy + own GLES** | Glyph atlas uploaded as a texture; quads in custom shaders (as niri does with its own shaders) | cosmic-text | DIY | smallest | Building blocks only. taffy 0.14.0 (2026-08-24); glyphon 0.12 is wgpu-only. |
| **wgpu on the compositor's context** (iced_wgpu, vello, glyphon) | `wgpu_hal::gles::Adapter::new_external(get_proc_address, opts)` with the context current (wgpu-hal 30.0.1) | per toolkit | per toolkit | another device abstraction | **No known user in a Smithay compositor.** See OPEN. |

Notes:
- **Text quality ranking** (from the stacks each uses, not a visual test):
  Pango ≈ Skia SkParagraph > cosmic-text ≈ parley/fontique (Slint, vello) >
  femtovg textlayout > egui.
- **Damage:** both proven CPU paths (cosmic-comp, Slint software) produce damage
  rects, so an idle island or bar costs nothing per frame. The GPU paths need
  their own damage tracking; Otto does it in lay-rs.
- **GL state sharing:** Skia caches GL state, so after Smithay touches GL, Skia
  needs `DirectContext::reset`, and Smithay needs its state restored after Skia
  draws. Otto lives with this daily. The exact handshake is in Otto's
  `src/renderer/`.

### 1.4 NVIDIA: memory of extra GL/Vulkan contexts

Measured on this host, 2026-10-02 (`/proc/<pid>/smaps`, `nvidia-smi`):

| Process | RSS | Pss | NVIDIA lib + /dev/nvidia RSS | private `/dev/nvidiactl` | VRAM |
|---|---|---|---|---|---|
| `qs` (Isle, Quickshell) | 458 MB | 342 MB | 82 MB | 32.9 MB | 83 MiB |
| Hyprland | 231 MB | 151 MB | 72 MB | 30.9 MB | 133 MiB |

- The largest shared libraries mapped: `libnvidia-gpucomp.so` (27.6 MB RSS),
  `libnvidia-eglcore.so` (11.3 MB) and `libnvidia-glcore.so` (6.8 MB). These are
  shared between processes (Pss of NVIDIA libraries in `qs` is 10.8 MB). The
  `/dev/nvidiactl` mapping is per process (Pss = RSS).
- So each **separate GPU client process** on NVIDIA starts at roughly
  **40+ MB Pss** of driver mappings plus its VRAM before drawing anything. Five
  always-running GPU clients would cost about 200 MB of driver overhead alone.
  This argues for keeping always-on UI inside the compositor, and for big apps
  being launched on demand or rendering on the CPU.
- Data points from others:
  - A process monitor rewritten from egui (GPU) to the Slint software renderer
    went from about 135 MB to about 30 MB RSS. The author says NVIDIA's
    libraries are about 20 MB. Trystan Sarrade, 2026-06-03,
    https://trystan-sarrade.com/article/rust-gui-135mb-to-30mb-egui-to-slint/
  - iced 0.14 empty window: about 55 MB with wgpu/Metal, about 115 MB with
    tiny-skia (the full-size frame buffer). This was macOS M4, not Linux.
    https://github.com/officialaritro/reqlite/issues/1 (read 2026-10-02)
  - GTK4 hello world: over 100 MB, 33 MB with `GSK_RENDERER=cairo`. EndeavourOS
    forum, 2026-04-28,
    https://forum.endeavouros.com/t/how-to-significantly-reduce-memory-usage-memory-usage-in-gtk4-and-libadwaita-apps/79360
- **Compositor VRAM on NVIDIA:** the driver's free-buffer pool can push a
  compositor towards 1 GiB. The fix is an application profile with
  `GLVidHeapReuseRatio = 0`, matched by process name, in
  `/etc/nvidia/nvidia-application-profiles-rc.d/`. A new compositor binary
  needs its own rule. niri expects about 100 MiB. niri wiki "Nvidia"
  (`docs/wiki/Nvidia.md` @ ed22699), upstream
  https://github.com/NVIDIA/egl-wayland/issues/126#issuecomment-2379945259
- A **second context inside the compositor** (for example a wgpu or Vulkan
  device next to Smithay's EGL) would need dmabuf export/import plus explicit
  sync between APIs to share textures. A shared EGL context
  (`EGLContext::new_shared`) avoids that. Per-context (as opposed to
  per-process) cost was not measured: see OPEN.

---

## 2. Client toolkits for the separate apps

| Toolkit | Version (date) | Model | Renderer on Wayland | Animation | Theming | Accessibility | Layer-shell | Hot reload |
|---|---|---|---|---|---|---|---|---|
| **iced** | 0.14.0 (2025-12-07); master 0.15.0-dev active 2026-10-02 | Elm (update/view/message), retained widgets | wgpu (wgpu 29 on master) or tiny-skia | `Animation` API (0.14) | `Theme` + per-widget style fns; system theme reactions | Upstream: none. pop-os/iced fork has `a11y` (accesskit) | `iced_layershell` 0.19.1 (waycrate, 2026-07-12); libcosmic via sctk | 0.14 "Hot reloading" via `subsecond` and `cargo-hot`, self-described "very, very experimental" (PR #3000) |
| **libcosmic** | git only; tags `epoch-1.9.0` (2026-09-23) | iced fork + COSMIC widgets and app framework | Default features omit wgpu (tiny-skia); apps opt into `wgpu` | iced | cosmic-theme (RON via cosmic-config, live) | a11y on by default | yes (applets, panels) | config live; code no |
| **Slint** | 1.18.1 (2026-09-21) | `.slint` DSL, compiled to Rust, or interpreted at runtime | winit + FemtoVG (GL or wgpu), Skia (GL, Vulkan, wgpu), software, experimental Vello (1.18) | `animate` declarations, states and transitions | globals and styles (fluent, material, cosmic, qt) | AccessKit; text-input content exposed in 1.18 | No native support; community crates (layer-shika, Spell) work around it (discussion #6910) | `live-preview` feature + `SLINT_LIVE_PREVIEW` (since 1.13.0, 2025-09-03); `slint-interpreter` 1.18.1 |
| **Freya** | stable 0.4.3 (2026-08-30); 0.5.0-rc.8 (2026-09-29) | Components + hooks (`use_state`), builder elements (`rect().child(..)`); no Dioxus dependency on main | Skia over GL (glutin) or Vulkan (ash); 0.5 adds a software renderer | `freya-animation` (transitions, sequences) | theme definitions, accent and preference examples | AccessKit (`freya-winit/accessibility.rs`) | none found in the repo | `hotreload` feature = `subsecond` + `dioxus-devtools` |
| **GTK4 (gtk4-rs) / Relm4** | gtk4 0.11.5 (2026-09-20; 0.11.0 2026-02-20); relm4 0.11.0 (2026-04-08); libadwaita 0.9.2 | Relm4: Elm-like components over GTK widgets | GSK Vulkan by default on Wayland since GTK 4.15/4.16; ngl; cairo | CSS transitions, `AdwAnimation` | CSS, libadwaita | AT-SPI native, the most complete | `gtk4-layer-shell` 0.8.1 (2026-08-10) | GTK Inspector live CSS; none for code |
| **egui / eframe** | 0.36.2 (2026-09-08) | Immediate mode | glow or wgpu | `animate_*` helpers | `Style`/`Visuals` | AccessKit | community crates only | n/a |
| **Xilem / Masonry** | 0.4.0 (2025-10-29); main active 2026-09-14 | Reactive view tree (Xilem) over a retained widget tree (Masonry) | Vello (compute) + wgpu 28; imaging backends, including vello_cpu and vello_hybrid, being made selectable (#1847) | basic | basic | AccessKit | no | no |

Idle RSS with a source:

| Toolkit | Number | Conditions | Source |
|---|---|---|---|
| Slint (software renderer) | ~30 MB | Linux process monitor app | trystan-sarrade.com, 2026-06-03 |
| egui | ~135 MB | same app, GPU | same |
| iced 0.14 | ~55 MB (wgpu), ~115 MB (tiny-skia) | macOS M4, empty window | reqlite issue #1 |
| Freya | ~30–60 MB | Windows, empty app, 2025 | freya discussion #1153 |
| GTK4 | >100 MB; 33 MB with cairo | hello world | EndeavourOS forum, 2026-04-28 |

None of these were measured on Linux with NVIDIA under the same conditions. See OPEN for a benchmark protocol.

Fit for "glass" apps: no toolkit needs to blur behind its own window. The
compositor implements `ext-background-effect-v1`, and the app sets a translucent
background and requests a blur region through the protocol. Freya's
`backdrop_blur` and Slint's Skia-only shadows apply *inside* a window.

---

## 3. Desktop services layer

zbus: **5.19.0** (2026-08-09); zbus_macros 5.19.0, zvariant 5.15.0,
zbus_xmlgen 5.4.1 (2026-07-17). The repository moved to `z-galaxy/zbus` and was
pushed 2026-10-02. No zbus 6 pre-release was found.

| Need | Crate(s) and version (date) | D-Bus stack | Notes |
|---|---|---|---|
| NetworkManager | `networkmanager` 0.5.0 (2025-06-26); `nm-rs` 0.1.3 (libnm, GObject); COSMIC `cosmic-dbus-networkmanager` (git, pop-os/dbus-settings-bindings) | zbus (COSMIC) | No maintained crates.io zbus crate with COSMIC's coverage. The dbus-settings-bindings repo has networkmanager, nm-secret-agent-manager, bluez, upower, mpris2, a11y, accounts, geoclue2, hostname1, locale1, timedate, switcheroo-control, cosmic-settings-daemon. A Wi-Fi password prompt needs an NM secret agent. |
| BlueZ | `bluer` 0.17.4 (2025-06-06, repo active 2026-08-17), official BlueZ | **libdbus** (`dbus`, `dbus-tokio`, `dbus-crossroads`) | Brings in a second D-Bus stack. COSMIC uses `bluez-zbus` from dbus-settings-bindings instead. |
| UPower | `upower_dbus` 0.3.2 (crates.io, 2023-01-09, stale); git version in dbus-settings-bindings; `wayle-battery` 0.1.2 | zbus | Write the proxies yourself, or vendor. |
| logind | `logind-zbus` 5.3.2 (2025-05-20, used by cosmic-osd and cosmic-greeter); `zbus_systemd` 0.26200.0 (2026-09-29, tracks systemd 262, covers login1, systemd1 and more) | zbus | Idle inhibit, lid, PrepareForSleep, session Lock/Unlock signals. |
| MPRIS client | `mpris` 2.1.0 (2026-04-18): **libdbus**, blocking; COSMIC `mpris2-zbus` (git); `wayle-media` 0.1.3 | dbus / zbus | `mpris-server` 0.10.0 (zbus) is for *being* a player. |
| Notifications server | None usable. `notify-rust` 4.18.1's `server` feature is commented out in Cargo.toml. Reference: cosmic-notifications (zbus 5.14, `p2p`), `wayle-notification` 0.1.4 | zbus `#[interface]` | Implement `org.freedesktop.Notifications` (Notify, CloseNotification, GetCapabilities, GetServerInformation, NotificationClosed, ActionInvoked, ActivationToken) yourself. It is small. |
| StatusNotifierItem host | `system-tray` 0.8.9 (2026-08-29, zbus 5, used by ironbar; optional GTK3 dbusmenu); `status-notifier` 0.1.1 (2026-09-30, new); `libtrayd` 0.2.0; `wayle-systray` 0.1.4; `stray` 0.1.3 (2023, stale) | zbus | You must also own `org.kde.StatusNotifierWatcher` if nothing else does (Isle does today). |
| polkit agent | Pure Rust: cosmic-osd `src/subscriptions/polkit_agent.rs` + `polkit_agent_helper.rs` (zbus 5.14, logind-zbus); talks to `/run/polkit/agent-helper.socket`, or spawns `polkit-agent-helper-1` if `POLKIT_AGENT_HELPER_1` is set at build. Crates: `zbus-polkit-agent` 0.4.3 (2026-05-07), `polkit-agent-rs` 0.3.0 (libpolkit-agent-1, GObject), `zbus_polkit` 5.1.0 (client, for CheckAuthorization) | zbus | On this host, polkit 127's `polkit-agent-helper-1` is mode 0755 (not setuid) and `polkit-agent-helper.socket` is active, so the socket path is required. Register with `org.freedesktop.PolicyKit1.Authority.RegisterAuthenticationAgent` and implement `org.freedesktop.PolicyKit1.AuthenticationAgent`. |
| Secret Service | Client: `oo7` 0.6.0 (stable 2026-02-21; 0.7.0.beta 2026-09-21; also reads keyring files directly), `secret-service` 5.2.0 (2026-08-29). Server: **oo7-daemon** (same repo; gnome-keyring replacement with PAM integration, a portal and KWallet migration) | zbus | For unlock prompts, oo7-daemon calls `org.gnome.keyring.SystemPrompter` (`org.gnome.keyring.internal.Prompter`, `/org/gnome/keyring/Prompter`), or `org.kde.secretprompter`. gnome-keyring 50 (installed) uses the same SystemPrompter name. A shell that wants its own unlock dialog serves that interface. |
| UDisks2 | `udisks2` 0.3.1 (2026-01-21, zbus 5.10) | zbus | Unofficial but current. |
| power-profiles-daemon | No dedicated crate (`wayle-power-profiles` 0.1.4). Bus names on host: `org.freedesktop.UPower.PowerProfiles` and the legacy `net.hadess.PowerProfiles` (ppd 0.30) | zbus proxy | A few properties (ActiveProfile, Profiles, PerformanceDegraded) plus HoldProfile. |
| GameMode | No crate. `com.feralinteractive.GameMode` on the session bus (gamemoded 1.8.2 running) | zbus proxy | RegisterGame/UnregisterGame, ClientCount. |
| PipeWire | `pipewire` 0.10.1 / `libspa` 0.10.1 (2026-08-19; gitlab.freedesktop.org/pipewire/pipewire-rs active 2026-08-26). Has `metadata::Metadata::set_property` and `add_listener_local().property(..)`, and `Node::set_param` (verified in the crate source). docs.rs failed to build 0.10.1; docs are at pipewire.pages.freedesktop.org | libpipewire FFI | Default sink: the `default` metadata object keys (`default.configured.audio.sink`). |
| WirePlumber | `wireplumber` 0.0.1 (2023, empty) | — | No Rust bindings. COSMIC's sound code uses **pipewire + libpulse-binding** 2.30 (through pipewire-pulse); wayle-audio uses libpulse-binding. See OPEN on per-device volume. |
| Portal backend | `ashpd` 0.13.13 (2026-07-17) has an *unstable* `backend` feature for portal implementations, with a backend demo. xdg-desktop-portal-cosmic uses ashpd only for types and implements `org.freedesktop.impl.portal.*` with zbus `#[interface]` directly (Access, FileChooser, RemoteDesktop, Screenshot, Settings, ScreenCast), with ScreenCast over pipewire-rs (git). Otto also ships `xdg-desktop-portal-otto`. | zbus | Isle already serves `org.freedesktop.impl.portal.desktop.isle`. |
| Gamepads | `gilrs` 0.11.2 (2026-05-30; evdev on Linux, SDL mappings, force feedback); `evdev` 0.13.2 (2025-09-15) | — | libinput (the compositor) ignores joysticks, so a big-picture client reads them itself. Access is through the logind/udev uaccess ACLs on joystick devices (standard systemd rule, not re-verified here). Steam Input may hold the device. |
| PAM | `pam-client` 0.5.0 (2022, used by cosmic-greeter); `pam-client2` 0.5.5 (2026-03-27, maintained fork); `nonstick` 0.1.2 (2026-03-13, apps and modules); `pam` 0.8.0 / `pam-sys` 0.5.6 (2023) | libpam FFI | Run PAM conversations off the compositor thread: modules such as fprintd block. Otto has a PAM-backed `ext-session-lock-v1` locker and a greetd greeter. |

Comparable service collections: **wayle-services** (wayle-rs, 2026-07-27,
zbus 5.11 + libpulse-binding; crates for audio, battery, bluetooth, network,
notification, systray, media, power-profiles, brightness, sysinfo, Hyprland and
niri IPC), and COSMIC's `cosmic-settings-subscriptions` (features: pipewire,
pulse, network_manager, bluetooth, upower, accessibility).

---

## 4. Config and live reload

| Pattern | Who | Mechanics | Crates (version, date) |
|---|---|---|---|
| **KDL, whole-file reload** | niri | `src/utils/watcher.rs` polls every **500 ms**, comparing mtime **and** the canonical path, because a Nix symlink's mtime is 1970. On change it re-parses and applies everything (binds, outputs, rules, animations). Parse errors appear in-compositor through `config_error_notification`. `include "other.kdl"` is supported. | `knuffel` 3.2.0 (2023-04-15, niri still uses it), `knus` 3.4.0 (2026-06-21, maintained fork), `kdl` 6.7.1 (2026-05-31, format-preserving, KDL v2), `miette` diagnostics |
| **RON, one file per key** | COSMIC | `cosmic-config`: `$XDG_CONFIG_HOME/cosmic/<app-id>/v<N>/<key>` files; `notify` 8.2 watcher (optional calloop source); `CosmicConfigEntry` derive gives typed partial updates; optional D-Bus settings-daemon path | `ron` 0.12.2 (2026-06-22), `notify` 8.2.0 (2026-08-30) |
| **TOML** | Otto (`otto_config.example.toml`, `toml_edit` for writing back from otto-settings over D-Bus) | settings app edits the running compositor over D-Bus | `toml` 1.1.6 (2026-09-10), `toml_edit` |
| **Lua** | Pinnacle (Lua or Rust config, run as a separate process talking gRPC to the compositor) | restart the config process to reload | `mlua` 0.12.1 (2026-08-29; Lua 5.1–5.5, LuaJIT, Luau, async) |
| **Rhai** | — | embedded scripting | `rhai` 1.26.1 (2026-09-10) |

Live-reloading UI layout and theme without a restart:
- **Theme or design tokens as data** (colours, radii, blur strength, durations)
  in a watched file, applied as state. Every retained toolkit re-renders on a
  state change. This is what cosmic-theme and niri do, and it is
  toolkit-agnostic.
- **Slint:** `slint-interpreter` compiles `.slint` at run time. A shell can
  rebuild the component from source on file change and keep its models. The
  `live-preview` feature (1.13+) does this for compiled apps in development.
  This is the only toolkit here where *layout* reload is a supported runtime
  feature rather than code patching.
- **Code hot-patching (development only):** `subsecond` 0.7.x (0.8.0-alpha.1,
  2026-07-30) patches functions in a running binary. iced 0.14 integrates it
  (with `cargo-hot`) and Freya has `hotreload = ["dep:subsecond",
  "dep:dioxus-devtools"]`. Using it inside a compositor process is untested
  (see OPEN).

---

## OPEN

- **Per-context GPU cost inside one process on NVIDIA.** How much RSS and VRAM a
  second EGL context (shared or not), or a wgpu/Vulkan device, adds to the
  compositor process. Only per-process figures were measured.
- **Idle RSS of each client toolkit on this machine** (Linux, NVIDIA 615,
  Wayland). No same-conditions benchmark exists. Proposed protocol: a release
  build of one window with a label and a button for iced 0.14 (wgpu and
  tiny-skia), libcosmic, Slint 1.18 (software, femtovg, skia), Freya 0.4/0.5
  (GL, software), gtk4-rs (vulkan, ngl, cairo) and egui (glow, wgpu). Record
  Pss and Rss from `smaps_rollup` after 30 s idle, plus `nvidia-smi` VRAM, on
  both the NVIDIA and AMD render nodes.
- **wgpu on Smithay's context.** Whether `wgpu_hal::gles::Adapter::new_external`
  plus `wgpu::Adapter::create_adapter_from_hal` is enough for iced_wgpu,
  glyphon or vello inside a GLES compositor, how textures cross between them,
  and whether iced's wgpu version (29 on master) lines up.
- **Freya embedded in a compositor.** How `freya_core::Runner`'s async/reactive
  runtime is driven from calloop. Whether input mapping (`PlatformEvent`) and
  accessibility work without `freya-winit`. Whether `freya-skia-safe` GL
  surfaces wrap Smithay's FBO the way Otto's skia-safe 0.93 does. Not tried.
- **Freya 0.4 release blog** (https://freyaui.dev/posts/0.4) returned HTTP 429;
  the claim that 0.4 dropped Dioxus rests on the current main Cargo.toml having
  no `dioxus` dependency and the README's API.
- **femtovg colour emoji** support and its exact fallback behaviour.
- **egui system-font fallback** in 0.36 (bundled fonts by default).
- **WirePlumber volume semantics.** Whether setting `Props.channelVolumes` on a
  hardware sink node through `Node::set_param` is equivalent to `wpctl
  set-volume`, which goes through WirePlumber's mixer API and device Route
  params. Not verified; the pulse API or `wpctl` are the known-good paths.
- **Slint licence terms** for this use (royalty-free licence for desktop apps).
  Not re-read here.
- **Otto's Smithay fork:** what it changes against upstream and whether the
  Skia-on-GLES wrapper needs it. Not diffed.
- **Gamepad device permissions** under logind uaccess for a non-root client on
  CachyOS. Not re-checked on this host.
- **The reqlite iced memory issue's date.** The fetch reported 2026-10-02, which
  may be the fetch date rather than the post date.

---

## Sources (all read 2026-10-02)

- crates.io API, `https://crates.io/api/v1/crates/<name>`, for every version and date above
- cosmic-comp source @ 3d55cba: https://github.com/pop-os/cosmic-comp (`src/utils/iced/mod.rs`)
- niri source @ ed22699 and the v26.04 release: https://github.com/niri-wm/niri, https://github.com/niri-wm/niri/releases/tag/v26.04
- niri NVIDIA wiki: https://github.com/niri-wm/niri/blob/main/docs/wiki/Nvidia.md
- Otto: https://github.com/nongio/otto (README, `docs/developer/rendering.md`, `otto-kit.md`, `accessibility.md`, `Cargo.toml`); layers: https://github.com/nongio/layers
- Pinnacle and Snowcap: https://github.com/pinnacle-comp/pinnacle
- Smithay issue #928: https://github.com/Smithay/smithay/issues/928; GlowRenderer docs: https://docs.rs/smithay/latest/smithay/backend/renderer/glow/struct.GlowRenderer.html; smithay-egui: https://github.com/Smithay/smithay-egui
- Freya source @ cfd97ac (main, 0.5.0-rc.8): https://github.com/marc2332/freya (`examples/feature_embedded.rs`, `crates/freya-core/Cargo.toml`); releases via `gh release list`
- Freya memory discussion: https://github.com/marc2332/freya/discussions/1153
- Slint CHANGELOG: https://github.com/slint-ui/slint/blob/master/CHANGELOG.md; femtovg renderer docs: https://docs.rs/slint/latest/slint/platform/femtovg_renderer/index.html; crate source `i-slint-renderer-femtovg` 1.18.1
- Slint layer-shell discussions: https://github.com/slint-ui/slint/discussions/1765, https://github.com/slint-ui/slint/discussions/6910
- femtovg CHANGELOG and source 0.27.0: https://github.com/femtovg/femtovg
- rust-skia 0.153.2 release: https://github.com/rust-skia/rust-skia/releases/tag/0.153.2
- iced 0.14.0 release and PR #3000: https://github.com/iced-rs/iced/releases/tag/0.14.0, https://github.com/iced-rs/iced/pull/3000
- pop-os/iced and libcosmic Cargo.toml: https://github.com/pop-os/iced, https://github.com/pop-os/libcosmic
- cosmic-text README: https://github.com/pop-os/cosmic-text
- Vello README: https://github.com/linebender/vello
- Xilem README and Cargo.toml: https://github.com/linebender/xilem
- egui CHANGELOG: https://github.com/emilk/egui/blob/main/CHANGELOG.md
- wgpu-hal `gles::Adapter::new_external`: https://docs.rs/wgpu-hal/latest/wgpu_hal/gles/struct.Adapter.html
- Memory reports: https://trystan-sarrade.com/article/rust-gui-135mb-to-30mb-egui-to-slint/ ; https://github.com/officialaritro/reqlite/issues/1 ; https://forum.endeavouros.com/t/how-to-significantly-reduce-memory-usage-memory-usage-in-gtk4-and-libadwaita-apps/79360
- gtk4-rs releases: https://newreleases.io/project/github/gtk-rs/gtk4-rs/release/0.11.0
- wayland-protocols tree and releases (ext-background-effect in staging; 1.49 2026-06-07): https://gitlab.freedesktop.org/wayland/wayland-protocols
- COSMIC services: https://github.com/pop-os/cosmic-osd (polkit agent), https://github.com/pop-os/cosmic-greeter, https://github.com/pop-os/xdg-desktop-portal-cosmic, https://github.com/pop-os/cosmic-notifications, https://github.com/pop-os/cosmic-applets, https://github.com/pop-os/cosmic-settings-daemon, https://github.com/pop-os/dbus-settings-bindings, https://github.com/pop-os/cosmic-settings-subscriptions
- oo7: https://github.com/linux-credentials/oo7 (README, `server/src/gnome/prompter.rs`, `server/src/plasma/prompter.rs`)
- ashpd README (backend feature): https://github.com/bilelmoussaoui/ashpd
- bluer: https://github.com/bluez/bluer; mpris-rs: https://github.com/Mange/mpris-rs; notify-rust: https://github.com/hoodie/notify-rust; system-tray: https://github.com/jakestanger/system-tray
- pipewire-rs: https://gitlab.freedesktop.org/pipewire/pipewire-rs (crate source 0.10.1)
- wayle-services: https://github.com/wayle-rs/wayle-services
- Host measurements: `/proc/<pid>/smaps{,_rollup}`, `nvidia-smi`, `busctl --system/--user list`, `pacman -Q`, `systemctl list-units polkit-agent-helper*`, on 2026-10-02
