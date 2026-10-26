# Shell

Intent document for the rust-desktop program. Nothing here is built yet.

The shell is every always-on surface Isle draws, from the island to the lock screen. Each is a Freya
tree drawn by the compositor on its own GL context (B7), fed by services on the worker thread (B9).
This document says what each surface must make possible and which rules every surface obeys; how
they look, where they sit, what they show and in what order are the design process's (P1; B3, B24),
whose open questions are listed under For the design process.

Evidence, cited by file: `isle-v1-inventory.md` (written "inventory §n"; its D-numbers are v1's
`main:docs/DECISIONS.md`), `macos-model.md`, `design-v1-lessons.md`, `shell-ui-spike.md`,
`ui-and-services.md` and `smithay-and-gaming.md`, all in `docs/research/rust-desktop/`.

## Rests on

- **B2**: every v1 shell capability is in scope, re-designed rather than ported; the parity map below is the checklist.
- **B3**: no v1 visuals carry over; this document holds capabilities and behaviour only.
- **B7**: shell surfaces are Freya trees in the compositor; the compositor animates open and close; Skia's cache is capped.
- **B8**: windows float and snap; minimise is a real state, which the switcher, App Exposé and Mission Control show.
- **B9**: one main loop, services on a worker; each surface sits behind a panic boundary.
- **B10**: the compositor owns settings; `tokens.kdl` reloads live into every surface.
- **B11**: surfaces read system state through the shared service crates, never directly.
- **B12**: the shell draws every system prompt and the lock screen; PAM runs off the main thread.
- **B13**: the shell draws the screencast source picker; window previews are compositor textures.
- **B14**: widgets are built-in Freya components, extended by out-of-process data providers.
- **B17**: where a choice is open, the shell does what a Mac user expects.
- **B18**: at rest the wallpaper and the island, with the clock, status glyphs only when active, and Control Centre opening from it; the front app's menu bar with the global menu appears on demand.
- **B19**: no Dock: launching from Spotlight's Apps grid, switching by Cmd+Tab and Mission Control, badges, progress and attention on the switcher's cards.
- **B20**: the menu bar and switcher speak in apps; Cmd+Tab switches apps and Cmd+` windows.
- **B21**: shortcut hints in menus show the Mac chord; Isle's own injected keys are never translated.
- **B22**: Isle has no console mode: no Big Picture home, quick menu or pad-to-keys bridge, so no surface is driven by a controller.
- **B24**: the island's content, where banners appear, the order of live activities and where widgets live are P1's; this document says only what must be possible.
- **B26**: the screencast picker is the shell's; the Open and Save panels and Quick Look run in the Files process; the greeter is the compositor's greeter mode reusing the lock screen.
- **B27**: v1's notification and clipboard history are not imported.
- **B28**: a Bluetooth pairing agent joins the prompts; layer-shell is served but `ext-session-lock` is not; providers run sandboxed with permissions the user grants.
- **B30**: Cmd+Q quits kept-running apps too; a fixed escape set works over a game; Focus modes as on macOS, with a Gaming Focus.

## Features

### Shell runtime

Not a surface the user sees, but what makes every surface behave like part of one system, as the
macOS shell does: nothing stutters, nothing goes missing, and one surface failing never takes the
desktop with it.

1. Every surface in this document is a Freya tree inside the compositor process. No shell surface is a separate Wayland client or process (B7).
2. Each surface's event handling and drawing runs behind a panic boundary. A panicking surface is logged with its name and rebuilt from service state; windows and the other surfaces keep running (B9).
3. A rebuilt surface comes back in the state it was in: open or closed, with the same page or query where that state is held outside the tree.
4. The lock screen stays locked whatever happens to its surface. A panic in it never shows session content; a second within a minute brings in the minimal lock (B68).
5. Open and close transitions, morphs between surface sizes and the minimise animation are drawn by the compositor on the surface's texture. Freya animates only what changes inside a surface (B7; `shell-ui-spike.md`, launcher rows).
6. Skia's GPU resource cache has a fixed budget, and a surface's cached resources are released when it closes (B7; `shell-ui-spike.md` measured cached layers as the extra VRAM).
7. A surface redraws only when its state changes. A closed surface draws nothing and runs no timers; a service that samples (CPU, storage, sensors) samples only while a visible surface shows it (inventory §6 Polling).
8. A surface never waits on a service. Data arrives over channels from the worker; a slow or dead service shows as stale or absent, never as a stalled frame (B9).
9. A change to `tokens.kdl` re-renders every surface with the new tokens; a parse error keeps the last good tokens and is shown by the shell (B10). Text scale, high contrast and reduced motion are token overrides (D50); reduced motion and Game Mode set every shell transition to none.
10. Spotlight, Mission Control, the capture tool, the session dialog and Force Quit close each other: one is open at a time, and Escape closes it. System prompts and Files' Open and Save panels close nothing (D67, B26).
11. When a surface closes, keyboard focus returns to the window that had it before any action the surface started (raise, paste, type) is delivered to that window (D63).
12. Every surface is fully usable from the keyboard; the focus ring shows for keyboard input only. No surface takes controller input (B22). Every surface exposes an accessibility tree through Freya's AccessKit integration.
13. No client surface, layer-shell overlays included, stacks above the lock screen, a system prompt or the capture tool (B28).
14. Each screen can carry the always-on surfaces. One screen is the main screen (chosen, or following focus); a transient event, Spotlight or a dialog appears on one screen, never on all (D49; inventory §3.7 Primary monitor). Mission Control is the exception: it opens on every screen at once, as macOS's does (`p1/structure/mission-control.md`).
15. Every surface can run outside the compositor in a `freya-winit` window with hot reload, fed by recorded or fake service state (B7's live-iteration condition).
16. Every surface can be opened, driven and read through the compositor's CLI, which is the test surface (inventory §13).

v1 lessons kept
- D72: a crash-prone part must not share a failure domain with everything else; here, a boundary per surface.
- D62: a surface open before a restart comes back; here, after a rebuild.
- D63: an action after a surface closes waits for the focus handover, or the handover undoes it.

Answered in P3 (`docs/research/rust-desktop/p3-runtime.md`)
- The Runner is driven from calloop through the fork's `freya-embed`: a waker pings the loop, which runs the tree's update; input reaches it through the seat, keyboard focus and Tab through Freya's accessibility tree, and the tree is on AT-SPI through an accesskit adapter, all without `freya-winit` (B37).
- Skia's cache is capped at 64 MiB and purged as a surface closes; a full-output surface with every tile fading in its own layer used 31 MB of it. Over a one-hour soak that opened, typed in, relayouted and closed surfaces VRAM held at 105 MiB; the process's own memory grew by a few KB a cycle, slowing, which is open until a heap profiler explains it before the 24-hour soak; the spike's 178 MB relayout run did not recur.
- Glass is blurred by the compositor (B39). Each output has its own instance of an every-screen surface; the island in P6 is the first to draw on several.

Open for its packet
- How many panics bring in the minimal lock that P1 designed (`lock.md`): P9's.

### Menu bar

B18's menu bar: the front app's menus and the system's own menu, shown on demand as macOS's
automatically hidden bar. At rest it is not on screen; the clock, status and Control Centre are the
island's. Its arrangement beyond what B18 fixes is P1's.

1. The bar carries a system menu, the app menu named for the app, the app's own menus, and a Window and a Help menu supplied by the desktop where the app has none.
2. From the system menu the user reaches the computer's details, System Settings, available updates with their count, Force Quit, sleep, restart, shut down, lock and log out; a restart owed is stated there (D46).
3. The app menu reaches the app's About, its settings, hide, hide others, show all and quit. Settings is the app's own item when its menu has one, else the app's settings chord from the Cmd table (`macos-model.md` §2.4 Cmd+,).
4. The Window menu reaches minimise, zoom, the snap zones of B8, bring all to front and the app's windows.
5. The user can search every item in the focused app's menus, the app's own Help or the desktop's; a match opens and highlights its menu (`macos-model.md` §1.3 item 9). The same items are a Spotlight source (Spotlight 2).
6. At rest the bar is hidden on every screen. It appears when the pointer pushes the top edge outside the island's zone, or from the keyboard, and leaves when the pointer leaves it and its menus or a menu closes. It slides over windows and reserves no space, so no window moves when it comes.
7. The bar carries no status items, Control Centre item or clock; those are the island's (Island 9–11). The island stays in front of the bar at the centre and nothing in the bar is hidden under it. When the app's menus do not fit, every one stays reachable.
8. The focused app's menus can be reached from the bar on any screen.
9. Ctrl+F2 (macOS's "move focus to the menu bar") reveals the bar with its first menu open; arrows, typing and Return work through menus.
10. Over a full-screen window the bar is revealed the same way, over the content. Over a game in Game Mode it needs a dwell at the edge instead of a push (inventory §4 Hot zone), and never appears while the game holds the pointer. It never appears during a window drag, when the top edge is the fill target.

v1 lessons kept
- Inventory §4 Hot zone: a short dwell at the edge, not a touch, reveals the bar over a game.
- D46: a restart owed is said where the user will see it.

Open for its packet: none.

### App menus

The global menu: the focused app's real menus in the bar, and a designed fallback for the many apps
that export none (B18). On macOS every app has a menu bar; on Linux Steam, Wine, games, libadwaita
apps, Firefox by default and Electron on Wayland export nothing, so the fallback is the common case
(`macos-model.md` §3 Notes).

1. Menus come from DBusMenu through `com.canonical.AppMenu.Registrar`, from `org_kde_kwin_appmenu`, from `gtk_shell1` with `org.gtk.Menus`, and from the X11 properties Isle's window manager reads; all are owned from session start (B18; `macos-model.md` §1.3). The serving end belongs to platform.md and compositor.md; this document owns what the bar does with the tree.
2. A window's menu is found in the order of `macos-model.md` §1.3: its surface's own address, then its X11 registrar entry and properties, then a registration from a bus connection with the window's PID, else the fallback. An app id alone never selects a menu.
3. The fallback menu is built from what the desktop knows: the app's name and icon from its desktop entry, its desktop-entry actions (New Window and the like), its windows, and the desktop's own hide, minimise and quit. Every item in it works for every app.
4. For an app with no exported menu but a known shortcut set, the fallback carries those shortcuts as menu items when the per-app Cmd table (B21) lists them, so the menu still teaches the app's keys.
5. Every call into an app is asynchronous with a timeout. A menu not yet fetched opens as the fallback with the app's items arriving into it, never as a blank or a hang (`macos-model.md` §1.3 item 8).
6. Isle calls AboutToShow before opening a submenu, refetches on LayoutUpdated and batches bursts of signals.
7. Shortcut hints show the Mac chord the user presses, mapped back through the Cmd table, not the Ctrl chord the app exported (`macos-model.md` §1.3 item 10).
8. An app that hides its in-window menubar because the desktop advertised a global one always has its menus in the bar; Isle advertises `gtk_shell1`'s menu-bar capability only once its reader works (B18).
9. Disabled, checked, radio and icon items, separators and submenus render as the app declares them.

v1 lessons kept
- None in v1, which had no menu bar (`macos-model.md` §3). The inventory's app-identity rules (§3.6 Apps: desktop entry lookup, `steam_app_<id>` to the game's name) name the app menu.

Open for its packet
- Whether the PID match recovers Electron's menus today, and which Electron release carries PR #54629.
- Flatpak: whether the D-Bus PID and the Wayland client PID agree.
- Whether GTK 4 apps without a menubar export an empty model, and what `gtk_shell1` v5's `titlebar_gesture` obliges.
- Qt 5 and Qt before 6.9 through the KDE platform theme; JetBrains under WLToolkit; whether Steam exports anything.
- Whether to load appmenu-gtk-module per launch for plain GtkMenuBar apps, with an exclusion list for the reported crash.

Settled in P6
- The lookup order and the PID match are built (B50). No Electron release carries PR #54629 yet; until one does, the PID match finds an app that registers, and with several windows each shows the first registration's menus.
- GTK 4 and libadwaita apps export a model only when they call `gtk_application_set_menubar`, which few do; the rest keep the fallback (`global-menu-2026.md`). `titlebar_gesture` obliges the compositor to act on header-bar clicks, which it does (compositor.md, Title bars).
- appmenu-gtk-module per launch waits for Spotlight's launching (P10). Flatpak's PIDs and Qt 5, JetBrains and Steam wait for the host run.

### Island

B18 makes the island, v1's pill at top centre, the one thing on screen at rest besides the
wallpaper: the clock, status glyphs only when active, and the way into Control Centre and
Notification Centre. What it shows while live, and where each kind of live activity appears, is P1's
(B24); this section says what the shell must make possible.

1. Live activity is what is happening now that the user may need to see or stop; the shell shows it for as long as it lasts (B24).
2. Live activities: screen recording with its timer and Stop; screen sharing with the sharing app's name and Stop, for any portal stream; microphone and camera in use; a call; media playing; an authentication waiting; a timer.
3. Transient events are acknowledged briefly: OSDs (see OSDs), Game Mode and Focus changes, a clipboard copy, Do Not Disturb toggled, an app asking for attention (B19).
4. Privacy indicators (recording, sharing, microphone, camera) always show, on every screen, while the capture runs, and cannot be dismissed. Other activities and events show on one screen, never on all (D49).
5. When several activities are live, privacy indicators still show and every other activity stays reachable.
6. From an activity the user reaches its controls (media transport and scrubbing, Stop, the requesting app) and where it belongs (an authentication's prompt, the player's full controls).
7. Activity the user is interacting with is not taken over by an event; the event waits until the pointer leaves (inventory §4 Island details).
8. Over a game's Space in Game Mode no live activity shows except privacy indicators and OSDs, until the user reveals the island at the top edge (`gaming.md` Game Mode 7).
9. The island always carries the clock; its format (date, weekday, seconds) is a setting. A click on the clock opens Notification Centre.
10. The island carries a glyph for Isle's own status: Wi-Fi, Bluetooth, sound, VPN, Focus (the Gaming Focus included), updates and a restart owed, battery, keyboard shortcuts inhibited, a configuration error. Each shows only while its state is not the normal one, and the user can set each to always or never show; the last two show whenever their state holds. Other programs' status items are Control Centre's (Control Centre 9).
11. Hovering the island opens Control Centre from it after a short dwell, without taking keyboard focus; a click pins it with focus. Hover never opens it during a window drag or while a menu is open, and a click that opens something elsewhere disarms it until the pointer leaves (`design-v1-lessons.md` §3.5).
12. Windows keep a strip along the top of each screen clear for the island. A full-screen window covers it and the island hides, leaving its privacy indicators and OSDs; a push at the top edge in the island's zone brings it back.

v1 lessons kept
- Inventory §4 Island details: an open panel is never taken over by an event.
- D49: every screen gets the island; an event shows on one screen, not on every one.
- `design-v1-lessons.md` §3.5: the hover guards (a dwell before opening, a grace before closing, hover held while the panel settles) are requirements.
- `design-v1-lessons.md` §2: a glyph shows only if its absence says something; the normal state shows none.
- D55: Stop on a share ends the stream at its source; a capture exists only while it is wanted.

Open for its packet: none.

### Control Centre

macOS's Control Centre: a panel holding the quick controls. It unfolds from the island, as v1's
hover panel did (inventory §4 Island), laid out the macOS way: live activity first, then the
controls, then other programs' tray items.

1. Control Centre opens from the island (hover, a click, or Ctrl+F8), on the screen it was opened from.
2. Its controls are modules in a grid, each small, medium, large or wide: Wi-Fi, Bluetooth, Now Playing, Focus, VPN, Sound (the output device and volume), Display (brightness per monitor), Night Light, Energy Mode (the power profile), Updates and Batteries (the machine's and peripherals') by default, and Dark Mode, Keyboard layout, Accessibility (zoom, the on-screen keyboard, contrast, motion and transparency), Screenshot and Phone (through its provider) from the gallery. Game Mode (the Gaming Focus) is not a module: it is a row of the Focus module's page, and the Focus module shows it while on (B30).
3. Each module leads to its detail as a page in the panel (Wi-Fi networks, Bluetooth devices and pairing, outputs with per-app volumes and inputs, VPN providers, media players) and from there to its full page in the Settings app. A page's head is the module itself, its switch included, so one control is on screen for each thing. A network that needs a password opens the Wi-Fi prompt (B12), never a field in the panel.
4. The user adds, removes, resizes and reorders modules. Controls are not pinned to the island: the glyphs of Island 10 are the only status it carries, and each opens its control's detail.
5. Provider-backed controls (VPN, Phone) render the provider's state sentence and actions and know nothing of the tool behind them (inventory §8).
6. Brightness over DDC shows the requested value at once and writes debounced, since one write takes about a second (inventory §3.7 Brightness).
7. Focus works as on macOS: named modes, Do Not Disturb among them, each with its allowed apps and people and an optional schedule; v1's quiet hours become a schedule (B30).
8. A Gaming Focus turns on by itself whenever a detected game is in front, windowed or full screen, and off when it leaves (B30). It takes over from a Focus turned on by hand and gives it back when the game leaves.
9. Other programs' status items (StatusNotifierItem) sit in Control Centre as its tray, in the user's order, reordered by dragging with Cmd held. A tray item's click and menu behave as the item asks (activate, secondary activate, DBusMenu menu); its menu opens as a page in the panel.
10. Media has one place in the panel, the Now Playing module; the panel's live-activity section shows a media row only when that module has been removed.

v1 lessons kept
- Inventory §3.7: DDC reads are cached and writes debounced.
- D42, D44, D51: VPNs and phones are providers; the shell renders their actions.
- Inventory §4 Island: tray icons and their menus as a page in the panel.

Open for its packet: none; game detection is gaming.md's.

### No Dock

Isle has no Dock (B19). Nothing at rest replaces it; each of its duties lives in a surface that opens
on demand.

1. Apps are launched from Spotlight and its Apps view, which keeps a row of favourites in the user's order (Spotlight 11).
2. Running apps are seen and switched in the app switcher and Mission Control. Apps hidden with Cmd+H, kept running after their last window closed (B20), with only minimised windows, or running in the background after quit are listed in the switcher with a mark (App switcher 5).
3. Badges and progress come from `com.canonical.Unity.LauncherEntry` and show on the app's switcher entry and its tile in the Apps view. An app asking for attention is marked on its switcher entry until activated, and announced by the island (Island 3).
4. Minimised windows are reached from App Exposé, Mission Control, the switcher and the Window menu. Minimise shrinks the window toward the island and restore returns it from there (`compositor.md` Minimise and maximise).
5. An app's desktop-entry actions and LauncherEntry quicklist, Open at login, Show in Files and the desktop it is assigned to are reached from its Spotlight result and its tile in the Apps view.
6. Files dragged to an app reach it through Cmd+Tab during the drag, which brings the app forward to take the drop, or through Files' Open With. Trash is in Files.
7. By default the bottom-left hot corner opens Mission Control and the bottom-right Spotlight's Apps view, so the pointer alone can switch and launch (Mission Control 8).
8. A game shows as the game's name and art wherever an app is shown: the switcher, Spotlight and Mission Control (inventory §3.6 Apps).

v1 lessons kept
- Inventory §3.6 Kept running: apps that quit on last-window close are hidden instead, per app; they show in the switcher rather than a tray row.
- D63: launching what is already open raises it.

Open for its packet
- LauncherEntry coverage outside Electron is uneven (`macos-model.md` §3 Dock).
- Which apps run in the background with no window; there is no shared signal for it.

### Spotlight

macOS Spotlight on Cmd+Space: one field that finds apps, files, actions, settings, menu items and
clipboard history, with the Apps grid that replaced Launchpad. It carries every v1 launcher source.

1. Cmd+Space opens Spotlight on the main screen with the field focused; keys typed during the opening transition reach the field.
2. Sources: apps; shell actions, each shown with its chord; Settings pages; open windows; the front app's menu items; file names; file contents; calculator and unit conversion; emoji; clipboard history; the vault; a web search fallback.
3. Mac quick keys narrow the view (Cmd+1 Apps, Cmd+2 Files, Cmd+3 Actions, Cmd+4 Clipboard; `macos-model.md` §3), and v1's prefixes keep working: `>` run, `=` calculate, `/` file names, `?` file contents, `.` emoji, `:` clipboard, `@` windows, `*` vault (inventory §4 Launcher).
4. Apps rank by launch count and recency. Choosing an app that is running raises it (D63).
5. Fast sources (apps, actions, settings, windows, menu items) answer from memory. Slow sources (files, contents, calculator) arrive after them and never change the selected row while the user has not moved.
6. Content search is debounced and bounded in results, and runs on the worker (inventory §4 Launcher).
7. Clipboard results include images and pins; a result can be pasted, pinned or deleted. Paste goes into the window focused before Spotlight opened, using that app's paste chord (terminals differ).
8. Emoji are pasted into the previous window: Isle offers the pick as the clipboard and sends that app's paste chord, putting the clipboard back after (B79). Ctrl+Cmd+Space opens Spotlight in emoji mode.
9. A vault result copies the password on Return, the username with Shift and the one-time code with Ctrl; each secret is fetched for that keypress and never cached (D42).
10. Cmd+Y on a file result opens Quick Look, which Spotlight calls over D-Bus in the Files process (B26); Space is typed into the field, which keeps focus. Cmd+Return reveals the file in Files.
11. The Apps view lists every installed app, filterable by typing and by category, with a row of favourites at the top in the user's order, added from a tile's menu or by dragging a tile there. It opens from Cmd+1, a hot corner or the Launchpad key. A tile shows the app's badge, and its menu reaches the app's actions (No Dock 5).
12. Clipboard history and the vault answer only in their own scopes (Cmd+4 or `:`, and `*`), never in the general search, so nothing copied and no vault title shows beside an app.
13. There is no assistant: an unmatched query ends at the web search row.

v1 lessons kept
- D63: raise, not relaunch, and only after the focus handover.
- Inventory §4 Launcher: debounce content search and cap its results; type into the previous window, not into Spotlight's own.
- D42: secrets are fetched per field.

Settled in P10
- File search is Isle's own index of names, kept live by inotify, with contents searched on demand (B76); localsearch and plocate were compared and rejected (`spotlight-2026.md` §1).
- The compositor owns Spotlight's model and its keys (B73); sources are `isle-search`'s (B74); results rank by tier, then use (B75).

### App switcher

macOS's Cmd+Tab: one entry per app in recent order, released on Cmd up, with Cmd+` cycling the
front app's windows (B20, D59).

