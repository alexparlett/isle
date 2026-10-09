# P11 · Capture and portals

Planned 2026-10-09. Reviewed by the user the same day: every
recommendation taken; one packet; the host run optional.

The roadmap's brief: `xdg-desktop-portal-isle` (B13), a bus name on the
compositor's service worker (B26): ScreenCast and Screenshot with PipeWire
streams from the compositor and the shell's picker, the screenshot tool,
ext-image-copy-capture; FileChooser follows in P15. A capture of a locked
screen shows the lock and a window share holds its last frame; the portal's
Inhibit joins idle's holds (from P9). Earlier packets left P11 more: the
eight capture chords P5 reserved (`screenshot-screen`, `screenshot-pick`,
`capture-toolbar`, their `-copy` forms, `colour-pick`, `stop-recording`),
already in the game escape set; the island's Recording and Sharing
activities and its screen privacy mark, which nothing posts (P6); the
banners' `capture` hold and `notifications.hold-sharing`, which nothing
feeds (P8); Control Centre's screenshot control, marked not yet (P8); the
capture tokens P1 added, which no code reads; the Settings portal B13 gives
Isle, still answered by xdg-desktop-portal-gtk.

P10 left a desktop that can be started from, searched and pasted into, but
cannot be shown. Shift+Cmd+3 logs "not built yet", there is no way to keep a
picture of the screen or a recording of it, and no app can share a screen:
Chromium, Firefox, OBS and Discord ask a portal nobody answers. P11 closes
all of it. What it must get right is that the picture is the screen: a shot
is the frame the user saw, never the tool's own drawing; a share shows what
the user chose and nothing else, stops when its source goes or the user says
so, and is never invisible; and nothing a capture does can take the
compositor down (D55).

## What it answers to

