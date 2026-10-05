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

## The VM (2026-10-05)

The guest's session on llvmpipe at 2560×1080, with kitty, gtk4-demo, Qt 6's Designer,
xterm and xeyes started from the session's environment:

- kitty and Designer negotiate server-side decorations and wear Isle's bar; Designer's
  New Form dialog opens centred over it with its own bar; Designer opens filled because
  it asked to be maximised before it mapped.
- GTK Demo keeps its own header bar and shadow; Isle adds neither and leaves its corners.
- xterm and xeyes, through Xwayland, wear Isle's bar; xeyes' shaped window shows what is
  behind its eyes.
- `isle windows tile` puts kitty in the left third and xterm in the right; minimised
  Designer windows leave the screen.

Found there, not headless: the supervisor waited forever for the ready pipe to close,
because Xwayland, started before the report, inherited its write end. The compositor now
marks that fd close-on-exec as it starts, so no child holds it.

## Shaped X11 windows (2026-10-05)

xeyes wore Isle's bar and shadow over a body that was mostly see-through: it
sets a bounding shape (SHAPE), which Smithay's window manager did not track.
With the fork's patch it wears neither. It then showed black outside the
eyes: Xwayland's buffer is transparent there, but the surface was drawn as
opaque, so nothing behind it was drawn. Isle now draws shaped windows
blended. What lies outside the shape is the client's buffer: xeyes clears
it, the headless test client's stays black, so the headless test checks only
that Isle adds no bar or shadow and the VM shows the see-through.

