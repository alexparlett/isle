# App switcher

P1 stage 5, wave A. How the switcher looks and moves, in Frost, from
`structure/switcher.md` and `structure/dock.md` (both binding). Values are
`tokens.kdl` names; numbers not yet tokens are listed at the end.

## Anatomy

One glass panel in the overlay layer, centred on the screen holding the
focused window: a row of plates, one per app, with the selected app's name
in a band under its row.

**Panel**

- Material `material-glass` at the Glass setting, never raised by the floor,
  since the panel carries no text but its name line (B58) (blur `material-blur`, saturation
  `material-saturation`, grain `material-noise-opacity`); `radius-panel`;
  `shadow-panel`.
- Padding `space-s` on every side. Plates sit `space-s` apart. Under each
  row of plates, `space-xs` below it, a name line in `type-style-headline`.
- Centred on the screen (not the work area) when it appears. Its top edge
  then stays put: the window row grows it downward, never re-centring it
  (brief P5).
- Width for n apps at full size: 2 × `space-s` + 144n + `space-s` (n − 1);
  six apps make 920 wide, sixteen fit the ultrawide. The rows never grow
  past three quarters of the screen's width (v1): 2580 on the ultrawide,
  1440 on the 1920 screen.

**Plate** (one app; icons since B59, after the user saw both drawn)

- The plate is the entry's hit area and its selection: the app's icon at
  `size-app-icon-hero` (128) inside `space-s`, so 144 square, `radius-card`,
  filled only when selected (below). Icons are Papirus, scaled from SVG; a
  game's 2:3 cover stands at the icon's height (85 × 128), `radius-small`,
  with a `colour-hairline` edge (`structure/spotlight.md`).
- **No preview** in the row of apps: previews are in the window row only.
- **Window count**: the quiet badge (`size-badge-quiet`) at the icon's lower
  trailing corner.
- **Badge**: the badge component (`size-badge`, `colour-danger`,
  `colour-on-danger`, subheadline semibold, "999+" past three digits) over
  the icon's top trailing corner.
- **Progress**: the progress component (6 high, track `colour-fill-strong`,
  fill `colour-accent`) under the icon inside the plate, three quarters of
  the icon's width. No indeterminate form.
- **Tags**, at the plate's top leading corner, inset `space-xs`, `space-xs`
  apart, attention first: 22-point circles in `material-solid`,
  `shadow-control`, a 13-point glyph in `colour-text-2`; attention filled in
  `colour-warn-text`.

