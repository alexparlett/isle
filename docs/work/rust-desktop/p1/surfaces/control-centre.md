# Control Centre

P1 stage 5, wave B. The insides of Control Centre: each module in each
size it supports, each module's page, the tray's menus as pages, edit mode
and its gallery, and their motion. Structure is binding from
`structure/control-centre.md` (the sixteen modules, sizes, defaults,
pages, edit mode, keys, copy) and `structure/focus.md` (the Focus module
and page). The panel, the grid, the four module frames and the page frame
are `surfaces/island.md`'s and are not restated. Values are `tokens.kdl`
names; numbers not yet tokens are listed at the end. Positions inside a
tile are from its top-leading corner.

## Anatomy

### Rules every module keeps

- **The glyph circle.** `size-module-glyph` (32), glyph `size-glyph-large`
  (20) at Small and Medium, 24 at Large and Wide. On: `colour-accent`,
  glyph `colour-on-accent`, Fill. Off: `colour-fill-strong`, glyph
  `colour-text`, Regular. A module whose glyph is only a mark (Display,
  Energy Mode, Updates, Batteries, Keyboard layout, Accessibility) always
  takes the off form: the accent is for a switch that is on.
- **Levels in a circle** (Sound, Batteries) are drawn in two layers as on
  the island: the full glyph in `colour-text-disabled`, the level glyph
  over it in the glyph's colour.
- **Medium text.** Name `type-style-headline` `colour-text`, state
  `type-style-subheadline` `colour-text-2`, the pair centred on the tile's
  height (name 17–33, state 33–47), from 60 to 160, each cut at its end;
  the full state is in the tooltip.
- **Hit boxes.** At Small the whole tile is the glyph's action. At Medium,
  Large and Wide the glyph's box is the tile's leading 54 by its full
  height (Large: the top row); the rest opens the page.
- **Tooltip** on a Small tile, and on any cut state: the tooltip component
  on `material-solid` (no glass on glass), "{Module}: {state}", `space-xs`
  under the tile, after `timing-tooltip-ms` (700), as every tooltip.
- **Transitional states** (Connecting, Pairing): the circle takes the
  target form at once; the state line says the rest. No spinner in a tile.
- **A failed glyph action**: the circle returns to the true state and the
  state line reads the error's first clause ("Couldn't connect") for
  `timing-event-ms`, then the true state; the whole sentence is in the
  tooltip and the accessible name.
- **Unavailable** (service dead) and **loading** (no answer yet): circle
  `colour-fill`, glyph and name `colour-text-disabled`; the state line
  stays `colour-text-2` so "Not available" or the last known state reads.

### Sliders

The slider component, laid in a content line `size-control` (24) tall:
track 4, radius 2, `colour-fill-strong`; knob 18, white, the component's
shadow. Sliders on glass (Control Centre, the island) fill with
`colour-text`, as the island's level bar; sliders in windows keep the
component's accent (`conventions.md`; Choices 1). Its hit box is the line's
full height.
Muted or unset: the fill keeps its length in `colour-text-disabled`. The
scrubber is the same track with no knob at rest; on hover, focus or drag a
`size-scrubber-knob` (12) knob appears at the position.

### The modules

