# System prompts

P1 stage 5, wave B. How the system prompts, the session dialog and Force
Quit look and move, in Frost, from `structure/prompts.md` (binding). The
crash notice and safe start are `notifications.md`'s and `island.md`'s
forms and are not drawn here. Values are `tokens.kdl` names; numbers not
yet tokens are listed at the end.

## Anatomy

### The frame

One frame for every agent prompt (polkit, keyring, network, Bluetooth).
It is the shell's glass, which no app window can be (windows are opaque,
sheets and alerts `colour-window`), over a scrim no client draws above:
the two together are what reads as the system's.

| Part | Look |
|---|---|
| Scrim | `colour-scrim` over every screen, above windows and every surface but the island's privacy form, the lock, the on-screen keyboard and the idle dim |
| Frame | `size-panel` (376) wide, `material-glass` at the user's glass position (`material-blur`, `material-saturation`, `material-noise-opacity`), `radius-panel`, `shadow-panel`, padding `space-xl` (inner 336). Centred across its screen, its top at a fifth of the screen's height (288 on the ultrawide, 216 on the 1920 screen); it grows downward only |
| Who asks | The app's icon at `size-app-icon-medium` (48), centred, with the kind's badge: a 20 circle of `material-solid` holding the kind's glyph at `size-glyph-small` in Fill, `colour-text`, centred on the icon's lower trailing corner inset 4. Kinds: `lock` (polkit), `key` (keyring), `wifi-high` (Wi-Fi, 802.1X), `isle-vpn` (VPN), `bluetooth` (pairing). No app: the kind's glyph alone at 48, Light, `colour-text`, no badge. Bluetooth: the device's kind glyph at 48 Light (`headphones`, `keyboard`, `mouse`, `game-controller`, `device-mobile`, `speaker-hifi`, else `bluetooth`) |
| Title | `space-m` under; `type-style-title-3`, `colour-text`, centred, at most two lines, cut at the end |
| Message | `space-xs` under; `type-style-callout`, `colour-text-2`, centred, plain text, at most `limit-prompt-lines` (4); past that cut at the end, the whole in Details |
| Fields | `space-l` under, aligned to the inner width. Each: its label in `type-style-subheadline` `colour-text-2`, `space-xs`, then the field: `size-control-large` (30) tall, full width, `radius-control`, `colour-fill` with a `stroke-hairline` inner edge in `colour-hairline-strong`, inset `space-s`, `type-style-body` `colour-text`; placeholder `colour-text-2`; secrets as the lock's dots. Focused: the edge in `colour-accent` (the field component). `space-m` between fields |
| Identity | Several administrators: the pop-up button component at `size-control-large`, full width, the user's picture at 16 before the name, above the password field. Its menu is `material-solid` |
| Choices | `space-s` under the last field: checkbox rows `size-row` tall, the checkbox component, `space-s`, `type-style-body` `colour-text` |
| Status line | `space-xs` under fields or choices, a reserved slot of 16; leading. The lock's forms (`lock.md`): a glyph at 12 then `type-style-callout`, `colour-text` for what needs acting on, `colour-text-2` for checking and the reader |
| Details | Open: `space-m` above the buttons, a box of `colour-fill`, `radius-small` (concentric floor), padding `space-s`, `type-mono` in `colour-text-2`, selectable, at most 160 tall, then it scrolls |
| Buttons | `space-l` under; `size-control` tall. "Details" leading where there are details: a plain button, `caret-right` 12 turning to `caret-down` when open. Cancel then the action trailing, `space-s` apart, equal width, at least `size-button-minimum` (80). The action is the default button once armed (below) |
| Queue | `space-m` under the buttons: "1 more request", "{n} more requests" in `type-style-subheadline` `colour-text-2`, centred |

**The armed moment.** For `timing-prompt-arm-ms` (600) after the frame
appears, the action is drawn as a regular button (`colour-control`) and
Return, Space and clicks on buttons do nothing; fields take typing at once.
On arming its fill crossfades to `colour-accent` over `motion-fade-ms`. The
accent arriving is the sign that Return now answers (brief P6: it marks the
one thing Return acts on).

### Each kind

