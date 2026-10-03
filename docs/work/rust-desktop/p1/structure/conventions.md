# App conventions

Batch 2. Answers to: `apps.md` For the design process (Conventions: the
unified toolbar, the menu set and order, the toolbar's contents, how a
modal dialog is presented) and the presentation side of `apps.md`
Conventions 3–6, 8–18, 21–23, 28–30; `menu-bar.md` (the app menu, Window
and Help, which this sheet fills in for Isle's apps); `window-chrome.md`
(the unified toolbar, sheets and alerts, sidebars to the edge, which this
sheet builds on); B8, B17, B20, B21, B25, B27.

## Purpose

The rules Isle's five apps (System Settings, Files, Monitor, Keychain,
Cleaner) share, so that they read as one desktop with the shell and with
each other (brief P2), and as Mac apps to a Mac user (brief P3): how a
window is laid out, what its toolbar and menus carry and in what order,
the standard keys, when a task is a sheet, an alert, a popover or a
window, how empty, loading and failed views read, where an app's own
settings live, drag and drop, undo, keyboard focus, what an app remembers,
windows and tabs, and the About window. It is the contract every app sheet
starts from (`settings.md`; `files.md` in batch 3; Monitor, Keychain and
Cleaner with the parity surfaces). It is not the look (stage 4's
components), not any one app's layout, and not the chrome the compositor
draws for other programs (`window-chrome.md`).

## Content

### Window anatomy

Four regions under one toolbar, leading to trailing: **sidebar**,
**content**, **inspector**; the toolbar spans the window's top and carries
the traffic lights (`window-chrome.md`, Unified toolbar; B25).

- **Sidebar.** Navigation: places, panes, views, kinds. It runs to the
  window's top, bottom and leading edge with the lights over its top
  (macOS 27; `window-chrome.md`). Its width is dragged at its trailing
  edge between a minimum and a maximum; below a window width it collapses
  on its own and returns when the window widens. Collapsible sidebars are
  toggled from the toolbar and View › Show Sidebar (⌃⌘S). Rows are
  grouped by spacing, with a heading only where groups are the user's own
  (Files' Favourites, Locations).
- **Content.** The window's subject. It runs under the toolbar, which
  takes a solid bar once content scrolls beneath it (`window-chrome.md`).
  It may split into a list and the selected item's detail, as Passwords
  and Mail do. In a wide window, reading content (forms, detail) keeps a
  maximum line width and centres; tables and grids use the whole width.
- **Inspector.** Optional detail about the selection, at the trailing
  edge, its fill running to the window's top as the sidebar's does (macOS
  27) and its rows starting under the toolbar, dragged to resize, toggled
  from the toolbar and View › Show Inspector (⌥⌘I). An app names it for what it
  holds where macOS does (Files' preview: View › Show Preview, ⇧⌘P, as
  Finder).
- **No bottom bar** by default. A status or path bar is an app's own View
  option (Files, as Finder).

Proposed per app, settled by each app's sheet:

| App | Sidebar | Content | Inspector | Tabs | Windows |
|---|---|---|---|---|---|
| System Settings | Search, account, panes; not collapsible | The pane | None | No | One |
| Files | Places | The folder | Preview | Yes | Any number |
| Monitor | None | The process table; the resource is chosen in the toolbar's centre | None; Inspect opens a window per process, as Activity Monitor | No | One, plus inspect windows |
| Keychain | Kinds and providers | Item list and the selected item's detail | None | No | One |
| Cleaner | Views | The view | The basket, when it holds anything | No | One |

### The toolbar

The groups and their rules are `window-chrome.md`'s (three groups at most;
text and symbol buttons never mixed in a group; one height across every
app; the window's own opaque controls). This sheet fixes the order inside
them:

| Group | In order |
|---|---|
| Leading | The lights; the sidebar toggle, over the sidebar's top trailing corner; back and forward, where the app has history (System Settings, Files); the title, with an optional subtitle (Files' item count) |
| Centre | The view's common controls (view switcher, grouping, sort, the app's frequent actions), folding from the trailing end into a » menu as the window narrows |
| Trailing | At most one primary action; a More menu (…) for the rest; the inspector toggle; the search field, last |

