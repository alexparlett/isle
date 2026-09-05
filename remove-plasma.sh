#!/usr/bin/env bash
#
# Retire the Plasma fallback, once Hyprland has earned your trust.
#
#   ./remove-plasma.sh            show the plan, change nothing
#   ./remove-plasma.sh --run      actually remove, after confirmation
#   ./remove-plasma.sh --run --apps   also remove the KDE applications
#
# Removes the Plasma *session* — the shell, KWin, the KCMs, the Plasma applets —
# while keeping the parts of KDE that are just libraries or applications:
# Dolphin, Ark, Okular and friends, and crucially kwallet, which kio and your
# Proton packages depend on.
#
# Dry run is the default. Nothing here is reversible in one command, so the
# script refuses to proceed unless the replacements for everything Plasma was
# doing are actually present and running.

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DO_RUN=0
DO_APPS=0

c_reset=$'\e[0m'; c_bold=$'\e[1m'; c_dim=$'\e[2m'
c_green=$'\e[32m'; c_yellow=$'\e[33m'; c_red=$'\e[31m'; c_blue=$'\e[34m'
info() { printf '%s==>%s %s\n' "$c_blue$c_bold" "$c_reset" "$*"; }
ok()   { printf '  %s✓%s %s\n' "$c_green" "$c_reset" "$*"; }
warn() { printf '  %s!%s %s\n' "$c_yellow" "$c_reset" "$*"; }
err()  { printf '  %s✗%s %s\n' "$c_red" "$c_reset" "$*" >&2; }
skip() { printf '  %s·%s %s\n' "$c_dim" "$c_reset" "$*"; }

while (($#)); do
    case "$1" in
        --run)  DO_RUN=1 ;;
        --apps) DO_APPS=1 ;;
        -h|--help) sed -n '3,16p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;;
        *) err "unknown option: $1"; exit 2 ;;
    esac
    shift
done

# --- what goes ---------------------------------------------------------------
#
# The Plasma set is derived from the `plasma` package group rather than written
# out by hand — a hand-written list missed 27 installed packages when this was
# first written (bluedevil, libplasma, plasma-systemmonitor, kactivitymanagerd
# and friends), which left half the KF6 stack pinned by things nobody wanted.
# The group is what pacman itself considers Plasma, so it stays correct.
#
# CachyOS ships its own Plasma theming outside the group; those are matched
# separately. Anything on the PROTECTED list below is filtered back out.

plasma_targets() {
    local group extras
    mapfile -t group < <(comm -12 <(pacman -Sgq plasma 2>/dev/null | sort -u) <(pacman -Qq | sort))
    mapfile -t extras < <(pacman -Qq | grep -E '^cachyos-.*(kde|plasma)')

    local p guard keep
    for p in "${group[@]}" "${extras[@]}"; do
        keep=0
        for guard in "${PROTECTED[@]}"; do
            [[ "$p" == "$guard" ]] && keep=1 && break
        done
        ((keep)) || printf '%s\n' "$p"
    done
}

# Applications, removed only with --apps. kwalletmanager and kdeconnect are
# deliberately absent: the wallet is still in use by kio and the Proton
# packages, and kdeconnect has no equivalent in the repos.
APP_PKGS=(
    dolphin ark okular gwenview kate konsole kcalc filelight kdialog haruna
)

# Removing any of these would break something this desktop actually relies on.
PROTECTED=(
    kwallet kwallet-pam            # kio needs it; so do your Proton packages
    kio kio-extras kwindowsystem kconfig kiconthemes
    breeze-icons                   # icon fallback for every KDE app
    sddm cachyos-themes-sddm       # the login manager stays
    hyprland waybar rofi swaync swayosd wlogout hyprlock hypridle hyprpaper
    hyprpolkitagent xdg-desktop-portal-hyprland xdg-desktop-portal-gtk
    pipewire wireplumber networkmanager polkit
    qt5-wayland qt6-wayland xorg-xwayland
)

# --- preflight ---------------------------------------------------------------

info "Checking that Hyprland can stand on its own"

fatal=0

if [[ "${XDG_CURRENT_DESKTOP:-}" != "Hyprland" ]]; then
    err "not running under Hyprland (XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-unset})"
    err "log into Hyprland and run this from there — removing your fallback from"
    err "inside the thing you are falling back to is how people end up at a TTY"
    fatal=1
else
    ok "running under Hyprland"
fi

# Everything Plasma was quietly providing needs a live replacement.
check_running() {
    local label="$1" pattern="$2"
    if pgrep -x "$pattern" >/dev/null 2>&1; then
        ok "$label is running ($pattern)"
    else
        err "$label is NOT running ($pattern)"
        fatal=1
    fi
}

check_running "Bar"                  waybar
check_running "Notification daemon"  swaync
check_running "Idle manager"         hypridle

if systemctl --user is-active hyprpolkitagent &>/dev/null || pgrep -f hyprpolkitagent >/dev/null; then
    ok "Polkit agent is running"
else
    err "hyprpolkitagent is NOT running — GUI privilege prompts will silently fail"
    fatal=1
fi

if command -v hyprlock &>/dev/null; then
    ok "hyprlock is installed"
else
    err "hyprlock missing — you would have no screen lock"
    fatal=1
fi

# The portal matters more than people expect: file dialogs and screen sharing.
if pgrep -f xdg-desktop-portal-hyprland >/dev/null; then
    ok "Hyprland portal is running"
else
    warn "xdg-desktop-portal-hyprland is not running — screen sharing will break"
    warn "(it is D-Bus activated, so this may just mean nothing has asked yet)"
fi

