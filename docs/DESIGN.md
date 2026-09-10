# Design

The look and feel of the shell. One material, one accent, one motion curve,
and an island that is the only thing on screen at rest.

## Principles

1. **Nothing at rest.** The desktop is wallpaper and a pill. Every other
   surface appears on intent (a key, a click, a hover) and leaves on its own.
2. **One material.** Every shell surface is the same dark glass. There are
   no bars, no panels with different backgrounds, no second style.
3. **Grow, don't pop.** Surfaces that belong to the island grow out of it
   and shrink back. Nothing appears at a screen edge unannounced.
4. **The accent is a signal, not a colour scheme.** Text is white on ink.
   The accent marks the one thing that is active, focused, or live.
5. **Same skin everywhere.** Installed apps get the same palette, type,
   icons and cursor as the shell, generated from the same token file.

## Tokens

Every value below is a token in `shell/theme/tokens.json`. The shell's
`Theme` singleton reads it; `theme/` templates render the same file into
GTK, Qt, kitty, yazi and btop themes so nothing is typed twice.

### Colour (dark, the default)

| token | value | use |
|---|---|---|
| `ink` | `#0A0B0D` | wallpaper fallback, lock backdrop |
| `glass` | `#121316` at 78% + blur | every shell surface |
| `raised` | `#1B1C21` | rows and cards on top of glass |
| `pressed` | `#26272D` | pressed / selected row |
| `hairline` | white 8% | every border |
| `hairline-strong` | white 14% | focused border, dividers under headers |
| `text` | `#F2F2F3` | primary |
| `text-2` | `#F2F2F3` at 62% | secondary |
| `text-3` | `#F2F2F3` at 38% | tertiary, disabled, placeholders |
| `accent` | `#7FA6FF` | the active thing; user-settable, or from wallpaper |
| `on-accent` | `#0A0B0D` | text on accent fills |
| `ok` | `#6FCF97` | connected, charged, success |
| `warn` | `#F2C94C` | attention |
| `danger` | `#FF6B6B` | destructive, error |
| `live` | `#FF453A` | recording, on-air |

Light is the same set inverted: `paper #F4F4F5`, glass `#FFFFFF` at 72%,
text `#16171A`, hairline black 10%. Same accent. Dark ships first; light is
a token swap, not a second design.

### Geometry

- 4pt grid. Spacing tokens 4 / 8 / 12 / 16 / 24 / 32.
- Radii: `pill` full, `panel` 20, `card` 14, `control` 10, `chip` 8.
- Hairline 1px, always. No 2px borders.
- Shadow, one: `0 12px 40px rgba(0,0,0,.45)`. Used by floating surfaces only.
- Blur: Hyprland layer blur, size 8, 3 passes, `ignorezero`. The glass alpha
  is tuned for that blur; without blur (game mode) glass goes to 94%.
- Island: 30px at rest; the control panel it unfolds into is 420px wide
  with a 44px header band. Launcher 640px. Dashboard widgets sit on a
  12-column grid with a 12px gutter and 24px page margin.

### Type

- UI: **Inter**, sizes 11 / 12 / 13 / 15 / 20 / 28, weights 400 / 500 / 600.
  Body is 13/500. Clocks and any changing number use tabular figures.
- Numbers and code: **JetBrains Mono** 12 / 13.
- No uppercase tracking labels. Sentence case everywhere.

### Motion

- Curve: OutQuint for enter, InQuint for exit. Exit is always faster.
- Durations: `quick` 140ms (hover, toggles), `move` 220ms (the control panel, rows),
  `morph` 360ms (island size changes).
- Island size changes use a spring (stiffness 180, damping 22), so a morph
  interrupted by another morph continues from where it is.
- Enter is fade + scale 0.96 → 1 from the surface's anchor. Exit is fade
  only.
- Reduced motion (preference) and game mode set every duration to 0.

### Icons

- Shell glyphs: **Lucide**, vendored SVG, 1.75px stroke, drawn at 16 and 20,
  recoloured through the text tokens.
- App icons: **Papirus-Dark**, 20 in the island, 32 in lists, 48 in the
  switcher and launcher, 96 in Big Picture. The same theme is set for GTK
  and Qt so an app looks the same in its own window as in the launcher.
