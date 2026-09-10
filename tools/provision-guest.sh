#!/usr/bin/env bash
#
# Turn a fresh CachyOS guest into the shell's build target.
#
# Runs INSIDE the VM, once, after a normal CachyOS + Hyprland install:
#
#     sudo mount -t 9p -o trans=virtio,version=9p2000.L,msize=512000 hyprrepo /repo
#     /repo/tools/provision-guest.sh
#
# Idempotent — safe to re-run after a package list changes.
#
# What it deliberately does NOT do: install a bar, a launcher, a notification
# daemon or a lock screen. The guest is the build target precisely because the
# shell is the only shell here, and anything installed to "make it usable"
# undoes the point of ground rule 1.

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MOUNT_TAG="hyprrepo"
MOUNT_POINT="/repo"

c_reset=$'\e[0m'; c_bold=$'\e[1m'; c_dim=$'\e[2m'
c_green=$'\e[32m'; c_yellow=$'\e[33m'; c_red=$'\e[31m'; c_blue=$'\e[34m'
info() { printf '%s==>%s %s\n' "$c_blue$c_bold" "$c_reset" "$*"; }
ok()   { printf '  %s✓%s %s\n' "$c_green" "$c_reset" "$*"; }
warn() { printf '  %s!%s %s\n' "$c_yellow" "$c_reset" "$*"; }
err()  { printf '  %s✗%s %s\n' "$c_red" "$c_reset" "$*" >&2; }
note() { printf '    %s%s%s\n' "$c_dim" "$*" "$c_reset"; }

fail=0

# --- sanity ------------------------------------------------------------------

info "Guest check"
if ! grep -qi 'cachyos\|arch' /etc/os-release 2>/dev/null; then
    err "this is not an Arch-derived system"; exit 1
fi
if ! systemd-detect-virt --quiet; then
    err "not running in a VM — this script installs a disposable build target"
    note "It removes nothing, but it is not meant for a real machine."
    exit 1
fi
ok "CachyOS guest under $(systemd-detect-virt)"

# --- ssh, so nothing else needs the QEMU window ------------------------------

info "Remote access"
if systemctl is-active --quiet sshd; then
    ok "sshd running"
else
    sudo systemctl enable --now sshd && ok "sshd enabled" || { err "sshd failed"; fail=1; }
fi

# --- persist the repo mount --------------------------------------------------

info "Repo share"
if mountpoint -q "$MOUNT_POINT"; then
    ok "$MOUNT_POINT mounted"
else
    warn "$MOUNT_POINT not mounted — mounting now"
    sudo mkdir -p "$MOUNT_POINT"
    sudo mount -t 9p -o trans=virtio,version=9p2000.L,msize=512000 \
        "$MOUNT_TAG" "$MOUNT_POINT" || { err "9p mount failed"; fail=1; }
fi

fstab_line="$MOUNT_TAG  $MOUNT_POINT  9p  trans=virtio,version=9p2000.L,msize=512000,nofail,x-systemd.device-timeout=5  0 0"
if grep -q "^$MOUNT_TAG" /etc/fstab 2>/dev/null; then
    ok "fstab entry present"
else
    # nofail matters: a guest that will not boot without the host's share is a
    # guest you cannot debug when the share is the thing that broke.
    echo "$fstab_line" | sudo tee -a /etc/fstab >/dev/null && ok "fstab entry added (nofail)"
fi

# --- packages ----------------------------------------------------------------

info "Packages"
list="$REPO/packages/shell.txt"
[[ -f "$list" ]] || { err "missing $list"; exit 1; }

# Same parser install.sh uses. A trailing space here once fed pacman
# "dex                     " and broke a real machine; tests/test-packages.sh
# exists because of it.
mapfile -t pkgs < <(sed 's/#.*//' "$list" | awk 'NF{print $1}')
note "${#pkgs[@]} packages from packages/shell.txt"

