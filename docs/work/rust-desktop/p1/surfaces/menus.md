# Menu bar and menus

P1 stage 5, wave A. The look and motion of the on-demand menu bar and of
every menu: the bar's, a title bar's window and zoom menus, the wallpaper's,
context menus and pop-up button menus in apps. Structure is binding from
`structure/menu-bar.md` (the bar, its menus, states, keys and copy),
`structure/conventions.md` (the menu set, context menus) and
`structure/spotlight.md` (menu items found from Spotlight, whose row is
Spotlight's). Values are `tokens.kdl` names; numbers not yet tokens are
listed at the end.

## Anatomy

### The bar

| Part | Look |
|---|---|
| Band | Full width of its screen, `size-bar-height` (48) from the screen's top edge, filling the `size-strip` (48), so the titles and the island share one centre line (gate 5); square ends. Material `material-glass` at the user's glass position, `material-blur`, `material-saturation`, `material-noise-opacity` |
| Edge | A `stroke-hairline` rule in `colour-hairline` along the lower edge, inside the band, and `shadow-bar` (new) beneath it: the bar is a place with an edge, not text over the wallpaper (`design-macos.md` §3.2) |
| Layout | Leading inset `space-s`. Each item is a title cell: `space-s` padding each side of its content, cells `space-xxs` apart. A cell's hit area is the bar's full height and runs to the top edge; the first cell's runs to the corner |
| Title highlight | The open title, or the focused one: a shape `size-control` (24) tall, centred in the bar (12 above and below), the cell's width, `radius-control`, `colour-fill-strong`. Neutral, never the accent: the accent marks the menu's selected row |
| System menu | Isle's mark, `isle-mark`, drawn by Isle on Phosphor's 256 grid in a `size-glyph` (16) box, `colour-text`, one weight in every state. Proposed form: **the isle**, a solid, slightly irregular island seen from above, about 14×12, with a small cove cut into its trailing side. It keeps the Mac's solid mark in the corner without Apple's, and shares nothing with Phosphor's glyphs (a dome over a line would read as Night Light's `sun-horizon`). Alternative for the gate: **the pill**, a filled capsule 14×7, the island in miniature, which risks reading as a minus or a switch |
| App name | `type-style-headline` (13 semibold), `colour-text`; at most `size-bar-name` (240), cut at its end past it |
| Menu titles | `type-style-body`, `colour-text`, never cut (they fold instead); a title the app marks disabled in `colour-text-disabled` |
| Overflow » | A title cell holding Phosphor `caret-double-right` at `size-glyph-small`, `colour-text`, where the folded titles would start. Its menu lists each folded menu as a submenu row, in order |
| Island reserve | The island's maximum width plus its guard gap each side (`island.md`'s values), centred on the screen; nothing is laid out in it. The right of the bar is empty |

Text sits on the band's vertical centre (`type-style-body-line` 16, so 16
above), the island's centre line. Vertical metrics hold on the 1920×1080 screen; only the width and
so the fold point change.

**The island in front.** The island is unchanged and does not move: it
sits at `size-island-top` with `size-island-height`, inside the bar, 9
points from its top and bottom edges, its clock on the titles' centre line.
The island's own shadow and its darker (dark) or whiter (light) tint are
what separate it from the bar; it is not restyled. The board checks light mode over a light
wallpaper; if the pill dissolves into the bar there, it takes its Increase
Contrast ring (`colour-hairline-strong`) while the bar is down, and nothing
else changes.

**Over a full-screen window** the band is the same, over the content's top
48 points with `shadow-bar` on the content; the island returns with it
(`island.md`), and for a window with Isle's title bar that bar comes
beneath, its top on the bar's lower edge (`window-chrome.md`'s look).
**Over a full-screen game** the band is `material-solid` with no blur or
noise, the edge hairline kept, `shadow-bar` dropped; the island in its
solid form. **While Game Mode is on** with a windowed game the bar takes
the same solid form, as the island does.

### Menus