- Decisions: B13 (the portal backend, the picker, the capture protocols,
  the Settings portal), B26 (the backend on the worker), B11 (no libdbus in
  the compositor), B17, B28 (layer-shell and security-context), B30 (the
  escape set), B37 (surfaces), B44 (one focus path), B47 and B48 (the
  island's queue and shape), B49, B51, B53 (a tool that reads keys without
  focus), B54 (previews as compositor drawings), B57 (centre surfaces), B60,
  B63 (banner holds), B66 (every locked frame from `render_locked`), B67
  (a helper process for what may crash), B69 (idle holds), B70 (the prompt
  queue), B72 (the session's end), B78 and B79 (the clipboard, drag out).
- Intent: `shell.md` Capture 1–8, Island 2 and 4, Shell surface hosting 10
  and 13; `platform.md` Portal backend 1–8 and its open points (the
  non-interactive screenshot policy; FileChooser's is P15's), Theming 5,
  Compositor D-Bus API 4; `compositor.md` Pointer 4, Shell surface hosting
  4, the protocol table's capture rows and xdg-foreign, Shortcuts 7;
  `gaming.md` Game Mode 2.
- Design, binding: `p1/structure/capture.md`, `island.md` (Recording,
  Sharing, the privacy marks, the ended event), `competition.md` (bottom
  centre, the bottom-trailing corner, the centre group, capture while
  locked), `lock.md` (a capture running), `notifications.md` (held while
  sharing or recording); `p1/surfaces/capture.md`, `island.md`; DESIGN.md's
  Capture entry; `review/accessibility.md` and `copy.md`.
- Evidence: `capture-2026.md` (new; the § numbers below are its sections),
  `smithay-and-gaming.md` §3, `isle-v1-inventory.md` §4 Capture bar and §16,
  v1's `shell/services/Capture.qml` and `shell/scripts/capture.sh`.

## Choices this plan makes

### Frames

1. **Every still is one render.** `capture.rs` renders a target into an
   offscreen texture through the compositor's own element list: a screen
   is `render_elements` for that output, so a locked screen gives the lock
   (B66) with no rule of its own; a window is its `window_pieces` (surface
   tree, popups, Isle's title bar) at its output's scale, refused while
   locked; a region is a screen's render cut to the rectangle. Two things
   are left out by name: the pointer, unless Show mouse pointer is on
   (`compositor.md` Pointer 4, D27), and the capture tool's own surfaces
   (pick, toolbar, loupe, thumbnail), so a shot from the live toolbar is
   clean without a freeze. The pixels are read back on the loop and encoded
   as PNG on the worker at `png`'s Fast setting (8 ms and 554 KiB for a full
   ultrawide frame, §7). `Test1.Screenshot` moves onto it.
2. **Files are written at once, by v1's folders and macOS's names.**
   `$XDG_PICTURES_DIR/Screenshots` and `$XDG_VIDEOS_DIR/Recordings`, made
   when missing; `Screenshot 2026-10-09 at 14.02.11.png`, " (2)" for a
   second screen or a second shot in the same second (`capture.md`). A
   write that fails puts the shot on the clipboard and says so in a banner.
   With Ctrl, the PNG goes to the clipboard through P10's `set_clipboard`
   and the island says "Screenshot copied"; it is kept in clipboard history
   as any copy is.

### The tool

3. **The freeze is the compositor's.** At Shift+Cmd+4 (and Shift+Cmd+6),
   each output is rendered without the pointer into a texture, and each
   shown window, open popup and open shell panel into one of its own with
   its shadow, from the buffers held at that moment (`capture.md`: a covered
   window comes out whole). While frozen, each output draws its frozen
   texture in place of the scene; clients keep running under it and are
   unaware. The textures go when the pick closes. The cost is a render per
   output and per shown window once, at the chord.
4. **The veil, the cut, the capture line and the lift are compositor
   elements; the shell draws only text.** The veil is a solid over the
   frozen frame with the selection or the lifted shape cut out, the line a
   pair of rectangles or the window's rounded outline, as the dim and the
   island's shape are already drawn (B48). The size readout and, for the
   toolbar, the bar are shell surfaces. A tree would need each frozen frame
   copied into Skia for nothing it could draw better.
5. **The pick reads keys and the pointer without taking focus.** As the
   switcher does (B53): the compositor holds the keyboard and pointer for
   the pick, the focused window keeps its focus, hot corners and the top
   edge are off, and a game's pointer lock or confinement is released until
   it closes and then given back (`capture.md`, Full-screen game). The pick
   closes nothing (`capture.md` Shared places): a shot can show Spotlight or
   an open Control Centre.
6. **The toolbar is a placed overlay surface in the centre group.** It has
   a menu and takes the keyboard, so it follows Spotlight's pattern (B73's
   surface side, B57): opening it closes Spotlight, the switcher, Force
   Quit and the session dialog, and they close it. Its mode drawing over the
   live screen (the portion's rectangle and handles, the lifted window, the
   marked screen) is the compositor's, as in the pick. The Options menu is
   `isle-ui`'s menu. Its options live in a new `capture` settings section
   (save to, timer, microphone, system audio, floating thumbnail, remember
   selection, mouse pointer, mouse clicks, the colour value's format), its
   place and last selection in a new `capture` state domain.
7. **The countdown is an island event.** A new OSD-class event kind
   (`capture.md` The countdown): it pre-empts, shows over a full-screen
   window, and a click or Cmd+Ctrl+Escape cancels. The shot or recording
   starts when it ends, so it is never in the frame.
8. **The thumbnail is a top-layer surface, and what waits for Files is left
   out.** The shot's texture is copied into its tree through P7's
   `image_of`. Hover holds it, a flick sends it, a drag carries the file
   through P10's drag source (the file stays), and its menu has Save to,
   Copy, Move to Trash and Close. A click opens the shot in the default
   image viewer through the launch path until Quick Look exists; Markup,
   Show in Files and Quick Look on a recording come with Files (P15), which
   then takes the click. This is P10's rule: left out, not hidden.
9. **The colour pick reads the frozen frame.** The loupe is a compositor
   element sampling the frozen texture unsmoothed, fifteen pixels square,
   with its grid; the value capsule is a shell surface. A click copies the
   value in the `capture.colour-format` setting (hex by default) and the
   island says "Colour copied". PickColor (choice 21) uses the same pick and
   answers the app instead. sRGB values pass straight through (§8).
10. **Over a game, under an inhibitor, while locked.** As `capture.md`'s
    states: Shift+Cmd+3 over a full-screen game shoots with nothing drawn,
    no thumbnail, "Screenshot saved" in the island and the screenshot sound;
    the pick and toolbar draw solid with no motion. Under
    shortcuts-inhibit the chords are the client's. Locked, the chords do
    nothing. The sound is the theme's `screen-capture` through P8's sound
    path, behind a `capture.sound` switch, on by default.

### Streams

11. **Streams are produced in the compositor, niri's way.** A new
    `cast.rs`: PipeWire's loop fd is a calloop source, so there is no
    PipeWire thread and every frame is rendered on the loop with the
    compositor's own GL context (§2.1, §3). The context is made with
    `module.rt` and `support.dbus` off, as the camera reader's is (B11; and
    module-rt would ask the Realtime portal, which a portal backend could
    deadlock on). Each stream drives itself (`DRIVER | ALLOC_BUFFERS`),
    offers BGRA and BGRx as dmabufs with each modifier that passes a GBM test
    allocation, one fixed-modifier pod each with LINEAR first, and shared
    memory last; writes the header and cursor metadata on every buffer;
    skips a frame with no damage and no cursor change; spaces frames by the
    negotiated rate; and waits on the render's fence before queueing (the
    NVIDIA flicker fix). Start reports each stream's `pipewire-serial`.
