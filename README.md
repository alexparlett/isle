# hyprland

A hand-rolled [Hyprland](https://hypr.land) desktop for CachyOS, replacing KDE Plasma.

Built for one specific machine — Ryzen 7 9800X3D, RTX 5070 Ti (nvidia-open), a
single AOC U34G2G4R3 ultrawide at 3440×1440, and a keyboard wearing Mac keycaps —
so the monitor, GPU, layout and modifier scheme are hardcoded rather than
detected. Everything machine-specific is listed under
[Machine-specific bits](#machine-specific-bits).

## Install

```bash
git clone https://github.com/alexparlett/hyprland ~/Development/hyprland
cd ~/Development/hyprland
./install.sh
```

Then log out, pick **Hyprland** at SDDM, log back in.

`install.sh` installs packages, symlinks `config/*` into place, generates a
wallpaper, enables the daily update check, wires the coding-agent status hooks
and recolours KDE apps. It is idempotent, it moves existing real files aside to
`*.bak-<timestamp>` rather than deleting them, and it does not touch Plasma —
that stays installed and selectable at SDDM as a fallback.

| Flag | Effect |
|---|---|
| `--no-packages` | symlinks only |
| `--no-aur` | skip the AUR theme packages |
| `--no-agent-hooks` | skip the Claude Code / Codex status hooks |
| `--remove-agent-hooks` | remove those hooks and exit |
| `--no-kde-colors` | leave KDE app colours alone |
| `--mac-keys` | install the keyd Mac profile (needs sudo) |
| `--dry-run` | print what it would do |
| `--unlink` | remove the symlinks it created |

## What's in here

| Component | Choice | Why |
|---|---|---|
| Compositor | Hyprland 0.56 | configured in Lua — see below |
| Bar | Waybar | the modules cover everything Plasma's panel did, and then some |
| Launch bar | rofi | ⌘Space, Spotlight-shaped, with a calculator built in |
| Notifications | swaync | has a control centre, not just toasts |
| Lock / idle | hyprlock + hypridle | |
| Wallpaper | hyprpaper | generated gradient, no binary in git |
| Night light | hyprsunset | profiles by time of day |
| Terminal | Alacritty | already installed and configured on this box |
| File manager | Dolphin | kept from KDE; recoloured to match |
| Auth agent | hyprpolkitagent | |
| Volume OSD | swayosd | |
| Power menu | wlogout | |
| Settings | custom GTK4 window | live-tunes what's worth tuning by feel |
| Updates | cachy-update + Waybar module | |
| Agent status | Claude Code + Codex hooks → Waybar | |
| Theme | Catppuccin Mocha | best port coverage of any dark palette |

## Layout

```
config/
  hypr/              Hyprland — Lua, one module per concern
    hyprland.lua       entrypoint, requires the rest in dependency order
    theme.lua          Catppuccin palette, shared by the other modules
    env.lua            environment, including the NVIDIA bits
    monitors.lua       the ultrawide, and workspace pinning
    input.lua          keyboard (Mac swap), mouse, gestures
    look.lua           gaps, borders, blur, shadows, animations
    binds.lua          every keybind, each with a description
    rules.lua          window / layer / workspace rules, AI panels
    gaming.lua         tearing, direct scanout, gamescope notes
    autostart.lua      what starts with the session
    hyprlock.conf      still hyprlang — these four have not moved to Lua
    hypridle.conf
    hyprpaper.conf
    hyprsunset.conf
    scripts/           launcher, screenshots, clipboard, Waybar modules
      hooks/           agent-state.sh — the Claude/Codex → Waybar bridge
    settings/          the GTK4 settings window
  waybar/ rofi/ swaync/ swayosd/ wlogout/ alacritty/ qt6ct/ gtk-3.0/ gtk-4.0/
  claude/ codex/      lifecycle hook manifests, merged into the agents' configs
  keyd/               optional Mac keyboard profile
  color-schemes/      Catppuccin Mocha for KDE apps
  applications/       .desktop entry for the settings window
  arch-update/        update notifier config
  xdg-desktop-portal/
packages/            repo.txt and aur.txt, one package per line
install.sh
```

### Why Lua and not `hyprland.conf`

Hyprland 0.55 deprecated hyprlang in favour of Lua, and 0.56 is what CachyOS
ships. `hyprland.conf` still works for a release or two but is on the way out, so
this config is Lua from the start: `hl.config{}`, `hl.bind()`, `hl.window_rule{}`,
`hl.on("hyprland.start", …)`. `hyprlock`, `hypridle`, `hyprpaper` and `hyprsunset`
have **not** moved — those four keep the old `.conf` syntax.

## Keybindings

The keyboard has Mac keycaps, so the bottom row reads `Ctrl / ⌥ / ⌘`. On a PC
board the ⌘ cap sits on the physical Alt key, so `input.lua` applies
`altwin:swap_alt_win` and it sends SUPER. Shortcuts follow macOS wherever macOS
has a convention.

**⌘ + ?** lists every bind, read live from `hyprctl binds`, so the cheatsheet can
never drift from the config.

| Keys | Action |
|---|---|
| `⌘ Space` | Launch bar (apps + calculator) |
| `⌃ ⌘ Space` | Emoji picker |
| `⌘ Return` | Terminal |
| `⌘ E` | File manager |
| `⌘ ,` | Settings window |
| `⌘ ?` | Keybind cheatsheet |
| `⌘ Q` / `⌘ W` | Close window |
| `⌥ ⌘ Esc` | Force quit |
| `⌃ ⌘ Q` | Lock screen |
| `⌘ Esc` | Power menu |
| `⌃ ⌘ F` | Fullscreen |
| `⌥ ⌘ F` | Maximise within gaps |
| `⌘ ⇧ F` | Toggle floating |
| `⌘ Tab` | Window switcher |
| `⌘ \`` | Cycle windows |
| `⌘ H` / `⌘ ⇧ H` | Hide window / show hidden |
| `⌥ ⌘ arrows` | Focus |
| `⌥ ⌘ ⇧ arrows` | Move window |
| `⌃ ⌘ arrows` | Resize |
| `⌥ ⌘ 1/2/3` | Snap to a third of the ultrawide |
| `⌘ 1…0` | Workspace |
| `⌘ ⇧ 1…0` | Send window to workspace |
| `⌃ ⌥ ←/→` | Previous / next workspace |
| `⌘ ⇧ 3/4/5/6` | Screenshot screen / region / annotate / window |
| `⌘ ⇧ V` | Clipboard history |
| `⌥ ⌘ A` / `⌥ ⌘ G` | Claude / ChatGPT panel |
| `⌥ ⌘ N` | Notification centre |
| `⌥ ⌘ U` | System update |

Bare `⌘C`, `⌘S`, `⌘F`, `⌘A` and `⌘←/→` are deliberately unbound — they belong to
applications and text fields.

### Making ⌘C actually copy

The xkb swap puts the window-manager shortcuts under the ⌘ cap, but it cannot
make `⌘C` mean copy inside applications, because Linux apps expect Ctrl. For
that, install the optional keyd profile:

```bash
./install.sh --mac-keys
```

keyd sits below Wayland at the evdev layer and rewrites `⌘C/⌘V/⌘X/⌘Z/⌘S/⌘T/⌘F…`
to their Ctrl equivalents, plus `⌘←/→` to Home/End and `⌥←/→` to word-wise
movement. Anything it doesn't list falls through as plain Super, so every bind in
the table above still reaches Hyprland. `input.lua` detects the profile by a
marker string and drops its own swap so the two don't fight.

## Coding-agent widgets

Both Claude Code and Codex expose lifecycle hooks, so the bar can show what your
agents are doing without polling anything:

- **`custom/claude`** and **`custom/codex`** show a session count, plus running
  background tasks and subagents.
- A session that is **blocked waiting on you** — a permission request, a
  question — turns the pill red and pulses it. On a 3440px screen a static
  indicator is easy to miss.
- The tooltip breaks it down per session: project, state, why it's waiting, how
  long ago.
- Left-click picks a session and focuses its terminal; right-click opens the
  desktop app.

`install.sh` merges `config/claude/hooks.json` into `~/.claude/settings.json` and
writes `config/codex/hooks.json` to `~/.codex/hooks.json`. Both merges are
idempotent, back up the original, and leave any hooks of your own untouched;
`--remove-agent-hooks` reverses them.

> **Codex requires one manual step**: it will not run a hook until you have
> reviewed and trusted it. Open Codex, run `/hooks`, and enable the profile.
> Trust is recorded against the hook's hash, so editing the file means
> re-approving it.

Codex has no `TaskCreated`/`TaskCompleted` equivalent, so its task counter stays
at zero; sessions, subagents, permission requests and interrupts all work.

There are also two presence-only pills for the **desktop apps** (Claude and
ChatGPT), which report running/wants-attention from Hyprland's own urgency flag —
neither app exposes anything richer.

`⌥⌘A` and `⌥⌘G` dock either app as a 1000px column down the right-hand edge, on
its own special workspace, so it overlays whatever you're doing and disappears
again without touching the layout underneath.

## Settings window

`⌘,` opens a small GTK4 window for the settings worth tuning by feel: gaps,
border width, rounding, inactive opacity, blur, shadows, animations, focus
behaviour, adaptive sync, tearing, cursor hiding, and colour temperature.

Every change applies to the running compositor immediately via
`hyprctl eval 'hl.config{…}'`, which is how 0.55+ takes runtime config. Nothing
is written to disk until you press **Save**, which writes `~/.config/hypr/local.lua`
— the git-ignored override file `hyprland.lua` requires last. **Reload from file**
throws away unsaved experiments.

Monitors, keybinds and window rules are deliberately absent: they want a text
editor and a diff, not a slider. The window links out to nwg-look, qt6ct,
pavucontrol and blueman for the things those own.

## Updates

CachyOS ships `cachy-update`, its build of
[arch-update](https://github.com/Antiz96/arch-update). This config wires it up
three ways:

- **`arch-update.timer`** — enabled by `install.sh`, checks daily and notifies
  through swaync.
- **Waybar module** — pending count, tooltip listing the packages, hidden
  entirely when there's nothing to do. Click to update, right-click for the news.
- **`⌥⌘U`** — the interactive updater. Shows the Arch news first, then handles
  orphans, pacnew files, cache trimming and services needing a restart.

## Machine-specific bits

| What | Where | Current value |
|---|---|---|
| Monitor output and mode | `config/hypr/monitors.lua` | `DP-4`, 3440×1440@144 |
| Primary GPU | `config/hypr/env.lua` | `AQ_DRM_DEVICES` → PCI `01:00.0` |
| Keyboard layout | `config/hypr/input.lua` | `gb`, Alt/Super swapped |
| Ultrawide thirds geometry | `config/hypr/binds.lua` | `COL_W`, `COL_H`, `TOP` |

`install.sh` checks the GPU node exists and warns if the PCI address differs on
the machine it runs on.

### NVIDIA notes

- The 50xx series **requires** the open kernel modules (`nvidia-open`); the
  legacy proprietary driver does not support Blackwell at all.
- `nvidia_drm modeset=1` is already set by Arch's packaging and `fbdev` follows
  it on 570+. `install.sh` verifies it.
- `GBM_BACKEND` and `WLR_NO_HARDWARE_CURSORS` are deliberately **not** set — both
  were pre-555 workarounds that cause problems on current drivers.
- The AMD iGPU drives nothing, so `AQ_DRM_DEVICES` pins rendering to the dGPU.
  Without it aquamarine can pick the headless iGPU.
- Electron apps are launched with `--enable-features=WaylandLinuxDrmSyncobj`,
  which is what stops them flickering.

## Coming from Plasma

Apps are unaffected — Hyprland replaces the compositor and shell, not the
application stack. Qt, GTK, Electron, Flatpak, Steam and anything X11 (via
XWayland) keep working, KDE apps included.

What changes:

- **XDG autostart** — Plasma runs `~/.config/autostart/*.desktop`; Hyprland
  doesn't. `autostart.lua` runs `dex --environment Hyprland`, which replays them
  while correctly skipping the `OnlyShowIn=KDE` entries (plasmashell, powerdevil,
  baloo) and keeping the ones that matter, like KDE Connect.
- **Qt theming** — KDE apps read `kdeglobals`; `install.sh` applies a Catppuccin
  Mocha colour scheme so Dolphin matches everything else. Non-KDE Qt apps go
  through qt6ct. Note this recolours the Plasma fallback session too, since it's
  the same file.
- **KDE global shortcuts** don't apply; rebind them in `binds.lua`.
- **HDR** — KWin's implementation is more mature than Hyprland's today.
- **Plasma widgets, Activities, desktop icons** have no equivalent.

### Consistency

Themed to Catppuccin Mocha: Hyprland, Waybar, rofi, swaync, swayosd, wlogout,
hyprlock, Alacritty, GTK3/GTK4 apps, non-KDE Qt apps, KDE apps, the settings
window, and the cursor.

Not themed, and not really fixable from here: the SDDM login screen (the AUR has
`catppuccin-sddm-theme-mocha` if you want it — it needs a root edit to
`/etc/sddm.conf.d/` so it isn't automated), browser chrome, and the interiors of
Electron apps like Claude and ChatGPT.

## Local overrides

`~/.config/hypr/local.lua` is required last if it exists and is git-ignored — the
place for anything you don't want to commit, and where the settings window saves.
