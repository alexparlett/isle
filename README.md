# hyprland

A hand-rolled [Hyprland](https://hypr.land) desktop for CachyOS, replacing KDE Plasma.

Built for one specific machine — Ryzen 7 9800X3D, RTX 5070 Ti (nvidia-open), and a
single AOC U34G2G4R3 ultrawide at 3440×1440 — so the monitor, GPU and keyboard
layout are hardcoded rather than auto-detected. Everything machine-specific is
called out below.

## Install

```bash
git clone https://github.com/alexparlett/hyprland ~/Development/hyprland
cd ~/Development/hyprland
./install.sh
```

Then log out, pick **Hyprland** at SDDM, log back in.

`install.sh` installs packages, symlinks `config/*` into `~/.config`, generates a
wallpaper, and enables the daily update check. It is idempotent, it moves any
existing real config aside to `*.bak-<timestamp>` rather than deleting it, and it
does not touch Plasma — that stays installed and selectable at SDDM as a fallback.

| Flag | Effect |
|---|---|
| `--no-packages` | symlinks only |
| `--no-aur` | skip the two AUR theme packages |
| `--dry-run` | print what it would do |
| `--unlink` | remove the symlinks it created |

## What's in here

| Component | Choice | Why |
|---|---|---|
| Compositor | Hyprland 0.56 | |
| Bar | Waybar | mature, and the modules cover everything Plasma's panel did |
| Launcher | rofi | drun, run, window switching, and the clipboard picker |
| Notifications | swaync | has a control centre, not just toasts |
| Lock / idle | hyprlock + hypridle | |
| Wallpaper | hyprpaper | |
| Terminal | Alacritty | already installed and configured on this box |
| File manager | Dolphin | kept from KDE; no reason to relearn one |
| Auth agent | hyprpolkitagent | |
| Volume OSD | swayosd | |
| Power menu | wlogout | |
| Updates | cachy-update + Waybar module | see [Updates](#updates) |
| Theme | Catppuccin Mocha | best port coverage of any dark palette |

## Layout

```
config/
  hypr/            Hyprland itself — Lua, one module per concern
    hyprland.lua     entrypoint, requires the rest in dependency order
    theme.lua        Catppuccin palette, shared by the other modules
    env.lua          environment variables, including the NVIDIA bits
    monitors.lua     the ultrawide, and workspace pinning
    input.lua        keyboard, mouse, gestures
    look.lua         gaps, borders, blur, shadows, animations
    binds.lua        every keybind, each with a description
    rules.lua        window / layer / workspace rules
    gaming.lua       tearing, direct scanout, gamescope notes
    autostart.lua    what starts with the session
    hyprlock.conf    still hyprlang — hyprlock has not moved to Lua
    hypridle.conf    likewise
    hyprpaper.conf   likewise
    hyprsunset.conf  likewise — night light profiles by time of day
    scripts/         screenshots, clipboard, cheatsheet, Waybar modules
  waybar/ rofi/ swaync/ swayosd/ wlogout/ alacritty/ qt6ct/ gtk-3.0/ gtk-4.0/
  arch-update/     update notifier config
  xdg-desktop-portal/
packages/          repo.txt and aur.txt, one package per line
install.sh
```

### Why Lua and not `hyprland.conf`

Hyprland 0.55 deprecated hyprlang in favour of Lua, and 0.56 is what CachyOS ships.
`hyprland.conf` still works for a release or two but is on the way out, so this
config is Lua from the start. `hyprlock`, `hypridle` and `hyprpaper` have **not**
moved — those three keep the old `.conf` syntax.

## Keybindings

`SUPER + /` lists every bind, read live out of `hyprctl binds`, so the cheatsheet
can never drift from the config.

| Key | Action |
|---|---|
| `SUPER + Return` | Terminal |
| `SUPER + E` | File manager |
| `SUPER + D` / `R` | App launcher / run a command |
| `SUPER + Tab` | Window switcher |
| `SUPER + V` | Clipboard history |
| `SUPER + Q` | Close window |
| `SUPER + F` / `SHIFT + F` | Fullscreen / maximise |
| `SUPER + Space` | Toggle floating |
| `SUPER + arrows` | Focus |
| `SUPER + SHIFT + arrows` | Move window |
| `SUPER + CTRL + arrows` | Resize |
| `SUPER + ALT + 1/2/3` | Snap to a third of the ultrawide |
| `SUPER + 1…0` | Workspace |
| `SUPER + S` | Scratchpad |
| `Print` | Screenshot a region |
| `SUPER + L` | Lock |
| `SUPER + Escape` | Power menu |
| `SUPER + U` | Run a system update |

## Updates

CachyOS ships `cachy-update`, its build of
[arch-update](https://github.com/Antiz96/arch-update). This config wires it up
three ways:

- **`arch-update.timer`** — enabled by `install.sh`, checks daily and sends a
  desktop notification through swaync.
- **Waybar module** — a badge with the pending count, tooltip listing the
  packages, hidden entirely when there is nothing to do. Click it to update,
  right-click for the Arch news.
- **`SUPER + U`** — the interactive updater in a terminal. It shows the news
  first, then handles orphans, pacnew files, cache trimming and services needing
  a restart afterwards.

`config/arch-update/arch-update.conf` sets paru as the AUR helper and keeps three
old package versions cached so a downgrade is always possible.

## Machine-specific bits

Change these if this config ever runs on different hardware:

| What | Where | Current value |
|---|---|---|
| Monitor output and mode | `config/hypr/monitors.lua` | `DP-4`, 3440×1440@144 |
| Primary GPU | `config/hypr/env.lua` | `AQ_DRM_DEVICES` → PCI `01:00.0` |
| Keyboard layout | `config/hypr/input.lua` | `gb` |
| Ultrawide thirds geometry | `config/hypr/binds.lua` | `COL_W`, `COL_H`, `TOP` |

`install.sh` checks the GPU node actually exists and warns if the PCI address is
wrong on the machine it is run on.

### NVIDIA notes

- The 50xx series **requires** the open kernel modules (`nvidia-open`); the legacy
  proprietary driver does not support Blackwell at all.
- `nvidia_drm modeset=1` is already set by Arch's packaging, and `fbdev` follows
  it automatically on 570+. `install.sh` verifies it.
- `GBM_BACKEND` and `WLR_NO_HARDWARE_CURSORS` are deliberately **not** set. Both
  were workarounds for pre-555 drivers and cause problems on current ones.
- The AMD iGPU is present but drives nothing, so `AQ_DRM_DEVICES` pins rendering
  to the dGPU. Without it aquamarine can pick the headless iGPU.

## Coming from Plasma

Apps are unaffected — Hyprland replaces the compositor and shell, not the
application stack. Qt, GTK, Electron, Flatpak, Steam and anything X11 (via
XWayland) all keep working, KDE apps included.

What does change:

- **XDG autostart** — Plasma runs `~/.config/autostart/*.desktop`; Hyprland does
  not. `autostart.lua` runs `dex` to replay them, so nothing silently stops.
- **Qt theming** — KDE apps read `kdeglobals` and stay correct. Everything else
  goes through qt6ct.
- **KDE global shortcuts** don't apply; rebind them in `binds.lua`.
- **HDR** — KWin's implementation is more mature than Hyprland's today.
- **Plasma widgets, Activities, desktop icons** have no equivalent.

## Local overrides

`~/.config/hypr/local.lua` is sourced last if it exists and is git-ignored — the
place for anything you don't want to commit.
