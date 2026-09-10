#!/usr/bin/env bash
#
# Install the login screen where the greeter user can run it.
#
#     sudo tools/install-greeter.sh USER
#
# greetd runs the greeter as its own user, before anyone has logged in, so it cannot live in a
# home directory. This puts a root-owned copy of greeter/ and the shell/ it draws with under
# /usr/local/share/isle-greeter and points greetd at it; USER is who the greeter logs in.
# tools/install.sh runs it again after a pull when greetd is set up this way.

set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
USER_NAME="${1:?the user the greeter logs in}"
DEST=/usr/local/share/isle-greeter
[[ $EUID -eq 0 ]] || { echo "run as root: sudo $0 $USER_NAME" >&2; exit 1; }
ok() { printf '  \e[32m✓\e[0m %s\n' "$*"; }

getent passwd greeter >/dev/null || useradd -r -M -d /var/lib/greetd -s /usr/bin/nologin greeter

# A fresh copy beside the old one, then swapped in, so a greeter starting mid-way never sees half a tree.
rm -rf "$DEST.new"
mkdir -p "$DEST.new"
cp -a "$REPO/greeter" "$REPO/shell" "$DEST.new/"
rm -rf "$DEST.new"/shell/scripts/__pycache__
chown -R root:root "$DEST.new"
chmod -R a+rX "$DEST.new"
rm -rf "$DEST"
mv "$DEST.new" "$DEST"
ok "greeter at $DEST"

install -d /etc/greetd
cat > /etc/greetd/config.toml <<T
[terminal]
vt = 1

[default_session]
command = "start-hyprland -- -c /etc/greetd/hyprland.lua"
user = "greeter"
T
cat > /etc/greetd/hyprland.lua <<T
hl.env("ISLE_GREETER_USER", "$USER_NAME")
hl.env("ISLE_GREETER_COMMAND", "start-hyprland")
hl.on("hyprland.start", function() hl.exec_cmd("qs -p $DEST/greeter") end)
T
ok "greetd logs $USER_NAME in through it"
