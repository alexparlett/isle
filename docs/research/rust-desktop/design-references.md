# Design references: where others beat macOS 27

State as of 2026-10-03. Stage 1b of P1 (`docs/work/rust-desktop/p1-design.md`).
Not a survey: each entry is something another desktop or tool presents
better than macOS 27 "Golden Gate" does, on a named surface, with what it
costs. Anything merely equivalent is left out; a short list of what was
checked and dropped closes each section where it matters.

**Method.** Web sources only, every link read on 2026-10-03; version dates
are the projects' own. The session's web-search quota ran out near the end,
so a few claims rest on a search summary or on prior knowledge; those are
marked *(unsure)*. macOS 27's baseline is from MacRumors, Six Colors and
Apple's support pages (links in §0).

**Versions read.** GNOME 51 (2026-09-16; 50 was 2026-03-18). KDE Plasma 6.7
(2026-06-16; 6.8 due 2026-10-14). COSMIC Epoch 1.1 (2026-06-23) and later
point releases. elementary OS 8.1 (2025-12-22). Otto at `nongio/otto` main.
Windows 11 25H2 with the 2026 drops; PowerToys 0.100. SteamOS 3.8
(2026-06-18). Bazzite 44.20260914. DankMaterialShell 1.6 (2026-09-03),
Noctalia 5.0.1 (2026-09-03), Caelestia and illogical-impulse at main.

---

## 0. The baseline: what macOS 27 already does

So these are not counted as wins for anyone else:

- **Menu bar overflow.** macOS 27 draws the whole bar as one window and adds
  a » button that expands hidden items in place beside the notch. Items can
  be switched off in System Settings › Menu Bar (26) and Cmd-dragged. No
  rules, no search, no second row, no spacing, no per-display layouts. The
  change broke Bartender, Ice, Thaw and SketchyBar's aliases.
  [badgeify](https://badgeify.app/macos-27-golden-gate-menu-bar-changes/),
  [brow-app](https://brow-app.com/blog/macos-27-menu-bar)
- **Liquid Glass slider** from clear to tinted, and a "Show borders"
  accessibility option.
  [MacRumors](https://www.macrumors.com/2026/09/17/new-features-changes-macos-27/)
- **Dock** marks apps still running in the background after quit (reviewers
  dislike it). [Six Colors](https://sixcolors.com/post/2026/09/macos-27-golden-gate-review-bridging-the-tahoe-gap/)
- **Spotlight** (26) has actions, quick keys, menu search and a 20-item
  clipboard history; in 27 it merges with Siri as "Search or Ask".
- **Tiling** is still halves, quarters, Fill, Centre and Fill & Arrange; no
  thirds, no zones, no groups.
  [Apple](https://support.apple.com/guide/mac-help/tile-app-windows-mchlef287e5d/mac)
- **Widgets**: extra-large desktop widgets that update live.
- **OSDs** (26): a small slider under Control Centre's icon, top right.

Reviewers report Mission Control, Spaces and Notification Centre as laggy in
27.0; that is a bug, not a design, and is not counted.

---

## 1. Menu bar and status-item overflow

- **KDE Plasma 6, tray visibility per item.** Each item is shown, shown when
  relevant, hidden, or disabled; a "when relevant" item steps out of the
  overflow by itself when it has news (SNI status `NeedsAttention`).
  *Better:* the user decides what is always there, and an item that needs
  attention cannot stay buried; macOS 27's » is all or nothing. *Cost:*
  one more settings page; depends on apps setting their status honestly.
  [cosmic-epoch #2268](https://github.com/pop-os/cosmic-epoch/issues/2268) (quotes KDE's options)
- **Bartender 6/7, rules.** An item shows only while a condition holds
  (battery below a level, a network, a time, a script, the icon changing).
  *Better:* items appear when they have something to say, which is v1's
  rest-glyph idea generalised. *Cost:* positions shift, so spatial memory
  suffers; the rule engine is opaque. *For Isle:* Isle's own items only need
  a "shows when it says something" flag, not a rule editor.
  [Bartender 6](https://www.macbartender.com/Bartender6/)
- **Bartender Quick Search / Command Bar.** Type to find and open any
  status item. *Better:* status items reachable from the keyboard; macOS's
  menu search covers app menus only *(unsure)*. *Cost:* invisible until
  learnt. *For Isle:* status items as a Spotlight source costs nothing.
- **Ice Bar / Bartender Bar.** Hidden items open in a strip under the menu
  bar rather than squeezing in beside the notch. *Better:* overflow is not
  limited by the width left of the notch. *Cost:* a second layer; Ice's
  spacing jumped between displays.
  [Ice](https://github.com/jordanbaird/Ice), [#988](https://github.com/jordanbaird/Ice/issues/988)
- **Ice, three tiers** (shown, hidden, always hidden) with reveal on hover,
  scroll or a click on empty bar. *Better:* a middle tier between on and
  off. *Cost:* hover reveals fire by accident.
- **Bartender 7, presets per display set.** Different visible items per
  display or display combination, applied automatically. *Better:* the
  1080p screen can carry a short bar. *Cost:* configuration.
  [release notes](https://www.macbartender.com/Bartender7/release_notes/)
- **DMS 1.0, up to four bar configurations**, each bound to a monitor.
  Same win as above, native rather than bolted on.
  [danklinux](https://danklinux.com/blog/v1-release)
- **Itsycal / Dato, clock opens a calendar.** A month grid, agenda, week
  numbers, join-meeting buttons, world clocks, next event in the bar.
  *Better:* the macOS clock opens Notification Centre and shows no month.
  *Cost:* almost none; it replaces the clock rather than adding width.
  *For Isle:* a direct answer to "where the clock sits and what it opens".
  [Itsycal](https://www.mowglii.com/itsycal/), [Dato](https://sindresorhus.com/dato)
- **SketchyBar setups: desktops in the bar.** Per-display pills listing
  each desktop's app icons. *Better:* macOS shows no Spaces in the bar.
  *Cost:* B24's proposal keeps desktops out of the island; this would be a
  status item if anywhere. [SketchyBar](https://github.com/FelixKratz/SketchyBar)

Lesson across the lot: every macOS manager broke because it poked at windows
Apple owns. Isle draws its own bar, so rules, search, a strip and per-display
presets are cheap to own natively.

## 2. The island and notch

- **boring.notch (2.8 rc).** Hover expands the notch into media with a
  visualiser, calendar, a file shelf, battery, a camera mirror, and
  replacements for the volume, brightness and backlight OSDs. *Better:*
  media and OSDs in one predictable place at top centre. *Cost:* opens when
  the pointer heads for browser tabs near the top
  ([#764](https://github.com/TheBoredTeam/boring.notch/issues/764));
  glitchy on multiple displays.
  [repo](https://github.com/TheBoredTeam/boring.notch)
- **NotchNook (1.6), two-stage open.** Hover nudges it slightly open; a
  click or swipe opens it fully. A file tray holds things dropped on it.
  *Better:* glance and commit are separate gestures, so accidental opens
  cost little. *Cost:* tray emptied on quit; fakes a notch on external
  displays, which users dislike.
  [comparison](https://favtray.com/blog/notchnook-vs-boring-notch)
- **Notch apps' shared failure.** A hover zone beside the bar steals clicks
  from the nearest status items, and one app opened on any passed-through
  click. *For Isle:* hover intent with a delay; never open on a click that
  also reached something else.
  [notchy #48](https://github.com/vishvavariya/notchy-feedback/issues/48)
- **Otto, notifications that shrink but stay.** A notification goes card,
  then pill, then an app-icon dot, and stays until dismissed; one app's
  notifications fan as a deck; any process can push a live activity over
  D-Bus (`org.otto.Island1`). *Better:* unread items stay visible; any
  long job (build, copy) gets a progress surface. macOS has neither for Mac
  apps *(unsure whether macOS 27 opened Live Activities to Mac apps)*.
  *Cost:* no history, no DND, one monitor; progress not drawn yet.
  [Otto island](https://nongio.github.io/otto/dynamic-island/)
- **DMS 1.6 "Dank Island".** A centre pill with satellite widgets; live
  faces for media, notifications and OSDs; springs open into the dash or
  Control Centre; its idle face is arranged by drag and drop, down to
  hiding the clock. *Better:* the empty centre of a 3440 px bar becomes the
  one place things happen. *Cost:* competes with the clock and title; no
  documented reduced-motion form.
  [v1.6](https://danklinux.com/blog/v1-6-release)
- **Caelestia, OSD shows the microphone.** Volume and brightness OSDs carry
  mic state. *Better:* macOS shows only an orange dot.
  [repo](https://github.com/caelestia-dots/shell)
- **Hand Mirror.** Click under the notch for an instant camera preview
  before a call. *Better:* macOS needs Photo Booth. *Cost:* an unlabelled
  target. [handmirror.app](https://handmirror.app/)

## 3. Dock

- **elementary OS 8.1, pressure reveal.** A hidden Dock appears only when
  the pointer pushes past the edge, not on contact. *Better:* no accidental
  reveals over a full-screen game or editor, a real problem on macOS.
  *Cost:* a deliberate push; needs pointer-barrier logic in the compositor,
  which Isle has. [blog](https://blog.elementary.io/os-8-1-available-now/)
- **elementary OS 8.1, one indicator per window.** *Better:* the window
  count is visible; macOS shows one dot. *Cost:* a busier Dock.
- **Windows 11, taskbar where the window is.** With bars on every display,
  each lists only its own windows, pinned apps on the main one. *Better:*
  the macOS Dock lives on one display and jumps on a bottom-edge dwell.
  *Cost:* icons move when windows move.
  [elevenforum](https://www.elevenforum.com/t/change-which-taskbar-to-show-apps-on-for-multiple-displays-in-windows-11.3682/)
- **Plasma 6 panels**: fit to content, centred, float until a window
  touches them *(unsure: from knowledge)*. *Better:* on an ultrawide a
  centred Dock that floats is closer to where the work is.

Dropped: elementary's background-apps item (macOS 27 now does it); Otto's
Dock (a faithful copy, primary monitor only).

## 4. App switcher

- **AltTab (11.x).** Per-window switching with thumbnails, including
  minimised, hidden and other-Space windows with a badge; close, minimise,
  quit from inside; a current-screen-only filter. *Better:* reaching a given
  window is one step, and a two-monitor setup does not mix screens. *Cost:*
  thumbnails need capture permission (not an issue for a compositor).
  [alt-tab.app](https://alt-tab.app/)
- **KDE task switcher filters.** Current desktop, current screen,
  minimised, one per app; a second switcher on its own chord. *Better:*
  the same per-screen filter, natively. *Cost:* many options.
  *(unsure the screen filter survives in 6.7)*
- **GNOME Alt+Tab.** Grouped by app, but an arrow drops down that app's
  window thumbnails in place. *Better:* the app/window split of Cmd+Tab and
  Cmd+` in one surface, with previews. *Cost:* by default lists only the
  current workspace *(unsure)*. *For Isle:* the closest fit to B20.
- **Windows Alt+Tab shows snap groups** as one entry. *Cost:* clutter.
  [winaero](https://winaero.com/windows-11-now-shows-snap-groups-in-alt-tab-and-taskbar-previews/)

## 5. Window snapping and its zone UI

The surface where macOS is furthest behind, and the one the ultrawide needs
most (§19).

- **Windows 11 Snap Layouts.** Hover the maximise button or Win+Z: a grid of
  layout thumbnails; click the slot this window takes; Snap Assist offers
  the other windows for the rest. On wide displays it adds thirds and a
  25/50/25 centre layout. *Better:* layout and slot chosen in one gesture,
  every arrangement visible first; macOS hides arrangements in a text menu
  with no thirds. *Cost:* accidental opens.
  [PCWorld](https://www.pcworld.com/article/2523976/windows-11-update-improves-snap-layouts-usability-adds-new-features.html),
  [WinCentral](https://thewincentral.com/windows-11-has-snap-layouts-with-3-columns-for-large-displays-improved-docking-multiple-monitor-experience/).
  *For Isle:* the green-button menu (B25) should be this grid, not a list.
- **Windows snap bar.** Dragging a window to the top edge drops a strip of
  layouts to release onto. *Better:* macOS's drag to the menu bar only
  fills. *Cost:* competes with drag-to-maximise.
- **Rectangle Pro Snap Panel.** While dragging, a panel of targets appears
  by the pointer, so the drop is local rather than at a far edge; per-edge
  snap areas (top gives thirds, its centre two-thirds); user-drawn zones;
  saved layouts. *Better:* short travel on a wide screen. *Cost:* clutter
  while dragging. [docs](https://rectangleapp.com/pro/docs/snap-areas/)
- **Loop, radial picker.** Hold a trigger key, flick the pointer; a ring
  highlights the layout and a preview shows the result before release;
  repeating cycles sizes. *Better:* a flick replaces a long drag. *Cost:*
  needs a trigger key; about eight directions.
  [Loop](https://github.com/MrKai77/Loop)
- **Rectangle, cycling and almost-maximise.** Repeating a half cycles half,
  third, two-thirds; "almost maximise" and a centred custom size.
  *Better:* the macOS Centre command keeps the size and Fill spans the full
  3440 px. *Cost:* many chords.
  [Rectangle](https://github.com/rxhanson/Rectangle)
- **KDE custom tiles (Meta+T).** Draw a zone layout per screen; Shift-drag
  into it; neighbours resize together. *Cost:* Shift-drag undiscoverable;
  Meta+arrow quick tiling ignores the custom tiles, which ultrawide users
  rebind round.
  [KDE discuss](https://discuss.kde.org/t/how-is-quick-tile-supposed-to-work/20809)
- **FancyZones (PowerToys).** Relative-grid or free-canvas zones per
  monitor; hovering a shared edge spans two zones; several windows can
  stack in one zone and cycle with a key; new windows reopen in their last
  zone; Win+Ctrl+Alt+digit swaps the whole layout, even mid-drag.
  *Better:* stacks, spans and per-task layouts with no tiling-WM learning.
  *Cost:* hidden gestures.
  [Microsoft Learn](https://learn.microsoft.com/en-us/windows/powertoys/fancyzones)
- **COSMIC, per-workspace auto-tiling** with per-app float exceptions and a
  drop outline. *Better:* one desktop tiled for code, others free. *Cost:*
  a floating window becomes tiled when moved to a tiled desktop.
  [LWN](https://lwn.net/Articles/1050268/)
- **COSMIC window stacks.** Windows of different apps grouped as tabs in
  one frame. *Better:* macOS tabs only within one app. *Cost:* new windows
  join the selected stack, which surprises.
- **Windows snap groups** remembered as a unit in the taskbar and Alt+Tab.
  *Cost:* broken by moving one window.

## 6. Mission Control and overview

- **KDE Overview and Desktop Grid (6.7).** All desktops as a live grid;
  drag windows between them in full context; search built in. *Better:*
  macOS shows other Spaces only as strip thumbnails.
  [6.7](https://kde.org/announcements/plasma/6/6.7.0/)
- **GNOME overview, dynamic desktops.** There is always one empty desktop
  at the end; dropping a window there makes it. *Better:* no hover "+".
  *Cost:* thumbnails small.
  [help](https://help.gnome.org/users/gnome-help/stable/shell-workspaces.html.en)
- **illogical-impulse, typed overview.** Super opens the overview and typing
  searches at once, with prefixes for clipboard, emoji and actions.
  *Better:* Mission Control and Spotlight merged in one gesture. *Cost:*
  prefixes to learn (it has a cheat sheet).
  [docs](https://ii.clsty.link/en/ii-qs/02usage/)
- **COSMIC 1.1, named and pinned desktops** that keep their tiling across
  reboots and can be dragged to another display. *Better:* Spaces cannot be
  named or moved between displays.
  [epoch-1.1.0](https://github.com/pop-os/cosmic-epoch/releases/tag/epoch-1.1.0)

## 7. Notifications and their history

- **KDE, job progress in notifications.** Copies and downloads show progress
  with pause and cancel in a notification that stays in history *(unsure:
  from knowledge)*; ending DND offers a "view what you missed" button (6.5).
  *Better:* macOS has no system-wide progress surface.
  [6.5](https://kde.org/announcements/plasma/6/6.5.0/)
- **DND that follows full screen.** Windows switches DND on while gaming or
  in a full-screen app; Plasma 6.4 does it while a full-screen window has
  focus and sets the freedesktop `Inhibited` property so apps know.
  *Better:* tied to focus, so a game on one screen silences banners while
  chat on the other stays reachable. *Cost:* full-screen video also mutes.
  [KDE](https://planet.kde.org/kristen-mcwilliam-2025-05-21-do-not-disturb/),
  [Microsoft](https://support.microsoft.com/en-us/windows/notifications-and-do-not-disturb-in-windows-feeca47f-0baf-5680-16f0-8801db1a8466).
  Matches B30's Gaming Focus.
- **Banner position.** GNOME puts banners top centre under the clock; KDE
  lets the user choose a corner or near the tray. *Better on an ultrawide:*
  banners near where the eye is; macOS fixes them top right. *Cost:* top
  centre covers tab titles.
  [OMG! Ubuntu](https://www.omgubuntu.co.uk/2021/12/change-position-of-notifications-on-ubuntu)
- **DMS 1.4, mute a source from the banner** (right-click), and a privacy
  mode that hides content. *Better:* muting is one step, not a Settings
  trip. [v1.4](https://danklinux.com/blog/v1-4-release)

Dropped: GNOME and KDE grouping (equivalent).

## 8. Control Centre and quick settings

- **Noctalia 5, tabbed Control Centre.** A Home tab plus Media, Audio,
  Network, Bluetooth, Notifications, Power; arrow keys move between panes;
  IPC opens a given tab. *Better:* deep links and a keyboard path; macOS
  Control Centre is poor from the keyboard. *Cost:* drifts toward a second
  Settings. [docs](https://docs.noctalia.dev/noctalia/control-center/)
- **illogical-impulse, detail cards open in place.** A toggle's detail
  slides down beneath it rather than replacing the panel. *Better:* context
  kept. *(unsure: from a summary)*
- **Brightness for external displays.** KDE drives monitors over DDC/CI;
  GNOME 51 remembers brightness per display; BetterDisplay does it on macOS.
  *Better:* macOS gives no slider for most third-party monitors. *Cost:*
  DDC is slow and patchy. [GNOME 51](https://www.omgubuntu.co.uk/2026/09/gnome-51-released)
- **Prevent sleep.** elementary has a quick-settings toggle; KDE's power
  item lists which apps are holding the machine awake *(unsure)*. *Better:*
  macOS needs `caffeinate`. Useful for game downloads.
- **Decky Loader in Steam's Quick Access Menu.** Plugins get a whole page
  in the quick panel, not a single tile. *Cost:* unsandboxed.
  [deckyloader.org](https://deckyloader.org/guide/best-decky-loader-plugins).
  *For Isle:* provider modules (B14) could own a drill-down page.

Dropped: GNOME's quick-settings drill-down, Windows Quick Settings (no
better than Tahoe's editable Control Centre).

## 9. Widgets

- **Xbox Game Bar, pinned widgets.** A widget can be pinned so it stays over
  the game after the bar closes. *Better:* macOS widgets cannot float over a
  full-screen app. *Cost:* the full bar is a collage that takes focus.
  [XDA](https://www.xda-developers.com/microsoft-xbox-game-bar-update-compact-mode/)
- **Plasma sensor widgets in the panel.** Any sensor (GPU temperature) as a
  panel widget, custom monitor pages *(unsure: from knowledge)*.
- **iStat Menus 7, combined item.** One status item showing chosen readings,
  history graphs in its menu, threshold alerts. *Better:* glanceable;
  Activity Monitor is a window. *Cost:* bar width.
  [MacRumors](https://www.macrumors.com/2024/07/31/istat-menus-7-0-brings-new-features/)

Nothing found that beats macOS 27's widget placement and editing; v1's grid
editor remains the reference.

## 10. Launcher and Spotlight

- **KRunner.** Finds and switches to open windows by title, kills
  processes, web shortcuts (`gg:`), desktops and power actions; tolerant of
  typos (6.5). *Better:* Spotlight cannot jump to a given window *(unsure
  for 27)*. *Cost:* plugin noise in ranking.
  [userbase](https://userbase.kde.org/Plasma/Krunner)
- **PowerToys Command Palette.** Explicit scope prefixes (`>` command, `$`
  settings, `=` maths, `??` web), window switching, clipboard history,
  pinned commands on its home. *Better:* predictable scoped results where
  27's Spotlight is semantic. v1's prefixes already do this.
  [Microsoft Learn](https://learn.microsoft.com/en-us/windows/powertoys/command-palette/overview)
- **Raycast 2.0, action panel.** Cmd+K lists everything that can be done to
  the selected result; any command gets its own alias and hotkey; clipboard
  history with preview, pins and type filter, kept for months. *Better:*
  Spotlight's clipboard holds 20 items for up to a week and strips rich
  text. *Cost:* a second search surface on macOS.
  [AlternativeTo](https://alternativeto.net/news/2026/8/raycast-2-0-is-officially-out-of-beta-with-a-new-interface-and-cross-platform-support/)
- **Alfred Universal Actions.** Select text, a file or a URL anywhere and
  act on it. *Better:* acts on a selection, not only on typed input.
  [alfredapp](https://www.alfredapp.com/universal-actions/)

## 11. Screenshots and recording

- **CleanShot X 5.0, thumbnail stack.** Recent captures pile up as floating
  thumbnails, each with copy, save, annotate and drag-out; freeze before
  select; scrolling capture; pin to screen; OCR; a month of history.
  *Better:* macOS shows one thumbnail that times out. *Cost:* the stack
  piles up in a corner. [features](https://cleanshot.com/features)
- **Shottr.** A pixel ruler and colour picker inside the capture.
  *Better:* measuring UI without another tool. [shottr.cc](https://shottr.cc/)
- **Windows Snipping Tool.** Mark up inside the overlay before saving;
  OCR text actions; automatic redaction of emails and phone numbers; colour
  picker. *Better:* macOS splits these across the thumbnail, Preview and
  Digital Color Meter. *Cost:* a busy toolbar.
  [MakeUseOf](https://www.makeuseof.com/new-snipping-tool-features-that-more-useful-than-it-used-to-be/)
- **Spectacle (6.6).** OCR; leave chosen windows out of a recording.
  [6.6](https://kde.org/announcements/plasma/6/6.6.0/)
- **GNOME 51, keyboard-adjusted selection.** Arrows resize the region,
  modifiers change the step or move it, R resets. *Better:* macOS selection
  is pointer-only bar Space and Shift.
- **Steam Game Recording.** A background buffer of the game only, with a
  timeline of automatic and manual markers, cut and share in place.
  *Better:* no replay buffer on macOS *(unsure)*.
  [MakeTechEasier](https://maketecheasier.com/review-gameplay-with-steam-game-recording/)

## 12. Lock, greeter and prompts

- **elementary 8.1, password prompts dim and hold focus.** Nothing else can
  take focus while a prompt is up. *Better:* a password cannot be typed into
  the wrong window. Directly relevant to Isle's polkit agent.
- **DMS 1.5/1.6, greeter follows the session.** Font, wallpaper and theme
  are read live from the user's config. *Better:* the macOS login window
  ignores the user's appearance. Fits B26 (the greeter is the lock screen).
  [v1.6](https://danklinux.com/blog/v1-6-release)

Dropped: GDM, SDDM, cosmic-greeter (nothing better found).

## 13. Settings app layout

- **KDE "Highlight changed settings".** Marks every page and control that
  differs from its default. *Better:* macOS cannot show what you changed.
  For Isle, where `settings.kdl` is also hand-edited, it doubles as a diff.
  [pointieststick](https://pointieststick.com/2020/08/14/this-week-in-kde-highlight-changed-settings-and-much-much-more/)
- **Windows 11 breadcrumbs** ("System › Display") that can be clicked to go
  up a level *(from knowledge)*; a home page of adaptive cards. *Better:*
  macOS has back and forward only. *Cost:* Microsoft uses the cards to
  upsell. [Pureinfotech](https://pureinfotech.com/windows-11-settings-new-home-cards/)
- **DMS, the shell's own options in Settings search**, plugins included.
  Nothing hidden in a defaults domain.

## 14. File manager

- **Dolphin.** Split view with drag between panes, a terminal panel that
  follows the folder, an editable location bar, a filter bar, git status.
  *Better:* Finder in 27 has no dual pane or terminal. *Cost:* dense chrome.
  [apps.kde.org](https://apps.kde.org/dolphin/)

## 15. System monitor

- **GNOME Resources.** Processes grouped by app; GPU pages with per-app
  VRAM and encoder use *(unsure on the per-app columns)*. *Better:*
  Activity Monitor has no per-app VRAM or encoder view, which gaming and
  streaming need. [nokyan/resources](https://github.com/nokyan/resources)
- **DMS 1.4, process list in Control Centre** with search and keyboard.
  *Better:* kill a hung game without opening a window.
- **Stats (exelban).** Each module picks a tiny widget style (line, bar,
  pie, speed). *Better:* free glanceable telemetry.
  [stats](https://github.com/exelban/stats)

## 16. Theming third-party apps

macOS passes only light/dark and an accent. Everything here beats it; the
question is which model Isle takes.

- **Omarchy, one palette file to templates.** About 22 semantic keys in
  `colors.toml`; one command renders the bar, notifications, lock,
  terminals, editors and btop, retints running apps, and lets a hand-written
  file in a theme override its template. *The clearest model for
  `tokens.kdl` feeding P14.*
  [theming.md](https://github.com/omacom/omarchy/blob/quattro/docs/theming.md)
- **COSMIC.** A full palette plus radii, applied to GTK 3/4 and Flatpak apps
  through libadwaita's CSS variables; Frosted Glass (2026-07) has switches
  per surface (panels, applets, system UI, windows) and sliders for frost
  and opacity. *Cost:* exports approximate; some users want it off.
  [linuxiac](https://linuxiac.com/cosmic-desktop-big-visual-upgrade-is-finally-here/)
- **matugen 4.x.** Wallpaper or seed to Material schemes for any template,
  with a contrast setting from -1 to 1 and base16 output.
  [releases](https://github.com/InioX/Matugen/releases)
- **Material colour utilities, contrast per role pair.** Each pair declares
  a minimum ratio at each level and the tone moves until it is met (text on
  surface 4.5, 7, 11, 21). *Better:* guaranteed ratios at three user
  levels, where macOS has one switch. *For Isle:* declare the targets in the
  token schema and let stage 6's contrast script enforce them.
  [color_spec_2021.ts](https://raw.githubusercontent.com/material-foundation/material-color-utilities/main/typescript/dynamiccolor/color_spec_2021.ts)
- **libadwaita, accent text by rule.** `--accent-color` is derived from the
  accent in Oklab by clamping lightness (at most 0.5 light, at least 0.85
  dark), hue kept. *For Isle:* derive `accent-text` the same way.
  [CSS variables](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/css-variables.html)
- **Terminal palettes that match yet read.** illogical-impulse rotates each
  ANSI hue toward the accent with a cap; DMS's `dank16` lifts lightness until
  each colour meets a contrast target. *Better:* red still reads as red.
  *Cost:* high harmony merges red and orange.
  [dank16](https://danklinux.com/docs/dankmaterialshell/cli-dank16/)
- **Noctalia 5, shell mode apart from app mode.** The bar can stay dark
  while apps follow day and night. *Better:* macOS ties them together.
  [theming](https://docs.noctalia.dev/noctalia/theming/)
- **KDE, wallpaper-declared accent.** A wallpaper's metadata can name its
  accent for light and dark, avoiding poor extraction for shipped
  wallpapers. [develop.kde.org](https://develop.kde.org/docs/plasma/wallpapers/)
- **The portal as the floor.** `org.freedesktop.appearance` publishes
  `color-scheme`, `accent-color`, `contrast` and `reduced-motion`; GTK, Qt,
  Firefox and Electron read at least some. Apps Isle cannot template still
  follow. [portal Settings](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.Settings.html)

What goes wrong, from all of the above: libadwaita takes colours but no
theme; GTK and Qt mostly need a restart (DMS's live GTK 3 switch is the
exception); Qt needs `QT_QPA_PLATFORMTHEME` before launch; Flatpaks need the
theme inside the sandbox; Electron takes light/dark only; wallpaper colours
fail contrast without a solver; base16 has no "on" colours.

## 17. Accessibility

- **Windows contrast themes.** Four high-contrast palettes whose colours per
  role can be edited and saved. *Better:* macOS Increase Contrast is one
  switch. *Cost:* old apps render badly.
- **Windows text cursor indicator.** A coloured, sized marker around the
  caret. *Better:* nothing like it on macOS *(unsure)*.
  [Microsoft](https://support.microsoft.com/en-us/accessibility/windows/make-windows-easier-to-see)
- **Transparency by surface and by degree.** Noctalia's solid, soft and
  glass; COSMIC's per-surface switches. macOS 27's slider is global, so the
  win left is per surface (keep glass on the Dock, none over a game).
  [Noctalia](https://docs.noctalia.dev/noctalia/configuration/shell/)
- **Motion speed apart from reduced motion.** DMS sets notification speed
  apart from the shell; Plasma has a global speed slider *(unsure)*.
  *Better:* macOS Reduce Motion is on or off. No shell documents what its
  springs become under reduced motion; Isle has to.

Dropped: GNOME 51 Reduced Motion, Plasma colour filters and zoom (matched).

## 18. Gaming presentation

- **Steam Deck, two buttons, two sides.** The Steam button opens navigation
  on the left; "…" opens the Quick Access Menu on the right (notifications,
  friends, quick settings, performance, music), both over the running game.
  *Better:* "go somewhere" and "change something" are split cleanly.
  [GamingOnLinux](https://www.gamingonlinux.com/2026/06/steamos-3-8-is-out-with-initial-steam-machine-support-desktop-mode-upgrades-new-graphics-drivers/)
- **QAM performance level slider.** Off, then levels 1 to 4 from FPS only to
  full telemetry. *Better:* one control reveals detail progressively;
  macOS's Metal HUD is for developers. *Cost:* fixed levels.
- **QAM per-game profiles.** Frame cap, refresh, power and scaling saved per
  game and reapplied on launch. *Better:* macOS Game Mode has no caps or
  memory. *Cost:* a forgotten profile surprises.
- **Steam Performance Monitor (2025).** Four levels, adjustable position,
  size, contrast and opacity, real frames shown apart from generated ones.
  [gHacks](https://www.ghacks.net/2025/07/01/steams-new-in-game-performance-monitor-is-now-available-for-everyone/)
- **MangoHud horizontal layout.** One strip along the top edge rather than a
  corner block; suits 21:9. [MangoHud](https://github.com/flightlessmango/MangoHud)
- **gamescope, overlays the compositor draws.** The overlay, HUD, frame
  limit and upscaler are applied by the compositor, so they work for every
  game alike. Isle is a compositor and can do the same.
  [gamescope](https://github.com/ValveSoftware/gamescope)
- **Windows HDR toggle** on a chord with a confirmation, Auto HDR per game.
  *Better:* compare HDR in game; macOS has a settings switch only. Feeds
  B23. [PC Gamer](https://www.pcgamer.com/windows-11-shortcut-hdr/)
- **Xbox full-screen experience (2026).** A controller-first shell entered
  and left within the same session. *Lesson, not a borrow:* B22 drops a
  console mode; Bazzite's whole-session swap is the worse model.
  [Windows blog](https://blogs.windows.com/windows-insider/2025/11/21/full-screen-experience-expands-to-more-windows-11-pc-form-factors-for-windows-insiders/)

---

## 19. Ultrawide (3440×1440) and multiple monitors

**What users complain about**

1. A maximised window is too wide to read; Fill spans the full width.
   [MPU forum](https://talk.macpowerusers.com/t/please-share-tips-for-managing-windows-on-an-ultrawide-computer-display/37989)
2. macOS tiling has no thirds or columns, so quarters leave a hole in the
   middle. [How-To Geek via Yahoo](https://tech.yahoo.com/general/articles/apple-still-cant-window-snapping-213013141.html)
3. The app menu sits at the far left while the window may be far right:
   long mouse travel (same thread as 1).
4. Banners fixed top right cover toolbars and tabs and sit far from the
   eye. [Apple discussions](https://discussions.apple.com/thread/252060916)
5. Windows' tray, clock and toasts pinned to the far corner.
   [ExplorerPatcher #875](https://github.com/valinet/ExplorerPatcher/discussions/875)
6. macOS full screen blanks the other display unless "Displays have
   separate Spaces" is on; with it on, the Dock jumps between screens; the
   bar and Dock reappear in games when the camera pans to an edge.
   [Macworld](https://www.macworld.com/article/227820/how-to-avoid-a-blank-second-display-when-using-full-screen-mode-in-an-os-x-app.html)
7. Windows' virtual desktops switch every monitor together, so a game on one
   screen moves with the other *(from knowledge)*.
8. KDE's quick tiling ignores its own custom tiles.
9. No Wayland desktop splits one panel into virtual outputs; Hyprland's
   request is open. [Hyprland #3575](https://github.com/hyprwm/Hyprland/issues/3575)

**Patterns that answer them**

- **Centre-weighted columns**: 25/50/25 and thirds, natively in Windows on
  wide displays; Tiling Shell for GNOME copies it.
  [tilingshell](https://github.com/domferr/tilingshell)
- **Almost-maximise and a centred fixed width** as the default "zoom", so a
  maximised window stays readable (Rectangle).
- **Pointer-local zone pickers** (Rectangle Pro's Snap Panel, Loop's ring) to
  cut travel across 3440 px.
- **Zone stacks**: a side column holds chat, music and docs, cycled by key
  (FancyZones).
- **Layouts saved per task** and recalled by chord (FancyZones).
- **Per-display bars and Docks** listing that display's windows (Windows),
  and per-display bar presets (Bartender 7, DMS).
- **Banners near the gaze** (GNOME top centre) rather than the far corner.
- **Split islands** of the taskbar so the tray is not stranded at the
  corner; a centred Dock that fits its content (Plasma).
- **Dock on the side** to keep vertical space, which is scarcer than width
  on 21:9.
- **Virtual split of one output** into two or three logical screens: no
  mainstream desktop does it natively, and Isle's compositor owns outputs,
  so it could. KDE's Mudeer script fakes it per window.
  [Mudeer](https://github.com/darkstego/Mudeer)
- **Focus-tied DND** so a game on one screen silences banners while the
  other stays live (§7).

---

## Open

- Whether macOS 27 lets Mac apps post Live Activities, and whether
  Spotlight in 27 switches to a given window; both decide how far Otto's
  island and KRunner's window search are ahead.
- Whether macOS 27's Gaming Focus can trigger on any full-screen game, or
  only on a controller.
- KDE claims from knowledge, not re-read: job-progress notifications, DDC
  brightness, which apps hold sleep, the task switcher's screen filter in
  6.7, fit-to-content floating panels, the animation speed slider.
- Windows Settings breadcrumbs and the caret indicator against macOS's
  insertion-point indicator.
- One weak source says 24H2 dropped the three-column Snap flyout on
  ultrawides; not believed, not disproved.
- Status of Ice upstream (Thaw is the maintained fork), AltTab 11.8 and
  NotchNook's availability.
- COSMIC's exact theme-export inputs and its 1.2–1.5 release dates.
- Dribbble and Behance reimaginings were not reviewed (search quota ran
  out); nothing is lost unless one solves a concrete problem above.
- How each shell's springs behave under reduced motion: none documents it,
  so Isle's answer is its own.
