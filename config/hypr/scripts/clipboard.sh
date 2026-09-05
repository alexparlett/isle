#!/usr/bin/env bash
# Clipboard history picker — cliphist through rofi.
#
# Delete one entry with Shift+Delete, wipe the whole history with Ctrl+Delete.
set -uo pipefail

# No `set -e`: rofi exits non-zero for the custom keybinds, and that is the
# whole point of them.
choice=$(
    cliphist list | rofi -dmenu \
        -p "Clipboard" \
        -i \
        -kb-custom-1 "Shift+Delete" \
        -kb-custom-2 "Control+Delete" \
        -theme-str 'window { width: 45%; }'
)
status=$?

case "$status" in
    0)  printf '%s' "$choice" | cliphist decode | wl-copy ;;
    10) printf '%s' "$choice" | cliphist delete ;;
    11) cliphist wipe && notify-send -a Clipboard "Clipboard history cleared" ;;
    *)  exit 0 ;; # cancelled
esac
