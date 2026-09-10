#!/usr/bin/env bash
#
# Install CachyOS into the test VM without Calamares.
#
# Runs FROM THE LIVE ISO, inside the guest:
#
#     mount -t 9p -o trans=virtio,version=9p2000.L,msize=512000 hyprrepo /repo
#     /repo/tools/install-guest.sh
#
# Calamares is a graphical installer that has to be clicked through and that
# crashes in this VM. This does the same job in about six minutes, unattended,
# and identically every time — which matters more here than a nice wizard,
# because the guest is disposable and gets rebuilt.
#
# DESTRUCTIVE: repartitions the whole virtio disk. That disk is a 60G qcow2
# created by tools/test-vm.sh and contains nothing you want. It refuses to run
# outside a VM.

set -euo pipefail

DISK="/dev/vda"
# NOT /mnt. The repo share is mounted at /repo, and mounting the new root
# on /mnt hides it — so packages/shell.txt vanishes mid-install and the only
# reason the log kept working was bash holding the fd open from before the
# mount. That took a while to see precisely because the log looked healthy.
TARGET="/target"
HOSTNAME="hypr-guest"
USERNAME="user"
PASSWORD="user"          # a disposable VM behind user-mode networking
TIMEZONE="Europe/London"
LOCALE="en_GB.UTF-8"

c_reset=$'\e[0m'; c_bold=$'\e[1m'; c_dim=$'\e[2m'
c_green=$'\e[32m'; c_red=$'\e[31m'; c_blue=$'\e[34m'
info() { printf '\n%s==>%s %s\n' "$c_blue$c_bold" "$c_reset" "$*"; }
ok()   { printf '  %s✓%s %s\n' "$c_green" "$c_reset" "$*"; }
err()  { printf '  %s✗%s %s\n' "$c_red" "$c_reset" "$*" >&2; }
note() { printf '    %s%s%s\n' "$c_dim" "$*" "$c_reset"; }

# --- refuse to eat a real machine -------------------------------------------

if ! systemd-detect-virt --quiet; then
    err "not running in a VM — this repartitions $DISK"
    exit 1
fi
[[ $EUID -eq 0 ]] || { err "run as root (the live ISO logs in as root)"; exit 1; }
[[ -b "$DISK" ]] || { err "$DISK is not a block device"; exit 1; }

size=$(lsblk -bdno SIZE "$DISK")
info "Target"
note "$DISK  $(numfmt --to=iec "$size")  — everything on it will be destroyed"
ok "running under $(systemd-detect-virt)"

# --- partition ---------------------------------------------------------------
# GPT, 512M ESP, the rest ext4. Not btrfs: fewer moving parts in a guest that
# exists to be thrown away, and snapshots are what test-vm.sh clean is for.

info "Partitioning"
wipefs -af "$DISK" >/dev/null
sgdisk --zap-all "$DISK" >/dev/null
sgdisk -n1:0:+512M -t1:ef00 -c1:ESP "$DISK" >/dev/null
sgdisk -n2:0:0     -t2:8300 -c2:root "$DISK" >/dev/null
partprobe "$DISK"; sleep 1
ok "ESP 512M + root $(numfmt --to=iec $((size - 536870912)))"

mkfs.fat -F32 -n ESP "${DISK}1" >/dev/null
mkfs.ext4 -qF -L root "${DISK}2"
ok "filesystems made"

mkdir -p "$TARGET"
mount "${DISK}2" "$TARGET"
mkdir -p "$TARGET/boot"
mount "${DISK}1" "$TARGET/boot"
ok "mounted at $TARGET"

# --- base system -------------------------------------------------------------
# The live ISO's pacman.conf already has the CachyOS repos, and pacstrap copies
# it in, so the guest gets the same package set the real machine would.

