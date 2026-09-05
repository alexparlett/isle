#!/usr/bin/env bash
# Generate the default wallpaper: a 3440x1440 Catppuccin Mocha gradient with a
# subtle vignette. Keeps a binary out of git while still giving hyprpaper
# something to show on first boot.
set -euo pipefail

out="${1:-$HOME/.config/hypr/wallpapers/mocha-gradient.png}"
w=${2:-3440}
h=${3:-1440}

mkdir -p "$(dirname "$out")"

magick -size "${w}x${h}" \
    gradient:'#181825-#1e1e2e' \
    \( -size "${w}x${h}" radial-gradient:'#302d41-transparent' -alpha set -channel A -evaluate multiply 0.55 +channel \) \
    -compose over -composite \
    \( -size "${w}x${h}" xc:'#11111b' -alpha set -virtual-pixel transparent \
       -channel A -blur 0x400 -evaluate multiply 0.0 +channel \) \
    -compose over -composite \
    -attenuate 0.35 +noise Gaussian \
    "$out"

echo "wallpaper written to $out"
