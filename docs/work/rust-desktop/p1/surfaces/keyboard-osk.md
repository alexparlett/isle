# On-screen keyboard

P1 stage 5, wave C. How the on-screen keyboard looks and moves, in Frost,
in the session and on the lock, from `structure/keyboard-osk.md`
(binding). Values are `tokens.kdl` names; numbers not yet tokens are
listed at the end.

## Anatomy

A panel docked to the bottom of one screen, drawn above the overlay and a
prompt's scrim, under the lock and the idle dim. It never takes focus.

**Panel**

- **Material**: `material-solid`, the screen's full width, square
  corners, a `stroke-hairline` of `colour-hairline` along its top edge, no
  shadow: it is docked, part of the screen's frame like the strip, not a
  floating surface. Solid, not glass: it sits over full-screen video and
  games, where glass would sample a changing frame across 3440 points every
  frame, and key legends want a constant ground.
- **Height**: `space-s` + the completion strip `size-osk-strip` (32) +
  `space-xs` + five key rows + `space-s`. Rows are `size-osk-key-height`
  (44) tall, `space-xs` apart: 236. The panel is 288 tall (20% of the
  ultrawide's height, 27% of the 1920 screen's); 322 with the function row.
- **Key block**: centred on the screen, as wide as the widest row: 14.5
  units at a pitch of `size-osk-key` (56) + `space-xs`, less one gap: 866.
  The same size on both screens; only a screen narrower than the block
  plus `space-xxxl` each side would shrink it.
- **Trailing controls**: centred on the strip's line, `space-l` in from the
  screen's trailing edge, `space-xs` apart: Close (`x`) then More
  (`dots-three`), leading to trailing, plain icon buttons `size-control`
  (24) square, `radius-control`, glyph `size-glyph` (16) in
  `colour-text-2`; hover `colour-control-hover`, pressed
  `colour-control-pressed`. They are outside the key block, so on the
  ultrawide they sit far to the trailing side, clear of the corner point by
  `space-l`.

**Keys**

- **Key**: `size-osk-key` (56) wide per unit, `size-osk-key-height` (44)
  tall, `radius-control`. Wide keys span their units with the gaps: Tab
  and Delete 1.5, Caps Lock and Return 1.75, Shift 2.25, Command 1.25,
  Space 4; the rest 1.
- **Rows**: Escape, the layout's number row (1 to =), Delete; Tab, the
  top letter row, `\`; Caps Lock, the home row, Return; Shift, the bottom
  letter row, Shift; then the key left of 1 on the layout (`` ` `` on a US
  layout, so a backtick and the dropdown terminal's Ctrl+Option+`` ` ``
  can be typed), the globe, Control, Option, Command, Space, Command,
  Option and the arrows. With one layout there is no globe key and
  Control is 2 units. With the function row on, Escape moves up to it,
  the key left of 1 takes Escape's place, and Space is 5 units. Every row
  is 14.5 units.
- **Fills**: letter, digit and symbol keys `colour-control`; named keys
  (Escape, Tab, Caps Lock, Shift, Control, Option, Command, Return,
  Delete, the arrows, the globe) `colour-fill`, a step quieter, as a Mac's
  keyboard viewer sets them apart.
- **Legends**: characters in `type-style-title-3` (15) at
  `type-weight-regular`, `colour-text`, centred; they follow the shift
  level of the latched modifiers and the active XKB layout, as macOS's
  keyboard viewer. Named keys carry macOS's symbols (⎋ ⇥ ⇪ ⇧ ⌃ ⌥ ⌘ ⏎ ⌫) in
  `type-style-body`, `colour-text-2`, leading-aligned at `space-s` on wide
  keys and centred on the rest; the globe is Phosphor `globe` at
  `size-glyph`. Arrows: `caret-left`, `caret-right` full height,
  `caret-up` over `caret-down` as two half-height keys (20 each, `space-xs`
  between) in the inverted T, glyphs at `size-glyph-small`, `colour-text-2`.
- **Function row** (when shown): F1 to F12 and Escape, keys 30 tall
  (`size-control-large`), legends in `type-style-callout`, `colour-text-2`.
- **Hover**: `colour-control-hover` on any key; legends unchanged.
- **Pressed** (from press until release, through key repeat):
  `colour-control-pressed`; legends unchanged. No pop-up magnified key:
  the pointer is not a finger.
