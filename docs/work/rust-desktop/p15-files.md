# P15 · Files

Planned 2026-10-20. Reviewed by the user the same day: every
recommendation taken.

The roadmap's brief: Finder-shaped Files with v1's browsing-engine lessons;
the Open and Save panels, handed FileChooser by the portal backend; Quick
Look as a D-Bus service Spotlight calls; all in the Files process (B26).
Files owns `org.freedesktop.FileManager1`, which Spotlight's Cmd+Return on a
file already calls, and a search that Spotlight's "Show all in Files" and
Cmd+Y reach. The capture thumbnail's click opens Quick Look with Markup, and
its menu gains Show in Files and Markup.

P1 designed all of it. `p1/structure/files.md` and `p1/surfaces/files.md`
fix the window, places, the three views, the preview, tabs, search, jobs,
undo, the Trash, volumes, Get Info, Quick Look with Markup and Trim, the
panels, Files' settings, every state, key, string and motion; its gate
decided no tags, an undo stack, no Connect to Server and no Gallery view.
What is open is how it is built, and the record leaves four points to this
packet by name: Quick Look's D-Bus name, the interface between the worker
and Files for a FileChooser request, a content index, and who watches
`~/Applications` while Files is not running (`apps.md` Open for its packet;
`platform.md` Portal backend).

Today nothing of it exists. There is no Files crate, no file-operations
crate, no MIME database, no thumbnail code and no `mimeapps.list` reader.
FileChooser is gtk's (`isle-portals.conf`). Spotlight's Cmd+Return calls
`ShowItems` with an unencoded URI and no token, and nothing answers; Cmd+Y
on a file does nothing; the capture thumbnail's click runs `xdg-open`; the
compositor's one trash is the home trash, in `thumbnail.rs`. `isle-app` gives
an app one window: no second window, no drag and drop, a text-only
clipboard, no xdg-foreign or xdg-dialog, no Go menu, nothing that keeps the
app alive while work runs. The virtual list and grid P10 promised to
`isle-ui` are still inside Spotlight's surface.

What P15 must get right is that Files never waits. No listing, stat, count
or thumbnail is on the thread that draws, a network or FUSE mount that
stops answering costs one worker thread and a mark, never the window
(v1's D64 and D65, cosmic's freezes; `files-2026.md` §4, §5, §8); and no
file job removes anything before what replaces it is in place (D70). The
panels must answer every request exactly once, whatever closes.

## What it answers to

- Decisions: B2, B9 (file work on workers), B11 (UDisks2 through the
  service crates), B13 and B26 (the panels and Quick Look in the Files
  process, the backend on the worker), B17, B19 (Trash and drops in
  Files), B20, B21, B25 and B95 (the lights over the toolbar), B27, B31 and
  B33 (no glass in Files), B73–B76 (the index stays names only), B83 (the
  portal's sender check), B94 (`isle-app`), B98, B100 and B101 (one
  package; the portal file per desktop).
- Intent: `apps.md` Conventions 1–32, Files 1–26 and its open points,
  acceptance bars 1–4, 7, 9 and 11 for Files, the Parity map's Files rows;
  `platform.md` App index 3 and 5, Portal backend 1, 2 and 7 with its open
  point, acceptance 15; `shell.md` No Dock 5–6, Spotlight 3 and 10,
  Notifications 8, Capture 4; `compositor.md` Stacking 6.
- Design, binding: `p1/structure/files.md` and `p1/surfaces/files.md`;
  `structure/capture.md` and `surfaces/capture.md` (Markup's tools, the
  thumbnail's menu); `structure/spotlight.md` (Show all in Files, Cmd+Y,
  Quick Look over Spotlight); `conventions.md`, `window-chrome.md`
  (sheets, alerts), `menus.md`, `dock.md`, `competition.md`,
  `notifications.md` (the drive and AppImage notices), `menu-bar.md` For the
  user 1, `wallpaper.md`, `copy.md`.
- Evidence: `files-2026.md` (new), `isle-v1-inventory.md` §3,
  `design-v1-lessons.md` § Files and the other windows, `main:docs/AUDIT.md`
  § Files, `spotlight-2026.md` (no content index), `session-2026.md` §6
  (Qt's dialogs, P14b's).

## Choices this plan makes

### Scope

