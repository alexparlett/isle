# Notification Centre

P1 stage 5, wave C. The insides of Notification Centre: the date, the
month, the agenda, the notifications as stacks, Earlier, clearing, and
their motion. Structure is binding from `structure/notification-centre.md`
(layout, order, stacks, times, clearing, keys, copy). The frame, the
morph from the pill and the morph with Control Centre are
`surfaces/island.md`'s; the card is `surfaces/notifications.md`'s, plain
(`colour-fill`, `radius-card`) inside the panel. Neither is restated.
Values are `tokens.kdl` names; numbers not yet tokens are listed at the
end.

## Anatomy

### The column

`island.md`'s panel: `size-panel` (376) wide, padding `space-s`, so an
inner width of 360, the card's width. Under the pill's row and its
`space-s`, top to bottom, `space-s` apart: the calendar card, the agenda,
the Notifications header and the live stacks, Earlier and its stacks.
Every card here is `colour-fill` on the panel's glass at `radius-card`
(18 less 8, concentric), no shadow. Headings sit on the glass, never inside
a card, as a page's group headings (`island.md`): `type-style-subheadline`
semibold, `colour-text-2`, `space-m` above and `space-xs` below, at the
cards' leading inset of `space-m`. Everything under the pill's row scrolls
as one, to `island.md`'s maximum height; the row stays.

### The calendar card

Padding `space-m` (inner 336). Its height is fixed (six weeks always), 290,
so nothing below it moves when the month changes.

| Part | Look |
|---|---|
| Date | "Saturday 3 October", `type-style-title-3`, `colour-text`, one line, leading |
| Month row | `space-s` under, `size-control` (24) tall. Leading, the month's name and year ("October 2026") as a plain button in `type-style-headline` `colour-text`: a click returns to today. Trailing, previous and next as plain icon buttons `size-control` square, `caret-left` and `caret-right` at `size-glyph` (16), `colour-text-2`, `space-xxs` apart |
| Weekdays | `space-xs` under: the locale's short names from its first weekday, `type-style-subheadline` `colour-text-2`, centred in their columns |
| Days | `space-xs` under: seven columns of 48 by six rows of `size-month-row` (32). A day's number in `type-style-body`, tabular, `colour-text`, centred in a mark circle `size-control` (24) at the cell's top centre. Days of other months are not drawn; weekends look as weekdays |
| Today | The mark circle in `colour-accent`, the number `colour-on-accent` semibold: the one marked day (brief P6) |
| Selected day | Another day once chosen: the circle `colour-fill-strong`, the number semibold. Today keeps its accent while another is selected |
| Event mark | A 4-point dot in `colour-text-2` centred under the circle, its centre 29 down the cell: neutral, whatever the calendars' colours. Today's dot is the same |
| Hover, pressed | The circle `colour-control-hover`, `colour-control-pressed` |
| Week numbers | When set: a leading column of 28, the number in subheadline tabular `colour-text-2`; the seven columns share the rest (44 each) |

### The agenda

Absent with no calendar. Otherwise a heading and a card per day shown:
"Today" then "Tomorrow", or the chosen day's date ("Tuesday 6 October")
alone. Rows run edge to edge in the card, no rules between.