12. **Three sources: a screen, a window, a portion.** A screen is choice 1's
    screen render with the pointer per the cursor mode (hidden, embedded, or
    metadata as a bitmap and position); a portion is a screen render cut to
    its rectangle; a window is its pieces at its scale, without its shadow,
    following its size by renegotiating. All three leave the capture tool
    out, and banners are held while any stream runs (choice 17).
13. **A source streams only while it is shown (D55).** A window that is
    hidden, minimised or on another desktop queues nothing, and the consumer
    holds its last frame; a shown window that is covered keeps its frame
    callbacks and keeps streaming. Locked, a screen stream carries the lock
    as drawn and a window stream queues nothing until unlock (`lock.md`). A
    window that closes or an output that goes ends its stream, posts the
    island's ended event with its reason and a silent entry in Notification
    Centre (`island.md`), and closes the portal session.

### Recording

14. **Recording runs in a helper process, `isle-record`.** Encoding brings
    CUDA, NVENC and an encoder's threads; none of it belongs in the
    compositor (B67's reasoning). The compositor makes a stream as for a
    share and starts `isle-record` with the stream's serial, the file's path
    and the options; the helper reads the stream with pipewire-rs (module-rt
    off) and encodes with `ffmpeg-next` built without libavdevice, which
    would bring libdbus (§4). It is not a portal client: Isle's own tool
    never asks itself.
15. **H.264 in MP4 with AAC, through NVENC when it opens.** H.264 High at
    the output's rate up to 60 fps, level chosen from the size (5.2 for a
    60 Hz ultrawide, §4), `h264_nvenc` when it opens and `libx264` when not
    (the VM); MP4 written in fragments while recording, so a crash leaves a
    file that plays, and rewritten with its index first on Stop. Audio, when
    asked: the default output's monitor and the chosen microphone as
    PipeWire capture streams, mixed in the helper, AAC-LC at 48 kHz. GStreamer
    is rejected: no MP4 muxer is installed without gst-plugins-good, and
    `pipewiresrc` brings module-rt. Frames reach the helper as shared memory
    in P11; a dmabuf path through Vulkan (`hwmap` and `h264_vulkan`) is built
    only if 11.15 measures the copy as too dear beside a game.