- **Title.** The window's subject (the pane, the folder, the view), never
  the app's name (HIG, `design-macos.md` §3.1).
- **Narrowing**, in order: the centre folds into »; the title truncates;
  the search field becomes a search button that opens the field in place
  when clicked or on ⌘F; the sidebar collapses. The lights, back and
  forward and the trailing group never fold.
- **Tooltips.** A symbol-only button shows its name and its chord after a
  hover dwell. The lights have none (`window-chrome.md`).
- **Not customisable.** No View › Customise Toolbar: every Isle window
  carries the same groups in the same places (brief P2), and an app's
  toolbar is short enough that nothing needs moving.

### Search

- Every window with a list or pages to search has a field on ⌘F (`apps.md`
  Conventions 30), at the trailing end of the toolbar. It filters the
  content in place as the user types; scope options, where an app has
  them, sit in a bar under the toolbar while a search stands (Files, as
  Finder's "Search: This Mac / folder").
- **System Settings is the exception**: its field sits at the top of the
  sidebar, because it searches panes rather than the shown content, as
  macOS's System Settings does (`settings.md`).
- Escape in the field clears it, a second Escape leaves it; clearing
  restores the view as it was. No results is a state (below), never an
  empty pane.

### Menus

Every Isle app exports its whole menu bar (`apps.md` Conventions 8),
shown in the on-demand bar (`menu-bar.md`). The set and order are
`menu-bar.md`'s Answer 11: the app menu, File, Edit, View, the app's own
menus, Window, Help. Every item comes from the app's command table, so a
command with a shortcut sits in exactly one menu (Conventions 10); where
the desktop already carries a command (the Window menu's first groups,
Help's search), the app does not repeat it.

| Menu | Items, in groups | Carried by |
|---|---|---|
| App menu | About {app} · Settings… ⌘, · Hide {app} ⌘H, Hide Others ⌥⌘H, Show All · Quit {app} ⌘Q | Every app, as `menu-bar.md`. Settings… is disabled in an app with no settings window, so the menu keeps its shape |
| File | New Window ⌘N, New Tab ⌘T, Open… ⌘O · Close Tab ⌘W, Close Window ⇧⌘W (⌘W where the app has no tabs), Close All ⌥⌘W · the app's own (Get Info, Compress…) · Print… ⌘P where something prints | Every app, with at least Close Window: the same place for ⌘W everywhere, matching the fallback's File (`menu-bar.md`) |
| Edit | Undo ⌘Z, Redo ⇧⌘Z · Cut ⌘X, Copy ⌘C, Paste ⌘V, Delete, Select All ⌘A · Find ▸ (Find… ⌘F, Find Next ⌘G, Find Previous ⇧⌘G, Use Selection for Find ⌘E) · the app's own · Emoji & Symbols ⌃⌘Space | Every app, since every app has text fields. Find's submenu carries only what the app's search does; Files leaves ⇧⌘G to Go to Folder, as Finder |
| View | Show Sidebar ⌃⌘S, Show Inspector ⌥⌘I · the app's views (⌘1 to ⌘4) · sort, grouping and the app's display options · Show Tab Bar ⇧⌘T in tabbed apps | Apps with any of these |
| The app's own | Go (Files: Back ⌘[, Forward ⌘], Enclosing Folder ⌘↑, the places, Go to Folder… ⇧⌘G), Process (Monitor), Item (Keychain) | Between View and Window, as on macOS |
| Window | The desktop's groups (Minimise, Zoom, Fill, Centre, Move and resize, Full screen tile, Enter Full Screen, Move to desktop, Keep on top); then the app's own: Show Previous Tab ⌃⇧Tab, Show Next Tab ⌃Tab, Move Tab to New Window, Merge All Windows in tabbed apps, and the app's named windows; then Bring All to Front and the window list | The desktop merges it (`menu-bar.md`, The Window menu, group 3) |
| Help | The desktop's search field; {app} Help, opening the app's page in Isle's documentation from its AppStream help link | Every app |

- Enter Full Screen is in the desktop's Window group, not in View as on
  macOS, because the desktop merges it into every app's Window menu and
  the command table allows one place.
- Emoji & Symbols opens the desktop's picker (Spotlight in emoji mode,
  `shell.md` Spotlight 8); the item is how a pointer user finds it.
- **Context menus** open on secondary click, Ctrl-click and the Menu key
  or ⇧F10 on the focused item. They carry the item's actions under the
  same names as their menu items, without shortcut hints and without
  icons (brief, Left out), in the rows `menu-bar.md` defines.

### Standard shortcuts

Isle's apps take Cmd natively (`apps.md` Conventions 12). Every app honours
these where it has the command; an app's command table may add, never
rebind.

| Keys | Command | Owner |
|---|---|---|
| ⌘N, ⌘T, ⌘O | New window, new tab, open | App |
| ⌘W, ⇧⌘W, ⌥⌘W | Close tab or window; close window; close all | App |
| ⌘Q | Quit, asking first when work is in flight (Conventions 5) | App |
| ⌘, | The app's settings window | App |
| ⌘Z, ⇧⌘Z | Undo, redo | App |
| ⌘X, ⌘C, ⌘V, ⌘A | Cut, copy, paste, select all | App |
| ⌘F, ⌘G, ⇧⌘G, ⌘E | Find, next, previous, use selection | App |
| ⌘[, ⌘] and the mouse's back and forward buttons | Back, forward | App |
| ⌘1 to ⌘4 | The app's views | App |
| ⌃⌘S, ⌥⌘I | Sidebar, inspector | App |
| ⌃Tab, ⌃⇧Tab | Next and previous tab | App |
| ⌘I | Get Info or inspect the selection | App |
| ⌘P | Print | App |
| Return, Escape, ⌘. | Default button; cancel | App, in sheets and alerts (`window-chrome.md`) |
| ⌘Delete | Move to Trash, or the destructive button of a sheet | App |
| ⌃F5 | Focus the toolbar (macOS) | App |
| ⌘H, ⌥⌘H, ⌘M, ⌃⌘F, ⌘Tab, ⌘`, ⌘Space, ⌃⌘Space, ⇧⌘/, ⌃F2, ⌃F8, the Ctrl+Option zones | Hide, hide others, minimise, full screen, switching, Spotlight, emoji, Help search, menu bar, Control Centre, zones | The desktop; never reach the app (Conventions 14; `competition.md` Keys) |

Text fields use the Mac's editing keys: ⌥ moves by word, ⌘←/→ to the line's
ends, and the Ctrl motions (⌃A, ⌃E, ⌃K) (Conventions 12).

### Sheets, alerts, popovers and windows

Each kind of interruption has one form. Sheets and alerts are structured
in `window-chrome.md`; this sheet says when each is used.

| Form | Used for | Behaviour |
|---|---|---|
| **Sheet** | A task that must finish or be cancelled before its window goes on: a form (new key, rename many, add a VPN), Files' Open and Save panels in Isle's apps, an item's details to edit (a Wi-Fi network's Details…) | Modal to its window only; Cancel and the action at its foot, the default trailing; never Cancel and Done beside Back (HIG, `design-macos.md` §3.1). One sheet at a time: a step inside a task replaces the sheet's content with a Back, rather than stacking a second sheet |
| **Alert** | A confirmation or a failure the user must read: something that cannot be undone, quitting with work in flight, a failed action | A sheet with a title, at most one line of text (a length for the copy: it wraps, never cut) and up to three buttons (`window-chrome.md`). A "Don't ask again" box only on a confirmation the user may sensibly silence, and the app's settings can turn it back on. With no window open, the alert is a small window of its own, centred |
| **Popover** | Quick detail or a small edit about one control or item: a colour, a date, a row's info | Anchored to what opened it; not modal; changes apply as made, so it has no OK or Cancel; closes on Escape, a click outside, or the opening control again; one open at a time. Drawn on the window's raised material, never glass (`design-v1-lessons.md` § Files and the other windows) |
| **Window** | Something worth keeping open beside others: Get Info, a process inspector, About, the app's settings | A window of the app, placed by the compositor (B8), with its own lights; in the switcher's window row and the Window menu |

- **Confirm or undo.** An action that can be undone acts at once and
  offers Undo (Move to Trash); one that cannot asks first, in an alert
  naming the loss (Empty Trash, Delete Immediately, Remove Account), with
  Cancel as the default (`window-chrome.md`: a destructive action is never
  the default).
- A privileged change asks once, in the desktop's polkit dialog (B12;
  Conventions 25), never in the app. A sheet waits under the prompt.

### Empty, loading and error states

Every view has a loading state distinct from an empty answer (Conventions
22) and a state for a missing daemon or device (Conventions 23). All three
sit in the place of the content they concern, never in a modal.

- **Loading.** The window's frame (toolbar, sidebar, headings) draws in
  the first frame; the content draws what it has and fills as data
  arrives (a listing streams, `apps.md` Files 1). A small indeterminate
  progress shows in the content only if nothing has arrived after a short
  wait (stage 4), so a fast answer shows none. A row waiting on one value
  shows its label and no value, never a spinner per row. No skeleton
  cards.
- **Empty.** One line in the content's centre saying what will appear
  ("No items in Trash"), and at most one button where there is an obvious
  first step ("Add Calendar…"). No illustration, no card of filler
  (`copy.md` Empty states; `design-v1-lessons.md` §8).
- **No results.** "No results for “{query}”" in the content's centre, the
  field kept as typed.
- **Missing daemon or device.** A sentence in the place of what it affects,
  what happened then what to do (`copy.md` rule 7): "No Bluetooth adapter
  is connected." Rows that do not depend on it keep working.
- **A failed action.** An alert on the window, naming what failed and what
  to do, with Try Again where trying again can help. A part that failed in
  a larger job is listed with the job's result, not one alert per item.
- **A refused write.** A desktop setting refused because `settings.kdl`
  does not parse: the control keeps its last good value and the window
  shows the configuration notice (`settings.md`, Rejected hand edit; B27).
  A value the daemon or the schema refused stays on its row with the reason
  under it (`apps.md` Settings 6).
- **Work finishing unseen.** A job that ends while its window is not in
  front posts a notification; while the window is in front it shows only
  in the window. Progress is in the window and on the switcher card
  (`dock.md`; Conventions 4).

### An app's own settings

Each app with preferences has a settings window on ⌘, and App › Settings…,
as on macOS. Its values are the app's section of `settings.kdl`, written
through the compositor (B27; Conventions 19).

- **A window, not a sheet**, one per app, titled with the shown pane's
  name. Its panes are tabs in its toolbar, symbol over label, as macOS
  apps' settings windows are (Finder: General, Sidebar, Advanced); an app
  with one pane has no tabs. The window resizes (Conventions 16) and its
  form keeps a fixed column, centred.
- **The same rows and controls as System Settings** (`settings.md`, Rows
  and Control types): changes apply as made, no Apply or OK.
- **Where a setting lives.** What changes how the app behaves inside its
  windows is the app's (Files' new-window folder, Keychain's clipboard
  timeout). What the desktop applies to an app from outside is System
  Settings', even for Isle's apps: notifications (Notifications), keep
  running and Cmd translation (Apps), open at login (General › Login
  Items).
- **The configuration notice** shows at the top of every pane while
  `settings.kdl` holds a rejected edit, as in System Settings.
- **Found from Spotlight.** Each app ships its settings rows in the same
  index format as System Settings (`apps.md` Settings 4), so Spotlight
  finds "Files Settings › Sidebar" and opens the window at that row.
- Monitor has no settings window, as Activity Monitor: its update
  frequency is in View. System Settings has none either.

### Drag and drop

- Anything that stands for a file, item or text can be dragged out of its
  view to another app as `text/uri-list` and text, and onto any target
  inside the app that takes it (`apps.md` Files 17).
- A target marks itself only while something it accepts is over it; a
  target that refuses shows nothing. The pointer carries a + while the
  drop will copy and the drag image shows a count for several items.
- Modifiers as Finder: a drop on the same volume moves, on another copies;
  ⌥ forces a copy, ⌘ a move.
- Hovering a folder or a tab during a drag opens it after a dwell
  (spring-loading), as Finder.
- Reordering by drag only where the order is the user's (Files'
  Favourites, Focus modes' allowed apps, the Apps grid's favourites).
- Escape cancels a drag; a cancelled or refused drop returns the image to
  where it came from.
- Every drag has a keyboard route: Cut and Paste, a Move To… item, or Move
  Up and Move Down in the item's context menu (Conventions 28).
- ⌘Tab during a drag switches apps and the drop lands in the new front
  window (`switcher.md`, During a drag).

### Undo

- Edit › Undo and Redo are always present, named for what they undo
  ("Undo Move to Trash", "Redo Rename"), disabled when there is nothing.
- ⌘Z goes to the focused text field first, then to the window's history,
  as on macOS. History is per window and ends with it; quitting forgets it.
- What is undoable is each app's: Files' file jobs (`apps.md` Files 7),
  the Cleaner's basket (one undo, Cleaner 10). System Settings has no undo,
  as on macOS: a change applies at once and is changed back the same way,
  with Reset to default where the changed-settings view is on
  (`settings.md`). Keychain's deletes cannot be undone and ask first.

### Keyboard focus and full keyboard access

- **Tab reaches every control** (Conventions 28), always, as macOS's
  "Keyboard navigation" turned on; there is no setting to narrow it.
- **The loop**: sidebar, content, inspector, toolbar, and round again;
  within a region in reading order. A list, grid, table or segmented
  control is one stop, moved inside with the arrows; a group of radio
  buttons is one stop. Each region remembers its last focused control.
- **On opening**, focus is on the window's main list: the sidebar in System
  Settings, the folder in Files, the table in Monitor. ⌃F5 moves focus to
  the toolbar.
- **Type to select** in every list and sidebar.
- Space presses the focused button or toggles a switch; Return takes a
  sheet's default button and opens the focused list item.
- **Focus returns** to the control that opened a sheet, popover or menu
  when it closes, and to the window's last focused control when the window
  is activated again.
- The focus ring shows for keyboard input only (Conventions 28;
  `shell.md` Shell runtime 12), on every control, in the accent (brief
  P6).
- Every control has a role, a name and a state for AccessKit (Conventions
  29); a symbol-only button's name is its tooltip.

### What an app remembers

| What | Where | Restored |
|---|---|---|
| Where each window sits and its size | The compositor's window places (B8; `apps.md` Conventions 6) | By the compositor, never by the app |
| Preferences from the app's settings window | The app's section of `settings.kdl` (B27) | Always |
| Each window's content: pane, folder, tabs, view, sort, grouping, column widths, the sidebar's width and whether it is collapsed, the inspector | The app's own domain in state (`platform.md` State), never in `settings.kdl` | When the app reopens its windows (For the user 1) |
| Search text, selection, scroll position within a pane, open sheets, popovers and menus, a half-filled form, a revealed secret | Nothing | Not restored |

### Windows and tabs

- **Single instance** (Conventions 3): launching an app that is running
  raises it, and a second launch's arguments (a folder, a pane) open in the
  running app.
- **One window** in System Settings, Keychain and Cleaner: ⌘N is absent
  and the Window menu names the one window. Monitor has its main window and
  a window per inspected process. Files has any number of windows, each
  with tabs.
- **Tabs** (Files): the tab bar sits under the toolbar and shows only while
  a window has two or more tabs, unless View › Show Tab Bar is on. A tab
  dragged out of the bar becomes a window; dragged onto another window's
  bar, it joins it. Window › Move Tab to New Window and Merge All Windows
  do the same from the keyboard. A middle click on a folder opens it in a
  tab (`apps.md` Files 17).
- **The last window closing** ends the app unless work is in flight
  (Conventions 4); B20's keep-running list does not apply to Isle's apps.
- **Quitting with work in flight** (Conventions 5): an alert on the front
  window naming the work ("Copying 3 items to “Backup”"), "Quitting stops
  it.", with Cancel (default) and Quit.

### The About window

App › About {app} opens Isle's About window for the app, the same panel
`menu-bar.md` gives any app without its own:

- the app's icon and name; "Version {v}" with the git commit Isle was built
  from, which can be selected and copied (B29); the one-line description
  from its desktop entry; the licence; a link to the source;
- a small window of fixed size, centred on its app's screen the first time,
  with the lights (minimise and zoom disabled, as for any fixed-size
  window, `window-chrome.md`). It is the one fixed-size window Isle's apps
  open, an exception to Conventions 16 that `apps.md` notes at the gate;
- one per app; choosing About again raises it; ⌘W and Escape close it.

## States

| State | What shows |
|---|---|
| Rest, active | The window with its toolbar, sidebar and content; the lights coloured (`window-chrome.md`) |
| Inactive | Lights neutral; toolbar symbols, sidebar and titles subdued; the selection kept but drawn without the accent (macOS) |
| Empty | The empty line in the content (above) |
| Loading | The frame at once; content as it arrives; a progress mark only after a wait |
| Error | Inline in the affected place; a failed action as an alert |
| Configuration error | The notice at the top of every settings pane; desktop-setting rows disabled (`settings.md`) |
| Work in flight | Progress in the window and on the switcher card; Quit asks first |
| With a sheet | The window's content dimmed; the lights work except close, which waits on the sheet (`window-chrome.md`) |
| Narrow window | The toolbar folds in the order above; the sidebar collapses; the window never shrinks below the size that holds its largest sheet (Conventions 16) |
| Overflowing content | Content scrolls under the toolbar, which turns solid; the sidebar scrolls on its own |
| Full screen | The toolbar stays; the on-demand menu bar reveals over it from the top edge (`window-chrome.md`) |
| Second screen | No difference; the window is placed and scaled by the compositor |
| Game in front | Isle's apps are ordinary windows; while Game Mode is on they open and close without transitions (`competition.md` For the user 1) |
| Locked | Nothing shows; the lock covers windows |
| Focus on | No change; notifications from the app follow the Focus |
| Reduce Motion | Sheets, popovers and sidebar collapse fade instead of moving |
| Reduce Transparency | Sidebar and inspector solid; toolbar solid at all times |
| Increase Contrast | Borders on controls, fields, the toolbar's edge, the sidebar's edge and sheets (macOS 27's Show Borders, `design-macos.md` §4.6) |
| Largest text size | Toolbar, rows and sidebar grow with the text; labels wrap rather than truncate in forms; the minimum window size grows so nothing is clipped |

## Opening and closing

- **An app's first window** maps only when its first complete frame is
  drawn (`apps.md` acceptance bar 1), with the compositor's open motion
  (`window-chrome.md` Choreography). It reopens where the compositor last
  placed it; a new window with no place opens centred on the screen of the
  focused window.
- **Closing** a window is ⌘W, the close light or File › Close Window; the
  compositor's close motion. The last window closing ends the process
  (Conventions 4).
- **Sheets** grow down from the toolbar's lower edge and leave faster
  (`window-chrome.md`). **Popovers** grow from their anchor and fade out
  faster. **Context menus** open at the pointer, or under the focused item
  from the keyboard.
- **Sidebar and inspector** slide in from their edge as the content
  narrows to make room; collapsing reverses it.
- **Settings and About windows** open as windows, placed by the
  compositor; ⌘, again raises the settings window at its last pane.

## Keyboard

The table under Standard shortcuts is the whole map; focus moves as
Keyboard focus describes. In addition:

| Key | Action |
|---|---|
| Tab, Shift+Tab | Next and previous control, through every region |
| Arrows | Within a list, grid, table, segmented control or radio group |
| Space | Press the focused button, toggle the focused switch; Quick Look in Files |
| Return | The default button; open the focused item |
| Escape | Close the popover, menu or sheet (as Cancel); clear then leave the search field |
| ⌃F5 | Focus the toolbar |
| Menu key, ⇧F10 | The focused item's context menu |

## Pointer

- **Click** selects; **double-click** opens; **secondary click** and
  Ctrl-click open the context menu for the item under the pointer, keeping
  a selection it is part of (`apps.md` Files 17).
- **Hover** opens nothing in Isle's apps, except a dwell over a folder
  during a drag (spring-loading) and the tooltips of symbol buttons.
- **Empty toolbar space**: drag moves the window; double-click runs the
  title-bar action; secondary click opens the window menu
  (`window-chrome.md`).
- **Sidebar and inspector edges** drag to resize; a double-click on the
  edge returns the default width.
- **Mouse back and forward buttons** go back and forward where the app has
  history.
- **Scroll** scrolls the region under the pointer, never a control the
  pointer happens to cross while the region scrolls.

## Shared places

| Place or input | Competes with | Proposed winner |
|---|---|---|
| Desktop chords (⌘H, ⌥⌘H, ⌘M, ⌃⌘F, ⌘Tab, ⌘`, ⌘Space, ⌃⌘Space, ⇧⌘/, ⌃F2, ⌃F8, the zones) | An app's own shortcuts | The desktop (`competition.md` Keys). Isle's apps bind none of them; their menus show the desktop's items where the desktop carries them |
| ⌃F5 | Nothing on the desktop | The app: focus to the toolbar. Not added to P5's chord table |
| The Window menu | The desktop's groups and the app's own items | Merged: the desktop's groups first, the app's own as group 3 (`menu-bar.md`). Enter Full Screen only in the desktop's group |
| Help's search | The app's own Help items | The desktop's search field leads; the app's items follow (`menu-bar.md`) |
| A window's top edge | The toolbar against the revealed menu bar | The bar, while revealed (`window-chrome.md`) |
| In-window menus, pop-up buttons and popovers | The top layer's menu-bar surfaces | Popups of their own window, drawn above it in the windows layer and under the top layer. Opening the on-demand bar, Control Centre or Notification Centre, or any surface taking focus, closes them. *Added for `competition.md` Layers* |
| A sheet's focus | System prompts | The prompt, above the sheet; the sheet waits (`competition.md` Keyboard focus, 2 over 6) |
| Long work's progress | The window and the switcher card | Both: the window while it is open, the switcher card always (`dock.md`) |
| A job's result | The window and a notification | The window while it is in front; a notification only when it is not |
| Where a window opens | The app and the compositor | The compositor (B8): an app never places itself, its settings and About windows included |
| Search | ⌘F in the app and ⌘Space for Spotlight | Separate: ⌘F searches the window, ⌘Space the system |
| Text size | The system's text size and a per-app size | The system's only; Isle's apps take no per-app override |