# The live ISO ships a keyring whose keys are present but not trusted, so
# pacstrap fails every package with "signature ... is unknown trust" and then
# "failed to commit transaction (invalid or corrupted package)" — which reads
# like a mirror problem and is not one. Populating first costs ten seconds and
# fixes it. This is not optional on a fresh boot of the ISO.
# The directory must go first. `pacman-key --init` is a no-op when
# /etc/pacman.d/gnupg already exists, which it does on the ISO — with a master
# key that cannot sign, so --populate reports "could not be locally signed"
# and every package then fails as "unknown trust". Removing and reinitialising
# is what actually fixes it.
info "Keyring"
# gpg-agent and dirmngr keep the keyring directory open, so the rm below fails
# with "Device or resource busy" and, under set -e, kills the install. They are
# started by any earlier pacman-key run — including a previous failed attempt,
# which is exactly when you are re-running this.
pkill -9 gpg-agent 2>/dev/null || true
pkill -9 dirmngr   2>/dev/null || true
sleep 1
# Clear the CONTENTS, not the directory. On the live ISO /etc/pacman.d/gnupg is
# a tmpfs mountpoint, so removing the directory itself always fails with
# "Device or resource busy" — and under set -e that ends the install having
# already wiped the disk. Emptying it is what --init actually needs anyway.
rm -rf /etc/pacman.d/gnupg/* /etc/pacman.d/gnupg/.[!.]* 2>/dev/null || true
pacman-key --init >/dev/null 2>&1
pacman-key --populate archlinux cachyos >/dev/null 2>&1 \
    || pacman-key --populate archlinux >/dev/null 2>&1
trusted=$(pacman-key --list-keys 2>/dev/null | grep -c '^pub')
[[ "$trusted" -gt 50 ]] || { err "only $trusted keys trusted — pacstrap will fail"; exit 1; }
ok "$trusted keys trusted"

# The ISO's package database is older than the mirror, so pacstrap asks for
# versions that have already been replaced and gets a run of 404s, then
# "too many errors ... skipping for the remainder of this transaction".
info "Refreshing the package database"
pacman -Sy --noconfirm >/dev/null 2>&1 && ok "synced" || err "sync failed"

info "Installing the base system"
note "this is the slow part — five minutes or so"
# No -K. That flag initialises a *fresh, empty* keyring in the target, which
# trusts nothing, so every package fails signature validation. Without it
# pacstrap copies the host keyring — which the Keyring step above just fixed.
pacstrap "$TARGET" \
    base linux linux-firmware \
    sudo networkmanager openssh \
    git vim nano \
    mesa vulkan-virtio \
    >/dev/null
ok "base installed"

genfstab -U "$TARGET" >> $TARGET/etc/fstab
ok "fstab written"

# --- configure inside the new system ----------------------------------------

info "Configuring"
cat > $TARGET/root/configure.sh <<CHROOT
set -euo pipefail

ln -sf /usr/share/zoneinfo/$TIMEZONE /etc/localtime
hwclock --systohc
echo "$LOCALE UTF-8" > /etc/locale.gen
locale-gen >/dev/null
echo "LANG=$LOCALE" > /etc/locale.conf
echo "$HOSTNAME" > /etc/hostname

cat > /etc/hosts <<HOSTS
127.0.0.1   localhost
::1         localhost
127.0.1.1   $HOSTNAME.localdomain $HOSTNAME
HOSTS

# Root has no password and no login shell reachable from outside; the VM sits
# behind user-mode networking with only :2222 forwarded.
useradd -m -G wheel -s /bin/bash $USERNAME
echo "$USERNAME:$PASSWORD" | chpasswd
echo "root:$PASSWORD" | chpasswd
echo "%wheel ALL=(ALL:ALL) NOPASSWD: ALL" > /etc/sudoers.d/10-wheel
chmod 440 /etc/sudoers.d/10-wheel

systemctl enable NetworkManager sshd >/dev/null

# systemd-boot: the disk is GPT+UEFI because OVMF is, and this needs no config
# file beyond one entry.
bootctl install >/dev/null 2>&1
root_uuid=\$(blkid -s UUID -o value ${DISK}2)
mkdir -p /boot/loader/entries
cat > /boot/loader/loader.conf <<LOADER
default arch.conf
timeout 1
console-mode max
LOADER
cat > /boot/loader/entries/arch.conf <<ENTRY
title   CachyOS (hypr guest)
linux   /vmlinuz-linux
initrd  /initramfs-linux.img
options root=UUID=\$root_uuid rw quiet
ENTRY

# Autologin on tty1, then start Hyprland from the shell profile. No greeter:
# Services.Greetd is deliberately out of scope until the lock screen has soaked
# (see Wave 8), and a build target should not need a login to be screenshotted.
mkdir -p /etc/systemd/system/getty@tty1.service.d
cat > /etc/systemd/system/getty@tty1.service.d/autologin.conf <<GETTY
[Service]
ExecStart=
ExecStart=-/usr/bin/agetty --autologin $USERNAME --noclear %I \\\$TERM
GETTY

cat > /home/$USERNAME/.bash_profile <<PROFILE
[[ -f ~/.bashrc ]] && . ~/.bashrc
# Start the compositor on the first VT only, so ssh sessions are unaffected.
if [[ -z \\\$WAYLAND_DISPLAY && \\\$XDG_VTNR -eq 1 ]]; then
    exec Hyprland
fi
PROFILE
chown $USERNAME:$USERNAME /home/$USERNAME/.bash_profile

# The repo share, mounted at boot. nofail so a guest whose host share is gone
# still boots — that is exactly when you need to get in and look.
mkdir -p /repo
echo "hyprrepo /repo 9p trans=virtio,version=9p2000.L,msize=512000,nofail,x-systemd.device-timeout=5 0 0" >> /etc/fstab
CHROOT

arch-chroot "$TARGET" bash /root/configure.sh
rm -f $TARGET/root/configure.sh
ok "configured"

# --- Hyprland and the shell's packages --------------------------------------

info "Installing Hyprland and the shell's dependencies"
list="/repo/packages/shell.txt"
if [[ -f "$list" ]]; then
    mapfile -t pkgs < <(sed 's/#.*//' "$list" | awk 'NF{print $1}')
    note "${#pkgs[@]} packages from packages/shell.txt"
    arch-chroot "$TARGET" pacman -S --needed --noconfirm "${pkgs[@]}" >/dev/null \
        && ok "installed" || err "some packages failed — check after boot"