- Cursor: Bibata Modern Classic, 24.

## Components

The kit in `shell/ui/`. Every surface is built from these and nothing else.

| component | notes |
|---|---|
| `Glass` | the material: colour, hairline, radius, optional shadow |
| `Glyph` | a Lucide icon at 16 or 20 in a text token colour |
| `Label` | Inter at a named size; `mono` and `tabular` switches |
| `Pill` | a rounded chip: glyph + label; the island's building block |
| `Toggle` | 36×20 switch, accent when on |
| `Slider` | 4px track, 16px knob, hairline track, accent fill |
| `Segmented` | 2–4 options, raised background, pressed indicator slides |
| `Field` | text input: raised, hairline, accent hairline when focused |
| `Row` | list row: 40px, leading glyph/icon, title, subtitle, trailing control |
| `Card` | raised container with 12 padding, card radius |
| `Panel` | glass container that hangs off the island |
| `Button` | text / raised / accent / danger variants |
| `FocusRing` | 2px accent ring at 2px offset; keyboard and controller only |
| `Dialog` | centred glass card for auth and confirmations |

Every interactive component has hover (raised), pressed (pressed), focus
(ring) and disabled (text-3) states. Controller and keyboard navigation use
the ring; the mouse never shows one.

## The island

Top centre of the focused monitor, 12px from the edge. The only persistent
surface. Other monitors show nothing.

**Rest.** 30px pill: desktop dots · clock · status glyphs. Glyphs only
appear when they say something: wifi when on wifi (never on ethernet),
bluetooth only when a device is connected, volume only when muted,
recording dot, mode chip.

**Hover.** The pill unfolds downward into the **control panel**: the
pill's own content becomes a 44px header band (desktops as app icons,
clock and date, glyphs) and beneath it, 420px wide, the toggle grid (wifi,
bluetooth, do not disturb, night light, focus, game), the output picker
with volume and brightness, media, and a footer line for controller
battery and power profile. Nothing needs a second click to reach. The
panel folds back when the pointer leaves.

**Events** morph the island for a few seconds, then it returns:

| event | shows |
|---|---|
| notification | app icon, summary, one line of body; click opens, swipe dismisses |
| volume / brightness | glyph + thin bar (this is the OSD; there is no other) |
| media starts | art + title slides in |
| capture / recording | live dot + timer + stop |
| drive inserted | drive name + mount / eject |
| auth pending | key glyph, click opens the dialog |
| mode change | "Game" / "Big Picture" chip |
| clipboard copy | check glyph, once |

**Click.** Desktop dots → Mission Control. Clock or bell → the dashboard.

**Modes.** Game: the island is gone; a 4px hot
zone at the top edge peeks it after 400ms hover. Big Picture: gone.

### Shortcuts in the launcher

A shell result that has a chord shows it at the right of its row, in the
keyboard in use's notation (Win or Cmd), mono, in the tertiary colour, before
the Enter hint. Every keymap action is searchable as a "Shortcut" result;
picking it runs the bind's own route. Held chords (the app switcher) and the
numbered ranges are not listed.

## Surfaces

Each is a layer surface in the same Quickshell process. See
`ARCHITECTURE.md` for what drives them; this is how they look.

One full-screen surface at a time: opening the dashboard, Mission Control,
the launcher, the capture bar or the power menu closes whichever of the
others is up, and a surface's own key closes it again. The switcher stays
shut over Mission Control and closes the dashboard when it picks a window.
Escape closes whatever is up.

- **Launcher** — 640px glass card, centred at 30% height. One field, results
  as rows with 48px icons. Prefixes: `>` run, `=` calculate, `/` files,
  `:` clipboard, `@` windows. Power actions match on their names.
- **Switcher** — a row of live previews across the centre, held open
  while the modifier is down: one 240×150 card per app showing its front
  window at the window's own shape (the icon until a frame arrives), the
  app's icon and name beneath, a count on the corner when it has more
  windows. The selected card is raised with an accent ring; its window
  title and the held keys read under the row. The row wraps past three
  quarters of the screen. Windows hidden with Super+M appear dimmed.
- **Mission Control** — the desktop pulled apart: the current space's
  windows spread out as live thumbnails, the spaces in a bar across the
  top. See "Mission Control" below.
