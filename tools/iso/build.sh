#!/usr/bin/env bash
#
# Build a CachyOS ISO whose installer offers Isle as a desktop.
#
#     tools/iso/build.sh                  build; the ISO lands in ~/.cache/isle-iso/out
#     tools/iso/build.sh --prepare        lay out the profile and stop (inspect before a long build)
#     tools/iso/build.sh --work DIR       build directory (default ~/.cache/isle-iso; needs ~15G)
#     tools/iso/build.sh --ref COMMIT     the upstream CachyOS-Live-ISO commit to build from
#
# The live session stays CachyOS's own (Plasma, Calamares). What changes: the repo
# rides along in the ISO as /usr/share/isle/isle.tar.zst, a shallow clone that the
# installed machine can pull from the checkout's origin; the installer's desktop list gains "Isle", whose
# package group is packages/shell.txt; and after the install the target gets
# tools/iso/target-setup.sh, which does what tools/bootstrap.sh does apart from the
# AUR packages and hyprbars, which need a login and run from the first-login prompt.
#
# Needs sudo (mkarchiso runs as root) and installs archiso if it is missing.

set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="${ISLE_ISO_WORK:-$HOME/.cache/isle-iso}"
REF="4937780"
UPSTREAM="https://github.com/CachyOS/CachyOS-Live-ISO"
prepare=0
while (($#)); do case "$1" in --prepare) prepare=1 ;; --work) WORK="$2"; shift ;; --ref) REF="$2"; shift ;; -h|--help) sed -n '3,17p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;; esac; shift; done

ok() { printf '  \e[32m✓\e[0m %s\n' "$*"; }
step() { printf '\n\e[1m%s\e[0m\n' "$*"; }
die() { printf '  \e[31m!\e[0m %s\n' "$*" >&2; exit 1; }

command -v pacman >/dev/null || die "this builds on CachyOS or Arch"
step "Tools"
tools=(archiso mkinitcpio-archiso squashfs-tools grub git zstd python-yaml)
if pacman -Q "${tools[@]}" >/dev/null 2>&1; then ok "archiso, squashfs-tools, grub, zstd"
elif ((prepare)); then echo "  · not all of ${tools[*]} are installed; the build step will install them"
else sudo pacman -S --needed --noconfirm "${tools[@]}" >/dev/null && ok "archiso, squashfs-tools, grub, zstd"; fi
# pacstrap signs with this machine's keyring; a stale one fails the build on the first newly-signed package.
if ((!prepare)); then sudo pacman -Sy --needed --noconfirm archlinux-keyring cachyos-keyring >/dev/null && sudo pacman-key --populate archlinux cachyos >/dev/null 2>&1 && ok "keyrings current"; fi

step "Upstream profile at $REF"
LIVE="$WORK/live-iso"
mkdir -p "$WORK"
if [[ ! -d "$LIVE/.git" ]]; then git clone -q "$UPSTREAM" "$LIVE"; fi
git -C "$LIVE" fetch -q origin
git -C "$LIVE" checkout -q --force "$REF"
# A previous build leaves root-owned work and out directories behind.
sudo rm -rf "$LIVE/build" "$LIVE/out"
git -C "$LIVE" clean -qfdx
ok "$LIVE"

step "Isle overlay"
A="$LIVE/archiso/airootfs"
cp -r "$REPO/tools/iso/airootfs/." "$A/"
# The repo itself as a shallow clone of HEAD with the checkout's origin, in https form, so the installed machine can pull updates.
tmp="$(mktemp -d)"
git clone -q --depth 1 --branch "$(git -C "$REPO" rev-parse --abbrev-ref HEAD)" "file://$REPO" "$tmp/isle"
origin="$(git -C "$REPO" remote get-url origin)"
case "$origin" in http*) ;; *) origin="$(printf '%s' "$origin" | sed -E 's#^(ssh://)?([[:alnum:]._-]+@)?([[:alnum:].-]+)[:/]+#https://\3/#; s#(\.git)?/?$#.git#')" ;; esac
git -C "$tmp/isle" remote set-url origin "$origin"
tar -C "$tmp" -cf - isle | zstd -q -T0 -o "$A/usr/share/isle/isle.tar.zst" -f
rm -rf "$tmp"
ok "isle.tar.zst from $(git -C "$REPO" rev-parse --short HEAD)"
# The desktop's package group: packages/shell.txt plus what the target setup and the first login need.
python3 - "$REPO/packages/shell.txt" "$A/usr/share/isle/isle-group.yaml" <<'PY'
import sys, yaml, re
pkgs = [l.split("#")[0].strip() for l in open(sys.argv[1])]
pkgs = [p for p in pkgs if p]
pkgs += ["greetd", "git", "base-devel", "cmake", "cpio", "zstd"]
group = {
    "name": "Isle",
    "description": "Hyprland with the Isle shell: one island, a dashboard, a launcher, Settings, a Monitor and a Keychain, for gaming and coding.",
    "platform": "desktop", "hidden": False, "selected": False, "expanded": False, "critical": True,
    "packages": pkgs,
}
yaml.safe_dump([group], open(sys.argv[2], "w"), sort_keys=False)
PY
ok "Isle group: $(grep -c '^- ' "$A/usr/share/isle/isle-group.yaml") packages"
# The installer launcher runs our setup once Calamares's own settings are in place.
sed -i 's|^\(\s*\)exec pkexec-wrapper calamares|\1sudo /usr/local/bin/isle-installer-setup >> $log 2>\&1\n\1exec pkexec-wrapper calamares|' "$A/usr/local/bin/calamares-online.sh"
grep -q isle-installer-setup "$A/usr/local/bin/calamares-online.sh" || die "calamares-online.sh has changed upstream; the hook did not apply"
# Executable bits are set by profiledef, not the tree.
sed -i 's|^)$|  ["/usr/local/bin/isle-installer-setup"]="0:0:755"\n)|' "$LIVE/archiso/profiledef.sh"
ok "installer hook"

if ((prepare)); then echo; echo "  prepared in $LIVE; build with: cd $LIVE && sudo ./buildiso.sh -p desktop -w"; exit 0; fi

step "mkarchiso (this takes a while and downloads the live system's packages)"
cd "$LIVE"
sudo ./buildiso.sh -p desktop -w
# Upstream renames the ISO to its own scheme at the end; ours says what it is.
mkdir -p "$WORK/out"
iso="$(ls -t "$LIVE"/out/desktop/*.iso | head -1)"
cp -f "$iso" "$WORK/out/cachyos-isle-$(date +%y%m%d).iso"
ok "$WORK/out/cachyos-isle-$(date +%y%m%d).iso"
