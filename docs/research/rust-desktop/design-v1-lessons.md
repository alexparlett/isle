# v1's design lessons

State as of 2026-10-03. Stage 1c of P1 (`docs/work/rust-desktop/p1-design.md`):
what Isle v1's design got right and wrong in use, as evidence for the new
design. The new design is fresh (B3) and macOS-style (B17); v1's look is not
carried over, its lessons are.

**Sources**, all read from git without checking out `main`:

- `main`: `docs/DESIGN.md`, `docs/DECISIONS.md` (D1–D92), `docs/AUDIT.md`,
  `docs/WIDGETS.md`, `docs/canvas/` (fourteen artboards), `docs/screenshots/`
  (seventeen VM captures), `shell/theme/tokens.json`, `hypr/hyprland.lua`, and
  the island's timings in `shell/surfaces/island/Island.qml` and
  `shell/services/*.qml`.
- Commit history: `main` has 234 commits from 2026-09-10 (`300ede6` squashed
  what came before). The 299 commits before the squash are on the local branch
  `history-before-squash`, including a first build (2026-09-05 to 09-07) with
  its own DESIGN.md and decisions, which a commit titled "Reset" (`a7af25e`,
  2026-09-09) deleted before v1 as it stands was built. Decisions from that
  first build are cited as "pre-reset D25" and read from `a7af25e^`.
- `isle-v1-inventory.md` §4, §9, §10, §14, §16 for the shape of v1.

**Method.** Commit subjects say what changed and are the best record of what
the user asked for, but they were written by agents, so a subject is evidence
of a change, not a quote. Anything about the user's taste is marked
*inference*. Screenshots are VM captures (software GL, scaled to 1280 wide),
not the host.

---

## 1. The short version

1. **Quiet at rest survived everything.** "Nothing on screen but the
   wallpaper and a pill", with status glyphs that show only when their
   absence would say something, held from the first spec to the last commit.
2. **The desktop must be one thing.** Every third-party GUI in the way was
   replaced by Isle's own (file manager, file chooser, on-screen keyboard,
   AppImage installer), each time for looking like "a second desktop inside
   this one" (v1 D82).
3. **Contrast is guaranteed for content against its own surface, never for a
   translucent surface against an arbitrary wallpaper.** Opacity walks and
   rescue hairlines were tried and reverted (pre-reset D8, `8ded377`).
4. **One material was too few in the end.** Glass inside a window cannot be
   seen; windows went opaque and their menus used a raised tone
   (`535ca6c`). A flat depth ladder of near-identical greys was
   imperceptible (pre-reset D25, `7a7d251`).
5. **Motion artefacts were noticed by eye, not by gates.** The pill lurching
   sideways, content sliced during a morph and windows sliding into their
   remembered place were all found by watching, then fixed (`d1af0a8`, v1 D91).
6. **Hover as a trigger cost a long tail of fixes**: dwell, grace, settle,
   disarm after a click, hold events while open, select on movement only.
7. **Where notifications live changed three times** (own panel, dashboard
   widget, island page). Settle it once.
8. **Density moved toward smaller units and no empty air**: the dashboard
   grid doubled its rows (v1 D35), slim one-row widgets became defaults,
   lists scroll at eight rows rather than grow.
9. **Look at true scale on the real renderer.** Icons were shrunk three times
   for a rendering bug that drew them at double size (`73c12ca`).
10. **Decide by human look at a gate, not by a numeric critic.** The first
    build's scored loop was reset; its own record says a human glance found
    what 27 checks and five critic rounds did not.

---

## 2. What held up

Principles, values and behaviours that survived the reset and every later
iteration. "Since" is the earliest commit or decision that states it.