- **Dashboard** — full screen, on its keybinding or a click on the
  island's clock or bell: wallpaper dimmed and blurred, a 12-column grid
  of glass widgets. Escape or a click on the backdrop closes it. See
  "Widgets" below.
- **Capture** — a thin toolbar at the bottom centre: region / window /
  screen, screenshot / record, annotate, colour picker. Region drag draws a
  hairline with dimensions.
- **Lock** — wallpaper blurred and dimmed; the island grows into a 28px
  clock; typing reveals a password field under it. Wrong password shakes it.
- **Auth** — a centred dialog: app icon, what is being asked, password field.
- **Power menu** — five pills in a row: lock, sleep, restart, shut down, log
  out. Keyboard first letter selects.
- **Big Picture** — 10-foot: 96px tiles in rows (Steam, Heroic, Lutris,
  recent games), 24px type, 3px focus ring, controller-driven.

## Mission Control

The word is desktop, everywhere a person reads it: the bar, the keys,
the widget. Workspace is the compositor's word and stays in code.

macOS's, as near as the compositor allows. One surface per monitor, over
the wallpaper dimmed and blurred as the dashboard dims it; the island is
under it and out of the way, as the menu bar is.

**In and out.** Ctrl+Up, a three-finger swipe up, a click on the island's
desktop dots. Escape, a swipe down, a click on the backdrop, or choosing
something closes it.

**The spaces bar.** Across the top, under the island's strip: one tile per
desktop on that monitor, a live picture of it, the wallpaper with its
windows at their true places, 160px wide at rest and 200px under the
pointer, "Desktop 1" (or its name) beneath, the current one
ringed in the accent. A `+` tile at the right end makes a new space; a
window dropped on it goes there. Click a tile to go to that space. Ctrl+Left
and Right move between spaces without leaving, and the stage follows.

**The stage.** The current space's windows spread so none overlaps,
keeping their relative places and proportions: sorted by row, packed into
`ceil(sqrt(n))` rows, each row scaled to the stage's width, a window never
drawn larger than it is. Every thumbnail is the window live, its corners
rounded, a hairline around it. Under the pointer it gets the accent ring
and its title in a pill beneath, with the app's icon. A `×` at its
top-left corner closes it, so does W or Delete on the highlighted one and
a middle click; that is Windows's Task View rather than macOS, which has
no way to close from here, and it is why the surface exists as much as
switching is. Click a window to go to it. Arrows move the highlight to the
nearest window in that direction, Tab cycles, Enter goes.

**Drag.** A thumbnail dragged follows the pointer at its own scale. Over a
tile in the spaces bar the tile lights; dropped there the window moves to
that space, dropped on `+` it gets a new one. Windows are never grouped by
app; the spread is spatial, which is macOS's default.

**Hidden windows.** macOS keeps minimised windows off Mission Control and
in the Dock. Isle has no Dock, so they sit in a tray along the bottom
edge: dimmed app icons with a count, click to bring one back.

**Motion.** Opening: the backdrop fades in over `quick`, every window
travels from its real rectangle to its stage rectangle over `morph`
(OutQuint), the bar slides down from the top edge. Closing reverses it;
choosing a window sends it back to its rectangle first and the surface
goes as it lands.

**In the switcher.** Held on an app, Q closes every window of it and W
its front window, so a stray app can be put down without switching to it.

## Widgets

The dashboard is a grid of widgets. A widget is a glass card with an
11px title, an optional right-hand meta, and content; it occupies whole
grid cells (3×1, 3×2, 3×4, 6×1, 6×2) and never scrolls the page.

Built in, in the default layout:

| widget | shows |
|---|---|
| Clock | time, date, next calendar item |
| Media | art, track, transport |
| Desktops | each desktop with its app icons; click to go |
| Notifications | grouped by app, inline actions, clear all |
| Controls | the toggle grid and the two sliders |
| System | CPU, GPU, memory, network sparklines; uptime, kernel |
| Agents | running coding-agent terminals and their state |
| Session | tray icons; lock, sleep, restart, power; power profile |
| Devices | bluetooth and controller batteries, headset |
| Storage | disks with usage bars; mount and eject |

