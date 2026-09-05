#!/usr/bin/env bash
# Keybind interop with HyprMod.
#
# HyprMod writes hyprland-gui.lua, which hyprland.lua requires after our bind
# modules. Hyprland registers both and fires both on a shared key, so
# binds-shared.lua reads what HyprMod claims and yields those keys to it.
#
# This checks the yielding actually happens, is insensitive to modifier order
# and case, leaves unrelated binds alone, and degrades to a no-op when HyprMod
# is not installed.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
fail=0

probe() {
    HOME="$1" lua -e '
        package.path="./config/hypr/?.lua;./tests/?.lua;"..package.path
        local m = require("hl-mock"); _G.hl = m.hl
        for _, x in ipairs({"env","monitors","input","look","snap","binds"}) do require(x) end
        local s = require("binds-shared")
        local found = {}
        for _, b in ipairs(m.calls.bind) do found[s.normalise(b.keys)] = true end
        print(#m.calls.bind)
        for key in pairs(found) do print("KEY " .. key) end
    '
}

check() {
    local label="$1" condition="$2"
    if [[ "$condition" == "yes" ]]; then
        echo "  ✓ $label"
    else
        echo "  ✗ $label"
        fail=1
    fi
}

baseline_home=$(mktemp -d)
baseline=$(probe "$baseline_home")
baseline_count=$(head -1 <<< "$baseline")
rm -rf "$baseline_home"

claimed_home=$(mktemp -d)
mkdir -p "$claimed_home/.config/hypr"
cat > "$claimed_home/.config/hypr/hyprland-gui.lua" <<'LUA'
-- Managed by HyprMod
-- Keybinds
hl.bind("SUPER + Return", hl.dsp.exec_cmd("foot"))
hl.bind("alt + super + 1", hl.dsp.exec_cmd("something"))
LUA
claimed=$(probe "$claimed_home")
claimed_count=$(head -1 <<< "$claimed")
rm -rf "$claimed_home"

echo "  baseline: $baseline_count binds · with HyprMod claiming 2: $claimed_count"

check "yields exactly the claimed keys" \
      "$([[ $((baseline_count - claimed_count)) -eq 2 ]] && echo yes)"
check "SUPER+Return yielded" \
      "$(grep -q '^KEY SUPER|return$' <<< "$claimed" && echo no || echo yes)"
check "ALT+SUPER+1 yielded despite different case and order" \
      "$(grep -q '^KEY ALT+SUPER|1$' <<< "$claimed" && echo no || echo yes)"
check "unrelated binds untouched (SUPER+E)" \
      "$(grep -q '^KEY SUPER|e$' <<< "$claimed" && echo yes)"
check "no HyprMod means no change" \
      "$(grep -q '^KEY SUPER|return$' <<< "$baseline" && echo yes)"

echo
((fail)) && { echo "  interop FAILED"; exit 1; }
echo "  keybind interop is sound"
