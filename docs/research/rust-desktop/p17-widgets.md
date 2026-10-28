# P17 · Widgets: measures and the walk

## Measures (2026-10-28)

Release build with `test-hooks`, headless on the host (5070 Ti through EGL), a
3440×1440 output, Frost wallpaper:
`cargo test --release -p isle-compositor --features test-hooks --test bench -- --ignored dashboard_cost --nocapture`.
Each case opens the dashboard by its chord, waits three seconds, then moves the
selection 80 times, Right and Left in turn, so every tap redraws.

| Case | Draws | p50 | p95 | max | Chord to first draw | RSS closed → open |
|---|---|---|---|---|---|---|
| Default layout (8 widgets) | 84 | 0.55 ms | 1.27 ms | 3.30 ms | 11.1 ms | 166 → 196 MB |
| P1's full canvas (20 widgets, outer columns in use) | 83 | 0.77 ms | 1.98 ms | 3.02 ms | 10.8 ms | 164 → 195 MB |
| Every cell (133 widgets, a worst case) | 85 | 1.60 ms | 2.63 ms | 3.59 ms | 17.3 ms | 164 → 201 MB |

Bar 5 (content draw ≤ 2 ms p95) holds for the default layout and P1's full
canvas; the full canvas is just under. A screen with every cell filled is over
it, and is a case nobody draws. After each closing no `bwrap` or
`xdg-dbus-proxy` was left under the compositor.

**What it took.** One surface at first: the default layout's p95 was 2.27 ms
counting the opening's cold draws, and with the plates' shadows switched off
1.45 ms. Freya draws every shadow with a blur mask on every frame, and a
selection move redrew the whole screen-sized surface, plates included. The
plates, their shadows and edit mode's cells moved to a back surface,
`dashboard-plates`, which redraws only as the layout changes; the front draws
content, the ring and edit chrome. The bench then measured from after the
opening, which it had counted before.

## The walk in the VM (2026-10-28)

The guest at 2560×1080 (a 28 × 10 grid from (56, 97)), on the checkout's
debug build, driven by real keys and pointer through QMP; Alt is ⌘ there.
The registry's `isle-2-providers` branch was cloned into `dev/` and set as
the only source.

Worked:

- Ctrl+Option+Cmd+W opened both surfaces; the weather was a real Open-Meteo
  forecast; System sampled (the VM has no GPU, so GPU and its temperature
  read "—").
- E entered edit mode with the gallery; a drag of the clock two cells
  pushed the rest along, written to `settings.kdl` at once; ⌘Z put every
  widget back.
- Tab into the gallery, typing "tim" and Return added a Timer at the top
  centre, selected; ⌘3 opened Browse, which fetched the `file://` source
  and listed its three providers with their permissions.
- Install opened the consent panel; Return did nothing until it armed, then
  installed and granted. Typing "hacker" and Return placed Hacker News,
  which fetched the real API through the sandbox's proxy.
- Start on the Timer, then Escape: the island ran the timer on.
- Settings, run from the checkout, opened at the provider's page from its
  link, and its permission switch revoked the grant.
- No panic in the journal, and no `bwrap` left after closing.

Found and fixed:

- Return on a Browse item placed the uninstalled provider on the grid
  (twice, as "isn't installed" strips). Return there now starts Install,
  and nothing not built in or installed can be placed; a compositor test
  holds it.
- The consent panel read "It runspython3 hn.py.": a label's trailing space
  is dropped, so the row now spaces its parts.
- The gap's menu offered Edit widgets in edit mode; there it offers only
  Close widgets.

Not walked here, for the host run: resizing by an edge or the corner with
the pointer, Option+Cmd+arrows and Delete by hand, the widget settings
popover's text field, the Widgets hot corner and the wallpaper's menu by
pointer (each has a headless test), Reduce Motion, Increase Contrast and the
largest text size, a second screen, and Update from a source with a new
commit. The guest's installed package is from 19 October, so Settings must
be run from the checkout, or the package rebuilt, to see P17's pages.