| Module | Small (84×64) | Medium (176×64) | Large / Wide |
|---|---|---|---|
| Wi-Fi | `wifi-high`; on while the radio is on | "Wi-Fi"; "{network}", "Not connected", "Connecting", "Off" | — |
| Bluetooth | `bluetooth` | "Bluetooth"; "{device}", "{n} devices", "On", "Off" | — |
| Now Playing | `pause` playing, `play` paused, Fill, always the off circle; no player: `music-note`, disabled | Art 32 in the circle's slot; title (headline) and artist (subheadline `colour-text-2`) from 60 to 132; play/pause, a plain icon button 24 with a 20 Fill glyph, at 136–160. Nothing playing: `music-note` circle, "Now Playing", "Not playing" | Below |
| Focus | Off: the last-used mode's symbol (`moon` until another mode is used), Regular, since the click turns that mode on. On: the on mode's symbol (`focus.md`), Fill | Off: "Focus", "Off". On: the mode's name, and on the state line "Until {time}" or how it ends or why ("For 47 min", "Until the end of {event}", "Schedule", "{app}", "{game}"); on by hand with no end, no state line (`focus.md`) | — |
| VPN | `isle-vpn`; on while connected or connecting | "VPN"; "{name}", "Not connected", "Connecting" | — |
| Sound | Level in two layers: `speaker-none`, `-low`, `-high` over `speaker-high`; muted `speaker-slash`. Always the off circle: muting is not "on" | "Sound"; the output's name, "Muted" | Wide: the circle (mute) at 24; name line "Sound", then the output's name as a second run, `space-s` after, subheadline `colour-text-2`, cut; the slider across the content line, 60 to 344 |
| Display | — | `sun-dim` under 50%, `sun` above; "Display"; this screen's name | Wide: as Sound, the circle a mark; the slider is this screen's. No DDC: the content line reads "Brightness can't be set on this display", subheadline `colour-text-2` |
| Night Light | `sun-horizon` | "Night Light"; "On until {time}", "Off until {time}", "Off during Game Mode" | — |
| Energy Mode | The mode's glyph: `leaf` Low Power, `gauge` Automatic, `lightning` High Power | "Energy Mode"; "Low Power", "Automatic", "High Power", "High Power while Game Mode is on" | — |
| Updates | `arrow-circle-up`; restart owed `arrows-clockwise`. Waiting: the quiet badge (16 tall, count) centred on the circle's top trailing point. Installing: a progress arc round the circle (below) | "Updates"; "Up to date", "{n} updates", "Installing {n} of {m}", "Restart to finish updating"; the arc as at Small, no badge | — |
| Batteries | The lowest battery's level in two layers (`battery-full` … `-low` over `battery-full`); low `battery-warning`; charging `battery-charging` | "Batteries"; the lowest, "{device} {n}%", or "Charging" | Large: below |
| Dark Mode | `circle-half`; on while dark | "Dark Mode"; "On", "Off" | — |
| Keyboard layout | `keyboard` | "Keyboard layout"; the layout's name | — |
| Accessibility | `person-arms-spread` | "Accessibility"; the one on, "{n} on", or "Off" | — |
| Screenshot | `selection-plus`; opens the capture toolbar | — | — |
| Phone | `device-mobile`; on while the provider reports the phone connected | "Phone"; the provider's state sentence, cut | — |

**Progress arc** (Updates installing): a ring on the circle's edge,
`stroke-progress` (2) wide, inside the 32: track `colour-fill-strong`,
arc `colour-accent` from twelve o'clock, clockwise. Installing is live, so
it carries the accent.

**Now Playing, Large** (176×136, the default). Top row at inset `space-m`:
the art 32×32, `radius-small`, in the circle's slot; title (headline) and
artist (subheadline `colour-text-2`) from 56 to 164. Content box 152×72
from 52:

| Part | Look |
|---|---|
| Scrubber | 52–64, the full box width; track centred at 58. Absent for a live stream (its space kept); drawn without a knob and not draggable when the player cannot seek |
| Transport | 68–98: `skip-back`, play/pause, `skip-forward` as plain icon buttons `size-control-large` (30), glyphs Fill `colour-text` at 20, 24, 20, `space-l` apart, centred |
| Player | 104–120, centred: the player's icon at `size-app-icon-small` (16), `space-xs`, its name in subheadline `colour-text-2`. A plain button: it raises the player |

Art not yet loaded: `colour-fill-strong` with `music-note` 16, as the
island's placeholder. Nothing playing: the `music-note` circle, "Now
Playing", "Not playing"; the transport drawn in `colour-text-disabled`, no
scrubber, no player line.

**Now Playing, Wide** (360×64): art 32 in the circle's slot; the name line
is the title, then the artist as a second run in `colour-text-2`; the
content line holds the scrubber from 60 to 252 and the transport at 264–344
(three plain icon buttons 24, glyphs 20 Fill, `space-xs` apart). The
player's name is in the tooltip and the accessible name.

**Batteries, Large**: Medium's top row at inset `space-m`; the content box
holds up to four rows 18 tall, lowest level first: the kind's glyph 12 in
`colour-text-2` (`mouse`, `keyboard`, `headphones`, `game-controller`,
`laptop`), `space-xs`, the name in `type-style-callout` `colour-text`, cut;
trailing, `lightning` 12 when charging, then "{n}%" in callout, tabular,
`colour-text-2`. Low: `battery-warning` 12 replaces the kind's glyph and
the figure turns `colour-text` semibold. Past four, the fourth row reads
"{n} more".

