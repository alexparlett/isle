# Cleaner

Batch 3. Answers to: `apps.md` For the design process (Cleaner: its views
and their names; how a kind is shown; whether Cleanup's findings also show
in Settings' Storage pane) and the presentation side of `apps.md` Cleaner
1–11 and the Parity map's Cleaner rows ("views and cards are P1's");
`settings.md` (General › Storage); `conventions.md` (the basket as the
inspector); B17, D87–D90.

## Purpose

What Broom and DaisyDisk give a Mac (D87, D88): where the space went, by
kind; what can go safely; duplicates, large and untouched files; and a
basket that sends what the user picks to the Trash in one job with one
undo (`apps.md` Cleaner). It is for the moment a disk is fuller than the
user expected. It is not a file manager (Files), not a scheduled cleaner
that removes anything on its own, and not a package manager: the system's
share is cleared by its own tools under polkit, one action at a time.

## Content

### The window

```
┌───────────────┬────────────────────────────────────────────┬──────────────┐
│ ● ● ●  ◧  ‹ › │ Cleanup   All disks · 412 GB scanned        │  ↻  …  ◨  🔍│  toolbar
│               │                                            │              │
│ Overview      │  Safe to clear                 18.2 GB ☑  │ Basket       │
│ Cleanup       │   ☑ ▪ Spotify cache   ~/.cache/…   3.1 GB │ 18.2 GB      │
│ Space         │   ☑ ▪ npm store       ~/.npm       2.4 GB │ 31 items     │
│ Duplicates    │  Worth a look                  41.0 GB    │ ▪ Spotify …  │
│ Large files   │   ☐ ▪ target/  (Cargo.toml beside) 9.8 GB │ ▪ npm …      │
│ Untouched     │  System                                    │              │
│               │   Package cache   6.2 GB        Clear…     │ Frees 18.2 GB│
│ ─ nvme0 ▰▰▰▱  │   Journal         1.1 GB        Clear…     │ on nvme0 once│
│ Trash 2.3 GB  │                                            │ emptied      │
│ Empty Trash…  │                                            │[Move to Trash│
└───────────────┴────────────────────────────────────────────┴──────────────┘
   sidebar         content                                     basket
```

Sidebar, content, and the basket as the inspector, shown while it holds
anything (`conventions.md`, Window anatomy table).

- **Toolbar.** Leading: the lights; the sidebar toggle; back and forward
  (the window keeps a history of views and, in Space, folders, as
  Settings keeps panes); the title, the view's name, with the scope and
  scan state as subtitle ("All disks · 412 GB scanned", "Scanning… 61%").
  Centre: the **scope**, a pop-up button (All disks, each mounted disk,
  recent folders, Choose folder…), then the view's own controls (Space:
  Map and List; Large files, Untouched and Duplicates: a Kind pop-up;
  Untouched: the period). Trailing: Rescan (↻), the one primary action,
  Stop while a scan runs; More (…); the basket toggle; search, which
  filters the shown list by name.
- **Sidebar.** The views, in one group: Overview, Cleanup, Space,
  Duplicates, Large files, Untouched. At its foot, not scrolling with it,
  the scope's disks: each a name and a bar of used and free from the
  disk's own figures, and the Trash's size on the scope's disks with
  Empty Trash… (D88). This is v1's disk card, kept in reach of every view.
- **Basket** (the inspector, View › Show basket, ⌥⌘I). Its total and item
  count; the items, each with its kind mark, name, size and a remove
  button; what it frees on each disk once the Trash is emptied; Move to
  Trash. It appears when the first item goes in and leaves when the last
  comes out (`conventions.md`: the inspector toggles; here it also opens
  itself, since an empty basket has nothing to show).

### Views

| View | Shows | Adds to the basket |
|---|---|---|
| Overview | The scope's total as one bar by kind with its legend (kind, size); the disks' used and free beside the scan's total, and what the scan could not reach (`apps.md` Cleaner 4); "{size} safe to clear" with Review, which opens Cleanup; the largest folders (hotspots) and the largest files, a handful each, each opening its place in Space or Large files | No: it points |
| Cleanup | **Safe to clear**, ticked by default; **Worth a look**, never ticked (D89); each row a rule's find, named for the app that made it, its path, size, kind mark and, for Worth a look, why it is there ("Build folder, Cargo.toml beside it", "Left by an uninstalled Steam game"). Then **System**: the package cache, orphaned packages, the journal and crash dumps, each with its size and its own Clear… or Remove…, and the line "These can't be put back." (`apps.md` Cleaner 8) | Ticks. System rows never: they go through their own tools under pkexec, one authorisation each |
| Space | The scope as a map (a squarified treemap, tiles coloured by the kind most of each is) or as a list with disclosure, chosen in the toolbar; a path at the content's top; a folder opens on double-click, ⌘↑ goes up | Secondary click or Space on a selected tile or row |
| Duplicates | Groups of identical files (D90), each a name, the size each, the count and the copies' paths; copies that share blocks through a reflink are marked "Shares storage" and free nothing | Ticks; "Select all but the newest" in More |
| Large files | The largest files the scan kept, by size, with their folder, kind and last opened | Ticks |
| Untouched | Files not opened in the toolbar's period (6 months, 1 year, 2 years), by size | Ticks |

