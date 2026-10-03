# Notification Centre

Batch 3. Answers to: `shell.md` § For the design process (Notification
Centre: its layout, grouping and order; Widgets: whether widgets also sit
in Notification Centre's column) and Notification Centre 1, 2;
`competition.md`'s open row (where Notification Centre opens, left to
`island.md`, which left it here); the month and agenda that `island.md` and
`menu-bar.md` passed here (`design-references.md` §1, Itsycal); the parts
of the centre `notifications.md` deferred (its times, the stack, where
"Earlier" sits, what the summary's Show opens). B24, B27, D26.

## Purpose

Where notifications wait to be read, acted on or cleared, and where the
date is, both one click on the island's clock (brief, From macOS: "Notification
Centre from the clock"). Everything that reaches the notification server,
held or shown as a banner, is here until the user deals with it or its
sender goes (`notifications.md`, Records). The clock's other half is a
month and an agenda, so a click on the time answers "what day is it, and
what is next". It is not a widgets surface (the dashboard is the one,
`wallpaper.md`, `widgets.md`), not a history of everything that ever
arrived (dealt with is gone, `notifications.md`), not where Focus is
switched (Control Centre's Focus module, `focus.md`), and not a place for
progress (the switcher's cards, `switcher.md`).

## Content

### Where it opens

From the island, as the island's second panel (For the user 1). A click on
the clock unfolds the pill downward into Notification Centre exactly as a
click elsewhere on it unfolds Control Centre (`island.md`): the panel's top
row is the pill's own row, so the clock stays where it was and a second
click on it closes the centre; the panel grows down from the strip,
centred on the screen, over window content. It opens on the screen whose
clock was clicked (`shell.md` Notification Centre 1); one centre is open
across all screens, and it and Control Centre are never open together on
one screen (`island.md`).

It takes Control Centre's width unless stage 4 finds that the notification
card needs more; the card is the banner's card (`notifications.md`, How a
record shows), so a banner and its entry here read as one thing. With one
width, switching between the two panels changes only the shape's height.

### Layout

Leading edge to trailing edge is one column; top to bottom:

| Part | What | From |
|---|---|---|
| Pill row | The island's own row: activity face, clock, glyphs, privacy marks. The Focus glyph here is the centre's only sign of a Focus; nothing below repeats it | `island.md` |
| Date | Weekday and date, "Saturday 3 October", since the clock shows the time only (`island.md` For the user 1) | timedate1, the region settings |
| Month | A month grid: the locale's first weekday, today marked, a small mark on days with events, the previous and next month controls; week numbers by setting, off by default | `platform.md` Calendars |
| Agenda | Today's remaining events, then tomorrow's, or the selected day's events once a day is chosen. Absent when no calendar is subscribed | `platform.md` Calendars (expanded from 60 days back to 400 ahead) |
| Notifications | A header row ("Notifications" and the Clear all control), then the live notifications, stacked by app | the notification server (`notifications.md`) |
| Earlier | A heading, then the records, stacked by app the same way | the records (`notifications.md`, Records) |

No widgets sit here (For the user 2). The month and agenda come first
because they are what a click on the clock is about and their height is
fixed, so arrivals below never move them and they never move under the
pointer. The whole panel below the pill row scrolls as one, as Control
Centre does, up to the work area's height less a margin; on the ultrawide a
month, a short agenda and a screenful of notifications fit without
scrolling.

### The month and the agenda

The Itsycal idea (`design-references.md` §1), placed under the clock that
opens it. It replaces nothing and adds no width at rest.

- **The month.** A click on a day selects it and shows its events in the
  agenda; the selection returns to today when the centre closes. The
  previous and next controls page by month; a click on the month's name
  returns to today. Today is the one marked day (brief P6); event marks
  are neutral.
- **The agenda.** One row per event: its time ("09:30–10:00", or "All day"
  first), title, the calendar's own colour as a mark (calendars are each
  named and coloured, `platform.md` Calendars 1: data, not a signal), and
  its location. An event under way reads "Now". Today shows events not yet
  ended; past ones fold under "Earlier today", collapsed. Tomorrow follows
  as its own group. A day with nothing shows one line, "No events" ("No
  more events today" for today).
- **An event row** expands in place to its details: the time range, the
  calendar's name, location and notes. Calendars are read-only
  subscriptions (`platform.md` Calendars 1), so nothing is edited here. An
  event whose location or link is a meeting address carries "Join", which
  opens it in the default browser (Itsycal, macOS Calendar).
- **No calendars.** The month alone; the agenda is absent. The month's
  secondary-click menu holds "Calendar Settings…" (Settings › Calendars,
  `settings.md`), which is also the way to subscribe. Nothing invites setup
  at rest (brief P1).
- **A calendar that cannot be fetched** shows its last good copy
  (`platform.md` Calendars 2); the centre says nothing about it, and
  Settings › Calendars carries the error.

World clocks and weather stay on the dashboard (`widgets.md`), where the
Clock widget takes several zones.

### Notifications

**Order.** Live notifications above records. Within each, one stack per
app, the stacks ordered by their newest arrival, newest first
(`shell.md` proposal), and each stack newest first. A notification that
replaces an earlier one (`replaces_id`) updates its card in place and
moves its stack to the top.

**The card.** `notifications.md`'s banner card, with its icon, app name
and time, summary, body cut at two lines, image, progress, actions and
Reply. Here the actions and the close control show while hovered or
focused, the body opens in full on hover, and a progress bar updates live.
A record is the same card without actions or Reply; its click opens the
app (`notifications.md`).

**Stacks** (macOS, `design-macos.md` §3.5):

- An app with one notification shows its card alone.
- An app with several shows the newest card with the edges of the others
  behind it and their count ("2 more"). A click on a collapsed stack
  expands it; it does not run the newest's default action.
- An expanded stack has a header row: the app's name, "Show less" and the
  stack's clear control. Its cards follow, each acting on its own. One
  stack is expanded at a time; expanding another collapses the first.
- A stack holds every notification from the app, live or record, in its
  own section: an app can have a stack above and another under Earlier.

**Times.** Under a minute "now"; under an hour "{n} min ago"; earlier today
the time ("14:05"); yesterday "Yesterday 18:04"; within the week the
weekday and time ("Tuesday 09:12"); older the date ("3 Oct"). Times are
refreshed only while the centre is open (`shell.md` Shell runtime 7).

**Clearing** (`shell.md` Notification Centre 2). Clearing a notification
removes it and its record and sends `NotificationClosed` as dismissed
(`notifications.md`: dealt with is gone).

| What | How |
|---|---|
| One | Its close control (hover or focus); dragged toward the trailing edge past a threshold (released short, it returns); middle click; Delete |
| A stack | The close control on its header, or Delete on the header |
| Everything | The Clear all control in the Notifications header: a close mark that widens to "Clear all" on hover or focus and acts on a second click or Return, macOS's two-step for a stack's clear, so one stray click never empties the centre. It clears live notifications and records alike (`notifications.md`). No confirmation dialog |

**Acting.** A click on a card runs the default action, or with none raises
the app; the centre closes and the app comes forward with the activation
token (`shell.md` Notification Centre 7). A button runs its action and,
unless the notification is `resident`, removes it. Reply opens the field in
the card with keyboard focus (`notifications.md`). Acting on a record
raises or launches its app.

**Arrivals while open.** Banners do not show while the centre is open; the
arrival goes to the top of its app's stack here (`notifications.md`,
States). While the pointer is over the list, an arrival waits until the
pointer leaves or rests outside the list (stage 4), as on the banner stack,
so nothing slides under a click (brief P5; `design-v1-lessons.md` §3.5).

