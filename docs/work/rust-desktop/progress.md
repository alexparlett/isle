# Progress

## 2026-10-26

- P12 host-run fixes: ⌘Q joins the escape set a game in front leaves to
  Isle, so a game that took it can be quit (B30, `gaming.md`); X11 windows
  out of view are stacked below those shown, since Xwayland routes a click
  by X11's stacking and a full-screen game left on another desktop took
  Steam's clicks after ⌘Tab. The X11 test client gains a pointer grab let
  go on focus out, as Wine's ClipCursor is.

## 2026-10-21

- P15 15.18, the close, host run pending: ARCHITECTURE gains How Files
  lists a folder, How a file job runs, How the panels answer and How Quick
  Look floats, and the app section many windows and the data device;
  B104–B111 record the packet's decisions; `apps.md`'s and `platform.md`'s
  open points are answered; the roadmap and state name P15 built;
  `p15-files.md` ends with its outcome. The host run on the installed
  package is the user's, its checklist in
  `docs/research/rust-desktop/p15-files.md`.
- P15 15.17: measures and the VM, in
  `docs/research/rust-desktop/p15-files.md`. Headless (`files_bars`): bar
  3 (gone within 5 s of its last window), 4 (answers with no UDisks2), 7
  (a kill answers a held panel with cancel) and 9 (50,000 entries: first
  batch at 0 ms, the listing done at 1.5 s, debug) pass; idle memory 87 MB
  PSS in a debug build, over bar 2's 80 MB, for the host's release build
  to settle. In the guest: a FileChooser request through the real
  frontend started Files by activation with the panel alone; cancel and a
  chosen folder answered; Firefox's ⌘O raised the panel as a sheet on its
  window; Show in folder, a loop image mounted and ejected from Locations,
  the drive notice as Files, Quick Look over Files. Found and fixed: a
  window closed before it was created stayed open (a quick Close left an
  orphaned panel). Files logs its first batch and the end of each listing.
- P15 15.15: the shell's ties. The compositor's calls to Files go through
  one place (`files_link.rs`) that encodes the URI and mints an activation
  token: Spotlight's ⌘Return and the tile menu's new Show in Files reveal
  through FileManager1, the Files scope gains "Show all in Files" (the
  query as a search of Home in a Files window), and both show only while
  Files is installed. The drive notice posts as Files and its click mounts
  and shows the volume through FileManager1; the island's settings-error
  Show in Files selects the file. With no window focused the on-demand bar
  names Files and shows its fallback menus without starting it.
  `shortcuts.files` is built and defaults to ⌥⌘Space: a new Files window
  with its search field focused (Files' `find` action), listed in Settings'
  Keyboard Shortcuts. Settings › Apps gains Default apps (web browser,
  mail, text editor through `mimeapps.list`; the terminal writes
  `shortcuts.terminal-command`, which Files' Open in terminal reads),
  indexed for Spotlight. `isle setup` gains `files-folders`, giving folders
  to Files in `isle-mimeapps.list` where Files is installed, with undo.
  Files' D-Bus activation starts it as a service with no window.
- P15 15.16: the package. The PKGBUILD builds `isle-files` and
  `isle-thumbnail` beside Settings, so no GPU renderer is linked into
  them, and installs them with Files' desktop entry and the D-Bus
  activation files for Files and FileManager1 (the latter under Isle's
  own file name, so another file manager's package does not conflict).
  `libarchive` becomes a dependency (Compress, Expand and archive
  listings use `bsdtar`); `udisks2` is optional, for drives and disk
  images. The package tests allow Files' `--write-index` as Settings'. Not
  yet run: `makepkg` itself (15.17).
- P15 15.14: Markup and Trim. Quick Look's toolbar gains Markup (PNG,
  JPEG and WebP; disabled for other kinds) and Trim (video and audio).
  Markup's tool row: pen, highlighter, line, arrow, rectangle, ellipse,
  text, redact, crop, undo, Rotate Left, the eight annotation colours in a
  popover, Revert and Done. Marks are a vector layer in the image's pixels,
  drawn as an SVG over the picture with the same fit; Done renders them at
  full resolution (redactions blurred, crop and turns applied) with
  `image` and `resvg`, writes the result beside the original, moves the
  original to the Trash and renames the new file into its place. Trim:
  a strip of ten frames from `isle-thumbnail`'s new `frames` mode, the kept
  range dragged from either end with its length, Revert and Done; Done
  re-encodes the range with ffmpeg (NVENC, else x264, with AAC; audio in
  its own codec) and replaces the file the same way. `QuickLook1.Show`
  takes `markup` and `trim`. The capture thumbnail's click opens a shot in
  Markup and a recording in Trim over the thumbnail, and its menu gains
  Show in Files and Markup. Not built: the trim bar's playback and
  playhead, Trim as a file job with progress, asking before closing with
  unsaved marks, moving or resizing a mark once drawn.
- P15 15.13: Quick Look. Space and ⌘Y in Files open it on the selection
  and it follows the selection; Escape or Space again close it. Images and
  SVGs fitted and centred, text and code to 256 KiB highlighted with
  syntect's own syntaxes (not bat's), PDF pages from `isle-thumbnail`'s
  new `page` mode as they scroll in, audio and video through freya-video,
  archives listed by `bsdtar`, icon and facts for the rest; ‹ › with
  "i of n", the index sheet (⌘Return), Open with {app}. Files serves
  `io.github.alexparlett.Isle.QuickLook1` (Show, Select, Close, Shown,
  Closed, Visible); Spotlight's ⌘Y on a file shows it and ⌘Y again or
  closing Spotlight closes it. `isle_titlebar_v1` gains version 2's
  `isle_float_v1` (float over a handle or `isle:` and a shell surface,
  keyboard none or on request, open from a rectangle): the compositor
  parents a float to its caller, raised with it and out of the switcher,
  never focuses it as it maps or on a click below its top band, and draws
  one over a shell surface above the shell's overlays, so Quick Look sits
  over Spotlight while Spotlight keeps typing. Not built: a click on
  Quick Look over Spotlight reaches Spotlight, the zoom from the item's
  rectangle, fonts as a sample, full screen, Spotlight following `Shown`
  through a list, and Get Info's and the preview pane's use of the
  renderers.
- P15 15.12: the panels. `isle-portal` serves FileChooser (version 4,
  OpenFile, SaveFile, SaveFiles) and hands each request to Files'
  `io.github.alexparlett.Isle.Files1.Choose`, starting Files by D-Bus
  activation (retried while it comes up) and waiting for its one
  `Answered` signal, the Request's Close (Files1.Cancel) or Files leaving
  the bus; `Files1` takes calls only from the backend's owner, and
  `isle-portals.conf` gives FileChooser to Isle. A start by activation is
  `--gapplication-service` and opens no window. `isle-app` imports a
  `wayland:` parent with xdg-foreign and marks the window modal with
  xdg-dialog, so the compositor hangs the panel as a sheet; an app can
  export its own window's handle, and `isle_app::portal::choose_file`
  asks the portal as any app does. The compositor counts a dialog on
  another app's window under that app in the switcher. The panel: Open
  with the sidebar, the three views, the folder pop-up, search, filters
  (by glob and by MIME type, wildcards included) dimming what cannot be
  chosen, choices as checkboxes and pop-ups, New Folder, Return and
  Escape; a folder request chooses the folder shown; Save collapsed (Save
  As, Where, Format changing the extension) and expanded into the
  browser, remembered per app with the folder and view; SaveFiles chooses
  a folder; a taken name asks Replace in the panel. Answers carry `uris`,
  `choices`, `current_filter` and `writable`; a chosen file joins
  `recently-used.xbel`. Settings › Wallpaper's Choose… opens the panel,
  falling back to typing a path with no portal. Not built: an X11
  parent's panel is a window of its own (an Xwayland surface cannot be an
  xdg-foreign parent), the parent closing does not yet cancel, the panel's
  size per app, and the sheet still carries the compositor's title bar.
- P15 15.11: Get Info and Files' settings. ⌘I opens a Get Info window per
  item, ⌃⌘I one summary, ⌥⌘I the Inspector following the focused window's
  selection; FileManager1's ShowItemProperties opens Get Info. Sections
  collapse and stay collapsed: General (size on disk, a folder counted),
  a volume's format and space, an image's dimensions, Name & Extension
  through the rename job, Comments in `user.xdg.comment`, Open with with
  Change All…, a preview, Sharing & Permissions with Allow running as a
  program, editable on the user's own items. Files' preferences replace
  the schema's example app section as `app[io.github.alexparlett.Isle.Files]`:
  the new-window folder, folders in tabs or windows on ⌘-double-click,
  each sidebar row, filename extensions shown, the extension and Empty
  Trash warnings, the thirty-day Trash purge, folders on top, and the
  search scope. ⌘, opens the settings window (General, Sidebar, Advanced)
  with the configuration-error notice and refusals on their rows. The
  rows are Files' own record; `isle-files --write-index` writes
  `data/files/settings-index.kdl`, which Spotlight shows as "Files Settings
  › …" while Files is installed, launching Files with
  `isle-settings://<pane>/<row>`. Not built: the window titled after its
  pane, raising the open settings window without a token.
- P15 15.10: search. ⌘F focuses the toolbar's search field; typing filters
  the place shown at once through `isle-fs`'s `Folder`, which now sets
  aside what a name filter leaves out as it does hidden items. Return
  searches everything under the folder (Home from a gathering): a
  `Place::Search` walked by `ignore`'s parallel walker, skipping hidden
  folders unless hidden files show and what `.gitignore` names, streaming
  batches into the tab, at most 5,000 results. The scope bar (This folder,
  Everything under “X”, Home, +, Save) and criteria rows (Kind, Last
  modified, Contents contain) change the search in place; Contents contain
  runs Spotlight's content match, now `isle_search::files::Contents`, on at
  most 500 files of 20 MB or less. Results show a Where column in List;
  Columns is off and shows as List while a search stands. Save names a
  search in a sheet; saved searches are kept in Files' memory and listed
  in Favourites, and run again when opened. `ActivateAction("search",
  [query, folder URI])` opens a search tab for Spotlight's "Show all in
  Files". Fixed in passing: `isle-apps`' name cache keyed a whole-name
  glob's answer by its extension, so one `todo.txt` made every `.txt` a
  Todo.txt file; names a glob other than `*.suffix` matches are now asked
  of the database each time, the rest cached by everything after their
  first dot. Not built: Last modified's date range, removing a saved
  search, Files › Settings' default scope (15.11).
- P15 15.9: volumes. `isle-disks` lists internal volumes that are not the
  system's beside removable drives and the user's loop images, with size,
  type, UUID and whether it can be taken out; an encrypted volume shows
  locked until unlocked, then as its filesystem alone; `Encrypted.Unlock`,
  a forced unmount for Force Eject, a busy refusal told apart, and the
  keyring under GNOME's `org.gnome.GVfs.Luks.Password` schema so either
  desktop unlocks with a kept passphrase. The compositor's drive notice
  keeps to removable drives. Files' Locations: the computer, each volume
  (dimmed while unmounted, its Eject button while a removable one is
  mounted, a capacity mark past nine-tenths full from free space read with
  a deadline), the network shares the system mounted (cifs, nfs, sshfs,
  rclone and the like, not the session's own FUSE helpers), and the Trash.
  A click mounts and shows a volume, unlocks one first with the keyring's
  passphrase or the passphrase sheet (Remember in Keychain); Eject asks
  with Force Eject when the volume is in use; a tab on a volume that goes
  moves to the new-window folder. The sidebar's folders and Trash take
  drops. Not tested headless: unlocking (the fake UDisks2 has no
  encrypted volume).

- P15 15.8: opening files. `isle-apps` reads entries' `MimeType` and the
  MIME apps lists in the spec's order, Isle's own list always first
  (`isle-mimeapps.list`), with defaults, added and removed associations
  over the types entries declare, a type's parents after it (Markdown
  offered the text editors); writes a default into `mimeapps.list` as GTK
  does. A file opens in its default, text falls back to the text editor,
  and a program or AppImage with no app asks rather than runs. The
  compositor's `Launch::Open` (Spotlight, the thumbnail, notifications,
  `isle apps open`) now resolves this way off the loop, links by their
  scheme handler, a file nothing opens going to Files' Open With
  (`isle-open-with:` handed to Files), and `xdg-open` only when Isle's
  lists name nothing. Files opens files through the compositor's launch
  path, asks with the Open With sheet (Show all apps, Always open with),
  opens a terminal in a folder (`shortcuts.terminal-command`, in a scope of
  its own), sets an image as the wallpaper (`wallpaper.source`), and calls
  an AppImage "Application (AppImage)". Not built yet: Open With as a
  submenu of a context menu (context menus come later in P15), and Files
  adding what it opens to the recently-used list.

- P15 15.7: rename, the file clipboard and drag and drop. `isle-app` gains
  its own `wl_data_device` on a thread and queue of its own beside winit's
  (smithay-client-toolkit 0.20, no calloop), replacing smithay-clipboard:
  the selection in any type, text for Freya's fields through it, drags out
  and onto the app's windows. A drag over a window reaches it as the data
  device's motion, not pointer events, so the window is fed Freya pointer
  moves and its hover follows the drag; drops arrive as `Dropped` with the
  data in the type asked for. Files: Return renames in place (a name
  already taken refused under the field, a leading dot and a changed
  extension asking first), several selected open Rename Many with `#` and
  a start number; ⌘C and ⌘X put files on the clipboard as
  `x-special/gnome-copied-files`, `text/uri-list` and text, ⌘V copies or
  (after ⌘X) moves, ⌥⌘V moves, ⌥⌘C copies the pathname; cut items are
  dimmed; a drag of the selection starts past four points, a folder under
  it takes the drop ring, and a drop moves within one volume and copies
  across. Found and fixed: handlers made anew each render rebuilt every
  row; the rename's and pick's are made once and act on the current tab.
  Not built yet: a drag image, spring-loading, the sidebar, tabs and path
  bar as drop targets, ⌥ and ⌘ during a drag, the cut item's dashed
  outline, and the extension left out of the rename's selection (the whole
  name is selected).