| Part | Look |
|---|---|
| Material | **Glass** for the shell's menus whose backdrop is the desktop or a window: the bar's menus, a title bar's window and zoom menus, the wallpaper's menu. **`colour-window-raised`**, opaque, for menus that belong to a window: context menus and pop-up button menus in apps (the menu material, brief P4; `conventions.md`, popovers likewise). **`material-solid`** for a menu opened from a shell glass panel (a Spotlight result's or tile's menu, Notification Centre), so glass never sits on glass |
| Why the bar's menus are glass | They hang from a glass bar in the top layer and are the shell's, not the window's; frosted at `material-blur` nothing behind them reads, and the window's own menus stay opaque so the two are told apart |
| Body | `radius-card`, padding 5 (`radius-card` less `radius-row`, so rows are concentric), `shadow-menu` (its inset half-point ring is the menu's edge). Width from `size-menu-min` (220) to `size-menu-max` (480); a longer label is cut in the middle |
| Placement | From the bar: top `space-xs` under the bar's lower edge, leading edge on the title highlight's leading edge. Context menu: top-leading corner on the pointer's hotspot, flipped to the side that fits; from the keyboard, under the focused item's leading edge. Pop-up button: the current item's row over the button, its label on the button's label, at least the button's width. Every menu stays inside its screen, `space-s` from the screen's edges |
| Row | `size-menu-row` (22), `radius-row`. Leading inset `space-xs`; a mark column of 16 in every menu, so labels align; `space-s` gap; the label in `type-style-body` `colour-text`; the trailing column; trailing inset `space-s` |
| Mark column | Phosphor at `size-glyph-small`, centred: `check` for checked and radio, `minus` for mixed, `diamond` (fill) for a minimised window in the Window menu's list |
| Icon, last resort | An item with neither label nor accessible name: its icon at `size-glyph` (16) in the label column, monochrome `colour-text` |
| Shortcut column | `type-style-body`, `colour-text-2`, `space-xxl` at least after the label. macOS's symbols in Apple's order ⌃ ⌥ ⇧ ⌘, then the key; the keys sit in a slot as wide as the menu's widest key, left-aligned, so letters line up and modifiers right-align against them. On a PC-layout profile, words ("Ctrl+Shift+T") right-aligned (`copy.md` rule 11). Context menus carry none and the column collapses |
| Submenu chevron | Phosphor `caret-right` at `size-glyph-small`, `colour-text-2`, at the trailing inset |
| Separator | 9 tall: a `stroke-hairline` rule in `colour-hairline`, `space-s` in from the row edges. Consecutive and trailing ones collapse |
| Section label | A `size-menu-row` row, not selectable: `type-style-subheadline`, `type-weight-semibold`, `colour-text-2`, aligned to the label column |
| Count | The system menu's Software Update… count: a quiet badge (`colour-fill-strong`, `colour-text`, `type-style-subheadline` semibold, padded `space-xs`, capsule, 16 tall) in the trailing column |
| Scroll arrows | A menu taller than its screen's room stops `space-s` from the edge; a `size-menu-row` row at the end that has more, holding `caret-up` or `caret-down` at `size-glyph-small` in `colour-text-2`, centred, on the menu's material, the rows clipped under it. The pointer resting on it scrolls a row at a time; the wheel scrolls freely |
| Submenu | The same body; its first row level with its parent row, its leading edge on the parent's trailing edge, flipped left when the screen ends. The parent keeps its full material and contrast while the submenu is open (26's opacity swap is the fault to avoid, `design-macos.md` §2) |

Row states are under States. The Window menu's window list cuts titles in
the middle; the fallback app menu looks exactly as an app's own (every item
works, so nothing marks it as built by Isle); Log Out shows the user's
display name.

### Help's search

| Part | Look |
|---|---|
| Field | The first row of Help: the search field component inside the menu's padding, `size-control` (24) tall, `radius-row` (concentric with the menu), `colour-fill`; Phosphor `magnifying-glass` at `size-glyph-small` in `colour-text-2`, `space-s` from its leading edge; "Search" placeholder in `colour-text-2` (not the board's `text-3`: on glass, tertiary text is never used) |
| Below the field | `space-xs`, then while a query stands the results, a separator, the Help items; with none, the Help items |
| Width | `size-menu-search` (360) while a query stands, so paths fit; back to the Help menu's own width when cleared |
| Result row | A menu row: the path in `colour-text-2` ("Edit › Find › "), then the item's label in `colour-text`, the hint in the trailing column. The path is cut in the middle first, the label last |
| Disabled result | Path and label in `colour-text-disabled`; it can be moved onto, so its place in the menus shows, and Return does nothing |
| No results | One row, not selectable, "No results for “{query}”" (`copy.md`) in `colour-text-2` |
| The found item | Moving onto a result opens its titles and menus in the bar down to it: its title highlighted, each parent row in the open-parent state, the item itself in `colour-fill-strong`. The accent stays on the result row, the one thing Return acts on. Where the found item's menus overlap Help's menu they draw on `material-solid`; Help's menu stays on top |

## States