1. Cmd+Tab switches to the previous app. If Cmd is held past a short delay, the switcher appears on the main screen; a quick tap switches without drawing it.
2. Each entry is an app; its window count, a live preview of its most recent window (B13: drawn by the compositor), its badge and progress, and whether it is hidden, minimised, not responding, asking for attention or running in the background are available to it (B19).
3. While held: Tab and Shift+Tab move, arrows move, the pointer selects, ` cycles the selected app's windows, Q quits it (kept-running apps too, B30), H hides it, W closes its front window, Escape cancels.
4. Releasing Cmd commits: the selected app comes forward with its most recent window, unhidden and restored if needed.
5. Apps with only hidden or minimised windows, and apps kept running with none (B20), are listed and restored on commit. Apps running in the background after quit are listed last, and Q stops them.
6. Cmd+` without the switcher cycles the front app's windows in a stable order, not recency, so every window is reachable (D59).
7. Cmd+Tab works out of a game in front; it is in the fixed escape set with Mission Control, lock and screenshot, and every other chord goes to the game (B30).
8. Modifiers held through the switch never stick in either app (B21).

v1 lessons kept
- D59: list apps, not windows; cycle windows in a stable order.
- D55: previews exist only while the switcher shows them.
- Inventory §16 Switcher: commit on release is tested, since v1 lost releases to the compositor's bind rules.

