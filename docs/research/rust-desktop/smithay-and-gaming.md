# Smithay, gaming and precedent compositors: state as of 2026-10-02

Research for a from-scratch Rust Wayland compositor on Smithay with the shell
(bar/island, launcher, switcher, overview, lock, notifications) drawn inside
the compositor. Target: CachyOS, RTX 5070 Ti, gaming plus coding.

Every claim carries a source and a date. Where a source is a git repository,
the claim was checked against a clone taken on 2026-10-02 (the commit date is
given). Anything not verified is under **OPEN** at the end and nowhere else.

---

## 0. The machine this is for (read from the host, 2026-10-02)

These come from `pacman -Q`, `/sys/class/drm` and `edid-decode` on the host,
not from the web. They change what matters below.

| Item | Value |
|---|---|
| dGPU | NVIDIA GB203 GeForce RTX 5070 Ti, `card2` / `renderD129`, PCI 01:00.0, `boot_vga=1` |
| iGPU | AMD Granite Ridge Radeon, `card1` / `renderD128`, PCI 79:00.0, `boot_vga=0` |
| Connected output | `card2-DP-4` (on the NVIDIA card) |
| Monitor | AOC U34G2G4R3, 3440x1440, modes to 144 Hz; EDID range limits 48-144 Hz; 10 bpc; HDR static metadata block with SMPTE ST2084; desired content max luminance about 302 cd/m2 |
| Kernel | `linux-cachyos 7.2.8-2` with `linux-cachyos-nvidia-open` |
| NVIDIA | `nvidia-utils 615.71.09`; `nvidia_drm` params `modeset=Y`, `fbdev=Y`, plus `color_pipeline` and `vblank` params exposed |
| Stack | mesa 26.2.4, wayland 1.26.0, wayland-protocols 1.49, xorg-xwayland 24.1.13, pipewire 1.6.9, xdg-desktop-portal 1.22.1, gamescope 3.16.31, egl-gbm 1.1.4, egl-wayland 1.1.23, egl-wayland2 1.0.2 |

Consequences:

- This is a two-GPU machine. The AMD iGPU enumerates first (`renderD128`).
  Smithay's `udev::primary_gpu()` picks the GPU with `boot_vga=1` first
  (`src/backend/udev.rs`, smithay master 2026-09-30), which here is the NVIDIA
  card, so the default is right. Anything that picks "the first render node"
  would pick the AMD iGPU. Keep a config override like niri's
  `render-drm-device` debug option.
- The monitor is VRR-capable (48-144 Hz) and nominally HDR10, but at about
  300 nits HDR has little payoff. VRR matters; HDR is optional.
