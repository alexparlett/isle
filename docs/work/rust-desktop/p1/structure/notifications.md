# Notification banners

Batch 2. Answers to: `shell.md` § For the design process (Notification
Centre: where banners appear, summary copy, the boundary of past records;
Shell runtime: banners on the main screen only); `platform.md` § For the
design process (Notifications and badges: how records without actions are
shown; Focus: the summary's copy); `gaming.md` § For the design process
(Game Mode: the held-notification summary's form and copy, where held
banners would have shown); `island.md`'s deferral (a banner on a rejected
hand edit). B12, B24, B27, B30.

## Purpose

The brief, interruptible sign that something arrived through the
notification server (`org.freedesktop.Notifications`, B12): a card near a
corner of the main screen that says who sent it and what it says, lets
the user act or reply in place, and goes by itself. Everything a banner
shows also reaches Notification Centre (`notification-centre.md`, batch
3), so a banner can be missed, swiped away or held without losing
anything. It is not the island (which carries privacy, live activities and
the shell's own acknowledgements, never a notification: `island.md`), not
a progress surface (long work is on the switcher's cards, `switcher.md`),
and not a place that holds state: a banner is the notification's first
showing and nothing more.

## Content

**Where.** The top right of the main screen (`shell.md` Shell runtime 14;
D49: one screen, never all), its top edge on the lower edge of the
island's reserved strip, inset from the trailing edge by the gap windows
keep (stage 4). It sits in the top layer above windows (`competition.md`
Layers). Because its top is at the strip's lower edge, the revealed menu
bar, which is no taller than the strip, never touches it (`menu-bar.md`).
The place is the same over a full-screen window, where the strip is
hidden. Why the top right and not under the island is For the user 1.

**A banner, by part.**

| Part | What | From |
|---|---|---|
| App icon | Leading; the app's icon, never the notification's picture | `desktop-entry` hint → the entry's icon; else `app_icon` when it names an icon; else a generic icon. A file given as `app_icon` is a picture, not an icon (v1 `Notifications.qml`): it goes to Image |
| App name and time | One line: the app's name, then "now" while on screen | the desktop entry's name; else `app_name` |
| Summary | One line, cut at the end | `summary` |
| Body | Two lines at rest, cut at the end; the whole of it, to a maximum, while hovered | `body`, with the sanitised markup the server advertises (bold, italic, underline; links shown as text, not followed from a banner) |
| Image | A thumbnail at the trailing edge; larger while hovered | `image-data` or `image-path` hint, or a file given as `app_icon`; copied at arrival (`platform.md` Notifications 3) |
| Progress | A thin bar under the body, only when given | the `value` hint (0–100), which scripts and some apps send |
| Count | "3 new" beside the time, when several from one app share the banner (Grouping, below) | the server |
| Actions | Up to three buttons in a row under the body, shown while hovered; more than three put the rest under an Options pop-up as the third button. The `default` action is not a button: it is the click on the banner | `actions`; action icons are not advertised (brief, Left out: no icons on menu-like items) |
| Reply | A Reply button among the actions, opening a field in the banner | the `inline-reply` action (`platform.md` Notifications 2) |
| Close | A close control at the leading top corner, shown while hovered and always on a persistent banner (macOS) | |

No banner shows a badge, an accent or a coloured edge for urgency: a
critical banner differs by staying, not by colour (brief P6).

**What arrives, and what becomes of it.** Every `Notify` call is decided
once, in this order:

1. **Transient** (`transient` hint): a banner if allowed, never kept in
   Notification Centre or the records.
2. **Muted app** (per-app "Allow notifications" off): no banner and no
   sound; kept in Notification Centre; not counted as held.
3. **Low urgency**: no banner and no sound; kept in Notification Centre
   (`platform.md` Notifications 4).
4. **Held**: a Focus, a full-screen window, a screen share, the user away
   (States, below). Kept in Notification Centre and counted for the
   summary. Critical and allowed apps break through as each silencer
   allows.
5. **Otherwise a banner**, with a sound if sounds are on.

A notification that replaces an earlier one (`replaces_id`) updates the
banner in place, without a new entrance and without a second sound, and
restarts its time.

**Lifetime.** A banner lasts the app's `expire_timeout` held between 3 and
15 seconds; an app that leaves it to the server gets the default (v1's 5
seconds as the start); an app that asks for "never" (0) gets 15 seconds as
a banner and stays in Notification Centre (`platform.md` Notifications
4). It is **persistent**, staying until acted on or closed, when it is
critical, or when the app's banner style is Persistent (per-app rules,
below). Leaving the screen is not closing: a banner that times out or is
swiped away stays live in Notification Centre with its actions while the
app's server object lives. `NotificationClosed` is sent only when the
user closes it, acts on it (unless `resident`), or clears it in the
centre.

**Grouping.** One banner per app. A second notification from an app whose
banner is showing takes that banner's place: its content becomes the
newest, the count shows how many arrived, its time restarts, and it does
not move. The earlier ones are in Notification Centre, stacked by app
there (`notification-centre.md`). A busy chat therefore never fills the
corner (brief P1).

**Stacking.** Up to three banners, newest at the anchor, older ones one
place down. A banner leaving lets those below it move up. A fourth app's
arrival makes the oldest temporary banner leave early to Notification
Centre; if all three are persistent, the newcomer is a single row under
the stack, "{n} more in Notification Centre", which opens it.

**Records and the "Earlier" boundary.** Notification Centre's layout is
`notification-centre.md`'s; what is live and what is a record is decided
here, since banners and the summary depend on it.

- **Live**: the server still holds the notification: this session, not
  closed by the app, not cleared or acted on by the user, and its sender
  still on the bus. Live notifications keep their actions, reply and
  image.
- **A record** (`platform.md` Notifications 3; D26): a notification that is
  no longer live without the user having dealt with it: its sender quit or
  dropped off the bus, the app closed it, or the session restarted. Records
  sit under "Earlier", which is defined by this and not by age.
- **Dealt with is gone.** Closing a notification or acting on it removes
  it and its record, so "Earlier" is what was missed, not a log of
  everything (v1 counted cleared items back into its history, §3.4).
  Clear All clears both. The 300-record cap and restart survival are
  `platform.md`'s.
- **How a record shows** (`platform.md` item): the same card as a live
  notification, so the two read as one list, with no action row and no
  Reply. Its time is absolute past a day ("Yesterday 18:04", "3 Oct").
  A click opens the app: it raises it if running and launches it from its
  desktop entry if not, without the default action, which no longer
  exists; a record with no desktop entry has no click. Its secondary
  click offers Clear and the app's mute.

**The summary after silence** (`gaming.md`, `platform.md` items;
`copy.md` recurring string). When everything that held banners has ended,
what was held is told once:

- none held: nothing;
- one held and still live: its own banner, as if it had just arrived,
  with its real time ("12 min ago");
- two or more: one summary banner, from Isle, titled "While you played"
  if Game Mode held any of them and "While you were away" otherwise, with
  the count and up to two app names ("4 notifications from Discord and
  Steam", "9 notifications from Mail, Discord and 3 more"). A click, or
  its one action Show, opens Notification Centre. It is temporary, makes
  no sound, and is not itself kept in the centre.

It shows when the last silencer ends, so leaving a game straight into a
full-screen video gives one summary afterwards, not two. Held critical
notifications are not in the count, since they were shown.

**Sounds** (brief: a few Mac-like sounds, each switchable). A banner
plays the alert sound chosen in Settings › Sound, or the app's own
`sound-name` where the sound theme has it or `sound-file` where readable,
unless the app sends `suppress-sound`, the app's "Play sound" is off, or
"Play sound for notifications" is off. Nothing held, muted or low urgency
makes a sound. A burst plays one sound: an arrival within a short window
(stage 4) of the last sound is silent. Sounds play at the alert volume,
under the output's mute.

**Per-app rules** (`shell.md` Notification Centre 4; inventory §4
Notifications). Settings › Notifications lists every app that has sent a
notification, with:

| Setting | What it does | Default |
|---|---|---|
| Allow notifications | Off mutes the app: no banner, no sound, not counted; still kept in the centre | On |
| Banner style | Temporary or Persistent (macOS's Banners and Alerts) | Temporary |
| Play sound | The app's sound | On |
| Show badges | The switcher's and the Apps grid's badge for the app (`switcher.md`) | On |
| Show over full-screen windows | The app breaks through full-screen holding (the allow list outside any Focus, `platform.md` Focus 5) | Off |

Allowing an app through a Focus is that Focus's setting (`focus.md`).
Global settings on the same page: "Hold banners while a window is full
screen" (on), "Hold banners while sharing your screen" (on; `platform.md`
Focus 3), and the lock screen's showing (below). Muting is also one step
from the banner itself (Pointer, below; `design-references.md` §7, DMS).

**The desktop's own notifications** (`shell.md` Notification Centre 8).
Isle's parts post through the same server, under the name of the part that
follows up:

| Event | App shown | Actions |
|---|---|---|
| A drive connected | Files | Open (Mount when not mounted), Eject |
| An AppImage installed | Files | Open, Show in Files |
| An update finished | Software Update | Restart… when a restart is owed |
| A calendar event starting | Calendar | none; the click opens the dashboard's Calendar widget |
| A provider needs signing in | the provider's name | Sign in… |
| A hand edit rejected (`island.md` deferral) | Settings | Open in editor. Once per broken state of a file, not per save; the island's glyph holds while it lasts |
| An app crashed | the app | Reopen, Report… (the notice's form is the crash sheet's, batch 3) |

## States

| State | What shows |
|---|---|
| Rest | Nothing. No banner area, no empty stack, no count anywhere at rest (brief P1). |
| One banner | The card at the anchor, timing out. |
| Hovered | Time paused for the whole stack; close control, actions and the full body shown; larger image. Arrivals wait until the pointer leaves (Pointer, below). |
| Replying | The banner holds keyboard focus with its field; no time limit; Send and Cancel. Other banners keep timing. |
| Persistent | No time limit; close control always shown. |
| Grouped | One banner for the app with "{n} new" and the newest content. |
| Several | Up to three, newest at the anchor. |
| Overflow | The oldest temporary banner leaves early; with three persistent, the "{n} more in Notification Centre" row. |
| Waiting | A menu or Control Centre is open on the main screen, or an overlay that covers it (Shared places, below): the banner waits. One that waits longer than its own lifetime is not shown, and stays in the centre. Persistent ones wait without limit. |
| Notification Centre open | No banners on that screen: arrivals appear in the open centre, at the top of their app's stack. |
| Loading | A banner is on screen within 100 ms (`shell.md` acceptance bar 9) and never waits on an icon or image lookup; one that arrives late fills in without resizing the banner past its first height. |
| Error | Missing icon: the generic one. An image that will not decode: no thumbnail. Markup that will not parse: plain text. An action invoked on a sender that has gone: the banner leaves and the notification becomes a record. A panicking banner surface is rebuilt from the server's state (shell runtime 2, 3). |
| Held by a Focus | No banner, no sound; counted for the summary. Critical notifications pass if the Focus allows them, and the Focus's allowed apps pass (`focus.md`). |
| Full-screen window, not a game | Held, if the setting is on, except critical notifications and apps set to show over full screen. With a second screen that has no full-screen window, banners show there instead of being held: a full-screen window on one screen does not silence the other (`design-references.md` §7, §19). |
| Screen share or recording of a screen | Held on every screen while any output is being captured, critical included, since a banner would be in the capture; a share of a single window holds nothing. Summarised when it ends. |
| Windowed game | Held by Game Mode (`gaming.md` Game Mode 6). What breaks through shows as normal, drawn solid and without transitions (Game Mode 9). |
| Full-screen game | Held by Game Mode. What breaks through shows at the anchor over the game, solid, without transition, and is never persistent: a critical banner lasts 15 seconds, since a game holding the pointer cannot reach a close control. It stays in the centre. |
| Away | While the screen is dimmed for idle, the displays sleep or the session is locked, nothing is a banner: arrivals are held and summarised as "While you were away" on return. |
| Locked | The lock shows, by setting, nothing, each app with its count (default), or previews (app and summary, no body or actions) (`shell.md` Notification Centre 9; For the user 2). Its place on the lock is `lock.md`'s. No sound, except for a critical notification. |
| Second screen | Banners on the main screen only (D49); the full-screen case above is the one exception. |
| Reduce Motion | Banners appear and leave by fading, and the stack's moves become cuts. |
| Reduce Transparency | Solid. |
| Increase Contrast | An edge around each banner and its buttons. |
| Largest text size | The banner grows taller, with the same line limits; the width grows with the text scale up to a maximum; past that, more is cut. |

## Opening and closing

- **Arrival.** The banner comes in from the trailing edge into its place,
  short and critically damped, with a fade (macOS; brief P5). Older
  banners move down one place at the same time.
- **Timeout.** It fades out where it is, faster than it came (brief P5),
  and the ones below move up.
- **Swipe.** Dragged toward the trailing edge past a threshold, it carries
  on off the screen and goes to the centre, still live; released short of
  the threshold, it returns.
- **Close.** Fades out; the notification is closed and gone (above).
- **Act.** The default action or a button runs, the banner fades, and the
  app is brought forward with the activation token
  (`platform.md` Notifications 5; `shell.md` Notification Centre 7).
- **Over a game, with Reduce Motion, or while Game Mode is on**: appear
  and leave without movement (Game Mode 9).

## Keyboard

A banner never takes keyboard focus except to reply (`shell.md`
Notification Centre 6), so typing in the window goes on while one shows.

| Action | Key |
|---|---|
| Act on the newest notification | The Notification Centre chord (`notification-centre.md`): while a banner shows, it opens the centre with that notification focused, so its default action, its buttons, Reply and Clear are one chord and a key away. There is no second chord to act on the banner in place (GNOME's Super+N): it would take one more key from apps for what the centre already does |
| Default action | Return, on the focused notification in the centre |
| Move between its actions | Tab, Shift+Tab |
| Reply | Return on Reply; in the field, Return sends, Shift+Return adds a line, Escape cancels and gives focus back to the window that had it (shell runtime 11) |
| Clear | Delete (⌫) on the focused notification in the centre |
| Do Not Disturb | Its chord (`focus.md`) |

Each banner is in the accessibility tree as a notification, announced
politely, critical ones assertively, and its parts readable.

## Pointer

| Input | On | Does |
|---|---|---|
| Hover | A banner | Pauses every banner's time; shows the close control, the actions and the full body. Counts only once the pointer moves: a banner that appears under a resting pointer is not hovered (v1 lessons §3.5) |
| Leave | The stack | Time resumes, with a short grace before anything leaves (stage 4); waiting arrivals come in |
| Click | The banner | The default action, or with none, raises the app; the banner goes |
| Click | A button | That action; the banner goes, unless the notification is `resident` |
| Click | Reply | Opens the field in the banner with focus |
| Click | Close | Closes it; gone from the centre too |
| Click | Options | A pop-up of the actions past the second |
| Middle click | A banner | Closes it, as the close control |
| Drag or horizontal scroll | Toward the trailing edge | Swipes it to the centre |
| Secondary click | A banner | A menu: "Mute {app} for 1 hour", "Mute {app}", "Notification settings…" |

While the pointer is over the stack nothing in it moves: timers pause and
new arrivals wait until it leaves, so a click never lands on a banner that
slid under it. A banner takes no hover while a window is being dragged.

## Shared places

What this sheet adds to `competition.md`, which noted the top right as
free and left banners' place to this sheet:

- **Top right of the main screen: banners against windows.** Banners, in
  the top layer, over a window's trailing toolbar controls and the right
  column's title bar (`window-chrome.md` Shared places;
  `design-references.md` §19 item 4). The stack is at most three banners
  tall, so it never reaches far down.
- **Banners against the revealed menu bar.** No contest: banners start
  under the strip and the bar puts nothing on the right.
- **Banners against menu-bar surfaces.** A banner arriving while a menu,
  Control Centre or the island's expanded form is open on the main screen
  waits until it closes (`menu-bar.md` proposal, adopted here), as v1's
  island held events while its panel was open (v1 lessons §3.5). While
  Notification Centre is open, arrivals go into it and no banner shows.
- **Banners against the island.** Separate places; both may show at once
  (`island.md`). The island never shows a notification and a banner never
  shows an island event.
- **Banners against an app's attention event.** Added: a banner from an
  app suppresses that app's non-urgent attention event in the island for
  the banner's lifetime, since chat apps both notify and request attention
  for one message. The switcher's attention mark still holds
  (`switcher.md`); an urgent request still shows.
- **Banners against the overlay.** The switcher and Spotlight sit above
  banners, which keep showing beneath. Mission Control, the capture tool,
  the session dialog and the widgets dashboard cover the screen: banners
  wait while they are open, so a capture never catches one by surprise.
- **Banners against system prompts and the lock.** A banner waits while a
  system prompt is up (it dims and holds focus, `prompts.md`); none shows
  over the lock (`lock.md`).
- **Bottom-trailing corner.** The capture thumbnail and picture-in-picture
  are at the bottom; banners never go there.
- **Full screen.** Banners are held unless allowed; what shows sits at the
  same anchor over the content. Over a game, solid and still.
- **Keyboard focus.** Only the reply field takes it, at the level of a
  menu-bar surface (`competition.md` Keyboard focus 5); a system prompt
  takes it from the field, which keeps its text.
- **Sound.** One alert sound at a time; OSD volume ticks are separate and
  never held.

## Answers

1. **Where banners appear** (`shell.md` Notification Centre, B24: top
   right of the main screen). **Adopted**, under the reserved strip, with
   a full-screen window on the main screen moving them to a free second
   screen rather than silencing them. Top centre near the island was
   weighed (`design-references.md` §7, §19): near the eye on 3440 px, but
   it covers the window being worked in, contends with Control Centre,
   Notification Centre and the island's events in one place, and would
   move whenever the island grows. For the user 1.
2. **Summary copy** (`shell.md`: "while you were away", "while you
   played"; `platform.md` Focus; `gaming.md` Game Mode). **Adopted** in
   sentence case, as `copy.md` has it, with the count and up to two app
   names; one held notification shows itself instead.
3. **The held-notification summary's form** (`gaming.md`: one summary
   leading to Notification Centre). **Adopted** as one summary banner,
   after the last silencer ends, opening the centre.
4. **Where held banners would have shown** (`gaming.md`: top right, B24).
   **Adopted**: the same anchor; what breaks through a game shows there.
5. **How past records show** (`shell.md`: under Earlier; `platform.md`:
   records without actions). **Changed**: "Earlier" is defined by
   liveness, not age, and holds only what was not dealt with; a record is
   the same card without actions or Reply, whose click opens the app. The
   list's layout is `notification-centre.md`'s.
6. **Shell runtime: what shows only on the main screen** (proposal:
   events, Spotlight, dialogs). For banners, **adopted**: main screen
   only, except the full-screen case above.
7. **A banner on a rejected hand edit** (`island.md` deferral).
   **Adopted**: once per broken state of the file, with Open in editor.
8. **Silencing set** (`shell.md` Notification Centre 4: Focus, full
   screen, a screen share, the app muted). **Adopted**, with a share of a
   window not silencing, the user away (idle, sleep, lock) added, and
   critical notifications held only during a screen share.

## Copy

| Where | String |
|---|---|
| Banner time | "now"; in the centre, `notification-centre.md`'s; on a re-shown held one, "12 min ago" |
| Grouped | "{n} new" |
| Overflow row | "{n} more in Notification Centre" |
| Buttons | "Reply", "Send", "Cancel", "Options" |
| Close (accessible name) | "Close notification" |
| Secondary click | "Mute {app} for 1 hour", "Mute {app}", "Notification settings…" |
| Summary | Title "While you were away" or "While you played"; body "{n} notifications from {app}", "{n} notifications from {app} and {app}", "{n} notifications from {app}, {app} and {k} more"; action "Show" |
| Lock | "{app}" with "{n} notifications"; with previews, the summary only |
| Settings › Notifications | "Allow notifications", "Banner style", "Temporary", "Persistent", "Play sound", "Show badges", "Show over full-screen windows", "Hold banners while a window is full screen", "Hold banners while sharing your screen", "On the lock screen", "Don't show", "Show count", "Show previews" |
| Settings › Sound | "Alert sound", "Play sound for notifications", "Alert volume" |
| Desktop's own | "{drive} connected" (Open, Mount, Eject); "{app} installed" (Open, Show in Files); "Update installed" (Restart…); "{event} starts now" or "{event} at {time}"; "{provider} needs you to sign in" (Sign in…); `copy.md`'s rejected-edit string (Open in editor); `copy.md`'s crash notice (Reopen, Report…) |
| Earlier | "Earlier" (the heading is `notification-centre.md`'s to place) |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Where banners appear.** Top right of the main screen, under the
   strip, as on macOS; or top centre, under the island, near where the eye
   rests on the ultrawide (GNOME, and where v1's toasts were).
   *Recommended: top right.* The centre is where the work is, and where
   Control Centre, Notification Centre and the island's events already
   open; a banner there would wait or move every time the island grows
   or is hovered, while the corner, now free (`competition.md`), contends
   with nothing but the right column's toolbar. Its distance from the
   eye is the cost; the sound and the arrival's movement in the periphery
   carry it.
2. **What the lock screen shows by default.** Nothing; each app with a
   count; or previews (app and summary). *Recommended: each app with a
   count*, as macOS's "previews when unlocked": enough to know whether to
   unlock, nothing a passer-by can read.