| Kind | Contents |
|---|---|
| Polkit | App icon with `lock`; "{App} needs your password" or "An administrator's password is needed"; polkit's message; the pop-up if several; "Password for {name}"; Details (action id, vendor; for `pkexec`, the command line), in `type-mono`; Cancel, Allow |
| Keyring, unlock | `key`; the requester's title and message; one password field; its "Automatically unlock…" choice when sent; its continue label, else "Unlock" |
| Keyring, new | Password, then Confirm; while they differ, "The passwords don't match." with `warning-circle` and the action disabled |
| Keyring, confirm | No fields: title, message, the two buttons it names |
| Wi-Fi | `wifi-high`; "“{network}” needs a password"; the security as the message ("WPA2 Personal"); a password field; "Show password" (shows the characters in `type-style-body`); Cancel, Join. Wrong: the old password kept and selected (`selection-mix` of the accent), the status line's message |
| 802.1X | `wifi-high`; "Sign in to “{network}”"; only the fields asked for: Username, Password, a key's password |
| VPN | App icon if a provider asked, else `isle-vpn`; "“{VPN}” needs a password"; the plugin's fields in its order, labelled as it labels them, secrets with "Show password"; Cancel, Connect |
| Bluetooth, confirm | The device's glyph; "Pair with {device}?"; "Check that {device} shows this code."; the code `space-l` under the message in `type-style-large-title`, tabular, tracking 0.08 em, `colour-text`, centred; Cancel, Pair |
| Bluetooth, passkey or PIN | "Pair with {device}"; one field (six digits, or up to sixteen characters), its text in `type-mono` |
| Bluetooth, display | "Type this code on {device}"; the code in cells: each digit in a `size-code-cell` (32) × `size-row-large` (40) box, `radius-small`, `space-xs` apart, the digit in `type-style-large-title`; untyped digits `colour-fill` and `colour-text-2`, typed ones `colour-fill-strong` and `colour-text`; Cancel |
| Bluetooth, authorise | "{device} wants to pair", Cancel, Pair; a service: "Allow {device} to connect?", its name as the message, Don't Allow, Allow |

### The session dialog

The frame's material, radius, shadow and padding, centred on the main
screen (its top fixed where it opened), over the scrim on every screen.

**Chooser.** No title. A row of five actions, each a column
`size-module-cell-width` (84) wide, `space-s` apart (452), then "Cancel"
trailing beside them, `space-l` after Shut Down: a regular button at
`size-control`, `size-button-minimum` (80) wide, centred on the circles
(inner 548, frame 588). Each action is a 48 circle of `colour-fill-strong`
with the glyph at 24, Regular, `colour-text` (Lock Screen `lock`, Log Out
`sign-out`, Sleep `moon-stars`, Restart `arrow-clockwise`, Shut Down
`power`), its name `space-s` under in
`type-style-callout` `colour-text`, centred, up to two lines. The selected
action (Shut Down at first): circle `colour-accent`, glyph Fill in
`colour-on-accent`. Hover steps an unselected circle to
`colour-control-pressed`. Then `space-l`, a `colour-hairline` rule across
the inner width, `space-l`; the open apps at `size-app-icon` (32), `space-s`
apart, centred, wrapping to two rows, past `limit-session-apps` (12) a
quiet badge "+{n}"; `space-l`; "Reopen windows when logging back in" as a
checkbox row, centred; `space-s`; the countdown line in `type-style-callout`
`colour-text-2`, tabular, centred, in a reserved slot.

**Confirmation.** The prompt frame (376) with the action's glyph at 48
Light in place of an app (`arrow-clockwise`, `power`, `sign-out`; for an
owed restart `arrows-clockwise`), the question as the title, the reasons as
the message for an owed restart, the open apps as above, the checkbox, the
countdown line, then Cancel and the action, armed after
`timing-prompt-arm-ms` as any prompt.

**Quitting.** The icons of apps not yet gone; the buttons disabled, a
spinner at 12 leading the countdown's slot. Interrupted: the slot says
"Couldn't restart because {App} didn't quit." with `warning-circle`, and
the buttons become Cancel and "Show {App}" (default). Not responding: that
app's icon takes the switcher's `hourglass-medium` tag (`switcher.md`), the
slot says "Not responding" with that glyph, and the buttons are Cancel and
"Force Quit {App}": destructive, never default, and on glass drawn as a
well (fill `material-solid`, label `colour-danger-text`) so its colour sits
on a solid ground, as the island's wells do.

### Force Quit

