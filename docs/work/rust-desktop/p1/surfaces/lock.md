# Lock, greeter and idle dim

P1 stage 5, wave B. How the lock, the greeter and the idle dim look and
move, in Frost, from `structure/lock.md` (binding). Values are `tokens.kdl`
names; numbers not yet tokens are listed at the end.

## Anatomy

### Backdrop and the lock's wash

Every screen: the wallpaper's softened form (`wallpaper.md`), then the
lock's own wash over it, `material-lock-wash` (dark `rgba(16,17,20,0.30)`,
light `rgba(246,246,248,0.20)`: the softened form's colour again). The
softened form promises no contrast; with the wash on it, the backdrop
meets the glass rule, so the date, time and anything else drawn straight
on it use `colour-text` and `colour-text-2`, never `colour-text-3`
(Accessibility). No text shadow: the wash is what `contrast.py` can check.

Everything else on the face sits on **lock plates**: `material-glass` at
the user's glass position, `material-noise-opacity`, `shadow-menu`, and no
backdrop blur pass, since the backdrop beneath is already blurred and holds
no shapes. A menu opened from a plate is `material-solid` (`menus.md`).

### The face, top to bottom

Centred on the screen. Positions are from the screen's edges, so the face
sits the same on the ultrawide and the 1920×1080 screen.

| Part | Look |
|---|---|
| Date | Top at `size-strip` + `space-xxxl` (80), below the island's privacy form and Level OSD (which end at 57). `type-style-title-2`, `colour-text`, "Saturday 3 October" in the locale's long form. The current Focus's symbol leads it at `size-glyph` in Fill, `space-s` before the text |
| Time | `space-xs` under the date. **`type-style-display`** (new: 96, semibold, line 104), Inter's display optical size, tabular figures, `colour-text`, tracking −0.02 em. Its box is the format's widest string ("88:88"), text centred, so nothing moves as the minutes change |
| Column | Top `space-xxxl` under the time; the notification card's width, `size-panel` less 2 × `space-s` (360); plates `space-s` apart. Now Playing first, then notifications, newest first |
| Now Playing | A plate 360 × 72, `radius-panel`, padding `space-m`: art 48 at `radius-control` (concentric: 18 less 12); `space-m`; title `type-style-headline` `colour-text` over artist `type-style-subheadline` `colour-text-2`, one line each, cut at the end; trailing, `skip-back`, `play` or `pause` (Fill), `skip-forward` at `size-glyph` as plain icon buttons (`size-control`), `space-xxs` apart. No art: a `colour-fill-strong` square with `music-note` 20 |
| Notification, by app (default) | A plate 360 × 56, `radius-panel`, padding `space-m`: the app's icon at `size-app-icon` (32); `space-m`; "{App}" in `type-style-headline` `colour-text` over "{n} notifications" in `type-style-subheadline` `colour-text-2` |
| Notification, previews | The notification card as `surfaces/notifications.md` draws it, summary only, no body or actions, at 360 |
| More | A plate 360 × `size-row` (28), "{n} more" centred in `type-style-callout` `colour-text-2`. Shown past `limit-lock-notifications` (5) or past what fits above the user, whichever is fewer |
| The user | Picture `size-avatar` (64), a circle with a `stroke-hairline` inner edge in `colour-hairline`; `space-s`; full name in `type-style-title-3` `colour-text`, one line. No picture yet: a `colour-fill-strong` circle with `user` 32 (Light) in `colour-text-2` |
| Password field | `space-m` under the name; its vertical centre at 80% of the screen's height. A capsule `size-lock-field` (240) × `size-control-large` (30), radius 15, a lock plate. Inset `space-m`. Placeholder "Enter password" or "Password or fingerprint" in `type-style-body` `colour-text-2`. Typed: dots 6 across in `colour-text`, `space-xs` apart, from the leading inset, the newest kept in view. The caret is the field's focus mark. Trailing, inset `space-xs`: `fingerprint` at `size-glyph` in `colour-text-2` while the reader listens |
| Status line | `space-s` under the field, a reserved slot of 16 (the line of `type-style-callout`), centred. Each message leads with a glyph at `size-glyph-small`, `space-xs` before the text, so none is told by colour: `warning-circle` (wrong, locked out, PAM error, restart stopped), the spinner component at 12 (checking), `arrow-fat-line-up` (Caps Lock), `fingerprint` (the reader). Text `colour-text` for what needs acting on (wrong, locked out, errors, Caps Lock), `colour-text-2` for checking and the reader |
| Power row | Bottom centre, its labels' baseline `space-xxxl` above the screen's bottom. Sleep (`moon-stars`), Restart (`arrow-clockwise`), Shut Down (`power`): each a lock-plate circle `size-lock-button` (40) with its glyph at `size-glyph-large`, Regular, `colour-text`; the label `space-xs` under it in `type-style-callout` `colour-text-2`; columns `space-xxl` apart |
| Utilities | The same row's trailing end, inset `space-xxxl`, centred on the circles, `space-s` apart, each a lock-plate capsule `size-control-large` (30) tall, radius 15, padding `space-m`: the layout menu (short name in `type-style-headline`, `caret-up` 12; its menu opens upward), the on-screen keyboard (`keyboard` 16 in a 30 circle), the battery (two-layer level glyph as the island's, then the percentage in `type-style-callout`, tabular). At the greeter the session menu leads them: the session's name and `caret-up` 12 |

