#!/usr/bin/env bash
#
# Edit QML on the host, watch the guest's desktop change.
#
#     tools/live.sh                 sync shell/ and hold, reloading on save
#     tools/live.sh --once          sync once, start the shell, exit
#     tools/live.sh --env=K=V       set a variable for the shell. Repeatable.
#                                   A change of environment restarts the shell.
#     tools/live.sh --headed        boot the VM in a window first, if it is down
#     tools/live.sh --keep          leave the shell running on exit
#     tools/live.sh --interval=SEC  how often the host looks. Default 0.25.
#
# --headed is the interactive loop: a QEMU window to watch, rather than the
# headless VM every model-driven run and every shot uses. The guest takes the
# window's size, so it is NOT the 2560x1080 the shot matrix is taken at —
# what you see here is the behaviour, not the layout that gets scored.
#
# RUNS ON THE HOST. Quickshell reloads its own config when a file under it
# changes, so this reloads nothing: it puts the change where the guest's
# inotify can see it.
#
# NOT /repo/shell, which is where the guest normally runs the shell from. That
# path is the host repo over 9p and a host write there raises no inotify event
# in the guest — the shell sits at the old version until it is killed. Neither
# does a `touch` of the same file from inside the guest. The tree is copied to
# a guest-local ext4 path, which is also the path a real machine installs to.
#
# Only files whose mtime or size moved since the last look are sent, so one
# saved file is one write in the guest and one reload.
#
# THE SHELL IS KILLED ON EXIT unless --keep. tools/shot/shot.sh reuses any
# running `quickshell` it finds, and a shell left up here is running the copy
# at $TARGET rather than /repo/shell — a shot taken against it is evidence
# about a tree the repo does not have.

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$REPO/shell"
GUEST="user@localhost"
SSH_PORT=2222
SSH_OPTS=(-p "$SSH_PORT" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null
          -o BatchMode=yes -o ConnectTimeout=8 -o LogLevel=ERROR)

# The config directory in the guest, and the file that says this tool owns it.
TARGET="/home/user/.config/quickshell/isle-live"
MARKER=".hypr-live"
LOG=/tmp/hypr-live.log
ENVFILE=/tmp/hypr-live.env
WARMUP=15           # seconds to wait at most for the shell to answer

once=0; keep=0; headed=0; interval=0.25
declare -a envs=()
for a in "$@"; do case "$a" in
    --once)       once=1 ;;
    --keep)       keep=1 ;;
    --headed)     headed=1 ;;
    --env=*)      envs+=("${a#*=}") ;;
    --interval=*) interval="${a#*=}" ;;
    --path=*)     TARGET="${a#*=}" ;;
    -h|--help)    sed -n '3,35p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;;
    *)            echo "unknown option: $a" >&2; exit 2 ;;
esac; done

c_g=$'\e[32m'; c_r=$'\e[31m'; c_y=$'\e[33m'; c_b=$'\e[34m'; c_d=$'\e[2m'; c_0=$'\e[0m'
info() { printf '%s==>%s %s\n' "$c_b" "$c_0" "$*"; }
ok()   { printf '  %s✓%s %s\n' "$c_g" "$c_0" "$*"; }
err()  { printf '  %s✗%s %s\n' "$c_r" "$c_0" "$*" >&2; }
warn() { printf '  %s!%s %s\n' "$c_y" "$c_0" "$*"; }
note() { printf '    %s%s%s\n' "$c_d" "$*" "$c_0"; }

guest() { ssh "${SSH_OPTS[@]}" "$GUEST" "$@"; }

# Same match as tools/test-vm.sh: the executable, not the command line, so this
# does not find the shell that is asking.
vm_pid() { ps -eo pid=,args= | awk '$2 ~ /qemu-system-x86_64$/ && /cachyos-hypr-test/ {print $1; exit}'; }

# --- the VM ------------------------------------------------------------------