- P15 15.6: jobs, undo and the Trash. `isle-fs` runs every job of
  `apps.md` Files 4–9 on the caller's thread. Copies go by reflink, the
  kernel's copy in 8 MB steps, or streaming, written to a part file and
  renamed in with `RENAME_NOREPLACE`, or exchanged with what they replace
  and the old dropped. Moves go by rename, across devices by copy then
  removal. Name conflicts are asked through the job's control (Keep Both,
  Replace, Merge for folders, Apply to all); failures are gathered; undo is
  recorded as the job goes (a replace or an erase can't be undone). The
  Trash follows the spec per volume (info first with `O_EXCL`, paths
  relative under a volume's top, Put Back refusing to overwrite and
  following a remounted volume). Files runs each job on a thread of its own
  and follows it from an app-level task (`isle-app` gains `AppHandle::spawn`
  for work that outlives its window), holding the app open. The jobs
  button and popover, the switcher card's progress, the conflict and
  failures sheets, the space check, Undo naming its job (an app's command
  now replaces a standard one of its id), the Trash view with Put Back,
  Delete Immediately… and Empty Trash… (both asking), Duplicate, New Folder
  and New Folder with Items, Make Alias, Compress and Expand. The capture
  thumbnail's Move to Trash goes through `isle-fs`. Found and fixed: a home
  trash not made yet counted as another volume, so items went to
  `$top/.Trash-$uid`; the tests had put files in the host's
  `/tmp/.Trash-1000`, removed. Not built yet: Redo, the notification when no
  Files window is in front, items being written shown dimmed with their
  bar, the window's attention request on a question and a window of its
  own for a question with none open, Replace leading and Merge under ⌥ in
  the conflict sheet (Merge is its own button), the 30-day removal.

- P15 15.5: thumbnails. `isle-thumbnail` (library and helper binary) keeps
  the freedesktop cache by spec 0.9.0: named by the URI's MD5, valid by
  `Thumb::URI`, `Thumb::MTime` and `Thumb::Size` read from the PNG's text
  chunks without decoding, a larger size serving a smaller, failures under
  `fail/isle-files-<version>/`, written beside and renamed, 0700 and 0600.
  The helper takes JSON lines on four threads: images through `image` with
  pixel and memory limits, SVG through `resvg`, a PDF's first page through
  `hayro`, everything else through the `.thumbnailer` entry that claims
  its type, each with a ten-second deadline. Files pins the helper by
  descriptor, queues asks newest first, drops an ask whose cell has gone,
  keeps at most four with the helper, restarts it if it dies, reads the
  cache off the window's thread, and holds 48 MB of thumbnails; Icons and
  the preview draw them fitted and centred, the kind's icon until then.
  Found and fixed: a window's command effect acted on the tab current when
  the window first drew, so after a reveal opened a tab, ⌘1 changed the
  first; commands now act on the tab current when each is taken. Files'
  saves of its state no longer race each other. tmpfs, the kernel's own
  filesystems, are local: only Remote mounts lose thumbnails and watching.

- P15 15.4: Icons and Columns, the preview and the bars. ⌘1 Icons (a grid
  that fills the width, the icon on a plate and its name in a capsule when
  selected, sizes 16 to 256 on ⌘+ and ⌘−, item info), ⌘2 List, ⌘3 Columns (a
  column per folder chosen, each its own listing, the preview last); groups
  by kind, date or size with headings (⌃⌘0 and View's Group By), Sort By;
  the toolbar's view switcher, Group and Sort menu and View Options (⌘J,
  with Use as Defaults); each folder's view remembered (the last 2,000)
  with a default for the rest. The preview pane (⇧⌘P): the item's icon,
  name, kind and size, Created, Modified, Last opened, a folder's count and
  size counted on a thread of its own, a stack for several. The path bar
  (⌥⌘P) and the status bar (⌘/) with free space read off the window's
  thread. Papirus's small icons come from its 22 point set, whose places
  are in colour. Not built yet: List's disclosure triangles, the header's
  column menu and column resizing, Columns' keys between columns and its
  Right Size menu, the status bar's icon slider, and a two-line name
  centred under its icon (one line centres; Freya centres text within the
  label's maximum width).

- P15 15.3: the window. `isle-files` (binary `isle-files`, id
  `io.github.alexparlett.Isle.Files`) opens windows from its delegate: the
  sidebar of Recents, Favourites (Home, the xdg-user-dirs folders in
  macOS's order, GTK's bookmarks) and Locations (the computer, Trash), the
  toolbar with back, forward and the folder's title and count, tabs with
  their bar, and List view over a virtual list (Name, Date Modified, Size,
  Kind; sort by header; stripes; joined selection; ⌘- and ⇧-click; the
  arrows). Each tab lists on its own thread, reads touched names again
  from the shared watcher, and holds its selection by path; hidden files
  wait aside until ⇧⌘. shows them in every window. The Go menu's places,
  back, forward and ⌘↑ ⌘↓; windows and tabs reopen at launch from the
  `files` state domain. FileManager1's ShowItems and ShowFolders become a
  tab in the front window with the item selected, and a second launch
  hands its folder over; off an Isle session Files takes no name and
  leaves. The desktop entry (`inode/directory`, Isle only) and the two
  D-Bus service files. `isle-app`'s chords gain named keys and Control;
  `isle-apps`' icon lookup gains file types and places (Papirus's
  mimetypes and places); the folder model is keyed by path, for
  gatherings. 32 Phosphor glyphs added from `@phosphor-icons/core` 2.1.1
  for Files' places and later phases. Gaps for later phases: opening files
  (15.8), Isle's own folder icons (Papirus's stand in), names cut at their
  end rather than the middle, and the tab bar cutting tabs past its
  minimum width rather than scrolling.

- P15 15.2: the engine. `isle-fs` lists a folder on a thread of its own in
  batches of 512 (`statx` with no automount and no sync, a link's target
  read for its kind), keeps a folder in order by merging each batch and
  placing single entries, never rebuilding, and reads touched names again
  from one inotify instance shared by every view, settled for 100 ms and
  reported at least every 500 ms. The mount table from mountinfo with each
  mount's class, and probes that give up on a mount at a deadline with one
  outstanding per mount. `isle-apps` gains file types from
  shared-mime-info (`xdg-mime` pinned by revision): by name, cached by
  extension, by content on request, canonical names, descriptions and
  icons. 50,000 entries: the first batch in 0.45 ms, all merged in 138 ms
  (release). Correction to the plan: names sort by Isle's own natural key
  (case folded, digit runs by value, compared as bytes) rather than
  `icu_collator`. The stalled-mount fixture is left to the VM, an sshfs
  mount cut off; the probe's deadline is tested headless.

- P15 15.1: `isle-app` runs above its windows. One inbox on the event loop
  takes launches, menu choices, the compositor's state and settings
  writes; a delegate opens windows for requests (`run_with`), and `run`
  keeps its shape for an app of one window, so Settings is unchanged. Each
  window has its own menu bar at `/MenuBar/<n>`, Wayland extras (the lights
  optional), command table and commands; the desktop's state and theme are
  shared. Holds keep the app running with no window open; the last window
  or hold gone ends it. LauncherEntry progress for the switcher card; the
  Go menu. Sheets and alerts move from Settings into `isle-ui`, which
  gains the popover. Correction to the plan: the virtual list, grid and
  table are built with Files' views in 15.3 and 15.4, on Freya's
  `VirtualScrollView` as Spotlight's are, rather than moved first.

## 2026-10-20

- P15 planned and reviewed (`p15-files.md`): every recommendation taken.
  Evidence in `docs/research/rust-desktop/files-2026.md`: the FileChooser
  backend at version 4, FileManager1's callers, the thumbnail spec and
  helpers, listing and mounts at scale from cosmic-files' issues, and v1's
  browsing engine read again.

- Windows reopened away from where they were left when they had been tiled:
  a place was recorded only when a move or resize of a floating window
  ended. A tile, fill or restore now records the place too, the zone kept
  with the floating box (`Place.zone`), and a main window with a zone
  reopens in it from its first frame, its box behind it for Restore.
  `compositor.md` Stacking 3 says so.

- A tiled window filled from its bar went back to the box it had before it
  was tiled, so it jumped (found on the host). A fill now remembers the
  tile it came from and goes back to it; from a floating window it goes
  back to the box as before.

- Settings › Wi-Fi, from the P14a host run: the joined network was missing
  and offered again under Other networks, and a network with two saved
  connections was known twice. The joined network is now the access point
  in use, found before access points are collapsed by name (the strongest
  of a name need not be the joined one); a known network is listed once
  with every saved connection behind it, so Forget and Auto-Join act on
  them all; known networks in range carry Connect in Known networks and
  leave Other networks; Connect joins through the saved connection and the
  access point, where it had sent no access point and a new connection;
  an open network keeps the lock's place. A fake-NetworkManager test joins
  the weaker of two access points with two saved connections.

- P13 follow-up from the P14a host run (gate `p13/style.md`, answered: the
  pop-up without pictures). `windows.tile-margin` is None, Small, Medium or
  Large from `space-s`, `space-m` and `space-l`, Medium by default, chosen
  on Settings › Desktop; `tile-margins` is retired, so an old line is
  ignored. Appearance › Advanced… shows the tokens file, whether it loaded
  and how many tokens it changes (`Source::changed`), with Show Shipped
  Tokens (a read-only copy in the cache) and Open or Create in Editor,
  which open the text/plain app through `Windows1.Launch`; Settings never
  writes the file. Found: `a_panic_anywhere_in_the_probe_rebuilds_it_with_
  its_model` failed once under the shell suite and passes alone.

- P14a host run, first sign-in: a black screen. `pacman -Syu` had updated
  `nvidia-utils` to 615.78 under a loaded 615.71 module, and EGL refused
  the GPU on every start (Hyprland and v1's greeter crashed alike). The
  supervisor now compares `/proc/driver/nvidia/version` with the installed
  `libnvidia-glcore` before each start and ends the session with the
  reason when they differ, and syncs its log and crash records, which a
  reset had left empty.

## 2026-10-19

- P14a 14a.7: closed but for the host run, the user's. B100–B103;
  ARCHITECTURE's How Isle is installed, its data table and the supervisor's
  exit; `platform.md`'s Packaging and Login Items points answered and its
  P14 references moved to P14a or P14b; the roadmap's P14a built;
  `p14a-session.md` ends with its outcome.

- P14a 14a.4 and 14a.6: the package and the VM. `pkg/arch/PKGBUILD` builds
  `isle-git` (42 MB) with the compositor's Skia built from the crate's
  sources and Settings' prebuilt; `tools/package.sh` packages HEAD with an
  incremental target directory. In the guest: installed over v1's unowned
  `isle.portal`, `isle setup` applied (root half through sudo), tuigreet
  listing Hyprland and Isle, Isle started by greetd from `/usr/bin` with
  its environment, target, Login Items and portal backend, a clean quit
  leaving the manager empty, Hyprland after it on v1's portal file and
  log, and `isle setup --undo` putting all three back. Results and the
  host checklist in `docs/research/rust-desktop/p14a-session.md`.

- P14a 14a.5: `isle setup`. A table of steps, each with a check, an apply
  and an undo, and a record of what it changed (`setup` in
  `/var/lib/isle` for root's steps, in the user's state for theirs);
  `--check` lists due, done, not here and blocked with the reason; the
  root steps run in one `sudo` or `pkexec` of `isle setup --as-root`;
  `--undo` reverses only what was recorded and leaves a file changed by
  hand since. The steps: greetd's greeter as tuigreet, the user's
  `portals.conf` as `hyprland-portals.conf`, the xremap drop-in. On the
  host, `--check` finds the portals and xremap steps due and the greeter
  blocked until greetd-tuigreet is installed.

- P14a 14a.3: Login Items. `isle_apps::autostart` reads the autostart
  directories by systemd's generator's rules; the compositor starts them
  through the launch path once the user manager names its display and
  `graphical-session.target` is up, and after a restart leaves those whose
  scope still has processes or whose bus name is owned. `Test1.LoginItems`
  starts them at once for tests; isle-tty runs without them. Correction to
  the plan: the launch path puts each app in a user scope of its own, not
  in the logind session's scope, so choice 17 no longer says so.

- P14a 14a.2: the session beside v1. `data/session/isle.desktop`; the
  supervisor's log in `~/.local/state/isle/logs/` and, when its output does
  not reach the journal already, the user journal as `isle-compositor`; the
  compositor's log lines plain when not on a terminal; the lock marker named
  for the logind session; the session's variables unset at the end only
  while the manager's display is still Isle's, and then only those still
  holding Isle's values; `SSH_AUTH_SOCK` for gcr's agent, and the cursor
  from the tokens as `XCURSOR_THEME` and `XCURSOR_SIZE`, sent to the
  supervisor with the display. Found: `a_window_holding_the_keys_gets_them_
  until_cmd_escape` failed once under the whole suite and passes alone.

- P14a 14a.1: the binaries stand alone. Settings' row index is compiled
  into the compositor; `isle-auth` and `isle-record` are opened when the
  compositor starts and run through that descriptor, so a package
  installed while Isle runs cannot hand it another build's helper (each
  names its process itself, as the kernel names it by the descriptor's
  number). `isle-package` holds the tests that the package and the tree
  agree, the first that no installed binary reads its build tree. Found:
  the capture test `a_shot_from_the_toolbar_holds_none_of_it` fails four
  runs in five with or without this change.

## 2026-10-17

- P13 follow-up: the sidebar's blur in software. Freya's software driver
  (`alexparlett/freya`, branch `isle/softbuffer-alpha`) presents
  premultiplied ARGB through softbuffer's unreleased `AlphaMode`, pinned by
  revision; an Isle app with a backdrop asks for the blur and a transparent
  window in every build and declares the rest of the window opaque.
  Settings idles at 59.8 MB PSS with the sidebar glass (151.6 MB on GL).
  Bar 2 reports private memory beside PSS. Presenting only what changed
  waits on incremental rendering in Freya and the host's frame times.
  Found by the user on the glass: an app with the lights over its content
  kept square top corners, as if a bar above it rounded them; they are
  rounded now, and the glass test checks the corner.