**Wrong-layout mark.** After a wrong password with several layouts, the
layout capsule takes the tray's attention dot (`island.md`): 6 points of
`colour-accent` ringed 1 point in `material-solid`, on its top trailing
corner, until the next unlock attempt or a layout change.

**Confirm in place.** Restart or Shut Down replaces the three circles with
one lock-plate capsule 40 tall, radius 20, centred where they were: inset
`space-l`, "Restart now?" in `type-style-body` `colour-text`, `space-l`,
Cancel (control) and Restart (default, `colour-accent`) at `size-control`,
`space-s` apart. The labels' row stays reserved and empty. Once confirmed,
the capsule holds the spinner at 12, `space-s`, and "Quitting apps…" in
`type-style-body` `colour-text`, in place of the question and buttons, until
the session ends or an app stops it (status line).

**The lock's keyboard.** The utilities' keyboard button shows the lock's
own on-screen keyboard; how it is drawn, and how the face moves up by its
height, is `keyboard-osk.md` (On the lock and the greeter).

**Greeter's user list** (two or more users): in the user's place, pictures
at `size-avatar` in a row, `space-xxxl` apart, each name under it in
`type-style-body`, `colour-text-2`; the hovered or keyboard-selected name
in `colour-text`. Choosing one collapses the row to that user and shows
the field.

### The minimal lock

No picture, plates or glass: a full screen of `colour-window-sunken`; the
time in `type-style-display` `colour-text` at the face's place; "The lock
screen had a problem." in `type-style-callout` `colour-text-2`, `space-m`
above the field; the field as the window's text field component (240 ×
`size-control-large`, `colour-window-raised`, `colour-hairline-strong`
edge); the status line as above. Window roles throughout, so `contrast.py`
already covers it. It cuts in; nothing on it moves.

### Idle dim

A layer over everything on a screen, lock and prompts included, of
`colour-dim` (`rgba(0,0,0,0.50)` in both appearances: it stands in for
backlight, so it does not change with the theme). It holds no content and
takes no hit target but the one input that ends it.

## States

| State | What changes |
|---|---|
| Locking | First frame: backdrop and wash on every screen; the face fades in (Motion). |
| Awake | The whole face. |
| Dozing | The user, field, status line, power row and utilities fade out; the pointer hides. Date, time, Now Playing and notifications keep full contrast: dozing removes, it never dims below the promise. |
| Typing | Dots; Caps Lock's message while it is on. |
| Pending | Dots kept, the caret gone; spinner and "Checking", after `timing-check-long-ms` "Still checking"; the fingerprint glyph stays. |
| Wrong password | The shake; the dots go with it; the field takes its error edge (`colour-danger`, 1 point, inside) until the next key; the message with its count; the layout mark if several layouts. |
| Locked out | Field disabled: placeholder `colour-text-disabled`, no caret; "Too many attempts. Try again in {n} min."; the fingerprint glyph and the power row unchanged. |
| Fingerprint | Glyph while listening; "Not recognised. Try again." for `timing-event-ms`; at the limit the glyph in `colour-text-disabled` and "Use your password" until the next wake. |
| Unlocking | Face out, then the backdrop dissolves (Motion). |
| Empty | Date, time, the user and field; the column absent, nothing reserved for it. |
| Loading | The login name and the placeholder circle; Now Playing and notifications fade in (`motion-fade-ms`) as they answer, the column growing downward only. |
| Error: lock panics | Rebuilt in the next frame as it was, dots cleared; after repeated panics, the minimal lock. |
| Error: compositor restarts | First frame the backdrop and wash; the face as Locking. |
| Error: PAM | "Couldn't check the password. Try again." with `warning-circle`; field cleared. |
| Many screens | The face on the pointer's screen; on the others the backdrop, date and time only. |
| Game in front, full-screen window | As Awake; the backdrop is the wallpaper's, never the game's frame. |
| A prompt showing, a capture running | Nothing on the face; the island's privacy form over it. |
| Focus on | Its symbol before the date; the column filtered. |
| On-screen keyboard shown | The face moved up by the panel's height (`keyboard-osk.md`). |
| Restart or Shut Down asked | Confirm in place; once confirmed, the spinner and "Quitting apps…"; after an app stops it, "Couldn't restart because an app didn't quit. Unlock to see it." |
| Greeter | No column, no island; the session capsule when several sessions; the user list when several users. |
| Greeter, fingerprint sign-in | PAM's message with `fingerprint`; a typed password's dots kept, and "Waiting for the fingerprint reader" on an early Return. |
| Greeter, session failed | "Couldn't start {session}." with `warning-circle`, field ready. |
| Dimmed | `colour-dim` over the screen. |
| Display off, idle held | Nothing drawn. |
| Reduce Motion, Transparency, Contrast, largest text | Motion and Accessibility. |

