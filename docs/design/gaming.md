# Gaming

Intent document for the rust-desktop program. Nothing here is built yet.

Isle is built for playing games as much as for work. It recognises a game
without being told, and while one is in front it gets out of the way: the
game reaches the screen from its own buffer, drives the refresh rate, may
tear if the user allows it, receives every key as pressed, and sees no shell.
Around that, Isle is the system layer, not the store (D38): Steam, Heroic
and Lutris keep their libraries, updates and saves, and Isle adds
full-screen Spaces, Game Mode and controllers. Steam's Big Picture is the
console experience; Isle has no console mode of its own (B22). Display
mechanics belong to `compositor.md`, shell surfaces to `shell.md`, Settings
pages to `apps.md` and service crates to `platform.md`; this document says
what gaming requires of them. How any of it is presented is the design
process's (P1), whose open questions are under For the design process. Evidence is cited by file
under `docs/research/rust-desktop/`: `inventory` (isle-v1-inventory.md),
`gaming` (smithay-and-gaming.md), `macos` (macos-model.md) and `services`
(ui-and-services.md). `Dn` is v1's `main:docs/DECISIONS.md`.

## Rests on

- B2: every v1 gaming capability is in scope, re-designed rather than ported.
- B3: no presentation here; the look and layout are P1's.
- B5: Steam and most games are X11, handled by Isle's own Xwayland WM.
- B6: tearing comes from the async page flip patch on Isle's Smithay fork; VRR covers the gap.
- B7: shell surfaces are Freya, drawn by the compositor.
- B8: full-screen Spaces, real minimise, remembered places.
- B9: nothing blocking on the main loop; pads and libraries are read on the service worker.
- B10: game settings live in `settings.kdl` and are served over Isle's D-Bus interface.
- B11: GameMode, UPower and power profiles are zbus service crates.
- B12: notifications are Isle's own server, so Isle decides when they show.
- B17: a Mac user's expectations decide what is open: Game Mode, full-screen Spaces.
- B18, B19: the island at rest and the menu bar on demand, both hidden over a game's Space; no Dock.
- B20: a game is an app; Cmd+Tab and Quit speak in apps.
- B21: no Cmd translation in game windows or under shortcuts-inhibit.
- B22: no console mode of Isle's own; Steam's Big Picture is the console experience.
- B23: HDR is its own packet after Isle replaces v1; games get it then.
- B24: where banners, live activities and OSDs appear is P1's; Game Mode decides which show over a game.
- B30: Game Mode is the Gaming Focus, on for any detected game in front; a fixed escape set; VRR while a full-screen app is in front.

## Features

### Games

The user gets: Isle knows which windows are games, and the user can correct it.

1. A window is a game window when its process, or an ancestor of it, is registered with GameMode (`com.feralinteractive.GameMode`); when its app id or X11 class is `steam_app_<n>`, `steam_proton`, a Wine `*.exe` class or `gamescope`; or when its process descends from Steam, Heroic or Lutris and is not the launcher's own UI.
2. The user can mark an app as a game or as never a game, and that choice wins over rule 1.
3. Isle decides at map time and again when GameMode registers or unregisters a process; it never polls.
4. Isle maps a window to its process through the Wayland client's credentials, or through its own XWM for X11 windows (B5).
5. A window is in front when it has keyboard focus and is the top window of the visible Space on its output.
6. Steam's Big Picture window counts as a game window; Steam's desktop client and Heroic's and Lutris' own windows do not.
7. A game window is never given a remembered place and its place is never recorded.

v1 lessons kept: inventory §3.8 (GameMode or launcher class, both needed: not every game registers); D48 (a borderless game is a plain window the size of the screen, so Isle must recognise it by more than fullscreen state); inventory §3.4 (window places emptied in game mode).

Open for its packet: whether `*.exe` matching catches Wine installers and launchers that are not games; process mapping for Flatpak Steam (macos OPEN on Flatpak PIDs).

### Games as apps

The user gets: the switcher, Spotlight, Mission Control and per-app volume show a game's name and cover, never `steam_app_1245620`.