| What | Since | Evidence it held |
|---|---|---|
| Nothing at rest but wallpaper and a pill; everything else on intent and leaves on its own | pre-reset principle 1 (`5b9dc3c`), v1 D2 | Same in v1 DESIGN principle 1 at the end; game-mode capture shows no shell at all |
| A glyph earns its place only if its absence says something (Wi-Fi only on Wi-Fi, Bluetooth only when connected, volume only when muted, restart owed in warn) | pre-reset D38 (`52627d8`) | Rest rules unchanged to the end; the island header lost its status glyphs (`f2bb6fd`); Odyssey's always-lit island refused as "way too cluttered" (pre-reset D36) |
| The accent is a signal, not a scheme: active, focused or live | pre-reset principle 2 | v1 principle 4; the one exception is data encoding (Cleaner kinds, v1 D88) |
| Fixed palette with its own names; no wallpaper-derived colour | `1a2bd4f` (matugen "unreadable on low-contrast wallpapers") | v1 D16; Odyssey's wallpaper palettes refused (pre-reset D36) |
| Tokens are the one source; installed apps wear the same skin | `1a2bd4f` ("tokens or nothing") | v1 principle 5; GTK, Qt via GTK (v1 D39), kitty, btop, zathura, title bars all rendered from `tokens.json` |
| Sentence case, no uppercase tracking labels, "desktop" never "workspace" | pre-reset § Type | v1 § Type and § Mission Control |
| Tabular figures for anything that changes; a mono face for numbers and code | pre-reset D4 | v1 § Type |
| Dark first, light as a full variant (not an inversion; accent darker in light) | `890ccf5` | v1 tokens `light` block: accent `#3F6FD6` against `#7FA6FF` |
| Reduced motion and game mode zero every duration | pre-reset D2 | v1 § Motion |
| Exits faster than entrances; enter is fade plus small scale from the anchor, exit a fade | pre-reset § Motion | v1 § Motion |
| One full-screen surface at a time; Escape closes whatever is up | pre-reset `1a6b813` | v1 `4d2052f` |
| Focus ring for keyboard and controller only; the mouse never shows one | v1 D9 | v1 § Components |
| A window is a task you sit inside, so it is near-opaque; surfaces are transient, so they transmit the desktop | pre-reset `2786b60` | v1 § Windows: "same material without blur" |
| The island is the OSD; there is no other | pre-reset D34 | v1 § The island, events table |
| Launching what is open raises it | v1 D63 | Unchanged |
| Close from the overview (Task View, not macOS) | v1 `0f9cc30` | v1 § Mission Control says it is "why the surface exists as much as switching is" |
| Things appear where they belong, not corrected into place | v1 D91 | Last design-relevant fix on `main` (`6f5c6ab`) |

---

## 3. What changed repeatedly, and which way

### 3.1 Values: first build, then v1 final

The first build measured its values against four synthetic wallpapers; v1
re-chose them after the reset without that harness. §14 of the inventory has
v1's full set; this table is the trajectory.

| Value | First build (pre-reset) | v1 final | Direction |
|---|---|---|---|
| Materials | six by role: island 0.92, panel 0.85, window 0.97, card, well, scrim (pre-reset § Material) | one glass, 0.78 dark and 0.72 light; windows opaque | collapsed to one, then windows split off again in practice |
| Opacity without blur (game mode) | n/a | 0.94 dark, 0.96 light | solid fallback per material |
| Blur | 24px, region computed (pre-reset D11) | Hyprland layer blur size 8, 3 passes, ignore alpha 0.2; canvas drew CSS blur 24 | runtime and mock-up never agreed (*inference*: nobody compared them) |
| Durations | 120 / 200 / 320 ms, ceiling 320 for a panel only (pre-reset D1) | 140 / 220 / 360 ms | slightly longer |
| Curves | OutCubic in, InCubic out | OutQuint in, InQuint out | sharper ease |
| Spring | QML spring 14, damping 0.95, on the island hover only (pre-reset D47, `18b1b86`) | 180/22 specified for the island, used only by Hyprland windows; the island morphs on OutQuint 360 ms | spring dropped from the shell |
| Radii | 24 / 16 / 8 with concentric nesting as a rule (pre-reset D37) | panel 20, card 14, control 10, chip 8; concentric rule not restated | rule lost in the reset |
| Corners | superellipse shader, capsule at full radius (pre-reset D3) | Qt's circular `Rectangle` | continuous curvature dropped |
| Body type | Inter 14/400; display 34 | Inter 13/500; display 28 | smaller, heavier |
| Accent | `#4A9EFF` | `#7FA6FF`, user-settable from six swatches | softer, and the user's |
| Shell icons | Phosphor, Regular/Fill as state pair (pre-reset D14) | Lucide, stroke 1.5 | changed set |
| Cursor | Capitaine 24 | Bibata Modern Classic 24 | changed |
| Shadow | MultiEffect blur 0.62, offset 2, opacity 0.55, pure black (pre-reset D9) | one shadow `0 12px 40px rgba(0,0,0,.45)` | larger, softer |
| Island | 400 max at rest | 30 high, 12 from the top; panel 420 with a 44 header | similar |
| Auto theme | from the wallpaper's luminance (`890ccf5`) | two clock times (`ddfeca6`), then sunrise and sunset from tzdata (`bb3396e`) | from content to time |

