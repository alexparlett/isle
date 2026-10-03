# Design

How Isle looks and moves. Made afresh (B3) by the design packet, P1 in
`docs/work/rust-desktop/roadmap.md`; this is its record. First draft, from
gate 4 (2026-10-03): the principles, the system and the components. Each
surface's look joins at stage 5 and the whole is signed off at gate 6.
Isle v1's design is on `main` for reference only.

The values live in `tokens.kdl` (now `docs/work/rust-desktop/p1/system/`,
until P2's workspace gives it a home); this document says what they are
for. What each surface does is in `docs/design/` and P1's structure
sheets (`docs/work/rust-desktop/p1/structure/`).

## The desktop

At rest each screen shows the wallpaper and the island: v1's pill at top
centre with the clock and a status glyph only while its state is not the
normal one. Control Centre unfolds from the island on hover and
Notification Centre from its clock. The front app's menus come down from
the top edge on demand, as macOS's automatically hidden bar. There is no
Dock: apps launch from Spotlight's Apps grid and switch with Cmd+Tab and
Mission Control (B18, B19, P1 revision 1).

## Principles

1. **Calm at rest.** The wallpaper and the island; nothing else shows
   unless it has something to say, and nothing reports the normal state.
2. **One desktop.** The shell, Isle's apps and installed apps share type,
   accent, icons and cursor from one token file.
3. **A Mac, built for this machine.** Where a choice is open, the answer a
   Mac user expects, sized for the ultrawide and tuned for gaming and
   coding. Departures are deliberate and recorded.
4. **Legible before translucent.** Frosted glass carries colour, never
   shapes. Contrast is promised against the surface text sits on. Every
   material has a solid form. Windows are opaque.
5. **Quick and still.** Short, critically damped, anchored motion: nothing
   bounces, nothing slides into a place it should have opened in, a resize
   never re-centres, exits are quicker than entrances.
6. **The accent is a signal.** One accent, the user's, marks what is
   active, focused or live. Status colours are for status; the rest is
   neutral.
7. **Out of the way of a game.** A game in front owns the screen and the
   keys; the shell keeps its escape set and a summary afterwards.

## Direction: Frost

macOS 27's frosted glass in neutral, slightly cool greys; Inter; soft
shadows for depth; medium corners; coloured tiles for Settings' panes;
critically damped motion. Dark first, light fully designed, Auto by the
sun. Chosen at gate 3 over Slate (solid, hairline-drawn, Geist) and Dusk
(deeper glass, warm, Figtree).

## Colour

Components use roles, never raw colours (`tokens.kdl` `colour`): window,
raised, sunken and view surfaces; title bar and sidebar; controls in
three states; three text levels and disabled; hairlines; fills; status
(ok, warn, danger, live) as fills and as text; the three privacy colours;
the island; the scrim. Increase Contrast replaces the roles it names.

- **Accent.** Eight, the user's choice, blue by default: each has a fill,
  the colour of text on it, and its form as text on a window. Dark mode's
  blue is deeper than macOS's so white text on it passes.
- **Data colours.** Four graph series and Settings' pane hues are data:
  never status, never the accent.
- **Terminal.** A 16-colour palette of its own, so blue stays blue
  whatever the accent.

## Contrast

Checked by `contrast.py` beside the token file, in light and dark, with and
without Increase Contrast:

- Every text role reaches 4.5:1 and every status mark 3:1 against the
  surface it sits on.
- On the shell's glass, only primary and secondary text are used.
  Primary passes over black, grey and white behind the glass; secondary
  over grey and the appearance's own extreme.
- Increase Contrast moves the glass to its tinted end and every role must
  pass everywhere; borders that identify controls reach 3:1.
- Reduce Transparency and Game Mode use the solid form.

## Materials

The shell's glass, from clear to tinted along the user's Glass setting,
default in the middle; blur 48, saturation 1.8. Glass is only on the
shell's floating surfaces (the island, its panels, menus over the
desktop, banners, Spotlight); windows are opaque and their menus and
popovers sit on the menu material.

## Type

Inter, with its optical sizes, at macOS's text styles and Mac density:
large title 26, title 22, title 2 17, title 3 15, headline 13 semibold,
body 13, callout 12, subheadline 11, footnote 10 (the minimum). Tabular
figures wherever digits change; the clock semibold. JetBrains Mono for
code and figures in tables. The user's text size multiplies every style.

## Space, corners and depth

- A 4-point grid, 2 to 32.
- Corners: window 10, panel 18, card 10, control 6, row 5, small 4.
  Nested corners are concentric: a child's radius is its parent's less
  the padding, never below small.
- Five shadows: window, inactive window (the active one is deeper),
  panel, menu, control.
- Hairlines are one point; the focus ring is the accent, 3 points at 1,
  for keyboard focus only.

## Motion

Five springs, critically damped or near it: morph (the island), open
(panels), exit (stiffer, so exits are quicker), move (windows), switch
(desktops). Fades for what does not move. Reduce Motion turns every
spring into a short fade in place; Game Mode draws shell surfaces
without transitions. Open and close are the compositor's (B7). The hover
guards' timings are tokens beside them.

## Icons and cursor

- **Symbols:** Phosphor, regular for outlines, fill for on, bold under
  Increase Contrast, in boxes of 12, 16, 20, 24, 32 and 48 points. Glyphs
  Phosphor lacks are drawn on its grid.
- **App icons:** Papirus (Papirus-Dark in dark), each app keeping its own
  outline, never forced into a squircle. Isle's five apps have icons drawn
  for them.
- **Cursor:** Bibata Modern Classic at 24.

## Components

The set the shell and Isle's apps share, each with hover, pressed,
focus, disabled and selected states and its keyboard behaviour: buttons
(regular, default, destructive, plain, icon; small, regular, large),
switches, checkboxes, radio buttons, sliders with and without ticks,
steppers, progress bars and spinners, segmented controls, tabs, text and
search fields with an error state, pop-up buttons, menus (no icons on
items; checked and radio both a check, mixed a dash), popovers, tooltips,
sheets, alerts, the toolbar with the traffic lights, sidebars, grouped
rows, tables, badges, Control Centre tiles, banners and the island.
Drawn on P1's system board (`p1/system-1.html`, `p1/system/components.css`).

## Installed apps

GTK 3, libadwaita, Qt, kitty, btop and zathura are themed from the tokens
(`p1/system/theming.md`); Isle's portal backend serves the appearance
settings, accent and reduced motion included (B13).

## Copy

The copy rules are P1's `structure/copy.md`: sentence case with Apple's
title case for macOS's standard items, macOS's words in British
spelling, buttons named for their action, short sentences, no blame.

## Surfaces

Each surface's look, states and motion join here from P1 stage 5.
