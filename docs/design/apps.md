# Apps

Intent document for the rust-desktop program. Nothing here is built yet.

Isle's own windows are five apps: Settings, Files, Monitor, Keychain and Cleaner, each a separate Freya
process on the user's fork (B7), the fork strata and cairn (`~/Development/`) also build on, started when
wanted and gone when closed. This document sets the rules every app follows, then what each gives the
user, measured against the Mac app it stands for (B17) and everything v1's window did (B2). The D-Bus
interfaces the apps call are in `platform.md`; the menu bar, Dock, launcher and shell-drawn dialogs are
in `shell.md`. Evidence: `docs/research/rust-desktop/` `isle-v1-inventory.md` (cited as "inventory §n"),
`macos-model.md` and `ui-and-services.md`. `Dn` is entry n of `main:docs/DECISIONS.md`.

## Rests on

- **B2** · v1 parity: every v1 window capability lands somewhere (Parity map).
- **B3** · fresh visual design: nothing here fixes a look or a layout; P1 decides them (For the
  design process).
- **B7** · the apps are Freya apps on `alexparlett/freya`, as separate processes.
- **B8** · window places, minimise and zoom zones are the compositor's; an app never places itself.
- **B9** · separate apps are separate processes; a crash is contained to its process.
- **B10** · the compositor owns settings and tokens; tokens reload live.
- **B11** · system state through shared zbus crates; desktop state from the compositor's D-Bus.
- **B12** · the polkit agent, the keyring prompter (the compositor's `SystemPrompter`) and the Wi-Fi
  secret agent are the desktop's; the apps are their clients.
- **B13**, **B26** · the portal backend is a bus name of the compositor's service worker and hands
  FileChooser to Files; the Open and Save panels and Quick Look run in the Files process; apps render
  on the GPU by default, measured in the Settings packet.
- **B17** · where a choice is open, the answer is the one a Mac user expects.
- **B18** · app menus live in the menu bar.
- **B19** · the Dock carries badges and progress.
- **B20** · app-centric windows: Cmd+Q quits, Cmd+W closes, launching what is open raises it.
- **B21** · Super is Cmd everywhere.
- **B22** · no Big Picture of Isle's own: Settings loses its options, the Controllers page stays.
- **B25** · Isle's apps draw a unified toolbar with the traffic lights.
- **B27** · app preferences are per-app sections of `settings.kdl`, written through the compositor.
- **B28** · the Bluetooth pairing agent is a desktop role, like B12's.
- **B29** · Isle is a package built from git, so Software Update updates it with the system.
- **B30** · Focus modes as on macOS, with a Gaming Focus; Alt and Super swapped on PC keyboards.

## Conventions

Every Isle app follows these rules.

**Process and lifetime**

1. Each app is its own binary and process, with its own Wayland app id and desktop entry:
   `isle-settings`, `isle-files`, `isle-monitor`, `isle-keychain`, `isle-cleaner` (D92). The switcher,
   Dock and menu bar show five apps under five names without matching titles (D75).
2. No app starts with the session. An app is started by its desktop entry, its command line, or D-Bus
   activation of its own well-known bus name (`ui-and-services.md` §1.4: big apps start on demand).
3. Each app is single-instance. A second launch hands its arguments (a pane, a folder, files) to the
   running instance and exits; the running instance raises its window or opens what was asked (D63).
4. An app exits when its last window closes and no work is in flight. Work in flight is a file job, a
   removal, a key generation or an update hand-off: the process stays while it runs, with progress on
   its Dock icon (B19; `macos-model.md` §3 Dock row). Isle's apps are not on B20's keep-running list.
5. Cmd+Q in an Isle app is the app's own Quit item, so an app with work in flight says what quitting
   stops and lets the user cancel.
6. An app quit with windows open reopens them with the same content (pane, folder, tabs, view) on its
   next launch. Where a window sits is the compositor's remembered place (B8, D91), never the app's.
7. A crash ends that app only. The session, the shell and the other apps carry on (B9), the app can be
   launched again at once, and its panic message goes to the session log.

**Menus and keys**

8. Every app exports its whole menu bar as DBusMenu, tied to its window's surface through
   `org_kde_kwin_appmenu`, which the compositor serves (B18; `macos-model.md` §1.1, §1.3). An Isle app
   never shows B18's fallback menu and never depends on the PID fallback.
9. Each app's menus reach About, its settings where it has preferences, hide, hide others and quit,
   and every command the app has (rule 10). The menu bar supplies search over the menus
   (`macos-model.md` §1.3 item 9; `shell.md`).
10. One command table per app is the record: menu items, shortcuts and their hints are built from it,
    and a test fails when a command with a shortcut is in no menu or in two (strata's
    `every_command_is_in_exactly_one_menu`, strata `docs/PLATFORMS.md`).
11. A menu item and its shortcut take one path: activating the item delivers the command to the
    focused window as the key press would (strata `docs/FREYA_STATE_ARCHITECTURE.md` §10). An item the
    focused window cannot act on is disabled.
12. Isle's apps take Cmd natively: they are on the compositor's no-translation list and bind Super as
    Cmd themselves (B21). Ctrl stays Ctrl: in a text field Ctrl+A, E, K and the rest are the Mac's
    Emacs motions, and no Ctrl chord stands in for a Cmd one. Option is Alt and moves by word.
13. Menus show every shortcut as the Mac chord the user presses.
14. Desktop chords stay the desktop's and never reach an app: Cmd+Tab, Cmd+`, Cmd+H, Cmd+Option+H,
    Cmd+M and Cmd+Space (B20; `macos-model.md` §2.4).

**Windows**

15. Each app draws a unified toolbar carrying the traffic lights and asks xdg-decoration for
    client-side mode, so the compositor draws no title bar over it (B25). Close, minimise and zoom act
    through the compositor (`xdg_toplevel` close, set_minimized and set_maximized), so they behave as
    for any other app: minimise goes to the Dock and zoom goes to B8's zones. A double click on the
    toolbar does what the user's system setting says.
16. Every window resizes from every edge and has a minimum size at which nothing is clipped.
17. Translucency is asked of the compositor through `ext-background-effect-v1`; no app blurs behind
    itself (`ui-and-services.md` §2).
18. A confirmation or other dialog is modal to its window, not to the app. Focus moves into it when it
    opens and back when it closes; Escape and Cmd+. cancel; Return takes the default button (strata
    `docs/UI_CONVENTIONS.md`; the fork's modal boundary).

**State and services**

19. Desktop state (settings, tokens, mode, the window list) is read from and written to the
    compositor's D-Bus interface (B10, B11; `platform.md`). An app's own preferences are its section
    of `settings.kdl`, through the same interface (B27). No app opens `settings.kdl` or `tokens.kdl`
    itself. A write refused because `settings.kdl` does not parse leaves the control at its last good
    value and says where the parse error is (B27).
20. System state comes through the shared service crates, which talk to the system daemons directly
    (B11): NetworkManager, BlueZ, UPower, logind, UDisks2, AccountsService, timedate1, locale1, power
    profiles, the Secret Service and pipewire-pulse. A subprocess runs only where the tool is the work
    itself (bsdtar, ssh-keygen, pacman under pkexec), never to read state a daemon or a file format
    already offers (inventory §1 lists the JSON scripts this replaces).
21. A value changed elsewhere (Control Centre, the CLI, a hand edit, another app) shows in an open
    window without reopening it.
22. The UI thread never waits on I/O, D-Bus or a subprocess. Work runs on a worker and comes back as a
    value, and every view has a loading state distinct from an empty answer (cairn
    `docs/design/concurrency.md`).
23. A missing daemon or device (no Bluetooth adapter, no printing service) is a state the window
    shows, never a crash or a blank.
24. Sampling and live views (meters, graphs, a pad drawing, SMART) run only while visible; a hidden or
    minimised window samples nothing (inventory §6 Polling; D54).
25. A privileged change goes through the daemon's own polkit action where one exists, and otherwise
    through one Isle helper run by pkexec that accepts a fixed set of operations (D47, D79). One
    change asks once, in the desktop's polkit dialog (B12); never a terminal, never sudo.
26. Token changes apply live: the accent, text scale, high contrast, reduced motion and Game Mode's
    zero motion re-render every open window without a restart (B10; inventory §9, §10).
27. v1's preferences, window places and launch counts are imported once (B27); nothing reads v1's
    files after that. Notification and clipboard history are not imported.

**Access and build**

28. Every action is reachable from the keyboard and Tab reaches every control. The focus ring shows
    for keyboard input only; no app takes controller input (B22).
29. Every control exposes its role, name and state through AccessKit (`ui-and-services.md` §2), so a
    screen reader reads every window.
30. A window with a list or pages to search has its search field on Cmd+F.
31. Every Freya crate in the workspace pins the fork at one revision, so one `freya-core` links;
    components are tested headlessly with `freya-testing` at that revision (cairn `Cargo.toml`).
32. Apps render on the GPU by default (B26).

**Open for the app packets**

- Whether rule 32 stays: the Settings packet measures an app's PSS and VRAM against acceptance bar 2
  (B26); a GPU process on NVIDIA starts at about 40 MB PSS of driver mappings (`ui-and-services.md`
  §1.4).
- The KDL shape of each app's section of `settings.kdl` (B27).
- Rule 8 needs a DBusMenu exporter in the fork: winit windows carry no native menu on Linux (strata
  `docs/PLATFORMS.md`).

## Features

### Settings

The user gets System Settings: one window holding every setting in panes, searchable, with back and
forward.

1. Every v1 page has a home (the 27 pages in inventory §5; Parity map).
2. Settings covers what B8 and B18–B21 create: the Dock and window snapping, the menu bar's
   status items and Control Centre modules, and per-device keyboard profiles with the Cmd translation
   exceptions and the Alt and Super swap, on by default for a PC-layout keyboard (B30).
3. Search covers every row's label and description on every pane; a result opens the pane, scrolls to
   the row and marks it (inventory §7 SettingsIndex).
4. The pane and row index is data installed with Settings, so the launcher finds a row while Settings
   is not running and choosing it starts Settings at that row (inventory §5).
5. Settings opens at a pane and row given by argument or D-Bus call; the menu bar, Control Centre,
   notifications and the CLI use it, and a second open raises the window at the new pane.
6. A change applies as it is made; there is no Apply button. Desktop settings go to the compositor
   (B10) and system settings to their daemon. A refused value stays on its row with the reason given.
7. When the compositor reports that `settings.kdl` does not parse, every pane says so and names the
   line (B10).
8. Back and forward are Cmd+[ and Cmd+] and the mouse's back and forward buttons (inventory §5).
9. While a shortcut is being recorded, or a keyboard detected, the compositor suspends desktop chords
   and Cmd translation so the pressed keys reach Settings; Escape or losing focus ends it (inventory
   §3.1 `isle-record`, §11; D58).
10. Key symbols are named as the keyboard's own layout produces them, asked of XKB (D61).
11. Provider panes (Passwords, VPN within Network, Snapshots, Phone) list the providers present, each
    with a switch, and draw each provider's state sentence and actions without knowing the tool
    (inventory §8; D42, D44, D45, D51).
12. Software Update runs a full upgrade only (D83) under pkexec, showing pacman's own per-package lines
    as progress (D47, D81). The run is not a child of Settings (D78): quitting Settings leaves it
    running and reopening shows where it is. A restart owed after kernel or NVIDIA module skew is
    stated until the restart (D46). Isle is a package built from git (B29), so its update is part of
   the same upgrade (inventory §7 IsleUpdate).
13. Displays: arranging the displays relative to each other; mode, scale, rotation, VRR and enabled
    per display; the last enabled display cannot be turned off; Redetect (inventory §3.7; D56).
14. Users & Groups: accounts through AccountsService; fingerprint enrolment beside the password and
    its PAM switches (D33, D34); automatic login for the greeter.
15. Live panes sample only while shown: the pad drawing (D54), sound meters, SMART.
16. Pairing prompts come from the desktop's Bluetooth agent (`org.bluez.Agent1`, B28; `platform.md`),
    so pairing from Control Centre works with Settings closed. Settings registers no agent.
17. Focus modes are set here, with a Gaming Focus that turns on for any detected game in front (B30).
    Game Mode has no Big Picture options; the controllers page stays (B22).

**v1 lessons kept.** D33, D34: fingerprint beside the password. D41: calendars are subscriptions. D42:
providers render themselves; secrets per field. D46: a restart owed is said. D47, D81: updates without a
terminal. D50: accessibility from what exists. D52: a font is set in one place. D53: session restore is
by app and desktop. D54: the controller pane shows, and sets only the dead zone. D56: redetect releases
outputs. D58, D61: render after devices load; ask XKB. D62: Settings reopens at its pane (rule 6 of
Conventions). D78: the update outlives its window. D83: no partial upgrades.

**Open for its packet.** Whether the Cmd translation exceptions are editable or read-only here. Where the periodic update check runs once Settings exits.

### Files

The user gets Finder: any number of windows, each with tabs; Recents, Favourites, Locations and saved
searches as places; Icons, List and Columns views; a preview; Quick Look; Get Info; Go to Folder. Files also draws the Open and Save panels every other app gets (B26).

**Engine**

1. A listing streams: rows arrive in batches from a worker while the view is already drawn and
   scrolling, and nothing is counted before it is drawn (D66).
2. A listing watches its folder for entries, writes and attribute changes, so a file being written
   shows its size and thumbnail as they change (D68).
3. Thumbnails are made on a worker into `~/.cache/thumbnails`, named by the URI's md5 and checked
   against the row's mtime from the PNG's text chunks; another app's are used as they are (D68).
4. Delete moves to the freedesktop trash of the item's own volume (home, `$mount/.Trash/$uid` or
   `$mount/.Trash-$uid`), writing the info file first. Trash shows every volume's trash at once, Empty
   Trash empties all after asking, and Put Back works after the volume mounts elsewhere (D70).
5. Nothing is removed before what replaces it is in place: a copy is written beside its destination and
   renamed over it, and a replaced folder is moved aside and dropped only afterwards (D70).
6. A job that meets an existing name stops and asks, with an answer for the rest of the job;
   cancelling is an answer (D70).
7. Undo is one step and covers what a job did even when it failed part way; a job that replaced
   something is not offered for undo (D70, D71).
8. Rename many takes one name with `#` for the number and appends one where there is none. Compress
   and Expand use bsdtar; expanding goes into a folder beside the archive, named after it (D71).
9. File operations are one crate shared with the Cleaner, so both trash, put back and undo alike.

**Places and search**

10. Favourites are the xdg-user-dirs folders and the bookmarks in `~/.config/gtk-3.0/bookmarks`, read
    and written as GTK writes them (D69); their order is the user's, dragged and kept by Files (D76).
11. Locations lists every volume UDisks2 reports, by name with its free space; mount, unmount and eject
    go through UDisks2 (B11), with polkit where it asks.
12. Recents is the shared recently-used list.
13. A search is a term, a scope (this folder, or everything under it), a kind and a date range. It runs
    on a worker and fills as it goes. A saved search is kept as a place and reopens the window as it
    was (D74).

**Views and actions**

14. Icons, List and Columns each show a folder's contents gathered by kind, date or size, and each
    opens folders in place (inventory §5). View, sort and grouping are remembered per folder, with a
    default for folders never set.
15. A preview of the selection can be shown in every view.
16. Keys are Finder's (`macos-model.md` §3 Finder row, §2.3 file-manager table): Cmd+↑ enclosing
    folder, Cmd+↓ open, Return rename, Cmd+Delete to Trash, Option+Cmd+Delete delete now after asking,
    Shift+Cmd+G Go to Folder, Shift+Cmd+. hidden files, Cmd+I Get Info, Cmd+C then Option+Cmd+V move,
    Cmd+1 to 3 views, Cmd+T, Cmd+N, Cmd+W, Cmd+[ and Cmd+], Shift+Cmd+N new folder, Cmd+D duplicate,
    Cmd+F search, Space Quick Look, Cmd+Z undo, and typing to select. Every function v1 put on a key
    has a key (`main:docs/AUDIT.md` § Files).
17. The mouse: click selects, Cmd-click adds, Shift-click extends, a band selects, a click on nothing
    deselects, a right click keeps a selection it is part of, a middle click opens a tab. A drag lands
    on a folder in any view, on a place (into it, or between two as a bookmark), on the Trash,
    and to and from other apps as `text/uri-list`.
18. Open With lists the apps for the type and can make one the default (`mimeapps.list`). Open in
    Terminal. Get Info shows kind, size, dates, location, permissions and Open With.
19. Files owns `org.freedesktop.FileManager1` and is the `inode/directory` handler (D73), so another
    app's "Show in folder" opens a Files window with the item selected.
20. An AppImage in `~/Applications` is an app: trashing it uninstalls it and putting it back
    reinstalls it. Its desktop entry is read from the squashfs at the end of the ELF sections, or
    through bsdtar for type 1, without running it (D82).

**Quick Look**

21. Space opens Quick Look on the selection; Space or Escape closes it; the arrows move the selection
    and the preview follows. It previews images, text and code, PDF, audio and video, fonts, archives
    (their listing) and folders (size and count). One Quick Look, in the Files process, serves Files
    windows and the chooser, and is a D-Bus service Spotlight calls, starting Files if it is not
    running (B26; `shell.md`; `macos-model.md` §3 Quick Look row).

**File chooser**

22. The portal backend, owned by the compositor's service worker, hands FileChooser to Files, which
    shows a chooser window, starting if it is not running (B13, B26; `platform.md`). The window is the
    requesting app's dialog, attached to its window. No chooser or Quick Look code loads into the
    compositor (B26).
23. The portal reply is held until the user accepts or cancels, and exactly one of the two answers it;
    closing the chooser, or Files ending, answers cancel, so no app waits forever (D67).
24. OpenFile (one or many, files or folders, filters as a type picker, choices), SaveFile (a name and
    a type in a folder) and SaveFiles (a folder, with the names joined on) are one chooser (D67).
25. The chooser has Files' places, views, search, Quick Look, New Folder, Go to Folder, hidden files
    and keys, and opens in the folder that app last used.
26. A chooser closes no shell surface (D67).

**v1 lessons kept.** D66: the listing streams. D67: one chooser, the reply held until answered. D68: the
shared thumbnail cache, validated from text chunks. D69: bookmarks are GTK's. D70, D71: per-volume trash,
replacement order, partial undo, `#` renames, bsdtar. D73: the folder handler reaches the running app.
D74: searches are narrowed and kept. D76: favourite order is the user's. D82: AppImages by folder, read
without running. D75, D92: one app id for every Files window.

**Open for its packet.** Whether undo becomes a stack, as Finder's is, or stays one step (D70). Tags,
which have no shared Linux standard (`macos-model.md` §3 Finder row). Connect to Server (SMB, NFS).
Content search and an index (`macos-model.md` OPEN, file indexing). A Gallery view. Who watches
`~/Applications` while Files is not running. Quick Look's D-Bus interface. How the chooser binds to
the requesting window (the portal's parent handle).

### Monitor

The user gets Activity Monitor: CPU, memory, energy, disk and network use over one process list,
inspection, Quit and Force Quit, plus the sensors and GPU figures a gaming machine needs.

1. One process table, seen by resource (CPU, memory, energy, disk, network), each with its own
   figures (inventory §5).
2. The list shows all processes, the user's, or windowed processes; a hierarchical view gives each
   process its descendants' totals (inventory §5). A windowed process is named and iconed from the
   compositor's window list (Conventions rule 19).
3. GPU use and memory, system-wide and per process, for every GPU present (the host has two,
   `ui-and-services.md` host context).
4. Sensors: temperatures, fans and GPU from hwmon and the GPU driver (inventory §5).
5. Energy shows power from RAPL and which processes hold sleep inhibitors, from logind (inventory §5).
6. Network shows traffic per process (inventory §5).
7. Quit sends TERM after asking, with Force Quit (KILL) offered; Stop and Continue; priority. Acting on
   another user's process goes through polkit (Conventions rule 25).
8. The update interval is chosen as in Activity Monitor; sampling stops while the window is hidden and
   ends with the process (Conventions rules 4, 24).
9. Cmd+I inspects a process: parent, user, command line, open files and sockets.
10. LACT and CoolerControl open from the window when installed (inventory §5).

**v1 lessons kept.** Inventory §5 Monitor: the five resources, the tree with totals, sensors, signals and
renice. Inventory §6 Polling: nothing samples unseen. D13: memory is measured on the host, never the VM.

**Open for its packet.** Per-process network without a capability-granted nethogs. How GPU figures are
read for both vendors.

### Keychain

The user gets the Passwords app with Keychain Access's reach: saved logins, Wi-Fi passwords, app
secrets, SSH keys and every password manager present, under one search.

1. Items come from the Secret Service through the shared client crate (`oo7`, `ui-and-services.md` §3),
   whether gnome-keyring 50 or oo7-daemon owns the bus name.
2. Items can be narrowed by kind (logins, Wi-Fi, browser, apps; inventory §5) and by password-manager
   provider (inventory §8; D42).
3. No secret is in memory until asked: lists hold titles and attributes, and reveal or copy fetches one
   field of one item at that moment (D42). A fetched value is zeroed after use and never logged.
4. A locked collection is unlocked through the Secret Service's Unlock, which brings up the
   compositor's keyring prompter (`org.gnome.keyring.SystemPrompter`, B12; `ui-and-services.md` §3);
   changing the keyring password goes the same way. The Keychain is a client: it draws no keyring
   dialog and serves no prompter. A fingerprint login leaves the keyring locked, so
   this prompt follows it (`macos-model.md` §3 Touch ID row).
5. A copied secret is marked with the `x-kde-passwordManagerHint` type so clipboard history skips it,
   and is withdrawn after a set time unless something else has been copied since.
6. Items can be edited and deleted; delete asks.
7. A generator makes passwords by length and character set (inventory §5).
8. SSH keys: each key in `~/.ssh` with its type, fingerprint, comment and whether the agent holds it;
   add to and remove from the agent; generate with a passphrase (inventory §5; the gcr agent, §3.1). A
   passphrase reaches ssh-keygen or ssh-add without appearing in an argument list or a file others
   can read.
9. A provider's sign-in, unlock and actions happen in the Keychain, as the provider names them (D42,
   D20).
10. Wi-Fi passwords of saved networks come from NetworkManager's own secrets call, which polkit gates.
11. Revealed values hide again after a period idle and when the window closes.

**v1 lessons kept.** D42: one vault behind providers; titles without secrets; one field on demand. D20:
a provider's sign-in is its own CLI flow. Inventory §5: categories, generator, SSH keys with agent state.

**Open for its packet.** What gates revealing a secret beyond an unlocked keyring (macOS asks per
reveal). Passkeys, which wait for a credential portal (D42). Certificates. Whether browsers' own
Secret Service entries are shown.

### Cleaner

The user gets what Broom and DaisyDisk give a Mac (D87, D88): where the space went, what can go
safely, duplicates, large and untouched files, removed through a basket with one undo.

1. The user sees space by kind, hotspots and largest items, as a map or a list; what can go by rule,
   safe or worth a look, and the system's share; duplicates; large files; untouched files
   (inventory §5).
2. The window opens on All disks: one scan over every mounted disk under one root, dropping a root
   another already reaches (D90).
3. The scan is a pool of workers reading one folder each. It keeps folders (own bytes, file count,
   totals, bytes by kind) and the 500 largest files; sizes are allocated blocks; a file with several
   names counts once; it crosses into mounts of the same device and filesystem and nowhere else, and
   never enters `.snapshots` (D87).
4. The disk's own used and free figures stand beside the scan's total (D87).
5. Ten fixed kinds, guessed from folder names first and file endings only where no folder says; a
   file has the same kind everywhere (D88).
6. Cleanup is rules over the scanned tree, in memory. The general rule is every entry in `~/.cache`
   and every Flatpak app's `cache`, less listed exceptions (model weights, shader caches, clipboard
   history). Named rules cover package stores, Electron caches in `~/.config`, build folders while
   their project file sits beside them, and Steam leftovers found by a missing `appmanifest`. A named
   rule wins over the general one. The user's rules file is read beside the shipped one (D89).
