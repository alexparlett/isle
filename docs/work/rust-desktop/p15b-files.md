# P15b · Files, finished

Planned 2026-10-26. Reviewed by the user the same day: every
recommendation taken.

P15 built Files' engine, its views, jobs, panels and Quick Look, and was
called built. It was not finished. On the installed package the user found
it unusable within minutes: it crashed, ⌘-click and ⇧-click did not select,
Back did not follow Columns, Show Hidden Files was forgotten, and there was
no context menu anywhere. An audit of the code against P1's sheets, with a
VM pass driving real pointer and keys, then found about 110 gaps: 4
breaking, about 25 high, 45 medium and 40 low. The list, ranked, is the
artifact `Files spec gaps` (https://claude.ai/artifact/BiXt1zTa2rvAVqCzqRaAVi).

P1's sheets for Files are binding and complete; nothing here redesigns
Files. P15b builds what they already say, fixes what P15 got wrong, and
proves it by use rather than by screenshots: every phase ends with its
behaviour driven by real input in the VM, and the packet closes with the
user using Files for a day on the host.

What P15b must get right is the basics of a file manager, in order:
nothing crashes or loses a file; selecting, opening and going back work by
mouse and keyboard as in Finder; every command has its menu, context menu
and shortcut. Polish comes after those, and is cuttable.

## What it answers to

- Decisions: B26 (Files in its process with the panels and Quick Look),
  B104–B111 (P15's engine, thumbnails, jobs, opening, panels, Quick Look),
  B53 (keys without focus), B60.
- Intent: `apps.md` Files 1–17 and Conventions; `platform.md`'s
  FileManager1 and FileChooser.
- Design, binding: `p1/structure/files.md`, `p1/surfaces/files.md`,
  `p1/structure/conventions.md` (Context menus, Standard shortcuts,
  Keyboard, Pointer), `p1/surfaces/menus.md` (context menus' look),
  `copy.md`; `review/critique.md` FL1–FL10.
- Evidence: the gaps artifact above; P15's `p15-files.md` and its outcome's
  Not built; `docs/research/rust-desktop/p15-files.md`; the host's crash
  dumps of 2026-10-26 (`tab.rs` go, "State already borrowed or dropped").
- The context menus' board: `Files context menus`
  (https://claude.ai/artifact/1FtiGhwYAZcPrwmLYHrQio), every target drawn
  with nine choices, folded in below as choices 6–9.

## Choices this plan makes

### Scope

1. **One packet before P17, in four tiers, the last cuttable.** Files is
   the app used most after the browser and terminal; P17's widgets can
   wait. Tier 1 (15b.1–15b.3): breaking bugs and data loss. Tier 2
   (15b.4–15b.8): selection, navigation, keys, tabs and context menus,
   everything a person does in the first minute. Tier 3 (15b.9–15b.12): the
   sidebar, views, panels and jobs as the sheets have them. Tier 4 (15b.13):
   the low items, each small; whatever is left when the rest is done moves
   to P20's audit, listed in the outcome. Alternative: fold all of it into
   P20. Not recommended: the user cannot live in Files as it is.
2. **Proof by use.** Every phase adds compositor tests in
   `tests/files.rs` for its behaviour, driven by pointer and key events
   rather than D-Bus or menu activation, and a VM check by real input
   (`dev/vm-qmp.py`, the interaction driver kept from the audit under
   `dev/`). The journal is checked for `panicked` and `coredumpctl` after
   every VM run. A screenshot alone proves nothing (memory: verify by
   interaction).
3. **A spec walk before the close.** 15b.14 walks `structure/files.md`'s
   Keyboard, Pointer, Acting, States and Copy sections line by line against
   the build and the gaps list, every line marked built, cut (with where it
   went) or declined (with why). That walk is the packet's Done when, not
   the phase list.

### The crash and the state model

4. **Every tab's State belongs to the window.** Freya runs an event handler
   in the scope of the element that fired it, and `State::create` attaches
   to that scope, so a tab made from a click (⌘-double-click, middle click,
   the tab bar's +) belongs to the list or tab bar and is dropped with it;
   the next navigation panics. `new_tab` takes the window's scope (captured
   by `current_scope_id()` in `FilesWindow`'s render) and creates with
   `State::create_in_scope`. The same audit covers Columns' per-column
   tabs, the panel, Get Info and Quick Look, and every `spawn` that writes a
   State after its component may be gone. A test opens a tab from a
   click, unmounts the view by switching views, and navigates.
5. **Modifiers from the pointer event.** Files reads ⌘ and ⇧ from keyboard
   events, which lag on Wayland (a modifier key's own press carries the
   state before it). The user's Freya fork gains the modifier state on
   mouse and pointer events (winit already tracks it in
   `modifiers_state`), and the views read it at the click. Recommended over
   tracking modifier keys by hand in each app, which every Isle app would
   need. A fork change: the user lands it in the fork, Isle pins the rev.

### Context menus (the board's choices)

6. **In the window, as Settings' row menu.** `isle_ui::ContextMenu`: the
   existing `isle_ui::Menu` at a point in window coordinates on
   `colour-window-raised`, flipped to stay inside the window, closed by a
   press outside or Escape; Settings' row menu moves onto it. A popup
   surface that can pass the window's edge, as `menus.md` says for the
   screen, needs xdg_popup in `isle-app` and is not worth it now; noted for
   P20.
7. **The board's recommendations as drawn:** an unselected target is acted
   on alone and ringed, the selection kept; Open With a submenu with the
   default first and marked, Other… opening the sheet; no Cut; Isle's
   extras (Open in Terminal, Set as Wallpaper, Expand) in a last group
   where they apply; counts in labels for several items; the List header's
   column checklist and the divider's Right Size; the toolbar's More (…);
   the Menu key and ⇧F10.
8. **One builder per target, the command table's ids and names.** A
   context menu row chooses the same command as the menu bar's item, with
   the clicked target in place of the selection where they differ. No
   second copy of any action.
9. **The board is a gate** (15b.6): the user answers lgtm or names changes
   before the menus are built; the answers go into `structure/files.md`.

### Navigation and keys

10. **Columns moves the tab.** The deepest chosen folder is the tab's
    place, with history as Finder does: Back from Columns goes to the
    folder before the last choice, the title and path bar follow, and a
    double-click in any column opens in that column's folder.
11. **Close Tab ⌘W, Close Window ⇧⌘W** (`conventions.md:118`), Close All
    ⌥⌘W, ⌃Tab and ⌃⇧Tab, Show Tab Bar ⇧⌘T (the sheets' and Finder's; this
    plan first had Reopen Closed Tab there, which no sheet asks for); with
    one tab ⌘W closes the window. `isle-app`'s default ⌘W becomes
    overridable by an app's table.
12. **Go to Folder as a sheet** with a path field completing from the
    filesystem and Recent Folders under it, ⇧⌘G, and typing `/` or `~` in
    a view opening it with that character; the same sheet in the panels.
13. **One key handler for the views.** ← → in Icons, between Columns,
    Home/End, Page Up/Down, ⌥↑ ⌥↓, List's → ← ⌥→ on disclosure rows, and
    type-to-select with a one-second reset, in one place shared by List,
    Icons, Columns and the panels, replacing the ↑/↓ handler in
    `window.rs`.

### Losing things

14. **Nothing is lost silently.** Undoing Put Back writes the
    `.trashinfo` back; undoing a move across volumes copies and deletes; ⌘Q
    with a job running closes the windows and lets it finish, and a job with
    no Files window shows in the Copy window (B119, revised at the board
    `Files alerts and Copy` from asking first); closing the parent app
    cancels its panel; closing Quick Look with unsaved Markup asks Don't Save,
    Cancel, Save; Markup takes the keyboard while on, so its text tool
    types and ⌘Z undoes a stroke.

### Shape of the code

15. **`window.rs` split first.** At 2,061 lines it holds the window, the
    command table and its handlers, the sidebar, the tab bar and the
    content. 15b.2 moves the sidebar, tab bar and command handling to
    their own files with no behaviour change, so later phases touch small
    files and two sessions can work at once.

## The workspace

```
crates/
  isle-files/src/
    window.rs           the window only
    commands.rs         the command table and its handlers (from window.rs)
    sidebar.rs          places, groups, reorder, collapse (from window.rs)
    tabbar.rs           tabs, scrolling, drops (from window.rs)
    keys.rs             the views' and panels' key handler, type-to-select
    menus.rs            the context menus, one builder per target
    go_to.rs            Go to Folder
    list_view.rs        disclosure, columns, band
    icons_view.rs       band, item info
    columns_view.rs     place and history, dividers
    tab.rs              scope, reveal into view
    panel.rs            keys, menus, Quick Look, drops
    jobs.rs, sheets.rs  questions without a window, failures, notification
    quicklook.rs, markup_view.rs   keyboard, close guard
  isle-ui/src/menu.rs   ContextMenu
  isle-app/             ⌘W overridable; quit asking
  isle-fs/              undo of Put Back and cross-volume moves; New file
  isle-compositor/tests/files.rs   interaction tests
freya (the user's fork)  modifiers on mouse events
```

## Phases

Each phase ends with its tests passing headless, its behaviour driven in
the VM by real input with the journal clean, and a line in `progress.md`.

### 15b.0 · The record

This sheet; the roadmap's P15b; P15's outcome amended to say what it left;
the gaps artifact linked from `p15-files.md`.

### 15b.1 · Breaking bugs

Choice 4's scope fix and its audit; choice 5's modifiers (the fork change,
then the views); choice 10's Columns; Show Hidden Files kept in
`files.json`. (The proposed session "Files: crash, multi-select, Back,
hidden files" is this phase.)

### 15b.2 · `window.rs` split

Choice 15, no behaviour change, every test unchanged.

### 15b.3 · Nothing lost

Choice 14.

### 15b.4 · Selection and opening

Band selection in List and Icons; middle click opens a folder in a tab;
⌥-double-click opens and closes the window; Rename selects the name
without its extension, and Save's field the same; New Folder and New File
(⌥⇧⌘N, `~/Templates`) open their name in place.

### 15b.5 · Keys and tabs

Choices 11–13: the tab chords and tab bar scrolling, Close Other Tabs and
Move Tab to New Window; Go to Folder; the shared key handler and
type-to-select; Show Original ⌘R, Add to Sidebar ⌃⌘T, Eject ⌘E, Redo ⇧⌘Z,
Group By and Sort By digits.

### 15b.6 · Context menus (gate)

The board's nine choices answered (choice 9), then `ContextMenu` and
`menus.rs` for every target of choice 7, the ring, the keyboard routes.

### 15b.7 · The toolbar

More (…); the sidebar toggle shown both ways; the title's path menu and
its proxy dragged as the folder; the narrowing fold.

### 15b.8 · Spring-loading and drops

Folders and tabs open under a dwell during a drag, Space at once; drops on
tabs, the path bar and between Favourites; ⌥ copies and ⌘ moves.

### 15b.9 · The sidebar

Favourites reordered by drag and written back as GTK bookmarks; headings
collapse and keep it; groups on or off in Settings; Templates and Public;
Recents newest first by day; Trash full or empty; free space in the
tooltip; Eject on shares; the mounting spinner.

### 15b.10 · Views and preview

List: disclosure, the header's column menu, columns reordered and resized
and kept per folder, header sort kept, Trash's Date Deleted and Original
Location, Calculate all sizes. Columns: dividers, Right Size, the preview
column following its toggle. The preview resized, Show More, per-kind
facts. Items being written dimmed with a bar; alias badges.

### 15b.11 · Panels

Choice 13's keys, the Edit, View and Go menus, Quick Look, the hidden
toggle, search with its scope, drops from Files, Go to Folder.

### 15b.12 · Jobs, search and states

The end-of-job notification; attention while a question waits; failures
named by the job with Try Again; the jobs button's one-second delay and
cleared failures; search's Return scope, This folder kept as itself;
loading progress; Go to Enclosing Folder on a gone folder; Get Info's
More Info and rename checks.

### 15b.13 · The low items

Middle truncation; Quick Look's size to content, full screen, fonts,
Rotate, toolbar focus; Trim's play and playhead; the copy strings; the
Trash sound; Spotlight's Trash; the panel's size per app. Cuttable.

### 15b.14 · The walk, the host and close

Choice 3's walk, written into `p15-files.md`'s research note; the user's
day on the host; decisions recorded; ARCHITECTURE's How a context menu
opens; this sheet ends with its outcome.

## Not in P15b

Context menus as popup surfaces past the window's edge (choice 6, P20).
Tags (declined in P1). Connect to Server (declined in P1). A content index
for search (P15's choice stands).

## Depends on, and feeds

- Needs P15. Needs a fork rev of Freya with modifiers on mouse events
  (choice 5), the user's to land.
- Feeds every Isle app: `ContextMenu`, the modifiers, ⌘W overridable and
  quit asking come through `isle-ui` and `isle-app`. P19's Cleaner shares
  `isle-fs`'s undo fixes.

## Done when

- The walk (choice 3) has every line of `structure/files.md`'s Keyboard,
  Pointer, Acting, States and Copy sections built, or cut to a named
  packet, or declined with a reason the user accepted.
- `cargo test` passes with the interaction tests of every phase, and no
  VM run leaves `panicked` in the journal or a core dump.
- On the host, the user uses Files for a day, copies across drives, uses
  the Trash, panels from Firefox and Zed, and finds nothing broken.

## Risks

- **The fork.** Modifiers on mouse events are a change in Freya's event
  path; if the user would rather not carry it, Isle tracks modifier keys in
  `isle-app` and hands them to every app instead.
- **State scope beyond tabs.** The same mistake may sit in other apps
  (Settings, the shell). 15b.1's audit reports any found outside Files
  rather than fixing them here.
- **Size.** About 110 items. Tiers 1 and 2 are what make Files usable; if
  the packet runs long, tier 4 and parts of tier 3 move to P20 at the
  user's call.

## Reviewed

The user reviewed the plan on 2026-10-26 and took every recommendation:
one packet before P17 in four tiers, the last cuttable to P20; context
menus in the window, the board's nine choices as drawn (gate 15b.6
answered with this review); modifiers on mouse events in the Freya fork;
`window.rs` split first; proof by real input and a walk of the sheet.

## Outcome

Built through 15b.13 and walked in 15b.14 (2026-10-28): the walk is in
`docs/research/rust-desktop/p15-files.md` § The spec walk, every line
built, cut to P20 or declined. Cut to P20, by the walk and the phases:
Quick Look's full screen, sizing to content, fonts, toolbar and media
focus; Markup's and Trim's keys and Trim's playhead; a panel's Tab stops
and its clamp to a narrow parent; Escape and ⌘Tab during a drag; the band
and the writing bar in Columns; volume rename; Increase Contrast's
borders; rows growing at the largest text size; Reduce Motion's fades;
the Trash sound; Spotlight's Trash; context menus past the window's edge.
Declined, and accepted by the user (B120): v1's Ctrl+1, 2 and 3
(desktops), v1's Alt with the arrows (Finder's ⌥ arrows in the same
table), Other…'s sheet opening on every app (Finder's recommended list,
all one tick away).
The Freya fork's `2cf6468f` is pinned: an app's event loop ends quietly at
a session's end, where every Isle app panicked. Still to come: the user's
day on the host.