1. A `steam_app_<id>` window takes that app's name and art from the Steam library.
2. A gamescope window takes the name of the library game running under it, otherwise the first word of its title.
3. A Heroic or Lutris game takes its name and art from that launcher's library.
4. A game is one app for B20: Cmd+Tab lists it once and its audio streams group under it.
5. The menu bar shows the fallback app menu for a game, since games export none (B18).
6. Quit asks the game to close its windows; Isle never kills a game unless the user chooses Force Quit.

v1 lessons kept: inventory §3.6 Apps (`steam_app_<id>` named from the library, gamescope from its title); D59 (the switcher lists apps, so the app's name is what the user reads).

Open for its packet: where current Steam keeps library art and in what form; how to find the game under a gamescope window that Isle did not start (B22).

### Game library

The user gets: one list of installed games across Steam, Heroic and Lutris, with art, from which every game on the desktop is named.

1. Isle reads what each launcher keeps (Steam's app manifests and library cache, Heroic's store cache, Lutris' database and art) and writes none of it.
2. Isle launches a game through its launcher (`steam://rungameid/<id>`, `heroic --no-gui heroic://launch/…`, `lutris lutris:rungameid/…`), never by running its binary.
3. Libraries are read on the service worker and re-read when their files change.

v1 lessons kept: D38 (reimplementing a launcher's library never catches up); inventory §3.8 Launching.

Open for its packet: library paths for Flatpak installs of each launcher.

### Full-screen Spaces for games

The user gets: as on a Mac, a full-screen game has a Space of its own; Cmd+Tab to a browser shows the browser, not a window behind the game, and the game is listed among the desktops by name.

1. A game window that goes full screen, or covers its output without decorations, moves to a new full-screen Space, and the view follows it when it has focus.
2. A borderless game gets its Space without being put into the compositor's fullscreen state, so its own resize back to windowed is honoured.
3. When full screen ends the window returns to the Space it came from; when the game closes, the view returns to the Space the user was on.
4. Leaving a game's Space releases its pointer lock or confinement; returning restores what the game asked for.
5. The game's Space shows no island or menu bar unless revealed (Game Mode).

Requires of `compositor.md`: full-screen Spaces as a window state; borderless detection by geometry. Of `shell.md`: game Spaces among the desktops in Mission Control.

v1 lessons kept: D48 (a fullscreen game on a shared Space left alt-tab focusing windows it covered; forcing compositor fullscreen on a borderless game broke its own resizes).

Open for its packet: whether a game whose Space is not visible is throttled, and whether games mind; X11 games that change resolution through Xwayland's emulated RandR modes.

### Game Mode

The user gets: Game Mode is the Gaming Focus (B30). Like a Mac's Gaming Focus it comes on by itself when a game is in front, says so, and everything else steps back.

1. Game Mode is on while a game window is in front, full screen or windowed (B30), or while the user has turned the Gaming Focus on from Control Centre or its shortcut; a manual on stays until turned off.
2. Turning the Gaming Focus's automatic start off in Settings leaves only the manual toggle. Its allowed apps are set like any Focus's.
3. On entering, Isle holds the performance power profile (power-profiles-daemon `HoldProfile`) and turns night light off; on leaving it releases the hold and restores night light.
4. Leaving restores only what Game Mode changed; a change the user made during Game Mode stands.
5. The island and the menu bar stay hidden over a game's Space; resting the pointer at the top edge reveals the island, or outside the island's zone the menu bar with it, never while the game holds the pointer locked or confined.
6. Notification banners are held and counted, except critical ones and the Focus's allowed apps; on leaving, one summary of what was held leads to Notification Centre.
7. Over a game's Space no live activity shows except privacy indicators, until the island is revealed. OSDs still show briefly without revealing the island or the menu bar.
8. No dim, lock or display sleep happens while a game window is in front.
9. A shell surface shown during Game Mode opens and closes without transitions or background blur. With a windowed game in front the shell stays shown and works as normal, drawn this way (P1, gate 2).
10. Game Mode's state is readable and settable over Isle's D-Bus interface (B10).

Requires of `shell.md`: the Gaming Focus among the Focus modes, a visible Game Mode state, the reveal, held banners and the summary. Of `platform.md`: the power profile hold, idle inhibition.

v1 lessons kept: inventory §10 (the mode's profile and the summary on leaving); inventory §4 Hot zone (a peek at the top edge); inventory §10 (leaving reloaded the whole compositor profile from files instead of undoing what the mode changed).

Open for its packet: whether games and Steam inhibit idle through `org.freedesktop.ScreenSaver` (served by Isle, `platform.md`) or rely on rule 8 alone.

### Keys and pointer in games

The user gets: a game receives keys as pressed. Ctrl is crouch, not copy, and only a short escape set of Isle's shortcuts survives (B30).

1. Isle translates no Cmd chord in a game window (B21), whether or not Game Mode is on.
2. While a game window is in front, Isle's desktop shortcuts shrink to a fixed escape set: Cmd+Tab, Mission Control, lock and screenshot (B30). Every other chord, Force Quit and the Game Mode shortcut among them, and Super alone reach the game.
3. Alt chords, Alt+Tab among them, reach the game: on a Mac-profile keyboard Alt is the Option key; on a PC-profile keyboard the Alt and Super swap is suspended in game windows, so Alt is the key labelled Alt (B30).
4. Modifiers held when focus leaves a game are released to it first (B21).
5. Keys Isle injects (the on-screen keyboard) are its own source and never translated (B21).

Requires of `compositor.md`: the escape set in the keyboard filter.

v1 lessons kept: macos §2.4 (Ctrl must reach games as Ctrl); macos §2.5 (games read keys from the compositor, so translating nothing is the whole guarantee).

Open for its packet: whether the escape set survives a game's keyboard-shortcuts-inhibit or Xwayland keyboard grab, so a hung game cannot trap the session.

### Presentation: direct scanout, VRR, tearing

The user gets: a full-screen game reaches the screen untouched: its own buffer scanned out, the refresh following its frame rate, and tearing when the user allows it.

1. A full-screen game in front with nothing of Isle's over it is scanned out from its own buffer; Isle composites nothing.
2. Anything Isle shows over a game costs composition only while it is up; the game returns to scanout when it goes.
3. VRR is set per output: Off, Full screen (the default) or Always. Full screen turns VRR on while any full-screen window is in front, game or not (B30).
4. With "Allow tearing in games" on, a full-screen game in front that asks for async presentation through wp-tearing-control-v1 is flipped asynchronously; Xwayland forwards an X11 game's request (gaming §2.3).
5. Only frames scanned out directly tear; a composited frame waits for vblank.
6. Game buffers are synchronised through linux-drm-syncobj, Xwayland's included.
7. Native Wayland games get wp-fifo-v1, wp-commit-timing-v1 and wp-presentation-time.
8. A game's frame never waits on shell work, and Isle never waits for its own GPU work to finish before queueing the game's frame.
9. Starting, leaving or switching away from a game never causes a modeset.

Requires of `compositor.md`: plane assignment, the async flip patch (B6), on-demand VRR, explicit sync, frame scheduling.

v1 lessons kept: inventory §3.1 and §3.3 (VRR on, direct scanout on, tearing only for the fullscreen game and only while allowed).

Open for its packet: async flips on Blackwell with 615.71.09 (only an RTX 4060 mobile is reported, gaming OPEN); direct scanout of game buffers on NVIDIA through `DrmCompositor` (gaming OPEN); whether cosmic's NVIDIA overlay-plane quirk still holds on Blackwell (gaming OPEN); whether NVIDIA's Vulkan WSI uses fifo and commit-timing (gaming OPEN); cursor-only motion under VRR (niri's flag, gaming §2); whether the host monitor flashes when VRR toggles; whether to tear for a game that does not ask, as v1's `immediate` rule did; the tearing setting's default.

### Steam's desktop client

The user gets: Steam behaves like any other app: menus where clicked that stay open, a window that minimises, no stray windows.

1. Steam's menus and popups, override-redirect windows or untitled ones that turn override-redirect after mapping, open where Steam asks, with no decoration, animation or focus change, and are never noted or placed.
2. A Steam menu stays open until the user picks an item, clicks outside it or presses Escape.
3. Steam's own title bar moves and resizes its window (`_NET_WM_MOVERESIZE`).
4. Closing Steam's window leaves Steam running in the tray; Quit asks Steam itself to exit (`steam -shutdown`).
5. Cmd translation applies in Steam's client; the menu bar shows the fallback menu (B18).

v1 lessons kept: D40 and inventory §3.8 (a month of misrendering was Isle placing Steam's untitled menus); inventory §3.4 (X11 windows cannot be moved by their own controls unless the WM answers `_NET_WM_MOVERESIZE`).

Open for its packet: whether Steam's "context menu focus compatibility mode" is ever needed under Isle's WM (gaming §1.4, satellite #491).

### Steam's Big Picture

The user gets: Steam's Big Picture takes the screen like a console; closing it puts the desktop back as it was, Steam's window included.

1. Steam's Big Picture is a window of class `steam`, or a gamescope window hosting Steam, whose title contains "Big Picture" and which is not minimised; title changes do not lose it.
2. It gets a full-screen Space of its own, and the Space it opened from is remembered.
3. Steam minimises Big Picture rather than closing it, so a minimised Big Picture counts as closed: its Space goes, the view returns, and the switcher and App Exposé do not show it as minimised.
4. Steam shows its desktop window again on whatever Space is current when Big Picture closes; Isle moves it back to the Space it was on before.
5. Steam's Big Picture is a full-screen game: Game Mode's rules apply while it is in front.
6. Steam Input owns the pad while Steam's Big Picture is in front.

v1 lessons kept: inventory §3.8 and v1 ARCHITECTURE § Big Picture (minimise-not-close, the client window reappearing on the wrong workspace, the title flipping).

Open for its packet: Big Picture on Smithay compositors has no current reports (gaming OPEN); how Steam asks to be minimised under Isle's WM; whether the title still says "Big Picture".

### gamescope

The user gets: gamescope runs nested as a window or full screen with VRR, and Steam can run inside it.

1. Isle provides every global gamescope's Wayland backend requires (`wl_compositor` v4, `wl_subcompositor`, `xdg_wm_base`, `zwp_linux_dmabuf_v1` v3+, `wp_viewporter`, `wp_presentation`, relative pointer, pointer constraints, `wl_shm`) and the optional single-pixel buffer, fractional scale, toplevel icon, data device and primary selection (gaming §2.2).
2. A gamescope window is a game window named after what it hosts.
3. Nested gamescope never tears, so its latency rests on VRR and Isle's scheduling.

v1 lessons kept: inventory §3.8 Launching.

Open for its packet: nested gamescope on NVIDIA under Smithay compositors (gaming OPEN: a dmabuf import failure, a 60 Hz refresh guess, keyboard latency); where gamescope reads the host's refresh from.

### Heroic and Lutris

The user gets: their games are games and their windows are apps.

1. Heroic's and Lutris' own windows are apps; games they start are game windows by ancestry.

### Controllers

The user gets: Isle knows every pad: its name, connection, battery, and whether it is real or Steam's virtual one, with a live test page.

1. A pad is an evdev device with `BTN_GAMEPAD` and an axis; a remapper's virtual keyboard is not.
2. A pad under `/sys/devices/virtual` came through uinput and is named by its origin: Steam Input for Valve's id, otherwise the program holding `/dev/uinput`.
3. Battery comes from UPower and is shown wherever the shell shows batteries.
4. Isle reads pad input on the service worker only while the Controllers page is open; presence and battery come from device events and UPower.
5. A pad that connects or disconnects appears or goes within a second.
6. Settings › Controllers draws each pad's live state; mapping and dead zones belong to the game and Steam Input, since Isle has no pad navigation of its own (B22).
7. The Steam Controller's Puck (28de:1304) has no kernel gamepad driver, so outside Steam it is a keyboard and mouse; with Steam running and a gamepad desktop layout, Steam Input presents it as an Xbox 360 pad, which Isle reads.

Requires of `apps.md`: the Controllers page. Of `platform.md`: UPower batteries.

v1 lessons kept: D54 (a pad is named by where it comes from, and Isle sets only what it uses, now nothing); D9 (the Steam Controller is a keyboard and mouse outside Steam); inventory §7 Gamepad (reading only while needed).

Open for its packet: device access through logind uaccess on CachyOS (services OPEN); `gilrs` or `evdev` (services §3); duplicate pads while Steam Input mirrors a real one.

## For the design process

What this document leaves to P1, per feature; "proposal" is the answer it
gave before.

- **Games as apps**: how a game's art stands in for an icon in per-app
  volume (proposal: its cover). In the switcher, Spotlight and Mission
  Control it is answered in P1
  (`docs/work/rust-desktop/p1/structure/switcher.md`, `dock.md`) and
  recorded in DESIGN.md at gate 6.
- **Full-screen Spaces**: how a game's Space is shown among the desktops
  (proposal: by name in Mission Control's Spaces bar).
- **Game Mode**: how the top-edge reveal looks; the held-notification
  summary's form and copy (proposal: one summary, "while you played",
  leading to Notification Centre); where held banners would have shown
  (proposal: top right, B24). Where its state is shown (the island's Focus
  glyph), the reveal's structure and the shell beside a windowed game are
  answered in P1 (`docs/work/rust-desktop/p1/structure/island.md`,
  `menu-bar.md`, `competition.md`) and recorded in DESIGN.md at gate 6.
- **Controllers**: where pad batteries show (proposal: Control Centre and
  the Devices widget); the names given to virtual pads (proposal: "Steam
  Input", "Virtual pad from" the program).

## Acceptance bars

Measured on the host (RTX 5070 Ti, the 3440×1440 48–144 Hz monitor) unless the bar says the VM.

1. A full-screen game, both a Proton game under Xwayland and a native Wayland Vulkan client, is scanned out from its own buffer on every frame of a ten-minute run with nothing of Isle's on screen, as the compositor's plane trace shows.
2. With tearing allowed, a game asking for async presentation gets async flips at an uncapped frame rate (trace and a visible tear in a test pattern); with tearing off, no frame tears.
3. With VRR at Full screen, a game holding a steady rate between 48 and 144 fps is presented at that rate (presentation-time intervals within a millisecond of its frame interval), and the output leaves VRR when the game leaves the front.
4. While a game is in front and nothing of Isle's is visible, the shell's surfaces draw no frames and the service worker runs no polling that only feeds hidden surfaces.
5. With the GPU at full load, an input event reaches the focused game within one millisecond of the compositor receiving it at the 99th percentile.
6. An hour of play logs no main-loop stall over a frame's budget (B9's watchdog).
7. Starting, leaving and Cmd+Tabbing out of a full-screen game never blanks the screen.
8. In a game window, Cmd+C and Ctrl+C reach the game as pressed, checked with an input-logging client marked as a game.
9. Every context menu in Steam's Library, Store and Friends views opens at the pointer and stays open until dismissed.
10. Opening and closing Steam's Big Picture twenty times leaves no stray window, returns the view each time, and puts Steam's window back on its Space.
11. Nested gamescope starts with no missing global and reports the output's refresh rate.
12. Xwayland on NVIDIA binds linux-drm-syncobj.
13. Thirty minutes of pad-only play in a game never dims or locks.
14. Controller detection, naming and battery pass in the VM with a uinput pad.

## Parity map

| v1 capability (inventory) | Lands in |
|---|---|
| Game detection by GameMode and by launcher app id (§3.8) | Games |
| Automatic game mode setting (§3.8 `gameAuto`) | Game Mode (the Gaming Focus's automatic start, any detected game, B30) |
| Game mode toggle: chord and panel tile (§10, §4) | Game Mode (the Gaming Focus in Control Centre, its shortcut) |
| Mode profile: animations, blur, gaps, borders off (§10) | Game Mode; full-screen Spaces |
| Tearing allowed for the fullscreen game (§3.3, §10) | Presentation |
| VRR on, direct scanout on (§3.1) | Presentation (VRR for any full-screen window, B30); compositor.md |
| Night light off and power profile to performance, both restored (§10) | Game Mode |
| Island unmapped, top-edge hot zone peek (§4, §10) | Game Mode 5, 7 (only privacy indicators and OSDs over a game; a top-edge dwell reveals the island, or the menu bar with it) |
| Notifications silenced, counted and summarised (§4, §10) | Game Mode 6 (banners held by the Gaming Focus); shell.md |
| Idle inhibited (§10) | Game Mode |
| Window places off in game mode (§3.4) | Games |
| Motion off (§9, §10) | Game Mode |
| Fullscreen and borderless get their own workspace (§3.6, D48) | Full-screen Spaces |
| Steam's menus left to the WM and styled as popups (§3.3, D40) | Steam's desktop client |
| `_NET_WM_MOVERESIZE` for X11 title bars (§3.4) | Steam's desktop client; compositor.md |
| Big Picture detection, parking, title flips, unparking (§3.8) | Steam's Big Picture |
| Mode follows Steam both ways (§3.8) | Dropped (B22): no Isle mode to follow |
| Steam under gamescope (§3.8) | Dropped (B22): it was launched from Isle's home |
| Heroic console mode and Lutris; headless launches (§3.8) | Heroic and Lutris (apps and games); console mode and headless launches dropped (B22) |
| Library and art from Steam, Heroic, Lutris (§3.8 `games.py`) | Game library |
| `steam_app_<id>` and gamescope naming with covers (§3.6) | Games as apps |
| Per-app volume rows with the game's cover (v1 ARCHITECTURE) | Games as apps; shell.md |
| Big Picture mode as a desktop state (§10) | Dropped (B22) |
| Big Picture home with search and pad legend (§4) | Dropped (B22) |
| Quick menu (§4) | Dropped (B22) |
| Guide tap and hold, Select hold, pad opens home (§10) | Dropped (B22) |
| Pad bridge for listed apps (§10) | Dropped (B22) |
| `shortcuts.vdf` entries (§3.8) | Dropped (B22) |
| Gamepad helper reading evdev (§7) | Controllers |
| Controllers page, live state, virtual origin (§5, D54) | Controllers; apps.md |
| Dead zone for Isle's pad navigation (§5) | Dropped (B22): no pad navigation |
| Controller battery (v1 ARCHITECTURE) | Controllers |
| Steam Controller handling (v1 ARCHITECTURE) | Controllers |
| On-screen keyboard driven by pad (§4) | shell.md (the keyboard); pad driving dropped with the console mode (B22) |
| Settings › Modes (§5) | apps.md, settings named here |
| IPC `modes`, `gamepad` (§13) | Isle's D-Bus interface (B10); `bigpicture` dropped (B22) |

## Not here

- Display mechanics (plane assignment, the async flip patch, VRR plumbing, frame scheduling, the NVIDIA VRAM profile): `compositor.md`.
- The island and its live activities, the menu bar, Control Centre, notification banners, Focus modes other than Gaming, the on-screen keyboard and Mission Control as surfaces: `shell.md`.
- The escape set's chords in the keyboard filter and the per-device Alt and Super swap (B30): `compositor.md`.
- Settings pages: `apps.md`. GameMode, UPower and power-profile crates: `platform.md`.
- HDR and colour management: in scope by B23 as their own packet after Isle replaces v1; the display side is `compositor.md`. Games, nested gamescope included, get HDR when it lands (Proton-CachyOS enables it on its own, gaming §2).
- DRM leasing for VR headsets.
- A store, library management, updates or cloud saves (D38).
- Pad mapping or dead zones inside games (D54).
- A console mode of Isle's own (home, quick menu, Guide, pad bridge, entries in Steam), dropped by B22, and a console-only session such as gamescope-session.
- Presentation (P1, For the design process), build order and estimates.
