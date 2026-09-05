#!/usr/bin/env bash
#
# Check the parts of this desktop that can only be verified while it is running.
#
#     ./doctor.sh
#
# tests/test-config.lua validates the config statically — that every API call
# exists and the rules are well-formed. This is the other half: whether the
# things that are supposed to be running are running, whether the widgets
# actually emit what Waybar expects, and whether the window classes the rules
# match on are the classes your applications really have.
#
# Read-only apart from one do-not-disturb round trip, which it puts back.

set -uo pipefail

pass=0 warn=0 fail=0

c_reset=$'\e[0m'; c_bold=$'\e[1m'; c_dim=$'\e[2m'
c_green=$'\e[32m'; c_yellow=$'\e[33m'; c_red=$'\e[31m'; c_blue=$'\e[34m'

section() { printf '\n%s==>%s %s\n' "$c_blue$c_bold" "$c_reset" "$*"; }
ok()   { printf '  %s✓%s %s\n' "$c_green" "$c_reset" "$*"; ((pass++)); }
warn() { printf '  %s!%s %s\n' "$c_yellow" "$c_reset" "$*"; ((warn++)); }
bad()  { printf '  %s✗%s %s\n' "$c_red" "$c_reset" "$*"; ((fail++)); }
note() { printf '    %s%s%s\n' "$c_dim" "$*" "$c_reset"; }

# --- is this even Hyprland ---------------------------------------------------

section "Session"

if ! command -v hyprctl >/dev/null || ! hyprctl version >/dev/null 2>&1; then
    bad "Hyprland is not running — run this from inside the session"
    exit 1
fi
ok "Hyprland $(hyprctl version -j 2>/dev/null | jq -r '.tag // "?"')"

# The single most valuable check here: Hyprland collects config errors rather
# than refusing to start, so a broken line is invisible until you look.
errors=$(hyprctl configerrors 2>/dev/null)
if [[ -z "$errors" || "$errors" == "no errors" ]]; then
    ok "config loaded with no errors"
else
    bad "config errors:"
    sed 's/^/      /' <<< "$errors"
fi

# --- what should be running --------------------------------------------------

section "Processes"

check_proc() {
    local label="$1" pattern="$2" severity="${3:-fail}"
    if pgrep -f "$pattern" >/dev/null 2>&1; then
        ok "$label"
    elif [[ "$severity" == "warn" ]]; then
        warn "$label is not running"
    else
        bad "$label is not running"
    fi
}

check_proc "Waybar"                 '^waybar'
check_proc "Notifications (swaync)" 'swaync'
check_proc "Idle manager"           'hypridle'
check_proc "Wallpaper"              'hyprpaper'
check_proc "Volume OSD"             'swayosd-server'
check_proc "Polkit agent"           'hyprpolkitagent'
check_proc "Clipboard watcher"      'wl-paste.*cliphist'
check_proc "Secret Service"         'gnome-keyring-daemon' warn
check_proc "Network applet"         'nm-applet' warn
check_proc "Night light"            'hyprsunset' warn

# --- display -----------------------------------------------------------------

section "Display"

mon=$(hyprctl monitors -j 2>/dev/null)
if [[ -n "$mon" ]]; then
    name=$(jq -r '.[0].name' <<< "$mon")
    desc=$(jq -r '.[0].description // ""' <<< "$mon")
    w=$(jq -r '.[0].width' <<< "$mon"); h=$(jq -r '.[0].height' <<< "$mon")
    rr=$(jq -r '.[0].refreshRate' <<< "$mon")
    ok "$name ${w}x${h}@${rr%.*}Hz"
    note "$desc"

    if [[ "$w" == "3440" && "$h" == "1440" ]]; then
        printf -v rr_int '%.0f' "$rr"
        ((rr_int >= 140)) && ok "running at the panel's full refresh rate" \
                          || warn "only ${rr_int}Hz — monitors.lua asks for 144"
    fi
    [[ "$(jq -r '.[0].vrr' <<< "$mon")" != "0" ]] \
        && ok "VRR active" || note "VRR idle (fullscreen-only, so this is normal on the desktop)"
fi

# The sysfs parameter is root-only readable, so prove it the other way:
# nvidia_drm only registers DRM connectors when modeset is enabled.
modeset="?"
if value=$(cat /sys/module/nvidia_drm/parameters/modeset 2>/dev/null) && [[ -n "$value" ]]; then
    modeset="$value"
else
    for card in /sys/class/drm/card[0-9]; do
        [[ -e "$card/device/driver" ]] || continue
        [[ "$(basename "$(readlink -f "$card/device/driver")")" == nvidia ]] || continue
        compgen -G "$card-*" >/dev/null && { modeset="Y"; break; }
    done