1. **One packet, ordered so the panels land halfway.** The preview pane,
   Spotlight's Cmd+Y and the capture thumbnail share Quick Look's renderer,
   and the panels share the views, so splitting at Quick Look would build
   the preview twice. The phases build the engine, the window and its
   views, the jobs, then the panels (switching FileChooser to Isle), then
   Quick Look, Markup and Trim, then the ties to the shell. P13's size, in
   eighteen phases.
2. **No new design gate.** P1's two sheets are binding and complete to
   copy and tokens; nothing here waits on a board. Where building finds a
   gap, the smallest fix in P1's terms is made and listed in the outcome,
   as P13 did. The one new set of rows (choice 34) follows Settings'
   conventions.

### The app and its crates

3. **Three new crates.**
   - `isle-fs`: the engine, with no UI. Listing, watching, the mount table
     and its probes, sort keys, the trash, file jobs, undo records,
     recents. `apps.md` Files 9 makes the jobs a crate the Cleaner shares;
     the compositor's thumbnail trash moves here.
   - `isle-files`: the app, binary `isle-files`, id and bus name
     `io.github.alexparlett.Isle.Files` (`apps.md` Conventions 1). Files'
     windows, Quick Look, the panels, Get Info and the settings window.
   - `isle-thumbnail`: the helper process of choice 14.
   `isle-apps` gains MIME (choice 11) and default apps (choice 12);
   `isle-disks` gains what Locations needs (choice 24); `isle-search`'s
   contents search becomes a function both the compositor and Files call.
4. **`isle-app` grows for many windows.** Files is the first app with more
   than one: Files windows, Quick Look, panels, Get Info, the settings
   window. Freya's fork already launches, closes, focuses and parents
   windows (`launch_window`, `set_window_parent`); `isle-app` gains a
   window API over it with a `Spec` per window kind, the Wayland extras
   bound per surface (lights inset, appmenu with the window's own menu
   path, gtk_shell1, activation), the focused window's command table, a
   keep-alive held while work runs (`apps.md` Conventions 4) with Cmd+Q
   naming what it stops (5), the Go menu, and the switcher card's progress
   through `com.canonical.Unity.LauncherEntry` (P7). Sheets and popovers
   move from Settings into `isle-ui` with Freya's modal focus; the virtual
   list and grid move from Spotlight's surface into `isle-ui`, gaining
   variable group headers and a column table.
5. **Drag and drop and the clipboard through one data device.** winit
   reports no Wayland drag and drop and smithay-clipboard carries text
   only. `isle-app` binds its own `wl_seat`, pointer and
   `wl_data_device` on its second queue (smithay-client-toolkit's data
   device), and replaces smithay-clipboard with it. Every drag that starts
   in Files is a Wayland drag, inside the window too, so one path serves a
   drop on a folder, a tab, another window, another app, the Trash or ⌘Tab
   to another app (`switcher.md`); the drag image is drawn with Skia into
   an shm buffer. Copying files offers `text/uri-list` and
   `x-special/gnome-copied-files` (with `cut` for ⌘X), so a paste in another
   file manager or an editor works; v1's clipboard was Files' own.
6. **Menus, pop-ups and sheets stay in the window** (B94, P13 choice 12).
   Files' smallest window, 560 by 400, holds its widest sheet; a context
   menu clamps inside. Quick Look and the panels are windows, not pop-ups.
7. **State and settings.** Files' state is an `isle-state` domain,
   `files`: open windows and tabs (reopened at launch, `apps.md`
   Conventions 6), per-folder view, sort, grouping, icon size and columns
   (the last 2,000 folders), the sidebar's order and collapsed groups,
   saved searches, Quick Look's size, and per requesting app the panel's
   folder, view, size and Save's expanded state. Files' settings are a
   `settings.kdl` section by app id (B27): the new-window folder, tabs for
   folders, sidebar rows, the Advanced rows; each row indexed for
   Spotlight as "Files Settings › …" from the window's own record, as
   Settings' panes are (B97).

### The engine

8. **A listing streams.** A worker reads the folder with `getdents` and
   sends batches of 512 entries; `d_type` first, then one `statx` per entry
   with `AT_NO_AUTOMOUNT` and `AT_STATX_DONT_SYNC`, a second for a link's
   target. The window draws the first batch in the frame it arrives,
   sorted, and merges the rest keeping the selection, the cursor and the
   scroll (`apps.md` Files 1; bar 9). A generation per view drops a
   listing that finishes after the view moved on (v1 67ee83d). Child
   counts, folder sizes and type sniffing never run on the listing path:
   each is a later request, for rows on screen, on a worker.
