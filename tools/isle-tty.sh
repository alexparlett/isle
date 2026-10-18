#!/usr/bin/env bash
#
# Run the new Isle built in this checkout on another virtual terminal, in a logind session
# of its own, beside the desktop you run it from, and come back when it ends.
#
#     tools/isle-tty.sh                  build (release) and run on tty8
#     tools/isle-tty.sh --vt 9           another terminal
#     tools/isle-tty.sh --minutes 30     come back here after thirty minutes and ask Isle to quit
#     tools/isle-tty.sh --debug          the debug build
#     tools/isle-tty.sh --no-build       what is already built
#     tools/isle-tty.sh --trace-planes   log why each frame's planes were or were not used
#                                        (Smithay's own reasons; a lot of log, so a short run)
#     tools/isle-tty.sh --back           from anywhere (another terminal, ssh): back to this
#                                        desktop now, and Isle asked to quit
#
# Inside Isle, `isle quit` (the checkout's target directory is on PATH) ends the run and the
# console comes back here. Coming back needs no password and no keys: a Mac keyboard has no
# Ctrl+Alt+F-key chord, so the console is switched through logind.
#
# xremap is stopped for the run, and the user manager's environment points at Isle; both
# are put back when it ends. Its log is ~/.local/state/isle/session.log.

set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

vt=8
minutes=
profile=release
build=1
back_only=0
log=info
while (($#)); do
    case "$1" in
        --vt) vt="$2"; shift ;;
        --minutes) minutes="$2"; shift ;;
        --debug) profile=debug ;;
        --no-build) build=0 ;;
        --back) back_only=1 ;;
        --trace-planes) log="info,smithay::backend::drm::compositor=trace" ;;
        -h|--help) sed -n '3,21p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "isle-tty: unknown argument $1" >&2; exit 2 ;;
    esac
    shift
done

bin="$REPO/target/$profile"
unit="isle-tty$vt"
# `isle quit` reaches Isle on the user's bus, which an ssh login may not name.
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=/run/user/$(id -u)/bus}"
# Where to come back to: this user's session on another terminal of the seat.
home_session() {
    loginctl list-sessions --no-legend | awk -v u="$(id -un)" -v vt="tty$vt" \
        '$3 == u && $4 == "seat0" && $7 ~ /^tty/ && $7 != vt { print $1, substr($7, 4); exit }'
}
isle_session() {
    loginctl list-sessions --no-legend | awk -v vt="tty$vt" '$7 == vt { print $1; exit }'
}

# Back to the desktop first, which needs nothing from Isle, then Isle asked to end.
go_back() {
    local id home_vt
    read -r id home_vt <<<"$(home_session)" || true
    if [[ -n "${id:-}" ]]; then
        # logind lets a session's own user switch to it; from outside the seat (ssh) only
        # root can, so chvt is tried without a password prompt.
        loginctl activate "$id" 2>/dev/null || sudo -n chvt "$home_vt" 2>/dev/null || true
    fi
    if systemctl is-active --quiet "$unit"; then
        "$bin/isle" quit 2>/dev/null || true
        for _ in $(seq 1 30); do
            systemctl is-active --quiet "$unit" || break
            sleep 0.5
        done
    fi
}

if ((back_only)); then
    go_back
    if systemctl is-active --quiet "$unit"; then
        echo "isle-tty: Isle did not quit; \`sudo systemctl stop $unit\` ends it" >&2
    fi
    exit 0
fi

if ((build)); then
    flag=()
    [[ $profile == release ]] && flag=(--release)
    cargo build "${flag[@]}" --manifest-path "$REPO/Cargo.toml" \
        -p isle-session -p isle-compositor -p isle-cli -p isle-auth -p isle-record \
        -p isle-system-settings
fi
for exe in isle-session isle-compositor isle isle-auth; do
    [[ -x "$bin/$exe" ]] || { echo "isle-tty: $bin/$exe is not built" >&2; exit 1; }
done
if systemctl is-active --quiet "$unit"; then
    echo "isle-tty: $unit is already running; \`tools/isle-tty.sh --back\` ends it" >&2
    exit 1
fi

xremap_was_active=0
systemctl --user is-active --quiet xremap.service && xremap_was_active=1

# What this desktop's apps need from the user manager, to give back after.
keep=(WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE XDG_SESSION_DESKTOP
      HYPRLAND_INSTANCE_SIGNATURE QT_QPA_PLATFORM QT_QPA_PLATFORMTHEME GDK_BACKEND)

come_back() {
    go_back
    present=()
    for name in "${keep[@]}"; do
        [[ -n "${!name:-}" ]] && present+=("$name")
    done
    ((${#present[@]})) && dbus-update-activation-environment --systemd "${present[@]}" || true
    ((xremap_was_active)) && systemctl --user start xremap.service || true
    if systemctl is-active --quiet "$unit"; then
        echo "isle-tty: Isle is still running on tty$vt; \`sudo systemctl stop $unit\` ends it"
    fi
    echo "isle-tty: back; the log is ~/.local/state/isle/session.log"
}
trap come_back EXIT
trap 'exit 130' INT TERM

# xremap grabs the keyboards and rewrites Cmd chords before Isle sees them.
((xremap_was_active)) && systemctl --user stop xremap.service

echo "isle-tty: Isle ($profile) on tty$vt; \`isle quit\` inside it, or \`tools/isle-tty.sh --back\`, ends it"
# A unit, not a login: PAM's `login` stack and the terminal make it a logind session on
# that seat, which libseat needs to open the GPU and input. Left alone, the session runs with
# CAP_WAKE_ALARM in its ambient set, which every app inherits and bwrap (Steam's runtime,
# Flatpak) refuses as "unexpected capabilities".
sudo systemd-run --unit="$unit" --collect \
    --uid="$(id -u)" --gid="$(id -g)" \
    -p PAMName=login \
    -p AmbientCapabilities= -p CapabilityBoundingSet=~CAP_WAKE_ALARM \
    -p TTYPath="/dev/tty$vt" -p StandardInput=tty -p StandardOutput=journal -p StandardError=journal \
    -p TTYReset=yes -p TTYVHangup=yes -p TTYVTDisallocate=yes \
    -p UtmpIdentifier="tty$vt" -p UtmpMode=user \
    --working-directory="$HOME" \
    --setenv=XDG_SESSION_TYPE=wayland \
    --setenv=XDG_SESSION_CLASS=user \
    --setenv=XDG_SESSION_DESKTOP=isle \
    --setenv=XDG_CURRENT_DESKTOP=Isle \
    --setenv=PATH="$bin:/usr/local/bin:/usr/bin" \
    --setenv=ISLE_COMPOSITOR="$bin/isle-compositor" \
    --setenv=RUST_LOG="$log" \
    "$bin/isle-session"

session=
for _ in $(seq 1 20); do
    session="$(isle_session)"
    [[ -n "$session" ]] && break
    sleep 0.25
done
if [[ -z "$session" ]] || ! loginctl activate "$session" 2>/dev/null; then
    sudo chvt "$vt"
fi

deadline=
[[ -n "$minutes" ]] && deadline=$((SECONDS + minutes * 60))
while systemctl is-active --quiet "$unit"; do
    if [[ -n "$deadline" ]] && ((SECONDS >= deadline)); then
        echo "isle-tty: $minutes minutes are up"
        break
    fi
    sleep 1
done
