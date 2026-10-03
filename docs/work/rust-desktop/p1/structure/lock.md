# Lock, greeter and idle

Batch 2. Answers to: `shell.md` Lock screen (what it shows, the greeter's
face) and Lock screen 1–11, Wallpaper and idle dimmer 3–4 (the dimmer,
which `wallpaper.md` passes here), Shell runtime Open (what the lock falls
back to); `platform.md` Lock, PAM and idle, Session start and greeter 1–3;
`compositor.md` Idle, DPMS and sleep, Session and robustness 4 and 9 and
its Open item (what the greeter reads before anyone has logged in);
`gaming.md` Game Mode 8; B12, B26, B28, B30.

## Purpose

The lock keeps the session private while the user is away and lets them
back in with nothing more than a typed password or a finger. It is the
compositor's own (B12, B28), so no other program can take it, end it or
outlive it. The same face, run by the compositor in greeter mode under
greetd, is the login screen (B26). Idle is how the screen gets there by
itself: dim, then lock, then display off. The lock is not a second
desktop: it offers getting back in, what is playing, that something
arrived, and sleeping or turning the machine off. Nothing else.

## Content

### The face

On the screen that has it (Many screens, under States), top to bottom.

| Part | What it is | Data from |
|---|---|---|
| Date and time | The date over a large time, below the island's strip, as macOS since Sonoma (`design-macos.md` §3.10). The lock's own clock, not the island's (`island.md` Locked); the clock setting's 24-hour form, no seconds. The current Focus's symbol beside the date while one is on (`focus.md`) | timedate1, the clock setting, Focus |
| Now Playing | One row: art, title, artist, previous, play or pause, next. Only while a player plays, or was paused recently (stage 4) | MPRIS, the island's media source (the most recently playing player) |
| Notifications | What arrived since locking, by `notifications.md`'s setting and Focus; cards are `notifications.md`'s, this sheet places them | The notification server |
| The user | Picture and full name | AccountsService, else the login name |
| Password field | Takes keys the moment they are typed (Lock 1); dots, no echo; a fingerprint mark at its trailing end while the reader listens | The lock's PAM thread; fprintd |
| Status line | Under the field, one message at a time (below) | The PAM thread, the keyboard state, fprintd |
| Bottom row | Sleep, Restart, Shut Down; the layout menu when more than one layout is set; the on-screen keyboard when Accessibility turns it on; the battery on a machine that has one | logind, the keyboard state, UPower |

Over the face the island draws only its privacy form and OSDs, in its own
place (`island.md` Locked). The backdrop is the wallpaper's softened form
on every screen (`wallpaper.md`).

**The status line** has its slot reserved, so a message appearing never
moves the field (brief P5). One message shows, in this order of priority:
locked out, checking, wrong password, Caps Lock, the fingerprint's message.

**Awake and dozing.** Any input wakes the face and everything above shows.
After a short pause with the field empty (v1: 12 s) it dozes: the date and
time, Now Playing and notifications stay, the user, field and bottom row
fade, and the pointer hides (v1 `b5bae3c`). A key or pointer movement wakes
it; a printable key is also typed into the field.

**Left out from macOS:** the password hint (no store for one on Linux), the
message when locked (one user, at home), and 26's clock styles, which are a
look rather than structure.

### Keyboard layout at the password

Each keyboard types through its own profile (`compositor.md` Keyboards
2–4), so the Mac keyboard and the PC keyboard behind the KVM type the same
password correctly with no switching: the Mac variant sits on its device
only. The layout menu is for languages. It shows only when more than one
layout is set, names the current one by its short name, and changes by a
click or the layout chord (P5's table; macOS uses Ctrl+Space). After a wrong
password with more than one layout set, the menu is marked, since a wrong
layout is the likeliest cause. A change raises the island's layout OSD.

### PAM and fingerprint

- **Off the main thread.** The password goes to Isle's PAM service on its
  own thread (B12; `platform.md` Lock 5). While it is checked the face is
  pending: the field keeps its dots and takes no keys, the status line says
  "Checking", the reader keeps listening, and the main loop never waits.
  A check that runs long (stage 4) says "Still checking"; one that errors
  clears the field and says so.
- **Fingerprint beside the password** (v1 D33; `platform.md` Lock 6). A
  match unlocks; a miss says "Not recognised" and listens again; at
  fprintd's retry limit the mark dims and says "Use your password" until
  the next wake. Nothing about the reader blocks or clears the field.
- **Wrong password.** The field shakes, clears and keeps focus; the status
  line says so, with the count from the second attempt (v1). The count
  resets at unlock.
- **Locked out.** If Isle's PAM service stacks `pam_faillock` (its contents
  are the packet's; Arch's default is 3 failures, then 10 min), the lock
  and the polkit prompt share one count. While PAM reports the account
  locked, the field is disabled and the status line counts down the time
  left; the fingerprint still unlocks and the bottom row still works.