Open for its packet: none beyond B21's mid-chord focus rules, which compositor.md owns.

Settled in P7: the switcher takes no focus and commits on release (B53); previews are compositor drawings (B54); background apps by their processes (B55). The ` key is the one that types it, as Cmd+` is (P5), not the key above Tab.

### Mission Control, hot corners and gestures

macOS's Mission Control (every window of the current desktop spread out, the desktops along the top,
App Exposé for one app's windows), reached by keys, hot corners and trackpad gestures. Mouse routes
come first, since the host is a desktop.

1. Mission Control opens from Ctrl+↑, a three-finger swipe up, a hot corner or the Mission Control key; App Exposé from Ctrl+↓ or a three-finger swipe down. The switcher's own window row replaces macOS's ↓ in the switcher (`p1/structure/mission-control.md`).
2. Every window of the current desktop is shown at once, so any can be picked. "Group windows by application" is a setting.
3. Every desktop and full-screen space is shown and can be switched to, and a desktop can be added.
4. A window is closed with its close button, W, Delete or a middle click; dragged onto another desktop, it moves there; dragged onto a new desktop, it gets one. Desktops are reordered and removed there too.
5. App Exposé shows one app's windows, its minimised and hidden windows included and told apart. Mission Control shows the current desktop's minimised windows, and its hidden apps' windows, in a tray along the bottom apart from the spread, since there is no Dock to hold them.
6. Arrows, Tab and Return work throughout; Escape or the opening gesture reversed closes it.
7. Window images are the compositor's own textures and exist only while Mission Control is open (B13, D55).
8. Each screen corner can run Mission Control, App Exposé, Show Desktop, Notification Centre, Spotlight's Apps view, the widgets dashboard, Lock Screen, Put Display to Sleep, or nothing, optionally only with a modifier held. A corner fires once per visit, after the pointer rests in it, and never over a full-screen game. By default the bottom-left corner runs Mission Control and the bottom-right Spotlight's Apps view (B19).
9. Gestures: three fingers up for Mission Control, down for App Exposé, left and right between desktops, thumb and three fingers spread for Show Desktop and pinched for Spotlight's Apps view. Each drives its surface's progress with the fingers and settles on release; each has a key and a pointer equivalent. Deferred to a trackpad packet of its own, since the host has no trackpad (`p16-mission-control.md` choice 2).

