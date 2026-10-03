# Theming installed apps

P1 stage 4, the system. What in `tokens.kdl` and `settings.kdl` reaches
which installed app, and by what route, for gate 4. P14 writes the
templates and the renderer (`platform.md` § Theming installed apps); this
note says what maps to what and which tokens the system must define for
it. Read and checked on 2026-10-03; claims not checked against a source
are marked *(unsure)*.

The model is Omarchy's (`design-references.md` §16): one palette, rendered
to a template per target, a hand-written file left alone. Two rules on top:

- **The portal is the floor.** Anything that reads
  `org.freedesktop.appearance` follows light and dark, contrast, reduced
  motion and the accent even when no template reaches it.
- **Targets without alpha get composited colours.** kitty, btop, zathura
  and `kdeglobals` take opaque hex; a token with alpha (`text-2`,
  `hairline`, `fill`) is composited over the surface it sits on before it
  is written. GTK CSS takes alpha as is.

## Findings that change the intent docs

Three things read for this note disagree with `platform.md` as written;
gate 4 decides them.

1. **The Settings portal should be Isle's.** `platform.md` keeps Settings
   on xdg-desktop-portal-gtk (Theming 5, Portal backend 1). Its Settings
   backend serves only `color-scheme` and `contrast` in
   `org.freedesktop.appearance` (its `settings.c`, read): no
   `accent-color`, no `reduced-motion`. The frontend merges Settings from
   every backend listed and the first listed wins per key (its
   `settings.c`, read), so `org.freedesktop.impl.portal.Settings=isle;gtk`
   lets Isle's backend serve the four appearance keys from `settings.kdl`
   and leaves gtk serving the `org.gnome.*` namespaces GTK 3 reads in a
   sandbox. B13 lists ScreenCast, Screenshot and FileChooser; this adds
   Settings.
