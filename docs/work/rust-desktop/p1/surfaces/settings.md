# System Settings

P1 stage 5, wave B. How System Settings looks and moves, in Frost. What it
is, its panes, states, keys and copy are `structure/settings.md` (binding),
on `structure/conventions.md`; the window's toolbar, sidebar, rows, sheets
and focus are `surfaces/conventions.md`'s and are not restated. Values are
`tokens.kdl` names; numbers not yet tokens are listed at the end.

## Anatomy

### The window

- **Default size** `size-settings-width` by `size-settings-height` (900 by
  760), minimum 784 by 564, which holds the widest sheet (Keyboard
  Shortcuts, 720 by 480) with `space-xxxl` each side and below. On
  the 3440×1440 screen the compositor centres it in the work area the first
  time: x 1270, y 364. It resizes from every edge; width beyond the default
  only adds margin round the column.
- **Regions**: the unified toolbar (`size-toolbar`, 52); the sidebar,
  `size-sidebar` (220), fixed, not collapsible, `colour-sidebar`, the lights
  over its top; the content, `colour-window`, the column `size-form` (600)
  centred in it (40 each side at the default width, never under `space-xxl`).
- **Toolbar**: back and forward `space-l` after the sidebar, then the
  title (the pane's name, or a sub-page's own name: "About"), nothing else.
  Back and forward show `colour-text-disabled` with nothing to go to.

### The sidebar

| Part | Look |
|---|---|
| Search field | Fixed under the lights: top at 52, `space-s` from the sidebar's sides, `size-control-large` (30) tall; the toolbar search field's look (`conventions.md`) |
| Fixed band | The toolbar's height plus the field and `space-m`: it stays as the rest scrolls under it, taking `colour-sidebar` and a `colour-hairline` lower edge once anything has |
| Account row | `space-m` under the field; inset `space-s`, `radius-row`, padding `space-s`: the picture as a 32 circle (`size-app-icon`; with none, initials in `type-style-headline` on `colour-fill-strong`), `space-s`, the full name in `type-style-headline` over the login name in `type-style-subheadline` `colour-text-2`; 48 tall; selected as a sidebar row when Users & Groups shows the user's own account |
| Pane list | `space-m` under the account row; rows of `conventions.md` (28, inset `space-s`), each a pane tile then the name; `space-m` between the six groups; no headings |
| Pane tile | `size-pane-tile` (20) square, its radius a quarter of its side (`radius-row` at 20), filled with the pane's `pane` hue; the glyph in a `size-glyph` (16) box, Phosphor Fill, white. It keeps its hue on the accent selection |
| Changed mark | With Show changed settings on, a `size-mark` (6) dot in `colour-text-2` (`colour-on-accent` on the selection) at the row's trailing end, `space-s` in |
| Scrolling | The account row and panes scroll under the fixed band; the selected pane is scrolled into view, a third of the way down when it was out of it |

