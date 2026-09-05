# Where everything is configured

A lookup table for "how do I change X", and how that compares to doing the same
thing on a traditional desktop. Written after assembling the whole thing, partly
as a reference and partly as an honest audit of where this setup is better,
equivalent, or worse than KDE Plasma.

**Legend** — how you change a thing:

- 🖱 **GUI** — a window with controls
- 📄 **File** — edit text, reload
- 🔌 **Service** — a system tool, usually `systemctl` or a CLI

---

## Appearance

| Feature | Here | Traditional desktop | Verdict |
|---|---|---|---|
| Compositor look (gaps, rounding, blur, shadows, animations) | 🖱 HyprMod, or 📄 `hypr/look.lua` | 🖱 System Settings → Appearance | **Equivalent.** HyprMod covers every option with search and undo |
| Colour theme | 📄 `hypr/theme.lua` + a hardcoded copy in each of waybar, rofi, swaync, kitty, wlogout, hyprlock | 🖱 One colour scheme picker, applied everywhere | **Worse.** Six copies of the same palette; changing accent colour means six edits |
| GTK app theme | 🖱 nwg-look | 🖱 System Settings → Appearance | Equivalent |
| Qt app theme | 🖱 qt6ct | 🖱 built in | Equivalent |
| KDE app colours | 🔌 `color-schemes/apply-colors.py`, run by install | 🖱 built in | Slightly worse — one-shot, not a picker |
| Icons | 📄 `gtk-3.0/settings.ini`, `qt6ct.conf` | 🖱 one picker | Worse, two places |
| Fonts | 📄 same two files, plus each app's own config | 🖱 one picker | Worse |
| Cursor theme | 📄 `hypr/env.lua` | 🖱 one picker | Worse |
| Wallpaper | 📄 `hypr/hyprpaper.conf` | 🖱 right-click desktop | Worse — no picker, no slideshow |

## Input

| Feature | Here | Traditional desktop | Verdict |
|---|---|---|---|
| Keyboard layout | 🖱 HyprMod, or 📄 `hypr/input.lua` | 🖱 Settings → Keyboard | Equivalent |
| Key repeat | 🖱 HyprMod | 🖱 built in | Equivalent |
| Keyboard shortcuts | 🖱 System Settings → Keyboard, or 📄 `hypr/keybinds.conf` | 🖱 Settings → Shortcuts | Equivalent |
| Shortcut *profile* (macOS / Windows) | 🖱 System Settings, `keymap.sh`, or ⌥⌘K | ❌ nothing comparable | **Better** |
| ⌘C/⌘V inside applications | 📄 `keyd/default.conf`, opt-in via `--mac-keys` | ❌ not possible on KDE | **Better** |
| Mouse speed, acceleration, natural scroll | 🖱 HyprMod | 🖱 Settings → Mouse | Equivalent |

## Display and power

| Feature | Here | Traditional desktop | Verdict |
|---|---|---|---|
| Resolution, refresh, arrangement | 🖱 nwg-displays, or 📄 `hypr/monitors.lua` | 🖱 Settings → Display | Equivalent |
| VRR / adaptive sync | 🖱 HyprMod, or 📄 `monitors.lua` | 🖱 built in | Equivalent |
| HDR | 🖱 HyprMod | 🖱 built in | Worse — Hyprland's HDR is less mature than KWin's |
| Night light | 🖱 System Settings, or 📄 `hypr/hyprsunset.conf` | 🖱 built in | Equivalent |
| Screen blank / lock timeouts | 🖱 System Settings → Power | 🖱 Settings → Power | Equivalent |
| Lock screen appearance | 📄 `hypr/hyprlock.conf` | 🖱 a picker | Worse |
| Power profile | 🖱 System Settings → Power | 🖱 battery applet | Equivalent |
| Suspend / reboot / shut down | 🖱 wlogout, System Settings | 🖱 application menu | Equivalent |

## Hardware and services