16. **One recording at a time, and it is never silent.** While it runs the
    island shows the Recording activity with its elapsed time, the screen
    privacy mark on every screen, the microphone's when recorded; Stop is
    the Now row's button, Cmd+Ctrl+Escape, or the toolbar opened again. A
    recorded window that closes or a screen that goes ends it and keeps the
    file (choice 13). Stop shows the thumbnail with the first frame. Show
    mouse clicks draws a ring at each press into the recording's frames, not
    the screen's.
17. **Capture holds banners.** Any stream, Isle's recording or an app's
    share, feeds `Holds::capture` while `notifications.hold-sharing` is on,
    so a recording does not catch a stranger's message (`capture.md`
    Recording).

### The portal backend

18. **A crate of hand-written zbus interfaces, owned on the worker.** A new
    `isle-portal` holds the impl interfaces, the Request and Session
    objects, the restore data and the sender check, with no compositor in
    it, reaching the loop through a trait as `isle-polkit` does; its held
    replies are P9's pattern. `ashpd`'s backend module is rejected: it is
    marked unstable, lacks Inhibit, and the interfaces are a few hundred
    lines (§2). `version` is exported lower-case (§1.1).
19. **The name is owned before anything can start the frontend.** The
    frontend reads a backend's version and cursor modes once, at its own
    start (§1.1). The worker takes `org.freedesktop.impl.portal.desktop.isle`
    before `isle-session.target` starts; when its supervisor takes the name
    again after a restart, it restarts `xdg-desktop-portal.service` if that
    is running. The package ships `isle-portals.conf` (ScreenCast,
    Screenshot, Access, Inhibit to Isle; Settings `isle;gtk`; FileChooser
    gtk until P15; Secret gnome-keyring; the rest gtk) and a new
    `isle.portal` naming the interfaces; v1's file stays on `main`.
20. **Only the frontend may call it.** Each call's sender must be the owner
    of `org.freedesktop.portal.Desktop`, resolved per call; any other caller
    is refused, so nothing skips the frontend's permission checks (§10).
21. **Isle answers Access, and with it the non-interactive policy.** The
    Screenshot portal needs an Access impl (§1.1). Isle's is a P9 prompt
    (B70) with the app's icon and the frontend's words. A non-interactive
    screenshot is then the frontend's own check: the permission store's
    `screenshot` table, asked once per app ("Allow {app} to take
    screenshots?"), the answer remembered; revoked in Settings › Privacy &
    Security (P13) and meanwhile with `isle portal permissions`. Apps with
    no app id share one answer, the frontend's rule; Isle cannot do better
    without seeing the caller.
22. **Screenshot: interactive opens the toolbar, non-interactive shoots.**
    An interactive request opens the toolbar with the shot modes and "{app}
    wants a screenshot", its action reading Share (`capture.md`); no file in
    Screenshots, no thumbnail. A non-interactive one, already allowed by the
    frontend, takes every screen as laid out (or the front window for
    `target` 8). The file is written under `$XDG_CACHE_HOME/isle/portal/`,
    where it outlives the request for the document portal and is pruned
    after a day. PickColor is choice 9's pick. `AvailableTargets` offers
    screen, window, area and active window.
23. **The picker is a sheet on the asking window when Isle can find it.**
    Isle serves xdg-foreign v2, and the fork gains a lookup from an exported
    handle to its surface (§1.2); `x11:` handles are found through the XWM.
    A found window gets the picker as a sheet attached to it (P4's attached
    surfaces, as the not-responding alert hangs on its window); a request
    with no usable parent is centred on the main screen. It closes no other
    surface and holds only its window's focus (`platform.md` Portal backend
    2). Tiles are P7's previews, drawn only while the picker is open (B54,
    D55). A request made while locked waits for unlock; the app going
    cancels it.
