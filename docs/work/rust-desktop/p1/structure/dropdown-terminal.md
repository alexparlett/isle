# Dropdown terminal

Batch 3. Answers to: `compositor.md` § For the design process (Dropdown
terminal); the dropdown's rows in `window-chrome.md` Shared places
(undecorated windows). B8, B21, B30.

## Purpose

A terminal one chord away, over whatever is in front, gone again on the
same chord: v1's `isle-dropdown`, kept running when hidden
(`isle-v1-inventory.md` § 3.10; `compositor.md` Dropdown terminal 1). It is
for a quick command in the middle of other work. It is not a terminal
window: it has no title bar, no place, no entry in the switcher, Mission
Control or session restore (Dropdown terminal 3), and a terminal kept open
beside other windows is an ordinary terminal app's job.

## Content

- **The terminal**: one session-long process of the user's terminal
  under an app id of Isle's own (Dropdown terminal 1; which terminal is
  the packet's, `compositor.md` Open). Isle adds nothing inside it.
- **Its frame**: no title bar and no lights (`window-chrome.md` Shared
  places, Undecorated windows); the corner radius every window has (brief,
  Left out) and the active window's shadow. Its colours and opacity are
  the terminal's own, themed like any installed terminal (P14).
- **Its place**: centred on the output with focus, its top edge on the
  lower edge of the island's strip, so the island, its privacy form and the
  menu bar's reveal stay above it. The same place over a full-screen
  window, where the strip is not drawn, so the top edge's reveal and the
  island's return zone stay clear (`competition.md`, Top edge over a
  full-screen window).
- **Its size**: a setting (Dropdown terminal 3) as proportions of the work
  area, so it suits both screens. Default: the width of the centred zone
  (`compositor.md` Snapping 1, the centred 60%), which keeps lines
  readable on the ultrawide where full width would not
  (`design-references.md` § 19 item 1); half the work area's height (v1
  used 70% by 50%). A full-width choice is offered. Dragging its bottom edge
  changes the height and either side edge changes the width about the
  centre; the result is written to the setting. It has no top edge to
  drag and cannot be moved, tiled or minimised.

## States

| State | What shows |
|---|---|
| Hidden | Nothing; the process keeps running, its scrollback intact. |
| First use | The chord starts the terminal with the pointer's starting form (`dock.md` Launch feedback); the dropdown comes down with the terminal's first frame, never as an empty frame. |
| Open | The terminal, focused, above every window. |
| Open, a shell surface holds focus | Spotlight, Control Centre, a banner's reply field or a system prompt (a sudo prompt from the terminal, say) take focus over it; it stays shown and gets focus back when they close. |
| Another window takes focus | It hides, unless "Hide when another window is focused" is off (For the user 1). Isle's own surfaces taking focus for a moment (Spotlight, a prompt) do not hide it. |
| The process exits | (`exit`, Cmd+W or Cmd+Q passed to the terminal.) The dropdown goes; the next chord starts a new one. |
| Error | The terminal will not start: nothing comes down, and the island shows the shell's acknowledgement "Couldn't open the dropdown terminal. Choose a terminal in Settings › Desktop." (`island.md`'s events; it answers a key pressed now). |
| Second screen | Opens on the output with focus, sized from that output's work area. The chord on a dropdown shown on the other screen moves it to the screen with focus and focuses it. |
| Full-screen app in front, not a game | Opens over it in the same place; the island stays hidden. |
| Windowed or full-screen game in front | The chord is not in B30's escape set, so it goes to the game. A game coming to the front hides an open dropdown. |
| Mission Control, Spotlight or another overlay open | The chord closes the overlay and opens the dropdown. Mission Control opening hides it (`mission-control.md`). |
| Locked | Under the lock; the chord does nothing (`lock.md`). |
| Focus on | No change. |
| Reduce Motion | Fades in and out at its place. |
| Reduce Transparency, Increase Contrast | The terminal's own background; Increase Contrast adds the outline every window gets (`window-chrome.md`). |
| Largest text | The terminal's own font size; Isle sets none inside it. |

## Opening and closing

- **The chord** toggles it on the output with focus (Dropdown terminal 2):
  hidden, it opens focused; shown and focused, it hides; shown but not
  focused, it takes focus (and moves to the focused output).
- **Opening** (the proposal: slides down from the top). It slides down
  from behind the island's strip, clipped at the strip's lower edge, so it
  appears to come out of the top of the work area and never passes over
  the island or the revealed menu bar. It is anchored there, so the slide
  is its open rather than a correction into place (brief P5). Short and
  critically damped; no bounce at the bottom.