# xdg-desktop-portal-kde is a dependency of plasma-workspace and will go with
# it. That is fine — hyprland;gtk is what hyprland-portals.conf asks for — but
# only if the file chooser is not pointed at kde.
portals="$HOME/.config/xdg-desktop-portal/hyprland-portals.conf"
if [[ -f "$portals" ]] && grep -qE '^\s*org\.freedesktop\.impl\.portal\.FileChooser\s*=\s*kde' "$portals"; then
    err "your portal config asks for the KDE file chooser, which is about to be removed"
    err "change FileChooser to gtk in $portals first"
    fatal=1
else
    ok "portal config does not depend on the KDE backend"
fi

if ((fatal)); then
    err "refusing to continue"
    exit 1
fi

# --- secrets -----------------------------------------------------------------

info "Secrets"

if pacman -Q python-proton-keyring-linux &>/dev/null; then
    ok "Proton packages use KWallet — kwallet is on the protected list and stays"
fi
if [[ -f "$HOME/.local/share/kwalletd/kdewallet.kwl" ]]; then
    ok "existing wallet found; kwallet and kwallet-pam are kept so it keeps working"
    skip "auto-unlock at login comes from pam_kwallet_init, which dex already starts"
fi

# --- build the removal list --------------------------------------------------

info "Working out what would be removed"

mapfile -t targets < <(plasma_targets)
ok "${#targets[@]} Plasma packages found via the plasma group"

if ((DO_APPS)); then
    for p in "${APP_PKGS[@]}"; do
        pacman -Q "$p" &>/dev/null && targets+=("$p")
    done
    ok "KDE applications included (--apps)"
fi

if ((${#targets[@]} == 0)); then
    ok "nothing to remove — Plasma looks gone already"
    exit 0
fi

# -Rsu: remove these, plus dependencies that nothing else needs, but never
# anything that is still required. --print asks pacman without touching a thing.
mapfile -t plan < <(pacman -Rsu --print --print-format '%n' "${targets[@]}" 2>/dev/null)

if ((${#plan[@]} == 0)); then
    err "pacman could not resolve a removal plan; run it by hand to see why:"
    err "  sudo pacman -Rsu ${targets[*]}"
    exit 1
fi

# Safety net: if the plan touches anything protected, stop.
violations=()
for p in "${plan[@]}"; do
    for guard in "${PROTECTED[@]}"; do
        [[ "$p" == "$guard" ]] && violations+=("$p")
    done
done

if ((${#violations[@]} > 0)); then
    err "the removal plan includes packages this desktop needs:"
    printf '      %s\n' "${violations[@]}"
    err "refusing to continue — remove the offending entry from SESSION_PKGS"
    exit 1
fi
ok "plan does not touch anything protected (${#PROTECTED[@]} guarded)"

printf '\n%s%d packages would be removed:%s\n\n' "$c_bold" "${#plan[@]}" "$c_reset"
printf '%s\n' "${plan[@]}" | column -c "${COLUMNS:-100}"
printf '\n'

if ((DO_APPS)); then
    mapfile -t replacements < <(sed 's/#.*//' "$REPO/packages/gtk-replacements.txt" | awk 'NF')
    printf '%sand %d GTK replacements would be installed first:%s\n\n' \
        "$c_bold" "${#replacements[@]}" "$c_reset"
    printf '      %s\n' "${replacements[*]}"
    printf '\n'
    warn "⌘E picks the first installed file manager, so it follows the swap by itself"
fi

if ! ((DO_RUN)); then
    info "Dry run. Re-run with --run to do it for real."
    exit 0
fi

# --- confirm and go ----------------------------------------------------------

printf '%sType REMOVE to proceed: %s' "$c_bold" "$c_reset"
read -r answer
[[ "$answer" == "REMOVE" ]] || { info "Nothing done."; exit 0; }

# Install the replacements BEFORE removing anything, so a failure here leaves
# you with a working desktop rather than no file manager.
if ((DO_APPS)); then
    info "Installing replacements first"
    if ! sudo pacman -S --needed --noconfirm "${replacements[@]}"; then
        err "replacement install failed — nothing removed"
        exit 1
    fi
    ok "replacements installed"
fi

info "Removing"
if ! sudo pacman -Rsu "${targets[@]}"; then
    err "pacman failed — nothing further attempted"
    exit 1
fi
ok "Plasma session removed"

# --- pick up the pieces ------------------------------------------------------

info "Cleaning up after it"

# plasma-apply-colorscheme went with plasma-workspace. Re-apply the scheme with
# the standalone merger so KDE apps keep their colours.
scheme="$REPO/config/color-schemes/CatppuccinMocha.colors"
if [[ -f "$scheme" ]] && command -v python3 &>/dev/null; then
    python3 "$REPO/config/color-schemes/apply-colors.py" "$scheme" && \
        ok "KDE app colours re-applied without plasma-apply-colorscheme"
fi

# SDDM's session list still advertises a Plasma entry until its desktop files go.
leftovers=$(ls /usr/share/wayland-sessions/plasma*.desktop \
               /usr/share/xsessions/plasma*.desktop 2>/dev/null | wc -l)
if ((leftovers > 0)); then
    warn "$leftovers Plasma session file(s) still in /usr/share — pacman usually"
    warn "removes these; check with: ls /usr/share/wayland-sessions/"
fi

cat <<EOF

$c_bold Done.$c_reset

  Plasma is gone. Hyprland is now the only session, so there is no fallback:
  if it fails to start you get a TTY. Ctrl+Alt+F2 gets you one, and
  \`Hyprland\` from there will tell you why it did not come up.

  Config left behind on purpose, harmless but yours to delete:
    ~/.config/plasma*        ~/.config/k*rc        ~/.local/share/plasma*

  Kept deliberately: kwallet (kio and your Proton packages need it),
  breeze-icons (icon fallback), SDDM, and every KDE application$( ((DO_APPS)) && echo " except the ones just removed").

EOF
