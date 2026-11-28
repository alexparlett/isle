# Keychron dongles, 2026

How the host's two Keychron 2.4 GHz receivers show up, and what they say about battery. Readings
taken on the host on 2026-11-28 over hidraw (the `70-isle-hidraw.rules` uaccess rule already gives
the user every hidraw node). Evidence for a packet, not its choices.

## 1. The receivers

| Receiver | USB id | Paired device (from the receiver) | libinput sees |
|---|---|---|---|
| Keychron Link | `3434:d030` | `3434:0861`, the Q6 Max | "Keychron  Keychron Link " (pointer), "Keychron  Keychron Link  Keyboard" (keyboard + pointer); interface 1 also has a node udev tags `ID_INPUT_JOYSTICK` (libinput ignores it) |
| Keychron Ultra-Link 8K | `3434:d028` | `3434:d056`, the M7 8K | "Keychron Keychron Ultra-Link 8K" (pointer), "Keychron Keychron Ultra-Link 8K Keyboard" (keyboard + pointer: the mouse's button-to-key interface) |

Both receivers' nodes share a `LIBINPUT_DEVICE_GROUP` per receiver (`3/3434/d030:…`, `3/3434/d028:…`),
so the group is the physical device. Names are the receiver's, with Keychron's doubled spaces; no
node carries the paired product's name.

What Isle showed that day: `Keyboard1.List` had both "…Keyboard" nodes as keyboards (the mouse among
them) and the Steam Controller Puck as four keyboards; `Pointers` had all four Keychron nodes.

## 2. Vendor interfaces

| hidraw usage page | Link (`d030`) | Ultra-Link 8K (`d028`) |
|---|---|---|
| `0xFF60` / `0x61`, 32-byte unnumbered (QMK raw HID) | yes (if 3) | yes (if 2) |
| `0x8C`, reports `0xB1` in / `0xB2` out | yes (if 1), with feature reports `0x51`–`0x53` and input `0x54` | yes (if 3) |
| `0xFFC1`, reports `0xB3` out / `0xB4` in (63 bytes), `0xB5` out / `0xB6` in (20 bytes) | no | yes (if 4) |

## 3. Paired devices

Both receivers answer `b2` on the raw HID interface (write `00 b2 00…`, 32 bytes):
`b2 <n> [<vid lo> <vid hi> <pid lo> <pid hi> <connected>]×n`. Read: Link `b2 01 34 34 61 08 01`,
Ultra-Link `b2 01 34 34 56 d0 01`. The Ultra-Link also answers `b5 03` on `0xFFC1` with the same slot
list behind `b6 03`.

## 4. Battery

**Mouse, through the Ultra-Link 8K: available.** Write report `b3 06` (64 bytes) to the `0xFFC1`
node; the reply is report `b4 06 …`, and byte 20 is the battery: bits 0–6 the percent, bit 7
charging. Read `0x89` and later `0x91`. The receiver also pushes `b6 e2 01 <link> <?> <pct> 02 02 05`
by itself when the mouse connects or wakes (`0x11` = 17% after a power cycle, matching the `0x91`
query). Same protocol as the out-of-tree `keychron-battery-dkms` module (`csutcliff/keychron-battery-dkms`),
which also lists this receiver.

**Mouse settings, in the same reply.** Decoded by cycling the M7 8K's DPI and polling buttons while
polling `b3 06` and reading the pushes:

| Where | Meaning |
|---|---|
| status `[3]`, `[4]` (equal) | high nibble the report-rate index, low nibble the DPI stage |
| status `[5]` | DPI stage, 0–4 |
| status `[6..15]` | DPI per stage, u16 LE: 400, 800, 1600, 3200, 5000 |
| status `[17]` | stage count (5) |
| status `[20]` | battery: bits 0–6 percent, bit 7 charging (set with the mouse on its cable, confirmed) |
| push `b6 e2 01 <link> <?> [5]` | battery percent, no charging bit |
| push `[6]`, `[7]`, `[8]` | DPI stage, report-rate index, stage count |

The button cycles the rate index through 0–5 and every press pushes a `b6 e2`, so changes arrive
without polling. Peak reports a second on the mouse's hidraw node while moving it fast, per index:
0 → 126, 1 → 501, 2 → 1003, 3 → 1999, 4 → 2809, 5 → 3573. So 0–3 are 125, 500, 1000 and 2000 Hz;
4 and 5 rise past 2000 but a hand and a Python reader cannot fill 4000 and 8000 a second, and they are
taken as those. Keychron's manual lists 125/500/1000 Hz for the M7; the 8K receiver adds the rest.

**Keyboard, through the Link: not available with stock firmware.** On power cycling the Q6 Max (twice) the Link
pushed `54 e2 01 0{1,2} 00 00 00 00 …` on the `0x8C` node: the shape of the battery event a Link sends
for an M-series mouse (`54 e2 01 <link> 00 <pct> 02 02`, per `whiskerd`), with the battery and the two
`02` bytes zero. Keychron's QMK has no host-readable battery command (Keychron/qmk_firmware#471);
community firmware forks for the Q1 HE and K10 HE add one (`0xA4`, pushed over 2.4 GHz) but none
exists for the Q6 Max. Unknown dongle commands were not probed.

Raw HID queries sent to the keyboard through the Link get no reply: `a0`, `a1` and VIA's `01` on the
`0xFF60` node, and the same wrapped in report `b2` on the `0x8C` node. §5 explains why.

## 5. A battery firmware for the Q6 Max

The Q6 Max ISO knob is `keyboards/keychron/q6_max/iso_encoder` (`pid 0x0861`, `device_version
1.1.1`) on Keychron's `2025q3` branch: STM32F401, ROM `stm32-dfu` bootloader, Keychron's LKBT51
radio module (`LK_WIRELESS_ENABLE`), and the `keychron` keymap with `VIA_ENABLE`. All of its wireless
and raw HID code is in `keyboards/keychron/common`, shared with the boards the community forks target.

### What stock firmware does

- **Battery is measured.** `battery.c` keeps the cell voltage and `battery_get_percentage()`; the
  firmware sends the level to the radio module (`LKBT51_CMD_UPDATE_BAT_LVL`) on every connect. The
  Link does not pass it on (§4).
- **Keyboard to host over 2.4 GHz works.** `kc_raw_hid_send()` with a wireless source goes out as
  `LKBT51_CMD_RAW_HID_TX` whenever the link is connected; nothing gates it.
- **Host to keyboard over 2.4 GHz is switched off.** The module delivers it (`LKBT51_CMD_RAW_HID_RX`
  → `EVT_RAW_HID`), but `wireless.c` handles that event only under `WILRESS_RAW_ENABLE` (Keychron's
  spelling), which no board defines. Its two branches also do not build: `via_raw_hid_receive()` does
  not exist and `kc_raw_hid_rx()` is undeclared there. Hence the silence in §4.

### The community forks

| Fork | Board | Command | 2.4 GHz model | Change |
|---|---|---|---|---|
| OutersGIT, Keychron/qmk_firmware PR #477 (open, base `2025q3`) | K10 HE (and a Q1 HE twin) | `0xA4`: `[1]` percent, `[2..3]` mV LE, `[4]` charging, `[5]` transport, `[6]` model id | Keyboard pushes the report every 2 s; host only reads | ~100 lines: `keychron_raw_hid.{c,h}`, `battery.{c,h}` (`battery_get_charging_state()`, voltage polled in cable mode too), a timer in the board's `keychron_task_kb()` |
| Freedye, `keychron-k3-max-qmk` v0.0.1 | K3 Max (STM32F401 + LKBT51, like the Q6 Max) | `0xA7 0xF0` (misc group): `[2]` status, `[3]` percent, `[4..5]` mV LE, `[6]` 0 none / 1 charging / 2 full, `[7]` transport 1 USB / 2 BT / 4 2.4G, `[8]` wireless state, `[9]` layout version 1 | Request and reply: defines `WILRESS_RAW_ENABLE` and routes `EVT_RAW_HID` to `raw_hid_receive()`, as USB does | Also rebases Keychron's code on QMK master of 2026-11-15 and adds a reconnect key buffer; the battery part is small, the rest is not |

PR #477 says the dongle "does not appear to bridge" host-to-keyboard raw HID; that is the
`WILRESS_RAW_ENABLE` gate above, which the K3 Max fork removes and reports working over 2.4 GHz.
Neither claim was checked on this host.

### What a Q6 Max port takes

On stock `2025q3`, not the K3 Max fork's rebase, so the diff stays small and Keychron's own updates
apply over it:

1. `q6_max/config.h`: define `WILRESS_RAW_ENABLE`. `wireless.c`: the `EVT_RAW_HID` case calls
   `raw_hid_receive(RAW_HID_SRC_WIRELESS, …)`. VIA's replies already go back by source through the
   `via_raw_hid_send()` override, so VIA and Keychron Launcher would also work over the dongle.
2. `keychron_raw_hid.{c,h}`: the battery query, in the K3 Max fork's `0xA7 0xF0` layout (it carries
   the wireless state and a layout version; `0xA4` is a top-level id Keychron may later take).
   `battery.{c,h}`: PR #477's `battery_get_charging_state()` and cable-mode voltage polling.