24. **Restore data names what the user sees.** `("Isle", 1, a{sv})`: a
    screen by its identity (make, model, serial, as places and wallpapers
    already key outputs), a portion by that and its rectangle, a window by
    app id and title, matched to that app's front shown window. Data that no
    longer resolves, or that Isle cannot read, opens the picker. Persisting
    "until revoked" is the frontend's permission store (§10); the
    picker's "Let {app} share this again without asking" is on when the app
    asks to persist.
25. **Chromium's second ask is answered once.** Chromium opens one session
    to preview and a second to capture (§9). A SelectSources from an app
    with a non-empty app id, with the same types, within ten seconds of a
    Start Isle granted to that app, is given the same sources without the
    picker. An app with no id always gets the picker.
26. **Sharing is never silent.** Each running stream is in the island's
    Sharing activity, one Now row per stream naming the app and the source,
    each with Stop; the screen privacy mark shows on every screen, on the
    lock too. Stop ends the stream from Isle's side and closes the session
    (`platform.md` Portal backend 5).
27. **Settings: the appearance namespace from Isle's own settings.**
    `color-scheme` from `appearance.mode` (Auto resolved), `accent-color`
    from the accent token as sRGB, `contrast` from Increase Contrast,
    `reduced-motion` from Reduce Motion, with `SettingChanged` on each
    change. `Settings=isle;gtk`: the frontend merges gtk's answers for the
    namespaces Isle does not serve (§1).
28. **Inhibit joins idle's holds and the session's end.** Idle and Suspend
    become idle holds named by the app (Isle suspends nothing by itself, so
    both mean "keep the clock"); Logout and User Switch are named by the
    session's end as another program's block inhibitor already is (B72).
    A monitor's `screensaver-active` follows the lock, and `session-state`
    goes to query-end when `end_session` starts asking apps and to ending
    when it commits; a `QueryEndResponse` or a second's wait lets it go on.

### Protocols and the rest

29. **ext-image-copy-capture with output and window sources, and
    ext-foreign-toplevel-list.** Smithay has all three (§5). Frames come
    from choice 1's render, held until the source has damage, at an exact
    size, `Xrgb8888` and `Argb8888` first; cursor sessions are served. A
    window source fails its frames while locked or not shown. Clients under
    security-context get none of them. A session that takes a second frame
    is a stream: it shows the screen mark and a Sharing row naming the
    client's app, whose Stop stops the session; a single frame (grim)
    shows nothing, as a screenshot through the portal does not.
    wlr-screencopy is not served: its only installed users are wf-recorder
    and hyprpicker, which Isle's recording and colour pick replace (§5).
30. **The CLI acts; it does not return pixels.** `isle capture
    screen|pick|toolbar|colour [--copy]`, `isle capture record start|stop`,
    `isle capture streams [--stop <id>]` and `isle portal status|permissions`
    do what the chords and rows do and print paths and states, never image
    data (`platform.md` Compositor D-Bus API 4). Control Centre's screenshot
    control opens the toolbar. Test1 gains the freeze, frame counts per
    stream and the pick's state as JSON.
31. **Tests run a PipeWire of their own.** Each headless test that streams
    starts a private `pipewire` on its runtime directory with a minimal
    configuration and no session manager; `isle-test-client` gains a
    consumer that links to a node by serial and reports formats, frames and
    metadata. The portal's tests drive the impl interfaces from a fake
    frontend that owns `org.freedesktop.portal.Desktop` on the test's bus.

## The workspace

