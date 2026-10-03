# Notification banners

P1 stage 5, wave B. The look and motion of notification banners: the card,
the stack, the overflow row, the summary after silence, and the card's form
inside Notification Centre. Structure is binding from
`structure/notifications.md` (what a banner holds, its states, keys and
copy), `structure/notification-centre.md` (the card is the same card there)
and `structure/competition.md` (the top right). Values are `tokens.kdl`
names; numbers not yet tokens are listed at the end.

## Anatomy

### The card

One card serves the banner and Notification Centre, at one width: the
panel's inner width, `size-panel` less 2 × `space-s`, so 360 (`island.md`,
Notification Centre). Only its material and corner change with where it
sits (In Notification Centre, below).

| Part | Look |
|---|---|
| Shape | 360 wide, height to content. `radius-panel` (18). Material `material-glass` at the user's glass position, `material-blur`, `material-saturation`, `material-noise-opacity`; `shadow-panel`. No edge, no accent, no colour for urgency |
| Padding | `space-m` (12) on every side. Inner corners are `radius-control` (18 less 12, concentric) |
| App icon | Papirus at `size-app-icon` (32), its own shape, no tile, top at the padding, leading at the padding (12, 12). Generic icon when none resolves: Papirus's `application-x-executable` |
| Text column | From 56 (padding, icon, `space-m`) to 348: 292 wide; 232 beside a thumbnail |
| Header line | 16 tall, the full text column (it runs over the thumbnail's column). App name in `type-style-subheadline` semibold, `colour-text-2`, cut at its end. Trailing: the count, then `space-xs`, then the time in `type-style-subheadline`, tabular, `colour-text-2` ("now") |
| Count | "{n} new" in the quiet badge (`colour-fill-strong`, `colour-text`, `type-style-subheadline` semibold, capsule 16 tall, padded `space-xs`), as menus' and the island's counts |
| Summary | Directly under the header: `type-style-headline` (13 semibold, line 16), `colour-text`, one line, cut at its end |
| Body | Under the summary: `type-style-body`, `colour-text`, `limit-banner-lines` (2) at rest, cut at the end of the last; bold, italic and underline drawn, links as plain text |
| Thumbnail | Trailing, a square of `size-banner-thumbnail` (48: the summary and two body lines), top on the summary's top, `radius-control`, a `stroke-hairline` edge in `colour-hairline`; the picture fills it, centre-cropped |
| Progress | The progress component (6 tall, radius 3, track `colour-fill-strong`, fill `colour-accent`), `space-s` under the body, the full text column. Absent without the `value` hint |
| Actions | Shown while hovered: `space-m` under the body or progress, a row of buttons across the text column, `space-s` apart, equal widths (three make 92 each). Each a regular button: `size-control` (24), `radius-control`, `colour-control`, `type-style-body` `colour-text`, label cut at its end. No glyphs. Past three actions, the third is "Options" with `caret-down` at `size-glyph-small` trailing its label |
| Close | A 20 circle (`size-control-small`), `material-solid`, `shadow-control`, a `stroke-hairline` edge in `colour-hairline`, holding `x` at `size-glyph-small` in `colour-text`. Its centre sits on the card's top leading corner curve (5 in from each edge), so it overhangs the corner and the icon's corner, as macOS's. Hit box 24 square. Shown while hovered; always on a persistent card |
| Card heights | Summary only 56 (the icon's 32 plus padding); summary and one body line 72; two lines 88; with progress 102; hovered with actions, plus 36 |

**Hovered.** The body opens to `limit-banner-lines-expanded` (10), cut
past it. A thumbnail leaves its slot and becomes a preview under the body:
the text column's width, at the picture's aspect, at most 16:9 of that
width (292 × 164), `radius-control`, `space-s` above it; the summary and
body take the slot's width back. Then the actions.

**Replying.** The action row gives way to the reply: the field component
in its glass form across the text column (`colour-fill`, a 1-point
`colour-hairline-strong` inner edge, `radius-control`, `size-control`
tall for one line, growing a body line at a time to `limit-reply-lines`
(4), then scrolling), placeholder "Reply" in `colour-text-2`; focused, its
edge is `colour-accent`. Under it, `space-s`, Cancel (regular) and Send (the
default button: `colour-accent`, `colour-on-accent`) at the trailing edge,
`space-s` apart; Send is disabled while the field is empty.

**Isle's own parts** that have no app icon (Software Update, Calendar, a
provider without an icon, the summary) take a glyph circle in the icon's
place: 32 (`size-module-glyph`), `colour-fill-strong`, the glyph at
`size-glyph-large` (20) Regular in `colour-text`: `arrow-circle-up`,
`calendar-blank`, `plug`; the summary below. The app name is the part's.

### The stack

| Part | Look |
|---|---|
| Anchor | The main screen's top trailing corner: the newest card's top on `size-strip`'s lower edge (48, or the strip's height at larger text), its trailing edge `space-l` (16) from the screen's edge. The stack grows down from there; nothing re-anchors |
| Order | Newest at the anchor, older below, `space-s` (8) apart; at most `limit-banners` (3) |
| Overflow row | Under the stack, `space-s` below the last card: a glass shape 360 × `size-row-large` (40), `radius-panel`, the card's material and shadow. `bell` at `size-glyph` (16) Regular in `colour-text-2`, centred on the icon column (x 28); "{n} more in Notification Centre" in `type-style-body` `colour-text` from 56; `caret-right` at `size-glyph-small` in `colour-text-2` at the trailing padding. The whole row is its hit area |
| Height | Three cards at rest with the row: about 330. It never reaches far down, and over a full-screen window it sits at the same anchor |

