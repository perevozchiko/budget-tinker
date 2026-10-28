#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
bash scripts/icon.sh "$PWD/target/BudgetTinker.iconset"
cargo build --release --locked
app_bundle="$PWD/dist/BudgetTinker.app"
mkdir -p "$app_bundle/Contents/MacOS" "$app_bundle/Contents/Resources" "$PWD/target/BudgetTinker.iconset"
install -m 755 target/release/budgettinker "$app_bundle/Contents/MacOS/.budgettinker.next"
mv -f "$app_bundle/Contents/MacOS/.budgettinker.next" "$app_bundle/Contents/MacOS/budgettinker"
iconutil -c icns target/BudgetTinker.iconset -o "$app_bundle/Contents/Resources/BudgetTinker.icns"
cat > "$app_bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>BudgetTinker</string>
<key>CFBundleDisplayName</key><string>BudgetTinker</string>
<key>CFBundleIdentifier</key><string>io.github.perevozchiko.budgettinker</string>
<key>CFBundleExecutable</key><string>budgettinker</string>
<key>CFBundleIconFile</key><string>BudgetTinker.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
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