3. `q6_max.c` `keychron_task_kb()`: push the same reply unasked when the link connects and when the
   percent or charging state changes, so a host still hears it if the Link turns out not to forward
   queries. Not on a 2 s timer: that keeps the radio busy. `kc_raw_hid_send()` already sends only while
   connected, so a push never wakes the radio.
4. Build `make keychron/q6_max/iso_encoder:keychron`. `qmk` 1.2.0 and `arm-none-eabi-gcc` are in
   the repos, but GCC 16 is newer than Keychron's ChibiOS fork has seen; QMK's `qmkfm/qmk_cli`
   container is the fallback.

### The Mac/Windows switch

The switch is DIP switch 0: `dip_switch_update_kb()` sets default layer 2 when it is on Windows and 0
on Mac (`MAC_BASE`, `WIN_BASE` in the `keychron` keymap). Stock firmware already answers
`KC_GET_DEFAULT_LAYER` (`0xA3`): `[1]` the default layer, `[2]` the highest active layer. Over the
cable that works today; over the dongle it needs step 1. Pushing the `0xA3` reply from
`dip_switch_update_kb()` as well would tell a host the moment the switch moves.

### Result on this host (2026-11-28)

Built from `2025q3` in QMK's `qmk_cli` container (branch `q6max-battery` in `~/Development/keychron-qmk`,
`q6max/README.md` there), and flashed. Through the Link the keyboard answers `a1`, `a3` and `a7 f0`
(`battery 83% 3974 mV charging over 2.4ghz link connected`), and pushes `a3` the moment the
Mac/Windows switch moves. So the Link does forward host queries; PR #477's push-only model was the
`WILRESS_RAW_ENABLE` gate. The settings store is flash sector 1 (`0x08004000`, QMK's
`WEAR_LEVELING_EFL_FIRST_SECTOR 1` for the F401): the `.bin` fills that gap with zeros, so the image
was written as two pieces around it to keep the settings.

