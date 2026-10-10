# P12 · Gaming

Planned 2026-10-10. Reviewed by the user the same day: every
recommendation taken; one packet; the host run part of done.

The roadmap's brief: the async page flip patch on the Smithay fork and
tearing control tested on the RTX 5070 Ti (B6); VRR on while a full-screen
app is in front, with its setting (B30), and direct scanout for games; game
detection, Game Mode as the automatic Gaming Focus and the escape set of
chords over a game (B30); Steam, its Big Picture as a full-screen game
(B22), and nested gamescope. Night light, the colour transform on each
output, with Control Centre's Night Light module and Game Mode turning it
off (given here at P8's review).

Earlier packets built much of the policy. P4 gave full-screen desktops and
the borderless check by geometry; P5 the game flag on the window record
(B46), the escape set and translation off over a game; P8 Game Mode as the
Gaming Focus with the profile hold and held banners (B64); P9 idle held by a
game in front; P10 the libraries and launching through launchers (B56, B77);
P11 the escape set's capture chords. What none of them touched is the path
from a game's buffer to the screen. Today every frame is presented at the
mode's fixed rate; the host lit the AOC at 60 Hz because its preferred mode
says so; the `vrr` setting is read by nothing; no frame tears; nobody can
tell whether a game is scanned out; presentation feedback carries the time
Isle handled the vblank, not when it happened; linux-drm-syncobj, fifo,
commit-timing, pointer constraints and relative pointer are not served, so
Xwayland has no explicit sync, NVIDIA's Vulkan clients fall back to frame
callbacks and a mouse-look game gets accelerated absolute motion. Night
light does not exist, and Control Centre's module is marked not yet.

What P12 must get right is that a game in front reaches the screen as it
would with no desktop at all: its own buffer on the plane, the refresh
following its frames, tearing only when the user allowed it and the game
asked, and keys and the pointer as pressed. Around that, Isle still knows
the game for what it is, Steam behaves like an app, and nothing Isle does
for games blanks the screen or stalls the loop.

## What it answers to

- Decisions: B6 (the fork carries async flips and tearing), B9 (nothing
  blocking on the loop), B10, B17, B20 (a game is an app), B21, B22 (Big
  Picture is a full-screen game), B23 (HDR later), B30 (Game Mode, the
  escape set, VRR while a full-screen app is in front), B35, B40 (the
  registry), B41 (full-screen desktops), B43 (the fork's patches stay in the
  fork), B46 (the game flag), B56, B64 (Game Mode a Focus), B69 (idle
  holds), B77, B80 (captures are the compositor's render).
- Intent: `gaming.md` Games 1–7, Full-screen Spaces 1–5, Game Mode 3, 5 and
  8, Presentation 1–9, Steam's desktop client 1–5, Steam's Big Picture 1–6,
  gamescope 1–3, Heroic and Lutris 1, its open points (bar the
  controllers', P18's) and acceptance bars 1–12; `compositor.md` Outputs 1
  and 8, Gaming display path 1–8, the wire protocols' rows for
  presentation, fifo, commit-timing, tearing-control, content-type, syncobj,
  pointer constraints and relative pointer; `shell.md`'s Control Centre
  Night Light module.
- Design, binding: `p1/structure/focus.md` (Game Mode's holds, "Off during
  Game Mode"), `control-centre.md` (the Night Light module, Displays owning
  its schedule and warmth, VRR and tearing per output), `settings.md`
  (Displays' rows, P13's to draw); `p1/surfaces/control-centre.md` (the
  module's glyph and lines), `focus.md`; `copy.md` (feature names).
- Evidence: `gaming-2026.md` (new; § numbers below are its sections),
  `smithay-and-gaming.md` §0–§3, `isle-v1-inventory.md` §3.1, §3.3, §3.8,
  §10, v1's `services/NightLight.qml` and `theme/sun.py`.

## Choices this plan makes

### The path to the screen

1. **Direct scanout is made provable, then left to Smithay.**
   `DrmCompositor` already offers a full-screen window's buffer to the
   primary plane (the default `FrameFlags`, and P4 draws a full-screen
   window bare). P12 adds what makes it happen and shows it: per-surface
   dmabuf feedback with a scanout tranche built from the primary plane's
   formats, as niri's `surface_dmabuf_feedback` does (§1), sent through
   `select_dmabuf_feedback` after each frame so a game reallocates into a
   format the plane takes; and a plane trace, `isle displays trace
   <output>`, printing for each presented frame what the primary and
   cursor planes carried (a window by id, or "composited"), whether the
   flip was async, whether VRR was on, and the interval since the last.
   Overlay planes stay off on NVIDIA (`compositor.md` Gaming display path
   6), as every other compositor keeps them (§2).
2. **Presentation feedback tells the truth.** The vblank's hardware
   timestamp and sequence from `DrmEventMetadata` replace the time Isle got
   round to it; the refresh is `Refresh::Variable` while VRR is on and the
   mode's interval otherwise; flags carry hardware clock, hardware
   completion, vsync (not on an async flip) and zero-copy for a scanned-out
   surface. Feedback is given to layer and X11 surfaces too, not only
   mapped windows. Nested gamescope reads its rate from this (§6), and
   acceptance bar 3 is measured with it.
3. **The game's protocols are served.** linux-drm-syncobj-v1 on the render
   device when it supports syncobj eventfds (Xwayland binds it, bar 12);
   wp-fifo-v1 and wp-commit-timing-v1 (NVIDIA's Vulkan WSI uses fifo since
   580, §3); wp-content-type-v1; pointer constraints and relative pointer
   (choice 9). A buffer whose dmabuf is not yet ready holds its commit
   back through a dmabuf blocker rather than being sampled or scanned out
   early, so the loop never waits on a game's GPU work and a game never
   waits on Isle's (Presentation 8).
4. **A frame for a full-screen game is queued at once.** The redraw already
   happens at the end of the dispatch that brought the commit, with no
   render-start delay (NVIDIA returns flips early, `compositor.md` Gaming
   display path 5); P12 keeps it so and adds nothing that waits. The
   shell's surfaces draw only on damage, and over a full-screen game
   nothing of theirs is shown (bar 4 is measured, not built).

### VRR

5. **VRR follows the output's setting, live.** `output.vrr` (Off, Full
   screen, Always; Full screen the default, B30) is applied when it
   changes, not only at connect. Full screen means: the output shows a
   full-screen desktop whose window is the front window (focused, game or
   not, B30). The decision is a pure function of the output's setting, its
   capability and the front window, called before every frame, and
   `use_vrr` is asked only when the answer changes. HDMI, which Smithay
   reports as needing a modeset (§1), gets VRR only at Always and only at
   the next modeset, never on demand (Presentation 9: no modeset on
   entering or leaving a game). `isle displays` shows each output's VRR
   capability, setting and state.
6. **Under VRR the pointer rides with the game's frames.** A frame that
   would change only the cursor plane is held while VRR is on, up to the
   panel's minimum interval (from the EDID's range descriptor, else 33 ms,
   KWin's floor, §2), then presented on its own; a game frame arriving
   first takes the pointer with it. So moving the mouse does not drag the
   refresh rate up and a game at 50 fps does not stutter, and the pointer
   still moves when the game stalls. cosmic-comp's 30 fps condition is not
   copied: the hold is bounded on its own.
7. **The default mode is the preferred size at its highest rate.** The
   preferred mode of the AOC is 60 Hz (P2's host run). An output with no
   `output.mode` set takes the preferred mode's resolution at the highest
   refresh offered at it (144 Hz here), as macOS takes a ProMotion
   display's highest rate. A set mode wins. Confirm-or-revert for mode
   changes stays P13's.

### Tearing

8. **Async flips come into the fork from #1948, and tear only when all of
   it holds.** alexparlett/smithay gains Smithay #1948 (async page flip
   v2) rebased onto `isle`, and a wp-tearing-control-v1 module (from #1325,
   or written small: it is a hint per surface), as pull requests merged
   and pinned as #1–#3 were (B43). Isle's policy, one pure function: a
   frame flips asynchronously when the output's new `output.tearing`
   setting is on, VRR or not; the front window is full screen on that
   output; its surface asks for async presentation (wp-tearing-control,
   which Xwayland forwards for an X11 game presenting with
   `PresentOptionAsync`, §2); the frame changes nothing but the primary
   plane's buffer (the kernel's rule, §2; NVIDIA has no async on other
   planes); and an async `TEST_ONLY` commit passes. A frame that fails any
   of these is a vsync frame: a cursor change, a shell surface appearing, a
   night light fade (choice 15). A refused test is logged once per output
   and the trace shows it. Tearing and VRR together tear only above the
   panel's range, as KWin's AdaptiveAsync does. A game that does not ask
   never tears (v1's `immediate` rule is not kept): DXVK, VKD3D and Mesa
   ask when the game turns vsync off, so the game's own setting is the
   switch. `output.tearing` is off by default: with VRR the range covers
   the games, and tearing is a choice for a player who wants uncapped
   frames.

### The pointer in games

9. **Pointer constraints and relative motion, released by the desktop.**
   Lock and confine are granted to the focused surface under the pointer,
   as the protocol says; relative motion carries libinput's unaccelerated
   delta beside the accelerated one. A constraint is released, and given
   back when its surface is focused and under the pointer again, when focus
   leaves (Cmd+Tab, a click elsewhere, a prompt), when the view leaves the
   game's desktop (Full-screen Spaces 4), when the capture pick opens (P11
   choice 5 promised it) and while the lock is up. The top-edge reveal is
   refused while a constraint is active (Game Mode 5); a game holding the
   pointer is reached by the escape set.

### Games

10. **Detection gains ancestry and the content type.** Rule 1's missing
    signals: a window whose process descends from Steam, Heroic or Lutris
    and is not the launcher's own UI (Steam's `steam` and `steamwebhelper`,
    Heroic's Electron, Lutris's Python) is a game; the process tree is read
    from `/proc` on the worker at map, as `SteamGameId` already is. A
    surface that sets wp-content-type `game` is a game. `steam_proton` joins
    the class list. `.exe` classes stay (a Wine installer counts as a game
    until corrected): GameMode is off by default under Steam (§7), so the
    wider net is worth an occasional correction, and rule 2's
    `windows.not-games` corrects it. The correction gains a door: `isle
    windows game <app> yes|no|auto`, until Settings draws it (P13). Flatpak
    Steam needs nothing more: the client's credentials and GameMode give
    host PIDs.
