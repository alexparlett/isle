# Focus and Game Mode

P1 stage 5, wave B. How a Focus shows: its symbol, the island's glyph and
mode event, Control Centre's Focus module and its page, and Game Mode's
place among them. Structure is binding from `structure/focus.md`,
`structure/control-centre.md` (the module and its page) and
`structure/island.md` (the glyph and the event). Focus has no surface of
its own: it uses the island's forms and Control Centre's frames as
`surfaces/island.md` draws them. Settings › Focus is `settings.md`'s; the
summary after silence is `notifications.md`'s. Values are `tokens.kdl`
names.

## Anatomy

### Symbols

Phosphor, Regular for off and Fill for on, as every on/off glyph.

| Mode | Symbol |
|---|---|
| Do Not Disturb | `moon` |
| Game Mode | `game-controller` |
| The user's own | One of `briefcase`, `bed`, `user`, `book-open`, `car`, `barbell`, `code` (icons.md's set), chosen in Settings |

`moon` and `game-controller` are kept for their modes and not offered for
the user's own, so the glyph alone always says which of the two is on.

### The island

| Part | Look |
|---|---|
| Glyph | The on mode's symbol at `size-glyph` (16), Fill, `colour-island-text`, in the status glyphs' fixed place (after VPN), hit box 24 by the pill's height (`island.md`). One glyph whatever the mode; Game Mode has no second |
| Mode event | The island's line form: 30 tall, inset `space-m`, the symbol at 16 (Fill for "on", Regular for "off"), `space-s`, "{Focus} on" or "{Focus} off" in `type-style-body` medium, `colour-island-text`. Shown for `timing-mode-ms` |

### The Focus module

`island.md`'s module frames; Small and Medium (the default).

| Part | Off | On |
|---|---|---|
| Circle | `size-module-glyph` (32), `colour-fill-strong`; the last-used mode's symbol (Do Not Disturb until another is used) at `size-glyph-large` (20), Regular, `colour-text` | `colour-accent`; the on mode's symbol, Fill, `colour-on-accent` |
| Name (Medium) | "Focus", `type-style-headline`, `colour-text` | The mode's name, the same style, cut at its end |
| State (Medium) | "Off", `type-style-subheadline`, `colour-text-2` | How it ends or why it is on: "Until 07:00", "For 47 min", "Until the end of {event}", "Schedule", "{app}", the game's name; tabular where a time. On by hand with no end: no state line, the name centred on the circle |
| Small | The circle alone; name and state in the tooltip | The same |

The circle is the switch (it turns the last-used mode on or off); the rest
of the tile opens the page. Hover, pressed, focus and disabled are the
frame's.

### The Focus page

`island.md`'s page frame: the pill's row above, the head, a card of rows,
the foot.

| Part | Look |
|---|---|
| Head | The module as a Wide tile: back button (`caret-left`), the circle, then the name over the state line as in Medium. No trailing switch: the circle is it, as in the grid |
| Mode rows | In one card, Do Not Disturb, Game Mode, then the user's modes in the order made. Every row `size-row-large` (40), so a row does not change height when it turns on: inset `space-m`; a 24 circle (`size-control`) holding the symbol at `size-glyph` (16), off `colour-fill-strong` with Regular `colour-text`, on `colour-accent` with Fill `colour-on-accent`; `space-m`; the name in `type-style-body` `colour-text` over the secondary line in `type-style-subheadline` `colour-text-2`; a disclosure `caret-right` at `size-glyph-small`, `colour-text-2`, at the trailing inset, `caret-down` while open. A row with nothing to say on its second line centres its name |
| Secondary line | On: the module's state line ("Until 07:00", "Elden Ring"). Off: the first schedule, as Settings states it ("Weekdays 22:00–07:00", "While Zed is in front", "When a game is in front"), with "+{n}" for more; nothing if none. Game Mode off for the game in front: "Off for {game}" |
| Durations | Opened from the disclosure, in place under the row, inside the same card: `size-row` (28) rows, labels aligned with the mode's name (from 48), `type-style-body` `colour-text`, no glyph: "For 1 hour", "Until this evening" or "Until tomorrow morning", "Until the end of this event" while one is in progress. For Game Mode with a game in front, its first row is "Off for {game}" |
| Row states | Hover `colour-fill-strong` at `radius-row`; pressed `colour-control-pressed`. The on mode is told by its accent circle and Fill symbol: no check as well |
| Foot | "Focus Settings…", plain small button, leading, `type-style-callout`, `colour-text-2` |

### Game Mode's holds elsewhere

Shown only on the modules that own them, as their state lines: Energy
Mode's "High Power while Game Mode is on", Night Light's "Off during Game
Mode" (Control Centre's spec draws those modules). Nothing at rest.

## States

