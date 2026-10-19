#!/usr/bin/env bash
#
# Package this checkout's HEAD with pkg/arch/PKGBUILD and install it.
#
#     tools/package.sh                build and install
#     tools/package.sh --no-install   build only; the package is left in ~/.cache/isle/pkg
#     tools/package.sh -- --nodeps     arguments after -- go to makepkg
#
# Only committed work is packaged, so the package's version names a commit. The build's
# target directory is kept in ~/.cache/isle/pkg, so a rebuild is incremental. A running Isle
# keeps the build it started with; the next sign-in runs this one.

set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
WORK="${XDG_CACHE_HOME:-$HOME/.cache}/isle/pkg"

install=1
makepkg_args=()
while (($#)); do
    case "$1" in
        --no-install) install=0 ;;
        --) shift; makepkg_args=("$@"); break ;;
        -h|--help) sed -n '3,11p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "package: unknown argument $1" >&2; exit 2 ;;
    esac
    shift
done

head="$(git -C "$REPO" rev-parse HEAD)"
if [[ -n "$(git -C "$REPO" status --porcelain --untracked-files=no)" ]]; then
    echo "package: uncommitted changes are left out; packaging $(git -C "$REPO" rev-parse --short HEAD)"
fi

mkdir -p "$WORK"
cp "$REPO/pkg/arch/PKGBUILD" "$WORK/PKGBUILD"
cd "$WORK"
export ISLE_SOURCE="git+file://$REPO#commit=$head"
export ISLE_TARGET_DIR="$WORK/target"
makepkg --force --cleanbuild --noconfirm "${makepkg_args[@]}"
package="$(makepkg --packagelist | grep -v -- '-debug-' | head -n 1)"
echo "package: built $package"

if ((install)); then
    # v1's installer left an isle.portal no package owns; this package's replaces it.
    sudo pacman -U --noconfirm \
        --overwrite /usr/share/xdg-desktop-portal/portals/isle.portal "$package"
fi
