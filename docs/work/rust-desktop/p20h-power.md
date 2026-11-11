# P20h · Power

Planned 2026-11-11 and approved the same day; built at once at the user's
word, ahead of 20b.9. Energy's board (`p20h/energy-1.html`) was accepted
with its recommendations.

A Claude session marked to keep the computer awake still let the screen
lock and turn off. The cause is a missing interface: Chromium sends an
app's "keep the computer running" hold to `org.gnome.SessionManager` or
`org.freedesktop.PowerManagement`, Isle owns neither, and Chromium gives up
without a word (`docs/research/rust-desktop/power-2026.md` §1). Behind that
is a larger gap. Isle never puts the computer to sleep by itself, so it has
no notion of a hold that keeps the computer running but lets the screen go
dark; the user has no way to keep the screen on by hand; and Energy has one
row where macOS has four.

P20h serves the missing interface, and splits holds in two: a display hold
keeps the screen on, and a sleep hold keeps only the computer running. It
adds sleep as idle's last step, behind macOS's "Prevent automatic sleeping
when the display is off". It gives the user a Keep Awake switch and
`isle awake`, and fills in Energy's rows: wake for network access, the
power button, and what is keeping the computer awake now.

What it must get right: no app's hold is lost because Isle served the wrong
name; music playing never keeps the screen lit; the computer never sleeps
while something holds it, or with the switch on; and the lock is on every
screen before an idle sleep, as before any other.

## What it answers to

- Decisions: B11 (service crates), B17 (macOS's model), B27 (settings
  through the store), B46 (a game holds idle), B69 (idle is one policy
  with holds), B72 (the session's end and others' inhibitors).
- Intent: `compositor.md` Idle, DPMS and sleep 1–5 and its open point;
  `platform.md` the session and power services, Lock, PAM and idle 3 and
  8; `shell.md` Island 10, Control Centre 2, Lock 4; `apps.md` Settings 18.