| State | What shows |
|---|---|
| Off | No glyph; module off: last-used symbol, "Focus", "Off" |
| On by hand | Glyph; module on with the name and no state line, or its duration ("For 47 min") |
| On by schedule | Glyph; state "Until {time}"; the mode event at start and end |
| On by an app in front | Glyph; state "{app}" |
| Game Mode, windowed game | `game-controller` glyph; state the game's name; "Game Mode on" in one frame; the island and panel solid and still |
| Game Mode, full-screen game | Nothing; the glyph when the island returns at the edge |
| Game Mode by hand, no game | As on by hand |
| Game Mode off for this game | No glyph; Game Mode's row reads "Off for {game}" |
| Two due at once | Only the winner shows anywhere |
| Ending | The glyph goes; "{Focus} off" if automatic; the summary banner if two or more were held |
| Second screen | Every island shows the glyph; the page is the same at the same size |
| Locked, idle | No events; the lock shows the symbol (`lock.md` places it) |
| Error | Calendar gone: no "Until the end of this event". Service gone: glyph and module keep their last state, controls in `colour-text-disabled` until it answers |
| Reduce Motion, Reduce Transparency, Increase Contrast, largest text | As the island and Control Centre, below |

## Motion

| Transition | How |
|---|---|
| Glyph appears or goes | The island's morph: the pill widens on `motion-morph` by the glyph's 24 and the glyph fades in by p from half; it goes on `motion-exit` |
| Automatic change or chord | Rest → the line event on `motion-morph`; after `timing-mode-ms`, `motion-exit` to rest, the glyph now among the status glyphs (or gone). The event's symbol does not travel to the glyph's place: it fades out with the event and the glyph fades in with the rest form |
| Module or row on and off | The circle's fill and the symbol's weight crossfade in place over `motion-fade-ms` (on) and `motion-exit-fade-ms` (off); the name and state line crossfade the same, left-aligned, so nothing moves |
| Page opens and back | The island's page clip (`motion-open`, back `motion-exit`) |
| Durations open | The card's height grows on `motion-open` below the row, top fixed, rows in by p from half; the caret turns to `caret-down` in one frame. Closing on `motion-exit`, rows out by the first half |
| Reduce Motion | The island's crossfade in place over `motion-reduced-fade-ms`; durations appear at the new height with a fade of that length |
| Game Mode | No transitions: each state replaces the last in one frame, events included |

## Accessibility

- **Colour roles.** On the island, `colour-island-text` only. In the
  panel, `colour-text` for names and durations and `colour-text-2` for
  state and secondary lines, on `colour-fill` tiles and cards over glass;
  never `colour-text-3`. The accent circle carries `colour-on-accent`.
- **Never colour alone.** On is Fill against Regular as well as the
  accent; the state line names the mode; a row's on state is announced.
- **Increase Contrast.** As `island.md`: glass tinted, edges on tiles and
  cards, symbols at `icon-weight-contrast` when off (Fill stays Fill).
- **Reduce Transparency.** The island's and panel's solid forms.
- **Focus ring.** On the island's glyph, the pill's capsule ring
  (`island.md`). In the panel: round the tile at `radius-card` plus
  `stroke-focus-offset`, round a row or duration row inside its card at
  `radius-row` plus the offset. Up and Down move between modes, Right and
  Left open and close durations.
- **Largest text size.** The island's rules (glyph to 24); rows grow with
  their lines (40 to 54 at 1.5×) and circles to 32; the state line may
  wrap to two lines on Medium, as `island.md` allows.
- **Hit targets.** The glyph's box 24 by the pill; circles 32 and 24;
  rows 40 and 28; all above `size-hit-minimum`.

## Board

At 3440×1440 and 1920×1080, dark first and each in light. Crops x, y,
width, height: pill scenes 920, 0, 1600, 300; panel scenes 920, 0, 1600,
900; the second screen's 160, 0, 1600, 400.

1. **Symbols**: the nine at 16 and 20, Regular and Fill, dark and light.
2. **Glyph in the pill**: Do Not Disturb and Game Mode at rest, beside
   Wi-Fi and Bluetooth.
3. **Mode events**: "Do Not Disturb on", "Work off", "Game Mode on" in its
   solid form.
4. **Module**: Medium off, on by hand (no state line), "Until 07:00",
   Game Mode "Elden Ring"; Small off and on; hover and focus.
5. **Page**: Do Not Disturb on until 07:00, Game Mode with its trigger,
   two user modes with schedules; the focus ring on a row.
6. **Durations**: Do Not Disturb's open; Game Mode's open with "Off for
   Elden Ring".
7. **Off for this game**: the row reading "Off for {game}", no glyph.
8. **Holds**: Energy Mode and Night Light tiles during Game Mode.
9. **Windowed game**: the solid pill with `game-controller` and the solid
   panel at the Focus page.
10. **Second screen**: the 1920 island with the glyph.
11. **Settings**: scenes 4 and 5 under Increase Contrast, Reduce
    Transparency and the largest text size.
12. **Motion**: an automatic start (rest → event → rest with glyph), a
    toggle in the module, durations opening; each under Reduce Motion and
    Game Mode.

## New tokens

None. Every value is an existing token or `island.md`'s.
