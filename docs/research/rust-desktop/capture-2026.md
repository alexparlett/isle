# Capture and portals for Isle: research note (2026-10-09)

Scope: what P11 needs underneath it: the portal backend's interfaces, the
PipeWire streams the compositor produces, recording, the standard capture
protocols, the freeze and the colour pick, and how the apps that share a
screen behave. "Local:" marks evidence gathered on this host today, in
installed files or in shallow clones (niri master `2c82b77` of 2026-10-08,
xdg-desktop-portal-cosmic `9d35e63` of 2026-10-07, Otto `36f7748` of
2026-10-09, xdg-desktop-portal tag 1.22.1); everything else carries a URL.
*(unverified)* marks recollection not re-checked in this pass.

---

## §1 The portal backend's interfaces (xdg-desktop-portal 1.22.1)

Local: the interface XML is in `/usr/share/dbus-1/interfaces/`. NEWS:
1.21.0 (2026-01-21) added `reduced-motion`; 1.21.2 (2026-05-06) Screenshot's
`target` and ScreenCast's `pipewire-serial`; 1.22.1 (2026-06-17) turned off
PipeWire's module-rt in the frontend "to prevent deadlocks" (#2012).
https://github.com/flatpak/xdg-desktop-portal/blob/main/NEWS.md

| impl interface | version | shape |
|---|---|---|
| ScreenCast | 6 | `CreateSession(o handle, o session, s app_id, a{sv}) → (u, a{sv}{session_id})`; `SelectSources(o, o, s, a{sv})`; `Start(o, o, s app_id, s parent_window, a{sv})`; properties `AvailableSourceTypes` (1 monitor, 2 window, 4 virtual), `AvailableCursorModes` (1 hidden, 2 embedded, 4 metadata), `version` |
| SelectSources options | | `types u`, `multiple b`, `cursor_mode u` (v2; a mode not advertised closes the session), `restore_data (suv)` and `persist_mode u` (0 none, 1 while the app runs, 2 until revoked; v4) |
| Start results | | `streams a(ua{sv})`: node id with `position (ii)` (monitors), `size (ii)` in logical coordinates, `source_type u` (v3), `mapping_id s` (v5), `pipewire-serial t` (v6; node ids are reused, so the serial is what consumers target); `persist_mode`, `restore_data` |
| Session | | `Close()`, signal `Closed`, property `version` |
| Request | | `Close()`, exported at the handle while the interaction lasts |
| Screenshot | 3 | `Screenshot(o, s app_id, s parent_window, a{sv}) → uri`, options `modal`, `interactive`, `target u` (v3: 1 screen, 2 window, 4 area, 8 active window; one value), `permission_store_checked b` (v2); `PickColor(o, s, s, a{sv}) → color (ddd)`, sRGB in [0, 1]; property `AvailableTargets` (v3) |
| Settings | frontend v2 | `ReadAll(as) → a{sa{sv}}`, `Read(s, s) → v`, signal `SettingChanged`. `org.freedesktop.appearance`: `color-scheme u` (0, 1 dark, 2 light), `accent-color (ddd)` sRGB, `contrast u` (0, 1 high), `reduced-motion u` (0, 1; 1.21.0). The frontend merges every configured Settings impl in order (`settings.c`) |
| Inhibit | | `Inhibit(o handle, s app_id, s window, u flags, a{sv}{reason})`, flags 1 logout, 2 user switch, 4 suspend, 8 idle, ended by `Request.Close`; `CreateMonitor(o, o session, s, s window) → u`, signal `StateChanged(o, a{sv}{screensaver-active b, session-state u: 1 running, 2 query-end, 3 ending})`, `QueryEndResponse(o)` within a second |
| Access | | `AccessDialog(o, s app_id, s parent_window, s title, s subtitle, s body, a{sv}) → (u, a{sv})`; the frontend raises it for permissions it checks itself |

### §1.1 Choosing the backend