| Part | Look |
|---|---|
| Event row | `size-row-large` (40), inset `space-m`. The calendar's mark, then `space-s`, then two lines: the title in `type-style-body` `colour-text`, cut at its end; under it the time and the location, "09:30–10:00 · Studio 2", in `type-style-subheadline` `colour-text-2`, tabular, cut at its end. All-day events lead the day with "All day" as their time |
| Calendar mark | A 12 box: a 10 circle of the calendar's colour ringed 1 point in `material-solid` (the tray dot's form), centred on the title's line. Colour on glass sits on solid, as the island's wells |
| Calendar colour | Each calendar takes one of the `series` colours (a to d), in the order subscribed, changed in Settings › Calendars; a fifth takes a again. A feed's own colour is not used: it can neither be promised against the glass nor kept from looking like the accent or a status |
| Now | An event under way: its time line starts with "Now" in subheadline semibold `colour-text`, then the range and location as before. No accent: weight carries it |
| Join | Where the location or link is a meeting address: a small button (`size-control-small`, `colour-control`, `type-style-callout` `colour-text`, padded `space-s`), trailing at inset `space-m`, centred on the row; the title cuts before it |
| Earlier today | First in Today's card while some of today's events have ended: a `size-row` row, `caret-right` 12 then "Earlier today" in `type-style-body` `colour-text-2`; open, `caret-down` and the ended events under it, their titles `colour-text-2` |
| Expanded event | The row grows in place: under the title, one subheadline `colour-text-2` line each for the range, the calendar's name (after its mark), the location (wrapping), then the notes in `type-style-callout` `colour-text`, wrapping, cut at `limit-banner-lines-expanded`; Join moves under the notes, leading at the text column. Padding `space-s` below |
| Nothing | One `size-row-large` row, "No more events today" or "No events", `type-style-body` `colour-text-2`, inset `space-m` |
| Hover, pressed | The row `colour-fill-strong`, `colour-control-pressed`, clipped by the card |

### Notifications

| Part | Look |
|---|---|
| Header | A heading row `size-row` (28) tall: "Notifications" in the heading style, leading; Clear all trailing, at the cards' trailing edge |
| Clear all, at rest | A `size-control-small` (20) circle of `colour-control` holding `x` at `size-glyph-small` in `colour-text`. Hit box 24 |
| Clear all, armed | On hover after `timing-hover-dwell-ms`, on keyboard focus, or after a click on the circle: a capsule 20 tall, trailing edge fixed, growing toward the leading side to hold "Clear all" in `type-style-callout` semibold `colour-text`, padded `space-s`, the `x` gone. A click or Return on it clears. It returns to the circle when the pointer leaves or focus moves |
| A card | `notifications.md`'s card in its centre form; the time is the sheet's centre format ("14:05", "Yesterday 18:04"). Hover shows its close control, actions and full body as on a banner. Cards `space-s` apart |
| Collapsed stack | The newest card, and behind it up to `limit-stack-layers` (2) layers: each inset `space-s` more on each side than the shape in front (344, then 328 wide) and showing `size-stack-layer` (6) below it, `colour-fill`, `radius-card`. A layer is drawn only where the shape in front does not cover it, less a 1-point seam of the panel's glass along that shape's edge, so fills never stack and the edges read as cards behind. "{n} more" takes the header's count place as the quiet badge (`notifications.md`'s "{n} new" slot). The whole stack is one target: a click expands it |
| Expanded stack | A header row `size-row`: the app's icon at `size-app-icon-small` (16), `space-s`, its name in `type-style-headline` `colour-text`; trailing "Show less" as a plain small button in `type-style-callout` `colour-text-2`, `space-xs`, then the stack's close as the Clear all circle at rest, one click, accessible name "Clear {app} notifications". Then its cards, `space-s` apart |
| Show more | Past `limit-stack-cards` (5) in an expanded stack: a `size-row` plain button "Show {n} more", `type-style-callout` `colour-text-2`, centred under the last card |
| Earlier | The heading "Earlier" in the heading style, then records as cards and stacks the same way |
| Empty | Under the header (no Clear all): one 64-tall line, "No notifications" in `type-style-body` `colour-text-2`, centred, as Control Centre's empty grid |

A record is the card without actions or Reply. Isle's own parts keep the
glyph circle (`notifications.md`): a calendar notification shows
`calendar-blank`, and its click opens this panel with that event expanded.

## States

