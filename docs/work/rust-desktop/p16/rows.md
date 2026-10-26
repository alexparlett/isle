# P16 · The rows

Phase 16.1's gate (`p16-mission-control.md` choice 3): the Settings rows
Mission Control, Stage Manager and Split View add, which P1 named but did
not lay out. Drawn on `rows-1.html`. Binding: `p1/structure/settings.md`
(row and control kinds), `p1/structure/mission-control.md` Copy and Mouse
routes, `copy.md`. Conventions as `p13/rows.md`: keys are `settings.kdl`'s;
*New* marks a key P16 adds. The corner dwell (250 ms,
`timing.corner-dwell-ms`) was settled at the plan's review and is not a
row.

## Desktop

*Set how windows tile and fill, and what the screen corners do.* The
Windows group is P13's, unchanged.

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| Mission Control | Automatically rearrange desktops based on most recent use | C · switch | off | `mission-control.rearrange-desktops` *new* |
| Mission Control | Group windows by application | C · switch | off | `mission-control.group-by-app` *new* |
| Mission Control (foot) | "Shortcuts…", "Hot Corners…" | D · buttons under the group | | the sheets below; Hot Corners… moves here from under Windows |
| Stage Manager | Show recent apps | C · switch, "Off, the apps show when the pointer presses the screen's left edge." | off | `stage-manager.recent-apps` *new* |
| Stage Manager | Show windows from an application | C · pop-up | All at once, One at a time | `stage-manager.windows` *new*: `all-at-once`, `one-at-a-time` |
| Stage Manager (foot) | "Turn Stage Manager on for a desktop from its menu in Mission Control." | callout | | |

Stage Manager is turned on per desktop from the desktop's menu in Mission
Control's bar; the desktop's layout is saved in the `desktops` state
domain, not here. No "Click wallpaper to reveal desktop" row: on a Stage
Manager desktop a click on the wallpaper shows it, on a stacking desktop it
does nothing.

**Shortcuts…** (sheet, 560; "Mission Control shortcuts", "A modifier held
while choosing a mouse button is needed with it.")

| Row | Keyboard | Mouse | Bound to |
|---|---|---|---|
| Mission Control | recorder, "⌃↑, F3" | pop-up, "—" | `shortcuts.mission-control` |
| Application windows | recorder, "⌃↓" | pop-up, "—" | `shortcuts.app-expose` |
| Show Desktop | recorder, "⌘F3" | pop-up, "—" | `shortcuts.show-desktop` |
| Done | A · default | | |

The keyboard column is the same list Keyboard Shortcuts edits; the mouse
pop-up offers "—", Back, Forward, Button 6, Button 7, Button 8, never the
left, right or middle button. A modifier held while choosing is required
and shown before the button ("⌃ Back"). A button is stored as another chord
in the action's list (`"Ctrl+Back"`), so an action has one list; the
recorder in Keyboard Shortcuts shows it among the keys.

**Hot Corners…** (P13's sheet): defaults bottom left Mission Control,
bottom right Apps, the top corners —. Menu order: Mission Control,
Application Windows, Desktop, Notification Centre, Apps, Lock Screen, Put
Display to Sleep, —; Widgets joins with P17. `hot-corners.<corner>` gains
`mission-control`, `app-windows`, `desktop`; `hot-corners.bottom-left`
defaults to `mission-control`.

## Keyboard Shortcuts…

| Category | Actions added (`shortcuts.<id>`) |
|---|---|
| Desktops | Mission Control (`mission-control`, ⌃↑ and the Mission Control key), Application windows (`app-expose`, ⌃↓), Show Desktop (`show-desktop`, ⌘F3), at the top, before Desktop to the left |
| Tiling | Full screen tile left (`split-left` *new*), Full screen tile right (`split-right` *new*), after Fill beside; empty by default, so their checkboxes are off |

Inside Mission Control, Ctrl+Shift+←/→ and Ctrl+Shift+1…9 move the
selected window or desktop; they are fixed, not rows.
