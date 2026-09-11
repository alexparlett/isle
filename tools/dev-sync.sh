#!/usr/bin/env bash
#
# Keep the installed shell in step with a development checkout, and reload what a change touched.
#
#     tools/dev-sync.sh            sync once
#     tools/dev-sync.sh --watch    keep syncing, every 2 s (the isle-dev-sync user unit, tools/install.sh --dev)
#
# Same copy rules as tools/install.sh: the installed copy keeps its own hypr/generated and built plugins.
# Then: theme/ re-renders the app themes, hypr/ reloads the compositor, shell/ restarts the shell (its
# session loop brings it back with the compositor's own environment), plugins/ is left for hyprpm.
set -uo pipefail
SRC="${ISLE_SRC:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)}"
DEST="${ISLE_HOME:-$HOME/.local/share/isle}"
EXCL=(--exclude '/hypr/generated' --exclude '/dev' --exclude '/shell/userwidgets' --exclude '__pycache__' --exclude '/plugins/*/*.so' --exclude '/plugins/*/*.o' --exclude '/.git')

[[ -d "$DEST/shell" ]] || { echo "no installed copy at $DEST; run tools/install.sh first"; exit 1; }
[[ "$(cd "$SRC" && pwd -P)" != "$(cd "$DEST" && pwd -P)" ]] || { echo "the checkout is the installed copy; nothing to sync"; exit 0; }

# The compositor may have restarted since this started: hyprctl is pointed at the newest instance.
hypr() {
    local sig
    sig="$(hyprctl -j instances 2>/dev/null | python3 -c 'import json,sys; l=json.load(sys.stdin); print(l[-1]["instance"] if l else "")' 2>/dev/null)"
    [[ -n "$sig" ]] && HYPRLAND_INSTANCE_SIGNATURE="$sig" hyprctl "$@"
}

sync_once() {
    local changed
    changed="$(rsync -ai --delete "${EXCL[@]}" "$SRC/" "$DEST/" | awk '{print $2}' | grep -v '/$')"
    [[ -z "$changed" ]] && return 0
    echo "$(date +%T) $(wc -l <<<"$changed") file(s): $(head -3 <<<"$changed" | tr '\n' ' ')"
    if grep -q '^theme/' <<<"$changed"; then python3 "$DEST/theme/render.py" >/dev/null 2>&1 && echo "  themes rendered"; fi
    if grep -q '^hypr/' <<<"$changed"; then hypr reload >/dev/null 2>&1 && echo "  compositor reloaded"; fi
    if grep -q '^shell/' <<<"$changed"; then pkill -x qs && echo "  shell restarted"; fi
    if grep -q '^plugins/' <<<"$changed"; then echo "  plugins changed: hyprpm update rebuilds them"; fi
}

if [[ "${1:-}" == "--watch" ]]; then
    echo "watching $SRC -> $DEST"
    while :; do sync_once; sleep 2; done
else
    sync_once && echo "in step"
fi