### The summary after silence

The card with the glyph circle holding `game-controller` ("While you
played") or `bell` ("While you were away"). Header: "Notification Centre"
(the part that follows up) and "now". Summary line: the title; body: "{n}
notifications from {app} and {app}". One action, "Show", shown on hover as
a button the width of the text column. A held notification re-shown alone
is its own card with its real time ("12 min ago").

### In Notification Centre

The same card, the same 360, the same parts, sizes and type, so a banner
and its entry read as one thing. Two changes follow from where it sits:
its fill is `colour-fill` on the panel's glass, not glass of its own (glass
never sits on glass), with no shadow; its corner is `radius-card` (the
panel's 18 less its 8 padding, concentric). The close control's overhang
fits in the panel's padding. A record is the card without actions or
Reply. Stacks, headers and the rest of the centre are wave C's.

## States

| State | What changes |
|---|---|
| Rest | Nothing drawn: no area, no empty stack, no count anywhere |
| One banner | One card at the anchor; no close control |
| Hovered | After `timing-hover-dwell-ms`, the hovered card grows: full body, preview, actions, close control. Others keep their rest form; every timer is paused |
| Replying | The reply in place of the actions; the card stays grown while the pointer is away; the ring rules below |
| Persistent, critical | The close control shown at rest; nothing else differs. Critical is told apart only by staying |
| Grouped | The count badge in the header; the newest content |
| Several | Up to three, newest at the anchor |
| Overflow | Three persistent cards and the overflow row |
| Waiting, held, away, Notification Centre open, locked | Nothing drawn for it |
| Summary | The summary card (Anatomy) |
| Loading | A card is drawn on arrival with what is in hand: the generic icon until the app's resolves, then swapped in place; a late picture fades into a new thumbnail slot, the text re-wrapping beside it, height never past its first |
| Error | Generic icon; no thumbnail; markup as plain text |
| Full-screen window, allowed | The same card at the same anchor over the content, glass |
| Windowed game, Game Mode | `material-solid`, no blur or noise; no transitions |
| Full-screen game, broken through | `material-solid` over the game, no transitions; a critical card is temporary (`timing-banner-max-ms`) and shows no close control at rest |
| Second screen | When a full-screen window holds the main screen and the second is free: the stack at the second screen's top trailing corner, same sizes |
| Reduce Motion, Reduce Transparency, Increase Contrast, largest text | Motion and Accessibility |

## Motion

The compositor draws the stack (B7). Content fades with the card's
growth, never on its own clock: each move has a progress p, the share of
its size change done; outgoing content's opacity is 1 − 2p, incoming
content's 2p − 1. Exits run quicker than entrances.

**Arrival: from the trailing edge**, not dropped from the strip. The
stack lives at the edge, a swipe sends a card back the way it came, and
movement in the periphery is what makes a banner seen 1700 points from
the eye (structure, For the user 1). A drop from the strip would read as
something falling out of the island, which never shows a notification.

| Transition | Spring or fade | From → to, anchor |
|---|---|---|
| Arrive | `motion-open` | x from just past the screen's trailing edge (card plus `space-l`) to its place; opacity min(1, 2p). Its top is fixed at its slot |
| Stack steps down | `motion-open`, the arrival's spring | Older cards move down by the newcomer's height plus `space-s`, together with it |
| Oldest leaves early | `motion-exit-fade-ms` | Fades in place as the arrival starts |
| Grouped or replaced | `motion-fade-ms` crossfade; height `motion-open` if taller, `motion-exit` if shorter | Content swaps in place, top fixed; no entrance, the count changes in place |
| Hover grows | `motion-open` | Height down from the top edge; new parts in by p from half; cards below move down on the same spring. Growth that adds no height (a short card) fades the close control over `motion-fade-ms` |
| Hover ends | `motion-exit`, after `timing-hover-grace-ms` | The reverse; parts out by the first half |
| Time out, close, act | `motion-exit-fade-ms` | Opacity to 0 in place; then the cards below close the gap on `motion-exit` |
| Swipe | Direct, then `motion-exit` | The card follows the pointer or scroll 1:1 toward the trailing edge (a third of the travel toward the leading edge), opacity 1 − d/360. Released past `size-swipe-threshold` (120), or with velocity the spring would carry past it, it leaves off-screen on `motion-exit` from that velocity; short, it returns on `motion-move` |
| Overflow row | `motion-fade-ms` in, `motion-exit-fade-ms` out | In place, under the stack; its count changes in place |
| Reply opens | `motion-open` | The action row out by the first half, the field in by the second |

While the pointer is over the stack nothing arrives and no timer runs; the
only movement is the hovered card's own growth, which happens under the
pointer and pushes cards below it, never under it. After the pointer leaves,
each card's time resumes, and none leaves sooner than
`timing-banner-resume-ms`.

**Reduce Motion.** Arrival and leaving are fades in place over
`motion-reduced-fade-ms` and `motion-exit-fade-ms`; the stack's moves and
the hover's height change are cuts, the content crossfading at the new
size over `motion-reduced-fade-ms`. A swipe still follows the pointer
(direct manipulation is not animation); released, the card fades.

**Game Mode** (and any banner over a game). No transitions: a card appears
whole in one frame and goes in one; the stack's moves, the hover and the
reply jump; a swipe follows the pointer and the card is gone on release.
`material-solid`, no blur sampled.

## Accessibility

- **Colour roles.** On the card's glass only `colour-text` (summary,
  body, button labels, the overflow row) and `colour-text-2` (app name,
  time, placeholder, glyphs on the row); `colour-text-3` is never used.
  The badge and glyph circles carry `colour-text` on `colour-fill-strong`;
  Send's label is `colour-on-accent` on `colour-accent`. The progress fill
  is a graphic on its track, as the switcher's. In Notification Centre the
  same roles on `colour-fill` over the panel's glass.
