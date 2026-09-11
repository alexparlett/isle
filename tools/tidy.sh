#!/usr/bin/env bash
#
# Take away what an earlier Isle installed or rendered and this one no longer uses.
#
#     tools/tidy.sh          show what would go
#     tools/tidy.sh --yes    take it away (packages need sudo)
#
# Packages that left packages/shell.txt, configuration Isle rendered for tools it no longer themes, and
# flags files Isle wrote for launchers that are gone. Files of the user's own are left alone.

set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
CFG="${XDG_CONFIG_HOME:-$HOME/.config}"
yes=0
for a in "$@"; do case "$a" in --yes) yes=1 ;; -h|--help) sed -n '3,9p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;; esac; done

ok()   { printf '  \e[32m✓\e[0m %s\n' "$*"; }
would() { printf '  \e[33m·\e[0m %s\n' "$*"; }
found=0

# Packages Isle once listed and no longer does (D39: qt6ct).
former=(qt6ct)
gone=()
for p in "${former[@]}"; do
    pacman -Q "$p" >/dev/null 2>&1 && ! grep -qx "$p" "$REPO/packages/shell.txt" && gone+=("$p")
done
if ((${#gone[@]})); then
    found=1
    if ((yes)); then sudo pacman -Rns --noconfirm "${gone[@]}" && ok "removed ${gone[*]}"; else would "packages: ${gone[*]}"; fi
fi

# Configuration Isle rendered for qt6ct (D39): only when nothing but Isle's files is there.
if [[ -d "$CFG/qt6ct" ]]; then
    others=$(find "$CFG/qt6ct" -type f ! -name qt6ct.conf ! -path '*/colors/isle.conf' | head -1)
    if [[ -z "$others" ]]; then
        found=1
        if ((yes)); then rm -rf "$CFG/qt6ct" && ok "removed $CFG/qt6ct"; else would "$CFG/qt6ct (Isle's qt6ct theme)"; fi
    fi
fi

# Flags files Isle wrote for launchers that are gone: the theme render strips its lines and removes an
# otherwise empty file.
stale=()
for f in "$CFG"/*.conf; do
    [[ -f "$f" ]] || continue
    grep -q 'added by Isle' "$f" || continue
    grep -qx "$(basename "$f")" <(python3 "$REPO/theme/render.py" --flags) || stale+=("$(basename "$f")")
done
if ((${#stale[@]})); then
    found=1
    if ((yes)); then python3 "$REPO/theme/render.py" --flags-only && ok "cleaned ${stale[*]}"; else would "flags files: ${stale[*]}"; fi
fi

((found)) || ok "nothing to take away"
((found && !yes)) && echo "  run with --yes to take these away" || true
