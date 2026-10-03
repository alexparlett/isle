# Keychain

P1 stage 5, wave C. How Keychain looks and moves, in Frost: the Passwords
app's shape over every secret the user keeps. What it is, its states, keys
and copy are `structure/keychain.md` (binding); the window's anatomy,
toolbar, sidebar rows, lists, states in place, popovers, the app's settings
window and focus are `surfaces/conventions.md`'s, sheets and alerts
`surfaces/window-chrome.md`'s, the reveal gate and the keyring unlock
`surfaces/prompts.md`'s, the "Password copied" event `surfaces/island.md`'s,
and are reused here, not redrawn. Values are `tokens.kdl` names; numbers not
yet tokens are listed at the end.

## Anatomy

### The window

- **Default size** `size-keychain-width` by `size-keychain-height` (1000 by
  640), minimum 560 by 400; first placed centred in the work area (x 1220,
  y 400 on the ultrawide).
- **Regions**: the toolbar (52); the sidebar, `size-sidebar` (220),
  `colour-sidebar`; the list, `size-list-pane` (300), dragged at its trailing
  edge between `size-list-pane-min` (240) and `size-list-pane-max` (420),
  `colour-view`, a `colour-hairline` at its trailing edge; the detail,
  `colour-window`, the rest.
- **Toolbar.** Leading: the lights; the sidebar toggle; then, `space-l` into
  the list's region, the title (the sidebar row's name) in
  `type-style-headline` over "{n} items" in `type-style-subheadline`
  `colour-text-2`, tabular. Nothing in the centre. Trailing, over the
  detail: the primary action, a regular button at `size-control-large`
  labelled "New login…" (or "New SSH key…" in SSH keys), disabled in Wi-Fi,
  Apps and a provider that cannot create; `space-l`; More
  (`dots-three-circle`); the search field.

### The sidebar

`conventions.md`'s rows (28, inset `space-s`), in two groups `space-m`
apart, no headings. Each row leads with a `size-pane-tile` (20) tile, as
Settings' panes and macOS's Passwords: radius `radius-row`, the glyph Fill
at 16, white, the tile keeping its hue on the selection.

| Row | Glyph | Tile |
|---|---|---|
| All | `key` | `pane-blue` |
| Logins | `globe` | `pane-indigo` |
| Wi-Fi | `wifi-high` | `pane-teal` |
| SSH keys | `terminal-window` | `pane-graphite` |
| Apps | `app-window` | `pane-purple` |
| A keyring (Login) | `vault` | `pane-grey` |
| A provider | The provider's app icon at `size-app-icon-row` (20), no tile | |

- **Count**: the trailing accessory, `type-style-subheadline` `colour-text-2`,
  tabular (`colour-on-accent` on the focused selection). Kinds count every
  unlocked source; a source counts its own items.
- **Lock**: a locked or signed-out source shows `lock-simple` Fill at
  `size-glyph-small` (12) in `colour-text-2` in the count's place.

### The list

`conventions.md`'s two-line rows: `size-row-large` (40), inset `space-s`,
`radius-row`; no stripes.

| Part | Look |
|---|---|
| Leading tile | 32 square, radius 8 (as `size-pane-tile-large`): a login's site icon at 20, centred, when its provider supplies one (Keychain fetches none); otherwise a monogram, the title's first letter in `type-style-title-3` `colour-text` on `colour-fill-strong`. Other kinds: their sidebar glyph, Regular at 20 in `colour-text-2` on `colour-fill-strong`; an Apps item the app's icon at 32 with no tile |
| Title | `type-style-body` `colour-text`, cut at its end |
| Second line | `type-style-subheadline` `colour-text-2`: the user name or main attribute, then, where the list spans sources, " · " and the source's name; cut at its end |
| Vaults | A provider with several vaults: `conventions.md`'s section header (24, `type-style-subheadline` semibold `colour-text-2`) over each vault's rows |
| Selected | `conventions.md`'s selection; the tile keeps its colours |
| Locked row | First in a list that spans a locked source, not selectable: the tile with `lock` Regular at 20 in `colour-text-2`; "{source} is locked" in `type-style-body`; "Unlock…" a regular button at `size-control`, trailing `space-s` in; a `colour-hairline` under the row. A signed-out provider's row carries the provider's own action in its words instead ("Unlock…" or its sign-in label), as a regular button |

