#!/usr/bin/env bash
#
# Spin up a CachyOS VM to test this repo against a machine that is not yours.
#
#     tools/test-vm.sh create     download the ISO and make a disk
#     tools/test-vm.sh install    boot the installer
#     tools/test-vm.sh run        boot the installed system
#     tools/test-vm.sh run --headed   ... in a QEMU window you can watch
#     tools/test-vm.sh ssh        shell into it (after setup inside the guest)
#     tools/test-vm.sh shot [f]   screenshot the guest, no GUI access needed
#     tools/test-vm.sh stop       shut the guest down
#     tools/test-vm.sh save       snapshot the disk as a known-good baseline
#     tools/test-vm.sh reset      restore that snapshot — seconds, not minutes
#     tools/test-vm.sh clean      delete the disk, keep the ISO
#
# The VM is driven, not watched. tools/vm-qmp.py sends keys, clicks and takes
# screenshots over QEMU's QMP socket, so the guest can be installed and the
# shell verified without anyone looking at the QEMU window. `run` and
# `install` are headless for that reason; `--headed` opens a GTK window as
# well, which is what tools/live.sh is worth watching in.
#
# QEMU/KVM rather than VirtualBox: AMD-V is already live in this kernel, and
# VirtualBox would want DKMS modules built against the CachyOS kernel for
# worse Wayland support.
#
# The point of this is not "does Hyprland run" — it is whether install.sh
# reproduces a desktop on hardware that shares nothing with the machine it was
# written on. Different GPU, different monitor name, no NVIDIA, no KDE to
# migrate from, none of the packages already present.

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VM_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/hypr-testvm"
DISK="$VM_DIR/cachyos.qcow2"
ISO_DIR="$VM_DIR/iso"
NVRAM="$VM_DIR/OVMF_VARS.fd"
QMP="$VM_DIR/qmp.sock"
SSH_PORT=2222

DISPLAY_MODE=headless   # `--headed` on run or install
DISK_SIZE="60G"
MEMORY="8G"
CORES="6"

c_reset=$'\e[0m'; c_bold=$'\e[1m'; c_dim=$'\e[2m'
c_green=$'\e[32m'; c_yellow=$'\e[33m'; c_red=$'\e[31m'; c_blue=$'\e[34m'
info() { printf '%s==>%s %s\n' "$c_blue$c_bold" "$c_reset" "$*"; }
ok()   { printf '  %s✓%s %s\n' "$c_green" "$c_reset" "$*"; }
warn() { printf '  %s!%s %s\n' "$c_yellow" "$c_reset" "$*"; }
err()  { printf '  %s✗%s %s\n' "$c_red" "$c_reset" "$*" >&2; }
note() { printf '    %s%s%s\n' "$c_dim" "$*" "$c_reset"; }

# --- preflight ---------------------------------------------------------------

