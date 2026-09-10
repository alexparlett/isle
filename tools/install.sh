#!/usr/bin/env bash
#
# Install the shell on this machine: packages, the installed copy, config links, the user unit.
#
#     tools/install.sh            install this checkout and link it into place
#     tools/install.sh --packages also install packages/shell.txt with pacman
#     tools/install.sh --pam      also let PAM unlock the keyring at login (edits /etc/pam.d/login)
#
# The desktop runs from ~/.local/share/isle (ISLE_HOME to change it). Run from a
# checkout elsewhere, this copies the checkout there first, so editing the
# checkout changes nothing until the next install; run from that location, it
# installs in place. ~/.config/quickshell/isle and ~/.config/hypr/hyprland.lua
# are links into the installed copy. An existing hyprland.conf is left alone;
# Hyprland prefers hyprland.lua when both exist.

set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
CFG="${XDG_CONFIG_HOME:-$HOME/.config}"
ISLE_HOME="${ISLE_HOME:-$HOME/.local/share/isle}"
packages=0; pam=0
for a in "$@"; do case "$a" in --packages) packages=1 ;; --pam) pam=1 ;; -h|--help) sed -n '3,12p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;; esac; done

ok() { printf '  \e[32m✓\e[0m %s\n' "$*"; }

if ((packages)); then
    mapfile -t pkgs < <(sed 's/#.*//' "$REPO/packages/shell.txt" | tr -d ' ' | grep -v '^$')
    sudo pacman -S --needed --noconfirm "${pkgs[@]}"
    ok "packages"
fi

if ((pam)); then
    # gnome-keyring unlocks with the login password; the lock screen uses the same stack, so unlocking the session unlocks the keyring.
    if ! grep -q pam_gnome_keyring /etc/pam.d/login; then
        sudo sed -i '/^auth.*include.*system-local-login/a auth       optional     pam_gnome_keyring.so' /etc/pam.d/login
        sudo sed -i '/^session.*include.*system-local-login/a session    optional     pam_gnome_keyring.so auto_start' /etc/pam.d/login
        ok "pam_gnome_keyring in /etc/pam.d/login"
    else
        ok "pam_gnome_keyring already in /etc/pam.d/login"
    fi
fi