7. What is safe to clear is selected by default; what is worth a look never is (D89).
8. The system's share is the package cache, orphaned packages, the journal and crash dumps, cleared
   through their own tools under pkexec, one authorisation each, and said not to come back (D89).
9. Duplicate candidates are at least 1 MB, not executable, outside hidden folders and git working
   trees, and claimed by no kind rule; they match by size, then both ends, then the whole content;
   copies sharing blocks through a reflink are counted apart (D90).
10. Removal is a basket: its total shows before anything moves, one job trashes it all through the
    shared file operations (Files rule 9) and one undo puts it all back. On the same volume the window
    then says nothing is freed yet and offers Empty Trash, which asks (D88, D70).
11. Closing the window cancels a scan; a removal in progress keeps the process until it ends
    (Conventions rule 4).

**v1 lessons kept.** D87: scan once, keep folders, never `.snapshots`. D88: by kind, through a basket.
D89: rules as data, XDG cache first, root's share by its own tools. D90: own files only, all disks.

**Open for its packet.** The rules file's format (inventory §5 `rules.json`, or KDL as B10 uses).

## For the design process

What this document leaves to P1, per app; "proposal" is the answer it gave before.

- **Conventions**: the menu set and order (proposal: app menu with About, Settings…, Hide, Hide
  Others, Quit, then File, Edit, View, Window, Help as needed); the unified toolbar's contents and look;
  how a modal dialog is presented (proposal: a sheet on its window).
