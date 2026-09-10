#!/usr/bin/env bash
#
# Make the guest photographable. Runs INSIDE the VM, idempotent:
#
#     ssh -p 2222 user@localhost /repo/tools/guest-baseline.sh
#
# Everything here exists because a shot came back wrong, not because it seemed
# tidy. Run it, then `tools/test-vm.sh stop && save` so the baseline snapshot
# carries it.
#
#   * Hyprland draws a persistent warning bar across the TOP of the screen for
#     the .conf deprecation and for being started without `start-hyprland`.
#     The island lives top centre. Every shot of the surface the whole design
#     rests on had two orange banners through it.
#
#   * No wallpaper was ever up, so all four backdrops produced the same black
#     frame. A shell only ever shot over black has not been shot over
#     `bright`, which is the one that breaks shells. hyprpaper 0.8.4 turns out
#     not to work here at all — it finds the output and then declines to make a
#     wallpaper for it — so the backdrop is drawn by the harness instead. See
#     tools/shot/backdrop/shell.qml.

set -uo pipefail
ok() { printf '  \e[32m✓\e[0m %s\n' "$*"; }

# The compositor config under test, and the Quickshell config directory every
# keybind in it addresses. The guest runs both straight off the 9p mount.
HYPR_CONFIG=/repo/hypr/hyprland.lua
SHELL_PATH=/repo/shell

# --- wallpaper daemon ---------------------------------------------------------

