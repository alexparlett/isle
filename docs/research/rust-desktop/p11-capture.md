# P11: capture's measures, and what the VM found

## Measures

`cargo test --release -p isle-compositor --features test-hooks --test bench
capture_cost -- --ignored --nocapture`, headless on the 5070 Ti: a 3440 by 1440
output over the frost wallpaper, a 1600 by 900 window, the private PipeWire of
the tests. Two runs.

| | Runs |
|---|---|
| Shift+Cmd+3 to the PNG on disk | 54–60 ms |
| Shift+Cmd+4 to the frozen pick, one window | 0.9–6.5 ms |
| Shift+Cmd+4 to the frozen pick, eight windows | 2.0–3.0 ms |
| A screen stream's frame, rendered and read back on the loop (shared memory, 254 frames) | p50 1.71 ms, p95 3.10–3.13 ms, max 4.9 ms |
| Output frames a second, a client drawing every frame: alone, streaming, recording | 52.7–54.3, 50.8, 50.2–51.2 |
| Compositor CPU, streaming or recording | 16–17% |
| `isle-record` CPU and RSS, recording 3440 by 1440 through NVENC | 22–25%, 669 MB |
| Still for 5 s with a stream open | no stream frames |
| RSS at rest, after five shots, frozen with eight windows, streaming, recording | 151, 176, 198, 362, 371 MB |
| The compositor's own heap (anonymous) through all of it | 28–36 MB |
| VRAM at rest, after five shots, frozen with one and eight windows, streaming | 92, 135, 183, 239, 122 MiB |

- **A shot is on disk in about 55 ms**: the render and readback on the loop,
  the PNG at `png`'s Fast setting on the worker, the file written before the
  thumbnail shows.
- **The freeze is a few milliseconds** even with eight windows held whole.
  Its textures are VRAM while the pick is open: 239 MiB with eight windows,
  over the shell's 200 MiB budget for as long as the pick is open, and back
  when it closes.
- **A screen stream costs under 2 ms of the loop a frame at p50**, nearly all
  of it the readback into shared memory, and the output keeps its frame rate
  beside a client drawing every frame. The dmabuf path, which skips the
  readback, is the udev backend's and was not measured headless.
- **A still screen streams nothing**: frames come only with damage.
- **Found and fixed: the heap kept every shot.** Five shots left the
  compositor's own memory 135 MB higher, the allocator keeping the freed
  pixels for the worker threads that freed them; `malloc_trim` after a still
  is written and when the pick closes keeps it at about 30 MB. RSS still
  counts the stream's eight shared-memory buffers (about 160 MB at 3440 by
  1440) and the GPU's mappings.
- **The recorder** is about a quarter of a core and 669 MB resident at
  3440 by 1440, most of it its own mapping of the stream's buffers and
  NVENC's surfaces; it is a process of its own and leaves the compositor's
  budgets alone.

## The VM

In the guest (llvmpipe, 2560 by 1080), the PC keyboard's Alt as Cmd, the
workspace's debug build from `/repo`:

- **`isle capture screen`** wrote `Screenshot 2026-10-09 at 18.21.42.png`
  into `~/Pictures/Screenshots`.
- **Shift+Cmd+4, Return, Shift+arrows, Return** drew the veil, the cut, the
  two-tone line and "300 × 150" over the frozen frame, and wrote a 300 by 150
  file.
- **`isle capture record start`, then `stop`** wrote a 3.4 s H.264 recording
  at 2560 by 1080 (x264: the guest has no NVENC).
- **grim** took the screen through ext-image-copy-capture.
- **The portal, through the guest's real xdg-desktop-portal**: a ScreenCast
  client (GLib over D-Bus) opened Isle's picker with the screen's live
  preview; choosing it and Share answered Start with node 62, its
  `pipewire-serial` 76, the screen's place and size, and the frontend's
  restore token; `isle-test-consumer` linked to the node and streamed
  2560 by 1080 in shared memory.
- **The sheet, through the real frontend**, after the leftovers were built:
  a GTK 4 window exported its handle over xdg-foreign and asked ScreenCast
  with `wayland:` that handle and the metadata cursor mode; Isle's picker
  hung on that window over it dimmed, the arrows and Space chose the screen
  from the sheet's keyboard, and Return answered Start with node 62.
  `isle-test-consumer` linked to it and printed the cursor's place at each
  pointer move and one 128 by 128 bitmap of the themed arrow, over two
  picture frames: the moves over a still screen sent the cursor alone.
- **Found: a user's `portals.conf` outranks the packaged
  `isle-portals.conf`.** The frontend looks in `~/.config` for
  `isle-portals.conf`, then `portals.conf`, before the system's folders; the
  guest's v1-era `~/.config/xdg-desktop-portal/portals.conf` (Hyprland's
  backend first) won, and ScreenCast read version 0 until
  `isle-portals.conf` was put in `~/.config` too. The host has v1's file as
  well: P14's install must move it aside or write Isle's beside it.
- **Not checked: Chromium and Firefox sharing.** Chromium's own chooser
  offered only its tabs; its screen side never reached the portal in this
  session, which the host run will see with a real Chromium and Firefox.
  OBS, Discord and the hybrid GPU's dmabufs are the host's too.