A glass panel as the frame, `size-panel` wide, padding `space-m`, centred
on the main screen, no scrim. Top to bottom: "Force Quit" in
`type-style-title-3` `colour-text`, leading; `space-xs`; "If an app doesn't
respond for a while, select it and choose Force Quit." in
`type-style-callout` `colour-text-2`; `space-m`; the list, a `colour-fill`
card at `radius-control` (18 less 12), rows `size-row` at `radius-small`,
inset `space-xxs`: the icon at `size-app-icon-small` (a game's art fitted),
`space-s`, the name in `type-style-body` `colour-text`, trailing "Not
responding" or "Couldn't force quit" (`warning-circle` 12) in
`type-style-callout` `colour-text-2`. Selected rows `colour-accent` with
`colour-on-accent` text; with the list unfocused,
`colour-selection-unfocused`. At most `limit-force-quit-rows` (10) show,
then it scrolls. `space-m`; "Force Quit" trailing, the default button.

**Its sheet.** Attached under the title row, the panel's inner width,
`material-solid` (glass never on glass), square top corners, bottom
`radius-card`, padding `space-l`; the question in `type-style-body`
`colour-text`; Cancel (default) and Force Quit (destructive), as
`window-chrome.md`'s alert. Return is Cancel, Escape cancels, and Force
Quit is reached by Tab (then Space). The list below takes `colour-scrim`.

## States

| State | What changes |
|---|---|
| One prompt | Frame and scrim; focus in the first field, else on Cancel until armed, then the action. |
| Armed | The action turns `colour-accent`. |
| Typing | Dots; Caps Lock's message. |
| Checking | Spinner and "Checking"; the action disabled (`colour-text-disabled`, no fill); Cancel works. |
| Wrong | The shake (Motion), dots gone, the field's `colour-danger` edge until the next key, the message. |
| Locked out | Field disabled, placeholder `colour-text-disabled`, the time left. |
| Fingerprint | `fingerprint` and the reader's message; typed dots kept; "Waiting for the fingerprint reader" on an early Return. |
| Details open | The box above the buttons; the frame taller. |
| Queued | The queue line; the frame one line taller when it first appears. |
| Requester cancelled, agent restarted | Dismissed (Motion), no message. |
| Waiting, set aside | No frame or scrim; the island's prompt face (`island.md`). |
| Loading | The kind's glyph at 48 until the icon resolves, then the icon with its badge crossfades in. |
| Many screens | Frame on one screen, scrim on all. |
| Windowed game | `material-solid`, no blur, no transitions. |
| Full-screen game or window, locked | Waiting. |
| Over Control Centre | The pinned panel under the scrim, still showing its outcome. |
| Session dialog | Countdown counting, then stopped (the line fades); quitting; interrupted; not responding. |
| Force Quit | Open with a selection; several selected; confirming (sheet); a row "Couldn't force quit". |
| Reduce Motion, Transparency, Contrast, largest text | Motion and Accessibility. |

## Motion

| Transition | Form |
|---|---|
| Prompt appears | `motion-open`: scale `motion-panel-scale` (0.96) → 1 about the frame's top centre, alpha with the spring's progress; content rides the frame's alpha, never its own clock. The scrim's alpha 0 → 1 over `motion-fade-ms`, starting with it. |
| Prompt dismissed | `motion-exit`: alpha → 0 and scale → 0.96 about the top centre; the scrim out over `motion-exit-fade-ms` (unless another follows). |
| Next in the queue | The scrim stays; the old frame out as above, then after `timing-prompt-gap-ms` (200) the next appears as new, armed afresh. |
| Set aside (Cmd+Tab) | The frame out as dismissed and the scrim with it; the island's face grows on `motion-morph` in the same frames, so the one goes as the other comes. Nothing travels between them. Show brings it back as a new prompt. |
| Grows (Details, queue line, a field appearing) | Height on `motion-open`, top fixed; the new part fades in by the growth's second half (`island.md`'s rule). Shrinking on `motion-exit`, the part out by the first half. |
| Status line | Replaced in one frame in its reserved slot. |
| Armed | The default fill over `motion-fade-ms`. |
| Shake | As the lock's (`lock.md`): the field alone, `motion-shake-ms`, `motion-shake-distance`. |
| Session dialog, Force Quit | Alpha over `motion-fade-ms` in, `motion-exit-fade-ms` out, in place, no scale. An app leaving the quitting row fades over `motion-exit-fade-ms`, then the rest close up on `motion-move`. |
| Force Quit sheet | `window-chrome.md`'s sheet: down from the title row on `motion-open`, up on `motion-exit`; the scrim on `motion-fade-ms`. |
| Chooser selection | Instant, as the switcher's. |

**Reduce Motion.** No scale and no shake: frames and scrim fade in place
over `motion-reduced-fade-ms`; growth is a crossfade to the new height at
the same top; the field's error edge carries a wrong password.

**Game Mode.** No transitions: frame and scrim appear and go in one frame;
no shake; the armed accent cuts in; `material-solid`, no blur.

## Accessibility

- **Colour roles.** On the frame (glass) only `colour-text` and
  `colour-text-2`; `colour-text-3` nowhere, placeholders included. Fields
  are `colour-fill` on the glass, so the glass rule holds inside them.
  Errors are told by glyph and words; the danger edge repeats them. The
  kind badge and Force Quit's sheet are `material-solid`.
