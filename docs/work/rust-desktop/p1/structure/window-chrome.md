# Window chrome

Batch 1, revised by revision 1. Answers to: `compositor.md` For the
design process (Stacking,
Snapping, Server-side decorations, Keyboard-shortcuts-inhibit, and this
surface's part of Shell surface hosting); `apps.md` For the design process
(Conventions: the unified toolbar, how a modal dialog is presented); B8,
B17, B20, B24, B25, B30.

## Purpose

The frame the desktop puts around a window and the moves it makes with
it: the title bar the compositor draws for apps that accept one, the
traffic lights, the zoom button's zones and the preview while dragging,
the window menu, how windows open, close, minimise, restore and go full
screen, and the conventions that make Isle's own apps (unified toolbar,
sheets, alerts) wear the same frame. It is there so every window reads as
part of one desktop (brief P2) and so placing windows on the ultrawide is
quick. It is not a place for status: nothing is added to a title bar to
report the normal state (brief P1). The menu bar's Window menu is the menu
bar's sheet; Spaces motion, Split View, Stage Manager, picture-in-picture
and the dropdown terminal are later batches.

## Content

**Server-side title bar.** Drawn by the compositor in Freya for a window
in server-side mode (`compositor.md` Server-side decorations 1–2). It is
part of the window, not a separate surface: the window's own body
colour, the same corner radius as every window (brief, Left out), and it
moves, scales and animates with the window (`design-v1-lessons.md` § Title
bars: the bar reads as part of the window).

- The traffic lights at the leading edge, in macOS's order: close,
  minimise, zoom. Each light's hit area fills the bar's height and the
  gaps between the lights, at least the HIG's 20 pt minimum
  (`design-macos.md` §4.6).
- The title, centred in the bar as on a macOS title-bar-only window,
  truncated at the end, never under the lights: when the window narrows,
  the title moves right of the lights and then truncates, and below that
  it drops. From `xdg_toplevel.title` or `_NET_WM_NAME`.
- Nothing else: no app icon, no status, no buttons of Isle's own (brief
  P1; the interview's dislike of noise).
- Every edge and corner resizes (Server-side decorations 5); the grab band
  straddles the visible edge and the corner grab follows the radius, so a
  small radius never pushes the hit area outside the window, a fault of
  macOS 26 (`design-macos.md` §0 item 4).
- A shadow the compositor draws, deeper for the active window than for
  inactive ones (macOS 27, brief As is). The bar is never pushed into the
  island's strip and never left off every output (D57).

**The lights.**

