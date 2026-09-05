#!/usr/bin/env bash
#
# Install this Hyprland desktop on CachyOS.
#
#   ./install.sh                        packages + symlinks + post-install
#   ./install.sh --no-packages          symlinks only
#   ./install.sh --no-aur               skip the AUR theme packages
#   ./install.sh --no-extras            skip packages/extras.txt
#   ./install.sh --no-agent-hooks       skip the Claude Code / Codex status hooks
#   ./install.sh --remove-agent-hooks   remove those hooks, then exit
#   ./install.sh --no-kde-colors        leave KDE app colours alone
#   ./install.sh --mac-keys             install the keyd Mac profile (needs sudo)
#   ./install.sh --dry-run              print what would happen, change nothing
#   ./install.sh --unlink               remove the symlinks this script created
#
# Safe to re-run. Existing real files are moved aside to *.bak-<timestamp>,
# never deleted. Plasma is left completely alone.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
STAMP="$(date +%Y%m%d-%H%M%S)"

DO_PACKAGES=1
DO_AUR=1
DO_EXTRAS=1
DO_AGENT_HOOKS=1
REMOVE_AGENT_HOOKS=0
DO_KDE_COLORS=1
DO_MAC_KEYS=0
DRY_RUN=0
UNLINK=0

CLAUDE_SETTINGS="$HOME/.claude/settings.json"
CODEX_HOOKS="$HOME/.codex/hooks.json"
HOOK_MARKER="hypr/scripts/hooks/agent-state.sh"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"

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

# Files under ~/.local/share rather than ~/.config. Same treatment.
DATA_FILE_LINKS=(
    "color-schemes/CatppuccinMocha.colors"
    "applications/hypr-settings.desktop"
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

usage() { sed -n '3,17p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0; }

while (($#)); do
    case "$1" in
        --no-packages)        DO_PACKAGES=0 ;;
        --no-aur)             DO_AUR=0 ;;
        --no-extras)          DO_EXTRAS=0 ;;
        --no-agent-hooks)     DO_AGENT_HOOKS=0 ;;
        --remove-agent-hooks) REMOVE_AGENT_HOOKS=1 ;;
        --no-kde-colors)      DO_KDE_COLORS=0 ;;
        --mac-keys)           DO_MAC_KEYS=1 ;;
        --dry-run)            DRY_RUN=1 ;;
        --unlink)             UNLINK=1 ;;
        -h|--help)            usage ;;
        *) err "unknown option: $1"; exit 2 ;;
    esac
    shift
done

# --- Coding-agent status hooks -----------------------------------------------
#
# These drive the Waybar modules that show running Claude Code / Codex sessions
# and, more usefully, whether one is blocked waiting for you. They live outside
# this repo (~/.claude/settings.json, ~/.codex/hooks.json), so both directions
# are handled explicitly and the original is always backed up first.

