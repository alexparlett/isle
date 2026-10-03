# Structure sheets

P1 stage 2: what every surface is, before what it looks like. One sheet
per surface, in this shape. No colours, sizes in points only where the
structure depends on them (a hit target, a row count), no type or
material: those are stage 4.

```
# <Surface>

Batch <n>. Answers to: <intent doc § items>, <decisions>.

## Purpose
One paragraph: what it is for, and what it is not for.

## Content
What it shows, by part, and where each part's data comes from.

## States
| State | What shows |
Every state that applies: rest, active, empty, loading, error, full or
overflowing, many items, the second screen, a game in front (windowed and
full screen), locked, Focus on, Reduce Motion, Reduce Transparency,
Increase Contrast, the largest text size.

## Opening and closing
What opens and closes it (key, pointer, event), where it comes from and
goes to, and the character of the motion. Values are stage 4's.

## Keyboard
Every action and its key, and how focus moves. Nothing reached only by
pointer.

## Pointer
Hover, click, secondary click, drag, scroll; hover guards where hover
opens anything (brief, v1 lessons §3.5).

## Shared places
What else wants the same place on screen or the same input, and what this
sheet proposes wins. Collected in `competition.md`.

## Answers
Each "For the design process" item this surface owns: the proposal, then
**adopted**, **changed** or **replaced**, and why, in a line or two.

## Copy
The surface's fixed strings, in the rules of `copy.md`.

## For the user
Questions only the user can answer, each with a recommendation.
```

Sheets cite the brief's principles by number ("brief P1") and evidence by
note and section (`design-macos.md` § Dock).