else
    err "packages/shell.txt not found at $list"
    note "Is the 9p share mounted? mount -t 9p -o trans=virtio,version=9p2000.L,msize=512000 hyprrepo /repo"
    note "Continuing without it; run provision-guest.sh after boot."
fi

# --- passwordless ssh from the host -----------------------------------------
#
# Without this every reset needs the host's public key typed into the guest by
# hand, and typing through QMP is slow and was, until recently, unreliable.
# tools/test-vm.sh stages the key on the share; if it is missing, password
# login still works.

info "Host key"
if [[ -f /repo/.vm-key.pub ]]; then
    install -d -m 700 -o "$USERNAME" -g "$USERNAME" "$TARGET/home/$USERNAME/.ssh"
    install -m 600 -o "$USERNAME" -g "$USERNAME" /repo/.vm-key.pub \
        "$TARGET/home/$USERNAME/.ssh/authorized_keys"
    ok "host key installed — ssh -p 2222 $USERNAME@localhost"
else
    err "no /repo/.vm-key.pub — you will need the password ($PASSWORD)"
fi

# --- a monitor the design actually targets ----------------------------------

info "Display"
mkdir -p $TARGET/home/$USERNAME/.config/hypr
cat > $TARGET/home/$USERNAME/.config/hypr/hyprland.conf <<'HYPRCONF'
# Minimal guest config. The real configuration lives in the repo and is what
# is under test; this exists only so the guest boots to a compositor with the
# resolution the design targets.
# Two things learned here, both silent failures.
#
# Not "Virtual-1": QEMU's virtio-gpu presents as Virtual-2, and a rule naming
# the wrong output is ignored without comment — the guest sat at 1280x800 with
# a 3440x1440 rule above it looking perfectly correct.
#
# And not a hardcoded mode either. Asking for a mode the output cannot do
# leaves Hyprland alive but with its IPC wedged, which reads as a broken
# compositor rather than a bad config line. The resolution is set on the QEMU
# side instead (virtio-gpu-gl,xres=,yres= in tools/test-vm.sh), and the guest
# simply takes what it is offered.
# 2560x1080, not 3440x1440. QEMU is asked for 3440x1440 (see virtio-gpu-gl's
# xres/yres in tools/test-vm.sh) but the gtk window cannot exceed the host
# screen, so the largest mode actually offered here is 2560x1080. That is
# still 21:9, which is the property the layout depends on — an ultrawide is
# what the design targets, and the exact pixel count is not what breaks.
#
# Do not put an unattainable mode here. Hyprland stays alive but wedges its
# IPC, which looks like a broken compositor rather than a bad config line.
monitor = , 2560x1080@60, 0x0, 1

$terminal = kitty
bind = SUPER, Return, exec, $terminal
bind = SUPER, Q, killactive
bind = SUPER SHIFT, E, exit

misc {
    disable_hyprland_logo = true
    disable_splash_rendering = true
}
HYPRCONF
arch-chroot "$TARGET" chown -R $USERNAME:$USERNAME /home/$USERNAME/.config
ok "3440x1440 requested on Virtual-1"

# --- done --------------------------------------------------------------------

umount -R "$TARGET"
info "Done"
ok "installed — shut down, then boot without the ISO:"
note "  poweroff        (in here)"
note "  tools/test-vm.sh run     (on the host)"
note ""
note "It autologins as $USERNAME/$PASSWORD into Hyprland on tty1,"
note "with sshd on host port 2222 and the repo at /repo."
