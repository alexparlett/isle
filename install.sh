#!/usr/bin/env bash
#
# Install this Hyprland desktop on CachyOS.
#
#   ./install.sh                 packages + symlinks + post-install steps
#   ./install.sh --no-packages   symlinks only
#   ./install.sh --no-aur        skip the two AUR theme packages
#   ./install.sh --dry-run       print what would happen, change nothing
#   ./install.sh --unlink        remove the symlinks this script created
#
# Safe to re-run. Existing real files are moved aside to *.bak-<timestamp>,
# never deleted. Plasma is left completely alone.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
STAMP="$(date +%Y%m%d-%H%M%S)"

DO_PACKAGES=1
DO_AUR=1
DRY_RUN=0
UNLINK=0

# Whole directories that become symlinks into the repo.
DIR_LINKS=(
    hypr
    waybar
    rofi
    swaync
    swayosd
    wlogout
    alacritty
    qt6ct
    arch-update
)

# Individual files, for directories that hold state we must not clobber
# (GTK bookmarks, other portal configs).
FILE_LINKS=(
    "gtk-3.0/settings.ini"
    "gtk-4.0/settings.ini"
    "xdg-desktop-portal/hyprland-portals.conf"
)

# --- plumbing ----------------------------------------------------------------

c_reset=$'\e[0m'; c_bold=$'\e[1m'; c_dim=$'\e[2m'
c_green=$'\e[32m'; c_yellow=$'\e[33m'; c_red=$'\e[31m'; c_blue=$'\e[34m'

info()  { printf '%s==>%s %s\n' "$c_blue$c_bold" "$c_reset" "$*"; }
ok()    { printf '  %s✓%s %s\n' "$c_green" "$c_reset" "$*"; }
warn()  { printf '  %s!%s %s\n' "$c_yellow" "$c_reset" "$*"; }
err()   { printf '  %s✗%s %s\n' "$c_red" "$c_reset" "$*" >&2; }
skip()  { printf '  %s·%s %s\n' "$c_dim" "$c_reset" "$*"; }

run() {
    if ((DRY_RUN)); then
        printf '  %swould run:%s %s\n' "$c_dim" "$c_reset" "$*"
    else
        "$@"
    fi
}

usage() { sed -n '3,13p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0; }

while (($#)); do
    case "$1" in
        --no-packages) DO_PACKAGES=0 ;;
        --no-aur)      DO_AUR=0 ;;
        --dry-run)     DRY_RUN=1 ;;
        --unlink)      UNLINK=1 ;;
        -h|--help)     usage ;;
        *) err "unknown option: $1"; exit 2 ;;
    esac
    shift
done

# --- unlink ------------------------------------------------------------------

if ((UNLINK)); then
    info "Removing symlinks"
    for d in "${DIR_LINKS[@]}"; do
        target="$CONFIG/$d"
        if [[ -L "$target" && "$(readlink -f "$target")" == "$REPO/config/$d" ]]; then
            run rm "$target"; ok "$target"
        else
            skip "$target is not one of ours"
        fi
    done
    for f in "${FILE_LINKS[@]}"; do
        target="$CONFIG/$f"
        if [[ -L "$target" && "$(readlink -f "$target")" == "$REPO/config/$f" ]]; then
            run rm "$target"; ok "$target"
        else
            skip "$target is not one of ours"
        fi
    done
    info "Done. Backups from previous installs are still in $CONFIG as *.bak-*"
    exit 0
fi

# --- sanity ------------------------------------------------------------------

info "Checking the machine"

if [[ ! -f /etc/os-release ]] || ! grep -qiE 'cachyos|arch' /etc/os-release; then
    warn "this is written for CachyOS/Arch; continuing anyway"
fi

if [[ $EUID -eq 0 ]]; then
    err "run this as your normal user, not root — it needs your \$HOME"
    exit 1
fi

ok "user $USER, config dir $CONFIG"

# --- packages ----------------------------------------------------------------

pkglist() { sed 's/#.*//' "$1" | awk 'NF'; }