preflight() {
    local fatal=0

    if [[ ! -e /dev/kvm ]]; then
        err "/dev/kvm missing — enable SVM/AMD-V in the BIOS"
        fatal=1
    elif [[ ! -r /dev/kvm || ! -w /dev/kvm ]]; then
        err "no access to /dev/kvm"
        note "sudo usermod -aG kvm $USER   then log out and back in"
        fatal=1
    else
        ok "KVM available"
    fi

    local missing=()
    for cmd in qemu-system-x86_64 qemu-img; do
        command -v "$cmd" >/dev/null || missing+=("$cmd")
    done
    if ((${#missing[@]})); then
        err "missing: ${missing[*]}"
        note "sudo pacman -S --needed qemu-desktop edk2-ovmf"
        fatal=1
    else
        ok "QEMU present"
    fi

    # OVMF, because CachyOS installs UEFI by default and a BIOS guest would be
    # testing a boot path you will never use.
    for f in /usr/share/edk2/x64/OVMF_CODE.4m.fd /usr/share/edk2-ovmf/x64/OVMF_CODE.fd \
             /usr/share/OVMF/OVMF_CODE.fd; do
        [[ -f "$f" ]] && { OVMF_CODE="$f"; break; }
    done
    if [[ -z "${OVMF_CODE:-}" ]]; then
        err "no OVMF firmware found — sudo pacman -S edk2-ovmf"
        fatal=1
    else
        ok "UEFI firmware: $(basename "$OVMF_CODE")"
    fi

    ((fatal)) && exit 1
}

# The QEMU window, headless by default. VNC stays up in both modes.
display_args() {
    if [[ "$DISPLAY_MODE" == headed ]]; then
        printf '%s\n' -display gtk,zoom-to-fit=on -vnc 127.0.0.1:9
    else
        printf '%s\n' -display none -vnc 127.0.0.1:9
    fi
}

# --headed on a host with no session, or a QEMU built without GTK, fails
# inside QEMU with the disk already locked.
want_headed() {
    DISPLAY_MODE=headed
    qemu-system-x86_64 -display help 2>/dev/null | grep -qx gtk || {
        err "this QEMU has no gtk display backend"
        note "sudo pacman -S --needed qemu-ui-gtk"
        exit 1
    }
    [[ -n "${WAYLAND_DISPLAY:-}${DISPLAY:-}" ]] || {
        err "--headed needs a graphical session; WAYLAND_DISPLAY and DISPLAY are both unset"
        exit 1
    }
}

parse_flags() {
    for a in "$@"; do case "$a" in
        --headed)   want_headed ;;
        --headless) DISPLAY_MODE=headless ;;
        *)          err "unknown option: $a"; exit 2 ;;
    esac; done
}

ovmf_vars() {
    for f in /usr/share/edk2/x64/OVMF_VARS.4m.fd /usr/share/edk2-ovmf/x64/OVMF_VARS.fd \
             /usr/share/OVMF/OVMF_VARS.fd; do
        [[ -f "$f" ]] && { echo "$f"; return; }
    done
}

# --- qemu invocation ---------------------------------------------------------

# Match the executable, not the command line. `pgrep -f "qemu-system-x86_64
# -name cachyos-hypr-test"` also matches the shell that is asking, so a script
# using it to decide whether to kill the VM kills itself instead — which is
# exactly as confusing as it sounds when the VM then never starts.
vm_pid() {
    ps -eo pid=,args= | awk '$2 ~ /qemu-system-x86_64$/ && /cachyos-hypr-test/ {print $1; exit}'
}
vm_running() { [[ -n "$(vm_pid)" ]]; }
vm_kill() {
    local pid; pid="$(vm_pid)"
    [[ -n "$pid" ]] || return 0
    kill "$pid" 2>/dev/null
    for _ in $(seq 1 30); do sleep 1; [[ -n "$(vm_pid)" ]] || return 0; done
    kill -9 "$pid" 2>/dev/null; sleep 2
}

