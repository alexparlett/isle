# P7: the switcher's measures and what the VM found

## Measures

`cargo test --release -p isle-compositor --features test-hooks --test bench
switcher_cost -- --ignored --nocapture`: twenty test apps on a 3440 by 1440
output, a third of them 3440 by 1400 and the rest 1600 by 900, the switcher
held five times while every app redraws at about 60 Hz. Draws are timed with
`glFinish` after each. Two runs:

| | Run 1 | Run 2 |
|---|---|---|
| Panel draws | 25, p50 0.52 ms, p95 1.43 ms, max 4.07 ms | 25, p50 0.51 ms, p95 1.42 ms, max 4.29 ms |
| Preview passes | 25, p50 7.47 ms, p95 14.74 ms | 25, p50 9.50 ms, p95 14.63 ms |
| RSS at rest, held, after | 273, 268, 270 MB | 273, 265, 268 MB |
| Own memory (anonymous) | 34 to 37 MB | 34 to 38 MB |
| VRAM at rest, held, after | 264, 222, 209 MiB | 264, 222, 209 MiB |

- **A preview from a texture already uploaded costs about 0.25 ms**: the first
  pass, twenty previews of windows on screen, took 4.9 ms.
- **The rest of a pass is the shm upload** of the test clients' buffers, up to
  19 MB each, which the next frame would otherwise pay for the windows on
  screen. GPU clients (dmabuf) have no upload. A pass draws again at most four
  windows that drew since, ten times a second; before that limit twenty
  redraws made a pass of 45 to 68 ms.
- **The test client never destroyed its buffers**, so RSS counted every pool
  it had mapped: 6.2 GB after this bench. It now destroys a buffer when the
  compositor releases it; the compositor held nothing of its own.
- **The panel adds no VRAM while held**: its texture and the previews (Skia's,
  budgeted, purged when the surface closes) are inside the noise of the
  twenty windows' own textures.

### With icons (B59)

The same bench with the window row open, since only it has previews now:
the panel draws at p50 0.48 ms, p95 1.44 ms; a preview pass at p50 1.69 ms,
p95 5.62 ms (one window's preview, most of the pass its shm upload);
RSS 271 MB held, own memory 37 MB, VRAM no higher than at rest. A Cmd+Tab
that never opens the window row renders no preview at all.

## The VM

In the guest (llvmpipe, 2560 by 1080, a PC keyboard so Cmd is the Alt key):
kitty, xterm on Xwayland, Chromium on Wayland, Qt Designer (two windows, one
minimised), GTK 4's demo hidden, and a test client running in the background
with its window closed. Held, the switcher listed every app in recency order
with live previews, Papirus icons, the hidden mark on GTK's dimmed preview,
and the background app after the divider; releasing Alt brought xterm
forward. `isle apps list` showed the same states.