11. **A game's window is never placed or remembered** (Games 7):
    `record_place` and placement skip a game, so a game opens where it asks
    or at the screen's origin, and its place is never written.
12. **Game Mode turns window motion off, and only that is new.** P8 already
    draws the shell solid and still under Game Mode. `compositor.md`'s game
    profile (Gaming display path 8) adds window motion (open, close,
    minimise, zoom animations) off while Game Mode is on, for every window,
    restored on leaving without a reload. Blur already goes with the solid
    drawing; shadows stay, since a windowed game sits among other windows
    and a shadow costs nothing on a scanned-out frame.
13. **A game on a hidden desktop is paced, not stopped.** A game window
    whose desktop is not in view gets frame callbacks and fifo barriers
    released at 30 Hz on the estimated clock, where other hidden windows
    get one a second. Games that block their main loop on present keep
    their network and audio alive, and an alt-tabbed game costs a fraction
    of its GPU time. This answers Full-screen Spaces' open point.

### Steam

14. **Steam is an app with three rules of its own.** Quit on Steam runs
    `steam -shutdown` through the launch path rather than closing windows,
    which only hides it to the tray (Steam's desktop client 4). Its toast
    windows (`notificationtoasts_<n>_desktop`) are placed where asked,
    never focused, never in the switcher and never given a title bar. Its
    menus are already unmanaged (P4). Big Picture is a small pure model
    beside the registry: a `steam` window whose title contains "Big
    Picture" is marked once, so a later title change does not lose it; it
    is a game (rule 6) and a full-screen desktop of its own whether it asks
    for full screen or sizes itself to the screen (P4's borderless check);
    minimised, it counts as closed (its desktop goes, the view returns, the
    switcher and App Exposé leave it out); and Steam's main window, which
    Steam shows again on whatever desktop is current when Big Picture
    closes, is moved back to the desktop it was on when Big Picture opened
    if it reappears within ten seconds. Its exact title and hints have no
    primary source (§5), so the model's matcher is data checked on the
    host, and the X11 test client mimics both shapes headless.

### gamescope

15. **Nested gamescope gets what it reads.** With choices 2, 3 and 9 every
    global gamescope's Wayland backend needs is served; `wl_output`
    advertises the current mode's real rate, which gamescope takes under
    VRR (§6). A test lists the globals gamescope binds and fails on one
    missing (bar 11). gamescope's window is a game by class (P5) and named
    by what it hosts (P10). Its `--backend sdl` advice for pointer lock and
    its device choice on the hybrid machine are the user's, noted in
    `gaming.md`.

### Night light

16. **Night light is the CRTC's gamma LUT, with a shader where there is
    none.** On the DRM backend Isle writes `GAMMA_LUT` on each CRTC
    outside `DrmCompositor`'s commits (niri's way, §4), falling back to the
    legacy gamma ramp; it keeps the blob it found and puts it back when the
    session leaves the VT or ends. Where neither exists (the VM's
    virtio-gpu, the headless backend, winit) the same curve is a final
    shader pass over the output's frame, which costs direct scanout and so
    is never the path on the host. The CTM is not used: NVIDIA applies it
    per plane before blending (§3), so it would tint planes, not the
    screen. The LUT is post-blend, so scanout stays on, the cursor is
    tinted with everything else, and a game in front costs nothing.
    Captures (B80) render the element list, so shots and streams are never
    tinted, as macOS's are not; the shader path is left out of capture
    renders by name.
17. **The warmth is a blackbody white point.** KWin's table (Ingo Thies's
    blackbody values in 100 K steps, interpolated, applied in linear light
    and encoded back, identity at 6500 K, §4) is ported as data. The
    warmth setting runs from 6000 K (less warm) to 2700 K (more warm), 3400
    K by default, macOS's Night Shift slider in kelvin underneath.