| State | Visually |
|---|---|
| Rest | No bar on any screen; the island alone in the strip |
| Revealed | The band with system mark, app name, titles, Window, Help on the left; the island in front; the right empty |
| A menu open | Its title highlighted; its menu below it; moving across titles swaps the highlight and the menu |
| Title focused, no menu open | The highlight plus the focus ring around it (Accessibility) |
| Menu row, normal | No fill |
| Selected row | `colour-accent` fill; label, hint, mark and chevron in `colour-on-accent` |
| Open parent | A row whose submenu is open while selection is deeper: `colour-fill-strong`, text unchanged. Only one row on the screen carries the accent |
| Disabled row | Label, hint, mark and chevron in `colour-text-disabled`; no fill under the pointer |
| Checked, radio, mixed | Their mark (Menus, Mark column); radio groups set apart by separators |
| Menus loading | The fallback titles at once; the app's own replace them once, in place, with no transition and no spinner. An open menu takes arriving items in place, the selected row kept |
| App not answering | The fallback stays; a menu shows what it had; a chosen item's menu closes as usual |
| No window focused | Files' name and its fallback menus |
| Many menus | » at the fold point; Window folds after the app's menus; system mark, app name and Help never fold |
| Second screen | The same band on the 1920×1080 screen, folding at its own width |
| Full-screen window | The band over the content with its shadow, the island in front, Isle's title bar beneath for a non-game window |
| Full-screen game, Game Mode | The solid band, the island solid in front, menus on `material-solid` |
| Shortcuts inhibited, locked | No visual change of their own; the bar is never drawn over the lock |
| Restart owed | "Restart to finish updating…" in the system menu's group 2; the Software Update… count |
| Reduce Motion, Reduce Transparency, Increase Contrast, largest text | Motion and Accessibility below |

## Motion

