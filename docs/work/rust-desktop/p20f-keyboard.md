# P20f · On-screen keyboard

Planned 2026-11-05, for review.

macOS's Accessibility Keyboard for Isle: a keyboard drawn at the bottom of
a screen that the pointer types with, into whatever has focus, without ever
taking focus, in the session, on the lock and at the greeter (`shell.md`
On-screen keyboard; P1's `keyboard-osk.md`, structure and surface).

Today the compositor latches Sticky Keys and Slow Keys (P5 choice 19), but
nothing draws a latched modifier. The `on-screen-keyboard` shortcut is in
the table on ⌥⌘F5 with nothing behind it. Control Centre's Accessibility
module and Settings › Accessibility have no keyboard rows, and the lock's
utilities have no keyboard button.

What P20f must get right: a click on the keyboard never moves focus, never
raises a window and never closes the surface being typed into; a key typed
on it behaves as the same key on the Mac keyboard, chords included; and
it covers nothing but a full-screen window's foot.

## What it answers to

- Decisions: B21 (Cmd translation; Isle's injected keys), B22 (no pad
  driving), B26 (the greeter mode), B30 (the escape set, the game in
  front), B141 (the look the session publishes for the greeter).
- Intent: `shell.md` On-screen keyboard 1–4 and its open point, Lock
  screen 3; `compositor.md` Keyboards and injected input 2; `apps.md`
  Settings (Accessibility).
- P1: `p1/structure/keyboard-osk.md` and `p1/surfaces/keyboard-osk.md`
  (binding); `p1/structure/lock.md` (the bottom row's button);
  `p1/structure/control-centre.md` (Accessibility);
  `p1/structure/settings.md` (Accessibility's Keyboard group).
- Packets: P5 choice 19 (Sticky Keys and Slow Keys in the engine, drawn
  here); P9 (the lock's face and field); P14b (the greeter); P20a (menus'
  grab, PiP's bottom corners).
- Evidence read 2026-11-05: the host has `/usr/share/dict` word lists
  (American and British English among them), unranked, and one layout
  (`gb`). The compositor serves no `text-input-v3`.

## Choices this plan makes

### Scope

1. **P20f holds** the keyboard in the session, on the lock and at the
   greeter; its completions; its rows in Settings and Control Centre and
   its Spotlight action; and a display of Sticky Keys' latched modifiers
   while the keyboard is closed. Not showing it by itself when a text field
   takes focus (choice 18).

### Typing

2. **The keyboard is a keyboard device of the engine's own,** `isle-osk`,
   on the Mac profile: its keys go through the engine like the Mac
   keyboard's, so ⌘C copies in any app through Cmd translation, the
   desktop's chords act, and Sticky Keys' latch is the same latch. It is
   never slowed by Slow Keys, and a press counts as activity for idle.
   This takes P1's "resolved the way the Mac keyboard's own Cmd is" over
   `compositor.md` Injected input 2's "untranslated", which stays true of
   emoji and paste. Rejected: the injected source, where ⌘C would reach a
   terminal untranslated.
3. **Press is key down, release is key up,** so a held key repeats at the
   keyboard repeat rate, and a key goes to whoever holds focus by
   `competition.md`'s order: the lock's field, a prompt, Spotlight, a
   sheet, the focused window. With nothing focused the keys send nothing
   and the strip says "Nothing to type into".
4. **Its modifiers always latch,** whether or not Sticky Keys is on: one
   click holds for the next key, two lock, a third releases. The latch is
   the engine's, so a modifier latched from a physical keyboard with Sticky
   Keys on shows on the keyboard, and the reverse.
5. **Legends from the seat's active layout.** Each key's label is the
   layout's keysym at the shift level the latched modifiers and Caps Lock
   give, through xkbcommon, as macOS's keyboard viewer. The globe key
   shows only with two or more layouts and steps the layout. Caps Lock's
   light follows the seat.
6. **Press and hold on a letter** opens its accented forms above it after
   the repeat delay, instead of repeating, as macOS: macOS's set for the
   letter, shipped as a table. A release on a cell types it, through the
   layout's keys when the layout has it, else as Isle's paste of the
   character (P10's path).

### Completions

7. **Words come from a shipped ranked list.** `data/words/en.txt`, the
   most frequent fifty thousand English words, from a frequency list under
   CC BY-SA 4.0 credited in the package. Another layout's language uses
   its `/usr/share/dict` list unranked, shorter words first, and no list
   gives no suggestions. Matching is a prefix search in the sorted list.
8. **The word under way is the keyboard's own,** the letters typed on it
   since the last space, punctuation, arrow, click elsewhere or focus
   change. It reads nothing from the field. A suggestion is typed as
   Backspaces for the partial word, then the word and a space, through the
   layout's keys. Nothing is learned or stored.
9. **No suggestions for a password**: on the lock and the greeter, in
   Isle's prompts that take a secret (polkit, the keyring, Wi-Fi and VPN
   secrets), and in a field whose app says so through `text-input-v3`'s
   content purpose. The compositor serves `text-input-v3` for that purpose
   alone, with no input method behind it.

### The panel

10. **A shell surface drawn as P1's sheet draws it:** solid, docked full
    width at the bottom of one screen, the key block centred and capped,
    the strip, Close and More, the function row when asked for; latched
    forms in the accent with the lock bar; Caps Lock's dot; accessible
    names per key.
11. **Above the overlay and a prompt's scrim, under the lock and the idle
    dim.** Its pointer events never move keyboard focus, never raise a
    window, and are never a click outside: Spotlight, a pinned Control
    Centre and a menu chain (P20a's grab) stay open while it types into
    them. The keys stop short of the bottom corners' points.
12. **It takes the bottom of the work area** on its screen while open:
    filled and zoned windows resize to the smaller area; a floating window
    it would cover moves up, shrinking only if it no longer fits, and moves
    back on close unless it was moved meanwhile. The capture toolbar, the
    capture thumbnail and PiP sit above it. Over a full-screen window it
    covers the window's foot and nothing reflows; over a full-screen game
    likewise, with no transitions.
13. **One screen:** that of the focused window when it opens, until it is
    closed or More's Move to {display} moves it.
14. **Shown is a setting,** `accessibility.on-screen-keyboard`, so it
    comes back after a restart and a sign-in, as macOS's. Locking hides it
    and unlocking shows it again.

### Opening

15. **⌥⌘F5 toggles it** (the `on-screen-keyboard` shortcut, already
    bound), not in the escape set; Control Centre's Accessibility module
    gains an On-screen keyboard switch; Spotlight gains "Show on-screen
    keyboard" and "Hide on-screen keyboard"; Settings › Accessibility's
    Keyboard group gains On-screen keyboard, Word suggestions, Show
    function keys and On the lock screen, with P1's copy.

### The lock and the greeter

16. **The lock draws its own keyboard** when On the lock screen is on: a
    keyboard button in the utilities' row, the panel as a lock plate with
    no strip and no More, the face moved up by its height, typing into
    the field through the same device. Dozing hides it with the field.
    The session's keyboard stays hidden under the lock.
17. **The greeter reads On the lock screen** from the look the session
    publishes (B141), so a sign-in can be typed with the pointer when the
    keyboard behind the KVM is missing.

### Left out

18. **Showing the keyboard when a text field takes focus is not built,
    and not measured.** P1 left it until `text-input-v3`'s reliability
    per toolkit was measured. The user types on two physical keyboards and
    opens this one by hand, so the measuring is not worth its phase.
    `shell.md`'s open point is answered "not built".

### Sticky Keys

19. **While the keyboard is closed, latched modifiers show in the
    island's rest pill,** as glyphs in the keyboard's held and locked forms
    (the accent fill; the bar for locked), in the tier that always shows.
    macOS draws them in the screen's top trailing corner, where Isle's
    banners stand. 20f.1 draws both on a board for the user's choice,
    since P1 never drew this.

### Proof

20. **Headless tests** in `isle-compositor`: a key from the device into a
    window, a prompt and Spotlight; ⌘C translated for a terminal and not
    for an Isle app; a desktop chord from latched ⌘ and Tab; the latch with
    Sticky Keys on and off, shared both ways; no Slow Keys; repeat on a
    held key; nothing focused; a click on the panel keeps focus, Spotlight
    and a menu chain; the work area reserve, the reflow and the return,
    full screen covered and not reflowed; the setting across a restart; the
    lock's keyboard into the field. In `isle-shell`: legends per layout and
    shift level, the globe with one and two layouts, the function row,
    latched forms, accents. Completions: the prefix search, the word
    under way and its resets, the typed replacement, no suggestions for a
    password purpose.
21. **Bars.** A key on its target within one frame of the press. A
    suggestion list within a millisecond of a key. The open motion holds
    the frame rate on the ultrawide with a game behind.
22. **The VM walk** with real input: typing into Files, a terminal,
    Spotlight and a polkit prompt; ⌘C and ⌘V; ⌘Tab by latching; a menu kept
    open while typing a letter; a filled window resized and a floating one
    moved and back; Chromium full screen; the lock's keyboard used to
    unlock; the greeter's; ⌥⌘F5, Control Centre and Spotlight; the Sticky
    Keys display.

## The workspace

- `isle-compositor`: `osk.rs` (the device, the latch shared with Sticky
  Keys, focus kept, the work area reserve and reflow, the setting,
  `text-input-v3`'s purpose), changes in `keyboard/`, `wm/geometry.rs`
  (the reserve), `control_edit.rs` (the switch), `spotlight.rs` (the
  actions), `lock_face.rs` (the lock's keyboard), `greeter_look.rs`.
- `isle-shell`: `osk/` (the panel, legends, latched forms, accents, the
  strip, More), the lock's plate and button in `lock/surface.rs`, the
  rest pill's modifier glyphs in `island/`.
- `isle-keys`: the completions (the list, the prefix search, the word under
  way) and the accent table.
- `data/words/en.txt` and its credit; the package installs it.
- `isle-settings`: the four keys; the shortcut loses its `P20` mark.
- `isle-system-settings`: Accessibility's Keyboard group.

## Phases

### 20f.0 · The note and the record

`shell.md` On-screen keyboard's open point answered and `compositor.md`
Injected input 2 narrowed (choice 2); `state.md`, `progress.md`, README.

### 20f.1 · The Sticky Keys board

The rest pill's glyphs and macOS's corner, drawn from the tokens with held
and locked modifiers, published for the user's choice.

### 20f.2 · The device

`isle-osk` in the engine on the Mac profile; the latch shared with Sticky
Keys; repeat; focus by `competition.md`'s order; nothing focused; legends
from xkbcommon; the globe. Tests.

### 20f.3 · The panel

The surface as P1 draws it: keys, legends, the function row, latched forms,
Caps Lock, accents, the strip, Close and More, accessibility. Tests.

### 20f.4 · Placement

The layer; pointer events that keep focus and are never outside; the work
area reserve and reflow; full screen and the game; one screen and Move to
{display}; the hot corners. Tests.

### 20f.5 · Completions

The shipped list and its credit; the dict fallback; the word under way;
the typed replacement; `text-input-v3`'s purpose and the password cases.
Tests.

### 20f.6 · Opening and settings

⌥⌘F5, Control Centre's switch, Spotlight's actions, Accessibility's four
rows, the shown setting, lock and unlock. Tests.

### 20f.7 · The lock and the greeter

The utilities' button, the plate, the face moved, the field, dozing; the
greeter from the published look. Tests.

### 20f.8 · Sticky Keys shown

As the board chose. Tests.

### 20f.9 · The bars and the walk

Choice 21's bars; choice 22's walk in the VM, the journal read after each
step.

### 20f.10 · The host and close

Package and install. With the user: typing a paragraph and a chord on the
ultrawide; the lock unlocked with it; the greeter with the KVM switched
away. Then the decisions from the next free number, ARCHITECTURE's How the
on-screen keyboard types, this sheet's outcome.

## Not in P20f

Showing it when a text field takes focus. Dwell clicking. Learning words.
Suggestions from the field's text. Input methods (Fcitx, IBus). A pad
driving it (B22). Ranked lists for languages other than English.

## Depends on, and feeds

- Needs P5 (the engine, profiles, Sticky Keys, Slow Keys, the shortcut
  table), P8 (Control Centre's module), P9 (the lock's face and field, the
  prompts), P10 (Spotlight's actions, the paste path), P13 (Settings ›
  Accessibility), P14b (the greeter's published look), P20a (the menu
  grab that its clicks must not end; PiP above it).
- Feeds P20g (the audit's keyboard line).

## Done when

- `cargo test` passes headless for choice 20.
- Choice 21's bars hold.
- Choice 22's walk passes in the VM.
- The host run passes with the user.

## Risks

- A keyboard device of Isle's own may be taken by the engine's per-device
  profile detection for a PC keyboard; it is pinned to the Mac profile by
  name.
- `text-input-v3` served with no input method may make some toolkits wait
  for an input method's reply; the walk watches GTK and Qt fields.
- A floating window moved up for the keyboard and resized by its app
  meanwhile may not return exactly.
- A layout without the letters of a suggestion types none of them; the
  suggestion is offered only when every letter has a key.