- Blackwell (RTX 50) is supported only by NVIDIA's open kernel modules
  (stated in NVIDIA R610 docs per
  [gist, 2026](https://gist.github.com/jatinkrmalik/86afb07cbe6abf5baa2d29d3842aa328)
  and [Medium, Debian 13 RTX 50](https://medium.com/@datatomas/getting-nvidia-rtx-50-series-working-on-debian-13-open-drivers-kde-and-gaming-considerations-d856730fbc11));
  the host already runs `nvidia-open`.

---

## 1. Smithay

### 1.1 Release and maintenance

- **Latest crates.io release: 0.7.0, published 2025-06-24.** Earlier: 0.6.0
  2025-04-25, 0.5.0/0.5.1 2025-02-27, 0.4.0-0.4.4 2025-01-23/24, and before
  that 0.3.0 on 2021-07-25. Source: crates.io API
  `https://crates.io/api/v1/crates/smithay/versions`, queried 2026-10-02.
- **No release for about 15 months, but master is very active.** 461 commits
  between 2025-06-24 and 2026-09-30; last commit 2026-09-30
  ("drm: allow disabling VRR on legacy surfaces"). `Cargo.toml` on master
  still says `0.7.0`, MSRV 1.87. Source: clone of
  [Smithay/smithay](https://github.com/Smithay/smithay) at 118e34f, 2026-09-30.
- **Everyone depends on git master, not crates.io.** niri uses
  `git = "https://github.com/Smithay/smithay.git"`; cosmic-comp patches to
  `rev = "e3d461a"`; Denial pins rev `812bd33`; Otto uses its own fork
  (`nongio/smithay`, branch `feat/vulkan-device`). Sources: each project's
  `Cargo.toml`, clones of 2026-10-01/02. A Rust forum answer from 2026-08
  gives the same advice: start from `smallvil`, then `anvil`, depend on git
  ([users.rust-lang.org, 2026-08](https://users.rust-lang.org/t/re-writing-a-wayland-compositor/141812)).
- **Who maintains it** (commits 2025-10-01 to 2026-09-30, `git shortlog`):
  Victoria Brekenfeld (Drakulix, System76/COSMIC) 113, Ian Douglas Scott
  (System76) 57, Brian J. Tarricone (xfwl4, funded by Xfce) 44, Hojjat 16,
  Ivan Molodetskikh (YaLTeR, niri) 14, Christian Meissl 10, PolyMeilex 9.
  In practice Smithay's direction follows COSMIC's needs, with niri and xfwl4
  as the other heavy users.
- **API churn on master (unreleased, in `CHANGELOG.md`):** all `delegate_*!`
  macros replaced by one `delegate_dispatch2!` built on new
  `Dispatch2`/`GlobalDispatch2` traits, which are meant to replace
  wayland-server's `Dispatch` in a future wayland-rs (open PR
  [#2017](https://github.com/Smithay/smithay/pull/2017), updated 2026-09-24,
  "WIP `Dispatch`, `WEnum` API changes in new wayland-rs"); xdg/layer/lock
  surfaces moved to `last_acked` committed state; drag-and-drop rewritten
  into a generic `input::dnd`; `InputTime` replaces `u32`/`u64` timestamps;
  `DrmOutputManager` now needs an explicit `lock()`. Expect to chase master.
- **Contribution policy:** `AI.md` on master asks contributors to disclose
  generated code in commit messages and "strongly advise[s] against" LLM use.
  In [PR #2099](https://github.com/Smithay/smithay/pull/2099) (2026-07-17) a
  maintainer closed an async-flip PR partly because it "looks very
  AI-generated". Relevant if this project ever upstreams fixes.
  xwayland-satellite added its own AI policy in v0.8.3 (2026-09-24).

### 1.2 In-tree backends and renderers (master, 2026-09-30)

From `src/backend/` and `[features]` in `Cargo.toml`:

| Area | What is there |
|---|---|
| Session | `backend_session_libseat` only (libseat covers logind and seatd). No direct-logind backend. |
| Devices | `backend_udev`, `backend_libinput`, `backend_libei` (default feature, EI input emulation) |
| DRM/KMS | `backend_drm`: atomic and legacy surfaces, `DrmCompositor` (plane assignment, direct scanout, damage, VRR API), `DrmOutputManager`, GBM framebuffer exporter, `smithay-drm-extras` (connector scanner, EDID, CRTC mapper) |
| Allocators | GBM, dumb buffers, udmabuf, Vulkan (`backend_vulkan`, allocator only) |
| Nested/dev | `backend_winit` (run as a window), `backend_x11` |
| Renderers | `renderer_gl` (GlesRenderer, GLES 2/3), `renderer_glow` (glow wrapper, used for egui), `renderer_pixman` (software), `renderer_multi` (`GpuManager`/`MultiRenderer` for render-on-one-GPU, scan-out-on-another) |
| Profiling | `tracy_gpu_profiling` (GPU zones in Tracy, added late 2025 by YaLTeR per niri 26.04 notes) |
| Vulkan renderer | Not in tree. Draft [PR #2165](https://github.com/Smithay/smithay/pull/2165) by Drakulix, opened 2026-09-09: "Very early draft ... still lacks a lot of functionality needed to run a full compositor." |

VRR: `DrmSurface::use_vrr`, `vrr_supported`, `vrr_enabled`, and
`DrmCompositor::vrr_supported` exist. Atomic VRR value testing was fixed
2025-11-13; `use_vrr(false)` became a no-op on legacy DRM 2026-09-30.

NVIDIA handling inside Smithay: `DrmCompositor` skips `IN_FENCE_FD` on
nvidia-drm older than 560.35.03 because it made commits fail
(`src/backend/drm/compositor/mod.rs`, citing
[NVIDIA/open-gpu-kernel-modules#622](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/622)).
`DrmDeviceFd::new` no longer tries to become master on render nodes
(2026-09-28).

Multi-GPU: `GpuManager` cache cleanup and invalidation were added 2026-08-14;
per-surface texture caches are freed on surface destroy (2026-07-16);
cross-device dmabuf export can be disabled (2026-06-29). This matters on the
host's AMD+NVIDIA box if a monitor is ever plugged into the motherboard.

### 1.3 Wayland protocol handlers in tree (master, 2026-09-30)

From `src/wayland/` and `src/wayland/shell/`:

| Protocol | Status in Smithay master | Notes |
|---|---|---|
| wlr-layer-shell | Yes, v5 (`set_exclusive_edge`, added 2025-10-30) | `shell/wlr_layer`; desktop helper `layer_map_for_output` |
| ext-session-lock-v1 | Yes | Lock reset when the locking client disconnects (2026-08-11); multiple in-flight lockers disambiguated |
| xdg-decoration | Yes (`shell/xdg/decoration.rs`) | Plus KDE server-decoration (`shell/kde`) |
| xdg-shell, xdg-dialog, xdg-foreign, xdg-activation, xdg-toplevel-icon, xdg-toplevel-tag, xdg-system-bell | Yes | |
| ext-image-capture-source-v1 + ext-image-copy-capture-v1 | Yes, merged 2026-02-03 ([PR #1902](https://github.com/Smithay/smithay/pull/1902)) | Output, toplevel and custom sources; cursor sessions. Protocol plumbing only: the compositor renders into the client's buffer itself. |
| wlr-screencopy-unstable-v1 | **No** | niri implements its own (`src/protocols/screencopy.rs`) |
| ext-foreign-toplevel-list-v1 | Yes (compositor may choose the identifier, 2026-09-02) | |
| wlr-foreign-toplevel-management | **No** | niri and Otto implement their own; COSMIC uses its own `cosmic-toplevel-info/management` |
| wp-tearing-control-v1 | **No.** [PR #1325](https://github.com/Smithay/smithay/pull/1325) (Wayland side, opened 2024-02-10) still open, last activity 2026-07-05 | |
| DRM async page flip (needed for tearing) | **No.** [PR #1948](https://github.com/Smithay/smithay/pull/1948) "Initial support for DRM async page flip v2" (cmeissl, opened 2026-02-28, last activity 2026-08-28) open. Compositor chooses the presentation mode per frame; overlay planes disabled during async flips. A user reported on 2026-07-02: "tested in niri with rtx 4060 mobile (proprietary 610.43.02) ... async flips engage as needed." | Earlier [PR #1335](https://github.com/Smithay/smithay/pull/1335) also open; [PR #2099](https://github.com/Smithay/smithay/pull/2099) closed |
| linux-drm-syncobj-v1 (explicit sync) | Yes (`wayland/drm_syncobj`), gated on `supports_syncobj_eventfd`; sync-file import and timeline accessors added 2026-07-22 | Compositor must wire it per import device |
| wp-fractional-scale-v1, wp-viewporter | Yes | |
| wp-presentation-time | Yes | |
| wp-fifo-v1, wp-commit-timing-v1 | Yes, merged 2024-11-22 ([PR #1588](https://github.com/Smithay/smithay/pull/1588)); in every release since 0.4 (both modules listed on docs.rs for 0.7.0) | Neither niri nor cosmic-comp ships them yet (section 2) |
| idle-inhibit, ext-idle-notify | Yes | |
| pointer-constraints, relative-pointer, pointer-gestures, pointer-warp-v1 | Yes | pointer-warp merged 2025-08-14 |
| keyboard-shortcuts-inhibit, xwayland-keyboard-grab, xwayland-shell | Yes | |
| wp-drm-lease-v1 | Yes; unauthenticated fds allowed since 2026-08-10 | niri and cosmic-comp implement the handler (VR headsets) |
| linux-dmabuf v4+, single-pixel-buffer, alpha-modifier, content-type, cursor-shape, security-context, tablet, text-input, input-method, virtual-keyboard | Yes | |
| wlr/ext data-control, primary selection | Yes | |
| ext-background-effect-v1 (blur requests) | Yes, merged 2026-02-04 ([PR #1860](https://github.com/Smithay/smithay/pull/1860)) | |
| Color management / color representation (HDR) | **No.** [PR #1143](https://github.com/Smithay/smithay/pull/1143), Drakulix, draft since 2023-09-25: "won't go out of draft or be merged any time soon ... There is no infrastructure in smithay to handle the new information." Last activity 2026-10-01. [PR #982](https://github.com/Smithay/smithay/pull/982) also open. A community fork (`dividebysandwich/smithay` branch `ds-hdr-master`, last commit 2026-07-10) carries color management plus atomic connector color state, with a matching niri branch. | |
| wlr-output-management, wlr-gamma-control, wlr-virtual-pointer, ext-workspace | **No** in tree. Open PRs: virtual pointer [#2036](https://github.com/Smithay/smithay/pull/2036) and [#2159](https://github.com/Smithay/smithay/pull/2159); workspace [#2171](https://github.com/Smithay/smithay/pull/2171) (2026-09-26) | niri implements output-management, gamma-control, virtual-pointer and ext-workspace itself (`src/protocols/`) |

### 1.4 Xwayland: in-tree XWM versus xwayland-satellite

**In-tree X11 WM (`src/xwayland/xwm`)** received a lot of work between
2025-11 and 2026-09 (git log): XDND both directions between X11 and Wayland
(2025-11-25), `_NET_ACTIVE_WINDOW`, EWMH states (sticky, shaded,
demands-attention), `_NET_WM_PING`, `_NET_WM_SYNC_REQUEST` (2026-05-13),
`_NET_WM_OPAQUE_REGION`, `_GTK_FRAME_EXTENTS` subtraction from geometry
(2026-05-27), Motif hints (2026-07-28), modal windows, iconic-state mapping,
`InputOnly` windows ignored, INCR selection fixes. cosmic-comp, Otto, Denial
and Pinnacle use it. It works but the compositor must implement X11 window
management policy on top (stacking, override-redirect, global coordinates,
focus models).

**xwayland-satellite** ([Supreeeme/xwayland-satellite](https://github.com/Supreeeme/xwayland-satellite)):
- Latest v0.8.3, 2026-09-24 (fix "for new Steam popups"); v0.8.2 2026-07-22;
  v0.8.1 2026-02-17; v0.8 2025-12-01 (clipboard rework fixing a Steam crash at
  launch; client-side decorations for undecorated X11 windows).
- Host compositor needs only core, xdg-shell and viewporter; it optionally
  uses linux-dmabuf, xdg-activation, xdg-decoration, xdg-foreign, pointer
  constraints, relative pointer, primary selection, tablet, fractional scale
  (README, 2026-09-30). Explicit sync passthrough
  (`wp_linux_drm_syncobj_manager_v1`) was added: issue
  [#333](https://github.com/Supreeeme/xwayland-satellite/issues/333) closed
  completed 2026-01-18, and `clientside.rs` binds the syncobj manager. It
  only helps if the host compositor exposes syncobj.
- No tearing-control or xwayland-keyboard-grab passthrough found in `src/`
  (grep, 2026-09-30).
- Compositor integration: pass a display number and `-listenfd`; niri spawns
  it on demand when an X11 client connects and restarts it if it dies (niri
  wiki `Xwayland.md`, since 25.08).
- Maintainer notes an async core rewrite is stalled (v0.8.2 notes, 2026-07-22).
- Known pain is Steam: popups mispositioned
  ([#156](https://github.com/Supreeeme/xwayland-satellite/issues/156), open,
  activity 2026-09-02); context menus closing within 0.1 s on CachyOS + niri
  with satellite 0.8.2/0.8.3, workaround "downgrade to 0.8.1" or Steam's
  "context menu focus compatibility mode"
  ([#491](https://github.com/Supreeeme/xwayland-satellite/issues/491),
  2026-09-04/06). "Very bad performance of xwayland apps with Nvidia GPUs"
  ([#192](https://github.com/Supreeeme/xwayland-satellite/issues/192)) is
  open; a 2026-01-15 commenter says it also happens on Hyprland, Sway, River,
  so it is not satellite-specific.
- niri's FAQ (2026-10-01) gives the reasons for satellite: X11 WM work is
  large, niri "doesn't have a good global coordinate system required by X11",
  and "an endless stream of X11 bugs". YaLTeR expects satellite may become
  "the standard way of integrating Xwayland into new compositors".

---

## 2. Gaming requirements and how niri and cosmic-comp handle them

Versions: niri latest release **v26.04 (2026-04-25)**; previous v25.11
(2025-11-29), v25.08 (2025-08-30). Repo moved to
[niri-wm/niri](https://github.com/niri-wm/niri); 28k stars; master
2026-10-01. cosmic-comp tags **epoch-1.0.0 (2025-12-09)** through
**epoch-1.9.0 (2026-09-22)**; master 2026-10-01.

| Requirement | niri | cosmic-comp | Smithay prerequisite |
|---|---|---|---|
| **VRR / adaptive sync** | Shipped. `variable-refresh-rate` per output since 0.1.5; `on-demand=true` (only while a window matching the `variable-refresh-rate` window rule is shown) since 0.1.9. Debug flag `skip-cursor-only-updates-during-vrr` since 25.08 (cursor motion alone does not redraw; documented caveat that the screen appears frozen if nothing else redraws). Wiki notes: cursor may move at low framerate under VRR on some drivers (fix: `disable-cursor-plane`); some monitors modeset-flash with VRR. | Shipped. `AdaptiveSync` config: `Enabled` (default) turns VRR on only while a fullscreen window is active; `Force` always; `Disabled`. Open [PR #2420](https://github.com/pop-os/cosmic-comp/pull/2420) (activity 2026-07-31) changes it to require focus and lets an exclusive fullscreen surface drive presentation. | In tree |
| **Tearing (async page flip, wp-tearing-control)** | Not shipped. [Issue #844](https://github.com/niri-wm/niri/issues/844) open: waiting on Smithay #1335/#1948; plan is a window rule like on-demand VRR. | Not shipped. [PR #2588](https://github.com/pop-os/cosmic-comp/pull/2588) (composited tearing) closed 2026-07-17; Drakulix: "this feature should be implemented in smithay and is already being worked on". Issue #248. | **Missing** (PRs #1325, #1948 open) |
| **Direct scanout** | Primary-plane scanout always on; overlay-plane scanout behind `enable-overlay-planes` debug flag ("may cause frame drops during some animations"); `disable-direct-scanout`, `restrict-primary-scanout-to-matching-format` flags. | On. Overlay planes are **disabled on any nvidia driver** by a quirk: "Using an overlay plane on a nvidia card breaks the display controller" (`src/backend/kms/mod.rs`, last touched 2026-09-01). Translucent surfaces no longer scanned out (2026-07-14). `COSMIC_DISABLE_DIRECT_SCANOUT` env var. | In tree (`DrmCompositor`); framebuffer-effect elements are never scanned out (2026-03-13) |
| **Explicit sync (linux-drm-syncobj)** | **Not implemented.** [Issue #843](https://github.com/niri-wm/niri/issues/843) open; YaLTeR 2025-07-13: "Someone needs to finish" PR #1449. No `syncobj` in niri `src/` (grep, 2026-10-01). | Shipped since epoch-1.0.0 ([PR #411](https://github.com/pop-os/cosmic-comp/pull/411), merged 2025-01-15) | In tree |
| **fifo-v1 / commit-timing-v1** | Not implemented. YaLTeR 2026-03-05 on a report of 5-30 ms Vulkan present overhead vs 0.06 ms on GNOME: "niri doesn't impl the fifo protocol while Mutter perhaps already does" ([#3562](https://github.com/niri-wm/niri/issues/3562), open). | Not merged. RFC [PR #2797](https://github.com/pop-os/cosmic-comp/pull/2797) open (2026-09-25); WIP #1549. | In tree since 2024-11-22 |
| **HDR / color management** | Not shipped. [Issue #1384](https://github.com/niri-wm/niri/issues/1384) open (2026-09-12). Community branch `dividebysandwich/niri` `hdr-smithay-master` (2026-07-10). niri has `disable-10bit-output` and `max-bpc`. | Not shipped. Open issues: AB30 10-bit modeset failures via Thunderbolt dock (#2193), "AMD HDR bring-up" (#2812). Open [PR #1518](https://github.com/pop-os/cosmic-comp/pull/1518) "Don't use more then 10-bit by default". | **Missing** (PR #1143 draft) |
| **DRM lease** | Implemented (`DrmLeaseHandler` in `src/handlers/mod.rs`) | Implemented (`handlers/drm_lease.rs`) | In tree |
| **Pointer constraints / relative pointer** | Yes | Yes | In tree |
| **Xwayland** | xwayland-satellite, built-in integration since 25.08 | In-tree XWM (`src/xwayland.rs`) | In tree |

### 2.1 NVIDIA proprietary driver specifics

Driver timeline (from release coverage):
- R595: 595.58.03 recommended stable (2026-03); fixed VRR flicker on some
  HDMI displays; `nvidia-drm modeset=1` default; 595.104.02 fixed a crash from
  unvalidated HDR metadata forwarded to the Wayland color-management protocol
  ([ubuntuhandbook 2026-03](https://ubuntuhandbook.org/index.php/2026/03/nvidia-595-58-03-released-with-better-wayland-linux-gaming-support/),
  [linuxcompatible](https://www.linuxcompatible.org/story/nvidia-linux-driver-59510402-fixes-pagetable-and-wayland-hdr-crashes/)).
  A Hyprland discussion reports "MASSIVE Flickering" with fullscreen games on
  595.45.04, fixed by rolling back to 590
  ([hyprwm/Hyprland#13792](https://github.com/hyprwm/Hyprland/discussions/13792)).
- R610: 610.43.02 beta 2026-05-26: FP16 EGL configs on Wayland, YCbCr
  multiplanar modifiers, mmap on exported dmabufs, and **per-plane DRM Color
  Pipeline API** (Linux 6.19+) so compositors can offload color/HDR to the
  display engine; caveat that some compositors mishandle non-bypassable
  colorops and blank on HDR, workaround `nvidia-drm color_pipeline=0`
  ([GamingOnLinux 2026-05-26](https://www.gamingonlinux.com/2026/05/nvidia-driver-610-43-02-arrives-for-linux-with-vulkan-upgrades-drm-colour-pipeline-api-support/)).
  610.57.04 2026-08-03, mostly game fixes
  ([GamingOnLinux 2026-08](https://www.gamingonlinux.com/2026/08/nvidia-stable-driver-595-91-07-and-new-feature-driver-610-57-04-released-for-linux/)).
- R615: 615.71.09 (2026-09, installed on the host): Reflex for Vulkan games
  under Proton via VK_NV_low_latency2, a fix so low_latency2 works on
  `VK_KHR_wayland_surface` swapchains, cgroups memory partitioning
  ([GamingOnLinux 2026-09](https://www.gamingonlinux.com/2026/09/nvidia-driver-615-71-09-released-for-linux-with-vulkan-proton-improvements-with-nvidia-reflex/),
  [linuxiac](https://linuxiac.com/nvidia-615-71-linux-driver-adds-proton-support-for-nvidia-reflex/)).

NVIDIA's own Wayland known-issues page for 615.71.09
([README/wayland-issues](https://download.nvidia.com/XFree86/Linux-x86_64/615.71.09/README/wayland-issues.html)):
simpledrm/nvidia-drm conflict (use `fbdev=1`); no `COLOR_ENCODING`/
`COLOR_RANGE` plane properties; explicit sync needs kernel 6.8+ commits; no
RTD3 API for compositors to power off VRAM; color-pipeline non-bypassable
colorop issue; GLX front-buffer rendering and indirect GLX broken under
Xwayland.

Async flips on nvidia-drm: the open kernel module sets
`dev->mode_config.async_page_flip = true` when the kernel has the
async-flip CRTC state (`kernel-open/nvidia-drm/nvidia-drm-drv.c`, main at
615.71.09). The only Smithay-side report is the RTX 4060 mobile / 610.43.02
test on PR #1948 above.

Workarounds compositors carry for NVIDIA:
- cosmic-comp: overlay planes off on nvidia (above); render-start timing
  forced to zero because "Nvidia returns `page_flip`/`commit` early"
  (`src/backend/kms/surface/timings.rs`, vendor 0x10de).
- niri: `emulate-zero-presentation-time` debug flag exists because zero
  presentation time "is a thing on NVIDIA proprietary drivers". niri wiki
  `Nvidia.md`: niri "should use on the order of 100 MiB of VRAM"; near 1 GiB
  means the driver's heap is not returning freed buffers; fix is an
  application profile setting `GLVidHeapReuseRatio` to 0 for the compositor's
  process name (from
  [NVIDIA/egl-wayland#126](https://github.com/NVIDIA/egl-wayland/issues/126)).
  **A new compositor needs the same profile under its own process name.**
  The NVIDIA screencast flicker flag was removed in 25.08 after a real fix.
- Smithay: `IN_FENCE_FD` skipped on nvidia < 560.35.03.

Reported NVIDIA gaming problems on Smithay compositors in 2026:
- niri + gamescope on GTX 750 Ti / 580.119.02: about 150 ms keyboard latency
  in Steam games and gamescope failing with "error importing dmabuf:
  Error::DeviceMissing" ([niri discussion #3176](https://github.com/YaLTeR/niri/discussions/3176),
  2026-01). Old Maxwell card; weak evidence for Blackwell.
- niri + nested gamescope + RTX 4060 (2026-09-25): gamescope guessed 60 Hz
  from the host and needed `--refresh`; niri's
  `wait-for-frame-completion-before-queueing` caused 20-95 ms input stalls
  when the GPU was saturated; `skip-cursor-only-updates-during-vrr` used
  ([notflask/.config PR #21](https://github.com/notflask/.config/pull/21)).
- cosmic-comp: linear CPU and memory growth to OOM after about 24 h on mixed
  NVIDIA+Intel multi-monitor
  ([cosmic-comp#2265](https://github.com/pop-os/cosmic-comp/issues/2265));
  open [PR #2500](https://github.com/pop-os/cosmic-comp/pull/2500) "VRAM and
  memory leaks in window, session, output, and multi-GPU paths" (2026-09-10).
- KDE (not Smithay) on an RTX 5070 Ti with 595.71.05: enabling HDR crashed
  the session with `GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT`
  ([NVIDIA forums](https://forums.developer.nvidia.com/t/rtx-5070-ti-blackwell-kde-plasma-6-6-4-wayland-hdr-causes-gl-framebuffer-incomplete-attachment-and-session-crash/368511)).

### 2.2 gamescope nested

From [ValveSoftware/gamescope](https://github.com/ValveSoftware/gamescope)
`src/Backends/WaylandBackend.cpp` (clone 2026-10-01):
- **Hard requirements on the host** (init fails with "Couldn't create Wayland
  objects" otherwise): `wl_compositor` v4, `wl_subcompositor`, `xdg_wm_base`,
  `zwp_linux_dmabuf_v1` v3+, `wp_viewporter`, `wp_presentation`,
  `zwp_relative_pointer_manager_v1`, `zwp_pointer_constraints_v1`, `wl_shm`.
- Optional: `wp_single_pixel_buffer_manager_v1`, `wp_color_manager_v1` or
  `frog_color_management_factory_v1` (HDR passthrough), fractional scale,
  xdg-toplevel-icon, data device and primary selection.
- `CWaylandBackend::SupportsTearing()` returns `false`: nested gamescope never
  tears. Low-latency nested play depends on the host's VRR and scheduling.
- NVIDIA proprietary needs 515.43.04+ with `nvidia-drm.modeset=1` (README).
- Nested HDR therefore needs a host color-management protocol, which no
  shipped Smithay compositor has.

### 2.3 Steam, Proton and Big Picture

- The Steam client is still X11 via Xwayland. The native Wayland feature
  request [steam-for-linux#4924](https://github.com/ValveSoftware/steam-for-linux/issues/4924)
  is open (last activity 2026-09-17). Blog claims of a native Wayland Steam
  client in 2026 are not backed by Valve; see OPEN.
- So Steam, Big Picture and the overlay go through whichever X11 path the
  compositor offers. On satellite that means the popup bugs above. Big Picture
  is a fullscreen X11 window; it rides the same VRR/fullscreen path as a game.
- Proton 10 (2025-11) did not expose the Wine Wayland driver; Proton 11
  (2026-07) began to. Proton-CachyOS 11.0-20260601 auto-enables HDR; GE-Proton
  11-7 has a large "Wineland" overhaul. `PROTON_ENABLE_HDR=1` does not by
  itself switch to Wine-Wayland
  ([proton-ge-custom#637](https://github.com/GloriousEggroll/proton-ge-custom/issues/637);
  [GamingOnLinux 2026-06](https://www.gamingonlinux.com/2026/06/proton-cachyos-update-brings-automatic-hdr-wayland-improvements-and-more-bug-fixes/)).
  Native-Wayland games want tearing-control, fifo/commit-timing and color
  management from the compositor, all three missing from Smithay's shipped
  compositors today.
- Xwayland 23.2 added tearing-control and 24.1 added explicit sync
  ([Phoronix: 23.2](https://www.phoronix.com/news/XWayland-23.2-Released),
  [24.1](https://www.phoronix.com/forums/forum/linux-graphics-x-org-drivers/wayland-display-server/1464763-xwayland-24-1-released-with-explicit-sync-better-rootful-experience/page2)).
  An X11 game can request tearing only if the X11 path (in-tree XWM or
  satellite) forwards it and the compositor implements it.

---

## 3. Screen capture for xdg-desktop-portal ScreenCast

There are four ways in, and they decide which protocols the compositor owes.

| Portal backend | What the compositor must provide | Status (2026) |
|---|---|---|
| **xdg-desktop-portal-wlr** (xdpw) | `ext-image-copy-capture-v1` + `ext-image-capture-source-v1` (outputs) and `ext-foreign-toplevel-list-v1` (window capture), or legacy `wlr-screencopy-v1`; plus `linux-dmabuf` and `xdg-output`. xdpw owns the PipeWire stream. | ext-image-copy-capture landed 2025-06-06, toplevel tracking/capture 2025-07-31, first released in **v0.8.0 (2025-10-21)**. v0.8.3 (2026-06-18) shipped with "This version will sometimes stall screen recording"; v0.8.4 2026-07-28. Source chooser is an external program (slurp/wofi style). Source: [emersion/xdg-desktop-portal-wlr](https://github.com/emersion/xdg-desktop-portal-wlr) `protocols/meson.build`, git log. Denial uses this route (`docs/SCREEN_CAPTURE.md`). |
| **xdg-desktop-portal-gnome** (niri's route) | No Wayland capture protocol. The compositor emulates Mutter/GNOME Shell D-Bus APIs and produces the PipeWire stream itself. niri implements `org.gnome.Mutter.ScreenCast`, `org.gnome.Mutter.DisplayConfig`, `org.gnome.Mutter.ServiceChannel`, `org.gnome.Shell.Introspect`, `org.gnome.Shell.Screenshot` (`src/dbus/`), behind the `xdp-gnome-screencast` cargo feature (zbus + pipewire-rs 0.10). `niri-portals.conf`: `default=gnome;gtk;`. | Mature in niri: window and monitor casts, block-out-from-screencast rules, dynamic cast target (25.05), cursor as metadata (26.04), cast IPC and `stop-cast` (26.04), PipeWire buffer count 16 to 8 (26.04). GTK window/monitor chooser for free. No RemoteDesktop interface in niri's list. |
| **xdg-desktop-portal-cosmic** | `ext-image-copy-capture` plus COSMIC private protocols (`cosmic-image-capture-source`, `cosmic-toplevel-info`, workspaces). | Tied to COSMIC protocols ([pop-os/cosmic-protocols](https://github.com/pop-os/cosmic-protocols) `unstable/`); not portable without implementing those. |
| **Own portal backend** | Whatever the compositor wants internally; implement `org.freedesktop.impl.portal.ScreenCast` (and Screenshot) as a D-Bus service with your own chooser drawn in the shell. | Otto does this ("an XDG Desktop Portal backend over PipeWire", README 2026-10-02). |

Notes:
- In all routes the compositor renders the captured frames; Smithay's
  ext-image-copy-capture module only handles the protocol. niri renders a
  frame to the DRM buffer, then separately into the PipeWire buffer, and
  Tracy shows the cast render running on the GPU in parallel (niri 26.04
  notes).
- niri added ext-image-copy-capture for outputs and cursors on master
  2026-09-12 (unreleased). Its 26.04 notes list why wlr-screencopy is poor:
  no way to tell screencasts from screenshots or when a cast ended, because
  xdpw keeps one manager alive.
- PipeWire pitfalls niri hit: a PipeWire memory-corruption bug triggered by
  not rewriting unchanged metadata every frame (fixed upstream); NVIDIA
  screencast flicker (fixed in niri by 25.08).

---

## 4. Precedent compositors on Smithay

Activity from GitHub API, 2026-10-02.

| Project | Activity | Shape |
|---|---|---|
| [niri](https://github.com/niri-wm/niri) | v26.04, master 2026-10-01, 28k stars | Scrollable tiling; 84k lines in `src/` + 9k in `niri-config` |
| [cosmic-comp](https://github.com/pop-os/cosmic-comp) | epoch-1.9.0 2026-09-22, master 2026-10-01 | COSMIC DE compositor; 72k lines |
| [Pinnacle](https://github.com/pinnacle-comp/pinnacle) | v0.2.4 2026-06-27; last push 2026-06-27 | AwesomeWM-like, configured in Lua or Rust |
| [Otto](https://github.com/nongio/otto) | push 2026-10-02 | Stacking/tiling with dock, top bar and island drawn by the compositor; Skia; about 330k lines across the repo including bundled apps; repo carries `CLAUDE.md`/`AGENTS.md` |
| [Denial](https://github.com/denialwm/denial) | v0.4.4, push 2026-10-01, public beta, packaged for Arch/CachyOS | Flutter engine embedded in the compositor |
| [DriftWM](https://github.com/malbiruk/driftwm) | push 2026-09-21, 1.75k stars | Trackpad-first infinite canvas |
| xfwl4 (Xfce) | Preview releases 2026-06-21 and 4.21.1 2026-08-10 ([Xfce blog](https://blog.xfce.org/2026/01/), [LWN](https://lwn.net/Articles/1056159/)) | xfwm4 replacement on Smithay |
| MagmaWM, Strata | Last push 2025-05 and 2024-03 | Stale |
| Catacomb | GitHub archived, moved to SourceHut | Mobile |

### 4.1 niri

- **State and loop:** `State { backend: Backend, niri: Niri }` where
  `Backend` is `Tty | Winit | Headless` (`src/niri.rs`, `src/backend/mod.rs`);
  calloop 0.14 with `executor`, `futures-io`, `signals`; D-Bus via zbus 5;
  PipeWire via pipewire-rs.
- **Redraw loop:** per-output `RedrawState` state machine (`Idle`, `Queued`,
  `WaitingForVBlank`, `WaitingForEstimatedVBlank`). Anything that changes the
  screen calls `queue_redraw()`; redraws run at the end of each event-loop
  dispatch. With no damage, niri still arms a timer at the estimated vblank so
  frame callbacks are throttled to once per refresh; without it clients spin
  (wiki `Development:-Redraw-Loop.md`).
- **Render lists:** 26.04 replaced `-> impl Iterator<Item = Element>` with
  push closures (`push: &mut dyn FnMut(Element)`), removing borrow fights with
  `&mut Renderer` and per-window temporary `Vec`s; render-list construction got
  2-3x faster ([niri PR #3113](https://github.com/niri-wm/niri/pull/3113);
  26.04 notes). cosmic-comp then reworked its element collection "with heavy
  inspiration" from it (cosmic-comp PR #2179). Lesson: design render-element
  collection as push-based from the start.
- **In-compositor UI:** `src/ui/` holds the hotkey overlay, screenshot UI, exit
  dialog, config-error notification, screen transition, and the MRU (Alt-Tab)
  switcher. Text is drawn with pango/cairo into images uploaded as textures
  (`pangocairo` in `hotkey_overlay.rs`). Bars, launchers and notifications are
  external layer-shell clients.
- **Config:** KDL via knuffel (`niri-config`), a file watcher thread
  (`src/utils/watcher.rs`) for live reload, includes and `optional=true`
  includes (26.04), `niri msg action load-config-file --path`. Parse errors
  show an in-compositor notification and keep the last good config.
- **Profiling:** Tracy throughout, including GPU zones via Smithay's
  `tracy_gpu_profiling`; YaLTeR names the lack of GPU profiling as a main
  blocker for blur until late 2025.
- **Memory:** wiki says about 100 MiB VRAM on NVIDIA when the heap profile is
  applied. A cmeissl fix for a VRAM leak after closing certain apps landed in
  26.04. An OOM after about 2 days (27 GB peak) is open with no cause found
  ([#1742](https://github.com/niri-wm/niri/issues/1742)). No primary
  idle-RSS figure found (OPEN).

### 4.2 cosmic-comp

- **State:** `State { backend, common, ... }`; `Common` holds config, display
  handle, `LoopHandle<'static, State>`, and `shell: Arc<parking_lot::RwLock<Shell>>`
  (`src/state.rs`).
- **Threads:** each KMS output renders on its own thread
  (`std::thread::Builder` named `surface-<output>`, `src/backend/kms/surface/mod.rs`),
  with a per-thread `GpuManager<GbmGlowBackend<DrmDeviceFd>>`. This is why the
  shell sits behind a lock. It is the main architectural difference from
  niri's single-threaded loop.
- **In-compositor UI:** iced (from libcosmic) rendered with `iced_tiny_skia`
  on the CPU into `MemoryRenderBuffer`s that become render elements
  (`src/utils/iced/mod.rs`), used for stack tabs, resize and swap indicators.
  Panel, launcher, app library and notifications are separate layer-shell
  clients. `smithay-egui` behind a `debug` feature for overlays.
- **Config:** `cosmic-comp-config` crate over cosmic-config; output config via
  wlr and COSMIC output-management protocols.
- **Private protocols:** corner radius, toplevel info/management, workspaces,
  overlap notify, image capture source, session-lock layer (2026-09-11),
  keyboard layout, a11y.

### 4.3 Pinnacle

- Config is a separate process speaking gRPC to the compositor; API defs in
  `pinnacle-api-defs`, clients in Lua and Rust. Widgets come from **Snowcap**,
  a separate Wayland client using smithay-client-toolkit and iced over
  wlr-layer-shell (`snowcap/README.md`). README credits niri for "rendering
  and protocol stuff ... And session stuff. And the test fixture."
- Activity slowed: last push 2026-06-27.

### 4.4 Otto (closest precedent for "shell drawn in the compositor")

From README and `docs/developer/rendering.md` (2026-10-02):
- Dock, top bar, dynamic island, Exposé, app switcher, notifications and
  volume/brightness OSDs are drawn by the compositor.
- "A **retained-mode** compositor wearing an immediate-mode compositor's
  clothes": all UI and windows live in one `lay-rs` scene graph (Taffy
  layout, springs/animations) handed to Smithay as a single `SceneElement`;
  `SkiaRenderer` wraps Smithay's current EGL framebuffer as a Skia surface.
  "Why is this not updating?" is "nearly always a damage question".
- KMS multi-plane scanout of dock, switcher, popups and top windows, with
  cross-plane backdrop blur. **Tested mostly on Intel; untested on NVIDIA**,
  with a README warning to suspect it first on AMD/NVIDIA.
- Uses a Smithay fork (dmabuf scanout, XWM focus, Vulkan device layer).
- Own protocol `otto-surface-style-unstable-v1` exposes the scene graph to
  clients (corner radius, blur, shadow driven by compositor springs).
- Own portal backend, RDP server, virtual outputs as PipeWire nodes.

### 4.5 Denial

From `docs/architecture.md`, `docs/direct_scanout.md`, `docs/KNOWN_ISSUES.md`
(clone 2026-09-21):
- `deniald` runs the Flutter engine and Dart isolate in-process: Rust owns
  Wayland resources, buffers, fences, KMS and input; Dart owns layout,
  launcher, overview, notifications, animation and hit regions, and never
  holds an fd, EGL image or fence. Dart publishes an immutable input layout
  that Rust swaps atomically.
- Flutter renders one desktop-wide XRGB8888 GBM atlas; each CRTC scans out a
  crop. Impeller GLES by default; a locked Flutter engine fork. Atomic KMS goes
  through its own `volition` module, not Smithay's `DrmCompositor`.
- **Direct scanout is designed but not implemented**; the design is primary
  plane = game dmabuf, overlay plane = transparent shell, cursor plane, never
  modesetting on promotion, UI frames piggybacking on game commits under VRR.
- Known issues: a GPU reset (AMD ring timeout from a Unity game under Proton)
  kills the session because the EGL context is not reset-aware; Impeller blur
  corrupts with undecorated X11 windows, so client decoration opt-outs are
  ignored; live scale change can end the session.
- Settings runs as a separate Flutter Wayland process "so it cannot add build,
  layout, paint, or raster work to the compositor's desktop frame".

### 4.6 Lessons these projects state themselves

- Rendering list construction and damage are where the work goes (niri PR
  #3113; Otto "nearly always a damage question").
- Background effects force invasive changes to Smithay's damage tracking and
  element APIs (cosmic PR #2179: "much more complex than anticipated, due to
  necessary and quite invasive changes to smithay's public api").
- NVIDIA needs explicit workarounds in timing, planes, VRAM and presentation
  feedback (section 2.1).
- X11 is where bug volume comes from (niri FAQ; Denial X11 blur issue;
  satellite Steam issues).
- A GPU reset kills a compositor with a non-reset-aware EGL context (Denial).
- Xfce's lesson from xfwm4: retrofitting Wayland into an X11 WM failed; they
  restarted on Smithay ([Xfce blog 2026-01](https://blog.xfce.org/2026/01/)).

---

## 5. Blur, rounded corners and shaders

### Smithay support (master, unreleased)
- `Element::is_framebuffer_effect` and `RenderElement::capture_framebuffer`
  let an element read what is already drawn beneath it and modify it; the
  damage tracker tells the element when the framebuffer under it is dirty so
  it can re-capture only then, and damages the whole element when capture is
  allowed ([PR #1910](https://github.com/Smithay/smithay/pull/1910), Drakulix,
  opened 2026-01-22, merged 2026-03-13; follow-ups "small blur fixes" #2121 2026-09-02).
  Framebuffer effects are never direct-scanned out (2026-03-13). Default
  implementations skip the feature, so custom `RenderElement` wrappers not
  built with `render_elements!` must be updated.
- `GlesRenderer::compile_custom_pixel_shader`,
  `compile_custom_texture_shader`, `render_pixel_shader_to`,
  `render_texture_from_to` (`src/backend/renderer/gles/mod.rs`) are the hooks
  for custom GLES shaders.
- `ext-background-effect-v1` in tree (2026-02-04).

### niri (shipped in v26.04, 2026-04-25)
- Blur, xray, saturation and noise via `background-effect {}` in window and
  layer rules, and via `ext-background-effect` from clients (foot 1.26, kitty
  0.46.2, DankMaterialShell, Noctalia named in the notes).
- Dual-Kawase-style down/up passes (`shaders/blur_down.frag`,
  `blur_up.frag`; `BlurOptions { passes, offset }`; notes cite the
  [frost.kiwi Dual Kawase post](https://blog.frost.kiwi/dual-kawase/)).
- **Xray is the default** because the blurred wallpaper is computed once and
  reused as a static image; non-xray reads back the just-rendered pixels
  mid-frame, blurs, and continues, and is "experimental" (disappears during
  open/close animations and while dragging a tiled window). Both needed
  separate code paths through overview and screencast block-out.
- YaLTeR: "one of the most complex niri features thus far"; with Tracy, blur
  "turned out to run faster than I thought". No numbers published.
- Rounded corners: `geometry-corner-radius` plus `clip-to-geometry`, via
  `clipped_surface.frag` and `rounding_alpha.frag`; also `border.frag`,
  `shadow.frag`, resize/open/close shaders, and user `custom-shader` GLSL for
  animations (wiki `Configuration:-Animations.md`). 26.04 trimmed shaders to
  fit an Eee PC's GPU. Fix for blur on GLES 2.0 GPUs 2026-04-29.
- Blur was "the most requested niri feature" and lived in forks before.

### cosmic-comp (shipped in epoch-1.3.0, 2026-07-14)
- "Frosted Glass (Blur)" [PR #2179](https://github.com/pop-os/cosmic-comp/pull/2179),
  merged 2026-07-09, implements `ext-background-effect-v1`; shaders
  `blur_downsample.frag`, `blur_upsample.frag`, `clipped_surface.frag`,
  `rounded_rectangle.frag`, `rounded_outline.frag`, `shadow.frag`. Element
  generation split into "lower" and "top" surfaces so drop shadows sit
  between tiled windows without affecting blur.
- After merge, a client author reported cosmic and Mutter differ on whether
  frame extents are blurred when no region is given (2026-09-11/15).

### Others
- Otto: Skia scene graph with cross-plane backdrop blur (README).
- Denial: Impeller blur; known corruption with undecorated X11 windows.
- Performance: cosmic caches the blur and re-captures only when the
  framebuffer below is dirty (PR #1910 description); niri prefers xray for
  cost. No published frame-time numbers for either (OPEN).

---

## 6. What this means for the design (facts restated as constraints)

1. Smithay master is the platform; pin a git rev and budget for churn
   (`delegate_dispatch2!`, wayland-rs API changes pending).
2. Tearing and HDR are not available off the shelf on any Smithay compositor
   as of 2026-10-02. Tearing needs Smithay PR #1948 (DRM async flip) plus
   #1325 (protocol) or an equivalent of your own. HDR needs color management
   plus NVIDIA's color-pipeline path, with a known RTX 5070 Ti HDR crash on
   KDE and only a 300-nit panel here.
3. VRR, direct scanout (primary plane), explicit sync, DRM lease, pointer
   constraints, relative pointer, presentation-time, fractional scale,
   session lock, layer shell, ext-image-copy-capture, fifo/commit-timing are
   all in tree. Explicit sync matters on NVIDIA and niri lacks it, so niri is
   not the reference for that piece; cosmic-comp is.
4. Xwayland: satellite gives the least compositor code but has open Steam
   popup bugs and passes through no tearing; the in-tree XWM is improving fast
   and is what cosmic-comp, Otto, Denial, Pinnacle use.
5. Screencast: either emulate Mutter's D-Bus APIs for xdp-gnome (niri, gets a
   chooser for free), implement ext-image-copy-capture for xdpw (needs an
   external chooser), or write a portal backend with the chooser in your own
   shell (Otto).
6. In-compositor shell UI precedents: niri pango/cairo textures; cosmic iced
   on tiny-skia (CPU) into memory buffers; Otto Skia on the GL framebuffer
   with a retained scene graph; Denial an embedded Flutter engine.
7. On NVIDIA: apply the `GLVidHeapReuseRatio` profile under the new process
   name; consider cosmic's overlay-plane quirk; do not rely on DRM
   presentation timestamps or early commit returns for latency scheduling.

---

## OPEN

Not verified; do not treat as fact.

- **Idle RSS of niri or cosmic-comp.** No primary measurement found. A
  third-party blog claims 40-60 MB for niri at idle
  ([petronellatech, 2026](https://petronellatech.com/blog/niri-scrollable-tiling-wayland-compositor-guide-2026/));
  unverified. The only primary figure is niri's ~100 MiB VRAM on NVIDIA
  (wiki).
- **Atomic async flips on Blackwell with Smithay.** Only an RTX 4060 mobile on
  610.43.02 is reported working (PR #1948). Not confirmed for RTX 50 or
  615.71.09.
- **Whether cosmic's "overlay planes break the nvidia display controller"
  quirk still applies on 615.x / Blackwell.** Code has it; no test result
  found.
- **Whether NVIDIA's Vulkan Wayland WSI uses wp-fifo-v1 / commit-timing-v1,**
  and whether implementing them fixes niri issue #3562's present overhead on
  NVIDIA.
- **HDR on RTX 5070 Ti through the DRM color pipeline in any Smithay
  compositor.** No report found; the community `ds-hdr-master` fork claims
  NVIDIA HDR working on two monitors (comment of 2026-08-21 on PR #1143) but
  the GPU model is not stated.
- **Direct scanout of fullscreen game buffers on NVIDIA via Smithay's
  `DrmCompositor`.** niri keeps primary-plane scanout on for all drivers and
  no NVIDIA exclusion was found, but no measurement or report confirms it
  works on Blackwell.
- **Native Wayland Steam client.** Blog posts (e.g.
  [newinlinux](https://www.newinlinux.com/the-dream-of-steam-on-wayland-just-got-closer-to-reality/))
  suggest progress; Valve's tracker issue #4924 is open. Treat Steam as X11.
- **Steam Big Picture specifics on niri/cosmic** beyond the popup/menu bugs:
  no dedicated issue found for BPM itself on current versions (satellite #18,
  "Steam big picture mode doesn't work", closed 2024-08-20 as no longer
  reproducible).
- **gamescope nested on NVIDIA under niri/cosmic in 2026:** mixed anecdotal
  reports (works with flags on RTX 4060; dmabuf import failure on GTX 750 Ti).
  No maintainer root cause.
- **Blur performance numbers** (frame time, VRAM) for niri or cosmic-comp:
  none published.
- **Pinnacle's support for VRR, explicit sync, screencast** in v0.2.4: not
  checked in source.
- **xfwl4 architecture details** (event loop, rendering) beyond the Xfce blog
  summary: not checked.
- **NVIDIA 610.57.04 and 615.71.09 full Wayland changelogs:** only press
  summaries were readable (Phoronix returned 403); the NVIDIA_Changelog file
  in the driver package is the primary source.
