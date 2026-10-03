# Monitor

P1 stage 5, wave C. How Monitor, its Inspect windows and its Sensors window
look and move, in Frost. What they are, their states, keys and copy are
`structure/monitor.md` (binding); the window's anatomy, toolbar, tables,
states in place and focus are `surfaces/conventions.md`'s, alerts
`surfaces/window-chrome.md`'s, the polkit dialog `surfaces/prompts.md`'s, and
are reused here, not redrawn. Values are `tokens.kdl` names; numbers not yet
tokens are listed at the end.

## Anatomy

### The window

- **Default size** `size-monitor-width` by `size-monitor-height` (1040 by
  680), minimum 640 by 420; first placed centred in the work area (x 1200,
  y 380 on the ultrawide). No sidebar, no inspector.
- **Regions**, top to bottom: the toolbar (52); the table, `colour-view`,
  scrolling under the toolbar; the footer, `size-monitor-footer` (136),
  `colour-window`, a `colour-hairline` on its top edge, fixed.

### The toolbar

| Part | Look |
|---|---|
| Leading | The lights; `space-l`; the title, the filter's name ("All Processes"), `type-style-headline` `colour-text`, no subtitle |
| Resource switch | Centre group, centred on the content: `conventions.md`'s segmented control with text segments: plate `size-control-large` (30), `colour-fill`, `radius-control`, padding `space-xxs`; six segments, each padded `space-m`, label `type-style-body`; chosen `colour-window-raised` with `shadow-control` and `colour-text`, others `colour-text-2` |
| Quit Process | The primary action: a regular button at `size-control-large` labelled "Quit Process", `colour-control`, never the accent; its own group, `space-l` before More. Disabled (`colour-text-disabled`) with nothing selected, or with Isle or the session's manager selected |
| More, search | `dots-three-circle` (Inspect Process, Stop or Continue, Priority, Send Signal to Process…), then the search field, placeholder "Search" |

Narrow: the switch folds into » before the title cuts, as `conventions.md`.

### The table

