# Decisions

Numbered so code comments can cite them: `// alpha on the effect: D3`.
Each entry is what was decided and why, in a few lines. Measurements and
history go here, not in comments.

**D1 · Hyprland + Quickshell.** Chosen on merit over niri, Sway, KWin
and over Astal, eww and a Rust shell: the only pairing with blur behind
layer surfaces, declarative animation, the gaming set (VRR, tearing,
direct scanout, HDR, NVIDIA) and native service types in one process.
Cost accepted: ~330 MB idle, Hyprland's release churn. Mitigation: the
compositor config is generated, and the compositor is reached through one
service file so niri stays a possible move. Full table in STACK.md.

**D2 · The island is the only persistent surface.** Top centre of the
focused monitor. Other monitors show nothing. Everything else appears on
intent and leaves on its own.

**D3 · Hover unfolds the island into the control panel.** Two hover
states were drawn and rejected: a 64px pill with sliders and buttons
inside it (too dense to read at a glance) and a set of lighter pills that
only enlarged what rest already showed (not worth a hover). The pill now
unfolds downward into a 420px panel: header band, toggles, output and
sliders, media, footer. Sliders and toggles live there and nowhere else.

**D4 · The dashboard replaces the control centre and notification
centre.** One full-screen surface on the control centre keybinding, a
12-column grid of widgets. Notifications are a widget in it. The island's
clock and bell open it too.

**D5 · Widgets are directories with a manifest and one QML file.** The
dashboard draws the card chrome; a widget draws only its content and
declares its sizes and requirements. Third-party widgets drop into
`~/.config/isle/widgets/`. Polled readouts use a command source so a
shell script is enough. Layout is a preference.

**D6 · Default widgets.** Clock, Media, Workspaces, Notifications,
Controls, System, Agents, Session, Devices, Storage. Available but off:
Network, GPU, Games, Clipboard, Timer, Updates, Weather.

**D7 · One bind set on SUPER; xremap shapes it per device.** Hyprland
never learns which keyboard sent a chord. The Mac profile turns Cmd+key
into Ctrl+key for apps except the chords the shell owns, and moves
workspace switching to Ctrl+digit and Ctrl+arrows because Cmd+digit must
reach browsers. Table in ARCHITECTURE.md.

**D8 · Material: dark glass, 78% at blur 24, hairline 8%, accent
`#7FA6FF`.** Two alternates (warm amber, opaque ink) are kept on the
canvas and are a token swap if wanted later. Game mode drops blur and
raises glass to 94%.

**D9 · Controller input is keyboard input.** The Steam Controller's
lizard mode and Steam Input both present as a keyboard and mouse, so
every controller-driven surface is keyboard-navigable with a focus ring
and no gamepad API is needed.

**D10 · Files: yazi first, Thunar for what a TUI cannot do.** Nautilus
was rejected for its indexer; Dolphin for the KDE frameworks it drags in.

**D11 · Hyprland 0.56 facts the code relies on.** Dispatchers over IPC
are Lua expressions (`hl.dsp.focus({ workspace = 2 })`), not the old
`workspace 2` strings, so `Compositor` builds Lua. Gap values are
`{ top, right, bottom, left }` tables; an array is read as zeros. The
example config and the API stubs ship in `/usr/share/hypr/`.

**D12 · Glyphs are SVG data URLs, not effect-tinted images.** The
vendored Lucide bodies live in `shell/assets/Icons.js`; `Glyph` builds
an SVG with the colour baked in and the token's alpha as `opacity`. No
shader effect, no extra Qt module, exact stroke colour. SVG rejects
Qt's `#AARRGGBB` form, which is why the alpha travels separately.

**D13 · The VM's memory numbers are not the baseline.** Under llvmpipe
the guest reports ~320 MB for Hyprland and ~440 MB for the shell,
inflated by software GL buffers. The idle budget in STACK.md is measured
on the real machine when the shell first runs there.

