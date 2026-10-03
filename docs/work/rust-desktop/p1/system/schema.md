# Token schema

P1 stage 4. The shape of `tokens.kdl`, which the shell and Isle's apps
read and which P3 implements. The file is KDL v2; the values are in
`tokens.kdl` beside this note; `contrast.py` checks them.

## Reading the file

- **Paths.** A token is named by its node path, joined with `-`:
  `type { style { body { size 13 } } }` is `type-style-body-size`. The
  boards map each path to a CSS custom property of the same name; the
  shell maps it to a field of its token struct.
- **Appearances.** A `dark` or `light` node anywhere on a path splits the
  tokens under it by appearance: `colour { dark { text "…" } }` and
  `shadow { window { dark "…" } }` both give a `colour-text` or
  `shadow-window` per appearance. A token without one applies to both.
- **Increase Contrast.** `colour { contrast { dark { … } light { … } } }`
  replaces the roles it names while the setting is on. Under it the
  shell's glass uses `material-glass-tinted`.
- **Units.** Numbers are logical points unless the name ends in `-ms`
  (milliseconds), ends in `opacity`, `weight`, `saturation`,
  `stiffness`, `damping`, `mass`, `mix`, `scale` or `scale-max`, or sits
  under `limit`; those are plain numbers. `motion-flick-speed` is in points
  per second. Colours are sRGB strings: `#rrggbb` or `rgba(r,g,b,a)`.
  Shadows are CSS shadow strings, which the shell parses into its own
  shadow list.
- **Version.** `version 1` at the top. A file with a newer major version
  is refused with a pointer to the error, as `settings.kdl` is (B27).
- **Errors.** A file that does not parse, or a required token that is
  missing, keeps the last good tokens in force and raises the island's
  configuration-error glyph (`structure/island.md`). An unknown token is
  ignored with a warning in the log, so a newer file degrades.
- **Reload.** The file is watched; a valid change applies to every
  surface and Isle app at once, without restarting anything (B10).

## Groups