- **Settings**: the window's layout (proposal: a sidebar of panes, search at its top, the user's
  account above the panes, each pane a list of grouped rows); the pane list, names, order and grouping
  (proposal: each takes the name and place of the macOS pane that covers it, an Isle-only pane beside
  its nearest macOS neighbour; the names in the Parity map); whether Focus is a pane of its own
  (proposal: yes, as on macOS) and whether Game Mode's settings fold into Gaming Focus or keep a pane;
  how a found row is marked; how the Displays arrangement is drawn (proposal: to scale, snapping on drop).
- **Files**: how places are laid out (proposal: a sidebar of Recents, Favourites, Locations and saved
  searches); where the preview sits (proposal: a pane beside the folder, the last column in Columns);
  the toolbar's contents; how a file job's question and progress are shown.
- **Monitor**: how resources are switched (proposal: five tabs, CPU, Memory, Energy, Disk, Network).
- **Keychain**: how kinds and providers are laid out (proposal: categories Logins, Wi-Fi, Browser,
  Apps, then one section per provider).
- **Cleaner**: its views and their names (proposal: Overview, Cleanup with Safe to clear, Worth a look
  and a System card, Space as a treemap or list, Duplicates, Large files, Untouched); how a kind is
  shown (proposal: the same way in every view, D88); whether Cleanup's findings also show in Settings'
  Storage pane, as macOS shows Recommendations.

