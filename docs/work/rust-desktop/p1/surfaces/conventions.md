# App conventions

P1 stage 5, wave B. How every Isle app's window looks and moves, in Frost:
the anatomy, the toolbar, sidebars, lists and tables, the states in place,
sheets, popovers, the About window, an app's settings window and keyboard
focus. What they are, their states, keys and copy are
`structure/conventions.md` (binding); the title bar, the unified toolbar's
height, the lights' inset, sheets and alerts are `surfaces/window-chrome.md`'s
and are reused here, not redrawn. Values are `tokens.kdl` names; numbers not
yet tokens are listed at the end.

## Anatomy

### The window in points

Measured from the window's top leading corner, in a window with every region.

| Region | Geometry | Fill |
|---|---|---|
| Toolbar | `size-toolbar` (52) across the whole width; the lights at `space-l` (16), centred on 26 (`window-chrome.md`) | None at rest: each region's colour shows through it |
| Sidebar | `size-sidebar` (220) by default, dragged at its trailing edge between `size-sidebar-min` (160) and `size-sidebar-max` (320); window top to bottom; a `colour-hairline` at its trailing edge; rows start at 52 + `space-s` | `colour-sidebar` |
| Content | Between sidebar and inspector, from 52 down, scrolling under the toolbar | `colour-view` for lists, tables and grids; `colour-window` for forms and detail |
| Reading column | Forms and detail keep `size-form` (600) at most, centred, with at least `space-xxl` (24) each side; tables and grids use the whole width | |
| Inspector | `size-inspector` (260), dragged between `size-inspector-min` (220) and `size-inspector-max` (360); its fill runs to the window's top as the sidebar's does, its rows start under the toolbar; a `colour-hairline` at its leading edge | `colour-window` |
| Auto-collapse | The sidebar collapses when content would fall under `size-content-min` (400), then the inspector; each returns when the window widens past the width that took it | |

A double-click on a sidebar or inspector edge returns its default width. The
edge's drag area is `size-resize-band` (8), centred on the hairline.

### The toolbar's items

| Part | Look |
|---|---|
| Icon button | `size-control-large` (30) square, `radius-control`, Phosphor regular at `size-glyph-large` (20) in `colour-text-2`; no fill at rest, `colour-control-hover` on hover, `colour-control-pressed` pressed and while its menu or popover is open; disabled glyph `colour-text-disabled` |
| Spacing | `space-xs` (4) between items in a group, `space-l` (16) between groups, `space-l` trailing padding (`window-chrome.md`) |
| Leading group | Sidebar toggle `sidebar-simple` at the sidebar's trailing edge less `space-s`; then, `space-l` after the sidebar, back `caret-left` and forward `caret-right` as two icon buttons; `space-s`, the title in `type-style-headline` `colour-text`, a subtitle under it in `type-style-subheadline` `colour-text-2` (the pair centred on the toolbar's height) |
| Grouped segments | Related symbol buttons that choose one of several (a view switcher) are a segmented control: a plate `size-control-large` tall, `colour-fill`, `radius-control`, padding `space-xxs`; segments 36 wide holding the glyph at 20; the chosen segment `colour-window-raised` with `shadow-control`, its glyph Fill in `colour-text`, others regular in `colour-text-2`. Text segments never share a group with symbols |
| Centre group | Centred on the content region, not the window; folds from its trailing end into a » button (`caret-double-right` at 20) whose menu lists the folded items by name |
| Primary action | At most one, a regular button with its label (`size-control-large`, `colour-control`), never the accent: the window's default button is a sheet's |
| More menu | An icon button holding `dots-three-circle`; its menu is the in-window menu (`menus.md`: `colour-window-raised`, `shadow-menu`), top `space-xs` under the button, trailing edges aligned |
| Inspector toggle | `sidebar-simple` mirrored, before the search field |
| Search field | Last. `size-control-large` tall, `size-toolbar-search` (180) wide at rest, `size-toolbar-search-wide` (260) while focused or holding a query, its trailing edge fixed so it grows toward the centre; `colour-fill`, `radius-control`; `magnifying-glass` at `size-glyph` (16) in `colour-text-2`, `space-s` from its leading edge; placeholder "Search" in `colour-text-3`; while a query stands, `x-circle` (Fill) at 16 in `colour-text-2` at `space-xs` from its trailing edge clears it |
| Folded search | Narrow windows: an icon button holding `magnifying-glass`; a click or ⌘F opens the field in place at `size-toolbar-search`, growing toward the leading side, the centre folding further to make room |
| Scope bar | Under the toolbar while a search stands (Files): `size-row` (28) tall, `colour-window`, a `colour-hairline` under it; scopes as a text segmented control at `size-control-small` (20), `space-l` from the content's leading edge |
| Tooltip | `colour-window-raised`, `shadow-menu`, `radius-small`, padding 3 by `space-s`, `type-style-callout`: the name in `colour-text`, `space-s`, the chord in `colour-text-2`; `space-xs` under the control, after `timing-tooltip-ms` (700) of rest, gone on press or leave |