### 3.2 Icon size: a bug read as taste

| Commit | Change | Why |
|---|---|---|
| `8ecc826` | every glyph × 0.8 | icons looked too big |
| `4a20c68` | glyphs sized to the text beside them, 10 to 14 | still too big |
| `66dbd86` | glyphs at x-height, 8 to 13, "chosen from three candidates side by side" | still too big |
| `73c12ca` | `Glyph` drew its 2× raster at double its box; fixed, then 10 to 16 chosen "at true scale" | the cause |
| `0e062e2` | `icon.scale` 1.15, event and lock glyphs a size up | once honest, they wanted to be a step larger |

Net: the user's preference was slightly larger than nominal, not smaller.
Three rounds of taste decisions were spent on a rendering defect.

### 3.3 Density

| Commit | Change |
|---|---|
| `c5ede35` | dashboard 12 × 4 grid of glass cards |
| v1 D35, `823e292` | 12 × 8: "the smallest card was a quarter of the screen tall"; widgets "that were mostly air" get one-row defaults |
| `e3cc2e1` | Session and Devices default to two columns, not three |
| `87783c0` | Clock two wide and one high by default; titles optional; the size badge goes |
| `a47554f` | launcher results scroll past eight rows instead of growing |
| `39c886d` | Settings: 47 row descriptions cut to those that say something; subtitles under 70 characters |
| `972bbe7` | durations as dropdowns, numbers as typed inputs, sliders only for continuous values |

The dashboard screenshots (`dashboard.jpg`, `light-dashboard.jpg`) show why:
large cards holding "Nothing playing", "No terminals open", "Nothing
connected". Every change in the table went toward smaller units and less air.

### 3.4 Where notifications live

| When | Where | Why it moved |
|---|---|---|
| pre-reset D26 | its own panel under the toasts | a full-width overlay to read three notifications is disproportionate |
| v1 D4 | a widget in the full-screen dashboard | the dashboard replaced control and notification centres |
| v1 D26, `cf60d9f`, `49a415d` | a page of the island's panel, behind the Notifications tile's arrow; the dashboard widget repeats it | "one hover away without leaving the window in use" |

The count was also wrong for a while: it counted toasts on screen, so after a
restart it said "All clear" over a full list (`5fc1186`).

### 3.5 Hover guards

The island opened on hover. Each guard below is a fix for a reported
misbehaviour.

| Guard | Value | Commit |
|---|---|---|
| Dwell before opening | 120 ms | `6da42ba` era, `Island.qml` |
| Grace before closing | 250 ms | same |
| Hold the hover while the panel's own size settles after a click | 500 ms | `d53aa83` |
| An event waits while the panel is open; an OSD raised from the panel's own slider is dropped | | `d53aa83` |
| A click that opens something folds the panel and disarms hover until the pointer leaves | | `4e53f92` |
| Launcher: hover selects only when the pointer moves | | `a47554f` |
| Cleaner: hover cleared when the pointer leaves the window | | `d39c998` |
| Game mode: 4px hot zone, 400 ms to peek, peek lasts 4 s | | v1 D49, `HotZone.qml` |

