#!/usr/bin/env bash
#
# Install the shell on this machine: packages, the installed copy, config links, the user unit.
#
#     tools/install.sh            install this checkout and link it into place
#     tools/install.sh --packages also install the whole of packages/shell.txt with pacman
#     tools/install.sh --pam      also let PAM unlock the keyring at login (edits /etc/pam.d/greetd and login)
#
# A name on that list this machine has not got is installed by every run as part of the root half
# (tools/isle-root.py), so a package the shell starts calling arrives with an update (D83).
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
ISLE_USER="${USER:-$(id -un)}"
packages=0; pam=0
for a in "$@"; do case "$a" in --packages) packages=1 ;; --pam) pam=1 ;; -h|--help) sed -n '3,15p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;; esac; done

ok()   { printf '  \e[32m✓\e[0m %s\n' "$*"; }
# What is about to happen, for whatever is watching this run go by.
step() { printf '  \e[34m→\e[0m %s\n' "$*"; }
# A step that is root's. Run from a terminal that is sudo; run from Settings there is no terminal to
# type a password into, and pkexec puts the shell's own auth dialog up instead.
root() { if [[ -t 0 ]]; then sudo "$@"; else pkexec "$@"; fi; }

if ((packages)); then
    mapfile -t pkgs < <(sed 's/#.*//' "$REPO/packages/shell.txt" | tr -d ' ' | grep -v '^$')
    root pacman -S --needed --noconfirm "${pkgs[@]}"
    ok "packages"
fi

if ((pam)); then
    # gnome-keyring unlocks with the login password: greetd is the stack the greeter signs in through, login
    # the one the lock screen uses, so unlocking either unlocks the keyring.
    for svc in greetd login; do
        f=/etc/pam.d/$svc
        [[ -f "$f" ]] || continue
        if ! grep -q pam_gnome_keyring "$f"; then
            root sed -i '/^auth.*include.*system-local-login/a auth       optional     pam_gnome_keyring.so' "$f"
            root sed -i '/^session.*include.*system-local-login/a session    optional     pam_gnome_keyring.so auto_start' "$f"
            ok "pam_gnome_keyring in $f"
        else
            ok "pam_gnome_keyring already in $f"
        fi
    done
fi

