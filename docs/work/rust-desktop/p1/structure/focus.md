# Focus and Game Mode

Batch 2. Answers to: `platform.md` § For the design process (Focus: the
summary's copy, with `notifications.md`; how the current Focus is shown,
completing `island.md`) and its open item on which mode wins, as
presented; `gaming.md` § For the design process (Game Mode: where its state
is shown, its effects as presented; the summary's form and copy, with
`notifications.md`; the top-edge reveal's look, left to stage 5);
`shell.md` Control Centre 7, 8 and its proposed module list (the Focus
and Game Mode modules); `apps.md` § For the design process (Settings:
whether Focus is a pane, whether Game Mode's settings fold into it). B27,
B30.

## Purpose

What decides when the desktop may interrupt. Focus modes work as on macOS
(B30): Do Not Disturb, Game Mode (B30's Gaming Focus) and any modes the
user makes, one at a time, each saying which apps and people still get
through, turned on by hand, on a schedule, by an app in front, or, for
Game Mode, by any detected game in front, windowed or full screen. Focus
governs banners and notification sounds; it never hides live activity,
privacy marks, OSDs, prompts or badges (`island.md`). Game Mode adds the
system's other steps back while a game plays (`gaming.md` Game Mode),
presented here so the user can see what it changed. Focus is not a
surface: it shows through the island's Focus glyph and events
(`island.md`), Control Centre's Focus module (`control-centre.md`) and a
Settings pane, and what it holds back is `notifications.md`'s.

## Content

**The modes.**

| Mode | Always present | Starts by itself | Notes |
|---|---|---|---|
| Do Not Disturb | Yes, cannot be deleted | Only by a schedule the user adds; v1's quiet hours are imported once as one (B27) | Allows no apps by default |
| Game Mode | Yes, cannot be deleted | When a game is in front, on by default (`gaming.md` Game Mode 1, 2) | B30's Gaming Focus; named Game Mode wherever the user reads it, the name v1 used and `island.md`'s events already use, so one thing has one name |
| The user's own | No: "Add Focus…" in Settings, with a name and a symbol from a small set (stage 4) | By its schedules | Deleted from its page |

No other modes ship (macOS's Work, Sleep and Personal are suggestions the
user can make in a step; brief P1).

**A mode's page** (Settings › Focus › the mode):

| Section | Content | Default |
|---|---|---|
| Allowed notifications: Apps | Apps whose banners and sounds still come through | none |
| Allowed notifications: People | Names whose calls and messages still come through. Applies only where a source names its sender, which today is the phone provider (B14, inventory §8): the freedesktop protocol carries no sender. Hidden while no such source is set up | none |
| Allow time-sensitive notifications | Critical-urgency notifications come through (`platform.md` Focus 1) | on |
| Turn on automatically | Schedules, each one of: **Time** (from, to, days); **App** (while a chosen app is in front, macOS's app trigger). Game Mode adds **When a game is in front** and the list **Off for these games** (below) | none; Game Mode's game trigger on |
| Delete Focus | The user's own modes only | |

Game Mode's page has nothing else. Its system effects (below) are fixed,
not options; VRR and tearing are per output in Settings › Displays and
apply to any full-screen window or game whatever the Focus (`gaming.md`
Presentation 3, 4). So Game Mode needs no pane of its own (`apps.md`
item); `settings.md` places the Focus pane.

**What Focus changes, and what it does not.**

| Thing | While a Focus is on |
|---|---|
| Notification banners | Held, except allowed apps and people and, if allowed, time-sensitive ones; counted for the summary (`notifications.md`) |
| Notification sounds | Held with their banners |
| Notification Centre | Unchanged: everything arrives there |
| Badges, the switcher's marks | Unchanged (`switcher.md`) |
| Live activities, privacy marks, OSDs | Unchanged (`island.md`: Focus governs banners and sounds, not live activity) |
| The island's events | Unchanged, attention included: they are the user's own actions or a window asking for them, not notifications |
| A timer ending, with its sound | Unchanged: the user set it to be told (macOS lets alarms through) |
| System prompts | Unchanged: something is waiting on the user (`prompts.md`) |
| System sounds (screenshot, volume tick, trash) | Unchanged: feedback for the user's own action |
| Apps that ask | The notification server's `Inhibited` property is true while banners are held for any reason, so apps that read it stop their own sounds and popups (`design-references.md` §7, KDE) |

**One at a time, and which wins.** `platform.md` leaves the rule to its
packet; this is the behaviour presented, for the packet to implement:

1. **Game Mode, while a detected game is in front, takes over** whatever
   else is on (B30: it turns on for any detected game in front). When the
   game leaves the front, the Focus that was on before returns if it is
   still due. For the user 1.
2. **A Focus turned on by hand** (Control Centre, a chord, the CLI) stays
   until turned off or its duration ends; schedules and app triggers do
   not replace it.
3. **Among automatic starts, the latest wins**; when it ends, the one
   still due comes back.
4. **Turning off by hand a Focus a schedule started** keeps it off until
   that schedule next starts (macOS).
5. **Turning Game Mode off while a game is in front** keeps it off for
   that game from then on (macOS 26 remembers it per game,
   `design-macos.md` §3.19). The game joins "Off for these games" in
   Game Mode's page, where it is removed. This is also the answer to a
   program wrongly detected as a game.

**How the current Focus shows.**

- **The island's Focus glyph** (`island.md`): the mode's symbol, shown when
  a Focus is on, on every screen's island; *Show when active* by default.
  Its click opens Control Centre at the Focus detail.
- **The island's events** (`island.md`): "{focus} on" and "{focus} off"
  when the change was automatic or by chord, not when made in Control
  Centre, where the module shows it. Over a full-screen window, a game
  included, events are dropped (`island.md` The queue 11), so a game
  starting full screen says nothing; the glyph is there when the island is
  revealed.
- **Control Centre's Focus module.** Its place and size are
  `control-centre.md`'s; what it holds is set here:
  - off: "Focus" and the last-used mode's symbol (Do Not Disturb's until
    another mode is used), since the symbol turns that mode on;
  - on: the mode's symbol and name, marked as on (the accent, brief P6),
    and a second line saying how it ends or why it is on: "Until 07:00",
    "For 47 min", "Until the end of {event}", "Schedule", "{app}", or the
    game's name for Game Mode;
  - at Medium its symbol turns the last-used mode on or off and the rest of
    the module opens the detail (macOS's Focus module); at Small the whole
    module is the switch, and Return or the secondary-click menu opens the
    detail.
- **The Focus detail**, a page in the panel (`island.md`): each mode as a
  row, the one on marked. A row turns its mode on until turned off, or
  off. Each row opens its durations: "For 1 hour", "Until this evening"
  (before evening) or "Until tomorrow morning" (after), and "Until the end
  of this event" while the calendar has one in progress (`platform.md`
  Calendars). Game Mode's row, with a game in front, reads "Off for
  {game}" as its off choice. The page ends with "Focus Settings…".
- **One switch per thing** (v1 lessons §8): Game Mode is a row of the
  Focus module, not a module of its own. `shell.md`'s proposed list has
  both; `control-centre.md` takes this.
- **The lock** shows the current mode's symbol beside its clock (macOS);
  where, is `lock.md`'s.

**Game Mode's other effects, as presented** (`gaming.md` Game Mode; brief
P7). Each is shown where the thing it changes already lives, and nowhere
at rest:

| Effect | When | Where the user sees it |
|---|---|---|
| Banners and their sounds held, except time-sensitive and allowed apps | Game Mode on | Nothing while playing; "While you played" afterwards (`notifications.md`) |
| Performance power profile held | Entering Game Mode (`gaming.md` Game Mode 3) | Control Centre's Energy Mode module shows High Power held by Game Mode (`control-centre.md`); a change the user makes stands and is kept on leaving (Game Mode 4) |
| Night light off | Entering Game Mode | The Night Light module reads "Off during Game Mode"; turning it on stands |
| No dim, lock or display sleep | While a game window is in front, whatever the Focus (Game Mode 8) | Nowhere: it is the normal state of playing |
| VRR | While any full-screen window is in front, by the output's setting (B30) | Settings › Displays only |
| Shell surfaces solid and still: no transitions, no blur | Game Mode on, windowed games included (Game Mode 9; `competition.md` For the user 1) | The surfaces themselves |
| The shell beside a windowed game | Game Mode on, windowed | Shown and working as normal (brief interview): the island at rest with Game Mode's glyph, the menu bar on demand, Control Centre on hover |
| The shell over a full-screen game | The game's Space in front | Hidden; the island and the bar by a dwell at the top edge (`island.md`, `menu-bar.md`; its look is stage 5's); privacy marks and OSDs without revealing (Game Mode 5, 7) |
| The escape set | A game window focused, whatever the Focus (B30) | Nothing; the chords are the documented set |

Leaving Game Mode undoes only what it changed (Game Mode 4): the profile
hold is released, night light returns if Game Mode turned it off, the
earlier Focus returns, and the held notifications are summarised.

## States

| State | What shows |
|---|---|
| Off | No glyph; the module reads "Focus". Banners as normal. |
| On by hand | The glyph; the module with the mode's name and no end, or its duration ("For 47 min"). |
| On by schedule | The same, with "Until {time}"; an event when it starts and ends (unless over full screen or locked). |
| On by an app in front | The same, with the app's name; it ends when the app leaves the front. |
| Game Mode, windowed game | The glyph shows Game Mode; the module shows the game's name; the shell solid and still, otherwise normal; event "Game Mode on". |
| Game Mode, full-screen game | Nothing visible; no event; the glyph when the island is revealed. |
| Game Mode by hand, no game | As on by hand; a manual on stays until turned off (`gaming.md` Game Mode 1), including after a game comes and goes. |
| Game Mode off for this game | No glyph; the detail's Game Mode row reads off for this game; the game is listed in Settings. |
| Two due at once | The winner by the order above; the other shows nowhere until it returns. |
| Ending | The glyph goes; "{focus} off" if automatic; the summary banner if two or more were held (`notifications.md`). |
| Second screen | Focus is one state for the session: every island shows the glyph. Banners are on the main screen anyway (`notifications.md`). |
| Locked, idle, asleep | Focus continues; schedules start and end, with no events; the lock shows the symbol. |
| Error | The calendar unavailable: no "Until the end of this event". An app trigger's app removed: the trigger stays, marked "Not installed", and never fires. A schedule whose times are equal is refused when entered. The Focus service gone: the glyph and module show their last state, stale, never a stalled frame (shell runtime 8). |
| Reduce Motion, Reduce Transparency, Increase Contrast, largest text | Focus has no surface of its own: the island, Control Centre and Settings take their own forms. |

## Opening and closing

Focus opens nothing. Turning a mode on or off changes the island's glyph,
which is the island's morph (`island.md`), and raises the event above when
automatic or by chord. The Focus detail opens and closes as a page in
Control Centre (`island.md`, `control-centre.md`); Settings › Focus as a
Settings pane (`settings.md`). Nothing in a mode change transitions over
a game (Game Mode 9).

## Keyboard

| Action | Key |
|---|---|
| Turn Do Not Disturb on or off | The Do Not Disturb chord in P5's table (*proposed*: Ctrl+Option+Cmd+D; v1's Cmd+Shift+D is VS Code's Run and Debug on a Mac). Also the Do Not Disturb key of a Mac-layout keyboard that sends one (Apple's F6) |
| Turn Game Mode on or off | The Game Mode chord (*proposed*: Ctrl+Option+Cmd+G). Over a game it goes to the game (`gaming.md` Keys 2), so it acts only from the desktop: on before starting an undetected game, off afterwards |
| Open the Focus detail | Ctrl+F8, then arrows to the Focus module and Return; or Return on the island's Focus glyph (`island.md`) |
| In the detail | Up, Down between modes; Return or Space turns the mode on or off; Right opens its durations, Left closes them; Escape goes back |
| Settings › Focus | The Settings app's own keys (`settings.md`) |

