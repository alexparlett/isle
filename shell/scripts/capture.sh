#!/usr/bin/env bash
# Overlay-based capture, launched by the compositor. slurp and hyprpicker create wlr-layer
# surfaces, which a window spawned by Quickshell (a Wayland client) cannot map; a compositor
# child can. Results go back to the shell over IPC.
#
# A shot is taken before the pick: every monitor is grabbed to a frozen frame, the shell shows
# those frames under the pick, and the region is cut out of them. Nothing that happens on the
# screen after the key press reaches the shot, and the pick's own overlay never can.
#
#   capture.sh shoot  MODE FILE [-c]     region/window/screen shot, copied
#   capture.sh pick                      colour picker
#   capture.sh geom   MODE OUT           write the region geometry to OUT (for recording)
set -u
# What went wrong in a pick lands here, since a compositor child has nowhere else to say it.
exec 2>>"${XDG_RUNTIME_DIR:-/tmp}/isle-capture.log"
DIR="$(dirname "$(readlink -f "$0")")"
QS="${ISLE_SHELL_PATH:-$HOME/.config/quickshell/isle}"
ipc() { qs -p "$QS" ipc call capture "$@" >/dev/null 2>&1; }
SLURP=(slurp -b 0A0B0DA6 -c 7FA6FFFF -s 7FA6FF22 -w 2 -d)

FREEZE="${XDG_RUNTIME_DIR:-/tmp}/isle-freeze"

freeze() { # freeze [-c]: one PNG per monitor plus index.json
    rm -rf "$FREEZE"; mkdir -p "$FREEZE"
    hyprctl -j monitors | python3 -c '
import json, sys
out = []
for m in json.load(sys.stdin):
    s = m["scale"] or 1
    out.append({"name": m["name"], "x": m["x"], "y": m["y"], "w": int(m["width"] / s), "h": int(m["height"] / s), "scale": s, "file": m["name"] + ".png"})
json.dump(out, open(sys.argv[1] + "/index.json", "w"))
print("\n".join(m["name"] for m in out))' "$FREEZE" | while read -r name; do grim ${1:-} -o "$name" "$FREEZE/$name.png"; done
}

geometry() { # geometry MODE
    case "$1" in
        screen) python3 "$DIR/geometry.py" screen ;;
        window) python3 "$DIR/geometry.py" windows | "${SLURP[@]}" -r ;;
        *)      "${SLURP[@]}" ;;
    esac
}

case "${1:-}" in
    shoot)
        MODE="$2"; FILE="$3"; CURSOR="${4:-}"
        freeze $CURSOR
        [ "$MODE" != screen ] && ipc frozen "$FREEZE"
        G=$(geometry "$MODE"); RC=$?
        ipc thaw
        { [ $RC -eq 0 ] && [ -n "$G" ]; } || { ipc cancelled; exit 0; }
        if python3 "$DIR/crop.py" "$FREEZE" "$G" "$FILE"; then wl-copy < "$FILE"; ipc saved "$FILE"; else ipc cancelled; fi
        ;;
    pick)
        C=$(hyprpicker -a -f hex) && [ -n "$C" ] && ipc picked "$C" ;;
    geom)
        MODE="$2"; OUT="$3"
        G=$(geometry "$MODE") || exit 0
        [ -n "$G" ] || exit 0
        printf '%s' "$G" > "$OUT"; ipc recordgeom "$OUT" ;;
esac