## Motion

The compositor draws all of it (B7). Nothing on the lock scales or zooms
(brief P5); every part fades in place with its parent.

| Transition | Form |
|---|---|
| Lock in | Backdrop and wash complete in the first frame on every screen (a cut: no frame of the session after the request). The face's alpha 0 → 1 over `motion-fade-ms`, all parts together. |
| Unlock | The face's alpha 1 → 0 over `motion-exit-fade-ms`; from its halfway point the backdrop's alpha 1 → 0 over `motion-exit-fade-ms`, uncovering the session already drawn beneath: the softened form clearing to the plain one. Each half is quicker than its entrance. No blur ramp. |
| Doze | The lower parts' alpha 1 → 0 over `motion-exit-fade-ms` after `timing-doze-ms` with the field empty. |
| Wake | Back over `motion-fade-ms`; a printable key's dot is drawn in the first frame, under the fading-in field. |
| Face to another screen | Out on the old screen over `motion-exit-fade-ms`, in on the new over `motion-fade-ms`, dots kept; date and time do not move. |
| Shake | The field (its fingerprint glyph with it) moves along x only: a sine of two cycles over `motion-shake-ms` (320), amplitude `motion-shake-distance` (6) decaying linearly to 0, ending exactly at rest. The dots fade over the first half. Nothing else moves. |
| Status line | A message replaces the last in one frame; its slot never changes size. |
| Column grows or shrinks | A plate arriving fades in at its place over `motion-fade-ms`; the plates below move on `motion-move`; one leaving at unlock goes with the face. |
| Confirm in place | Circles out over `motion-exit-fade-ms`, the capsule in over `motion-fade-ms`, crossing in place; Cancel the reverse. Once confirmed, "Quitting apps…" replaces the question and buttons in one frame, the capsule's width on `motion-move`. |
| Greeter: user chosen | The others fade out over `motion-exit-fade-ms`; the chosen picture moves to the centre on `motion-move`; name and field fade in after. Escape the reverse. |
| Greeter to session | The face out as Unlock; the backdrop stays until the session's compositor clears its own softened first frame. |
| Idle dim | `colour-dim`'s alpha 0 → 1 over `motion-dim-ms` (2000), linear, so it reads as a warning; any input cuts it to 0. |
| Dim to display off | No motion of its own; outputs go off under the dim. |

**Reduce Motion.** No shake: the error edge and the message carry it. The
picture in the greeter does not travel: old and new rows crossfade over
`motion-reduced-fade-ms`. Lock in, unlock, doze, wake and the dim stay
fades at their own lengths, since a fade moves nothing; plates arriving
cut into place, those below them jump.

**Game Mode.** Lock in and unlock are cuts; no shake (as Reduce Motion);
doze and wake are cuts. The plates already skip the blur pass. The dim
still fades, since it is one layer.

## Accessibility

- **Colour roles.** On the washed backdrop and on lock plates, only
  `colour-text` and `colour-text-2`, by the glass rule: with the wash, text
  passes 4.5:1 over a white, grey and black picture (dark 4.69 at worst,
  over white), text-2 over grey and the appearance's own extreme (dark 5.16,
  light 5.09). `colour-text-3` appears nowhere on the lock or greeter.
  Status meaning is by glyph and wording; colour on the face sits only in
  the solid-ringed dot and the field's error edge, both repeated in words.
  The minimal lock uses window roles.
- **Increase Contrast.** The wash steps to `material-lock-wash-contrast`
  (dark 0.45, light 0.30), where every role passes everywhere (text-2's
  contrast form 5.18 at worst); plates take `material-glass-tinted` and a
  `stroke-hairline` edge of `colour-hairline-strong`, the field and
  capsules likewise; the power circles get the same edge; glyphs step to
  `icon-weight-contrast`; the status line's `colour-text-2` messages move to
  `colour-text`.
- **Reduce Transparency.** The backdrop is unchanged (a picture); plates,
  the field, circles and capsules are `material-solid`.