The frontend reads `xdg-desktop-portal/<desktop>-portals.conf`, then
`portals.conf`, for each name in `XDG_CURRENT_DESKTOP` lowercased, from
`$XDG_CONFIG_HOME` through `/usr/share`. `[preferred]` takes `default=` and
per-interface keys (`org.freedesktop.impl.portal.ScreenCast=isle`), lists
split by `;`. `UseIn=` in a `.portal` file is a deprecated fallback; Local:
`xdp-portal-config.c:910` warns when it is used.
https://flatpak.github.io/xdg-desktop-portal/docs/portals.conf.html

- Local: `isle-session` sets `XDG_CURRENT_DESKTOP=Isle`, so the package ships
  `isle-portals.conf`. v1's `system/portal/isle.portal` claims the same bus
  name with `UseIn=Hyprland` and FileChooser only.
- Local: **the Screenshot portal does not start without an Access impl**
  (`init_screenshot`: "The screenshot portal requires an access impl").
  gtk's Access is `UseIn=gnome` only.
- Local: **the frontend reads an impl's properties once, at its own start.**
  `screen-cast.c:1138-1146` takes MIN(impl `version`, 6) and binds
  `available-cursor-modes` only if the version is 2 or more then;
  `screenshot.c:617` likewise. There is no activation file for
  `.desktop.isle` (Local: `/usr/share/dbus-1/services`), so if the frontend
  starts before Isle's worker owns the name, cursor modes stay 0 and every
  `cursor_mode` is refused until the frontend restarts.
- The property is exported as lower-case `version`; zbus's default
  `Version` reads as 0 and silently drops everything after v1 (Otto
  `docs/developer/screenshare.md`; COSMIC `src/screencast.rs:495`).

### §1.2 Who is asking

- Local: for unsandboxed apps `xdp-app-info-host.c` takes the app id from the
  caller's systemd user unit (`sd_pid_get_user_unit`, regex
  `^app-(?:[[:alnum:]]+\-)?(.+?)(?:\-[[:alnum:]]*)(?:\.scope|\.slice)$`), kept
  only if `<id>.desktop` exists. Isle launches into
  `app-isle-<name>-<hex>.scope` (`isle-compositor/src/launch.rs:105`), so apps
  it starts arrive with their id; one started from a terminal arrives with
  `""`. `org.freedesktop.host.portal.Registry.Register` lets an app name
  itself.
- The backend's caller is the frontend; it never sees the app's own name.
  COSMIC and Otto use `app_id` for display only and ignore `parent_window`
  (Local: xdpc `src/access.rs`, `src/screenshot.rs`).
- `parent_window` is `wayland:<handle>` (an xdg-foreign v2 export),
  `x11:<hex xid>` or `""`. Local: Smithay at the fork's `764ad1b` keeps
  `XdgForeignState.exported` private with no lookup; a compositor-side
  backend needs a `surface_for_handle` accessor in the fork. GTK 4 and Qt 6
  export through v2; GTK 3 through v1, which Smithay does not serve
  *(unverified)*.

### §1.3 The PipeWire remote

Local: `screen-cast.c:702-748` (1.22.1): the frontend opens its own remote
with `pipewire.access.portal.app_id`, grants Core RWX, NodeFactory R and
each stream node RWX by the ids in Start's results, then `PW_ID_ANY` 0, and
hands the app the fd. The backend returns the right ids and serials and
keeps the nodes alive; its own PipeWire connection needs no restriction.

## §2 Prior art