missing=()
for p in "${pkgs[@]}"; do pacman -Qq "$p" &>/dev/null || missing+=("$p"); done

if ((${#missing[@]} == 0)); then
    ok "all present"
else
    note "installing ${#missing[@]}: ${missing[*]}"
    sudo pacman -S --needed --noconfirm "${missing[@]}" \
        && ok "installed" || { err "pacman failed"; fail=1; }
fi

# --- what the guest must NOT have -------------------------------------------
#
# Every package below provides a capability the shell owns. Leaving one
# installed hides the surface that is supposed to provide it, and the guest
# stops being a place where a missing surface is obvious. CachyOS's Hyprland
# edition ships several of them, so this is not hypothetical.

info "Clearing the field"
unwanted=(waybar rofi rofi-wayland wofi swaync mako dunst swayosd wlogout
          hypridle hyprlock hyprpolkitagent pavucontrol blueman network-manager-applet
          nwg-look qt6ct)
present=()
for p in "${unwanted[@]}"; do pacman -Qq "$p" &>/dev/null && present+=("$p"); done

# arch-update is kept — its timer and CLI are how the shell learns about
# updates — but CachyOS autostarts its tray applet on any desktop, and that is
# a foreign GUI for a capability the shell owns. A Hidden=true override is the
# XDG way to suppress an /etc/xdg/autostart entry without touching the package.
if [[ -f /etc/xdg/autostart/arch-update-tray.desktop ]]; then
    mkdir -p "$HOME/.config/autostart"
    printf '[Desktop Entry]\nType=Application\nName=Arch Update Tray\nHidden=true\n' \
        > "$HOME/.config/autostart/arch-update-tray.desktop"
    ok "suppressed the arch-update tray applet (its timer stays)"
fi

if ((${#present[@]} == 0)); then
    ok "none installed"
else
    note "found ${#present[@]}: ${present[*]}"
    note "Removing these is the point: with no fallback installed, a surface"
    note "that does not work is visible immediately instead of being covered."
    sudo pacman -Rns --noconfirm "${present[@]}" 2>/dev/null \
        && ok "removed" || warn "some could not be removed (dependencies) — not fatal"
fi

# --- display -----------------------------------------------------------------
#
# The design targets one 3440x1440 output. virtio-gpu will happily give us that
# mode, and testing a shell built for an ultrawide on a 1280x800 virtual screen
# would find layout bugs that do not exist and miss the ones that do.

info "Display"
res=$(hyprctl monitors -j 2>/dev/null | grep -o '"width": *[0-9]*' | head -1 | grep -o '[0-9]*')
if [[ "${res:-0}" -ge 3440 ]]; then
    ok "already ${res}px wide"
else
    warn "guest display is ${res:-unknown}px wide; the design targets 3440"
    note "Add to the guest's Hyprland config, or set it in the shell's monitor rule:"
    note "  monitor = Virtual-1, 3440x1440@60, 0x0, 1"
    note "QEMU's virtio-gpu supports arbitrary modes, so this should just work."
fi

# --- the verification loop ---------------------------------------------------

info "Shot harness"
for c in grim jq quickshell; do
    if command -v "$c" >/dev/null; then ok "$c"; else err "missing $c"; fail=1; fi
done
mkdir -p "$HOME/shots" && ok "shots directory"

if [[ -z "${WAYLAND_DISPLAY:-}" ]]; then
    warn "no WAYLAND_DISPLAY in this shell"
    note "Over ssh you must point at the session before shooting:"
    note "  export WAYLAND_DISPLAY=wayland-1"
    note "  export XDG_RUNTIME_DIR=/run/user/\$(id -u)"
fi

# --- done --------------------------------------------------------------------

echo
if ((fail)); then
    err "provisioning finished with errors — see above"
    exit 1
fi
ok "guest provisioned"
note "Verify with:  quickshell --version && grim -o '' /tmp/t.png && echo shots work"