**The panes, their glyphs and hues** (Phosphor names; `isle-` glyphs are
Isle-drawn on Phosphor's grid, `icons.md`):

| Group | Pane | Glyph | Hue |
|---|---|---|---|
| 1 | Wi-Fi | `wifi-high` | `pane-blue` |
| 1 | Bluetooth | `bluetooth` | `pane-blue` |
| 1 | Network | `globe` | `pane-blue` |
| 1 | VPN | `isle-vpn` | `pane-blue` |
| 1 | Phone | `device-mobile` | `pane-teal` |
| 1 | Energy · Battery | `lightning` · `battery-high` | `pane-orange` · `pane-green` |
| 2 | General | `gear` | `pane-grey` |
| 2 | Accessibility | `person-arms-spread` | `pane-blue` |
| 2 | Appearance | `circle-half` | `pane-graphite` |
| 2 | Apps | `browsers` | `pane-indigo` |
| 2 | Control Centre | `squares-four` | `pane-purple` |
| 2 | Desktop | `app-window` | `pane-graphite` |
| 2 | Displays | `monitor` | `pane-blue` |
| 2 | Spotlight | `magnifying-glass` | `pane-graphite` |
| 2 | Wallpaper | `image` | `pane-teal` |
| 3 | Notifications | `bell` | `pane-red` |
| 3 | Sound | `speaker-high` | `pane-pink` |
| 3 | Focus | `moon` | `pane-indigo` |
| 4 | Lock Screen | `lock` | `pane-graphite` |
| 4 | Privacy & Security | `hand-palm` | `pane-blue` |
| 4 | Login Password · Fingerprint & Password | `password` · `fingerprint` | `pane-red` |
| 4 | Users & Groups | `users` | `pane-blue` |
| 5 | Calendars | `calendar-blank` | `pane-red` |
| 6 | Keyboard | `keyboard` | `pane-grey` |
| 6 | Mouse | `mouse` | `pane-grey` |
| 6 | Trackpad | `hand-swipe-right` | `pane-grey` |
| 6 | Game Controllers | `game-controller` | `pane-grey` |
| 6 | Printers | `printer` | `pane-grey` |

Hues follow macOS's where it has the pane (the network panes blue, the
input devices grey, Notifications red, Sound pink, Focus indigo), and
neighbours in a group differ where they can, so a pane is found by colour
before its name is read.

### A pane

| Part | Look |
|---|---|
| Top padding | `space-xl` (20) under the toolbar |
| Header | A row: the pane tile at `size-pane-tile-large` (32, radius 8) with its glyph in a 24 box; `space-m`; the name in `type-style-title-2` `colour-text` over the subtitle in `type-style-body` `colour-text-2`, `space-xxs` apart, the subtitle wrapping. A sub-page's header is its own name and sentence, with no tile, or the device's or app's icon at 32 |
| Visual lead | Where a pane leads with its subject (the Displays arrangement, the Wallpaper preview, the Appearance choices), it sits `space-l` under the header, the column's width |
| Groups | `space-xl` under the header or lead, `space-xl` apart; `space-xxxl` at the foot |
| Group | `colour-window-raised`, `radius-card`, a 1-point `colour-hairline` ring |
| Group title | Where a pane's packet names one: `type-style-headline` `colour-text` `space-s` above the group, at the rows' label edge |
| Row | Padding `space-m` (12) at the sides and top and bottom, at least `size-row-large` (40); label `type-style-body` `colour-text` leading; a description only where the label leaves something to say, `type-style-callout` `colour-text-2`, `space-xxs` under; the control trailing, centred on the label's line; label column and control at least `space-l` apart |
| Leading icon | Optional, a device's or app's icon or a pane tile at 20, `space-s` before the label |
| Separator | `colour-hairline`, from the label's leading edge to the row's trailing edge |
| Disabled row | Label `colour-text-2`; the control in its disabled form, still showing its value |

**Rows by kind**:

| Kind | Trailing part |
|---|---|
| Control | One control (below) |
| Navigation | The current value, if any, in `type-style-body` `colour-text-2`, `space-s`, then `caret-right` at `size-glyph-small` (12) in `colour-text-2`. The whole row is the target: pressed `colour-fill` |
| Details | A regular button, "Details…" or "Options…" (`size-control`) |
| List | A table inside the group, edge to edge: rows of `size-row` (28) with an icon at 20, label, a trailing value in `colour-text-2`; selection as `conventions.md`. A foot `size-control` (24) tall on `colour-window-sunken`, a `colour-hairline` above it: + and − as plain icon buttons at `size-control-small` with `plus` and `minus` at 12, a `colour-hairline` 12 tall between them; − disabled with nothing selected. Empty: its line ("No login items") in `type-style-body` `colour-text-2`, centred in a body 80 tall, as Missing daemon in a form. Where order matters, rows reorder by drag in `conventions.md`'s Drag and drop form |
| Provider | `widgets.md`'s provider row in its window form: the provider's icon at 32, `space-m`, name in `type-style-headline` over the state sentence in `type-style-subheadline` `colour-text-2`, one line cut at the end; the on switch trailing; while on, up to two actions as regular buttons under the sentence, the rest under "Options" |
| Action | A regular button named for its act; a destructive one in `colour-danger-text`, asking first in an alert |
| Live | Sampled while shown. A meter: 6 tall, radius 3 (half its height), `size-row-slider` (200) wide, track `colour-fill-strong`, fill `colour-accent` (live), no segments. Figures (SMART, levels) in `type-style-mono` tabular, `colour-text`. A pad: its outline at 160 by 100 in `colour-hairline-strong` strokes, a pressed control filled `colour-accent`, sticks as dots that move |