| State | What changes |
|---|---|
| Closed | Nothing; the clock carries no mark |
| Open | The column; keyboard focus on the newest card (else today), the ring only after a key |
| Day chosen | That day's selected circle; the agenda shows its heading and card alone; back to today on closing |
| Another month | The days crossfade; today's circle absent if not in it; the month's name as the way back |
| Event expanded | Its row grown; the others unchanged |
| Empty | Header without Clear all, the "No notifications" line; no Earlier heading |
| No calendars | The calendar card alone; no agenda headings or cards |
| Only records | No live stacks; "Earlier" directly under the header |
| Stack collapsed, expanded | Layers and "{n} more"; the header row, cards, "Show less" |
| Many | The column scrolls under the fixed row; "Show {n} more" past the cap |
| Clear all armed | The capsule |
| Replying | `notifications.md`'s reply in the card, which holds focus |
| Loading | Records absent until read, then Earlier fills below; a calendar not fetched shows its cached days, else no agenda |
| Error | Unreadable records: no Earlier. A dead sender's card becomes a record (actions go, the card's height on `motion-exit`). An image that fails: no thumbnail |
| Second screen | The same panel at the same size under that screen's pill |
| Windowed game, Game Mode | `material-solid` panel, cards `colour-fill` on it; no transitions |
| Focus on | Nothing here changes; the pill's row shows the glyph |
| Reduce Motion, Transparency, Contrast, largest text | Motion and Accessibility |

## Motion

The panel's opening, closing and its morph with Control Centre are
`island.md`'s. Inside it, content follows the growth: each change of height
springs the panel's shape (`motion-open` when taller, `motion-exit` when
shorter, top fixed) and what moves below rides that spring; incoming
content's opacity is 2p − 1, outgoing 1 − 2p. Nothing fades on its own
clock.

| Transition | Spring or fade | From → to |
|---|---|---|
| Day chosen | none, then the panel's height | The circles change in the next frame; the agenda's card crossfades to the day's at its new height |
| Month paged | `motion-fade-ms` | Days and the month's name crossfade in place; the grid's size is fixed, so nothing slides and nothing below moves |
| Event expands, collapses | `motion-open`; `motion-exit` | The row's height from its top; rows and sections below move with it; details in by 2p − 1. The caret changes in one frame |
| Stack expands | `motion-open` | The header row opens above the front card, which moves down by its height; each card behind grows from its layer (width from its inset, top from its edge) to its place in the list, its content in by 2p − 1 |
| Stack collapses (Show less, another expanding) | `motion-exit` | The reverse: cards back into their layers, the header out by the first half. Two stacks changing at once share the panel's one spring |
| A card cleared (close, Delete, middle click) | `motion-exit-fade-ms`, then `motion-exit` | Out in place; the cards below close the gap |
| Swipe | Direct, then `motion-exit` | `notifications.md`'s swipe inside the panel's clip: 1:1 toward the trailing edge, opacity 1 − d/360, away past `size-swipe-threshold`, back on `motion-move` short of it |
| Stack cleared | `motion-exit-fade-ms`, then `motion-exit` | Its cards and layers out together; the gap closes |
| Clear all | `motion-exit-fade-ms`, then `motion-exit` | Every card, stack and the Earlier heading out together; the panel shrinks to the empty form, the line in by 2p − 1 |
| Clear all arms | `motion-open`; back `motion-exit` | The capsule's width from the circle, trailing edge fixed; the label in by 2p − 1 |
| Arrival, new stack | `motion-open` | Its slot opens at the top of the live section, its top fixed, pushing the rest down; the card in by 2p − 1. Held while the pointer is over the list |
| Arrival, existing stack | `motion-fade-ms`; height as the panel's | A collapsed front card crossfades in place and "{n} more" changes in place; a stack lower down leaves as a clear does while it opens at the top as a new stack, in the same frames |
| Card hover | `motion-open`; ends `motion-exit` after `timing-hover-grace-ms` | `notifications.md`'s growth, pushing cards below, never above |
| Records arrive | `motion-fade-ms` | Earlier fades in below what shows; nothing above moves |
| Times, progress | none; `motion-exit` | Times change in one frame when refreshed; a progress fill retargets on `motion-exit` |

**Reduce Motion.** The panel takes each new height at once and the content
crossfades at it over `motion-reduced-fade-ms`. Stacks expand and collapse
at once; a cleared card fades over `motion-exit-fade-ms` without moving and
the list closes up at once. A swipe still follows the pointer; released,
the card fades.

**Game Mode.** No transitions: every change replaces in one frame, no fade;
`material-solid`, no blur or noise.

