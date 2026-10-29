# P18 · Controllers

Planned 2026-10-29. Set aside by the user the same day, unreviewed.

The roadmap's brief: pad detection and naming, batteries in Control
Centre, Settings › Game Controllers, idle held off by pad play (a hold on
P9's idle policy).

`gaming.md` Controllers 1–7 says what Isle knows of a pad and how it reads
one; P1 named virtual pads (`structure/copy.md`), placed pad batteries
(`structure/control-centre.md`: the Batteries module and page, the
Bluetooth page's row, the island's low glyph, the Devices widget), drew the
Live row kind with the pad's drawing (`surfaces/settings.md`) and made
controller input count as activity (`structure/lock.md`). What is open is
how it is built and `gaming.md`'s three points: device access, `gilrs` or
`evdev`, and duplicate pads while Steam Input mirrors a real one.

Today nothing reads a pad. Pad batteries already reach every place P1 put
them, through UPower (P8), under UPower's model name. libinput ignores
joysticks, so pad input is not activity, and a DualSense's touchpad moves
the pointer. Settings has no Game Controllers pane.
`docs/research/rust-desktop/controllers-2026.md` has the probe: uaccess
already gives the user every joystick node; the host's receiver for its
keyboard is tagged a joystick; and the host's own controller, a Steam
Controller through its Puck, is seen only as Steam Input's virtual pad,
with no battery outside Steam.

What P18 must get right is cost and trust in the idle line. A pad can send
a thousand events a second and a drifting stick never stops, so reading
pads for activity must cost nothing while play is steady and must not keep
a screen awake for a pad left on the sofa. And Isle sets nothing on a pad:
it watches and names.

## What it answers to

- Decisions: B9 (the service worker), B17, B21, B22 (no pad navigation, no
  console mode; the Game Controllers pane stays), B26 (Settings in
  software), B28, B30 (Game Mode), B60 (services), B66–B72 (the lock and
  idle), B94 and B97 (Settings' rows).
- Intent: `gaming.md` Controllers 1–7, its open points and bars 13 and 14;
  `apps.md` Settings 15, 18 and 24 and its D54 lesson; `shell.md` 12;
  `platform.md` (devices through udev, UPower batteries).
- Design, binding: `p1/structure/control-centre.md` (Batteries, pad
  batteries), `p1/surfaces/control-centre.md` (the Batteries rows),
  `p1/structure/lock.md` (controller input as activity, nothing holds the
  locked display-off), `p1/structure/settings.md` and
  `p1/surfaces/settings.md` (the pane, its icon, the Live kind),
  `p1/structure/copy.md` (virtual pads' names), `p1/surfaces/prompts.md`
  (the pairing prompt's glyph), `p1/structure/widgets.md` (Devices).
- Evidence: `controllers-2026.md` (new), `ui-and-services.md` §3,
  `isle-v1-inventory.md` §7 Gamepad, v1's `gamepad.py` and
  `ControllersPage.qml` on `main`.

## Choices this plan makes

### Scope

1. **One packet, nothing cuttable but the Puck's battery** (choice 16). It
   is small: one crate, a worker task, one idle input, a name on existing
   batteries, a libinput rule and one Settings pane.
2. **Isle sets nothing on a pad.** No dead zone, mapping, rumble, light
   bar or player LEDs: those are the game's and Steam Input's (`gaming.md`
   Controllers 6, B22), and D54's dead zone went with pad navigation. The
   pane shows and tests; it writes no setting.
3. **One gate, 18.1, on one board.** P1 gives the pane one table line and
   the Live kind; its rows, the drawing's faces, the empty state and a
   virtual pad's row are drawn and decided before 18.7 builds them.

### Detection

4. **`isle-pads`, a crate on the `evdev` crate, not `gilrs`.** Isle needs a
   node's capabilities, absinfo, ids and raw state. `gilrs` adds SDL
   mappings, its own event loop and force feedback, none of which Isle
   uses. The crate is pure where it can be: classification, grouping,
   naming and faces read a sysfs root passed in, so tests run on fixtures.
   The compositor's worker and Settings both use it, so a pad has one name
   everywhere.
5. **A pad is `BTN_GAMEPAD` and an axis** (`gaming.md` Controllers 1).
   `ID_INPUT_JOYSTICK` is not used: on the host it tags the keyboard's
   receiver, which has joystick buttons and axes but no gamepad button.
6. **Presence by a udev monitor on the service worker** (B9), filtered to
   `input` and `event*` nodes, not by polling `/dev/input`. A pad appears
   or goes on its udev event, within a second (`gaming.md` Controllers 5).
   Access is uaccess's ACL on joystick nodes, checked on the host; Isle
   ships no rule and opens nodes read-only without grabbing them.
7. **One controller per physical device.** A DualSense is three nodes (the
   pad, its touchpad and its motion sensors) under one HID parent; they
   are grouped by that parent and the pad node stands for the controller.

### Naming

8. **A real pad's name**: a Bluetooth pad's BlueZ alias (what the user
   named it, matched by the node's `uniq` address); otherwise the kernel's
   name with the vendor's prefix trimmed from a short list ("Sony
   Interactive Entertainment ", "Microsoft ", "Nintendo Co., Ltd. "), so
   "DualSense Wireless Controller" and "Xbox Wireless Controller"; a
   trailing " 0" or device-number suffix dropped. No table of models.
9. **A virtual pad's name** is P1's: "Steam Input" for Valve's vendor id,
   otherwise "Virtual pad from {program}", the program found by scanning
   `/proc/*/fd` for `/dev/uinput` once when the pad arrives (v1's way);
   none found, "Virtual pad".
10. **Duplicates are listed, not merged.** While Steam mirrors a real pad
    both show, real pads first, each named for what it is; evdev cannot
    say which real pad a virtual one mirrors. One exception: when the only
    controller behind Steam is one Isle cannot read as a pad (a Valve
    receiver on USB, as the host's Puck), the Steam Input row's detail
    names it, "Steam Controller through Steam Input".

### Activity

11. **Pad input is activity, read cheaply on the worker.** Each pad node is
    a calloop source on the worker. On a qualifying event (choice 12) the
    worker records the time, drains the node and stops watching it for a
    second, re-armed by a timer; the main loop gets at most one message a
    second, which calls `State::idle_activity`. Steady play costs one
    wakeup per pad per second; the kernel's buffer overflowing meanwhile
    (`SYN_DROPPED`) is drained and ignored.
12. **Drift is not activity.** A button, a d-pad press, or a stick or
    trigger past a quarter of its travel from rest counts; a small change
    in an axis does not. A pad on the sofa with a drifting stick lets the
    screen sleep.
13. **Not while locked.** Pad input neither counts nor wakes a locked
    display (`structure/lock.md`: nothing holds the locked display-off).
    The worker stops watching pads for activity while the lock is up.
14. **Whatever is in front.** Pad activity does not depend on Game Mode,
    which already holds idle while a game is in front (`gaming.md` Game
    Mode 8); it is what keeps an undetected game, an emulator or a
    streaming client awake.

### Batteries

15. **Batteries stay UPower's** (P8); a pad's battery takes the pad's name.
    UPower's `NativePath` names a `power_supply` whose device is the
    controller's HID parent in sysfs; the pad with that parent lends its
    name (choice 8). A battery with no matching pad keeps UPower's name.
    Control Centre, the island's glyph and the Devices widget follow with
    no change of their own; virtual pads have no battery and are not
    listed there (P1).
16. **The Puck's controller's battery, cuttable.** 18.0 probes whether the
    receiver's hidraw reports can be read beside Steam without disturbing
    it and carry the level. If so, 18.5 adds it as a battery source in
    `isle-pads`, shown as the Steam Input row's and in Batteries under
    "Steam Controller"; if not, the row shows no battery and P18 does not
    wait for it.

### A pad's other nodes

17. **A pad's touchpad and motion nodes are not pointers.** libinput's
    send-events is turned off for a device whose HID parent is a pad's, so
    a DualSense's touchpad does not move the pointer and does not count as
    a trackpad for the Trackpad packet's pane. Games and Steam read those
    nodes and hidraw as before; libinput's setting touches only libinput.

### Settings › Game Controllers

18. **The pane** in group 6 with P1's icon (`game-controller`,
    `pane-grey`), always shown. Each controller is a group: its name; its
    connection (USB, Bluetooth, Virtual) and battery, as Batteries reads
    them; the Live drawing below. None: "No controller connected". The
    exact rows are gate 18.1's.
19. **The drawing is P1's Live kind**: the outline 160 × 100 in
    `colour-hairline-strong`, a pressed control filled `colour-accent`,
    sticks as dots that move, triggers filling as they travel, the face
    buttons labelled by vendor (Xbox letters, PlayStation's four shapes
    drawn as paths, Nintendo's letters in their places), generic pads as
    Xbox. Shapes are Isle-drawn; Phosphor has none.
20. **Settings reads pads itself**, through `isle-pads`: presence always,
    the live state only while the pane is shown (`apps.md` Settings 15),
    redrawn at most once a frame. Nothing reaches the compositor and
    nothing is written. The pane's rows join the generated index, so
    Spotlight finds "Game Controllers".

### Proof

21. **A uinput pad for tests.** `isle-fakes` gains a pad made through
    `/dev/uinput` (buttons, sticks, triggers, a hat, a chosen vendor and
    product), and the fake UPower a device whose `NativePath` points into a
    sysfs fixture. Headless tests drive classification, naming, grouping
    and batteries from fixtures; the VM drives a uinput pad end to end
    (`gaming.md` bar 14).
22. **`isle pads`** on the CLI lists what the worker sees (name, origin,
    connection, node, battery) as text or JSON, and `Test1` exposes the
    last pad activity, so the idle tests need no screen.
23. **Bar 13 on the host by a game that is not a game.** A test window
    marked never a game (`isle windows game`) and thirty minutes of
    pad-only input through Steam Input: no dim, no lock. With the pad put
    down, the screen dims on time.

## The workspace

```
crates/
  isle-pads/              classification, grouping, naming, faces, live
                          state, the udev watch (new)
  isle-compositor/src/
    pads.rs               the worker's watch, activity, Pads published (new)
    idle.rs               pad activity in, not while locked
    backend/udev.rs       send-events off for a pad's other nodes
    services/power.rs     pad names on batteries
    dbus.rs               Shell1 Pads, Test1
  isle-power/             NativePath kept on Battery
  isle-fakes/             the uinput pad, UPower's NativePath
  isle-system-settings/   Game Controllers
  isle-cli/               isle pads
data/
  settings/index.kdl      the pane's rows
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 18.0 · The note and the record

`controllers-2026.md` (written with this plan); the roadmap's P18 entry;
`gaming.md`'s open points answered by choices 4, 6 and 10. The Puck probe
(choice 16) on the host with Steam running, written into the note.

### 18.1 · Gate: the pane

One board drawn from the real tokens: the pane with one real pad, a real
and a Steam Input pad together, the host's Steam Input with the Puck named,
a low battery, the empty state; the drawing pressed and at rest in its
three faces. The user answers lgtm or names changes; the answers go into
`p1/structure/settings.md` and `surfaces/settings.md`.

### 18.2 · `isle-pads`

Classification, grouping by HID parent, origin, the uinput holder, names
(choices 5, 7–10), faces by vendor; all on a sysfs root, tested on
fixtures copied from the host's two nodes and a DualSense's three.

### 18.3 · Presence

The udev watch on the worker; `Pads` published to the main loop and on
`Shell1`; `isle pads`; the uinput pad in `isle-fakes`. A pad made or
removed shows or goes within a second.

### 18.4 · Activity

Choices 11–14: the sampled watch, the drift threshold, the lock's stop,
`Test1`'s last activity. Tests: a press holds idle, a drift of a tenth of
travel does not, nothing counts while locked, a steady stream costs one
message a second.

### 18.5 · Batteries

`NativePath` kept by `isle-power`; pad names on batteries (choice 15);
the fake UPower's pad. The Puck's battery if 18.0 allowed it (choice 16).

### 18.6 · A pad's other nodes

Send-events off for a pad's touchpad and motion nodes (choice 17), tested
in the VM with a uinput touchpad under a uinput pad's parent where uinput
allows it, else on the host with whatever pad is at hand.

### 18.7 · Settings › Game Controllers

The pane as gate 18.1 passed it, the drawing and its faces, live only
while shown; the index rows.

### 18.8 · The VM and the walk

The uinput pad in the VM: detection, naming, a battery from the fake
UPower, the pane's drawing following every control, idle held by presses
and not by drift. Walked by real input as `verify-by-interaction` asks:
open the pane, press, unplug, replug, lock.

### 18.9 · The host and close

The host run on the installed package: the Puck through Steam Input named
and drawn, bar 13 (choice 23), Settings' idle cost with the pane closed.
Decisions recorded from B132; ARCHITECTURE gains How pads are read;
`gaming.md`'s open points closed; the roadmap and `state.md` follow; this
file ends with its outcome.

## Not in P18

Mapping, dead zones, rumble, lights and per-app pad profiles (choice 2).
Pad navigation of Isle's own surfaces and a console mode (B22). Pairing,
which is Bluetooth's and the pairing agent's (P9). Gyro and touchpad
drawings in the pane. A low-battery notification for peripherals (P1 gives
it to `notifications.md` without a form; P20's audit). Steam Input's own
configuration.

## Depends on, and feeds

- Needs P8 (UPower batteries, Bluetooth), P9 (idle and the lock), P12
  (Game Mode's idle hold, `isle windows game`), P13 (Settings' panes and
  index), P14a (the package), P17 (the Devices widget).
- Feeds the Trackpad packet (a pad's touchpad is not a trackpad) and P20's
  audit.

## Done when

- `cargo test` passes headless with the classification, naming, grouping,
  battery and idle tests.
- A uinput pad in the VM is detected, named and drawn live, appears and
  goes within a second, and holds idle by presses but not by drift.
- With no pad and the pane closed, nothing reads a pad; with pads in
  steady play, the worker wakes once a second per pad.
- On the host, Steam Input's pad is named and drawn, and thirty minutes of
  pad-only play in a window that is not a game neither dims nor locks.

## Risks

- **The Puck's battery.** The new Steam Controller's reports are not
  documented, and reading hidraw beside Steam may be refused by Steam's
  rules or disturb it. Choice 16 is cuttable for this.
- **Steam Input's virtual pads come and go** with Steam's layouts and
  games, sometimes several at once. Each arrival scans `/proc`; if that
  shows in the worker's time, the scan is cached by Steam's pid.
- **Send-events off on a pad's touchpad** removes something a few people
  use as a mouse. If the host shows a need, it becomes a Mouse row; it is
  not one now.
- **Drift's threshold.** A quarter of travel is v1's half halved; the VM
  and the host tune it, and the number is recorded in the decision.