18. **One `night-light` section for every screen.** Schedule: Off, Sunset
    to Sunrise (the default), or Custom (from, to); warmth; and Turn On
    Until… as state, not settings: Control Centre's tap turns it on until
    the next scheduled off (or sunrise) or off until the next on, which is
    what the module's "On until {time}" and "Off until {time}" say.
    Sunrise and sunset come from a small `isle-sun` crate (NOAA's
    formulas) with the coordinates of the system's time zone from tzdata's
    `zone1970.tab`, as v1's `theme/sun.py` did, recomputed daily and on a
    time-zone change; no location service. The same crate makes
    `appearance.mode` Auto real, which today resolves to Dark. Changes fade
    over a second when toggled and over a minute at a schedule's edge, in
    vsync frames (choice 8).
19. **Game Mode turns night light off and gives it back** (Game Mode 3, 4).
    Entering, Game Mode takes a hold that forces the transform to identity
    at once, no fade; leaving, the hold goes and the schedule's state
    returns with a fade. A user who turns night light on during Game Mode
    wins: the hold is dropped for this run. Control Centre's module reads
    "Off during Game Mode" while held (`p1/surfaces/control-centre.md`).
    The module itself, not yet since P8, becomes a toggle with the
    `sun-horizon` glyph and those lines; its "Settings…" waits for
    Displays (P13).