- **Increase Contrast.** `material-glass-tinted`; a `stroke-hairline` edge
  in `colour-hairline-strong` round every card, the overflow row, each
  button, the field and the close control; glyphs at
  `icon-weight-contrast`; the progress track takes the strong hairline.
- **Reduce Transparency.** Every card and the row on `material-solid`; in
  the centre, the card stays `colour-fill` on the solid panel.
- **Focus ring.** A banner takes keyboard focus only to reply. The field
  shows its focused edge; Tab moves to Cancel and Send, which take
  `stroke-focus-ring` in the accent at `stroke-focus-offset`, round their
  `radius-control`. The card's 12 padding leaves room for the ring. Keyboard
  work on notifications happens in the centre, where the ring goes round
  the whole card at `radius-card` plus the offset (the panel's padding
  holds it).
- **Largest text size** (1.5×, as `island.md`). The card stays the panel's
  inner width, which grows with the text: 528 at 1.5×, matching the centre.
  The icon steps to `size-app-icon-medium` (48), the thumbnail to the new
  summary and body height (72), the strip and so the anchor grow; line
  limits hold; past the width, text is cut, never clipped.
- **Hit targets.** The card is its own target (56 at least); buttons 24;
  the close control's box 24; the overflow row 40. All above
  `size-hit-minimum`.

