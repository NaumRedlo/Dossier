#!/bin/bash
set -euo pipefail
project_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$project_root"
if [[ "$(uname -s)" != Darwin ]]; then
    echo 'This prototype requires macOS.' >&2
    exit 1
fi
cargo build --locked --manifest-path tools/overlay-host/Cargo.toml --bin hud-preview
bundle="$project_root/tools/macos-overlay/target/Dossier HUD Demo.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
echo "Собираю macOS-приложение HUD Demo…"
swiftc -module-cache-path "$project_root/tools/macos-overlay/target/module-cache" tools/macos-overlay/main.swift tools/macos-overlay/LazerSocket.swift -o "$bundle/Contents/MacOS/macos-overlay.new"
mv -f "$bundle/Contents/MacOS/macos-overlay.new" "$bundle/Contents/MacOS/macos-overlay"
# Replace the inode: overwriting a signed Mach-O can leave a stale macOS code-sign cache.
cp tools/overlay-host/target/debug/hud-preview "$bundle/Contents/Resources/hud-preview.new"
mv -f "$bundle/Contents/Resources/hud-preview.new" "$bundle/Contents/Resources/hud-preview"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>app.dossier.hud-demo</string>
<key>CFBundleName</key><string>Dossier HUD Demo</string>
<key>CFBundleExecutable</key><string>macos-overlay</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSUIElement</key><true/>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
if [[ "${1:-}" == --build-only ]]; then
    echo "$bundle"
    exit 0
fi
log="$project_root/tools/macos-overlay/target/hud.log"
echo "Dossier HUD: после запуска переключитесь в lazer. Состояние видно в меню оверлея."
echo "Журнал: $log"
exec open -W -g --stdout "$log" --stderr "$log" "$bundle" --args "$@"