**Sounds.** The centre plays none of its own; an arrival's sound is the
banner rule's, which applies as if the banner had shown.

### After silence

The summary banner (`notifications.md`) is the only summary. Its Show, or a
click on it, opens the centre on the main screen with keyboard focus on
the newest held notification and its stack expanded; the held
notifications are not otherwise set apart, since they are already the
newest. Machine-written summaries of a stack (macOS's Apple Intelligence
summaries, `design-macos.md` §3.5) are not taken: there is no model to
write them and the intent asks for none.

### Not here

| What | Where |
|---|---|
| Widgets, world clocks, weather | The dashboard (`widgets.md`) |
| Do Not Disturb and the other Focus modes | Control Centre's Focus module; their state is the pill row's Focus glyph |
| Per-app rules, lock-screen showing, holding while full screen or sharing | Settings › Notifications (`notifications.md`), reached from any card's or header's secondary click |
| A count at rest | Nowhere: the clock carries no badge (brief P1; `dock.md` For the user 2) |
| Calendar subscriptions | Settings › Calendars |

## States

| State | What shows |
|---|---|
| Closed | Nothing. The clock carries no count or mark for waiting notifications. |
| Open | The pill row, date, month, agenda, notifications and Earlier, as above, with keyboard focus (opened by click, the ring shows only after a key). |
| Empty | No notifications and no records: the Notifications header without Clear all, then one line, "No notifications" (`copy.md`). No Earlier heading. With no calendars either, the panel is the pill row, date, month and that line. |
| Only records | No live section; Earlier directly under the header. |
| Loading | The records are read from disk off the main loop at session start; Earlier is absent until they arrive and then fills without moving what is above it. Calendars not yet fetched show the cached copy, or no agenda. Nothing waits (`shell.md` Shell runtime 8). |
| Error | A records file that will not read: no Earlier, and live notifications as normal; the file is kept aside for the packet. An image that will not decode: no thumbnail. An action on a sender that has gone: the card becomes a record. A panic in the centre: rebuilt from the server's state, open, with the same stack expanded (`shell.md` Shell runtime 2, 3). |
| Many | Stacks keep the list short; the panel scrolls past its maximum height with the pill row fixed. An expanded stack of more than a cap (stage 4) shows the newest and "Show {n} more". The 300-record cap is `platform.md`'s. |
| A reply open | The card holds keyboard focus with its field; other cards stay; Escape cancels the reply before it closes the centre. |
| Second screen | Opens on the screen whose clock was clicked; from the chord, on the screen with the focused window. One centre across screens: opening it on one closes it on the other. While it is open on any screen, no banner shows on any screen and arrivals come here. |
| Game in front, windowed | As normal, solid and without transitions (`competition.md` For the user 1). |
| Full screen, game or not | Reached through the island's return at the top edge, then the clock (`island.md`); over a game only after the dwell, never while the game holds the pointer. The chord is not in B30's escape set, so over a game it goes to the game. |
| Locked | Nothing. The lock has its own list (`lock.md`); after unlock everything is here. |
| Focus on | Unchanged: everything arrives here (`focus.md`). The pill row shows the Focus glyph. |
| Reduce Motion | No morph: the panel takes its size at once and content fades. Stacks expand and collapse at once; cards leave without moving, and the list closes up at once. |
| Reduce Transparency | Solid. |
| Increase Contrast | Edges around the panel, cards and stacks; today's mark and the selected day differ by more than colour (a filled mark against an outline). |
| Largest text size | The panel grows wider within a maximum and taller; the month's cells grow with the text; cards keep their line limits and grow taller; the panel scrolls sooner. |