## Board

At 3440×1440 with the 1920×1080 screen beside it, dark first and each in
light, over a dark and a light wallpaper and over a white window. Crops
are x, y, width, height on the ultrawide: banner scenes 1840, 0, 1600, 600
(the top right at actual size), hover and reply 1840, 0, 1600, 800.

1. **One banner**: Discord with summary and two lines, over the wallpaper
   and over a white window at the same place.
2. **Card anatomy**: summary only; one body line; two lines with a
   thumbnail; with progress; a missing icon; a desktop part's glyph circle.
3. **Hovered**: full body with the preview, three actions; four actions
   with Options and its menu open on `material-solid`; the close control.
4. **Replying**: an empty field with Send disabled, then three lines typed
   and the ring on Send.
5. **Grouped**: Discord "3 new".
6. **Stack**: three cards, newest at the anchor, the island's pill in frame
   for scale (crop 920, 0, 2520, 600).
7. **Overflow**: three persistent cards and "2 more in Notification
   Centre".
8. **Persistent and critical**: the close control at rest.
9. **Summary**: "While you played" and "While you were away"; a re-shown
   single held card "12 min ago".
10. **Over a full-screen window**: glass over video; and over a full-screen
    game, solid.
11. **Windowed game**: the solid card beside the game.
12. **Second screen**: the stack on the 1920 screen while the main holds a
    full-screen window (that screen's 320, 0, 1600, 500).
13. **Same card**: a banner and its entry in Notification Centre side by
    side at actual size (crop 1840, 0, 1600, 900 with the centre placed
    beside).
14. **Settings**: scenes 1 and 3 under Increase Contrast, Reduce
    Transparency and the largest text size.
15. **Motion**, on the harness's spring: arrival with the stack stepping
    down; a grouped update; hover grow and shrink; time out with the gap
    closing; a swipe past and short of the threshold; the oldest leaving
    early; each under Reduce Motion and Game Mode.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `banner-thumbnail` | 48 (a banner's thumbnail: the summary and two body lines) |
| `size` | `swipe-threshold` | 120 (travel that sends a card away; banners and Notification Centre) |
| `limit` | `banners` | 3 |
| `limit` | `banner-lines` | 2 (body lines at rest; the centre's cards too) |
| `limit` | `banner-lines-expanded` | 10 |
| `limit` | `reply-lines` | 4 |
| `timing` | `banner-min-ms` | 3000 (the shortest lifetime an app may ask) |
| `timing` | `banner-max-ms` | 15000 (the longest; "never" as a banner; critical over a game) |
| `timing` | `banner-resume-ms` | 1500 (the least time left after the pointer leaves) |
| `timing` | `sound-burst-ms` | 2000 (an arrival within this of the last alert sound is silent) |

The card's width is not a token: it is `size-panel` less 2 × `space-s`.
Glyphs to vendor from Phosphor: `calendar-blank`, `plug`.