**D14 · Widgets load through the `qs:` scheme.** Quickshell serves the
config as `qs:@/qs/...`; a component loaded by `file://` URL cannot
import `qs.services` or `qs.ui`, and nested modules (`qs.widgets`) and
relative directory imports do not resolve under that scheme. So
`WidgetBase` lives in `qs.ui`, built-ins load via `Qt.resolvedUrl`
from inside the config, and third-party widgets by path cannot yet see
the services. Open: hand them a `services` object from the card.

**D15 · Never restart the shell while the session is locked.** The
shell is the locker; if its process dies under an active lock, Hyprland
shows its "lockscreen app died" screen, and lock requests from a fresh
shell fail with a protocol error until `hyprctl eval
'hl.clear_crashed_lockscreen()'` runs and, in practice, the compositor
restarts. The unit's `Restart=always` is right for every other crash;
a lock-time crash is the one to engineer out, not restart through.

**D16 · The app themes are rendered in-repo, not by matugen.** matugen
derives a Material palette from an image and templates against that
palette's names; our tokens are fixed values with our own names. A
sixty-line renderer (`theme/render.py`) substitutes tokens into
templates for GTK 3 and 4 (with adw-gtk3 as the GTK 3 bridge), qt6ct's
palette, kitty, yazi, btop, the portal config and the Hyprland theme
fragment, then sets the gsettings keys libadwaita reads. The shell
re-runs it when the accent changes.

**D17 · The shell runs in the session scope, not a user unit.** A
systemd user service belongs to no logind session, and polkit's
implicit policies key on that: from the unit, NetworkManager refuses
Wi-Fi toggles outright, udisks demands an admin password to mount a USB
stick, and logind demands one to suspend. Started from the compositor's
start hook, the process sits in the session scope and all three are
allowed. `shell/scripts/isle-session` provides the restart the unit used
to; xremap stays a unit because it needs nothing from polkit.

**D18 · Idle monitors are recreated, not retuned.** An `IdleMonitor`
whose `timeout` changes while it is enabled never fires again, and one
created with a timeout of 0 fires at once, which is how the shell locked
itself at startup while a preference was still loading. Each idle policy
is a Loader that builds its monitor with the final timeout and rebuilds
it when the preference or game mode changes; 0 means no monitor.

**D19 · Mission Control is drawn by the shell, not a compositor plugin.**
The plugins in this space, hyprexpo, Hyprspace and hyprtasking, are
exposés of workspaces: each workspace a scaled tile with its windows in
place, in the plugin's own colours and borders, and the only one still
tracking this Hyprland (hyprtasking, pinned to 0.56.2) has no spread of
windows, no title or close under the pointer, and no hidden-window tray.
macOS's Mission Control is itself drawn from window snapshots, which is
what the shell already has through `ScreencopyView` on each toplevel, and
the surface then shares the shell's material, motion, keyboard and drag
handling with everything else. A plugin would buy animating the real
windows, which snapshots animated from their real rectangles give as
well. Gestures come from the compositor's `hl.gesture`, which runs a
function.


**D20 · The Proton sign-in is the CLI fed over stdin, not a terminal.** The
CLI reads the password and the 2FA code with `getpass`, which falls back to
stdin when it has no terminal; run under `setsid` it never has one, so the
shell's own fields (as for Wi-Fi passwords) hand it the secret directly
and the code when it asks. The alternative, opening a terminal for the
prompt, kept the password out of the shell but put a stray window in the
flow.

**D21 · The desktop runs from an installed copy, not the checkout.** With
`~/.config` linked straight into a working checkout, every edit and every
pull was live at once, and a half-finished change or a broken file took the
running desktop down with it. `tools/install.sh` now copies the checkout to
`~/.local/share/isle` and links into that; the VM mounts the checkout for
trying things first. Copy rather than clone, so uncommitted work installs
too when it is wanted, and the copy keeps its git history for Settings ›
Updates.