9. **The model changes by rows, never by reset.** The engine diffs a new
   batch or event against what is shown and hands the view inserts,
   removals and changed rows by path; the selection is held by path (v1
   1b50dce). Sort keys are computed once per entry on the worker with
   `icu_collator`, numeric on, folders first while sorting by name.
10. **One watcher for the process.** One inotify instance serves every
    window and tab, with non-recursive watches on each folder shown,
    opened in place or in a column, released when no view shows it. Entry
    events, `CLOSE_WRITE` and `ATTRIB` become row updates, not rescans, so a
    file being written updates its size and thumbnail (`apps.md` Files 2)
    without v1's rescan storms (f4e5dfe); events settle for 100 ms;
    `IN_Q_OVERFLOW` rescans. Remote folders get no watch and are read again
    each time they are shown (`structure/files.md` § Keyboard, F5).
11. **MIME from shared-mime-info, by name first.** `isle-apps` reads globs
    and magic through `xdg-mime` pinned by git revision (crates.io's is
    two years old), cached per extension. A name settles the type; magic is
    read only for an extensionless file on screen, on a worker, and never
    on a remote mount (`files-2026.md` §4). Kinds and icons come from the
    type (Papirus's mimetype icons through the existing lookup; Isle's
    folders, `icons.md`).
12. **Opening a file is Isle's own, everywhere.** `isle-apps` parses
    desktop entries' `MimeType` and reads and writes the MIME apps lists
    by the spec, `isle-mimeapps.list` first. Files, the compositor's launch
    path (Spotlight, the thumbnail, notifications) and `isle open` resolve
    a file's app the same way: the default, then the first that handles the
    type; `text/*` falls back to the text editor; an AppImage or script runs
    nothing and an unknown type opens Open With › Other… (v1 1c5df32).
    `xdg-open` remains only for URLs. "Always open with" and Change All…
    write `mimeapps.list` as GTK does, so Hyprland's apps agree; only the
    folder handler is Isle's alone (choice 25).
13. **Mounts are learned, classed and probed off the UI path.** The engine
    follows `/proc/self/mountinfo` (POLLPRI) and classes each mount local,
    remote (cifs, nfs, `fuse.*`) or autofs. Every call that touches a
    remote mount runs on a blocking pool with a two-second deadline and one
    outstanding probe per mount; a mount that misses it is marked slow, its
    view shows the frame and the mark (`structure/files.md` States, Slow
    volume) and the thread is left to finish. Remote folders get no magic,
    no thumbnails and no child counts; an autofs point is touched only when
    opened. Btrfs subvolume mounts collapse to the device's root, and
    `/boot`, `/var` and swap are hidden (v1 c75cdce).

### Thumbnails