Ticking a row puts it in the basket and unticking takes it out, in every
view: the basket is one, and a file ticked in Large files is ticked in
Space. Safe to clear's rows go into the basket when a scan finishes
(`apps.md` Cleaner 7), so the basket opens after the first scan showing
what would go.

### Kinds

Ten fixed kinds (`apps.md` Cleaner 5): Games, Video, Images, Audio,
Documents, Archives and disk images, Developer, Apps and system, Caches and
logs, Other. A kind is shown the same way everywhere (D88): its colour,
its symbol and its name. The colour is data, the one place Isle draws in
colours beyond the accent and status (`design-v1-lessons.md` §5), and its
ten values are tokens in `tokens.kdl`, checked against both themes in
stage 4. Colour never stands alone: a row carries the kind's symbol beside
its mark, a map tile shows the symbol when it is large enough and the name
on hover or selection, and the legend names every kind. Settings' Storage
bar uses the same colours and order (For the user 1).

### Scanning

- The window opens on All disks and scans at once (`apps.md` Cleaner 2).
  Views fill as folders arrive; the subtitle shows the share of the disks'
  used space scanned so far, which makes the progress determinate. A
  later open shows nothing kept: each run scans afresh, which D87 makes
  fast.
- Rescan (⌘R) scans the scope again; changing the scope scans the new one.
  Stop ends a scan and keeps what it has, marked "Partial scan".
- Closing the window cancels a scan (`apps.md` Cleaner 11).

### The basket and the Trash

- **Move to Trash** (⌘Delete) runs one job through the shared file
  operations (`apps.md` Cleaner 10; Files 9): progress in the basket and
  on the switcher card; the process stays until it ends.
- **After the move** the basket shows "Moved {n} items to the Trash", Undo
  (⌘Z, one undo puts all of it back), and, for items on a disk whose Trash
  is on the same volume, "Nothing is freed until the Trash is emptied"
  with Empty Trash…. It stays until the next change.
- **Empty Trash…** asks, as Files' does (`files.md`): Cancel default.
- A part that failed is listed with the job's result, not one alert per
  item (`conventions.md`).

## States

| State | What shows |
|---|---|
| Scanning | Views fill as data arrives; the subtitle's share; Rescan becomes Stop |
| Rest | The view; the basket if it holds anything |
| Empty view | One line: "Nothing safe to clear", "No duplicates found", "No files untouched in {period}" |
| No results | "No results for “{query}”" |
| Partial scan | The subtitle says "Partial scan"; Overview notes what was not reached |
| Unreadable folders | Counted apart and named in Overview: "{n} folders couldn't be read" |
| A disk not mounted | Not in the scope pop-up; a disk removed mid-scan ends that part, marked partial |
| Moving to Trash | Progress in the basket and on the switcher card; Quit asks (`conventions.md`) |
| Moved | The basket's after-move form with Undo and Empty Trash… |
| System clear refused | The polkit dialog said no: the row is unchanged |
| Narrow window | The basket collapses to its toolbar toggle with a count, then the sidebar collapses; the map keeps its tiles and drops labels |
| Second screen | No difference |
| Game in front | An ordinary window; a scan reads the disk hard, so Rescan is the user's, and none starts while the window is not shown |
| Locked | Nothing shows; a running job carries on |
| Focus on | No change |
| Reduce Motion | Map tiles change in place without growing into a folder |
| Reduce Transparency | Sidebar and basket solid |
| Increase Contrast | Borders on map tiles and rows; kind marks outlined |
| Largest text size | Rows grow; the map shows labels on fewer tiles |

## Opening and closing

From Spotlight, the Apps grid, its desktop entry, and Settings › General ›
Storage (For the user 1), which opens it on Cleanup or Large files. It
opens as any Isle app, on the last view, focus on the content. In Space,
opening a folder grows its tile to fill the map and going up shrinks it
back, anchored to the tile (brief P5). The basket slides in from the
trailing edge as `conventions.md` has the inspector.

## Keyboard

