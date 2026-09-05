#!/usr/bin/env bash
# Jump to whichever agent session is blocked waiting on you.
#
# Bound to ⌘⌥⇧A. With one blocked session it goes straight there; with several
# it offers the list. Matching a session to a window is best-effort — neither
# agent publishes a window handle — so it matches the project directory name
# against window titles.
set -uo pipefail

runtime="${XDG_RUNTIME_DIR:-/tmp}/hypr-ai"

blocked=()
for agent in claude codex; do
    dir="$runtime/$agent"
    compgen -G "$dir/*.json" >/dev/null || continue
    while IFS= read -r line; do
        [[ -n "$line" ]] && blocked+=("$line")
    done < <(jq -r --arg a "$agent" '
        select(.state == "attention")
        | "\($a)\t\((.cwd // "") | split("/") | last)\t\(.reason // "waiting")"
    ' "$dir"/*.json 2>/dev/null)
done

if ((${#blocked[@]} == 0)); then
    notify-send -a "Agents" "Nothing waiting" "No session is blocked on you."
    exit 0
fi

if ((${#blocked[@]} == 1)); then
    choice="${blocked[0]}"
else
    choice=$(printf '%s\n' "${blocked[@]}" \
        | awk -F'\t' '{ printf "%-8s %-22s %s\n", $1, $2, $3 }' \
        | rofi -dmenu -i -p "Waiting on you" -theme-str 'window { width: 46%; }')
    [[ -z "$choice" ]] && exit 0
fi

project=$(awk '{print $2}' <<< "$choice")
[[ -z "$project" ]] && exit 0

if hyprctl clients -j | jq -e --arg p "$project" 'any(.[]; (.title // "") | test($p; "i"))' >/dev/null; then
    hyprctl dispatch "hl.dsp.focus({ window = \"title:.*$project.*\" })"
else
    notify-send -a "Agents" "$project needs you" \
        "Could not find its window — no title mentions the project."
fi
