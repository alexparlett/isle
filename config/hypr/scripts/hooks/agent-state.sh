#!/usr/bin/env bash
# Coding-agent hook → Waybar state. Serves both Claude Code and Codex, which
# expose near-identical hook APIs.
#
#   agent-state.sh <claude|codex> start|working|idle|attention|end
#   agent-state.sh <claude|codex> task+|task-|agent+|agent-
#
# Wired into ~/.claude/settings.json and ~/.codex/hooks.json by install.sh.
# Writes one JSON file per session under $XDG_RUNTIME_DIR/hypr-ai/<agent>/ and
# signals Waybar, so the bar reacts the moment something needs you instead of
# waiting for a poll.
#
# The hook payload arrives on stdin as JSON (session_id, cwd, hook_event_name,
# and event-specific extras). Hooks run on the agent's critical path, so this
# stays cheap and never blocks.
set -uo pipefail

agent="${1:-}"
verb="${2:-}"
[[ -z "$agent" || -z "$verb" ]] && exit 0

case "$agent" in
    claude) signal=8 ;;
    codex)  signal=9 ;;
    *) exit 0 ;;
esac

state_dir="${XDG_RUNTIME_DIR:-/tmp}/hypr-ai/$agent"
mkdir -p "$state_dir"

refresh() { pkill -RTMIN+$signal waybar 2>/dev/null; return 0; }

payload=$(timeout 2 cat) || payload='{}'
[[ -z "$payload" ]] && payload='{}'

session=$(jq -r '.session_id // empty' <<< "$payload" 2>/dev/null)
[[ -z "$session" ]] && exit 0

cwd=$(jq -r '.cwd // empty' <<< "$payload" 2>/dev/null)
# Claude puts the prompt text in .message on Notification; Codex uses .reason
# on some events. Either way it is the human-readable "why".
message=$(jq -r '.message // .reason // empty' <<< "$payload" 2>/dev/null)

file="$state_dir/$session.json"

if [[ "$verb" == "end" ]]; then
    rm -f "$file"
    refresh
    exit 0
fi

prev=$(cat "$file" 2>/dev/null || echo '{}')
now=$(date +%s)

new=$(jq -n \
    --argjson prev "$prev" \
    --arg verb "$verb" \
    --arg session "$session" \
    --arg cwd "$cwd" \
    --arg message "$message" \
    --argjson now "$now" '
    ($prev | if type == "object" then . else {} end) as $p

    | ($p.tasks  // 0) as $tasks
    | ($p.agents // 0) as $agents

    | {
        session: $session,
        cwd:     (if $cwd != "" then $cwd else ($p.cwd // "") end),
        updated: $now,
        started: ($p.started // $now),
        tasks:   (if   $verb == "task+"  then $tasks + 1
                  elif $verb == "task-"  then ([$tasks - 1, 0] | max)
                  else $tasks end),
        agents:  (if   $verb == "agent+" then $agents + 1
                  elif $verb == "agent-" then ([$agents - 1, 0] | max)
                  else $agents end),
        state:   (if   $verb == "attention" then "attention"
                  elif $verb == "working"   then "working"
                  elif $verb == "idle"      then "idle"
                  elif $verb == "start"     then "idle"
                  # Counter-only events must not clobber a real state.
                  else ($p.state // "idle") end),
        reason:  (if   $verb == "attention" then (if $message != "" then $message else "Waiting for you" end)
                  elif $verb == "working" or $verb == "idle" then ""
                  else ($p.reason // "") end)
      }
')

# Atomic, so the Waybar reader never catches a half-written file.
printf "%s" "$new" > "$file.tmp" && mv "$file.tmp" "$file"

refresh

# A pill in the bar is easy to miss on a 3440px screen when you have looked
# away. Toast the transition into `attention` — only the transition, so a
# session sitting blocked does not nag on every subsequent hook.
if [[ "$verb" == "attention" ]]; then
    was=$(jq -r '.state // ""' <<< "$prev" 2>/dev/null)
    if [[ "$was" != "attention" ]] && command -v notify-send >/dev/null; then
        project=$(basename "${cwd:-$PWD}")
        # Never let a missing notification daemon put noise on a hook's stderr —
        # the agent surfaces that as a hook failure.
        notify-send -a "${agent^}" -u normal -i dialog-question \
            "${agent^} needs you" \
            "${message:-Waiting for input}${project:+  ·  $project}" \
            2>/dev/null || true
    fi
fi

exit 0
