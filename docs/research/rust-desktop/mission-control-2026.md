# Mission Control, 2026

Evidence for P16 (`roadmap.md`, Mission Control): macOS 27's Mission
Control, Spaces, App Exposé, Show Desktop, Split View and Stage Manager; the
trackpad's gestures; how libinput, pointer-gestures and Smithay carry them;
how niri, GNOME, KWin and Hyprland track a swipe and draw an overview; what
this host has. Read 2026-10-26 from Apple's macOS 27 support pages, AppKit's
documentation, niri at `5f23510`, Smithay at `24647fc`, GNOME Shell's and
KWin's main branches, Hyprland's wiki for 0.51. Marked *unverified* where
the only source is old, third-party or memory. Sections are cited from the
plan as §n.

## 1. Mission Control and Spaces on macOS 27

- **Opening.** Three fingers up (a Trackpad setting), the Mission Control
  key (F3) or Control-↑, a hot corner with an optional modifier, Magic
  Mouse's two-finger double tap, or a window dragged to the top of the
  screen ("Drag windows to top of screen to enter Mission Control"). Every
  corner is off by default. A click on the desktop closes it; Escape too
  (*third-party*). On a second display it shows that display's windows and
  Spaces.
- **Layout.** Every window of the current Space in one layer, ungrouped,
  over the real wallpaper; the desktop does not zoom out (since Yosemite).
  The Spaces bar is collapsed to labels until the pointer reaches the top
  edge, then shows live thumbnails until Mission Control closes; it opens
  expanded from a full-screen app or a top corner. No setting keeps it
  expanded. P1 chose the bar at full size always (`structure/mission-control.md`).
- **Arrangement.** Undocumented. A Sierra manual: thumbnails "placed
  relative to their full-size windows' positions". Two open references:
  - GNOME's `UnalignedLayoutStrategy` (`js/ui/workspace.js`): windows sorted
    by centre y fill rows towards an ideal width (total ÷ rows), each row
    sorted by centre x; several row counts are tried and scored by scale
    (weight 1) and space used (0.1); a window is never shown above 0.95 of
    its size, and small windows get a boost of `lerp(1.5, 1, h / screen h)`.
  - KWin's `ExpoLayout` (`plugins/private/expolayout.cpp`): the row split as
    a least-weight subsequence, O(n log n), with a concave weight.
- **Keys inside.** Arrows and Return choose; Space over a window shows it
  larger and again returns (Macworld, MacMost); Control-←/→ change Space,
  also during a drag; Control-1…n jump (off by default, only for Spaces
  that exist); Option-click on a Space switches without leaving.
- **Spaces.** + at the end of the bar adds one; the limit is 16. A window
  dropped on + gets a new Space. Hovering a Space shows ×, Option shows it on
  all; a full-screen or Split View Space shows an exit button instead. Spaces
  reorder by drag; desktops are numbered by position, unnamed. A window moves
  by a drag onto a Space in the bar, or to a screen edge with a pause (one
  display only). A window dropped on a full-screen Space makes a Split View.
- **Settings** (Desktop & Dock › Mission Control): "Automatically rearrange
  Spaces based on most recent use"; "When switching to an application,
  switch to a Space with open windows for the application" (on); "Group
  windows by application" (off since El Capitan); "Displays have separate
  Spaces". An app is assigned from the Dock's Options › Assign To: All
  Desktops, This Desktop, Desktop on Display n, None.
- **App Exposé.** Control-↓, or three or four fingers down (off by
  default). The front app's windows with its minimised ones; Tab moves to
  the next app.
- **Show Desktop.** Command with the Mission Control key, Fn-F11, or thumb
  and three fingers spread; pinch or press again to restore. Windows slide
  to the edges and leave a sliver. "Click wallpaper to reveal desktop":
  Always or Only in Stage Manager.
- **macOS 27.** "Smoother animations in Mission Control and Spaces" in
  Apple's list of changes, no layout change named. MacG reports moving a
  window to a new Space at about 25 fps, a second's freeze picking a Space
  on M5 Macs, black thumbnails and a drifting selection after a full-screen
  Space appears, open at 27.2 beta. Launchpad is gone since 26: the
  thumb-and-three pinch and Fn-Shift-A open Apps.

## 2. The trackpad

- **Point & Click:** tracking speed, click pressure, Quiet Click, Force
  Click, Look up (force click or three-finger tap), secondary click (two
  fingers, or a corner), tap to click.
- **Scroll & Zoom:** natural scrolling, pinch to zoom, smart zoom, rotate.
- **More Gestures:** swipe between pages; swipe between full-screen
  applications (three or four fingers sideways, which moves between Spaces
  too); Notification Centre (two fingers in from the right edge); Mission
  Control (three or four up); App Exposé (three or four down, off); Apps
  (thumb and three pinched); Show Desktop (thumb and three spread).
