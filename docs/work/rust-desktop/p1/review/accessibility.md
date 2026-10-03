# Review: accessibility

P1 stage 6, the whole-program pass. Every surface spec (`surfaces/`) and
its structure sheet (`structure/`) against WCAG 2.1 AA and the brief:
contrast, the keyboard, Reduce Motion, Reduce Transparency, Increase
Contrast, the largest text size, hit targets, colour alone, accessible
names, timing and flashing. Paths are relative to
`docs/work/rust-desktop/p1/`; `DESIGN.md` is `docs/DESIGN.md`. Findings
only: nothing here has been applied. Contrast figures below were computed
with `system/contrast.py`'s own functions over `system/tokens.kdl`.

## Summary

`python3 system/contrast.py` passes everything it checks: dark 169/169,
dark with Increase Contrast 87/87, light 167/167, light with Increase
Contrast 87/87, every accent's `on` and `text` forms. The failures below
are in pairs the script does not check: the focus ring, the badge, a
destructive button's label, the primary button's hover, text on the dark
`match` highlight, placeholders on fills, and anything on glass other than
`text` and `text-2`.

The keyboard maps are the strongest part: every surface has one, focus
enters and leaves as `competition.md` orders it, Escape closes everything
that opens, and the two traps (the lock, a system prompt) are deliberate
and stated. Three places lack a key route (the Split View divider, Quick
Look's media controls and Markup and Trim). Every animation has a Reduce
Motion form and every glass surface a solid one, with two small gaps in
Mission Control's list. Nothing flashes more than twice a second: the menu
blink is one flash, spring-loading two, and the lock's shake is movement,
not a flash, and goes under Reduce Motion. Timed things are recoverable
(banners in Notification Centre, the thumbnail's file, OSD levels in
Control Centre), except Control Centre's failed-action clause and the
island's "ended" reason. The weakest areas are accessible names (given in
copy tables for about half the surfaces and missing for most glyph-only
states), the largest text size (never defined; controls and badges have
fixed heights), and picture-in-picture, which shows no focus at all.

Counts: 57 findings, 7 blocker, 28 fix, 22 polish, and 16 checks
recorded as passing; 7 questions for the user.

## Findings

### Across surfaces: contrast and the system