- **Closing** slides it back up behind the strip, faster than it came
  (brief P5). Focus returns to the window that had it in the same frame
  (`compositor.md` Shell surface hosting 3).
- During Game Mode's profile, and over a full-screen window in Game Mode,
  it appears and goes without motion (`gaming.md` Game Mode 9).

## Keyboard

| Action | Key |
|---|---|
| Show, focus or hide | Ctrl+Option+` (*proposed*; the key above Tab, whatever it is labelled) |
| Everything else | The terminal's, translated by the terminal table (B21): Cmd+C, Cmd+V, Cmd+T and the rest as in any terminal; Escape is the terminal's |
| Size | Settings › Desktop › Dropdown terminal, so the size is reached without the pointer |

The chord is the desktop's, on the physical keys, before Cmd translation.
v1 used Super+`, which is now Cmd+`, the front app's window cycle (B20).
Ctrl+` is VS Code's own terminal toggle and Option+` the grave accent's dead
key on a Mac layout, so neither is taken. The Ctrl+Option family is the
desktop's (`competition.md` Keys), and ` keeps the Quake console's key and
v1's.

## Pointer

- **Click inside**: the terminal's.
- **Bottom and side edges**: resize, with the resize cursor; the change is
  saved as the setting.
- **Click on another window**: focuses it and hides the dropdown (For the
  user 1).
- **Scroll**: the terminal's. Nothing opens on hover.

## Shared places

| Place or input | Contenders | Proposed winner |
|---|---|---|
| Top centre, under the strip | The dropdown; the island's expanded forms, Control Centre and Notification Centre; the revealed menu bar | The top layer above the dropdown: the island, its panels and the bar draw over it. The bar is no taller than the strip, so a revealed bar never covers the terminal's first line. While the dropdown has focus the bar shows its menus |
| The windows layer | Ordinary windows, windows kept on top, picture-in-picture | From the bottom: windows, windows kept on top, picture-in-picture, then the dropdown while shown. All under the top layer and the overlay. *Adds to `competition.md` Layers* |
| The top right | Banners over a full-width dropdown | Banners, in the top layer. At its default width the dropdown is clear of them on the ultrawide |
| Keyboard focus | The dropdown against shell surfaces | It is a window (level 7 of `competition.md`'s order): every shell surface takes focus over it and gives focus back |
| Ctrl+Option+` | Apps' Ctrl+Alt+` | The desktop, on the physical keys; apps keep theirs through Cmd+Option, translated |
| The game escape set | The dropdown's chord | Not in it: a game gets the chord (B30) |
| Mission Control, the switcher, places, session restore | The dropdown | Excluded from all of them (Dropdown terminal 3); Mission Control hides it |

## Answers

- **How it appears** (`compositor.md`, proposal: slides down from the
  top). **Adopted, changed** in where it starts: it slides down from
  behind the island's strip, not from the screen's top edge, so the island,
  its privacy form and the menu bar's reveal keep the top edge
  (`revision-1.md`), and it keeps the same place over a full-screen window.
- **Its chord** (v1 Super+`). **Replaced** by Ctrl+Option+`, since Cmd+`
  is now the window cycle.
- **Its size** (a setting, Dropdown terminal 3). Centred, the centred
  zone's width and half the work area's height by default, a full-width
  choice, and resized by its own edges into the setting.
- **Focus and full screen.** Focused on opening, focus returned on hiding;
  over a full-screen app as normal; a game keeps the chord.

## Copy

- Settings › Desktop: "Dropdown terminal", "Shortcut", "Width" with
  "Centred" and "Full width", "Height", and "Hide when another window is
  focused" (on by default, For the user 1).
- Error: "Couldn't open the dropdown terminal. Choose a terminal in
  Settings › Desktop."
- The menu bar names it as its terminal app does.

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **When another window takes focus.** (a) The dropdown hides, as
   iTerm2's hotkey window does by default: a click on another window or a
   Cmd+Tab sends it up, and Isle's own surfaces taking focus for a moment
   (Spotlight, a prompt) do not; (b) it stays until the chord, as v1's
   special workspace did, sitting over the windows beneath. **Recommended:
   (a)**, with (b) as the setting "Hide when another window is focused"
   turned off. It sits above every window, so left open it covers the work
   the user has moved to, and a terminal meant to stay belongs in a window.
