# P5 · Keyboard: findings

Measurements and observations from building P5 (`docs/work/rust-desktop/p5-keyboard.md`).

## The VM (2026-10-05)

The guest's session on llvmpipe at 2560×1080. QEMU's keyboards (`QEMU Virtio Keyboard`,
`AT Translated Set 2 keyboard`) are guessed PC, so their left Alt is Cmd. Keys were sent
over QMP and read back with `wev` (Wayland), `xev` (X11 through Xwayland), kitty's
`kitten show-key` and a GTK 4 `zenity --entry`:

- **Wayland.** Alt+C (Cmd+C) reaches `wev` as Control_L, C, with the modifiers event
  saying Control; Alt+Left (Cmd+Left) as Home with no modifier.
- **Xwayland.** Cmd+C reaches `xev` as Control_L then c with state 0x14 (Control and Num
  Lock), Shift+Cmd+Z as Shift_L, Control_L, Z with 0x15; Super (Mod4) and Alt (Mod1)
  never appear. The state X11 reports agrees with the key events at every step, so the
  open point in `compositor.md` (whether Xwayland needs modifier key events beside the
  `modifiers` event) is moot: Isle sends only real key events and the state follows.
- **Mac profile through Xwayland.** With `gb` and the keyboards set to `mac`, Option+3
  reaches `xev` as ISO_Level3_Shift then numbersign (state 0x90, Mod5); Option+Up as
  Alt_L then Up (0x18).
- **kitty.** Option+Left arrives as Alt+B (the terminal rule) and Ctrl+A raw (terminals
  skip the emacs keys).
- **GTK 4.** With "pasted-ok" on the clipboard, Cmd+V in a zenity entry pastes it, Cmd+Left
  goes to the line's start, and a typed `>` lands there: the entry returned `>pasted-ok`.
- **xremap.** Started by hand, it was stopped the moment `isle-session.target` came up
  (`Conflicts=xremap.service`). The conflict runs both ways: starting xremap stops the
  Isle session's target, which matters on the host while v1's units are still enabled
  (P14).

Not checked in the VM: a Qt 6 text field (the guest has no Qt 6 app with one); the
keyboard list right after start is empty until the first input event, since libinput's
added devices are dispatched with it.

## Text injection

Isle types through a keymap made for the characters (Smithay's `inject_text_keysyms`),
not through text-input-v3. Serving text-input-v3 with the compositor as the input method
makes GTK 4 hand composition to it, which would take dead keys away from every GTK app
for the sake of an occasional emoji. Headless, an emoji arrives through the injection
keymap; X11 windows are left to the host run.

## Games

Neither KWin nor CachyOS tells the desktop a game is in front: KDE turns Do Not Disturb on
for any full-screen window, CachyOS's `game-performance` is an opt-in launch wrapper, and
ananicy-cpp's 15,000 "Game" process names only set priorities. Steam labels its own games:
`STEAM_GAME` on X11 windows (gamescope reads it), `steam_app_<id>` classes for Proton and
`SteamGameId` in the process environment. Feral GameMode's D-Bus service is
session-activated, so asking it for its games starts it; Isle asks only while the name has
an owner.

`STEAM_GAME` needs Smithay's window manager to read the property: the fork's
window manager reads it (alexparlett/smithay#2) and reports a change, and Isle
works the game flag out again then.