### Pages

The page frame is `island.md`'s: a head (the module as a Wide tile with
back), a body of cards and group headings, a foot. What this sheet adds:

- **Head.** Back (`caret-left` 16, plain icon button 24) at 8; the circle
  at 36, glyph 24; name and state from 80; the trailing control at inset
  `space-l`: the switch component (32×18) for Wi-Fi, Bluetooth and VPN;
  for Sound, the slider in the content line from 80 to 344 under the name.
  Where the trailing switch is the control, the head's circle is a mark
  and takes no click, so one control does one thing; Sound's circle stays
  its mute. The Focus and Display heads carry no switch or slider: their
  rows do that job (Choices 2).
- **Rows.** Edge to edge in the card, `size-row` (28) for one line,
  `size-row-large` (40) for two (every Focus row is 40, `focus.md`). Inset `space-m`; a leading slot (glyph 16
  in `colour-text`, a 24 circle, or an icon), `space-s`, the label in
  `type-style-body` `colour-text`, its second line in subheadline
  `colour-text-2`; then trailing detail (`colour-text-2` runs, the lock,
  a battery), then the trailing control (check, switch, button, chevron),
  inset `space-m`. Hover `colour-fill-strong`, clipped by the card;
  pressed `colour-control-pressed`.
- **Device circle.** A 24 circle holding the kind's glyph at 16: connected
  or chosen, `colour-accent` with `colour-on-accent` Fill; otherwise
  `colour-fill-strong` with `colour-text` Regular (Bluetooth, VPN).
- **In-row progress.** The spinner component at 12 in the trailing slot,
  the second line "Connecting" or "Pairing"; the row keeps its height.
- **In-row error.** The second line becomes the error sentence in
  subheadline `colour-text`, wrapping; the leading glyph becomes
  `warning-circle` 16 until the row is next used. The row grows to fit:
  the user's own act moved it.
- **Lists that arrive.** Scanning rows join at their group's end; the
  group re-sorts only while the pointer is outside it. A heading of a
  group that is scanning carries the spinner at 12, `space-s` after it.
- **Empty lines.** One row, label in `colour-text-2`: "No networks found",
  "No devices", "No apps playing sound", "Not playing".

