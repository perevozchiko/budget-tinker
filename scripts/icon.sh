#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
iconset="${1:?Pass the output .iconset directory}"
mkdir -p "$iconset"
# The same padded PNG is used by eframe (Dock override) and the .icns bundle.
swift scripts/icon-padding.swift assets/icon-artwork.png assets/app-icon.png
master="assets/app-icon.png"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$master" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" "$master" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