- **The keyring at unlock.** The login keyring normally stays unlocked
  while the screen is locked; where it does not, `pam_gnome_keyring` in the
  lock's stack unlocks it from the password, and after a fingerprint unlock
  the keyring prompt follows when the keyring is next asked for
  (`prompts.md`; Lock 9).

### Power on the lock

Sleep acts at once. Restart and Shut Down ask once, in place: the row
becomes "Restart now?" with Cancel and Restart. There is no countdown and
no list of open apps, because the lock does not name what is open behind
it. Apps are then asked to quit as by Cmd+Q (`compositor.md` Session 8).
An app that will not quit (it asks to save) stops the action; the status
line says so without naming it, and after unlock that app is in front with
its question. Log Out is not on the lock: it would end the session without
the password.

### Notifications on the lock

Coordinated with `notifications.md`, which draws the cards; this sheet
places them under Now Playing, newest first, capped with a count of the
rest (stage 4). What they show is `notifications.md`'s setting (Settings ›
Notifications): nothing; each app once with its icon, name and count (the
default, as macOS's "Show previews: When unlocked"); or previews, the
summary without body or actions.
Only what the current Focus lets through reaches the list (`focus.md`). A
click wakes the field, and the notification's default action runs after
unlock. Nothing on the lock dismisses a notification; after unlock they are
in Notification Centre. No banner shows over the lock, and a notification
never lights a dark or dimmed screen.

### The greeter

The same face, run by the compositor in greeter mode (B26; `compositor.md`
Session 9), with these differences:

- **Users.** With one user, as on this machine: their picture and name,
  the field ready, no list. With two or more: a row of pictures and names;
  choosing one shows its field. Users come from AccountsService, system
  accounts left out; there is no "Other" entry for a typed name.
- **Sessions.** A session menu in the bottom row, only when more than one
  Wayland session is installed (while v1 is still the daily desktop:
  Hyprland and Isle), remembering the last choice.
- **Fingerprint sign-in**, when turned on (opt-in, v1 D34): the greetd
  session is created as the greeter shows, the status line shows what PAM
  says, and a typed password is held until PAM asks for it. Return before
  then shows "Waiting for the fingerprint reader". That wait is why it is
  opt-in.
- **No session parts**: no Now Playing, notifications, island or OSDs.
- **Power**: Sleep, Restart and Shut Down act at once; there is nothing to
  quit.
- **Look**: the shipped tokens, never the user's token file (`platform.md`
  Session start 2), with the choices the user's session publishes for it:
  the softened wallpaper, light or dark, the accent, the keyboard layouts
  and per-device profiles, and which screen is main (For the user 1).
  Before any session has published them: the shipped wallpaper, dark, the
  system keymap, the first screen found.
- **A session that fails to start** returns to the field with
  "Couldn't start {session}." (Copy).
