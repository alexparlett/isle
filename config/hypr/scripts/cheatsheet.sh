#!/usr/bin/env bash
# Keybind cheatsheet — reads the live binds out of Hyprland, so it can never
# drift from binds.lua. Only binds carrying a `description` flag show up.
set -euo pipefail

hyprctl binds -j | jq -r '
    def bit($b): (.modmask / $b | floor) % 2 == 1;

    .[]
    | select((.description // "") != "")
    | ([ (if bit(64) then "SUPER" else empty end),
         (if bit(8)  then "ALT"   else empty end),
         (if bit(4)  then "CTRL"  else empty end),
         (if bit(1)  then "SHIFT" else empty end),
         (if (.key // "") != "" then .key else "code:" + (.keycode | tostring) end)
       ] | join(" + ")) as $keys
    | "\($keys)\t\(.description)"
' | sort -u | awk -F'\t' '{ printf "%-30s %s\n", $1, $2 }' \
  | rofi -dmenu -i -p "Keybinds" \
         -theme-str 'window { width: 46%; } listview { lines: 18; }' \
  >/dev/null
