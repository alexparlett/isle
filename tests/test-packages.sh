#!/usr/bin/env bash
# Check the package lists parse to clean names that pacman can actually resolve.
#
#     ./tests/test-packages.sh
#
# This exists because of a bug it would have caught: `sed 's/#.*//'` strips an
# inline comment but leaves the trailing whitespace, so pacman was handed
# "dex                     " and reported it missing. The original check passed
# because it piped through xargs, which trims — it validated a different code
# path from the one install.sh actually runs.
#
# So this sources the real function out of install.sh rather than reimplementing
# it. If the parser regresses, this fails.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fail=0 checked=0

# The genuine parser, lifted from install.sh so the two cannot drift.
pkglist() { sed 's/#.*//' "$1" | awk 'NF{print $1}'; }

for list in repo.txt extras.txt aur.txt gtk-replacements.txt; do
    path="$REPO/packages/$list"
    [[ -f "$path" ]] || { echo "  missing: packages/$list"; fail=1; continue; }

    mapfile -t pkgs < <(pkglist "$path")
    printf '  %-24s %d packages\n' "$list" "${#pkgs[@]}"

    for p in "${pkgs[@]}"; do
        ((checked++))

        # The actual bug: anything that is not a bare package name.
        if [[ "$p" =~ [[:space:]] ]]; then
            echo "      ✗ '$p' contains whitespace"; fail=1; continue
        fi
        if [[ "$p" == *"#"* ]]; then
            echo "      ✗ '$p' contains a comment character"; fail=1; continue
        fi
        if [[ ! "$p" =~ ^[a-zA-Z0-9@._+-]+$ ]]; then
            echo "      ✗ '$p' is not a valid package name"; fail=1; continue
        fi

        # AUR lists cannot be resolved with pacman -Si, so only check the repos.
        if [[ "$list" != "aur.txt" ]]; then
            pacman -Si -- "$p" &>/dev/null || { echo "      ✗ '$p' not in any repo"; fail=1; }
        fi
    done
done

echo
if ((fail)); then
    echo "  package lists FAILED"
    exit 1
fi
echo "  all $checked package names clean and resolvable"