### The detail

A column `size-form` (600) at most, centred, `space-xxl` at the sides at
least, `space-xl` from the toolbar.

| Part | Look |
|---|---|
| Header | The item's tile at `size-app-icon-medium` (48), radius 12, drawn as the list's at that size (site icon at 32, monogram in `type-style-title`); `space-m`; the title in `type-style-title-2` `colour-text` over "{kind} · {source}" in `type-style-body` `colour-text-2`. Trailing, centred on the header: "Edit", a regular button at `size-control`. Wi-Fi items have no Edit button, and Return does nothing there |
| Field group | `space-xl` under: `settings.md`'s group (`colour-window-raised`, `radius-card`, `colour-hairline` ring, separators from the label edge). Rows at least `size-row-large` (40), padding `space-m`: the label in a `size-detail-label` (120) column, `type-style-body` `colour-text-2`; `space-m`; the value in `type-style-body` `colour-text`, selectable, cut at its end; the row's buttons trailing, `space-s` apart |
| Secret value | Hidden: ten dots, always ten whatever the length, in `type-style-body` `colour-text`, tracked 0.12 em. Shown: the value in `type-style-mono` `colour-text`, wrapping, selectable |
| Show, Copy | Regular buttons at `size-control` (24), padded `space-s`: "Show" (then "Hide") and "Copy". Show precedes Copy; non-secret fields (user name) carry Copy alone |
| Verification code | Hidden: "••• •••". Shown: the six digits as two groups of three in `type-mono` at `type-style-title-3`'s size, semibold, tabular, `space-xs` between groups; `space-s` after, the countdown ring (below) |
| Countdown ring | `size-glyph` (16) across, `stroke-progress` (2): track `colour-fill-strong`, the time left as an arc in `colour-text-2` from twelve o'clock, shrinking anticlockwise once a second; no number, no colour change near the end. Its accessible name is "Changes in {n} seconds", read on focus, never announced each second |
| Website | The value as a plain button in `colour-accent-text`, opening the site |
| Notes | The row's value wraps, `type-style-body`, no cut |
| About group | `space-xl` under: Kind, Stored by, Source, Created, Modified, as the field group with rows of `size-row` (28) and no buttons; dates in the user's format |
| Foot | Wi-Fi: "Wi-Fi Settings…" a regular button, leading, `space-l` under the last group |
| Nothing selected | `conventions.md`'s empty line, "No item selected", centred in the detail |

**SSH keys.** The field group holds Type, Fingerprint (`type-style-mono`,
selectable), Comment, and In agent with "Add to agent" or "Remove from
agent" trailing. The public key follows in a well (`colour-window-sunken`,
`radius-control`, padding `space-s`), `type-style-mono` `colour-text-2`,
wrapping, at most three lines then cut at the end, with "Copy public key"
under it, trailing.

**Edit.** In place: each editable value becomes `settings.md`'s text field
at `size-control`, the value column's width; "Edit" is replaced by "Cancel"
and "Done", Done the default in `colour-accent` (Return); the password field
carries a 20 icon button at its trailing end, `dice-five` at 16 in
`colour-text-2`, which opens the generator's popover. "Delete…", a regular
button in `colour-danger-text`, sits leading under the last group.

### The generator

| Part | Sheet (Item › Generate password…) | Popover (in a password field) |
|---|---|---|
| Frame | `window-chrome.md`'s sheet, 400 wide | `conventions.md`'s popover, `size-generator` (320) wide, under the field |
| Password | A well, `colour-window-sunken`, `radius-control`, padding `space-m`: the password in `type-mono` at `type-style-title-2`'s size, `colour-text`, wrapping, selectable, centred, at most three lines | The same well at `type-style-title-3`'s size, at most two lines |
| Length | A row: "Length" in `type-style-body`, the slider (`settings.md`, track 4, knob 18, accent fill) filling the row, then a number field 48 wide at `size-control`, tabular, with its stepper | As the sheet |
| Sets | Four rows of `size-row` (28): the label, the switch trailing | As the sheet |
| Buttons | `space-l` under: "Generate again" leading (regular); "Copy" (regular), then "Done", the default, trailing | "Generate again" leading; "Use" trailing, the default |

