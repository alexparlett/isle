# Mission Control

Batch 3. Answers to: `shell.md` § For the design process (Mission Control;
the Mission Control part of Shell runtime); `compositor.md` § For the
design process (Spaces, Split View, Stage Manager); `gaming.md` § For the
design process (Full-screen Spaces); the Mission Control rows `dock.md`
hands on (minimised windows, show all windows). B8, B13, B17, B19, B20,
B25, B30.

## Purpose

Mission Control is where the user sees everything at once and puts it in
order: every window of the current desktop spread out so any can be picked
or closed, the desktops and full-screen Spaces of each screen along the
top, and the minimised windows that, with no Dock, have nowhere else to be
seen (`dock.md`). App Exposé is the same view for one app. The sheet also
owns what surrounds it: the Space-switch motion, Split View, Stage Manager,
hot corners, and the gestures and mouse routes that reach them. With no
Dock, Mission Control is the pointer's way to switch (`dock.md`, A pointer
route to switching). It is not a launcher (Spotlight is), not a recency
switcher (Cmd+Tab is), and not a widgets surface (the dashboard is,
`widgets.md`).

## Content

**The Spaces bar.** Along the top of each screen, under the island's strip
(Shared places), one row per screen ("displays have separate Spaces",
`compositor.md` Spaces 1).

| Part | What | Data |
|---|---|---|
| Desktop | A live miniature: the screen's wallpaper with that desktop's windows at their real places, scaled; labelled "Desktop 1", "Desktop 2" in bar order | compositor's window list and textures, drawn only while Mission Control is open (B13, v1 D55) |
| Full-screen Space | A live miniature of its window, labelled with the app's name; a game's by the game's name from the library, its cover as the label's icon (`gaming.md` Games as apps) | as above |
| Split View Space | Both windows side by side at their split, labelled "{app} and {app}" | as above |
| The current Space | Marked by a neutral ring; the accent is kept for the selection, the one thing (brief P6) | |
| Add | A + tile at the end of the row, always shown, not on hover (GNOME's critique of macOS's hover +, `design-references.md` § 6). Hidden when the screen has as many desktops as the compositor allows (`compositor.md` Spaces, Open) | |
| Remove | On hover over a desktop, a remove control at its top leading corner; never on a screen's only desktop. On a full-screen or Split View Space, the control exits full screen instead | |

The bar is always shown at full size, as v1's was, not macOS's row of names
that grows into miniatures only when the pointer reaches it
(`design-macos.md` § 3.15): a desktop's content is the point of the bar,
the ultrawide has the room, and a bar that opens on hover would need hover
guards for nothing. Order is the user's: full-screen Spaces sit to the
right of the desktop they came from (`compositor.md` Spaces 5), and the
row is reordered by drag. "Automatically rearrange desktops based on most
recent use" is off by default (Answers).

**The spread.** The current desktop's windows, laid out over its wallpaper
(`wallpaper.md` States):

- Without overlap, never larger than real, each keeping its aspect, and
  each placed as near its real position as the layout allows, so a window
  is found where it was (v1 D19, `isle-v1-inventory.md` § 4 Mission
  Control). Rows when the windows will not fit side by side.
- Each tile carries its app's icon over its lower edge, as the switcher's
  cards do; the window's title shows under the hovered or selected tile
  only (brief P1). A game's tile shows the cover as its icon.
- A dialog is drawn on its parent's tile, not as a tile of its own.
- Windows kept on top are in the spread of whichever desktop is current.
  Picture-in-picture and the dropdown terminal are not (`pip.md`,
  `dropdown-terminal.md`).
- "Group windows by application" (`shell.md` Mission Control 2), off by
  default as on macOS: each app's windows gather in one cluster with the
  app's icon and name under it.
- On hover, a close control at the tile's top leading corner, where a
  window's close light sits (v1, brief Changed).