## Acceptance bars

Measured on the host, not the VM (D13).

1. **Cold start.** With a warm cache each app draws its first complete frame within 300 ms of launch,
   and the chooser is on screen within 300 ms of the portal request when Files is not running.
2. **Idle memory.** Each app with one window, idle for 30 s, is at most 80 MB PSS by `smaps_rollup`,
   with its VRAM from `nvidia-smi` reported beside it (`ui-and-services.md` OPEN gives the protocol;
   D92 measured a bare Freya window at 51 MB private).
3. **Nothing when closed.** Five seconds after an app's last window closes with no work in flight, its
   process is gone.
4. **Responsive under failure.** With any daemon an app uses stopped or frozen with SIGSTOP, the app
   opens, every window answers input within a frame, and the affected rows say what is missing.
5. **Menus.** In every app the menu bar shows the app's own menus, never the fallback, and the command
   table test of Conventions rule 10 passes.
6. **Cmd.** Every shortcut an app's menus show works with Super as Cmd on a Mac-profile keyboard and on
   a PC keyboard, and no Ctrl chord triggers a Cmd command.
7. **Containment.** `kill -9` of any app leaves the session, the shell and the other apps running; the
   app relaunches at once; a portal request it held answers cancel.
8. **Live settings.** A hand edit to `settings.kdl` or `tokens.kdl` shows in every open window within a
   second, without reopening it.
