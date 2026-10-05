# P4 · Windows: measurements and findings

## A title bar per window

Each decorated window has its own Freya tree, an attached surface (B37's third scope).
`tests/bench.rs::title_bar_cost` opens thirty 600×400 clients with and without Isle's bar,
headless on the 5070 Ti (2026-10-05):

| | Heap | Own memory | Skia cache | VRAM |
|---|---|---|---|---|
| 30 windows, no bars | +1.2 MB | 22.2 MB | 0 | 59 MiB |
| 30 windows, bars | +9.1 MB | 29.8 MB | 6.6 MB | 73 MiB |

So a bar costs about 0.27 MB of heap and 0.47 MiB of VRAM, the VRAM mostly its texture
and Skia's start on first use. The atlas the plan held in reserve is not needed; the
budget in `shell.md` is 0.5 MB and 0.5 MiB a bar.
