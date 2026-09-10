#!/usr/bin/env bash
# Run by Steam as a non-Steam "game": opens Isle's quick menu (or leaves to the desktop with `desktop`) and
# stays alive while the menu is up, so Steam keeps handing the controller to it.
cfg="$HOME/.config/quickshell/isle"
if [[ "${1:-}" == "desktop" ]]; then
    qs -p "$cfg" ipc call bigpicture desktop
    exit 0
fi
qs -p "$cfg" ipc call bigpicture quick open
sleep 0.5
while [[ "$(qs -p "$cfg" ipc call bigpicture quickOpen 2>/dev/null)" == "true" ]]; do sleep 0.3; done