- P1: `p1/structure/settings.md` Energy (row 1 of the pane list: "Power
  profile, sleep and wake, batteries"); `p1/structure/control-centre.md`
  the module table and Display's page; `p1/structure/island.md` the
  glyphs.
- Earlier packets: P9 choices 10, 12–14 (the lock before sleep, idle's
  clock, its holds, ScreenSaver); P11 choice 28 (the portal's Suspend
  counted as idle, "Isle suspends nothing by itself"), which this packet
  revises; P13 the Energy pane.
- v1: Settings › Power (profiles, batteries, idle dim, lock, screens off),
  `isle-v1-inventory.md` §4 and §7. v1 never slept on idle either.
- Evidence: `docs/research/rust-desktop/power-2026.md` (2026-11-11).

## Choices this plan makes

### Scope

1. **P20 gains an eighth packet, P20h, power,** built after P20b's host
   run (the user's word, 2026-11-11). The roadmap's P20 entry lists it.
2. **P20h holds:** `org.freedesktop.PowerManagement` served; display and
   sleep holds; sleep as idle's fourth step; Keep Awake (Control Centre,
   the island, `isle awake`); Energy's rows for automatic sleep, wake for
   network access and the power button; and a list of what keeps the
   computer awake.
3. **Left out:** hibernation; scheduled wake and sleep (macOS dropped the
   schedule from Settings); the lid and a laptop battery's actions, which
   stay logind's; "Start up automatically after a power failure", which is
   the firmware's; serving `org.gnome.SessionManager`, which apps read as
   "this is GNOME".

### Holds

4. **Two kinds of hold.** A *display hold* holds the whole clock, as every
   hold does today: idle-inhibit on a shown surface, ScreenSaver cookies,
   the portal's Idle flag, logind's `idle` block inhibitors, a game in
   front, and the user's Keep Awake. A *sleep hold* holds only the sleep
   step (choice 8): PowerManagement cookies, the portal's Suspend flag,
   other programs' logind `sleep` block inhibitors, a sound playing (a
   stream in `isle-audio`'s `playing`), and a screen being cast. P11 choice
   28 is revised: the portal's Suspend becomes a sleep hold.
   So by default an Electron app's keep-awake, Claude's among them, keeps
   the computer running and lets the screen dim, lock and turn off, as on
   a Mac. When the screen has to stay on and unlocked, the user says so:
   for one app (choice 16), or for everything while Keep Awake is on
   (choice 11). Rejected: every PowerManagement cookie holding the whole
   clock, since Chromium is believed to take this hold while audio plays
   (checked in 20h.2, `power-2026.md` §1), so music in a browser tab would
   keep the screen lit all night. Reviewed: the user asked for options
   rather than one rule (2026-11-11).
5. **PowerManagement is served beside ScreenSaver.** The worker owns
   `org.freedesktop.PowerManagement` and serves
   `org.freedesktop.PowerManagement.Inhibit` at
   `/org/freedesktop/PowerManagement/Inhibit`: `Inhibit` and `UnInhibit`
   on the same cookie map, each cookie marked display or sleep, dropped
   when its caller leaves the bus (P9 choice 14). `HasInhibit` is true
   while any hold keeps the computer from sleeping; `HasInhibitChanged`
   says so when that changes. If another program owns the name, Isle
   logs it and serves nothing, as for ScreenSaver.
6. **Holds name their kind everywhere they are listed.** `client_holds`
   writes `what` as logind does: `idle` for a display hold, `sleep` for a
   sleep hold. `isle idle` prints both kinds with their apps, Monitor's
   Preventing Sleep reads the same list (P19a), and Display's page keeps
   "Kept awake by {app}" for display holds only.

### Sleep

7. **"Prevent automatic sleeping when the display is off"** on Energy, as
   on a macOS desktop, `energy.prevent-sleep`, **on by default**. On, Isle
   never sleeps by itself, as today. Off, sleep is idle's fourth step.
   Rejected: a minutes field of its own (macOS's desktop pane has none,
   and display off on Lock Screen is already the user's time).
8. **The sleep step** comes once the screens have been off for one minute
   with no hold of either kind, locked or not, so a glance back within the
   minute needs no wake. The minute is a token. Display off set to Never
   means the computer never sleeps by idle. As on a Mac, a session locked
   by hand turns its screens off on the lock's own timing and then
   sleeps. A sleep hold that ends while the screens are off starts the
   minute again; a display hold that ends counts as input (P9 choice 13).
9. **Sleeping is the session dialog's Sleep**: logind's `Suspend(false)`,
   so the lock is on every screen first (P9 choice 10). If logind refuses
   (a block inhibitor taken in between, or polkit), Isle logs it and does
   not try again before the next input.

### Keep Awake

10. **Keep Awake is a Control Centre module,** a switch with a page,
    listed in the gallery and not in the default grid. While it is on it
    is a hold named "Keep Awake". The page offers how long: Until turned
    off, 1 hour, 2 hours or 5 hours. It ends then, at log out, or when the
    compositor restarts; it is never saved.
11. **Keep Awake's page has "Keep the display on",** on by default
    (`energy.keep-awake-display`, remembered for next time). On, Keep
    Awake is a display hold: no dim, lock, display off or sleep, so the
    session stays unlocked. Off, it is a sleep hold, and the screen dims,
    locks and turns off while the computer keeps running. Locking by hand
    still locks either way. The island glyph, choice 12, is drawn
    differently for each.
12. **An island glyph while Keep Awake is on,** joining `shell.md` Island
    10's list, Show when active by default. A click opens the module's
    page. Apps' holds show no glyph, as P9 choice 13 has it.
13. **`isle awake [--display] [-- command…]`**, as macOS's `caffeinate`: a
    sleep hold, or with `--display` a display hold, named by the command
    or "isle awake". It ends when the command exits, on Ctrl+C, or when
    the CLI leaves the bus. It goes through the compositor's session
    interface with the same cookies, so it is listed like any other hold.
14. **The board (20h.1)** draws the module in its sizes and states, its
    page, the island glyph in both forms, and Energy's pane (choices
    15–19) from the tokens, published for the user's choice before any of
    it is built.

### Energy

15. **Energy's rows, in macOS's order:** Energy Mode (built in P13),
    "Prevent automatic sleeping when the display is off", "Wake for
    network access", then "When the power button is pressed".
16. **"Apps that keep the display on"**, a List row with + and −
    (`energy.display-apps`, app ids, empty by default). A listed app's
    sleep holds count as display holds, so while it holds, the screen
    stays on and unlocked. + opens the app picker Focus's allowed-apps list
    uses. A hold is matched to its app by its caller's pid through the
    process registry, or by the name the app gave when there is no pid
    (the portal's).
17. **"Keeping this computer awake"**, a group under the rows: one line
    per hold with its app's icon and name, "Keeps the display on" or
    "Keeps the computer running", and the app's reason when it gave one.
    "Nothing" when there are none. Live while the pane is shown. A
    "Keeps the computer running" line's ⋯ menu has "Keep the Display On
    for {app}", which adds the app to choice 16's list; a listed app's
    line has "Let the Display Turn Off for {app}".
18. **Wake for network access** is `energy.wake-for-network`, off by
    default. The network service applies it to every wired and Wi-Fi
    connection profile: `802-3-ethernet.wake-on-lan` `magic` and
    `802-11-wireless.wake-on-wlan` `magic` on, the default (`0x1`) off.
    It applies again when a profile is added. A system profile asks
    through Isle's polkit agent if NetworkManager wants it. The row is
    disabled with "This computer's network devices can't wake it." when no
    device reports wake support.
19. **"When the power button is pressed"**: Show the session dialog (the
    default, as today), Sleep, Lock Screen. `energy.power-button`. While
    locked, the key still wakes the lock's face (P9). Holding the button
    down stays the firmware's.

### Proof

20. **Headless tests** in `isle-compositor`: PowerManagement served at its
    path, `HasInhibit` and its signal, cookies dropped when the caller
    leaves; each source sorted into its kind; a listed app's sleep hold
    counted as a display hold, by pid and by name; the sleep step's minute, the
    switch and Never, the minute restarting after a sleep hold, a display
    hold's end counted as input; `Suspend` called on the fake logind,
    which gains it, and a refusal not retried; Keep Awake's durations, its
    end and "Keep the display on" both ways; `isle awake` with and without `--display`; the power button's
    three choices. `isle-network`: wake for network against the fake
    NetworkManager, which gains the two properties.
21. **Bars.** A hold is in force before `Inhibit` replies. The sleep step
    fires within a second of when it is due. No idle sleep while any hold
    of either kind is listed.
22. **The VM walk** with real input: the Electron script from the
    evidence holds sleep and the screen still locks; Firefox playing
    audio holds sleep and not the screen; a video holds the screen; Keep
    Awake from Control Centre for an hour with the display on, then with it
    off and the screen locking; the Electron script's app added from the
    holds group and the screen staying unlocked; `isle awake -- sleep
    120`; the switch off and the guest sleeping a minute after display
    off, then waking to the lock; the power button through QMP's
    `system_powerdown` in each choice; Energy's holds group live.

## The workspace

- `isle-compositor`: `screensaver.rs` gains PowerManagement and each
  cookie's kind; `idle.rs` gains the sleep step, the two kinds and
  `idle_holders` split; `keep_awake.rs` (the user's hold and its timer);
  `control_centre.rs` and `control_edit.rs` the module; `island.rs` the
  glyph; `keyboard/mod.rs` and `lock.rs` the power button's choice;
  `dbus.rs` the hold call for `isle awake`.
- `isle-settings`: an `Energy` section: `prevent-sleep`,
  `keep-awake-display`, `display-apps`, `wake-for-network`,
  `power-button`.
- `isle-shell`: the module, its page and the glyph as the board draws
  them.
- `isle-system-settings`: Energy's rows and holds group.
- `isle-network`: the wake properties on wired and Wi-Fi profiles.
- `isle-ipc`: `Hold.what` documented for both kinds; the hold call.
- `isle-cli`: `isle awake`; `isle idle` prints both kinds.
- `isle-fakes`: logind's `Suspend`; NetworkManager's wake properties.
- `isle-tokens`: the sleep step's minute.

## Phases

### 20h.0 · The note and the record

`compositor.md` Idle, DPMS and sleep for the two kinds and the fourth step,
its open point answered; `platform.md` the PowerManagement name; `shell.md`
Island 10 and Control Centre 2; `apps.md` Energy; P11 choice 28 marked
revised; `lock-and-prompts-2026.md` §6.2's Chromium row corrected;
`state.md`, `progress.md`, README.

### 20h.1 · The board

Keep Awake's module and page, the island glyph, Energy's rows and holds
group, drawn from the tokens and published for the user's choice.

### 20h.2 · PowerManagement and the two kinds

The name served, cookies marked by kind, the holds split, `client_holds`
and `isle idle`. In the VM: whether Chromium takes the app-suspension hold
for audio (choice 4 stands or comes back to the user). Tests.

### 20h.3 · The sleep step

The switch, the step and its minute, the sleep holds read, `Suspend`
through logind, a refusal. Tests.

### 20h.4 · Keep Awake and `isle awake`

The user's hold, its durations and "Keep the display on", the module
and page, the glyph in both forms, the CLI. Tests.

### 20h.5 · Energy's rows

The prevent-sleep row, Apps that keep the display on, the holds group
live with its menu, the power button row. Tests.

### 20h.6 · The power button

The three choices in the compositor, the lock's face kept. Tests.

### 20h.7 · Wake for network access

The network service's profiles, the polkit path, the disabled row. Tests.

### 20h.8 · The bars and the walk

Choice 21's bars; choice 22's walk in the VM, the journal read after each
step.

### 20h.9 · The host and close

Package and install. With the user: a Claude session's keep-awake with the
switch off, the screen locking and the computer staying up, then with
Claude listed and the screen staying on and unlocked; Keep Awake
holding the screen; an idle sleep and its wake to the lock; a magic packet
from the phone over Wi-Fi and, if cabled, Ethernet; the power button. Then
the decisions from the next free number, ARCHITECTURE's How idle decides
updated, this sheet's outcome.

## Not in P20h

Hibernation and suspend-then-hibernate. Scheduled wake. The lid and laptop
battery actions. Power-failure start. `org.gnome.SessionManager`. A sleep
timer per power source.

## Depends on, and feeds

- Needs P8 (`isle-power`, `isle-network`, `isle-audio`, Control Centre's
  grid and gallery), P9 (idle's clock and holds, the lock before sleep,
  ScreenSaver, the power key), P11 (the portal's Inhibit), P13 (Settings ›
  Energy), P19a (Monitor's sleep holds).
- Feeds P20g (the audit's Power line).

## Done when

- `cargo test` passes headless for choice 20.
- Choice 21's bars hold.
- Choice 22's walk passes in the VM.
- The host run passes with the user.

## Risks

- Another program may already own `org.freedesktop.PowerManagement` on a
  machine with PowerDevil or xfce4-power-manager installed beside Isle;
  Isle then serves nothing and says so in the journal.
- An idle sleep that does not come back is worse than one the user asked
  for, and the host's NVIDIA suspend path is its own (P9's evidence §6.4).
  The switch stays on by default until the host run shows waking is
  sound.
- Wake for network may need firmware settings Isle cannot see; the row
  says only what the devices report.
- An app holding sleep forever (a stuck call) keeps the computer up; the
  holds group names it, which is the remedy.
