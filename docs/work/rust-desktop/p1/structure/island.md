# Island

Batch 1, revised by revision 1. Answers to: `shell.md` Island, OSDs, Menu
bar 6, 7, 9 (status items and the clock, which move here), Control Centre 1
(where it opens from), System prompts (where a waiting prompt is
announced), Capture (where the recording timer shows), Shell runtime
(which screens carry the island); `platform.md` Portal backend (where a
running cast is shown), Calendars, Updates (restart owed), Focus (in part),
Settings (the rejected hand edit); `compositor.md` Keyboard-shortcuts-inhibit
(where it shows), Shell surface hosting (the island's layer); `gaming.md`
Game Mode (in part); B18, B19, B24, B30.

## Purpose

The one thing on screen at rest besides the wallpaper (brief P1): v1's
pill at top centre, with the clock and a glyph only for what is not
normal. It grows to show what is happening now that the user may need to
see or stop: privacy indicators, live activities and the shell's own brief
acknowledgements, OSDs included. It is the way into Control Centre (on
hover) and Notification Centre (from the clock). It is not a notification
surface (banners and Notification Centre hold everything that arrives
through the notification server), not a launcher, shelf or desktop
switcher, and not the menu bar (`menu-bar.md`, which appears on demand
around it).

## Content

The pill's row, leading to trailing: the leading live activity's compact
face and the other activities' marks; the clock; the status glyphs that
are showing; the privacy marks. The pill is centred on the screen and
sized to what it holds.

**The clock.** Always shown. Time only by default, 24-hour on this
machine; weekday, date and seconds are settings (`shell.md` Island 9;
For the user 1). Its width does not change as the time does, so nothing
beside it moves. No seconds by default (`shell.md` acceptance bar 2).
Source: timedate1 and the clock setting.

**Status glyphs.** One per state, in this fixed order after the clock,
each from its service crate (B11). Each has a tier, set per glyph in
Settings › Control Centre: *Always show*, *Show when active* (the default
for all of them), *Never show*. "Active" is defined so that the normal
state shows nothing (brief P1; v1 lessons §5, the conditional-glyph rule).

| Glyph | Active means | Click opens Control Centre at |
|---|---|---|
| Wi-Fi | On Wi-Fi, or no connection at all; wired and connected is normal | Wi-Fi detail |
| Bluetooth | A device is connected | Bluetooth detail |
| Sound | The output is muted | Sound detail |
| VPN | A VPN is connected or connecting | VPN detail |
| Focus | A Focus is on, the Gaming Focus included; the glyph is that Focus's symbol | Focus detail |
| Updates | An update is installing, or a restart is owed (two forms of one glyph) | Updates detail, with the restart-owed reasons |
| Battery | The machine has a battery and is on it, or a battery is low; absent on a desktop | Battery detail |
| Shortcuts inhibited | A window holds a keyboard-shortcuts inhibitor | its notice row |
| Configuration error | `settings.kdl` or `tokens.kdl` holds a rejected edit, or Isle started without them after they stopped it starting (the safe start, `prompts.md`) | its notice row |

The last two are not tiered: they show while their state holds, because
the user cannot otherwise tell why chords do nothing or settings will not
save or are not in force. Updates waiting are not a glyph: they are
routine, counted in the system menu (`menu-bar.md`) and in Control
Centre's Updates module. The
keyboard layout is not a glyph: a change is an OSD and the layout is in
Control Centre. Other programs' status items never show in the pill; they
are Control Centre's tray (below).

**Privacy indicators.** One mark per kind in use: microphone, camera,
screen (recording or sharing), at the pill's trailing end, inside its
shape, so they travel with every form, stay visible over full screen and
are never hidden by an open panel. Shown on every screen while the
capture runs; never dismissed, covered or displaced (`shell.md` Island 4).
Sources: the audio service's capture streams for the microphone, with the
app (`platform.md` services, audio); PipeWire camera sources in use for the
camera (what an app opening the device directly shows is the services
packet's); the compositor's capture sessions for the screen, both portal
streams (B13) and capture-protocol clients.

**Live activities.** Last as long as their source; one activity per kind.
The leader's compact face sits before the clock; the others add a mark
each beside it.

| Kind | Source | Compact face | Row in Control Centre |
|---|---|---|---|
| A prompt waiting | the shell's prompt queue (B12, B28): polkit, keyring, Wi-Fi and VPN secrets, Bluetooth pairing | the prompt's kind | who asked, for what; Show, Cancel |
| Recording | the capture tool | the elapsed time | elapsed time, what is recorded, audio or not; Stop |
| Sharing | the portal backend's ScreenCast streams (B13) | the sharing app | each stream: app, screen or window, audio; Stop per stream |
| Call | the phone provider's active call (B14, inventory §8) | the elapsed time | caller, time; the provider's actions |
| Timer | a widget or provider that runs one (B14) | the time left | label, time left; the source's actions |
| Media | MPRIS, the most recently playing player (`platform.md` media) | art and a playing mark | art, title, artist, transport, scrubber, the player's name |

Recording and sharing are both a privacy indicator and an activity: when
one leads, its compact face stands in for the screen mark rather than
repeating it. A prompt waiting is an activity only while its dialog is not
on screen (behind a full-screen window or game, or queued behind another
prompt; `shell.md` System prompts 5, 6). Desktop calls (a browser or chat
app) have no shared source and show through the microphone and camera
indicators with the app's name.

**Events.** Transient acknowledgements, where the user would otherwise
not see the change. An event replaces the activity face, the clock and the
glyphs for its duration; the privacy marks stay.

| Event | Shown when | Duration by importance (v1 §5 values as the start; stage 4 sets them) |
|---|---|---|
| OSD: volume, mute, microphone mute, brightness, keyboard brightness, keyboard layout | any key, device or app changes the level (`shell.md` OSDs 1); shows the level and the device or monitor | about 1.5 s after the last change |
| Copied | the shell copied something: colour pick, a Keychain or Spotlight copy, capture to the clipboard | about 2 s |
| Focus, Do Not Disturb, Game Mode on or off | the change was automatic (schedule, detected game) or made by chord; not when made in Control Centre | about 2 s |
| An app asks for attention | an activation request without focus, or an urgent hint (revision 1: attention moves off the Dock) | about 3 s; an urgent request until the app is focused or the event clicked |
| An activity starts | a new activity that leads: its row form briefly, then compact | media about 4 s; others about 3 s |
| An activity ends with a reason | a recording or share ends because its source went (`shell.md` Capture 8) | about 5 s |
| A timer ends | holds until clicked, with the alert sound if sounds are on | until acknowledged |

**Control Centre, opened from the island.** The panel unfolds downward
from the pill (v1's panel, laid out the macOS way; revision 1). Its top row
is the pill's own row, so the clock and glyphs stay where they were and
stay clickable. Then, in order:

1. **Now.** Only while something is live or a notice holds. Every live
   activity as a row (the table above), the leader first and in full, the
   others collapsed and expanding in place, media only when the Now
   Playing module has been removed from the grid, since that module is
   media's one place in the panel (`control-centre.md`); privacy rows
   naming each app using the microphone, camera or screen; and two notice
   rows: the configuration error (the message, file, line and column; Open
   in editor, Show in Files; after a safe start, `copy.md`'s safe-start
   line with Use my settings and Report…, `prompts.md`) and shortcuts
   inhibited (the app that has the keys, the escape chord, Take keys
   back).
2. **Modules.** Control Centre's grid, a module taking one, two or four
   cells (`design-macos.md` §3.7). Which modules exist, their grouping,
   order and defaults are `control-centre.md`'s (batch 2). A module's
   detail opens as a page in the panel with a way back (v1's drill-down
   pages; `shell.md` Control Centre 3).
3. **Tray.** Other programs' StatusNotifierItems as icons, in the user's
   order, those asking for attention marked. A menu opens as a page in the
   panel, as v1's did (`4eb009c`). Absent when no program has one.
4. **Footer.** Edit controls (the gallery editor, `control-centre.md`) and
   Control Centre settings….

**Elsewhere, not in the island.**

| What | Where |
|---|---|
| Notifications, including the desktop's own (drive inserted, calendar start, update finished, AppImage installed) | banners and Notification Centre (later batch); the island never shows a notification |
| File jobs, downloads, an app's long work | the switcher's cards (revision 1, LauncherEntry); its finish is a notification |
| Updates waiting | the system menu's count and Control Centre's Updates module |
| Calendar events | Calendar widget; a start is a notification (`platform.md` Calendars) |
| Desktops | Mission Control's Spaces bar; no desktop dots (brief, interview) |
| A screenshot taken, a recording saved | the capture thumbnail (capture sheet) |
| An ordinary Cmd+C in an app | nothing; the app is the feedback |

## The reserved strip

A strip across the top of every screen belongs to the island: no window
sits under it. Maximise, Fill and every zone stop at its lower edge, a
window's bar is never dragged into it, and new windows are placed below it
(v1 kept a top gap, 48 px, as the start; stage 4 sets it). A full-screen
window covers it. The on-demand menu bar comes down into it
(`menu-bar.md`). The bottom edge belongs to windows (revision 1).

## States

| State | What shows |
|---|---|
| Rest | The pill: the clock alone, or the clock and the glyphs that are active. |
| Rest, privacy | The same, with the privacy marks trailing. |
| One live | The activity's compact face before the clock; glyphs and privacy marks as at rest. |
| Several live | The leader's face, then one mark per other activity in queue order, then the clock, glyphs and privacy marks; past a cap (stage 4) the other activities become a count. |
| Event | The event's form fills the pill for its duration, privacy marks kept; then the island returns to its live state. |
| Control Centre peeked | Hover: the panel as above, without keyboard focus. |
| Control Centre pinned | A click: the same panel, staying when the pointer leaves, with keyboard focus. |
| Empty | Nothing is empty: the clock is always there, and hover always opens Control Centre. |
| Loading | A face whose data is late (media art) draws without it; a module waiting for its service shows its last state; nothing waits. |
| Error | A dead or slow service's glyph, activity or module is absent or stale, never a stalled frame (shell runtime 8); a share whose source vanished ends with its "ended" event. |
| Overflow | The pill never exceeds a maximum width, which is what the menu bar reserves for it. Past it, text in the leader's face shortens, then drops to the kind's mark, then the other activities' marks become a count; the clock, the glyphs and the privacy marks never drop. The panel grows downward and scrolls past a fixed height. |
| Second screen | An island on every screen, each with the clock, its glyphs and the privacy marks (v1 D49). Compact faces and events only on the main screen (shell runtime 14); a monitor's brightness OSD on that monitor. Hover opens Control Centre on the screen it is on, Now included. |
| Game in front, windowed | As normal (interview), in its solid form and without transitions (`competition.md`, decided at gate 2). The Focus glyph shows the Gaming Focus. |
| Full screen, game or not | The island hides with the strip. Its privacy form stays at top centre while a capture runs: the shape shrunk to the marks. OSDs show briefly in the same place. Activities wait; other events are dropped, not held. A push at the top edge inside the island's zone brings the island back (over a game, a dwell, never while the game holds the pointer locked or confined; `gaming.md` Game Mode 5); outside the zone the menu bar comes with it (`menu-bar.md`). Over a game nothing transitions or is blurred (Game Mode 9). With no capture running the island contributes no render element (acceptance bar 13). |
| Locked | The lock draws only the island's privacy form and OSDs, in the same place; the lock has its own clock. Events are dropped; activities resume at unlock; a prompt raised while locked waits (System prompts 7). |
| Focus on | The Focus glyph. Focus governs banners and sounds, not live activity. |
| Shortcuts inhibited, configuration error | Their glyph, and their notice row in Now. |
| Reduce Motion | No morph: the shape takes its new size at once and content changes at once or by a short fade (stage 4, shell runtime 9). |
| Reduce Transparency | The pill is solid already; the panel loses any blur behind it. |
| Increase Contrast | An edge around the pill and the panel (stage 4). |
| Largest text size | The pill grows with the text, within its maximum, dropping activity text before marks; the panel grows. |

## Opening and closing

- **Growth.** The island's size follows its state: rest, compact, event,
  panel. A glyph appearing or leaving is a size change too. Every change
  is a compositor morph of one shape (B7, shell runtime 5;
  `shell-ui-spike.md`: the compositor springs size and radius, content is
  clipped to the shape).
- **Anchored.** The top edge and the horizontal centre never move; growth
  is outward and downward only, and a size change never re-centres or
  slides (brief P5; v1 lessons §5, `d1af0a8`). The menu bar coming or
  going does not move it.
- **Content with the shape.** Content is clipped to the shape and its
  opacity follows the growth rather than running on its own clock (v1
  lessons §5). A tall form's corner radius is capped so text stays out of
  the curve (`6680fe6`).
- **Character.** Critically damped, no bounce; shrinking is faster than
  growing (brief P5). A change mid-morph retargets from the current shape.
  A face that changes at the same size crossfades with no morph.
- **Peek** opens Control Centre after a dwell on the island and closes it
  after a grace once the pointer leaves. **Pinned** comes from a click,
  Return or Ctrl+F8 and closes on Escape, a click outside, Ctrl+F8 again or
  opening another top-layer surface.
- **Notification Centre** opens from the clock and closes the panel; the
  panel and Notification Centre are never open together on one screen.
- **Over a full-screen window** the island leaves with the strip and comes
  back with a top-edge push, at the same anchor; the privacy form does not
  move.
- **Minimise** shrinks a window toward the island (revision 1); the island
  does not change for it.

## Keyboard

| Action | Key |
|---|---|
| Open Control Centre, pinned, with focus on its first control | Ctrl+F8 (macOS's "move focus to status menus") |
| Move through the top row (activity, clock, glyphs) | Left, Right |
| Open Notification Centre | Return or Space on the clock |
| Open a glyph's detail or notice | Return or Space on the glyph |
| Move between sections and controls | Tab, Shift+Tab |
| Act on a row (Stop, Show, Cancel) | Return or Space |
| In the module grid: move between modules; a module's switch or action; its page | Arrows; Space; Return (`control-centre.md`) |
| Open a row's sub-page or a tray item's menu page; back | Right; Left or Escape |
| Close | Escape at the top page; focus returns to the window that had it (shell runtime 11) |
| Stop a recording | the capture sheet's chord (proposal: Cmd+Ctrl+Escape, as macOS) |
| Media and volume | the media and volume keys, which raise OSDs |

A peek never takes keyboard focus, so typing continues in the window.
Ctrl+F8 is a desktop chord, taken before Cmd translation (B21) and given
up under shortcuts-inhibit.

## Pointer

- **Hover** over the island peeks Control Centre. The island's zone is its
  shape, the strip above it to the screen's top edge (so a pointer thrown
  to the top centre lands on it) and a guard gap each side. It never
  extends over a revealed menu bar's titles (`design-references.md` §2,
  notch apps' shared failure). The top edge outside the zone reveals the
  menu bar (`menu-bar.md`).
- **Hover guards**, as requirements (v1 lessons §3.5; values stage 4's,
  v1's as the start):
  - a dwell before the peek opens (v1 120 ms);
  - a grace before it closes (v1 250 ms);
  - the hover held while the panel's own size settles after a click (v1
    500 ms);
  - an event waits while the panel is peeked or pinned (`shell.md`
    Island 7). An OSD raised by a key moves the matching module's slider
    in place, or shows as a line at the panel's foot when no module shows
    that level; one raised by the panel's own controls is dropped;
  - a click that opens something elsewhere (Show, the player's app,
    Notification Centre, a module's Settings page) folds the panel and
    disarms hover until the pointer leaves the island;
  - inside the panel, hover selects a row only when the pointer moves;
  - after a top-edge return over a full-screen window, a peek needs the
    pointer to move first, so the return and the peek never fire from one
    dwell;
  - while a menu bar menu is open or a window is being dragged, the island
    takes no hover.
- **Click** on the clock opens or closes Notification Centre on this screen.
  A click elsewhere on the pill, or inside a peeked panel, pins Control
  Centre with keyboard focus; on a glyph it pins it at that glyph's detail
  or notice. A click on a control acts at once (Stop, a toggle, play). A
  click on an activity's app or prompt goes where it belongs: Show brings
  the prompt forward, the player's name raises the player, a share's app
  name raises the app (`shell.md` Island 6). A click on an attention event
  raises the app. A click that also reached something else never opens the
  panel.
- **Tray items**, as the item asks: click opens its menu page if it is a
  menu, else Activate; secondary click its menu page; middle click
  SecondaryActivate; scroll passes to the item (`shell.md` Control Centre 9).
  Cmd+drag reorders them within the tray.
- **Secondary click** on a compact activity: Hide (until that activity
  stops and starts again; not offered for privacy, recording, sharing or
  prompts). On a glyph: its tier (Always show, Show when active, Never
  show), where it has one.
- **Scroll** on the pill does nothing; in the panel it scrolls. **Drag**:
  sliders, the media scrubber and tray reordering; nothing is dropped on
  the island.

## Shared places

- **The top strip: the island against windows.** The strip is the
  island's on every screen; windows, zones and placement stop below it. A
  full-screen window covers it, and the island hides.
- **Top centre with the menu bar shown: the island against the bar.** The
  island stays where it is, in front of the bar; the bar reserves the
  island's maximum width plus the guard gaps, and app menus that would
  reach it fold into the bar's overflow (`menu-bar.md`). Neither covers
  the other's items.
- **The top edge: the island's zone against the menu bar's reveal.** Inside
  the island's zone the edge belongs to the island; outside it, to the
  reveal. During a window drag, to the fill target (`window-chrome.md`).
- **Below the pill: the panel against menus, Notification Centre,
  Spotlight and window content.** Control Centre, Notification Centre and
  the menu bar's menus are top-layer surfaces: one open per screen,
  opening one closes the other. Spotlight, Mission Control, the capture
  toolbar and the session dialog close the panel (shell runtime 10); the
  freeze pick (Shift+Cmd+4) closes nothing, so a shot can include the
  panel (`capture.md`). System
  prompts close nothing and take focus over it.
- **OSDs.** The island, the only OSD (v1). macOS 26's capsule hangs under
  the Sound item, which no longer exists.
- **The island's queue against notification banners.** Anything from the
  notification server is a banner; the island carries only privacy, live
  activity and the shell's own acknowledgements, attention included. Both
  may show at once, in their own places.
- **Privacy indicators against Control Centre.** The marks in the pill are
  the indicator; Now names the apps; no second mark.
- **Media against Control Centre's Now Playing module.** The module is
  media's one place in the panel; Now carries a media row only when the
  module has been removed from the grid (`control-centre.md`). The pill's
  compact media face stays as above; it opens the module's page, as a
  glyph opens its module's.
- **Status state: glyphs against modules.** A glyph is a module's state
  shown at rest; its click opens that module. Toggles and sliders exist
  only in Control Centre (`shell.md` Control Centre, v1 D3).
- **The top edge over a full-screen window.** Privacy and OSDs show
  without any reveal; a push inside the island's zone brings the island
  back, outside it the menu bar with the island; a peek needs movement
  after either.
- **Attention: the island against the switcher.** An attention request is
  an island event and a mark on the app's switcher card (revision 1).
- **Attention against a banner from the same app.** The banner wins: the
  app's non-urgent attention event is suppressed for the banner's lifetime,
  as chat apps both notify and ask for attention for one message
  (`notifications.md`). The switcher's mark still holds; an urgent request
  still shows.
- **Layering against other programs.** The island and its panel sit in the
  top layer (`compositor.md` Shell surface hosting); the privacy form is
  drawn above any client's layer-shell surface, so no other program can
  cover it.
- **Keys.** Ctrl+F8 is the island's; Ctrl+F2 and Shift+Cmd+/ are the menu
  bar's.

### The queue

1. **Privacy indicators are outside the queue.** Always shown, on every
   screen, in every form, while a capture runs.
2. **Live activities rank:** a prompt waiting; recording and sharing; a
   call; a timer; media. The highest-ranked leads; between equals, the most
   recently started. The leader takes the compact face; the others show as
   marks in rank order.
3. **One per kind.** Several streams make one sharing activity listing each;
   media is the most recently playing player; timers are the soonest ending,
   with a count; prompts are the first in the prompt queue.
4. **A new activity that leads** is announced by its start event; one that
   does not lead adds its mark.
5. **Events** take the pill over the activity face, the clock and the
   glyphs for their duration, keeping the privacy marks, then return to the
   live state.
6. **Same kind replaces.** A second event of a kind already showing updates
   it in place and restarts its duration.
7. **An OSD pre-empts.** It replaces any other event at once and is never
   queued, since it answers a key being pressed now.
8. **Other events queue** in arrival order, one at a time. One that has
   waited longer than its own duration is dropped; an urgent attention
   request is not dropped while its app is unfocused. A non-urgent
   attention event is suppressed while a banner from the same app shows.
9. **Durations are by importance:** an OSD shortest, then a copy, then a
   mode change, attention, an activity start, an activity ending with a
   reason; a timer's end and an urgent attention request hold; activities
   hold until they end.
10. **Nothing takes over an open panel** (`shell.md` Island 7); events wait
    for the pointer to leave, OSDs show in the panel.
11. **Over a full-screen window and on the lock screen** only privacy and
    OSDs show; other events are dropped and activities wait.

## Answers

- **Island, what it shows at rest and while live** (proposal: unstated):
  **replaced** by revision 1: v1's pill with the clock and glyphs that show
  only when active; compact faces beside the clock and events over it while
  live; Control Centre as the expanded form.
- **Island, which activities and events it carries** (proposal: every live
  activity, transient event and OSD): **changed**. Every live activity and
  OSD, and only those transient events where the user would otherwise not
  see the change, plus attention (revision 1). File jobs and downloads go
  to the switcher's cards, capture results to the thumbnail, everything
  from the notification server to banners. Calm at rest (brief P1).
- **Island, repeats menu bar state** (proposal: never; clock, connection,
  sound and battery in the bar): **replaced**. There is no bar at rest; the
  clock and those states are the island's, and the on-demand bar carries
  none of them.
- **Island, order when several compete** (proposal: privacy always; then
  recording and sharing, authentication, call, media, timer): **changed**.
  A waiting prompt leads, because it blocks something until answered and
  the privacy marks already show a recording; a timer ranks above media,
  because it is bounded and set to be told. The full rule is under The
  queue.
- **Island, how an activity opens to its controls** (proposal: hover
  expands, click opens Control Centre's Now Playing or the prompt):
  **changed**. Hover opens Control Centre with every activity's row in Now
  at the top, media's in the Now Playing module just under it
  (`control-centre.md` For the user 2); a click pins it; each row has its
  controls and a way to where it belongs (`design-references.md` §2, "hover peeks, click
  commits").
- **Island, size-change motion** (proposal: a compositor morph):
  **adopted**, anchored at top centre, content fading with growth, radius
  capped (v1 lessons §5).
- **Island, desktops** (proposal: no): **adopted**; no desktop dots.
- **Menu bar, which status items show by default** (proposal: Isle's own
  only when they say something): **changed**. They are the island's glyphs,
  each with Always, When active or Never and When active by default (brief,
  From macOS); the configuration error and shortcuts inhibited untiered;
  other programs' items in Control Centre's tray, never at rest. Keyboard
  layout and updates waiting are not glyphs.
- **Menu bar, reorder and remove status items** (formerly `shell.md` Menu bar 7, removed with the permanent bar):
  **changed**: the glyphs' order is fixed and each can be set to Never; the
  tray reorders by Cmd+drag.
- **Menu bar, where the clock sits and what it opens** (proposal: right
  end, Notification Centre): **changed**: in the island; it opens
  Notification Centre. The month and agenda (`design-references.md` §1,
  Itsycal) go to `notification-centre.md` as a candidate.
- **Control Centre, where it opens from** (`shell.md` Control Centre 1: the
  menu bar or a chord): **changed**: hover or a click on the island, and
  Ctrl+F8, on the screen it was opened from.
- **OSDs, where they show** (proposal: the island): **adopted**. With no
  menu bar at rest, macOS's capsule has nothing to hang from.
- **System prompts, where a waiting prompt is announced** (proposal: the
  island): **adopted**, only while the dialog is not on screen.
- **Capture, where the recording timer shows** (proposal: the island):
  **adopted**, as the recording activity's compact face.
- **Shell runtime, which screens carry the island** (proposal: every
  screen; events on the main one): **adopted**: an island with the clock,
  glyphs and privacy on every screen; activities and events on the main
  screen; Control Centre on any.
- **Portal backend, where a running cast is shown** (proposal: the island,
  with Stop): **adopted**, with Stop per stream in Now.
- **Calendars, an event soon to start as live activity** (proposal: no):
  **adopted**.
- **Focus, how the current Focus is shown**: the Focus glyph, When active
  by default as on macOS (`design-macos.md` §3.9), and an event when it
  changes automatically or by chord. The summary's copy is the banners
  sheet's.
- **Game Mode, where its state is shown** (proposal: a status item):
  **changed**: Game Mode is the Gaming Focus (B30), so the Focus glyph
  shows it; no second glyph. On and off are events, as Focus.
- **Updates, restart owed** (`platform.md`: a status item and a system
  menu line): **adopted** as the Updates glyph's restart-owed form, listing
  the reasons (kernel, NVIDIA driver, Isle) in its detail, and the system
  menu's line (`menu-bar.md`).
- **Settings, a rejected hand edit** (`platform.md`: by the shell, with
  file, line, column and message, until fixed): **adopted** as the
  configuration error glyph and notice row, also covering `tokens.kdl`
  (B10). A banner on first rejection is `notifications.md`'s.
- **Keyboard-shortcuts-inhibit indicator** (`compositor.md`: the menu
  bar): **changed**: an untiered glyph and a notice row naming the app,
  the escape chord (open in `compositor.md`) and Take keys back, which ends
  the inhibitor for that window; with `window-chrome.md`'s brief notice
  inside the window's top edge, since the island is hidden over a
  full-screen VM.
- **Shell surface hosting, the island's layer** (proposal: top):
  **adopted** for the island and its panel, **changed** for its privacy
  form, which is drawn above client layer-shell surfaces.

## Copy

| Where | String |
|---|---|
| Recording row | "Recording", "Recording with audio"; Stop (accessible name "Stop recording") |
| Sharing row | "Sharing your screen with {app}", "Sharing a window with {app}"; Stop (accessible name "Stop sharing with {app}") |
| Privacy rows | "{app} is using the microphone", "{app} is using the camera" |
| Prompt waiting | "{app} needs your password", "“{network}” needs a password", "{device} wants to pair"; Show, Cancel |
| Activity ended | "Recording stopped: the window closed", "Sharing stopped: the screen was disconnected" |
| Copied | "Copied", "Colour copied", "Password copied" |
| Mode events | "{Focus} on", "{Focus} off", "Do Not Disturb on", "Game Mode on", "Game Mode off" |
| Attention | "{app} needs your attention" |
| OSDs | the device's name (output, monitor or keyboard); "Muted"; the layout's name |
| Timer end | "{label} done", or "Timer done" |
| Glyphs (accessible names) | "Wi-Fi: {network}", "Not connected", "Bluetooth: {device}", "Sound muted", "VPN: {name}", "{Focus}", "Updating", "Restart to finish updating", "Battery {n}%", "{app} is receiving all keyboard shortcuts", "Configuration error" |
| Shortcuts inhibited row | "{app} is receiving all keyboard shortcuts", the escape chord, "Take keys back" |
| Configuration error row | `copy.md`'s rejected-edit string, then "Open in editor", "Show in Files"; after a safe start, `copy.md`'s safe-start string, then "Use my settings", "Report…" |
| Restart reasons | "The kernel was updated", "The graphics driver was updated", "Isle was updated" |
| Footer | "Edit controls", "Control Centre settings…" |
| Tiers | "Always show", "Show when active", "Never show" |
| Secondary click on an activity | Hide |

Times read as "0:42" and "1:02:15", without units.

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **The clock's default.** Time only, as v1; or weekday, date and time,
   as macOS's menu bar. *Recommended: time only*, so the pill stays small
   at rest; the date is a click away in Notification Centre and the
   weekday and date are settings.
2. **Pinning Control Centre controls.** On macOS any control can be pinned
   to the menu bar. Here the glyphs above are the only things the pill
   carries, and setting one to *Always show* is pinning it; or any module
   can be pinned as a glyph. *Recommended: only these glyphs*, so the pill
   cannot grow into a menu bar's status area; `shell.md` Control Centre 4
   changes to match.
3. **Activities from any program.** Let any process push a live activity
   with progress (Otto's `org.otto.Island1`), for builds and long jobs; or
   only the sources listed here. *Recommended: only these for now*: long
   work shows on the switcher's cards and ends in a notification. With the
   Dock gone that progress is out of sight until Cmd+Tab, so revisit early
   if it is missed.
4. **The order.** This sheet's (a waiting prompt, recording and sharing,
   call, timer, media) or B24's proposal (recording and sharing, prompt,
   call, media, timer). *Recommended: this sheet's*, for the reasons under
   Answers.
