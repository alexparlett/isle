#!/usr/bin/env bash
# Screenshots: region / screen / window / edit
#
# Every mode saves a PNG to ~/Pictures/Screenshots and also puts it on the
# clipboard, because you almost always want one or the other and never know
# which in advance.
set -euo pipefail

mode="${1:-region}"
dir="${XDG_PICTURES_DIR:-$HOME/Pictures}/Screenshots"
file="$dir/$(date +%Y-%m-%d_%H-%M-%S).png"
mkdir -p "$dir"

notify() { command -v notify-send >/dev/null && notify-send -a Screenshot "$@"; }

# Freeze nothing, just grab. slurp draws the selection in Catppuccin colours.
SLURP_ARGS=(-b 1e1e2ecc -c cba6f7ff -s cba6f71a -w 2)

case "$mode" in
region)
    grim -g "$(slurp "${SLURP_ARGS[@]}")" "$file"
    ;;
screen)
    grim "$file"
    ;;
window)
    geom=$(hyprctl activewindow -j | jq -r '"\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"')
    grim -g "$geom" "$file"
    ;;
edit)
    # Annotate before saving. swappy writes the final image itself.
    grim -g "$(slurp "${SLURP_ARGS[@]}")" - | swappy -f - -o "$file"
    [[ -f "$file" ]] || exit 0 # user closed swappy without saving
    ;;
*)
    echo "usage: ${0##*/} {region|screen|window|edit}" >&2
    exit 1
    ;;
esac

wl-copy < "$file"
notify "Saved and copied" "${file/#$HOME/\~}" -i "$file"
