# Files

Batch 3. Answers to: `apps.md` For the design process (Files: how places
are laid out; where the preview sits; the toolbar's contents; how a file
job's question and progress are shown) and the presentation side of
`apps.md` Files 1–26 and the Parity map's Files rows; `apps.md` Files'
"Open for its packet" items where they are presentation (tags, undo,
Connect to Server, a Gallery view, Quick Look's interface, how the chooser
binds to its window); `capture.md` (trimming in Quick Look; Markup's
place); `dock.md` (Trash; dropping files on an app); `menu-bar.md` For the
user 1 (Files as the front app with no window focused); `wallpaper.md`
(Set as wallpaper); `shell.md` Spotlight 10 and Capture 4; B13, B17, B21,
B25, B26, B27.

## Purpose

Finder for Isle: browse, find, move, open and throw away files, in any
number of windows with tabs, with Quick Look and Get Info, and the Open and
Save panels every other app gets (B26). A Mac user should find Finder's
places, views, menus and keys where they expect them (brief P3), tested by
use rather than by feature list (`design-v1-lessons.md` § Files and the
other windows; `main:docs/AUDIT.md` § Files). It is not a desktop (Isle
shows no files on the wallpaper, `wallpaper.md`), not Spotlight (system
search and launching), not the Cleaner (where the space went), not a
two-pane or terminal file manager: on the ultrawide two Files windows
tiled side by side are the two panes, and Open in terminal is the
terminal (`design-references.md` §14, Dolphin: not taken, below).

## Content

The window follows `conventions.md`'s anatomy: sidebar, content,
inspector under one toolbar.

```
┌──────────────────┬──────────────────────────────────────────────────────────────┐
│ ● ● ●       [◧]  │ ‹ › Projects    [▦ ☰ ▥] [⊞▾]       (◔) […] [◨] [⌕ Search ]   │  toolbar
│                  ├──────────────────────────────────────────────────────────────┤
│ Recents          │ [ Projects ][ isle ][ Downloads ] +                          │  tab bar, 2+ tabs
│                  ├────────────────────────────────────────────┬─────────────────┤
│ Favourites       │ Name          Date Modified   Size  Kind   │                 │
│  Home            │ ▸ hyprland    Today 14:02      --   Folder │   [ preview ]   │
│  Applications    │ ▸ isle-wid…   Yesterday        --   Folder │                 │
│  Documents       │   notes.md    2 Oct          4 KB   Markd… │ notes.md        │
│  Downloads       │   …                                        │ Markdown, 4 KB  │
│  Development     │                                            │ Modified …      │
│  Builds          │                                            │ Show More       │
│                  │                                            │                 │
│ Locations        │                                            │                 │
│  cachyos         │                                            │                 │
│  Backup       ⏏  │                                            │                 │
│  Trash           │                                            │                 │
└──────────────────┴────────────────────────────────────────────┴─────────────────┘
                    path bar and status bar below, when turned on in View
```

### Places: the sidebar

Rows in three groups; no Tags group for now (For the user 1), which would
come fourth if tags are built later. A heading only over groups that are
the user's own (`conventions.md`):

| Group | Rows | Data |
|---|---|---|
| (none) | **Recents**: files recently opened, newest first, grouped by day | the shared recently-used list (`apps.md` Files 12) |
| **Favourites** | Home, then the xdg-user-dirs folders that exist in macOS's order (Applications when `~/Applications` exists, Desktop, Documents, Downloads, Music, Pictures, Videos), the user's bookmarks, and saved searches, all in the user's order | xdg-user-dirs; GTK's bookmarks file read and written as GTK writes it (D69); the order and saved searches in Files' state (D74, D76) |
| **Locations** | The computer, by its name, opening a view of every volume; each other volume UDisks2 reports, mounted or not; network shares the system has mounted; **Trash** last | UDisks2 (B11); the mount table; every volume's trash (`apps.md` Files 4) |
| **Tags** | Not built for now: the place the user's tags would take, each opening a search for it | For the user 1 |

- **Saved searches sit in Favourites**, as Finder's do when added to the
  sidebar, and are dragged among the folders like any favourite. Saving
  a search always adds it; Remove from Sidebar deletes it.
- **Templates and Public** are off by default; Files › Settings › Sidebar
  turns any row or group on or off, Recents and Trash included.
- **Volumes**: an unmounted one is listed dimmed and mounts on a click,
  through UDisks2 with polkit where it asks. An encrypted one asks for its
  passphrase in a sheet on the window, with "Remember in Keychain". A
  removable or network volume has an Eject button at the row's trailing
  edge.
- **Free space** is not drawn on every row (brief P1). It is in the row's
  tooltip and accessible description, the status bar and Get Info; a
  volume past nine-tenths full carries a small capacity mark on its row,
  since that is when it has something to say. *Changed* from `apps.md`
  Files 11's "by name with its free space".
- **Trash** shows full or empty in its symbol, never a count. It is a drop
  target, and the place `dock.md` moved the Dock's Trash to.
- Section headings collapse on a click, as Finder's; the state is kept.

### The content: views

Three views, each over the same gathering (grouping), sort and selection,
each opening folders in place (`apps.md` Files 14):

| View | Shape | Specific to it |
|---|---|---|
| **Icons** ⌘1 | A grid of thumbnails with names under them, always arranged by the sort; no free placement | Icon size from View Options or ⌘+ and ⌘−; names wrap to two lines, then truncate in the middle |
| **List** ⌘2 | Rows with columns: Name, Date Modified, Size, Kind by default; Date Created and Date Last Opened, and Tags and Comments if tags are built later, from the header's menu; disclosure triangles open a folder in place | Click a header to sort, again to reverse; drag to reorder or resize columns; folders show "--" for size unless View Options' Calculate all sizes is on |
| **Columns** ⌘3 | A column per level of the path, the selection's preview as the last column | Drag a divider to resize; double-click fits the longest name; ⌥ resizes all; a divider's menu carries Right Size This Column, Right Size All Columns Individually, Right Size All Columns Equally (macOS 27's autosize and reset, `design-macos.md` §3.17) |

- **No Gallery view.** Icons at a large size with the preview open does
  what Gallery does; ⌘4 stays unbound. *Decided here* (`apps.md` Open for
  its packet): the user's folders are code and games, not photo albums.
- **Groups** (View › Use Groups ⌃⌘0): by Kind, Date Modified, Size, and
  Tags if built later, with a heading per group; each group sorted by the
  sort. Off by default.
- **Remembered per folder**: view, sort, grouping, icon size and list
  columns, with View Options' Use as Defaults setting the default for
  folders never set (`apps.md` Files 14). In Files' state, never in
  `settings.kdl` (`conventions.md`, What an app remembers).
- **Folders on top** when sorting by name (Finder's "Keep folders on top",
  on by default here: Finder ships it off, but a coding machine's folders
  are full of folders and Linux file managers all put them first).
- **Filename extensions shown** (Finder's "Show all filename extensions",
  on by default here, off on macOS): on Linux the extension is often the
  only type a file has, and the user reads code by it.
- **Hidden files** (⇧⌘.) show dimmed; the toggle is app-wide, as Finder's.
- **View Options** (⌘J): a popover from the view switcher, as its changes
  apply at once: Group By, Sort By, icon size and Show item info (Icons),
  the columns shown (List), Calculate all sizes, Use as Defaults.
- **Status bar** (View › Show Status Bar ⌘/): "{n} items, {free}
  available", the selection's count and size, and in Icons the icon-size
  slider. **Path bar** (View › Show Path Bar ⌥⌘P): the folder's path, each
  segment a drop target. Both off by default (`conventions.md`, No bottom
  bar).

### The preview

The inspector region (`conventions.md`), named for what it holds: View ›
Show Preview ⇧⌘P and the toolbar's preview toggle.

- **In Icons and List**, a pane at the trailing edge, full height under the
  toolbar, dragged to resize. **In Columns**, the last column, and the
  pane does not show; the one toggle governs both (`apps.md` For the
  design process: adopted).
- **Content**: the item's preview by Quick Look's renderer (one renderer,
  below), its name and kind and size, then Created, Modified and Last
  Opened, the facts its kind has (dimensions, duration, page count, an
  archive's item count), tags if built later, and Show More, which opens Get Info. One
  action, Markup, for an image. Several selected: a stack of their
  thumbnails, "{n} items", the total size.
- A folder previews its icon, item count and, once counted on a worker,
  its size.

### The toolbar

`conventions.md`'s groups, filled:

| Group | Controls |
|---|---|
| Leading | The lights; the sidebar toggle; back and forward; the title, the folder's name with "{n} items" as its subtitle |
| Centre | The view switcher (Icons, List, Columns, a segmented control); the Group and Sort pop-up (⊞▾), whose menu is View's Use Groups, Group By and Sort By |
| Trailing | The jobs button (◔), only while a job runs, waits on a question or has failed; the primary action where the folder has one (Empty Trash… in the Trash; nothing elsewhere); More (…), carrying the selection's context menu, or the folder's with nothing selected; the preview toggle; the search field |

- **The title** carries the folder's icon as a proxy: dragged, it carries
  the folder; ⌘-click or secondary click on the title opens the path menu
  of enclosing folders (Finder). In the Trash it reads "Trash"; in a
  search, "Searching “{folder}”" or the saved search's name.
- Narrowing folds the centre into », then truncates the title, then turns
  the search field into a button (`conventions.md`, The toolbar).

### Tabs and windows

`conventions.md`'s Windows and tabs, with Finder's specifics:

- **A new tab** (⌘T, + at the bar's end) opens at the new-window folder,
  as Finder's does; ⌘-double-click or a middle click on a folder opens
  that folder in a new tab.
- **The tab bar** gives tabs equal widths down to a minimum, then scrolls,
  keeping the selected tab in view (v1, `main:docs/AUDIT.md` § Files);
  each tab is its folder's name, with a close button on hover and a
  middle click to close. A tab's menu: New Tab, Close Tab, Close Other
  Tabs, Move Tab to New Window.
- **The new-window folder** is Home by default, Files › Settings › General
  sets it. Finder opens Recents; here Recents depends on apps writing the
  shared list, which most Qt, Electron and terminal tools do not, so it
  would open on a list that misses most of the user's work.
- **Show in folder** from another app (`org.freedesktop.FileManager1`
  ShowItems, ShowFolders; Spotlight's Cmd+Return; a capture's Show in
  Files): a new tab in the front Files window on the current desktop, the
  item selected and scrolled to; a new window when Files has none there.
  ShowItemProperties opens Get Info. A stream of reveals from an editor
  stacks tabs rather than windows.

### Searching

On ⌘F, the toolbar's field (`conventions.md`, Search). Typing filters the
shown folder at once. A scope bar under the toolbar while a search stands:

- **Scope**: "This folder" (filtering as typed) and "Everything under
  “{folder}”" (on a worker, filling as it goes; Return in the field
  switches to it, as v1's Enter searched underneath), and "Home"; Files ›
  Settings › Advanced picks which one a new search starts in (`apps.md`
  Files 13).
- **Criteria**: a + adds a row: Kind is (Any, Folder, Document, Image,
  Audio, Video, Archive, Code, App), Last modified (today, the last 7 days,
  the last 30 days, this year, a range), Tags is (if built later), Contents
  contain, on the same content search Spotlight's Contents switch uses,
  with its limits (`spotlight.md`).
- **Results** show in List with a Where column, or Icons; Columns turns
  into List while a search stands. Selecting a result shows its path in
  the path bar.
- **Save** at the scope bar's end names the search in a sheet and adds it
  to Favourites. Reopened, it runs again with its term, scope and criteria
  (`apps.md` Files 13; D74).

### Acting on files

Commands are in File and Edit and the item's context menu
(`conventions.md`, Context menus), under the same names.

- **New Folder** (⇧⌘N) and **New file** (⌥⇧⌘N) appear in place with their
  name field open, never in a card (v1 audit's fault); New Folder with
  Items (⌃⌘N) gathers the selection. With files in `~/Templates`, New file
  becomes a submenu of them.
- **Rename** on Return: the name is edited in place with the name selected
  and the extension not. Return commits, Escape cancels. A taken name
  keeps the field open with "“{name}” is already taken. Choose a different
  name." under it. A name
  starting with a dot asks first ("A name that begins with a dot makes the
  item hidden."). Changing an extension asks while Settings › Advanced's
  warning is on, with "Keep .{old}" as the default and "Use .{new}".
- **Rename many** (several selected, Return or Rename {n} Items…): a sheet
  with one name field where `#` stands for the number, a start number,
  and an example line showing the first and last result live (`apps.md`
  Files 8; D71).
- **Duplicate** ⌘D, **Make Alias** ⌃⌘A (a symbolic link, "{name} alias"),
  **Show Original** ⌘R, **Compress** ("Archive.zip" for several, as
  Finder), **Expand** (into a folder beside it; also a double-click on an
  archive with no default app set), **Copy “{name}” as Pathname** ⌥⌘C.
- **Cut, Copy, Paste**: ⌘C then ⌘V copies; ⌘C then ⌥⌘V moves, as Finder;
  ⌘X then ⌘V moves too, as Linux file managers and v1 did. Finder leaves
  ⌘X disabled for files, so taking it costs a Mac user nothing.
- **Open With**: the default first, marked, then the other apps for the
  type, then Other…, a sheet of every app with "Always open with" (writes
  `mimeapps.list`) (`apps.md` Files 18). This is how a file reaches an app
  the Dock used to take drops for (`dock.md`).
- **Open in terminal** opens the user's terminal in the folder, or the
  item's folder. **Set as wallpaper** on an image, through the wallpaper
  setting (`wallpaper.md`).
- **An AppImage** in `~/Applications` is an app: Move to Trash uninstalls
  it and Put Back reinstalls it (`apps.md` Files 20); its kind reads
  "Application (AppImage)".

### File jobs: progress and questions

A job is any copy, move, trash, put back, delete, compress or expand
(`apps.md` Files 4–9). `conventions.md` puts progress in the window and on
the switcher card; here is how.

- **In the folder.** An item being written appears in its destination at
  once, dimmed, with a progress bar across its icon or row, and takes its
  normal form when done, as Finder's. A moved item leaves its source when
  its move completes.
- **The jobs button** (◔) in the toolbar's trailing group of every Files
  window, shown only while a job runs, waits or has failed (brief P1). Its
  ring fills with the total of the running jobs. It opens a popover with a
  row per job: "Copying {n} items to “{folder}”", a bar, "{done} of
  {total}, about {time}", and a stop button. A job that ends leaves its
  row after a moment; a short job (under a second or so, stage 4) shows no
  button at all.
- **The switcher card** carries the total progress of running jobs
  (LauncherEntry, `switcher.md`). The island carries none (`island.md`
  For the user 3).
- **The end.** While a Files window is in front, the window is the
  report. Otherwise a notification from Files: "Copied {n} items to
  “{folder}”", or "Copied {k} of {n} items to “{folder}”" with Show
  (`conventions.md`, Work finishing unseen). Focus holds it as any other.
- **A question** (a name already taken, `apps.md` Files 6) is an alert
  sheet on the window that started the job, or on another Files window if
  that one closed, or on a small window of its own if none is open. The
  job waits; its row reads "Waiting"; the window asks for attention, so
  the switcher marks Files and the island shows its attention event
  (`island.md`). For a file: "An item named “{name}” already exists in
  “{folder}”.", one line saying which is newer, "Apply to all" for the rest
  of the job, and Stop, Keep Both (the default) and Replace; for a folder,
  ⌥ turns Keep Both into Merge, as Finder. Replace is never the default
  (`window-chrome.md`).
- **Before starting**, a job that will not fit says so in an alert and
  does not start: "“{volume}” doesn't have enough space.", "{needed} is
  needed and {free} is available."
- **Failures** (permission, a vanished file, a full disk part way) do not
  stop the rest. At the end, one alert on the job's window: "{k} items
  couldn't be copied.", the reason when they share one, the items in a
  short list under it, and Try Again and Done (`conventions.md`, A failed
  action). Stopping a job leaves nothing half written (`apps.md` Files 5).
- **No elevation.** Files writes as the user. A folder it may not write
  is reported as a failure naming what to do; it does not ask polkit to
  copy as root, which would make Files a privileged writer of any path.

### Undo

Edit › Undo names the job ("Undo Move to Trash", "Undo Copy of 3 Items",
"Undo Rename") and covers what it did even if it failed part way; a job
that replaced something reads "Can't Undo" (`apps.md` Files 7; D70, D71).
It is a stack per window (`conventions.md`, Undo; For the user 2): each
step one job, Redo after it, and a step that replaced something reads
"Can't Undo" and ends the stack there.

### Trash

- **Moving to the Trash** (⌘⌫, a drag to the Trash row) acts at once,
  plays the trash sound (Settings › Sound switches it) and offers Undo
  (`conventions.md`, Confirm or undo). It goes to the item's own volume's
  trash (`apps.md` Files 4).
- **The Trash view** shows every mounted volume's trash at once, in List
  by default with Name, Date Deleted, Original Location and Size. Quick
  Look works; Open is disabled ("Put it back or drag it out to open it" as
  its accessible description). Put Back (⌘⌫ in the Trash, as Finder)
  returns items, even after their volume has mounted somewhere else.
- **Empty** (the toolbar's primary action, File › Empty Trash ⇧⌘⌫) asks
  in an alert while Settings › Advanced's warning is on: "Permanently erase
  {n} items in the Trash?", "You can't undo this.", Cancel (the default)
  and Empty Trash; ⌥⇧⌘⌫ empties without asking, as Finder. Delete
  Immediately… (⌥⌘⌫) asks the same way for the selection, anywhere.
- **Remove items from the Trash after 30 days**, Finder's setting, in
  Settings › Advanced, off by default.
- Spotlight finds "Trash" and opens this view (`dock.md`).

### Volumes: eject and mount

Eject from the row's button, the item's menu or ⌘E (Finder's Eject; Files
has no Use Selection for Find, so `conventions.md`'s ⌘E is free here).
A volume in use: an alert, "“{volume}” is in use and can't be ejected.",
"Quit the apps using it and try again.", with Cancel, Try Again and Force
Eject (never the default). Tabs showing a volume that goes move to the
new-window folder rather than closing under the user's eyes (Finder closes
them). The drive-connected notification is `notifications.md`'s.

### Get Info

A window per item (`conventions.md`, Window), ⌘I; several selected open one
each; Get Summary Info (⌃⌘I) opens one window for all of them; Show
Inspector (⌥⌘I) opens one Get Info window that follows the selection, as
Finder (the inspector *region* in Files is the preview, ⇧⌘P). Changes apply
as made.

| Section | Holds |
|---|---|
| Header | Icon, name, size, modified |
| Tags | The item's tags, if built later |
| General | Kind, size (and on disk), where, created, modified, last opened |
| More Info | What its kind has: dimensions, duration, pages, codec, an AppImage's version |
| Name & Extension | The name, edited here as in place |
| Comments | `user.xdg.comment`, with tags if they are built later (For the user 1) |
| Open with | A pop-up of apps, and Change All…, which asks and then sets the type's default (`apps.md` Files 18) |
| Preview | Quick Look's renderer |
| Sharing & Permissions | Owner, group and others, each Read & Write, Read only or No access; "Allow running as a program" (the execute bit, which a Linux machine needs for scripts and AppImages); editable only on the user's own items |

A volume's Get Info shows its name, format, capacity, available, used and
mount point. Sections collapse; their state is kept.

### Quick Look

One Quick Look in the Files process serves Files, the panels, Spotlight and
the capture thumbnail (`apps.md` Files 21; B26).

- **Opened by** Space in Files and in the panels' file list, ⌘Y anywhere
  (File › Quick Look, as Finder; Spotlight's chord, `spotlight.md`), a
  click on a recording's thumbnail (`capture.md`). Space, Escape or ⌘Y
  again close it.
- **A window of its own** with a title bar: the lights (minimise disabled; zoom is full screen, also ⌥⌘Y), the
  file's name, and at the trailing end the actions its kind has: Open with
  {app}, Markup (images), Rotate Left (images), Trim (video and audio).
  Sized to the content within the screen, resizable, its size kept.
- **Over its caller.** It floats above the window or surface that opened
  it, on its screen, as that caller's child: it is not a switcher entry
  of its own and goes with its caller's desktop. Opened from Spotlight it
  draws above Spotlight, which stays open behind it.
- **It never takes the keyboard** from its caller: in Files the arrows
  keep moving the selection and the preview follows; in Spotlight the
  field keeps typing. Tab moves focus into its toolbar, and Escape there
  hands it back.
- **Several items**: ‹ and › with "{i} of {n}", and an index sheet of
  thumbnails (⌘Return).
- **What it shows**: images; text and code, monospaced; PDF, scrolled by
  page; audio and video with playback controls; fonts as a sample;
  archives as their listing; folders as size and count; anything else as
  its icon, kind, size and dates.
- **Markup** (`capture.md`, which sets its tools): the tool row appears
  under the title bar, Done and Revert at its trailing end; ⌘Z undoes a
  stroke. It takes the keyboard while it is on. **Trim** (`capture.md`
  hands it here): handles on a strip of frames, with Done and Revert.
  **Done** writes the file in place and moves the original to the Trash,
  so Put Back recovers it, keeping Files 5's rule that nothing is lost
  before what replaces it is in place. Closing with unsaved Markup asks:
  Don't Save, Cancel, Save.
- **The D-Bus service** (`apps.md` Open for its packet, the name is the
  packet's) carries: the files, which one to show first, the caller's
  surface as an exported handle so it can float above it, whether to open
  in Markup or Trim; and it reports when it closes and which item was
  showing, so Spotlight's selection follows. A call starts Files if it is
  not running.

### The Open and Save panels

Files draws the panel for any app's FileChooser request (`apps.md` Files
22–26; B13, B26) and for Isle's apps' "Choose…".

**Where it sits.** The request's parent handle (xdg-foreign) makes the
panel a modal child of the asking window, and the compositor hangs it as a
sheet under that window's title bar or toolbar, moving with it
(`window-chrome.md` Sheets; For the user 4 there). It opens at its last
size clamped to the parent's width; a parent too small for the panel's
minimum gets it centred over it instead. A request with no parent, or a
parent that has gone, gets a small window of its own centred on the
focused screen, titled with the request's title. One panel per parent
window: a second request from it waits for the first.

```
Open
┌──────────────────────────────────────────────────────┐
│ Choose an image                                      │  the request's title
│ ‹ › [▦ ☰ ▥] [ Pictures ▾ ]           [⌕ Search ]     │
├───────────────┬──────────────────────────────────────┤
│ Recents       │ files; those the filter excludes     │
│ Favourites    │ are dimmed                           │
│ Locations     │                                      │
│               │                                      │
├───────────────┴──────────────────────────────────────┤
│ Show: [ Images ▾ ]   choices                         │
│ [New Folder]                       [Cancel] [Open]   │
└──────────────────────────────────────────────────────┘

Save, collapsed
┌────────────────────────────────────────────┐
│ Save As:  [ Export.png               ]     │
│ Where:    [ Pictures               ▾] [⌄]  │  ⌄ expands into the Open
│ Format:   [ PNG                    ▾]      │  panel's browser
│ choices                                    │
│ [New Folder]             [Cancel] [Save]   │
└────────────────────────────────────────────┘
```

- **Open** (OpenFile): Files' sidebar, views (Icons, List, Columns),
  search, preview column in Columns, hidden files, Quick Look, New Folder
  and Go to Folder (`apps.md` Files 25). The filter is a pop-up at the
  foot ("Show:"); items it excludes show dimmed and cannot be chosen.
  Multiple selection when the request allows it. A folder request makes
  folders the choice. The button takes the request's accept label, else
  "Open", or "Choose" for a folder.
- **Save** (SaveFile) opens collapsed: the name field with the base name
  selected, "Where" as a pop-up of the current folder, recent folders and
  Favourites, and a disclosure that expands it into the full browser,
  remembered per app. "Format" from the filters changes the extension in
  the name. A name already taken asks in the panel's place, as the next
  step of the same sheet (`conventions.md`, Sheets): "An item named
  “{name}” already exists in “{folder}”.", "Replacing it overwrites it.",
  Cancel (back to the panel as it was, the default) and Replace.
- **SaveFiles** is the Open panel choosing one folder, with "Save" as its
  button; each name that is taken asks as above (`apps.md` Files 24).
- **Choices** the request carries (checkboxes and pop-ups) sit in a row
  above the buttons.
- **Remembered** per requesting app: the folder (`apps.md` Files 25), the
  view, the size, Save's expanded state.
- **A drop** of a file from Files onto the panel goes to its folder and
  selects it, as macOS.
- **The answer.** Exactly one of accept or cancel answers the request;
  closing the parent, the panel's own Escape, or Files ending answers
  cancel (`apps.md` Files 23; D67). The panel closes no shell surface
  (Files 26).

### Files' settings

The app's settings window (⌘,; `conventions.md`, An app's own settings),
Finder's panes:

| Pane | Rows |
|---|---|
| General | New windows open (Home by default); Open folders in tabs instead of new windows (on: ⌘-double-click opens a tab; off, a window) |
| Sidebar | Each row and group on or off |
| Tags | The tag list, if tags are built later |
| Advanced | Show all filename extensions (on); Show warning before changing an extension (on); Show warning before emptying the Trash (on); Remove items from the Trash after 30 days (off); Keep folders on top (on); When performing a search (This folder, Everything under it, Home, Use the previous scope) |

## States

| State | What shows |
|---|---|
| Rest | The window at its folder; the jobs button absent |
| Loading | The frame and the rows that have arrived in the first frame; rows stream in sorted order, keeping the selection and scroll (`apps.md` Files 1); the subtitle's count grows as they come; a progress mark in the content only after a wait (`conventions.md`) |
| Many items | 50,000 entries draw their first screen before the listing ends and scroll without a dropped frame (`apps.md` acceptance bar 9); thumbnails fill on a worker, the kind's icon until then |
| Empty folder | "No items" in the content's centre; Recents: "No recent files"; Trash: "No items in Trash" |
| No results | "No results for “{query}”", the scope bar kept |
| Error: no permission | "You don't have permission to see the contents of “{folder}”." in the content |
| Error: folder gone | "“{folder}” was moved or deleted." with Go to Enclosing Folder; tabs on an ejected volume move to the new-window folder |
| Slow volume | A network folder that answers slowly shows the frame and the mark after the wait; the window keeps answering (`apps.md` acceptance bar 4) |
| Work in flight | Writing items dimmed with bars; the jobs button; the switcher card's progress; ⌘Q asks (`conventions.md`) |
| A job waiting | Its question as a sheet; Files marked in the switcher and the island's attention event while not in front |
| A job failed part way | One alert at the end listing the failures |
| No window, job running | Files stays running with its background mark and progress in the switcher; activating it opens a window with the jobs popover open |
| No window focused | The on-demand bar shows Files' name and fallback menus without starting Files (`menu-bar.md` For the user 1) |
| Narrow window | The toolbar folds; the preview, then the sidebar, collapse; a panel clamps to its parent down to its minimum |
| Quick Look open | Its window over the caller; the caller keeps the keyboard |
| Second screen | No difference: windows, panels and Quick Look sit on their own or their parent's screen |
| Game in front | Files' windows are ordinary; a panel requested by a full-screen game hangs from it; job notifications are held by Game Mode and summarised; no transitions while Game Mode is on (`competition.md` For the user 1) |
| Locked | Nothing shows; jobs run on; a question waits for unlock |
| Focus on | Job notifications follow the Focus; the window's own report is unchanged |
| Configuration error | Files' settings window shows the notice; its rows keep their last good values (`conventions.md`) |
| Reduce Motion | Quick Look and sheets fade instead of growing; spring-loaded folders blink as before and open by a cut, as every folder does; a cancelled drag's image fades rather than flying back |
| Reduce Transparency | Sidebar, preview and toolbar solid (`conventions.md`) |
| Increase Contrast | Borders on rows' selection, the tab bar, the panel's edges and Quick Look's window |
| Largest text size | Rows and the sidebar grow; Icons' names wrap to two lines then truncate in the middle; List's columns keep their widths and truncate in the middle |

## Opening and closing

- **Launching Files** (Spotlight, the folder handler, `FileManager1`)
  opens a window at the new-window folder, or raises the app with a new
  window when it is running with none. A second launch with a path opens
  it in the running app (`conventions.md`, Single instance; `apps.md`
  Files 19).
- **The last window** closing ends Files unless a job runs or a panel or
  Quick Look is open (`apps.md` Conventions 4; acceptance bar 3).
- **Quick Look** grows from the item's rectangle (its thumbnail or row)
  and shrinks back to it on close, faster; from Spotlight, from the
  result's row. Never a slide from elsewhere (brief P5).
- **Panels** grow down from the parent's toolbar edge as any sheet, and
  leave faster (`window-chrome.md`). The panel is on screen within 300 ms
  of the request when Files is not running (`apps.md` acceptance bar 1).
- **Get Info and the settings window** open as windows, placed by the
  compositor (B8).
- **Spring-loaded folders and tabs** open after a dwell during a drag;
  Space during the dwell opens at once, as Finder.

## Keyboard

Finder's keys (`apps.md` Files 16; `macos-model.md` §3 Finder row, §2.3
file managers), on top of `conventions.md`'s standard table. Cmd is Super
(B21).

| Area | Keys |
|---|---|
| Windows and tabs | ⌘N new window, ⌘T new tab, ⌘W close tab or window, ⇧⌘W close window, ⌥⌘W close all, ⌃Tab and ⌃⇧Tab tabs, ⇧⌘T tab bar |
| Go | ⌘[ ⌘] back and forward (and the mouse's buttons), ⌘↑ enclosing folder, ⌘↓ open, ⇧⌘G Go to Folder (also / or ~ typed in a list, which opens it with that typed), ⇧⌘H Home, ⇧⌘D Desktop, ⇧⌘O Documents, ⌥⌘L Downloads, ⇧⌘A Applications, ⇧⌘F Recents, ⇧⌘C Computer |
| Views | ⌘1 Icons, ⌘2 List, ⌘3 Columns, ⌘J View Options, ⌃⌘0 Use Groups, ⌃⌘ and ⌃⌥⌘ with digits for Group By and Sort By in the menus' order (Finder), ⌘+ ⌘− icon size, ⇧⌘. hidden files, ⇧⌘P preview, ⌃⌘S sidebar, ⌘/ status bar, ⌥⌘P path bar |
| Moving in a view | Arrows; ⌥↑ ⌥↓ first and last; Home, End, Page Up, Page Down; in List → and ← open and close a disclosure, ⌥→ opens all beneath; in Columns ← and → between columns; typing selects by name |
| Selection | ⌘A select all, ⌥⌘A deselect all, ⇧ with arrows extends |
| Acting | Return rename, ⌘O or ⌘↓ open, ⇧⌘N new folder, ⌥⇧⌘N new file, ⌃⌘N new folder with items, ⌘D duplicate, ⌃⌘A make alias, ⌘R show original, ⌘I Get Info, ⌃⌘I summary, ⌥⌘I inspector, ⌘C ⌘X ⌘V, ⌥⌘V move here, ⌥⌘C copy as pathname, ⌃⌘T add to sidebar, ⌘E eject, ⌘Z ⇧⌘Z undo and redo |
| Trash | ⌘⌫ move to Trash (Put Back in the Trash), ⌥⌘⌫ delete immediately (asks), ⇧⌘⌫ empty (asks), ⌥⇧⌘⌫ empty without asking |
| Search | ⌘F; Return searches everything under the folder; Escape clears then leaves |
| Quick Look | Space or ⌘Y opens and closes; Escape closes; arrows move the selection; ⌘Return index sheet; ⌥⌘Y full screen; Tab into its toolbar, then the media controls; Return plays or pauses the focused control, arrows move a focused slider |
| Markup | Tab through tools and colours, Return chooses; Return on the image places the chosen shape, crop or redaction at a default size at its centre; a mark is selected by Tab, moved with the arrows, resized with Option and the arrows (capture's portion mode), Delete removes it |
| Trim | Tab to each handle; arrows move it a frame, Shift and the arrows a second; Space plays |
| Panels | Return default, Escape and ⌘. cancel; Tab through the name field, sidebar, list, choices and buttons; the Go, view, hidden-file, New Folder and Quick Look keys as in a window |
| Drag | Escape cancels; ⌥ copies, ⌘ moves; Space opens a spring-loading folder at once; ⌘Tab carries the drag to another app (`switcher.md`) |

**v1's keys, each kept** (`apps.md` Files 16; `main:docs/AUDIT.md` §
Files, Wired): Alt with the arrows and Backspace to ⌘[, ⌘] and ⌘↑; Ctrl+H
to ⇧⌘.; Ctrl+F with Enter underneath to ⌘F with Return; Ctrl+1/2/3 to ⌘1/2/3;
Ctrl+Shift+P to ⇧⌘P; F2 to Return; Ctrl+Shift+N and Ctrl+Shift+F to ⇧⌘N and
⌥⇧⌘N; Delete and Shift+Delete to ⌘⌫ and ⌥⌘⌫; Ctrl+L to ⇧⌘G; Ctrl+I to ⌘I;
Ctrl+plus and minus to ⌘+ and ⌘−; the rest by their Cmd form. F5 has no
successor: every listing is watched (`apps.md` Files 2), and a network
folder is read again each time it is shown.

## Pointer

- **Click** selects, **⌘-click** adds or removes, **⇧-click** extends, a
  **band** from empty space selects, a **click on nothing** deselects
  everything, the keyboard cursor included (`apps.md` Files 17).
- **Double-click** opens; **⌘-double-click** opens a folder in a new tab;
  **⌥-double-click** opens it and closes the window, as Finder; **middle
  click** opens a folder in a tab, or closes a tab.
- **Secondary click** opens the menu for what is under the pointer,
  keeping a selection it is part of: an item, the folder's background, a
  tab, a sidebar row (Open in New Tab, Open in New Window, Show in
  Enclosing Folder, Get Info, Remove from Sidebar, Rename, Eject, Empty
  Trash), a column header (the columns shown), a column divider (Right
  Size).
- **Drag** carries the items under the pointer, or the selection they
  are part of (`conventions.md`, Drag and drop): onto a folder in any view,
  a tab, the path bar, a sidebar place (into it), between two Favourites
  (a new bookmark, at an insertion line), onto Trash, out to other apps as
  `text/uri-list`. Into Files from other apps, file lists only; data with
  no file behind it is refused and nothing is marked.
- **Sidebar**: Favourites reorder by drag (D76); the edge resizes; a
  heading collapses its group.
- **Hover** opens nothing, except the dwell over a folder or tab during a
  drag and the tooltips of symbol buttons (`conventions.md`).
- **Quick Look**: a click on its content does nothing in the caller;
  video and audio controls work by pointer and by Tab.

## Shared places

| Place or input | Competes with | Proposed winner |
|---|---|---|
| The asking app's window | The Open or Save panel from the Files process | The panel, as a sheet on it (modal child through the portal's parent handle), moving with it; centred over it when the parent is too small; a centred window when there is no parent. One per parent; a second request waits. *Added for `competition.md`* |
| The panel in the switcher, Mission Control and the Window menu | Files and the asking app | The asking app: the panel is its dialog, so it never appears as a Files window or entry, and Files has no switcher entry for a panel alone |
| The on-demand menu bar while a panel has focus | The asking app's menus and Files' | The asking app's name, with the panel's Edit, View, Go, Window and Help; the app's own menus wait, as macOS disables them under a sheet |
| Quick Look against Spotlight | The overlay layer | Quick Look draws above Spotlight as its child; Spotlight stays open beneath and keeps the keyboard; Escape closes Quick Look first. *Added for `competition.md` Layers* |
| Quick Look's keyboard | Its caller (a Files window, a panel, Spotlight) | The caller, except while Markup or Trim is on or Tab has moved focus into it |
| A job's progress | The window, the switcher card, the island | The window's jobs button and dimmed items while open, the switcher card always, the island never (`island.md` For the user 3) |
| A job's finish | The window and a notification | The window while a Files window is in front; a notification otherwise (`conventions.md`) |
| A job's question | Files' window and the user's attention elsewhere | A sheet on the job's window, which asks for attention: the switcher's mark and the island's attention event (`island.md`). No notification |
| ⌘E | `conventions.md`'s Use Selection for Find | Eject, as Finder; Files has no Find Next or Use Selection |
| ⌥⌘I | `conventions.md`'s Show Inspector | Finder's Inspector window; the preview pane, Files' inspector region, is ⇧⌘P |
| ⇧⌘D, ⇧⌘H, ⇧⌘A, ⇧⌘C, ⇧⌘O, ⇧⌘F | Desktop chords | None collide (`competition.md` Keys); they stay Files' |
| ⌘X on files | Finder, where it is disabled | Cut, then ⌘V moves; ⌥⌘V after ⌘C moves as well |
| Reveals from other apps | A new window each | A tab in the front Files window on the current desktop; a window only when none is there |
| Trash's sound | Focus and Game Mode | Plays: feedback for the user's own act (`focus.md`) |
| Dropping a file on an app | The Dock that is gone | Open With in Files, or ⌘Tab during the drag (`dock.md`) |

## Answers

1. **How places are laid out** (proposal: a sidebar of Recents,
   Favourites, Locations and saved searches). **Changed**: Recents above
   the headings, as v1; saved searches inside Favourites, as Finder puts
   them, ordered with the folders; Locations with the computer, every
   volume, mounted shares and Trash last (Trash's home with no Dock,
   `dock.md`); no Tags for now. Free space is a mark only on a nearly full
   volume, the figure in the tooltip, status bar and Get Info (brief P1),
   changing `apps.md` Files 11's wording at the gate.
2. **Where the preview sits** (proposal: a pane beside the folder, the
   last column in Columns). **Adopted**, as `conventions.md`'s inspector
   region named Preview, one toggle (⇧⌘P) for both forms; ⌥⌘I is Finder's
   Inspector window instead.
3. **The toolbar's contents.** **Answered**: back and forward and the
   folder's title with its count; the view switcher and the Group and Sort
   pop-up in the centre; the jobs button only while it has something to
   say, Empty Trash… as the Trash's one primary action, More, the preview
   toggle and search at the trailing end.
4. **How a file job's question and progress show.** **Answered**: dimmed
   items with bars in the folder, a jobs button and popover in every Files
   window, the switcher card's total, a notification at the end only when
   no Files window is in front; a question as an alert sheet on the job's
   window with an attention request, Keep Both as its default; failures
   gathered into one alert at the end.
5. **The Open and Save panels** (B26; `apps.md` Files 22–26). **New**:
   sheets on the asking window through its parent handle (`window-chrome.md`
   For the user 4), macOS's two forms, Save collapsed by default, the
   request's labels, filters and choices; the asking app owns the panel in
   the switcher and the menu bar.
6. **Quick Look** (`apps.md` Files 21; `shell.md` Spotlight 10, Capture
   4). **New**: a window floating over its caller that never takes the
   keyboard but in Markup and Trim; Done replaces the file and trashes the
   original. Its D-Bus service's contents are listed; the name is the
   packet's. Trim, which `capture.md` handed here, is Quick Look's.
7. **Tags** (`apps.md` Open for its packet). **Not for now**, as v1; the
   places they would take are marked above (For the user 1).
8. **Undo, one step or a stack** (`apps.md` Open for its packet; D70).
   **A stack per window**, as Finder, each step keeping D70's rule for a
   job that failed part way (For the user 2).
9. **Connect to Server** (`apps.md` Open for its packet). **Left out**:
   shares the system mounts show in Locations, and ⌘K stays free (For the
   user 3).
10. **A Gallery view** (`apps.md` Open for its packet). **Decided: no**;
    Icons with the preview covers it, ⌘4 stays unbound.
11. **Content search and an index** (`apps.md` Open for its packet).
    **Answered for the window**: a Contents criterion on the same search
    Spotlight's Contents switch uses, with its limits; an index remains the
    packet's, and saved searches (and tags, if built later) work without
    one, by walking.
12. **How the chooser binds to the requesting window** (`apps.md` Open
    for its packet). **Answered**: xdg-foreign's handle makes the panel a
    modal child the compositor hangs as a sheet; no handle, a centred
    window (`compositor.md` Stacking 6).
13. **Files as the front app with no window focused** (`menu-bar.md` For
    the user 1). **Adopted**: its name and fallback menus, without
    starting it; New Window starts it.
14. **Set as wallpaper** (`wallpaper.md`). **Adopted** in an image's menu.
15. **Dolphin's split view, terminal panel, editable location bar and git
    status** (`design-references.md` §14). **Not taken**: two tiled
    windows on the ultrawide are the split, Open in terminal and the
    dropdown terminal are the terminal (whether that one follows Files'
    folder is `dropdown-terminal.md`'s), Go to Folder and typing / or ~ is
    the location bar, and git status in rows is noise at rest (brief P1).

## Copy

Finder's menu items keep Apple's title case (`copy.md` rule 1); Isle's own
in sentence case.

| Where | String |
|---|---|
| Sidebar | "Recents", "Favourites", "Locations", "Tags", "Trash", "Home", "Applications" |
| File | "New Window", "New Tab", "New Folder", "New Folder with Items", "New file", "Open", "Open With", "Other…", "Open in terminal", "Close Window", "Close Tab", "Get Info", "Get Summary Info", "Rename", "Rename {n} Items…", "Compress “{name}”", "Compress {n} Items", "Expand", "Duplicate", "Make Alias", "Show Original", "Quick Look", "Add to Sidebar", "Move to Trash", "Put Back", "Delete Immediately…", "Empty Trash…", "Eject “{volume}”", "Set as wallpaper" |
| Edit | "Copy “{name}” as Pathname", "Move Item Here", "Deselect All", "Undo Move to Trash", "Undo Copy of {n} Items", "Undo Rename", "Can't Undo" |
| View | "as Icons", "as List", "as Columns", "Use Groups", "Group By", "Sort By", "Show View Options", "Show Preview", "Show Path Bar", "Show Status Bar", "Show Hidden Files", "Calculate all sizes", "Use as Defaults" |
| Go | "Back", "Forward", "Enclosing Folder", "Recents", "Documents", "Desktop", "Downloads", "Home", "Computer", "Applications", "Recent Folders", "Go to Folder…" |
| Titles and counts | "{n} items", "1 item", "{n} items, {free} available", "Searching “{folder}”", "{n} of {total}"; the jobs button "File jobs: {n} running", "…, waiting for you", "…, failed" |
| Search | "This folder", "Everything under “{folder}”", "Home", "Kind", "Last modified", "Contents contain", "Save" |
| Empty | "No items", "No recent files", "No items in Trash", "No results for “{query}”" |
| Errors | "You don't have permission to see the contents of “{folder}”.", "“{folder}” was moved or deleted.", "Go to Enclosing Folder", "“{name}” is already taken. Choose a different name.", "“{volume}” doesn't have enough space." with "{needed} is needed and {free} is available." and "Free up space or choose another location.", "{n} items couldn't be copied.", "“{volume}” is in use and can't be ejected." with "Quit the apps using it and try again." |
| Jobs | "Copying {n} items to “{folder}”", "Moving {n} items to “{folder}”", "Waiting", "{n} of {total}, about {time}", notification "Copied {n} items to “{folder}”", "Copied {n} of {total} items to “{folder}”", "Show", "Try Again", "Done" |
| Name taken | "An item named “{name}” already exists in “{folder}”.", "Apply to all", "Stop", "Keep Both", "Merge", "Replace" |
| Trash | "Permanently erase {n} items in the Trash?", "You can't undo this.", "Empty Trash", "Empty Trash…", "Delete {n} items" |
| Rename asks | "A name that begins with a dot makes the item hidden.", "Change the extension from “.{old}” to “.{new}”?", "Keep .{old}", "Use .{new}" |
| Volumes | "Eject", "Force Eject", "Try Again", "Remember in Keychain" |
| Panels | "Open", "Choose", "Save", "Cancel", "Save As:", "Where:", "Format:", "Show:", "New Folder", "An item named “{name}” already exists in “{folder}”.", "Replacing it overwrites it.", "Replace" |
| Quick Look | "Open with {app}", "Markup", "Rotate Left", "Trim", "Done", "Revert", "Don't Save", "Save" |
| Preview | "Information", "Calculating…" |
| Get Info | "{name} Info", "General", "More Info", "Name & Extension", "Comments", "Open with", "Change All…", "Preview", "Sharing & Permissions", "Read & Write", "Read only", "No access", "Allow running as a program" |
| Kind | "Folder", "Application (AppImage)", "Alias" |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Tags.** Linux has no shared tag standard beyond the freedesktop
   extended attributes `user.xdg.tags` and `user.xdg.comment`, which
   Dolphin also reads. (a) Tags as Finder's: stored there, the seven
   Finder colours as the starting tags (colour as the user's own data,
   as the Cleaner's kind colours are, v1 D88), shown as dots after a
   name, in Get Info, the context menu, a search criterion and a Tags
   group in the sidebar, each a search that walks the home folder; lost
   on volumes without extended attributes, as Dolphin's are; (b) no tags,
   as v1. **Recommended: (b) for now.** Without an index a tag is a slow
   search, the user's folders are code and games where names and folders
   already sort things, and dots on rows are colour beyond the accent
   (brief P6). The places to add them are marked above, so (a) can come
   later without moving anything.
2. **Undo.** (a) A stack per window, as Finder: each step one job, Redo
   after it, and a step that replaced something reads "Can't Undo" and
   ends the stack there; (b) one step, as v1 (D70). **Recommended: (a).**
   A Mac user presses ⌘Z twice and expects the second to work, and
   `conventions.md` already speaks of a window's history; each step keeps
   D70's rule for a job that failed part way.
3. **Connect to Server** (Go › Connect to Server… ⌘K, SMB and NFS). (a)
   Build it: a sheet with an address and recent servers, shares mounted
   for the user and listed in Locations; (b) leave it out, showing shares
   the system mounts (fstab, automount) in Locations. **Recommended: (b)**,
   unless there is a NAS or another machine's share to reach day to day:
   it needs a user-space mounter (gvfs or its like) that nothing else in
   Isle wants, and ⌘K stays free until then.