2. **Qt's GTK 3 theme bypasses Isle's Open panel.** With
   `QT_QPA_PLATFORMTHEME=gtk3` (v1 D39), Qt opens GTK's own file chooser in
   process, not the portal's (Qt's `qgtk3theme.cpp`, read: it always
   returns a `QGtk3FileDialogHelper`). Qt apps would miss Files' Open and
   Save panels (v1 D67's "every app meets Isle at the file dialog"). See
   Qt below.
3. **Frost's colours fail as text in places.** White on the dark accent
   `#4f8dff` is 3.19:1; light `ok` `#1f9d5c` 3.22, `warn` `#b7860b` 3.02,
   `danger` `#d93c32` 4.19 against `window`; light `text-3` 2.33. Fine as
   fills and glyphs (3:1), not as text. GTK, Qt and btop use status colours
   as text, so the system needs text forms of them (tokens, below).

## By target

### GTK 3

adw-gtk3 (`adw-gtk-theme` 6.5, extra; LGPL-2.1), as v1. It takes
libadwaita's named colours through `@define-color` in
`~/.config/gtk-3.0/gtk.css`: `window_bg_color`, `window_fg_color`,
`view_bg_color`, `view_fg_color`, `headerbar_*`, `sidebar_*`, `card_*`,
`popover_*`, `dialog_*`, `accent_bg_color`, `accent_fg_color`,
`accent_color`, and the `destructive_`, `success_`, `warning_`, `error_`
triples. `settings.ini` and gsettings name the theme (`adw-gtk3` or
`adw-gtk3-dark`), `gtk-application-prefer-dark-theme`, fonts, icons and
cursor.

Limits: colours only; corner radii, metrics and shadows are adw-gtk3's.
GTK 3 does not reload `gtk.css` while running *(unsure)*, so an accent
change reaches open GTK 3 apps on their next start; the theme name switch
for light and dark is live through gsettings. GTK 3 matters beyond its own
apps: Firefox, Chromium and Electron read colours from it (below).

### GTK 4 and libadwaita

`~/.config/gtk-4.0/gtk.css` sets libadwaita's CSS variables (1.6 and
later): the surfaces (`--window-`, `--view-`, `--headerbar-`,
`--sidebar-`, `--secondary-sidebar-`, `--card-`, `--dialog-`,
`--popover-`, `--thumbnail-` bg and fg), `--accent-bg-color`,
`--accent-fg-color`, `--accent-color`, the status triples, `--border-color`,
`--dim-opacity`, `--disabled-opacity`, `--window-radius`, and the
document and monospace font families. gsettings (served to apps by the gtk
backend's `org.gnome.desktop.interface`) carries `color-scheme`, fonts,
icon theme, cursor and `text-scaling-factor`.

- **Accent.** libadwaita reads the portal's `accent-color` but snaps it to
  the nearest of its nine named accents (blue, teal, green, yellow,
  orange, red, pink, purple, slate), in Oklch by hue (libadwaita 1.6
  notes). Writing `--accent-bg-color` in `gtk.css` gives Isle's exact
  accent to every libadwaita app that does not set its own. GTK 4.24
  (2026-09-11) adds a `gtk-accent-color` setting from the portal, exposed
  to CSS as `accent_color` (marked unstable).
- **Contrast and motion.** GTK 4.22 reads its settings from the portal on
  Wayland and adds `prefers-reduced-motion`; 4.20 brought
  `prefers-color-scheme` and `prefers-contrast` media queries (GTK NEWS
  and docs; versions approximate from the NEWS dates *(unsure)*). So the
  portal keys reach GTK 4 apps live.
- **One file for both appearances** *(unsure)*: if user CSS honours
  `@media (prefers-color-scheme: dark)` on the installed GTK, one
  `gtk.css` holds both palettes and a light/dark switch is live; otherwise
  the file is re-rendered per appearance and takes effect on restart.
  P14 checks which.
- Limits: libadwaita's widget shapes, spacing, shadows and animations are
  its own; there is no blur; an app may hard-code its colours. Window
  controls follow gsettings `button-layout` (below).

### Qt 5 and 6

v1 set `QT_QPA_PLATFORMTHEME=gtk3` (D39): Qt builds its palette, fonts
and icon theme from the GTK 3 theme above, one source for both Qt versions.
It keeps that, with finding 2 to settle in P14. The options:

| Route | Palette | File dialog | Cost |
|---|---|---|---|
| `gtk3` (v1) | from adw-gtk3, so from tokens | GTK's own, in process | misses Isle's Open panel |
| `xdgdesktopportal` | from the base theme Qt picks for the desktop name; for an unknown name, Qt's generic one *(unsure)* | the portal: Isle's | no Isle palette unless the base is `gtk3` |
| `qt6ct` or Kvantum | a palette file or an SVG theme Isle would write | Qt's own, or the portal with `qt6ct`'s option *(unsure)* | another theme to write and keep; v1 removed qt6ct |

Whatever the route, Isle also writes `~/.config/kdeglobals`: KDE
Frameworks apps read their colour scheme from it (`[Colors:Window]`,
`[Colors:View]`, `[Colors:Button]`, `[Colors:Selection]`,
`[Colors:Tooltip]`, `[Colors:Header]`; Background, Foreground, Decoration
keys), the accent (`[General] AccentColor`), fonts and `[Icons] Theme`
*(which keys a KDE app honours outside Plasma, unsure)*. Qt reads the
portal's `color-scheme` from 6.5 on; `QPalette::Accent` falls back to
Highlight, so the selection colour carries the accent.

### Chromium, Electron, Firefox

None takes a palette; each takes light or dark and a little colour.

- **Chromium** reads the portal's `color-scheme` for web content since
  114; its "GTK" mode takes frame and control colours from GTK 3. v1's
  flags files (`--ozone-platform=wayland`, `--qt-version=6`) carry over.
- **Electron** derives `nativeTheme` from the GTK theme rather than the
  portal *(search summary, unsure by version)*, so the GTK 3 theme name
  and `gtk-application-prefer-dark-theme` must flip with the appearance.
  Accent: none.
- **Firefox** follows the portal's `color-scheme`, draws its widgets from
  GTK 3, and takes the CSS `AccentColor` and selection from GTK's selected
  colours, so adw-gtk3's accent reaches it. Its client-side title bar
  follows `button-layout`.

### The portal's appearance keys

Served by Isle's backend (finding 1), from `settings.kdl`, changing live
with a `SettingChanged` signal:

| Key | Value | From |
|---|---|---|
| `color-scheme` | 1 dark, 2 light | Appearance, with Auto resolved to the current one |
| `accent-color` | `(ddd)` sRGB 0–1 | the accent |
| `contrast` | 1 when on | Increase Contrast |
| `reduced-motion` | 1 when on (portal 1.21) | Reduce Motion |

Game Mode's stillness is the shell's own and is not published. The gtk
backend keeps serving `org.gnome.desktop.interface` (fonts, cursor,
`enable-animations`) and `org.gnome.desktop.a11y.interface`
`high-contrast`, which Isle sets alongside.

### Terminals and terminal programs

kitty, as v1: an `isle.conf` included from `kitty.conf`, reloaded by
signal. The 16 colours are their own tokens, not the accent (principle 6:
blue stays blue whatever the accent), contrast-checked against kitty's
background. Rules: ANSI 1–6 and 9–15 at least 4.5:1; bright black at least
3:1 (it is dim text); black in dark and the two whites in light are
background colours and are not checked. Increase Contrast raises the
targets to 7:1. A draft, checked by script against `window`:

| ANSI | Dark | ratio | Light | ratio |
|---|---|---|---|---|
| 0 black | `#2a2b30` | bg | `#1b1c1f` | 15.8 |
| 1 red | `#ff6961` | 5.8 | `#c4302b` | 5.1 |
| 2 green | `#4cc38a` | 7.4 | `#17804c` | 4.6 |
| 3 yellow | `#f2c14e` | 9.8 | `#8a6400` | 5.0 |
| 4 blue | `#5c9bff` | 6.0 | `#1f5fcc` | 5.5 |
| 5 magenta | `#d68cf5` | 7.0 | `#9a35b8` | 5.4 |
| 6 cyan | `#4fc7d9` | 8.2 | `#0a7486` | 5.0 |
| 7 white | `#c8cad0` | 10.1 | `#d9dadd` | bg |
| 8 bright black | `#7a7c85` | 4.0 | `#6b6d75` | 4.8 |
| 9 bright red | `#ff8a84` | 7.2 | `#a52520` | 6.7 |
| 10 bright green | `#6fd8a4` | 9.4 | `#106a3e` | 6.2 |
| 11 bright yellow | `#f7d27a` | 11.3 | `#715200` | 6.7 |
| 12 bright blue | `#86b4ff` | 7.8 | `#184ea8` | 7.2 |
| 13 bright magenta | `#e3aaf9` | 8.9 | `#7f2a99` | 7.2 |
| 14 bright cyan | `#7fd9e6` | 10.2 | `#075f6e` | 6.8 |
| 15 bright white | `#f2f3f5` | 14.8 | `#ffffff` | bg |

Dark's red, green and yellow are Frost's `danger`, `ok` and `warn`; light's
are darkened from them, which is the text form finding 3 asks for. In
light, the bright colours are darker than the normal ones, so "bright"
still reads as more emphatic on a light ground. Beyond the 16: `foreground` `text`, `background` `window`,
`selection_background` `selection`, `selection_foreground none`, `cursor`
`text`, `url_color` `link`, tab bar from `window-raised` and `text-2`,
`mark1_background` `match`. kitty stays opaque (principle 4).

Programs that draw with ANSI colours (Neovim's default scheme, fzf, bat,
delta, lazygit, the shell prompt) are themed by this palette with no
template of their own.

**btop** (`themes/isle.theme`): `main_bg` `window`, `main_fg` `text`,
`title` `text`, `hi_fg` `accent-text`, `selected_bg` `selection`,
`selected_fg` `text`, `inactive_fg` `text-3`, `graph_text` `text-2`,
`div_line` and the box outlines `hairline-strong`, meter and temperature
gradients `ok` → `warn` → `danger`, the CPU, memory and network graphs
from `series.*`.

**zathura** (`isle`): `default-bg` `window`, `default-fg` `text`,
statusbar and inputbar `window-raised` and `text`, `highlight-color`
`match` at alpha, `highlight-active-color` `accent`, `index-active-bg`
`selection`, notification error and warning `danger` and `warn`, and
`recolor-lightcolor` `window` with `recolor-darkcolor` `text` for its
recolour mode.

### Fonts

- fontconfig (`~/.config/fontconfig/conf.d/`, an Isle file): `sans-serif`
  and `system-ui` prefer Inter; `monospace` prefers JetBrains Mono; emoji
  falls to Noto Color Emoji. Inter Display is installed with Inter
  (`inter-font` 4.1); the shell picks it for display sizes itself.
- gsettings `font-name` `Inter 10`, `monospace-font-name` `JetBrains Mono
  10`, `document-font-name` `Inter 11`: GTK sizes are points at 96 dpi, so
  10 pt is 13.3 px, Frost's body. Text size multiplies these, and sets
  `text-scaling-factor`. Hinting slight and greyscale antialiasing *(the
  subpixel order of this panel is not checked)*.
- The same faces in `kdeglobals` and kitty (`font_family`, `font_size 10`).
- Free to ship: Inter OFL-1.1, JetBrains Mono OFL, Noto Color Emoji OFL.

### Icons and cursor

From `icons.md`: the icon theme is Papirus-Dark in dark and Papirus in
light, through gsettings `icon-theme`, `gtk-icon-theme-name`, the Qt route
and `kdeglobals [Icons]`. The cursor is Bibata Modern Classic 24 through
gsettings `cursor-theme` and `cursor-size`, `gtk-cursor-theme-*`,
`XCURSOR_THEME` and `XCURSOR_SIZE` in the session environment, and
`~/.icons/default/index.theme` for X11 clients; clients using
wp-cursor-shape get the compositor's.

### Window controls

Not a token, but every client-side title bar reads it: gsettings
`org.gnome.desktop.wm.preferences button-layout` set to
`close,minimize,maximize:` puts libadwaita's, Firefox's and Chromium's
controls on the left in macOS's order. The host's v1 value is
`icon:minimize,maximize,close`. Their look stays the toolkit's
(`window-chrome.md`, B25).

### What cannot be themed

- **Steam** and its overlay (its own skin), **games** and Proton or Wine
  windows: nothing beyond the cursor.
- **Electron apps' own colours** (VS Code, Discord, Obsidian, Heroic):
  light and dark at most; each has its own theme setting.
- **JetBrains IDEs** and other own-look toolkits (Java Swing, Tk, SDL):
  light and dark where they follow the system *(unsure for each)*; fonts
  and cursor at most.
- **Shapes and materials anywhere outside Isle**: no blur, no Frost
  shadows, no control radii in GTK or Qt apps; libadwaita's
  `--window-radius` is the one radius that passes.
- **Live changes**: GTK 3, Qt and Electron take a new accent or palette on
  restart *(GTK 3 and Qt unsure)*; light and dark through the portal and
  theme name are live.
- **Flatpak apps**, unless the GTK 3 theme is installed as a Flathub
  extension and `~/.config/gtk-4.0` is exposed by an override; the portal
  keys reach them regardless.
- **Web content**: sites follow `prefers-color-scheme` only.

## Token to target

`settings` rows are preferences in `settings.kdl`; *new* marks a token the
system must add (next section).

| Token | GTK 3 (adw-gtk3) | GTK 4 / libadwaita | Qt (`kdeglobals`) | kitty | btop | zathura |
|---|---|---|---|---|---|---|
| `window` | `window_bg_color`, `dialog_bg_color` | `--window-bg-color`, `--dialog-bg-color` | Window and Button BackgroundNormal | `background` | `main_bg` | `default-bg` |
| `view` *new* | `view_bg_color` | `--view-bg-color` | View BackgroundNormal | | | |
| `window-raised` | `popover_bg_color` | `--popover-bg-color` | Tooltip BackgroundNormal | tab bar | | statusbar, inputbar |
| `titlebar` *new* | `headerbar_bg_color`, `_backdrop_color` | `--headerbar-bg-color`, `--headerbar-backdrop-color` | Header BackgroundNormal | | | |
| `sidebar` *new* | `sidebar_bg_color` | `--sidebar-bg-color`, `--sidebar-backdrop-color` | | | | |
| `fill` | `card_bg_color` | `--card-bg-color` | Button BackgroundAlternate | | | |
| `text` | `*_fg_color` | `--*-fg-color` | ForegroundNormal | `foreground`, `cursor` | `main_fg`, `title` | `default-fg` |
| `text-2` | | `--dim-opacity` (as its alpha) | ForegroundInactive | tab bar | `graph_text` | |
| `text-3` | | `--disabled-opacity` | (disabled colours) | | `inactive_fg` | |
| `hairline`, `-strong` | `headerbar_border_color` | `--border-color`, `--headerbar-border-color` | DecorationHover *(unsure)* | | `div_line`, boxes | |
| `accent` | `accent_bg_color` | `--accent-bg-color` | `AccentColor`, Selection BackgroundNormal | | | `highlight-active-color` |
| `on-accent` | `accent_fg_color` | `--accent-fg-color` | Selection ForegroundNormal | | | |
| `accent-text` *new* | `accent_color` (links too) | `--accent-color` | ForegroundLink | | `hi_fg` | |
| `selection` *new* | (from accent) | (from accent) | Selection BackgroundNormal *(unsure)* | `selection_background` | `selected_bg` | `index-active-bg` |
| `focus-ring` *new* | `outline-color` rule | `outline-color` rule | DecorationFocus | | | |
| `link` *new* | `accent_color` | `--accent-color` | ForegroundLink | `url_color` | | |
| `ok`, `warn`, `danger` and their `-text` *new* | `success_`, `warning_`, `destructive_`, `error_` bg and text | `--success-*`, `--warning-*`, `--destructive-*`, `--error-*` | ForegroundPositive, Neutral, Negative | (ANSI 1–3) | gradients | notifications |
| `match` *new* | | | | `mark1_background` | | `highlight-color` |
| `term.*` *new* | | | | `color0`–`color15` | | |
| `series.*` *new* | | | | | graphs | |
| `radius.window` | | `--window-radius` | | | | |
| `type.family` | `gtk-font-name` | `font-name`, `--document-font-family` | `font` | | | font |
| `type.mono` | | `monospace-font-name`, `--monospace-font-family` | `fixed` | `font_family` | | |
| settings: appearance | theme name, prefer-dark | `color-scheme`, portal | portal, palette | palette | theme | palette |
| settings: accent | as `accent` | `gtk.css`; portal (snapped) | as `accent` | `selection` only | `hi_fg` | |
| settings: Increase Contrast | high-contrast tokens; `high-contrast` | portal `contrast`; tokens | tokens | 7:1 palette | tokens | tokens |
| settings: Reduce Motion | `enable-animations` | portal `reduced-motion` | | | | |
| settings: text size | font sizes | `text-scaling-factor` | font sizes | `font_size` | | |
| icons, cursor | `settings.ini` | gsettings | `[Icons]`, environment | | | |

Shell-only tokens with no target: `live`, `island`, `island-text`,
`scrim`, `material.*`, `shadow.*`, the other radii, `space`, `size`,
`motion`.

## Tokens the system needs for this

Added to `tokens.kdl` at gate 4, each in dark, light and the
increased-contrast set:

- **`view`**: the content background (lists, text fields, documents).
  Darker than `window` in dark, white in light, as macOS and libadwaita
  draw it; Frost's `window-sunken` is darker in both.
- **`titlebar`** and **`titlebar-inactive`**, **`sidebar`** and
  **`sidebar-inactive`**: what a window's top and side read as, active
  and in the background; Isle's own title bars use the same (B25).
- **`accent-text`**: the accent as text on `window` and `view`, derived by
  libadwaita's rule (Oklab lightness at most 0.5 in light, at least 0.85
  in dark, hue kept) and checked at 4.5:1. Links default to it.