---

## 4. Reversed or removed

| What | First | Then | Why | Evidence |
|---|---|---|---|---|
| Island on the focused monitor only | v1 D2 | an island on every screen; events on one | a second monitor had no clock, no dots, no way to the panel | v1 D49 |
| No graphical file manager | `deb1acd`, `5b9dc3c` (yazi; "a GTK file manager would be the one window out of step") | Thunar (`c14f1a3`), then Isle's own Files (v1 D73), then yazi removed (v1 D85) | the out-of-step window was the problem; owning it solved it | as cited |
| GTK file dialog | xdg-desktop-portal-gtk | Isle's own chooser | every app meets Isle at the file dialog | v1 D67 |
| Tray in the dashboard only | `5b9dc3c` ("wrong thing in a 400px pill") | also in the island panel, menus as pages | it had to be one hover away | `4eb009c` |
| Shell never manages windows | pre-reset principle 5 | float everything, places, drag tiling, minimise, fullscreen spaces in a plugin | dynamic tiling was turned off in practice | `87517b0`, `05eee80`, v1 D40, D48, D91; B8 |
| Green title-bar button | zoom (`8db75f9`) | fullscreen with its own space; double click zooms | macOS's meaning | `f638cf1` |
| No rebinding UI; one chord table, no profiles | `115c8e8`, `743afd0` | chord rebinding (`754c907`); Windows and Mac profiles per keyboard | a Mac keyboard needs app-first chords | v1 D58, D60 |
| Focus mode | built in slice 10 (`33e3256`) | removed: "do not disturb is the one toggle" | two overlapping silencers | `10228e7` |
| Title-bar exception list of classes | `9a2ebe3` | read what windows declare (Motif hints, `_GTK_FRAME_EXTENTS`, X11 types, xdg-decoration) | lists rot | `5f91b5d`, `856109e` |
| Switcher content | icons, previews captured on open (`9c1f99a`) | live previews (`b25c67d`), one card per window (`bc9b82d`), one card per app (v1 D59) | per window "says the same name three times" | as cited |
| Auto theme from wallpaper | `890ccf5` | clock times, then the sun | *inference*: predictability over cleverness | `ddfeca6`, `bb3396e` |
| Island spring | pre-reset hover spring | curve, OutQuint 360 ms | the spring was tuned against a measuring window it barely met (`18b1b86`) | `Island.qml` on `main` |
| Six materials, squircle shader, Phosphor, concentric radii | pre-reset | one glass, circular corners, Lucide, a plain ladder | the reset rebuilt fast, simpler | `43c576c` |
| Big Picture | a tile grid launching Steam's UI | Isle as the console's system layer (v1 D38) | a launcher for a launcher; fought Steam Input for the pad | then dropped entirely (B22) |
| Cloud drives, Proton Pass, a Goodix driver, the Steam-in-Big-Picture launch | added | removed | not worth their cost (`e705aed`, `75a7d6e`, `8183a31`, `9edda6e`) | Proton Pass came back as a provider (v1 D42) |
| Opacity walked 85% to 96%; a hairline on every pill | Wave 0 | both reverted | "a translucent shell transmitting four percent is an opaque shell with an expensive blur behind it" | `8ded377`, pre-reset D8 |
| "Dark only, deliberately" | Wave 0 | light and Auto | "the shell does not control the backdrop" | `890ccf5` |
| The scored build loop (critic rounds, gauntlet, builders barred from docs) | 2026-09-05 to 09-07 | reset; slices 0–13 rebuilt in a day | see §6 | `a7af25e`, `58a35d1` |

---

## 5. Per surface

