#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
mkdir -p target/icons
# Keep generated native formats checked in; ordinary builds need no converter.
rsvg-convert -w 256 -h 256 assets/icons/typerr-logo.svg -o target/icons/typerr-logo.png
magick target/icons/typerr-logo.png -resize 64x64 -depth 8 RGBA:assets/icons/typerr-logo.rgba
magick target/icons/typerr-logo.png -define icon:auto-resize=256,128,64,48,32,16 assets/icons/typerr-logo.ico