- **Autologin** (greetd's initial session, `isle setup`) skips the greeter.

### Idle

Three steps, each at its own timeout from the last input on any device. A
timeout of 0 turns its step off and never fires at once (v1 D18); a changed
timeout counts from the current idle time (`compositor.md` Idle 2).

| Step | What happens | Starting value (v1's) |
|---|---|---|
| Dim | Every screen darkens over a slow fade, everything on it included: windows, the island, a prompt, the lock. Input restores it at once and does not unlock (`shell.md` Wallpaper 3) | 5 min |
| Lock | The lock, as by Ctrl+Cmd+Q | 10 min |
| Display off | Outputs off | 15 min |

The three are set in Settings › Lock Screen (`settings.md`), as macOS keeps
display sleep and the password delay there.

- **Locked.** Once locked, screens dim and turn off a minute after the last
  input on the lock (stage 4 sets the value), whatever the unlocked
  timeouts: nothing on the lock needs a lit screen.
- **What holds them off**, all three together: idle-inhibit from a visible
  surface (a playing video), `org.freedesktop.ScreenSaver` Inhibit (served
  by Isle), logind idle inhibitors, a game window in front, windowed or
  full screen (`gaming.md` Game Mode 8), and controller input, which counts
  as activity (roadmap P18). While locked, nothing holds the locked
  display-off: nothing behind the lock can be seen.
- **Not shown at rest.** An inhibitor is the normal state of watching a
  video, so it has no glyph (brief P1). Control Centre's display detail
  names what holds the screen awake (`control-centre.md`); the CLI lists it.
- **Waking.** From dim, the input that ends it is delivered as usual: the
  screen was readable. From display off, the waking input is consumed, so a
  click never lands on something unseen; on the lock, keys typed after it
  go into the field before the screen has lit. Waking sends the screen state
  to every output again, and a screen returning through the KVM wakes with
  the rest (v1 D76).
- **Sleep** is not an idle step. The session locks before every sleep, and
  sleep proceeds only once every screen shows the lock (Lock 4).

## States

| State | What shows |
|---|---|
| Locking | The first frame after a lock request shows the softened backdrop on every screen; the face fades in over it. No frame of the session is shown after the request (Lock 5, acceptance bar 10). |
| Awake | The whole face. |
| Dozing | The date and time, Now Playing, notifications; no pointer. |
| Typing | Dots in the field; Caps Lock in the status line while it is on. |
| Pending | "Checking", field held, reader listening. |
| Wrong password | Shake, cleared field, the message and count; the layout menu marked if there is one. |
| Locked out | Field disabled, the time left counting down; fingerprint and bottom row still work. |
| Fingerprint | The mark while listening; "Not recognised"; dimmed with "Use your password" at the limit. |
| Unlocking | The face fades, then the backdrop dissolves into the session, faster than the lock came. |
| Empty | Nothing playing, no notifications: the date and time, the user and the field. |
| Loading | Before AccountsService answers, the login name and no picture; Now Playing and notifications appear as their sources answer; the field never waits. |
| Error: the lock surface panics | Rebuilt at once, still locked (B9). After repeated panics (count is the packet's) it is replaced by the minimal lock: a solid backdrop, the time, the field and its status line, PAM still off the main thread, with "The lock screen had a problem." above the field. Session content is never shown (acceptance bar 10). After unlock the crash notice says what happened (`prompts.md`). |
| Error: the compositor restarts while locked | It comes back locked from the marker, its first frame the lock (`platform.md` Lock 4). |
| Error: PAM fails | "Couldn't check the password. Try again.", field cleared. |
| Many screens | The face on the screen with the pointer (Lock 6); the others show the backdrop with the date and time. Moving the pointer to another screen moves the face there with its dots kept. A screen added while locked shows the lock in its first frame. The island's privacy form shows on every screen. |
| Game in front | The lock chord is in the escape set (B30). Locking releases the pointer lock and grabs and ends any keyboard-shortcuts inhibitor; the game keeps running and playing sound. At unlock the game is in front with its focus and pointer lock back, as after the switcher (`switcher.md`). |
| Full-screen window | Covered as anything else. |
| A prompt showing | Set back to waiting with its field cleared; it returns after unlock (`prompts.md`). Prompts raised while locked wait. |
| A capture running | It keeps running; the privacy form shows. A screen recording or share carries the lock as drawn, never the session; a window recording or share holds its last frame until unlock (`capture.md`). |
| Focus on | Its symbol beside the date; it filters the notifications list. |
| Greeter | As the face, with the greeter's differences above. |
| Greeter, fingerprint sign-in | PAM's message in the status line; a typed password held. |
| Dimmed | Everything darker; any input restores. |
| Display off | Nothing; input wakes. |
| Idle held | Nothing shows; Control Centre's display detail names the holder. |
| Reduce Motion | No shake and no movement; the face, the unlock and the dim fade. |
| Reduce Transparency | The softened backdrop stays (it is a picture); the field, cards and bottom row turn solid. |
| Increase Contrast | Edges on the field, the cards and the bottom row; the status line's messages in their stronger forms. |
| Largest text size | The date, name, field, status line and bottom row grow; the large time keeps its size. |

## Opening and closing

- **Locks on** Ctrl+Cmd+Q, Lock Screen in the system menu, L in the session
  dialog, a hot corner set to Lock Screen, idle, lid close, logind's Lock
  signal (`loginctl lock-session`, which is how scripts lock), before every
  sleep, and the CLI (Lock 4). Other lockers find no `ext-session-lock`
  and fail (B28).
- **Opening.** The backdrop is complete in the first frame; only the face
  fades in. macOS's zoom is left out (brief P5).
- **Unlocking** is the one transition that may show the session: the face
  goes, then the backdrop dissolves into the desktop. It is the softened
  form clearing to the plain one that `wallpaper.md` hands here.
- **Dim** fades in slowly so it reads as a warning, and goes at once.
- **Display off** has no motion of its own; the dim precedes it.
- **Greeter to session.** The greeter's face goes; the session's compositor
  starts on the softened wallpaper and clears it as its first surfaces are
  ready.
- Values are stage 4's. In Game Mode and under Reduce Motion everything
  here is a fade or a cut.

## Keyboard

| Action | Key |
|---|---|
| Type the password | Any printable key, dozing or not |
| Unlock | Return |
| Clear the field | Escape |
| Move through the face | Tab, Shift+Tab: field, layout menu, bottom row, Now Playing, notifications |
| Act on the focused control | Return or Space |
| Change layout | The layout chord, or the menu |
| Media | Play or pause, next and previous keys |
| Volume and brightness | Their keys, with the island's OSD |
| Power key | Wakes the face with focus on Sleep in the bottom row; the session dialog never opens over the lock |
| Greeter: choose a user | Arrows and Return; Escape goes back to the list |
| Wake from display off | Any key; the waking key is not typed |

While locked only these keys act. Every desktop chord, the escape set
included (Cmd+Tab, Mission Control, screenshots), does nothing; Ctrl+Cmd+Q
does nothing. Focus starts in the field and returns there after each
action.

## Pointer

- **Movement** wakes the face and moves it to the pointer's screen.
- **Click**: on the field, focus (it has it already); on a Now Playing
  control or a bottom-row action, that action; on a notification, wake the
  field, then its action after unlock; on the layout or session menu, open
  it.
- **Secondary click, scroll, drag**: nothing.
- **Hover** opens nothing, so no hover guards apply.

## Shared places

- **Over everything.** The lock is above every layer, system prompts
  included (`competition.md`). Only the island's privacy form, the island's
  OSDs and the idle dim draw over it.
- **Top centre on the lock.** The island's privacy form and OSDs keep their
  place; the lock's date and time sit below the strip, so the two never
  overlap.
- **The idle dim** is drawn over everything on each screen, the lock and a
  prompt included; it is not a surface and takes no input but the one that
  ends it. Adds a row to `competition.md`.
- **Keyboard focus.** The lock owns the keyboard (first in
  `competition.md`'s order). Only the media, volume, brightness, layout and
  power keys act; the escape set does nothing.
- **Prompts while locked** wait, and one on screen when the lock comes goes
  back to waiting (`prompts.md`). The session dialog and Force Quit never
  open over the lock; the power key goes to the lock's bottom row.
- **Banners while locked** do not show; what arrives goes to the lock's
  list by its setting, and to Notification Centre (`notifications.md`).
- **Now Playing on the lock against the island's media activity**: the
  island's media activity waits while locked (`island.md`); the lock's row
  is the one view, with the same player.
- **A game's keys.** The lock chord is in B30's escape set; locking takes
  the pointer and the keys from the game and gives them back at unlock.
- **Hot corners.** Lock Screen and Put display to sleep are corner actions;
  over the lock, corners do nothing.
- **The greeter** owns every screen; there is no island, no layer for other
  programs and no session to cover.

## Answers

- **Lock screen, what it shows** (`shell.md` proposal: time and date, the
  user, the password field): **changed** by adding Now Playing,
  notifications by setting, a status line, and Sleep, Restart and Shut Down,
  which `shell.md` Lock 3 already requires. The time leads, as macOS since
  Sonoma; the user and field sit low and wake on input (v1).
- **The greeter's face** (proposal: unstated): the lock's face with the
  user's picture and name, a user list only with more than one user, a
  session menu only with more than one session, and fingerprint sign-in as
  D34 has it.
- **The lock's backdrop** (answered by `wallpaper.md`): **adopted**; the
  unlock is the softened form clearing to the plain one.
- **What the lock falls back to if its surface keeps panicking**
  (`shell.md` Shell runtime and Lock screen, Open; `compositor.md` Session 4):
  the minimal lock under States, with its line of copy and a crash notice
  after unlock. How many panics trigger it is the packet's.
- **What the greeter reads before anyone has logged in** (`compositor.md`
  Session, Open): the choices the last session published for it (wallpaper,
  appearance, accent, layouts and per-device profiles, the main screen),
  else shipped defaults. Where the session writes them so the greeter's
  user can read them is the packet's.
- **The greeter draws with shipped tokens, never a user's** (`platform.md`
  Session start 2): **changed** to shipped tokens plus the published
  choices above, never the token file (`design-references.md` §12, DMS;
  For the user 1).
- **Keyboard layout at the password** (`shell.md` Lock 2): per-device
  profiles make the KVM's two keyboards need no switching; a layout menu
  for languages, only when there are several.
- **Power on the lock** (`shell.md` Lock 3): Sleep, Restart and Shut Down;
  Restart and Shut Down confirm in place; Log Out left out.
- **Media on the lock** (Lock 3): **adopted** as one Now Playing row.
- **Notifications on the lock** (Lock 3, by setting): **adopted** from
  `notifications.md`, which asks the default (its For the user 2); this
  sheet places them under Now Playing.
- **The idle dimmer** (`shell.md` Wallpaper and idle dimmer 3–4, passed here
  by `wallpaper.md`): **adopted** with v1's timings as starting values, a
  shorter display-off once locked, controller input counting as activity,
  and no indicator at rest.

## Copy

| Where | String |
|---|---|
| Field | "Enter password"; with a finger enrolled, "Password or fingerprint" |
| Status: Caps Lock | "Caps Lock is on" |
| Status: checking | "Checking", then "Still checking" |
| Status: wrong | "Wrong password. Try again."; from the second, "Wrong password, {n} attempts. Try again." |
| Status: locked out | "Too many attempts. Try again in {n} min." |
| Status: PAM error | "Couldn't check the password. Try again." |
| Status: fingerprint | "Not recognised. Try again.", "Use your password"; at the greeter, PAM's own message |
| Status: greeter waiting | "Waiting for the fingerprint reader" |
| Status: restart stopped | "Couldn't restart because an app didn't quit. Unlock to see it." ("shut down" likewise) |
| Bottom row | "Sleep", "Restart", "Shut Down" (`copy.md` rule 1's title case) |
| Confirm in place | "Restart now?", "Shut down now?"; "Cancel", "Restart", "Shut Down" |
| Notifications, without previews | "{App}", "{n} notifications" |
| More notifications | "{n} more" |
| Minimal lock | "The lock screen had a problem." |
| Greeter | "Session"; "Couldn't start {session}." |
| Layout menu | The layout's short name; its full name in the menu |
| Settings › Lock Screen | "Dim the screen after", "Lock after", "Turn display off after", each with "Never" |
| Accessible names | "Password", "Fingerprint reader listening", "Keyboard layout: {name}" |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

Notifications on the lock are asked once, in `notifications.md` (For the
user 2); this sheet takes its answer.

1. **The greeter in the user's look.** Shipped look only, as `platform.md`
   has it; or the user's wallpaper, appearance and accent published by the
   session for the greeter to read, never their token file. *Recommended:
   the user's choices*, so the login screen and the lock are one face and
   signing in does not flash a different desktop (`design-references.md`
   §12); a broken hand edit still cannot reach the greeter, since it never
   reads the token file.
