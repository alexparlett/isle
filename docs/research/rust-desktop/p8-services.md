# P8: services' measures and what the VM found

## Measures

`cargo test --release -p isle-compositor --features test-hooks --test bench
services_cost -- --ignored --nocapture`, headless on the 5070 Ti: a 3440 by
1440 output, every service live (fake NetworkManager with a VPN, BlueZ with
a connected headset, UPower with a mouse, power-profiles-daemon, logind
with a backlight, an MPRIS player, a private PipeWire), a tray item, nine
modules in the default arrangement. Draws are timed with `glFinish` after
each. Three runs:

| | Run 1 | Run 2 | Run 3 |
|---|---|---|---|
| Control Centre, every page three times | 1262 draws, p50 0.69 ms, p95 1.37 ms, max 3.50 ms | 1262, p50 0.77, p95 1.35, max 3.75 | 1260, p50 0.80, p95 1.35, max 3.85 |
| Banners, ten arrivals | 140 draws, p50 0.56 ms, p95 0.94 ms, max 1.23 ms | 140, p50 0.55, p95 0.92, max 2.14 | 154, p50 0.58, p95 0.95, max 1.19 |
| Notify to the stack | p50 2.6 ms, max 2.7 ms | | |
| RSS at rest, panel open, after | 156, 159, 164 MB | 156, 160, 162 MB | 156, 160, 163 MB |
| Own memory (anonymous) | 30 to 37 MB | 32 to 35 MB | 30 to 37 MB |
| VRAM at rest, panel open, after | 96, 73, 79 MiB | 96, 73, 79 MiB | 96, 73, 53 MiB |

- **Both surfaces draw well under 2 ms at p95**, Control Centre on every
  page and banners as they arrive, slide and stack.
- **A banner reaches the stack 2.6 ms after the app's Notify**, through the
  server on the worker, the decision on the loop and the model. The first
  Notify a test client sends took 200 ms, all of it the client opening its
  own bus connection; the bench sends one first and does not count it.
- **The budgets hold with every service live**: RSS 156 to 165 MB against
  250, VRAM no higher than 96 MiB against 200.
- **At rest nothing is sampled or drawn**: in ten seconds after the panel
  and banners closed, the island drew at most once (the clock's minute), and Isle's
  only child process was Xwayland (platform acceptance bar 10). The meter,
  the Wi-Fi scan, Bluetooth discovery and the media clock run only while
  their page shows.

## The VM

In the guest (llvmpipe, 2560 by 1080, real NetworkManager on a wired link,
PipeWire with pipewire-pulse and WirePlumber, UPower, UDisks2 and
power-profiles-daemon as installed but not enabled, BlueZ's unit not
enabled and no adapter), with nm-applet, Chromium and Firefox:

- **Found and fixed: daemons started on demand never started.** UPower,
  UDisks2 and power-profiles-daemon are D-Bus activated, and Isle only
  waited for their names. `isle-bus`'s `NameWatch` now asks the bus to
  start a name it can activate before it waits. Power, Energy Mode and
  disks came up; BlueZ, whose unit is disabled, cannot be activated and
  Bluetooth stays absent, as it should with no adapter.
- **Found and fixed: audio could wait forever.** pipewire-pulse is socket
  activated; a connection made while it was starting got no answer to the
  handshake, and Isle waited on it. The handshake now has five seconds,
  then Isle connects again.
- **Found and fixed: the pointer did not hold a banner, and its buttons
  lost the click.** The stack's own pointer-enter never fired (the card
  under the pointer takes it), so a hovered banner went on expiring; and
  moving onto a card's button counted as leaving the card, so the button
  row went before the click landed. Each card now tracks the pointer over
  it and its buttons and tells the stack's model; a card that goes from
  under the pointer clears it. The buttons were also laid out each at the
  left of its own column; they are now equal regular buttons across the
  text column, as `surfaces/notifications.md` draws them.
- notify-send's notifications with actions as Firefox and Chromium (by
  their desktop entries): banners with the apps' icons, held while
  hovered, an action reaching the sender. Web notifications from the
  browsers themselves were not driven: they need a page the browser has
  granted permission.
- Control Centre: Now Playing with Chromium's tab as the player (art from
  its icon), Focus, Energy Mode, Sound with Chromium's stream, nm-applet in
  the tray; Wi-Fi, VPN, Bluetooth, Display and Batteries absent, each with
  its reason in the gallery. nm-applet's menu opens as a page through
  DBusMenu, with its submenu, disabled rows and check marks.
- The tray menu page's head showed a generic window glyph; it now shows the
  item's own icon.
- **Open: a click on the tray item twice closed Control Centre** instead of
  opening nm-applet's menu, in the first minutes after a session restart.
  It did not happen again in eight later clicks, with the press and every
  close logged, so its cause is not known.

The host run is optional, as it was for P6 and P7, and is where DDC/CI
(read at 92 of 100 on the AOC over `/dev/i2c-4` during 8.4) and Bluetooth
are real.
