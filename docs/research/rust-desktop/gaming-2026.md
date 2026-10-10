# Gaming for Isle: research note (2026-10-10)

Scope: what P12 needs underneath it: async page flips and tearing, VRR on
demand, direct scanout, the presentation protocols, night light, Steam and
its Big Picture, nested gamescope and GameMode. It adds to
`smithay-and-gaming.md` (2026-10-02) and does not repeat it. "Local:" marks
code read in shallow clones made today: Smithay `cf43a02a` (2026-10-08),
niri `2c82b77a` (2026-10-08), cosmic-comp `4d14528` (2026-10-08), KWin
`c55a16f` (2026-10-09), Hyprland, aquamarine and hyprsunset (2026-10-09),
gamescope (2026-10-09), NVIDIA open-gpu-kernel-modules `main` (newest tag
615.78.08) and mainline Linux. PR and issue states were read with `gh` on
2026-10-10. *(unverified)* marks what no primary source settled.

---

## §1 Smithay upstream

**Async page flips and tearing are not merged.** Smithay has no
`PageFlipFlags::ASYNC` (only `legacy.rs` uses `PageFlipFlags`, and only
`EVENT`) and no `wayland/tearing_control`. Open:
[#1948](https://github.com/Smithay/smithay/pull/1948) "Initial support for
DRM async page flip v2" (cmeissl, last active 2026-08-28),
[#1335](https://github.com/Smithay/smithay/pull/1335) (2026-02-28) and the
protocol side, [#1325](https://github.com/Smithay/smithay/pull/1325)
(2026-07-05). #2099, #2046 and #2047 were closed. #1948's shape: the
compositor passes a presentation mode with each frame, overlay planes are
switched off for async flips, and when to tear stays the compositor's
policy; it warns that some Intel formats cannot flip asynchronously. Reports
on it: AMD works (2026-03, 2026-04); NVIDIA Turing on nvidia-open works in
niri and anvil (2026-04-05) but logs `[PROP:IN_FENCE_FD] No prop can be
changed during async flip`; an RTX 4060 mobile on 610.43.02 and kernel 6.18
"async flips engage as needed" (2026-07-02). No Blackwell report.

**VRR is in tree.** Local (`backend/drm/surface/{mod,atomic}.rs`):
`vrr_supported(conn)` answers `NotSupported`, `RequiresModeset` or
`Supported` from the CRTC's `VRR_ENABLED` and the connector's `vrr_capable`,
and calls every HDMI connector `RequiresModeset` for amdgpu's HDMI flicker
(drm/amd#2200); DisplayPort is `Supported`. `use_vrr(bool)` tests a commit
without `ALLOW_MODESET` first and, if refused, with it, in which case VRR
changes only at the next modeset. `DrmCompositor` reaches both, and Isle's
pin has them (the survey: `compositor/mod.rs:2730-2746`).

**Direct scanout is in tree.** `FrameFlags` has
`ALLOW_PRIMARY_PLANE_SCANOUT` (the swapchain's format),
`ALLOW_PRIMARY_PLANE_SCANOUT_ANY`, `ALLOW_OVERLAY_PLANE_SCANOUT`,
`ALLOW_CURSOR_PLANE_SCANOUT` and `SKIP_CURSOR_ONLY_UPDATES` (an empty frame
when only the cursor plane would change); the default is every scanout
flag. `select_dmabuf_feedback` (element/utils/wayland.rs) gives a surface
the scanout feedback when its element was put on a plane or was a candidate
that failed the test; the compositor builds the tranches. niri's
`surface_dmabuf_feedback` (tty.rs ~L2924) builds a primary-plane tranche and
a primary-and-overlay tranche from `plane_info().formats`. `DrmCompositor`
leaves `IN_FENCE_FD` out on nvidia-drm before 560.35.03.

**Explicit sync, fifo and commit-timing are in tree**: linux-drm-syncobj-v1
since 2024-09 (#1356), wp-fifo-v1 and wp-commit-timing-v1 since 2024-11.
Also there and unused by Isle: pointer constraints, relative pointer,
content-type, DRM lease, the DRM event's hardware time and sequence
(`DrmEventMetadata`). New since the earlier note: `zwlr_virtual_pointer_v1`
(2026-10-07).

**No gamma or CTM helper.** Compositors write the CRTC's properties
themselves (§4).

## §2 Tearing, VRR and scanout elsewhere

**The kernel's rules for atomic async flips.** Local (mainline
`drm_atomic_uapi.c`): only `FB_ID`, `IN_FENCE_FD` and `FB_DAMAGE_CLIPS` may
change in an async commit, every other property written at its current
value; a plane other than the primary needs the driver's
`atomic_async_check`, which nvidia-drm lacks, so on NVIDIA only the primary
plane can flip asynchronously. A `TEST_ONLY` commit may carry ASYNC; a flip
event may not be asked for in a test. `IN_FENCE_FD` in async commits came
with a 2024 patch
([LKML](https://lkml.iu.edu/hypermail/linux/kernel/2406.2/09657.html)),
about 6.11 *(unverified)*; the host runs 7.2.

**KWin tears; niri and cosmic-comp do not.** niri
[#844](https://github.com/niri-wm/niri/issues/844) is open (2026-08-10);
cosmic-comp has nothing. KWin's atomic tearing
([MR 4800](https://invent.kde.org/plasma/kwin/-/merge_requests/4800), Plasma
6.2, kernel 6.8+) detects `DRM_CAP_ATOMIC_ASYNC_PAGE_FLIP`, sets
`DRM_MODE_PAGE_FLIP_ASYNC` for its `Async` and `AdaptiveAsync` modes (tearing
and VRR together), tears only for an active full-screen window that asks
through wp-tearing-control or a rule (`compositor.cpp` ~L848), turns the
hardware cursor off while tearing, and falls back to vsync for a frame that
changes colour properties.

**VRR on demand.**
- niri (`refresh_on_demand_vrr`, niri.rs ~L4870): VRR is on while a window
  matching the `variable-refresh-rate` rule is shown on that output,
  full screen or not; each change calls `use_vrr`. A debug flag skips
  cursor-only updates, and its wiki warns the screen "will appear to
  completely freeze" if nothing else draws; some monitors "continuously
  modeset (flash black)" with VRR on.
- cosmic-comp (`kms/surface/mod.rs` ~L1026-1090): on exactly while the
  active workspace has a full-screen surface, `use_vrr` called before every
  `render_frame`; cursor-only updates skipped only while the full-screen
  surface draws at 30 fps or better and the panel's minimum interval has not
  passed, so the pointer still moves at the panel's lowest rate; overlay
  scanout off while anything is full screen or animating.
  [PR #2420](https://github.com/pop-os/cosmic-comp/pull/2420) (open) adds
  focus to the rule.
- KWin, the best answer for the pointer: VRR on while the active window
  covers the output and wants it (full screen by default), written in
  ordinary commits rather than modesets (`drm_pipeline.cpp` ~L219);
  cursor-only updates under VRR are held up to 1/(minimum VRR rate + 2 Hz),
  and at least 30 Hz, waiting to ride with the game's next frame
  (`compositor.cpp` ~L699, `drm_commit_thread.cpp` ~L99).

**Does a VRR change blank the screen?** On DisplayPort the kernel and
Smithay need no modeset. nvidia-drm passes `vrr_enabled` into every head
commit; whether NVKMS retrains the link is not visible in the code
*(unverified)*. [NVIDIA forum
314590](https://forums.developer.nvidia.com/t/screen-goes-black-for-a-few-seconds-when-alt-tabbing-out-of-games-with-vrr-at-240hz-on-wayland/314590)
reports seconds of black on Alt+Tab with VRR at 240 Hz under KDE and GNOME,
still on 610.43.02 (2026-06-21), worked round by VRR always on; likely HDMI
or high-rate, and the host's AOC is DisplayPort at 144 Hz.

**Overlay planes on NVIDIA stay off everywhere.** cosmic-comp clears
`planes.overlay` for any NVIDIA driver ("breaks the display controller",
`kms/mod.rs` ~L1070) and keeps primary scanout; KWin's
`recommendsOverlayUse()` is false for NVIDIA and true only on i915 and xe;
niri keeps overlays behind a debug flag. No report confirms or refutes
primary-plane scanout of a game's dmabuf on Blackwell.

## §3 NVIDIA

- **Async flips:** nvidia-drm sets `mode_config.async_page_flip` from
  535.54.03 to `main` and maps an async atomic flip to `minPresentInterval
  = 0; tearing = NV_TRUE` (`nvidia-drm-crtc.c` ~L1785). The kernel's
  `DRM_CAP_ATOMIC_ASYNC_PAGE_FLIP` follows, so any recent driver on 6.8+
  flips asynchronously on the primary plane. No Blackwell-specific report
  either way.
- **VRR:** `vrr_capable` attached from `vrrSupported`; `VRR_ENABLED`
  honoured.
- **`GAMMA_LUT` and CTM:** both since 545.29.02
  ([ubuntuhandbook](https://ubuntuhandbook.org/index.php/2023/11/nvidia-545-29-02-night-light-wayland/)).
  The CTM is applied per plane before blending (`ctm_to_csc()` writes it
  into each layer's CSC, `nvidia-drm-crtc.c` ~L1494); `GAMMA_LUT` is the
  head's output LUT, after blending. KWin therefore never uses the CTM as a
  post-blend step on NVIDIA (`drm_crtc.cpp` ~L67), and Hyprland turns its CTM
  fade off before 575 for display freezes (`CTMControl.cpp`).
- **Explicit sync** needs kernel 6.8+
  ([615.71.09 known issues](https://download.nvidia.com/XFree86/Linux-x86_64/615.71.09/README/wayland-issues.html));
  that page lists nothing on VRR, async flips, gamma or overlays.
- **Vulkan WSI:** 580.65.06 (2025-08-04) added fifo-v1
  ([GamingOnLinux](https://www.gamingonlinux.com/2025/08/nvidia-beta-driver-580-65-06-released-for-linux-with-bug-fixes-and-expanded-wayland-support/));
  595.45 (2026-03-05) added `VK_EXT_present_timing`
  ([linuxiac](https://linuxiac.com/nvidia-595-45-04-beta-linux-driver-released/)).
  Whether it uses commit-timing-v1 is in no changelog *(unverified)*. Mesa's
  WSI uses both. Without fifo-v1 a compositor leaves NVIDIA's native Vulkan
  clients on frame callbacks, a plausible cause of niri
  [#3562](https://github.com/niri-wm/niri/issues/3562) (5-30 ms of present
  overhead against 0.06 ms on GNOME).

## §4 Night light

- **niri:** `GammaProps` (tty.rs ~L2640-2850) writes `GAMMA_LUT` with
  `set_property` on the CRTC outside `DrmCompositor`'s commits, else the
  legacy `set_gamma`, else (since 2026-10-01, for Asahi) a CTM
  approximation, noting that the CTM "applies per-plane pre-blending on some
  drivers". It keeps the blob it found and restores it on leaving the VT.
  It is driven by wlr-gamma-control.
- **cosmic-comp:** no gamma. Its screen filters (inversion, colour-blindness)
  are a shader pass over the output (`render/mod.rs`).
- **KWin:** night light moves the output's white point
  (`applyNightLight`, `drm_output.cpp` ~L410); `tryKmsColorOffloading`
  fits it into `GAMMA_LUT`, a genuinely post-blend CTM, or the colour
  pipeline, and otherwise renders through a shadow buffer (KMS first,
  shader second since
  [MR 4710](https://invent.kde.org/plasma/kwin/-/merge_requests/4710),
  2023-12). Temperature to RGB: Ingo Thies's 2013 blackbody table in 100 K
  steps (from Redshift's colour ramp), interpolated, in linear light,
  identity at 6500 K and above (`src/3rdparty/colortemperature.h`).
- **Hyprland:** hyprsunset sends a 3×3 matrix built with
  [Tanner Helland's formulas](https://tannerhelland.com/2012/09/18/convert-temperature-rgb-algorithm-code.html)
  over `hyprland-ctm-control-v1`, which aquamarine writes to the CTM.
- **What each costs a game:** a shader pass composites every frame, so no
  direct scanout; `GAMMA_LUT` keeps scanout and tints the hardware cursor
  too. A gamma change cannot ride an async commit, so a fade needs vsync
  frames.

## §5 Steam

- Big Picture's window is class `steam`, title "Steam Big Picture Mode";
  games are `steam_app_<id>` (community window rules,
  [r/hyprland](https://lr.eu.psf.lt/r/hyprland/comments/1jzly11/conditional_windowrules)).
  Whether it sets `_NET_WM_STATE_FULLSCREEN` or only sizes itself to the
  screen, and whether it still minimises rather than closes, have no
  primary source *(unverified)*: the host shows.
- Toasts are windows titled `notificationtoasts_<n>_desktop`, class `steam`
  (niri wiki, Application-Issues). Some systems get black Steam windows,
  fixed by turning off GPU web views or `-system-composer`.
- The client is X11 and CEF; Valve has announced no Wayland client
  ([#4924](https://github.com/ValveSoftware/steam-for-linux/issues/4924)
  open).
- Context menus are override-redirect popups. Under xwayland-satellite they
  were mispositioned ([#156](https://github.com/Supreeeme/xwayland-satellite/issues/156),
  open) and closed within 0.1 s on CachyOS with niri (#491, niri #4532,
  closed 2026-09). Isle's own XWM already leaves them unmanaged (P4).
- `steam -shutdown` exits the client (Valve's command-line options page).
  `-bigpicture`, `-gamepadui` and `steam://open/bigpicture` exist;
  `steam://close/bigpicture` is undocumented.
- Open Big Picture bugs: [mutter#3126](https://gitlab.gnome.org/GNOME/mutter/-/issues/3126)
  (glitches on leaving, 2023, open); Xwayland's black screen on opening it
  ([xserver#1679](https://gitlab.freedesktop.org/xorg/xserver/-/issues/1679))
  was closed in 2024. Nothing from niri or cosmic in 2026.

## §6 Nested gamescope

Local (`WaylandBackend.cpp`, `vblankmanager.cpp`): the nested refresh starts
at `-r`/`--nested-refresh`, else 60 Hz. It binds wp_presentation version 1:
a `presented` event with a non-zero refresh sets the rate and marks the host
not VRR; a zero refresh means a VRR host, and gamescope takes the highest
`wl_output` mode among the outputs its surface entered. `-r` overrides all
of it. Smithay sends 0 to a version 1 client for `Refresh::Variable` and the
minimum interval to version 2. So under VRR Isle must advertise the real
current mode on `wl_output` (144000 mHz), and without presentation feedback
gamescope stays at 60 Hz, which fits the "guessed 60 Hz" reports. Nested
gamescope never tears. niri's wiki needs `--backend sdl` for a locked
pointer (gamescope#1711). Smithay's `DeviceMissing` comes from the
multi-GPU import when no device matches; on the hybrid host gamescope should
allocate on the NVIDIA node (`--prefer-vk-device`).

## §7 GameMode

`com.feralinteractive.GameMode` at `/com/feralinteractive/GameMode`
(`daemon/gamemode-dbus.c`, master 2026-06-15; release 1.8.2 of 2024-08):
`ListGames() → a(io)`, `GameRegistered(io)`, `GameUnregistered(io)`,
`ClientCount`, each game an object with `ProcessId`, `Executable`,
`Requester`, `Timestamp`. The Flatpak portal maps sandbox PIDs to host PIDs
before the daemon sees them, so `ListGames` gives host PIDs. Steam and
Proton register only under `gamemoderun %command%`, and CachyOS recommends
its own `game-performance` wrapper instead
([CachyOS wiki](https://wiki.cachyos.org/configuration/gaming/)): GameMode
cannot be the only signal, as P5 already assumed.

## §8 What is still open

- Async flips on Blackwell with 615.x, and with `IN_FENCE_FD` in the commit.
- Primary-plane scanout of a game's dmabuf on NVIDIA through `DrmCompositor`.
- Whether NVKMS blanks the AOC when `VRR_ENABLED` changes over DisplayPort.
- Whether NVIDIA's WSI uses commit-timing-v1.
- Big Picture's full-screen hints and what it does on close.
- Whether cosmic-comp's overlay-plane quirk still holds on 615.x (Isle keeps
  overlays off regardless).