- **Finger counts disagree between Apple's own pages:** article 102482
  says four for the swipes, "some versions use three"; the macOS 27
  Mission Control page says three. Three Finger Drag moves the swipes to
  four (*unverified*). The count is a setting either way.
- **How a swipe tracks.** AppKit's fluid swipe
  (`NSEvent.trackSwipeEvent(options:dampenAmountThresholdMin:max:usingHandler:)`)
  reports the amount one to one in pages, keeps calling while the system
  animates to −1, 0 or 1 after the lift (so commit and cancel both
  animate), and pre-dampens an amount past the thresholds "to provide an
  elastic feel" (0.50 becomes about 0.125 with no page that way). That
  Spaces use it is an inference. Speed decides the outcome: a tool that
  switches instantly does it by simulating a swipe with a large velocity.

## 3. Split View, tiling and Stage Manager

- **Split View.** The green button's Full Screen › Left of Screen or Right
  of Screen; a click on a window offered in the other half. The pair is a
  new Space. The divider resizes and its double click restores; a drag on
  a title swaps sides; Replace Tiled Window picks another partner; Move
  Window to Desktop or Make Window Full Screen breaks the pair, the other
  app staying full screen alone. In Mission Control a window dropped on a
  full-screen Space, or one full-screen Space on another, pairs them.
- **Tiling** (Sequoia onwards, kept in 27) stays on the Space: edges for
  halves, corners for quarters, the menu bar to fill, Option to show
  targets at once; the green button's Move & Resize and Fill & Arrange;
  Fn-Control chords; "Tiled windows have margins". Isle's zones (B8) are
  this, built in P4.
- **Stage Manager.** From Control Centre or Desktop & Dock; needs
  "Displays have separate Spaces". The current set in the centre, recent
  apps and sets in a strip on the left. Dragging a strip item to the
  centre adds it to the set, back removes it. Settings: "Show recent apps
  in Stage Manager" (off: the strip shows only at the left edge); "Show
  windows from an application", All at Once or One at a Time (the most
  recent, a click again cycles). How Mission Control draws sets is
  undocumented; a Ventura guide says every window stays visible
  (*unverified*).

## 4. Gestures on Linux