### Proof

20. **What headless can prove, it proves; the rest is the host's.** The
    headless backend has no DRM, so VRR, async flips, scanout and the gamma
    LUT cannot run there. Each is split into a pure decision (VRR, tearing,
    the cursor hold, night light's schedule and curve, Big Picture, the
    detection rules) tested in isolation, and an application checked by
    the trace on real hardware. Headless tests cover the protocols through
    the test client (presentation times and refresh, fifo, commit-timing,
    syncobj where the render node supports it, constraints and relative
    motion, content type, tearing hints), the shader path of night light
    in screenshots, hidden pacing, places, Steam's rules and Big Picture
    with the X11 test client. The VM runs gamescope nested and Xwayland's
    syncobj on llvmpipe or virtio. Acceptance bars 1–3, 5–7 and 9–10 are
    measured on the 5070 Ti and the AOC, so the host run on tty8 is part
    of P12's done, not optional: the user runs it, with a game, Steam and
    Big Picture.

## The workspace

```
crates/
  isle-sun/              sunrise, sunset and time-zone coordinates (new)
  isle-settings/         `output.tearing`, `night-light`
  isle-state/            `night-light` (the Turn On Until… override)
  isle-games/            launcher process names for ancestry
  isle-compositor/src/
    backend/udev.rs      scanout feedback, hardware presentation times,
                         VRR and tearing applied, the cursor hold, gamma
    present.rs           the pure VRR, tearing and cursor-hold decisions,
                         the plane trace (new)
    night_light.rs       schedule, curve, holds, the LUT and the shader pass
                         (new)
    pointer.rs           constraints and relative motion (from input.rs)
    keyboard/games.rs    ancestry, content type, the Big Picture model
    wm/                  places skipped for games, hidden pacing, motion off
  isle-shell/src/
    control_centre/      the Night Light module
  isle-ipc/, isle-cli/   `isle displays trace`, VRR and tearing in
                         `isle displays`; `isle windows game`;
                         `isle night-light`
  isle-test-client/      presentation, fifo, commit-timing, syncobj,
                         constraints, relative motion, content type, tearing
```

