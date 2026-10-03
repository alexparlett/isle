# Data colours

P1 stage 5, wave C. How `series` and `kind` in `tokens.kdl` were chosen
and checked. Neither is status, the accent or a control colour.

## Method

The dataviz skill's validator (`validate_palette.js`): OKLab ΔE ×100
between colours under normal vision and under protanopia, deuteranopia
and tritanopia (Machado 2009, severity 1); target 8 under each deficiency
and 15 in normal vision; OKLCH lightness in the appearance's band (0.48 to
0.67 dark, 0.43 to 0.77 light) and chroma at least 0.10; 3:1 against each
surface the colours sit on. `contrast.py` repeats the 3:1 check on every
change to the file.

## `series`

Four colours that any two of may meet (calendar marks, the System widget,
Monitor's stacked graphs), so every pair is checked, not only neighbours.
They were searched for on an OKLCH grid inside the bands with every pair
passing, then held away from `ok`, `warn`, `danger`, `live` and the blue
accent. The hues: cyan, pink, violet, amber.

| | a | b | c | d |
|---|---|---|---|---|
| Dark | `#1ca2c3` | `#fa378c` | `#bd50fd` | `#d1791e` |
| Light | `#0f7eae` | `#dc0075` | `#8f38ed` | `#a6560a` |

| Worst pair (all pairs) | Normal | Protan | Deutan | Tritan | Contrast, least |
|---|---|---|---|---|---|
| Dark | 20.4 | 15.0 | 10.6 | 10.3 | 3.96 (c on `window-raised`) |
| Light | 19.7 | 13.2 | 10.1 | 11.1 | 3.93 (a on `window-sunken`) |

Contrast is against `window`, `window-raised`, `window-sunken`, `view`,
`sidebar` and `material-solid`. Distance from the nearest status or accent
colour, normal vision: 11.2 dark (b from `live`), 10.2 light (d from
`warn`). The previous set failed: dark a and c fell to 1.5 under
protanopia, and a beside b to 14.6 in normal vision.

## `kind`

Cleaner's ten colours (`surfaces/cleaner.md`), checked for neighbours in
their fixed order, which every bar and legend keeps; on the map any two
may touch, and symbols, names and gaps carry identity there.

| Worst neighbours | Normal | Protan | Deutan | Tritan | Contrast, least |
|---|---|---|---|---|---|
| Dark | 16.2 | 10.5 | 9.9 | 13.3 | 4.69 |
| Light | 16.6 | 11.4 | 10.4 | 12.5 | 3.51 |

Kept as they are. Two of the validator's checks fail by design: `other`
is grey, below the chroma floor, so it reads as no kind; the dark set sits
above the dark band (lightness 0.68 to 0.80), because each kind tile
carries a dark glyph in `colour-view` that needs a light fill.
