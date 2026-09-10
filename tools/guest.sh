#!/usr/bin/env bash
# Run a command in the test guest with the Hyprland session's environment.
#     tools/guest.sh 'hyprctl clients'
exec ssh -p 2222 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o BatchMode=yes -o LogLevel=ERROR user@localhost \
    'export XDG_RUNTIME_DIR=/run/user/$(id -u); export HYPRLAND_INSTANCE_SIGNATURE=$(ls -t $XDG_RUNTIME_DIR/hypr 2>/dev/null | head -1); export WAYLAND_DISPLAY=$(ls $XDG_RUNTIME_DIR | grep -m1 "^wayland-[0-9]*$"); export QT_QPA_PLATFORM=wayland; '"$*"