- P13 13.13–13.15: ext-background-effect-v1 served, a still window's blur
  region drawn as untinted glass from what lies below it and dropped under
  Reduce Transparency; Settings' sidebar is glass in a GPU build. Identify,
  reordering by drag, Focus schedules edited in place, the Keyboard
  Shortcuts sheet's modifier pop-ups and terminal, and a found row
  scrolled into view. Headless bar tests (3 to 8) and an idle memory
  sample; the VM pass, which found the panes' daemon state freezing once
  the first pane to ask closed (fixed). Closed: ARCHITECTURE's How an app
  is built and How Settings writes, B94–B99 with B25 and B26 revised,
  `apps.md`'s conventions answered, the rows in the Settings structure
  doc, the roadmap. The host run is the user's.

- P13 13.3–13.12, the panes: all 21 drawn on the gated rows, each a
  component over the desktop's values and the daemons' state, every row
  bound to its key or call. Settings writes through `Settings1` (a row
  reads its new value at once and shows the compositor's refusal under
  it); the daemons are followed only once a pane asks (NetworkManager,
  BlueZ, PulseAudio, power-profiles-daemon, the compositor's displays,
  keyboards, pointers, modules, Focus, the portal's permissions and the
  app index). Sheets: Hot Corners…, Night Light…, Advanced…, Clock
  Options…, Input Sources…, Modifier Keys…, Keyboard Shortcuts… with the
  recorder, Search Privacy…, Add Focus…, Join other network…, Wi-Fi's
  Details…, the keep-or-revert alert counting down to the compositor's own
  revert. Displays arranges the screens to scale with the island's mark,
  snapping a dropped one to its neighbour; a mode, rotation or turning a
  screen on or off goes through `Displays1.Configure`, a scale applies at
  once. View › Show Changed Settings marks rows and panes whose value
  differs from its default; Reset to Default is in a changed row's context
  menu. The rows are in the panes' record, so search and Spotlight find
  them and a found row on a sheet opens it. Headless: every pane opens on
  its defaults; switches, pop-ups, Wi-Fi joining, a display change kept, a
  scale, a module's size, an app's notification rule, a hot corner and a
  reset are driven and their writes and calls read back. Not yet: dragging
  to reorder lists, editing a Focus schedule in place, Identify, the
  found row scrolled into view, the Desktops shortcuts' modifier pop-ups,
  the Open panel (typed paths until P15); nothing seen in the VM yet.

- P13 13.1, the rows: every pane's subtitle, groups, rows, sheets and
  sub-pages in `p13/rows.md`, drawn on `p13/rows-1.html` and published as a
  private board. The user took every recommendation: scale as a pop-up of
  steps with Other…, glass as a slider, a switch per Spotlight source
  (`spotlight.sources-off`), the Energy pane, every connection on Network,
  a group per mouse, virtual devices left out, a pop-up per Control Centre
  module, a sub-page per privacy kind, scale applying without
  keep-or-revert.

- P13 13.9, hot corners: a `hot-corners` section (each corner's action and
  the modifiers it needs; bottom right Apps by default) and the engine
  beside the top edge: a corner where no other screen meets fires after the
  pointer rests in it (`timing.hover-dwell-ms`), once a visit, never with a
  button held, the pointer grabbed or constrained, the menu bar open, the
  screen locked or a full-screen game in front, and only with exactly its
  modifiers held. The actions built today: Notification Centre, Apps, Lock
  Screen, Put Display to Sleep; firing the Apps corner again closes what it
  opened. With it, `windows.edge-tiling` (drag-to-edge tiling, on) and
  `control-centre.tray-hidden` (items kept out of the tray). Behind the
  Settings panes, all tested headless: `Notifications1.Apps` (every app
  that has posted, kept with the records), `Island1.Modules` and
  `EditControls`, `Displays1.SetBrightness`; saved Wi-Fi networks with
  auto-join, forget and a device's details in `isle-network`; forget,
  rename and the adapter's name in `isle-bluetooth`; balance in
  `isle-audio`; XKB's layouts and variants in `isle-keys`.

- P13 13.4, the compositor's half of a display change: `Displays1.Configure`
  takes each display's new mode, scale, place, rotation or on and off (JSON
  by make, model and serial) and applies it at once: a mode, a rotation or
  turning a screen on or off relights that connector alone, the rest is
  placed and scaled. The compositor holds what it replaced until `Keep`
  writes the change into `settings.kdl`; its own deadline
  (`timing.display-revert-ms`, 15 s), `Revert` or the asking client leaving
  the bus puts it back. The last display that is on cannot be turned off.
  Lighting reads the effective configuration (a change not yet kept, else
  the settings). `Displays1.List` now gives each connector's modes, its
  rotation, its physical size, the EDID's refresh range and whether it is
  on, listing displays turned off. Headless: a change goes back at its
  deadline with nothing written; a kept one is written and stays; one goes
  back when its caller leaves; a display turns off and back, and the last
  cannot. Not yet on hardware.

- P13 13.3, opening Settings: the compositor's `open_settings(pane, row)`
  launches Settings through its bus name when its entry is installed, else
  the `isle-settings` built beside the compositor, with
  `isle-settings://<pane>[/<row>]`. Control Centre's "… Settings…" and Set
  up… (its pane names turned to ids), the system menu's About This Computer
  (General) and System Settings…, Spotlight's settings results, the
  `settings` shortcut, `Shell1.OpenSettings` and `isle settings open [pane]
  [row]` all use it. `data/apps/` holds the desktop entry (DBusActivatable,
  the `isle-settings` scheme) and the D-Bus service file. Headless, Isle
  starts Settings at Displays and a second open moves the same window.

- P13 13.3, the frame: the panes in the sidebar's order with their glyphs,
  hues, groups and one-sentence subtitles (provisional until the rows'
  board) as the record (`panes.rs`); the index Spotlight reads written from
  it by `isle-settings --write-index`, a test failing when the committed
  file differs, and a pane found as "Settings". The sidebar has the search
  field, the account row from AccountsService (initials, or the login when
  there is no name) and the panes; Up and Down move through them. Search
  ranks with Spotlight's matcher, panes whose name matches first, keywords
  finding without showing, and says "No results for …". The toolbar has Back
  and Forward (and Cmd+[, Cmd+], the mouse's buttons) and the pane's name;
  the content has the rejected-edit notice for `settings.kdl`, and for
  `tokens.kdl` on Appearance and Accessibility, then the pane's header. A
  launch, an activation or `isle-settings://` opens at a pane; the pane last
  shown is kept in `settings-app.json` and opened next time, General the
  first. Tested headless (`isle-app`'s `testing` feature gives a tree its
  contexts), and in a headless Isle a second launch's link opens Displays.

- P13 13.2, the text field: `isle-ui::TextField` is Freya's editor (caret,
  selection, clipboard, undo) dressed in Isle's tokens; with the Mac's
  conventions declared, Cmd+Right goes to the line's end, Option+Backspace
  takes a word and Cmd+A selects all (tested headless). The form's controls
  are tested with `freya-testing` at the pinned revision.

- P13 13.2, the form: `isle-ui::form` has the pane header, the grouped card
  with hairlines, the row (label, description, a control or a navigation
  value and chevron, the found ring, the refused line), the segmented
  control, the pop-up button with its menu attached in the window, the
  stepper with its unit, the swatches and the rejected-edit notice, drawn
  as `p1/surfaces/settings.md` has them (a headless shot of a sample pane
  matches). `isle-app` declares the Mac's conventions to Freya
  (`TargetPlatform::MacOs` at the root), so its editor takes Cmd as primary,
  Option by word and Cmd with the arrows to the line's ends.

- P13 13.2, menus and the clipboard: `isle-menus` gains a real DBusMenu
  exporter (`export::MenuBar`: the tree replaced whole and announced, a
  choice back by id), tested against Isle's own reader. `isle-app` has one
  command table per app: the standard commands (About, Hide, Hide Others,
  Show All, Quit; Undo to Select All; Minimise, Zoom, Close Window) and the
  app's own through `use_commands`. The menus are built from it and served
  on the app's connection, their address given through the appmenu; a key
  press matching a command runs it, and a chosen item with a chord arrives
  as that key press, so item and key take one path; the edit commands go
  to the focused field as keys. Headless, Settings' menus show in Isle's bar
  with Quit as ⌘Q, and choosing it ends the app. The clipboard goes through
  the window's own `wl_data_device` (`smithay-clipboard`) in place of
  arboard, so apps need no data-control; it is checked once a field exists.

- P13 13.2, first part: `isle-app` and the lights over an app. The user
  revised choice 14: an Isle app keeps server-side decorations with a
  transparent bar and names the lights' inset, as a Mac app does, and the
  compositor draws the lights alone over its toolbar. No standard protocol
  says this, so Isle serves `isle_titlebar_v1` (`set_overlay`, answered with
  the group's size); such a window gets no bar height, its bar surface is
  placed and drawn at the inset and takes the pointer there first. The
  toolbar's double, middle and secondary clicks go to the compositor through
  `gtk_shell1`'s `titlebar_gesture`, so `windows.titlebar-action` lives in
  one place. `isle-app` has the worker (tokio on its own thread), one
  instance through `org.freedesktop.Application` (zbus reports a taken name
  as `NameTaken`, not `Exists`), the theme and every setting followed from
  `Shell1` and `Settings1`, activation, appmenu, the overlay and gestures
  bound on a queue of its own against winit's display, the software
  renderer unless `FREYA_RENDERER` says, and logging. `isle-settings` is a
  bare window on it so far. Headless: the window opens 900 by 760 under
  Isle's lights with nothing above them, and a second launch hands its
  `isle-settings://displays` to the first and exits. Open: Freya's
  clipboard is arboard, which wants data-control and falls back to X11,
  and Isle serves no data-control to clients.

- P13 13.0: the store and the theme over D-Bus. `Settings1` gains `Reset`
  (a key, a section or an entry back to its defaults, an optional key
  unset), `Remove` (a keyed entry deleted: a Focus mode, an app's
  notification rule) and `Values` (every key in force, defaults included).
  An empty list is written as a bare node and read back empty, so a
  shortcut can be turned off and `isle windows game` works on the empty
  default; a reset that would clash with another shortcut is refused. The
  file keeps every byte not changed: a removed node takes its line and its
  comment, or only itself and its `;` from a shared line. `isle settings
  values|reset|remove`, and `set` with no value empties a list.
  `Shell1.Theme` gives an app the user's `tokens.kdl` in force and the
  appearance with Auto decided; `ThemeChanged` fires on a token reload, a
  rejected file, an appearance change and Auto's turn at sunrise and
  sunset. Isle's own apps (`io.github.alexparlett.Isle.*`) are never
  translated whatever `keyboard.untranslated-apps` holds, rather than
  entered in its default as the plan had it, so editing the list cannot
  take Cmd from them.

## 2026-10-10

- P12 closed but for the host run and the fork's third pull request.
  ARCHITECTURE has How a frame reaches the screen and How night light is
  applied, with `isle-sun` in the repository's layout and the fork's three
  new patches in its dependencies; B86–B91 recorded; `compositor.md`'s
  night light and gaming display points settled and DRM lease taken out of
  its protocol table; `gaming.md`'s points on detection, hidden games,
  presentation, Big Picture and gamescope answered or left to the host run;
  the roadmap's P12, P13 and P22 entries follow; `p12-gaming.md` ends with
  its outcome.

- P12 12.8, the measure and the VM (`docs/research/rust-desktop/p12-gaming.md`).
  Headless at 3440 by 1440 with a full-screen game drawing every frame: the
  shell draws nothing over it, and a key reaches the game 0.09 ms at p99
  after the loop takes it (the game still; drawing, the test client's own
  buffer fill makes it 2.2 ms). Found and fixed: the loop flushed clients
  only after drawing, so a key taken in a frame's iteration waited for the
  frame (3.7 ms at p99); clients are flushed before the redraw too. The
  bench `gaming_cost`, `Test1.KeyTimed` and the test client's `stamp-keys`.
  The host run's checklist is in the note; it is the user's.

- P12 12.7: night light. `isle-sun` gives sunrise and sunset from the
  system time zone's coordinates in tzdata's `zone1970.tab` (NOAA's sunrise
  equation, within three minutes of London's and New York's tables);
  `appearance.mode` Auto now follows it. A `night-light` section (schedule
  Off, Sunset to Sunrise or Custom with `from` and `to`; warmth 2700 to 6000
  K, 3400 by default) drives every screen. The warmth is a blackbody white
  point computed from Kim et al.'s fit of the Planckian locus, normalised so
  6500 K is identity, within 0.03 of Thies's table in linear light (KWin's
  copy of that table is GPL code, so it is not ported). It reaches each
  CRTC as `GAMMA_LUT` written outside `DrmCompositor`'s commits, else the
  legacy ramp; the ramp found is kept and put back when the session leaves
  the console or ends, and given again on return and to screens lit later.
  Control Centre's tap turns it the other way until the schedule's next
  change ("On until 18:19"); Game Mode turns it off at once and gives it
  back with a second's fade, unless the user turned it on meanwhile;
  schedule edges fade over a minute. `isle night-light [status|toggle]`
  and `Displays1.NightLight`, `ToggleNightLight`. Different from the plan:
  there is no shader pass where a screen has no ramp (smithay's GL frame
  sets its own blend function, so a tint needs a whole extra pass a frame);
  such a screen is reported as `none`, which the VM's virtio screen is.
  Captures are untinted because they never pass the ramp. Four headless
  tests and ten unit tests.

- P12 12.6: Steam and gamescope. Quit on Steam (`steam` or the Flatpak's
  id) runs `steam -shutdown` through the launch path instead of closing its
  windows. Steam's toasts (`notificationtoasts_<n>_desktop`) are left
  unmanaged. Big Picture is marked once by class and title, is a game, and
  takes a full-screen desktop when it asks or covers its screen; Steam's
  minimise of it closes that desktop, returns the view and parks it out of
  the switcher and the app list until Steam shows it again; Steam's main
  window, when it reappears within ten seconds, goes back to the desktop it
  was on when Big Picture opened. Nested gamescope 3.16 runs on a headless
  Isle on the 5070 Ti with no missing global and its window a game; the
  globals test lists every global its Wayland backend binds. Three X11
  tests, a gamescope test (skipped where gamescope is missing) and two unit
  tests.