Both chords are desktop chords, taken before Cmd translation (B21), given
up under keyboard-shortcuts-inhibit, and each raises the island's event.

## Pointer

| Input | On | Does |
|---|---|---|
| Click | The Focus module's symbol | Turns the last-used mode on or off |
| Click | The rest of the module | Opens the Focus detail |
| Click | A mode's row in the detail | Turns it on until turned off, or off |
| Click | A row's disclosure | Opens its durations |
| Click | The island's Focus glyph | Control Centre at the Focus detail (`island.md`) |
| Secondary click | The glyph | Its tier (`island.md`) |

Hover opens nothing of Focus's own; Control Centre's hover guards apply
(`island.md`).

## Shared places

What this sheet adds to `competition.md`:

- **One Focus at a time.** Game Mode over every other while a game is in
  front; a Focus turned on by hand over schedules and app triggers; among
  automatic starts, the latest.
- **Focus against the other holds.** A full-screen window, a screen share
  and the user away also hold banners (`notifications.md`). They are
  situations, not modes: no toggle in Control Centre, no glyph, the same
  summary. Only Focus is a mode, so the user never manages two silencers
  (v1 lessons §8, Do not repeat: Focus beside DND).
- **The island's glyph.** The only sign at rest; Game Mode has no second
  glyph (`island.md`).