## Answers

- **The unified toolbar's look** (`apps.md` Conventions). **Split**: its
  structure is answered here and in `window-chrome.md` (groups and their
  order, folding, one height, the solid bar on scroll); its look is stage
  4's toolbar component.
- **The toolbar's contents** (`apps.md`, answered in `window-chrome.md`).
  **Adopted** and completed: the order within each group, search last at
  the trailing end, how it folds, tooltips, and no customisation.
- **The menu set and order** (`apps.md`: app menu with About, Settings…,
  Hide, Hide Others, Quit; then File, Edit, View, Window, Help).
  **Adopted** as `menu-bar.md` Answer 11 has it, and filled in: the
  standard items of each menu, File always present for ⌘W, the app's own
  menus between View and Window, Enter Full Screen in the desktop's Window
  group only, Emoji & Symbols at the end of Edit.
- **How a modal dialog is presented** (proposal: a sheet). **Adopted**
  (`window-chrome.md`), and extended with when a task is an alert, a
  popover or a window instead.
- **Window anatomy** (B25, `window-chrome.md`). **New**: sidebar, content,
  inspector under one toolbar, with each app's use proposed.
- **Search placement.** **New**: the toolbar's trailing end on ⌘F,
  filtering in place; System Settings' field at the top of its sidebar.