| project | shape | files |
|---|---|---|
| niri | Serves Mutter's `org.gnome.Mutter.ScreenCast` and lets xdg-desktop-portal-gnome be the backend (feature `xdp-gnome-screencast`); the PipeWire producer is in the compositor | `src/screencasting/pw_utils.rs` (1,953 lines), `src/screencasting/mod.rs`, `src/dbus/mutter_screen_cast.rs`, `src/dbus/gnome_shell_screenshot.rs` (PickColor), `src/handlers/image_copy_capture.rs`, `src/protocols/screencopy.rs`, `src/ui/screenshot_ui.rs`, `src/input/pick_color_grab.rs` |
| xdg-desktop-portal-cosmic | A separate process: captures with ext-image-copy-capture from cosmic-comp and republishes into PipeWire on its own thread; zbus 5 on tokio, ashpd 0.12 for types | `src/main.rs` (Request and Session with `AbortHandle`), `src/screencast.rs` (restore data), `src/screencast_thread.rs`, `src/screenshot.rs`, `src/widget/{screenshot,rectangle_selection,output_selection}.rs`, `data/cosmic-portals.conf` |
| Otto (nongio/otto; Smithay and Skia, macOS-like) | A separate `xdg-desktop-portal-otto` talks to the compositor's private `org.otto.ScreenCast`; zbus bridged into calloop by a channel; PipeWire on its own thread | `src/screenshare/{pipewire_stream.rs,dbus_service.rs}`, `components/xdg-desktop-portal-otto/src/portal/{interface.rs,restore.rs,request.rs,session.rs,access.rs}`, `specs/screenshare.md` |

`ashpd` 0.13.13 (2026-07-17) has an unstable `backend` module with impl
traits for ScreenCast, Screenshot, Settings and Access among others, and
none for Inhibit; whether it carries `pipewire-serial` was not checked.
https://docs.rs/ashpd/0.13.13/ashpd/backend/ . niri and COSMIC write the
zbus interfaces by hand: Request and Session are about 150 lines.

### §2.1 niri's producer (Local)

- **Loop.** The PipeWire `MainLoopRc` fd is a calloop `Generic` source
  (level-triggered) that calls `iterate(Timeout::None)`: no PipeWire thread.
  Its context is made with no properties, so module-rt loads.
- **Stream.** `DRIVER | ALLOC_BUFFERS`, 2 to 16 buffers, 8 by default,
  blocks equal to the plane count.
- **Formats.** BGRA then BGRx with modifiers from
  `egl.dmabuf_render_formats()`. A modifier offered with `DONT_FIXATE` is
  test-allocated through GBM, then the params are sent again with it fixed;
  with no modifier, sealed memfd shared memory.
- **Meta.** `SPA_META_Header` on every queued buffer (seq, flags cleared of
  CORRUPTED, pts −1). `SPA_META_Cursor` in metadata mode, sized for a
  384 × 384 bitmap, written on **every** dequeued buffer because each buffer
  has its own meta memory; the bitmap only when the image changes. A hidden
  cursor is a 1 × 1 transparent bitmap: OBS checks `size != 0` and libwebrtc
  takes 0 as "keep the last". A dmabuf's data `size` is 1 for OBS.
- **Damage and pacing.** A damage tracker per cast; a frame with no damage
  and no cursor change is skipped. Frames are spaced by the negotiated
  maximum rate with a calloop timer (100 µs allowance).
- **Sync.** After rendering, a fence fd is waited on in calloop before the
  buffer is queued: the fix for NVIDIA's cast flicker, after which the old
  debug flag went (25.08). Cursor meta needs PipeWire 1.4.8 or later; Local
  1.6.9.

## §3 pipewire-rs

- `pipewire` 0.10.1 (2026-08-19); Local: Isle already depends on it
  (`isle-audio`, the camera reader).
- A producer: `StreamRc::new` → listener (`param_changed`, `add_buffer`,
  `remove_buffer`, `state_changed`, `process`) → `connect(Direction::Output,
  None, DRIVER | ALLOC_BUFFERS, [EnumFormat pods])` → on `param_changed`,
  `update_params([Buffers{dataType}, Meta Header, Meta Cursor])`. In
  `add_buffer`, a GBM bo per buffer filling each data's `DmaBuf` type, fd,
  offset and stride. Per frame: `dequeue_raw_buffer`, render, header, chunk
  size, cursor meta, `queue_raw_buffer`.
- `pipewire-serial` is the node's `object.serial` from its info, not
  `stream.node_id()`.
- **libdbus (B11).** Local: `client.conf` loads `libpipewire-module-rt`
  unless `module.rt = false`; it and `libspa-dbus.so` link
  `libdbus-1.so.3`. Isle's camera reader passes `module.rt = false` and
  `support.dbus = false` (`isle-audio/src/camera.rs:70-76`), and
  `tests/audio.rs:182` asserts neither is mapped. module-rt also asks the
  Realtime portal for priority, which a compositor that is a portal backend
  could deadlock on, as the frontend did before 1.22.1.