**Controls by kind** (components from gate 4; sizes in the row):

| Value | Look |
|---|---|
| On or off | Switch 32×18, `colour-fill-strong` off, `colour-accent` on, white knob 14 |
| Two or three choices | Segmented control at `size-control` (24), text segments padded `space-m` |
| A visual choice | Pictures 64×44, `radius-control`, `space-l` apart, each with its name centred `space-xs` under in `type-style-callout`; the chosen one ringed outside in `colour-accent` at `stroke-found`, offset `space-xxs`, its name `colour-text`, the others `colour-text-2` |
| One of more; a duration | Pop-up button at `size-control`, at least 120 wide, label then the accent indicator (components); "Never" last in a duration's menu after a separator |
| A number | A field 64 wide at `size-control`, right-aligned, tabular; the stepper 14×24 `space-xs` after it; the unit `space-xs` after in `colour-text-2` |
| A continuous value | Slider at `size-row-slider` (200): track 4, knob 18; at each end a glyph at 12 or a word in `type-style-callout` `colour-text-2`, `space-s` from the track; ticks under it where only steps are allowed |
| A colour | Swatches 16 across, `space-s` apart, each the colour's fill; the chosen one holds a 6 dot in that colour's `on`; the last, a `colour-fill-strong` circle with `plus` at 12, opens the picker in a popover |
| A file or folder | Its icon at 16 and name in `type-style-body`, cut in the middle, `space-s`, "Choose…" (regular) |
| A chord | The recorder (below) |
| Text | A text field at `size-control`, `size-row-field` (200) wide, committing on Return or leaving |

### Search

- **Results** replace the pane list under the account row: rows of
  `size-row-large` (40), inset as sidebar rows, the pane's tile at 20, then
  the row's label in `type-style-body` over its path ("Displays › Night
  Light") in `type-style-subheadline` `colour-text-2`; matched characters on
  a `colour-match` run. Grouped by pane, `space-m` between groups.
- **The selected result** takes the sidebar's selection (accent while the
  field or list has the keyboard, which it does while searching); its pane
  shows in the content.
- **No results**: "No results for “{query}”" in `type-style-body`
  `colour-text-2`, centred in the sidebar's width, `space-xxl` under the
  field.
- **The found row** is scrolled so its top sits a third of the way down the
  content's visible height, then ringed: a `stroke-found` (2) ring in
  `colour-accent`, inside the row inset `space-xxs`, concentric with the
  group at its ends and `radius-row` between. A row inside a sub-page or
  sheet is ringed there once it has opened.

### Show changed settings

A `size-mark` (6) dot in `colour-text-2` in the column's gutter,
centred `space-m` outside the group's leading edge on the row's label line,
so rows keep their layout; the same dot on the pane's sidebar row.
"Reset to default" is a context menu item (`menus.md`).

### The rejected-edit notice

At the column's top, above the header, `space-xl` above it: a card,
`colour-window-raised`, `radius-card`, a 1-point `colour-warn` ring,
padding `space-m`. `warning-circle` Fill at `size-glyph-large` (20) in
`colour-warn`; `space-m`; the string in `type-style-body` `colour-text`, the
parser's message under it in `type-style-mono` `colour-text-2`, `space-xxs`
apart; then, `space-m` below and aligned to the text, "Open in editor" and
"Show in Files" as regular buttons `space-s` apart. It scrolls with the
pane. The `tokens.kdl` form is the same card with its string, on Appearance
and Accessibility only.