- **libinput.** Swipes need three or more fingers: begin, update (the
  centre's dx, dy, also unaccelerated), end with a cancelled flag. Pinches
  need two or more: dx, dy, an absolute scale and a relative angle. Hold
  begins and ends; it exists to stop kinetic scrolling. **Two-finger motion
  is scrolling (axis events), not a gesture, and no gesture event carries a
  position**, so an edge start cannot be told from libinput's events. A
  gesture never changes its finger count: a fourth finger cancels a
  three-finger swipe and may begin a four-finger one. libinput's own
  three-finger drag, where enabled, takes three-finger swipes
  (*interaction unverified*).
- **pointer-gestures-unstable-v1**, version 3: `get_swipe_gesture`,
  `get_pinch_gesture`, `get_hold_gesture` (3); `begin(serial, time,
  surface, fingers)`, `update`, `end(serial, time, cancelled)`. One gesture
  a seat. The compositor may cancel; clients undo and act on nothing
  irreversible before `end`. niri, Mutter, KWin, Hyprland and Sway serve it.
- **Smithay.** `InputEvent::GestureSwipeBegin/Update/End`, the pinch
  three and `GestureHoldBegin/End` from the libinput backend
  (`GestureBeginEvent::fingers`, `GestureEndEvent::cancelled`).
  `PointerHandle::gesture_swipe_begin` and the rest forward them;
  `PointerGrab` has a hook for each; the global is
  `PointerGesturesState::new::<D>(&dh)` at version 3. Isle's `Focus` and
  grabs already implement the hooks (`focus.rs`, `grabs.rs`); nothing calls
  them, and the global is not created.
- **niri** (`src/input/mod.rs`). On a swipe's begin, unlocked: three
  fingers start an accumulator and are not forwarded; four begin the
  overview at once; any other count is forwarded. Updates use the
  unaccelerated deltas, negated under natural scrolling. The three-finger
  direction locks once the movement reaches 16 (GNOME's figure): sideways
  scrolls the view, vertical changes workspace. Pinches and holds are always
  forwarded.
- **niri's tracker** (`swipe_tracker.rs`, `layout/monitor.rs`). 150 ms of
  samples; velocity is their sum over the span; the projected end is
  `pos − v / (1000 · ln 0.997)`, from GNOME and libadwaita. 300 units are
  one workspace. A rubber band (`stiffness 0.5, limit 0.05`) past the ends;
  outside the overview the gesture is held to one workspace either way. At
  the end a zero sample is pushed at "now" so a paused finger kills the
  velocity; the target is the rounded projected end; a spring (damping 1,
  stiffness 1000 for workspaces, 800 for the overview) starts from the
  current position with the release velocity.
- **GNOME** (`swipeTracker.js`). 300 units vertical, 400 horizontal; the
  lock at 16; 150 ms of history; below a velocity of 0.6 on a touchpad
  the nearest point wins; deceleration 0.997; a projection back onto the
  start is pushed one point in the fling's direction; duration |Δ / v| · 3
  clamped to 100 ms … 400 · log2(1 + n) ms; progress hard-clamped to one
  point, no rubber band.
- **Hyprland** commits by distance (ratio 0.5 of 300) or by the average
  speed over the whole gesture (30); a direction lock at 10; no rubber band.
- **KWin** toggles Overview and Grid on four fingers up and down, as
  continuous states; desktop switching is four fingers sideways in recent
  Plasma 6, three in 6.0.

## 5. Drawing an overview

- **Rescaled in the main pass** (niri): every workspace's elements wrapped
  in Smithay's `RescaleRenderElement` and `RelocateRenderElement`. No
  offscreen copy; damage stays per surface; the cost is drawing each
  surface's full texture smaller.
- **Cloned scene nodes** (GNOME): `Clutter.Clone` re-issues the window's
  paint nodes under a transform; offscreen only for opacity.
- **A texture per thumbnail** (KWin): each window rendered into a
  framebuffer on `preFrameRender` and handed to Qt Quick; flexible for a
  toolkit, an extra pass and VRAM per window.
- For Isle: the current desktop's windows are large and live and their
  travel from the real rectangle is a per-window rescale, which
  `wm::draw::place` already does for motion; the miniatures of other
  desktops can be textures refreshed slowly, as B54's previews are
  (*inference; no project above measured the split*).
- **Hidden windows' frames.** Smithay's `send_frames_surface_tree` throttles
  a surface whose primary output is elsewhere; niri passes 995 ms. Once an
  overview shows every window they are all visible again. Isle parks windows
  off the current desktop and paces them (B89); a miniature drawn from a
  parked window's last buffer wakes nothing.

## 6. Reduced motion

macOS 27 moved Reduce motion to Accessibility › Motion. It turns the Space
switch and Mission Control's entry into a short crossfade that pops into
place, and full screen into a crossfade (Sierra-era write-ups and current
reports); there is still a delay. Whether a swipe still tracks under it is
*unverified*. Isle's Reduce Motion already turns every spring into a fade
(DESIGN, Motion); P1 has the fingers drive the crossfade.

## 7. This host

No trackpad: `libinput list-devices` shows the Keychron receiver's
keyboard and mouse and nothing with tap-to-click. P16 defers the gestures
to a packet of their own for that reason; §2 and §4 are its evidence, and
a uinput touchpad in the VM, which libinput reads as a real one, its way
to prove them.

## Sources

- Apple: Use Mission Control (mh35798), Work in multiple spaces (mh14112),
  Desktop & Dock settings (mchlp1119), Trackpad gestures (102482),
  Trackpad settings (mchlp1226), Keyboard shortcuts (102650), Split View
  (mchl4fbe2921), Tile windows (mchl9674d0b0), Stage Manager
  (mchl534ba392), Motion (mchlc03f57a1), all at support.apple.com;
  `NSEvent.trackSwipeEvent` at developer.apple.com.
- Third party: Wikipedia, Mission Control (macOS); Macworld 213968 and
  215453; MacMost 34342 and "Window tiling in macOS Sequoia"; TidBITS
  29365; mjtsai 37439; TekRevue, Reduce motion in Sierra; arhan.sh, native
  instant space switching; MacG, macOS 27 Mission Control bugs (2026-09);
  MacRumors, 250 changes in macOS Golden Gate.
- libinput, Gestures (wayland.freedesktop.org/libinput/doc/latest);
  pointer-gestures-unstable-v1 (wayland.app).
- niri (github.com/YaLTeR/niri): `src/input/mod.rs`,
  `src/input/swipe_tracker.rs`, `src/rubber_band.rs`,
  `src/layout/monitor.rs`, `docs/wiki/Gestures.md`,
  `Development:-Redraw-Loop.md`.
- Smithay (github.com/Smithay/smithay): `src/wayland/pointer_gestures.rs`,
  `src/input/pointer/mod.rs`, `src/backend/input/mod.rs`,
  `src/desktop/wayland/utils.rs`,
  `src/backend/renderer/element/utils/elements.rs`.
- GNOME Shell: `js/ui/swipeTracker.js`, `workspace.js`,
  `workspaceThumbnail.js`, `windowPreview.js`. KWin:
  `plugins/overview/overvieweffect.cpp`, `plugins/private/expolayout.cpp`,
  `scripting/windowthumbnailitem.cpp`. Hyprland wiki 0.51, Gestures.