## §4 Recording

Local inventory: ffmpeg 9.0.2; GStreamer 1.28.7 with -base, -bad, -ugly
and gst-libav; gst-plugin-pipewire 1.6.9; x264, x265; libva-nvidia-driver
(decode only); nvidia-utils 615.71.09 on the **open** kernel module.

- **gst-plugins-good is not installed**: no `mp4mux`, `qtmux` or
  `matroskamux`; `avmux_mp4` is marked not recommended by GStreamer.
- ffmpeg encoders: `h264_nvenc`, `hevc_nvenc`, `av1_nvenc`, `libx264`,
  `libx265`, `libsvtav1`, `h264_vulkan`, `hevc_vulkan`, `av1_vulkan`,
  `*_vaapi`; audio `aac`, `libopus`.
- GStreamer: `nvh264enc`, `nvh265enc`, `nvav1enc`, `x264enc`,
  `openh264enc`, `vulkanh264enc`, `fdkaacenc`, `avenc_aac`, `opusenc`,
  `pipewiresrc`; no `vah264enc`.
- Local: **NVENC works on this Blackwell card under nvidia-open**: H.264,
  HEVC and AV1 through ffmpeg at 3440 × 1440 60 fps, and `nvh264enc` in a
  GStreamer pipeline.
- Local: ffmpeg 9.0.2 has no PipeWire input device (`kmsgrab`, `x11grab`,
  `pulse` only). `libavdevice.so.63` links libpulse and so libdbus;
  avcodec, avformat, avutil, avfilter, swscale, libgstreamer,
  libgstnvcodec and libgstpipewire do not, but `pipewiresrc` loads
  module-rt at run time through the default `client.conf`.

| option | for Isle |
|---|---|
| GStreamer in the compositor (`pipewiresrc` → `nvh264enc` → `mp4mux`) | Least code; CUDA, NVENC and GStreamer's threads inside the compositor, module-rt through `pipewiresrc`, and gst-plugins-good a new dependency |
| `ffmpeg-next` 9.0.0 (2026-08-05, FFmpeg 9) | Full control (`h264_nvenc`, `+faststart` or fragments); built without the `device` feature, libavdevice and libdbus stay out; the frames come from pipewire-rs. A dmabuf into the encoder needs interop (CUDA, or `hwmap` to Vulkan and `h264_vulkan`); shared memory is simple at 19.8 MB a frame at 3440 × 1440 |
| An external recorder | wf-recorder 0.6.0 speaks only wlr-screencopy (Local: `strings`). gpu-screen-recorder's `-w portal` goes through the portal and so through Isle's picker; not installed. https://man.archlinux.org/man/gpu-screen-recorder.1.en |
| A helper process of Isle's own reading Isle's node by serial | Crash isolation, no CUDA in the compositor, no portal for Isle's own tool; inside it, one of the first two |

- **GNOME Shell** records in a separate D-Bus service with GStreamer:
  `pipewiresrc path=<node> do-timestamp=true keepalive-time=1000
  resend-last=true`, encoders tried in order (`vah264lpenc`, `vah264enc`,
  `openh264enc`) into `mp4mux` with fragment mode
  `first-moov-then-finalise`, so a cancelled file plays; no audio.
  https://gitlab.gnome.org/GNOME/gnome-shell/-/raw/main/js/dbusServices/screencast/screencastService.js
- **Spectacle** records through KPipeWire with libav; on NVIDIA VAAPI has no
  encoder, so it falls back to x264. https://bugs.kde.org/show_bug.cgi?id=519941
- Audio in the helper: PipeWire capture streams on the default sink's
  monitor (`stream.capture.sink = true`) and the chosen source, mixed, as
  AAC-LC at 48 kHz; Opus in MP4 does not play in QuickTime.
