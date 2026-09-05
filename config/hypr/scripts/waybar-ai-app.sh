#!/usr/bin/env bash
# Waybar module: Claude / ChatGPT desktop app presence.
#
#   waybar-ai-app.sh claude
#   waybar-ai-app.sh chatgpt
#
# Neither app exposes task state to the outside world, so this reports only
# what Hyprland can actually see: whether the app is running, and whether it
# has raised an urgency hint (which is what an app does when it wants your
# attention — a finished reply, usually). For real task-level detail on the
# Claude Code CLI, see waybar-claude.sh, which is hook-driven.
set -uo pipefail

case "${1:-}" in
claude)  class="com.anthropic.Claude"; icon="󰛄" ;;
chatgpt) class="chatgpt";              icon="󰭹" ;;
*) printf '{"text":"","class":"none"}\n'; exit 0 ;;
esac

hyprctl clients -j 2>/dev/null | jq -r --arg c "$class" --arg icon "$icon" '
    map(select((.class // "") | ascii_downcase == ($c | ascii_downcase))) as $wins
    | if ($wins | length) == 0 then
        { text: "", tooltip: "Not running", class: "none" }
      elif ($wins | map(.urgent // false) | any) then
        { text: $icon, tooltip: "Wants your attention", class: "attention" }
      else
        { text: $icon, tooltip: "Running", class: "running" }
      end
' || printf '{"text":"","class":"none"}\n'