14. **Thumbnails are made by a helper process, `isle-thumbnail`.** Files
    opens it at start (pinned by descriptor, as P14a pins the compositor's)
    and talks to it over a pipe: a request names a file, its type and a
    size, and the answer is the cache path. It decodes images with
    `image`, `jxl-oxide` and `resvg`, a PDF's first page with `hayro`, and
    hands every other type to the `.thumbnailer` entry that claims it
    (ffmpegthumbnailer, glycin, gsf-office on this host), each with a
    timeout. Four at once at most, with a pixel budget per image. A file
    that crashes the helper fails alone; Files restarts it. Not tumbler:
    no priority by what is on screen, and v1 removed it (D85).
15. **The shared cache, by the spec.** `normal` for icons up to 128 points
    at scale 1, `large` above that or at scale 2, `x-large` for the preview
    pane and Quick Look's index sheet. Validity read from the PNG's text
    chunks without decoding (D68); other apps' thumbnails reused; failures
    in `fail/isle-files-<version>/`. The queue is ordered by what is on
    screen, newest scroll first, and cancels what scrolled away. Decoded
    thumbnails in Files are held in an LRU of 48 MB, inside bar 2.

### Jobs

16. **Copying uses the filesystem.** On one filesystem `FICLONE` first
    (the host is btrfs, so a copy there is instant), then
    `copy_file_range` in 8 MB steps for progress, then read and write;
    mode, times and `user.*` xattrs kept. A move is `rename`; across devices
    a copy and then the source removed, item by item.
17. **Nothing is removed before its replacement is in place** (D70,
    `apps.md` Files 5). A copy is written to `.{name}.isle-part` beside the
    destination and moved over it with `renameat2`: `RENAME_NOREPLACE`
    for a new name, `RENAME_EXCHANGE` for a replace, after which the old
    item is dropped. A replaced folder is exchanged, then removed. A
    stopped job removes only its own part files.
18. **A job runs on the engine's worker and asks through a channel.** A
    conflict parks the job, not a thread (v1's hung condition variable,
    a470fed), and its question goes to the window that started it, another
    Files window, or a small window of its own (`structure/files.md` § File
    jobs). Failures are gathered and reported once; space is checked
    before starting. Every job records its undo as it goes, so a stopped
    or failed job can be undone (D71; v1 4266362); a job that replaced
    something records "Can't Undo". Undo is a stack per window (gate 2).
19. **The trash is Isle's own, by the spec, per volume.** Not the `trash`
    crate: its listing walks every mount, which froze cosmic (`files-2026.md`
    §4). The info file first with `O_EXCL`, relative paths under a topdir;
    the Trash view lists the home trash and each local mount's, on a
    worker; Put Back refuses to overwrite and works after a remount;
    Empty and Delete Immediately ask as P1 says; the 30-day removal runs
    when Files starts, off by default.
20. **Compress and Expand through `bsdtar`.** Zip for several items,
    expansion into a folder beside the archive (D71), as jobs with
    progress; `bsdtar` refuses absolute and `..` paths. libarchive becomes
    a dependency.

### The panels

21. **The worker hands FileChooser to Files over a private interface.**
    `isle-portal` gains `FileChooser` (version 4, the three methods). The
    handler holds the frontend's reply and exports the Request, as P11's
    handlers do, and calls `io.github.alexparlett.Isle.Files1.Choose(kind,
    app_id, parent, title, options) → id` on Files, starting it by D-Bus
    activation; Files answers with an `Answered(id, response, results)`
    signal. The Request's Close calls `Files1.Cancel(id)`; Files' name
    leaving the bus answers cancel; so does the parent window closing. One
    answer per request, always (`apps.md` Files 23, bar 7). A signal rather
    than a held method reply, so no D-Bus timeout can answer for the user.
    `Files1` takes calls only from the portal backend's owner (B83's
    check, inverted). `isle-portals.conf` gives FileChooser to Isle.
22. **The panel is a modal child through standard protocols.** Files
    imports `wayland:` handles with xdg-foreign v2 and marks the panel
    modal with xdg-dialog; the compositor already hangs a modal child under
    its parent's bar as a sheet (`wm/places.rs`). For an `x11:` parent the
    worker exports the X11 window's surface itself and hands Files a
    `wayland:` handle, so Files has one path. No parent, or a parent gone:
    a window of its own, centred and titled. One panel per parent; a
    second request waits (`structure/files.md` § The Open and Save panels).
23. **The panel belongs to the asking app.** The compositor counts a
    window parented to another client's window as that window's app: it
    is no switcher entry, Mission Control and the Window menu show it under
    the asking app, and the on-demand bar shows the asking app's name with
    the panel's own Edit, View, Go, Window and Help (`structure/files.md` §
    Shared places). Filters match by glob and by MIME type, wildcards
    included (v1 hid every file on a MIME rule); `choices` and
    `current_filter` are answered; `writable` is always sent; a chosen file
    joins the recently-used list, as GTK's chooser does.

### Volumes and handlers

24. **`isle-disks` grows to Locations.** Internal volumes
    beside removable ones, with sizes and free space (from UDisks2 and a
    probed `statvfs`), the hints honoured, LUKS unlock through
    `Encrypted.Unlock` with polkit, and "Remember in Keychain" storing the
    passphrase in the Secret Service under GNOME's `gvfs-luks-uuid` schema,
    so the keyring that unlocks at sign-in unlocks the volume (Keychain,
    the app, is P19). Network shares come from the mount table. GIO's
    `MountOperationHandler` is not served: nothing in Isle mounts through
    gvfs (P9 left it here; Connect to Server is out, gate 2).
25. **Files is the folder handler and FileManager1 only in Isle.** Files'
    D-Bus service files activate `io.github.alexparlett.Isle.Files` and
    `org.freedesktop.FileManager1`; outside an Isle session Files exits
    without taking a name, so under Hyprland a browser's Show in folder
    falls back to opening the folder through v1's handler. The folder
    handler is set in `~/.config/isle-mimeapps.list`, read only when
    `XDG_CURRENT_DESKTOP` is Isle, by a new user step of `isle setup`.
    FileManager1 selects the items, uses the startup id as an activation
    token, opens a tab in the front window on the current desktop (a window
    when there is none), and ShowItemProperties opens Get Info.