- P12 12.5: games. A window's process lineage is read on the worker at
  map: a process descending from Steam, Heroic or Lutris that is not the
  launcher's own (`steam`, `steamwebhelper`, `heroic`, `lutris`) is a game,
  and so is one whose ancestor GameMode registered; a surface whose
  wp-content-type says game is one; `steam_proton` joins the labelled
  classes. `isle windows game <app> yes|no|auto` edits `windows.games` and
  `windows.not-games`. A game is never planned at a remembered place nor
  recorded, and an X11 game opens where it asks when that is on a screen.
  Game Mode holds window motion still. Windows mapped but out of view get
  frame callbacks and fifo releases from a pacer: a game every 33 ms, any
  other window once a second, the pacer itself once a second while no game
  is hidden. The test client keeps one chain of frame callbacks, and
  `count-frames` prints them. Four headless tests and three unit tests.

- P12 12.4: tearing. `output.tearing` (off by default) and
  wp_tearing_control: a frame is rendered as async when the output allows
  it, its full-screen window has the keyboard, and that window's surface
  tree asks; it also allows any format on the primary plane. Smithay then
  flips it asynchronously only when nothing but the primary plane changed,
  and with the fork's new `FrameFlags::ASYNC_SCANOUT_ONLY`
  (alexparlett/smithay `async-scanout-only`, pinned by its commit until it
  is merged) only when that plane holds the client's buffer, so composited
  frames keep vsync. The trace marks frames that asked and did not get it,
  and an async flip refused on a scanned-out frame is logged once. In the
  VM, with the scanout check taken out for the run, virtio-gpu flipped 40
  frames asynchronously without a DRM error; with it, composited frames
  that asked stayed on vsync. Found, not P12's: a full-screen window in the
  VM is drawn every 66.7 ms where a windowed one is drawn every 13.3 ms, the
  same with 12.0's build; 12.8 measures it on the host. `gh` lost its login,
  so the fork's pull request is not open yet.

- P12 12.3: VRR. Each output's VRR is decided before every frame from its
  `vrr` setting, what its connector can do (on demand, only at a modeset,
  or not at all) and whether its full-screen desktop is in view with its
  window holding the keyboard; `use_vrr` is asked only when the answer
  changes, so a settings change applies at the next frame. A connector that
  needs a modeset (Smithay calls every HDMI one so) gets VRR only at Always,
  set before its first frame. Under VRR a frame that would move only the
  pointer is held up to the panel's slowest refresh from the EDID's range
  descriptor plus 2 Hz, 30 Hz at most, then drawn on its own if the window
  in front has not drawn first. `isle displays` shows each output's VRR
  capability, setting and state. Headless outputs act as on-demand VRR
  screens so the policy runs in tests. The VM's virtio screen reports none
  and a 50 Hz minimum. Two unit tests and a headless one.

- P12 12.2: the game protocols. linux-drm-syncobj where the device can wait
  on sync points (the DRM device; headless, the render node), wp-fifo,
  wp-commit-timing and wp-content-type served; a commit's buffer is held
  until it can be read, by its explicit acquire point or else the dmabuf's
  own fence; fifo barriers are passed once a frame holding what set them is
  drawn, of surfaces mostly on that output and of those shown nowhere;
  timed commits apply before the frame that covers them, or at their
  deadline by a timer while nothing draws, one timer per surface. Pointer
  constraints and relative pointer: a lock or confinement holds only on the
  surface under the pointer while its window has the keyboard, and lets go
  at Cmd+Tab, a click away, the pick and the lock, coming back after;
  relative motion carries libinput's unaccelerated delta; the top edge's
  dwells reveal nothing while a constraint holds. The test client gained
  `fifo`, `commit-at`, `content-type`, `lock-pointer`, `confine-pointer`,
  `relative` and `globals`. Found and fixed on the way: barriers passed at
  a vblank let a commit replace a frame never shown, and passed before the
  frame's state was recorded lost the redraw the next commit asked for.
  Found: `target/debug/isle-auth` built without test-hooks ignored the lock
  tests' own PAM service and fell back to `login`, so the tests' wrong
  passwords counted against the real account's faillock and locked it for
  ten minutes; `isle-auth` now refuses when `ISLE_PAM_CONFDIR` is set in a
  build that would ignore it. The VM serves syncobj and its Xwayland binds
  it. Eleven headless tests.

- P12 12.1: presentation and the trace. A vblank's time and sequence are
  the display's own from the DRM event (Isle's clock only when it gives
  none, or zero, or a time ahead of now), feedback carries the hardware
  clock and completion flags, vsync except on an async flip, and
  `Refresh::Variable` while an output's VRR is on; layer surfaces and X11
  override-redirect windows get feedback too. Each output keeps a plane
  trace: per frame, whether the primary plane held Isle's drawing or a
  window's buffer (by id and app), where the pointer was, overlays, async,
  VRR and the interval, the last 1024 frames and counters since a reset,
  read with `isle displays trace <output> [--follow] [--reset]`
  (`Displays1.Trace`, `ResetTrace`). Each output's surfaces get dmabuf
  feedback with a scanout tranche of the primary plane's formats Isle can
  also render, linear only from another device. An output with no mode set
  lights at its preferred size's fastest progressive rate. The VM's
  virtio display lit at 75 Hz and traced hardware times, 13.33 ms apart,
  with flags 7; it offers no dmabufs, so scanout is the host's to see.
  Three headless tests and three unit tests.

- P12 12.0 done: the user merged alexparlett/smithay#4 and #5, Isle pins
  the merge (`049c787`) and passes vsync to `render_frame`; the workspace's
  581 tests pass.

- P12 12.0, the fork, waiting for the user's merges. alexparlett/smithay#4
  is Smithay#1948 (async page flips) rebased onto `isle`; its one conflict
  was `handle_flip`, which upstream since made take the prepared frame by
  reference, and the presentation mode now rides on the callers' pending
  frame. alexparlett/smithay#5 is wp_tearing_control_v1, written on
  `content_type`'s current shape rather than taking Smithay#1325, which
  held its surface strongly and would touch a dead one. Smithay's lib tests
  pass and anvil builds; Isle, checked against both, needs only the
  `PresentationMode` argument `render_frame` now takes. anvil would not
  start in the VM (no DRM master at start, no render node for virtio), so
  the clean refusal of an async flip on a driver without it is checked with
  Isle in 12.4 instead.

- P12 planned (`p12-gaming.md`), 20 choices and phases 12.0–12.9, with
  the research note `gaming-2026.md`. Reviewed: every recommendation taken.

## 2026-10-09

- P11's leftovers. alexparlett/smithay#3 merged and pinned; xdg-foreign v2
  served, and the share picker hangs as a sheet on the window ScreenCast's
  `parent_window` names (a wayland handle or an X11 id), over it dimmed,
  holding only its keyboard, centred when no window is found or it goes.
  The picker's Portion of screen tab opens the toolbar in a
  Close-and-Choose form and comes back with the portion's live tile; typing
  on the Windows tab narrows its tiles; Tab is taken by the compositor, so
  Freya's own Tab no longer walks focus onto Cancel. ScreenCast's metadata
  cursor: each buffer carries the pointer's place and, when its image
  changes, a bitmap, and a pointer moving over a still frame sends a buffer
  with no picture. ext-image-copy-capture copies into dmabufs on the
  renderer's device by a GPU blit, serves pointer cursor sessions, and a
  streaming session is in the Sharing row by its client's app with Stop.
  The headless backend opens GBM on its render node, so dmabufs are tested.
  Show mouse clicks marks each press in the recording only. Open panels lift
  in the pick's window mode. ScreenCast, Screenshot and PickColor asks made
  while locked wait for the unlock. An app's interactive shot waits out the
  timer; a click on the countdown stops it. Round handles and the rounded
  line round a lifted window are the shadow program at no blur; the
  colour's swatch sits beside its value; a camera stands in for the pointer
  in window mode. Found and fixed: Spotlight took Shift+Cmd+3 and
  Shift+Cmd+4 as its quick actions, and the toolbar's state was kept only
  after a drag. The VM shared a screen from a GTK 4 window's sheet through
  the real frontend with the metadata cursor; no host run.

- P11 closed. ARCHITECTURE has How a capture is rendered, How streams are
  produced and How the portal backend answers, with `isle-record` in the
  process layout and `isle-portal` and `isle-record` in the repository's;
  B80–B85 recorded; `platform.md`'s non-interactive screenshot point
  answered; the roadmap's P11, P13, P14, P15 and P22 entries follow;
  `p11-capture.md` ends with its outcome.

- P11 11.14: the measure and the VM (`docs/research/rust-desktop/p11-capture.md`).
  At 3440×1440 a shot is on disk in about 55 ms, the freeze takes a few
  milliseconds with eight windows, a screen stream costs 1.7 ms of the loop a
  frame at p50 (3.1 at p95) without lowering the output's frame rate, and a
  still screen streams nothing. Found and fixed: the allocator kept every
  shot's pixels, 135 MB after five; `malloc_trim` after a still and when the
  pick closes keeps the compositor's own heap at about 30 MB. Freezing eight
  windows holds 239 MiB of VRAM while the pick is open. The VM: the CLI shot,
  the pick by keys, a recording, grim, and a ScreenCast through the guest's
  real frontend and Isle's picker to a consumer. Found: a user's
  `~/.config/xdg-desktop-portal/portals.conf` (v1's) outranks the packaged
  `isle-portals.conf`, which P14 must deal with. Not checked: Chromium and
  Firefox sharing (Chromium offered only tabs), OBS, Discord, dmabufs on the
  hybrid host. The test client gained `animate`, a window drawn every frame.

- P11 11.13: the CLI and the rest. `Capture1` (`Shoot`, `Record`, `Status`,
  `StopStream`) does what the chords and Stop do and never returns pixels;
  `isle capture screen|pick|toolbar|colour [--copy]`, `isle capture record
  start|stop` (the main screen), `isle capture streams [--stop <id>]`, `isle
  portal status` and `isle portal permissions [--revoke <app>]`, which reads
  and edits the permission store's `screenshot` and `screencast` tables.
  Control Centre's Screenshot control, not yet since P8, opens the toolbar,
  which closes the panel as a centre surface. Test1 has the pick, the
  thumbnail and its menu, the toolbar, streams, recordings, shares and
  capture sessions.

- P11 11.12: the capture protocols. ext-foreign-toplevel-list names every
  window (title and app id kept current from the window list's changes);
  ext-image-capture-source makes a source of an output or a foreign toplevel;
  ext-image-copy-capture copies it into the client's shared-memory buffer
  (`Xrgb8888` or `Argb8888`) through the same render as a shot, so a locked
  screen gives the lock and a window is refused while locked or not shown. A
  session's first frame is copied at once and later ones after the source's
  next frame, so a client asking in a loop gets one per redraw; a session
  past its first frame shows the screen privacy mark. grim takes the screen
  and a window by its identifier. Differences: no dmabuf buffers or cursor
  sessions yet, and a capture-protocol session shows the mark but no Sharing
  row with Stop. wlr-screencopy is not served.

- P11 11.11: Settings and Inhibit. The Settings portal serves
  `org.freedesktop.appearance` from the theme in force: `color-scheme` from
  the resolved mode, `accent-color` as sRGB, `contrast` from Increase
  Contrast, `reduced-motion` from Reduce Motion; a settings or tokens change
  that moves one emits `SettingChanged` for it, and the frontend merges gtk's
  other namespaces after Isle's (`Settings=isle;gtk`). Inhibit's idle and
  suspend flags hold idle under the app's name until the frontend closes the
  request; logout and user switch stop the session's end at once, naming the
  app and its reason as a block inhibitor is named. Monitors hear
  `screensaver-active` with the lock and `session-state` going to query-end
  when the session's end starts and to ending when it commits.

- P11 11.10: Screenshot and PickColor. An interactive Screenshot opens the
  toolbar on the shot modes with the app's icon and "{app} wants a
  screenshot", its action Share; the shot goes to the app, not to
  Screenshots, with no thumbnail, and Close or Escape answers cancel. A
  non-interactive one, already allowed by the frontend through Isle's Access,
  takes the front window for the window targets and every screen as laid out
  otherwise. Shots for apps are written under `$XDG_CACHE_HOME/isle/portal/`
  and those over a day old are removed at the next. PickColor opens the
  colour pick and answers the colour under the loupe as sRGB in [0, 1].
  Not yet: the countdown for an app's shot (it shoots at once).

- P11 11.9: ScreenCast and the picker. CreateSession exports a Session;
  SelectSources keeps the types, cursor mode and persistence asked for (a
  cursor mode not offered is refused; hidden and embedded are); Start finds
  the sources in the restore data `("Isle", 1, [specs])` (a screen by its
  identity, a window by its app and title), or from the same app's second ask
  within ten seconds of a share it was granted, or asks in the picker; then
  streams each source and answers with each node, its `pipewire-serial`,
  position, size, source type and mapping id, and the restore data when the
  user let it be kept. The picker is a shell surface with the asking app's
  icon and name, Screens and Windows tabs as the request allows, a tile per
  screen or shown window with its live preview (every half second, while it
  is open, and never of the picker itself), the remember check, Cancel and
  Share; Tab changes tab, the arrows move, Space chooses, Return shares,
  Escape cancels, and a second click on a tile shares it alone. The frontend
  closing a session ends its streams; a stream whose source goes ends its
  session with Closed and an ended event. While any share runs, the island's
  Sharing activity lists each app and its sources, each with Stop.
  Differences: the picker is centred on the main screen, not yet a sheet on
  the asking window: that needs the fork's xdg-foreign lookup, opened as
  alexparlett/smithay#3 and waiting for the user's merge. No Portion of
  screen tab, no typing filter, and a request made while locked is answered
  cancel rather than held until unlock.

- P11 11.8: the backend. `isle-portal` holds the impl interfaces as
  hand-written zbus (Access, ScreenCast with its Session objects, Screenshot,
  Settings, Inhibit), the Request object each held reply exports while it
  waits, and the check that every call's sender owns
  `org.freedesktop.portal.Desktop`; `version` is exported lower-case. The
  compositor serves it on a connection of its own, takes
  `org.freedesktop.impl.portal.desktop.isle` before it reports ready, and
  asks systemd to try-restart `xdg-desktop-portal.service` so a frontend
  already running reads Isle's versions. Access is a P9 prompt with the app's
  icon, Allow and Don't Allow; the frontend closing the request takes it
  away. `data/portal/` has `isle.portal` and `isle-portals.conf`. The other
  interfaces answer nothing until 11.9 to 11.11.

- P11 11.7: recording. The toolbar's three recording modes (shown while
  `isle-record` is installed) and its countdown start a recording; one at a
  time. The compositor streams the source (the pointer embedded when
  `capture.show-pointer` is on) and starts `isle-record` with the node, which
  links itself port to port, reads the frames as shared memory, and encodes
  H.264 High through NVENC when it opens and x264 when not, and, when asked,
  the default output's monitor and the chosen microphone mixed as AAC at
  48 kHz, into `$XDG_VIDEOS_DIR/Recordings/Screen Recording … .mp4`. The
  recorder is built against FFmpeg 9 without libavdevice and with
  PipeWire's realtime module off: it links neither libdbus nor libpulse.
  While it runs the island shows the Recording activity with its elapsed
  time and Stop; Stop is that row's button, Cmd+Ctrl+Escape (after a
  countdown) or the toolbar reopened on the running mode. A recorded window
  that closes or a screen that goes stops it and keeps the file, with
  "Recording stopped: the window closed" in the island. The thumbnail
  follows with the source's first frame. Differences: the file stays a
  fragmented MP4, each frame a fragment flushed as it goes, rather than being
  rewritten with its index first: its index already leads the file, a
  recorder killed midway leaves a file that plays, and the rewrite failed on
  AAC's packets through the API. The audio streams set
  `adapter.auto-port-config` so they have ports with no session manager. Not
  yet: Show mouse clicks; the dmabuf path into the encoder.

