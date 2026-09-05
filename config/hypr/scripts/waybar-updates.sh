#!/usr/bin/env bash
# Waybar module: pending system updates.
#
# Uses checkupdates (pacman-contrib) for repo packages and paru for the AUR.
# Neither touches the pacman database lock, so this is safe to poll.
# Emits Waybar JSON: text, tooltip, class.
set -uo pipefail

repo=$(checkupdates 2>/dev/null || true)
aur=$(paru -Qua 2>/dev/null || true)

repo_n=$([[ -n "$repo" ]] && wc -l <<< "$repo" || echo 0)
aur_n=$([[ -n "$aur" ]] && wc -l <<< "$aur" || echo 0)
total=$((repo_n + aur_n))

if ((total == 0)); then
    printf '{"text":"","tooltip":"System is up to date","class":"updated","alt":"updated"}\n'
    exit 0
fi

tooltip="$total update(s) pending"
((repo_n)) && tooltip+=$'\n\n<b>Repo</b>\n'"$(head -n 25 <<< "$repo")"
((aur_n)) && tooltip+=$'\n\n<b>AUR</b>\n'"$(head -n 15 <<< "$aur")"
((total > 40)) && tooltip+=$'\n…'

# Escape for JSON, then for Pango markup in the tooltip.
tooltip=${tooltip//&/&amp;}
tooltip=$(jq -Rs . <<< "$tooltip")

printf '{"text":"%s","tooltip":%s,"class":"pending","alt":"pending"}\n' "$total" "$tooltip"
