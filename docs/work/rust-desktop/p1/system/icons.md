# Icons, app icons and the cursor

P1 stage 4, the system. Which symbol set the shell draws, which icon theme
installed apps get, which cursor, and the sizes, for gate 4. Read and
checked on 2026-10-03; versions are the projects' own, licences from their
repositories and the Arch package database. Claims not checked against a
source are marked *(unsure)*.

Constraints from the brief: free to ship in an Arch package, nothing from
SF Symbols or Apple artwork; app icons keep their own shape (no squircle,
no grey tile); no icons on menu items; principle 2, one desktop.

## Summary

| What | Recommendation | Licence | Ships as |
|---|---|---|---|
| Shell symbols | **Phosphor 2.1**: Regular for outlines, Fill for "on", Bold under Increase Contrast; gaps drawn by Isle on Phosphor's grid | MIT | SVGs vendored into the shell crate, drawn by Skia's SVG DOM |
| App icons | **Papirus** (Papirus-Dark in dark, Papirus in light), each app's own hicolor icon as fallback | GPL-3.0 | `papirus-icon-theme`, Arch extra |
| Isle's five apps | Drawn for Isle as one family, SVG in `hicolor/scalable/apps` | the repo's | in Isle's package |
| Cursor | **Bibata Modern Classic**, size 24, in both appearances | GPL-3.0 | xcursor, `bibata-cursor-theme` (AUR) |

## Shell symbols

### What the sheets need

From `structure/`: the island's status glyphs (Wi-Fi with strength and
none, Bluetooth connected, sound muted, VPN, the Focus symbol, updates
installing and restart owed, battery levels and charging, shortcuts
inhibited, configuration error) and privacy marks; Control Centre's
modules (Wi-Fi, Bluetooth, Now Playing, Focus, VPN, Sound, Display, Night
Light, Energy Mode, Updates, Batteries, Dark Mode, Keyboard layout,
Accessibility, Screenshot, Phone); Focus's "small set" of symbols (Do Not
Disturb, Gaming, Work, Sleep, Personal, Reading, Driving, Fitness, Coding);
about thirty Settings panes; Files' kinds (folder, document, image, audio,
video, archive, code, PDF, disc); transport (play, pause, skip, seek,
shuffle, repeat); and the controls (chevrons, check, close, search, add,
remove, trash).

Two needs decide most of it. Control Centre and the island show **on
against off** as filled against outlined (`control-centre.md`, Increase
Contrast: "a filled glyph against an outlined one"), so the set needs a
fill for every glyph that can be on. And status glyphs show **levels**
(Wi-Fi strength, volume, battery), so the set needs level forms on one
geometry.

### Candidates

| | Lucide | Phosphor | Tabler | Material Symbols Rounded | Fluent UI System | Remix |
|---|---|---|---|---|---|---|
| Version read | 1.51.0, 2026-10-03 | 2.1.1, 2024-03-29 (npm); repository active in 2026 | 3.48.0, 2026-09-22 | 0.47.6 (npm), 2026-10-02 | 1.1.343, 2026-09-25 | 4.x |
| Size of set | about 1,850 | 1,512, each in six weights | 5,130 outline, 1,054 filled | 3,926 | about 3,000 names, each at several sizes | about 3,000 *(unsure)* |
| Weight control | stroke width, any value | Thin, Light, Regular, Bold (and Duotone), baked | stroke width, any value | weight 100–700, grade, optical size 20–48 (font axes) | Light (some), Regular, drawn per size 10–48 | none |
| Filled form | none (a community fork, unofficial) | Fill for every glyph | about a fifth of the set | a fill axis for every glyph | Filled for every glyph | Fill for most |
| Levels on one geometry | Wi-Fi zero, low, high; battery low, medium, full; volume 1, 2, x *(from memory)* | Wi-Fi none, low, medium, high, slash; speaker none, low, high, x; battery empty to full, charging, warning; Bluetooth connected, slash | Wi-Fi 0–2; battery 1–4; volume 2–3 *(unsure)* | Wi-Fi 1–3 bars; battery 0–6 bars; volume mute, down, up, off | Wi-Fi 1–4; battery 0–10; speaker 0–2 | some |
| VPN glyph | no | no | yes *(unsure)* | `vpn_key`, `vpn_lock` | `shield_lock` | yes |
| Reads like SF Symbols | close in line, but thin and with no fill it reads as web UI | closest in drawing: monoline, round caps and joins, a fill that is the same shape solid | heavier 2-px line; reads as a dashboard | closest in its model (weight matched to text, optical size, grade) but the drawings read as Android | reads as Windows 11 | reads as web UI |
| Licence | ISC | MIT | MIT | Apache-2.0 | MIT | Remix Icon License 1.0 since January 2026: not OSI, forbids competing icon sets and logo use |
| Form | stroked SVG on 24 | filled-outline SVG on 256, one file per weight | stroked SVG on 24 | variable font; static SVG per weight on 960 | filled-outline SVG per size | SVG, font |