if ((DO_PACKAGES)); then
    info "Installing packages from the repositories"

    mapfile -t repo_pkgs < <(pkglist "$REPO/packages/repo.txt")
    mapfile -t want < <(printf '%s\n' "${repo_pkgs[@]}")

    missing=()
    for p in "${want[@]}"; do
        pacman -Q "$p" &>/dev/null || missing+=("$p")
    done

    if ((${#missing[@]} == 0)); then
        ok "all ${#want[@]} packages already installed"
    else
        printf '  installing %d of %d: %s\n' "${#missing[@]}" "${#want[@]}" "${missing[*]}"
        run sudo pacman -S --needed --noconfirm "${missing[@]}"
        ok "repository packages installed"
    fi

    if ((DO_AUR)); then
        info "Installing AUR packages"
        if ! command -v paru &>/dev/null; then
            warn "paru not found — skipping AUR (themes will fall back to defaults)"
        else
            mapfile -t aur_pkgs < <(pkglist "$REPO/packages/aur.txt")
            aur_missing=()
            for p in "${aur_pkgs[@]}"; do
                pacman -Q "$p" &>/dev/null || aur_missing+=("$p")
            done
            if ((${#aur_missing[@]} == 0)); then
                ok "AUR packages already installed"
            else
                run paru -S --needed --noconfirm "${aur_missing[@]}"
                ok "AUR packages installed"
            fi
        fi
    else
        skip "AUR skipped (--no-aur)"
    fi
else
    skip "package installation skipped (--no-packages)"
fi

# --- symlinks ----------------------------------------------------------------

link() {
    local src="$1" target="$2"

    if [[ -L "$target" ]]; then
        if [[ "$(readlink -f "$target")" == "$src" ]]; then
            ok "$target (already linked)"
            return
        fi
        run rm "$target"
    elif [[ -e "$target" ]]; then
        run mv "$target" "$target.bak-$STAMP"
        warn "moved existing $target to $target.bak-$STAMP"
    fi

    run mkdir -p "$(dirname "$target")"
    run ln -s "$src" "$target"
    ok "$target -> $src"
}

info "Linking configuration"
for d in "${DIR_LINKS[@]}"; do
    link "$REPO/config/$d" "$CONFIG/$d"
done
for f in "${FILE_LINKS[@]}"; do
    link "$REPO/config/$f" "$CONFIG/$f"
done

# --- post-install ------------------------------------------------------------

info "Post-install"

# Scripts need to be executable; git tracks the bit but a fresh checkout on a
# noexec-mounted filesystem or a zip download will not have it.
if ((DRY_RUN)); then
    skip "would chmod +x $REPO/config/hypr/scripts/*.sh"
else
    chmod +x "$REPO"/config/hypr/scripts/*.sh
    ok "scripts are executable"
fi

# Screenshot destination
run mkdir -p "${XDG_PICTURES_DIR:-$HOME/Pictures}/Screenshots"
ok "screenshot directory ready"

# XDG user dirs, so file dialogs land somewhere sensible
if command -v xdg-user-dirs-update &>/dev/null; then
    run xdg-user-dirs-update
    ok "XDG user directories updated"
fi

# Wallpaper: generated rather than committed, so no binary lives in git.
wallpaper="$CONFIG/hypr/wallpapers/mocha-gradient.png"
if [[ -f "$wallpaper" ]]; then
    ok "wallpaper already present"
elif command -v magick &>/dev/null; then
    run "$REPO/config/hypr/scripts/gen-wallpaper.sh" "$wallpaper"
    ok "wallpaper generated"
else
    warn "imagemagick missing — drop any image at $wallpaper"
fi

# Daily update check + desktop notification. The Waybar module reads the same
# package lists, so the badge and the notification never disagree.
if systemctl --user list-unit-files arch-update.timer &>/dev/null; then
    run systemctl --user enable --now arch-update.timer
    ok "update checks enabled (arch-update.timer, daily)"
else
    warn "arch-update.timer not found — is cachy-update installed?"
fi

# --- checks ------------------------------------------------------------------

info "Verifying the NVIDIA setup"

modeset=$(cat /sys/module/nvidia_drm/parameters/modeset 2>/dev/null || echo "?")
if [[ "$modeset" == "Y" ]]; then
    ok "nvidia_drm modeset is enabled"
else
    warn "nvidia_drm modeset reads '$modeset' — Hyprland needs Y."
    warn "Add 'options nvidia_drm modeset=1' to /etc/modprobe.d/nvidia.conf, then"
    warn "sudo mkinitcpio -P && reboot"
fi

gpu_path=$(grep -oP 'AQ_DRM_DEVICES", "\K[^"]+' "$REPO/config/hypr/env.lua" || true)
if [[ -n "$gpu_path" && -e "$gpu_path" ]]; then
    ok "primary GPU node $gpu_path exists"
elif [[ -n "$gpu_path" ]]; then
    warn "$gpu_path does not exist — the NVIDIA PCI address differs on this box."
    warn "Run: ls -l /dev/dri/by-path   and fix AQ_DRM_DEVICES in config/hypr/env.lua"
fi

if pacman -Q nvidia-open-dkms linux-cachyos-nvidia-open nvidia-open &>/dev/null; then
    ok "open kernel modules installed (required for the 50xx series)"
else
    warn "could not confirm the open NVIDIA kernel modules; 50xx cards require them"
fi

# --- done --------------------------------------------------------------------

cat <<EOF

$c_bold Done.$c_reset

  Log out, pick $c_bold Hyprland$c_reset at the SDDM session menu, and log back in.
  Plasma is untouched and still in that menu if you need to get back.

  First things to try:
    SUPER + Return    terminal
    SUPER + D         app launcher
    SUPER + /         every keybind, searchable
    SUPER + Escape    power menu

EOF
