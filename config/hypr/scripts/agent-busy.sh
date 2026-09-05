#!/usr/bin/env bash
# hypridle condition: is a coding agent mid-task?
#
# Wired into hypridle.conf as condition_cmd. hypridle runs this when a timeout
# fires and only proceeds if it exits 0, so:
#
#   exit 1  -> an agent is working, defer blanking/locking
#   exit 0  -> nothing running, carry on
#
# Reads the same state files the Waybar modules read, written by the agents'
# own lifecycle hooks — so "busy" means the agent told us it was busy, not that
# a process happens to exist.
#
# Deliberately only counts `working`. A session in `attention` is blocked on
# you and might sit that way for hours; the notification and the bar pill are
# the right way to be told about that, not an unlockable screen.
#
# hypridle runs this synchronously on its event loop, so it stays cheap: a
# handful of small reads, no subprocesses beyond jq, no network.
set -uo pipefail

runtime="${XDG_RUNTIME_DIR:-/tmp}/hypr-ai"

for agent in claude codex; do
    dir="$runtime/$agent"
    compgen -G "$dir/*.json" >/dev/null || continue

    if jq -se 'any(.[]; .state == "working")' "$dir"/*.json >/dev/null 2>&1; then
        exit 1
    fi
done

exit 0