### Quick Look

26. **Quick Look is `io.github.alexparlett.Isle.QuickLook1` on Files' name**
    (the name the record left open). `Show(uris, index, caller, origin,
    mode, token)`, `Select(index)`, `Close()`; signals `Shown(index)` and
    `Closed(index)`; a `Visible` property. `caller` is an xdg-foreign handle
    or the compositor's own handle for a shell surface (Spotlight, the
    capture thumbnail); `origin` the item's rectangle in the caller for the
    zoom; `mode` none, markup or trim. The caller keeps the keyboard and
    calls `Select` as its selection moves, so Quick Look needs no keys of
    its own until Tab, Markup or Trim (`structure/files.md` § Quick Look).
    A Files window and a panel use the same window in-process.
27. **Quick Look's window floats over its caller through an Isle
    protocol.** Nothing standard says "a child of that surface, above it,
    never focused on map, opening from this rectangle". `isle_titlebar_v1`
    gains version 2 with a per-toplevel object: `set_float_over(handle)`,
    `set_keyboard(none | on_request)` and `open_from(x, y, w, h)`. The
    compositor keeps the window above its caller (above Spotlight in the
    overlay layer), on its caller's desktop, out of the switcher, gives it
    the keyboard only on a click into its toolbar or a request while Markup
    or Trim is on, and runs the zoom from the rectangle with the window's
    own open as the fallback.
28. **The renderers.** Images through Freya's Skia decoders, fitted;
    text and code read to 256 KiB on a worker and highlighted with
    `syntect` and bat's syntaxes (not tree-sitter's grammar per language);
    PDF pages rendered by `isle-thumbnail` with `hayro` on request, the
    pages near the scroll position only; audio and video through
    `freya-video` (ffmpeg as a process); fonts as Skia typefaces; archives
    listed by `bsdtar -t` on a worker; folders and everything else as icon
    and facts. The preview pane and Get Info's Preview use the same
    renderers at their sizes.
29. **Markup edits PNG, JPEG and WebP; Done keeps the original in the
    Trash.** Marks are a vector layer over the image (`capture.md`'s pen,
    highlighter, line, arrow, rectangle, ellipse, text, blurred redaction
    and crop, the eight colours) with Rotate Left; Done renders them into
    the image at full resolution, writes it beside the original, moves the
    original to the Trash and renames the new one into its place, so Put
    Back recovers it and nothing is lost first. Other image types show
    Markup disabled.
30. **Trim cuts exactly by re-encoding.** A stream copy cuts at keyframes,
    seconds off what the handles show. Done runs ffmpeg as a file job with
    `isle-record`'s encoder choice (NVENC, else x264; AAC), writes beside the
    original and replaces it as Markup does. The strip's frames come from
    `isle-thumbnail`.

### Search

31. **Files searches by walking, as P1 answered; no content index.**
    "This folder" filters in memory as typed; "Everything under" and
    "Home" walk with `ignore`'s parallel walker on the engine's worker,
    streaming results, skipping hidden folders unless hidden files are
    shown and honouring `.gitignore`, as Spotlight's index does (B76).
    Contents contain uses the content search Spotlight uses, with
    Files' limit of 500 matching files rather than Spotlight's twenty.
    Saved searches run again when opened. The compositor's index is not
    asked: it holds names only, ranked for Spotlight, and a walk of home
    is fast enough (`p10-spotlight.md`). B76's no-content-index stands.