- **Latched modifiers**: held for one key, the key fills `colour-accent`
  and its symbol takes `colour-on-accent`; locked, the same fill with a
  bar 12 × 2, radius 1, in `colour-on-accent`, `space-xs` above the key's
  foot, so held and locked differ by a mark, not a shade. The accent is the
  one live thing on the keyboard (brief P6).
- **Caps Lock on**: a dot of `size-mark` (6) in `colour-ok` at the key's
  top leading corner, inset `space-s`, as a Mac key's light; on the lock's
  plate the dot sits in a 1-point `material-solid` ring, as the tray's
  attention dot; absent when off. The key's fill is not changed: Caps Lock
  is a state, not a latch.
- **Accents** (press and hold on a letter): a row of key-sized cells
  above the key, centred on it and kept on the screen, on
  `colour-window-raised` with `shadow-menu`, `radius-card`, padding
  `space-xs`; each cell a key with no fill at rest; the one under the
  pointer `colour-accent` with `colour-on-accent`. Each cell's number
  (1, 2, …) in `type-style-footnote`, `colour-text-2`, under its legend,
  as macOS.

**Completion strip**

- `size-osk-strip` (32) tall across the key block's width, no fill of its
  own. Three equal slots, the likeliest in the middle; between them a
  1 × 16 rule of `colour-hairline`, centred on the strip's height.
- A suggestion: `type-style-body`, `colour-text`, centred in its slot, one
  line cut at its end. Hover: the slot `colour-control-hover` inset
  `space-xxs`, `radius-control`; pressed `colour-control-pressed`.
- Empty (no word under way, a password field, suggestions off): the slots
  blank, the rules gone, the height kept.
- Nothing focused: "Nothing to type into" across the strip, centred,
  `type-style-callout`, `colour-text-2`; the keys keep their fills.
- **More menu**: a menu as `menus.md` draws it, on `material-solid`, its
  foot `space-xs` above the panel, trailing edge on the More button's.
  From a secondary click on the panel's margin it opens at the pointer
  (`menus.md`).

**On the lock and the greeter**

- The lock draws its own keyboard in its own layer, the same anatomy with
  three changes: the panel is a lock plate (`material-glass` at the user's
  glass position, `material-noise-opacity`, no blur pass, as `lock.md`),
  with no top hairline; there is no completion strip and no More (the
  panel is 252 tall, 286 with function keys); Close hides it, as the
  utilities' keyboard button does, and sits centred on the first key
  row's line, `space-l` in from the trailing edge.
- The face moves up by the panel's height: the lock's positions measured
  from the screen's foot (the power row and utilities) are measured from
  the panel's top, and the password field's centre moves from 80% of the
  screen's height to 80% of the height above the panel. The date and time
  stay where they are; the notification column loses rows from its end if
  it would meet the user.
- The keyboard button in the utilities shows it on: its circle filled
  `colour-fill-strong` with `keyboard` in Fill.

## States

| State | What changes |
|---|---|
| Closed | Nothing drawn; the work area is whole. |
| Open | The panel; the work area ends at its top hairline. |
| Typing a word | Up to three suggestions in the strip. |
| Password field | The strip blank, rules gone. |
| Modifier held, locked | The accent fill; locked adds the bar. |
| Caps Lock on | The dot on Caps Lock. |
| Nothing focused | "Nothing to type into"; key presses draw their pressed fill and send nothing. |
| A prompt or Spotlight open | The panel above them and their scrim, unchanged. |
| Full-screen window | The panel over its bottom, unchanged. |
| Game in front, full screen | The panel over its bottom; no transitions. |
| Game in front, windowed | As Open. |
| Second screen | On one screen only, sized as on the ultrawide. |
| Locked | The session's panel hidden; the lock's own when its button is used. |
| Focus on | No change. |
| Reduce Motion, Transparency, Contrast, largest text | Motion and Accessibility. |

## Motion

