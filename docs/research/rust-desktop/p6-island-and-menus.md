# P6 · Island, menus and wallpaper: findings

Measurements and observations from building P6 (`docs/work/rust-desktop/p6-island-and-menus.md`).

## Cost at 3440×1440

`tests/bench.rs`'s `island_and_menu_bar_cost`, headless on the host's RTX 5070 Ti with
one 3440×1440 output and a test client exporting a DBusMenu tree: twenty Control Centre
opens and closes (Ctrl+F8, Escape), then ten menu bar reveals with the system menu open
(Ctrl+F2, Escape, Escape).

```
cargo test --release -p isle-compositor --features test-hooks --test bench island_and_menu -- --ignored --nocapture
```

Draw times, every draw timed with `glFinish` after it, three release runs:

| | draws | p50 | p95 | max |
|---|---|---|---|---|
| island with Control Centre, 20 opens | 1,604–1,697 | 0.46–0.61 ms | 0.77–2.70 ms | 3.6–28 ms |
| menu bar, 10 reveals with a menu open | 504–530 | 0.29–0.46 ms | 0.50–2.35 ms | 1.8–4.0 ms |

The run with both p95s over 2 ms was the first after a long build, with the host busy;
the other two sat at 0.77 and 1.06 ms for the island and 0.50 and 0.51 ms for the bar,
inside acceptance bar 5's 2 ms. The longest draws are a tree's first, as its panels are
built and laid out. A debug build measured 1.1–1.7 ms p50 and 1.9–2.5 ms p95 for both.
The island's tree is drawn on every frame of a morph, about eighty draws an open and
close, and stays inside the budget, so the plan's fallback for the morph's cost (the
outgoing form drawn once into its own texture) was not needed.

Memory, release (RSS, the process's own anonymous memory, VRAM from `nvidia-smi`):

| | RSS | own | VRAM |
|---|---|---|---|
| at rest, the island shown | 140–143 MB | 27–30 MB | 92 MiB |
| Control Centre open | 156 MB | 31–32 MB | 75 MiB |
| after ten bar reveals | 159–160 MB | 34–35 MB | 49 MiB |
| bar down, system menu open | 160–163 MB | 35–38 MB | 89 MiB |
| at rest again | 160–163 MB | 35–38 MB | 49 MiB |

Inside acceptance bar 3's 250 MB and 200 MiB. The first rest figure holds Skia's caches
from start-up, purged once a surface closes. The revealed bar is a full-output surface
and adds about 40 MiB while it is down (its texture and its glass backdrop) and gives it
back on hiding, so the menus stay in it rather than moving to placed surfaces of their
own. A debug build sits at about 206 MB RSS.

## A closed tree's state

A surface's model writes its frame into the `State` the tree registered. When a bar,
island or panel closed, its tree unmounted but the compositor still held the handle, and
the next write panicked inside Freya ("Peeking the State failed"). The boundary caught
it and rebuilt the surface, so nothing showed on screen, but each panic symbolised a
backtrace: RSS rose from 192 to 500 MB over forty reveals in a debug build. Every tree
that registers a state now removes it in `use_drop`; the menu bar test and the bench
assert that nothing panicked.

## Freya layout

- An absolutely positioned child that fills, inside a parent sized to its content, takes
  the room the parent was offered, not the parent's size: a menu panel stretched to the
  output's height. Menu panels are given their height.
- A flex child grows only when its parent says `.content(Content::flex())`; without it
  the spacer between a label and its hint did nothing and the hint overflowed.
- `.shadow()` replaces the shadow rather than adding one; a list of shadows is folded in.
- `use_memo` over a component's props kept the first props' value; the menu derives its
  rows in `render`.
- A menu that auto-focuses takes the keyboard from a search field above it; the menu's
  focus follows whether it takes typing.

## The wallpaper

Decoding and fitting a 5K PNG took about 3 s in a debug build, from the image crates'
unoptimised loops. The dev profile builds `image`, `zune-jpeg`, `image-webp`, `fdeflate`
and `png` at `opt-level 3`; a release build decodes on the worker without a frame being
late. Frost needs no decode and stands in until a picture is ready.

## Menus

- DBusMenu properties arrive each wrapped in a variant inside the `a{sv}`; read without
  unwrapping, every label was empty. `GetLayout` replies `(u(ia{sv}av))`.
- Search covers what the app has given: a submenu filled only on asking (Qt's Open
  Recent) is not in the index until it is filled. Opening Help asks the app to fill every
  submenu, then tells it each has closed again (`closed` for DBusMenu, the
  `submenu-action` off for GMenuModel), so a query typed into Help finds them.
- While the bar holds the keyboard, focus is on the bar, not a window; the bar keeps the
  window it was revealed for so its menus stay that app's.
- **Electron and VS Code.** Unchanged from `global-menu-2026.md`: Electron's Wayland
  menu export is still unmerged, and VS Code installs no native menu off macOS, so it
  shows the fallback with the editor table's labels. An Electron app that does register
  is found through the PID of its bus connection, taking the first registration from
  that process, so with several windows each shows the same window's menus until
  Electron's fix gives them their own ids.