```
crates/
  isle-portal/           the impl interfaces (ScreenCast, Screenshot, Access,
                         Settings, Inhibit), Request and Session, restore data,
                         the sender check (new)
  isle-record/           the recording helper: PipeWire in, ffmpeg out (new)
  isle-settings/         `capture`
  isle-state/            `capture` (toolbar place, last selection)
  isle-ui/               the capture bar's segmented modes, the sheet frame
  isle-shell/src/
    capture/             the toolbar, readout, loupe value, thumbnail, picker
    island/              the countdown event
  isle-compositor/src/
    capture.rs           renders, the freeze, the pick's grab, files, the
                         clipboard, the colour pick
    cast.rs              PipeWire streams, sources, pacing, cursor metadata
    recording.rs         the helper's life, Stop, the activity
    portal.rs            the loop's side of the backend, the picker, Access
    image_copy.rs        ext-image-copy-capture, sources, foreign-toplevel-list
  isle-ipc/, isle-cli/   Capture1, Portal status; isle capture, isle portal
  isle-test-client/      a PipeWire consumer
data/
  portal/                isle.portal, isle-portals.conf
```

The fork gains the xdg-foreign handle lookup as a pull request, as
alexparlett/smithay#1 and #2 were.

## Phases

Each phase ends in a check run by script or in the VM and lands as its own
commits.

### 11.0 · The render and the file

- `capture.rs`'s render of a screen, window and region with the pointer
  and capture surfaces left out by name; readback; PNG on the worker; names
  and folders; the clipboard copy; `capture` settings and state;
  `Test1.Screenshot` on it.

**Check:** headless tests: a screen, a covered window whole, a region, the
locked screen giving the lock, a window refused while locked, two outputs
writing two files, a full disk falling back to the clipboard.

### 11.1 · Shift+Cmd+3

- The screen chords and their copy forms, the island's "Screenshot copied"
  and "Screenshot saved", the sound, the game case, the banner on failure.

**Check:** headless chord tests over a window and over a full-screen game
flag; the VM.

### 11.2 · The freeze and the pick

- Frozen textures per output and per shown window, popup and panel; the
  veil, cut, line and lift as elements; the readout surface; the pick's
  grab with region and window modes and every key in `capture.md`'s table;
  pointer lock released and given back.

**Check:** headless: a region and a window shot from a frozen frame after
the window has changed, keys only, Option without the shadow, the window
keeping focus; the VM.

### 11.3 · The thumbnail

- The top-layer surface, its timing, hover, flick, drag, menu, the click
  through the launch path, replacement by a new shot.

**Check:** headless: a drag delivering the file to the test client, Move to
Trash, Save to; the VM.

### 11.4 · The toolbar and the countdown

- The bar in the centre group, the modes, Options and the `capture` section,
  the live portion, window and screen drawing, the countdown event, the
  shot without the tool's parts.

**Check:** headless: a portion shot with the bar open containing none of
it, the countdown cancelled, the centre group both ways; the VM.

### 11.5 · The colour pick

- The loupe element, the value surface, the formats, "Colour copied".

**Check:** headless: the copied value under a known pixel; arrows by one and
ten.

### 11.6 · Streams

- `cast.rs`: the PipeWire loop in calloop, negotiation with dmabufs and
  shared memory, header and cursor metadata, damage, pacing, the fence; the
  three sources; shown-only, the lock and the ends; the capture hold;
  private PipeWire in the tests and the test consumer.

**Check:** headless: a consumer receiving frames of a screen, a window and
a portion; no frames while the window is hidden and frames again when
shown; the lock in a screen stream; the stream ending when the window
closes and the output goes; neither `libdbus` nor module-rt mapped.

### 11.7 · Recording

- `isle-record`, the encoder choice, fragments then the index, audio, the
  Recording activity, Stop by every route, clicks, the thumbnail's first
  frame.

**Check:** headless: a recording of a test client played back by ffprobe
with the right size, rate and audio track; killing the helper leaves a file
that plays; the VM with libx264.

### 11.8 · The backend

- `isle-portal`, the name and its order, the frontend's restart, the sender
  check, the portal files, Access as a prompt.

**Check:** a fake frontend on the test's bus: calls from another sender
refused, Access answered both ways, the name taken before the target.

### 11.9 · ScreenCast and the picker