- **Focus ring.** `stroke-focus-ring` in the accent at `stroke-focus-offset`,
  for keyboard focus only: a capsule round the field when Tab returns to it
  (the caret alone marks it at rest), circles round the power buttons and
  the greeter's pictures, capsules round the utilities, `radius-panel` plus
  the offset round a plate. Tab order is the structure's.
- **Largest text size** (1.5×). Date, name, labels, the field's
  placeholder and dots, the status line, plates' text and capsules grow; the
  field is its line plus 14 tall and 360 wide; plates grow downward and the
  column shows fewer before "{n} more". The time keeps its size.
- **Hit targets.** Power circles 40 and their labels; capsules 30; Now
  Playing's buttons `size-control`; plates whole; the field whole. All
  above `size-hit-minimum`.
- **Never colour alone.** Caps Lock, wrong, checking and the reader each
  have their glyph; the layout mark is backed by the message.

## Board

At 3440×1440 with the 1920×1080 screen beside it, dark first, each scene
also in light. Crops: face top `[920, 0, 2520, 720]`, face bottom
`[920, 720, 2520, 1440]`, utilities `[1840, 1240, 3440, 1440]`, the second
screen whole.

1. **Awake**, Frost backdrop: Do Not Disturb by the date, Now Playing, three
   apps by count; whole screen and both crops.
2. **The wash**: scene 1 over Ridge, Night side and a near-white user
   photo, each with and without the wash, to judge the promise.
3. **Previews**: the column with four preview cards and "3 more".
4. **Empty** and **Loading** (no picture, column arriving).
5. **The status line**: Caps Lock, Checking, Still checking, wrong with
   count, locked out, PAM error, each as a face-bottom crop.
6. **Wrong password**: shake at 0, 25, 50, 100%, the layout mark shown.
7. **Fingerprint**: listening, not recognised, at the limit.
8. **Power**: confirm in place; "Quitting apps…"; restart stopped.
9. **Dozing** beside Awake, same backdrop.
10. **Many screens**: the face on the ultrawide, date and time on the
    1920 screen; then moved.
11. **Island over the lock**: privacy form and a volume OSD; crop
    `[920, 0, 2520, 300]`, showing they clear the date.
12. **Minimal lock**, dark and light.
13. **Greeter**: one user; three users; session menu open; fingerprint
    waiting; session failed.
14. **Idle dim**: a desktop, the lock, and a prompt, each at 0, 50, 100%.
15. **Settings**: scene 1 under Increase Contrast, Reduce Transparency and
    the largest text.
16. **Motion**: lock in, unlock, doze and wake, shake, user chosen, dim;
    live on the harness spring and as strips, each with Reduce Motion and
    Game Mode.

## Choices

1. **Text over the softened form.** (a) The lock's own wash, checked by
   `contrast.py`; (b) the large time with a soft shadow and the rest on
   plates; (c) everything, the time included, on glass plates.
   **Recommended: (a).** A shadow cannot be checked and still fails over a
   pale picture; a plate behind 96-point digits is a box on the lock.
2. **The shake under brief P5.** (a) Kept: two decaying cycles of 6
   points on the field only; (b) no shake, the error edge alone.
   **Recommended: (a).** It is feedback that answers a key, not a
   transition into a place, and it ends where it began; the structure asks
   for it, macOS has it, and Reduce Motion and Game Mode remove it.

## New tokens

| Group | Name | Value |
|---|---|---|
| `type` | `style.display` | size 96, weight 600, line 104 (the lock's time) |
| `material` | `lock-wash` (per appearance) | dark `rgba(16,17,20,0.30)`; light `rgba(246,246,248,0.20)` |
| `material` | `lock-wash-contrast` (per appearance) | dark `rgba(16,17,20,0.45)`; light `rgba(246,246,248,0.30)` |
| `colour` | `dim` (both appearances) | `rgba(0,0,0,0.50)` |
| `size` | `avatar` | 64 |
| `size` | `lock-field` | 240 |
| `size` | `lock-button` | 40 |
| `motion` | `shake-ms` | 320 |
| `motion` | `shake-distance` | 6 |
| `motion` | `dim-ms` | 2000 |
| `timing` | `doze-ms` | 12000 |
| `timing` | `check-long-ms` | 5000 |
| `timing` | `lock-dim-ms` | 45000 (dim once locked) |
| `timing` | `lock-off-ms` | 60000 (display off once locked) |
| `limit` | `lock-notifications` | 5 |

`contrast.py` gains the lock's backdrop (softened form plus wash over black,
grey and white) as a glass-rule surface. Phosphor glyphs to vendor for the
boards: `moon-stars`, `arrow-clockwise`, `fingerprint`, `arrow-fat-line-up`.