if ((headed)); then
    pid=$(vm_pid)
    if [[ -z "$pid" ]]; then
        info "Booting the guest in a window"
        nohup "$REPO/tools/test-vm.sh" run --headed > /tmp/hypr-live-vm.log 2>&1 &
        for _ in $(seq 1 40); do
            guest true 2>/dev/null && break
            sleep 5
        done
        guest true 2>/dev/null || {
            err "the guest did not come up in 200s"
            tail -5 /tmp/hypr-live-vm.log
            exit 1
        }
        ok "up  (tools/test-vm.sh stop when you are done with it)"
    elif ! tr '\0' ' ' < "/proc/$pid/cmdline" | grep -q -- '-display gtk'; then
        warn "the VM running (pid $pid) is headless — there is no window to watch"
        note "tools/test-vm.sh stop && tools/live.sh --headed"
    else
        ok "the VM is already up in a window (pid $pid)"
    fi
fi

# Everything in the guest needs the session's environment; an ssh shell has
# none of it. `ls -t` newest first: a restarted compositor leaves its old
# instance directories behind and the live one is not the last alphabetically.
ENV_PREAMBLE='export XDG_RUNTIME_DIR=/run/user/$(id -u)
export WAYLAND_DISPLAY=$(ls $XDG_RUNTIME_DIR | grep -m1 "^wayland-[0-9]*$")
export HYPRLAND_INSTANCE_SIGNATURE=$(ls -t $XDG_RUNTIME_DIR/hypr 2>/dev/null | head -1)'

# --- preflight ---------------------------------------------------------------

guest true 2>/dev/null || {
    err "cannot reach the guest on port $SSH_PORT"
    note "tools/test-vm.sh run     to boot it"
    exit 1
}
[[ -d "$SRC" ]] || { err "no $SRC"; exit 1; }