# A checkout elsewhere is a working copy; the desktop runs from the installed one. The compositor's
# generated fragments and the built plugins are the installed copy's own and are kept.
if [[ "$REPO" != "$(mkdir -p "$ISLE_HOME" && cd "$ISLE_HOME" && pwd -P)" ]]; then
    [[ "$ISLE_HOME" != "$HOME" && "$ISLE_HOME" != "/" ]] || { echo "  ! ISLE_HOME must be a directory of its own" >&2; exit 1; }
    rsync -a --delete --exclude '/hypr/generated' --exclude '/dev' --exclude '/shell/userwidgets' --exclude '__pycache__' --exclude '/plugins/*/*.so' --exclude '/plugins/*/*.o' "$REPO/" "$ISLE_HOME/"
    # A first copy has no fragments yet, and the compositor reloads before the shell renders them: the
    # checkout's serve until then.
    mkdir -p "$ISLE_HOME/hypr/generated"
    [[ -n "$(ls -A "$ISLE_HOME/hypr/generated")" ]] || cp "$REPO"/hypr/generated/*.lua "$ISLE_HOME/hypr/generated/" 2>/dev/null || true
    ok "installed to $ISLE_HOME from $REPO"
    REPO="$ISLE_HOME"
fi

link() { # link <target> <link>
    local dir; dir="$(dirname "$2")"
    # A dangling link where the directory should be, from an older layout, blocks mkdir.
    if [[ -L "$dir" && ! -e "$dir" ]]; then rm "$dir"; echo "  · removed dangling link $dir"; fi
    mkdir -p "$dir"
    if [[ -e "$2" && ! -L "$2" ]]; then echo "  ! $2 exists and is not a link; leaving it" >&2; return; fi
    ln -sfn "$1" "$2"; ok "$2 -> $1"
}
# A user's own QML widget imports the services only from inside the served tree, so the user widget
# directory is linked in under it (D14).
ISLE_WIDGETS="${XDG_CONFIG_HOME:-$HOME/.config}/isle/widgets"
mkdir -p "$ISLE_WIDGETS"
ln -sfn "$ISLE_WIDGETS" "$REPO/shell/userwidgets"
ok "user widgets reach the services (shell/userwidgets -> $ISLE_WIDGETS)"
link "$REPO/shell" "$CFG/quickshell/isle"
link "$REPO/hypr/hyprland.lua" "$CFG/hypr/hyprland.lua"
link "$REPO/hypr/generated" "$CFG/hypr/generated"
link "$REPO/systemd/xremap.service" "$CFG/systemd/user/xremap.service"
# nethogs counts network traffic per process; it needs two capabilities rather than root.
if command -v nethogs >/dev/null; then
    sudo setcap cap_net_admin,cap_net_raw+ep "$(command -v nethogs)" && ok "nethogs can read the network (per-process traffic in Monitor)"
fi
python3 "$REPO/theme/render.py" && ok "app themes rendered (GTK, Qt, kitty, yazi, btop, portals)"
# Raw HID for the browser keyboard configurators (Keychron Launcher, VIA): the rule is root's.
if ! cmp -s "$REPO/system/udev/70-isle-hidraw.rules" /etc/udev/rules.d/70-isle-hidraw.rules; then
    sudo install -Dm644 "$REPO/system/udev/70-isle-hidraw.rules" /etc/udev/rules.d/70-isle-hidraw.rules
    sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw
    ok "hidraw devices are the logged-in user's (keyboard configurators)"
fi
# Sites that are really apps get their own window and a launcher entry.
mkdir -p "$HOME/.local/share/applications"
cat > "$HOME/.local/share/applications/isle-keychron-launcher.desktop" <<D
[Desktop Entry]
Type=Application
Name=Keychron Launcher
Comment=Remap keys and update firmware on Keychron keyboards
Exec=$REPO/shell/scripts/webapp.sh https://launcher.keychron.com
Icon=input-keyboard
Categories=Settings;HardwareSettings;
Keywords=keyboard;keychron;firmware;via;
D
ok "Keychron Launcher in the launcher (opens in the browser as its own window)"
# The login screen is a root-owned copy; after a pull it is refreshed.
if grep -qs isle-greeter /etc/greetd/hyprland.lua; then
    sudo "$REPO/tools/install-greeter.sh" "$USER" && ok "greeter copy refreshed"
fi
systemctl --user daemon-reload
ok "the shell starts from hyprland.lua's start hook (shell/scripts/isle-session)"
xdg-user-dirs-update >/dev/null 2>&1
if command -v thunar >/dev/null && [ "$(xdg-mime query default inode/directory 2>/dev/null)" = "" ]; then
    xdg-mime default thunar.desktop inode/directory && ok "Thunar opens folders"
fi
# Thunar's "open terminal here" runs exo's TerminalEmulator helper, which is kitty.
mkdir -p "$CFG/xfce4" "$HOME/.local/share/xfce4/helpers"
cat > "$HOME/.local/share/xfce4/helpers/kitty.desktop" <<'H'
[Desktop Entry]
NoDisplay=true
Version=1.0
Encoding=UTF-8
Type=X-XFCE-Helper
X-XFCE-Category=TerminalEmulator
X-XFCE-Commands=kitty
X-XFCE-CommandsWithParameter=kitty -e %s
Name=kitty
Icon=kitty
H
grep -q "^TerminalEmulator=" "$CFG/xfce4/helpers.rc" 2>/dev/null || echo "TerminalEmulator=kitty" >> "$CFG/xfce4/helpers.rc"
ok "kitty is Thunar's terminal"
# Mousepad for text unless something other than Zed was chosen already.
if command -v mousepad >/dev/null; then
    case "$(xdg-mime query default text/plain 2>/dev/null)" in ""|dev.zed.Zed.desktop) xdg-mime default org.xfce.mousepad.desktop text/plain && ok "Mousepad opens text" ;; esac
fi
systemctl --user enable gcr-ssh-agent.socket >/dev/null 2>&1 && ok "gcr-ssh-agent.socket enabled (SSH agent)"
if command -v xremap >/dev/null; then
    systemctl --user enable xremap.service >/dev/null 2>&1 && ok "xremap.service enabled (keyboard profiles)"
else
    echo "  ! xremap not installed: keyboard profiles are off until it is (paru -S xremap-hypr-bin)"
fi