- **Control Centre's modules.** One Focus module with Game Mode as a row.
  The Energy Mode and Night Light modules show Game Mode's holds,
  so each switch stays in one place and the user can override it there.
- **Island events over full screen.** Focus and Game Mode events are
  dropped there with every other event (`island.md`).
- **Keys.** The Do Not Disturb and Game Mode chords join P5's table; over a
  game they go to the game.
- **Idle.** A game window in front holds idle off whatever the Focus;
  Game Mode does not own it.

## Answers

1. **Focus, the summary's copy** (`platform.md`). **Adopted** as
   `copy.md` has it: "While you were away", or "While you played" when
   Game Mode held any of it; its form is `notifications.md`'s.
2. **Focus, how the current Focus is shown** (`platform.md`; answered for
   the island in `island.md`). **Completed**: the glyph and events as
   `island.md` has them, Control Centre's Focus module saying which mode
   and until when, and the lock showing its symbol.
3. **Which mode wins** (`platform.md`, open for its packet). **Proposed**
   above for the packet: Game Mode while a game is in front, then by hand,
   then the latest automatic start; turning a scheduled one off holds
   until its next start; Game Mode turned off for a game stays off for it.
4. **Game Mode, where its state is shown** (`gaming.md`; `island.md`: the
   Focus glyph). **Adopted**, with the game's name in the Focus module.
