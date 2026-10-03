# Spotlight

P1 stage 5, wave B. How Spotlight looks and moves, in Frost, from
`structure/spotlight.md` (binding). Values are `tokens.kdl` names; numbers
not yet tokens are listed at the end.

## Anatomy

Two glass shapes on one vertical axis in the overlay layer: the **field**,
and under it, `space-s` below, one **panel** holding the results, a scope's
grid or the Apps view. Each is one glass surface with its own
`shadow-panel`; nothing on them is glass, and a menu opened from them is
`material-solid` (`menus.md`), so glass never sits on glass.

**Place.** Centred horizontally on its screen. The field's top sits at
`size-spotlight-top-scale` (two ninths) of the screen's height, rounded to
a whole point: y 320 on the 3440×1440 screen (x 1360 to 2080), y 240 on
the 1920×1080 one. The field never moves or resizes while Spotlight is
open; the panel's top is fixed at the field's foot plus `space-s` (y 376)
and everything grows down from it. The island's strip stays clear above.

### The field

| Part | Look |
|---|---|
| Shape | `size-spotlight-width` (720) × `size-spotlight-field` (48), `radius-panel`; `material-glass` at the user's glass position, `material-blur`, `material-saturation`, `material-noise-opacity`; `shadow-panel` |
| Query | `type-style-title-2`'s size and line (17/22) at `type-weight-regular`, `colour-text`, from `space-l` in (after the scope token when one is on), cut at its start as it scrolls. No magnifier glyph (the sheet) |
| Placeholder | The scope's string (`structure/spotlight.md` Copy) in the query's style, `colour-text-2` |
| Caret, selection | Caret 2 wide, the line's height, `colour-accent`. A reopened query's selection: `colour-accent` mixed in at `selection-mix` behind `colour-text` |
| Scope token | A capsule `size-control` (24) tall, `space-m` from the leading edge, padding `space-s`, `colour-fill-strong`; its name in `type-style-body` at `type-weight-medium`, `colour-text`. One app: the app's icon at `size-app-icon-small` (16), `space-xs`, its name. The query starts `space-s` after it |
| Quick keys | Four icon buttons while the query is empty, trailing, `space-s` (8) from the top, foot and trailing edge, `space-xs` apart: 32 square, `radius-card` (concentric: 18 less 8), `colour-fill`; hover `colour-control-hover`, pressed `colour-control-pressed`. Glyphs at `size-glyph-large` (20), Regular, `colour-text-2`: Apps `squares-four`, Files `folder`, Actions `command`, Clipboard `clipboard-text`. Hover after `timing-tooltip-ms` (700) shows a tooltip (component) under the button: "Apps ⌘1" |
| Activity mark | While a slow source searches: `dots-three` at `size-glyph` (16), still, `colour-text-2`, `space-l` from the trailing edge. Never with the quick keys (they need an empty query; slow sources need one typed) |

### The results panel

| Part | Look |
|---|---|
| Shape | 720 wide, `radius-panel`, `material-glass` and its blur, `shadow-panel`; padding `space-s` |
| Height | `limit-results-rows` (8) × `size-row-large` (40) plus padding: 336 (y 376 to 712). Set when the first result appears and kept until the query is cleared; with fewer results the rest is empty glass. The Files scope adds a fixed strip on top (below) |
| List | Clipped to the panel's inner rectangle at `radius-card`; group headers scroll with the rows |
| Group header | `size-row` (28) tall, its label at the foot: `type-style-subheadline`, `type-weight-semibold`, `colour-text-2`, aligned with the rows' icons. No rule above it |
| Scroller | Overlay only: a thumb 6 wide, radius 3, `colour-hairline-strong`, inset `space-xxs` from the trailing edge, shown while scrolling and for `timing-hover-settle-ms` after |
| Files strip | In the Files scope: a segmented control (component) "Names", "Contents" at `size-control`, leading at the rows' inset, then `space-s`; the panel is 368 tall in that scope |