## Accessibility

- **Colour roles.** On the glass and on cards (`colour-fill` over it):
  `colour-text` and `colour-text-2` only; `colour-text-3` appears nowhere,
  the agenda's ended events included. Today is `colour-on-accent` on
  `colour-accent`. Calendar colours sit in `material-solid` rings, each
  `series` colour passes 3:1 on `material-solid` (`contrast.py` checks the
  pair), and any two are told apart under each colour-vision deficiency
  (`system/data-colours.md`). Event marks and dots are never the only sign: the agenda names
  each event and its calendar.
- **Increase Contrast.** `island.md`'s panel edge; a `stroke-hairline`
  edge in `colour-hairline-strong` round the calendar card, agenda cards,
  every notification card and each stack layer; Clear all, Join and the
  month's buttons take the same edge. The selected day becomes a 1.5-point
  ring of `colour-text` with no fill, so it differs from today's filled
  circle by more than colour. Glyphs Bold.
- **Reduce Transparency.** The panel `material-solid`; cards keep
  `colour-fill` on it.
- **Focus ring.** `stroke-focus-ring` in the accent at
  `stroke-focus-offset`, keyboard only: round a card or a collapsed stack
  (front card and layers as one shape) at `radius-card` plus the offset,
  which the panel's 8 padding holds; round a day inside its 48 by 32 cell
  at `radius-row`, so it never covers the next row's mark; round an event
  row inset by the ring's width; round buttons and Clear all at their own
  radius.
- **Largest text size** (1.5×, as `island.md`). The panel is 544 wide and
  cards 528 (`notifications.md`). Day cells grow with the text (72 by 48,
  mark 36); the calendar card grows to match and stays fixed in height
  from then on. Event rows are their two lines plus 12; titles cut before
  Join. The panel scrolls sooner.
- **Hit targets.** Day cells 48 by 32; month buttons 24; event rows 40;
  Join and Clear all 20 with a 24 hit box; stack headers 28; cards their
  own size. All at or above `size-hit-minimum`.

## Board

At 3440×1440, dark first and each in light, over a dark and a light
wallpaper and over a white window. Panel scenes crop `[920, 0, 2520, 1000]`;
long scenes `[920, 0, 2520, 1440]`. The second screen `[160, 0, 1760, 1080]`.

1. **Open, typical**: Saturday, three events today (one Now with Join, one
   ended under "Earlier today"), two tomorrow, four live stacks (one
   collapsed of three, one single, one with a thumbnail, one with
   progress), Earlier with two records.
2. **Calendar card**: today; another day chosen with its agenda; another
   month; week numbers on; hover and focus on a day.
3. **Agenda rows**: all day, Now, Join, an event expanded with notes, four
   calendars' colours, "No more events today", "No events".
4. **Stacks**: collapsed with one and two layers; expanded with its header;
   "Show 3 more"; a stack's close focused.
5. **Clear all**: at rest, armed by hover, armed by focus.
6. **Hover and reply** in the centre: actions, full body, the close control;
   a reply open with the ring on Send.
7. **Empty**: no notifications with calendars; no notifications and no
   calendars (the smallest panel).
8. **Only records; many** (scrolled, the row fixed above the clip).
9. **Arrival while open**, pointer outside the list: the frames of a new
   stack opening at the top.
10. **Second screen**, and **windowed game** (solid).
11. **Settings**: scene 1 under Increase Contrast (the selected day's ring),
    Reduce Transparency and the largest text size.
12. **Motion**, on the harness spring: a stack expanding and collapsing; a
    card swiped and cleared; Clear all; a day chosen; an event expanding;
    the month paged; each with Reduce Motion and Game Mode.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `month-row` | 32 (a week's row in the month, mark and event dot) |
| `size` | `stack-layer` | 6 (how much of each layer shows below the shape in front) |
| `limit` | `stack-layers` | 2 |
| `limit` | `stack-cards` | 5 (an expanded stack's cards before "Show {n} more") |

`contrast.py` gains `series` a to d on `material-solid` at 3:1, for
calendar marks. No new glyphs.
