# Stack

The technology decision, with the alternatives that were weighed and what
each would cost. Memory figures are idle resident set on a desktop with one
monitor; they are ballpark from upstream and community measurements and are
re-measured in the VM at slice 0.

## What the design needs from the stack

1. One persistent surface (the island) that morphs: springs, size
   animation, alpha, blur behind it. Everything else appears on demand.
2. A full feature list: audio, network, bluetooth, notifications, media,
   lock, idle, polkit, keyring, disks, monitor, capture, clipboard,
   launcher, switcher, settings, big picture. Most of that is service
   plumbing, not pixels.
3. Gaming: VRR, tearing, direct scanout for fullscreen, HDR when it comes,
   NVIDIA (RTX 5070 Ti) working, and the shell out of the way while a game
   runs.
4. Low idle cost. Memory on a gaming desktop is cheap; idle CPU wakeups
   and per-frame GPU work while a game is running are not.

## Compositor

| | Hyprland | niri | Sway | SwayFX | KWin (no Plasma) | labwc |
|---|---|---|---|---|---|---|
| language | C++ | Rust | C | C | C++ | C |
| idle RSS | 100–140 MB | 50–80 MB | 30–60 MB | 40–70 MB | 200–300 MB | ~30 MB |
| VRR | yes | yes | yes | yes | yes | yes |
| tearing | yes | planned | yes | yes | yes | no |
| HDR | yes (0.5x) | no | no | no | yes, best | no |
| direct scanout | yes | yes | yes | yes | yes | yes |
| NVIDIA | fine (explicit sync) | fine | fine | fine | best | fine |
| blur behind layer surfaces | yes | no | no | yes | no | no |
| animations | yes | yes | no | minimal | yes | no |
| IPC / events | rich socket | rich JSON socket | i3 | i3 | D-Bus, scripting | none |
| Xwayland | built in | xwayland-satellite | built in | built in | built in | built in |
| release churn | high (Lua config in 0.55, .conf gone in 0.57) | low | very low | lags Sway | low | low |
| paradigm | dynamic tiling | scrollable tiling | manual tiling | manual tiling | stacking | stacking |

**Hyprland.** It is the only option that gives the glass material (blur
behind layer surfaces) and the animation toolkit while also having the
gaming set: tearing, VRR, direct scanout, HDR, NVIDIA. Its cost is churn.
That is contained by generating the compositor config from the shell, so a
format change is one template, and by pinning the version CachyOS ships.

**Runner-up: niri.** Rust, half the memory, very stable, excellent IPC.
It loses blur (the island would be opaque glass, still fine), tearing
(planned, not shipped), and HDR, and it is scrollable tiling, which is a
different way to work. If churn on Hyprland becomes intolerable, this is
the move, and the shell is written so that the compositor is one service
behind an interface.

**Not chosen.** Sway: no animation or blur, so the design cannot be built
on it. SwayFX: blur but trails Sway and has a small team. KWin without
Plasma: best gaming support but 2–3× the memory and a shell-replacement
story that is second class. labwc: stacking, no IPC.

## Shell toolkit

| | Quickshell | Astal (GTK4 + Gjs/TS) | eww | Rust from scratch (iced + layer-shell) | COSMIC panel/applets |
|---|---|---|---|---|---|
| language | QML + JS | TypeScript on GJS | Yuck + shell | Rust | Rust |
| rendering | Qt Quick scenegraph, GPU | GTK4 GPU renderer | GTK3, mostly CPU | wgpu | iced |
| idle RSS, whole shell in one process | 120–200 MB | 100–180 MB | 60–100 MB | 30–70 MB | ~100 MB |
| idle CPU | 0 when static | 0 when static; GC pauses | 0 | 0 | 0 |
| native services | Pipewire, NetworkManager, BlueZ, notifications, MPRIS, UPower, tray, session lock, PAM, polkit, idle-notify, screencopy, toplevels, greetd, Hyprland, i3 | WirePlumber, NetworkManager, BlueZ, notifications, MPRIS, battery, tray, apps, Hyprland, river, auth, greetd, power profiles. No lock, no screencopy | none | none; zbus, pipewire-rs, libpulse etc. by hand | COSMIC only |
| size / spring animation | declarative, first class | manual, libadwaita springs help | poor | manual | manual |
| layer shell + real windows | both | both | layer only | both, via crates | layer |
| hot reload | yes | partial | yes | no | no |
| effort to reach the feature list | weeks | weeks to months | not reachable | months | not customisable |