**A row.** `size-row-large` (40) tall, every row the same; `radius-card`
(concentric: the panel's 18 less 8). Inset `space-s`, a leading slot 32
square, `space-m`, the text column, the trailing column, inset `space-m`.

| Part | Look |
|---|---|
| Title | `type-style-body`, `colour-text`, one line, cut at its end (window titles and paths in the middle). Centred in the row when there is no second line |
| Second line | `type-style-subheadline`, `colour-text-2`, one line: a path with home as ~, an app's name, "{App} · Desktop 2", a pane ("Settings › Displays"), a menu path ("Edit › Find") |
| Trailing | A chord for an action or menu item in `type-style-body`, `colour-text-2`, laid out as a menu's shortcut column (`menus.md`); a pinned clipboard item's `push-pin` at `size-glyph-small`, Fill, `colour-text-2`. Nothing else: no row names its kind |
| Top hit | The same 40-point row (the sheet's uniform rows), its title in `type-style-headline`, under the "Top hit" header |
| Calculation | `calculator` glyph; the answer as the title in `type-style-headline`, tabular; the normalised expression as the second line. Until the calculator answers, the title is "=" in `colour-text-2`; the answer replaces it in place |

**The leading slot**, 32 square, content centred:

| Kind | Slot |
|---|---|
| App, window, desktop-entry action | The app's icon at `size-app-icon` (32), its own shape |
| Game | Its cover fitted to 32 tall keeping its shape (a 2:3 cover is 21×32), `radius-small`, a `stroke-hairline` edge in `colour-hairline` |
| Menu item | The front app's icon at `size-app-icon-small` (16) |
| Settings row | The pane's tile at 32 as Settings' pane header draws it (`icons.md`): its `pane-*` hue, the pane's glyph in Fill, white |
| Shell action | Its glyph (P5's action table) at `size-glyph-large` (20), Regular, `colour-text` |
| File | Its thumbnail fitted to 32, `radius-small`, `colour-hairline` edge; else its Papirus type icon at 32 |
| Clipboard | Text: `text-align-left` 20. Image: its thumbnail as a file's, on a `colour-fill` square for transparency. Files: the first file's icon |
| Calculator, web, run, vault, "Show all in Files" | `calculator`, `globe`, `terminal-window`, `key`, `magnifying-glass` at 20, `colour-text` |
| Locked provider, error | `lock`, `warning-circle` at 20, `colour-text` |

**Contents rows.** The title is the name followed, `space-s` on, by its
folder in `colour-text-2`; the second line is the matching line with the
match on `colour-match`, `radius-small`. On a selected row the match is
`type-weight-semibold` instead of the fill.

**Selection.** The selected row fills `colour-accent`; title, second line,
chord, pin and a glyph in `colour-on-accent`. The accent is the one thing
Return acts on (brief P6), as a menu's selected row. One selection on
screen; none in the field.

### Clipboard preview (Cmd+Y)

Inside the panel, over the list's area at the same size, replacing it while
open: the whole text in `type-style-body`, `colour-text`, wrapped and
scrolling, inset `space-m`; or the image fitted inside `space-s`,
`radius-card`, on `colour-fill`. Nothing else in the panel changes.

### The emoji grid

The panel at the list's width: cells 44 square (`size-emoji-cell`),
sixteen columns (704, the inner width), the glyph drawn by the colour emoji
font at 28. Headers as the list's. Seven rows (308) show; below them a
name line `size-row` tall with a `colour-hairline` on its top edge: the selected glyph's
name, `type-style-body`, `colour-text`, leading at the cells' inset. The
selected cell fills `colour-accent`, `radius-card`. The panel is 352 tall.
A glyph's menu lists its variants as menu rows, the glyph leading.

### The Apps view

| Part | Look |
|---|---|
| Panel | The same glass, `radius-panel`, `shadow-panel`; `limit-apps-columns` (8) × `size-spotlight-tile` (112) plus `space-l` each side: 928 wide (x 1256 to 2184). Its top at the field's foot plus `space-s`; whole tile rows, as many as fit `space-xxxl` above the screen's foot, at most `limit-apps-rows` (7), then `space-s` at the foot: 832 tall on the ultrawide (y 376 to 1208), six rows on the 1920 screen |
| Top bar | Fixed, `size-row-large` (40): leading at the tiles' inset, the title of the section at the grid's top ("Favourites", "All apps", or the category's name) in `type-style-subheadline` semibold, `colour-text-2`; trailing at `space-m`, the category filter. A `colour-hairline` under it once tiles scroll beneath |
| Category filter | The pop-up button component, `size-control`, on glass at `colour-control`: "All apps" or the category. Reached from the grid by Tab (Accessibility). Its menu is `material-solid`: "All apps", a separator, then the categories below with a check on the current one |
| Section header | Between Favourites and All apps: `size-row` (28), as the list's header. Absent while a query or a category is on |
| Tile | 112 square, no fill at rest. Icon at `size-app-icon-large` (64), its own shape, centred, `space-m` from the top; `space-s`; the name in `type-style-body`, `colour-text`, centred, one line, cut at its end inside `space-xs` each side; the full name in a tooltip on hover |
| Game tile | The cover in the icon's place: fitted to 64 tall keeping its shape (a 2:3 cover is 43×64), at most 104 wide, `radius-small`, `colour-hairline` edge; centred in the 64 slot |
| Badge | The badge component (18, `colour-danger`, white, 11 semibold, tabular, "999+" past three digits), its centre on the icon's top trailing corner inset 4, as the switcher's cards; on a cover, on the cover's corner |
| Selection | Finder's form: a plate 72 square (the icon and `space-xs` round it), `radius-card`, `colour-fill-strong`, behind the icon; the name in a `colour-accent` capsule (padding `space-xs` each side, the line's height plus 2) in `colour-on-accent` |
| Drag | The picked tile follows the pointer at full opacity with `shadow-menu`; its place empties; neighbours make room (Motion) |
| No match | "No results for “{query}”" in `type-style-body`, `colour-text-2`, in the first row's place, leading |

**Categories** (desktop entries' main categories under Mac-style names,
sentence case per `copy.md` rule 1), in this order: Developer tools
(Development), Games (Game, and the game library), Graphics & design
(Graphics), Internet (Network), Music (Audio), Video (Video, bare
AudioVideo), Productivity (Office), Education (Education, Science),
Utilities (Utility, System, Settings), Other (none of these). A category
with no apps is not listed.

### Menus from Spotlight

The tile's and a result's menu (secondary click, Cmd+K) are menus as
`menus.md` draws them on `material-solid`, the sheet's items. From a click,
at the pointer; from Cmd+K, under the selected row's slot or the tile's
icon, leading edges aligned. Options ▸ is a submenu with checks; Assign
to ▸ a radio group.

## States

| State | Visually |
|---|---|
| Rest | Nothing drawn; no surface exists. |
| Open, empty field | The field with its placeholder and quick keys; no panel. Or the last general query, selected, with its panel. |
| Typing, general | Quick keys gone; the panel with groups; the first row selected. |
| Scoped | The token in the field; that scope's list or grid; an empty scoped query lists what the scope holds. |
| Apps view | The Apps token; the Apps panel, Favourites first, the first tile selected. |
| Apps view, filtered | Section header and favourites gone; the bar's title empty; matching tiles in rank order, the first selected. |
| One app (Tab) | The app's icon and name as the token; its groups in the list. |
| Loading: fast sources | Nothing drawn for it: the previous list stays until the new one replaces it whole. |
| Loading: slow sources | The activity mark; arrivals only below the selected row (Motion). |
| No results | General: the web row alone. A scope: the one line, as the Apps view's no-match. |
| Empty sources | That line ("No copied items", "No open windows"); "No password manager" as a row with `key`, second line "Open Keychain". |
| Error | The failed group absent; a provider's error as a row with `warning-circle` and its reason. |
| Vault locked | A row per provider: `lock`, "{Provider} is locked", second line "Unlock in Keychain". |
| Many results | The list scrolls with the overlay scroller; the selection kept in view. |
| Nothing to paste into | No visual change; the island's "Copied" after closing. |
| Second screen | The same sizes, at that screen's two ninths; the Apps view six rows. |
| Full-screen app in front | As normal, over it. |
| Windowed game in front | `material-solid`, no blur, no transitions. |
| Full-screen game in front | Not drawn. |
| A system prompt | Spotlight unchanged beneath; the field shows its unfocused form (caret hidden) until focus returns. |
| Locked, shortcuts inhibited, Focus on | Closed; no change; no change. |
| Reduce Motion, Reduce Transparency, Increase Contrast, largest text | Motion and Accessibility. |

## Motion

The compositor draws it all (B7). Content rides its shape's progress p,
never its own clock; exits are quicker than entrances.

| Transition | Form |
|---|---|
| Open | On `motion-open`, the field (and the panel, when the last query returns with results) scales from `motion-panel-scale` (0.96) to 1 about the field's centre, opacity min(1, 2p). No drop and no growth from a hot corner. Keys typed from the chord land in the field from the first frame |
| Panel appears (first result) | Its rectangle is final at once; a clip reveals it downward from the field's foot on `motion-open`, rows at opacity 2p − 1. The field does not move |
| Keystroke | The new list replaces the old in one frame; nothing animates per key |
| Slow rows arriving | Only below the selected row. Rows beneath the insertion part on `motion-move`; the new row's opacity follows that spring from its half. Appended at the end, it fades in over `motion-fade-ms`. The selected row and all above it never move |
| Selection | Instant, in lists and grids (as menus and the switcher). Scrolling to keep it in view runs on `motion-move` |
| Quick keys | Out over `motion-exit-fade-ms` with the first character; back over `motion-fade-ms` when the query empties |
| Scope token | In and out in one frame; the query shifts with it |
| Results ↔ Apps view | The panel's rectangle springs (width about the axis, height from its fixed top) on `motion-open` toward the Apps view and `motion-exit` back; outgoing content 1 − 2p, incoming 2p − 1 |
| Apps view from closed | As Open, the panel revealed downward as above |
| Tiles making room | Neighbours move on `motion-move`; a dropped tile settles on the same spring |
| Row or tile leaves (deleted, unpinned) | Out over `motion-exit-fade-ms`; the rest close the gap on `motion-move` |
| Clipboard preview | Crossfades with the list over `motion-fade-ms`; out over `motion-exit-fade-ms` |
| Quick Look | Grows from the selected row's rectangle and back (`files.md`); Spotlight beneath does not change |
| Close (choice, Escape, click outside) | Everything to opacity 0 over `motion-exit-fade-ms`, in place, no scale. Focus returns in the first frame; the action is delivered then |
| Drag out | Spotlight fades over `motion-exit-fade-ms` as the drag leaves it |
| Reduce Motion | Open and close are fades of `motion-reduced-fade-ms`; the panel and the Apps view appear at their size; the results ↔ Apps change is a crossfade at the new size; tiles and rows jump; scrolling jumps |
| Game Mode | No transitions: drawn whole in one frame, gone in one; `material-solid`, no blur sampled |

## Accessibility

- **Text roles.** On the glass only `colour-text` (titles, query, names,
  tile names) and `colour-text-2` (second lines, headers, placeholder,
  chords, the activity mark, the empty line); never `colour-text-3`. On the
  selection, `colour-on-accent` over `colour-accent`. Settings' tiles carry
  white on their `pane-*` hue as in Settings. The badge is the component's.
- **Increase Contrast.** Field and panels at `material-glass-tinted` with a
  `stroke-hairline` edge in `colour-hairline-strong`; the selected row, the
  tile's plate and capsule gain a 1-point inner `colour-hairline-strong`
  edge; the badge a 1-point `material-solid` ring; quick keys and the scope
  token a `colour-hairline-strong` edge; glyphs step to Bold.
- **Reduce Transparency.** Field and panels on `material-solid`, no blur or
  noise; shadows kept.
- **Focus.** The field holds the keyboard throughout; the selection is the
  focus, so no ring is drawn on rows, tiles or cells. Quick keys and the
  Names / Contents switch are pointer targets with chords. In the Apps
  view, Tab moves focus from the grid to the category pop-up, which takes
  the focus ring (`stroke-focus-ring` at `stroke-focus-offset`, round
  `radius-control`) and opens on Space; Tab again returns to the grid with
  its selection kept. A menu opened from Spotlight follows `menus.md`.
- **Largest text size** (1.5×, as `island.md`). The field is the query's
  line plus 26 (59); rows are the two lines plus 10, rounded up to the grid (56) and the panel
  still shows eight (464); headers and the name line grow. Tiles grow by
  the name's line (120), and the column count is reset on the next open to
  what fits 928 (seven). Icons, covers and badges keep their size.
- **Hit targets.** Rows 40, tiles 112, emoji cells 44, quick keys 32, the
  pop-up and switch at `size-control`: all above `size-hit-minimum`.
- **Never colour alone.** Pins have the pin glyph; a locked vault says so;
  the selection is a fill and, in the grid, the plate.

## Board

At 3440×1440 with the 1920×1080 screen beside it, dark first, then light,
over a window-covered desktop on the Haze wallpaper. Crops (harness
`[x0, y0, x1, y1]`): list scenes `[920, 240, 2520, 800]`, Apps view
`[920, 240, 2520, 1280]`, both 1600 wide.

1. **Open, empty**: the field with quick keys, one hovered with its
   tooltip. Whole screen, then crop.
2. **General search** "fi": Top hit Firefox, Apps, Actions with chords,
   Menu items, Settings, Windows, Files, the web row; the second row
   selected.
3. **Calculation** "12*48": the row before and after the answer.
4. **Slow arrivals**: three frames as files arrive below the selection,
   the activity mark on.
5. **Scopes**: Files with Names / Contents and a contents match; Clipboard
   with Pinned and Recent, an image and a text item; Windows; Run.
6. **Clipboard preview**: an image, then a long text.
7. **Emoji grid**: recent and Smileys, one selected, its name line.
8. **Passwords**: items, a locked provider, "No password manager".
9. **One app**: Tab on Firefox.
10. **Empty and none**: "No results" in a scope; the general web row alone;
    "No copied items".
11. **Apps view**: two rows of favourites, All apps with games' covers and
    a badge, one tile selected; the bar's hairline after a scroll.
12. **Category filter** open on Games; and "ste" typed in the Apps view.
13. **Menus**: the tile's menu with Options ▸ open; a result's menu.
14. **Quick Look** over Spotlight from a Files row.
15. **Second screen**: Spotlight on the 1920 screen, list and Apps view.
16. **Windowed game**: the solid field and panel over a game.
17. **Settings**: scenes 2 and 11 under Increase Contrast, Reduce
    Transparency and the largest text size.
18. **Motion** on the harness's spring: open with a restored query; first
    result; slow arrivals; results to Apps view and back; tiles making
    room; close; each under Reduce Motion and Game Mode.

## Choices

1. **The Apps view's width.** (a) The field keeps 720 and the panel under
   it is 928; (b) the field widens to 928 with it. **Recommended: (a).**
   The field's place never changes while open (the sheet), and eight
   64-point tiles need the room.
2. **A tile's selection.** (a) Finder's grey plate behind the icon with the
   name in an accent capsule; (b) an accent plate behind the whole tile.
   **Recommended: (a).** It is the Mac's icon-view selection, and colourful
   icons stay legible on grey where they clash on the accent.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `spotlight-top-scale` | 0.222 (two ninths of the screen's height; unitless by its suffix) |
| `size` | `spotlight-width` | 720 (field and results panel) |
| `size` | `spotlight-field` | 48 |
| `size` | `spotlight-tile` | 112 (the Apps view's tile, square) |
| `size` | `emoji-cell` | 44 |
| `limit` | `results-rows` | 8 |
| `limit` | `apps-columns` | 8 (at the default text size) |
| `limit` | `apps-rows` | 7 (the most the Apps view shows at once) |

Category names are in `copy.md`'s recurring strings.
