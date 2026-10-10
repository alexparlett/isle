# P12: gaming's measures, the VM, and the host run

## Measures

`cargo test --release -p isle-compositor --features test-hooks --test bench
gaming_cost -- --ignored --nocapture`, headless on the 5070 Ti: a 3440 by 1440
output, a full-screen game window (the test client, flagged by
`windows.games`) drawing a whole 3440 by 1440 shared-memory buffer on every
frame callback. Three runs.

| | Runs |
|---|---|
| Shell surface draws in 5 s over the game | 0, 0, 0 |
| Output frame interval (headless timer, 60 Hz mode) | p50 18.2–18.3 ms, p99 19.0–19.3 ms |
| A key, taken by the loop to read by the game, the game drawing every frame | p50 0.03 ms, p99 2.1–2.3 ms |
| The same, the game still | p50 0.04 ms, p99 0.09 ms, max 0.09 ms |
| RSS and the compositor's own heap with the game in front | 162–164 MB, 29–30 MB |
| VRAM | 76 MiB |

- **Nothing of the shell draws over a game** (acceptance bar 4): the island
  and the bar are hidden over a full-screen desktop and draw no frames.
- **A key reaches a game in under 0.1 ms of the loop taking it** (bar 5's
  compositor part): with the game still, p99 0.09 ms. While the test client
  draws every frame its p99 is about 2.2 ms, which is the client filling a
  20 MB buffer before it reads its socket, not the compositor.
- **Found and fixed: input waited for the frame.** The loop flushed clients
  once, after drawing, so a key taken in the iteration that drew a frame
  waited for the frame: p99 3.7 ms with the game drawing. Clients are now
  flushed before the redraw as well.
- The headless output paces by a timer after each frame, so its intervals
  are the mode's 16.7 ms plus the frame's own cost; VRR, tearing and
  scanout have no headless measure (the plan's choice 20).

## The VM

- virtio-gpu: the trace runs on real DRM with hardware timestamps (75 Hz,
  13.33 ms apart, flags 7). It offers no dmabufs, so no buffer is scanned
  out there. No VRR (capability `none`; its EDID gives a 50 Hz minimum). No
  gamma ramp, so night light reports `none`.
- With the scanout check taken out for one run, virtio-gpu flipped 40
  frames asynchronously with no DRM error; with it, composited frames that
  asked to tear stayed on vsync.
- Xwayland 24.1 binds `wp_linux_drm_syncobj_manager_v1`.
- A full-screen window is drawn every 66.7 ms where a windowed one is drawn
  every 13.3 ms, the same with 12.0's build: llvmpipe compositing the whole
  2560 by 1080 screen, not P12's. The host's measure is the one that counts.
- Nested gamescope was not run in the VM (it has none); it runs nested on a
  headless Isle on the host's 5070 Ti in the tests.

## The host run (tty8, the user's)

Not yet run. What it is for, by acceptance bar:

1. A Proton game and a native Vulkan game full screen for ten minutes:
   `isle displays trace DP-4 --reset`, then `isle displays trace DP-4`
   shows every frame `window <id> <app>` (scanned out), none `composited`.
2. With `output[...].tearing true` and the game's vsync off, frames marked
   `async` and a visible tear in a test pattern; with tearing off, none
   async.
3. With VRR at Full screen, the trace's intervals follow a game holding a
   steady rate between 48 and 144 fps, `vrr` on while it is in front and
   off when it leaves. `isle displays` shows the AOC at 144 Hz with VRR
   `on-demand`.
5. A key reaches the game within a millisecond with the GPU at full load
   (the headless measure stands in; a game is the check).
6. An hour of play: no "main loop stalled" in `session.log`.
7. No blank screen entering, leaving or Cmd+Tabbing out of a full-screen
   game, VRR on and off.
9. Every context menu in Steam's Library, Store and Friends opens at the
   pointer and stays open.
10. Big Picture opened and closed twenty times: no stray window, the view
    back each time, Steam's window back on its desktop. Its title and
    whether it asks for full screen are noted here.
11. Nested gamescope: `gamescope -- vkcube` reports the output's rate.
- Night light on the AOC: `isle night-light toggle` warms the screen, the
  trace still shows a game scanned out, a screenshot is untinted, and
  leaving to a VTY gives the console its colours back.