| Light | Click | Option-click | Hover |
|---|---|---|---|
| Close | The close path of `compositor.md` Apps 8 (Cmd+W's, keep-running included) | Closes every window of the app (macOS) | Glyph |
| Minimise | Minimise: the window shrinks toward the island | Minimises every window of the app (macOS) | Glyph |
| Zoom | Full screen in its own Space, or out of it when full screen (Spaces 5; v1's reversal, `design-v1-lessons.md` §4) | Fill, or back to the previous box when filled (Minimise and maximise 2) | Glyph; after a dwell, the zoom menu |

Hovering the group shows all three glyphs at once, on an inactive window
too, and a click on an inactive window's light acts without first raising
it (macOS). A fixed-size window (minimum equals maximum) has zoom disabled.
A window with a parent (a dialog) has minimise disabled; minimising the
parent takes its dialogs with it. A full-screen window has minimise
disabled, as on macOS.

**Title-bar actions.** Double-click runs the user's title-bar action, fill
by default (macOS's "Zoom"), or minimise, or nothing (Minimise and
maximise 3); on a filled window it returns the previous box. Secondary
click opens the window menu at the pointer. Dragging moves the window with
snapping. Dragging an inactive window's bar with Cmd held moves it without
raising it (macOS).

**Zoom menu.** Opened by resting on the zoom button, by pressing and
holding it, by a chord, or from the window menu. A grid of layout
thumbnails, after Windows' snap layouts (`design-references.md` §5), not
macOS's text list; each thumbnail is drawn in the shape of the window's
output, so thirds on the ultrawide read as thirds, and each slot in a
thumbnail is one zone. Choosing a slot puts the window there; hovering a
slot shows the zone preview on the real screen. The window's current zone
is marked.

In macOS's sections (`design-macos.md` §3.1, the green button's menu):

1. **Move and resize**, the zones of B8 and `compositor.md` Snapping 1, in
   this order:
   - halves (left, right);
   - thirds (left, centre, right);
   - two-thirds and a third (left two-thirds, right third), and its mirror;
   - quarters;
   - centre (the centred 60%, one slot);
   - fill;
   - beside: only when the work area has a gap beside another window, a
     thumbnail of the real arrangement with the other windows outlined and
     each gap a slot;
   - "Return to previous size", as a text item, only when the window is
     tiled or filled.
2. **Full screen**: full screen; left of screen and right of screen, which
   start Split View (Split View 1; its candidates are the Split View
   sheet's).
3. **Move to <display>**, only while a second output is connected.

Arrangements of several windows at once (macOS's Fill & Arrange) are open
for P4 (`compositor.md` Snapping, Open); the menu gains a section for them
if the packet builds them, and draws nothing for them until then.

**Zone preview.** While a dragged window's pointer is in an edge or corner
region, or over a slot of the zoom menu or the pointer panel, the
compositor draws the target zone's real rectangle (inside the tile margins
when they are on), under the dragged window and over every other window. A
shape, not a picture of the window (macOS's outline). Moving from one
region to another moves the shape between rectangles rather than
replacing it, so the preview never flickers. The window itself keeps its
size until it is dropped.

Edge regions are macOS's: the left and right edges give halves, the
corners give quarters, the top edge gives fill (Snapping 3); the bottom
edge gives nothing, as on macOS (For the user 7). The top edge's region
runs along the island's strip, island included, so a drag reaches it
anywhere along the top. Thirds, centre and beside are reached by the zoom
menu, the chords and the pointer panel.

**Work area.** Each screen's work area is the whole screen less a strip
along the top kept clear for the island at rest (v1 kept a 48 px gap; the
height is stage 4's, and is the island's rest height plus its margin).
Fill, the zones and snapping stop at the strip; a title bar is never
dragged into it. The bottom edge belongs to windows: fill and the lower
zones reach the screen's bottom edge, inside the tile margins when they
are on. The on-demand menu bar reserves nothing: it slides down over
windows when revealed and leaves them where they are, as macOS's
auto-hidden bar does (`revision-1.md`). This replaces `compositor.md`
Stacking 7's menu bar and Dock reserve at the gate.

**Pointer panel.** Holding Option while dragging shows the zone targets at
once on macOS (Snapping 3). Here the targets come to the pointer: the zoom
menu's Move and resize grid appears beside the pointer, and releasing over
a slot tiles the window there, so nothing needs a trip to a far edge of
3440 px (Rectangle Pro's snap panel, `design-references.md` §5, §19).
Releasing Option before the drop hides it. A user decision (For the user).

**Unsnapping.** Dragging a tiled or filled window away returns it to its
pre-tile size under the pointer, keeping the pointer at the same relative
place in the bar (Snapping 7).

**Window menu.** Secondary click on the bar, a client's
`xdg_toplevel.show_window_menu` (GTK sends its header bar's right-click
here), or a chord. In order:

```
Minimise                       Cmd+M
Zoom
Enter Full Screen              Ctrl+Cmd+F
Move and resize              ▸ (the zones by name, with their chords)
—
Move to desktop              ▸ (each desktop, New desktop)
Move to <display>                (with a second output)
Keep on top                    ✓ when on
—
Move
Resize
—
Close                          Cmd+W
```

"Keep on top" is v1's pin: on top and on every desktop (Stacking 9).
"Move" and "Resize" start a keyboard move or resize (arrows, Shift for
larger steps, Return to keep, Escape to undo), so placing a window freely
is not reached only by pointer; macOS has no equivalent, Windows, GNOME
and KDE do. The window menu's first group and the menu bar's Window menu
carry the same items in the same order; the menu bar's sheet owns that
order and this sheet follows it.

**What other clients get.**

| Client | Title bar | Also |
|---|---|---|
| Wayland, accepts server-side (Qt, Firefox with its title bar on, Chromium with "Use system title bar") | Isle's | Everything on this sheet |
| X11 normal window without Motif decorations off or `_GTK_FRAME_EXTENTS` | Isle's | `_NET_WM_MOVERESIZE` from its own controls moves and snaps (Xwayland 5) |
| Wayland client-side (GTK 4 and libadwaita, Chromium and Electron by default) and X11 with its own frame (Steam) | None (Server-side decorations 2) | The window menu through `show_window_menu`; snapping on every move it hands over; the title-bar action where GTK hands it over (`gtk_shell1` v5's `titlebar_gesture`, `compositor.md` Open) |
| Override-redirect and untitled X11 windows | None, and no shadow or animation (Xwayland 2–3) | |
| Isle's own apps | None; the unified toolbar | Below |
| Full-screen and borderless windows, games among them | None | |

The decision is taken again whenever the client's mode, full-screen state
or hints change (D48). The compositor draws a shadow for a client-side
window only when the client draws none, read from the protocol rather than
a class list: a window whose geometry is inset from its surface draws its
own (`design-v1-lessons.md` §8, Do not repeat). Client-side GTK apps get
their buttons on the left from the `gtk-decoration-layout` setting Isle's
theming writes (`macos-model.md` §3, Window controls); whether `gtk.css`
can also draw them as Isle's lights is for stage 4's installed-apps
mapping.

**Unified toolbar (Isle's apps).** Each Isle app draws one toolbar that
carries the lights (`apps.md` Conventions 15; B25), so its windows and a
server-side window side by side read alike (brief P2):

- The lights in the same place, size and order as on the server-side bar,
  acting through the compositor (`xdg_toplevel` close, set_minimized,
  set_maximized); their states as above.
- Leading: the sidebar toggle, back and forward, then the window's title
  (never the app's name). Centre: the window's common controls, which fold
  into a » menu as the window narrows. Trailing: search, an inspector
  toggle, at most one primary action. Three groups at most, and text and
  symbol buttons never mixed in one group (HIG, `design-macos.md` §3.1
  Toolbars).
- A sidebar runs to the window's top, bottom and leading edge, and the
  lights sit over its top; the toolbar's controls start after it (macOS
  27, brief As is).
- One toolbar height across every Isle app (brief As is: one toolbar
  style). When content scrolls under the toolbar it gets a solid bar
  behind it (macOS 27, `design-macos.md` §3.1).
- Empty toolbar space moves the window; double-click runs the user's
  title-bar action, read from settings (Conventions 15); secondary click
  opens the window menu.
- The toolbar's controls are the window's own opaque components, never
  glass (brief P4; `design-v1-lessons.md` §8).

**Sheets and alerts (Isle's apps).** A modal dialog is a sheet on its
window (`apps.md` For the design process, Conventions; rule 18): a card
attached under the toolbar, centred across the window, that dims the
window's content and never covers most of it. The parent can still be
moved; other windows stay usable. The window's minimum size holds its
largest sheet (Conventions 16). An alert is a sheet with a title, at most
one line of informative text, and up to three buttons named for their
action, the default one trailing; a destructive action is never the
default. No app icon, since the sheet hangs from its own app's window; a
caution symbol only for what cannot be undone. Files' Open and Save panels
as sheets are the Files sheet's.

**Not responding.** A window whose app misses a ping (Apps 10) shows the
busy pointer while the pointer is over it, and the Force Quit dialog marks
the app. Closing it while it does not respond: For the user.

**Keyboard-shortcuts-inhibit.** While a focused window holds an inhibitor
(a VM, a remote desktop), two things show:

- a glyph in the island, present only while an inhibitor is active
  (`revision-1.md`'s list of glyphs; not tiered, so it cannot be set to
  never). Where the app's name, the escape chord and "Take keys back" sit
  when the island opens is `island.md`'s;
- once each time the window gains the keys, a short notice inside the
  window's top edge naming the escape chord, then gone. The island at rest
  is hidden when a VM runs full screen, which is when the notice matters.

The escape chord itself is `compositor.md`'s (Keyboard-shortcuts-inhibit,
Open).

**Show Desktop.** The window side of Show Desktop (Stacking 10): every
window on the output moves to its nearest
output edge (left, right or bottom; the top is the island's strip and the
menu bar's reveal), keeping its size, until only a sliver of each shows at
the edge (macOS). Clicking a
sliver, the chord again, activating any window or a window mapping puts
them all back in their boxes along the same path. It reveals the plain
wallpaper. It is the only such move: there is no "Show widgets" move,
since the widgets dashboard covers the windows and moves none
(`widgets.md`).

**Choreography.** Every move starts and ends at a window's real rectangle;
nothing slides into a place it should have opened in (brief P5, v1 D91).
Exits are faster than entrances. Character only; springs and durations
are stage 4's.

| Event | From | To | Character |
|---|---|---|---|
| Open | Its placed box, slightly smaller | Its box | Fade in and a small scale about the box's centre; no translation |
| Close, hide | Its box | Its box, slightly smaller | Fade out, quicker than open |
| Minimise | Its box | A small rectangle just under the island, at top centre of its output | Shrinks and moves toward the island while fading out (brief Changed: no genie, no squash); the island takes no part and does not change |
| Restore | Under the island (from the switcher, the Window menu, Cmd+Tab); or its tile in App Exposé or Mission Control when restored there | Its box, exactly | The reverse; the box is set before the move starts |
| Show again after hide | Its box | Its box | Fade in |
| Zoom, fill, tile, return | The old box | The new box | A short move of the frame; the old content held until the client commits at the new size, then crossfaded |
| Drop into a zone | Where it was released | The zone | The same move, short |
| Enter full screen | Its box | The whole output, in its new Space | The bar goes first, the window grows; the Space motion is the Spaces sheet's |
| Leave full screen | The output | Its box in its origin Space | The reverse; the bar returns last |
| Sheet | The toolbar's lower edge | Its place | Grows down from its anchor, parent's dim fades in |
| Show Desktop | Its box | Its edge | Moves straight out; back the same way |

## States

| State | What shows |
|---|---|
| Rest, active | The bar or toolbar with coloured lights, the deeper shadow |
| Inactive | Lights neutral, title and toolbar symbols subdued, lighter shadow (macOS 27 dims inactive icons and text, `design-macos.md` §3.1) |
| Hover over the lights | All three glyphs, active or inactive |
| Pressed light | The light shows pressed; the action runs on release inside it |
| Disabled light | Zoom on a fixed-size window, minimise on a dialog or a full-screen window: shown, not coloured, not clickable |
| Loading | The bar appears with the window's first frame, never before it; no empty frame |
| Error | A panicking bar surface is rebuilt from state (`shell.md` Shell runtime 2); meanwhile the window is undecorated and still moves by modifier drag and chords |
| With a sheet | Parent content dimmed, lights still work except close, which waits on the sheet |
| Not responding | Busy pointer over the window |
| Tiled or filled | The bar as usual; with margins on, the decoration sits inside the tile (Snapping 6) |
| Dragging | The zone preview while in a region; the pointer panel with Option |
| Full screen | No bar; the pointer pushing the top edge reveals the on-demand menu bar and the window's own bar beneath it, so the lights are reachable (macOS) |
| Long title, narrow window | Title truncated, then dropped; the lights stay |
| Title bars off (Server-side decorations 6) | No bars; modifier drag, chords and the window menu's chord |
| Keys inhibited | The island's glyph; the notice once per gain |
| Show Desktop | Slivers at the edges |
| Second screen | Bars drawn at that output's scale; zones from that output's work area; zoom-menu thumbnails in its shape (16:9 beside 21:9); a drag shows the preview on the output under the pointer |
| Game in front, windowed | Chrome as normal (interview: the shell stays as normal); while Game Mode's profile applies, no motion and no shadows (`compositor.md` Gaming display path 8, `gaming.md` Game Mode 9) |
| Game in front, full screen | No chrome; the top-edge reveal shows the on-demand menu bar only, never the game's bar (`gaming.md` Game Mode 5) |
| Locked | Nothing; the lock covers windows and takes no hover |
| Focus on | No change |
| Reduce Motion | Open and close fade in place; minimise fades the window out in place and restore fades it in; zoom and tiling cut to the new box with a crossfade; the preview jumps between zones; Show Desktop fades windows out and back; sheets fade |
| Reduce Transparency | Bars and toolbars are opaque already; the zone preview, the zoom menu and the pointer panel take their solid forms |
| Increase Contrast | An outline around every window, the zone preview and each light; inactive lights keep a visible ring |
| Largest text | The bar and toolbar grow with the title's size so it is never clipped; the lights and their hit areas keep their size |

## Opening and closing

The chrome has no opening of its own: it comes and goes with its window
(Choreography above). Its transient parts:

- **Zoom menu**: opens on a dwell over the zoom button, a press and hold,
  its chord, or the window menu; closes on a choice, Escape, a click
  elsewhere, or the pointer leaving it after the grace. It grows from the
  zoom button and fades out, quicker than it came.
- **Pointer panel**: appears beside the pointer when Option is held during
  a drag, goes on release of the drag or of Option. A fade, no movement.
- **Zone preview**: fades in when a region is entered, moves between
  zones, fades out on leaving the region or on drop as the window settles
  into it.
- **Window menu**: a menu at the pointer (or under the lights when opened
  by chord), as every menu opens (the menu bar's sheet).
- **Inhibit notice**: fades in inside the window's top edge when the window
  gains the keys, fades out after a short read; never while a game holds
  the pointer.

## Keyboard

Chords marked *proposed* are this sheet's proposals for the shortcut set
(`compositor.md` Shortcuts and keybinds), which owns them and refuses a
clash. Tiling chords are macOS's own letters with Fn+Ctrl replaced by
Ctrl+Option, since Fn is invisible on most keyboards (`macos-model.md` §3),
and match Rectangle's long-standing defaults. They are bound to the
physical keys (Shortcuts 3), so an app still receives Cmd+Option chords
translated.

| Action | Key |
|---|---|
| Close window | Cmd+W |
| Minimise | Cmd+M |
| Minimise all of the app's windows | Cmd+Option+M (*proposed*, macOS's) |
| Hide app, hide others | Cmd+H, Cmd+Option+H |
| Enter or leave full screen | Ctrl+Cmd+F |
| Left or right half; repeated, two-thirds, then a third | Ctrl+Option+← / → (*proposed*) |
| Centre (60%); repeated, the centre third | Ctrl+Option+C (*proposed*) |
| Fill | Ctrl+Option+F (*proposed*) |
| Return to previous size | Ctrl+Option+R (*proposed*) |
| Quarters | Ctrl+Option+U, I, J, K (*proposed*) |
| Fill the space beside | Ctrl+Option+B (*proposed*) |
| Move to the next or previous display | Ctrl+Option+Cmd+← / → (*proposed*) |
| Open the zoom menu on the focused window | Ctrl+Option+Z (*proposed*) |
| Open the window menu | Ctrl+Option+Space (*proposed*) |
| Keep on top | the shortcut set's pin chord (v1 had one) |
| Take keys back from an inhibitor | the escape chord (`compositor.md` Open) |

Focus: the zoom menu and the window menu take keyboard focus when opened
by key; opened by hover, the zoom menu takes it at the first key press. In
the zoom menu, arrows move between slots across thumbnails, Tab moves
between sections, Return chooses, Escape closes. When either closes,
focus returns to the window in the same frame (Shell surface hosting 3).
Keyboard move and resize from the window menu: arrows, Shift+arrows,
Return, Escape. In a sheet: Return takes the default button, Escape and
Cmd+. cancel, Cmd+Delete takes a destructive button such as "Don't save"
(macOS), Tab reaches every control (Conventions 18, 28).

## Pointer

- **Bar**: press and drag moves, with snapping to other windows' edges and
  the work area (Snapping 8); double-click, the title-bar action;
  secondary click, the window menu; Cmd-drag on an inactive window moves
  it without raising it. Click on an inactive bar raises and focuses it.
  Scroll and middle click do nothing.
- **Lights**: click, Option-click and hover as in the table above. No
  tooltips.
- **Zoom button hover**: opens the zoom menu after a dwell long enough that
  passing over never opens it; a grace while the pointer crosses from the
  button to the menu; disarmed after a click until the pointer leaves the
  button; never during a drag (the hover guards, `design-v1-lessons.md`
  §3.5, brief).
- **Edges**: resize from every edge and corner, with the resize cursor.
- **Drag**: edge and corner regions show the preview; Option shows the
  pointer panel; release tiles. Dragging a tiled window away unsnaps it.
- **Zoom menu**: hover over a slot shows the preview; click chooses.
- **Sheet**: a click on the dimmed parent does nothing.
- **Show Desktop**: a click on a sliver puts every window back.

## Shared places

| Place or input | Competes with | Proposed winner |
|---|---|---|
| The top edge during a window drag | The on-demand menu bar's reveal (the pointer pushing the top edge outside the island's zone), the island's hover zone | The drag: the whole top edge is the fill target; the bar is not revealed and the island takes no hover until the drag ends |
| The top edge over a full-screen window | The on-demand menu bar's reveal (`shell.md` Menu bar 10); Game Mode's reveal | One reveal: the menu bar and, for a non-game window with Isle's bar, that bar beneath it; for a game, the menu bar only |
| The top of the work area | The island's strip against every window's bar | The strip; fill and zones stop under it and a bar is never dragged into it |
| The revealed menu bar over a window's top | The bar slides down over the window's title bar or toolbar | The bar, while revealed; the window does not move, and its lights are reachable again when the bar leaves (macOS's auto-hidden bar) |
| The top right of the screen | Banners over a window's trailing toolbar controls and the right half's or third's bar (`design-references.md` §19 item 4) | For `competition.md`; the top right is no longer a status area, banners' place is batch 2's, and banners stay above windows |
| The bottom edge | Windows against a drag region | Windows: the work area reaches the bottom edge; no zone (For the user 7) |
| Minimise's target | The island at top centre | `island.md` supplies the island's rectangle on each output; this sheet animates toward it. The island does not react |
| The island's glyphs | The inhibit glyph against the island's other glyphs and its live activities | The glyph while active, not tiered; its order among the glyphs is `island.md`'s. Not a live activity: inhibit is a state of one window, shown by the glyph and the in-window notice |
| Layers | Title bars, the zone preview, the zoom menu, the pointer panel, the window menu | Bars and shadows with their window in the windows layer; the zone preview in the windows layer, just under the dragged window; the zoom menu, pointer panel and window menu in top, under every overlay (`compositor.md` Shell surface hosting 1) |
| Keyboard: Ctrl+Option chords | Apps' own Ctrl+Alt chords (IDEs) | The desktop, on the physical keys; Mac-chord users reach the app's through Cmd+Option, which is translated |
| Keyboard: the inhibit escape chord | The game escape set (B30); a game's own inhibitor | `competition.md` with `gaming.md` Open: the escape chord survives every inhibitor |
| Keyboard focus | The zoom and window menus against the window; a sheet against its window; system prompts against a sheet | Menus take focus on demand and give it back; a sheet holds its window's focus; a system prompt takes focus over everything (system prompts sheet) |
| Show Desktop | Hot corner, the spread gesture, Cmd+F3, Mission Control | Show Desktop is the only window move; the widgets dashboard moves no window (`widgets.md`) |
| The zoom menu's Full screen section | Split View's tile left and right | The entries live here; their behaviour and candidates are the Split View sheet's (batch 3) |
| Undecorated windows | Picture-in-picture (proposal: no bar), the dropdown terminal | Their sheets (batch 3); this sheet draws no bar on either |
| Full-screen motion | The Space switch | The window grows in this sheet; the Space moves in the Spaces sheet |
| Closing elsewhere | Mission Control's close, the switcher's close | The same close path and the same close motion |

## Answers

- **Stacking: the choreography of open, close, minimise and restore**
  (proposal: from and to the window's real rectangle). **Adopted**, and
  extended to zoom, tiling, full screen, sheets and Show Desktop: every
  move begins and ends at a real box, open and close scale about the box
  without translation, minimise shrinks the window toward the island and
  restore returns it from there, or from its tile in App Exposé or Mission
  Control, to its real box (brief P5, Changed; revision 1; v1 D91).
- **Stacking: whether Show Desktop reveals widgets, and on which chord**
  (proposal, B24: "Show widgets" on v1's dashboard chord). **Replaced**:
  the window move is answered here and is Show Desktop's alone, revealing
  the plain wallpaper; the widgets are a dashboard over the windows on
  their own chord, which moves none (`widgets.md`).
- **Stacking: where windows go when moved aside.** **New**: each to its
  nearest output edge, keeping its size, a sliver left showing, back along
  the same path (macOS).
- **Snapping: the look of the zone preview.** **New**: the target zone's
  real rectangle as a shape under the dragged window, moving between zones
  rather than replaced; its material is stage 4's.
- **Snapping: how the zones are laid out in the zoom button's menu.**
  **New**: a grid of layout thumbnails in the output's shape, each slot a
  zone, in macOS's sections (Windows' snap layouts against macOS's text
  list, `design-references.md` §5), with the beside zone drawn from the
  real arrangement.
- **Server-side decorations: where the controls and title sit** (proposal:
  controls on the left, as on macOS). **Adopted**: close, minimise, zoom
  at the leading edge, title centred (B25, B17).
- **Server-side decorations: how zoom offers its zones** (proposal: a menu
  on hover). **Adopted**, with the hover guards, a press-and-hold and a
  chord route; click stays full screen and Option-click fill, as
  `compositor.md` and v1's reversal already settle.
- **Server-side decorations: the window menu's layout.** Answered above:
  window state, zones, desktops and displays, keep on top, keyboard move
  and resize, close. "Move" and "Resize" are a departure from macOS, for
  keyboard reach.
- **Keyboard-shortcuts-inhibit: where its active state is shown**
  (proposal: the menu bar). **Replaced**: a glyph in the island while
  active (revision 1: there is no menu bar at rest), plus a one-time
  notice inside the window naming the escape chord, because the island is
  hidden when the VM is full screen.
- **Shell surface hosting: which layer each surface takes.** **Partly**:
  this sheet's parts only (Shared places, Layers); the rest is
  `competition.md`'s.
- **Conventions: the menu set and order.** The menu bar's sheet. This
  sheet only requires that the window menu's first group match the
  Window menu.
- **Conventions: the unified toolbar's contents.** Answered above: the
  lights as on the server-side bar, three groups in HIG order, a sidebar
  to the edge, one height, a solid bar when content scrolls under. Its
  look is stage 4's.
- **Conventions: how a modal dialog is presented** (proposal: a sheet on
  its window). **Adopted** for Isle's apps, alerts included; for other
  apps' modal dialogs, For the user.

## Copy

In sentence case (`design-v1-lessons.md` §8), except macOS's standard
items, which keep Apple's title case (`copy.md` rule 1).

- Window menu: Minimise, Zoom, Enter Full Screen, Exit Full Screen, Move
  and resize, Move to desktop, Desktop 1…, New desktop, Move to
  <display>, Keep on top, Move, Resize, Close.
- Zones: Left half, Right half, Left third, Centre third, Right third,
  Left two-thirds, Right two-thirds, Top left, Top right, Bottom left,
  Bottom right, Centre, Fill, Fill the space beside, Return to previous
  size.
- Zoom menu sections: Move and resize, Full screen; items Full screen,
  Left of screen, Right of screen, Move to <display>.
- Inhibit: the glyph's detail "<App> has the keys", "Take keys back"; the
  notice "<App> has the keys. Press <chord> to take them back."
- Not responding: "{App} is not responding", buttons "Force Quit" and
  "Wait".
- Sheets: buttons named for their action ("Delete", "Don't save"),
  "Cancel"; never "OK" where a verb fits.

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **The pointer panel.** Holding Option while dragging shows the zone grid
   beside the pointer (Rectangle Pro), instead of macOS's targets at the
   screen edges. Recommendation: yes; plain drags keep macOS's edges, so
   nothing appears unless Option is held (brief P1).
2. **Cycling on repeated chords.** Pressing Ctrl+Option+← again goes half,
   two-thirds, third; Ctrl+Option+C again goes 60%, centre third
   (Rectangle). Recommendation: yes; it costs nothing at rest and saves
   five chords.
3. **Almost-maximise.** A zone that fills all but a margin, or making the
   centred 60% what Fill does on the ultrawide (`design-references.md`
   §19). Recommendation: no; Fill stays Fill, as a Mac user expects, the
   centred 60% is the readable large size, and the margins setting already
   gives Fill an edge.
4. **Other apps' modal dialogs as sheets.** Draw a third-party modal dialog
   (xdg-dialog modal, X11 modal transient) attached under its parent's
   bar and moving with it, as GNOME does, rather than centred over the
   parent (`compositor.md` Stacking 6). Recommendation: yes for a modal
   dialog smaller than its parent; anything larger stays centred over it.
5. **Closing an app that is not responding.** A Mac does nothing until
   Force Quit. Offer an alert ("{App} is not responding": Force Quit,
   Wait) when close is pressed on a window whose app has missed its ping.
   Recommendation: yes; otherwise close looks broken, and Force Quit is a
   chord away few remember.
6. **Granting keyboard-shortcuts-inhibit.** Always, or asked once per app
   (`compositor.md` Keyboard-shortcuts-inhibit, Open). Recommendation:
   always, since the escape chord, the island's glyph and the notice make
   it visible and reversible, and a prompt is noise for one user.
7. **The bottom edge.** With no Dock the bottom edge is free. (a) No zone,
   as macOS: a drag there only moves the window; (b) the bottom edge gives
   the bottom half. Recommendation: (a). A full-width bottom half is
   rarely wanted on the ultrawide, the bottom corners still give quarters,
   and a window dragged low to look at something behind it should not
   tile (brief P3).