### Flashing and getting back

Mode switch to Cable, hold Esc while plugging in, and the keyboard enumerates as `0483:df11` (STM32
BOOTLOADER, already `uaccess` in `70-isle-hidraw.rules`); then `dfu-util -a 0 -d 0483:df11 -s
0x08000000:leave -D <file>.bin`. The bootloader is in ROM, so a bad flash is fixed by flashing again,
and Keychron Launcher's firmware update puts stock back. The radio module's firmware and the
dongle's are separate and untouched. Save the VIA layout from Launcher first: same-base firmware
should keep EEPROM, but a changed EECONFIG layout resets it.

### Without new firmware

In Bluetooth mode the module gets the level too (`UPDATE_BAT_LVL`); if it serves the standard BLE
Battery Service, BlueZ and UPower would show it with no change to Isle. Not checked.

## Sources

- `Zamiul-rashid/whiskerd`, `src/drivers/keychron.rs`: the Link's `0x54` event and `b2` slot query.
- `csutcliff/keychron-battery-dkms`, `keychron_battery.c`: the Ultra-Link's `b3 06` / `b4` query and
  byte 20.
- Keychron/qmk_firmware issue #471: no battery over raw HID in stock firmware.
- Keychron/qmk_firmware `2025q3`: `keyboards/keychron/common/{keychron_raw_hid.c,wireless/*.c}`,
  `keyboards/keychron/q6_max/`.
- Keychron/qmk_firmware PR #477 (OutersGIT): `0xA4` push for the K10 HE.
- `Freedye/keychron-k3-max-qmk`, release notes v0.0.1, `common/keychron_raw_hid.c`,
  `common/wireless/wireless.c`: `0xA7 0xF0` and the `WILRESS_RAW_ENABLE` fix.
