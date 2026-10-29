# Controllers, 2026

Evidence for P18 (`docs/work/rust-desktop/p18-controllers.md`): how pads
reach a user process on this host, what the host's own controller looks
like, and what the tree has today. Gathered 2026-10-29 on the host
(CachyOS, kernel 7.2.10, systemd 262) and from the tree at `086cceb`.

## 1. Access

systemd's `70-uaccess.rules` tags every `ID_INPUT_JOYSTICK` input node
`uaccess`, so the seat's user gets an ACL on it:

```
/usr/lib/udev/rules.d/70-uaccess.rules:61:
SUBSYSTEM=="input", ENV{ID_INPUT_JOYSTICK}=="?*", TAG+="uaccess"
```

Both joystick-tagged nodes on the host carry `user:alexparlett:rw-`. The
compositor and Settings can open them directly; nothing goes through
logind's `TakeDevice`, and Isle ships no rule for them. This closes the
open point in `ui-and-services.md` (gamepad permissions under uaccess).

## 2. What `ID_INPUT_JOYSTICK` tags here

| Node | Path | Id | Keys (word 4, bits 256–319) | Axes | A pad? |
|---|---|---|---|---|---|
| event13, "Keychron Keychron Link" | USB, `0003:3434:D030` | 3434:d030 | `ffff00000000`: `BTN_TRIGGER`…`BTN_DEAD` (0x120–0x12f) | `3f` | No: joystick buttons, no `BTN_GAMEPAD` |
| event26, "Microsoft X-Box 360 pad 0" | `/sys/devices/virtual/input/input37` | 28de:11ff | `7cdb000000000000`: `BTN_SOUTH` (0x130) and the rest | `3003f` | Yes, virtual |

The keyboard's receiver is tagged a joystick by udev. `gaming.md`
Controllers 1 (`BTN_GAMEPAD` and an axis) excludes it; `ID_INPUT_JOYSTICK`
alone would not. A remapper's virtual keyboard, which claims every key,
is excluded by the axis.

## 3. The host's controller

`lsusb`: `28de:1304 Valve Software Steam Controller Puck`. No kernel driver
binds it as a pad (`gaming.md` Controllers 7). With Steam running, Steam
holds `/dev/uinput` (the only holder) and presents event26, Valve's
vendor id with product `11ff`, under `/sys/devices/virtual`. So the only pad
Isle sees on the host is Steam Input's, and it names it "Steam Input".

`/sys/class/power_supply/` is empty and `upower -e` lists no gaming
device: the Puck's controller reports its battery to Steam over hidraw
only. A pad battery on the host needs either the hidraw report (P18
choice 16) or another pad.

## 4. The tree

- `isle-power` maps UPower's type 12 to `Kind::Pad` with the
  `game-controller` glyph and "Controller" as its fallback name (P8).
  Pad batteries therefore already reach Control Centre's Batteries, the
  island's low glyph ("{name} battery low, {n}%") and the Devices widget
  (P17), under UPower's model name.
- `isle-bluetooth` and the pairing agent map BlueZ's `input-gaming` icon
  to `game-controller`.
- Idle (`isle-compositor/src/idle.rs`) has one entry for activity,
  `State::idle_activity`, called from libinput's events. libinput ignores
  joysticks, so pad input reaches nothing today.
- `pointer_names` (the Mouse pane's devices) leaves out virtual devices
  and anything with tap fingers, so a pad's touchpad is already kept off
  the Mouse pane; nothing yet stops it moving the pointer.
- Settings has no Game Controllers pane (P13 left it to P18). Settings
  follows services in its own process (`isle-system-settings/src/services.rs`).
- No crate reads evdev. `udev` 0.9.3 and `input` 0.10.0 are in the lock
  through Smithay; `evdev` and `gilrs` are not.

## 5. Crates

From `ui-and-services.md` §3 (2026-10-02): `gilrs` 0.11.2 (evdev on Linux,
SDL mappings, force feedback, its own event loop) and `evdev` 0.13.2. Isle
needs presence, capabilities, absinfo and raw state; mappings belong to
games and Steam Input (B22). Versions are re-checked when the crate is
added.

## 6. v1

`shell/scripts/gamepad.py` on `main`: raw evdev, `BTN_GAMEPAD` and an axis,
`--live` JSON for Settings › Controllers (buttons, sticks, triggers, hat),
the uinput holder found by scanning `/proc/*/fd` for `/dev/uinput`, the
origin from `/sys/devices/virtual`, the vendor to a face (`054c` Sony,
`045e` Xbox, `057e` Nintendo). `ControllersPage.qml` matched batteries to
pads by name, which fails whenever UPower's model name differs from the
input node's.