**Phosphor.** It is the only set with every glyph in a matching outline
and fill, levels drawn on one geometry, and weights that keep the line
close to Inter's stem at every size. Its line reads nearest SF Symbols':
even, rounded, with a fill that is the outline made solid rather than a
different drawing. v1's first build used it as a Regular and Fill state
pair (pre-reset D14); the reset moved to Lucide to rebuild fast, not
because Phosphor failed (`design-v1-lessons.md` §3).

Why not the others:

- **Lucide** has no fill, so on and off would differ by colour alone,
  which Increase Contrast forbids. Freya bundles it (`freya-icons`), which
  is convenient but not a reason.
- **Material Symbols Rounded** has the best model and the most coverage,
  but its drawings are Google's and a desktop of them reads as Android.
  Its static SVGs fix the optical size at 24, so the model is only had
  through the font.
- **Fluent** is crisp at small sizes because it is drawn per size, but it
  reads as Windows.
- **Tabler**'s fill covers a fifth of the set and its 2-px line is heavy
  beside Inter at 13.
- **Remix** changed to a licence that is not open source in January 2026.

Phosphor's risk is upkeep: its core package has not been released since
March 2024. Isle vendors the SVGs it uses, so a stalled upstream costs
nothing already shipped; a glyph it lacks is drawn by Isle.

### How it is used

- **Weights by size**, so the line stays between 1 and 1.5 px: Regular at
  16 to 24 (1.0 to 1.5 px), Light at 32 and above (1.5 px and up). Fill
  for "on". Increase Contrast steps one weight up (Regular to Bold). At 16,
  Regular's 1.0-px line is what v1's Lucide 1.5 on a 24 grid drew at 16
  (`isle-v1-inventory.md` §14), which the user accepted at true scale.
- **Levels in two layers**: the full glyph (`wifi-high`, `speaker-high`)
  in `text-3`, the level glyph over it in the glyph's colour. Phosphor's
  level glyphs share the full glyph's paths (checked for Wi-Fi and the
  speaker), so the empty bars show dimmed, as SF Symbols' variable colour
  does.
- **Isle-drawn glyphs** for what Phosphor lacks, on its 256 grid with its
  stroke, caps and joins: VPN (a shield over the network glyph), shortcuts
  inhibited (keyboard with a slash), the island's activity marks if the
  set has none. Kept in the same vendored directory, with the source SVG.
- **One symbol per thing**, in a table in the shell crate that the sheets
  reference by name; a sample of the mapping:

| Use | Glyph |
|---|---|
| Wi-Fi strength, none | `wifi-low`, `-medium`, `-high`; `wifi-slash` |
| Bluetooth, connected | `bluetooth`, `bluetooth-connected` |
| Sound, muted | `speaker-low`, `-high`; `speaker-slash` |
| Battery, charging | `battery-empty` to `battery-full`; `battery-charging` |
| Updates installing, restart owed | `arrow-circle-up`; `arrows-clockwise` |
| Configuration error | `warning-circle` |
| Focus: Do Not Disturb, Gaming, Work, Sleep, Personal, Reading, Driving, Fitness, Coding | `moon`, `game-controller`, `briefcase`, `bed`, `user`, `book-open`, `car`, `barbell`, `code` |
| Display, Night Light, Energy, Dark Mode | `sun`, `sun-horizon`, `lightning`, `circle-half` |
| Accessibility, Keyboard, Screenshot, Phone | `person-arms-spread`, `keyboard`, `selection-plus`, `device-mobile` |
| Transport | `play`, `pause`, `skip-back`, `skip-forward`, `rewind`, `fast-forward`, `shuffle`, `repeat` |

- **Drawing in Freya.** Freya's `SvgViewer` parses the SVG with Skia's SVG
  DOM, sets `currentColor` and rasterises at the size it is given, caching
  per colour (read in the fork, `freya-components/src/svg_viewer.rs`).
  Phosphor's files fill with `currentColor`, so a token colour reaches them
  directly. The raster must be made at the box times the output's scale, in
  device pixels, and placed on whole pixels: v1's glyphs drew at double
  size for three rounds because a 2× raster was drawn at 2× its box
  (`design-v1-lessons.md` §3.2). A glyph box is checked at true scale on
  the ultrawide before its sizes are final.