| Transition | How |
|---|---|
| Bar reveal | **A slide**: the band moves from just above the screen (its height plus `shadow-bar`'s reach) to its place on `motion-open`, anchored at the top edge. It comes from the edge the pointer pushed and leaves back into it; a fade would not say where it lives (`menu-bar.md` Character). Its content rides with it, never on its own clock. The island does not move; the band passes behind it |
| Bar leave | Back up on `motion-exit`, after `timing-hover-grace-ms`. A pointer returning mid-leave retargets the spring from where the band is |
| Over a full-screen window | The island and Isle's title bar ride the same spring and travel as the band, so the three arrive and leave as one, unless a capture runs: then the island grows from its privacy form in place; only the band and title bar travel |
| Front app changes while revealed | The name and titles are replaced in place in one frame, anchored at the left |
| Menu open | **Instant**, as macOS: the menu is whole in the next frame. A press-drag-release must find it there, and a menu opened dozens of times an hour should cost no time (brief P5) |
| Menu close (Escape, click elsewhere) | Opacity to zero over `motion-exit-fade-ms`, in place |
| Switching menus across titles | The old menu goes in one frame and the new one appears: two menus fading over each other is noise |
| Submenu | Opens instantly after `timing-hover-dwell-ms` on its row (at once from the keyboard); a sibling replaces it in one frame; a pointer heading for it on the diagonal keeps it |
| Choosing an item | **The blink, kept**: the selected row's accent goes off and on once over `motion-blink-ms` (80), then every open level fades together over `motion-exit-fade-ms`. The action is delivered at the press, not after the blink, so nothing waits on it; the blink is the one sign a choice landed when the app shows nothing (App not answering) |
| Title highlight | Moves between titles in one frame |
| Help results | Replace in place as the query changes; the menu's height follows at once, top fixed. Menus never animate their size |
| Reduce Motion | The bar fades in and out in place over `motion-reduced-fade-ms`, no slide; the island and title bar likewise. Menus as above (they do not move); the blink kept, being one change of colour |
| Game Mode | The bar appears and goes in one frame, solid. Menus appear and go in one frame, with no fade and no blink |

## Accessibility

- **Text on glass** (the band, the bar's menus, the window and zoom menus,
  the wallpaper's menu): `colour-text` for titles and labels;
  `colour-text-2` for hints, chevrons, section labels, paths, the
  placeholder and scroll arrows; `colour-text-disabled` only for disabled
  items. Never `colour-text-3`. On `colour-window-raised` and
  `material-solid` the same roles, which pass with margin. On the selected
  row, `colour-on-accent` over `colour-accent`, each accent built to pass.
- **Increase Contrast**: glass at `material-glass-tinted`; the bar's edge
  rule in `colour-hairline-strong`; every menu ringed in a `stroke-hairline`
  of `colour-hairline-strong`; separators take the contrast `colour-hairline`;
  glyphs (the mark aside) step to `icon-weight-contrast`. Disabled stays
  `colour-text-disabled`, as WCAG exempts it, and is never the only sign:
  arrows skip it (Help's results excepted), and it is in the accessibility
  tree as dimmed, so a screen reader's own navigation reads it.
- **Reduce Transparency**: the band and every glass menu on
  `material-solid`, no blur or noise; edges and shadows kept.
- **Focus.** In a menu the selected row is the focus, as on macOS; no ring
  is drawn inside menus. A focused title with no menu open shows the focus
  ring, `stroke-focus-ring` in the full accent at `stroke-focus-offset`
  around the title highlight (it fits the band's 5 points), with a 1-point
  `colour-text` line inside it on the glass. Help's field takes the focused
  field's look. Rows reached by type-to-select take the selection the same
  way; the typed letters are not shown, and they reset after
  `timing-type-select-ms`.
- **Largest text size**: the band is the larger of `size-bar-height` and
  the headline line plus 18, and the strip grows with it (`menu-bar.md`);
  rows are the body line plus 6; menus widen to `size-menu-max` and then cut
  in the middle; more titles fold, since the island's reserve grows too.
  Nothing is clipped.
- **Names.** The system mark is "System menu", » "More menus", the scroll
  arrows "Scroll up" and "Scroll down", the Software Update count
  "{n} updates".
- **Targets**: title cells are the bar's height and reach the top edge, the
  first the corner; rows (22) and scroll arrows (22) exceed
  `size-hit-minimum`; the » cell is 28 wide. Separators and section labels
  are not targets.

## Board

Gate 5's wave A board, at 3440×1440 with the 1920×1080 screen beside it,
dark and light, on a dark and a light wallpaper. Each scene is drawn whole
and in crops near 1600 points wide at actual size; crops are given as x, y,
width, height on the ultrawide.

1. **Rest and revealed**: the strip with the island, then the bar revealed
   over the wallpaper. Crops 0, 0, 1600, 300 (the left) and 920, 0, 1600,
   300 (the island in front on the titles' centre line, and the empty right).
2. **The system menu**, open, Software Update… with a count and the restart line.
   Crop 0, 0, 1600, 700.
3. **The app menu of Files** over a Files window: About, Settings…, Hide,
   Quit; the open title's highlight. Crop 0, 0, 1600, 700.
4. **The fallback**: Steam in front, its app menu with entry actions, File,
   Edit and View from the Cmd table, Help with its links. Crop 0, 0, 1600,
   700.
5. **Every kind of row**: Files' View menu with checks, a radio group, a
   section label, a disabled row, hints, and Sort By's submenu open with a
   mixed dash, the parent in its open-parent state. Crop 0, 0, 1600, 800.
6. **Keyboard**: Ctrl+F2's focused title with its ring after Escape, and a
   row reached by typing. Crop 0, 0, 1600, 500.
7. **The Window menu**: Move and resize open, Keep on top checked, the
   window list with a check and a diamond. Crop 0, 0, 1600, 900.
8. **Help's search**: "find" with paths, a disabled result, the found item
   opened in Edit › Find; and "zzz" with no results. Crop 0, 0, 1600, 900.
9. **Overflow** on the 1920×1080 screen: an app with many menus, » open
   with its submenus. The second screen whole, at actual size.
10. **A scrolling menu**: a long History menu with arrows at both ends.
    Crop 0, 0, 1600, 1440.
11. **Over a full-screen window**: a browser in full screen, the bar over
    its tabs, the island in front, Isle's title bar beneath for Files.
    Crop 0, 0, 1600, 400.
12. **Over a game**: the solid band and the solid island over a game frame,
    a menu open on `material-solid`. Crop 0, 0, 1600, 600.
13. **Menus in and on windows**: a Files context menu on
    `colour-window-raised`, the wallpaper's menu on glass, a pop-up
    button's menu open in Settings over its button, a title bar's window
    menu on glass. Crop 600, 300, 1600, 900.
14. **Accessibility**: scenes 1 and 5 under Increase Contrast, Reduce
    Transparency and the largest text size. Crops as theirs.
15. **The system mark**: the isle and the pill at 16, 32 and 64, in the bar
    at actual size, dark and light.
16. **Motion**, on the harness's spring: the reveal and leave, over the
    wallpaper and over a full-screen window; a menu opened, switched across
    titles and closed; a choice's blink and fade; each under Reduce Motion
    and Game Mode.

Menus loading and an app not answering look like scene 4 and need no
scene of their own.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `bar-height` | 48 (the band fills the strip, `size-strip`) |
| `size` | `menu-row` | 22 |
| `size` | `menu-min` | 220 |
| `size` | `menu-max` | 480 |
| `size` | `menu-search` | 360 (Help's width while a query stands) |
| `size` | `bar-name` | 240 (the app name's maximum width) |
| `size` | `edge-push` | 24 (pointer travel into the top edge that reveals the bar; shared with the island's top-edge return) |
| `timing` | `edge-dwell-ms` | 400 (the reveal over a game; v1's hot zone) |
| `timing` | `type-select-ms` | 1000 (type-to-select's buffer, in menus and every list) |
| `motion` | `blink-ms` | 80 (a chosen row's one off and on) |
| `shadow` | `bar` | dark "0 6px 18px rgba(0,0,0,0.30)"; light "0 6px 18px rgba(0,0,0,0.10)" |