32. **Spotlight reaches Files through the Application interface.** "Show
    all in Files" calls `ActivateAction("search", [query, scope])`;
    Cmd+Return calls FileManager1 with the URI encoded and an activation
    token (P10's call sends neither); Cmd+Y on a file calls
    `QuickLook1.Show` with the result list, the row's rectangle and
    Spotlight's handle, and follows `Shown` to move its selection. The
    Files scope gains its "Show all in Files" row and the tile menu its
    Show in Files, each shown only while a FileManager1 owner is
    activatable, as P10 left them.

### The shell's ties

33. **The rest of the shell joins.** The capture thumbnail's click opens
    Quick Look in Markup on a shot and in Trim on a recording; its menu
    gains Show in Files and Markup; its Move to Trash goes through
    `isle-fs`. The island's settings-error notice and the drive notice
    reveal through FileManager1 and post as Files. The on-demand bar with
    no window focused shows Files' name and menus without starting it
    (`menu-bar.md` For the user 1). `shortcuts.files` defaults to ⌥⌘Space,
    macOS's Finder search window: a new Files window with its search field
    focused. Settings › Wallpaper's Choose… opens the panel.
34. **Settings › Apps gains Default apps.** P13 left default apps to P15
    (`p13-settings.md` Not in P13). Four rows: web browser, mail, text
    editor, terminal, each a pop-up of the apps that handle it, writing
    `mimeapps.list` through `isle-apps`; Open in terminal uses the terminal
    row. Per-type defaults stay in Files' Open With and Get Info.

### Proof

35. **Tested headless, run in the VM, used on the host.** Headless: the
    engine against a 50,000-entry folder (first batch, merge, a write
    during the listing), the watcher's row updates and overflow, a mount
    that never answers (a FUSE fixture that sleeps), every job's order of
    renames against a fault injected at each step, undo of a stopped job,
    the trash's topdirs, MIME and default-app resolution, the panel
    protocol against a fake frontend (answer once on accept, cancel, Close,
    parent closed, Files killed), Quick Look's float and keyboard against
    the compositor, Files' component tests as Settings' are. Bars 1, 2, 3,
    4, 7 and 9 measured. The VM: Firefox, Zed, a GTK 4 app and an Electron
    app opening and saving through the panel with Files not running,
    Chromium's Show in folder, a USB image mounted and ejected, an
    encrypted image unlocked, Quick Look from Files, a panel, Spotlight and
    the capture thumbnail. The host, the user's: a day of use on the
    installed package.

## The workspace

```
crates/
  isle-fs/                listing, watching, mounts and probes, sort keys,
                          trash, jobs, undo records, recents (new)
  isle-files/             the app: windows, views, sidebar, search, jobs
                          UI, Get Info, settings window, Quick Look,
                          Markup, Trim, panels, Files1, QuickLook1,
                          FileManager1; binary isle-files (new)
  isle-thumbnail/         the helper: decoders, .thumbnailer entries, PDF
                          pages, video frames (new)
  isle-app/               many windows, per-surface extras, data device
                          (drag and drop, clipboard), keep-alive, Go menu,
                          LauncherEntry, xdg-foreign import, xdg-dialog,
                          the float protocol
  isle-ui/                sheets, popovers, virtual list, grid and table
  isle-apps/              MIME (xdg-mime), MimeType, the MIME apps lists
  isle-disks/             internal volumes, sizes, hints, unlock, mounts
  isle-search/            the content search as a shared function
  isle-portal/            FileChooser
  isle-compositor/src/
    portal.rs             FileChooser's handler and the hand-off
    wm/                   foreign-parented children under their app;
                          Quick Look's float, keyboard and zoom
    spotlight.rs          Cmd+Y, Show in Files, Show all in Files
    thumbnail.rs          Quick Look, Markup, Show in Files
    launch.rs             files opened through isle-apps
  isle-system-settings/   Apps › Default apps; Wallpaper's Choose…
  isle-cli/src/setup/     the folder handler step; `isle open`
data/
  apps/                   Files' desktop entry and two service files
  protocols/              isle-titlebar-v1.xml version 2
  portal/                 isle-portals.conf: FileChooser=isle
pkg/arch/PKGBUILD         isle-files, isle-thumbnail; libarchive
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 15.0 · The note and the roadmap

`docs/research/rust-desktop/files-2026.md`; the roadmap's P15 entry.

### 15.1 · The app platform for many windows

`isle-app`'s window API, the extras per surface, the focused window's
commands, the Go menu, the keep-alive, LauncherEntry; sheets and popovers
into `isle-ui`; the virtual list, grid and table out of Spotlight into
`isle-ui`, Spotlight moved onto them.

### 15.2 · The engine

`isle-fs`: the streamed listing, the row diff, sort keys, the one watcher,
the mount table, classes and probes; MIME in `isle-apps`. The 50,000-entry
and stalled-mount tests.

### 15.3 · The window

`isle-files` with its window, toolbar, sidebar (Recents, Favourites with
GTK's bookmarks, Locations, Trash), tabs, List view, navigation, the keys,
state; one instance, FileManager1 and Files1's skeleton; the desktop entry
and service files; leaving without a name off Isle.

### 15.4 · Icons, Columns and the preview pane

The other two views, groups, View Options, per-folder memory, path and
status bars, the preview pane with icons and facts.

### 15.5 · Thumbnails

`isle-thumbnail`, the cache, the queue by what is on screen, the LRU.

### 15.6 · Jobs, undo and the Trash

Every job with its order of renames, progress, questions, failures and
the space check; the undo stack; the jobs button and popover, the
switcher card, the notification at the end; the Trash view, Put Back,
Empty, the 30-day removal; the thumbnail's trash moved here.

### 15.7 · Selection, rename, drag and drop, clipboard

Selection by pointer and keys, rename in place and many, the data device
in `isle-app`, drags in and out, spring-loading, the file clipboard.

### 15.8 · Opening files

MimeType and the MIME apps lists in `isle-apps`; Open With and Other…;
the compositor's launch path and `isle open` on them; Open in terminal;
Set as wallpaper; AppImages' kind and their Trash as uninstall.

### 15.9 · Volumes

`isle-disks` grown; mount, unmount, eject and the in-use alert; LUKS with
the Secret Service; tabs leaving an ejected volume; the drive notice as
Files'.

### 15.10 · Search

The scope bar, the walk, criteria, contents, saved searches; Spotlight's
"Show all in Files".

### 15.11 · Get Info and Files' settings

Get Info, Summary and the Inspector; comments; permissions; the settings
window and its index rows.

### 15.12 · The panels

FileChooser in `isle-portal` and the worker's hand-off; `Files1`; the
panel as a sheet through xdg-foreign and xdg-dialog, the X11 parent's
handle, the parentless window; the compositor's grouping under the asking
app; Open, Save collapsed and expanded, SaveFiles, filters, choices, the
Replace step, memory per app; `FileChooser=isle`; Settings' Choose….

### 15.13 · Quick Look

`QuickLook1`; the float protocol in the compositor and `isle-app`; the
renderers; several items, the index sheet, full screen; the preview pane
and Get Info on the renderers; Spotlight's Cmd+Y.

### 15.14 · Markup and Trim

The tool row and marks, Done and Revert, Rotate Left; the trim bar,
re-encoding as a job; the capture thumbnail's click and menu.

### 15.15 · The shell's ties

FileManager1's caller fixed; the tile menu and the island's notice; the
on-demand bar's Files; `shortcuts.files`; Settings › Apps' Default apps;
the `isle setup` step.

### 15.16 · The package

The two binaries, the desktop entry and service files, libarchive and the
optional thumbnailers in the PKGBUILD; the package tests.

### 15.17 · Measures and the VM

Bars 1, 2, 3, 4, 7 and 9; the VM's run of choice 35; results in
`docs/research/rust-desktop/p15-files.md`.

### 15.18 · The host and close

The user's run on the installed package. ARCHITECTURE gains How Files
lists a folder, How a file job runs, How the panels answer and How Quick
Look floats; decisions recorded; `apps.md`'s and `platform.md`'s open points
answered; the roadmap's P15; this sheet ends with its outcome.

## Reviewed

The user reviewed the plan on 2026-10-20 and took every recommendation: one
packet with the panels halfway, no new gate, `isle-fs`, `isle-files` and the
`isle-thumbnail` helper, `isle-app` for many windows with its own data
device, the hand-off over `Files1` with an answer signal, the panel through
xdg-foreign and xdg-dialog, `QuickLook1` floating through
`isle_titlebar_v1` version 2, Isle's own opening of files, Files as the file
manager only in Isle, Trim by re-encoding, Markup on PNG, JPEG and WebP,
Default apps in Settings › Apps, ⌥⌘Space for `shortcuts.files`, search by
walking.

## Not in P15

Tags and a Tags group (gate 2; the places are kept). Connect to Server,
gvfs mounting and MTP (gate 2). A Gallery view. A content index (B76).
Session restore across sign-ins (P20; Files reopens its own windows at
launch). Qt apps reaching the panels under `QT_QPA_PLATFORMTHEME=gtk3`
(P14b, `session-2026.md` §6). The Cleaner's views over `isle-fs` (P19).
The Keychain app (P19). Thumbnails on network volumes. Quick Look for
apps' own surfaces beyond the panels, Spotlight and the thumbnail.

## Depends on, and feeds

- Needs P6 (the on-demand bar), P7 (LauncherEntry, the switcher card),
  P8 (`isle-disks`, notifications), P9 (polkit for mounts), P10 (Spotlight,
  the content search), P11 (`isle-portal`, the thumbnail, `isle-record`'s
  encoder choice), P13 (`isle-app`, Settings), P14a (the package, `isle
  setup`).
- P16's Mission Control shows panels under the asking app; P19's Cleaner
  builds on `isle-fs`'s jobs and Trash, its Keychain on the Secret Service
  items; P14b's Qt route ends at these panels; P20's session restore reopens
  Files' windows through its state.

## Done when

- `cargo test` passes headless with the engine, jobs, trash, MIME, panel
  and Quick Look tests.
- A 50,000-entry folder draws its first screen before its listing ends and
  scrolls without a dropped frame (bar 9); a stalled mount never stops
  the window (bar 4).
- In the VM, Firefox's Open dialog is Isle's panel, a sheet on its window,
  with Files not running beforehand (`platform.md` acceptance 15), within
  300 ms (bar 1); `kill -9` of Files answers cancel (bar 7).
- Every Files row of `apps.md`'s Parity map works (bar 11).
- On the host, the user lives a day in Isle with Files as the file manager
  and chooser.

## Risks

- **The cold-start bar.** A panel within 300 ms of a request when Files is
  not running means D-Bus activation, a Freya window and the first listing
  in that time. Settings' start is the measure to beat; if it misses, Files
  starts windowless with the session as a Login Item, which costs bar 3's
  exit and needs the user's say.
- **Drag and drop beside winit.** A second `wl_pointer` must see the same
  press serial winit's does for `start_drag`; Wayland sends button events
  to every pointer resource of the client, but this is new ground for
  `isle-app`. 15.7 tests it first; the fallback is drags inside a window
  only, through Freya's `DragZone`, with drops from other apps still read.
- **hayro's fidelity.** It is experimental and cannot draw unembedded CID
  fonts; some PDFs will preview badly. pdfium is the better renderer but
  not packaged here; if hayro fails too often, `isle-thumbnail` loads
  `libpdfium.so` where installed.
- **Memory.** Bar 2 is 80 MB PSS in software. Thumbnails, the syntax set
  and a decoded image add up; the LRU is the lever, and Quick Look's
  decoded image is dropped when it closes.
- **Freya's many windows on Wayland.** Every window is another software
  surface and another set of extras; the second window is new to
  `isle-app`, and P13's single-window assumptions will surface in 15.1.
- **The folder handler step.** If `isle-mimeapps.list` is missed,
  `xdg-open` of a folder in Isle opens v1's Files window under Isle.
  `isle setup --check` lists it.

## Outcome

Built 2026-10-21 in phases 15.0–15.17; 15.18's host run, part of done, is
the user's, with its checklist in `docs/research/rust-desktop/p15-files.md`.
B104–B111 record the decisions; ARCHITECTURE gains How Files lists a
folder, How a file job runs, How the panels answer and How Quick Look
floats; `apps.md`'s and `platform.md`'s open points are answered.
Differences from the plan:

- **An X11 parent's panel is a window of its own** (choice 22): an
  Xwayland surface cannot be an xdg-foreign parent and the compositor has
  no way to export one, so the worker hands Files no handle for it.
- **A started-by-activation Files opens no window**: its service files run
  `isle-files --gapplication-service`, which `isle-app` takes as a request
  for nothing, as GApplication does.
- **Quick Look over Spotlight is drawn above the shell's overlays** but a
  click on it still reaches Spotlight; Files' own windows and panels are
  floated over as planned.
- **Syntax highlighting uses syntect's own syntaxes,** not bat's (choice 28).
- **The on-demand bar's Files** names Files with the fallback menus; its
  key-translation group is for other file managers and is left alone.
- **The thumbnail helper gained `page` and `frames` modes** for Quick Look's
  PDF pages and Trim's strip.

Found and fixed on the way: `isle-apps`' name cache keyed a whole-name
glob's answer by its extension, so one `todo.txt` made every `.txt` a
Todo.txt file; a window closed before it was created stayed open (a quick
Close of a portal request left an orphaned panel, found in the VM).

Not built, carried in `progress.md`: the panel's parent closing does not
yet cancel; the panel's size per app; Quick Look's zoom from the item, full
screen and fonts; the trim bar's playback and Trim as a job with progress;
asking before closing unsaved Markup; Last modified's date range; removing
a saved search; Get Info's and the preview pane's use of the renderers.