# Display, and why it is deliberately unaccelerated.
#
# Two findings, both of which made every screenshot pure black and neither of
# which announced itself:
#
#   1. `-vga none` is not optional. q35 adds a default VGA adapter, which the
#      guest drives with bochs-drm. Hyprland then enumerates TWO GPUs, makes
#      the bochs one primary and scans out to it — while QMP screendump reads
#      the virtio console. Every shot came back 2560x1080 of #000000 with a
#      live desktop sitting on the other card. It also invented a second
#      monitor, which a shell that declares one surface per screen will
#      cheerfully draw itself onto twice.
#
#   2. GL and screendump are mutually exclusive here. With `virtio-gpu-gl`
#      the scanout is a dmabuf and QEMU keeps no host-side surface: `-display
#      gtk,gl=on` screendumps black and `-display egl-headless` fails outright
#      with "no surface". Plain `virtio-gpu` gives QEMU a real pixman surface
#      that screendump can read, at the cost of running Hyprland and Quickshell
#      on llvmpipe.
#
# Software rendering is the right trade for a screenshot harness: the pixels
# are identical, only the frame rate is not, and a still is what we score. The
# visible cost is memory — llvmpipe's framebuffers put a trivial shell at
# ~320MB RSS, which is why the gauntlet's budget is what it is.
#
# `-vnc` is here because a surface QEMU never shows is a surface QEMU is free
# to skip updating. Nothing has to connect to it.
#
# `run --headed` adds `-display gtk` to that, and only that: no `gl=on`, so
# screendump keeps working while the window is open. It is how a change made
# with tools/live.sh is watched arriving.
#
# The guest resizes to the GTK window: 640x480 before it is mapped, then
# whatever the window becomes. A headed guest is therefore NOT the 2560x1080
# that xres/yres give a headless one, which is the resolution every shot and
# every layout gate is taken at. Headed is for watching behaviour; the
# default is what evidence is made on.
launch() {
    local -a boot=("$@")

    # A second QEMU on the same disk fails on the write lock — but not before
    # it has unlinked and rebound the QMP socket, leaving the FIRST vm alive
    # and unreachable. Refusing up front is cheaper than explaining that.
    if vm_running; then
        err "a test VM is already running (pid $(vm_pid))"
        note "  tools/test-vm.sh ssh     to get into it"
        note "  tools/test-vm.sh stop    to shut it down"
        exit 1
    fi
    rm -f "$QMP"

    local -a disp; mapfile -t disp < <(display_args)

    qemu-system-x86_64 \
        -name "cachyos-hypr-test" \
        -machine q35,accel=kvm \
        -cpu host \
        -smp "$CORES" \
        -m "$MEMORY" \
        -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE" \
        -drive "if=pflash,format=raw,file=$NVRAM" \
        -drive "file=$DISK,if=virtio,cache=writeback,discard=unmap" \
        -vga none \
        -device virtio-gpu,id=gpu0,xres=2560,yres=1080,max_outputs=1 \
        "${disp[@]}" \
        -device virtio-keyboard-pci \
        -device virtio-tablet-pci \
        -audiodev pipewire,id=snd0 \
        -device intel-hda -device hda-duplex,audiodev=snd0 \
        -qmp "unix:$QMP,server=on,wait=off" \
        -netdev "user,id=net0,hostfwd=tcp::${SSH_PORT}-:22" \
        -device virtio-net-pci,netdev=net0 \
        -virtfs "local,path=$REPO,mount_tag=hyprrepo,security_model=mapped-xattr,id=hyprrepo" \
        -rtc base=localtime \
        "${boot[@]}"
}

# --- commands ----------------------------------------------------------------

cmd_create() {
    preflight
    mkdir -p "$ISO_DIR"

    local iso
    iso=$(find "$ISO_DIR" -name '*.iso' -print -quit 2>/dev/null)
    if [[ -z "$iso" ]]; then
        warn "no ISO in $ISO_DIR"
        note "Download the CachyOS desktop ISO from https://cachyos.org/download/"
        note "and drop it in that directory. It is ~2.5GB, so this script does not"
        note "fetch it for you — pick your own mirror and check the signature."
        note ""
        note "  mkdir -p $ISO_DIR"
        note "  # then move the .iso there and re-run"
        exit 1
    fi
    ok "ISO: $(basename "$iso")"

    if [[ -f "$DISK" ]]; then
        warn "disk already exists: $DISK"
        note "tools/test-vm.sh clean   to start over"
    else
        qemu-img create -f qcow2 "$DISK" "$DISK_SIZE" >/dev/null
        ok "created $DISK_SIZE disk"
    fi

    [[ -f "$NVRAM" ]] || { cp "$(ovmf_vars)" "$NVRAM"; ok "UEFI variables initialised"; }

    info "Next"
    note "tools/test-vm.sh install"
}

cmd_install() {
    parse_flags "$@"
    preflight
    local iso
    iso=$(find "$ISO_DIR" -name '*.iso' -print -quit 2>/dev/null)
    [[ -n "$iso" ]] || { err "no ISO — run: tools/test-vm.sh create"; exit 1; }
    [[ -f "$DISK" ]] || { err "no disk — run: tools/test-vm.sh create"; exit 1; }

    info "Booting the installer"
    note "Install CachyOS with the Hyprland edition or a minimal desktop, then:"
    note "  1. enable sshd in the guest so 'test-vm.sh ssh' works"
    note "  2. git clone the repo and run ./install.sh"
    note "  3. log out, pick Hyprland, run ./doctor.sh"
    note ""
    note "What this is testing: the config has no NVIDIA here, the monitor is"
    note "Virtual-1 rather than DP-4, and none of the packages are pre-installed."
    note "Everything that silently depended on this machine will show up."

    launch -boot d -cdrom "$iso"
}