### Island and its control panel

v1: a 30px pill at top centre with desktop dots, clock and conditional
glyphs; hover unfolds it into a 420px panel (44px header with clock and date
centred, a toggle grid, output and sliders, media, tray, a footer of
batteries, updates and power profile); events morph it for a few seconds.
Screenshots: `island-rest.jpg` (dot, 19:22, a download glyph: three items),
`island-panel.jpg`, `island-notification.jpg`.

- **Held:** the conditional-glyph rule; the island as the only OSD; events in
  one queue where the same kind replaces itself; durations by importance
  (OSD 1.5 s, copy confirmations 1.8–2.5 s, mode 2 s, auth 3 s, media 4 s,
  notification 5 s or 8 s critical, app timeouts clamped 3–15 s, update and
  capture results 5–6 s, calendar and drive 8 s, pairing 30 s, recording and
  sharing until stopped).
- **Churned:** the hover guards (§3.5); a tall event rounded to half its
  height and pushed text into the curve, so the radius caps at the panel's
  (`6680fe6`); the surface had to stay a constant width because a top-anchored
  layer whose width changes is re-centred by the compositor mid-animation, a
  56px lurch (`d1af0a8`, `05eee80`); content was sliced while the box grew
  until its opacity followed the growth (`d1af0a8`, `ca658b8`).
- **Toggles:** the Notifications tile is the DND switch, with the count on it
  and the centre behind its arrow; drill-down pages carry no toggle of their
  own (`49a415d`, `921f032`). One switch per thing, in one place.
- **Desktop dots** opened Mission Control. B24's proposal takes desktops out
  of the island.
- **Lessons for the new island:** it becomes the notch between menu-bar
  halves (B18), so v1's rest glyphs move to status items under the same
  "only when it says something" rule. A compositor-drawn morph must keep its
  anchor fixed so a size change never re-centres. Content fades with growth
  rather than on its own clock. If hover expands anything, the §3.5 guards
  are requirements, not polish.

### Dashboard and widgets

v1: a full-screen overlay (wallpaper dimmed and blurred) on a key or a click
on the clock, a 12 × 8 grid of glass cards, pages, an edit mode, a library and
a store with grants (v1 D23, D24, D28–D31, D35, D36).

- **Held:** widgets as folders with a manifest; the host fetches for a widget
  (v1 D31); grants are the user's, narrowed per scope (v1 D28).
- **Churned:** grid density (§3.3); the drag editor took six commits in a day
  to reach "a soft rearrange rather than a snap" (`f19a4a0`, `108035f`,
  `4144694`, `06578cd`): neighbours flow live from the pre-drag layout, the
  push cascades in the drag direction; accidental closes (a click on a card's
  body, then a click on the gaps, used to close it: `8db75f9`, `52e7b89`);
  unsaved edits ask on every close route (`7ed0716`).
- **What the screenshots show:** most cards empty-stated at a glance; the
  surface is a place you go to, not something seen in passing (*inference*:
  it was used less than the island; its widget work is all on 2026-09-10 and
  09-11, and after that only notification and sheet fixes touch it, while
  the island kept changing).
- **Lessons:** B24 proposes widgets in Notification Centre and on the desktop.
  Keep the editor's live push and keyboard control; size units small enough
  that a strip of tray icons or two batteries is one row; design empty states
  to collapse rather than fill a card.

### Launcher

v1: 640px card at 30% height; prefixes as hints inside the field
(`launcher.jpg`), no magnifier glyph (`0e062e2`), 48px icons, Enter, Shift and
Ctrl variants.

- **Held:** prefixes; ranking by use; shell actions first (`97bcbf0`); raise
  rather than relaunch (v1 D63); a chord shown on its result, never a
  shortcut as a result (`7b66ce5`).
- **Churned:** results scroll at eight rows; hover selects only on movement.
- **Lesson:** a fixed-height result list and pointer-stable selection carry
  over to Spotlight unchanged.