- **Settings pane icons** are a coloured tile with a white Fill glyph (the
  Frost direction's coloured pane icons). The tile colours are a fixed set
  of system hues in `tokens.kdl` (`pane.*`), never the accent: the accent
  marks the selected row (principle 6).
- **Menus carry no icons** (brief). Isle's apps' toolbars use the same set.

## App icons for installed apps

| | Papirus | WhiteSur | Colloid | Tela | Fluent | Qogir | Reversal | McMojave-circle |
|---|---|---|---|---|---|---|---|---|
| Last change | release 2026-08-01; pushed 2026-09-21 | 2026-09-10 | 2026-08-29 | 2026-08-10 | 2026-08-30 | 2025-11-04 | 2026-03-17 | 2025-12-08 |
| In Arch | extra (`papirus-icon-theme` 20260801) | AUR | AUR, git only | AUR | AUR *(unsure)* | AUR *(unsure)* | AUR *(unsure)* | AUR *(unsure)* |
| Shape | each app's own silhouette, redrawn flat | every app in a Big Sur squircle | soft flat, mixed shapes *(unsure)* | flat, mixed shapes | Windows 11 style | flat, mixed | every app in a rounded tile | every app in a circle |
| macOS feel | low: flat | highest | medium *(unsure)* | low | none | low | medium | medium |
| Forces a shape | no | yes | no *(unsure)* | no | no | no | yes | yes |
| Coverage | the widest: 8,321 app icons at 64 here, 387 Steam games by app id, Lutris games, Steam, Heroic, VS Code, RustRover, Zed, kitty, Discord, OBS | wide | medium | medium | medium | narrow | medium | narrow |
| Licence | GPL-3.0 | GPL-3.0; carries Apple's logo and Apple-derived app icons by default | GPL-3.0 | GPL-3.0 | GPL-3.0 | GPL-3.0 | GPL-3.0 | GPL-3.0 |

**Papirus.** WhiteSur, Reversal and McMojave-circle are out on the brief
(they force a shape), and WhiteSur also carries Apple artwork. Of the rest,
Papirus is the only one with the coverage this machine needs (Steam games
by app id, launchers, dev tools), the only one in Arch's own repositories,
and the best kept. Its flat style is less Mac than the squircle themes,
but it keeps each app's silhouette, which is what the brief asks; macOS's
own look comes from the shell, not from third-party icons. Papirus-Dark
and Papirus differ only in their symbolic icons, so the theme follows the
appearance and app icons stay the same.

Notes:

- A game shows its cover art in the switcher and Spotlight (`switcher.md`,
  `spotlight.md`), so Steam's small `steam_icon_<appid>` icons matter only
  where a cover is missing.
- An app Papirus lacks shows its own hicolor icon. Mixed styles are the
  cost; colouring every unknown icon into a tile is what the brief refused.
- Papirus's folder colour can be set (`papirus-folders`), but Isle's Files
  draws its own folders and Isle's portal draws the Open and Save panels,
  so it is left at blue.

### Isle's five apps

Settings, Files, Monitor, Keychain and Cleaner get icons drawn for Isle,
one family: one shape (Isle's choice for its own apps, as Apple's apps
share one), Frost's greys with one system hue each, a Phosphor-weight
glyph (gear, folder, pulse, key, broom). Shipped as SVG in
`/usr/share/icons/hicolor/scalable/apps/` under the desktop entries' names,
with hand-tuned 16 and 32 where the scalable one blurs. Papirus does not
override them because it has no entries for those names; if it ever does,
Isle's names are distinct (`isle-settings`, and so on). Licensed with the
repo, so free to ship.

## Cursor

| | Bibata Modern Classic | Capitaine | apple_cursor | WhiteSur cursors | Phinger |
|---|---|---|---|---|---|
| Look | black arrow, white edge, rounded; Mac-like | macOS-inspired, from KDE Breeze | a copy of macOS's | from Capitaine | its own, not Mac |
| Licence | GPL-3.0 | LGPL-3.0 | GPL-3.0, but a reproduction of Apple's design | GPL-3.0 | CC-BY-SA-4.0 |
| Sizes | xcursor 16 to 96 in 14 steps (read from the installed theme) | 24 to 72 *(unsure)* | 16 to 96 *(unsure)* | *(unsure)* | 24 to 112 *(unsure)* |
| In Arch | AUR (`bibata-cursor-theme`, 2.0.7) | extra (`capitaine-cursors`) | AUR | AUR, git, 2021 | AUR |
| Upkeep | release 2024-06; repository pushed 2026-09 | last change 2022 | last change 2024 | 2025-04 | 2024-07 |

**Bibata Modern Classic**, as v1 shipped. It reads Mac-like, has every
size a HiDPI output needs, and covers the CSS cursor names the compositor
asks for (resize edges, grab, text, progress, not-allowed, crosshair).
apple_cursor is out as a copy of Apple's artwork; Capitaine is the
fallback if Bibata stalls, and is in extra. The cursor does not change
with the appearance, as on macOS.

Format: the compositor draws the cursor for clients that use
wp-cursor-shape (in `compositor.md`) and for its own surfaces, reading
xcursor files (Smithay's examples use the `xcursor` crate *(unsure for
Isle's choice)*). hyprcursor is Hyprland's format and is not needed.
Clients that draw their own (Xwayland, older toolkits) get the theme and
size through `XCURSOR_THEME`, `XCURSOR_SIZE` and the gsettings in
`theming.md`. At a fractional scale the compositor picks the next xcursor
size up and scales down.

## Sizes

All in logical pixels on the 4-pt grid; type sizes are Frost's (body 13).

**Glyph boxes.** A Phosphor glyph fills about three-quarters of its box,
so a 16 box beside 13-pt text draws a glyph near the text's cap height.

| Box | Weight | Where |
|---|---|---|
| 12 | Regular | inline with caption and small text (11, 12); badges; disclosure chevrons |
| 16 | Regular | inline with body text; the island's status glyphs and privacy marks (in its 30 pill); list and sidebar rows; Settings' pane tiles (glyph 16 in a 20 tile); toolbar buttons |
| 20 | Regular | Control Centre glyphs at Small and Medium (in a 32 circle); title-bar toolbar items in Isle's apps; text-field accessories |
| 24 | Regular; Fill in tiles | Control Centre at Large and Wide; OSD capsule; Settings pane header tiles (a 32 tile, glyph 24 in Fill) |
| 32 | Light | the lock's picture placeholder (`user`) |
| 48 | Light | system prompts' glyph where no app asks (the padlock); the session confirmation's action |

**App icons.**

| Size | Where |
|---|---|
| 16 | Files list and column views; Control Centre's tray row; the island's compact face |
| 20 | Control Centre's per-app sound rows; Settings › Notifications and Apps rows |
| 32 | Spotlight result rows; notification banners and Notification Centre; Settings' per-app pages' heads |
| 48 | The switcher's icon over its card's lower edge (v1: 48); system prompts naming an app |
| 64 | Spotlight's Apps grid tiles; Files' icon view default (16 to 256 on its slider) |
| 128 | Quick Look and Files' preview of an app |

Papirus has SVGs at 16, 22, 24, 32, 48, 64 and up, so Skia draws each
size from the nearest directory without blur. The switcher at 48 and the
Apps grid at 64 are the sizes to judge at true scale first: they decide
whether the shell reads as a Mac.

**Cursor.** 24, scaled with the output; Accessibility's pointer size
steps through Bibata's sizes to 96.

## Sources

Read 2026-10-03.

- Phosphor: `@phosphor-icons/core` 2.1.1 on npm (assets inspected),
  [github.com/phosphor-icons/core](https://github.com/phosphor-icons/core)
- Lucide: `lucide-static` 1.51.0 on npm;
  [InfoQ on Lucide 1.0](https://www.infoq.com/news/2026/06/lucide-v1-icons/)
- Tabler: `@tabler/icons` 3.48.0;
  [6,000 icons](https://tabler.io/blog/tabler-icons-6000)
- Material Symbols: `@material-symbols/svg-400` 0.47.6 (assets inspected),
  [google/material-design-icons](https://github.com/google/material-design-icons)
- Fluent: `@fluentui/svg-icons` 1.1.343 (assets inspected),
  [microsoft/fluentui-system-icons](https://github.com/microsoft/fluentui-system-icons)
- Remix: [License Change Notice #1069](https://github.com/Remix-Design/RemixIcon/issues/1069)
- Icon and cursor themes: GitHub repository metadata for each; Arch
  package search and the AUR RPC; [WhiteSur](https://github.com/vinceliuice/WhiteSur-icon-theme),
  [Colloid](https://github.com/vinceliuice/Colloid-icon-theme); the
  installed Papirus 20260801 and Bibata 2.0.7 on the host.
- Freya: `crates/freya-components/src/svg_viewer.rs` and `freya-icons` in
  the fork at `~/Development/freya`.