| id | where | severity | finding | fix |
|---|---|---|---|---|
| A-01 | `system/components.css:50`; `surfaces/conventions.md:144`; `system/schema.md` `stroke` row | blocker | The focus ring is the accent at 55%. Over the window surfaces it measures 1.84 to 2.36 for blue (dark 1.94 on `window`, 1.84 on `window-raised`; light 2.29 on `window`) and 2.1 to 2.3 for graphite, yellow and green in light: under 3:1, so it fails 1.4.11 for a focus indicator and contradicts `system/theming.md:323` ("the accent, at least 3:1 against what it surrounds"). At full opacity every accent passes on every window surface and `material-solid` (least 3.11, dark purple on `window-raised`; light least 3.83, red on `titlebar`). The 55% rule is gate 4's, so the choice is the user's (For the user 1) | `components.css:50`: `outline: var(--stroke-focus-ring) solid var(--colour-accent);`. `conventions.md:144`: "`stroke-focus-ring` (3) in `colour-accent`, at `stroke-focus-offset` (1) outside the control"; drop "the focus ring at full accent" from `conventions.md:208`. `schema.md` `stroke` note: "The focus ring is the accent at full strength, keyboard focus only" |
| A-02 | `system/components.css:163`; `surfaces/switcher.md:47`, `:66`; `surfaces/spotlight.md:108` | blocker | The badge sets white on `colour-danger`: dark 2.82:1 for 11-point semibold figures (4.5 needed). The switcher's urgent attention tag draws a white Fill glyph on the same capsule: 2.82, under 3:1 for a graphic. Light passes (4.52) | Add `colour.dark.on-danger "#1c0806"` (6.85 on dark `danger`) and `colour.light.on-danger "#ffffff"`; badge and urgent tag use `colour-on-danger`. `components.css:163` `color: var(--colour-on-danger)`; `switcher.md:47` "the badge component (… `colour-danger`, `colour-on-danger`, …)", `:66` "fill weight, `colour-on-danger`, on a `colour-danger` capsule"; `spotlight.md:108` likewise |
| A-03 | `system/components.css:59`; `system/tokens.kdl:31`, `:69`; destructive buttons in `surfaces/window-chrome.md:143`, `settings.md:101`, `keychain.md:102`, `cleaner.md:122`, `files.md:204`, `monitor.md:108` | blocker | `components.css` draws a destructive label in `colour-danger` (dark 4.32, light 3.67 on `colour-control` over `window`). The specs say `colour-danger-text`, which also fails on a control: light 4.48 over `window`, 4.18 over `window-sunken` and `sidebar`, 4.10 over `titlebar`; dark 4.21 over `window-raised` (Settings' action rows sit in groups on `window-raised`) | `tokens.kdl`: dark `danger-text "#ff8a83"` (4.65 on a control over `window-raised`, 4.85 over `titlebar`); light `danger-text "#b82a25"` (5.01 over `window`, 4.68 over `window-sunken` and `sidebar`, 4.60 over `titlebar`). `components.css:59` `color: var(--colour-danger-text)` |
| A-04 | `system/tokens.kdl:32`; `surfaces/conventions.md:78`; `spotlight.md:73`; `settings.md:124` | blocker | Dark `match` (`rgba(242,193,78,0.55)`) under `colour-text` ("text unchanged") gives 3.96 on `view`, 3.79 on `window`, 3.57 on `window-raised`; dark text on it is no better (3.87 to 4.30). Light passes (13+) | Dark `match "rgba(242,193,78,0.40)"`: `colour-text` 5.86 on `view`, 5.50 on `window`, 5.02 on `window-raised`, and the run still reads at 2.6 to 2.7:1 against its surface |
| A-05 | `system/components.css:57` | blocker | The default button's hover mixes 12% white into the accent, which lowers `on-accent`'s contrast: dark blue 3.79, dark purple 3.80; light blue 4.36, pink 3.97, red 3.86, orange 3.90, yellow 4.26, green 4.12, graphite 3.99 | Hover moves the fill away from its label: `color-mix(in srgb, var(--colour-accent) 92%, black)` where `on` is white, `92%, white` where `on` is dark (every such pair rises above the fill's own ratio). `contrast.py` checks `on` against the hover and pressed fills of every accent |
| A-06 | `surfaces/conventions.md:42`, `:203`; `surfaces/settings.md:183-184` | fix | Tertiary text on a fill: the toolbar search's "Search" placeholder is `colour-text-3` on `colour-fill` over the content or the scroll-edge bar (dark 4.47 over `window`, 4.23 over `titlebar`; light 4.47 over `titlebar`); the recorder's "None" and "Type shortcut" are `colour-text-3` on `colour-fill` in a group row (dark 4.12 over `window-raised`) | `conventions.md:42` "placeholder "Search" in `colour-text-2`"; `settings.md:183-184` "None" and "Type shortcut" in `colour-text-2`. `conventions.md:203`: "`colour-text-3` … only for an inactive window's title and glyphs and placeholders in fields on `colour-window-raised`; never on a fill" |
| A-07 | `system/contrast.py:11-18`, `:96-111` | fix | The script checks text roles and marks only on the window surfaces, glass, the island and the lock. Pairs the specs rely on that it never computes: the focus ring; status and accent colours on glass (Spotlight's caret, a prompt field's accent and danger edges, the lock keyboard's Caps Lock dot); text roles on `control`, `fill`, `fill-strong` and `selection-unfocused`; `on-accent` on hover and pressed fills; badge text; text on `match`; `kind` glyphs on their fills and text on a map tile's mix (`cleaner.md:201-205` quotes figures the script does not produce); Increase Contrast's `hairline-strong` on tinted glass (it passes: least 3.71 dark, 3.77 light) | Add each pair with its threshold: ring and marks 3:1, text 4.5:1; text-3 checked on fills so A-06 cannot recur; every accent, not only the default, for ring, hover and selection |
| A-08 | `system/components.css:47`; `system/tokens.kdl:86-101` | fix | Under Increase Contrast the board's CSS outlines only windows, glass and the island; buttons, search fields, segmented plates, pop-ups and tiles keep fill-only edges (dark `control` 1.66:1, light 1.25:1), though `surfaces/conventions.md:206-209` requires borders on every control. The contrast block leaves `fill`, `fill-strong` and `selection-unfocused` unchanged, so an unfocused selection stays at 1.55 (dark) and 1.22 (light) unless each spec adds an edge | `components.css`: under `[data-ic="1"]`, `box-shadow: inset 0 0 0 1px var(--colour-hairline-strong)` on `.s-btn`, `.s-field.search`, `.s-seg`, `.s-popup`, `.s-switch`, `.s-tile`, `.s-side-item.sel`, `.s-table .tr.sel-unfocused`. `DESIGN.md` Contrast: "Increase Contrast … borders that identify controls and unfocused selections reach 3:1" |
| A-09 | `system/schema.md` `type` row; `DESIGN.md:101`; `surfaces/island.md:288` | blocker | The largest text size is never defined. Every spec assumes 1.5× (`island.md:288` "assumed 1.5×; Gaps", a section that does not exist), `tokens.kdl` has no scale, and `DESIGN.md` says only "the user's text size multiplies every style". Stage 6's text-scale check has no setting to check against | `tokens.kdl` `type { scale-max 1.5 }`; `schema.md` `type` row: "`scale-max`: the largest text size the user can set; every style multiplies by the user's factor, 1 to `scale-max`". `DESIGN.md:101`: "The user's text size multiplies every style, up to 1.5 (`type-scale-max`); Zoom magnifies past it". `island.md:288`: "(1.5×, `type-scale-max`)" |
| A-10 | `system/components.css:53`, `:61`, `:163`; `surfaces/notifications.md:27`; `menus.md:66`; `island.md:94-97`; `spotlight.md:205-206` | fix | Controls and badges keep fixed heights at large text. A small button (20) holds callout's 15-point line, 22.5 at 1.5×; the quiet badge and the island's count capsule (16) and the badge (18) hold subheadline's 14, 21 at 1.5×. `spotlight.md:205-206` keeps badges at their size while `switcher.md:176` scales the same badge | `DESIGN.md` Type: "At a larger text size a control's or badge's height is the larger of its token and its label's line plus the same padding; widths follow the label". `spotlight.md:205-206`: "Icons and covers keep their size; badges grow with their text, as the switcher's" |
| A-11 | `surfaces/window-chrome.md:139-141`; `structure/conventions.md:171` | fix | An alert has "one line of informative text" and no rule for when it does not fit, at large text or in a long locale. A failed action's line is what to do: it must not be cut | `window-chrome.md:140`: "one line of informative text in `type-style-callout`, `colour-text-2`, wrapping, never cut: "one line" is a length for the copy, not a clip" |
| A-12 | `structure/copy.md` | fix | No rule for accessible names. Names are given in copy tables for the island's glyphs, Control Centre, Notification Centre, capture's toolbar, the lock, the on-screen keyboard, widgets and Mission Control; the rest have glyph-only controls and states with none (per surface below) | `copy.md` new rule: "**Accessible names.** Every symbol-only control, glyph and mark has a name: the thing, then its state as a second clause ("Wi-Fi: Home, signal low"; "Shuffle, on"). A control with a tooltip uses the tooltip's words without the chord. A state shown only by a mark (changed, waiting, nearly full) is the item's accessible description" |
| A-13 | `surfaces/conventions.md:45`; `control-centre.md:33-35`; `spotlight.md:31` | polish | Tooltips show only after a pointer rest and go on press or leave. A keyboard user never sees one, so Control Centre's Small tiles and every toolbar symbol button show no visible name or state; and the tooltip cannot be dismissed without moving the pointer (1.4.13) | `conventions.md:45`: "after `timing-tooltip-ms` (700) of rest, or of keyboard focus on the control; gone on press, leave, focus moving or Escape" |
| A-14 | `system/components.css:61`; `island.md:138`; `notification-centre.md:57`; `widgets.md:67`, `:99`, `:180`; `keychain.md:81`; `settings.md:99`; `files.md:50`, `:72`, `:105`; `cleaner.md:159` | polish | Small buttons and 20-point glyph buttons take a 20 target even in rows of 28 to 56 that have room for 24, which the badges and close circles already get. All meet `size-hit-minimum`; the brief asks for near 24 where possible | `DESIGN.md` Components: "A control smaller than `size-control` takes a hit box `size-control` (24) tall, centred, wherever its row is at least that tall". Keychain's Show and Copy at `size-control` (`keychain.md:81`) |
| A-15 | `DESIGN.md` | fix | No Accessibility section. Stage 6's output is `DESIGN.md`, and the rules that hold across surfaces are scattered across 29 specs: the ring, keyboard-only focus, hit minimum, text size, names, the solid and tinted forms, motion | New section after Motion: "**Accessibility.** Every surface has a keyboard map; nothing is reached only by pointer. The focus ring shows for keyboard focus only: the accent, 3 points, 1 outside. Targets are at least `size-hit-minimum` (20), 24 where the row allows. Text size runs to 1.5×; controls, rows and bars grow with it and nothing is clipped. Reduce Motion turns every move into a fade in place; Reduce Transparency draws every material solid; Increase Contrast tints the glass, borders every control at 3:1 and steps symbols to bold. Colour is never the only sign. Every control, glyph and mark has an accessible name (`copy.md`). Transient things are announced, and anything that leaves on a timer can be found again" |

### Window chrome

| id | where | severity | finding | fix |
|---|---|---|---|---|
| W-01 | `surfaces/window-chrome.md:33-57`; `structure/window-chrome.md:374-375` | fix | The lights take no keyboard focus and have no tooltips, which is right, but they have no accessible names either, and a screen reader activates them through the tree | `window-chrome.md` Accessibility: "The lights are named "Close", "Minimise", and "Enter Full Screen" or "Exit Full Screen" ("Zoom" while Option is held); a disabled light is announced as dimmed" |
| W-02 | `structure/window-chrome.md:328-365` | pass | Every chrome action has a chord or the window menu (keyboard Move and Resize included); the zoom menu takes focus on a key and Escape closes it; focus returns to the window. The inhibit notice leaves after `timing-event-ms`, but its chord stays in the island's notice row, reached by a top-edge push over a full-screen VM | None |

### Menu bar and menus

| id | where | severity | finding | fix |
|---|---|---|---|---|
| M-01 | `surfaces/menus.md:22`, `:25`, `:66-67` | fix | Glyph-only parts have no names: the system menu's isle mark, the overflow », the scroll arrows, the Updates count | Names: "System menu" (the mark), "More menus" (»), "Scroll up" and "Scroll down", "{n} updates" |
| M-02 | `surfaces/menus.md:83`, `:141-143`; `structure/menu-bar.md` Keyboard | polish | The structure says Up and Down skip disabled rows; `menus.md:83` lets them land on a disabled Help result; `:142-143` says disabled rows are "skipped and announced as dimmed", which cannot both hold for arrow navigation | `menus.md:142-143`: "and is never the only sign: arrows skip it (Help's results excepted), and it is in the accessibility tree as dimmed, so a screen reader's own navigation reads it". `menu-bar.md` Keyboard, Up and Down: "skipping separators and disabled rows, except Help's results" |
| M-03 | `surfaces/menus.md:123`, `:126` | pass | The choice blink is one off-on over 80 ms on one row: a single flash, far under three a second and under the area threshold | None |

### Island

| id | where | severity | finding | fix |
|---|---|---|---|---|
| I-01 | `structure/island.md:485`; `surfaces/island.md:61-97` | fix | Names are incomplete. Wi-Fi's name drops the signal level the glyph draws and the Connecting form; VPN's drops Connecting; Battery's drops charging and low; the privacy marks, the compact faces, the other activities' marks and the "+{n}" count have none | `island.md` Copy, glyph names: "Wi-Fi: {network}, signal {low, medium, high}", "Connecting to Wi-Fi", "VPN: connecting", "Battery {n}%, charging", "Battery low, {n}%". New rows: privacy marks "Microphone in use", "Camera in use", "Screen being recorded" or "Screen being shared"; faces "Recording, {time}", "Sharing with {app}", "Call, {time}", "Timer, {time} left", "Now playing: {title}", "{app} is waiting for an answer"; marks by their kind; the count "{n} more activities" |
| I-02 | `surfaces/island.md:249-254`, `:264-293` | fix | Nothing says events are announced. Banners are (`structure/notifications.md:243`); an OSD, "Copied", a mode change, attention, an activity starting or ending, and a privacy mark arriving are silent to a screen reader | `island.md` Accessibility: "**Announcements.** Each event is announced politely as it shows (an OSD as its level, "Volume 40%"); a held event (timer done, urgent attention) and a privacy mark's arrival assertively" |
| I-03 | `surfaces/island.md:111-112`, `:288-293` | fix | At large text the pill, glyphs, wells and cells are sized, but the Level form (300 by 48) and the Start row (72) are not: at 1.5× the Level's name line and bar need 57, the leader row's two lines plus insets 69 | `island.md:288-293` add: "The Level form is its name line, the bar and its insets (57 at 1.5×) and widens to 360; the Start row is its leader row's lines plus 24" |
| I-04 | `surfaces/island.md:126`; `structure/island.md:109` | polish | "Recording stopped: the window closed" shows for `timing-ended-ms` and then cannot be found again (2.2.1); the island shows no notifications, so nothing keeps it | The ending with a reason also leaves a silent record in Notification Centre under the capture part (no banner, no sound), so the reason can be read later |

### Control Centre

| id | where | severity | finding | fix |
|---|---|---|---|---|
| C-01 | `surfaces/control-centre.md:38-41` | fix | A failed glyph action shows its clause on the state line for `timing-event-ms`, then the true state: the error leaves on a timer (2.2.1, 3.3.1). A Small tile has no state line, so the failure shows only in a tooltip that needs a pointer rest | "**A failed glyph action**: the circle returns to the true state; the state line (at Small, the tooltip and the name) reads the error's first clause until the module is next used or the panel closes, and the failure is announced assertively; the whole sentence is in the tooltip and the accessible name" |
| C-02 | `surfaces/control-centre.md:64`, `:92`, `:155`, `:158`, `:173-195`; `structure/control-centre.md:471` | fix | Names cover tiles and Back only. Unnamed: transport buttons, shuffle and repeat with their state, the scrubber, each app's volume slider and mute, the input mute and gain, disclosures, remove badges, the gallery's add buttons, tray items | `control-centre.md` Copy, Accessible names, add: "Previous track", "Play", "Pause", "Next track", "Shuffle, on/off", "Repeat, off/all/one", "Position", "{app} volume", "Mute {app}", "Mute microphone", "Input level", "Other Networks, collapsed/expanded", "Remove {module}", "Add {module}"; a tray item by its title |
| C-03 | `surfaces/control-centre.md:155` | polish | Shuffle and repeat on differ from off by a `colour-fill-strong` plate (about 1.5:1 on glass) and `colour-text` against `colour-text-2`: lightness only, no weight change, unlike every other on/off glyph | "Shuffle and repeat on: Fill in `colour-text` on a 30 circle of `colour-fill-strong`; off: Regular in `colour-text-2`, no circle; under Increase Contrast the circle takes the `colour-hairline-strong` edge" |
| C-04 | `structure/control-centre.md:304-337` | pass | The grid, pages, tray and edit mode have full keyboard paths, including moving, resizing and removing tiles; Escape steps back then closes; focus returns to the window | None |

### Focus and Game Mode

| id | where | severity | finding | fix |
|---|---|---|---|---|
| FO-01 | `surfaces/focus.md:101-121` | pass | On and off differ by Fill against Regular as well as the accent; the state line names the mode; rows and durations have keys; Reduce Motion and the solid forms are given. The mode event's announcement is I-02 | None |

### Notification banners

| id | where | severity | finding | fix |
|---|---|---|---|---|
| B-01 | `surfaces/notifications.md:91`, `:133-137`; `structure/notifications.md:74-84`, `:229-243` | pass | Timing holds: hover pauses every banner, per-app Persistent style keeps them, a timed-out banner stays live in Notification Centre, and the centre's chord focuses the newest; banners are announced, critical ones assertively; urgency is never a colour. The count badge's height is A-10 | None |

### Notification Centre

| id | where | severity | finding | fix |
|---|---|---|---|---|
| N-01 | `surfaces/notification-centre.md:39-40`, `:153-155` | polish | At default, a selected day differs from today only by colour (a `colour-fill-strong` circle against an accent circle, both semibold). Increase Contrast fixes it with a ring; the default does not | `notification-centre.md:40`: "Selected day: a 1.5-point ring of `colour-text`, no fill, the number semibold", in every mode; drop the Increase Contrast sentence that introduces it |
| N-02 | `surfaces/notification-centre.md:53-55`, `:59` | polish | A collapsed event row names its calendar only by the mark's colour; the name appears only when the row expands | The event row's accessible name ends with its calendar ("Stand-up, 09:30 to 10:00, Work"); the mark's tooltip names the calendar |

### Lock, greeter and idle dim

| id | where | severity | finding | fix |
|---|---|---|---|---|
| L-01 | `surfaces/lock.md:44-47`, `:178-179`; `structure/lock.md` Copy | polish | The wrong-layout mark is an accent dot, said to be "backed by the message", but no message names the layout ("Wrong password. Try again.") | With several layouts set, the wrong-password message reads "Wrong password. Check the keyboard layout ({name})." (for the copy review), and the capsule's name becomes "Keyboard layout: {name}, check it" |
| L-02 | `surfaces/lock.md:34`, `:42`; `structure/lock.md:359` | fix | Names cover the field, the reader and the layout only. Unnamed: Now Playing's transport, the on-screen keyboard button, the battery capsule, the session capsule, notification plates | Add: "Previous track", "Play", "Pause", "Next track", "On-screen keyboard", "Battery {n}%" ("charging"), "Session: {name}", plates "{App}, {n} notifications" |
| L-03 | `keyboard-osk.md:67-69`, `:95-100` | polish | On the lock the keyboard is a glass plate, and the Caps Lock dot is `colour-ok` straight on it: unchecked, and light `ok` on glass over a dark picture is 1.81:1 | On the lock's plate the dot sits in a 1-point `material-solid` ring, as the tray's attention dot |
| L-04 | `structure/lock.md:238-257`; `structure/competition.md:81-83` | pass | The lock holds the keyboard by design and says so: only media, volume, brightness, layout and power keys act; Tab reaches every control; focus returns to the field. The shake is two cycles of 6 points on the field (movement, not a flash) and goes under Reduce Motion and Game Mode; doze and dim timings are settable and any input restores | None |

### System prompts

| id | where | severity | finding | fix |
|---|---|---|---|---|
| P-01 | `surfaces/prompts.md:123`; `structure/prompts.md:296`; `surfaces/widgets.md:149-150` | fix | With no field, focus starts on Cancel and moves to the action when armed: an unrequested focus change after 600 ms, which a screen reader announces as a jump, and the structure puts focus on the default button | "One prompt: frame and scrim; focus in the first field, else on the action from the first frame; until armed, Return and Space on it do nothing; arming moves no focus". The widgets consent panel follows |
| P-02 | `surfaces/prompts.md:53` | polish | Typed and untyped digits of a displayed pairing code differ only by fill and text colour | Typed cells' digits semibold; the code's accessible name "{n} of {m} digits typed"; under Increase Contrast, typed cells take the `colour-hairline-strong` edge |
| P-03 | `structure/competition.md:84-90`; `structure/prompts.md:339-358` | pass | Focus is held deliberately: chords cannot take it, Escape cancels, Cmd+Tab sets the prompt aside into the island and Show brings it back. The session countdown stops on any input (2.2.1) | None |

### Spotlight

| id | where | severity | finding | fix |
|---|---|---|---|---|
| S-01 | `surfaces/spotlight.md:29` | fix | The caret is `colour-accent` on glass: dark blue on the default glass over a white window is 1.17:1, so the only focus mark in the field can vanish | "Caret 2 wide, the line's height, `colour-text`" |
| S-02 | `surfaces/spotlight.md:195-201` | fix | Focus stays in the field while the arrows move a selection; nothing says how a screen reader learns of it | Accessibility, Focus, add: "The field is a combo box controlling the list or grid; the selected row or tile is its active descendant, announced with its group's header; a new result count is announced politely" |
| S-03 | `structure/spotlight.md:244-276` | pass | Every action has a key or is in Cmd+K's menu; Escape clears then closes; focus returns in the first frame | None |

### Capture

| id | where | severity | finding | fix |
|---|---|---|---|---|
| K-01 | `surfaces/capture.md:91-102`; `structure/capture.md:358` | fix | Markup's tools, undo and the colour swatches are icon-only with no names or tooltips; the copy names only the capture toolbar | `capture.md` Copy, Markup: "Pen", "Highlighter", "Line", "Arrow", "Rectangle", "Ellipse", "Text", "Redact", "Crop", "Undo", "Colour"; swatches "Red", "Orange", "Yellow", "Green", "Blue", "Purple", "Black", "White"; each a tooltip and the accessible name |
| K-02 | `surfaces/capture.md:130` | polish | In the picker the chosen tile wears the focus ring's form, and the grid's focus is another ring, so focus and choice look alike | "Chosen: the preview ringed `stroke-found` (2) in `colour-accent` at `stroke-focus-offset`, with a `check-circle` Fill 16 at its top trailing corner on a `material-solid` circle"; the focused tile takes the focus ring |
| K-03 | `structure/capture.md:237-282`; `surfaces/capture.md:73-83` | pass | Pick, toolbar, colour pick and picker have full keys; the thumbnail never takes focus, holds on hover, and everything it offers is reachable from the file, so its 5 s timer loses nothing | None |

### System Settings and app conventions

| id | where | severity | finding | fix |
|---|---|---|---|---|
| SE-01 | `surfaces/settings.md:35`, `:140-143` | fix | The changed-setting dot is a mark with no words; a screen reader is not told | Accessibility, add: "A changed row's accessible description is "Changed from default"; a pane's sidebar row "{Pane}, has changed settings"" |
| SE-02 | `surfaces/settings.md:99`, `:112`, `:114`, `:164` | fix | Unnamed: the accent swatches, the list's + and −, the stepper's halves, the Displays map's island mark | Names: each swatch by its colour ("Blue" … "Graphite", "Other colour…"), selected as state; "Add {item}", "Remove {item}"; "Increase {setting}", "Decrease {setting}"; "Island: main display" |
| SE-03 | `surfaces/settings.md:171-175` | polish | Keep or revert reverts after 15 s: a time limit kept under 2.2.1's essential exception (a display that shows nothing cannot be answered), but nothing says it is announced | "The sheet is announced on opening ("Keep these display settings? Reverting in 15 seconds") and again at 5 seconds" |
| SE-04 | `structure/settings.md:392-411`; `structure/conventions.md:278-299` | pass | Every pane, row and the Displays map have keys; Tab reaches every control; focus returns on closing; a symbol button's name is its tooltip | None |

### App switcher

| id | where | severity | finding | fix |
|---|---|---|---|---|
| SW-01 | `surfaces/switcher.md:160-183`; `structure/switcher.md` Copy | fix | Only the name band is text; the badge, window count, progress, attention and urgency have no names, and the selection's move is not announced | Accessibility, add: "Each plate's name is "{App}" then its states: "{n} windows", "{n} notifications", "{n}% done", "needs attention" (urgently), "hidden", "minimised", "not responding", "running in background", "quitting"; the selection's move is announced" |
| SW-02 | `structure/switcher.md` Keyboard | pass | Every action has a key (Q, H, W, ↓, `, Return, Escape); the keys work while Cmd is held and Return commits without releasing it | None |

### Mission Control, Spaces, Split View, Stage Manager

| id | where | severity | finding | fix |
|---|---|---|---|---|
| MC-01 | `surfaces/mission-control.md:148-151`, `:237`; `structure/mission-control.md` Keyboard; `structure/window-chrome.md:362-363` | fix | The Split View divider moves only by drag (and double-click to halves): no key route | `window-chrome.md` Keyboard: "On a Split View pane, the window menu's Resize moves the divider: arrows, Shift for larger steps, Return keeps, Escape undoes". `mission-control.md:237` adds "or by the window menu's Resize" |
| MC-02 | `surfaces/mission-control.md:161-164`, `:200`, `:257-258` | polish | Reduce Transparency lists bar, tray, capsules and header but not the two `material-glass-clear` forms: the Split View drop half and the zone-preview form on the other screen's spread | `mission-control.md:257-258`: "Bar, tray, capsules, header, the drop half and the other screen's drop form `material-solid`" |
| MC-03 | `structure/mission-control.md:412` | polish | Names cover the bar's controls only | Tiles "{title}, {app}"; tray thumbnails "{title}, minimised" or "hidden"; miniatures "Desktop {n}" with "current" as state |

### Widgets

| id | where | severity | finding | fix |
|---|---|---|---|---|
| WG-01 | `surfaces/widgets.md:63`, `:65`, `:68` | fix | Graphics carry readings with no text: the analogue clock, Weather's glyph-only conditions (2×1, the hourly and daily rows), System's rings and graphs | Accessibility, add: "The dial is named for the time and city; each weather glyph by its condition ("Light rain"); a ring or graph by its series and value ("CPU 34%")" |
| WG-02 | `surfaces/widgets.md:250-251` | polish | At large text Media drops the skip buttons: a control lost in place | "Media drops the art, then the artist; the transport stays" |
| WG-03 | `surfaces/widgets.md:98` | polish | A failing widget's reason is cut to one line with no way to read the rest | "…then the reason, `colour-text`, cut; the whole reason in its tooltip and accessible name" |
| WG-04 | `structure/widgets.md` Keyboard | pass | Full keys, including Cmd+arrows to move with the push and Option+Cmd+arrows to resize, so the 8-point resize bands are not the only route | None |

### Files and Quick Look

| id | where | severity | finding | fix |
|---|---|---|---|---|
| F-01 | `surfaces/files.md:266`, `:268`; `structure/files.md:521-548` | fix | Quick Look's video controls and its full-screen toolbar fade after `timing-controls-ms` and return on pointer movement only; the keys reach the toolbar (Tab) but not the controls capsule (play, position, volume) | `files.md:266` and `:268`: "shows on pointer movement or any key, stays while focus is in it, and fades after `timing-controls-ms` at rest". `structure/files.md` Quick Look row: "Tab into its toolbar, then the media controls; Return plays or pauses the focused control, arrows move a focused slider" |
| F-02 | `surfaces/files.md:270-271`; `surfaces/capture.md:85-111`; `structure/files.md` Keyboard | fix | Markup and Trim have no keyboard map. Freehand drawing is path-dependent and exempt; choosing tools, placing shapes, redaction, crop and trimming are not | `structure/files.md` Keyboard, new row: "Markup: Tab through tools and colours, Return chooses; Return on the image places the chosen shape, crop or redaction at a default size at its centre; a mark is selected by Tab, moved with the arrows, resized with Option and the arrows (capture's portion mode), Delete removes. Trim: Tab to each handle, arrows move it a frame, Shift a second; Space plays" |
| F-03 | `surfaces/files.md:34` | fix | The jobs button's states (running, the waiting dot, failed) are drawn only | Its name: "File jobs: {n} running"; "…, waiting for you"; "…, failed" |
| F-04 | `surfaces/files.md:166-168`, `:332`; `surfaces/menus.md:126` | polish | Spring-loading drops its blink under Reduce Motion; the menu's blink is kept there "being one change of colour". Neither moves anything; the two rules disagree. Both pass the flash rule (two blinks within 160 ms on a row or cell) | `files.md:332` Reduce Motion: "Same: the blinks change colour, not place", as menus |
| F-05 | `structure/files.md:521-548` | pass | Finder's map is complete for windows, tabs, views, selection, the Trash, panels and drags; every drag has Cut and Paste or Move To | None |

### Monitor

| id | where | severity | finding | fix |
|---|---|---|---|---|
| MO-01 | `surfaces/monitor.md:74` | fix | A graph's history is readable only by pointer hover; the figures give the current value only (1.1.1) | Accessibility, add: "Each well is a focusable graph named for its title, with a summary ("CPU load, last 2 minutes, 12 to 46%, now 31%"); focused, Left and Right move the hover line by a sample and read its value and time" |
| MO-02 | `surfaces/monitor.md:88-90` | polish | The chosen figure group (operations or data) differs from the other by text colour only | "the chosen group's labels take `colour-text` semibold"; the group is a toggle with "selected" as its state |
| MO-03 | `structure/monitor.md:95-101` | polish | Rows hold their order while the pointer is over the table, not while the keyboard works in it, so a row moves under arrowing at each sample | "…except while the pointer is over the table, for `timing-type-select-ms` after a key in it, or while a menu or alert is open" |

### Keychain

| id | where | severity | finding | fix |
|---|---|---|---|---|
| KC-01 | `surfaces/keychain.md:83` | polish | The code's countdown ring has no value: the time left is graphic only | The ring's name "Changes in {n} seconds", read on focus, not announced each second |
| KC-02 | `structure/keychain.md:88-104`, `:175` | pass | Shown values hide and copied secrets clear on timers the user sets in Keychain's settings; the code's 30 s period is real-time and essential (2.2.1) | None |

### Cleaner

| id | where | severity | finding | fix |
|---|---|---|---|---|
| CL-01 | `surfaces/cleaner.md:79-81` | polish | A nearly full disk is told only by its bar turning `colour-warn`, unlike Storage's widget (`warning` glyph and weight) | "past nine-tenths `colour-warn`, `warning` 12 in place of `hard-drive`, and the free figure semibold `colour-text`" |
| CL-02 | `surfaces/cleaner.md:159`; `structure/cleaner.md` Keyboard | polish | Taking an item out of the basket has a pointer target only | Keyboard: "Delete removes the selected basket item"; the target's name "Remove {name} from basket" |
| CL-03 | `surfaces/cleaner.md:206-209` | pass | Kinds carry symbol, name and colour together; the map's identity is in symbols, labels, gaps and tooltips | None |

### Picture in picture

| id | where | severity | finding | fix |
|---|---|---|---|---|
| PI-01 | `surfaces/pip.md:54`, `:92-95` | blocker | Focus is reached by direction from the keyboard, and a focused PiP shows no change: it already wears the active shadow, has no bar, and draws no ring. Keyboard focus is invisible (2.4.7) | `pip.md:54` Focused: "By keyboard, the focus ring round the window at `radius-window` plus `stroke-focus-offset` while it holds focus; by a click, no change". `:92-95`: "Focus. A keyboard arrival draws the ring; a click does not" |

### On-screen keyboard, dropdown terminal, wallpaper

| id | where | severity | finding | fix |
|---|---|---|---|---|
| X-01 | `surfaces/keyboard-osk.md:144-169`; `surfaces/dropdown-terminal.md:76-88`; `surfaces/wallpaper.md:123-133` | pass | The keyboard never takes focus (its own chord and Settings reach it), latches differ by a bar, not a shade, and names say "held" or "locked"; the terminal's focus is its caret and its size is in Settings; the wallpaper has no text and no keys of its own. The lock keyboard's dot is L-03 | None |

## For the user

1. **The focus ring's strength** (A-01). Gate 4 made it the accent at
   reduced opacity (55%); at that strength it is under 3:1 on every window
   surface, and `theming.md` already promises 3:1. (a) Full accent
   everywhere; (b) keep 55% in windows and full under Increase Contrast
   only. **Recommended: (a)**: every accent passes 3:1 at full strength on
   every window surface and `material-solid`, and a ring the user cannot
   see fails the one job it has. On glass no accent is safe over every
   backdrop (dark blue on default glass over white is 1.17), so the ring on
   glass also carries a 1-point `colour-text` line inside it, as the
   capture line pairs two tones.
2. **Secondary text on dark glass over a bright window** (`DESIGN.md:76-78`).
   The rule lets `text-2` fail over white in dark mode: 3.04:1 on the
   default glass, which is a common case (a light web page behind a banner,
   Control Centre or Spotlight). (a) Accept and record it; (b) raise dark
   `glass` to 0.84 opacity, near the tinted end; (c) an adaptive floor: the
   compositor reads the blurred backdrop's mean luminance under each glass
   surface and moves its tint toward `glass-tinted` until `text-2` holds
   4.5:1. **Recommended: (c)**, with (b) as the fallback if P3 cannot
   sample the blur cheaply: it keeps the frosted look over dark content and
   fixes the case that fails.
3. **The glass slider's clear end.** At `glass-clear` even primary text
   fails over the opposite extreme: 2.68 in dark over white, 4.31 in light
   over black. (a) Let the user's choice stand; (b) apply the floor from
   item 2 to text-bearing surfaces at every slider position. **Recommended:
   (b)**: the slider then changes how much colour shows through, never
   whether text can be read (brief P4).
4. **Non-text contrast at the default.** Control edges and selections sit
   at the Mac's levels (checkbox and field edges 1.69 to 1.91, unfocused
   selections 1.22 to 1.55), reaching 3:1 only under Increase Contrast;
   WCAG 1.4.11 asks 3:1 always. (a) Keep the Mac's levels and record the
   departure in `DESIGN.md` Contrast; (b) raise `hairline-strong` to 3:1 by
   default. **Recommended: (a)**: (b) makes every control outlined at rest,
   against brief P1 and P3, and Increase Contrast is one switch away.
5. **The largest text size** (A-09). WCAG 1.4.4 asks for 200%. (a) Text
   size to 1.5×, with the compositor's Zoom for anything larger; (b) text
   size to 2×. **Recommended: (a)**: the specs are drawn to 1.5×, the island
   and panels stop fitting the strip past it, and Zoom meets 1.4.4 for
   every surface at once.
6. **Escape on a Control Centre peek** (1.4.13). A peek covers the window
   and goes only when the pointer leaves. (a) Escape, while the pointer
   rests in the island's zone, closes the peek and disarms hover until the
   pointer leaves, taking that one Escape from the window; (b) as now.
   **Recommended: (a)**: the pointer is on the island, so the Escape is
   meant for it, and the window keeps every other key.
7. **Sticky Keys.** Every desktop chord and Cmd+Tab need a held modifier;
   only the on-screen keyboard latches. (a) Add Sticky Keys (and Slow
   Keys) to Settings › Accessibility, latched in the compositor and drawn
   with the keyboard's held and locked forms; (b) leave it to a later
   packet. **Recommended: (a)**, scheduled with P5's chord table: it is the
   one keyboard route a user who cannot hold two keys has.