Available but off by default: Network (interface, address, throughput),
GPU (LACT clocks, power limit, fans from CoolerControl), Games (recent,
quick launch), Clipboard (recent entries), Timer (focus / pomodoro),
Updates (pending packages), Weather.

Edit mode (E, or the pencil in the corner) is a grid editor. Dragging a
card lifts it to float under the cursor while its neighbours flow live to
make room, re-planned each move from the pre-drag arrangement so leaving
un-pushes; the cards it lands on cascade the way it came, each shoving
the next along the row (or column), so a card dragged across pushes the
others into the space it vacates rather than swapping one to the origin. The push follows the card's centre and the neighbours glide, so it
reads as a soft rearrange rather than a snap; the card settles into its
cell on release. The right edge, bottom edge and corner
resize the card, held between the manifest's smallest and largest spans.
The focused card (click, or Tab through them) takes an accent ring;
arrows nudge it a cell, Shift+arrows resize it, Delete removes it.

The grid gives up a 340px column on the right for the library and a
strip at the bottom for the toolbar (Reset layout, Discard, Done), so no
control sits over a card. The library lists every widget by category
with a search field: glyph, name, description, and "on" with a tick when
placed (×2 for a second instance). Clicking a row places it at the first
free spot; dragging one drops it on the cell under the pointer; clicking
an empty cell first marks it, and the next pick lands there. New text
widget at the foot opens the form; the user's own carry edit and delete
on hover. When nothing fits, the toolbar says which size it needs.

A header strip runs across the top: the page tabs centred, the Edit
toggle at the right, always reserved so no control sits over a widget.
Pages are dashboards in their own right, switched by clicking a tab or a
number key; editing adds one with the plus, renames by double-click and
removes with the ×. Each page's layout, and which page is shown, are
preferences.

Clicking the gaps between widgets does nothing; only the margin around
the grid, Escape or the keybind closes the dashboard. Leaving edit mode
or closing with an unsaved change asks first: Keep, Discard, or Cancel.

A user's own QML widget declares in its `widget.json` the services it
uses, as `permissions` like `["audio", "media.write"]`: a name grants
read, a `.write` scope grants that service's actions. It receives a
`host` wrapper exposing only those, read-only, plus `host.theme`; the
library row names what a widget uses, or "Trusted · full access" for one
that carries `trust: true`. A widget refused for reaching past the
sandbox (a forbidden import or escape) says so on its card, naming what
tripped it.

Text widgets, the user's own, draw through one renderer: a title, then
the output as wrapped text, a large number with its unit, a gauge bar
against a maximum, a sparkline of the readings so far, or a list of
lines with "label: value" split into two columns. A failing command
shows its last line of error in the danger colour.

A glyph is an Item of its box, so layouts size it by the box and never
by the raster (a 2× raster once doubled every icon inside a layout).
Boxes: 10 by captions, 14 by body text and in two-line tiles and rows
(about 11px of visible glyph beside 13px text), 16 in a 32px icon box,
17 to 20 for a tile's own picture, 13 to 14 for the pill's event
glyphs; `icon.scale` (1.15) lifts every one by the same step. A field's
glyph is the size of its text; the launcher's field has none, the
placeholder says enough. The stroke is 1.5 on Lucide's 24 grid
(`icon.weight`), rasterised at 2× and scaled.

## Windows

Three real windows, same material without blur (they sit over other
windows, not the wallpaper): **Settings**, **Monitor**, **Keychain**. A
sidebar of sections on the left, content on the right, 15px section titles.
Settings keeps a trail of the pages it has shown: back and forward
buttons beside its title, the mouse's back and forward buttons, and Alt
with the arrows walk it, the way a browser or Windows Settings does.
Every app window carries a 28px title bar from hyprbars: the title at the left in the text's secondary colour, three dots at the right in warn, ok and danger (minimise, fullscreen, close) that show their glyph on hover, over the window colour so the bar reads as part of the window. It can be turned off in Appearance.

New windows float by default and snap to the screen's edges and to each
other while dragged, which is what a Windows or macOS hand expects; the
dwindle layout is one toggle away for those who want columns. Tiling is
macOS's: dragged to a side a window takes that half, to a corner that
quarter, to the top the whole work area, with a translucent preview of
the target while the button is down and an 8px margin between tiles;
dragged off again it gets its old size back. Super+Ctrl+arrows do the
same by key, and a double click on the title bar fills or restores.
Focus never moves the pointer.