hooks_strip() {
    # Drop any hook entries pointing at our script, leaving the rest of the
    # user's config untouched.
    jq --arg marker "$HOOK_MARKER" '
        def ours: ((.hooks // []) | map(.command // "") | any(contains($marker)));
        if .hooks then
            .hooks |= (with_entries(.value |= map(select(ours | not)))
                       | with_entries(select(.value | length > 0)))
            | if (.hooks | length) == 0 then del(.hooks) else . end
        else . end
    '
}

# hooks_merge <target-file> <repo-manifest> <label>
hooks_merge() {
    local target="$1" manifest="$2" label="$3"
    local generated merged

    generated=$(sed "s|__HOME__|$HOME|g" "$manifest" | jq 'del(._comment)')

    if [[ -f "$target" ]]; then
        if ! jq -e . "$target" >/dev/null 2>&1; then
            err "$target is not valid JSON — refusing to touch it"
            return
        fi
    else
        run mkdir -p "$(dirname "$target")"
        ((DRY_RUN)) || echo '{}' > "$target"
        [[ -f "$target" ]] || return
    fi

    # Idempotent: strip our previous entries, then add the current ones.
    merged=$(hooks_strip < "$target" | jq -s '
        .[0] as $cur | .[1] as $add
        | ($cur + ($add | del(.hooks)))
        | .hooks = (reduce ($add.hooks | to_entries[]) as $e
                      (($cur.hooks // {}); .[$e.key] = ((.[$e.key] // []) + $e.value)))
    ' - <(printf '%s' "$generated"))

    if [[ -z "$merged" ]] || ! jq -e . <<< "$merged" >/dev/null 2>&1; then
        err "hook merge produced invalid JSON — leaving $target alone"
        return
    fi

    if ((DRY_RUN)); then
        skip "would add $(jq '.hooks | length' <<< "$generated") $label hook events to $target"
        return
    fi

    [[ -s "$target" ]] && cp "$target" "$target.bak-$STAMP"
    printf '%s\n' "$merged" > "$target"
    ok "$label status hooks installed (backup: $(basename "$target").bak-$STAMP)"
}

agent_hooks_install() {
    if command -v claude &>/dev/null; then
        hooks_merge "$CLAUDE_SETTINGS" "$REPO/config/claude/hooks.json" "Claude Code"
    else
        skip "claude not installed — skipping its hooks"
    fi

    if [[ -d "$HOME/.codex" ]]; then
        hooks_merge "$CODEX_HOOKS" "$REPO/config/codex/hooks.json" "Codex"
        warn "Codex will not run these until you trust them: open Codex and run /hooks"
    else
        skip "no ~/.codex — skipping Codex hooks"
    fi
}

agent_hooks_remove() {
    local target
    for target in "$CLAUDE_SETTINGS" "$CODEX_HOOKS"; do
        [[ -f "$target" ]] || continue
        local stripped
        stripped=$(hooks_strip < "$target")
        if ((DRY_RUN)); then
            skip "would remove our hooks from $target"
            continue
        fi
        cp "$target" "$target.bak-$STAMP"
        printf '%s\n' "$stripped" > "$target"
        ok "hooks removed from $target (backup kept)"
    done
}

if ((REMOVE_AGENT_HOOKS)); then
    info "Removing coding-agent status hooks"
    agent_hooks_remove
    exit 0
fi

# --- KDE application colours -------------------------------------------------
#
# Dolphin, Ark and Okular read ~/.config/kdeglobals, not GTK or qt6ct settings.
# Without this they stay Breeze-coloured and stand out badly next to everything
# else. Note this also recolours the Plasma fallback session — same kdeglobals.

kde_colors_apply() {
    if ((DRY_RUN)); then
        skip "would apply the CatppuccinMocha colour scheme to KDE apps"
        return
    fi

    [[ -f "$HOME/.config/kdeglobals" ]] && cp "$HOME/.config/kdeglobals" "$HOME/.config/kdeglobals.bak-$STAMP"

    # plasma-apply-colorscheme does the better job, but it ships in
    # plasma-workspace — the package that disappears when you retire the Plasma
    # fallback. Fall back to merging the scheme ourselves so this keeps working
    # afterwards.
    if command -v plasma-apply-colorscheme &>/dev/null &&
       plasma-apply-colorscheme CatppuccinMocha &>/dev/null; then
        ok "KDE apps recoloured (kdeglobals backed up)"
    elif command -v python3 &>/dev/null; then
        if python3 "$REPO/config/color-schemes/apply-colors.py" \
                   "$REPO/config/color-schemes/CatppuccinMocha.colors" --no-backup >/dev/null; then
            ok "KDE apps recoloured via the standalone merger (kdeglobals backed up)"
        else
            warn "could not apply the colour scheme; KDE apps keep their current colours"
        fi
    else
        warn "no way to apply the colour scheme — install python3 or plasma-workspace"
    fi
}

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
    for f in "${DATA_FILE_LINKS[@]}"; do
        target="$DATA_HOME/$f"
        if [[ -L "$target" && "$(readlink -f "$target")" == "$REPO/config/$f" ]]; then
            run rm "$target"; ok "$target"
        else
            skip "$target is not one of ours"
        fi
    done
    info "Done. Backups from previous installs are still in place as *.bak-*"
    info "Agent hooks are separate: ./install.sh --remove-agent-hooks"
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

# One package name per line, comments and surrounding whitespace removed.
# `awk NF{print $1}` rather than `awk NF`: stripping "dex  # comment" leaves the
# trailing spaces behind, and pacman then looks for a package literally called
# "dex                     " and reports it missing.
pkglist() { sed 's/#.*//' "$1" | awk 'NF{print $1}'; }

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

    if ((DO_EXTRAS)); then
        info "Installing extras (agentic tooling, backups)"
        mapfile -t extra_pkgs < <(pkglist "$REPO/packages/extras.txt")
        extra_missing=()
        for p in "${extra_pkgs[@]}"; do
            pacman -Q "$p" &>/dev/null || extra_missing+=("$p")
        done
        if ((${#extra_missing[@]} == 0)); then
            ok "extras already installed"
        else
            printf '  installing: %s\n' "${extra_missing[*]}"
            run sudo pacman -S --needed --noconfirm "${extra_missing[@]}"
            ok "extras installed"
        fi
    else
        skip "extras skipped (--no-extras)"
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
for f in "${DATA_FILE_LINKS[@]}"; do
    link "$REPO/config/$f" "$DATA_HOME/$f"
done

# --- post-install ------------------------------------------------------------

info "Post-install"

# Scripts need to be executable; git tracks the bit but a fresh checkout on a
# noexec-mounted filesystem or a zip download will not have it.
if ((DRY_RUN)); then
    skip "would chmod +x $REPO/config/hypr/scripts/**.sh"
else
    chmod +x "$REPO"/config/hypr/scripts/*.sh "$REPO"/config/hypr/scripts/hooks/*.sh
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

# The hyprland package ships two session entries, so SDDM asks which Hyprland
# you want on every login. This config does not use uwsm, so mask that one.
# /usr/local wins over /usr/share in SDDM's SessionDir order and pacman never
# writes there, so the mask survives package updates.
sessions_dir=/usr/local/share/wayland-sessions
if [[ -f "$sessions_dir/hyprland-uwsm.desktop" ]]; then
    ok "uwsm session entry already masked"
elif ((DRY_RUN)); then
    skip "would mask the Hyprland (uwsm) session entry"
else
    if sudo install -Dm644 "$REPO/config/sddm/hyprland-uwsm.desktop" \
                           "$sessions_dir/hyprland-uwsm.desktop"; then
        ok "masked the Hyprland (uwsm) entry — SDDM will offer one Hyprland"
    else
        warn "could not mask the uwsm session entry"
    fi
fi

# earlyoom does nothing until its service runs. Defaults are sensible: it acts
# at 10% free memory, killing the largest offender rather than letting the
# machine thrash itself unusable.
if pacman -Q earlyoom &>/dev/null; then
    run sudo systemctl enable --now earlyoom
    ok "earlyoom enabled"
fi

# Agent session widgets
if ((DO_AGENT_HOOKS)); then
    agent_hooks_install
else
    skip "agent status hooks skipped (--no-agent-hooks)"
fi

# KDE app colours — the one real look-and-feel gap otherwise
if ((DO_KDE_COLORS)); then
    kde_colors_apply
else
    skip "KDE colours left alone (--no-kde-colors)"
fi

# The settings window and its desktop entry
if command -v update-desktop-database &>/dev/null; then
    run update-desktop-database "$DATA_HOME/applications"
fi

# Mac keyboard profile — opt-in, because it is a system daemon reading every
# keystroke before anything else does.
if ((DO_MAC_KEYS)); then
    info "Installing the keyd Mac keyboard profile"
    if ((DRY_RUN)); then
        skip "would install keyd and write /etc/keyd/default.conf"
    else
        pacman -Q keyd &>/dev/null || sudo pacman -S --needed --noconfirm keyd
        if [[ -f /etc/keyd/default.conf ]] && ! grep -q hypr-mac-profile /etc/keyd/default.conf; then
            sudo cp /etc/keyd/default.conf "/etc/keyd/default.conf.bak-$STAMP"
            warn "existing /etc/keyd/default.conf backed up"
        fi
        sudo mkdir -p /etc/keyd
        sudo cp "$REPO/config/keyd/default.conf" /etc/keyd/default.conf
        sudo systemctl enable --now keyd
        sudo keyd reload 2>/dev/null || true
        ok "keyd Mac profile active — ⌘C/⌘V and friends now work in apps"
        ok "input.lua detects this and drops its own Alt/Super swap on next reload"
    fi
else
    skip "keyd Mac profile not installed (--mac-keys to add ⌘C/⌘V inside apps)"
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

  Keys are Mac-shaped — the ⌘ cap sends SUPER. First things to try:
    ⌘ + Space       launch bar
    ⌘ + Return      terminal
    ⌘ + ?           every keybind, searchable
    ⌘ + Escape      power menu
    ⌘ + ⇧ + 4       screenshot a region

EOF