**The minimised tray.** A row along the bottom of the screen, under the
spread, only while the current desktop has minimised windows or windows of
hidden apps (v1's hidden-window tray, back with no Dock: `dock.md`,
`design-v1-lessons.md` § 5 Mission Control).

- The desktop's minimised windows first, then hidden apps' windows on this
  desktop as a second group, each tile marked "Minimised" or "Hidden".
  Smaller than spread tiles, live like them (`compositor.md` Minimise 4).
- Its ends stop short of the bottom corners, so the hot corners stay
  reachable while it shows.

**App Exposé.** The same surface for one app (`shell.md` Mission Control
5): no Spaces bar; the app's windows on the current desktop in the spread;
below them, set apart in the tray's place, its minimised windows, then its
hidden ones, then its windows on this screen's other desktops, each tile
labelled "Minimised", "Hidden" or the desktop's name. The app's name and
icon above the spread. macOS's recent documents under App Exposé are left
out: Isle keeps no per-app recent-documents list outside Spotlight.

**Second screen.** Mission Control opens on every screen at once, as on
macOS: each screen its own Spaces bar, spread and tray. App Exposé likewise
shows the app's windows per screen; the set-apart row is on the screen with
focus.

**The Space switch.** Ctrl+←/→, Ctrl+1…9, a gesture, a desktop chosen in
Mission Control, or an app activated on another Space (`compositor.md`
Apps 3). The wallpaper stays still (`wallpaper.md`: one picture per
screen), the island and the top layer stay, and only that screen's windows
move. The windows of the old Space move toward the side away from the new
one while the new Space's windows come in from the other side, in the
direction of the Space's place in the bar. They travel a short distance
while the two desktops crossfade, the fingers driving the progress in a
gesture (For the user 1). At either end of the row a further switch moves
nothing: no bounce and no rubber band (brief P5). Nothing announces the new
desktop: no OSD, no dots (brief, interview); Mission Control shows it.

Entering full screen joins the window's growth (`window-chrome.md`
Choreography) to the switch: the window grows from its box to the output
while the old Space's other windows leave by the switch's motion, so the
window itself never slides. Leaving full screen is the reverse, the view
returning with the window (`compositor.md` Spaces 7).

**Split View.** Two windows sharing a full-screen Space with a divider
(`compositor.md` Split View).

- Entered from the zoom menu's Full screen section ("Left of screen",
  "Right of screen", `window-chrome.md`), the Window menu's Full screen
  tile ▸ (`menu-bar.md`), their chords, or by dropping a window on a
  full-screen Space in Mission Control's bar.
- The window takes its half of a new Space; the free half shows the
  candidates: the origin desktop's other windows as tiles laid out as the
  spread lays out, inside that half (the proposal). Choosing one fills the
  half. A window whose minimum width will not fit the half is shown dimmed
  and is skipped by the keys.
- Cancelling (Escape, or a click on the free half's empty wallpaper)
  returns the first window to its box and removes the Space: the user
  asked for a pair, not a full-screen window.
- The divider moves both windows and stops at either's minimum size
  (Split View 3); a double-click on it returns them to halves. On a
  Split View Space the half chords cycle the focused pane's share through
  half, two-thirds and a third, as they cycle zones (`window-chrome.md`
  For the user 2).
- While a pane's top is revealed, its zoom menu's Full screen section
  shows "Swap sides", "Replace window…" (the candidates again, in that
  half) and Exit Full Screen. Dragging one pane's revealed bar onto the
  other also swaps them.
- Leaving full screen or closing one pane leaves the other alone in its
  full-screen Space (Split View 4).
- Two panes only. A third column on the ultrawide (Split View, Open) is
  not offered: thirds are the zones' job on a desktop, and Split View is a
  full-screen idea from smaller screens.

**Stage Manager.** A layout module chosen per desktop, off by default
(B8, B25; `compositor.md` Stage Manager).

- Turned on for the current desktop from the desktop's menu in Mission
  Control's bar or from Control Centre's Stage Manager module, if
  `control-centre.md`'s gallery takes it; its options in Settings ›
  Desktop.
- The recent apps sit in a column at the leading edge of the work area,
  under the island's strip, as macOS's strip does (the proposal; macOS on
  the left, `design-macos.md` § 3.15). Each entry is an app's window set
  as a small stacked miniature with the app's icon; as many as fit the
  screen's height, the oldest dropping off (still reached by Cmd+Tab and
  Mission Control).