**The scroll-edge bar.** Once content scrolls under the toolbar, a
`colour-titlebar` bar with a `colour-hairline` lower edge fills the toolbar
over the content region only (`colour-titlebar-inactive` inactive). Its
opacity follows the scroll offset from 0 to `space-s` (8): it rides the
scroll and has no clock of its own. A sidebar scrolled under the toolbar
does the same over the sidebar's width in `colour-sidebar`.

### Sidebar rows

| Part | Look |
|---|---|
| Row | `size-row` (28), inset `space-s` (8) from the sidebar's sides, `radius-row`; contiguous, no gap between rows |
| Content | Inset `space-s`: glyph at `size-glyph` (16) Phosphor regular in `colour-accent-text` (gate 4's sidebar), or a pane tile (`settings.md`), or an app or volume icon at `size-app-icon-small`; `space-s`; label `type-style-body` `colour-text`, cut at its end; a trailing accessory (count in `type-style-subheadline` `colour-text-2`, an eject button at `size-control-small`) at `space-s` from the end |
| Groups | `space-m` (12) between groups with no heading |
| Section header | Only for the user's own groups (Files' Favourites, Locations): 24 tall, `type-style-subheadline` semibold `colour-text-2`, aligned to the rows' glyph edge (16 from the sidebar's edge), its baseline `space-xs` above the first row; `space-m` above it |
| Selected, sidebar focused | `colour-accent` fill; label, glyph and accessory `colour-on-accent`; tiles and app icons keep their colours |
| Selected, not focused or window inactive | `colour-fill-strong`; text unchanged |
| Hover | Nothing, as macOS; pressed `colour-fill` |
| Drop target | Under a drag it accepts: `colour-accent` 2-point ring (`stroke-found`) inside the row, `radius-row` |

### Lists and tables

| Part | Look |
|---|---|
| List row | Single column: `size-row` (28) for one line, `size-row-large` (40) for two (a title in `type-style-body`, a second line in `type-style-subheadline` `colour-text-2`, `space-xxs` apart); inset `space-s` from the view's sides, `radius-row`; leading icon at 16 (one line) or 32 (two), `space-s` |
| Table header | `size-control` (24), `type-style-callout` `colour-text-2`, a `colour-hairline` under it; the sorted column's title in `colour-text` with `caret-up` or `caret-down` at `size-glyph-small` (12) `space-xs` after it; dividers between titles 12 tall in `colour-hairline`, shown only on hover over the header |
| Table row | `size-row` (28), inset `space-s` with `radius-row`, cells `space-l` apart, padding `space-s` at the row's ends; text `type-style-body`; figures in `type-style-mono` with tabular figures, right-aligned, as are their column titles |
| Alternating rows | **Yes in tables, no elsewhere.** Every second row of a multi-column table takes `colour-fill` on `colour-view`, inset and rounded as the selection is, so a row of figures can be followed across a wide window; single-column lists, sidebars and grouped rows have none, being short enough to read without. The stripes stay under Increase Contrast |
| Selection, focused | `colour-accent`, text and glyphs `colour-on-accent`, figures too; adjacent selected rows join into one shape, corners only at the run's ends |
| Selection, unfocused or inactive | `colour-selection-unfocused`, text unchanged; stripes show through no row it covers |
| Text selection | In fields and text views, the accent mixed into the surface at `selection-mix`, text staying `colour-text` (the `selection` group) |
| Search match | Matched characters in a filtered list on a `colour-match` run, `radius-small` 2, text unchanged |

