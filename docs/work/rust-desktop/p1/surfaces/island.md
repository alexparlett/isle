# Island

P1 stage 5, wave A. The look and motion of the island and the panels that
unfold from it: the pill and its forms, Control Centre's frame, grid and
pages, and Notification Centre's frame. Structure is binding from
`structure/island.md`, `structure/control-centre.md` (sections, grid,
pages; wave B designs each module's insides), `structure/notification-centre.md`
(its unfolding and the morph; its content is wave C) and `structure/focus.md`
(the Focus glyph). Values are `tokens.kdl` names; numbers not yet tokens
are listed at the end.

## Anatomy

### One shape

Every form below is one shape the compositor draws and springs: a rounded
rectangle whose top edge sits at `size-island-top` (9) and whose centre is
the screen's horizontal centre, always. Its radius is derived from its
height every frame, never sprung on its own: half the height, capped at
`radius-panel` (18), so a 30-tall pill is a capsule and anything taller
than 36 has panel corners and keeps text out of the curve. Content is
clipped to the shape.

| Form | Fill | Shadow | Edge |
|---|---|---|---|
| Pill, compact, events, the start row | `colour-island` | `shadow-menu` | none |
| Control Centre, Notification Centre | `material-glass` at the user's glass position, `material-blur`, `material-saturation`, `material-noise-opacity` | `shadow-panel` | none |
| Privacy form | `material-solid` | `shadow-menu` | none |

Between the pill and a panel the fill and shadow blend with the growth
(Motion). The island's text is `colour-island-text`; secondary text in the
pill is `colour-text-2` under the glass rule; `colour-text-3` never
appears on any form.

**Wells.** Colour in the pill is never drawn on the translucent fill: over
a white window a privacy or live colour on `colour-island` falls to about
2:1. Anything coloured sits in a well: a capsule `size-island-well` (22)
tall, `material-solid`, inset `space-xs` (4) from the shape's top and from
its edge where it meets one, radius 11 (the pill's 15 less 4, concentric).
On `material-solid` every privacy, live and status colour passes 3:1 in
both appearances.

### The pill at rest

| Part | Look |
|---|---|
| Strip | `size-strip` (48) across every screen's top, nothing drawn in it but the island; the pill is centred in it, 9 above and below |
| Pill | `size-island-height` (30) tall, width to content, at most `size-island-max` (560). Leading and trailing inset `space-m` (12), or `space-xs` where a well or the art meets the end |
| Row order | Leader's face, other activities' marks, clock, status glyphs, privacy well. Gaps: `space-s` (8) between face and marks and between marks and clock; `space-m` (12) from the clock to the first glyph; `space-s` between glyphs and before the well |
| Clock | `type-style-clock` (13 semibold, line 16), tabular figures, `colour-island-text`. Its box is the width of the format's widest string ("88:88", or with weekday, date, seconds as set), text centred in it, so nothing beside it moves as the time does. Clock alone: a pill of about 64 |
| Status glyphs | Phosphor at `size-glyph` (16), `icon-weight` (Regular) unless named, `colour-island-text`. Monochrome: severity shows by shape, never colour. Each glyph's hit box is 24 wide, the pill's height |
| Privacy well | Trailing, in the well above; marks at `size-glyph-small` (12), Fill weight, `space-xs` apart, the well padded so one mark sits in a 22 circle. Present only while a capture runs |
| Hover zone | The shape, the strip above it to the screen's top edge, and `size-island-guard` (24) each side; never over a revealed bar's titles |

**Status glyphs, in their fixed order.** Levels are drawn in two layers
(icons.md): the full glyph in `colour-text-disabled`, the level glyph over
it in `colour-island-text`.

| Glyph | Forms (Phosphor name) |
|---|---|
| Wi-Fi | On Wi-Fi: `wifi-low`, `wifi-medium`, `wifi-high` over `wifi-high` (signal under 40, under 70, above). Connecting: `wifi-none` over `wifi-high`. No connection at all: `wifi-slash` |
| Bluetooth | A device connected: `bluetooth-connected` |
| Sound | Output muted: `speaker-slash` |
| VPN | Connected: `isle-vpn` (Isle-drawn, shield over the network) in Fill. Connecting: `isle-vpn` Regular in `colour-text-2` |
| Focus | The mode's symbol in Fill: `moon` (Do Not Disturb), `game-controller` (Game Mode), `briefcase`, `bed`, `user`, `book-open`, `car`, `barbell`, `code` |
| Updates | Installing: `arrow-circle-up`. Restart owed: `arrows-clockwise` |
| Battery | On battery: `battery-full`, `-high`, `-medium`, `-low` (above 80, 60, 35, 15%). Low: `battery-warning`. Low and charging: `battery-charging`. No percentage text; it is in the accessible name and Batteries |
| Shortcuts inhibited | `isle-keyboard-slash` (Isle-drawn: keyboard with a slash) |
| Configuration error | `warning-circle` in Fill |

**Privacy marks.** Microphone `microphone`, camera `video-camera`, screen
`screencast`, each in Fill and its colour role (`colour-privacy-microphone`,
`-camera`, `-screen`), in that order leading to trailing. Shape tells them
apart without the colour.

### Compact live faces

The leader's face sits at the pill's leading end, at most
`size-island-face-max` (160) wide. Faces holding colour sit in a well at
the leading end (inset 4, radius 11), the same well the trailing marks use.

| Kind | Face |
|---|---|
| Media | Cover art 22×22 at the leading end, inset 4, radius 11 (a circle, by the concentric rule); `space-s`, then `isle-equaliser` (three bars, Isle-drawn) at 12, still, `colour-island-text`; paused, `pause` at 12. Art not yet loaded: a `colour-fill-strong` circle with `music-note` 12 |
| Recording | A well: an 8-point dot in `colour-live`, `space-xs`, the elapsed time ("0:42") in `type-style-clock`, tabular, `colour-text`, padded `space-s`. It stands in for the screen mark, which leaves the trailing well |
| Sharing | A well: `screencast` 12 Fill in `colour-privacy-screen`, `space-xs`, the sharing app's icon at `size-app-icon-small` (16); then, outside the well, the app's name in `type-style-body`, cut at the face's maximum. It stands in for the screen mark |
| Prompt waiting | The prompt's kind at 16 in Fill: `lock` (polkit, keyring), `wifi-high` (Wi-Fi secret), `isle-vpn` (VPN secret), `bluetooth` (pairing); then the asking app's icon at 16 |
| Call | A well: `phone` 12 Fill in `colour-ok`, `space-xs`, the elapsed time in `type-style-clock`, tabular |
| Timer | `timer` 16, `space-xs`, the time left in `type-style-clock`, tabular; with several running, the soonest, then "+{n}" in `type-style-subheadline`, `colour-text-2` |

**Several live.** After the leader's face, one mark per other activity in
queue order, at `size-glyph-small` (12), Fill, `colour-island-text`,
`space-xs` apart: `lock` (prompt), `record`, `screencast`, `phone`,
`timer`, `music-note`. Past `limit-island-marks` (2) every other mark
gives way to one count, "+{n}", in `type-style-subheadline` semibold,
`colour-island-text`, in a `colour-fill-strong` capsule 16 tall, padded
`space-xs`.

**Overflow.** Past `size-island-max` the leader's text cuts at its end,
then the face drops to its kind's mark, then the marks become the count.
The clock, glyphs and privacy well never drop.

### Events

An event replaces the face, clock and glyphs for its duration; the privacy
well stays at the trailing end, top inset 4, in every event form.

| Form | Geometry | Used by |
|---|---|---|
| Line | `size-island-height` (30) tall, width to content; inset `space-m`; glyph at 16, `space-s`, text in `type-style-body` medium (`type-weight-medium`), `colour-island-text` | Copied, mode, attention, ended, timer done, microphone mute, keyboard layout |
| Level | `size-island-event` (48) tall, `size-island-osd` (300) wide; inset `space-l` (16) at the sides, `space-m` (12) top and bottom; glyph at `size-glyph-large` (20), centred; `space-m`; a column of the device's name (`type-style-subheadline`, `colour-text-2`, cut at its end) over the bar, `space-xs` between | Volume, brightness, keyboard brightness |
| Start row | `size-panel` (376) wide; the Now leader row (below) inside an inset of `space-s`, so 72 tall | An activity starting that leads |

- **Level bar.** 6 tall, radius 3, the column's width; track
  `colour-fill-strong`, fill `colour-island-text`. Continuous, no
  segments, no percentage. Volume's glyph is `speaker-none`, `-low`,
  `-high` over `speaker-high` (0, under 50%, above); muted, `speaker-slash`
  with an empty track and the name line reading "Muted". Brightness is
  `sun-dim` under 50% and `sun` above; keyboard brightness `keyboard`.
- **Line events.** Copied: `check`; "Password copied" `key`; "Colour
  copied" leads with a 16 circle of the colour itself, ringed in
  `colour-hairline-strong`. Mode: the Focus symbol, Fill for "on", Regular
  for "off" ("Game Mode on" uses `game-controller`). Attention: the app's
  icon at 16 and "{app} needs your attention"; urgent looks the same and
  holds. Ended: `record` or `screencast` Regular and "Recording stopped:
  the window closed". Timer done: `timer` Fill and "{label} done", held.
  Microphone mute: `microphone-slash` and "Muted", unmute `microphone` and
  the device's name. Keyboard layout: `keyboard` and the layout's name.

### Control Centre

| Part | Look |
|---|---|
| Panel | `size-panel` (376) wide, never narrower than the pill: when the pill is wider, the panel takes its width and the column stays centred. Padding `space-s` (8) on every side, so a card in a corner is `radius-card` (18 less 8, concentric). Grows down to the output's height less `size-strip` and `space-xxxl` (32), then scrolls between the pill's row and the footer, which stay |
| Pill's row | The pill's own row, unchanged and in the same screen position: the panel opens around it. Clock, glyphs and well stay clickable. `space-s` below it |
| Sections | Now, grid, tray, footer, top to bottom, `space-s` apart. No headings: each section is its own shape |
| Now | A card (`colour-fill`, `radius-card`), rows edge to edge in it, no rules between |
| Leader row | 56 tall: inset `space-m`, the kind's icon or the app's at `size-app-icon` (32), `space-m`, two lines (`type-style-headline` `colour-text`; `type-style-subheadline` `colour-text-2`, tabular where a time), then the row's actions as small buttons (`size-control-small`, `space-xs` apart). Recording's 32 is a well circle with the 8 `colour-live` dot; sharing lists its streams under the lines as `size-row` sub-rows, each with Stop |
| Collapsed row | `size-row` (28): inset `space-m`, glyph or icon at 16, `space-s`, one line `type-style-body` cut at its end, its one action as a small button, `caret-down` at 12 to expand |
| Privacy row | `size-row`: the mark at 12 in a `size-island-well` circle, `space-s`, "{app} is using the microphone" |
| Notice rows | Configuration error: `warning-circle` Fill 16, the message in `type-style-callout` (file, line and column in `type-mono`), then "Open in editor" and "Show in Files" as small buttons on a line of their own. Shortcuts inhibited: `isle-keyboard-slash`, the line, the escape chord in `colour-text-2`, "Take keys back" |
| More | Past `limit-now-rows` (4), one collapsed row "{n} more", expanding in place, "Show less" |
| Grid | Four columns of `size-module-cell-width` (84) by `size-module-cell-height` (64), gaps `space-s`: an inner width of 360 |
| Tray | A card, padding `space-s`: items in boxes of `size-row` (28), `space-xs` apart, wrapping; icons at `size-app-icon-small`. Attention: a 6 dot in `colour-accent`, ringed 1 point in `material-solid`, on the box's top trailing corner |
| Footer | `size-row` tall, no card: "Edit controls" leading and "Control Centre settings…" trailing, plain small buttons in `type-style-callout`, `colour-text-2`; in edit mode "Done", a default button |

**A module's frames.** Every module is a tile: `colour-fill`,
`radius-card`. Its glyph is a `size-module-glyph` (32) circle holding the
glyph at `size-glyph-large` (20), 24 at Large and Wide: on, `colour-accent`
with `colour-on-accent` and Fill; off, `colour-fill-strong` with
`colour-text` and Regular. Hover `colour-fill-strong` tile; pressed
`colour-control-pressed`; disabled, glyph and text `colour-text-disabled`.

| Size | Frame | What wave B fills |
|---|---|---|
| Small | 84×64; the circle centred | Nothing else; name in the tooltip |
| Medium | 176×64; circle at inset `space-l` (16 on every side); text from 60: name `type-style-headline` `colour-text`, state `type-style-subheadline` `colour-text-2`, each one line, cut at its end | The state line's wording |
| Large | 176×136; Medium's top row with inset `space-m`, a content box of 152×72 below it, `space-s` under the row | Art and transport, device lists |
| Wide | 360×64; circle at inset `space-l`, name above a content line 24 tall (`size-control`) from 60 to 344 | The slider or the short row |

**A page's frame.** The page replaces Now, grid and tray; the pill's row
stays above, the footer below. Head: the module as a Wide tile with a back
button (`caret-left` 16, plain icon button, `size-control`) in its leading
slot before the circle, its switch or slider trailing. Body: a card of
rows, `size-row` for one line and `size-row-large` (40) for two; group
headings in `type-style-subheadline` semibold `colour-text-2`, `space-m`
above and `space-xs` below, never inside the card; a chosen row's
`check` 12 in Fill, `colour-text` (the accent's text form is for
windows, not glass); a sub-page's `caret-right` 12 trailing,
`colour-text-2`. Foot: "{Module} Settings…" as a plain button, leading. A sub-page uses the same frame with
its row as the head. The page scrolls under its head.

**Edit mode.** Remove badges, 16 circles of `colour-control` holding
`minus` 12, centred on each tile's top leading corner. The gallery is a
second shape of the panel's width and material at the same top, `space-s`
to the panel's leading side; its insides are wave B's.

### Notification Centre

The same shape, `size-panel` wide: the pill's row on top, `space-s`, then
wave C's column at an inner width of 360, which is also the notification
card's width, so a banner and its entry here are one width. Same maximum
height and scrolling as Control Centre.

### The privacy form

Over a full-screen window and on the lock: the shape shrunk to the well
alone, `size-island-well` (22) tall, its top at `size-island-top`, centred,
`material-solid`, the marks as in the pill. OSDs take the Level form at the
same anchor. With no capture running nothing is drawn.

## States

| State | What changes |
|---|---|
| Rest | The clock alone, a pill of about 64; glyphs widen it by 24 each |
| Rest, privacy | The well joins the trailing end |
| One live | The face leads; the pill grows by its width |
| Several live | Face, marks or count, then clock, glyphs, well |
| Event | The event form; the well kept; then back to the live state |
| Peeked | Control Centre; no focus ring anywhere; looks the same as pinned |
| Pinned | The same; from Ctrl+F8, the ring on the first module; from a click, no ring until a key |
| Page, sub-page | The page's frame; the pill's row and footer kept |
| Edit mode | Badges, the gallery beside, "Done" in the footer; Now and tray folded |
| Notification Centre | Its frame under the same row |
| Empty | Rest; the panel is the row, grid and footer |
| Loading | Media without art draws the art's placeholder; a module shows its last state, controls in `colour-text-disabled` until the first answer |
| Error | A dead service's glyph or face is absent; its module reads "Not available", disabled |
| Overflow | The steps under Overflow; the panel scrolls with row and footer fixed |
| Second screen | The 1920×1080 island: clock, glyphs, well; faces and events stay on the main screen, except that monitor's brightness OSD. Its panel is the main screen's design at the same size |
| Windowed game, Game Mode | Every form `material-solid`, no blur; the Focus glyph `game-controller`; no transitions |
| Full screen | The privacy form alone, or nothing; OSDs in the Level form; the island returns at its anchor on a push |
| Locked | The privacy form and OSDs over the lock, nothing else |
| Focus on | The mode's symbol among the glyphs |
| Shortcuts inhibited, configuration error | Their glyph; their notice row in Now |
| Reduce Motion, Reduce Transparency, Increase Contrast, largest text | See Motion and Accessibility |

## Motion

The compositor springs the shape's width and height (B7); the radius
follows the height. The anchor is the top edge and the horizontal centre:
the shape's left edge is always centre less half the width, so nothing
re-centres or slides. A change mid-spring retargets from the current size
and velocity.

**Content follows the growth.** Each move has a progress p from 0 to 1,
the share of the size change done (on whichever of width and height
changes more). Outgoing content's opacity is 1 − 2p, reaching 0 at half;
incoming content's is 2p − 1, from half to the end. Nothing fades on its
own clock. Content that is in both forms (the pill's row into a panel, the
well in every form) does not fade and does not move on screen, except the
well, which rides the trailing edge.

| Transition | Spring | From → to |
|---|---|---|
| Glyph, face or mark appears, rest → compact, compact → event | `motion-morph` | The current pill → the new width (and height for Level or Start row) |
| Anything shrinking: glyph leaves, event ends, face goes | `motion-exit` | Back to the live form; quicker than the growth |
| Face changes at the same size | none | A crossfade over `motion-fade-ms` |
| OSD repeat | none | The level bar's fill moves on `motion-exit`, so it keeps up with key repeat; the duration restarts |
| Activity start | `motion-morph` | Pill → Start row; after its time (`timing-event-ms`, media `timing-start-media-ms`), `motion-exit` to the compact form |
| Control Centre opens (peek or pin) | `motion-open` | Pill → panel. The fill blends `colour-island` → `material-glass` and the shadow `shadow-menu` → `shadow-panel` by p; sections fade in by p from half |
| Control Centre closes | `motion-exit` | Panel → pill; sections out by the first half |
| Page and back | `motion-open`; back `motion-exit` | The panel's height changes; the page is revealed by a clip growing from the module's tile to the body, while the grid fades by p; back closes the clip onto the tile |
| Control Centre ↔ Notification Centre | `motion-open` if taller, `motion-exit` if shorter | Height only (one width); the column below the row crossfades by p |
| Edit mode | `motion-open`; Done `motion-exit` | The gallery grows from the panel's leading edge, its top fixed; badges fade by its p |
| Return over full screen | `motion-open`; leaving `motion-exit` | With no capture, the pill comes down from above the screen's top to its place and leaves back up, with the bar when it comes too (`menus.md`); with a capture, it grows out of the privacy form, which stays put |
| Pill → privacy form | `motion-exit` | Width and height to the well's; the clock and glyphs out by the first half |

**Durations.** OSD `timing-osd-ms`; copied `timing-copied-ms`; mode
`timing-mode-ms`; attention `timing-event-ms`; start `timing-event-ms`,
media `timing-start-media-ms`; ended `timing-ended-ms`; a timer's end and
urgent attention hold. The hover guards are `timing-hover-dwell-ms`,
`timing-hover-grace-ms` and `timing-hover-settle-ms`; the return over a
game is `timing-edge-dwell-ms` (`menus.md`).

**Reduce Motion.** No spring moves the shape. Old form and new crossfade in
place at the anchor over `motion-reduced-fade-ms`; the page's clip and the
gallery's growth are the same crossfade. The level bar jumps.

**Game Mode.** No transitions at all: each form replaces the last in one
frame, no fade, no blend; `material-solid`, no blur, no noise. The return
over a game is in place, not a slide.

## Accessibility

- **Colour roles.** Pill text `colour-island-text`; secondary
  `colour-text-2`, which passes over grey and the appearance's own extreme
  (7.4 and 5.8 in dark, 5.1 and 4.5 in light). Panel text `colour-text` and
  `colour-text-2` on `material-glass`; tiles on `colour-fill` over it.
  `colour-text-3` is on no form here. Colour marks sit on
  `material-solid` wells (Anatomy).
- **Increase Contrast.** Every shape gets a `stroke-hairline` edge in
  `colour-hairline-strong`; panels use `material-glass-tinted`; glyphs step
  to `icon-weight-contrast` (Bold); tiles and the Now card get the same
  edge; a module's on and off differ by Fill against Regular as well as
  colour; level bars' tracks take `colour-hairline-strong`.
- **Reduce Transparency.** Pill and events `material-solid` (the wells
  then match it); panels `material-solid`, no blur.
- **Focus ring.** `stroke-focus-ring` in the accent, `stroke-focus-offset`
  out, keyboard only. In the pill's row the ring is a capsule 30 tall
  around a 22 box (the item and `space-xs` each side), concentric with the
  pill and never clipped by it. In the panel: round tiles at `radius-card`
  plus the offset, round rows inside the card, round buttons at their own
  radius; the panel's 8 padding leaves room for every ring.
- **Hit targets.** Pill items are the pill's full height and at least 24
  wide; tray boxes 28; buttons `size-control-small` and up; badges 16 with
  a 24 hit box. All at or above `size-hit-minimum`.
- **Largest text size** (assumed 1.5×; Gaps). The pill is the clock's line
  plus 14: 38 tall, the strip 56 with it, the menu bar's reserve growing
  to match; glyphs step to 24, privacy marks to 16, wells to 30. Activity
  text drops before marks. Cells scale with the text (126×96), so the
  panel is 544 wide; Medium's state line may take two lines; Small keeps
  no text.

## Board

At 3440×1440 and 1920×1080, dark first and each in light, over a light and
a dark wallpaper. Crops (harness `[x0, y0, x1, y1]`): pill scenes
`[920, 0, 2520, 300]`, panel scenes `[920, 0, 2520, 900]`, both 1600 wide
at top centre; the second screen `[160, 0, 1760, 400]`.

1. **Rest**: the clock alone; with Wi-Fi medium and a Focus.
2. **Glyph sheet**: one pill per glyph form, over a white and a black window.
3. **Privacy**: all three marks, over a white window (the well's contrast).
4. **Faces**: each kind, media with and without art.
5. **Several live, overflow**: two marks; the count; the widest pill with
   the bar revealed behind it.
6. **Events**: volume, muted, brightness, each copy, Game Mode on,
   attention, the start row (media, recording), ended, timer done.
7. **Peek**: the default arrangement, a recording leading, a privacy row,
   a notice row, a tray.
8. **Pinned by Ctrl+F8**: the ring on Wi-Fi, then on the clock.
9. **Module frames**: four sizes; on, off, hover, focus, disabled.
10. **Page**: Bluetooth's frame and a sub-page.
11. **Notification Centre**: its frame, and mid-morph from Control Centre.
12. **Second screen**: clock, glyphs, privacy, its brightness OSD.
13. **Full screen**: the privacy form over video, an OSD, the return.
14. **Lock**: the privacy form and a volume OSD.
15. **Windowed game**: the solid pill and panel.
16. **Settings**: Increase Contrast, Reduce Transparency, largest text.
17. **Motion**: rest → event → rest, rest → face → several, peek and close,
    page and back, Control Centre ↔ Notification Centre, the return; each
    with Reduce Motion and Game Mode.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `island-well` | 22 (wells, the media art, the privacy form) |
| `size` | `island-max` | 560 (the pill's widest; the menu bar's reserve) |
| `size` | `island-guard` | 24 (hover zone each side; the bar's guard gap) |
| `size` | `island-face-max` | 160 |
| `size` | `island-event` | 48 (the Level form's height) |
| `size` | `island-osd` | 300 (the Level form's width) |
| `size` | `panel` | 376 (Control Centre, Notification Centre, the start row) |
| `size` | `module-cell-width` | 84 |
| `size` | `module-cell-height` | 64 |
| `size` | `module-glyph` | 32 (a module's glyph circle) |
| `timing` | `mode-ms` | 2000 |
| `timing` | `start-media-ms` | 4000 |
| `timing` | `ended-ms` | 5000 |
| `limit` (new group: counts, plain numbers) | `island-marks` | 2 |
| `limit` | `now-rows` | 4 |

Shared with `menus.md`, not new here: `timing-edge-dwell-ms`, `size-edge-push`.
Isle-drawn glyphs added to icons.md's list: `isle-equaliser`.
