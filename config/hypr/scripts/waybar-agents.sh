#!/usr/bin/env bash
# Waybar module: coding-agent session status.
#
#   waybar-agents.sh claude
#   waybar-agents.sh codex
#
# Reads the per-session state written by hooks/agent-state.sh. Updates arrive
# as SIGRTMIN+8 (claude) / SIGRTMIN+9 (codex) from the hooks, so this is
# event-driven; the interval in config.jsonc is only a safety net.
#
# Classes: attention (blocked on you) > working > idle > none
set -uo pipefail

case "${1:-}" in
claude) agent="claude"; label="Claude Code" ;;
codex)  agent="codex";  label="Codex" ;;
*) printf '{"text":"","class":"none"}\n'; exit 0 ;;
esac

state_dir="${XDG_RUNTIME_DIR:-/tmp}/hypr-ai/$agent"

none() { printf '{"text":"","tooltip":"No %s sessions","class":"none"}\n' "$label"; exit 0; }

[[ -d "$state_dir" ]] || none
compgen -G "$state_dir/*.json" >/dev/null || none

# A session whose hooks never fired SessionEnd (killed terminal, crash) would
# linger forever otherwise. Untouched for 12h means dead.
find "$state_dir" -name '*.json' -mmin +720 -delete 2>/dev/null

# The program lives in a quoted heredoc so the single quotes Pango needs around
# colour attributes survive — inline in a '...' shell argument they would end
# the shell quoting instead.
read -r -d '' PROG <<'JQ'
def relative($secs):
    if   $secs < 60   then "just now"
    elif $secs < 3600 then "\(($secs / 60) | floor)m ago"
    else                   "\(($secs / 3600) | floor)h ago" end;

(now | floor) as $now

| map(select(type == "object" and .session != null))

| if length == 0 then
    { text: "", tooltip: "No \($label) sessions", class: "none" }
  else
    (map(select(.state == "attention"))) as $attention
  | (map(select(.state == "working")))   as $working
  | (map(.tasks  // 0) | add // 0)       as $tasks
  | (map(.agents // 0) | add // 0)       as $agents

  | (if   ($attention | length) > 0 then "attention"
     elif ($working   | length) > 0 then "working"
     else                               "idle" end) as $class

  | ((length | tostring)
     + (if $tasks  > 0 then " ·  \($tasks)"  else "" end)
     + (if $agents > 0 then " · 󰭆 \($agents)" else "" end)) as $text

  | (map(
        (((.cwd // "") | split("/") | last) // "~") as $project
      | (if   .state == "attention" then "<span color='#f38ba8'><b>󰀦 needs you</b></span>"
         elif .state == "working"   then "<span color='#cba6f7'>󰑮 working</span>"
         else                            "<span color='#a6adc8'>󰒲 idle</span>" end) as $badge
      | "<b>\($project)</b>  \($badge)"
        + (if (.reason // "") != "" then "\n   \(.reason)" else "" end)
        + (if (.tasks  // 0) > 0 then "\n    \(.tasks) task(s)" else "" end)
        + (if (.agents // 0) > 0 then "\n   󰭆 \(.agents) subagent(s)" else "" end)
        + "\n   <i>\(relative($now - (.updated // $now)))</i>"
    ) | join("\n\n")) as $tooltip

  | { text: $text,
      tooltip: "<b>\($label)</b>\n\n" + $tooltip,
      class: $class,
      alt: $class }
  end
JQ

jq -s --arg label "$label" "$PROG" "$state_dir"/*.json 2>/dev/null \
    || printf '{"text":"","tooltip":"%s state unreadable","class":"none"}\n' "$label"