### Empty, loading and error states in place

| State | Look |
|---|---|
| Empty | One line, centred in the content it concerns: `type-style-title-3` in `colour-text-2`; at most one regular button `space-m` below. No glyph, no card, no illustration |
| No results | "No results for “{query}”" in the same form; the field keeps its text |
| Loading | The frame (toolbar, sidebar, headings, labels) in the first frame; data fills in place as it arrives, with no fade. Only if nothing has arrived after `timing-progress-delay-ms` (500): a spinner at `size-glyph-large` (20) in `colour-text-2`, centred in the content. A row waiting on one value shows its label and an empty value |
| Missing daemon or device | The sentence where the affected part would be, `type-style-body` `colour-text-2`, centred: in a form, inside a group of its own 80 tall; in a list or table, centred in the view as Empty is. What does not depend on it draws as normal |
| Refused value | Under its row, `type-style-callout` in `colour-danger-text`, `warning-circle` Fill at 12 leading, `space-xs`: "Couldn't change {setting}. {reason}" |
| Failed action | An alert (`window-chrome.md`): title naming what failed, one line on what to do, Try Again as the default where it can help |
| Configuration error | The notice (`settings.md`, Anatomy) at the top of every settings pane; rows bound to `settings.kdl` disabled |
| Work in flight | Progress in the window (each app's own place: Files' jobs button) as a determinate bar 6 tall, `radius-small` 3, track `colour-fill-strong`, fill `colour-accent` |

### Sheets, alerts and popovers

- **Sheets and alerts** are `window-chrome.md`'s: attached under the toolbar,
  `colour-window`, `radius-window` lower corners, the content beneath under
  `colour-scrim`; alerts 320 wide, centred text, equal buttons, the default
  trailing in the accent, destructive in `colour-danger-text` and never
  default, `warning` 32 in `colour-warn` only over what cannot be undone. A
  step inside a sheet replaces its content with a Back button
  (`caret-left`, plain, `size-control`) at its top leading corner. A
  "Don't ask again" checkbox sits leading on the buttons' line.
- **An alert with no window** is a small window of its own: the alert's body
  with `radius-window` on all corners, `shadow-window`, no bar and no
  lights, centred on the screen of the app's last window.
- **Popover.** `colour-window-raised`, `radius-card`, `shadow-menu`, padding
  `space-m`; no arrow: its edge sits `space-xs` from the anchor's, centred on
  it, flipped to fit inside the window's screen, and the anchor stays in its
  pressed form while it is open, which names it. Width to content, at least
  200. Its rows and controls are the grouped rows' own, without the group's
  card; no OK or Cancel.

### The About window

A fixed window `size-about` (280) wide, height to content, `colour-window`.
A server-style bar of `size-titlebar` holding the lights only (minimise and
zoom in their disabled form), no title. Then, centred: the app's icon at
`size-app-icon-large` (64); `space-m`; the name in `type-style-title-2`
`colour-text`; `space-xs`; "Version {v} ({commit})" in `type-style-callout`
`colour-text-2`, the commit in `type-mono`, selectable; `space-m`; the
description in `type-style-callout` `colour-text`, wrapping at 232; `space-m`;
"From {package}" and the licence in `type-style-footnote` `colour-text-2`;
"Source" as a plain button in `colour-accent-text`; `space-xl` to the
bottom. Padding `space-xxl` at the sides.

### An app's settings window

| Part | Look |
|---|---|
| Toolbar | Two rows in one band, no rule between: the lights row (`size-titlebar`, the window's title, the shown pane's name, centred in `type-style-headline`), then the tab row of `size-toolbar-tab` (44); a `colour-hairline` under the band; fill `colour-titlebar` |
| Tab | At least `size-toolbar-tab-width` (64) wide, padding `space-s`, `radius-control`: a glyph at `size-glyph-large` (20) over its label in `type-style-subheadline`, `space-xxs` between, the pair centred. Tabs centred in the row, `space-xs` apart |
| Chosen tab | Plate `colour-fill-strong`; glyph Fill in `colour-accent-text`; label `colour-text`. Others: glyph regular, glyph and label `colour-text-2` |
| One pane | No tab row; the band is the lights row only |
| Body | `colour-window`; the grouped rows of `settings.md` in a fixed column of `size-form-narrow` (480), centred, `space-xl` from the band, `space-xxl` at the sides; window default width 480 + 2 × `space-xxl`, height to the tallest pane, capped at two thirds of the screen, scrolling past it |

