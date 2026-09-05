#!/usr/bin/env bash
# The launch bar — ALT+Space.
#
# Apps plus a calculator in one prompt: type "firefox" to launch, or "18*7.5"
# to get an answer you can copy with Enter. Falls back to plain drun if the
# rofi-calc plugin is not installed, rather than erroring out.
set -uo pipefail

theme="$HOME/.config/rofi/launchbar.rasi"

# rofi plugins live here; the package name is rofi-calc.
plugin_dir=$(pkg-config --variable=pluginsdir rofi 2>/dev/null || echo /usr/lib/rofi)

args=(
    -show drun
    -theme "$theme"
    -no-lazy-grab          # let the first keystrokes land in the entry
)

if [[ -f "$plugin_dir/calc.so" ]]; then
    args=(
        -show combi
        -modes "combi,drun,calc"
        -combi-modes "drun,calc"
        -theme "$theme"
        -no-lazy-grab
        -calc-command 'echo -n "{result}" | wl-copy'   # Enter copies the answer
        -no-history
    )
fi

exec rofi "${args[@]}"
