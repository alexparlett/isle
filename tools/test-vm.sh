#!/usr/bin/env bash
#
# Spin up a CachyOS VM to test this repo against a machine that is not yours.
#
#     tools/test-vm.sh create     download the ISO and make a disk
#     tools/test-vm.sh install    boot the installer
#     tools/test-vm.sh run        boot the installed system
#     tools/test-vm.sh ssh        shell into it (after setup inside the guest)
#     tools/test-vm.sh clean      delete the disk, keep the ISO
#
# QEMU/KVM rather than VirtualBox: AMD-V is already live in this kernel,
# virtio-gpu-gl gives Hyprland real GL, and VirtualBox would want DKMS modules
# built against the CachyOS kernel for worse Wayland support.
#
# The point of this is not "does Hyprland run" — it is whether install.sh
# reproduces a desktop on hardware that shares nothing with the machine it was
# written on. Different GPU, different monitor name, no NVIDIA, no KDE to
# migrate from, none of the packages already present.

set -uo pipefail

VM_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/hypr-testvm"
DISK="$VM_DIR/cachyos.qcow2"
ISO_DIR="$VM_DIR/iso"
NVRAM="$VM_DIR/OVMF_VARS.fd"
SSH_PORT=2222

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

ovmf_vars() {
    for f in /usr/share/edk2/x64/OVMF_VARS.4m.fd /usr/share/edk2-ovmf/x64/OVMF_VARS.fd \
             /usr/share/OVMF/OVMF_VARS.fd; do
        [[ -f "$f" ]] && { echo "$f"; return; }
    done
}

# --- qemu invocation ---------------------------------------------------------
#
# virtio-gpu-gl + display gtk,gl=on is what makes this useful: Hyprland needs
# GLES, and without acceleration it either refuses or crawls. venus/virgl gives
# the guest real GL through the host GPU.

launch() {
    local -a boot=("$@")

    qemu-system-x86_64 \
        -name "cachyos-hypr-test" \
        -machine q35,accel=kvm \
        -cpu host \
        -smp "$CORES" \
        -m "$MEMORY" \
        -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE" \
        -drive "if=pflash,format=raw,file=$NVRAM" \
        -drive "file=$DISK,if=virtio,cache=writeback,discard=unmap" \
        -device virtio-gpu-gl \
        -display gtk,gl=on,show-cursor=on \
        -device virtio-keyboard-pci \
        -device virtio-tablet-pci \
        -audiodev pipewire,id=snd0 \
        -device intel-hda -device hda-duplex,audiodev=snd0 \
        -netdev "user,id=net0,hostfwd=tcp::${SSH_PORT}-:22" \
        -device virtio-net-pci,netdev=net0 \
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
    preflight
    [[ -f "$DISK" ]] || { err "no disk — run: tools/test-vm.sh create"; exit 1; }
    info "Booting the installed system"
    note "ssh -p $SSH_PORT <user>@localhost   once sshd is running in the guest"
    launch
}

cmd_ssh() {
    exec ssh -p "$SSH_PORT" -o StrictHostKeyChecking=no \
             -o UserKnownHostsFile=/dev/null "${1:-$USER}@localhost"
}

cmd_clean() {
    [[ -f "$DISK" ]] && { rm -f "$DISK" "$NVRAM"; ok "disk and UEFI vars removed"; } \
                     || warn "nothing to remove"
    note "ISO kept in $ISO_DIR"
}

case "${1:-}" in
    create)  cmd_create ;;
    install) cmd_install ;;
    run)     cmd_run ;;
    ssh)     shift; cmd_ssh "$@" ;;
    clean)   cmd_clean ;;
    *)       sed -n '3,16p' "${BASH_SOURCE[0]}" | sed 's/^# \?//' ;;
esac