### Sliders

Sliders in windows keep the component's accent fill. Sliders on glass
(Control Centre, the island) fill with `colour-text`, as the island's level
bar (`control-centre.md`).

### Focus ring and full keyboard access

- **The ring**: `stroke-focus-ring` (3) in `colour-accent` at 55%, at
  `stroke-focus-offset` (1) outside the control, its radius the control's
  plus the offset, for keyboard focus only, on every control including the
  toolbar's (reached by Tab or ⌃F5).
- **Inside a list, table, sidebar or grid** no ring is drawn around the
  region: focus shows as the selection taking the accent. With nothing
  selected, the first row shows the ring inside its bounds.
- **A whole-row target** (a navigation row, a result) draws the ring inside
  the row, inset `space-xxs`, concentric with its group's corners at the ends
  and `radius-row` elsewhere, so neighbours never clip it.
- **Text fields** show a 1-point `colour-accent` inner edge whenever they are
  editing, by pointer or keyboard; the outer ring is added only when reached
  by keyboard.
- **Segmented controls and radio groups** ring the whole control; the arrows
  move the choice.

## States

| State | What changes |
|---|---|
| Rest, active | Anatomy as above; lights coloured |
| Inactive | Lights neutral; title and toolbar glyphs `colour-text-3`; sidebar and list selections `colour-fill-strong` / `colour-selection-unfocused`; the scroll-edge bar `colour-titlebar-inactive` |
| Empty, no results, loading, missing device, refused value, failed action | Their rows in Anatomy |
| Configuration error | The notice on every settings pane; bound rows disabled (`settings.md`) |
| Work in flight | The window's determinate bar; the switcher card's (`switcher.md`) |
| With a sheet | Content under `colour-scrim`; bar and lights clear; close waits |
| Narrow window | In order: centre into »; title cut at its end; search into its button; sidebar collapsed; then the inspector |
| Overflowing content | The scroll-edge bar over the content; the sidebar's own over the sidebar |
| Full screen | Toolbar kept; corners square; the revealed menu bar slides over it |
| Second screen, locked, Focus on | No change of their own |
| Game Mode on | The same look; every transition below a cut |
| Reduce Motion, Reduce Transparency, Increase Contrast, largest text | Motion and Accessibility |

## Motion

Content rides its spring's progress, never its own clock; exits run on
`motion-exit` and `motion-exit-fade-ms`.

| Transition | How | Reduce Motion |
|---|---|---|
| Sheet, alert | `window-chrome.md`: grows down from the toolbar's lower edge on `motion-open`, out on `motion-exit`; scrim on `motion-fade-ms` | Fade in place, scrim with it |
| Popover | `motion-open` from `motion-panel-scale` (0.96) and opacity 0, anchored at the point of its edge nearest the anchor; out, opacity to 0 over `motion-exit-fade-ms` in place | Fade in place, `motion-reduced-fade-ms` |
| Context and pop-up menus | `menus.md`: whole in the next frame, fading out, the blink kept | As there |
| Sidebar, inspector show and hide | The region's width on `motion-move` from 0 to its width, its rows clipped, not scaled; the content's edge rides with it and lays out each frame. Auto-collapse uses the same | The window's content crossfades from the old layout to the new over `motion-reduced-fade-ms` |
| Edge drag | Follows the pointer, no spring; a double-click springs to the default on `motion-move` | Cut |
| Search field widening | Width on `motion-move` between its two widths; the folded button opens to the field the same way | Cut |
| Toolbar folding | None: items fold and return in the frame the width changes |
| Scroll-edge bar | Follows the scroll offset (Anatomy); no clock | Same |
| Selection, focus ring, hover, pressed | At once | Same |
| Switch knob, segmented plate | `motion-move` from the old place to the new | Cut |
| Progress spinner | Fades in over `motion-fade-ms` after the delay; turns at one revolution a second; goes at once when content arrives | Kept turning: it is the state, not decoration |
| Window open and close | The compositor's (`window-chrome.md`) | As there |