`conventions.md`'s table: header `size-control` (24), rows `size-row` (28)
inset `space-s` with `radius-row`, cells `space-l` apart, stripes of
`colour-fill`, figures in `type-style-mono` tabular, right-aligned with their
titles. The sorted column's title is `colour-text` with `caret-down`
(descending, every resource's default) or `caret-up` at 12, `space-xs` after.

| Cell | Look |
|---|---|
| Process name | First, flexible, at least 240: the app's icon at `size-app-icon-small` (16) for a windowed process, else an empty 16 slot so names align; `space-s`; the name in `type-style-body`, cut at its end. A stopped process adds "Stopped" in `type-style-callout` `colour-text-2`, `space-s` after the name |
| Figures | % CPU 64, CPU time 80, Threads 64, PID 64, Memory 88, Energy impact 96, % GPU 64, Bytes written and read 104, Bytes received 112, Bytes sent 104, GPU memory 104. Units in the cell ("1.2 GB", "84 KB"); a rate column with no second sample is empty |
| Text columns | User 96, Preventing sleep 120 ("Sleep", "Idle", "Shut down", joined by commas), GPU 120 (the GPU's short name: "RTX 5070 Ti"); `type-style-body`, leading-aligned |
| Selected | Focused: `colour-accent` with `colour-on-accent` for every cell, figures too; runs of rows join. Unfocused: `colour-selection-unfocused` |
| Unavailable | A column whose figure the system lacks shows every cell empty; the reason is the footer's (below) |

Widths are defaults in Monitor's own column table, remembered per resource
(`structure/monitor.md`); they are not tokens.

**Hierarchy** (All Processes, Hierarchically). Each level indents the name
cell by `space-l` (16). A process with children carries `caret-right` at 12
in `colour-text-2` in a 16 slot before its icon, `caret-down` when open; a
leaf keeps the empty slot. Each figure column holds two figures: the
process's own in `colour-text`, then, `space-s` after and only on rows with
descendants, the subtree's total in `colour-text-2`; the column widens by 56
to hold both, and sorting uses the total.

### The footer

Padding `space-l`; three regions `space-xxl` apart: leading figures, the
graph, trailing figures. Figure columns are `size-monitor-figures` (200);
the graph takes the rest, at least 240 and at most 520, centred in it.

| Part | Look |
|---|---|
| Figure row | `size-control-small` (20) tall: label `type-style-callout` `colour-text-2` leading, value `type-style-mono` tabular `colour-text` trailing in the column. A series key, where the row names a series: a `size-mark` (6) dot in the series colour, `space-xs` before the label |
| Graph title | `type-style-subheadline` `colour-text-2`, centred over the well, `space-xs` above it ("CPU load", "Memory pressure", "Power", the GPU's name) |
| Well | `size-graph` (84) tall, `colour-window-sunken`, `radius-control`, a `colour-hairline` ring; inside it, hairlines at a quarter, half and three quarters of the height in `colour-hairline`; no vertical grid |
| Series | A line of `stroke-graph` (2), round joins, in the series colour, over an area of the same colour at `series-area-opacity` (0.24) down to the baseline. One sample every `size-graph-step` (4) points, newest at the trailing edge |
| Scale | Percentages run 0 to 100 and show no scale. Rates and watts scale to the largest value shown, rounded up to a 1, 2 or 5 step, written top leading inside the well in `type-style-footnote` `colour-text-2` ("300 W", "50 MB/s"), inset `space-xs` |
| Gap | Time not sampled is empty: no line, no area, the trace resuming at the next sample |
| Hover | A 1-point `colour-hairline-strong` line at the nearest sample; the tooltip of `conventions.md` above the well at that x: the value and the sample's time, in `type-style-callout` tabular |

**Series by resource.** `series-a` first, then `series-b`, `series-c` and
`series-d` as a graph needs them; any two may share a graph
(`system/data-colours.md`).

| Resource | Leading figures | Graph | Trailing figures |
|---|---|---|---|
| CPU | System (key `series-b`), User (key `series-a`), Idle | "CPU load": stacked, User below, System above | Threads, Processes; then, a third column where wide enough, the package temperature and CPU fan |
| Memory | Physical memory, Memory used, Cached files | "Memory pressure": `series-a` alone, 0 to 100, never coloured by level | Swap used, Compressed |
| Energy | Power (the total, watts), the battery where there is one | "Power": stacked, CPU package (`series-a`), then each GPU (`series-b`, `series-c`), each keyed on its figure row in the trailing column | CPU package and each GPU's board power |
| Disk | Reads in/s, Writes out/s | Mirrored about a centre line of `colour-hairline-strong`: in or read above in `series-a`, out or written below in `series-b` | Data read/s, Data written/s; then each drive's temperature |
| Network | Packets in/s, Packets out/s | As Disk: received above, sent below | Data received/s, Data sent/s |

Disk and Network draw either the operations or the data: a click on either
figure group's labels chooses it, and the chosen group's labels take
`colour-text` semibold, the other's stay `colour-text-2` regular. The group
is a toggle with "selected" as its state.

**GPU.** One block per GPU, side by side, sharing the footer's width
equally. A block is a figure column (Memory "9.2 of 16 GB", temperature,
power, fan, clocks) and a graph titled with the GPU's name in `colour-text`,
utilisation in `series-a` from 0 to 100. With one GPU the block fills the
footer as the other resources do. "Open LACT", a small regular button
(`size-control-small`), sits on the graph-title line at the footer's
trailing end when LACT is installed. Narrow with two GPUs: each block
keeps its graph and Memory row; below 2 × (240 + `space-xxl`) the blocks
stack and the footer grows by one well.

**Unavailable.** "Per-process network traffic isn't available." and its
kind take a line across the footer's top: `info` at 12, `space-xs`,
`type-style-callout` `colour-text-2`; the footer grows by 20. A sensor not
found is left out.

### The Quit Process alert

`window-chrome.md`'s alert, 320 wide, on the window: "Quit “{process}”?" in
`type-style-headline`; three equal buttons (88 each), Force Quit leading in
`colour-danger-text`, then Cancel, then Quit trailing as the default in
`colour-accent`. No caution glyph: a quit can be asked again. A raised
priority or another user's process brings `prompts.md`'s polkit frame with
Monitor's icon and the `lock` badge.

**Send Signal to Process…** is `window-chrome.md`'s sheet, `size-alert`
wide: the title; one row with a pop-up of signals by name and number
(TERM first); Cancel, and Send as the default.

### The Inspect window

`size-inspect-width` by `size-inspect-height` (520 by 600), placed by the
compositor. Toolbar: the lights, the title "{process} ({pid})" in
`type-style-headline` cut at its end; centre, a text segmented control
(General, Open Files and Ports); trailing, Quit Process as the primary
action, then search on the second tab.

- **General**: `colour-window`, `space-xl` padding. Three groups
  (`settings.md`'s group: `colour-window-raised`, `radius-card`,
  `colour-hairline` ring) of 28-point rows. Group 1: Parent process, User,
  Started, State, Priority. Group 2: % CPU, Memory, GPU memory, Threads.
  Then the command-line well with Copy, then Group 3: Executable, Working
  folder, Unit. A label column `size-detail-label` (120) wide,
  trailing-aligned, `type-style-callout` `colour-text-2`; `space-m`; the
  value in `type-style-body` `colour-text` (figures `type-style-mono`
  tabular). Parent process is a plain button in `colour-accent-text`. The
  command line sits between groups 2 and 3 in a well
  (`colour-window-sunken`, `radius-control`, padding `space-s`),
  `type-style-mono`, wrapping, selectable, at most six lines then
  scrolling, with "Copy" (small regular) `space-s` under it, trailing.
- **Open Files and Ports**: `conventions.md`'s table on `colour-view`, files
  first then sockets, the path column cut in the middle.
- **Ended**: above the first group, `info` at 16 and "This process has
  ended." in `type-style-body` `colour-text-2`, `space-m` under; every
  figure turns `colour-text-2`; Quit Process disabled.

### The Sensors window

`size-sensors-width` by `size-sensors-height` (560 by 640). Toolbar: the
lights, "Sensors", and "Open CoolerControl" as its primary action when
installed. The content is one table on `colour-view`: each device a section
header (`conventions.md`, 24 tall, `type-style-subheadline` semibold
`colour-text-2`), then its readings: label, value, the low and high since
opening, figures right-aligned, mono and tabular.

## States

| State | What changes |
|---|---|
| Rest | The table sorted by the resource's first column; the footer live |
| Loading | Toolbar, header, names and footer labels in the first frame; rate cells, rate figures and wells empty until the second sample; other figures fill from the first; no spinner |
| No results | `conventions.md`'s line in the table's place; the footer live |
| A figure not available | Empty cells; the footer's line (Anatomy) |
| A refused action | Nothing changes |
| Process ended while selected | The row leaves (Motion); the selection clears; its Inspect window's ended form |
| Stopped | "Stopped" after the name; Continue replaces Stop in More |
| Hierarchical | Indents, carets and the paired figures |
| Many processes | The table scrolls under the toolbar's scroll-edge bar; the footer stays |
| Hidden or covered | Nothing drawn or sampled; on return, the gap in each well |
| Pointer over the table, a key in it within `timing-type-select-ms`, menu or alert open | Rows hold their order; figures change in place |
| Narrow window | The switch into »; the footer drops its sensors first (CPU's third column, Disk's drive temperatures, each GPU block's temperature, power, fan and clocks), then its trailing figures. Energy keeps its keyed trailing column at every width and drops the battery row instead. Columns past the edge scroll sideways |
| Two GPUs | The GPU column in the table; two blocks in the GPU footer, stacked when narrow (GPU, above) |
| Inactive | `conventions.md`'s inactive form; wells and series unchanged |
| Second screen, game in front, locked, Focus on | No change of their own |

## Motion

Monitor's motion is small and tied to its samples; it draws nothing between
them except what the user moves.

| Transition | How | Reduce Motion |
|---|---|---|
| A new sample | **Stepped, with a short glide.** The trace moves one step (4 points) toward the leading edge on `motion-move` and then stands still until the next sample, so the window draws for a few frames per sample instead of every frame beside a game | The trace jumps one step at the sample |
| Re-sort | At each sample, rows whose place changed move to it on `motion-move`; the selection rides with its row | Rows take their new places in one frame |
| A row leaves | Out over `motion-exit-fade-ms`, then the rows under it close up on `motion-move` | Cut |
| Disclosure | The caret turns on `motion-move`; the children's rows grow in, the rows under them moving on `motion-move` and the new rows' content fading with the second half | Cut |
| Resource switch | None: table and footer swap in the frame (structure) | Same |
| Column drag | The column's title and cells lift as a `colour-window-raised` column with `shadow-control` and follow the pointer; neighbours move on `motion-move` | Neighbours jump |
| Column resize | At a title's trailing divider, a `size-resize-band` band, the column-resize cursor; the divider `colour-hairline-strong` while dragged; live with the pointer | Same |
| Figures | Change in place; no counting up | Same |
| Graph hover | At once | Same |
| Alert, Inspect, Sensors | `window-chrome.md`'s sheet and the compositor's open | As there |

**Game Mode:** every row is a cut, and the glide is a jump; a dragged
column's neighbours jump.

## Accessibility

- **Surfaces.** Table text on `colour-view` and its stripes; footer text on
  `colour-window`; scale labels on `colour-window-sunken`. Window roles
  only; `colour-text-3` only for the search placeholder.
- **Series as marks.** `series-a` to `series-d` pass 3:1 on
  `colour-window-sunken` in both appearances (dark 5.8, 5.0, 4.7, 5.4; light
  3.9, 4.2, 4.6, 4.6). Identity is never colour alone: every two-series graph has keyed
  figure rows, and the mirrored graphs also separate by side.
- **Increase Contrast.** Vertical column dividers in `colour-hairline-strong`
  through the table; the footer's top edge `colour-hairline-strong`; wells
  ringed `colour-hairline-strong`; grid hairlines `colour-hairline` (the
  stronger contrast form); the area's opacity unchanged, its line kept.
- **Reduce Transparency.** No change: the window is opaque; the toolbar keeps
  its solid bar.
- **Focus.** The table shows focus as the accent selection (with nothing
  selected, the ring inside the first row); the resource switch, Quit
  Process, More and search take the ring outside; figure-group labels that
  choose a graph are buttons and take the ring around the group.
- **Graphs.** Each well is a focusable graph named for its title, with a
  summary ("CPU load, last 2 minutes, 12 to 46%, now 31%"); focused, Left
  and Right move the hover line by a sample and read its value and time.
- **Largest text size.** Rows grow to their line plus 12; the footer's
  figure columns wrap into two columns each before any value is cut, and
  the footer grows to hold them; wells keep their height; column defaults
  grow by the text factor.
- **Targets.** Rows 28, header 24, segments 30, the caret's 20 square, the
  figure groups their full height. Graph hover takes the well's height.

## Board

Wave C's Monitor board at 3440×1440 with the 1920×1080 screen beside it,
dark first and each in light. The window at its default place; crops are x,
y, width, height on the ultrawide.

1. **CPU at rest**: Firefox and a game near the top, a striped table, % CPU
   sorted, the footer with keys, sensors and a stacked graph. Crop 920, 300,
   1600, 900.
2. **Every resource**: Memory, Energy (one GPU and two), Disk, Network,
   their footers and columns. Crops: the footer band, 1040 by 160, stacked.
3. **GPU, two GPUs**: the GPU column, two blocks and Open LACT. Crop as 1.
4. **Selection and sort**: three rows selected focused, then unfocused; the
   sort reversed. Crop: the table, 1040 by 400.
5. **Hierarchical**: kworkers under kthreadd collapsed, systemd expanded
   with paired figures. Crop as 4.
6. **States**: loading, no results, network unavailable, a stopped row, a
   gap after being covered. Crops as 4 and the footer band.
7. **Hover**: the crosshair and tooltip on CPU load. Crop: the footer 400%.
8. **Quit Process**: the alert on one process and on three; the polkit
   frame for root's process. Crops as 1 and `prompts.md`'s.
9. **Inspect**: General, Open Files and Ports filtered, ended. Crop: each.
10. **Sensors**: three devices with low and high. Crop: the window.
11. **Narrow**: 640 wide with the switch in » and the footer reduced;
    Energy keeping its keys; two GPU blocks stacked.
12. **Columns and signals**: a column mid-drag; a divider resized; the
    Send Signal sheet.
13. **Accessibility**: scene 1 under Increase Contrast and at the largest
    text size; a focused graph with its hover line.
14. **Motion**, on the harness's spring: three samples' glides, a re-sort,
    a row leaving, a disclosure; each under Reduce Motion.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `monitor-width`, `monitor-height` | 1040, 680 |
| | `monitor-footer` | 136 |
| | `monitor-figures` | 200 (a footer figure column) |
| | `graph` | 84 (a graph well's height) |
| | `graph-step` | 4 (points per sample) |
| | `inspect-width`, `inspect-height` | 520, 600 |
| | `sensors-width`, `sensors-height` | 560, 640 |
| `stroke` | `graph` | 2 |
| `series` | `area-opacity` | 0.24 (both appearances) |

With `keychain.md`'s `size-detail-label` (120). Phosphor glyphs to vendor:
`info`.