| Tag | Glyph | Also |
|---|---|---|
| Hidden | `eye-slash` | The icon at 50% opacity |
| Minimised only | `minus` (the minimise light's glyph) | |
| Not responding | `hourglass-medium` | |
| Running in background | `tray` | See background entries |
| Attention | `bell-simple-ringing` in `colour-warn-text` | |
| Attention, urgent | `bell-simple-ringing` fill weight, `colour-on-danger`, on a `colour-danger` capsule | |
| Quitting (after Q) | `circle-notch` | Until the app has gone |

**Selection** (recommended, Choices 1): the selected plate filled
`colour-fill-strong`, as macOS's grey plate behind the selected icon. With
the window row open, the app's plate turns `colour-selection-unfocused` and
the selected window's plate takes `colour-fill-strong`.

**Name**: the selected app's name in `type-style-headline`, `colour-text`,
centred under its icon and kept inside the panel, as macOS's, one line, cut at its end, on a capsule of
glass padded `space-m` and `space-xs`, which the floor raises over a light
backdrop (B58); not shown while the window row is open. A state follows as a second run,
`space-s` after it, in `type-style-callout`, `colour-text-2`: "Hidden",
"Minimised", "Not responding", "Running in background". Other rows' bands
are empty.

**Background entries**: after the last app with a window, a vertical
`colour-hairline` (1, the icon's height) with `space-m` each side, then
plates like any other with the `tray` tag. No count or progress; a badge if
the app keeps one. With many apps the
background group stays whole at the end of the last row; if it does not fit,
it starts its own row, centred, with no hairline.

**Window row** (↓ or `): under the row, a `colour-hairline` across the panel
inside its padding, then one plate per window in Cmd+`'s order, centred in
the panel. A window plate is 256 wide: its 240 × 144 card at `space-s`, the
window's title under it in `type-style-callout`, `space-xs` below the card.
The selected window's plate takes `colour-fill-strong`. A minimised window
carries the `minus` tag.

**Many apps**: past what fits in three quarters of the width, icons shrink
together to `size-app-icon-large` (64; plate 80). The badge, count, tags and
progress keep their size. Past the floor, the row wraps into rows of equal
count, each centred, each with its own name line. At the floor the ultrawide
holds 29 a row, the 1920 screen 16. The window row's plates
(`size-switcher-card-width`, 256, a 240 × 144 card) shrink to
`size-switcher-card-min-width` (160) on their own count.

**Second screen**: the same panel at the same point sizes, centred on the
1920 × 1080 screen when it holds the focused window; five full-size cards
a row before shrinking.

## States

| State | What changes |
|---|---|
| Rest | Nothing drawn; no surface exists. |
| Quick tap | Nothing drawn: Cmd released before `timing-switcher-delay-ms`; no texture is captured. |
| Held | Panel shown, second plate selected (Shift+Cmd+Tab: the last app with a window). |
| Window row open | Panel taller; app plate `colour-selection-unfocused`; window plate selected; title in the window band. |
| One app | One plate (and any background plates); the panel is that narrow. |
| No apps with a window | Background plates alone, no gap or hairline before them; nothing if none. |
| Badges, progress | Badge and bar on their plates; nothing else moves. |
| Attention | The attention tag, normal or urgent, held until the app is activated. |
| Loading | Plates with icon and empty well at once; previews, badges, count and progress fade in as they arrive (Motion). |
| Error (protected surface) | The well stays empty `colour-fill`; the icon in its place says whose it is. |
| App quitting | Spinner tag; then the plate leaves (Motion); to the background group if it quits to the background. |
| Last window closed | Process running: the plate moves to the background group (Motion). A keep-running app: the plate stays, with the Hidden tag. |
| Not responding | `hourglass-medium` tag; name band "Not responding". |
| During a drag | As Held; the dragged item stays drawn under the pointer, above the panel. |
| Many apps | Shrunk cards, then centred rows of equal count. |
| Second screen | As Held, centred on that screen. |
| Game Mode (windowed or full-screen game) | `material-solid`, no blur; no transitions (Motion). Previews live. |
| Full-screen app in front | As Held. |
| Locked | Never drawn. |
| Focus on | Unchanged. |
| Reduce Motion | Fades in place (Motion). |
| Reduce Transparency | `material-solid`. |
| Increase Contrast | See Accessibility. |
| Largest text size | Bands, badge and count grow; cards do not. |

## Motion

The compositor draws all of it (B7). Content fades with its container,
never on its own clock; exits run quicker than entrances.

| Transition | Form |
|---|---|
| Appear | After `timing-switcher-delay-ms` with Cmd held, the whole panel fades 0 → 1 over `motion-fade-ms`, in place: no scale, no slide (Choices 2). Plates, icons and the name come with the panel's alpha. A second Tab inside the delay draws it at once, so a fast double Tab never selects blind. |
| Late content | A preview, badge, count or bar arriving after the panel is shown fades in over `motion-fade-ms` in its reserved place; nothing reflows. |
| Selection | Instant: the plate is drawn on the new entry in the next frame (Choices 3). Hover selects the same way. |
| Window row open | The panel's lower edge grows on `motion-open` from its app-row height, top fixed; if the window row is wider, the width grows about the panel's centre on the same spring. The row's alpha is the spring's progress. |
| Window row close (↑) | The reverse on `motion-exit`. |
| Commit, Escape | The panel fades 1 → 0 over `motion-exit-fade-ms`. The chosen window is raised in the commit frame, under the fading panel, not after it; a restored window grows from the island (`window-chrome.md`). |
| H | The preview crossfades to or from its hidden form, and the tag in or out, over `motion-fade-ms`. |
| W | The preview crossfades to the next window's texture over `motion-fade-ms`; the count changes in place. In the window row, the closed window's plate leaves as below. |
| A plate leaves (app gone) | Its alpha 1 → 0 over `motion-exit-fade-ms`; then the plates after it close the gap on `motion-move`, and the panel narrows about its centre on the same spring. The selection moves to the next plate, instantly. |
| To the background group (last window closed, process running) | The plate fades out of the row and a new one fades in at the group's end; nothing travels across the row. A keep-running app's last window closing moves nothing: the Hidden tag fades in. |
| Reduce Motion | Every spring becomes `motion-reduced-fade-ms` in place: the panel at its new size crossfades with the old for the window row and for a plate leaving; appear and dismissal are fades of that length. Selection is instant anyway. |
| Game Mode | No transitions: the panel is drawn whole on the first frame after the delay and gone on the commit frame; the window row and leaving plates jump. No backdrop blur is sampled. |

## Accessibility

- **Text roles.** On the glass only `colour-text` (names, titles, the
  count) and `colour-text-2` (the state word); never `colour-text-3`. Tag
  glyphs are `colour-text-2` on `material-solid`; the badge is
  `colour-on-danger` on `colour-danger`, as the component; the urgent tag
  likewise.
- **Increase Contrast.** The panel moves to `material-glass-tinted` with a
  `colour-hairline-strong` edge (components). The selected plate fills
  `colour-control-pressed` with a 1-point `colour-hairline-strong` inner
  edge; the unfocused plate keeps `colour-selection-unfocused` with the
  same edge. Tags gain a `colour-hairline-strong` edge and bold glyphs;
  the progress track a `colour-hairline-strong` edge; the background
  hairline and window-row rule become `colour-hairline-strong`.
- **Reduce Transparency.** `material-solid`; nothing else changes.
- **Focus.** The plate is the keyboard selection; no focus ring is drawn
  in the switcher, since it holds no controls.
- **Largest text size.** Name, state word, title, badge and count scale
  with the text size; a band is its line height plus `space-m`. Cards,
  icons and tags keep their size. A name longer than the panel is cut at
  its end.
- **Hit targets.** A plate is at least 176 × 154, far over
  `size-hit-minimum`; hover and click take the whole plate.
- **Names.** Each plate's name is "{app}" then its states: "{n} windows",
  "{n} notifications", "{n}% done", "needs attention" (urgently),
  "hidden", "minimised", "not responding", "running in background",
  "quitting". The selection's move is announced.
- **Never colour alone.** Hidden, minimised, background and attention each
  have a glyph, and the selected app's state is spelt out in its band.

## Board

At 3440 × 1440 with the 1920 × 1080 screen beside it, dark first, then
light; crops about 1720 × 620 centred on the panel unless noted.

1. **Held, six apps** over windows on the Haze wallpaper: Firefox, kitty
   (3 windows), Steam, Discord (badge 4), Files (progress 40%), Zed.
   Whole screen, then crop. Light the same.
2. **Selection choices** side by side: neutral plate, accent ring, neutral
   ring (Choices 1). Crop of three plates each.
3. **Marks**: hidden, minimised-only, not responding, attention, urgent
   attention, quitting, badge "999+", count, progress; then the background
   group (Steam and a sync client) after the hairline. Crop 1720 × 620.
4. **Selection on a marked app**: name band with state word ("Steam  Running
   in background").
5. **Window row open** on kitty (3 windows, one minimised), title in the
   window band; the app plate unfocused. Crop 1720 × 900.
6. **Game cover**: a game's plate with its cover over a live frame.
7. **Loading and protected**: empty wells with icons; one with a preview
   fading in at 50%.
8. **Many apps**: 12 shrunk; 20 in two rows of 10 (whole screen and a crop
   of the first row's start).
9. **Q feedback** in three frames: spinner, plate at 50% alpha, gap closing.
10. **Second screen**: Held on the 1920 screen with four apps; crop
    1600 × 500.
11. **Game Mode**: Cmd+Tab over a full-screen game, solid panel.
12. **Increase Contrast** and **Reduce Transparency**, dark and light.
13. **Largest text size**: the name band and badge grown.
14. **Motion strips**: appear (0, 50, 100%), window row unfolding (0, 50,
    100%), dismissal; each with its token named; and the harness playing
    them on its spring.

## Choices

1. **The selection.** (a) A neutral plate, `colour-fill-strong`, behind the
   selected card, as macOS; (b) the accent as a 3-point ring round the
   card, as the focus ring; (c) a neutral 2-point ring in `colour-text`.
   **Recommended: (a).** It is the Mac's answer (brief P3); it leaves the
   accent to the one live thing on a card, the progress bar (brief P6); a
   ring on a busy live preview reads as part of the window, and an accent
   ring sits badly beside a red badge when the accent is red or orange.
2. **Appearing.** (a) A fade alone; (b) a fade with a slight scale from
   0.98. **Recommended: (a)**, as the sheet says; a scale on a panel
   1700 wide moves its edges by 17 points, which reads as growth.
3. **Moving the selection.** (a) Instant; (b) the plate slides on
   `motion-move`. **Recommended: (a).** Tab's key repeat outruns any
   spring, wrapping from last to first would sweep the whole row, and
   macOS's plate jumps.

## New tokens

| Group | Name | Value |
|---|---|---|
| `size` | `app-icon-medium` | 48 (icons.md's switcher size; also prompts naming an app) |
| `size` | `switcher-card-width` | 256 |
| `size` | `switcher-card-height` | 160 |
| `size` | `switcher-card-min-width` | 160 (height follows at 16:10) |
| `timing` | `switcher-delay-ms` | 150 |
