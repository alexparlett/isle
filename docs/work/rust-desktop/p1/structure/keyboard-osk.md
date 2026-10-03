# On-screen keyboard

Batch 3. Answers to: `shell.md` For the design process (On-screen
keyboard: its layout and where completions show) and the presentation side
of `shell.md` On-screen keyboard 1–4 and Lock screen 3 ("the on-screen
keyboard is available"); the Parity map's row (two cursors and legend
dropped); `compositor.md` Keyboard-shortcuts-inhibit and injected input 2;
`lock.md` (the bottom row's keyboard button); `control-centre.md`
(Accessibility module); `settings.md` (Accessibility); B21, B22, D37.

## Purpose

macOS's Accessibility Keyboard: a keyboard drawn on screen that a pointer
types with, into whatever has keyboard focus, without ever taking focus
itself (`shell.md` On-screen keyboard; D37). With B22 the pad-driven use
v1 built it for is gone; what remains is pointer typing, for a user who
cannot use a physical keyboard, or for the moment one is missing (a
dead keyboard at the lock screen). It is a shell surface, not an app. It
is not an input method or a candidate window for other scripts (Fcitx and
IBus are the system's), not a gamepad keyboard (Steam's, in Big Picture),
and not a touch keyboard that appears by itself on every text field
(`shell.md` On-screen keyboard Open).

## Content

```
 ┌──────────────────────────────── the screen's width ────────────────────────────────┐
 │                  ┌───────────────────────────────────────────────┐            ×  … │
 │                  │   suggestion   ·   suggestion   ·   suggestion  │                 │  strip
 │                  ├───────────────────────────────────────────────┤                 │
 │                  │ esc  1  2  3  4  5  6  7  8  9  0  -  =   ⌫   │                 │
 │                  │ ⇥   q  w  e  r  t  y  u  i  o  p  [  ]   \    │                 │
 │                  │ ⇪    a  s  d  f  g  h  j  k  l  ;  '    ⏎     │                 │  keys
 │                  │ ⇧     z  x  c  v  b  n  m  ,  .  /       ⇧    │                 │
 │                  │ `  🌐  ⌃  ⌥  ⌘      space       ⌘  ⌥  ← ↑↓ →  │                 │
 │                  └───────────────────────────────────────────────┘                 │
 └────────────────────────────────────────────────────────────────────────────────────┘
```

- **A panel docked to the bottom** of one screen, as wide as the screen,
  holding the keys in a centred block whose width is capped (stage 4), so
  the keys are the size of a real keyboard's on the ultrawide rather than
  stretched across it. The panel's full width is reserved (Placement).
- **Keys**: a Mac layout in the order above, with the legends of the
  active XKB layout (a German layout shows its own letters), modifiers as
  ⌃ ⌥ ⌘ ⇧ whatever the physical keyboards' profiles are, and 🌐 to switch
  layouts when more than one is set. The key left of 1 on the layout
  (`` ` `` on a US layout) leads the bottom row, so a backtick and the
  dropdown terminal's Ctrl+Option+`` ` `` can be typed. An optional
  function row (F1 to F12, Escape moving up to it and that key taking
  Escape's place) is off by default (Settings, below).
- **Modifiers latch**, as macOS's Sticky Keys and Accessibility Keyboard
  do: one click holds a modifier for the next key, two clicks lock it until
  clicked again. A held or locked modifier is marked. Caps Lock is marked
  while on.
- **The completion strip** (`shell.md` proposal: a strip), along the top of
  the key block: up to three words completing the word being typed on the
  keyboard, most likely in the middle, as macOS's typing suggestions. A
  click replaces the partial word with the suggestion and a space. Words
  come from the active layout's language list, ranked by frequency. The
  keyboard keeps no record of what was typed: it learns nothing and
  stores nothing, since it types passwords too. The strip keeps its height
  when empty, so the keys never move (brief P5).
- **Close** (×) and **More** (…) at the panel's trailing end: More holds
  Show function keys, the layouts, and Accessibility Settings… (the
  keyboard's own rows).

### What it types into

Keys go to whatever holds keyboard focus by `competition.md`'s order:
the lock's field, a system prompt, Spotlight, a menu-bar surface's field,
a sheet, the focused window. They enter as Isle's injected-key source,
aimed at that target, never translated (B21; `compositor.md` injected
input 2). A ⌘ chord typed on the keyboard is resolved the way the Mac
keyboard's own Cmd is for the app in front, so ⌘C copies in any app, and
the desktop's chords act on the desktop: ⌘ locked with Tab clicked twice
moves two apps along in the switcher, and unlocking ⌘ commits. With
nothing focused, the keys do nothing and the strip says "Nothing to type
into".

Completions are off where Isle knows a password is being typed: the lock,
the greeter, password prompts, Keychain's fields, and any field whose app
declares it a password through text-input-v3. The strip is then empty.

### Placement

`shell.md` On-screen keyboard 3: it covers nothing.

- **The work area** of its screen ends above the panel while it shows
  (`window-chrome.md`, Work area; `competition.md`: the bottom edge
  belongs to windows, and this is the one thing that takes it). Filled
  and zoned windows resize to the smaller area; a floating window whose
  bottom would be covered moves up, shrinking only if it no longer fits,
  and goes back when the keyboard closes unless it was moved meanwhile.
  Things placed against the work area's bottom (the capture toolbar, the
  capture thumbnail, picture-in-picture) sit above the panel.
- **Over a full-screen window** it overlays the window's bottom: a
  full-screen surface is not reflowed. Over a full-screen game it does the
  same, in its solid form; a game holding the pointer cannot be typed into
  with it.
- **One screen at a time**: the screen of the focused window when it
  opens. It stays there until closed; More offers Move to {display} when
  there are two.

### On the lock and the greeter

The lock's bottom row carries the keyboard's button when Settings turns
it on (`lock.md`). There the lock itself draws the keyboard, in its own
layer, at the bottom of the screen with the face, the field and the
bottom row moved up above it; it types into the field, with no
completions. Dozing hides it with the field; a wake brings both back. The
greeter does the same, from the setting the session last published
(`lock.md`, Greeter). The keyboard open in the session stays closed over
the lock: the lock has its own.

### Settings

Settings › Accessibility, in a Keyboard group (`settings.md`): On-screen
keyboard (on or off; the same state as Control Centre's switch), Word
suggestions (on), Show function keys (off), On the lock screen (off: shows
the button on the lock and the greeter). Showing it by itself when a text
field takes focus waits on text-input-v3's reliability (`shell.md` Open)
and is not drawn until its packet measures it.

## States

| State | What shows |
|---|---|
| Closed | Nothing; the work area is whole |
| Open | The panel at the bottom of its screen, the work area above it |
| Typing a word | Up to three suggestions in the strip |
| Password field | The strip empty |
| Modifier held, locked | The key marked, once or locked |
| Nothing focused | Keys inert; the strip says "Nothing to type into" |
| A prompt or Spotlight open | The keyboard above them, typing into them |
| Full-screen window | Over its bottom; nothing reflowed |
| Game in front, full screen | Over its bottom, solid, no transitions |
| Game in front, windowed | As any window: the work area shrinks |
| Second screen | On one screen; the other's work area is whole |
| Locked | The session's keyboard hidden; the lock's own when its button is used |
| Focus on | No change |
| Reduce Motion | Fades in and out instead of sliding |
| Reduce Transparency | Solid panel |
| Increase Contrast | Key borders; the strip's divider |
| Largest text size | Legends and suggestions grow, the panel with them, up to a cap on its share of the screen's height (stage 4); past it the legends stop growing |

## Opening and closing

- **Opens** from ⌥⌘F5, macOS's accessibility shortcut, as a toggle (P5's
  chord table); Control Centre's Accessibility module; Settings ›
  Accessibility; Spotlight's action "Show on-screen keyboard"; and the
  lock's button (`shell.md` On-screen keyboard 4).
- **Closes** from the same toggles and its own ×. Escape is never its: the
  keyboard has no focus, and Escape typed on it goes to the target.
- **Motion**: the panel rises from the bottom edge it is docked to, while
  windows take the smaller work area with the compositor's resize motion;
  closing reverses it, faster (brief P5). Nothing else moves.
- It stays open across focus changes, app switches and Mission Control,
  until closed. Locking hides it; unlocking brings it back.

## Keyboard

The keyboard is a pointer tool and takes no keys. Everything about it is
also reached without it: ⌥⌘F5 toggles it, and its options are rows in
Settings › Accessibility. Keys clicked on it behave as the same keys
pressed on a Mac keyboard, including the desktop's chords.

## Pointer

- **Press** sends key down, **release** key up, so holding Delete, Space or
  an arrow repeats at the keyboard repeat rate. Holding a letter with
  accented forms opens them above the key, as macOS's press and hold; a
  release on one types it.
- **Hover** marks the key under the pointer and opens nothing. Dwell
  (hover to press) is left out: not in the intent.
- **Clicks on the panel never take focus, never raise a window, and never
  count as a click outside another surface**: Spotlight, a pinned Control
  Centre or an open menu stays open while it is typed into.
- Secondary click does nothing on keys; on the panel's margin it opens
  More.
- The keys stop short of the screen's bottom corners, so a click on a key
  never rests in a hot corner.

## Shared places

| Place or input | Competes with | Proposed winner |
|---|---|---|
| The bottom of the work area | Windows (`competition.md`: the bottom edge is windows') | The keyboard, while open: the work area ends above it, windows reflow, nothing is covered. *Added for `competition.md` Bottom edge* |
| Bottom centre | The capture toolbar | Both: the toolbar sits above the keyboard |
| Bottom-trailing corner of the work area | Capture thumbnail; picture-in-picture | Both stay at the work area's corner, which is now above the keyboard (`pip.md`) |
| Bottom corners | Hot corners | Hot corners keep the corner points; keys stop short of them |
| Layers | The overlay (Spotlight and the rest) and a system prompt's scrim | The keyboard is drawn above the overlay and above a prompt's scrim, so it can type into either, and under the lock and the idle dim. *Added for `competition.md` Layers* |
| A click on it | Surfaces that close on a click outside | The keyboard: its clicks are never outside clicks |
| Keyboard focus | Every focus holder | None: the keyboard never takes focus and types into whoever holds it. Not added to `competition.md`'s focus order |
| The lock's bottom | The lock's bottom row and field | The lock's own keyboard at the bottom, the face moved above it (`lock.md`) |
| ⌥⌘F5 | Apps' own bindings | The desktop, for P5's chord table; not in the game escape set |

## Answers

- **What remains after B22.** **Decided**: macOS's Accessibility Keyboard,
  for pointer typing in the session, on the lock and at the greeter. v1's
  two cursors, pad legend and pad search are gone with the pad (Parity
  map).
- **Its layout** (`shell.md`). **New**: a Mac layout with the active XKB
  layout's legends, latching modifiers, an optional function row, docked
  full width at the bottom with the keys in a capped, centred block.
- **Where completions show** (proposal: a strip). **Adopted**: a strip of
  up to three along the top of the keys, empty for passwords, learning
  nothing.
- **Placement** (`shell.md` On-screen keyboard 3). **Adopted** as a work
  area reserve on its screen; over a full-screen window it overlays.
- **On the lock** (`shell.md` Lock screen 3; `lock.md`). **New**: the
  lock draws its own, with the face moved up, typing into the field.
- **How it opens** (`shell.md` On-screen keyboard 4). **New**: ⌥⌘F5, Control
  Centre, Settings, Spotlight and the lock's button.

## Copy

| Where | String |
|---|---|
| Name | "On-screen keyboard" (Control Centre, Settings, Spotlight's action "Show on-screen keyboard", "Hide on-screen keyboard") |
| More | "Show function keys", "Hide function keys", "Move to {display}", "Accessibility Settings…" |
| Strip | "Nothing to type into" |
| Settings | "On-screen keyboard", "Word suggestions", "Show function keys", "On the lock screen" |
| Accessible names | "Close on-screen keyboard", "More", each key by its character or name ("Command", "Option", "Control", "Shift", "Caps Lock", "Delete", "Return", "Tab", "Escape", "Space", "Next layout"); a latched modifier "{key}, held" or "{key}, locked" |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

None. The intent settles what it is for and that it covers nothing; the
rest follows macOS's Accessibility Keyboard.