- CreateSession, SelectSources, Start with serials; the picker as a sheet
  and centred; xdg-foreign v2 and the fork's lookup; restore data; the
  second ask; Sharing rows and Stop.

**Check:** headless through the fake frontend: a share chosen in the picker
reaching the test consumer, a restore with no picker, a stale restore
opening it, Stop ending the session; the VM with Firefox and Chromium.

### 11.10 · Screenshot and PickColor

- Interactive through the toolbar, non-interactive by target, the cache
  folder and its pruning, PickColor through the pick.

**Check:** headless through the fake frontend for each target; the VM with
a portal client (`ashpd-demo` or a script).

### 11.11 · Settings and Inhibit

- The appearance namespace and its changes; Inhibit's holds, the session's
  end naming the app, monitors and `QueryEndResponse`.

**Check:** headless: `ReadAll` after a dark-mode change, an idle hold
keeping the clock, a logout listing the app.

### 11.12 · The capture protocols

- ext-image-capture-source, ext-image-copy-capture with cursor sessions,
  ext-foreign-toplevel-list; the stream rule; security-context refused.

**Check:** headless with a protocol test client; grim in the VM, a window
by its toplevel.

### 11.13 · The CLI and the rest

- `isle capture`, `isle portal`, Control Centre's screenshot control,
  Test1's additions.

**Check:** CLI tests.

### 11.14 · The measure, the VM and the host

- Release build at 3440 × 1440: the chord to a written file, the freeze's
  cost per window, a screen stream's cost per frame on the loop, a
  recording's CPU and the loop's frame time beside a 60 Hz game-like
  client, nothing drawn or streamed while idle, the shell inside its
  budgets.
- The VM: every chord, the pick, a recording, Firefox and Chromium sharing
  a screen and a window, OBS through the portal, grim. The host run, on
  tty8, is where NVENC, dmabuf to OBS and Chromium on the hybrid machine, a
  game recorded and Discord are seen; it is the user's choice.

**Check:** the measures in `docs/research/rust-desktop/p11-capture.md`; the
VM's screenshots.

### 11.15 · Close

- ARCHITECTURE: how a capture is rendered, how streams are produced, how
  the portal backend answers. DECISIONS for what later packets keep. The
  intent docs' open points; the roadmap's P11 entry, P13 (Privacy's
  remembered shares and screenshot permissions), P15 (Markup, Quick Look on
  shots and recordings, Show in Files, the thumbnail's click, FileChooser
  through `isle-portal`), P22 (capture of HDR outputs); `state.md`,
  `progress.md`.

## Not in P11

FileChooser (P15, on `isle-portal`); Markup, Quick Look, Show in Files and a
recording's trim (P15); Settings › Privacy & Security (P13); the virtual
monitor source type; RemoteDesktop and InputCapture (`platform.md`);
wlr-screencopy; a replay buffer, GIFs and OCR (`capture.md`); capture of HDR
outputs (P22); importing v1's capture settings (P14).

## Depends on, and feeds

- Needs P3 (surfaces, attached and placed), P4 (the registry, attached
  surfaces, the XWM), P5 (the shortcut table and the escape set), P6 (the
  island, the launch path), P7 (previews, `image_of`, the centre group),
  P8 (banner holds, the sound path, Control Centre), P9 (the lock's render
  path, idle holds, the prompt queue, the session's end), P10 (the
  clipboard and the drag source).
- P13 draws Privacy & Security over the permission store's tables. P15
  owns FileChooser, Quick Look and Markup. P22 brings HDR to every render
  here.

## Done when

- `cargo test` passes headless: every render target and the lock rule, the
  freeze, the pick's keys, the thumbnail's drag, the colour value, a
  stream of each source with its ends, a recording that plays, the
  portal's calls through a fake frontend, the capture protocols.
- In the VM, Shift+Cmd+4 then a drag writes a PNG of the frozen frame; a
  recording plays in ffprobe; Firefox shares a window chosen in Isle's
  picker; grim takes a shot.