| Feature | Here | Traditional desktop | Verdict |
|---|---|---|---|
| Sound | 🖱 System Settings (volume, device), pavucontrol (per-app) | 🖱 Settings → Sound | Equivalent |
| Wi-Fi and network | 🖱 nm-applet, System Settings, nm-connection-editor | 🖱 Settings → Network | Equivalent |
| Bluetooth | 🖱 Blueman, System Settings | 🖱 Settings → Bluetooth | Equivalent |
| Printers | 🖱 system-config-printer | 🖱 Settings → Printers | Equivalent *(cups not installed — see optional.txt)* |
| Disks | 🖱 gnome-disk-utility | 🖱 Settings → Disks | Equivalent |
| Sensors and fans | 🖱 CoolerControl | ❌ needs a third-party app anyway | Equivalent |
| Date, time, timezone | 🖱 System Settings | 🖱 Settings → Date & Time | Equivalent |
| Users | 🖱 read-only list; 🔌 `useradd`/`passwd` to change | 🖱 full user manager | **Worse** — no GUI for adding or editing accounts |
| Secrets / keyring | 🔌 gnome-keyring, unlocked by PAM | 🖱 KWallet manager | Slightly worse — no browser GUI |
| Updates | 🖱 Waybar module, ⌥⌘U, `cachy-update` | 🖱 Discover | Equivalent |

## Desktop behaviour

| Feature | Here | Traditional desktop | Verdict |
|---|---|---|---|
| Window tiling | automatic (dwindle) | ❌ manual, or KWin scripts | **Better** |
| Drag-to-edge snapping | 📄 `hypr/snap.lua` | 🖱 built in (Aero Snap) | Equivalent |
| Keyboard window zones | ⌥⌘1-6 / Win+arrows | 🖱 limited | **Better** |
| Window rules (float this, opacity that) | 📄 `hypr/rules.lua`, or 🖱 HyprMod | 🖱 KWin Window Rules | Equivalent |
| Virtual desktops | 📄 `monitors.lua` pins them; ⌘1-0 | 🖱 Settings → Virtual Desktops | Equivalent |
| Panel / bar contents | 📄 `waybar/config.jsonc` + `style.css` | 🖱 right-click → Edit Panel | **Worse** — no GUI, but far more capable |
| Launcher | 📄 `rofi/*.rasi` | 🖱 limited | Equivalent |
| Notifications | 📄 `swaync/config.json` | 🖱 Settings → Notifications | Worse — no GUI |
| Clipboard history | ⌘⇧V (cliphist + rofi) | 🖱 Klipper | Equivalent |
| Screenshots | Print, ⌘⇧3/4/5 | 🖱 Spectacle | Equivalent |
| Screen recording | 🔌 `wf-recorder` by hand | 🖱 Spectacle | **Worse** — no keybind, no GUI |
| Default applications | 🖱 System Settings → Default Apps | 🖱 Settings → Default Applications | Equivalent |
| Autostart | 📄 `hypr/autostart.lua`, plus `dex` replaying `~/.config/autostart` | 🖱 Settings → Autostart | Worse — no GUI, but XDG entries still honoured |
| Screen sharing | xdg-desktop-portal-hyprland | built in | Equivalent |

## Things a traditional desktop does not have

| Feature | Where |
|---|---|
| Agent status in the bar — Claude Code and Codex sessions, tasks, and whether one is blocked on you | `hypr/scripts/waybar-agents.sh`, driven by the agents' lifecycle hooks |
| Idle deferral while an agent is working | `hypr/scripts/agent-busy.sh` via hypridle's `condition_cmd` |
| Auto do-not-disturb on screen share or fullscreen game | `hypr/dnd.lua` |
| Docked AI panels on their own special workspaces | `hypr/rules.lua`, ⌥⌘A / ⌥⌘G |
| GameMode indicator that shows whether it is *actually* active | `hypr/scripts/waybar-gamemode.sh` |
| Shortcut profiles switchable at runtime | `hypr/keymap.lua` |
| A test suite for the desktop config | `tests/run.sh` |

## Honest summary

**Better than KDE:** tiling, keyboard-driven window zones, shortcut profiles,
Mac-key emulation, and everything in the agent-integration section — none of
which a traditional desktop offers at all.

**Equivalent:** most hardware and service configuration. The GUIs are separate
windows instead of panels in one app, which is a navigation difference rather
than a capability one.

**Worse, and worth knowing:**

1. **The palette is copied six times.** One source of truth in `theme.lua` with
   generated colour blocks would fix this; it is the largest remaining wart.
2. **No wallpaper picker.** Edit a file and reload.
3. **No user management GUI.** `useradd` and `passwd`.
4. **No screen-recording keybind.** `wf-recorder` is installed but unbound.
5. **Notifications, the bar and the lock screen are file-only.** More capable
   than their KDE equivalents, but there is no dialog.
6. **HDR is less mature** than KWin's.

**The trade that defines the whole thing:** a traditional desktop gives you one
window that configures everything and can express only what its designers
anticipated. This gives you text files that can express anything, a test suite
that catches mistakes before login, and two GUIs covering the parts worth
clicking. Whether that is an upgrade depends entirely on whether you would
rather edit a file than hunt through a settings tree.