## Opening and closing

- **Opens** on a click on the clock, Return or Space on the clock
  (`island.md`), the Notification Centre chord (Keyboard), a hot corner set
  to Notification Centre (`dock.md` For the user 3: none by default), the
  summary banner's Show, the overflow row "{n} more in Notification
  Centre", Spotlight's "Notification Centre" action, the CLI and D-Bus
  (`shell.md` Shell runtime 16), and macOS's two-finger swipe in from a
  trackpad's right edge if `compositor.md`'s packet can recognise an edge
  start (its Open item).
- **From the island.** A compositor morph of the island's one shape, as
  Control Centre's: anchored at the top and the horizontal centre, growing
  down, content fading with the growth (`island.md`). From Control Centre
  to Notification Centre and back is one morph between the two shapes with
  the content crossfading, never a close and an open.
- **Closes** on a click on the clock, Escape (after a reply field or menu
  inside it), the chord again, a click outside the panel and the pill,
  acting on a notification or event that goes elsewhere, a click elsewhere
  on the pill (which turns it into Control Centre, pinned), any other
  top-layer surface opening (a menu of the revealed bar), any overlay
  (Spotlight, Mission Control, the capture tool, the session dialog, Force
  Quit, the widgets dashboard), Cmd+Tab, and locking. Pointer leaving does
  not close it: it opens on a click, as a pinned panel.
- **Character.** Critically damped, no bounce; closing faster than opening
  (brief P5). Over a game and under Reduce Motion, none.
- **Focus.** Taken on opening, given back on closing to the window that
  had it, before any action the centre started reaches that window
  (`shell.md` Shell runtime 11).

## Keyboard