The well's text changes in place on every change; the switch for the last
set left on is disabled, so a password always has characters.

### The reveal gate

`prompts.md`'s polkit frame, unchanged: Keychain's icon at 48 with the
`lock` badge, "Keychain needs your password", Isle's message for the
action, the password field (or the reader's fingerprint form), Cancel and
Allow armed after `timing-prompt-arm-ms`. The keyring's unlock is
`prompts.md`'s keyring form, called by the Secret Service; Keychain draws
neither.

### Copy feedback

Copy changes nothing in the window. The island shows its line event for
`timing-copied-ms`: `key` and "Password copied" for a password, `check` and
"Copied" for a user name, a code or a public key (`island.md`).

### Sheets and alerts

New login and New SSH key are `window-chrome.md` sheets of `settings.md`'s
rows, labels leading, fields at `size-row-field` (200); "Save in" a pop-up
button; Cancel and Save (default). Generating a key shows
`conventions.md`'s determinate bar under the fields and disables Save.
The passphrase sheet: "Enter the passphrase for “{key}”" in
`type-style-headline`, one secure field, Cancel and Add. A provider's
sign-in is `window-chrome.md`'s sheet of the fields it declares, Cancel
and its action as the default. Deleting a login or an Apps item is the
alert form with the `warning` glyph (it cannot be undone), the Apps line
as its informative text, Cancel default, Delete in `colour-danger-text`.
Deleting an SSH key asks nothing: both files go to the Trash, with Undo,
as Files. Leaving Edit with changes is `window-chrome.md`'s alert:
Discard (`colour-danger-text`), Keep Editing, Save the default.

### Keychain's settings

`conventions.md`'s settings window with one pane (no tab row): two pop-up
rows, "Clear copied passwords after" and "Hide shown passwords after".

## States

| State | What changes |
|---|---|
| Rest | The last sidebar row, the list, the selected item's detail |
| Loading | Sidebar and the frame at once; rows appear as each source answers, with no fade; a slow provider's rows join below without moving the selection |
| Empty kind | `conventions.md`'s empty line in the list ("No saved logins"); SSH keys adds "New SSH key…" under it |
| Nothing selected | The detail's line |
| No results | `conventions.md`'s line in the list; the detail empty |
| Locked source | The sidebar lock; the locked row heading every list that spans it; its titles listed below |
| Provider failed | Its state sentence and error in the list's place, as `conventions.md`'s missing-daemon form; in All and the kinds, one row at the top in the locked row's form, carrying the error |
| No Secret Service | "No keyring service is running. Choose one in Settings › General › Passwords." in the list's place for Logins, Apps and the keyring; in All, one row at the top in the locked row's form, Wi-Fi and SSH items listed under it; Wi-Fi and SSH keys as normal |
| Shown value | The value in mono in place of the dots; "Hide" in place of "Show" |
| Code shown | The digits and the ring |
| Editing | Fields, Cancel and Done, Delete…; the list and sidebar still work; leaving with changes asks (Sheets and alerts) |
| Generating a key | The sheet's bar; the switcher card's |
| Narrow | The sidebar collapses; then list and detail stack, the detail over the list with a Back button (`caret-left`) leading in the toolbar |
| Inactive | `conventions.md`'s inactive form |
| Second screen, game in front, Focus on | No change of their own |
| Locked session | Nothing shows; shown values are hidden before the lock draws |

## Motion

| Transition | How | Reduce Motion |
|---|---|---|
| Show, Hide | Dots and value swap in one frame; the row's height, if the value wraps, on `motion-move` | Height cut |
| Selection, detail | The detail is replaced in the frame the selection changes, as a list does | Same |
| Edit, Done, Cancel | Values crossfade to fields over `motion-fade-ms` (out over `motion-exit-fade-ms`) in place; rows that change height move on `motion-move`; the header's buttons swap in one frame | Crossfade over `motion-reduced-fade-ms`, heights cut |
| Code changes | The old digits out over `motion-exit-fade-ms` and the new in over `motion-fade-ms`, in place; the ring refills in one frame | Same |
| Ring | One step a second, cut | Same |
| A locked source unlocks | The locked row leaves over `motion-exit-fade-ms`; rows under it close up on `motion-move`; secrets now reachable change nothing visible | Cut |
| Narrow stack | Detail over list and Back: a crossfade over `motion-fade-ms`, no slide (as Settings' sub-pages) | Cut |
| A login dragged out | Its drag image as `files.md`'s: the row's tile and title at 70% | Same |
| Generator popover, sheets, alert | `conventions.md` and `window-chrome.md` | As there |
| Prompts, island event | `prompts.md`, `island.md` | As there |

**Game Mode:** every row is a cut.

## Accessibility

- **Surfaces.** Sidebar text on `colour-sidebar`, list on `colour-view`,
  detail on `colour-window` and `colour-window-raised`, the generator's text
  on `colour-window-sunken`. Window roles only; `colour-text-3` only for
  placeholders. The reveal gate is glass: `prompts.md`'s rule holds.
- **Tiles.** The sidebar tiles' white glyphs are decoration beside their
  names, as Settings' pane tiles; monograms are `colour-text` on
  `colour-fill-strong` and pass 4.5:1.
- **Secrets.** Dots are a fixed count, so length never shows. The ring
  carries no meaning the digits lack.
- **Increase Contrast.** `conventions.md`'s borders; the list's rows ringed
  `colour-hairline-strong` (structure); groups and wells ringed; tiles and
  the ring's track ringed; glyphs bold.
- **Reduce Transparency.** No change: the sidebar is opaque already.
- **Focus.** Sidebar and list show focus as the accent selection; Show,
  Copy, Edit, the website button and fields take the ring outside; a
  locked row's Unlock… takes it as any button. ⌘R and ⌘C act on the
  selected item whichever region has focus.
- **Largest text size.** Rows grow to their line plus 12; the detail's labels
  go above their values (structure), the buttons staying trailing on the
  value's first line; the dots and digits grow with the text; tiles keep
  their size.
- **Targets.** Sidebar and list rows 28 and 40; Show and Copy 24 tall and at
  least 44 wide; the generator button 20; the ring is not a target.

## Board

Wave C's Keychain board at 3440×1440 with the 1920×1080 screen beside it,
dark first and each in light. The window at its default place; crops are x,
y, width, height on the ultrawide.

1. **All at rest**: GitHub selected, site icons and monograms, counts, the
   Login keyring and Proton Pass. Crop 920, 300, 1600, 900.
2. **Login detail**: password hidden; shown; the code shown with its ring at
   20 s and at 4 s; the island's "Password copied" over the screen's top
   (crop 1420, 0, 600, 60).
3. **Every kind**: a Wi-Fi network with Wi-Fi Settings…, an SSH key in the
   agent, an Apps item. Crops as 1.
4. **Locked**: the keyring locked, its sidebar lock, the locked row over its
   titles in All; Proton Pass signed out.
5. **Edit**: fields, Done in the accent, the generator's popover open.
6. **Generator sheet**: 24 characters, symbols off; then 64.
7. **Sheets and alerts**: New login with Save in open; New SSH key
   generating; the passphrase sheet; a provider's sign-in; Delete for an
   Apps item; leaving Edit with changes.
8. **The reveal gate**: the polkit frame over the window, password and
   fingerprint forms. Crop: `prompts.md`'s.
9. **States**: empty SSH keys, nothing selected, no results, no Secret
   Service (in Logins and in All), a provider failed (its own list and
   All); inactive.
10. **Narrow**: the stacked form with Back, 560 wide.
11. **Accessibility**: scenes 1 and 2 under Increase Contrast and at the
    largest text size.
12. **Motion**, on the harness's spring: Edit in and out, a code changing,
    an unlock, the stacked crossfade; each under Reduce Motion.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `keychain-width`, `keychain-height` | 1000, 640 |
| | `list-pane`, `list-pane-min`, `list-pane-max` | 300, 240, 420 (a list column beside a detail) |
| | `detail-label` | 120 (a detail row's label column; Monitor's Inspect uses it too) |
| | `generator` | 320 (the generator popover's width) |

Phosphor glyphs to vendor: `key`, `globe`, `terminal-window`, `vault`,
`lock-simple`, `dice-five`.