### Switcher

v1: 240×150 live previews, one card per app with a count, backtick through
the app's windows, Q quits and W closes while held, commit on modifier
release.

- **Churned:** four content models in four days (§4); recency cycling made a
  third window unreachable, so the within-app cycle walks a stable order
  (v1 D59); capture only while visible (v1 D55).
- **Lesson:** B20 confirms app-centric. Keep the stable within-app order and
  Q and W on the held app.

### Mission Control

v1: spaces bar with live miniatures and a `+`, the current desktop's windows
spread without overlap and never larger than real, close with ×, W, Delete or
middle click, drag between desktops, a hidden-window tray, windows travelling
from their real rectangles (v1 D19).

- **Held:** all of it; it is one of the few surfaces built once and left.
- **Lesson:** the hidden-window tray existed because v1 had no Dock; with B19
  minimised windows go to the Dock. Closing from here is a deliberate
  departure from macOS worth keeping (*inference*: the user values it, since
  the DESIGN text argues for it).

### Notifications

v1: the toast is an island event (summary, body, up to three actions, inline
reply, image); the centre is an island page; "Earlier" holds records that
survive a restart (v1 D26); silence rules with a "while you were away" and
"while you played" summary; per-app rules.

- **Churned:** location (§3.4); the count (§3.4); Focus mode removed for one
  DND (`10228e7`).
- **Lesson:** records versus live notifications is a real distinction to
  keep in the design (Earlier reads the same, acts on nothing). The summary
  after silence is the proof the shell got out of the way.

### Lock, auth and power

v1: wallpaper blurred and dimmed, the island grown into a 28px clock, the
password field on typing, a shake on failure, attempt count, Caps Lock
warning, fingerprint beside the password (v1 D33); power actions appear on
any input and fade when idle (`b5bae3c`); auth a centred dialog announced in
the island; the power menu five pills with first-letter selection and a
countdown before restart or shut down.

- **Lesson:** "the island grows into the clock" is v1 principle 3 at its
  best; under B18 the equivalent is the menu bar giving way to the lock.
  Fingerprint never blocks the password (v1 D33) is a requirement.

### Settings

v1: a real window with a sidebar, search, a back and forward trail
(`6943013`), pages shaped after whichever desktop does each best: Displays
after macOS (`4df2e61`), keyboards and layouts after Plasma (`d172a5e`,
`10228e7`), Devices after Windows' Device Manager (`d431488`).

- **Churned:** sidebar grouping twice (`9423468`, `d6a032c`); copy cut hard
  (`39c886d`); control types by kind of value (`972bbe7`).
- **Lesson:** copy rules are part of the design (one-sentence subtitles, a
  description only where the label leaves something to say, no instructions
  under switches). Under B17 the ordering follows macOS; v1 shows the user
  accepts other desktops' layouts where they are better for a page.

### Files and the other windows

v1: Finder-shaped Files (tabs, List, Columns, Grid, Quick Look, saved
searches, one-step undo); Monitor after Activity Monitor; Cleaner after
Broom; Keychain.

- **Churned:** Files' tabs alone took four commits ("A new tab is a new tab",
  "The tab strip is a strip of tabs", "letting go means letting go":
  `6892e1e`, `c9e0b59`, `8a21b58`); the audit was redone "read off the code,
  not off a memory of Finder" because use found faults in a minute
  (`cc11570`, AUDIT § Files).
- **Glass in windows failed:** a context menu drawn in glass inside a window
  could not be seen, since the blur rule matched layer surfaces only; menus
  and cards in a window are the raised tone (`535ca6c`).
- **Colour:** the Cleaner's ten kind colours are the one place v1 used
  colours beyond accent and state, justified as data (v1 D88).
- **Lesson:** windows are opaque with their own raised tones; materials for
  in-window menus and popovers must be designed separately from shell glass.
  Standard behaviours (deselect on empty click, right-click keeps a
  selection, rename in place) are tested by use, not by feature list.