The fork gains #1948 and the tearing-control module as pull requests.

## Phases

Each phase ends in a check run by script or in the VM and lands as its own
commits.

### 12.0 · The fork

- #1948 rebased onto `isle`, wp-tearing-control-v1, both through pull
  requests merged and pinned; anvil's async path run in the VM to see it
  build and refuse cleanly where the driver cannot.

**Check:** the fork's tests; Isle builds on the new pin with nothing
changed.

### 12.1 · Presentation and the trace

- Hardware timestamps and sequences, `Refresh::Variable`, the flags,
  feedback for layers and X11; scanout dmabuf feedback; the plane trace and
  `isle displays trace`; the default mode at the highest rate.

**Check:** headless: presentation times monotonic with the refresh,
feedback on a layer surface; the VM: the trace showing composited frames.

### 12.2 · The protocols

- syncobj, fifo, commit-timing, content-type, the dmabuf blocker; pointer
  constraints and relative pointer with their releases; the test client's
  side of each.

**Check:** headless: a fifo client paced by the output, a commit-timing
target honoured, a locked pointer receiving relative motion only and
released by Cmd+Tab and by the pick, the reveal refused while locked; the VM:
Xwayland binding syncobj.

### 12.3 · VRR

- The pure decision, `use_vrr` on change, HDMI's rule, the cursor hold with
  the EDID range, the live setting, `isle displays`.

**Check:** unit tests of the decision and the hold; headless: the decision
following a full-screen window in and out of front.

### 12.4 · Tearing

- `output.tearing`, the pure eligibility function, the async test commit
  and its fallback, the trace's async mark.

**Check:** unit tests of eligibility (cursor change, shell surface, fade,
setting off, no hint); headless: the hint read from the test client.

### 12.5 · Games

- Ancestry, content type, `steam_proton`, `isle windows game`, places
  skipped, window motion off under Game Mode, hidden pacing at 30 Hz.

**Check:** headless: a child of a fake launcher flagged and the launcher's
own window not; a content-type game; a game opening where it asks and its
place unwritten; a hidden game's callbacks at 30 Hz and another window's at
one a second.

### 12.6 · Steam and gamescope

- Quit through `steam -shutdown`, toasts, the Big Picture model; the
  globals test; `wl_output`'s rate.

**Check:** headless with the X11 test client: Big Picture by title and by
size, minimised as closed, Steam's window moved back; the globals test; the
VM: nested gamescope running a client full screen.

### 12.7 · Night light

