# Widgets

Batch 3. Answers to: `shell.md` § For the design process (Widgets: whether
widgets also sit in Notification Centre's column, whether the
Notifications widget is dashboard only, the gallery's sections, whether
v1's pages survive) and Widgets 1–9 with its Open item on the registry's
widgets; `platform.md` § For the design process (Providers: how grants are
shown, how a state sentence and actions are laid out); `window-chrome.md`'s
split (whether "Show widgets" moves windows); the dashboard `wallpaper.md`
settled at gate 2; `control-centre.md`'s Controls widget. B14, B24, B28;
v1 D23, D24, D28–D31, D35, D36.

## Purpose

The widgets dashboard: a full-screen surface on one chord, drawn over the
softened wallpaper, holding the widgets the user placed. It is for
glancing at what is not worth a window: the time elsewhere, the weather,
the day's events, what is playing, the machine's load, batteries, disks,
and whatever a provider adds. It is a place the user goes to and leaves
(`design-v1-lessons.md` §5), the one widgets surface (`wallpaper.md` For
the user 1), and nothing of it shows at rest (brief P1). It is not
Control Centre (switches live there; the Controls widget draws the same
modules), not Notification Centre, not a launcher, not Show Desktop, and
it runs nothing while closed.

## Content

### The surface

- **Where.** The overlay layer, one of the surfaces that are open one at a
  time (`competition.md` Centre of the screen). It opens on one screen: the
  one holding the focused window, else the one under the pointer; from a
  hot corner, that corner's screen (as `spotlight.md`). Opening it on
  another screen closes it on the first.
- **Backdrop.** That screen's softened wallpaper (`wallpaper.md`), drawn
  opaque over the windows, so windows neither show through nor move.
- **The island.** Covered, as on the lock: the dashboard draws the island's
  privacy form and OSDs above itself in their place (`island.md`). The grid
  starts under the strip, so neither covers a widget. The clock is there to
  be placed as a widget.
- **Two controls** at the bottom centre, outside the grid: "Edit widgets"
  and a close control. In edit mode they read "Add widgets" (shows or hides
  the gallery) and "Done". They are the pointer's way out, since a click on
  a gap does nothing (Opening and closing).

### The grid

- **Cells of one fixed size**, the same on every screen, so a widget looks
  the same on the ultrawide and on the 1080p screen, and the ultrawide
  holds more of them. Stage 4 sets the size, finer than v1's 12 × 8 on the
  ultrawide (v1 D35): a row must be able to hold one line of reading, two
  batteries or one disk's bar (`design-v1-lessons.md` §3.3, §5).
- **The grid fills the screen** under the strip within a margin, laid out
  from the centre so the margin is even on both sides.
- **A widget is a rectangle of whole cells.** Each declares the sizes it
  supports, a default, a minimum and a maximum (built-ins in code,
  providers in their manifest, v1 D23); it is resized on whole cells within
  them (`shell.md` Widgets 3).
- **Nothing overlaps and nothing moves by itself.** The layout is the
  user's and changes only by an edit. A widget that has nothing to show
  keeps its place (Density, below).
- **Layouts per screen** (`shell.md` Widgets 3), keyed by the monitor's
  identity, not its connector, as the wallpaper is (v1 D56, D76), and kept
  in `settings.kdl`. A screen seen for the first time gets the default
  layout.
- **One canvas per screen**, no pages and no scrolling (For the user 1).
- **Text scale** leaves the cells as they are: content grows inside them,
  dropping secondary parts before cutting, down to the widget's one-line
  form. A larger text size never reflows the user's layout.

### Density and empty states

- **Defaults are the smallest size that reads** (v1 `87783c0`: one-row
  defaults for widgets that were mostly air).
- **Empty collapses.** A widget with nothing to show draws one line at the
  top of its place ("Not playing", "No devices", "No events today") and no
  card behind the rest, so its other cells show the softened wallpaper.
  Never a card of filler (`copy.md` Empty states; `design-v1-lessons.md` §8,
  Do not repeat: big cards of empty states).
- **Lists** inside a widget show the rows its size holds and end with
  "{n} more", whose click goes where the list belongs (Notification Centre,
  Files, Monitor). Nothing scrolls inside a glance.

### The built-in widgets

`shell.md` Widgets 2, with the registry's QML widgets folded in (below).
Every built-in fetches and samples only while the dashboard shows it
(`shell.md` Shell runtime 7; inventory §6 Polling).

| Widget | Shows | Several | Click or keys | Empty | From |
|---|---|---|---|---|---|
| Clock | The time in a zone with its city; digital or analogue face (the registry's analog-clock as the face setting); 24-hour as Date & Time sets | Yes | None | Not applicable | timedate1, tzdata |
| Calendar | The agenda: today, then the next days as the size allows; a month at its largest size; "Join" on meeting events | Yes (one per set of calendars) | An event's details in a popover | "No events today"; with no calendars, "No calendars" and "Add Calendar…" (Settings › Calendars; `conventions.md` Empty: one obvious first step) | `platform.md` Calendars |
| Weather | Now, high and low; then hours, then days, as the size allows; units from the region | Yes (one per place) | None | Not applicable | Open-Meteo, fetched by the host (v1 D31); the place defaults to the time zone's city |
| Media | The Now Playing module's player: art, title, artist, transport; the scrubber at larger sizes | No | Transport acts; the player's name raises it | "Not playing" | MPRIS, the same choice as Control Centre (`control-centre.md`) |
| Timer | A countdown with a label and presets; Start, Pause, Reset. New: the registry's focus-timer as a built-in | Yes | Its buttons | Not applicable | The timer service: a running timer is the island's Timer activity and its end the island's timer-end event (`island.md`), and it runs on with the dashboard closed |
| System | CPU, memory, GPU, temperatures, network, uptime, each as a number, a ring or a graph, with its own editor for series and styles (v1 D36; the registry's system-rings as a style, uptime as a series) | Yes | Opens Monitor (`monitor.md`) | Not applicable | services, sampled while shown |
| Devices | Every battery the Batteries module lists: name, level, charging | No | None | "No devices" | power, bluetooth (`control-centre.md`) |
| Storage | Each mounted volume: name, used and free as a bar | No | Opens it in Files (`files.md`) | Not applicable | UDisks2 |
| Desktops | This screen's desktops as rows of their windows' app icons, the current one marked (v1's Workspaces) | No | Switches to that desktop | Not applicable | compositor |
| Controls | Control Centre modules chosen in its settings (default Sound and Display), drawn by the same component with the same state: the same switches, not a second set (`control-centre.md` Shared places) | Yes | As the module | Not applicable | as each module |
| Session | Lock Screen, Sleep, Restart…, Shut Down…, Log Out…, and the Energy Mode module | No | Each asks as the system menu does (`copy.md` Log out) | Not applicable | session, power |
| Notifications | A glance: each app with waiting notifications by icon, name and count, newest first; no actions and no clearing (For the user 2) | No | Opens Notification Centre with that app's stack expanded | "No notifications" | the notification server |

A click that goes somewhere (raise the player, Files, Monitor, a desktop,
Notification Centre) closes the dashboard first and hands focus over
(`shell.md` Shell runtime 11).

**The default layout.** Clock, Calendar, Weather, Media, System, Devices
and Storage at their default sizes, packed as one block centred in the
middle half of the ultrawide, where the eye rests
(`design-references.md` §19), leaving the outer columns for the user. The
same block on the 1080p screen fills more of it. Timer, Desktops,
Controls, Session, Notifications and further clocks are in the gallery.

**The registry's widgets** (`shell.md` Widgets, Open; inventory §6),
proposed: analog-clock becomes the Clock's face, system-rings a System
style, transport the Media widget, battery the Devices widget, focus-timer
the Timer, uptime a System series. Each needs a face Isle draws or
controls no template has, so all six become built-ins or parts of them,
and the registry carries providers only (B14).

### Providers' widgets

A provider is a program in any language that emits JSON into one of the
built-in templates: text, number, gauge, sparkline, graph, list or card
(`shell.md` Widgets 4; B14). Its widget is a card holding the template,
with the provider's name as its title (a setting) and the sizes its
manifest declares. It runs sandboxed, on its interval or streaming, only
while its widget is visible, and is restarted if it dies (`shell.md`
Widgets 4; B28). Providers are display only here; anything they might do
on a click is the packet's (`shell.md` Widgets, Open: interactive
widgets).

| State | What the widget shows |
|---|---|
| Waiting for its first answer | Its title and nothing else; no spinner (`conventions.md` Loading) |
| Answering | The template with the latest data; the last data is kept and drawn first on the next opening |
| Stale | The last data, with "Updated {time}" once it is older than twice its interval |
| Failing | The last data, or with none one line: the reason as the provider gave it (`copy.md` rule 7); "Try Again" in its menu. A provider that keeps dying is left stopped with the same line |
| Its source not allowed | "{widget} isn't allowed to run." with "Allow…" (Installing and trust, below) |
| A permission it needs not granted | It runs and is denied inside the sandbox (`platform.md` Providers 5); its own error shows as Failing |
| Not installed | A layout naming a widget that is gone keeps its place and shows "{widget} isn't installed." with "Remove"; nothing is dropped silently |

### Edit mode

v1's grid editor and its soft rearrange (v1 D24, D35;
`design-v1-lessons.md` §5), with macOS's gallery.

- **Entering.** "Edit widgets" at the bottom; E; "Edit widgets" in a
  widget's or a gap's secondary-click menu; the wallpaper's "Edit
  widgets…", which opens the dashboard straight into edit mode
  (`wallpaper.md`); Spotlight's "Edit widgets".
- **What changes.** A faint grid of cells shows, so places can be read.
  Each widget shows a remove badge and resize handles on its edges and
  trailing bottom corner. The gallery opens (below).
- **Move.** Drag a widget. A live outline shows where it will land and the
  others make room with the soft push: neighbours flow live from the layout
  as it was before the drag began, so dragging back restores them, and a
  push cascades in the drag's direction (v1 `f19a4a0`, `108035f`,
  `4144694`, `06578cd`). Where nothing can be pushed further (the screen's
  edge), the outline shows the place is refused and the widget returns on
  release.
- **Resize.** Drag an edge or the corner, a whole cell at a time, between
  its minimum and maximum, pushing the same way.
- **Remove.** The badge, Delete, Remove in its menu, or a drag onto the
  gallery.
- **Settings.** A click on a selected widget, or "Edit “{widget}”…" in its
  menu, opens a popover beside it with its form, built from the manifest's
  settings by `settings.md`'s control types (a toggle is a switch, a
  choice a pop-up button, a number a number field, text a text field;
  `shell.md` Widgets 5). A built-in with its own editor (System, v1 D36)
  opens it in a larger panel centred on the dashboard.