- The front app's windows keep their own arrangement; a set staged for the
  first time is centred in the area beside the column (the proposal's
  "front app centred", changed so a set the user arranged stays arranged).
- "Show recent apps" is off by default, unlike macOS: the column hides,
  and comes back while the pointer presses the leading edge, with the
  hover guards of `design-v1-lessons.md` § 3.5, so the screen at rest is
  still the wallpaper, the island and the windows (brief P1). On, the
  column hides only while a staged window covers it.
- Clicking an entry swaps its set in; the front set goes to the column.
  Dropping a window on an entry moves it into that set; dragging an entry
  onto the stage adds its windows to the front set (macOS). Activating an
  app any other way swaps its set in (Stage Manager 2).
- "Show windows from an application": all at once (default) or one at a
  time, as on macOS.
- Miniatures update when their windows change, at a low rate the packet
  sets, so the column costs nothing while nothing in it changes.
- Mission Control on a Stage Manager desktop spreads every window, staged
  and parked, grouped by set.

**Hot corners.** Each screen corner can run Mission Control, Application
Windows (App Exposé), Desktop (Show Desktop), Notification Centre, Apps
(Spotlight's Apps view), Widgets (the dashboard, `widgets.md`, *added*),
Lock Screen, Put Display to Sleep, or nothing, optionally only with a
modifier held (`shell.md` Mission Control 8). Defaults: bottom left
Mission Control, bottom right Apps; the top corners none, kept for the
menu bar's reveal (B19, `dock.md` For the user 3).

- A corner fires after the pointer rests in it, once per visit; leaving
  re-arms it. Firing the same corner again while its surface is open
  closes that surface.
- It never fires while a mouse button is held (a window drag is the
  quarter zones', a file drag is Cmd+Tab's), while a menu or menu-bar
  surface is open (except the corner that opened it, which closes it),
  over the lock, while the pointer is locked or confined,
  or over a full-screen game (`competition.md`). It fires beside a windowed
  game and over a full-screen app that is not a game.
- Corners where two screens meet are `compositor.md`'s (Pointer, Open).
- Set in Settings › Desktop › Hot Corners… (`settings.md`).

**Gestures.** On a trackpad (`compositor.md` Pointer 6–7): three fingers
up, Mission Control; down, App Exposé; three or four fingers sideways (a
setting), between Spaces; thumb and three fingers spread, Show Desktop;
thumb and three fingers pinched, the Apps view (macOS's Launchpad gesture,
*added*). Each drives its surface's progress with the fingers and settles
on release by distance or speed; the reverse gesture closes. Each has a key
and a pointer route (below).

**Mouse routes.** The host is a desktop with a mouse (`shell.md` Mission
Control): hot corners first; then macOS's mouse shortcuts, a mouse button
for Mission Control, Application Windows and Show Desktop, set in Settings
› Desktop › Shortcuts…, none by default since side buttons are a browser's
back and forward. Inside Mission Control everything is reached by click,
drag and scroll.

## States

| State | What shows |
|---|---|
| Rest | Nothing. Mission Control exists only while open. |
| Open | The Spaces bar, the spread, the tray when there is something in it, on every screen. Keyboard selection on the window that had focus, shown only after the first key (v1 D9). |
| App Exposé | The app's name, its windows in the spread, the set-apart row; on every screen. |
| Empty desktop | The bar and the wallpaper; no text. |
| One window | Its tile at real size, at its place (never larger than real). |
| Many windows | Tiles shrink to fit; every window stays on screen at once (`shell.md` Mission Control 2). Titles only for the hovered or selected tile. |
| Many desktops | Miniatures shrink to a floor, then the bar scrolls sideways; the keys reach every one and keep the selected one in view. |
| Minimised or hidden windows | The tray, in two groups. In App Exposé, the set-apart row. |
| Dragging a window | Desktops in the bar, the + tile, a full-screen Space (Split View) and the other screen's spread mark as targets under the pointer; a Split View Space and a Space on which the window cannot fit refuse it, and the tile returns. |
| A window opens or closes while open | The spread re-lays out, tiles moving to their new places. |
| Close asks to save | The tile stays with the app's dialog drawn on it; choosing it brings both forward. |
| Loading | Each tile appears at once as a frame at its place with the app's icon; its texture fades in when it arrives. The layout never waits on a texture. |
| Error | A window whose texture cannot be had (a protected surface) keeps the icon in its frame. |
| Second screen | Open on both, each with its own bar, spread and tray; a window dragged to the other screen's spread or bar moves there (`compositor.md` Outputs). |
| Full-screen app in front | The bar with every Space; the spread holds that Space's window, or both panes of a Split View. |
| Windowed game in front | As normal, in solid forms and without transitions (`competition.md` For the user 1). |
| Full-screen game in front | Ctrl+↑ and the Mission Control key work (B30's escape set); Ctrl+↓, Ctrl+←/→ and Ctrl+1…9 go to the game. Drawn over the game without motion or blur; the game's pointer lock or confinement is released until it closes and given back on Escape (as the switcher, `gaming.md` Full-screen Spaces 4). Hot corners never open it there. |
| Split View: choosing | The first window in its half, the candidates in the other; dimmed ones are skipped. |
| Stage Manager desktop | The column at the leading edge when shown; in Mission Control every window, grouped by set. |
| A system prompt | Takes focus over Mission Control, which stays open beneath its scrim and gets focus back (`competition.md` Keyboard focus 2). |
| Locked | Locking closes Mission Control; it does not open over the lock. |
| Focus on | No change. |
| Reduce Motion | Mission Control fades in with tiles at their places and fades out; the Space switch is a crossfade; Split View and Stage Manager swaps crossfade; gestures drive the fade. |
| Reduce Transparency | The bar's and the tray's backgrounds take their solid forms; the wallpaper behind the spread is unaffected. |
| Increase Contrast | An outline on every tile and miniature; the current Space's and the selection's marks take their stronger forms. |
| Largest text | Labels and titles scale; tiles and miniatures do not. |

## Opening and closing

- **Mission Control** opens on Ctrl+↑, the Mission Control key, a hot
  corner, a mouse button if set, three fingers up, or Spotlight's
  "Mission Control" action. The same key, Escape, the reverse gesture, the
  same hot corner, or a click on empty wallpaper closes it; choosing a
  window or a Space closes it and acts.
- **Opening**: each window travels from its real rectangle to its tile
  while the bar and the tray fade in at their places (v1 D19). Nothing
  grows from a corner or drops from the top (brief P5). Gesture-driven, the
  windows' travel follows the fingers.
- **Closing** is the reverse and faster (brief P5): each window returns to
  its real rectangle, the chosen one last and on top; if a Space was
  chosen, the Space switch carries the view there. Focus returns before
  any action is delivered (`shell.md` Shell runtime 11).
- **App Exposé** opens on Ctrl+↓, Ctrl with the Mission Control key, a
  hot corner, a mouse button, or three fingers down, for the front app;
  the same routes close it. macOS's ↓ in the switcher is replaced by the
  switcher's own window row (`switcher.md` Answers). The set-apart row
  rises from the bottom edge as the spread forms.
- **Show Desktop** (Cmd+F3, Cmd with the Mission Control key, a hot corner,
  the spread gesture) is `window-chrome.md`'s move.
- **One at a time** with Spotlight, the switcher, the capture tool, the
  session dialog, Force Quit and the widgets dashboard (`shell.md` Shell
  runtime 10). Opening Mission Control closes Control Centre, Notification
  Centre, the revealed menu bar and in-window menus; it hides the dropdown
  terminal.

## Keyboard

Chords marked *proposed* are for the shortcut set (`compositor.md`
Shortcuts and keybinds), which owns them and refuses a clash. Space chords
are taken from the physical Ctrl key only, so Option+arrow word motion
translated to Ctrl stays the app's (`macos-model.md` § 3).

| Action | Key |
|---|---|
| Open or close Mission Control | Ctrl+↑; the Mission Control key |
| Open or close App Exposé for the front app | Ctrl+↓; Ctrl with the Mission Control key |
| Show Desktop | Cmd+F3; Cmd with the Mission Control key (*proposed*, macOS's) |
| Previous or next Space | Ctrl+← / → |
| Space 1 to 9 | Ctrl+1…9 |

Inside Mission Control and App Exposé:

| Action | Key |
|---|---|
| Move the selection | Arrows, spatially; ↑ from the top row enters the bar, ↓ from the bottom row the tray; past the edge toward the other screen, onto it |
| Every item in order | Tab and Shift+Tab |
| Next or previous app (App Exposé) | Tab and Shift+Tab, as macOS |
| Choose (the window, the Space, the + tile, restore from the tray) | Return |
| Look at the selected window at full size | Space, held or pressed again (macOS) |
| Close the selected window | W, Delete or Cmd+W (v1) |
| Remove the selected desktop; exit full screen on a full-screen Space | Delete |
| Move the selected window to the previous or next desktop; move the selected desktop along the bar | Ctrl+Shift+← / → (*proposed*, Mission Control only) |
| Move the selected window to desktop n | Ctrl+Shift+1…9 (*proposed*, Mission Control only) |
| Switch the current Space without closing | Ctrl+← / →, Ctrl+1…9 |
| Open the selected window's menu | Ctrl+Option+Space (`window-chrome.md`) |
| App Exposé for the selected window's app | Ctrl+↓ |
| Close | Escape, Ctrl+↑ |

Split View's candidates: arrows, Return chooses, Escape cancels. Stage
Manager adds no keys: Cmd+Tab swaps a set in. Hot corners and gestures each
have a key above.

Focus: Mission Control takes the keyboard while open and is level 4 of
`competition.md`'s order. The selection starts on the window that had
focus; Return with nothing moved returns to it.

## Pointer

| Input | Where | Does |
|---|---|---|
| Hover | A tile | Highlights it and shows its title and close control, only when the pointer moves (`switcher.md`) |
| Click | A tile | Brings that window forward and closes Mission Control |
| Click | A Space in the bar | Switches to it and closes Mission Control |
| Click | The + tile | Adds a desktop at the end of the row; the view stays |
| Click | Empty wallpaper | Closes Mission Control |
| Middle click | A tile | Closes the window (v1) |
| Secondary click | A tile | The window menu (`window-chrome.md`) |
| Secondary click | A desktop | "Stage Manager" (checked when on), "Remove desktop" |
| Drag | A tile onto a desktop, the + tile, another screen's spread, a full-screen Space | Moves it there; to a new desktop; to that screen; into Split View with that Space's window |
| Drag | A tray tile into the spread, or onto a desktop | Restores it there |
| Drag | A Space along the bar | Reorders |
| Scroll | The bar, when it overflows | Scrolls it sideways |
| Rest | A hot corner | Its action (above) |

No hover opens anything, so no hover guards are needed inside Mission
Control. Stage Manager's edge reveal carries the guards.

## Shared places

| Place or input | Contenders | Proposed winner |
|---|---|---|
| The top strip while Mission Control is open | The Spaces bar; the island and its privacy form | The island keeps its strip, drawn above the overlay; the bar starts under it. The island shows its rest form, privacy marks and OSDs, and takes no hover or click until Mission Control closes. *Adds to `competition.md`* |
| The top edge while Mission Control is open | The menu bar's reveal; the top hot corners | Neither: the bar does not reveal over an overlay; a top corner set to an action still fires after its rest, closing Mission Control first |
| The bottom of the screen | The minimised tray; the bottom hot corners | The tray, with its ends short of the corners, so the corners stay reachable. *Adds to `competition.md`'s Bottom corners row* |
| Centre, overlay | Spotlight, the switcher, the capture tool, the session dialog, Force Quit, the dashboard | One at a time (`competition.md`), unchanged |
| Banners | Mission Control | Banners wait while it is open (`competition.md`), unchanged |
| Picture-in-picture and the dropdown terminal | The spread | Neither is in the spread. PiP stays in its corner under the overlay; the dropdown hides when Mission Control opens (`pip.md`, `dropdown-terminal.md`) |
| Windows kept on top | The spread | In the current desktop's spread, like any window |
| The Space switch against top-layer surfaces | Control Centre, Notification Centre, banners, the revealed bar | Control Centre, Notification Centre and banners stay; the revealed bar and its menus close, since the front app changes; in-window menus and popovers close with their windows. *Adds to `competition.md`* |
| The zoom menu in a Split View | `window-chrome.md`'s Full screen section | Gains "Swap sides", "Replace window…" and Exit Full Screen while the window is in a Split View |
| The leading edge on a Stage Manager desktop | The strip's reveal; the left-half drag region | While dragging, a drop on a strip entry moves the window into that set; elsewhere along the edge, the left-half zone as usual. Without a drag, a press at the edge reveals a hidden strip |
| Ctrl+↑, Ctrl+↓, Ctrl+←/→, Ctrl+1…9 | Apps' own Ctrl bindings; Option-translated word motion | The desktop, from the physical Ctrl key only (`compositor.md` Spaces 2) |
| Ctrl+↑ and the Mission Control key with a game in front | The game | Mission Control: B30's escape set. Ctrl+↓, Ctrl+←/→ and Ctrl+1…9 go to the game. *Names the escape set's Mission Control chords for `competition.md`* |
| Cmd+F3 | Apps | The desktop, before Cmd translation (*proposed*) |
| Three- and four-finger gestures | Apps reading pointer-gestures | The desktop takes the counts it uses; others reach the client (`compositor.md` Pointer 7) |

## Answers

- **How windows are arranged** (`shell.md`, proposal: spread without
  overlap, never larger than real, moving from and back to their real
  places, D19). **Adopted**, with each tile kept as near its real position
  as the layout allows, the app's icon on every tile and the title only on
  the hovered or selected one.
- **How desktops show and are added** (proposal: a Spaces bar along the
  top, live miniatures, an add button). **Adopted**, changed from macOS in
  two ways: the bar is always at full size rather than names until hovered,
  and the + is always shown rather than on hover. Full-screen and Split
  View Spaces sit among the desktops by name.
- **How App Exposé sets minimised and hidden windows apart** (proposal:
  below). **Adopted**: a row below the spread, minimised, then hidden,
  then the app's windows on other desktops, each labelled.
- **How Mission Control shows the desktop's minimised windows.**
  **New**: v1's tray along the bottom, back with no Dock (`dock.md`),
  holding the desktop's hidden apps' windows as a second group, so the
  pointer alone can bring back anything that is not on screen.
- **Closing from Mission Control** (brief Changed, v1). **Adopted**: a
  close control on hover, W, Delete, Cmd+W, middle click, through the
  same close path as Cmd+W (`compositor.md` Apps 8).
- **How Mission Control shows Split View and Stage Manager** (`shell.md`
  Mission Control, Open). **Answered**: a Split View Space is one bar entry
  with both windows and both names; a Stage Manager desktop spreads every
  window, grouped by set.
- **A game's Space among the desktops** (`gaming.md`, proposal: by name
  in the Spaces bar). **Adopted**: a live miniature labelled with the
  game's name and its cover as the label's icon.
- **The Space-switch motion** (`compositor.md` Spaces). **Answered**:
  windows move by direction over a still wallpaper, the island and the top
  layer staying; no bounce at the ends; full screen joins the window's
  growth to it. A short slide with a crossfade, not a full one (For the
  user 1).
- **How Split View's candidates are offered** (proposal: shown in the free
  half). **Adopted**, with cancel returning the first window to its box,
  windows too large for the half dimmed, and Swap and Replace in the zoom
  menu.
- **Where Stage Manager's recent apps sit and how the front app is laid
  out** (proposal: a strip beside the work area, front app centred).
  **Adopted** for the strip, at the leading edge, hidden until the pointer
  presses the edge by default (brief P1). **Changed** for the front app:
  its windows keep the user's arrangement, centred only the first time.
- **Hot corners.** The defaults are B19's. **Adds** Widgets to the
  actions and says when a corner never fires.
- **Gestures.** `compositor.md` Pointer 6 as it stands, **plus** the
  pinch for the Apps view (macOS's Launchpad gesture) and mouse buttons as
  macOS's Mission Control shortcuts.
- **"Automatically rearrange desktops based on most recent use."** Off by
  default, a departure from macOS's default: Ctrl+1…9 and the bar's order
  stay where the user put them, as v1's numbered desktops did (v1 D59's
  stable-order lesson).
- **Named desktops** (COSMIC, `design-references.md` § 6). Not taken:
  macOS has none, and with miniatures always shown a desktop is known by
  its content.
- **Dragging a window to the top edge to open Mission Control** (macOS,
  `macos-model.md` § 3). Not taken: the top edge is the fill target during
  a drag (`window-chrome.md`).

## Copy

Words follow `copy.md`: "desktop", never "workspace" or "Space", in
anything the user reads; macOS's feature names keep their capitals.

- Bar: "Desktop 1"…; a full-screen Space's app or game name; "{app} and
  {app}" for a Split View. Accessible names: "Add desktop", "Remove
  desktop", "Exit Full Screen", "Close"; a miniature "Desktop {n}", with
  "current" as its state.
- Spread: a tile's accessible name "{title}, {app}". Tray and set-apart
  thumbnails: "{title}, minimised", "{title}, hidden", "{title}, Desktop 2".
- Tray and App Exposé labels: "Minimised", "Hidden", "Desktop 2".
- App Exposé with nothing to show: "No windows".
- Split View: "Left of screen", "Right of screen" (`window-chrome.md`),
  "Swap sides", "Replace window…", "Exit Full Screen"; a dimmed candidate:
  "Doesn't fit in Split View".
- Desktop menu: "Stage Manager", "Remove desktop".
- Hot corners: "Mission Control", "Application Windows", "Desktop",
  "Notification Centre", "Apps", "Widgets", "Lock Screen", "Put Display
  to Sleep", "—" (`settings.md` takes these).
- Settings › Desktop, Mission Control group: "Automatically rearrange
  desktops based on most recent use", "Group windows by application",
  "Shortcuts…", "Hot Corners…". Stage Manager group: "Show recent apps",
  "Show windows from an application" with "All at once", "One at a time".
- Settings › Trackpad: "Mission Control", "App Exposé", "Swipe between
  desktops", "Show Desktop", "Apps".

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **How far windows travel in a Space switch.** (a) A full slide, as
   macOS: the old desktop's windows leave by the screen's whole width and
   the new ones arrive from the other side, tracking the fingers one to
   one; (b) a short slide with a crossfade: windows move a small distance
   in the switch's direction while the desktops cross-fade, the fingers
   driving the progress. **Recommended: (b).** On 3440 px a full slide is
   the largest movement the desktop makes, and it happens on every Ctrl+→;
   the short slide keeps the direction readable and is quicker and
   stiller (brief P5), close to v1's workspace fade, which the user lived
   with. (a) is the Mac answer (brief P3) and the better one if the
   trackpad becomes the main route.