### Title bars

v1: hyprbars, 28px, title at the left in secondary text, three dots at the
right in warn, ok and danger (minimise, fullscreen, close) showing their
glyph on hover, over the window colour; none over a client that draws its own
(`settings-appearance.jpg`, `game-mode.jpg`).

- **Churned:** the green button's meaning (§4); deciding which windows get a
  bar went from a class list to what windows declare (§4); late-declaring X11
  windows needed a re-check on draw (`129a457`).
- **Lesson:** B25 moves the buttons left as traffic lights. Keep: the bar
  reads as part of the window (same colour), and decoration follows the
  client's declaration, re-checked late.

### Game mode

v1: automatic on a gamemoded registration or a game-like fullscreen window,
also on a key and a toggle; animations, blur, shadows, gaps and borders off;
glass to 94%; the island unmapped with a 4px hot zone to peek it; idle
inhibited; notifications held and summarised on leaving; night light off and
the performance profile on, both restored (inventory §10).

- **Held:** "the shell's job is to get out of the way, and a single line on
  return proves it did" (pre-reset § Game mode); no pill over a game, and no
  OSD (pre-reset D34).
- **Churned:** "detected, never toggled" (pre-reset) became detected plus a
  toggle; Big Picture went from launcher to system layer to dropped (B22).
