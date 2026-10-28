# P15: Files' measures, the VM, and the host run

## Headless

`cargo test -p isle-compositor --features test-hooks --test files_bars`
against `target/debug/isle-files`, on a headless Isle with a private system
bus (no UDisks2):

| `apps.md` bar | Check | Result |
|---|---|---|
| 3 Nothing when closed | ⌘W on the last window, then the process | gone, well within 5 s |
| 4 Responsive under failure | Go › Home with no UDisks2 | the command taken at once; Locations lists the computer and the Trash |
| 7 Containment | `kill -9` while a panel holds a FileChooser request | the request answers cancel (the portal sees Files leave the bus); the shell answers; Files opens again |
| 9 Large folder | a folder of 50,000 empty files | first batch drawn at 0 ms, the listing done at 1,480 ms (debug build) |

Bars 1 and 2 are the host's. `tests/files.rs` (17) drives the window, the
views, thumbnails, jobs, rename, clipboard and drag, Open With, volumes,
search, Get Info, the settings window, the panels through a fake frontend
(Open, Save, SaveFiles' folder, cancel, a sheet on a foreign window), Quick
Look from Files and from Spotlight (its float and keyboard), Markup and
Trim.

## Memory, headless

One sample, debug build, Files idle 30 s with one window at Home on a
headless Isle, `smaps_rollup` (`files_bars::idle_memory`, ignored, run by
name):

| Pss | Pss_Anon | Pss_File | Private_Clean | Private_Dirty | Rss |
|---|---|---|---|---|---|
| 87.2 MB | 21.0 MB | 61.5 MB | 55.1 MB | 21.0 MB | 131.4 MB |

Over bar 2's 80 MB in a debug build, whose code pages are several times a
release build's (Settings' debug build measured 59.5 MB with 45 MB of file
pages; Files links far more: syntect, resvg, image, freya-video and its
audio). The anonymous 21 MB is twice Settings'; the release build on the host
settles it.

## The VM

In the guest (llvmpipe, 2560 by 1080), the workspace's debug build from
`/repo`, Isle's session on tty1, Files' desktop entry and D-Bus activation
files installed for the user pointing at `/repo`, `isle-portals.conf`
giving FileChooser to Isle:

- **A FileChooser request with Files not running.** Through the real
  `xdg-desktop-portal` frontend: Isle's backend called `Files1.Choose`,
  D-Bus activation started `isle-files --gapplication-service`, which
  opened the panel alone (no browser window). Escape answered cancel; with
  its last window gone Files exited (bar 3). A folder request with
  `current_folder` `/repo/data` opened there and Return answered
  `file:///repo/data`.
- **Found and fixed: a panel orphaned by a quick Close.** A caller that
  leaves at once (`gdbus call` exits after its reply) makes the frontend
  close the Request; the portal sent `Cancel` before the panel's window
  existed, `AppHandle::close` ignored a window with no id yet, and the
  panel stayed up answering nothing. A window closed before it is created
  now closes as it opens.
- **Found: the session bus reads a user's service files only at its
  start.** Installed under `~/.local/share/dbus-1/services` after login,
  the activation failed with `ServiceUnknown` until `ReloadConfig`; the
  package installs them under `/usr/share`, read before the session.
- **Firefox** (`widget.use-xdg-desktop-portal.file-picker` on): ⌘O raised
  Isle's panel as a sheet on Firefox's window through its xdg-foreign
  handle, the window dimmed under it, with Firefox's own All Files filter
  in Show:. Escape answered cancel. The frontend passes an empty app id
  for an unsandboxed app, so every host app shares one remembered folder
  and view.
- **Show in folder** as Chromium sends it (FileManager1 `ShowItems` from
  another process): a Files window at `/repo/data/apps` with the item
  selected.
- **A loop image** (ext4, `udisksctl loop-setup`): the drive notice posted
  as Files ("VMSTICK connected"); VMSTICK listed dimmed in Locations; a
  click mounted it through UDisks2 in the session and showed it with its
  Eject button; Eject unmounted and detached it, and the tab on it moved
  to Home.
