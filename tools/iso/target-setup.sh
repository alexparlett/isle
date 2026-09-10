#!/usr/bin/env bash
#
# The system side of tools/bootstrap.sh, run as root inside the installer's chroot:
#
#     tools/iso/target-setup.sh USER
#
# The packages are already in (the installer's Isle group). This links the repo into the
# user's config, sets PAM for the keyring, enables the services, adds the groups, and makes
# greetd with the shell's greeter the display manager. The AUR packages and hyprbars need a
# session and a password, so it leaves a marker the shell turns into a first-login prompt
# for tools/bootstrap.sh --finish.

set -euo pipefail
USER_NAME="${1:?user}"
HOME_DIR="$(getent passwd "$USER_NAME" | cut -d: -f6)"
DEST="$HOME_DIR/.local/share/isle"
[[ -f "$DEST/packages/shell.txt" ]] || { echo "isle: no checkout at $DEST" >&2; exit 1; }
ok() { printf '  isle: %s\n' "$*"; }

chown -R "$USER_NAME:" "$DEST"
mapfile -t pkgs < <(sed 's/#.*//' "$DEST/packages/shell.txt" | tr -d ' ' | grep -v '^$')
pacman -D --asexplicit "${pkgs[@]}" >/dev/null 2>&1 || true
runuser -u "$USER_NAME" -- env HOME="$HOME_DIR" "$DEST/tools/install.sh" && ok "linked into $HOME_DIR/.config"

if ! grep -q pam_gnome_keyring /etc/pam.d/login; then
    sed -i '/^auth.*include.*system-local-login/a auth       optional     pam_gnome_keyring.so' /etc/pam.d/login
    sed -i '/^session.*include.*system-local-login/a session    optional     pam_gnome_keyring.so auto_start' /etc/pam.d/login
    ok "pam_gnome_keyring in /etc/pam.d/login"
fi

systemctl enable NetworkManager bluetooth power-profiles-daemon udisks2 accounts-daemon cups.socket >/dev/null 2>&1 && ok "services enabled"
systemctl enable lactd.service >/dev/null 2>&1 || true
for g in i2c input; do
    getent group "$g" >/dev/null || groupadd -r "$g"
    usermod -aG "$g" "$USER_NAME"
done
echo i2c-dev > /etc/modules-load.d/i2c-dev.conf
ok "groups i2c and input; i2c-dev at boot"

install -Dm644 "$DEST/system/udev/70-isle-hidraw.rules" /etc/udev/rules.d/70-isle-hidraw.rules
"$DEST/tools/install-greeter.sh" "$USER_NAME"
rm -f /etc/systemd/system/display-manager.service
ln -sf /usr/lib/systemd/system/greetd.service /etc/systemd/system/display-manager.service
ok "greetd with the shell's greeter"

# The shell shows a first-login prompt while this exists; bootstrap.sh --finish removes it.
runuser -u "$USER_NAME" -- touch "$DEST/.finish-setup"
ok "first login finishes the AUR packages and hyprbars"