| Page | Head | Body |
|---|---|---|
| Wi-Fi | Switch | A wired card when there is a connection: `plugs-connected` (unplugged: `plugs`), its name, trailing "Connected" or "Cable unplugged". The current network: row-large, the level in two layers at 16 (thresholds as the island's), name, "Connected", `lock-simple` 12 `colour-text-2` if secured, `check`. "Known Networks", then "Other Networks" as a disclosure row (`caret-right` 12, `caret-down` open) whose rows expand in place under it; a network row is the level glyph, name, the lock; last, "Join Other Network…" as a row in `colour-text` |
| Bluetooth | Switch | "My devices": device circles; connected rows row-large with "Connected" and trailing battery (level glyph 16 in two layers, "{n}%" subheadline tabular `colour-text-2`); others one line. "Nearby devices" with its scanning spinner; one-line rows, neutral circles. A pairing row shows "Pairing" while the pairing prompt is up |
| Now Playing | Circle `music-note` (mark), state the player's name; no trailing control | The player card, padding `space-m`: art `size-media-art` (64), `radius-small`; `space-m`; title `type-style-title-3`, artist body `colour-text-2`, album subheadline `colour-text-2`, the player button (icon 16 and name). Under it the scrubber across 336 with times beneath, subheadline tabular `colour-text-2` ("1:24" leading, "−2:13" trailing); then the transport centred: `shuffle`, `skip-back`, play/pause, `skip-forward`, `repeat` (`repeat-once` when one), buttons 30 with glyphs 20, 24, 32, 24, 20 Fill. Shuffle and repeat on: a 30 circle of `colour-fill-strong` behind the glyph; off: glyph in `colour-text-2`. Then a card of other players, row-large: art 28 `radius-small`, title and the player's name, `isle-equaliser` 12 `colour-text-2` trailing while it plays |
| Focus | The module's circle, still the switch for the last-used mode; name and state as Medium; no switch | As `focus.md` draws it: one card of modes, Do Not Disturb, Game Mode, then the user's; every row row-large (40), on or off, with a device circle holding its symbol (on: accent, Fill); the second line, on, the module's state line ("Until 07:00", "{game}"), off, the first schedule ("Weekdays 22:00–07:00", "+{n}" for more), else none and the name centred; `caret-right` 12 trailing opens its durations in place: rows 28 aligned to the label column ("For 1 hour", "Until this evening" or "Until tomorrow morning", "Until the end of this event"; Game Mode with a game in front, "Off for {game}") |
| VPN | Switch (the last connection) | Per provider, a card headed by `widgets.md`'s provider row on glass: the provider's icon at `size-app-icon-row` (20), its name over its state sentence cut to one line, no switch (the page's head has it), and while on up to two actions as small buttons under the sentence ("Sign in…"), the rest under "Options"; then its connections as one-line rows with device circles (`isle-vpn`, or `globe` for a location), `check` on the connected one. NetworkManager's connections under their own heading, the same rows |
| Sound | Circle the mute; slider; state the output | "Output": rows with the kind's glyph (`speaker-high`, `headphones`, `monitor`), name, a Bluetooth headset's battery trailing, `check` on the default. "Apps": row-large, one line: icon at `size-app-icon-row`, name in a `size-slider-label` (112) column, cut; the slider; mute as a plain icon button 24 (`speaker-high`, `speaker-slash`). A game's cover stands in for its icon, fitted to 20 high (a 2:3 cover is 13×20), `radius-small`, `colour-hairline` edge, as the switcher's. "Input": device rows as Output's (`microphone`), then a row-large: mute button (`microphone`, `microphone-slash`), the gain slider at the top of the row and the level meter under it, 4 tall, the slider's width, track `colour-fill-strong`, fill `colour-text`, continuous |
| Display | Circle `sun` (mark); state this screen's name; no slider | One row-large per screen, this screen first: `monitor` 16, the monitor's name in a 112 column, its slider. No DDC: two lines, the name over "Brightness can't be set on this display". A failed write: the second line "Couldn't set brightness on {monitor}.", the slider back at the true value |
| Energy Mode | The mode's glyph | Three rows: `leaf` Low Power, `gauge` Automatic, `lightning` High Power, `check` on the current. During Game Mode's hold the head's state says so; the rows do not |
| Updates | The arc while installing | Up to date: row-large "Up to date", "Checked {time}", trailing "Check now" (small button; the spinner while checking). Waiting: rows per source with their counts trailing, tabular `colour-text-2`; under the card, trailing, "Install updates…" as a default button. Installing: row-large, "Installing {n} of {m}" over the progress component across the label column. Restart owed: "Restart to finish updating", one row per reason in `colour-text-2`, then "Restart…" as a default button |
| Batteries | The lowest's glyph | A laptop's own first, row-large with "{time} left". Then devices grouped by kind, one-line rows: kind glyph, name, trailing `lightning` 12 if charging, the level glyph 16 in two layers (low `battery-warning`, charging `battery-charging`), "{n}%" in a 36 column, right-aligned, tabular |
| Keyboard layout | `keyboard` | A heading with the focused keyboard's name; rows of layouts, `check` on the current |
| Accessibility | `person-arms-spread` | Zoom, On-screen keyboard, Increase Contrast, Reduce Motion, Reduce Transparency: one-line rows, no glyphs, the switch component trailing |
| Phone | `device-mobile` | `widgets.md`'s provider row on glass, as VPN's head; then the phone's battery as Batteries' row |

**The foot** is `island.md`'s: a plain button naming the Settings pane it
opens, in title case as a standard item, "{Pane} Settings…": "Wi-Fi
Settings…", "Focus Settings…", "Software Update Settings…", "Keyboard
Settings…".

### Tray menus as pages

The head is the item: back, its icon at `size-app-icon` (32) in the
circle's slot (no circle; its own shape), its title and tooltip line, no
trailing control. The body is one card of the DBusMenu's items as
one-line rows, with menus' columns (`menus.md`): a mark column of 16 in
every row (`check` 12 for checked and radio, `minus` for mixed), the
label, shortcuts in `colour-text-2` trailing, `caret-right` 12 to a
sub-page. Item icons are not drawn (brief: no icons on menu items).
Separators are menus' 9-point rule. Disabled rows `colour-text-disabled`.
No foot link.

### Edit mode

| Part | Look |
|---|---|
| Tiles | As in use, their controls inert; `island.md`'s remove badges. A click selects: the focus ring's form, drawn for pointer selection too, since a selected tile is what Delete and the arrows act on |
| Size menu | Secondary click or Return: a menu on `material-solid` (opened from glass), the sizes the module supports with `check` on the current, a separator, "Remove" |
| Lifted tile | Under the pointer at its own size, full opacity, `shadow-menu`; no scale |
| Landing outline | The cell range it would take: `stroke-hairline` in `colour-hairline-strong` inside a tile of `colour-fill`, `radius-card`; the other tiles reflow round it. Outside the panel the outline goes, the grid closes up, and the lifted tile drops to 50% opacity: release removes it |
| Gallery | `island.md`'s second shape: `size-panel` wide, glass, `radius-panel`, `shadow-panel`, padding `space-s`, `space-s` to the panel's leading side, top level with the panel's; height to content, at most the panel's, scrolling under its field |
| Search | The search field, `size-control` tall, `radius-card` (concentric with the shape), `colour-fill`; `magnifying-glass` 12 and "Search controls" in `colour-text-2`. No match: "No results for “{query}”" |
| Groups | Headings as a page's: "Connectivity", "Sound and display", "Focus", "System", "Accessibility", "Providers"; each a card |
| Row | `size-row-large`: the module's circle (off form, glyph 20), `space-m`, name (body `colour-text`) over its sizes ("Small, Medium", subheadline `colour-text-2`); trailing `plus` 16 as a plain icon button that adds it at the grid's end. The whole row drags: the lifted tile is the module at its default size |
| Unavailable row | Circle and name `colour-text-disabled`; the second line is why ("No Bluetooth adapter"), `colour-text-2`; trailing "Set up…" as a small button. It does not drag |

**Disabled modules** (no device) are never in the grid: the grid closes up
(on a wired desktop Now Playing leads) and the module waits in the
gallery as an unavailable row.

## States

| State | What changes |
|---|---|
| Peeked, pinned | `island.md`; pinned by Ctrl+F8, the ring on the first module |
| On, off | The circle's two forms; state line |
| Hover, pressed | Tile `colour-fill-strong`, `colour-control-pressed`; rows likewise |
| Connecting, pairing | Circle in the target form, "Connecting"; on a page the row's spinner |
| Page, sub-page | The page frame; a disclosure (Other Networks, Focus durations) expands in place instead |
| Edit mode | Badges, the gallery, Done; Now and the tray folded |
| Dragging | Lifted tile, landing outline, the rest reflowed |
| Empty | "No controls" in body `colour-text-2`, centred in one 64 row of the grid |
| Loading | Last state, disabled form; pages show what they have, spinner on the scanning heading |
| Error | "Not available" in the disabled form; a page's body one row naming what is missing and the foot; a failed row's error in the row |
| A device absent | The module out of the grid; its unavailable row in the gallery |
| Many items | Pages scroll under the head; Batteries Large shows "{n} more" |
| Second screen | The same sizes; Display's module and page lead with that screen |
| Windowed game, Game Mode | `material-solid` everywhere; Focus shows `game-controller` and the game; Energy Mode "High Power while Game Mode is on"; Night Light "Off during Game Mode" |
| Focus on | The Focus module's on form |
| Reduce Motion, Reduce Transparency, Increase Contrast, largest text | Motion and Accessibility |

## Motion

Content follows the growth (`island.md`): outgoing at 1 − 2p, incoming at
2p − 1, on the spring that moves the shape. Exits run on `motion-exit`.

| Transition | Spring | From → to |
|---|---|---|
| Page opens | `motion-open` | The panel's height to the page's. A clip, `radius-card`, grows from the module's tile to the body between the pill's row and the footer, anchored at the tile's top edge and widening both ways from it to the body's edges; the page is laid out in its final place inside the clip, never sliding, and comes in by 2p − 1; the grid, Now and tray go by 1 − 2p. From a glyph on the island (no tile on screen) the clip starts at the body's top, full width |
| Back | `motion-exit` | The reverse, the clip closing onto the tile, which holds the focus ring if the key came from the keyboard |
| Sub-page | `motion-open`; back `motion-exit` | The same from its row, the clip from the row's rect |
| Disclosure (Other Networks, Focus durations) | `motion-open`; closing `motion-exit` | The page's height; rows below move with the growth; the new rows come in by 2p − 1. The caret changes in one frame |
| Toggle | none | The circle's fill and weight and the state line change in the frame after the press. The switch component's fill changes at once and its knob travels on `motion-exit`, no overshoot |
| Slider by key (+, −, volume and brightness keys, arrows on a page) | `motion-exit` per step | The fill and knob retarget on each repeat from where they are, so the slider keeps pace with key repeat; nothing queues. No OSD while the module shows the level |
| Slider by drag | none | Fill and knob under the pointer, 1:1 |
| Slider set from outside | `motion-exit` | To the new value |
| Track changes | none | Art, title and artist crossfade in place over `motion-fade-ms` |
| Updates arc | `motion-exit` | To the new share |
| Row joins, leaves | none, then `motion-move` | Joins at its group's end over `motion-fade-ms`; leaves over `motion-exit-fade-ms`, then the rows below close the gap |
| Edit mode in | `motion-open` | The gallery grows from the panel's leading edge, its top and trailing edge fixed; Now and the tray go by 1 − 2p and the grid's top rides up with the shrinking height; badges come in by the gallery's 2p − 1 |
| Edit mode out (Done, Escape) | `motion-exit` | The reverse |
| Reflow during a drag, a removal, a size change | `motion-move` | Each tile to its new cell; a resized tile springs its frame, its old content out by 1 − 2p, its new in by 2p − 1 |
| Drop | `motion-move` | The lifted tile from the pointer into its outline; the shadow fades with p |
| Remove (badge, Delete, dragged out) | none, then `motion-move` | The tile goes over `motion-exit-fade-ms`; the grid closes up; its gallery row comes in over `motion-fade-ms` |

**Reduce Motion.** No clip, no reflow motion: the old content and the new
crossfade in place over `motion-reduced-fade-ms` and the panel takes its
new height at once. Tiles jump to their cells; the lifted tile still
follows the pointer (direct manipulation, not motion). Sliders and the
switch's knob jump; the spinner turns.

**Game Mode.** No transitions: pages, disclosures, edit mode and reflow
replace in one frame; sliders jump; crossfades are cuts. `material-solid`,
no blur or noise.

## Accessibility

- **Colour roles.** On the glass and on tiles and cards (`colour-fill`
  over it): `colour-text` and `colour-text-2` only; `colour-text-3` never
  appears. Disabled `colour-text-disabled`, never carrying a reason: the
  reason ("Not available", why a gallery row is unavailable) is
  `colour-text-2`. The accent carries `colour-on-accent` glyphs only. No
  status colour on the panel: low batteries and errors show by glyph and
  weight.
- **Increase Contrast.** `island.md`'s edges on tiles and the panel;
  page cards, the gallery and its rows' circles get a `stroke-hairline` in
  `colour-hairline-strong`; slider tracks, the meter and the switch
  component take the same edge; glyphs Bold; on and off differ by Fill
  against Regular and accent against neutral; the scrubber's knob shows at
  rest.
- **Reduce Transparency.** Panel and gallery `material-solid`; tiles and
  cards keep `colour-fill` over it.
- **Focus ring.** `stroke-focus-ring` at `stroke-focus-offset`, keyboard
  only (edit mode's selection aside). Round a tile at `radius-card` plus
  the offset; round a page row inset by the ring's width so the card does
  not clip it; round a slider's knob (and the scrubber's, shown while
  focused); round buttons at their own radius; round a gallery row inside
  its card.
- **Largest text size.** Cells scale with the text (126×96, the panel
  544, as `island.md`); circles, glyphs, art and knobs scale with them.
  Medium's state takes a second line. Rows are their lines plus 12; the
  slider-label column grows 1.5×. Now Playing Large's player line drops
  before the transport. Pages and the gallery scroll sooner.
- **Hit targets.** Tiles 84×64 and up; glyph boxes 54 wide; slider lines
  24 tall; transport buttons 24 and 30; badges 16 with a 24 hit box; rows
  28; switches the row's height. All at or above `size-hit-minimum`.
- **Names.** Every tile and row has "{Module}: {state}" with "on" or
  "off"; the head's back is "Back to Control Centre" (the sheet's Copy).

## Board

At 3440×1440, dark first and each in light, over a light and a dark
wallpaper. Panel scenes crop `[920, 0, 2520, 900]`, long pages
`[920, 0, 2520, 1200]`; the gallery falls inside both. The module sheet
is drawn on the stage at actual size, crop `[920, 0, 2520, 1100]`. The
second screen `[160, 0, 1760, 1080]`.

1. **Module sheet**: every module in each size it supports, on and off,
   hover, focus, pressed, disabled.
2. **Default arrangement, live**: Home Wi-Fi, music playing in Large, two
   Bluetooth devices, VPN connected, Sound and Display Wide, Night Light
   on, 14 updates, a pad low.
3. **Wired desktop**: no Wi-Fi; Now Playing leads, the grid closed up.
4. **States of a tile**: connecting, a failed action's clause, loading,
   Not available, Updates installing with its arc, restart owed.
5. **Now Playing**: Small, Medium, Large, Wide playing and paused; no art;
   a live stream; nothing playing.
6. **Wi-Fi page**: wired row, current network, known, Other Networks open
   and scanning, one row connecting, one with its error.
7. **Bluetooth page**: connected with battery, paired, nearby discovering,
   pairing with the pairing prompt over the pinned panel.
8. **Now Playing page**: shuffle on, repeat one, two other players.
9. **Focus page**: Do Not Disturb on until 07:00, its durations open; Game
   Mode on for a game with "Off for {game}".
10. **VPN page**: a provider signed out, one connected with locations,
    NetworkManager's heading, a provider's timeout as its sentence.
11. **Sound page**: a Bluetooth headset with battery, three apps (a game's
    cover, one muted), input with the meter mid-level.
12. **Display page**: two screens, one without DDC, a failed write.
13. **The rest of the pages**: Energy Mode under Game Mode's hold; Updates
    in its four states; Batteries; Keyboard layout; Accessibility; Phone.
14. **Tray**: a menu page with checks, a separator, a disabled row and a
    submenu's sub-page.
15. **Edit mode**: badges, the gallery with its groups and an unavailable
    row, the size menu open, a drag mid-way with outline and reflow, a
    tile dragged out, search with no match.
16. **Keyboard**: the ring on a tile, a page row, a slider's knob, a
    gallery row.
17. **Second screen**: its Display page, that screen first.
18. **Windowed game**: the solid panel, Focus, Energy Mode, Night Light.
19. **Settings**: scenes 1 and 11 under Increase Contrast, Reduce
    Transparency and the largest text size.
20. **Motion**: a page from Wi-Fi (top leading) and from Now Playing (top
    trailing, Large) and back; from an island glyph; a sub-page; Other
    networks opening; a toggle frame by frame; Sound under key repeat; edit
    mode in and out; a drag, a resize and a removal. Each with Reduce
    Motion and Game Mode.

## Choices

1. **Slider fill on glass.** (a) `colour-text`, as the island's level bar,
   so the OSD and the module show one level the same way and the accent
   stays on switches that are on (as macOS's Control Centre); (b) the
   slider component's accent fill. **Recommended: (a)**; the component
   keeps the accent in windows.
2. **Heads without a switch.** Focus's rows each turn a mode on or off, so
   a head switch would repeat one of them; Display's rows are one slider
   per screen, so a head slider would repeat the first. **Recommended:**
   both heads carry name and state only, keeping the sheet's "no row
   repeats the head's switch".

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `app-icon-row` | 20 (per-app sound rows, provider rows; icons.md's 20) |
| `size` | `media-art` | 64 (Now Playing's page) |
| `size` | `slider-label` | 112 (the name column before a row's slider) |
| `size` | `scrubber-knob` | 12 |
| `stroke` | `progress` | 2 (the Updates arc) |
| `timing` | `tooltip-ms` | 700 (the tooltip's delay, shared by every tooltip; as `conventions.md`) |

Phosphor glyphs to vendor beyond the boards' set: `leaf`, `lock-simple`,
`plugs`, `mouse`, `laptop`, `shuffle`, `repeat`, `repeat-once`,
`selection-plus`, `device-mobile`.