- **Quick Look** from Files on a shell script: highlighted, floating over
  the window, which kept the keyboard and its accent selection; Space
  closed it. "Open with kitty URL Launcher" named the guest's default for
  `text/x-shellscript`, which `kitty-open.desktop` claims.

Not run in the guest: Zed, a GTK 4 app and an Electron app through the
panel (none installed), an encrypted image (no `cryptsetup` image prepared),
Spotlight's ⌘Y and the capture thumbnail's Markup (both checked headless).

## The host run, the user's

Not done. What it settles:

1. Bar 1: Files' first complete frame within 300 ms of launch, warm, and
   the panel on screen within 300 ms of a portal request with Files not
   running, on the release build.
2. Bar 2: PSS, private memory and VRAM after 30 s idle for the release
   build, alone and with the usual apps open.
3. Bar 9's scrolling: no dropped frame scrolling a 50,000-entry folder
   while it fills, at the screens' scale, in software.
4. A day of use: copies across the drives, the Trash, a USB stick and an
   encrypted one, the panels from Firefox, Zed, Steam and an Electron app,
   Quick Look and Markup on screenshots.

## The interaction audit, 2026-10-27

The VM pass above drove Files through menus and D-Bus and took
screenshots; it never clicked as a person does, so it missed what the user
found in minutes. A second pass drove real pointer and keys
(`dev/vm-ix.py`, git-ignored) and an audit walked the sheets against the
code: about 110 gaps, 4 breaking (the crash, multi-select, Columns' Back,
hidden files forgotten), ranked in the artifact `Files spec gaps`
(https://claude.ai/artifact/BiXt1zTa2rvAVqCzqRaAVi). P15b's proof is by
real input for that reason.

## The spec walk, 2026-10-28

P15b's close (choice 3): every line of `structure/files.md`'s Keyboard,
Pointer, Acting on files, States and Copy against the build, by four
read-only passes over the code and the interaction tests, then the gaps
built in 15b.14 or named here. **Built** lines are proved by a compositor
test driven by real input or a VM pass by real input, named where one
phase carries it; **cut** names the packet; **declined** gives the reason,
each accepted by the user (B120).

### Keyboard

Built: every row of the table's Windows and tabs, Go (with the mouse's
back and forward buttons and / or ~ typed), Views (⌘+ also as ⇧⌘=),
Moving in a view, Selection, Acting, Trash and Search rows; Quick Look's
Space, ⌘Y (and ⌘Y in its own window, 15b.14), Escape, arrows and
⌘Return; Panels' Return, Escape, ⌘. (15b.14) and the Go, view, hidden-file,
New Folder and Quick Look keys; Drag's ⌥ and ⌘ and Space. v1's keys
(15b.14, `Command::also` in `isle-app` carries a second chord unshown):
F2, Delete, ⇧Delete, Backspace, Ctrl+H, Ctrl+F, ⇧Ctrl+P, ⇧Ctrl+N,
⇧Ctrl+F, Ctrl+L, Ctrl+I, Ctrl+plus and Ctrl+minus. F5 has nothing to do.

