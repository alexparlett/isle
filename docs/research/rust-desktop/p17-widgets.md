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
