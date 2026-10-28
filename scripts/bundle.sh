#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Cargo.toml is the single source of truth for the app version.
app_version="$(cargo metadata --locked --no-deps --format-version 1 |
  python3 -c 'import json, sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "budgettinker"))')"
# Apple requires a numeric CFBundleShortVersionString, without prerelease labels.
short_version="${app_version%%-*}"
bash scripts/icon.sh "$PWD/target/BudgetTinker.iconset"
cargo build --release --locked
app_bundle="$PWD/dist/BudgetTinker.app"
mkdir -p "$app_bundle/Contents/MacOS" "$app_bundle/Contents/Resources" "$PWD/target/BudgetTinker.iconset"
install -m 755 target/release/budgettinker "$app_bundle/Contents/MacOS/.budgettinker.next"
mv -f "$app_bundle/Contents/MacOS/.budgettinker.next" "$app_bundle/Contents/MacOS/budgettinker"
iconutil -c icns target/BudgetTinker.iconset -o "$app_bundle/Contents/Resources/BudgetTinker.icns"
cat > "$app_bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>BudgetTinker</string>
<key>CFBundleDisplayName</key><string>BudgetTinker</string>
<key>CFBundleIdentifier</key><string>io.github.perevozchiko.budgettinker</string>
<key>CFBundleExecutable</key><string>budgettinker</string>
<key>CFBundleIconFile</key><string>BudgetTinker.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$short_version</string>
<key>CFBundleGetInfoString</key><string>BudgetTinker $app_version</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSRequiresAquaSystemAppearance</key><true/>
<key>NSPrincipalClass</key><string>NSApplication</string>
<key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict></plist>
PLIST
python3 scripts/stage-notices.py "$app_bundle/Contents/Resources/Notices"
codesign --force --deep --sign - "$app_bundle"
printf 'Готово: %s\n' "$app_bundle"