- **Standard shortcuts.** **New**: the Mac set, with the desktop's chords
  marked as never reaching an app.
- **Empty, loading and error states** (Conventions 22, 23). **New**: in
  place, a line not a card, a progress mark only after a wait, failures as
  alerts, refused writes on their row or by the configuration notice.
- **Settings windows per app.** **New**: a window on ⌘, with toolbar tabs,
  System Settings' rows, the app's section of `settings.kdl`; desktop
  policies about an app stay in System Settings.
- **Drag and drop, undo, keyboard focus, what an app remembers, windows
  and tabs, the About window.** **New**, each above, following Finder and
  AppKit's defaults except where Conventions 28 asks more (Tab reaches
  every control).

## Copy

Standard menu items keep Apple's title case (`copy.md` rule 1); Isle's
own phrases are in sentence case.

| Where | String |
|---|---|
| App menu | "About {app}", "Settings…", "Hide {app}", "Hide Others", "Show All", "Quit {app}" (`menu-bar.md`) |
| File | "New Window", "New Tab", "Open…", "Close Tab", "Close Window", "Close All", "Print…" |
| Edit | "Undo", "Redo", "Undo {act}", "Cut", "Copy", "Paste", "Delete", "Select All", "Find", "Find…", "Find Next", "Find Previous", "Use Selection for Find", "Emoji & Symbols" |
| View | "Show Sidebar", "Hide Sidebar", "Show Inspector", "Hide Inspector", "Show Tab Bar", "Hide Tab Bar" |
| Window (app's group) | "Show Previous Tab", "Show Next Tab", "Move Tab to New Window", "Merge All Windows" |
| Help | "{app} Help" |
| Search field | Placeholder "Search" |
| No results | "No results for “{query}”" |
| Quit with work in flight | Title naming the work ("Copying 3 items to “Backup”"), "Quitting stops it.", buttons "Cancel", "Quit" |
| A failed action | What failed, then what to do; "Try Again" where it can help |
| Don't ask again | "Don't ask again" |
| About | "Version {v} ({commit})", "From {package}" |
| Settings window | Pane names as tabs; the window's title is the shown pane's name |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Reopening an app's windows.** `apps.md` Conventions 6 reopens an app's
   windows, with their content, the next time it launches (v1 D62). macOS
   does the opposite by default: its Desktop & Dock switch "Close windows
   when quitting an application" is on, so a relaunched app opens fresh.
   (a) Reopen, as Conventions 6, with macOS's switch in Settings › Desktop,
   off by default; (b) macOS's default, the switch on. **Recommended:
   (a).** v1 reopened and the user kept it, a quit Files or Settings
   coming back where it was is what Conventions 6 is for, and the switch
   gives the Mac behaviour to anyone who wants it.