v1 lessons kept
- D19: Mission Control is drawn from window images; their motion is a P1 proposal.
- D55: a window is captured only while its image is on screen.
- D48: a full-screen window has a space of its own, shown among the desktops.
- Inventory §3.6: closing goes by window identity, never through a dying handle.
- D56, D76: screens come and go behind a KVM; corners follow the real screen list.

Open for its packet
- An edge swipe for Notification Centre needs touch positions libinput's gesture events do not carry (`macos-model.md` OPEN); deferred with the gestures (9).
- Split View and Stage Manager (a later layout module, off by default, B25) are compositor.md's; how Mission Control shows them follows from there.

### Notification Centre

macOS's Notification Centre: notifications that wait to be read later, with banners for new arrivals
(B24).

1. Notification Centre opens from the island's clock, a chord or a hot corner, on the screen it was opened from. It unfolds from the pill under the clock, as Control Centre does from the rest of the pill, and one centre is open across all screens.
2. Notifications are cleared singly, by app or all. Each keeps its actions, inline reply and image while the app's server object lives.
3. Each arrival is also kept as a record; records that are no longer live stay readable, without actions, and survive restarts (D26).
4. A new notification shows a banner at the top right of the main screen, under the island's strip, unless silenced. Silenced: the active Focus (Gaming Focus included, B30), a full-screen window (unless another screen is free of one, where the banner shows instead), a share or recording of a screen (not of one window), the user away (idle dim, sleep, lock), the app muted. Critical notifications and allowed apps break through, except that a screen capture holds even critical ones; low urgency never shows a banner.
5. What was silenced is counted, and when silence lifts the count is summarised once with a way into the centre.
6. A banner lasts what the app asks within bounds, pauses while hovered, takes keyboard focus only to reply, and may play a sound.
7. Clicking a banner or entry runs its default action and brings the app forward with an activation token.
8. The desktop's own events arrive as notifications with actions: a drive inserted (Mount, Eject), an AppImage installed, an update finished, a restart owed, a calendar event starting, a provider's sign-in needing attention.
9. On the lock screen notifications show as none, each app with a count, or previews, by setting; each app with a count by default.
10. History starts empty: v1's notification records are not imported (B27).
11. Above the notifications it shows the date, a month and the agenda of the subscribed calendars (`platform.md` Calendars). It holds no widgets.