Settings orders its sidebar as KDE, macOS and Windows do: Network and
Bluetooth first, then Look and feel (Appearance, Notifications, Modes),
Hardware (Displays, Audio, Keyboard, Mouse, Printers, Storage, Power),
System (Apps, Users, Date and time, Language and region, Updates) and
About. With more than one display, the Displays page opens with an
arrangement map: drag a display and its edges snap to the others; the
layout is written as absolute positions so nothing else moves. A primary
display, when chosen, is where every shell surface appears; the default
follows focus, since the island is meant to be where you are looking.
Appearance has Dark, Light and Auto; Auto is light from sunrise to
sunset, computed from the timezone's coordinates in tzdata, since
nothing in the stack knows where the machine is more precisely.

The Monitor's process table is a tree: a process carries its
descendants' totals and opens on its chevron, sorted at every level, so
a browser's thirty helpers are one row until asked. Session managers and
the compositor are not groups. The list keeps its place across samples
and has a thin scrollbar.

The Monitor is Activity Monitor's shape: CPU, Memory, Energy, Disk and
Network tabs each show every process in that tab's columns with a strip
of the system's totals beneath; Sensors holds temperatures, fans and the
GPU. The dashboard's System widget already has the live graphs, so the
window does not repeat them.
The Keychain lists the system keyring under category chips (logins,
Wi-Fi, browser, apps) and the SSH keys under their own: the keys in
`~/.ssh` with fingerprint and agent state, and a form that generates a
login secret or a key. A password manager with its own vault (Proton
Pass, Bitwarden) is its own app, not folded into the shell.

Settings › Devices is Device Manager's shape: a tree of categories on
the left, each device under its own, a count on the category and a
chevron to fold it, and on the right the selected device's properties as
label and value pairs with a button to the page that sets it up. System
devices start folded. The tree follows what is plugged in and out.

## Installed apps

The token file renders to:

- GTK 3 and GTK 4 / libadwaita CSS (Nautilus, LACT, Firefox chrome)
- Qt 6 palette through qt6ct, Fusion style
- kitty colours and font
- yazi theme, btop theme
- Hyprland border colours and rounding, matching `hairline` and `control`

Icons Papirus-Dark, cursor Bibata, font Inter, for both toolkits. Portals
use the GTK file dialog so every app's open/save looks the same.

Chromium-based browsers and Electron apps run native Wayland: the
render adds `--ozone-platform-hint=auto` to the flags file of each
installed browser that has nothing about ozone set, and the compositor
exports the Electron hint. Their GPU process logs that Vulkan is not
available on Wayland and carries on with GL; that line is noise, not a
refusal. On Wayland Chromium points its Qt toolkit integration at the
Wayland platform, so the flags file also asks for Qt 6, the one the
desktop themes; qt5-wayland is in the package list for every other Qt 5
app, since the compositor tells all of them to run on Wayland.

### Keyboards

The Keyboards group is Plasma's Hardware tab: one row per physical keyboard,
named from the vendor and the compositor's slug, with the bus and what the
key bitmap says about its size as the description, then the XKB model
dropdown (Generic models first, marked "guessed" until chosen), and the
Windows or Mac profile. NumLock at start and a test area follow.

### Keyboard layouts

As Plasma lays them out: a table of the layouts in use (Layout, Variant,
Code) with the selected row raised, then Add…, Remove, Move up and Move
down acting on it. Add… opens a picker under the table: a search field, the
layouts on the left, the chosen layout's variants on the right (Default
first), and Add. Below the quick rows for Caps Lock, Alt/Win and Compose, an
Advanced group lists every XKB option group folded, with a toggle per option;
a group that is a position or a key takes one choice.

### VPN

The panel's VPN tile is disabled until something can come up: a Proton
account signed in, or a NetworkManager connection. Its page lists the fastest
Proton server, then countries, then NetworkManager connections. Settings ›
Network › VPN holds sign-in (username and password fields, then a code field
when the account has two-factor), the country choice, kill switch and NetShield.
