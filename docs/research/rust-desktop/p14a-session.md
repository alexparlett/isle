# P14a: the package in the VM, and the host run

What `p14a-session.md` 14a.6 found in the guest on 2026-10-19, and the host
run that is part of P14a's done.

## The package

`tools/package.sh --no-install -- --nodeps` built `isle-git
r476.g91d37e5ba531` from the checkout's HEAD: 42 MB compressed, the six
binaries and the eleven data files, no reference to the build directory
(the first build had one, Settings' `--write-index` path, now in debug
builds only). `--nodeps` because `gn` is not installed on the host; Skia's
crate fetched its own. The compositor's Skia, with GL, has no prebuilt
archive and is built from the sources the crate carries; Settings' is the
prebuilt one, listed in `source=()` with its sum.

## The guest

CachyOS guest, llvmpipe, 2560 by 1080, its own getty autologin on tty1.

| Check | Result |
|---|---|
| `pacman -U` over v1's unowned `isle.portal` | refused without `--overwrite` for that path, as expected; `tools/package.sh` passes it. The guest also had a hand-copied `isle-portals.conf` from P11's run (identical to the repo's), removed by hand; the host has none |
| `isle setup --check` | the three steps due |
| `isle setup` over ssh without a terminal | the user's steps applied; the root half chose pkexec, which had no agent over ssh, and failed with pkexec's message |
| `isle setup` with a terminal | the root half through sudo; `/var/lib/isle/setup.json` records the old greetd command |
| tuigreet | greetd started with the new command; F3 lists Hyprland, Hyprland (uwsm-managed) and Isle |
| Isle from greetd | `initial_session` with `isle-session` (no password typed): a logind session of class user on seat0 tty1; `/usr/bin/isle-compositor` at the package's commit; the manager names `WAYLAND_DISPLAY`, `DISPLAY`, `XDG_CURRENT_DESKTOP=Isle`, `XCURSOR_THEME=Bibata-Modern-Classic` and `XCURSOR_SIZE=24`; `isle-session.target` and `graphical-session.target` active; the log in `~/.local/state/isle/logs/` and the user journal as `isle-compositor` |
| Login Items | nm-applet and KDE Connect started; the keyring's entries (GNOME only) and at-spi's left out |
| The portal under Isle | `Settings.ReadOne` of `accent-color` answers from Isle's backend, so `isle-portals.conf` is in force |
| `isle quit` | the session's variables gone from the manager, the targets inactive, the Login Items gone with the session, tuigreet back |
| Hyprland from greetd after it | `XDG_CURRENT_DESKTOP=Hyprland`; `isle-session.target` inactive; `accent-color` not found (gtk serves Settings, as `hyprland-portals.conf` says) and v1's shell owns `org.freedesktop.impl.portal.desktop.isle` for FileChooser; v1's log at `~/.local/state/isle/session.log`, Isle's beside it in `logs/` |
| `isle setup --undo` | greetd's command back to the guest's `agreety`, `portals.conf` back, the xremap drop-in and its directory gone; re-applied after |

Not checked in the guest: typing a password at tuigreet (the guest has no
test password to type), xremap under Hyprland (the guest has no xremap),
NVIDIA. The guest was left on its getty autologin with greetd installed
but not enabled, the package installed and setup applied.

## The host run, the user's

1. `sudo pacman -S greetd-tuigreet`.
2. `tools/package.sh`: installs what the build needs (`--syncdeps`),
   builds, and asks before installing; the first install replaces v1's
   unowned `isle.portal`. A 404 from the mirror means the package database
   is behind: `sudo pacman -Syu` first.
3. `isle setup --check`, then `isle setup`: greetd's greeter, the portals
   file, the xremap drop-in. `/etc/pam.d/greetd` already unlocks the
   keyring.
4. Log out of Hyprland. At tuigreet, F3, Isle, the password: check that a
   password with symbols types right on the Keychron (the console's
   keymap).
5. In Isle: `isle version` names the package's commit; `systemctl --user
   show-environment` has `SSH_AUTH_SOCK`; `ssh -T git@github.com` uses
   gcr's agent; Claude, Proton Mail and KDE Connect started; Cmd chords
   reach Isle (xremap not running: `systemctl --user status xremap`
   says its condition failed); `pkexec true` shows Isle's dialog; Wi-Fi
   toggles and a USB stick mounts without a password (`platform.md` bar 3).
6. P12's and P13's host runs on this build (`p12-gaming.md`,
   `p13-settings.md`).
7. `isle quit`, then Hyprland from tuigreet: xremap running, v1's file
   chooser in a browser's Open dialog, v1's lock.
8. Optional: `isle setup --undo` puts greetd's v1 greeter, the portals
   file and xremap back; `isle setup` again to return.