| Key | Action |
|---|---|
| ⌘1 to ⌘6 | Overview, Cleanup, Space, Duplicates, Large files, Untouched |
| ⌘[ ⌘], ⌘↑ | Back, forward; up a folder in Space |
| Arrows | Move in a list or between map tiles |
| Return | Open a folder in Space; Review from Overview |
| Space | Tick or untick the selected row; add or remove the selected tile |
| ⌘Delete | Move the basket to the Trash |
| ⇧⌘Delete | Empty Trash… |
| ⌘Z | Undo the last move |
| ⌘R | Rescan |
| ⌥⌘I | Show or hide the basket |
| ⌘O | Scan a folder… |
| ⌘F | Search |

## Pointer

- Click selects; double-click opens a folder in Space or shows a file in
  Files; secondary click opens the item's menu: Add to basket or Remove
  from Basket, Show in Files, Quick Look, Copy path.
- A map tile shows its name, size and kind on hover after a dwell; hover
  opens nothing else.
- Rows and tiles drag out as files (`conventions.md`); dropping a folder
  on the window scans it.

## Shared places

| Place or input | Competes with | Proposed winner |
|---|---|---|
| The Trash and Undo | Files, which trashes and puts back through the same operations | Shared: an item Cleaner moved can be put back from Files too; Cleaner's ⌘Z undoes only its own last move |
| Empty Trash | Files' Empty Trash | The same alert from either (`files.md`) |
| Storage figures | Settings › General › Storage | Settings shows each disk as a bar by kind and Recommendations that open Cleaner; only Cleaner removes anything but the Trash (For the user 1) |
| Kind colours | The accent and status colours | Kind colours are data, shown only in Cleaner and the Storage bar, never on a control |
| Disk reads | A game in front | The game: no scan starts while Cleaner is not shown |
| ⌘1 to ⌘6 | `conventions.md`'s ⌘1 to ⌘4 | Cleaner's six views, as an app's command table may add |

## Answers

- **Its views and their names** (proposal: Overview, Cleanup with Safe to
  clear, Worth a look and a System card, Space as a treemap or list,
  Duplicates, Large files, Untouched). **Adopted** as named, in a sidebar,
  with the basket as the inspector and the disks at the sidebar's foot.
  Space's map or list is a toolbar choice, map first, as DaisyDisk.
- **How a kind is shown** (proposal: the same way in every view, D88).
  **Adopted**: colour, symbol and name together, colour never alone, the
  ten colours as tokens.
- **Whether Cleanup's findings also show in Settings' Storage pane**.
  **Yes**, as macOS: each disk as a bar by kind from the shared scan, and
  Recommendations whose Review opens Cleaner at that view; Settings removes
  nothing but the Trash (For the user 1).
- **Where the scan's scope is chosen.** **New**: a pop-up in the toolbar,
  since scope and view are independent and the sidebar holds one choice.
- **Progress.** **New**: the scanned share of the disks' used space, so a
  scan's progress is a figure rather than a spinner.

## Copy

| Where | String |
|---|---|
| Sidebar | "Overview", "Cleanup", "Space", "Duplicates", "Large files", "Untouched" |
| Kinds | "Games", "Video", "Images", "Audio", "Documents", "Archives and disk images", "Developer", "Apps and system", "Caches and logs", "Other" |
| Cleanup | "Safe to clear", "Worth a look", "System", "These can't be put back.", "Clear…", "Remove…" |
| Overview | "{size} safe to clear", "Review", "Largest folders", "Largest files", "Not scanned", "{n} folders couldn't be read" |
| Scope | "All disks", "Choose folder…" |
| Subtitle | "{scope} · {size} scanned", "Scanning… {n}%", "Partial scan" |
| Space | "Map", "List" |
| Untouched | "Not opened in", "6 months", "1 year", "2 years" |
| Duplicates | "Shares storage", "Select all but the newest" |
| Basket | "Basket", "{size}", "{n} items", "Frees {size} on {disk} once the Trash is emptied", "Move to Trash", "Moved {n} items to the Trash", "Undo", "Nothing is freed until the Trash is emptied", "Empty Trash…" |
| Item menu | "Add to basket", "Remove from basket", "Show in Files", "Quick Look", "Copy path" |
| Empty | "Nothing safe to clear", "No duplicates found", "No files untouched in {period}" |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Cleaner's findings in Settings › General › Storage.** (a) As macOS:
   Storage shows each disk as a bar by kind, computed by the shared scan
   when the page is shown and dropped when it closes, and a
   Recommendations group: "{size} safe to clear" with Review…, the Trash
   with Empty Trash…, and "Large and untouched files" with Review…, each
   Review opening Cleaner at that view; Settings never removes anything
   but the Trash. (b) Storage keeps its drives and SMART, with the used
   and free bar from the filesystem and one "Open Cleaner" row.
   **Recommended: (a).** A Mac user looks in Storage first (B17,
   `design-macos.md` §3.18), the scan is under a second from a warm cache
   (D87), and keeping removal in Cleaner's basket leaves one place that
   deletes things.
