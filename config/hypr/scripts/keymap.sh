#!/usr/bin/env bash
# Switch the keybinding profile.
#
#     keymap.sh              print the current one
#     keymap.sh mac
#     keymap.sh windows
#     keymap.sh toggle
#
# Writes ~/.config/hypr/keymap and reloads Hyprland. Binds are registered while
# the config loads, so a reload is what actually swaps them over.
set -uo pipefail

file="$HOME/.config/hypr/keymap"
current=$(cat "$file" 2>/dev/null | tr -d '[:space:]')
current=${current:-mac}

case "${1:-}" in
"")
    echo "$current"
    exit 0
    ;;
mac | windows)
    target="$1"
    ;;
toggle)
    [[ "$current" == "mac" ]] && target=windows || target=mac
    ;;
*)
    echo "usage: ${0##*/} {mac|windows|toggle}" >&2
    exit 1
    ;;
esac

if [[ "$target" == "$current" ]]; then
    notify-send -a "Keymap" "Already using the $target profile" 2>/dev/null
    exit 0
fi

mkdir -p "$(dirname "$file")"
printf '%s\n' "$target" > "$file"

if hyprctl reload >/dev/null 2>&1; then
    notify-send -a "Keymap" "Switched to the ${target} profile" \
        "$( [[ $target == windows ]] \
            && echo 'Win+arrows snap · Alt+Tab · Alt+F4 · Win+Shift+S' \
            || echo '⌘Space launcher · ⌃⌘F fullscreen · ⌘⇧4 region shot' )" 2>/dev/null
    echo "switched to $target"
else
    echo "wrote $file — reload Hyprland to apply"
fi
