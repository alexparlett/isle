#!/usr/bin/env bash
# Toggle a docked AI assistant panel.
#
#   ai.sh claude    Claude (claude-desktop)
#   ai.sh chatgpt   ChatGPT (chatgpt-desktop)
#
# Each lives on its own special workspace, so it overlays whatever you are
# doing and vanishes again without disturbing the layout underneath. First
# invocation launches the app; rules.lua puts it on the right workspace and
# sizes it as a right-hand column.
set -uo pipefail

# Both are Electron. WaylandLinuxDrmSyncobj is what stops the flickering on
# NVIDIA — Electron 35+ supports explicit sync but does not enable it itself.
ELECTRON_FLAGS="--enable-features=WaylandLinuxDrmSyncobj"

case "${1:-}" in
claude)
    workspace="claude"
    class="com.anthropic.Claude"
    cmd="claude-desktop $ELECTRON_FLAGS"
    ;;
chatgpt)
    workspace="chatgpt"
    class="chatgpt"
    cmd="chatgpt $ELECTRON_FLAGS"
    ;;
*)
    echo "usage: ${0##*/} {claude|chatgpt}" >&2
    exit 1
    ;;
esac

running=$(hyprctl clients -j | jq --arg c "$class" 'any(.[]; (.class // "") | ascii_downcase == ($c | ascii_downcase))')

if [[ "$running" == "true" ]]; then
    hyprctl dispatch "hl.dsp.workspace.toggle_special(\"$workspace\")"
else
    # The window rule drops it onto special:$workspace and shows it.
    hyprctl dispatch "hl.dsp.exec_cmd(\"$cmd\")"
fi