Cut to P20: Quick Look's ⌥⌘Y full screen (the compositor refuses full
screen to a window with a parent), Tab into its toolbar and media
controls; Markup's keys (Tab through tools and colours, Return placing a
shape, a mark moved, resized and removed by keys); Trim's handles by keys
and Space playing; a panel's Tab through its sidebar and list (only its
fields and buttons take focus); Escape cancelling a drag and ⌘Tab
carrying one (the compositor's drag grab).

Declined: v1's Ctrl+1, 2 and 3 (the compositor's Ctrl+1…9 go to
desktops, as macOS's do); v1's Alt with the arrows (the same table gives
⌥↑ and ⌥↓ to first and last and List's ⌥→ to opening every folder under
it, as Finder; Backspace stands in for ⌘↑ and ⌘[ ⌘] stay).

### Pointer

Built: click, ⌘-click, ⇧-click; the band in List and Icons; a click on
nothing clearing the selection and the cursor; double-click,
⌘-double-click, ⌥-double-click, middle click on a folder and on a tab;
secondary click on an item, the background, a tab, a sidebar row (Open in
New Tab and Window, Show in Enclosing Folder, Get Info, Remove from
Sidebar, Eject, Empty Trash), List's header and a divider (Right Size;
Columns' also Right Size All Columns Individually and Equally, 15b.14);
the menus as the board drew them, every row named, counts, New Folder
with Selection, the extras, the trashed item's and the Trash's menus, the
Menu key and ⇧F10; drags onto folders, tabs, the path bar, sidebar places,
between Favourites, onto the Trash and out as `text/uri-list`; drops of
files only, a drag of anything else marking nothing (15b.14); Favourites
reordered, the sidebar's edge dragged and kept (15b.14), headings
collapsing; the spring's dwell; symbol buttons' tooltips (15b.14); Quick
Look's clicks kept from the caller and its media controls by pointer.

Cut to P20: the band in Columns; Rename in a volume's sidebar menu;
Quick Look's media controls by Tab (with its toolbar focus).

### Acting on files

Built: New Folder, New file (a submenu of `~/Templates` when there are
any) and New Folder with Items, in place with the name open; Rename in
place (the name selected, not the extension), the taken name under the
field, the dot and extension questions; Rename many; Duplicate, Make
Alias, Show Original, Compress ("Archive.zip" for several), Expand beside
it, and an archive with no default app set expanding when opened (15b.14,
so with Ark installed and no default an archive still expands); Copy as
Pathname; ⌘C ⌘V, ⌥⌘V and ⌘X ⌘V; Open With (the default marked, the rest,
Other…) and its sheet with Always Open With writing `mimeapps.list`; Open
in terminal; Set as wallpaper; an AppImage in `~/Applications` installed
and uninstalled by the compositor's watch, its kind "Application
(AppImage)".

Declined: Other…'s sheet opening on every app. It opens on the type's
apps with every app one tick away, as Finder's Recommended and All
Applications.

### States

Built: Rest, Loading (rows streamed in order, the mark after a wait),
Many items (the first screen before the listing ends; no dropped frame is
the host run's bar 9), Empty, No results, both errors, Slow volume, Work
in flight (rows dimmed with bars in List and Icons, the jobs button, the
switcher's progress, ⌘Q finishing the work), A job waiting, A job failed
part way, No window job running and the Copy window, No window focused
(the compositor's on-demand bar), Narrow window (the toolbar folding, the
preview then the sidebar collapsing, 15b.14), Quick Look open, Second
screen, Game in front (ordinary windows, Game Mode holding notifications,
no transitions to stop), Locked (jobs on their own threads; the question
waits behind the lock), Focus on, Configuration error, Reduce Motion's
spring blink and cut, Reduce Transparency through the tokens, Largest
text's middle cuts (15b.13).

Cut to P20: the writing bar in Columns' rows; a panel clamped to a
parent narrower than it (the compositor); a panel hanging from a
full-screen X11 game (an Xwayland surface cannot be an xdg-foreign
parent); Reduce Motion's fades (Quick Look and sheets have no growing to
reduce, and a drag has no image to fade); Increase Contrast's borders;
rows and the sidebar growing at the largest text size.

### Copy

Built: every string in the table, with three made right in 15b.14: the
menu bar's "Compress “{name}”" and "Compress {n} Items", "Undo Copy of {n}
Items" and "Can't Undo" (with "Can't Redo"). The context menu's own items
now follow the Copy table's case ("Open in terminal", "Set as wallpaper",
"New file"), and the spec's Pointer section with them. Where the spec's
sections word one message two ways (Acting's taken name, States' errors),
the build has the Copy table's.

Declined in P1: "Tags".
