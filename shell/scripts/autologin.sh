#!/usr/bin/env bash
# Automatic login through greetd: an initial session started as USER at boot, or none.
#     autologin.sh on USER | off
# Run as root; the shell runs it through pkexec.
set -euo pipefail
CONF=/etc/greetd/config.toml
[[ -f $CONF ]] || { echo "no greetd configuration" >&2; exit 1; }
# Everything but the initial_session block stays as it is.
rest=$(awk 'BEGIN{skip=0} /^\[initial_session\]/{skip=1; next} /^\[/{skip=0} !skip' "$CONF")
case "${1:-}" in
    on)
        user="${2:?user}"
        printf '%s\n\n[initial_session]\ncommand = "start-hyprland"\nuser = "%s"\n' "$(printf '%s' "$rest" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}')" "$user" > "$CONF" ;;
    off)
        printf '%s\n' "$(printf '%s' "$rest" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}')" > "$CONF" ;;
    *) echo "autologin.sh on USER | off" >&2; exit 2 ;;
esac