- **Lesson:** every material needs a solid form for game mode from the start
  (as P1's risks already say); the reveal at the top edge is a competition
  item for the structure stage (p1-design § Competition). B30's Gaming Focus
  takes over the silencing.

---

## 6. Process lessons for P1

These are about how v1's design was decided, and bear on P1's gates.

| Lesson | Evidence |
|---|---|
| A human look at a gate outranks a score. Wave 0 passed "on human review rather than on the critic's number" after four rounds in which the last disputed whether a panel was 904 or 905 pixels wide | `58a35d1`, `bd4c69d` |
| Motion must be judged moving. The owner watched a 4.6 fps capture and found the lurch and the slicing that 27 gauntlet checks and five critic rounds had not | `d1af0a8` |
| Whoever builds must not quietly edit the spec. A missing font was "fixed" by changing the type spec to another face | `58a35d1` |
| Check what actually renders. The mono face silently substituted (`58a35d1`); icons drew at double size (`73c12ca`); the canvas blurred at 24 where Hyprland blurred at 8 | as cited |
| A rule that lives only in a comment gets reported as a defect; a deleted section stays deleted unless something reads it | pre-reset D2, D30 |
| Contrast gates measure token against token as painted; a translucent surface over someone's wallpaper cannot be gated analytically. State the guarantee precisely (content against its own body) | `8ded377`, `7a7d251`, pre-reset § What contrast is guaranteed |
| The reset itself: heavy process produced a well-measured clock in a pill over three days; the slices after it produced every surface in one | `a7af25e`, slices `27bfaa5`–`6c4f2b8` dated 2026-09-09 |

---

## 7. The user's evident taste

All *inference*, each with the evidence it rests on.

| Inference | Evidence |
|---|---|
| Wants a calm, empty screen and is irritated by indicators that report the normal state | principle 1 in both eras; conditional glyphs; status glyphs removed from the panel header (`f2bb6fd`); Odyssey's always-lit island refused |
| Wants Isle to be one coherent thing and will replace a working third-party tool to get it | Thunar, the GTK chooser, wvkbd, AppImageLauncher and Gear Lever all replaced or refused (v1 D37, D67, D73, D82) |
| Expects platform-standard behaviour exactly, and borrows from whichever platform does a surface best | Finder, Activity Monitor, Broom, Plasma keyboards, Device Manager, Task View's close, a browser's back and forward; the Files audit redone from use |
| Dark by default but wants light and Auto, with Auto predictable | light added over "dark only"; Auto by the sun (`bb3396e`); `settings-appearance.jpg` captured in light at Auto |
| Picks the accent and the palette; does not want colour taken from the wallpaper | v1 D16, `1a2bd4f`, the six-swatch accent picker |
| Prefers glass to opaque, but not at the cost of reading | v1 D8 chose dark glass over the canvas's Ink alternate (opaque, no blur); windows near-opaque; opacity walks reverted both ways |
| Sensitive to motion defects; wants things to appear in place rather than be corrected | lurch and slicing fixes; v1 D91 (`6f5c6ab`, "instead of sliding there from the middle"); island stays folded under an unmoved pointer |
| Quick, unshowy motion | 140–360 ms with exits faster; the spring dropped from the shell |
| Moderate density: no empty air, but not cramped | dashboard units halved; lists scroll; icons a step up once rendering was honest |
| Terse copy | `39c886d`; "no instructions under switches" |
| Keyboard-first with a Mac keyboard | chords on every action; first-letter selection; the Mac profile work (v1 D58–D61) |
| Gaming means the desktop vanishes and owns nothing of the console | game mode unmaps the island; v1 D38 "whoever is fullscreen owns the pad"; B22 drops Isle's console layer |
| Adds features quickly and cuts what does not earn its keep | cloud drives, Focus mode, Proton Pass, the Goodix patch, the Steam launch experiment, yazi |
| Decides by looking, at speed | the reset; Wave 0 passed on human review; candidates "side by side" for icon sizes (`66dbd86`) |

---

## 8. Carry as requirement, and do not repeat

### Carry as requirement

- Nothing at rest that does not say something; every status item has a rule
  for when it shows (v1 § The island, Rest).
- One accent, as a signal; fixed palette, user-chosen accent; no
  wallpaper-derived colour (v1 D16).
- Contrast guaranteed for text against the surface it sits on, in every
  theme and in increased contrast; no promise that a translucent body
  separates from the wallpaper; the reduced-transparency form is the lever.
- A solid form of every material for game mode and reduced transparency,
  defined with the material, not after.
- Windows opaque, with in-window menus and popovers on their own raised
  material; shell glass only where the compositor blurs.
- Morphs anchored so a size change never re-centres; content revealed with
  growth, not on its own timer; a radius cap so tall states keep their text
  out of the corner (`d1af0a8`, `6680fe6`).
- If hover opens anything: dwell, grace, settle, disarm after a click, hold
  events while open, select on movement only (§3.5).
- Things open where they belong: no visible slide into a remembered place
  (v1 D91).
- Exits faster than entrances; reduced motion and game mode remove movement.
- One switch per thing in one place; drill-downs carry no duplicate toggle.
- Event durations scaled by importance; same-kind events replace; live
  activities (recording, sharing) hold until stopped.
- Silence is followed by a one-line summary ("while you were away", "while
  you played").
- Copy: sentence case, "desktop", one-sentence subtitles, descriptions only
  where needed, no instructions under switches; buttons named for their
  action.
- Small layout units and empty states that collapse; long lists scroll at a
  fixed height.
- Decoration follows what a client declares, re-checked late.
- Judge sizes at true scale on the real renderer, and verify the font face
  resolved.

### Do not repeat

- Moving the notification centre between surfaces: decide its home at gate 2.
- Walking opacity up, or adding hairlines to every surface, to rescue
  contrast over one wallpaper (pre-reset D8).
- A flat ladder of near-identical greys for depth (surface to raised at
  1.10:1, pre-reset D25).
- Glass inside windows (it cannot be seen).
- A mock-up whose material values differ from the runtime's (canvas blur 24
  against Hyprland size 8).
- Taste rounds spent on a rendering defect: confirm the renderer before
  resizing anything (`73c12ca`).
- Overlapping silencers (Focus beside DND); B30's Focus model replaces both.
- Class lists for behaviour a protocol can declare.
- A full-screen surface that a stray click on a gap closes.
- Critic scores or pixel gauntlets as the gate; spec text edited to make a
  build pass.
- Big cards of empty states in a widget surface.