# A checkout elsewhere is a working copy; the desktop runs from the installed one. The compositor's
# generated fragments and the built plugins are the installed copy's own and are kept.
if [[ "$REPO" != "$(mkdir -p "$ISLE_HOME" && cd "$ISLE_HOME" && pwd -P)" ]]; then
    [[ "$ISLE_HOME" != "$HOME" && "$ISLE_HOME" != "/" ]] || { echo "  ! ISLE_HOME must be a directory of its own" >&2; exit 1; }
    step "Copying the checkout into place"
    rsync -a --delete --exclude '/hypr/generated' --exclude '/dev' --exclude '/shell/userwidgets' --exclude '__pycache__' --exclude '/plugins/*/*.so' --exclude '/plugins/*/*.o' --exclude '/native/*/build' --exclude '/qml' "$REPO/" "$ISLE_HOME/"
    # A first copy has no fragments yet, and the compositor reloads before the shell renders them: the
    # checkout's serve until then.
    mkdir -p "$ISLE_HOME/hypr/generated"
    [[ -n "$(ls -A "$ISLE_HOME/hypr/generated")" ]] || cp "$REPO"/hypr/generated/*.lua "$ISLE_HOME/hypr/generated/" 2>/dev/null || true
    ok "installed to $ISLE_HOME from $REPO"
    REPO="$ISLE_HOME"
fi

# The browsing engine is a QML module compiled against the Qt on this machine, installed to the import
# path the session exports (D66). Without it the file manager and the file chooser are absent; the rest
# of the shell runs.
if command -v cmake >/dev/null; then
    step "Building the browsing engine"
    # Mtimes cannot decide what to rebuild here: rsync gives the copy the checkout's mtime, which is
    # older than the objects this machine already built, so ninja sees a changed source as up to date
    # and a changed source list keeps the moc output of the old one. The sources are hashed instead,
    # and any difference builds the lot; there are four files and it takes seconds.
    engine_stamp="$REPO/native/files/build/.isle-sources"
    engine_hash="$(find "$REPO/native/files" -path "$REPO/native/files/build" -prune -o -type f -print0 |
                   sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1)"
    if [[ -f "$REPO/native/files/build/CMakeCache.txt" && "$(cat "$engine_stamp" 2>/dev/null)" != "$engine_hash" ]]; then
        rm -rf "$REPO/native/files/build"
    fi
    if cmake -S "$REPO/native/files" -B "$REPO/native/files/build" -G Ninja \
            -DCMAKE_BUILD_TYPE=Release -DISLE_QML_DIR="$REPO/qml" >/dev/null &&
       cmake --build "$REPO/native/files/build" >/dev/null &&
       cmake --install "$REPO/native/files/build" >/dev/null; then
        printf %s "$engine_hash" > "$engine_stamp"
        ok "browsing engine built (Isle.Files)"
    else
        echo "  ! the browsing engine did not build; Files and the file chooser are off" >&2
    fi
else
    echo "  ! cmake not installed: the browsing engine is off until it is" >&2
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
step "Rendering the app themes"
python3 "$REPO/theme/render.py" && ok "app themes rendered (GTK, Qt, kitty, yazi, btop, zathura, portals)"
# Where an AppImage is installed by being put there; the shell watches it.
mkdir -p "$HOME/Applications"
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
systemctl --user daemon-reload
ok "the shell starts from hyprland.lua's start hook (shell/scripts/isle-session)"
# A machine without xdg-user-dirs is not a reason to stop: set -e would end the install here.
xdg-user-dirs-update >/dev/null 2>&1 || true
# Files is the shell's own window, reached through a desktop entry so anything opening a folder finds it (D73).
cat > "$HOME/.local/share/applications/isle-files.desktop" <<D
[Desktop Entry]
Type=Application
Name=Files
Comment=Browse the files on this machine
Exec=$REPO/shell/scripts/isle-files %f
Icon=system-file-manager
Terminal=false
Categories=System;FileTools;FileManager;
MimeType=inode/directory;
D
update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true
if xdg-mime default isle-files.desktop inode/directory 2>/dev/null; then ok "Files opens folders"; else echo "  ! xdg-mime could not make Files the handler for folders" >&2; fi
# Mousepad for text unless something other than Zed was chosen already.
if command -v mousepad >/dev/null; then
    case "$(xdg-mime query default text/plain 2>/dev/null)" in ""|dev.zed.Zed.desktop) xdg-mime default org.xfce.mousepad.desktop text/plain && ok "Mousepad opens text" ;; esac
fi
# Everything root has to do, in one run at the end, so one authorisation covers the lot and only when
# something of it is pending. The portal's own restart is the user's, so it stays out here.
if python3 "$REPO/tools/isle-root.py" --due "$REPO" "$ISLE_USER"; then
    step "Asking for permission"
    portal_changed=0
    cmp -s "$REPO/system/portal/isle.portal" /usr/share/xdg-desktop-portal/portals/isle.portal || portal_changed=1
    root python3 "$REPO/tools/isle-root.py" "$REPO" "$ISLE_USER" \
        "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}" "${HYPRLAND_INSTANCE_SIGNATURE:-}"
    ((portal_changed)) && systemctl --user restart xdg-desktop-portal.service 2>/dev/null || true
    hyprpm reload -n >/dev/null 2>&1 || true
fi
# Said after the root half, which installs what the package list asks for: these are what is left.
command -v unsquashfs >/dev/null ||
    echo "  ! squashfs-tools not installed: an AppImage gets an entry with no name or icon until it is (paru -S squashfs-tools)"
[ -e /usr/lib/libfuse.so.2 ] ||
    echo "  ! fuse2 not installed: an AppImage will not run until it is (paru -S fuse2)"
systemctl --user enable gcr-ssh-agent.socket >/dev/null 2>&1 && ok "gcr-ssh-agent.socket enabled (SSH agent)"
if command -v xremap >/dev/null; then
    systemctl --user enable xremap.service >/dev/null 2>&1 && ok "xremap.service enabled (keyboard profiles)"
else
    echo "  ! xremap not installed: keyboard profiles are off until it is (paru -S xremap-hypr-bin)"
fi
