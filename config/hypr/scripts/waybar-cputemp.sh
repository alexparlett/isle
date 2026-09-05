#!/usr/bin/env bash
# Waybar module: CPU temperature.
#
# Finds the sensor by name rather than hardcoding a hwmon number — those are
# assigned in probe order and move between boots. On this box it is k10temp
# (Ryzen 9800X3D); Tctl is the control temperature AMD's boost algorithm
# actually reacts to, which is the number worth watching.
set -uo pipefail

sensor=""
for hwmon in /sys/class/hwmon/hwmon*; do
    [[ -r "$hwmon/name" ]] || continue
    case "$(<"$hwmon/name")" in
        k10temp | zenpower | coretemp)
            sensor="$hwmon"
            break
            ;;
    esac
done

if [[ -z "$sensor" || ! -r "$sensor/temp1_input" ]]; then
    printf '{"text":"","tooltip":"No CPU sensor found","class":"none"}\n'
    exit 0
fi

milli=$(<"$sensor/temp1_input")
temp=$((milli / 1000))

class=normal
((temp >= 85)) && class=critical
((temp >= 75 && temp < 85)) && class=warning

# Per-CCD readings where the sensor exposes them.
tooltip="CPU $temp°C"
for label in "$sensor"/temp*_label; do
    [[ -r "$label" ]] || continue
    input="${label%_label}_input"
    [[ -r "$input" ]] || continue
    tooltip+=$'\n'"$(<"$label"): $(( $(<"$input") / 1000 ))°C"
done

printf '{"text":"%s°C","tooltip":%s,"class":"%s"}\n' \
    "$temp" "$(jq -Rs . <<< "$tooltip")" "$class"
