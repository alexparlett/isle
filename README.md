# Isle

A desktop shell for Hyprland, built in Quickshell. Nothing on screen at
rest but a pill at the top of the display. Everything else, the control
panel, notifications, a dashboard, a launcher, the lock screen, Settings,
a Monitor and a Keychain, comes out of that one process when asked and
goes away when done. Built for two workloads, gaming and running coding
agents, on CachyOS.

![The control panel unfolded from the island](docs/screenshots/island-panel.jpg)

## The island

The pill is the clock, the desktop dots, and a glyph for anything that
has something to say: recording, a muted mic, a VPN, pending updates. It
morphs into whatever is happening, a notification with its actions, the
volume, the track that just started, a drive that was plugged in, an
authentication prompt, and returns to rest.

![At rest](docs/screenshots/island-rest.jpg)
![A notification](docs/screenshots/island-notification.jpg)
![Now playing](docs/screenshots/island-media.jpg)

Hovering unfolds the control panel: Wi-Fi, Bluetooth, do not disturb,
night light, do not disturb and game mode, the output and its volume, media, and
a footer with the power profile and the update count.

## The dashboard

A grid of widgets on the wallpaper: system, calendar, weather, media,
notifications, clipboard, devices, disks, games, sessions. Every widget
moves, resizes and has its own settings. Edit mode is one key.

![The dashboard](docs/screenshots/dashboard.jpg)

## The launcher

Apps, windows, files, the clipboard, passwords and a calculator, from one
field with a prefix for each. Mission Control spreads the desktop's windows out, with every
desktop in a bar above them, to switch, move or close.

![The launcher](docs/screenshots/launcher.jpg)
![The overview](docs/screenshots/overview.jpg)

## Settings, Monitor, Keychain

Settings is grouped the way Plasma, macOS and Windows group it and has a
search that jumps to the row. Displays are dragged into place. Appearance
is dark, light, or auto from sunrise to sunset; the installed apps follow,
GTK and Qt alike, title bars included. Updates covers the shell itself
and the packages.

![Settings](docs/screenshots/settings-appearance.jpg)

The Monitor is Activity Monitor's shape: CPU, memory, energy, disk,
network and sensors, with the processes grouped under each.

![The Monitor](docs/screenshots/monitor-cpu.jpg)

The Keychain is the keyring and the SSH agent in one window: logins, Wi-Fi
and app secrets, and the keys, with a generator and a passphrase prompt.

![The Keychain](docs/screenshots/keychain.jpg)

## Modes

Do not disturb holds the notifications. Game mode drops every effect,
turns on VRR and tearing where allowed, and hides the shell until the game
ends. Big Picture is a couch view driven by a controller.

![Game mode](docs/screenshots/game-mode.jpg)
![Big Picture](docs/screenshots/big-picture.jpg)

The lock screen and the greeter draw with the same material.

![Lock](docs/screenshots/lock.jpg)
![Light](docs/screenshots/light-dashboard.jpg)

## Install

On CachyOS, as your user:

```bash
curl -fsSL https://raw.githubusercontent.com/alexparlett/isle/main/tools/bootstrap.sh | bash
```

Then log out and pick Hyprland at the login screen. Or build the ISO with
`tools/iso/build.sh`: CachyOS's own live ISO with Isle in the installer's
desktop list. Either way, Settings › Updates keeps the shell current.

The desktop runs from `~/.local/share/isle`. To work on it, clone anywhere
else, test in the VM (`tools/test-vm.sh`), and run `tools/install.sh` from
that checkout to copy it into place; editing the checkout changes nothing
until then.

## Documents

| | |
|---|---|
| [docs/DESIGN.md](docs/DESIGN.md) | Look and feel: tokens, material, geometry, type, motion, the island, every surface |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Process layout, what drives each capability, the install and ISO paths, the test VM |
| [docs/STACK.md](docs/STACK.md) | Why Hyprland and Quickshell; the idle budget |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Numbered decisions and why |
| [docs/canvas/](docs/canvas/build.mjs) | The design canvas source |
