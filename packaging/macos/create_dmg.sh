#!/usr/bin/env bash
set -euo pipefail
# Usage: packaging/macos/create_dmg.sh <version>
# Builds a universal (arm64 + x86_64) .app + DMG + tarball from release builds in
# target/{aarch64,x86_64}-apple-darwin/release. Requires macOS (lipo, hdiutil).
# Config via env (defaults below are set per project):
APP_NAME="FlashKraft"          # bundle display name
GUI_BIN="flashkraft"            # GUI binary name (inside .app)
TUI_BIN="flashkraft-tui"            # CLI/TUI binary name (shipped next to the .app)
SLUG="flashkraft"                  # artifact file prefix
BUNDLE_ID="com.sorinirimies.flashkraft"
ICNS="packaging/macos/flashkraft.icns"                  # optional; skipped if missing

VERSION="${1#v}"
DIST="dist"
STAGE="${DIST}/dmg"
APP="${STAGE}/${APP_NAME}.app"
rm -rf "$DIST"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

for BIN in "$GUI_BIN" "$TUI_BIN"; do
  lipo -create \
    "target/aarch64-apple-darwin/release/${BIN}" \
    "target/x86_64-apple-darwin/release/${BIN}" \
    -output "${DIST}/${BIN}"
done

cp "${DIST}/${GUI_BIN}" "$APP/Contents/MacOS/${GUI_BIN}"
chmod +x "$APP/Contents/MacOS/${GUI_BIN}"
ICON_KEY=""
if [ -f "$ICNS" ]; then
  cp "$ICNS" "$APP/Contents/Resources/${GUI_BIN}.icns"
  ICON_KEY="<key>CFBundleIconFile</key><string>${GUI_BIN}</string>"
fi

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>${GUI_BIN}</string>
  <key>CFBundleIdentifier</key><string>${BUNDLE_ID}</string>
  <key>CFBundleName</key><string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key><string>${APP_NAME}</string>
  ${ICON_KEY}
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

# Ad-hoc sign so Apple Silicon will run the unsigned bundle.
codesign --force --deep --sign - "$APP"

cp "${DIST}/${TUI_BIN}" "$STAGE/${TUI_BIN}"
ln -s /Applications "$STAGE/Applications"

hdiutil create -volname "${APP_NAME} ${VERSION}" -srcfolder "$STAGE" \
  -ov -format UDZO "${DIST}/${SLUG}-v${VERSION}-macos-universal.dmg"
tar -C "$DIST" -czf "${DIST}/${SLUG}-v${VERSION}-macos-universal.tar.gz" "$GUI_BIN" "$TUI_BIN"
ls -lh "$DIST"