cmd_run() {
    parse_flags "$@"
    preflight
    [[ -f "$DISK" ]] || { err "no disk — run: tools/test-vm.sh create"; exit 1; }
    info "Booting the installed system ($DISPLAY_MODE)"
    note "ssh -p $SSH_PORT <user>@localhost   once sshd is running in the guest"
    note ""
    note "First boot, type this in the guest (user/user):"
    note "  sudo mkdir -p /repo && sudo mount -t 9p -o trans=virtio,version=9p2000.L,msize=512000 hyprrepo /repo"
    note "  /repo/tools/provision-guest.sh"
    note ""
    note "That enables sshd, installs packages/shell.txt, removes what the"
    note "shell replaces, and makes the mount persist. Everything after it"
    note "can happen over ssh or QMP — no one needs to watch this window."
    note ""
    note "Then edit on the host and the guest sees it immediately —"
    note "no rsync, no rebuild, no commit-to-test loop."
    launch
}

cmd_ssh() {
    exec ssh -p "$SSH_PORT" -o StrictHostKeyChecking=no \
             -o UserKnownHostsFile=/dev/null "${1:-$USER}@localhost"
}

# The guest is meant to be broken. Reinstalling takes six minutes; restoring a
# qcow2 internal snapshot takes about one second, so `save` once after
# provisioning and `reset` as often as you like. Anything a wave needs in the
# baseline goes in before the save, not into a list of manual steps.
SNAPSHOT="baseline"

cmd_stop() {
    if ! vm_running; then warn "no VM running"; rm -f "$QMP"; return 0; fi
    info "Stopping the guest"
    vm_kill
    rm -f "$QMP"
    vm_running && { err "still running"; exit 1; } || ok "stopped"
}

cmd_save() {
    [[ -f "$DISK" ]] || { err "no disk"; exit 1; }
    if vm_running; then
        err "shut the guest down first — snapshotting a running disk saves a torn filesystem"
        note "  tools/test-vm.sh stop"
        exit 1
    fi
    qemu-img snapshot -d "$SNAPSHOT" "$DISK" 2>/dev/null   # replace any previous
    qemu-img snapshot -c "$SNAPSHOT" "$DISK" || { err "snapshot failed"; exit 1; }
    ok "saved '$SNAPSHOT'"
    note "tools/test-vm.sh reset    to come back here"
}

cmd_reset() {
    [[ -f "$DISK" ]] || { err "no disk"; exit 1; }
    if vm_running; then
        err "shut the guest down first"
        note "  tools/test-vm.sh stop"
        exit 1
    fi
    qemu-img snapshot -l "$DISK" 2>/dev/null | grep -q "$SNAPSHOT" || {
        err "no '$SNAPSHOT' snapshot — run: tools/test-vm.sh save"
        exit 1
    }
    qemu-img snapshot -a "$SNAPSHOT" "$DISK" || { err "restore failed"; exit 1; }
    ok "restored '$SNAPSHOT'"
}

cmd_shot() {
    exec python3 "$REPO/tools/vm-qmp.py" shot "$@"
}

cmd_clean() {
    [[ -f "$DISK" ]] && { rm -f "$DISK" "$NVRAM"; ok "disk and UEFI vars removed"; } \
                     || warn "nothing to remove"
    note "ISO kept in $ISO_DIR"
}

case "${1:-}" in
    create)  cmd_create ;;
    install) shift; cmd_install "$@" ;;
    run)     shift; cmd_run "$@" ;;
    ssh)     shift; cmd_ssh "$@" ;;
    shot)    shift; cmd_shot "$@" ;;
    stop)    cmd_stop ;;
    save)    cmd_save ;;
    reset)   cmd_reset ;;
    clean)   cmd_clean ;;
    *)       sed -n '3,25p' "${BASH_SOURCE[0]}" | sed 's/^# \?//' ;;
esac