fi
[[ "$modeset" == "Y" ]] && ok "nvidia_drm modeset enabled" \
                        || bad "nvidia_drm modeset could not be confirmed (read '$modeset')"

gpu=$(hyprctl getoption env 2>/dev/null | grep -o '/dev/dri/by-path/[^ ,]*' | head -1)
gpu="${gpu:-/dev/dri/by-path/pci-0000:01:00.0-card}"
[[ -e "$gpu" ]] && ok "primary GPU node exists ($(basename "$gpu"))" \
                || bad "$gpu missing — AQ_DRM_DEVICES in env.lua is wrong for this machine"

# --- keyboard ----------------------------------------------------------------

section "Keyboard"

kb=$(hyprctl devices -j 2>/dev/null | jq -r '.keyboards[] | select(.main == true) | "\(.layout)  \(.rules)"' | head -1)
note "main keyboard: ${kb:-unknown}"
if grep -q hypr-mac-profile /etc/keyd/default.conf 2>/dev/null; then
    ok "keyd Mac profile installed — ⌘C/⌘V work inside apps"
    systemctl is-active keyd >/dev/null 2>&1 && ok "keyd running" || bad "keyd installed but not running"
else
    opts=$(hyprctl getoption input:kb_options -j 2>/dev/null | jq -r '.str // ""')
    [[ "$opts" == *swap_alt_win* ]] \
        && ok "Alt/Super swapped in xkb — the ⌘ cap sends SUPER" \
        || warn "no Alt/Super swap active; Mac keycaps will be off by one modifier"
fi

binds=$(hyprctl binds -j 2>/dev/null | jq 'length')
described=$(hyprctl binds -j 2>/dev/null | jq '[.[] | select((.description // "") != "")] | length')
ok "$binds keybinds registered, $described described (⌘? cheatsheet reads these)"

# --- widgets -----------------------------------------------------------------

section "Waybar modules"

for script in waybar-agents.sh:claude waybar-agents.sh:codex waybar-ai-app.sh:claude \
              waybar-ai-app.sh:chatgpt waybar-updates.sh: waybar-gpu.sh: \
              waybar-cputemp.sh: waybar-gamemode.sh:; do
    file="${script%%:*}"; arg="${script##*:}"
    path="$HOME/.config/hypr/scripts/$file"
    [[ -x "$path" ]] || { bad "$file is not executable"; continue; }
    out=$("$path" $arg 2>/dev/null)
    if jq -e . >/dev/null 2>&1 <<< "$out"; then
        ok "$file ${arg:+$arg }→ $(jq -rc '{text,class}' <<< "$out")"
    else
        bad "$file ${arg:+$arg }produced invalid JSON"
    fi
done

# --- notifications and DND ---------------------------------------------------

section "Do not disturb"

if command -v swaync-client >/dev/null; then
    before=$(swaync-client --get-dnd 2>/dev/null)
    swaync-client --dnd-on >/dev/null 2>&1
    during=$(swaync-client --get-dnd 2>/dev/null)
    [[ "$during" == "true" ]] && ok "DND can be toggled on" || bad "DND did not turn on"
    [[ "$before" == "true" ]] || swaync-client --dnd-off >/dev/null 2>&1
    after=$(swaync-client --get-dnd 2>/dev/null)
    [[ "$after" == "$before" ]] && ok "DND restored to '$before'" || warn "DND left as '$after'"
    note "dnd.lua drives this from screenshare.state and window.fullscreen"
else
    bad "swaync-client missing"
fi

# --- agents ------------------------------------------------------------------

section "Coding agents"

marker="hypr/scripts/hooks/agent-state.sh"
if jq -e --arg m "$marker" '[.hooks // {} | .[][]? | .hooks[]?.command // ""] | any(contains($m))' \
       "$HOME/.claude/settings.json" >/dev/null 2>&1; then
    n=$(jq --arg m "$marker" '[.hooks // {} | to_entries[] | select([.value[].hooks[]?.command // ""] | any(contains($m)))] | length' "$HOME/.claude/settings.json")
    ok "Claude Code hooks installed ($n events)"
else
    warn "Claude Code hooks not found in ~/.claude/settings.json"
fi

if jq -e --arg m "$marker" '[.hooks // {} | .[][]? | .hooks[]?.command // ""] | any(contains($m))' \
       "$HOME/.codex/hooks.json" >/dev/null 2>&1; then
    ok "Codex hooks written to ~/.codex/hooks.json"
    note "Codex will not run them until trusted — run /hooks inside Codex"
else
    warn "Codex hooks not found in ~/.codex/hooks.json"