**Quickshell.** The island is an animation problem, and QML is the best
declarative animation system on Linux. It also ships nearly every service
the feature list needs as a native type, in one process, so there is no
D-Bus glue to write and no helper daemons to start. Memory is the price:
Qt is heavy. On a 32–64 GB gaming desktop that is not a real cost; idle
CPU is what matters and it is zero when nothing moves.

**Runner-up: Rust with iced.** A quarter of the memory and no runtime
surprises, but every service binding is hand-written, animations are hand
rolled, and there is no hot reload. For a bar it would be the choice; for
a full desktop it multiplies the work by three to five.

**Not chosen.** Astal: comparable memory, worse animation story, GJS
garbage collector, and no session lock or screencopy. eww: not a shell.
COSMIC: the applets are bound to cosmic-comp and cosmic-panel.

## The rest

| role | choice | weighed against | why |
|---|---|---|---|
| terminal | kitty (~90 MB) | foot (~15 MB), ghostty (~110 MB), wezterm | GPU rendering, images, remote control from the shell (dropdown, tabs), themes from tokens. foot is the lean alternative if the shell ever needs to shave memory |
| file manager | yazi in kitty; Thunar for drag-drop, trash, mounts | Nautilus (~150 MB + indexer), Dolphin (KDE frameworks) | yazi is the daily tool; Thunar is the lightest GUI that does what a TUI cannot and themes with the GTK3 CSS we ship anyway |
| launcher, switcher, notifications, OSD, lock, idle, polkit agent, power menu | the shell | fuzzel, rofi, mako, dunst, swayosd, hyprlock, hypridle, hyprpolkitagent | each is a surface in the shell's material; the separate tools each bring their own look |
| greeter | greetd + a shell-drawn greeter | SDDM, ly | same material from the login screen; Quickshell speaks greetd natively |
| keyboard profiles | xremap | keyd, kanata | the only one that remaps per device and per app (Hyprland backend), which the Mac/Windows dual-keyboard setup needs |
| keyring | gnome-keyring | KeePassXC secret service, kwallet | unlocks at login through PAM; secret service and ssh agent for every app. KeePassXC is the upgrade if a real password manager is wanted |
| disks | udisks2 | udiskie | events and mount/eject straight from udisksctl; gnome-disk-utility for partitioning |
| monitor | the shell, over /proc, hwmon, nvidia-smi, LACT, CoolerControl; btop for deep dives | Mission Center, psensor | readouts in the shell's material; GPU clocks, power limits and fan curves belong to LACT and CoolerControl, which the shell drives through their APIs rather than reimplementing |
| capture | grim, slurp, wf-recorder, satty, hyprpicker | hyprshot, flameshot, kooha | composable, headless, one toolbar in the shell over all of them |
| clipboard | cliphist + wl-clipboard | copyq | history with images, no GUI of its own |
| night light | hyprsunset | wlsunset, gammastep | Hyprland's own, controllable over hyprctl |
| game mode | gamemode, power-profiles-daemon, gamescope (optional) | — | governor and process priority; gamescope for HDR / upscaling / a Steam Deck-like session |
| big picture | the shell, launching Steam -gamepadui, Heroic, Lutris | gamescope-session | one 10-foot surface over all three; gamescope-session is Steam-only |
| app theming | `theme/render.py` over templates, from the token file | matugen, pywal | one source renders GTK3, GTK4, Qt (qt6ct + Fusion), kitty, yazi, btop, portals, Hyprland; matugen's image-derived palette does not map onto fixed tokens (D16) |
| icons, cursor, fonts | Papirus-Dark, Bibata, Inter + JetBrains Mono | Adwaita, Tela | same icon theme in GTK, Qt and the shell |
| portals | xdg-desktop-portal-hyprland + -gtk | -kde | screencast through Hyprland; GTK file dialog themed by our CSS |

## Idle budget

| process | RSS |
|---|---|
| Hyprland | ~120 MB |
| shell (Quickshell) | ~150 MB |
| xremap, gamemoded, keyring, udisks, portals | ~60 MB together |
| **total** | **~330 MB**, 0% CPU when nothing is animating |

In game mode: blur off, animations off, the island unmapped, fullscreen
window on direct scanout. The compositor and shell are out of the frame
path.