| Transition | Form |
|---|---|
| Open | The panel rises from below the screen's foot to its place on `motion-open`, anchored to the foot; its content rides with it. On the same frame the work area shrinks, and windows that must give way move or resize to it on `motion-move` (`window-chrome.md`'s fill and tile motion); things placed on the work area's foot (the capture toolbar and thumbnail, picture-in-picture) move up on `motion-move`. |
| Close | The reverse on `motion-exit`: the panel down past the foot, the windows back to their boxes on `motion-move`. |
| Key press, release, hover | Instant: fills change in the next frame, as menus' rows. |
| Latch, lock, unlatch | Instant. |
| Suggestions change | In place, in the frame the word changes; nothing slides. |
| Accents row | Appears at once after the hold (the keyboard's repeat delay); goes at once on release. |
| Move to {display} | A close on one screen and an open on the other from the same frame; nothing travels between screens. |
| On the lock | The lock's panel rises on `motion-open` while the face moves up by the same height on the same spring, so the field stays the same distance above the keys throughout; closing reverses both on `motion-exit`. Dozing fades it with the field over `motion-exit-fade-ms`; a wake fades both back over `motion-fade-ms`. |
| Reduce Motion | The panel fades in and out in place over `motion-reduced-fade-ms`; windows jump to the new work area with `window-chrome.md`'s crossfade; on the lock the face jumps and the panel fades. |
| Game Mode | Cuts: the panel whole on its first frame, gone on the next; windows jump. |

## Accessibility

- **Colour roles.** Legends `colour-text` on `colour-control` and
  `colour-text-2` on `colour-fill`, over `material-solid`: window-like
  roles on an opaque ground, so `contrast.py` checks them as such; the
  latched legends `colour-on-accent` on `colour-accent`. On the lock's
  plate, glass rules hold: `colour-text` and `colour-text-2` only.
  `colour-text-3` is not used.
- **Increase Contrast.** Every key and strip slot gets a `stroke-hairline`
  inner edge of `colour-hairline-strong`; the strip's rules and the
  panel's top edge become `colour-hairline-strong`; glyphs step to Bold;
  the locked bar grows to 3 tall.
- **Reduce Transparency.** No change in the session (solid already); the
  lock's panel takes `material-solid`.
- **Focus.** None: it never takes focus and draws no ring. Its own
  controls are reached by ⌥⌘F5 and Settings, not by Tab.
- **Largest text.** Legends and suggestions scale; key heights grow to the
  legend's line plus `space-l`, the units in proportion, the panel with
  them, until the panel is `size-osk-max-scale` (0.33) of the screen's
  height; past it the legends stop growing. The block may then reach
  the screen's width less `space-xxxl` each side.
- **Hit targets.** Keys 56 × 44; the half-height arrows 56 × 20, at
  `size-hit-minimum`; trailing buttons 24; suggestion slots the strip's
  height. The block never reaches a bottom corner.
- **Never colour alone.** Locked has its bar; Caps Lock its dot; the
  accessible names say "{key}, held" or "{key}, locked".

## Board

At 3440 × 1440 with the 1920 × 1080 screen beside it, dark first, each in
light. Crops 1600 × 400 of the bottom centre `[920, 1040, 2520, 1440]`
unless noted.

1. **Open** under two tiled windows: whole screen, then the crop.
2. **Typing**: "keyb" with three suggestions; a key hovered and one
   pressed.
3. **Modifiers**: Shift held (legends shifted), Command locked, Caps Lock
   on, Option held (Option legends).
4. **Accents**: e held, the row above it, one cell under the pointer.
5. **States of the strip**: password field (blank), nothing focused.
6. **Function row** on; one layout (no globe); and a German layout's
   legends.
7. **Over a full-screen video**, and **over a prompt's scrim** typing into
   it.
8. **Second screen**: open on the 1920 screen; crop `[160, 680, 1760, 1080]`.
9. **On the lock**: the face moved up and the lock plate panel; dark and
   light over two wallpapers; whole screen.
10. **Increase Contrast** and **largest text** (the panel at its cap).
11. **Motion**: open with windows reflowing, close, and the lock's rise;
    each with Reduce Motion and Game Mode.

## New tokens

| Group | Name | Value |
|---|---|---|
| `size` | `osk-key` | 56 (one unit's width; wide keys span units and gaps) |
| `size` | `osk-key-height` | 44 |
| `size` | `osk-strip` | 32 |
| `size` | `osk-max-scale` | 0.33 (the panel's largest share of the screen's height) |
