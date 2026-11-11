# Power, sleep and keep-awake holds: research note (2026-11-11)

Scope: what P20h builds on. Host: CachyOS, kernel 7.2.10, Isle's compositor
as the session (`XDG_CURRENT_DESKTOP=Isle`), Claude desktop on Electron
44.7.2, Vivaldi, electron42 and electron43 installed.

"Local:" marks evidence gathered on this host today. *Unknown* marks what
was not settled.

---

## 1 Why Claude's keep-awake does nothing under Isle

- Local: Claude desktop's keep-awake is
  `powerSaveBlocker.start("prevent-app-suspension")` (read from its
  `app.asar`), logged as `[keep-awake] started`.
- Local: with keep-awake turned on, no `Inhibit` call of any interface
  reached the session bus, and logind listed no new inhibitor from
  `claude-desktop` (its only entries are `sleep` and `shutdown` delays,
  "Application cleanup before suspend").
- Local: a bare Electron 43 app calling the same blocker, watched with
  `dbus-monitor`:
  - Chromium asks `NameHasOwner` for `org.gnome.SessionManager`, then for
    `org.freedesktop.PowerManagement`. Neither is owned under Isle, so it
    calls nothing and `isStarted` still answers true.
  - With a stub owning `org.freedesktop.PowerManagement`, it called
    `org.freedesktop.PowerManagement.Inhibit.Inhibit` at
    `/org/freedesktop/PowerManagement/Inhibit` with
    `("/usr/lib/electron43/electron", "Electron")` at once.
- Local: Vivaldi's "Video Wake Lock" (`prevent-display-sleep`) reaches
  Isle's `org.freedesktop.ScreenSaver` and is logged as "idle inhibited".
- So Chromium splits its two holds by interface: a display hold goes to
  ScreenSaver, an app-suspension hold to the session manager or
  PowerManagement. `lock-and-prompts-2026.md` §6.2 lists only the first.
- *Unknown*: which of Chromium's own wake locks use app suspension. Audio
  playback without video is believed to, which would make a
  PowerManagement hold that also held the screen keep it lit under music.
  To be checked in the VM before 20h.2 closes.

## 2 The interface

`org.freedesktop.PowerManagement.Inhibit` at
`/org/freedesktop/PowerManagement/Inhibit` (freedesktop's draft, served by
KDE's PowerDevil and xfce4-power-manager):

- `Inhibit(s application, s reason) -> u cookie`
- `UnInhibit(u cookie)`
- `HasInhibit() -> b`
- signal `HasInhibitChanged(b)`

Its meaning is "do not suspend the computer"; the display may still sleep.
Firefox also calls it (`lock-and-prompts-2026.md` §6.2). As with
ScreenSaver, cookies are dropped when the caller leaves the bus.

## 3 Sleep today

- Local: `/sys/power/state` offers `freeze mem disk`; `mem_sleep` is
  `s2idle [deep]`.
- Local: `/etc/systemd/logind.conf` sets none of `HandlePowerKey`,
  `IdleAction` or `HandleLid*`, so logind never suspends on idle. Isle's
  idle has three steps (dim, lock, display off) and nothing suspends the
  computer except the session dialog's Sleep and the system menu.
- Local: Isle holds `handle-power-key` (block) and `sleep` (delay) from
  logind.
- So Claude's keep-awake today has nothing to keep away: the computer
  never sleeps, and the screen locks and turns off as it would on a Mac
  with the same hold.

## 4 Waking by the network

- Local: the host's active connection is Wi-Fi (`wlan0`, iwlwifi); the
  wired `enp7s0` (r8169) is down. Both report `power/wakeup` disabled.
- Local: `iw phy` lists WoWLAN with wake on magic packet, among others.
- Local: the Wi-Fi connection's `802-11-wireless.wake-on-wlan` is `0x1`
  (the global default, which is off). NetworkManager sets magic-packet
  wake per connection: `802-3-ethernet.wake-on-lan` `magic` (0x40) and
  `802-11-wireless.wake-on-wlan` `magic` (0x8).
- *Unknown*: whether this board's firmware wakes from `deep` on WoWLAN or
  on the r8169's magic packet. To be tried at 20h.10.
