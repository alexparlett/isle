#!/usr/bin/env bash
# Waybar module: is gamemode actually active?
#
# gamemoderun is easy to put in a launch option and easy to get wrong — a typo
# means it silently does nothing. This tells you whether the CPU governor and
# scheduler tweaks are really applied, rather than whether you meant them to be.
set -uo pipefail

if ! command -v gamemoded >/dev/null; then
    printf '{"text":"","class":"none"}\n'
    exit 0
fi

# `gamemoded -s` prints "gamemode is active" / "gamemode is inactive" and, when
# active, the clients holding it.
status=$(gamemoded -s 2>/dev/null)

if [[ "$status" != *"is active"* ]]; then
    printf '{"text":"","tooltip":"GameMode inactive","class":"none"}\n'
    exit 0
fi

clients=$(gamemoded -s 2>/dev/null | tail -n +2 | head -5)
tooltip="GameMode active"
[[ -n "$clients" ]] && tooltip+=$'\n'"$clients"

printf '{"text":"󰊴","tooltip":%s,"class":"active"}\n' "$(jq -Rs . <<< "$tooltip")"