### The Displays arrangement

| Part | Look |
|---|---|
| Map | The visual lead: a card `size-arrangement` (220) tall, the column's width, `colour-window-sunken`, `radius-card`, `colour-hairline` ring. Displays scaled together to fit inside `space-xl` padding, centred; on this machine 0.103, so the ultrawide is 354×148 beside a 1080p screen of 198×111 |
| Display | Its logical rectangle at that scale, `radius-small`, `colour-window-raised`, a 1-point `colour-hairline-strong` ring; its name in `type-style-headline` `colour-text` over its resolution in `type-style-subheadline` `colour-text-2`, centred, the name cut at its end |
| Selected | A `stroke-found` ring in `colour-accent` at offset `space-xxs` outside |
| Island mark | On the main display: a 28×8 capsule in `colour-text`, its top `space-xs` under the display's top edge, centred; a 28×20 hit area. While dragged it carries `shadow-control`. With Follow focus: on the display holding the focused window, `colour-text-2`, not draggable |
| Dragging a display | It follows the pointer 1:1 above the others with `shadow-control`; the edge it will snap to is drawn as a `colour-accent` line, `stroke-found` thick, along that side |
| Disabled displays | Rows in the group under the map: the name, "Off" in `colour-text-2`, its switch |
| Names on the real screens | While the pointer is over the map or a display is being dragged, at each screen's centre in the top layer: a panel on `material-glass` (blur, saturation), `radius-panel`, `shadow-panel`, padding `space-l` by `space-xxl`: the name in `type-style-large-title` `colour-text`, its resolution in `type-style-body` `colour-text-2` under it, centred |

### Sheets

- **Keep or revert**: the alert form (`window-chrome.md`, 320): "Keep these
  display settings?" in `type-style-headline`; "Reverting in {n} seconds."
  in `type-style-callout` `colour-text-2`, the number tabular; Revert
  leading, Keep the default trailing. It counts `timing-display-revert-ms`
  (15 s) down a second at a time.
- **Keyboard Shortcuts…**: a sheet 720 by 480: a list of categories 180 wide
  on `colour-window-sunken` (sidebar rows), a `colour-hairline`, then the
  category's rows (checkbox, action, recorder trailing) as a table of
  `size-row`; "Done" as the default at the foot.