- **For a file a Mac user can share**: H.264 High, 4:2:0, MP4, AAC. HEVC
  plays in QuickTime only tagged `hvc1` *(unverified)*; AV1 decodes in
  hardware on Apple silicon from M3 *(unverified)*. 3440 × 1440 at 60 fps is
  about 1.16 M macroblocks a second, past H.264 level 5.1's 983,040, so
  level 5.2; at 30 fps 5.1 holds.

## §5 ext-image-copy-capture in Smithay

- Merged as PR #1902 (2026-02-03), from cosmic-comp's screencopy.
  https://github.com/Smithay/smithay/pull/1902 . Local: the fork at
  `764ad1b` has `wayland/image_capture_source` (`OutputCaptureSourceHandler`,
  `ToplevelCaptureSourceHandler`), `wayland/image_copy_capture` and
  `wayland/foreign_toplevel_list`; Isle serves none of them yet.
- `ImageCopyCaptureHandler`: `capture_constraints(&source) →
  Option<BufferConstraints{size, shm, dma: Option<DmabufConstraints{node,
  formats}>}>`, `new_session`, `frame(&SessionRef, Frame)`,
  `new_cursor_session`, `cursor_frame`, `cursor_capture_constraints`,
  `frame_aborted`, `session_destroyed`. `SessionRef::update_constraints()` on
  a resize or scale change; `CursorSessionRef::set_cursor_pos` and
  `set_cursor_hotspot`. `Frame::success(transform, damage, presented)` or
  `fail(reason)`; a dropped frame sends Unknown, a dropped session `stopped`.
  Buffers are checked against the constraints before `frame()`.
- niri (`src/handlers/image_copy_capture.rs`): holds the frame until the
  output has damage since the last capture; renders at an exact size match;
  offers shm `Xrgb8888` (cursor `Argb8888`) and every render format for
  dmabuf with `Xrgb8888` and `Argb8888` first, since wl-mirror and grim take
  the first; takes the DRM node from the backend (Mesa 23.3–26.1.8 loses
  the EGLDevice when a second EGLDisplay is made, MR 44351). Output sources
  and cursor sessions only, with wlr-screencopy beside them.
- Clients, Local (`strings`): grim 1.5.0 speaks ext-image-copy-capture with
  toplevel sources and cursor sessions, and wlr-screencopy; wf-recorder 0.6.0
  and hyprpicker 0.4.7 speak only wlr-screencopy; slurp is layer-shell only.
  xdg-desktop-portal-wlr 0.8 speaks ext-image-copy-capture.
  https://github.com/emersion/xdg-desktop-portal-wlr/releases
- The only installed tools that need wlr-screencopy are the two Isle's own
  recorder and colour pick replace.

## §6 A window alone

- niri renders `bbox_with_popups()` at the output's scale (surface tree,
  subsurfaces, popups, the pointer moved into the window's space) and
  renegotiates on a size change; it renders casts for every window on the
  output, workspaces included, and its own FIXME says hidden windows get
  frame callbacks "by chance" (niri.rs ≈4856).
- Otto renders the surface tree from the geometry's origin, cropping
  client-side shadows, rounds the size down to even, follows resizes after
  250 ms, and keeps streaming while occluded, minimised or on another
  workspace by pinning the window's frame callbacks to full rate.
- COSMIC persists toplevels by their ext-foreign-toplevel identifiers,
  which die with the window.

## §7 The freeze and the shot

- niri (`niri.rs:2090`): renders every output into Abgr8888 textures, the
  pointer separately, then opens its UI over them; the pointer is
  composited into the shot only when asked. Space or Return captures; held
  Space moves the selection; arrows nudge; a selection stays on one output.
  No window mode in that UI. Saving: `copy_texture` and `map_texture` on
  the loop, PNG on a thread, the bytes to the clipboard through
  `set_data_device_selection`.
- COSMIC captures every output and toplevel over shm before showing its
  layer-shell UI; rectangle, output and window; it remembers the last
  rectangle; PNG in `spawn_blocking`; a non-interactive call is answered
  without asking, trusting the frontend's check.