v1 lessons kept
- D26: history is records, not live notifications.
- Inventory §4 Notifications: the silencing set, per-app mute and allow lists, the summary when silence lifts, the toast's bounded lifetime.
- D49: a banner on one screen, never all.

Open for its packet: none.

### Widgets

Widgets built into the shell and extended by providers (B14). v1's dashboard keeps its capabilities:
widgets placed and edited on a grid, and the gallery (B24). They live on the full-screen dashboard and nowhere else, not in Notification Centre (P1, `wallpaper.md`, `widgets.md`).

1. The user places widgets and can bring them into view with one desktop chord, Ctrl+Option+Cmd+W, from whatever is open but a game in front, which keeps the chord (B30). v1's Super+A cannot carry: it is Cmd+A, Select All (B21).
2. Built-in widgets: Clock (several, each with a zone), Calendar, Weather, Media, Desktops, Notifications (a glance at each app's waiting count that opens Notification Centre, never a second list), Controls, System (with its own editor for series, D36), Session (power and profile), Devices (batteries), Storage.
3. Widgets sit on a grid with an editor: a live outline of where a drag lands, pushing to make room, resizing between a widget's minimum and maximum sizes, keyboard nudges (D24, D35). Each screen has one canvas with its own layout, without pages or scrolling.
4. A provider is a program in any language that emits JSON into a built-in template: text, number, gauge, sparkline, graph, list or card (B14; v1's manifest-only views, D23). It runs as its own sandboxed process (B28), on an interval or streaming, is restarted if it dies, and runs only while its widget is visible.
5. A widget's settings are a form built from its manifest; a built-in may bring its own editor (D36).
6. The widget gallery shows installed widgets, the user's own and those a source offers, with sources that are git repositories carrying an index, pulled periodically, installed through staging and updated by version (D29). The isle-widgets registry is the default source and carries providers.
7. A provider declares the permissions it needs (network, files, commands); the sandbox allows only those the user has granted (B28; D28, D31).
8. Installing a provider from a source asks first, showing what it runs and the permissions it declares; a provider from a source the user has not granted is installed but not run (D28, D29). The user's own providers run without asking for the source, but still only with granted permissions.
9. Author tools (check a manifest, picture a widget, tag a release) stay with the gallery (D30).

v1 lessons kept
- D24, D35: the grid editor and its row count.
- D26: the Notifications widget and the centre read the same records.
- D28, D29: consent before third-party code runs; installs swap in whole.
- D30: authors write in their own editor; the gallery checks and pictures.
- Inventory §6 Polling: nothing samples while hidden.

Open for its packet
- The sandbox mechanism (bubblewrap or Landlock) and the permission vocabulary mapped from v1's (D31).
- Which of the registry's QML widgets (analog-clock, system-rings, transport, battery, focus-timer, uptime) become built-ins and which providers (inventory §6 Registry).
- Interactive widgets (macOS 27 live widgets) beyond buttons and toggles.

### OSDs

Feedback for volume, brightness, keyboard backlight, mute and the like; where it shows is P1's (B24).

1. Volume, mute, microphone mute, brightness, keyboard brightness and keyboard layout changes raise an OSD, whichever key, device or app caused the change.
2. An OSD shows the new level and the device it applies to (which output, which monitor).
3. Repeated changes update one OSD in place; it leaves shortly after the last change.
4. In Game Mode OSDs still show, briefly, without revealing the island or the menu bar.

v1 lessons kept
- Inventory §2.4: v1 raised an OSD only for volume; every level change gets one here.

Open for its packet: none.

### Lock screen

macOS's lock screen, drawn by the compositor itself rather than a separate locker (B12).

1. The lock takes the password as soon as the user types; fingerprint verification runs beside the password, never in front of it (D33).
2. A wrong password says so and counts attempts; Caps Lock is shown when on; the keyboard layout can be changed.
3. Now Playing controls, battery, notifications (by setting), Sleep, Restart and Shut Down are reachable without unlocking. The on-screen keyboard is available.
4. The session locks on Ctrl+Cmd+Q, the system menu, a hot corner, idle, lid close, login1's Lock signal and before every sleep. Sleep proceeds only once every screen shows the lock.
5. No frame of session content is shown between a lock request and the lock, or on resume.
6. Screens added while locked show the lock; the password field follows the pointer's screen.
7. PAM conversations run in a helper process; a module that blocks or crashes never stalls the lock (B67).
8. If the compositor restarts while locked, it comes back locked (D15, D43).
9. Unlock unlocks the login keyring where PAM allows; when it cannot (a fingerprint unlock), the keyring prompt follows.
10. The lock is Isle's alone: `ext-session-lock` is not served, so no other locker can take or end it (B28).
11. At login the compositor runs in greeter mode under greetd and this screen is the greeter: the same password route and power actions, with a choice of user, starting the session instead of unlocking it (B26). It shows the user's wallpaper, appearance and accent as the session published them for it, never the token file.

v1 lessons kept
- D15, D43: a lock must survive the death of whatever draws it; the session restarts locked.
- D33: fingerprint beside the password.

Open for its packet
- The login keyring after a fingerprint unlock (`macos-model.md` §3 Touch ID, reasoning only), with fingerprint itself: P9 left fingerprint out until there is a reader.

Answered in P9: the second panic within a minute brings in the minimal lock (B68).

### System prompts

The system's own dialogs, drawn by the shell (B12): administrator authentication, keyring unlock,
Wi-Fi and VPN secrets, Bluetooth pairing, and the session and Force Quit dialogs.

1. The polkit dialog names the app that asked and the action in plain words, offers a choice of administrator where there is more than one, and takes a password; fingerprint works where polkit's PAM stack allows it.
2. The keyring prompter unlocks a keyring, or creates one with a confirmed password (`org.gnome.keyring.SystemPrompter`).
3. The Wi-Fi prompt names the network and its security and takes the secret; VPN secrets use the same dialog.
4. Bluetooth pairing, asked by Isle's `org.bluez.Agent1`, shows a passkey to confirm or takes a PIN (B28).
5. A prompt takes keyboard focus when it appears and stays above every window; Escape cancels. Cmd+Tab sets it aside as a waiting prompt and switches; nothing else takes focus from it. One prompt shows at a time; others queue. Text from the requester is shown as plain text, never markup.
6. A prompt waiting behind a full-screen window or game, or set aside, shows as live activity, from which the user brings it forward.
7. A prompt raised while the session is locked waits until unlock.
8. The session dialog (from a chord or the power key) offers Restart, Sleep, Shut Down, Log Out and Lock, each by its first letter; Restart and Shut Down count down before acting, and say which apps are still open.
9. Force Quit (Cmd+Option+Escape) lists running apps, marks the ones not responding, and ends the chosen one.

v1 lessons kept
- D67: a dialog an app asked for closes nothing else.
- Inventory §4 Auth and Power menu: an authentication waiting is announced; a countdown precedes restart and shut down.
- D47: authorisation in a dialog, never a terminal.

Open for its packet: none; the agents behind these dialogs are platform.md's.

### Capture

macOS's screenshot tool (Shift+Cmd+3, 4 and 5), plus v1's recording, colour pick and annotation, and
the source picker for screen sharing (B13).

1. Shift+Cmd+3 takes the screen; Shift+Cmd+4 picks a region, with Space switching to window mode; Shift+Cmd+5 opens the capture toolbar. Adding Ctrl sends the result to the clipboard instead of a file.
2. The toolbar offers screen, window or region; shot or recording; a countdown; whether the pointer is included; whether audio is recorded; where to save.
3. The screen is frozen before a region or window is picked, and the shot is that frozen frame, so the picker's own veil never reaches the image (inventory §16 Capture). The pointer is in a shot only when asked for (D27).
4. After a shot one thumbnail shows for a short time, the file already saved; a new shot replaces it. The thumbnail opens the shot in Quick Look, where Markup annotates it (apps.md, Files), or is dragged out as a file.
5. A recording's timer and Stop are live activity; Stop ends and saves it.
6. Colour pick reads the colour under the pointer from a frozen frame and copies it.
7. The screencast source picker lets the user choose among screens and windows, each seen live, says which app is asking and whether it wants audio, and offers to share or cancel. Previews run only while the picker is open (D55). Screenshot portal requests that ask for interaction open the capture tool.
8. A source that disappears during a share or recording ends it cleanly and says so.

v1 lessons kept
- Inventory §16 Capture: freeze before the pick.
- D27: no pointer in shots by default.
- D55: capture only what is on screen, only while wanted; a vanishing output must not take anything down.

Open for its packet: none.

Built in P11 (B80–B85). Until Files brings Quick Look (P15), a click on the
thumbnail opens the shot in the default image viewer and Markup waits.

### Wallpaper and idle dimmer

The desktop picture per screen, and macOS's dim before the display turns off, with v1's dim, lock and
screen-off steps.

1. Each screen shows an image, a colour or a drawn backdrop, set per screen or for all. A folder can be the source, cycled on a schedule; light and dark variants follow the appearance.
2. A wallpaper change applies live; any transition is drawn by the compositor. Nothing on the wallpaper animates in Game Mode or under a full-screen window.
3. After the dim timeout the screens dim, and any input restores them without unlocking; after the lock timeout the session locks; after the screen-off timeout the screens turn off.
4. Idle inhibitors (video, calls, presentations) and Game Mode hold all three. A timeout of zero means that step is off, never that it fires at once.

v1 lessons kept
- Inventory §4 Wallpaper: an image or a drawn backdrop, and a folder to pick from.
- D18: an idle timer is rebuilt when its timeout changes, and zero means off.
- D76: screen power is said again whenever the screen list changes.

Open for its packet: none.

### On-screen keyboard

macOS's Accessibility Keyboard: an on-screen keyboard for pointer users (D37). It is not driven by a controller (B22).

1. The keyboard never takes focus; it types into the focused window through Isle's injected-key source, which Cmd translation never touches (B21).
2. Keys are pressed with the pointer; word completions are offered.
3. It reserves space so it covers nothing.
4. It opens from a chord, Control Centre's Accessibility module and the lock screen.

v1 lessons kept
- D37: a shell surface in the shell's conventions, not a separate keyboard app.

Open for its packet
- Showing it automatically when a text field takes focus, through text-input-v3, whose reliability per toolkit is unmeasured (`macos-model.md` OPEN).

## For the design process

What this document leaves to P1, per feature; "proposal" is the answer it gave before B24. Items
P1 has answered point to the sheet that answers them.

- **Shell runtime**: open, close and morph choreography. Which screens carry the island, the menu bar and the wallpaper, which other surfaces show only on the main screen, and the UI copy rules, are answered in P1 (`docs/work/rust-desktop/p1/structure/island.md`, `menu-bar.md`, `wallpaper.md`, `notifications.md`, `spotlight.md`, `prompts.md`, `copy.md`) and recorded in DESIGN.md.
- **Menu bar**: Answered in P1 (`docs/work/rust-desktop/p1/structure/menu-bar.md`, `island.md`); recorded in DESIGN.md.
- **App menus**: Answered in P1 (`docs/work/rust-desktop/p1/structure/menu-bar.md`); recorded in DESIGN.md.
- **Island**: Answered in P1 (`docs/work/rust-desktop/p1/structure/island.md`); recorded in DESIGN.md.
- **Control Centre**: Answered in P1 (`docs/work/rust-desktop/p1/structure/control-centre.md`, `focus.md`, `island.md`); recorded in DESIGN.md.
- **No Dock**: the Dock's arrangement, attention, badges and running apps are answered in P1 (`docs/work/rust-desktop/p1/structure/dock.md`, `switcher.md`) and recorded in DESIGN.md.
- **Spotlight**: Answered in P1 (`docs/work/rust-desktop/p1/structure/spotlight.md`); recorded in DESIGN.md.
- **App switcher**: Answered in P1 (`docs/work/rust-desktop/p1/structure/switcher.md`); recorded in DESIGN.md.
- **Mission Control**: Answered in P1 (`docs/work/rust-desktop/p1/structure/mission-control.md`); recorded in DESIGN.md.
- **Notification Centre**: Answered in P1 (`docs/work/rust-desktop/p1/structure/notification-centre.md`, `notifications.md`, `focus.md`, `island.md`); recorded in DESIGN.md.
- **Widgets**: Answered in P1 (`docs/work/rust-desktop/p1/structure/widgets.md`, `wallpaper.md`); recorded in DESIGN.md.
- **OSDs**: Answered in P1 (`docs/work/rust-desktop/p1/structure/island.md`); recorded in DESIGN.md.
- **Lock screen**: Answered in P1 (`docs/work/rust-desktop/p1/structure/lock.md`, `wallpaper.md`); recorded in DESIGN.md.
- **System prompts**: Answered in P1 (`docs/work/rust-desktop/p1/structure/prompts.md`, `island.md`); recorded in DESIGN.md.
- **Capture**: Answered in P1 (`docs/work/rust-desktop/p1/structure/capture.md`, `island.md`); recorded in DESIGN.md.
- **Wallpaper**: Answered in P1 (`docs/work/rust-desktop/p1/structure/wallpaper.md`); recorded in DESIGN.md.
- **On-screen keyboard**: Answered in P1 (`docs/work/rust-desktop/p1/structure/keyboard-osk.md`); recorded in DESIGN.md.

## Acceptance bars

Measured on the host (RTX 5070 Ti, 3440×1440 up to 144 Hz) unless stated.

1. Every row of the parity map exists as stated or is reshaped as stated.
2. At rest, with no surface open and a minutes clock, the shell causes at most one frame a minute; with a seconds clock, one a second. A closed surface fires no timers (checked by tracing).
3. The compositor process with every always-on surface at rest uses less RSS than v1's Quickshell process alone (458 MB, `ui-and-services.md` §1.4). The budget, set in P3: under 250 MB RSS and 200 MiB of VRAM on the host. P3 measured 101 MB and 68 MiB with no surface open on a 3440×1440 output, and 126 MB with its probes drawn (`p3-runtime.md`). A window's title bar, set in P4: under 0.5 MB of heap and 0.5 MiB of VRAM each past the first; thirty bars measured 0.27 MB and 0.47 MiB each, headless on the 5070 Ti (`p4-windows.md` in research).
4. After a 24-hour soak that opens and closes every surface, RSS and VRAM return to within 10% or 20 MB, whichever is larger, of their first-hour values (the margin P3 set; the 24-hour run waits for real surfaces, at the end of M1).
5. No shell surface's content draw exceeds 2 ms at p95 at full output size (the spike measured 1.2 ms for a full-screen launcher, `shell-ui-spike.md`). Shell transitions drop no frames at 144 Hz.
6. Spotlight: keys typed from the moment Cmd+Space is pressed are never lost; apps, actions, settings and windows answer within 50 ms of a keystroke; the selected row never changes under the user as slower results arrive.
7. App switcher: a quick Cmd+Tab switches without drawing the switcher; 1,000 scripted switches, some with focus changing mid-chord, commit on release every time and leave no modifier stuck.
8. Menu bar: every focused window has a menu whenever the bar is revealed; the app's menu titles appear within 100 ms of the reveal when its tree is cached; an app that does not answer is replaced by the fallback within its timeout. A matrix of a Qt 6.9+ app, a Chromium 137+ browser, LibreOffice on GTK 3 and a GtkApplication app with a menubar shows their real menus.
9. Notifications: a new notification's banner is on screen within 100 ms of the Notify call reaching the bus.
10. Lock: every screen shows the lock before sleep proceeds; no frame of session content is scanned out between a lock request and the lock, or on resume; 100 injected panics in the lock surface never show session content.
11. An injected panic in any other surface rebuilds it within a second while windows and other surfaces keep running.
12. A saved change to `tokens.kdl` reaches every visible surface within a second; a broken file keeps the last good tokens and shows the error.
13. Over a full-screen game at rest, the shell contributes no render element, so direct scanout stays possible.
14. A scripted keyboard-only walk reaches every action of every surface.

## Parity map

Every v1 capability in this area (inventory §4, §6, the shell chords of §11) and where it lands.

| v1 capability | Lands in |
|---|---|
| Wallpaper: image, drawn backdrop, folder pick | Wallpaper and idle dimmer |
| Rest pill: clock | The island's clock (Island 9), opening Notification Centre |
| Rest pill: Wi-Fi, Bluetooth, muted volume, VPN, updates, restart owed and mode glyphs | The island's status glyphs, shown only when active (Island 10); restart owed also in the system menu; mode as the Focus glyph |
| Rest pill: recording and screen share glyphs | Privacy indicators (Island 4) |
| Rest pill: calendar within 30 minutes | Calendar widget; the event's start is a notification |
| Rest pill: desktop dots, which opened Mission Control | Capability kept in Mission Control (rule 3); the island shows no desktops |
| Hover panel: opened by hover on the pill, with its guards | Control Centre from the island (Island 11) |
| Hover panel: header of desktops as app icons | Mission Control's Spaces bar and the app switcher |
| Hover panel: toggles (Wi-Fi, Bluetooth, night light, DND as a Focus, game mode, VPN), output picker, volume, brightness, per-app volumes, media, batteries, updates, power profile | Control Centre modules (Control Centre 2); game mode as a row of the Focus page |
| Hover panel: Wi-Fi password and Bluetooth pairing drill-downs, VPN providers page | Control Centre drill-downs and the system prompts |
| Hover panel: tray icons and tray menu page | Control Centre's tray (Control Centre 9) |
| Hover panel: notifications page | Notification Centre |
| Events: volume OSD | OSDs, widened to every level change |
| Events: media start, recording and share with Stop, auth pending, mode change, clipboard copy | Live activities and events (Island 2, 3); where each shows is P1's |
| Events: notification toast with actions, reply and image | Notification Centre banners; their place is P1's (B24) |
| Events: drive inserted with Mount and Eject, calendar start, AppImage installed, update finished, restart needed | Desktop notifications with actions |
| An open panel is never taken over by an event | Island rule 7 |
| Island on every screen, events on one (D49) | Shell runtime rule 14; which surfaces each screen carries is P1's |
| Game mode: island unmapped, hot zone peeks it | Island 8, 12, Menu bar 10: nothing but privacy and OSDs over a game; a top-edge dwell reveals the island, or the bar with it |
| Dashboard: full-screen widget grid on a chord | Capability kept (Widgets 1, 3); the dashboard is the one widgets surface (P1, `wallpaper.md`) |
| Dashboard: edit mode, grid editor, pages | Widget grid editor (Widgets 3); pages replaced by one canvas per screen |
| Built-in widgets (Clock, Media, Workspaces, Notifications, Controls, System, Session, Devices, Storage, Calendar, Weather) | Built-in widgets |
| Agents widget (terminal windows) | Dropped: no longer a feature |
| Manifest-only widgets and the text renderer | Provider templates |
| QML user widgets, sandbox, permissions, grants and trust | Providers, sandboxed with user-granted permissions (B28) |
| Widget store: sources, install, update, consent, author tools | Widget gallery |
| Registry's QML widgets | Re-made as built-ins or providers (open) |
| Launcher: ranked apps, shell actions with chords, settings pages, windows, vault, web fallback | Spotlight sources, plus menu items and Cmd+1 to Cmd+4 |
| Launcher prefixes `>` `=` `/` `?` `.` `:` `@` `*` | Spotlight prefixes |
| Clipboard: thumbnails, pins, delete, paste into the previous window | Spotlight clipboard |
| Emoji typed after close | Spotlight emoji through the injected-key source |
| Apps ranked in the launcher (no Launchpad) | Spotlight's Apps view, with favourites |
| Switcher: one entry per app, count, live previews, backtick cycle, Q quits, W closes, commit on release | App switcher, plus H hides and Cmd+` without the switcher |
| Mission Control: spaces bar with add, spread, close, drag between desktops, keyboard, real-rectangle animation, previews only while visible | Mission Control; the spaces bar, spread and animation are P1 proposals |
| Mission Control: hidden-window tray | App Exposé's minimised set and Mission Control's minimised windows (Mission Control 5) |
| Three-finger swipe up and down | Mission Control gestures |
| Capture: region, window, screen, shot, record, countdown, cursor, audio, freeze before pick | Capture toolbar and Mac chords |
| Capture: annotate, colour pick, recording timer with Stop | Capture; annotation as Markup in Quick Look (apps.md); the timer as live activity |
| Lock: wallpaper, clock, password on typing, failure count, Caps Lock, fingerprint, login1 signals, relock after restart | Lock screen |
| Greeter (inventory §12) | Lock screen in greeter mode (B26) |
| Idle: dim, lock, screen off, inhibitors | Wallpaper and idle dimmer |
| Polkit dialog | System prompts |
| Power menu: five actions, first letter, countdown | Session dialog and system menu |
| Notifications: Earlier records, DND, quiet hours, full-screen and game silencing, summary, per-app lists, sound | Notification Centre; DND, quiet hours and game silencing as Focus modes (B30); records not imported (B27) |
| On-screen keyboard: two cursors, completions, legend, no focus, exclusion | On-screen keyboard, pointer-driven; two cursors and legend dropped (B22) |
| Surfaces close each other; Escape closes | Shell runtime rule 10 |
| Surfaces back after a reload (D62) | Shell runtime rule 3; windows in apps.md |
| Kept-running apps in the tray row | App switcher (App switcher 5) |
| Reduced motion, text scale, high contrast | Shell runtime rule 9 |
| IPC as the test surface | Shell runtime rule 16 |
| File chooser surface | Open and Save panels in the Files process (B26); apps.md |
| Big Picture home, quick menu, Guide button and pad-to-keys bridge | Dropped (B22) |
| Snap ghost | compositor.md |

## Not here

- **compositor.md**: the window model and layouts (the work area under the island's strip, snapping and its preview, minimise, hide, quit, Split View, Stage Manager, picture-in-picture, the dropdown terminal); title bars and window controls (B25); Cmd translation, the Alt and Super swap and the full chord table; gesture and hot-corner recognition; outputs, KVM recovery and screen power; the magnifier and night light; app identity from app ids and desktop entries; window images; the X11 window manager's menu properties; session restore; greeter mode under greetd and the session start, whose screen is the lock screen here (B26).
- **gaming.md**: game detection (which turns on the Gaming Focus), Game Mode's profile, Steam's Big Picture handling and controllers.
- **apps.md**: Settings and its page index, Files with its Open and Save panels and the Quick Look service (B26), Monitor, Keychain, Cleaner.
- **P1, the design process**: every question under For the design process.
- **platform.md**: the notification server, tray watcher and host, AppMenu registrar, polkit agent, keyring prompter, NetworkManager secret agent, Bluetooth agent, the portal backend and its PipeWire streams, providers for vaults, VPNs, snapshots and phones, the clipboard history store, calendars and updates.
