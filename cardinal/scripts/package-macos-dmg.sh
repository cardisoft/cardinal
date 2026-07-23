#!/usr/bin/env bash
set -euo pipefail

APP_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_TAURI="$APP_ROOT/src-tauri"
TARGET_DIR="$SRC_TAURI/target/release"
APP_BUNDLE="$TARGET_DIR/bundle/macos/Cardinal.app"
SERVICE_MENUS="$APP_BUNDLE/Contents/Resources/share/batch/macOS service menus"
VERSION="$(node -e 'const fs = require("fs"); console.log(JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8")).version)')"
ARCH="$(uname -m)"

if [[ "$ARCH" == "arm64" ]]; then
  ARCH="aarch64"
fi

DMG_PATH="$TARGET_DIR/Cardinal_${VERSION}_${ARCH}.dmg"
STAGE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cardinal-dmg-stage.XXXXXX")"

cleanup() {
  rm -rf "$STAGE_DIR"
}
trap cleanup EXIT

cd "$APP_ROOT"
npm run tauri build -- --bundles app

if [[ ! -d "$APP_BUNDLE" ]]; then
  echo "Missing app bundle: $APP_BUNDLE" >&2
  exit 1
fi

if [[ ! -d "$SERVICE_MENUS" ]]; then
  echo "Missing macOS service menus: $SERVICE_MENUS" >&2
  exit 1
fi

ditto "$APP_BUNDLE" "$STAGE_DIR/Cardinal.app"
ln -s /Applications "$STAGE_DIR/Applications"
ditto "$SERVICE_MENUS" "$STAGE_DIR/macOS service menus"

cat > "$STAGE_DIR/README - Finder Quick Action.txt" <<'EOF'
Install Cardinal first by dragging Cardinal.app to Applications.

To install the Finder Quick Action, open the "macOS service menus" folder and
double-click "Cardinal, search in folder.workflow".

After installation, Finder shows "Search in Cardinal" under Quick Actions or
Services for selected files and folders.
EOF

hdiutil create \
  -volname "Cardinal ${VERSION}" \
  -srcfolder "$STAGE_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH"

echo "Created $DMG_PATH"
