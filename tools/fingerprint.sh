#!/usr/bin/env bash
#
# Fingerprint sign-in: fprintd, a driver for the reader, and your prints.
#
#     tools/fingerprint.sh              install what the reader needs and enroll a finger
#     tools/fingerprint.sh --sudo       also let a fingerprint stand in for the sudo password
#
# Readers libfprint knows are served by the repo package; the Goodix HTK32 ids
# the community goodix53x5 driver covers come from the AUR. The lock screen
# picks fingerprints up on its own once a finger is enrolled.

set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ok()   { printf '  \033[32m✓\033[0m %s\n' "$*"; }
info() { printf '  \033[34m·\033[0m %s\n' "$*"; }
err()  { printf '  \033[31m✗\033[0m %s\n' "$*" >&2; }

want_sudo=0
for a in "$@"; do case "$a" in --sudo) want_sudo=1 ;; -h|--help) sed -n '3,12p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;; esac; done

reader="$(lsusb | grep -io '27c6:[0-9a-f]\{4\}' | head -1 || true)"
[[ -n "$reader" ]] || { err "no Goodix reader on USB; other readers need only: sudo pacman -S fprintd"; exit 1; }
info "reader $reader"

aur() { if command -v paru >/dev/null; then paru "$@"; elif command -v yay >/dev/null; then yay "$@"; else err "paru or yay is needed to build the driver"; exit 1; fi; }

case "${reader#27c6:}" in
    5335|5385|5395)
        aur -S --needed libfprint-goodix53x5 ;;
    *)
        info "id $reader is not one this script knows; trying the repo libfprint"
        sudo pacman -S --needed libfprint ;;
esac

sudo pacman -S --needed --noconfirm fprintd
ok "fprintd"

if fprintd-list "$USER" 2>/dev/null | grep -q -- '- #'; then
    ok "a finger is already enrolled; fprintd-enroll adds another"
else
    info "enroll: touch the reader when asked, several times"
    fprintd-enroll
    ok "enrolled"
fi

if ((want_sudo)); then
    if ! grep -q pam_fprintd /etc/pam.d/sudo; then
        sudo sed -i '1a auth       sufficient   pam_fprintd.so' /etc/pam.d/sudo
        ok "pam_fprintd in /etc/pam.d/sudo"
    else
        ok "pam_fprintd already in /etc/pam.d/sudo"
    fi
fi

info "try it: fprintd-verify"
info "the lock screen offers the reader on its own from the next lock"