**Game Mode:** every row is a cut; nothing fades.

## Accessibility

- **Surfaces.** Window text sits on `colour-window`, `colour-window-raised`,
  `colour-view`, `colour-sidebar`, `colour-titlebar` and the selections:
  window roles, so `colour-text-3` is allowed, used only for placeholders and
  an inactive window's title and glyphs. On `colour-accent`,
  `colour-on-accent` only. No Isle app surface is glass.
- **Increase Contrast.** `colour-hairline-strong` borders on every control,
  field, segmented plate, the toolbar's lower edge (shown at rest too), the
  sidebar's and inspector's edges, groups, sheets, popovers and the About
  window; the focus ring at full accent; Phosphor bold; table stripes kept.
- **Reduce Transparency.** Windows are opaque already; the toolbar shows its
  scroll-edge bar at all times.
- **Largest text size** (1.5×). The toolbar is the larger of `size-toolbar`
  and the headline and subheadline lines plus 16; rows grow to their line
  plus 12; the sidebar's default and minimum grow by the text factor up to
  `size-sidebar-max`; labels in forms wrap rather than cut; tab labels wrap to
  two lines; the window's minimum grows so nothing is clipped. Glyph boxes
  stay their size.
- **Targets.** Toolbar items 30; sidebar and list rows 28; table header 24;
  edges 8 wide by full height; small buttons 20 (`size-hit-minimum`);
  `x-circle` in the field takes a 20 square.

## Board

Wave B's conventions board at 3440×1440 with the 1920×1080 screen beside it,
dark first and each in light. Windows are drawn at their real size; crops are
about 1600 wide at actual size.

1. **Anatomy**: a 1400×860 window with sidebar, a table in the content and
   an inspector, annotated in points. Crop: the window.
2. **Toolbar groups**: back and forward, title and subtitle, a symbol
   segmented control, a primary action, the More menu open, the inspector
   toggle, search at rest, focused, and with a query. Crop: the toolbar band.
3. **Narrowing**: one window at five widths: whole; » holding the centre;
   title cut; search folded; sidebar collapsed. Crop: the bands stacked.
4. **Sidebar rows**: a Files sidebar with headers and volumes, selection
   focused, unfocused and inactive, a drop target. Crop: three sidebars.
5. **Scroll edge**: the same window at rest and scrolled. Crop: top 300.
6. **Tables and lists**: Monitor's table with stripes, a sorted column, three
   rows selected focused, then unfocused; Keychain's two-line list; a search
   match. Crop 1600 by 900.
7. **States in place**: empty, no results, loading with the spinner, missing
   device in a form and in a list, a refused value, a failed action's alert.
   Crop: six windows tiled.
8. **Sheets, alerts, popovers**: a sheet with a Back step, a destructive
   confirmation with "Don't ask again", an alert with no window, a colour
   popover with its anchor pressed. Crop: each.
9. **About window** and **an app's settings window** (Files Settings:
   General, Sidebar, Advanced) side by side. Crop: both.
10. **Focus**: the ring on a button, a field, a switch, a segmented control, a
    pop-up, a toolbar item reached by ⌃F5, a navigation row, and a focused
    list's selection. Crop: a 1600 strip with a 400% inset of two rings.
11. **Accessibility**: scenes 1 and 6 under Increase Contrast, and scene 1 at
    the largest text. Crops as theirs.
12. **Motion**, on the harness's spring: sidebar hide and show, a popover
    in and out, search widening, a sheet; each under Reduce Motion.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `sidebar-min`, `sidebar-max` | 160, 320 |
| | `inspector`, `inspector-min`, `inspector-max` | 260, 220, 360 |
| | `content-min` | 400 |
| | `form`, `form-narrow` | 600, 480 |
| | `toolbar-search`, `toolbar-search-wide` | 180, 260 |
| | `toolbar-tab`, `toolbar-tab-width` | 44, 64 |
| | `about` | 280 |
| `stroke` | `found` | 2 (a found or target ring) |
| `timing` | `tooltip-ms` | 700 |
| | `progress-delay-ms` | 500 |