| Action | Key |
|---|---|
| Open or close | The Notification Centre chord (*proposed*: Ctrl+Option+Cmd+N, beside Do Not Disturb's Ctrl+Option+Cmd+D; macOS's Fn+N needs a Globe key most keyboards cannot send, `macos-model.md` §3). Return or Space on the clock (`island.md`) |
| Where focus lands | From a banner showing: that notification (`notifications.md` Keyboard). Otherwise the newest notification, or today in the month when there are none |
| Move between parts: month, agenda, Notifications header, notifications, Earlier | Tab, Shift+Tab |
| Month: move by day; by week | Left, Right; Up, Down |
| Month: previous or next month | Page Up, Page Down |
| Month: today | Cmd+T (as Calendar) |
| Agenda: move between events | Up, Down |
| Agenda: expand or collapse an event | Return or Space |
| Agenda: Join | Tab to it, Return |
| Notifications: move between cards and stack headers | Up, Down; Home, End |
| Expand or collapse a stack | Right, Left; Return or Space on a collapsed stack |
| Default action, or open a record's app | Return on a card |
| A card's buttons, Reply | Tab, Shift+Tab within the card; Return or Space |
| Reply field | Return sends, Shift+Return adds a line, Escape cancels |
| Clear the card, or the stack on its header | Delete (⌫) |
| Clear all | Tab to the control, Return (it shows "Clear all" once focused, so one Return after focusing it acts) |
| A card's or header's secondary-click menu | Shift+F10, or the menu key |
| Close | Escape; focus returns to the window that had it |

The chord is a desktop chord, taken before Cmd translation (B21) and given
up under keyboard-shortcuts-inhibit. The centre is in the accessibility
tree as a list of groups: the month as a grid, the agenda as a list, each
stack as a group of notifications whose parts are readable.

## Pointer

| Input | On | Does |
|---|---|---|
| Click | The clock | Opens or closes the centre on this screen |
| Hover | The island, while the centre is open on that screen | Nothing: the island takes no hover while the centre is open, so a peek never replaces it (added to `island.md`'s hover guards) |
| Click | The pill outside the clock, while open | Switches to Control Centre, pinned (`island.md`) |
| Hover | A card | Shows its close control and actions, the body in full; highlights only when the pointer moves (`island.md` guards) |
| Click | A card; a collapsed stack | The default action; expands the stack |
| Click | A button, Reply, Join | That action |
| Click | A day; the month's name; previous or next | Shows that day's events; back to today; pages |
| Click | An event row | Expands it in place |
| Middle click | A card | Clears it |
| Drag or horizontal scroll | A card toward the trailing edge | Clears it past a threshold, returns short of it |
| Secondary click | A card or stack header | "Mute {app} for 1 hour", "Mute {app}", "Clear", "Notification Settings…" (`notifications.md`) |
| Secondary click | The month | "Show week numbers", "Calendar Settings…" |
| Scroll | The panel | Scrolls it; never pages the month (Page Up and the controls do) |
| Click | Outside the panel and the pill | Closes the centre; the click does not also reach the window beneath |

## Shared places

What this sheet adds to `competition.md`:

- **Below the island, centre: where Notification Centre opens.** Settles
  the row `competition.md` left open: it unfolds from the pill, the
  island's second panel beside Control Centre, a menu-bar surface in the
  top layer; one open per screen, one centre across screens. Replaces
  B24's proposal of a column at the right edge (For the user 1).
- **The island while the centre is open.** The pill takes no hover on that
  screen; a click on the clock closes the centre, a click elsewhere on the
  pill turns it into Control Centre. An addition to `island.md`'s hover
  guards.
- **Banners against the centre.** The top right (banners) and top centre
  (the centre) do not overlap. While the centre is open on any screen, no
  banner shows anywhere and arrivals go into it: `competition.md`'s
  "While Notification Centre is open, arrivals go into it" read as any
  screen, which `notifications.md`'s States row ("that screen") follows at
  the gate. A banner swiped away is here, live.
- **The calendar notification's click.** `notifications.md` sends the click
  on "{event} starts now" to the dashboard's Calendar widget. Changed: it
  opens the centre with that event expanded in the agenda, one click from
  any window, instead of covering the screen. The dashboard's Calendar
  widget stays (`widgets.md`).
- **Widgets.** None here; the dashboard is the one widgets surface
  (`wallpaper.md`, `widgets.md`). The dashboard's Notifications widget
  opens the centre at a stack and never holds a second list (`widgets.md`
  For the user 2).
- **The revealed menu bar and its menus.** As `menu-bar.md`: opening a menu
  closes the centre.
- **Overlays and prompts.** Every overlay closes the centre; a system
  prompt takes focus over it and gives it back, the centre staying open
  (`competition.md` Keyboard focus 2).
- **Keys.** Ctrl+Option+Cmd+N (*proposed*) for P5's table. Inside the
  centre, the arrows, Page Up and Down, Cmd+T and Delete are the centre's
  only while it has focus.
- **Hot corners.** "Notification Centre" is one of the corner actions
  (`shell.md` Notification Centre 1); no corner carries it by default.

## Answers

1. **Notification Centre, its layout** (`shell.md` proposal: notifications
   above, widgets below). **Replaced**: the date, month and agenda above,
   notifications and Earlier below, no widgets. The dashboard is the one
   widgets surface (`wallpaper.md`); the clock's own content is the date.
   For the user 2.
2. **Notification Centre, grouping and order** (proposal: by app, newest
   first). **Adopted**, as macOS's stacks: one stack per app, stacks by
   their newest arrival, live above Earlier, records stacked the same way.
3. **Where Notification Centre opens** (`competition.md`, `island.md`;
   B24: a column at the right edge, as macOS). **Changed**: it unfolds from
   the pill under the clock that opens it. For the user 1.
4. **Notification Centre 1, how it opens** (the clock, a chord or a hot
   corner, on the screen it was opened from). **Adopted**: the clock, a
   proposed Ctrl+Option+Cmd+N, a corner action, the summary's Show, the
   overflow row, Spotlight, and the trackpad's edge swipe if the packet can
   recognise it.
5. **Notification Centre 2, clearing singly, by app or all.** **Adopted**:
   a card's close, a swipe, middle click and Delete; a stack's close; Clear
   all behind macOS's two-step; no dialog.
6. **Whether widgets also sit in Notification Centre's column** (`shell.md`
   Widgets; `wallpaper.md` For the user 1). **No**, for both sheets; see
   `widgets.md`.
