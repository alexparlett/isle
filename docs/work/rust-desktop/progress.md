# Progress

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
  after a drag. Not done here: the VM and the host.

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