# A first sync clears the target, so it must be one this tool made.
state=$(guest "if [ ! -e '$TARGET' ]; then echo absent
               elif [ -e '$TARGET/$MARKER' ]; then echo ours
               else echo foreign; fi" 2>/dev/null)
if [[ "$state" == foreign ]]; then
    err "$TARGET exists in the guest and is not this tool's"
    note "move it aside, or pass --path=<somewhere else>"
    exit 1
fi

# --- the file list -----------------------------------------------------------

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
prev="$tmp/prev"; cur="$tmp/cur"

# Path, mtime and size per file. mtime is fractional, so a save inside the
# same second is still a change.
scan() { (cd "$SRC" && find . -type f -printf '%p\t%T@\t%s\n' | sort); }

# --- sync --------------------------------------------------------------------

# Sends the named files and removes the named paths. Both lists may be empty.
push() {
    local -n _changed=$1 _gone=$2
    if ((${#_changed[@]})); then
        printf '%s\n' "${_changed[@]}" \
            | tar cf - -C "$SRC" --no-recursion -T - 2>/dev/null \
            | guest "mkdir -p '$TARGET' && tar xf - -C '$TARGET'" || return 1
    fi
    if ((${#_gone[@]})); then
        local args=""
        for f in "${_gone[@]}"; do args+=" $(printf '%q' "$f")"; done
        guest "cd '$TARGET' && rm -f$args" || return 1
    fi
}

info "First sync"
guest "rm -rf '$TARGET' && mkdir -p '$TARGET' && touch '$TARGET/$MARKER'" || exit 1
scan > "$prev"
mapfile -t all < <(cut -f1 "$prev")
gone=()
push all gone || { err "sync failed"; exit 1; }
ok "$TARGET  ${#all[@]} files"

# --- the shell ---------------------------------------------------------------

want_env=""
for e in $(printf '%s\n' "${envs[@]:-}" | grep -v '^$' | sort); do want_env+="$e "; done

# One shell at a time: two would draw the same island on the same output.
start_shell() {
    guest "$ENV_PREAMBLE
        pkill -f scripts/isle-session 2>/dev/null
        pkill -x quickshell 2>/dev/null; sleep 0.5
        printf '%s' '$want_env' > $ENVFILE
        $want_env nohup quickshell -p '$TARGET' > $LOG 2>&1 &
        for _ in \$(seq 1 $((WARMUP * 10))); do
            qs -p '$TARGET' ipc show >/dev/null 2>&1 && exit 0
            sleep 0.1
        done
        exit 1" 2>/dev/null
}

# The session runs the shell as isle.service, which has Restart=always:
# killing the process just makes systemd bring it back a second later. The
# unit is stopped for the run and started again on the way out.
unit_was_active=$(guest "systemctl --user is-active --quiet isle && echo yes" 2>/dev/null)

info "Starting the shell"
if start_shell; then
    ok "up${want_env:+  $want_env}"
else
    err "the shell did not answer in ${WARMUP}s — its log:"
    guest "grep -v MESA $LOG | tail -20" 2>/dev/null
fi
note "qs -p $TARGET ipc call <surface> <state>"

# Only the instance this tool started: `pkill -x quickshell` would also take
# a shell someone else has running from /repo/shell.
stop_shell() {
    ((keep)) && { echo; note "left running: qs -p $TARGET ipc call ..."; return; }
    guest "for p in \$(pgrep -x quickshell); do
               tr '\\0' ' ' < /proc/\$p/cmdline | grep -q -- '$TARGET' && kill \$p
           done" 2>/dev/null
    if [[ "$unit_was_active" == yes ]]; then
        guest "systemctl --user start isle" 2>/dev/null
        echo; note "shell stopped, isle.service started again"
    else
        echo; note "shell stopped"
    fi
}

# --- the log -----------------------------------------------------------------

# Bytes of $LOG already printed. Everything after them is this reload's.
off=0

# Waits up to 3s for the load to land, then prints what the shell said.
drain() {
    local verb="${1:-reloaded}" text="" end=$((SECONDS + 3))
    while ((SECONDS < end)); do
        text=$(guest "tail -c +$((off + 1)) $LOG 2>/dev/null" 2>/dev/null)
        [[ "$text" == *"Configuration Loaded"* || "$text" == *ERROR* ]] && break
        sleep 0.2
    done
    local size; size=$(guest "wc -c < $LOG 2>/dev/null || echo $off" 2>/dev/null | tr -d ' \r\n')
    [[ "$size" =~ ^[0-9]+$ ]] && off="$size"

    printf '%s\n' "$text" | grep -v 'MESA\|^$' | while IFS= read -r line; do
        case "$line" in
            *ERROR*)   printf '    %s%s%s\n' "$c_r" "$line" "$c_0" ;;
            *WARN*)    printf '    %s%s%s\n' "$c_y" "$line" "$c_0" ;;
            *qml:*)    printf '    %s\n' "$line" ;;
            *Reloading*|*"Configuration Loaded"*) ;;
            *)         printf '    %s%s%s\n' "$c_d" "$line" "$c_0" ;;
        esac
    done
    case "$text" in
        *ERROR*)                  printf '  %s✗%s reload failed — the shell kept the last good config\n' "$c_r" "$c_0" ;;
        *"Configuration Loaded"*) printf '  %s✓%s %s\n' "$c_g" "$c_0" "$verb" ;;
        *)                        printf '  %s!%s nothing came back\n' "$c_y" "$c_0" ;;
    esac
}

drain loaded
((once)) && { stop_shell; exit 0; }

# --- watch -------------------------------------------------------------------

trap 'stop_shell; exit 0' INT TERM
info "Watching $SRC — ctrl-c to stop"

while sleep "$interval"; do
    scan > "$cur"
    cmp -s "$prev" "$cur" && continue

    mapfile -t changed < <(comm -13 "$prev" "$cur" | cut -f1)
    mapfile -t gone < <(comm -23 <(cut -f1 "$prev") <(cut -f1 "$cur"))
    mv "$cur" "$prev"

    printf '%s%s%s  ' "$c_d" "$(date +%H:%M:%S)" "$c_0"
    ((${#changed[@]})) && printf '%s' "$(printf '%s ' "${changed[@]#./}")"
    ((${#gone[@]}))    && printf '%s-%s%s ' "$c_r" "$(printf '%s ' "${gone[@]#./}")" "$c_0"
    echo

    push changed gone || { err "sync failed"; continue; }

    # A config broken badly enough to stop the process needs a start, not a
    # reload; a reload error leaves the last good config running.
    if ! guest "pgrep -x quickshell >/dev/null" 2>/dev/null; then
        printf '  %s!%s the shell is not running — restarting\n' "$c_y" "$c_0"
        start_shell || err "it did not come back"
        off=0
    fi
    drain
done