## Risks

- **dmabufs on a hybrid NVIDIA machine.** A consumer that imports on the
  AMD node gets black frames (§9). LINEAR first among the offered
  modifiers and the shared-memory fallback are the guard; the host run is
  where it is seen.
- **A stream's cost on the loop.** Each stream is another render on the
  loop's thread. Damage-skipped frames and the negotiated rate bound it;
  11.14 measures a screen stream beside a game-like client, and a second
  GL context on a thread is the fallback if it costs a frame.
- **The frontend's start.** If anything starts xdg-desktop-portal before the
  name is owned, cursor modes are lost until it restarts (§1.1). Choice 19's
  order and restart cover the session and the worker; a hand-started
  frontend is the user's.
- **Recording copies.** Shared memory at 3440 × 1440 and 60 fps is about a
  gigabyte a second of readback. If it costs a game frames, the Vulkan
  dmabuf path (choice 15) is built.
- **Chromium's two sessions.** If ten seconds is too short or Chromium's
  order differs, the dialog shows twice; it is a choice in the model and the
  VM checks it.
- **The freeze on a busy desktop.** A render per shown window at the chord
  may take more than a frame with many windows; the measure says, and the
  fallback is to freeze windows lazily from their held buffers when window
  mode is first entered.

## Reviewed

The user reviewed the plan on 2026-10-09 and took every recommendation:
one packet, the host run optional, as for P6 to P10.

## Outcome

Built 2026-10-09 in phases 11.0–11.15, as planned, with these differences:

- **The share picker became a sheet after the close**, once
  alexparlett/smithay#3 was merged: closed with the picker centred, the sheet
  followed with xdg-foreign served.
- **Recordings stay fragmented MP4s**, each frame a fragment flushed as it
  goes, not rewritten with the index first on Stop: the index already leads
  the file, a recorder killed midway leaves one that plays, and the rewrite
  failed on AAC's packets through the API.
- **No metadata cursor in streams yet**: ScreenCast offers hidden and
  embedded; metadata waits for a bitmap of the pointer's image.
- **dmabufs only through the udev backend**, so the headless tests and the VM
  stream shared memory; ext-image-copy-capture copies into shared memory only,
  with no cursor sessions.
- **A capture-protocol session shows the screen mark**, but no Sharing row
  with Stop.
- **Smaller gaps**: no click on the countdown to cancel it (Cmd+Ctrl+Escape
  does), square handles, no swatch beside the colour value, no camera
  pointer in window mode, the line square round a lifted window, shell panels
  not pickable apart from the region, no Show mouse clicks, no Portion of
  screen tab or typing filter in the picker, a request made while locked
  answered cancel, no countdown for an app's shot.
- **The thumbnail's click opens the default image viewer** until Quick Look
  (P15).

Found and fixed on the way: the allocator kept every still's pixels for the
worker threads that freed them, 135 MB after five shots; the audio streams'
ports configure themselves with no session manager
(`adapter.auto-port-config`); a callback on PipeWire's data thread
(`RT_PROCESS`) raced the recorder's main loop. Found and left for P14: a
user's `~/.config/xdg-desktop-portal/portals.conf` outranks the packaged
`isle-portals.conf`.

The measures and the VM are in `docs/research/rust-desktop/p11-capture.md`:
a shot is on disk in about 55 ms, the freeze takes a few milliseconds, a
3440 by 1440 stream costs 1.7 ms of the loop a frame at p50 without slowing
the output, a still screen streams nothing, and the compositor's own heap
stays near 30 MB. The VM shot by CLI and by the pick's keys, recorded, ran
grim, and shared a screen through the guest's real frontend and Isle's
picker to a consumer. Not checked: Chromium and Firefox sharing, OBS,
Discord, NVENC and dmabufs on the host, a game recorded. B80–B85 record the
decisions.

