#!/usr/bin/env bash
# Waybar module: NVIDIA GPU utilisation, temperature and VRAM.
set -uo pipefail

read -r util temp used total power <<< "$(
    nvidia-smi --query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total,power.draw \
               --format=csv,noheader,nounits 2>/dev/null | tr -d ','
)" || true

if [[ -z "${util:-}" ]]; then
    printf '{"text":"","tooltip":"nvidia-smi unavailable","class":"unknown"}\n'
    exit 0
fi

class=normal
((temp >= 80)) && class=critical
((temp >= 70 && temp < 80)) && class=warning

tooltip=$(printf 'GPU %s%%\nTemp %s°C\nVRAM %s / %s MiB\nPower %sW' \
    "$util" "$temp" "$used" "$total" "${power%.*}")

printf '{"text":"%s%%","tooltip":%s,"class":"%s"}\n' \
    "$util" "$(jq -Rs . <<< "$tooltip")" "$class"