- **Undo.** Cmd+Z and Shift+Cmd+Z undo and redo every move, resize, add and
  remove made since edit mode began.
- **Changes apply as they are made** (`apps.md` Settings 6). Nothing is
  pending, so no close route asks about unsaved edits (v1 asked on every
  route, `7ed0716`, because it had a pending layout).
- **Leaving.** Done, Escape (after any popover or panel), or closing the
  dashboard; the changes stay. "Reset to default" is in Settings › Desktop,
  as Control Centre's is in Settings (`control-centre.md`).

### The gallery

- **Where.** A sidebar on the leading side, the full height under the
  strip, over the grid's leading columns; "Add widgets" shows or hides it.
  While a widget is dragged out of it, it fades so the whole grid is clear,
  and returns after the drop.
- **Parts.** A search field, then three sections as a segmented control
  (`shell.md` proposal; v1's tabs):
  - **Installed**: the built-ins and the installed providers, grouped
    (Time and calendar, Weather, Media, System, Devices, Desktop, then
    each source's providers). Each item is a live picture at its default
    size, its name and its sizes. One that allows a single instance and is
    placed reads "Added". Updates waiting show as a count on the section.
  - **Yours**: the user's own providers, from their folder, with the
    author tools (`shell.md` Widgets 9; v1 D30): "New widget…" (makes the
    folder from a template and opens it in the user's editor), "Check",
    "Update picture", "Tag release…", "Show in Files".
  - **Browse**: each source's widgets, the default registry
    (`isle-widgets`) first; Install on those not installed, Update on
    those with a newer version. "Sources…" opens the sources panel: each
    source's address, when it was last pulled and any error, and its
    switch "Allow widgets from this source to run" (v1 D29); + adds a
    git address, − removes one. Sources are pulled on the worker
    periodically (hourly, as v1) and on opening Browse.
- **Adding.** A click adds the widget at its default size in the free place
  nearest the top centre of the screen, selected and outlined so the user
  sees where it went; a drag places it where it is dropped. With no room,
  the item says "No room for {widget}. Remove or resize a widget to make
  space." and a drag shows the place refused.
- **Details.** Space, or "Details" in an item's menu, opens a panel over the
  gallery: description, author, version, licence, source, sizes, the
  pictures or a live preview, the README and changelog, what it runs, and
  its permissions as rows with their state and "Change…" (Grants, below);
  Install, Update or Remove.

### Installing and trust

`shell.md` Widgets 6–8; B28; v1 D28, D29.

- **Install** from Browse opens a consent panel on the dashboard: "Install
  “{widget}”?", what it runs (its command), each permission it declares in
  words, and its source. For the first install from a source not yet
  allowed, one more line: "This also allows other widgets from {source} to
  run." Buttons "Install" and "Cancel". Install grants the declared
  permissions and allows the source, then installs through staging and
  swaps in whole (v1 D29).
- **Update.** One that declares no new permission installs on Update
  without asking; one that adds permissions shows the panel with only the
  new ones.
- **A source turned off** in Sources keeps its providers installed and does
  not run them (`shell.md` Widgets 8); their widgets say so with "Allow…",
  which opens the same panel for the source.
- **The user's own providers** need no source consent; their declared
  permissions start ungranted, and the widget's "Allow…" opens the panel
  listing them with "Allow" and "Cancel" (`shell.md` Widgets 8).

### Grants (`platform.md` Providers)

- **As switches**, one per declared permission, labelled in words, not
  in the manifest's vocabulary: "Connect to api.open-meteo.com", "Read
  ~/Documents", "Read and change ~/Notes", "Use org.kde.kdeconnect", "Run
  nmcli". On is granted.
- **In one place**: Settings › Privacy & Security › Providers
  (`settings.md` puts providers' permissions there), a list of every
  provider of every kind (widgets, vaults, VPNs, snapshots, phones) by name
  and kind, each a page with what it runs, its source and that source's
  allow switch, and its permissions as switches. The gallery's details,
  Settings' provider panes and Keychain show the same permissions read
  only, with "Change…" to that page: one switch per thing in one place
  (`design-v1-lessons.md` §8).
- **Revoking** denies the permission inside the sandbox and leaves the
  provider running (`platform.md` Providers 5); what it can no longer do
  shows as its own error.

### The provider row (`platform.md` Providers)

One layout wherever a provider's state sentence and actions are drawn:
Settings' provider panes (`settings.md` Rows, Provider; `apps.md`
Settings 11), Control Centre's VPN and Phone pages (`control-centre.md`),
Keychain's providers (`keychain.md`), and Spotlight's locked-vault row
(`spotlight.md`).

- **Leading**: the provider's name, and under it the state sentence as the
  provider gives it (`status`), one line cut at the end, whole in the
  tooltip and the accessible description. The last sentence stays while a
  new call runs; nothing reads "Loading".
- **Trailing**: the switch that turns the provider on or off, where the
  place has one (`apps.md` Settings 11).
- **Under the sentence**, while on: the actions as buttons in the
  provider's order, up to two, the rest under an "Options" pop-up, as on a
  banner (`notifications.md`). An action marked `terminal`, and a sign-in,
  end in an ellipsis, since each asks for more before acting (`copy.md`
  rule 6). A running action's button is disabled until it returns.
- **Its `list`**, where it has one (connections, vaults, snapshots), as rows
  under it that choose or act, as Control Centre's VPN page (v1
  `921f032`).
- **An error** replaces the sentence with the reason the provider wrote,
  cut to one line, and keeps the actions; a provider that overran its time
  limit reads "{provider} didn't answer." (`copy.md` rule 7;
  `platform.md` Providers 4).
- **Secondary click**: "Permissions…" (its Privacy & Security page), and
  "Show in Files" for the user's own.

## States

| State | What shows |
|---|---|
| Closed | Nothing. No provider runs and nothing samples; a running timer runs on as the island's activity. |
| Open | The grid over the softened wallpaper, the two controls; keyboard focus on the first widget in reading order, the ring only after a key. |
| Edit mode | The faint grid, badges and handles, the gallery, "Add widgets" and "Done". |
| Dragging, resizing | The live outline and the soft push; the gallery faded while a widget is dragged from it. |
| Empty | No widgets: one line, "No widgets", centred, and "Edit widgets". |
| Full | No free place: the gallery's "No room" line; a drag shows the place refused. |
| Loading | Every widget draws its data from the last showing in the first frame, then refreshes; one with none shows its title only; nothing waits (`shell.md` Shell runtime 8). |
| Error | Per widget, as above. A panic in the dashboard rebuilds it open, in edit mode if it was (`shell.md` Shell runtime 2, 3); whether one widget's panic can be contained to that widget is the packet's. A sources pull that fails leaves Browse with the last good index and the error in Sources. |
| Many widgets | One canvas: no pages, no scrolling. |
| Second screen | Its own layout. One dashboard open at a time; it opens on the screen with the focused window, else the pointer's. |
| Game in front, windowed | The chord is not in B30's escape set and goes to the game; a hot corner opens it beside a windowed game. Open, it is solid and without transitions (`competition.md` For the user 1), and its widgets run as normal while shown. |
| Full-screen game | Not opened: the chord is the game's and hot corners never fire over a full-screen game. Cmd+Tab out first. |
| Full-screen window, not a game | Opens over it, on that screen. |
| Shortcuts inhibited | The chord goes to the inhibiting window; a hot corner still opens it. |
| Locked | Closed: locking closes it. |
| Focus on | No change. |
| Reduce Motion | Opens and closes by a fade; the soft push's moves become cuts while the outline still shows where things land. |
| Reduce Transparency | The softened wallpaper is a picture and stays; cards, the gallery and panels are solid. |
| Increase Contrast | Edges on cards, the outline, handles and the gallery's items. |
| Largest text size | Cells keep their size; content drops parts before cutting, down to each widget's one-line form; the gallery and panels grow. |

## Opening and closing

- **Opens** on the widgets chord (Keyboard), a hot corner set to Widgets
  (none by default, `dock.md` For the user 3), Spotlight's "Widgets" and
  "Edit widgets", the wallpaper's "Edit widgets…" (in edit mode), and the
  CLI and D-Bus (`shell.md` Shell runtime 16).
- **Motion.** The softened wallpaper fades in over the windows, and the
  widgets fade in with a small scale about their own places; nothing
  slides in from an edge (brief P5). Closing reverses it, faster. Over a
  game, none; under Reduce Motion, a fade.
- **Closes** on Escape (after leaving any popover, panel or edit mode),
  the chord again, the close control, the hot corner that opened it, Cmd+Tab,
  any other overlay opening, locking, and an action that goes elsewhere.
- **Never closes** on a click on a gap or on a widget's body, or on the
  pointer leaving anything (v1 `8db75f9`, `52e7b89`;
  `design-v1-lessons.md` §8, Do not repeat). Edits are already applied, so
  no close route asks.
- **Focus** is taken on opening and given back on closing to the window
  that had it, before any action the dashboard started reaches it
  (`shell.md` Shell runtime 11).

## Keyboard

| Action | Key |
|---|---|
| Open or close | The widgets chord, Ctrl+Option+Cmd+W (decided at gate 2), with Notification Centre's Ctrl+Option+Cmd+N and Do Not Disturb's Ctrl+Option+Cmd+D. v1's Super+A and Super+N cannot carry: Super is Cmd, and Cmd+A and Cmd+N are the app's Select All and New (B21) |
| Move between widgets | Arrows, by position; Home, End |
| Through a widget's controls, then on to the next widget | Tab, Shift+Tab |
| The widget's primary control (play or pause, start a timer, a module's switch) | Space |
| What the widget opens (the player, Files, Monitor, an event's details, a desktop) | Return |
| A widget's menu | Shift+F10, or the menu key |
| Edit mode | E, or "Edit widgets" |
| Edit: select a widget | Arrows |
| Edit: move it a cell, pushing as a drag does | Cmd+arrows (v1 D24's nudges) |
| Edit: resize it a cell | Option+Cmd+arrows: Right and Down grow, Left and Up shrink, within its sizes |
| Edit: its settings | Return |
| Edit: remove it | Delete (⌫) |
| Edit: undo, redo | Cmd+Z, Shift+Cmd+Z |
| Edit: to the gallery and back | Tab |
| Gallery: search | Typing, or Cmd+F |
| Gallery: Installed, Yours, Browse | Cmd+1, Cmd+2, Cmd+3 |
| Gallery: move between items | Arrows |
| Gallery: add the item | Return, in the free place nearest the top centre |
| Gallery: its details | Space |
| Leave edit mode | Escape, or Done |
| Close | Escape; focus returns to the window that had it |

The chord is a desktop chord, taken before Cmd translation (B21) and given
up under keyboard-shortcuts-inhibit. E, the arrows and the edit keys act
only while the dashboard has focus. The dashboard is in the accessibility
tree as a grid of groups, each widget named and its parts readable.

## Pointer

| Input | On | Does |
|---|---|---|
| Hover | A widget | Highlights it only when the pointer moves; opens nothing |
| Click | A gap | Nothing; in edit mode, clears the selection |
| Click | A widget's control | Acts |
| Click | A widget's body | What it opens, where it has one (Return's action); else nothing |
| Click | Edit mode: a widget; a selected widget | Selects it; opens its settings popover |
| Drag | View mode | Only within a widget's own controls (a slider, the scrubber): a widget never moves outside edit mode, so dragging a slider cannot move its widget |
| Drag | Edit mode: a widget, an edge or corner, an item from the gallery, a widget onto the gallery | Moves, resizes, adds, removes |
| Secondary click | A widget | "Edit “{widget}”…", its sizes, "Remove widget", "Edit widgets" |
| Secondary click | A gap | "Edit widgets", "Close widgets" |
| Secondary click | A gallery item | "Add", "Details", and "Remove" or "Update" where they apply |
| Scroll | Anywhere | Nothing, except inside a gallery or panel list |

## Shared places

What this sheet adds to `competition.md`:

- **Centre of the screen and the overlay.** The dashboard stays one of the
  one-at-a-time overlays (`competition.md`), covering its whole screen,
  strip included.
- **The island over the dashboard.** New row: the dashboard draws the
  island's privacy form and OSDs above itself, as the lock does; Control
  Centre and Notification Centre cannot open over it and close when it
  opens; other events wait until it closes and are dropped past their
  duration (`island.md` The queue 8); activities show again when it
  closes.
- **Show Desktop and "Show widgets".** Settles `window-chrome.md`'s split:
  the dashboard moves no window, so there is no "Show widgets" move. Show
  Desktop keeps its move and reveals the plain wallpaper (`wallpaper.md`);
  the widgets have their own chord.
- **Banners.** Wait while it is open (`notifications.md`, unchanged).
- **System prompts** sit above it with their scrim and take focus; the
  dashboard stays open beneath.
- **Capture.** The freeze pick (Shift+Cmd+4) can shoot it; the capture
  toolbar closes it, as one overlay at a time.
- **Hot corners.** "Widgets" is a corner action; the corner that opened it
  closes it; none carries it by default.
- **Keys.** Ctrl+Option+Cmd+W (decided at gate 2) for P5's table; not in
  B30's escape set.
- **Two views of one thing, never two switches.** The Controls widget draws
  Control Centre's modules; the Timer widget's running timer is the
  island's Timer activity; the Media widget follows the Now Playing
  module's player; the Calendar widget and Notification Centre's agenda
  read the same calendars; the Notifications widget opens Notification
  Centre and holds no second list. Control Centre and the
  dashboard are never on screen together.
- **Settings.** "Reset to default" for the layout in Settings › Desktop
  (`settings.md` places the widgets dashboard there); grants in Privacy &
  Security › Providers.

## Answers

1. **Whether widgets also sit in Notification Centre's column** (`shell.md`
   Widgets). **No**: the dashboard is the one widgets surface
   (`wallpaper.md`); Notification Centre carries the date, month and
   agenda instead (`notification-centre.md` For the user 2).
2. **Whether the Notifications widget is dashboard only** (proposal: yes).
   **Changed**: every widget is dashboard only, and this one becomes a
   glance that opens Notification Centre rather than a second list. For
   the user 2.
3. **The gallery's sections** (proposal: Installed, Yours, Browse).
   **Adopted**, with the built-ins under Installed, the author tools under
   Yours, and Sources and their trust switches under Browse.
4. **Whether v1's dashboard pages survive as several layouts.**
   **Replaced**: one canvas per screen, with finer cells. For the user 1.
5. **Widgets 1, one chord whatever is open** (v1's dashboard chord).
   **Changed**: v1's Super+A cannot carry under the Mac model, so it is a
   desktop chord, Ctrl+Option+Cmd+W, outside the escape set; a corner
   action and Spotlight reach it too.
6. **Widgets 2, the built-ins.** **Adopted**, with the Timer added from the
   registry, Desktops kept, Controls drawing Control Centre's modules,
   Session as the session actions and Energy Mode, and Notifications as a
   glance (For the user 2).
7. **Widgets 3, the grid and its editor, per screen.** **Adopted**, with
   cells of one fixed size on every screen, the soft push from the
   pre-drag layout, Cmd+arrow nudges, Option+Cmd+arrow resizing, undo, and
   layouts keyed by monitor identity.
8. **Widgets 5, a widget's settings.** **Adopted**: a popover built from the
   manifest by `settings.md`'s control types; System's own editor in a
   panel.
9. **Widgets 6–9, gallery, sources, consent and author tools.**
   **Adopted** as Installing and trust above: consent once per install and
   per new permission, the source allowed by its first install, an
   un-allowed source's widgets installed and not run, author tools in
   Yours.
10. **The registry's QML widgets** (`shell.md` Widgets, Open for its
    packet). **Proposed**: all six become built-ins or parts of them; the
    registry carries providers only.
11. **Density and empty states** (`design-v1-lessons.md` §3.3, §5, §8).
    **Adopted**: smallest-that-reads defaults, finer cells, empty widgets
    as one line with no card.
12. **Accidental closes** (v1 `8db75f9`, `52e7b89`). **Adopted** as a rule: a
    gap or a widget's body never closes the dashboard; the pointer's way
    out is the close control or the corner.
13. **Providers: how grants are shown** (`platform.md`, proposal:
    toggles). **Adopted** as switches labelled in words, in one place,
    Privacy & Security › Providers, read only with "Change…" everywhere
    else.
14. **Providers: how a state sentence and actions are laid out**
    (`platform.md`). **Answered** as the provider row: name and sentence
    leading, the on switch trailing, up to two action buttons and Options,
    the list as rows, errors in the sentence's place.
15. **Stacking: whether "Show widgets" moves windows** (`window-chrome.md`
    split; `compositor.md` B24). **Replaced**: the dashboard moves no
    window; Show Desktop alone uses the move.

## Copy

| Where | String |
|---|---|
| Controls | "Edit widgets", "Add widgets", "Done"; close control's accessible name "Close widgets" |
| Menus | "Edit “{widget}”…", "Remove widget", "Edit widgets", "Close widgets", "Add", "Details", "Remove", "Update"; sizes as cells, "2 × 1" |
| Empty | "No widgets"; "Not playing", "No devices", "No events today", "No notifications", "No calendars" with "Add Calendar…" |
| Lists | "{n} more" |
| Widget names | "Clock", "Calendar", "Weather", "Media", "Timer", "System", "Devices", "Storage", "Desktops", "Controls", "Session", "Notifications" |
| Timer | "Start", "Pause", "Reset"; "{label} done" (the island's, `island.md`) |
| Session | "Lock Screen", "Sleep", "Restart…", "Shut Down…", "Log Out…" (`copy.md` rule 1) |
| Provider widget | "Updated {time}", "Try Again", "{widget} isn't allowed to run.", "Allow…", "{widget} isn't installed.", "Remove" |
| Gallery | "Search widgets", "Installed", "Yours", "Browse", "Added", "Install", "Update", "Remove", "Sources…", "New widget…", "Check", "Update picture", "Tag release…", "Show in Files"; "No room for {widget}. Remove or resize a widget to make space."; groups "Time and calendar", "Weather", "Media", "System", "Devices", "Desktop" |
| Sources | "Allow widgets from this source to run", "Last updated {time}", "Couldn't update {source}. {reason}" |
| Consent | "Install “{widget}”?", "It runs {command}.", "It asks to:" then the permissions; "This also allows other widgets from {source} to run."; "Install", "Cancel"; for new permissions "“{widget}” now asks to:", "Update", "Cancel"; for the user's own "Allow “{widget}” to:", "Allow", "Cancel" |
| Permissions | "Connect to {host}", "Read {path}", "Read and change {path}", "Use {bus name}", "Run {command}"; "Change…" |
| Provider row | The provider's own sentence; "Options"; "{provider} didn't answer."; "Permissions…" |
| Settings › Desktop | "Widgets", "Reset to default" |
| Settings › Privacy & Security | "Providers"; a provider's page titled with its name |
| Spotlight actions | "Widgets", "Edit widgets" |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Pages or one canvas.** (a) One canvas per screen, with cells finer
   than v1's; (b) v1's pages, switched by keys, a swipe and page dots.
   *Recommended: (a).* Finer cells on the ultrawide hold more than a v1
   page did; pages add dots (the user dropped desktop dots from the
   island), keys and places that are out of sight; macOS's widgets have
   none; and v1's widget work stopped two days after it began, which
   suggests the dashboard was glanced at rather than organised
   (`design-v1-lessons.md` §5, an inference).
2. **The Notifications widget.** (a) A glance: each app with waiting
   notifications by icon, name and count, opening Notification Centre at
   that app's stack; (b) no Notifications widget; (c) v1's full list with
   actions and clearing. *Recommended: (a).* It keeps v1's capability of
   seeing what is waiting from the dashboard (`shell.md` parity map)
   without a second list to act in, so where notifications live stays
   settled (`design-v1-lessons.md` §3.4: moved three times, and the
   widget's count once read "All clear" over a full list).
