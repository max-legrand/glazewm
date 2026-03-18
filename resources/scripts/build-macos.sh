#!/bin/bash
# Builds GlazeWM and creates a macOS app bundle.
#
# Usage:
#   ./resources/scripts/build-macos.sh [--release]
#
# Options:
#   --release    Build in release mode (default: debug)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

BUILD_MODE="debug"
CARGO_FLAGS=""

for arg in "$@"; do
  case $arg in
    --release)
      BUILD_MODE="release"
      CARGO_FLAGS="--release"
      ;;
  esac
done

echo "Building GlazeWM ($BUILD_MODE)..."
cargo build $CARGO_FLAGS

BUILD_DIR="$PROJECT_ROOT/target/$BUILD_MODE"
APP_DIR="$BUILD_DIR/GlazeWM.app"
CONTENTS_DIR="$APP_DIR/Contents"

echo "Creating app bundle at $APP_DIR..."

rm -rf "$APP_DIR"
mkdir -p "$CONTENTS_DIR/MacOS" "$CONTENTS_DIR/Resources"

cp "$BUILD_DIR/glazewm" "$CONTENTS_DIR/MacOS/"
cp "$BUILD_DIR/glazewm-cli" "$CONTENTS_DIR/MacOS/"
chmod +x "$CONTENTS_DIR/MacOS"/*

# Convert PNG icon to ICNS.
ICONSET_DIR="$BUILD_DIR/icon.iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

ICON_PNG="$PROJECT_ROOT/resources/assets/icon.png"
sips -z 16 16     "$ICON_PNG" --out "$ICONSET_DIR/icon_16x16.png"      > /dev/null
sips -z 32 32     "$ICON_PNG" --out "$ICONSET_DIR/icon_16x16@2x.png"   > /dev/null
sips -z 32 32     "$ICON_PNG" --out "$ICONSET_DIR/icon_32x32.png"      > /dev/null
sips -z 64 64     "$ICON_PNG" --out "$ICONSET_DIR/icon_32x32@2x.png"   > /dev/null
sips -z 128 128   "$ICON_PNG" --out "$ICONSET_DIR/icon_128x128.png"    > /dev/null
sips -z 256 256   "$ICON_PNG" --out "$ICONSET_DIR/icon_128x128@2x.png" > /dev/null
sips -z 256 256   "$ICON_PNG" --out "$ICONSET_DIR/icon_256x256.png"    > /dev/null
sips -z 512 512   "$ICON_PNG" --out "$ICONSET_DIR/icon_256x256@2x.png" > /dev/null
sips -z 512 512   "$ICON_PNG" --out "$ICONSET_DIR/icon_512x512.png"    > /dev/null
sips -z 1024 1024 "$ICON_PNG" --out "$ICONSET_DIR/icon_512x512@2x.png" > /dev/null
iconutil -c icns "$ICONSET_DIR" -o "$CONTENTS_DIR/Resources/icon.icns"
rm -rf "$ICONSET_DIR"

VERSION="${VERSION_NUMBER:-0.0.0}"
sed "s/\${VERSION}/$VERSION/g" "$PROJECT_ROOT/resources/Info.plist" > "$CONTENTS_DIR/Info.plist"

echo -n "APPL????" > "$CONTENTS_DIR/PkgInfo"

echo "Done: $APP_DIR"