- P11 11.6: streams. `cast.rs` produces PipeWire video nodes in the
  compositor: the PipeWire loop is a calloop source, the context has
  module-rt and D-Bus support off (checked: neither is mapped), and each
  stream drives itself. It offers BGRx and BGRA as dmabufs with each modifier
  the renderer draws to, LINEAR first, fixing one by a GBM test allocation
  when the consumer leaves the choice open, and shared memory (a sealed
  memfd) last; the header is written on every buffer, frames with no damage
  are skipped, frames are spaced by the negotiated rate with a timer for the
  last change, and a dmabuf is queued once its render fence signals. Sources:
  a screen (what a shot of it holds, so a locked screen streams the lock), a
  portion of one, a window alone (which queues nothing while hidden,
  minimised, on another desktop or behind the lock, the consumer holding its
  last frame). A window closing or a screen going ends its streams. Any
  stream shows the screen privacy mark and holds banners under
  `notifications.hold-sharing`. Node serials come from the registry. Tests
  stream on the private PipeWire to `isle-test-consumer`, which links itself
  port to port. Differences: the cursor is hidden or embedded, not offered as
  metadata (a bitmap of the pointer's image is not drawn yet); the dmabuf
  path is the udev backend's, so only shared memory is checked headless; a
  source that changes size holds its stream rather than renegotiating.
  Under the full suite's load the lock face's typing test and the keyboard's
  Cmd test each failed once and pass alone.

- P11 11.5: the colour pick. Shift+Cmd+6, or Pick colour on the toolbar,
  freezes the screens and reads their pixels back once; a loupe stands in for
  the pointer, fifteen pixels square at eight points each, unsmoothed, with
  its grid, the centre ringed and a rim, drawn on the CPU into a buffer kept
  while the pointer stays on its pixel; the value shows in the readout's
  capsule under it. The arrows move it a pixel (Shift, ten); a click or
  Return copies the value in `capture.colour-format` (hex, rgb or hsl) and
  the island says "Colour copied"; Escape leaves. Not yet: the swatch beside
  the value, and the capsule centred under the loupe rather than at its
  trailing edge.

- P11 11.4: the toolbar and the countdown. Shift+Cmd+5 (Ctrl for the
  clipboard) opens the bar at the bottom centre of the screen with the
  pointer, in the centre group both ways, on the last shot mode: Close, the
  six modes (the three recordings shown and disabled until 11.7), Pick
  colour, Options and the action. The compositor reads its keys (Tab between
  groups, Left and Right through the modes, Up and Down and the arrows off
  the modes moving the portion, Option sizing it, Return acting, Escape
  closing) and draws each mode over the live screen under the bar: the
  portion with its line, handles, veil and readout, dragged by its body, an
  edge or a corner; the window under the pointer lifted whole, a click
  taking it and never reaching it; on two screens, the screen under the
  pointer marked. A shot from it holds none of the tool. Options writes the
  `capture` section through the store. The bar's place once dragged, the
  mode and the portion are kept in the new `capture` state domain. A timer
  runs a countdown in the island, a new OSD-class event that holds until
  withdrawn, which Cmd+Ctrl+Escape cancels; the shot waits for the island to
  return to rest. Not yet: a click on the countdown cancelling it, round
  handles, Pick colour (11.5).

- P11 11.3: the thumbnail. After a shot to a file (not a copy, not over a
  full-screen game, not with `capture.floating-thumbnail` off) a top-layer
  surface shows the shot, made small on the worker, in the bottom-trailing
  corner of the work area of the screen shot. It goes after
  `timing-thumbnail-ms`, held while the pointer is over it or its menu is
  open; a new shot replaces it. A click opens the file through the launch
  path (Quick Look and Markup take the click in P15); a press that heads for
  the trailing or bottom edge is a flick that dismisses it; one that heads
  elsewhere drags the file out as Spotlight's rows are dragged, and a drop
  into an app keeps the file. Its menu (Save to Screenshots, Documents or
  Clipboard, Copy, Move to Trash, Close) is drawn in the window menu's
  surface, which now opens for owners other than windows; Move to Trash
  follows the freedesktop.org trash layout. Not yet: the thumbnail following
  the pointer during a drag or flick (it goes at once), and Show in Files
  and Markup in its menu (P15).

- P11 11.2: the freeze and the pick. Shift+Cmd+4 renders every screen and
  every shown window (with and without its shadow) into textures of their own
  and draws the frozen frames in place of the scene; clients keep running,
  unaware. The veil, the cut, the two-tone line and the lifted window are
  compositor elements; the size readout is a placed shell surface, flipped
  inside then above the selection at a screen's foot. Region mode by a drag
  (Shift locks an axis, Option grows from the centre, Space moves it) or by
  the keys alone (Return, arrows by one or ten, Return); Space before a drag
  switches to window mode, where the window under the pointer or stepped to
  with Tab lifts whole and a click or Return takes it, Option without its
  shadow. The focused window keeps the keyboard, the pick's clicks never
  reach a client, and the lock or a screen going ends it. Not yet: shell
  panels as pickable targets of their own (they are in a region's frame),
  the camera pointer (the arrow stands in), the line along a window's
  rounded corners. No pointer lock to release: Isle serves no pointer
  constraints yet.

- P11 11.1: Shift+Cmd+3. Every screen is shot, the main one first and the
  second ending " (2)"; with Ctrl the main screen's shot goes to the
  clipboard and the island says "Screenshot copied". The theme's
  `screen-capture` sound plays behind `capture.sound`. Over a full-screen
  game the island says "Screenshot saved" in the OSD's place and the chord
  never reaches the game. The VM's look at the chords waits for 11.14.

- P11 11.0: the render and the file. `capture.rs` renders a screen, a region
  of one, or a window alone (its surfaces, popups and title bar, with its
  shadow on a transparent ground when asked) into a texture of its own, the
  pointer left out unless asked and the capture tool's surfaces left out by
  name; a locked screen gives the lock, and a window is refused while locked.
  PNGs are encoded at `png`'s Fast setting on the worker and written at once
  into `$XDG_PICTURES_DIR/Screenshots` by macOS's names, a taken name moving
  on to " (2)"; a write that fails copies the shot and says so in a banner.
  The `capture` settings section. `Test1.Capture` takes a still by a JSON
  spec; `Test1.Screenshot` shares the offscreen path but still shows what
  the output shows, dim included.

- P11 planned (`p11-capture.md`), 31 choices and phases 11.0–11.15, with
  the research note `capture-2026.md`. Reviewed: every recommendation taken.

- P10 closed. ARCHITECTURE has How Spotlight searches and How the clipboard
  is kept, with `isle-search` in the repository's layout; B73–B79 recorded;
  `shell.md`'s file-search point, `platform.md`'s clipboard points and
  `gaming.md`'s library and gamescope points answered, Spotlight 8 and
  Clipboard 3 and 7 now say pasted; the roadmap's P10, P13, P15, P16 and P20
  entries follow; `p10-spotlight.md` ends with its outcome.

- P10 10.13: the measure and the VM (`docs/research/rust-desktop/p10-spotlight.md`).
  At 3440×1440 the fast sources answer a keystroke in 0.3 ms at p95; the
  list, the Apps view and the emoji grid draw under 2 ms at p95; the first
  open takes up to 63 ms and later ones a frame; nothing draws while closed;
  the shell stays inside its budgets (about 190 MB with the emoji grid open,
  once 255). Found and fixed: since 10.1 each test compositor scanned every
  font for want of fontconfig's cache, about 53 MB and a burst of CPU each,
  which the P9 bench showed as 216 MB at rest against 147; the tests keep the
  user's cache folder and the suite's CPU time fell from 230 s to 134 s. The
  VM: kitty launched and raised, the Apps view with Papirus's icons, an
  emoji pasted into kitty, a `wl-copy` copy in the Clipboard scope, a file
  written over ssh found by name, `isle search` over ssh. xterm started from
  Spotlight but takes no pasted emoji: it has no clipboard paste chord by
  default.

- P10 10.12: the CLI and the one-app scope. `isle spotlight open|apps|emoji|close`
  and `isle search <query> [--scope …]` (`--json` for the rows), which runs
  every source on the worker without opening Spotlight, through `Shell1`'s
  `Spotlight` and `Search`. Tab on an app lists its windows, its
  desktop-entry actions and the files `recently-used.xbel` says it opened,
  matched by the app's name or command; Backspace brings the query back.

- P10 10.11: dragging out. A press on a file or clipboard row that leaves
  Spotlight's bounds starts a drag whose source is Isle (Smithay's
  `DnDGrab` with a source of Isle's own), offering a file as its link and
  path and a clipboard item as its types; Spotlight closes so the target
  shows, and the target reads what it asks for. The test client prints what
  is dropped on it. No drag icon yet.

- P10 10.10: clipboard history. Every clipboard copy, from a Wayland app or
  through Xwayland from an X11 one, is read off the loop (its text, an
  image, a file list) once Smithay has made it current, and kept under
  `~/.local/state/isle/clipboard/` (folder 0700, files 0600): 200 unpinned
  items or 30 days, 8 MB an item at most, pins never evicted, the same text
  again moving up rather than adding. Not kept: an offer carrying
  `x-kde-passwordManagerHint`, a copy while an app on
  `spotlight.clipboard-exclude` is in front. Isle's own copies (an answer, a
  path) are kept too. The Clipboard scope lists Pinned then Recent; Return
  pastes into the window that had focus, Shift+Return copies, Cmd+P pins,
  Cmd+Delete deletes, Cmd+Y shows the whole text or image over the list,
  and the row's menu has Paste, Copy, Pin, Delete and Clear clipboard
  history… with its confirmation.

- P10 10.9: emoji. `data/emoji/generate.py` writes `emoji.tsv` from
  `emoji-test.txt` (Emoji 17: 1,920 emoji with skin tones as variants) and
  UnicodeData (1,295 symbols in v1's blocks), with CLDR's keywords when an
  annotations file is given, which the host has none of. A tab-separated
  file, not KDL as planned: parsing 273 KB of KDL cost about 600 ms of CPU
  in debug builds, which starved the test compositors' loops. The grid
  leads with recent picks; the colour font's coverage is checked on the
  loop the first time the Emoji scope searches, through the font manager
  the shell holds (a second one on the worker held the loop's text up
  behind fontconfig's lock). Every pick is pasted: Isle reads the clipboard
  as it was, offers the pick, sends the app's paste chord and puts the
  clipboard back once the pick was read or after a second. Found on the
  way: Isle never told Xwayland when the Wayland side set the clipboard,
  so an X11 app could not paste what a Wayland app copied, and Xwayland
  could not read a selection Isle offers. The test clients paste and copy.
  The full suite's timing tests fail one at a time under load and pass
  alone.

- P10 10.8: the calculator, Run and the web. `fend-core` evaluates on the
  worker, given up after 100 ms or at the next key, offline; the general
  search places its row when the query reads as a sum. Return copies the
  answer and the island says "Copied". `>` runs a command line in a scope
  of its own, Cmd+Return in the terminal, and the last fifty are kept in
  the `commands` state domain; launches now start in the home folder, as
  from a terminal, rather than in Isle's own. The web row searches
  `spotlight.web-search`, DuckDuckGo unless set.

- P10 10.7: files and contents. `isle-search`'s file index walks home and
  `spotlight.files-include` with the `ignore` crate, leaving out hidden
  folders, what `.gitignore` files exclude and `spotlight.files-exclude`;
  the compositor's watcher holds one inotify watch per folder, reads a
  changed folder again (walking new folders, forgetting gone ones) and
  builds afresh on an overflow or a settings change. Names match against a
  lower-cased copy first. On the host: 83,960 entries in 7,022 folders,
  built in 165 ms, about 20 MB, a name search in about 2 ms. The Files
  scope lists recent files from `recently-used.xbel` when empty; Contents
  waits 250 ms, searches plain text with ripgrep's engine, stops at twenty
  files and gives up when the next key comes. A file opens in its default
  app; Cmd+Return shows it through FileManager1.

- P10 10.6: the Apps view. Favourites first in the user's order, then
  every app and game by name; the fixed bar with the section's title and
  the category filter, a menu of the categories that have apps; typing
  drops the sections. The tile's and a row's menu inside Spotlight's tree
  (Open, the entry's actions, favourites, Keep running after last window,
  Show in {launcher}, Copy, Quit while running), which takes the keys while
  open. Favourites are written to `spotlight.favourites` from the menu,
  Option+Cmd+arrows and drags between tiles, and show at once. Badges on
  tiles. Found on the way: the compositor re-sorted each group by rank,
  which undid a scope's own order.

- P10 10.5: the fast sources in `isle-search/src/sources/`: apps and library
  games (generic names and keywords as other words, launchers' entries
  folded into their games, unlisted entries out), shell actions from
  `data/spotlight/actions.kdl` with their chords in the menu bar's form and
  Sleep, Restart, Shut Down, Dark Mode and each Focus as commands, apps'
  desktop-entry actions, windows with their app and desktop, the front
  app's enabled menu items from the same titles Help's search reads, and
  Settings rows from `data/settings/index.kdl`, which ships empty. Choosing
  raises a running app or starts it, runs an action, raises a window or
  activates a menu item once focus is back. The `spotlight` settings
  section (favourites, file folders, the clipboard's exclusions, the web
  search). The menus test `gtk_s_menus_arrive…` failed once under the full
  suite's load and passes alone, with and without this change.

- P10 10.4: the surface and the field. Spotlight opens on the main screen
  from Cmd+Space, Ctrl+Cmd+Space and the Launchpad key; the compositor owns
  the model and takes Spotlight's keys before the keyboard engine while it
  holds the keyboard (text from the seat's layout, its own chords, repeat),
  so keys typed with the chord reach the field. A prompt takes the keys and
  gives them back; the lock, the switcher, Force Quit, the session dialog
  and a click outside close it, and it closes them. The field (token,
  caret, selected query, quick keys, activity mark), the results list and
  the Apps view's grid on virtual scroll views, sized to the screen. The
  clipboard core: Isle's own selections, reading a client's offer off the
  loop for Cmd+V in the field, and a fix found on the way: a Wayland
  client reading an X11 app's clipboard was never handed to Xwayland.

- P10 10.3: Spotlight's model in `isle-shell/src/spotlight/model.rs`: the
  field as a text field (caret, words, select all, paste of a line), scopes
  from prefixes, quick keys and Tab on an app with Backspace back out, the
  general search's fixed groups under a top hit from the fast groups, the
  calculation row placed before its answer, slow answers that wait rather
  than land above a moved selection, Return before an answer acting on it
  once it comes, the panel held until the query is cleared, the last general
  query kept and a scoped one never, the grid's arrows across sections. The
  compositor will own the model and feed it keys from the chord's press, so
  no key needs buffering.

- P10 10.2: games from every launcher. `isle-games` reads Steam, Heroic
  (Epic, GOG and Amazon store caches with legendary's and gogdl's installed
  records, and sideloaded apps; covers from its image cache, named by the
  URL's SHA-256) and Lutris (`pga.db` read only through `rusqlite`), from
  every native and Flatpak root found. A cover the user set in Steam wins.
  Steam's runtimes are not games. Where Steam is installed it alone decides
  its games: on the host Lutris marked six Steam imports installed that
  Steam has not. A game starts through its launcher's entry with its link
  (`steam://rungameid/`, `heroic://launch/<runner>/<app>` as v1 used,
  `lutris:rungameid/`), a launcher's desktop entry folds into its game by
  that link, and gamescope started by Steam becomes the game it hosts. On
  the host: 18 games read in 132 ms; Steam's entries for ERA ONE and Path of
  Exile 2 fold.

- P10 10.1: the app index. Entries gain GenericName, Keywords, Categories,
  OnlyShowIn and NotShowIn (against `XDG_CURRENT_DESKTOP`), DBusActivatable,
  SingleMainWindow, TryExec, their translations for the session's language
  and the spec's escapes; Flatpak's exports join the data path; categories
  map to the Apps view's ten. AppImages in `~/Applications` become entries
  as v1 wrote them, read with `unsquashfs` at the ELF's end and never run.
  The launch path asks a DBusActivatable app over `org.freedesktop.Application`
  with the token, falling back to Exec, and runs a command line in a scope.
  On the host: 115 entries, 68 listed. Tests now get a HOME of their own,
  which found that Isle's Pulse client sent an empty cookie when
  `~/.config/pulse/cookie` is missing; it sends 256 zero bytes, which
  pipewire-pulse takes.

- P10 10.0: `isle-search` with the `Source` trait, rows, scopes and items;
  the tiered matcher (prefix, word start, initials, inside, then nucleo's
  fuzzy score at 18 a character or better, which keeps "kty" for kitty and
  drops letters scattered inside words); ranking by tier, use, action, score
  and name; the `usage` state domain halving every 30 days with a week's
  bonus. Help's menu search is on the same matcher, so "far" finds Find and
  Replace.

- P10 planned (`p10-spotlight.md`), 28 choices and phases 10.0–10.14, with
  the research note `spotlight-2026.md`. Reviewed: every recommendation
  taken but choice 16, now every pick pasted and nothing injected.

- P9 closed. ARCHITECTURE has How the lock holds, How idle decides, How
  prompts are asked and How the session ends, with `isle-auth` in the
  process layout; B66–B72 are recorded and B12 revised; the intent docs'
  open points answered; the roadmap's P9, P11, P14, P16, P18 and P20
  entries follow; `p9-lock-and-prompts.md` ends with its outcome.

- P9 9.13: the measure and the VM
  (`docs/research/rust-desktop/p9-lock-and-prompts.md`). At 3440×1440 a lock
  reaches every screen about 20 ms after the request (45 ms the first time,
  making the softened wallpaper); in quiet runs the face draws at 0.6–0.8 ms
  p95 and a prompt at 0.41 ms, with single slow frames of up to 3.5 ms in
  three earlier runs that did not come back; nothing draws at rest; RSS
  150–164 MB, VRAM at most 108 MiB. The VM: lock and unlock through logind, a
  wrong password through the real PAM stack, pkexec, the keyring's new
  keyring, an OpenVPN password through the plugin's auth dialog, the power
  key's chooser and a real restart, and the idle steps. Found and fixed:
  pkexec in a terminal named no app (the asker is now the nearest ancestor
  with a window), and the CLI panicked writing into a closed pipe.

## 2026-10-08

- P9 9.12: Force Quit on Option+Cmd+Escape and the system menu, every running
  app with background ones, a frozen app marked and selected, the sheet, the
  kill; close on a frozen window hangs the not-responding alert on it.

- P9 9.11: ending the session. One engine quits every app as Cmd+Q does,
  then logs out or asks logind to restart or shut down; an app that keeps
  its window stops it (Show {app}), one that misses its ping offers Force
  Quit {app}, another program's block inhibitor is named. The chooser from
  the power key and Ctrl+Cmd+Backspace with its letters and countdown, the
  confirmation from the system menu and Shift+Cmd+Q, log out with no
  confirmation, and the lock's power row confirming in place. The test
  client now runs every command line that arrives together.

- P9 9.6–9.10: the prompt frame, its queue and the compositor's scrim, the
  armed moment, Cmd+Tab setting a prompt aside into the island; the polkit
  agent, the Gcr system prompter for the keyring (`sx-aes-1`), NetworkManager's
  secret agent with VPN auth dialogs and keyring saves, BlueZ's pairing
  agent as the default.

- P9 9.2–9.5: the lock's face and the minimal lock at the second panic;
  logind's sleep delay released once every screen shows the lock, the
  power-key inhibitor, Lock and Unlock, the hints; one idle policy with the
  dim, the lock and display power, its holds (idle-inhibit, ScreenSaver,
  logind's idle block, a game); ScreenSaver served.

- P9 9.0–9.1: the locked core (the marker first, the lock's own render
  path, input held) and authentication through the `isle-auth` PAM helper.

- P9 planned in `p9-lock-and-prompts.md`, with research in
  `docs/research/rust-desktop/lock-and-prompts-2026.md`: the lock as a state
  of the core drawn by its own path, the runtime marker, PAM in an
  `isle-auth` helper process (replacing ARCHITECTURE's PAM thread), Isle's
  auth-and-account PAM service, the password held by the compositor, the
  minimal lock at the second panic, sleep and power-key inhibitors, the
  softened wallpaper made on the worker, one idle policy with its holds,
  ScreenSaver, the dim and display off, one prompt queue over a compositor
  scrim, the polkit, keyring, NetworkManager (VPN external UI included)
  and Bluetooth agents, one session-ending engine, Force Quit. Reviewed
  choice by choice: every recommendation taken but fingerprint, left out
  until there is a reader, and logind's `Unlock`, honoured. One packet,
  phases 9.0–9.14; the host run optional.

## 2026-10-07

- P8 closed. ARCHITECTURE has How services reach the shell, How
  notifications are decided and shown and How Focus decides; B60–B65 are
  recorded and B11 revised; the intent docs' open points answered; the
  roadmap's P8, P9, P12 and P13 entries follow; `p8-services.md` ends with
  its outcome.

- P8 8.14: the measure and the VM (`docs/research/rust-desktop/p8-services.md`).
  At 3440×1440 with every service live, Control Centre draws at 1.35 ms p95
  over every page and banners at 0.95 ms; a banner reaches the stack 2.6 ms
  after Notify; RSS 156–165 MB and VRAM at most 96 MiB; at rest nothing
  draws or samples and Xwayland is Isle's only child. The VM found four
  bugs, fixed: D-Bus-activated daemons (UPower, UDisks2, power profiles)
  were never started; a Pulse handshake could wait forever on a
  socket-activated pipewire-pulse; the pointer neither held a banner nor
  reached its buttons; and the banner's buttons were laid out each at the
  left of its own column (the user saw it), now equal across the text
  column. A tray page's head shows the item's icon.

- P8 8.13: Control Centre completed. The `control-centre` default is the
  design's arrangement again (Dark Mode and Keyboard layout move to the
  gallery). A catalogue holds every module with its sizes, group, Settings
  pane and why it may be missing; Accessibility's module and page switch
  Increase contrast, Reduce motion and Reduce transparency. Edit mode from
  the footer or E: remove badges, a size menu by secondary click or Return,
  moves by drag or Cmd+arrows, Delete, and the gallery by group with Add or,
  for what cannot show here, why and "Set up…"; every change writes the
  arrangement, Escape closes the menu then edit mode. The keyboard: one tile
  of the grid and one tray item take it at a time, so Tab passes each as one
  stop; the arrows move by place in the grid, Home and End, Return opens a
  page and Space switches, + and − step a slider, focus comes back to the
  module whose page was left. Now Playing at Wide is one row. A grid that
  fits shows no scrollbar. Not built: the gallery sits below the grid rather
  than beside the panel, has no search, a drag shows no live outline, and a
  module's own secondary-click menu outside edit mode.

- P8 8.12: the tray. `isle-tray` owns `org.kde.StatusNotifierWatcher`
  before the socket is announced and is the host: items by path on their
  sender or by well-known name, gone when their owner leaves, read again on
  their signals. Icons by name through the item's theme folder then
  Papirus's status and panel icons as well as apps', else their pixmaps
  written to PNG in the cache. Control Centre shows them in a card under
  the grid, a dot for attention; a click activates or opens the menu as the
  item asks, a secondary click opens the menu, middle is SecondaryActivate,
  the wheel scrolls; menus are `isle-menus`' DBusMenu client as pages, a
  submenu filled on asking when opened. Cmd+drag writes
  `control-centre.tray`. A dropped watcher owns its name again within a
  quarter second and items register again. Passive items are not shown.

- P8 8.11: disks and the desktop's notifications. `isle-disks` reads
  UDisks2's filesystems on removable drives and the user's loop images, and
  mounts, unmounts and ejects; nothing mounts by itself. The desktop posts
  through the same decision as any app: "{volume} connected" from Files with
  Open (mounting first), Mount and Eject; a rejected `settings.kdl` once per
  broken place, with Open in editor; and Isle's last crash record at the
  next start, with Report….

- P8 8.10: Focus. `isle-focus` is a pure model of `structure/focus.md`'s
  order: Game Mode while a detected game is in front unless turned off for
  it, then a mode turned on by hand until off or its end, then the latest
  automatic start (schedules across midnight, an app in front); a scheduled
  mode turned off stays off until its next start. Do Not Disturb and Game
  Mode are built in, the user's modes are `focus-mode` blocks. On the loop:
  one timer for the next edge, the Focus glyph, "{mode} on/off" events for
  automatic and chord changes, banners held with allowed apps and
  time-sensitive ones passing, Game Mode holding the performance profile and
  releasing it, Game Mode drawing the shell solid and still in place of P6's
  game-in-front stand-in, the summary once the last silencer ends ("While you
  played", or the one held shown again). The Do Not Disturb and Game Mode
  chords, the Focus module and page with durations, `Focus1` and `isle focus`.

- P8 8.9: Notification Centre, moved in from P17. The island's second panel:
  the clock, Ctrl+Option+Cmd+N and the banners' overflow open it, a click on
  the pill turns it into Control Centre in one morph, and no peek replaces
  it. The date, a fixed six-week month from Monday with today in the accent,
  then live notifications stacked by app with layered edges (Show All, a
  clear per app), Earlier for records, and Clear All in two steps. While it
  is open no banner shows anywhere and arrivals go into it. The agenda waits
  for calendars.

- P8 8.8: banners. `isle-shell`'s `banners::model` is pure: one card per app
  with its count, three at most newest first, the oldest temporary one
  leaving for a fourth, the overflow row, lives from the tokens' banner
  timings, time paused while hovered and resumed after a grace, arrivals
  waiting under the pointer or behind a panel, those waiting past their life
  dropped. The `banners` surface is a placed top-layer surface at the main
  screen's top right under the strip, its hit shapes the cards; cards come in
  from the trailing edge, still in Game Mode. Clicks, actions, close and the
  reply field (which takes the keyboard and gives it back) go through the
  server; a banner suppresses its app's attention event. Sounds: the app's
  file or theme name, else the alert, uploaded once to pipewire-pulse as a
  sample and played, one per burst. The `notifications` settings section and
  per-app `notification-app` rules. A banner is on screen inside 100 ms of
  Notify in the headless test. The summary after silence comes with Focus.

- P8 8.7: the notification server. `isle-notifications` serves
  `org.freedesktop.Notifications` (spec 1.3 with `inline-reply`,
  `NotificationReplied`, `ActivationToken` and `Inhibited`), owned before the
  compositor announces its socket; hints read leniently, pictures sent as
  pixels written out as PNGs and files copied into the cache at arrival.
  Each arrival is decided once on the loop (transient, muted, low, held, else
  a banner) against what holds banners; a replacement keeps its id; an app's
  close or its leaving the bus makes a record, the user's close or action
  removes it; actions send an activation token first, which brings the app
  forward. Records persist in the `notifications` state domain, 300 kept,
  transient ones never. `Notifications1` and `isle notify list|close|clear`;
  the test client notifies and prints the server's signals.

- P8 8.6: media. `isle-media` follows every MPRIS player, the most recently
  playing first, its position run on from when it was read, and fetches a
  track's art (`file://`, `data:`, HTTPS capped at 2 MB and 5 s) into the
  cache. The island's media face with the art, the Now Playing module (Large:
  art, track, transport) and its page (the player in full with its scrubber,
  shuffle and repeat, then the other players), the play, next and previous
  keys through MPRIS; every media key now goes through a service, and v1's
  `wpctl`, `brightnessctl` and `playerctl` commands are gone. Media leaves
  Now while the module is in the grid; a page whose rows hold the control
  has a plain head.

- P8 8.5: Bluetooth. `isle-bluetooth` reads BlueZ's object tree in one call
  after any of its signals: the adapter, devices paired and nearby with
  their batteries. The Bluetooth glyph while a device is connected, the
  module and its page (My devices, Nearby devices), discovery started when
  the page opens and stopped when it closes, and a row that connects,
  disconnects or pairs and connects; a device that needs a code waits for
  P9's agent.

- P8 8.4: network. `isle-network` reads NetworkManager whole after any of
  its signals: the wire, Wi-Fi with the network joined, known networks and
  the rest in range one per name, and VPN and WireGuard connections; on the
  host it read the joined network and the unplugged wire. The Wi-Fi glyph
  (on Wi-Fi, connecting, or offline; a wire is normal) and the VPN glyph;
  the Wi-Fi module and page (Ethernet's state, known networks, "Other
  networks" folded, scanning every ten seconds only while the page shows)
  and the VPN module. A secured network NetworkManager has no password for
  says "needs a password" in its row until P9's prompt. Service tests now
  count only stalls in worker jobs, since a first panel draw in a debug build
  can stall the loop by itself.

- P8 8.3: brightness. `isle-brightness` speaks DDC/CI's brightness itself
  and finds each monitor's bus by its EDID, since the NVIDIA connectors have
  no `ddc` link; on the host it read the AOC on `/dev/i2c-4` at 92 of 100,
  as ddcutil does. Laptop panels go through logind's `SetBrightness`. The
  Display module and page (one slider per screen, the panel's first), the
  brightness keys on the main screen with their OSD there, values shown at
  once and written behind, at most one write per 200 ms; the keyboard
  backlight's OSD from UPower.

- P8 8.2: audio. `isle-audio` speaks the Pulse protocol to pipewire-pulse
  over its own async connection on the worker (the `pulseaudio` crate's
  high-level client has no subscribe or volume, so Isle drives its protocol
  layer): devices, defaults, volumes, per-app streams, recordings, an input
  meter that runs only while Sound's page shows; and watches PipeWire's graph
  for a camera node linked to an app. The Sound glyph, the microphone and
  camera marks with their apps in Now, volume and microphone OSDs raised by
  any change but Control Centre's own, the volume and mute keys through the
  service, the Sound module (Wide, with its slider) and its page. Tiles now
  switch on their glyph and open their page from the rest. Tests run a
  private PipeWire with null devices. Found: the NVIDIA EGL driver dlopens
  libdbus itself, so the rule is checked against Isle's own audio modules.

- P8 8.1: power. `isle-power` reads UPower's batteries (the machine's and
  peripherals' by kind, coarse levels included) and power-profiles-daemon's
  profiles and holds; `isle-bus` gains whole-object property reads and a
  follow loop that re-reads a small daemon after any of its signals. The
  Battery glyph shows on battery or when any battery is low; Energy Mode and
  Batteries modules with their pages; a user's choice of profile ends a
  hold. `Test1.Panel` drives Control Centre, and the island's status lists
  its modules and the open page's rows.

- P8 8.0: foundations. `isle-bus` (the buses, a daemon's name followed,
  snapshots, listeners); a private system bus in the test harness with
  `isle-fakes`' logind; services reach the loop as snapshots, a burst's last
  value once a frame (`isle services`, `Services1`); logind's
  `PrepareForSleep` re-reads the island's clock on resume. Control Centre's
  pages hold rows by kind (choices, devices, sliders, switches, headings,
  folded groups, status lines, sub-pages) and scroll past the room they have;
  the body is laid out at the shape's settled height, so a growing panel
  shows no scroll bar. `isle-ui` gains the slider, scrubber, switch, spinner
  and scroll area, drawn in the gallery.

- P8 planned: services, notifications and Control Centre. Reviewed; every
  recommendation taken: Notification Centre's notifications in P8, night
  light to P12, one packet. Research in `services-2026.md`.

- B59: the switcher shows icons, with live previews in the window row only;
  chosen by the user from both drawn in the VM. Theme SVGs without a viewBox
  now scale, so an icon drawn larger than its file fills its box.

- B58 widened: the Glass setting's default is the clear end, so the island's
  panels, menus and Control Centre are clear where the backdrop allows and
  the floor darkens them only as their text needs. The island's pill no
  longer stacks the glass tint under its own fill.

- B58: glass with no text on it is clear and never raised by the floor; the
  switcher's panel is clear and its name sits on a strip of the default
  glass. Chosen by the user from three renders.

- P7: the switcher drawn to the gate-5 board, not the sheet's drifted
  numbers: window-coloured cards with hairline edges, round marks, progress
  in the card, the name centred, titles in the window row; the sheet follows.
  An app with no id is named by its process.

- P7 closed. ARCHITECTURE has How switching works, B53–B57 are recorded,
  the intent docs' open points answered, the plan ends with its outcome.

- P7 7.6: the measure and the VM. Over twenty apps at 3440×1440 the panel
  draws at 0.5 ms p50, 1.4 ms p95; a preview from an uploaded texture costs
  0.25 ms, and changed windows are drawn again four at a time, ten times a
  second. The VM's kitty, xterm, Chromium, Qt Designer, a hidden GTK 4 app
  and a background app read as the board. Found and fixed: the test client
  never let go of its buffers.

- P7 7.5: the drag's icon is drawn, and a drag carries through Cmd+Tab.

- P7 7.4: the panel and its live previews.

- P7 7.3: the hold. A thousand scripted switches commit on release with no
  modifier left held. Found: letting Shift go ended a Shift+Cmd+Tab hold.

- P7 7.2: the model. 7.1: LauncherEntry and Steam's names and covers. 7.0:
  attention levels, background apps, one window order.

- P7 planned: the switcher. Reviewed; every recommendation taken. P6's
  host run waived.

- P6 screenshots in the VM, and what they showed: menus and the inhibit
  notice drew their glass inside their padding, leaving a ring; the About
  panel could not draw SVG icons and clipped its button; a dialog with its
  own app id named itself in the bar; Control Centre's check was the
  filled glyph; a submenu with nothing enabled stayed enabled. All fixed.

## 2026-10-06

- P6 6.8: closed. ARCHITECTURE has How the top of the screen works,
  B47–B52 are recorded, the open points of `shell.md`, `platform.md` and
  `compositor.md` answered, the plan ends with its outcome. The host run
  is the user's.

- P6 6.7: the VM. Qt 6 (Wayland and X11), Chromium 153, LibreOffice 26.8
  on GTK 3 and GtkApplication menubars on GTK 3 and 4 show their real
  menus and take activations; kitty, zenity and xterm fall back. Found and
  fixed: lookups only when the bar was built, a dialog without the app's
  menus, LibreOffice's late-filled menubar (GTK signals only subscribers),
  GTK toggles watched in the wrong group, an unblurred menu opened from
  the keyboard, the bar outliving its window, a blank name for a window
  with no app id, and swapped PC keyboards' hints naming the wrong key.

- P6 6.6: menu search and the measure. Help holds a search field over the
  app's menus, folded titles included, each result with its path; opening it
  asks the app to fill its submenus. `isle menus search`. At 3440×1440 in a
  release build the island and the bar draw at 0.8–1.1 and 0.5 ms p95 and
  the process sits at 163 MB and 92 MiB at most (`p6-island-and-menus.md` in
  research). Found and fixed: a closed tree's state written after it
  unmounted panicked on every reveal, unseen, and the backtraces took RSS to
  500 MB.

## 2026-10-05

- P6 6.5: the menu sources. `isle-menus` reads DBusMenu and GMenuModel into
  one tree and serves the registrar, owned before the socket is announced; the
  compositor serves `org_kde_kwin_appmenu` v2 and `gtk_shell1` v5 with its
  title-bar gestures, reads X11 windows' menu properties over a connection of
  its own (no fork patch needed), and finds an Electron-style registration by
  the app's process. Real menus replace the table fallback, hints map back to
  the Mac chord, the app's Window and Help items join Isle's.

- P6 6.4: the menu bar and the fallback. A push at the top edge outside the
  island's zone, Ctrl+F2 or Shift+Cmd+/ brings the bar down on that screen with
  the system menu, the app menu, File, Edit, View and the rest from labelled
  rules in the translation tables, Window and Help; titles that reach the
  island's reserve fold into ». The window menu opens from a title bar, a
  client's own and its chord, with keyboard move and resize and Keep on top.
  Isle's About panel. Menus1 and `isle menus bar|activate`. Found: Freya sizes
  an absolutely placed fill against the room an auto-sized parent had, so a
  menu's glass reached the bottom of the screen; menus now set their height.

- P6 6.3: Control Centre and the top edge. The panel unfolds from the pill
  with its notices (a rejected configuration, the safe start, shortcuts
  inhibited), activity rows, the packed grid and pages; Dark Mode and Keyboard
  layout are its first modules. Hover peeks without the keyboard, a click or
  Ctrl+F8 pins with it, Escape over a peek is not passed on, a push at the
  edge brings the island back over full screen. A window that takes the keys
  shows a notice inside its top edge. Found: a moving island that drew nothing
  visible stopped the loop's redraws, and a glass shape measured before its
  surface was sized was negative.

- P6 6.2: the island. One queue model sources post to (glyph tiers, privacy,
  ranked activities, events with replacement, OSD pre-emption, waiting and
  dropping), a surface on every output whose shape the compositor springs and
  feeds to the tree with the morph's progress, the clock waking once a minute,
  the main screen following focus. Real sources so far: shortcuts inhibited,
  a rejected configuration or safe start, the layout chord, attention, a game
  in front. The island sits in the overlay layer, so its privacy form stays
  above other programs' overlays. Minimise shrinks into it.

- P6 6.1: `isle-ui`'s first components (glyphs with Phosphor and Isle's own
  four, text styles, buttons, the level bar, the menu with its keyboard), a
  gallery surface in test builds and the preview, and in `isle-apps` entry
  actions, Exec field codes, icon theme lookup and AppStream links. Apps start
  in a systemd scope with an activation token (`isle apps launch|open`). A
  menu's width is estimated from its text: Freya cannot size a row with a
  spring in it to its content.

- P6 6.0: the wallpaper. Frost drawn by a shader, pictures decoded and fitted
  on the worker, per screen or for all, folders, the crossfade and its cut,
  nothing drawn under a full-screen window. A 5K PNG takes about 3 s to
  decode and scale in a debug build.

- P6 planned: island, menus and wallpaper. Reviewed; every
  recommendation taken, kept as one packet.

- P5 host run by the user on tty8: both keyboards and the apps behaved.
  Found and fixed: the US layout by default, and no input after a start on
  a console not in front.

- P5 closed but for the user's host run. ARCHITECTURE describes how keys are
  read; B44–B46 record the seat fed only Isle's keys, the tables as a data
  file, and the game flag.

- P5 5.6: the VM. Cmd chords reach wev and xev with modifiers that agree
  with the keys; Option types # through Xwayland on a Mac profile; GTK 4
  pastes with Cmd+V. xremap stops when the Isle session starts.

- P5 5.5: Sticky Keys and Slow Keys.

- P5 5.4: games from Steam's labels and GameMode, inhibitors and Cmd+Escape,
  Isle's own keys, recording. GameMode is session-activated, so Isle asks it
  only while it runs.

- P5 5.3: the translation tables; every shipped rule typed into an app.

- P5 5.2: the shortcut set, with clashes refused.

- P5 5.1: the engine. Num Lock had reached Freya as a held modifier and
  stopped Tab moving focus in shell surfaces.

- P5 5.0: keyboards, profiles and keymaps.

- P5 planned: keyboard. Reviewed; v1's chords revisited, games read from
  Steam's labels.

- P4 host run by the user on tty8: Chromium on Wayland and X11, VS Code,
  Steam and a full-screen game behaved. Isle no longer draws while its
  console is in the background. For P14: the supervisor's exit clears the
  shared session variables; v1 and Isle share a session log path.

- P4 follow-up: decorations by each protocol's rules, the Smithay fork with
  SHAPE in its X11 window manager, shaped windows bare and blended. xeyes in
  the VM had worn a bar over a see-through body.

- P4 done but for the user's host run. ARCHITECTURE describes how windows
  are kept; B40–B42 record the registry, attached and placed surfaces, and
  the X11 policy.

- P4 4.9: the VM. kitty, Qt 6 Designer, xterm and xeyes wear Isle's bar,
  GTK 4 keeps its own; tiling, minimising and dialogs behave. The
  supervisor never exported the session until the ready pipe was made
  close-on-exec: Xwayland held it open.

- P4 4.8: Xwayland under Smithay's window manager, restarted on its
  display; X11 windows are records, override-redirect and untitled ones are
  left where they ask.

- P4 4.7: the zoom menu from a dwell or Ctrl+Option+Z, and the pointer
  panel with Option in a drag.

- P4 4.6: title bars as attached Freya surfaces, with the lights, the
  title's layout, the resize band; 0.27 MB and 0.47 MiB a bar.

- P4 4.5: rounded corners and shadows from shaders; open, close, hide,
  minimise, tile and desktop motion, close and morph drawn from snapshots.

- P4 4.4: desktops per output and full-screen desktops.

- P4 4.3: `isle-apps`, the close path with keep-running, Cmd+Q, Cmd+H,
  Cmd+M, Cmd+`, pings.

- P4 4.2: zones, edge regions, snapping, tiling chords.

- P4 4.1: placement, places, dialogs.

- P4 4.0: the window registry.

## 2026-10-04

- P3 done. ARCHITECTURE describes how the shell runs; B37–B39 record the
  surface model, typed tokens and compositor glass; `shell.md`'s budgets
  are set.

- P3 3.9: the VM and the host. The probe draws in the guest session on
  llvmpipe and takes QEMU's keyboard and tablet. On the 5070 Ti at
  3440×1440 a full-output draw is p95 0.57 ms, 1.28 ms with 24 elements
  fading in layers; glass blurs in p95 0.75 ms. A one-hour soak held VRAM at
  105 MiB; RSS is unreadable under the host's swap, and the process's own
  memory keeps a few KB a cycle, not yet explained.

- P3 3.8: the preview. `isle-preview <surface>` with live tokens. dx
  0.7.10's hot patch fails on an edit to `isle-shell` (it replays the
  compositor, which the preview does not build); `--watch` rebuilds and
  restarts in 2.7 s.

- P3 3.7: glass. The backdrop behind each glass shape is drawn again
  through its own damage tracker and blurred with dual Kawase passes
  sized from `material-blur`; the floor reads a 4×4 reduction back a
  frame late and raises the tint until `text-2` holds 4.5:1.

- P3 3.6: transitions and the cache. Open and close are compositor
  springs on the texture with a rounded clip; a closing surface keeps its
  last frame until the exit ends. Skia's cache over a hundred opens: 12.7
  MB after the first close, 13.4 MB after the hundredth, under the 64 MiB
  cap.

- P3 3.5: tokens. `isle-tokens` types every shipped token from the file
  at build time; the user's file lies over it by path; `appearance` in
  settings. Found: with fonts loading the loop starts after the bus, so
  the test harness waits for the loop.

- P3 3.4: panic boundaries. Found in Freya: a panic in a component left
  its reactive context on a thread-local stack, so the rebuilt surface
  panicked again outside the boundary; the fork now pops it on unwind.

- P3 3.3: accessibility. Each surface has an accesskit_unix adapter on
  its own D-Bus thread, answered from the loop; `isle shell tree` dumps
  the tree. On a private session with its own at-spi bus the probe's
  window, buttons, field and rows are listed; the host's AT-SPI is off
  and turning it on would persist, so it was not used.

- P3 3.2: input and focus. The seat's focus is a client surface or a
  shell surface; a key-taking surface takes the keyboard and gives it
  back before its actions run. Found: `surface_under` held the layer map's
  lock while asking for it again, which wedged the loop on the first
  pointer motion over the probe.

- P3 3.1: hosting. Surfaces draw through one Skia context into textures
  on the compositor's context and stack against layer-shell. Found in
  the fork: Freya's `Timeout` (every `Input` and `ScrollView`) woke the
  app twice a second forever; it now sleeps until it is reset.

- P3 3.0: `freya-embed` in the fork (branch `isle/freya-embed`), rebuilt
  on `freya-web`'s frame step, with seven tests on Skia's CPU raster. One
  race found: the runner's future can finish early on dirty scopes, so
  `update` loops until it is pending and the waker is armed. bindgen 0.73
  breaks when syn 2 and 3 meet; prettyplease is pinned to match.

- P2 done. ARCHITECTURE describes the core as built; B35 records its
  shape and B36 how settings are written; the roadmap moves second-GPU
  scan-out and v1's units to P14 and mode confirmation to P13.

- P2 host smoke run: Isle lit the ultrawide on the 5070 Ti from tty8 and
  took input. Two findings: the pointer redrew only when something else
  did (fixed: motion queues a redraw whenever Isle draws the cursor), and
  xremap from the Hyprland session rewrites Cmd chords before Isle sees
  them, and would be started by isle-session.target too.

## 2026-10-03

- P2 phase 2.7: the supervisor, tested over the headless compositor: a
  core panic, `kill -9`, three quick deaths into a safe start and back,
  and a clean quit.

- P2 phase 2.6: outputs, layer-shell and DRM. The compositor runs on the
  VM's real DRM in a logind session on tty8. Found there: a GPU opened
  before the session is active cannot become DRM master, so opening waits
  for activation; a re-lit output's first frame drew no windows, because
  the space learns window-output overlap in its refresh, which ran after
  the redraw. Twelve headless tests pass.

- P2 phase 2.5: the headless backend, the Test interface's input,
  outputs, frames and screenshots, a Wayland test client and seven
  integration tests, passing twice in a row in about four seconds.

- P2 phase 2.4: the state store and the worker's debounced writer, with
  tests for a round trip, a damaged file, a newer version and a burst of
  fifty saves written once.

- P2 phase 2.3: settings. A broken hand edit keeps the last good settings
  and refuses Sets with its line and column until it is fixed; Sets keep
  the user's comments, create keyed nodes and app sections where absent,
  and write through symlinks; an editor's rename-replace is picked up.
  Two bugs found in the run and fixed: reading the file raised inotify
  access events, which re-read it every 100 ms; an output's new scale did
  not reach clients' fractional scale. The `binds` section runs a
  terminal, closes the focused window and quits.

- P2 phase 2.2: the worker, the D-Bus API and `isle`. Checked nested on
  a private bus: version, window and display lists, close (kitty asked
  first, as it should), an unknown id refused, `Windows1.Changed` on
  close, quit; a request during an injected 3 s stall failed after 1 s
  while the watchdog logged the stall; an injected service panic was
  restarted in a second with the compositor untouched and no crash
  record. The build time now comes from the binary's own file, since a
  build script only reran on git changes.

- P2 phase 2.1: the nested compositor. One calloop loop over `State {
  backend, isle }`; xdg-shell with toplevels placed centred on their
  first buffer, click to focus and raise, popups kept on the output and
  grabbed, client moves and resizes; the protocol baseline; damage-only
  rendering through niri's redraw states, with an estimated-vblank timer
  for frame callbacks. A heartbeat logs dispatches over a frame's budget,
  a watchdog aborts after ten seconds stuck, and panics write a crash
  record. Found on NVIDIA: Smithay never sets the swap interval, so every
  swap waited about 16.7 ms inside the loop; Isle sets it to 0.

- P2 phase 2.0: the workspace (`Cargo.toml`, `rust-toolchain.toml`,
  seven crates), `data/tokens/` and `data/icons/phosphor/` moved from P1,
  the board server rooted at the repository so boards find them. Smithay
  pinned upstream; the fork waits until a patch needs it. `kdl` 6.7.1
  reprints `} }` as `}}` (its parser discards the space after a children
  block, still so on main), so the settings writer will splice by span
  rather than print the document; a test checks every span in
  `tokens.kdl`.

- P2 planned (`p2-compositor.md`): the workspace at the repository root
  with seven crates and `data/`, the Smithay fork on an `isle` branch,
  and phases from a nested compositor through D-Bus, settings, state, a
  headless test backend, DRM and outputs, and the supervisor, to booting
  the VM into kitty. The user took every recommendation.

- P1 done. Stage 6 reviewed every surface for critique, accessibility and
  copy (298 findings, all applied); the contrast script grew to cover
  focus rings, badges, hover states, labels on fills and kind colours, and
  passes. The user signed off DESIGN.md at gate 6 ("lock it in for now").
  B31–B34 record Frost as data, how attention is presented, where glass
  may go, and the readings of the principles.

- P1 gate 5, wave C passed: Notification Centre and widgets, Mission
  Control and the window surfaces, Files, Monitor, Keychain and Cleaner,
  drawn on the wave C board. A colour-vision check failed stage 4's graph
  colours; a new four pass it, and Cleaner's ten kind colours were checked
  the same way. Stage 5 is complete.

- P1 gate 5, wave B passed: specs for banners, Focus, Control Centre's
  modules, the lock and prompts, Spotlight, capture, Settings and the app
  conventions, reconciled and drawn on the wave B board. The lock adds its
  own wash so its text passes; prompts are one glass frame over a scrim
  with an armed moment; sliders on glass fill with the text colour.
  About seventy tokens added; every glyph the specs name is vendored.

- P1 gate 5, wave A passed: specs for the island, menus, window chrome,
  the switcher and the wallpaper, drawn on the wave A board with a shared
  scene kit. The strip and the revealed bar are both 48 so their centres
  align; colour on the island sits in solid wells; menus are glass in the
  shell, raised in windows, solid over glass. About forty tokens added;
  the contrast check covers the island. DESIGN.md has its first surfaces.

- P1 gate 4 passed: Frost made into the system. The token file grew roles
  for every surface, eight accents, the glass range, macOS's text styles
  in Inter, the concentric corner rule, five springs, a terminal palette
  and the icon group; a contrast script checks every pair in both
  appearances and under Increase Contrast. Phosphor, Papirus and Bibata
  chosen. Research found Isle must serve the Settings portal (B13) and
  that Qt under the GTK platform theme bypasses Isle's file panels (P14).
  DESIGN.md's first draft written.

- P1 gate 2, batch 3 passed: Notification Centre unfolding from the pill
  with the date, month and agenda; the widgets dashboard as one canvas
  per screen on Ctrl+Option+Cmd+W; Mission Control with a minimised tray
  and a short slide between desktops; Files without tags or Connect to
  Server, undo as a stack; Monitor with a GPU view; Keychain's reveal
  gated once on opening. Stage 2 is complete; the intent docs follow.

- P1 gate 3: the user chose Frost. Three directions (Frost, Slate, Dusk)
  were drawn on the harness, now able to switch directions, each from its
  own `tokens.kdl` through a shared kit of components. Batch 3 of the
  structure sheets ran beside it and is on its own board.

- P1 gate 2, batch 2 passed with every recommendation accepted: banners
  top right under the island's strip; Game Mode wins over a manual Focus;
  Control Centre a four-column grid of sixteen modules, media only in Now
  Playing, Game Mode a row of the Focus page; Cmd+Tab sets a prompt
  aside; Spotlight without an assistant, Quick Look on Cmd+Y; one capture
  thumbnail with Markup in Quick Look; the greeter in the user's look;
  System Settings in 28 panes with Game Mode folded into Focus and
  Displays. The batch 1 sheets and the intent docs follow.

- P1 gate 2, batch 1 passed. The first board (menu bar always shown, Dock
  at the bottom) was redrawn when the user preferred v1's minimal desktop:
  revision 1 keeps v1's pill with the clock and active glyphs, opens
  Control Centre from it, brings the app's menus down on demand, and drops
  the Dock for Spotlight's Apps grid and the switcher. Every
  recommendation accepted; standard menu items keep Apple's title case.
  B18 and B19 revised; shell, compositor, gaming, apps and platform intent
  docs, DESIGN, ARCHITECTURE, README and the roadmap (P6 island and menus,
  P7 switcher and launching) updated.
- P1 stage 2 started: the structure template, the harness
  (`p1/harness/`, served by `serve.py`, `p1-boards` in launch.json), and
  batch 1's sheets for window chrome, the menu bar, the island, the Dock,
  the switcher and the wallpaper, with the copy and competition sheets.

- P1 stage 1: research notes on macOS 27, other desktops and v1's
  lessons written; the interview took four rounds (both appearances dark
  first, frosted glass with a slider, Mac density, one picked accent,
  restrained motion, Dock at the bottom, the island a dark pill opening
  on hover, a full-screen widget dashboard kept). The brief passed
  gate 1 ("agreed for now").

- P1 planned (`p1-design.md`). The user chose HTML mock-ups first, then
  Freya from P3's preview mode on, and working files in the repo with
  each gate's board also published as an Artifact.

## 2026-10-02

- The user pointed out that presentation (the island's priority order and
  the like) belongs to a real design process, and that coding agents are
  no longer a feature. B24 rewritten to hand presentation of attention
  surfaces to P1; P1 rewritten as a six-stage design process; coding-agent
  references removed from DECISIONS; the intent docs are being split into
  capability rules and a "For the design process" list.

- B30 refined: the Alt/Super swap is for PC-layout keyboards and is
  suspended in game windows; Mac-layout keyboards need none.

- B24–B30 folded into the intent docs, ARCHITECTURE, DESIGN and the
  roadmap. A coverage audit checked both directions and fixed what
  disagreed (Game Mode wording, the island as the home of casts and
  activities, calendars as notifications, apps' Modes rows, roadmap
  briefs; P22 moved after cut-over). Nothing committed yet.

- The user accepted all 21 proposed answers; locked as B24–B30.

- B23: HDR in scope as P22 after M2. The 21 open points from the intent
  drafts are listed in `brainstorm.md` for the user.

- Intent docs drafted for all five areas. The user dropped Isle's own
  console mode (B22); gaming, shell, compositor and platform docs and the
  roadmap updated.

- The user set the north star (macOS-style on Arch) and parity scope:
  B2 revised; B17–B21 locked (menu bar with global menu and the island in
  the notch, Dock, app-centric windows, Cmd everywhere).

- Round 4 locked B13 (own portal backend), B14 (built-in widgets plus
  providers), B15 (Isle) and B16 (this repo and branch). Brainstorm
  complete.

- Round 3 locked B10 (compositor-owned settings, KDL, live tokens), B11
  (shared zbus service crates, pulse audio) and B12 (prompt, tray and
  lock roles).

- Round 2 locked B8 (stacking with snapping, layout per workspace) and
  B9 (one loop, service worker, panic boundaries, supervisor, watchdog).

- Locked B7: Freya through the user's fork for shell and apps. Round 2 of
  the spike (a 3440×1440 launcher) decided it; final numbers rerun one at
  a time on a quiet host. The user updates the fork to rc.8.

- Spike results in `docs/research/rust-desktop/shell-ui-spike.md`: both
  variants run the island at about +0.1–0.6 ms over the empty base and
  idle correctly; Freya wins text (colour emoji, font features) and GPU
  scaling, Slint wins the iteration loop (live reload 80–96 ms, state
  kept), maturity and VRAM. Large full-output surfaces untested. The base
  now paces by frame callbacks; the host itself paces new windows at
  30 fps (es2gears shows the same).

- Spike base built in `spikes/shell-ui/base`: Smithay 118e34f, winit
  backend, island shape sprung and drawn by GLES shaders, content clipped to
  it. Empty-content floor on the host: morph frame 0.87 ms p50, 1 idle
  frame in 10 s, RSS 145 MB / PSS 81 MB, 38 MiB VRAM. Freya and Slint
  variants handed to two agents in parallel.

- Recon: inventory of v1, Smithay and gaming research, UI and services
  research, in `docs/research/rust-desktop/`.
- Round 1 locked B1–B6. Shell UI rendering compared in depth (Freya on
  Skia, own scene graph, CPU-drawn); the user chose to spike Freya against
  Slint.
- Otto checked at d308f56: the compositor draws dock, exposé, switcher,
  OSDs and window chrome; bar, islands, launcher and lock are otto-kit
  clients, styled by the compositor through `otto-surface-style-unstable-v1`.