mkdir -p ~/.config/hypr ~/wallpapers
# The four backdrops live in the guest so a shot is one scp of one file, not
# four every time.
if [[ -d /repo/tools/shot/wallpapers ]]; then
    cp /repo/tools/shot/wallpapers/*.jpg ~/wallpapers/ 2>/dev/null
    ok "backdrops copied to ~/wallpapers"
fi
rm -f ~/.config/hypr/hyprpaper.conf

# --- packages -----------------------------------------------------------------
#
# The guest was found missing 35 of packages/shell.txt's 78, `mpv` among them,
# which a sheet's verification needs. The install is the list, not a name: a
# package a slice needs is added to packages/shell.txt and lands here.

# ONE UNRESOLVABLE NAME MUST NOT COST THE OTHER SEVENTY-SEVEN. pacman fails a
# whole transaction on a target it cannot find, and this guest cannot find
# `cachy-update`, so the single call installed nothing at all and reported one
# line about a package no slice before 10 needs. The list goes in one
# transaction, and a failure retries per package so what is installable lands
# and what is not is named.
if [[ -r /repo/packages/shell.txt ]]; then
    mapfile -t pkgs < <(sed 's/#.*//' /repo/packages/shell.txt | tr -d ' ' | grep -v '^$')
    missing=()
    for p in "${pkgs[@]}"; do
        pacman -Qq "$p" >/dev/null 2>&1 || missing+=("$p")
    done
    if (( ${#missing[@]} == 0 )); then
        ok "all ${#pkgs[@]} packages from packages/shell.txt present"
    elif sudo -n pacman -S --needed --noconfirm "${missing[@]}" >/tmp/guest-packages.log 2>&1; then
        ok "installed ${#missing[@]} missing package(s) from packages/shell.txt"
    else
        unresolved=()
        for p in "${missing[@]}"; do
            sudo -n pacman -S --needed --noconfirm "$p" >>/tmp/guest-packages.log 2>&1 \
                || unresolved+=("$p")
        done
        if (( ${#unresolved[@]} == 0 )); then
            ok "installed ${#missing[@]} missing package(s) from packages/shell.txt"
        else
            printf '  \e[33m!\e[0m %d package(s) pacman cannot resolve here: %s\n' \
                   "${#unresolved[@]}" "${unresolved[*]}" >&2
        fi
    fi
fi

# --- audio devices ------------------------------------------------------------
#
# The emulated ac97 card is the only sound hardware here and its name and
# volume are the machine's rather than the harness's. A PipeWire null sink and
# null source, loaded from the user's own config at every boot, give the audio
# slice two endpoints with names a gate can assert on and a level nothing else
# moves.
#
# media.class is `Audio/Source`, NOT `Audio/Source/Virtual`. Quickshell reads
# a virtual source as PwNodeType.Untracked with a null `audio` interface, so
# `sourceMuted` had nothing behind it and `toggleSourceMute()` reached nothing.

mkdir -p ~/.config/pipewire/pipewire.conf.d
cat > ~/.config/pipewire/pipewire.conf.d/10-null-devices.conf <<'PWCONF'
context.objects = [
    {   factory = adapter
        args = {
            factory.name     = support.null-audio-sink
            node.name        = "null-sink"
            node.description = "Null Output"
            media.class      = Audio/Sink
            object.linger    = true
            audio.position   = [ FL FR ]
        }
    }
    {   factory = adapter
        args = {
            factory.name     = support.null-audio-sink
            node.name        = "null-source"
            node.description = "Null Input"
            media.class      = Audio/Source
            object.linger    = true
            audio.position   = [ FL FR ]
        }
    }
]
PWCONF
ok "null sink and null source declared in ~/.config/pipewire/pipewire.conf.d"

export XDG_RUNTIME_DIR=/run/user/$(id -u)
systemctl --user restart pipewire pipewire-pulse wireplumber
sleep 3

# The defaults and the level are wireplumber state, which persists, so a boot
# comes up on the null devices at the same volume every time.
#
# `wpctl set-volume 0.4` writes channelVolumes 0.064 — the cubic curve — and
# Quickshell reports the cubic value back, so PwNodeAudio.volume reads 0.4.
null_sink=$(pw-dump | jq -r '.[] | select(.info.props."node.name"=="null-sink") | .id' | head -1)
null_source=$(pw-dump | jq -r '.[] | select(.info.props."node.name"=="null-source") | .id' | head -1)
if [[ -n "$null_sink" && -n "$null_source" ]]; then
    wpctl set-default "$null_sink"
    wpctl set-default "$null_source"
    wpctl set-volume "$null_sink" 0.4
    wpctl set-mute "$null_sink" 0
    wpctl set-mute "$null_source" 0
    ok "null-sink and null-source are the defaults, output at 0.4"
else
    printf '  \e[31m✗\e[0m the null devices did not appear in pw-dump\n' >&2
fi

# --- the compositor config ----------------------------------------------------
#
# The guest boots on the repo's own hypr/hyprland.lua. It no longer writes a
# hyprland.conf of its own, so what the VM runs is what ships: the gaming
# settings, the window and layer rules, the animation curves and the whole
# chord table are under test rather than approximated.
#
# start-hyprland passes everything after `--` to Hyprland, and HYPR_SHELL_PATH
# has to be in the compositor's environment before it starts, because both the
# startup command and every surface keybind read it.

rm -f ~/.config/hypr/hyprland.conf

cat > ~/.bash_profile <<PROFILE
[[ -f ~/.bashrc ]] && . ~/.bashrc
# Start the compositor on the first VT only, so ssh sessions are unaffected.
if [[ -z \$WAYLAND_DISPLAY && \$XDG_VTNR -eq 1 ]]; then
    export HYPR_SHELL_PATH=$SHELL_PATH
    exec start-hyprland -- -c $HYPR_CONFIG
fi
PROFILE
ok "login starts Hyprland on $HYPR_CONFIG with HYPR_SHELL_PATH=$SHELL_PATH"

# --- the shell's systemd user unit -------------------------------------------
#
# Wave 11 owns installing this properly. The baseline carries it so that
# `systemctl --user restart hypr-shell` is testable in the guest at all, and so
# a reset does not silently remove the only thing that proves the unit works.
#
# The drop-in is where the guest's own path lives: the unit ships with
# HYPR_SHELL_PATH=~/.config/quickshell/hypr-shell and the guest runs the shell
# straight off the 9p mount instead.

mkdir -p ~/.config/systemd/user/hypr-shell.service.d
install -m 644 /repo/systemd/hypr-shell.service ~/.config/systemd/user/hypr-shell.service
cat > ~/.config/systemd/user/hypr-shell.service.d/10-repo.conf <<'DROPIN'
[Service]
Environment=HYPR_SHELL_PATH=/repo/shell
DROPIN
ok "hypr-shell.service installed with the /repo drop-in"

# Installed, NOT enabled and NOT started. The unit has Restart=always, so a
# running one would race tools/shot/shot.sh: the harness kills the shell
# between shots and systemd would bring a second one straight back up. The unit
# is here to be started by hand when it is what is under test.
systemctl --user disable hypr-shell.service >/dev/null 2>&1
systemctl --user stop hypr-shell.service    >/dev/null 2>&1

# --- apply to the running session, so this need not wait for a reboot --------

export XDG_RUNTIME_DIR=/run/user/$(id -u)
export HYPRLAND_INSTANCE_SIGNATURE=$(ls "$XDG_RUNTIME_DIR/hypr" 2>/dev/null | head -1)

# DISCOVERED, NOT INHERITED, AND THIS IS THE LINE THAT WAS WRONG. This script
# runs over ssh, and an ssh shell has no WAYLAND_DISPLAY. The old code said
#
#     systemctl --user import-environment WAYLAND_DISPLAY ...
#
# three lines below a comment describing precisely the failure it was there to
# prevent — and import-environment imported what this shell had, which was
# nothing, leaving the user manager holding WAYLAND_DISPLAY as the EMPTY
# STRING. `systemctl --user show-environment` said `WAYLAND_DISPLAY=` and every
# unit started from it inherited an empty display: Quickshell logged "Failed to
# create wl_display (Connection refused)", could not load the wayland platform
# plugin, could not create WlrLayershell's attached properties, and fell back
# to XWayland as a floating window with no layer surface at all. The unit
# reported active throughout.
#
# tools/shot/shot.sh had had the right answer all along, and this is it: the
# socket is a file in XDG_RUNTIME_DIR, so look for it.
export WAYLAND_DISPLAY=$(ls "$XDG_RUNTIME_DIR" 2>/dev/null | grep -m1 '^wayland-[0-9]*$')

if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
    hyprctl reload >/dev/null 2>&1 && ok "reloaded"
    pkill -x hyprpaper 2>/dev/null && ok "stopped the idle hyprpaper"

    if [[ -z "${WAYLAND_DISPLAY:-}" ]]; then
        # Loud, because silence here is what round 1 shipped.
        printf '  \e[31m✗\e[0m no wayland-N socket in %s — the user manager will\n' \
               "$XDG_RUNTIME_DIR" >&2
        printf '    not be able to start the shell. Is Hyprland running?\n' >&2
    else
        # set-environment, not import-environment: this hands over VALUES this
        # script worked out, rather than whatever an ssh shell happened to
        # have. hypr/hyprland.lua does the same job at login on a real machine.
        systemctl --user set-environment \
            "WAYLAND_DISPLAY=$WAYLAND_DISPLAY" \
            "HYPRLAND_INSTANCE_SIGNATURE=$HYPRLAND_INSTANCE_SIGNATURE" \
            "XDG_CURRENT_DESKTOP=Hyprland" \
            "XDG_SESSION_TYPE=wayland" 2>/dev/null
        systemctl --user daemon-reload 2>/dev/null
        ok "session environment handed to the user manager (WAYLAND_DISPLAY=$WAYLAND_DISPLAY)"
    fi
fi