- Local, PNG with `png` 0.18.1 in release, one thread, 3440 × 1440 RGBA
  (tiled from a screenshot, so kind to the compressor):

  | compression | time | size |
  |---|---|---|
  | Fastest | 4.6 ms | 783 KiB |
  | Fast | 8.1 ms | 554 KiB |
  | Balanced | 55 ms | 348 KiB |
  | High | 130 ms | 342 KiB |

## §8 Colour pick

niri renders a 1 × 1 region at the pointer and downloads it; COSMIC
answers PickColor with an error; hyprpicker reads a wlr-screencopy frame.
The frontend documents the answer as sRGB in [0, 1]; Isle's frames are
sRGB-encoded 8-bit offscreen renders, so the values pass straight through
until HDR (P22) brings a wider space.

## §9 Apps that share a screen

| client | behaviour |
|---|---|
| Chromium, Chrome (Local: chromium 153) | PipeWire capturer on by default since 110. Sends the request's `types`, `multiple=false`, `cursor_mode` metadata by default (embedded with `prefer_cursor_embedded`, 2026-01-09: https://webrtc.googlesource.com/src/+/92db68165cf531b4076a944f82ddd8a737fa990b), `persist_mode` and a reusable `restore_token` from v4. **Its picker opens one session for a preview and a second to capture**, so the dialog shows twice unless the second is restored (Otto `specs/screenshare.md`). Tab audio is Chrome's own |
| Firefox | The same libwebrtc portal code |
| Electron, VS Code | `desktopCapturer.getSources()` returns one source on PipeWire. Discord, teams-for-linux and Slack show the portal's dialog, then their own picker with one entry. https://electronjs.org/docs/latest/api/desktop-capturer |
| Discord | Wayland sharing with audio since January 2025, audio from Pulse apps only, x264. https://www.gamingonlinux.com/2025/01/discord-screen-sharing-with-audio-on-linux-wayland-is-officially-here |
| Zoom | Still unreliable on Wayland; `enableWaylandShare=true`, and some setups need `XDG_CURRENT_DESKTOP=gnome`. https://community.zoom.com/meetings-2/share-screen-linux-wayland-broken-9439 |
| OBS | Metadata cursor, else embedded; explicit sync (`SPA_META_SyncTimeline`) since 31.1; drops to LINEAR or shm when a modifier fails |

**NVIDIA and dmabuf.** niri #2223 (closed 2026-09-19): black casts in OBS,
WebRTC and Vesktop when the consumer imported on another GPU, since
"linear images are external only on nvidia".
https://github.com/niri-wm/niri/issues/2223 . Local: this machine is
hybrid: the AMD iGPU is `renderD128` and the NVIDIA card `renderD129`, the
AOC on the NVIDIA card's DP-4; a consumer taking the first render node
imports on AMD. Hence: modifiers that pass a test allocation, LINEAR among
them, one fixed-modifier format pod each with LINEAR first (Otto; GStreamer's
DMA_DRM consumers), and the shared-memory fallback kept.

## §10 Security

- The session bus enforces nothing, so any process can call
  `org.freedesktop.impl.portal.*` and skip the frontend's checks. Checking
  that the sender is the owner of `org.freedesktop.portal.Desktop`,
  resolved on each call since the frontend restarts, closes that. COSMIC
  and Otto do not check (Local). GNOME Shell allow-lists the callers of its
  own screenshot interface since 41.
- **Non-interactive screenshots** (Local: `screenshot.c`). For impl v2 and
  later the frontend checks the permission store's `screenshot` table
  itself: yes passes, no denies, unset or ask raises the **Access** dialog
  and stores the answer; for host apps with no app id one answer covers all
  of them. It then calls the impl with `permission_store_checked=true`.
- **Restore tokens** (Local: `xdp-session-persistence.c`): the frontend
  makes the token and the backend never sees it, only its `(suv)` data.
  Mode 1 keeps it in memory under `<sender>/<token>`; mode 2 in the
  permission store's `screencast` table. Vendors tag the data: COSMIC
  `("COSMIC", 1, (connectors, toplevel identifiers))`, Otto `("otto", 1,
  {source-type, id})`.
- Inhibit's permission table is `inhibit`.