5. **Game Mode, the summary's form and copy** (`gaming.md`: one summary,
   "while you played", leading to Notification Centre). **Adopted**; see
   `notifications.md`.
6. **Game Mode, the top-edge reveal's look** (`gaming.md`). Its structure
   is `menu-bar.md`'s and `island.md`'s; its look is stage 5's. Nothing
   here.
7. **Control Centre's proposed modules** (`shell.md`: Focus and Game Mode
   separately). **Changed**: Game Mode is a row of the Focus module, since
   it is a Focus (B30) and one thing gets one switch.
8. **Focus as a Settings pane** (`apps.md`: yes, as on macOS).
   **Adopted**. **Game Mode's settings** (`apps.md`: fold into the Gaming
   Focus or keep a pane): **fold**, as Game Mode's page in the Focus pane;
   VRR and tearing stay in Displays, as they are per output and not tied to
   Game Mode.
9. **Named modes with allowed apps and people and a schedule; v1's quiet
   hours as a schedule** (`shell.md` Control Centre 7). **Adopted**, with
   people applying where a source names its sender (the phone provider),
   an app trigger beside the time schedule (macOS), and quiet hours
   imported as a schedule on Do Not Disturb.
10. **The Gaming Focus's name.** **Changed** for the user: "Game Mode"
    everywhere it is read, matching `island.md`'s events and v1; B30's
    "Gaming Focus" stays the documents' term.

## Copy

| Where | String |
|---|---|
| Modes | "Do Not Disturb", "Game Mode" (Apple's title case, `copy.md` rule 1), the user's own names as typed |
| Control Centre module | "Focus"; "{focus}" with "Until {time}", "For {n} min", "Until the end of {event}", "Schedule", "{app}", "{game}" |
| Focus detail | "For 1 hour", "Until this evening", "Until tomorrow morning", "Until the end of this event", "Off for {game}", "Focus Settings…" |
| Settings › Focus | "Add Focus…", "Name", "Symbol", "Allowed notifications", "Apps", "People", "Add…", "Allow time-sensitive notifications", "Turn on automatically", "Add schedule…", "Time", "App", "From", "To", "Every day", "Weekdays", "Weekends", "While {app} is in front", "When a game is in front", "Off for these games", "Not installed", "Delete Focus" |
| Island events | `island.md`'s: "{focus} on", "{focus} off", "Do Not Disturb on", "Game Mode on", "Game Mode off" |
| Other modules | `control-centre.md`'s: Energy Mode's hold, "Off during Game Mode" for Night Light |
| Summary | `notifications.md`'s |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Game Mode over a Focus turned on by hand.** A game coming to the
   front switches to Game Mode even if the user had turned on Do Not
   Disturb or a mode of their own, and gives it back when the game
   leaves; or a Focus turned on by hand wins, and the game runs under it
   with Game Mode's allowed apps ignored. *Recommended: Game Mode takes
   over.* B30 says it turns on for any detected game in front; its
   allowed apps are the ones chosen for playing (a voice chat); and the
   user cannot reach Control Centre over a full-screen game to fix a
   wrong mode, so the predictable rule is the safe one.