- **The recorder**: a field at `size-control`, at least 120 wide, `colour-fill`,
  `radius-control`, the chord right-aligned in `type-style-body` (macOS
  symbols in Apple's order, or words on a PC profile, `menus.md`); none set,
  "None" in `colour-text-2`. Recording: the field's editing edge and the
  focus ring; "Type shortcut" in `colour-text-2` until a key is held, then the
  held modifiers in `colour-text` as they are pressed. Refused: a 1-point
  `colour-danger` edge and the refused-value line under the row, "Used by
  {action}".
- **Detect** (Keyboard): the row's button reads "Press any key…" in its
  pressed form with the focus ring; the detected device's name replaces the
  value on the first key; Escape cancels.
- **Hot Corners…**: the main display's wallpaper drawn at `size-hot-corners-screen`
  (240) wide in its shape (100 tall on the ultrawide), `radius-small`,
  `colour-hairline-strong` ring; a pop-up button at each corner outside it,
  180 wide, `space-m` from the picture, level with its top and bottom edges;
  sheet padding as sheets; "Done" the default at the foot. A corner set with
  a modifier shows the modifier in `copy.md` rule 11's form before the action
  ("⌘ Mission Control" on a Mac keyboard); none shows "—".

## States

| State | What changes |
|---|---|
| Rest | The shown pane selected in the sidebar (accent while the sidebar has focus, as on opening) |
| Searching | Results in place of the pane list; the selected result's pane in the content |
| Found row | Its ring until the user clicks, types, scrolls or changes pane |
| Empty | The pane's line in its list's body ("No printers"), its + kept |
| Loading | Header, labels and controls at once; a value awaiting its daemon blank |
| Missing daemon or device | The sentence in a group of its own in place of the rows that need it |
| Refused value | The row keeps its value; the refused line under it |
| Rejected hand edit | The notice on every pane; bound rows in their disabled form; daemon rows as normal |
| Changed settings shown | Dots in the gutter and the sidebar |
| Recording a chord, detecting a keyboard | The recorder's recording form; Detect's waiting button |
| Privileged change | The row at its old value under the polkit prompt; "Permission was not given." as the refused line if refused |
| Display change pending | The keep-or-revert sheet |
| Arranging | The names on the real screens; a dragged display or mark lifted |
| Many panes, long pane | The sidebar scrolls under its band; the pane under the toolbar's scroll-edge bar |
| Inactive | `conventions.md`'s inactive form; the sidebar selection `colour-selection-unfocused` |
| Second screen | The map draws both; the window is ordinary on either |
| Game Mode on | Unchanged look; no transitions; the names' panel `material-solid` |
| Locked, Focus on | Nothing / no change |

## Motion

| Transition | How | Reduce Motion |
|---|---|---|
| Pane switch | **A quick crossfade, anchored** (structure): the old pane to 0 over `motion-exit-fade-ms` and the new from 0 over `motion-fade-ms`, starting together; the toolbar title crossfades with it; nothing moves. A new switch mid-fade drops the outgoing pane where it is, so arrowing down the sidebar never queues fades | Cut |
| Sub-page and Back | **The same crossfade, no slide**: the structure fixes it, and a slide would say the page came from beside the pane when it opened in place | Cut |
| Search results | Replace the pane list in place, in the frame the query changes | Same |
| Found row | The content scrolls on `motion-move` to its place; at the scroll's settle the ring springs on `motion-open` from `space-xs` outside the row to its inset place, opacity riding the spring; it leaves by fading over `motion-exit-fade-ms`; it never pulses | Scroll cut; the ring fades in place over `motion-reduced-fade-ms` without growing, and out the same |
| Notice arriving or leaving | Its height on `motion-move`, its content riding the growth, the pane below moving with it | Cut, a crossfade |
| A refused write | The control returns to the value in force on `motion-move` (knob, plate, slider) | Cut |
| Display drop | Snaps flush on `motion-move` from where it was let go; displays pushed out of overlap move on the same spring | Cut |
| Island mark drop | `motion-move` to the new main display's top centre | Cut |
| Names on the real screens | In over `motion-fade-ms`, out over `motion-exit-fade-ms`, in place | Same |
| Keep-or-revert, Hot Corners, Keyboard Shortcuts | Sheets (`conventions.md`); the countdown changes in place | As sheets |

**Game Mode:** every row is a cut, the names' panel too.

## Accessibility

- **Surfaces.** Sidebar text on `colour-sidebar`; pane text on `colour-window`
  and `colour-window-raised`; the map's labels on `colour-window-raised`. The
  names on the real screens are glass: `colour-text` and `colour-text-2`
  only. `colour-text-3` only for placeholders in fields on
  `colour-window-raised`; never on a fill.
- **Pane tiles** are not the only sign: the name is beside every tile, so the
  white glyph on the lighter hues (`pane-orange`, `pane-green`, `pane-teal`,
  under 3:1) is decoration; the tile has no accessible name of its own.
- **Marks.** The changed dot and the found ring pass 3:1 as
  `colour-text-2` and `colour-accent` on window roles; the notice's
  `colour-warn` ring and glyph pass 3:1 on `colour-window-raised`.
- **Increase Contrast.** `conventions.md`'s borders; groups, the map and
  each display ringed in `colour-hairline-strong`; the changed dot gains a
  1-point `colour-text` ring; the names' panel `material-glass-tinted`.
- **Reduce Transparency.** The names' panel `material-solid`; the rest is
  opaque already.
- **Names and descriptions.** A changed row's accessible description is
  "Changed from default"; a pane's sidebar row "{pane}, has changed
  settings". The accent swatches are named by colour ("Blue" … "Graphite",
  "Other colour…"), the chosen one with "selected" as its state; a list's +
  and − "Add {item}" and "Remove {item}"; a stepper's halves "Increase
  {setting}" and "Decrease {setting}"; the Displays map's mark "Island: main
  display".