9. **Large folder.** A folder of 50,000 entries draws its first screen before its listing completes and
   scrolls without a dropped frame while it fills.
10. **Scan.** The Cleaner scans the host's home at least as fast as D87 records for the same tree.
11. **Parity.** Every row of the Parity map is present, or its reshape is recorded in its packet with
    the reason.

## Parity map

Every v1 window capability (inventory §5, §4 File chooser, `main:docs/AUDIT.md` § Files) and where it
lands. Pane names are proposals; P1 settles them (For the design process).

| v1 capability | Lands in |
|---|---|
| Settings › Network (Wi-Fi, VPN providers) | Wi-Fi and Network panes, VPN as providers (Settings 11); Wi-Fi passwords through the desktop's secret agent (B12) |
| Settings › Bluetooth (pairing codes) | Bluetooth pane; pairing prompts from the desktop's agent (Settings 16; B28) |
| Settings › Phone, Passwords, Snapshots | Panes of their own, each a provider list (Settings 11; D42, D45, D51) |
| Settings › Appearance (theme, auto by sun, accent, fonts, title bars, reduced motion) | Appearance pane, written through the compositor (B10); title bars are other apps' (B25) |
| Settings › Wallpaper, Audio, Mouse, Printers, Date and time, Region, About | Wallpaper, Sound, Mouse and Trackpad, Printers, Date & Time, Language & Region, About |
| Settings › Accessibility (magnifier, text scale, high contrast, on-screen keyboard) | Accessibility pane |
| Settings › Notifications (do not disturb, quiet hours, per-app rules) | Notifications pane; do not disturb and quiet hours as Focus (Settings 17; B30) |
| Settings › Modes (game auto, tearing) | Game Mode settings; game auto as Gaming Focus (Settings 17) |
| Settings › Modes (pad home, Steam shortcuts, gamescope, Steam follows) | Dropped with Isle's Big Picture (B22; `gaming.md` parity map) |
| Settings › Apps (session restore, keep running, autostart, user services, start commands, default apps) | Desktop & Dock (session restore, B20's keep-running policy), Login Items, default apps |
| Settings › Keyboard (per-device profile, Detect, layouts, XKB options) and Shortcuts (chord recording) | Keyboard pane (Settings 9, 10); profiles applied by the compositor (B21) |
| Settings › Displays (arrangement, mode, scale, VRR, redetect) | Displays pane (Settings 13) |
| Settings › Controllers (live pad, dead zone) | Controllers page (D54; kept by B22) |
| Settings › Power (profiles, batteries, idle dim, lock, screens off) | Energy pane; idle timers in Lock Screen |
| Settings › Storage (drives, SMART) and Devices (device tree) | Storage in General; About, as a system report |
| Settings › Users (accounts, fingerprint, PAM switches, autologin) | Users & Groups (Settings 14) |
| Settings › Calendars (subscriptions) | Internet Accounts or Calendars pane (D41) |
| Settings › Updates (system, AUR, Isle, restart owed) | Software Update (Settings 12) |
| Settings search, launcher entries, back and forward | Settings 3, 4, 8; the launcher in `shell.md` |
| Files windows, tabs, sidebar, saved searches | Conventions 3; Files 10–13; the sidebar's layout is P1's |
| Files List, Columns, Grid, disclosure, grouping, sorting, widths, preview, Quick Look | Files 14, 15, 21 |
| Files drag and drop, keyboard map | Files 16, 17 |
| Files copy, move, trash, put back, empty, delete, undo, rename many, pack, unpack | Files 4–8 |
| Files open with, set default, open in terminal, info; folder handler `isle-files.desktop` | Files 18, 19 |
| Files mount, eject; AppImage install by folder | Files 11, 20; the drive-inserted notification in `shell.md` |
| File chooser surface and portal reply | Files 22–26, in the Files process (B26); the backend in `platform.md` |
| Monitor tabs, process tree, sensors, GPU, energy, inhibitors, per-process network | Monitor 1–6; tabs are P1's |
| Monitor STOP, CONT, TERM, KILL, renice; LACT and CoolerControl | Monitor 7, 10 |
| Keychain categories, reveal, copy, delete, generator, providers | Keychain 2, 3, 6, 7, 9 |
| Keychain SSH keys and agent | Keychain 8 |
| Cleaner Overview, Space, Large files, Untouched, disk cards | Cleaner 1, 3–5; views and cards are P1's |
| Cleaner Cleanup, System card, Duplicates, basket, undo, Empty Trash | Cleaner 6–10 |
| Windows reopened after a shell reload (D62) | Conventions 6; apps outlive a shell restart (B9) |
| One app id per shell window (D92, D75); raise, don't relaunch (D63) | Conventions 1, 3 |
| Shell windows float, centred (inventory §3.3) | The compositor's window model (B8) |
| App preferences, window places, launch counts | Imported once (Conventions 27; B27) |

## Not here

- The compositor's D-Bus interface for settings, tokens, the window list and chord capture; the portal
  backend; the polkit agent, keyring prompter, Wi-Fi secret agent, Bluetooth pairing agent and
  notification server: `platform.md`.
- The menu bar, its fallback menu and menu search; the Dock; the launcher and its Settings and file
  results; Control Centre; the polkit, keyring and pairing dialogs; clipboard history; live activity:
  `shell.md`.
- Server-side title bars for other apps (B25): `compositor.md`.
- How any window looks and is laid out: P1, the design process (B3; For the design process).
- The Cmd translation tables for other apps (B21) and the theme templates for installed apps (B10):
  their own packets.
- The provider contract and its scripts (inventory §8), and dashboard widgets (B14).
- Build order, phases and estimates: the roadmap.