- **Increase Contrast.** Frames take `material-glass-tinted` and a
  `stroke-hairline` edge of `colour-hairline-strong`; fields keep their
  `colour-hairline-strong` edge, buttons and checkboxes take theirs
  (`conventions.md`); Force Quit's list card and the chooser's circles get
  the same edge; glyphs bold; the danger edge 2 points.
- **Reduce Transparency.** Frames and panels `material-solid`; the scrim
  stays.
- **Focus ring.** `stroke-focus-ring` in the accent at `stroke-focus-offset`,
  keyboard only, outside each button, checkbox, pop-up and the Details
  button; a focused field shows its accent edge instead. The chooser's
  selection is its accent circle (no ring), as the switcher's plate. The
  frame's 20 padding leaves room for every ring.
- **Largest text size** (1.5×). The frame grows downward; the title takes
  three lines and the message six before Details takes the rest; fields
  are their line plus 14; buttons grow with their text and stay equal
  width; chooser labels wrap to two lines; Force Quit's rows are their
  line plus 12. Icons and the code cells keep their size.
- **Hit targets.** Buttons `size-control` by at least 80; checkbox rows
  `size-row` with the label clickable; chooser columns 84 × 82; Force Quit
  rows `size-row`. All above `size-hit-minimum`.

## Board

At 3440×1440 with the 1920×1080 screen beside it, dark first, each scene
also in light, over windows. Crops: prompts `[920, 0, 2520, 900]`, the
session dialog and Force Quit `[920, 320, 2520, 1120]`.

1. **Polkit**: Settings asking (icon and badge); `pkexec` in kitty with
   Details open; no app known; three administrators with the pop-up open.
2. **The armed moment**: frames at 0, 300, 600 and 760 ms.
3. **Polkit states**: Checking, wrong (shake at 0, 50, 100%), locked out,
   fingerprint waiting.
4. **Keyring**: unlock with its choice; new with "The passwords don't
   match."; confirm.
5. **Network**: Wi-Fi; Wi-Fi wrong (old password selected); 802.1X; VPN
   with a plugin's three fields.
6. **Bluetooth**: each of the five kinds; display passkey with three of
   six digits typed.
7. **Queue**: "2 more requests"; the swap to the next in three frames.
8. **Set aside**: the frame leaving as the island's prompt face grows;
   crop `[920, 0, 2520, 400]` for the face.
9. **Over Control Centre**: Wi-Fi asked from the pinned panel.
10. **Many screens**: a prompt on the 1920 screen, scrim on both, whole.
11. **Session dialog**: chooser with countdown at 42 s; restart
    confirmation; owed restart; quitting with three icons; interrupted;
    not responding.
12. **Force Quit**: a not-responding app selected; two selected; the
    sheet; a row that couldn't be force quit.
13. **Windowed game**: polkit solid over a game.
14. **Settings**: scenes 1, 5 and 11 under Increase Contrast, Reduce
    Transparency and the largest text.
15. **Motion**: appear, dismiss, queue swap, Details opening, set aside;
    live on the harness spring and as strips, with Reduce Motion and Game
    Mode.

## Choices

1. **The frame.** (a) Glass over the scrim; (b) a solid `colour-window`
   dialog over the scrim. **Recommended: (a).** A solid dialog is an app's
   alert in everything but place; glass is the shell's, no window can draw
   it, and it sits with the island, banners and Spotlight.
2. **Showing the armed moment.** (a) The action takes its accent when
   armed; (b) no visible change; (c) the action disabled until armed.
   **Recommended: (a).** The change tells a fast typist why Return did
   nothing; disabled would read as a refusal.
3. **Force Quit's sheet default.** (a) Cancel, by `window-chrome.md`'s rule
   that a destructive button is never default; (b) Force Quit, as macOS.
   **Recommended: (a).** Return then cannot end an app twice by accident;
   a click or Tab and Space confirms.

## New tokens

| Group | Name | Value |
|---|---|---|
| `size` | `button-minimum` | 80 (equal-width dialog buttons) |
| `size` | `code-cell` | 32 (a pairing code's digit cell) |
| `timing` | `prompt-arm-ms` | 600 |
| `timing` | `prompt-gap-ms` | 200 |
| `timing` | `session-countdown-ms` | 60000 |
| `limit` | `prompt-lines` | 4 (the message, 6 at the largest text) |
| `limit` | `session-apps` | 12 |
| `limit` | `force-quit-rows` | 10 |

Shared with `lock.md`: `motion-shake-ms`, `motion-shake-distance`. Phosphor
glyphs to vendor for the boards: `sign-out`, `moon-stars`,
`arrow-clockwise`, `fingerprint`, `mouse`, `device-mobile`, `speaker-hifi`.