- `isle-sun`, the `night-light` section and state, the curve, the LUT with
  its restore, the shader pass, the fades, Game Mode's hold, Control
  Centre's module, `isle night-light`, appearance Auto.

**Check:** unit tests of sun times for known places and dates and of the
curve; headless: a screenshot warm through the shader pass with the
capture untinted, Game Mode's hold and the user's override; the VM: the
module and the shader path.

### 12.8 · The measure, the VM and the host

- Release build at 3440 × 1440 and 144 Hz: input to the focused client at
  the 99th percentile under GPU load (bar 5), the loop's frame time with a
  full-screen client, nothing drawn by the shell over a game (bar 4).
- The host run on tty8, the user's, with the trace: a Proton game and a
  native Vulkan game scanned out for ten minutes (bar 1), tearing on and
  off with a test pattern (bar 2), VRR intervals across 48–144 fps and VRR
  leaving with the game (bar 3), an hour without a stall (bar 6), no blank
  on entering, leaving or Cmd+Tab (bar 7), Steam's menus (bar 9), Big
  Picture twenty times (bar 10), gamescope's rate (bar 11), night light on
  the LUT with a game in front still scanned out.

**Check:** the measures in `docs/research/rust-desktop/p12-gaming.md`.

### 12.9 · Close

- ARCHITECTURE: how a frame reaches the screen (scanout, VRR, tearing, the
  trace), how night light is applied. DECISIONS for what later packets
  keep. `gaming.md`'s and `compositor.md`'s open points; the roadmap's P12
  entry, P13 (Displays' VRR, tearing and night light rows, the game
  correction in Settings), P22 (night light beside HDR); `state.md`,
  `progress.md`.

## Not in P12

Controllers (P18); HDR and colour management (P22); DRM lease for VR
headsets (`gaming.md` Not here; `compositor.md`'s protocol table is
corrected at close); a second GPU's scan-out (P14); Settings › Displays and
Focus › Game Mode as panes (P13); overlay planes on NVIDIA; tearing for a
game that does not ask.

## Depends on, and feeds

- Needs P2 (the DRM backend, the redraw loop), P4 (full-screen desktops,
  the XWM, places), P5 (the game flag, the escape set), P8 (Game Mode,
  Control Centre), P9 (idle holds), P10 (libraries), P11 (the capture
  render, the pick's grab).
- P13 draws the Displays and Focus rows over these settings. P18 adds pads.
  P22 brings HDR, which must then share the CRTC's colour pipeline with
  night light.

## Done when

- `cargo test` passes headless: the decisions, the protocols through the
  test client, night light's shader path and capture rule, detection,
  places, pacing, Steam and Big Picture.
- In the VM, nested gamescope runs and Xwayland binds syncobj.
- On the host, the trace shows a full-screen game scanned out with VRR
  following it, an async flip when allowed and asked, and night light
  leaving scanout on.

## Risks

- **Async flips on Blackwell.** Nothing reports them either way (§8). The
  test commit is the guard: a refusal means vsync and a log line, never a
  failed frame. If they never pass, tearing waits and VRR carries the games
  (B6's own fallback).
- **Scanout on NVIDIA.** If the primary plane refuses game buffers, every
  game frame is composited. The trace says so; the scanout tranche is the
  first fix, and a game's format that the plane cannot take is the second
  look.
- **VRR blanking the AOC.** If changing `VRR_ENABLED` blanks the screen on
  this driver, Full screen would break Presentation 9. The host run
  watches; the fallback is to recommend Always for that output, which
  B30's setting already allows.
- **Big Picture's shape.** Its title and hints are unconfirmed (§5); the
  matcher is data and the host run settles it.
- **#1948's rebase.** It is open upstream and may move. The fork carries it
  as P2 planned (B6); if it will not rebase cleanly, the tearing work is the
  phase that waits, not the packet.

## Reviewed

The user reviewed the plan on 2026-10-10 and took every recommendation:
tearing off by default, hidden games paced at 30 Hz, `isle-sun` built here
with appearance Auto, and the host run on tty8 part of done.
