#!/usr/bin/env bash
# Session picker for the agent Waybar modules (left-click).
#
#   agent-sessions.sh claude
#   agent-sessions.sh codex
#
# Lists live sessions with their state and focuses the terminal running the
# chosen one. Matching a session to a window is best-effort: neither agent
# publishes a window handle, so this matches the project directory name against
# window titles. If nothing matches it says so rather than guessing.
set -uo pipefail

agent="${1:-claude}"
label=$([[ "$agent" == codex ]] && echo Codex || echo "Claude Code")
state_dir="${XDG_RUNTIME_DIR:-/tmp}/hypr-ai/$agent"

if ! compgen -G "$state_dir/*.json" >/dev/null; then
    notify-send -a "$label" "No active sessions"
    exit 0
fi

choice=$(
    jq -rs '
        map(select(type == "object" and .session != null))
        | sort_by(if .state == "attention" then 0 elif .state == "working" then 1 else 2 end)
        | .[]
        | (((.cwd // "") | split("/") | last) // "~") as $project
        | (if   .state == "attention" then "󰀦 needs you"
           elif .state == "working"   then "󰑮 working  "
           else                            "󰒲 idle     " end) as $badge
        | "\($badge)\t\($project)\t\(.cwd // "")"
    ' "$state_dir"/*.json \
    | column -t -s $'\t' \
    | rofi -dmenu -i -p "$label" -theme-str 'window { width: 44%; }'
)

[[ -z "$choice" ]] && exit 0

# Last field is the full path; its basename is the best window-title handle.
project=$(awk '{print $NF}' <<< "$choice" | xargs -r basename)
[[ -z "$project" ]] && exit 0

if hyprctl clients -j | jq -e --arg p "$project" 'any(.[]; (.title // "") | test($p; "i"))' >/dev/null; then
    hyprctl dispatch "hl.dsp.focus({ window = \"title:.*$project.*\" })"
else
    notify-send -a "$label" "No window found for $project" \
        "The session is live, but no window title mentions it."
fi