7. **The clock's month and agenda** (`island.md`, `menu-bar.md`: a
   candidate from Itsycal, `design-references.md` §1). **Adopted** at the
   top of the centre: date, month, agenda with Join.
8. **The centre's times, the stack and Earlier's place**
   (`notifications.md`, deferred). **Answered** above.
9. **The summary's pointer route** (`notifications.md`; `gaming.md` Game
   Mode 6). **Adopted**: Show opens the centre at the newest held
   notification. Machine-written stack summaries are not taken.
10. **v1's notifications page in the hover panel** (`shell.md` parity map).
    Lands here, one click on the clock instead of a hover and a tile's
    arrow, so where notifications live is settled once
    (`design-v1-lessons.md` §3.4, §8).

## Copy

| Where | String |
|---|---|
| Date | "{weekday} {day} {month}", e.g. "Saturday 3 October" |
| Agenda | "Today", "Tomorrow", "Earlier today", "All day", "Now", "No events", "No more events today", "Join" |
| Month | Accessible names "Previous month", "Next month", "{date}, {n} events"; menu "Show week numbers", "Calendar Settings…" |
| Notifications header | "Notifications"; "Clear all" (accessible name of the close mark before it widens, too) |
| Stacks | "{n} more", "Show less", "Show {n} more"; accessible name of a stack's close "Clear {app} notifications" |
| Earlier | "Earlier" |
| Times | "now", "{n} min ago", "14:05", "Yesterday 18:04", "Tuesday 09:12", "3 Oct" |
| Empty | "No notifications" |
| Secondary click | "Mute {app} for 1 hour", "Mute {app}", "Clear", "Notification Settings…" |
| Accessible name of the panel | "Notification Centre" |
| Spotlight action | "Notification Centre" |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Where Notification Centre opens.** (a) It unfolds from the pill under
   the clock, as Control Centre does from the rest of the pill; (b) a
   column down the right edge of the screen, as macOS, where banners
   arrive. *Recommended: (a).* On macOS the centre opens under the clock
   that was clicked; Isle's clock is at top centre, and a column nearly half
   the ultrawide's width away would open far from the click and from where the
   eye rests (`design-references.md` §19, items 4 and 5: tray, clock and
   toasts stranded in a far corner). From the pill it is one shape with
   Control Centre, so switching between them is a morph rather than one
   panel closing and another opening across the screen. The cost is the
   distance from banners at the top right: a banner swiped away lands in a
   centre that opens elsewhere, which the summary and the chord make up
   for.
2. **What sits beside the notifications.** (a) The date, a month and an
   agenda from the subscribed calendars, and no widgets; (b) macOS's
   widgets under the notifications, edited from the same gallery as the
   dashboard; (c) notifications alone, as v1's page. *Recommended: (a).*
   The dashboard is already the one widgets surface (`wallpaper.md`), and
   two homes mean two layouts to edit; the date is what a click on a
   time-only clock is most often for (`island.md` For the user 1); and v1
   moved its notifications three times, so this keeps the centre to
   notifications and the clock's own content.