| Group | Tokens | Notes |
|---|---|---|
| `colour` | Roles per appearance: `window`, `window-raised`, `window-sunken`, `view`, `sidebar`, `titlebar`, `titlebar-inactive`, `control`, `control-hover`, `control-pressed`, `text`, `text-2`, `text-3`, `text-disabled`, `hairline`, `hairline-strong`, `fill`, `fill-strong`, `selection-unfocused`, `ok`, `warn`, `danger` (fills and marks), `on-danger` (a label on `danger`) and `ok-text`, `warn-text`, `danger-text` (as text, a destructive button's label included), `match`, `live`, `privacy-microphone`, `privacy-camera`, `privacy-screen`, `island`, `island-text`, `scrim`, `shadow`, `dim`, `veil`, `capture-line`, `capture-line-shadow`; and `contrast` overrides | Components use roles, never raw colours. `text-3` is for windows only and never on a fill; on glass, `text` and `text-2`, and on a `fill-strong` plate on glass `text` only. `text-disabled` is for disabled controls only, on windows or glass (WCAG exempts them). A focused selection in lists and tables is the full `accent` fill with `on-accent` text; unfocused, `selection-unfocused`. A badge's count is `on-danger` on `danger`. |
| `accent` | `default`, `hover-mix`, `pressed-mix`, then per accent (`blue`, `purple`, `pink`, `red`, `orange`, `yellow`, `green`, `graphite`) and appearance: `fill`, `on`, `text` | The user's choice (settings.kdl) picks one; components read `colour-accent`, `colour-on-accent`, `colour-accent-text`, which the runtime points at the chosen set. A default button's hover and pressed fills mix the inverse of `on` into `fill` at `hover-mix` and `pressed-mix`, so the label's contrast rises. |
| `selection` | `mix` per appearance | Text selection only: in fields and text views the accent is mixed into the surface at this share, and `text` stays on it at 4.5:1. Row and item selections are `colour`'s. |
| `series` | `a` to `d` per appearance; `area-opacity` | Categorical colours for graphs, calendars and the System widget, used in order and never status; any two may meet (`data-colours.md`). An area under a line is its series at `area-opacity`. |
| `kind` | Cleaner's ten kinds per appearance, `games` to `other` in their fixed order; `mix` | Data colours, never on a control; neighbours in the order are told apart (`data-colours.md`). A map tile is its kind mixed into `colour-view` at `mix`. |
| `pane` | Fixed hues for Settings' pane tiles | Data, not the accent. |
| `term` | The 16 ANSI colours per appearance | Terminals and the programs in them (`theming.md`). |
| `traffic` | The traffic lights' fills, pressed fills, glyphs and edge | The one fixed exception to colour roles. |
| `annotation` | `default`, then Markup's fixed palette, `red` to `white` | One set for both appearances (`surfaces/capture.md`). |
| `limit` | Counts: `island-marks`, `now-rows`, the surfaces' row and card caps | Plain numbers, never points. |
| `icon` | `set`, `weight`, `weight-large`, `weight-large-from`, `weight-on`, `weight-contrast`; `theme` per appearance; `cursor`, `cursor-size` | Phosphor, Papirus, Bibata (`icons.md`). Outlines are `weight`, and `weight-large` from `weight-large-from` points up. |
| `material` | `glass`, `glass-clear`, `glass-tinted`, `solid` per appearance; `blur`, `saturation`, `noise-opacity`; `softened` per appearance, `softened-blur`, `softened-saturation`; `lock-wash`, `lock-wash-contrast` per appearance | The glass setting interpolates between `glass-clear` and `glass-tinted`, `glass` being its default. Reduce Transparency and Game Mode use `solid`. `softened` is the wallpaper under the lock and the dashboard, blurred then washed; the lock adds its own wash. |
| `type` | `family`, `mono`; `style.<name>.{size, weight, line}` for `large-title`, `title`, `title-2`, `title-3`, `headline`, `body`, `callout`, `subheadline`, `footnote`, `clock`, `mono`, `display` (the lock's time), `figure` (a widget's one large figure); `weight.*`; `scale-max` | Inter's optical size follows the size. `scale-max`: the largest text size the user can set; every style multiplies by the user's factor, 1 to `scale-max`. |
| `space` | `xxs` 2 to `xxxl` 32 | The 4-point grid. |
| `radius` | `window`, `panel`, `card`, `control`, `row`, `small` | Nested corners are concentric: a child's radius is its parent's less the padding between them, never below `small`. A progress or capacity bar and a badge are capsules: radius half their height, not a token. |
| `stroke` | `hairline`, `focus-ring`, `focus-offset`, `progress`, `found`, `graph`, `kind` | The focus ring is the accent at full strength, keyboard focus only. On glass a `stroke-hairline` line of `colour-text` fills its offset, inside it. |
| `size` | The island's strip and pill, bars, rows, the three control heights, the two badge heights, the hit minimum, glyph and app icon sizes, and each surface's own | Control and badge heights are minimums: at a larger text size each grows to its label's line plus the same padding. |
| `shadow` | `window`, `window-inactive`, `panel`, `menu`, `control`, `bar` per appearance | |
| `motion` | Springs `morph`, `open`, `exit`, `move`, `switch` as `{stiffness, damping}`; `fade-ms`, `exit-fade-ms`, `reduced-fade-ms`, `blink-ms`, `wallpaper-fade-ms`, `dim-ms`; `window-scale`, `panel-scale`; `shake-ms`, `shake-distance`; `flick-speed`, `switch-distance` | Springs are critically damped or close to it. Reduce Motion turns each spring into `reduced-fade-ms` in place. Game Mode draws shell surfaces without transitions. |
| `timing` | The hover guards (`hover-dwell-ms`, `hover-grace-ms`, `hover-settle-ms`) and event durations | Behaviour timings shared by the island and banners. |

What installed apps take from these is in `theming.md`.

## What is not a token

The user's choices (appearance, accent, glass position, text size, the
accessibility switches) live in `settings.kdl`, written through the
compositor. Tokens are the design; settings choose within it.