- **Announcements.** The keep-or-revert sheet is announced on opening
  ("Keep these display settings? Reverting in 15 seconds.") and again at 5
  seconds.
- **Focus.** The sidebar and results show focus as the accent selection; a
  pane's controls take the ring outside; navigation rows and the map's
  selected display take it inside (the display's ring becomes the focus ring
  while the map has the keyboard); the island mark, moved by ⌘ arrows,
  shows the ring round its hit area.
- **Largest text size** (1.5×). The sidebar widens to the longest pane name
  plus the tile and paddings, up to `size-sidebar-max`; rows grow and labels
  wrap; a control too wide for the space left moves under its label,
  trailing-aligned; the header's subtitle wraps; the map keeps its scale and
  its labels drop the resolution line before cutting the name; the window's
  minimum grows with the column.
- **Targets.** Sidebar and result rows 28 and 40; navigation rows their full
  height; swatches 16 drawn in 24 targets; the stepper's halves 14 by 12
  drawn with 20 by 20 targets reaching past the field's edges; the island
  mark 28 by 20; list + and − 20.

## Board

Wave B's Settings board at 3440×1440 with the 1920×1080 screen beside it,
dark first and each in light. The window sits at its default place; crops
are x, y, width, height on the ultrawide, about 1600 wide at actual size.

1. **Default window** on the ultrawide, General shown, sidebar focused. Whole
   screen at fit; crop 920, 300, 1600, 900.
2. **All 28 panes**: the sidebar scrolled through as one tall strip, with
   the alternates (Battery, Fingerprint & Password) and the conditional
   panes (Phone, Trackpad); dark and light side by side. Crop: the strips.
3. **Every row kind** on one pane: header, control, navigation with a value,
   details, a list with its foot and an empty list, a provider, an action, a
   live meter. Crop as scene 1.
4. **Every control kind**: Appearance (pictures, swatches, glass slider with
   words), Desktop (number field and stepper, duration pop-up), Wallpaper's
   folder with Choose…, General's computer name. Crops as scene 1.
5. **Search**: "night" with results grouped by pane, the selected result's
   pane shown; then Return, the found row scrolled and ringed; "zzz" with
   no results. Three crops as scene 1.
6. **Show changed settings**: marked rows and panes, Reset to default open.
7. **Rejected hand edit** on Desktop: the notice, bound rows disabled; Wi-Fi
   working beside it; the `tokens.kdl` notice on Appearance.
8. **Displays**: the map with the island on the ultrawide, the 1080p screen
   selected; mid-drag with a snap edge shown and the names on both real
   screens (whole stage at fit); a disabled display's row; one display alone.
9. **Keep or revert** at 12 seconds, over Displays.
10. **Keyboard Shortcuts**: the recorder at rest, recording with ⌃⌥ held, and
    refused with "Used by Mission Control".
11. **Hot Corners**: the defaults, one corner with a modifier, a pop-up open.
12. **States**: Bluetooth with no adapter, Printers empty, a refused value,
    "Permission was not given.", loading values; the window inactive.
13. **Accessibility**: scenes 1 and 3 under Increase Contrast, and at the
    largest text size.
14. **Motion**, on the harness's spring: a pane switch and a sub-page,
    arrowing quickly down the sidebar, the found ring's arrival, a display's
    snap and the island mark's drop; each under Reduce Motion.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `settings-width`, `settings-height` | 900, 760 |
| | `pane-tile`, `pane-tile-large` | 20, 32 |
| | `mark` | 6 (the changed dot; also the tray's attention dot in `island.md`) |
| | `row-slider`, `row-field` | 200, 200 |
| | `arrangement` | 220 |
| | `hot-corners-screen` | 240 |
| `timing` | `display-revert-ms` | 15000 |

With `conventions.md`'s `form`, `sidebar-max` and `stroke-found`.