fi

state="${XDG_RUNTIME_DIR:-/tmp}/hypr-ai"
if [[ -d "$state" ]]; then
    live=$(find "$state" -name '*.json' 2>/dev/null | wc -l)
    ok "agent state directory present ($live session file(s))"
    note "if this stays empty while an agent runs, the hooks are not firing"
else
    note "no agent state yet — it appears when a session starts"
fi

"$HOME/.config/hypr/scripts/agent-busy.sh" >/dev/null 2>&1 \
    && ok "idle deferral: nothing working, lock allowed" \
    || ok "idle deferral: an agent is working, lock deferred"

# --- window classes ----------------------------------------------------------
#
# The rules match on class. These are guesses for apps that were not running
# when the rules were written, so print the truth.

section "Window classes (what the rules match against)"

clients=$(hyprctl clients -j 2>/dev/null)
if [[ "$(jq 'length' <<< "$clients")" == "0" ]]; then
    note "no windows open — open Claude, ChatGPT and a game, then re-run"
else
    jq -r '.[] | "    \(.class)   —   \(.title[0:48])"' <<< "$clients" | sort -u
    for expect in com.anthropic.Claude chatgpt proton-mail; do
        if jq -e --arg c "$expect" 'any(.[]; (.class // "") | ascii_downcase == ($c | ascii_downcase))' <<< "$clients" >/dev/null; then
            ok "class '$expect' confirmed — rules for it will match"
        fi
    done
fi

# --- portals -----------------------------------------------------------------

section "Portals"

for backend in hyprland gtk kde; do
    pgrep -f "xdg-desktop-portal-$backend" >/dev/null 2>&1 && note "running: xdg-desktop-portal-$backend"
done
pgrep -f xdg-desktop-portal-hyprland >/dev/null 2>&1 \
    && ok "Hyprland portal running (screen sharing, screenshots)" \
    || warn "Hyprland portal not running — it is D-Bus activated, so open a share to test"

# --- summary -----------------------------------------------------------------

printf '\n%s%d passed, %d warnings, %d failed%s\n' \
    "$c_bold" "$pass" "$warn" "$fail" "$c_reset"

# --- are you ready to drop the fallback? -------------------------------------
#
# The whole point of keeping Plasma is to have somewhere to land while Hyprland
# is unproven. This is the signal for when that stops being true.

section "Login sessions"

mapfile -t offered < <(
    for d in /usr/local/share/wayland-sessions /usr/share/wayland-sessions; do
        [[ -d "$d" ]] || continue
        for f in "$d"/*.desktop; do
            [[ -f "$f" ]] || continue
            base=$(basename "$f")
            # A mask in the higher-precedence dir hides the system one.
            if [[ -f "/usr/local/share/wayland-sessions/$base" ]] &&
               grep -qiE '^(Hidden|NoDisplay)=true' "/usr/local/share/wayland-sessions/$base"; then
                continue
            fi
            grep -qiE '^(Hidden|NoDisplay)=true' "$f" && continue
            printf '%s\n' "$base"
        done
    done | sort -u
)
for s in "${offered[@]}"; do note "SDDM offers: $s"; done

if printf '%s\n' "${offered[@]}" | grep -q '^hyprland-uwsm.desktop$'; then
    warn "the uwsm entry is still offered, and uwsm $(pacman -Q uwsm &>/dev/null && echo 'is installed' || echo 'is NOT installed — picking it would fail')"
    note "re-run ./install.sh, or see the NoExtract fallback in the README"
else
    ok "only one Hyprland session offered"
fi

section "Plasma fallback"

if ! pacman -Q plasma-desktop &>/dev/null; then
    ok "already removed — Hyprland is the only session"
elif ((fail > 0)); then
    warn "still installed, and rightly so — fix the $fail failure(s) above first"
    note "it is one logout away at the SDDM session menu"
else
    days_installed=""
    if [[ -d "$HOME/.config/hypr" ]]; then
        age=$(( ( $(date +%s) - $(stat -c %Y "$HOME/.config/hypr" 2>/dev/null || date +%s) ) / 86400 ))
        days_installed=" (installed ~${age}d ago)"
    fi
    ok "everything passing${days_installed}"
    note "when you have lived with it long enough to trust it:"
    note "    ./remove-plasma.sh                        # dry run, shows the plan"
    note "    ./migrate-secrets.py --run                # move KWallet into gnome-keyring"
    note "    ./remove-plasma.sh --run --apps --drop-kwallet"
    note "there is no fallback session afterwards — Ctrl+Alt+F2 gets you a TTY"
fi

printf '\n'
((fail == 0))