- **`on-accent`** derived, not fixed: white or near-black, whichever
  passes on the user's accent (white fails on Frost's dark accent).
- **`selection`** and **`selection-inactive`**: the accent mixed into
  `view` (about 35% in dark, 25% in light), with `text` on it at 4.5:1.
- **`focus-ring`**: the accent, at least 3:1 against what it surrounds;
  width and offset with it (v1: 2 at 2).
- **`link`**: `accent-text` unless the user's accent fails as text.
- **`ok-text`**, **`warn-text`**, **`danger-text`**: the status colours
  as text at 4.5:1 on `window` and `view`; the current values stay as
  fills.
- **`match`**: a find highlight, yellow, as macOS's.
- **`term`**: the 16 ANSI colours above, per appearance and contrast.
- **`series`**: four categorical colours for graphs (btop, Monitor),
  distinct from the status colours.
- **`pane`**: the fixed hues of Settings' pane tiles (`icons.md`).
- **`icons`** and **`cursor`**: the symbol set and weights, the app icon
  theme per appearance, the cursor theme and size (`platform.md` lists
  `icons` among the token groups already).

## Sources

Read 2026-10-03.

- xdg-desktop-portal: `desktop-portal/settings.c` and
  `data/org.freedesktop.portal.Settings.xml` (main);
  [portals.conf](https://flatpak.github.io/xdg-desktop-portal/docs/portals.conf.html);
  [1.21 with reduced motion](https://www.phoronix.com/news/XDG-Desktop-Portal-1.21)
- xdg-desktop-portal-gtk 1.15.3: `src/settings.c` (main)
- [libadwaita CSS variables](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/css-variables.html);
  [libadwaita 1.6](https://blogs.gnome.org/alicem/2024/09/13/libadwaita-1-6/)
- GTK: [NEWS](https://gitlab.gnome.org/GNOME/gtk/-/raw/main/NEWS);
  [Gtk.Settings](https://docs.gtk.org/gtk4/class.Settings.html);
  [CSS properties](https://docs.gtk.org/gtk4/css-properties.html)
- [adw-gtk3](https://github.com/lassekongo83/adw-gtk3)
- Qt: `src/plugins/platformthemes/gtk3/qgtk3theme.cpp` and
  `xdgdesktopportal/qxdgdesktopportaltheme.cpp` (qtbase dev);
  [QPalette](https://doc.qt.io/qt-6/qpalette.html)
- Firefox: [bug 1749112](https://bugzilla.mozilla.org/show_bug.cgi?id=1749112),
  [bug 1703679](https://bugzilla.mozilla.org/show_bug.cgi?id=1703679)
- Electron: [nativeTheme](https://www.electronjs.org/docs/latest/api/native-theme),
  [electron#38961](https://github.com/electron/electron/issues/38961)
- Arch package database for `adw-gtk-theme`, `inter-font`,
  `ttf-jetbrains-mono`, `kvantum`, `qt6ct`; the host's installed fonts and
  gsettings.
- In the repo: `design-references.md` §16, `isle-v1-inventory.md` §9 and
  §14, `platform.md` § Design tokens, § Theming installed apps, § Portal
  backend.
