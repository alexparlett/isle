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
  (milliseconds) or ends in `opacity`, `weight`, `saturation`,
  `stiffness`, `damping`, `mass` or `mix`, which are plain numbers. Colours are
  sRGB strings: `#rrggbb` or `rgba(r,g,b,a)`. Shadows are CSS shadow
  strings, which the shell parses into its own shadow list.
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
| `colour` | Roles per appearance: `window`, `window-raised`, `window-sunken`, `view`, `sidebar`, `titlebar`, `titlebar-inactive`, `control`, `control-hover`, `control-pressed`, `text`, `text-2`, `text-3`, `text-disabled`, `hairline`, `hairline-strong`, `fill`, `fill-strong`, `selection-unfocused`, `ok`, `warn`, `danger` (fills and marks) and `ok-text`, `warn-text`, `danger-text` (as text), `match`, `live`, `privacy-microphone`, `privacy-camera`, `privacy-screen`, `island`, `island-text`, `scrim`, `shadow`; and `contrast` overrides | Components use roles, never raw colours. `text-3` is for windows only; on glass, `text` and `text-2`. |
| `accent` | `default`, then per accent (`blue`, `purple`, `pink`, `red`, `orange`, `yellow`, `green`, `graphite`) and appearance: `fill`, `on`, `text` | The user's choice (settings.kdl) picks one; components read `colour-accent`, `colour-on-accent`, `colour-accent-text`, which the runtime points at the chosen set. |
| `selection` | `mix` per appearance | A selection on `view` is the accent mixed in at this share; `text` stays on it at 4.5:1. |
| `series` | `a` to `d` per appearance | Categorical colours for graphs; never status. |
| `pane` | Fixed hues for Settings' pane tiles | Data, not the accent. |
| `term` | The 16 ANSI colours per appearance | Terminals and the programs in them (`theming.md`). |
| `icon` | `set`, `weight`, `weight-on`, `weight-contrast`; `theme` per appearance; `cursor`, `cursor-size` | Phosphor, Papirus, Bibata (`icons.md`). |
| `material` | `glass`, `glass-clear`, `glass-tinted`, `solid` per appearance; `blur`, `saturation`, `noise-opacity` | The glass setting interpolates between `glass-clear` and `glass-tinted`, `glass` being its default. Reduce Transparency and Game Mode use `solid`. |
| `type` | `family`, `mono`; `style.<name>.{size, weight, line}` for `large-title`, `title`, `title-2`, `title-3`, `headline`, `body`, `callout`, `subheadline`, `footnote`, `clock`, `mono`; `weight.*` | Inter's optical size follows the size. The user's text size multiplies every style. |
| `space` | `xxs` 2 to `xxxl` 32 | The 4-point grid. |
| `radius` | `window`, `panel`, `card`, `control`, `row`, `small` | Nested corners are concentric: a child's radius is its parent's less the padding between them, never below `small`. |
| `stroke` | `hairline`, `focus-ring`, `focus-offset` | The focus ring is the accent at reduced opacity, keyboard focus only. |
| `size` | The island's strip and pill, bars, rows, the three control heights, the hit minimum, glyph and app icon sizes | |
| `shadow` | `window`, `window-inactive`, `panel`, `menu`, `control` per appearance | |
| `motion` | Springs `morph`, `open`, `exit`, `move`, `switch` as `{stiffness, damping}`; `fade-ms`, `exit-fade-ms`, `reduced-fade-ms` | Springs are critically damped or close to it. Reduce Motion turns each spring into `reduced-fade-ms` in place. Game Mode draws shell surfaces without transitions. |
| `timing` | The hover guards (`hover-dwell-ms`, `hover-grace-ms`, `hover-settle-ms`) and event durations | Behaviour timings shared by the island and banners. |

What installed apps take from these is in `theming.md`. The Cleaner's
kind colours join as a `kind` group when its packet starts.

## What is not a token

The user's choices (appearance, accent, glass position, text size, the
accessibility switches) live in `settings.kdl`, written through the
compositor. Tokens are the design; settings choose within it.
