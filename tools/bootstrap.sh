#!/usr/bin/env bash
#
# Set up the shell on a fresh CachyOS (or Arch) install, from nothing to a login.
#
# Run it from a checkout, or with nothing at all:
#
#     ISLE_REPO_URL=<repository url> bash <(curl -fsSL <raw url of this script>)
#
# which clones the repo to ~/.local/share/isle first.
#
#     tools/bootstrap.sh                  from a checkout
#     tools/bootstrap.sh --no-aur         skip paru and the AUR packages (xremap, cursor theme, CoolerControl)
#     tools/bootstrap.sh --greeter        also install greetd with the shell's greeter (replaces SDDM)
#     tools/bootstrap.sh --no-bars        skip building hyprbars (title bars with close, fullscreen, minimise)
#     tools/bootstrap.sh --finish         after an install from the Isle ISO: only the AUR packages and hyprbars
#
# What it does, in order: packages from packages/shell.txt; paru and the AUR
# packages; the repo linked into ~/.config (tools/install.sh); PAM for the
# keyring; the system services the shell talks to; the user's groups. It
# never touches the display manager unless --greeter is given: CachyOS's
# SDDM lists Hyprland as a session, and Hyprland reads ~/.config/hypr/hyprland.lua.
#
# Safe to run again: everything is --needed, enable, or a link.

set -euo pipefail
REPO_URL="${ISLE_REPO_URL:-}"
DEST="${ISLE_REPO:-$HOME/.local/share/isle}"
aur=1; greeter=0; bars=1; finish=0
for a in "$@"; do case "$a" in --no-aur) aur=0 ;; --greeter) greeter=1 ;; --no-bars) bars=0 ;; --finish) finish=1 ;; -h|--help) sed -n '3,25p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;; esac; done

ok() { printf '  \e[32m✓\e[0m %s\n' "$*"; }
step() { printf '\n\e[1m%s\e[0m\n' "$*"; }
die() { printf '  \e[31m!\e[0m %s\n' "$*" >&2; exit 1; }

[[ $EUID -eq 0 ]] && die "run as your user, not root; sudo is asked for when needed"
command -v pacman >/dev/null || die "this is for CachyOS or Arch (pacman)"
sudo -v || die "sudo is needed"

# --- the repo -----------------------------------------------------------------
step "Repo"
if [[ -f "$(dirname "${BASH_SOURCE[0]}")/../packages/shell.txt" ]]; then
    DEST="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    ok "using the checkout at $DEST"
else
    sudo pacman -S --needed --noconfirm git >/dev/null
    if [[ -d "$DEST/.git" ]]; then git -C "$DEST" pull --ff-only && ok "updated $DEST"
    elif [[ -n "$REPO_URL" ]]; then git clone "$REPO_URL" "$DEST" && ok "cloned to $DEST"
    else die "no checkout at $DEST: clone the repository there, or set ISLE_REPO_URL"; fi
fi

# --- packages ---------------------------------------------------------------------
if ((!finish)); then
step "Packages"
mapfile -t pkgs < <(sed 's/#.*//' "$DEST/packages/shell.txt" | tr -d ' ' | grep -v '^$')
if ! pacman -Sl multilib >/dev/null 2>&1; then
    pkgs=("${pkgs[@]/lib32-gamemode}")
    echo "  · multilib is off, skipping lib32-gamemode (enable it in /etc/pacman.conf for 32-bit games)"
fi
sudo pacman -Syu --needed --noconfirm "${pkgs[@]}"
# Explicit, so one that arrived as another desktop's dependency survives that desktop's removal.
sudo pacman -D --asexplicit "${pkgs[@]}" >/dev/null 2>&1 || true
ok "${#pkgs[@]} packages"
fi

if ((aur)); then
    step "AUR"
    if ! command -v paru >/dev/null && ! command -v yay >/dev/null; then
        sudo pacman -S --needed --noconfirm base-devel >/dev/null
        tmp="$(mktemp -d)"
        git clone https://aur.archlinux.org/paru-bin.git "$tmp/paru-bin" >/dev/null
        (cd "$tmp/paru-bin" && makepkg -si --noconfirm)
        rm -rf "$tmp"
        ok "paru"
    fi
    helper="$(command -v paru || command -v yay)"
    "$helper" -S --needed --noconfirm xremap-hypr-bin bibata-cursor-theme coolercontrol && ok "xremap, Bibata cursor, CoolerControl"
fi

# --- compositor plugins: hyprbars (title bars) and isle-windows (drag tiling), built by hyprpm ------
if ((bars)); then
    step "Title bars"
    sudo pacman -S --needed --noconfirm base-devel cmake cpio >/dev/null
    # This repo is the plugin source: hyprbars as upstream builds it, plus no bar over a window that draws its own controls.
    if hyprpm update && (yes | hyprpm add "$DEST") && hyprpm enable hyprbars && hyprpm enable isle-windows; then
        ok "hyprbars and isle-windows built and enabled; hyprland.lua loads them at start"
    else
        echo "  ! hyprbars did not build; windows get no title bars (Win+Q, Win+Shift+F, Win+M still work)"
    fi
fi

if ((finish)); then
    rm -f "$DEST/.finish-setup"
    step "Done"
    echo "  Log out and in again for the title bars and keyboard profiles."
    exit 0
fi

# --- the shell ----------------------------------------------------------------------
step "Shell"
"$DEST/tools/install.sh" --pam

# --- system services and groups ---------------------------------------------------
step "Services"
sudo systemctl enable --now NetworkManager bluetooth power-profiles-daemon udisks2 accounts-daemon cups.socket >/dev/null 2>&1 && ok "NetworkManager, bluetooth, power-profiles-daemon, udisks2, accounts-daemon, cups"
if systemctl list-unit-files coolercontrold.service >/dev/null 2>&1; then
    sudo systemctl enable --now coolercontrold.service >/dev/null 2>&1 && ok "coolercontrold"
fi
if systemctl list-unit-files lactd.service >/dev/null 2>&1; then
    sudo systemctl enable --now lactd.service >/dev/null 2>&1 && ok "lactd"
fi
for g in i2c input; do
    getent group "$g" >/dev/null || sudo groupadd -r "$g"
    id -nG | grep -qw "$g" || sudo usermod -aG "$g" "$USER"
done
ok "groups: i2c (DDC/CI brightness), input (gamepads); they apply at the next login"
if [[ ! -f /etc/modules-load.d/i2c-dev.conf ]]; then
    echo i2c-dev | sudo tee /etc/modules-load.d/i2c-dev.conf >/dev/null && ok "i2c-dev loads at boot (ddcutil)"
fi

# --- the greeter, only when asked -------------------------------------------------
if ((greeter)); then
    step "Greeter"
    sudo pacman -S --needed --noconfirm greetd >/dev/null
    sudo "$DEST/tools/install-greeter.sh" "$USER"
    sudo systemctl disable sddm >/dev/null 2>&1 || true
    sudo systemctl enable greetd >/dev/null && ok "greetd with the shell's greeter; SDDM disabled"
fi

step "Done"
echo "  Log out and choose Hyprland at the login screen. The shell starts with the compositor."
echo "  Then: Settings › Keyboard for the Mac profile, Settings › Displays for monitors, tools/install.sh anytime after a git pull."
