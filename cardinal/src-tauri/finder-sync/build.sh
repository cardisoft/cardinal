#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
OUTPUT_DIR="$SCRIPT_DIR/build/CardinalFinderSync.appex"
CONTENTS_DIR="$OUTPUT_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RUST_SYSROOT=$(rustc --print sysroot)
LINKER="$RUST_SYSROOT/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/ld64.lld"

rm -rf "$OUTPUT_DIR"
mkdir -p "$MACOS_DIR"

xcrun clang "-fuse-ld=$LINKER" -mmacosx-version-min=12.0 \
  -fobjc-arc -framework Cocoa -framework FinderSync \
  "$SCRIPT_DIR/main.m" \
  "$SCRIPT_DIR/CardinalFinderSync.m" \
  -o "$MACOS_DIR/CardinalFinderSync"

cp "$SCRIPT_DIR/Info.plist" "$CONTENTS_DIR/Info.plist"
codesign --force --sign - --options runtime \
  --entitlements "$SCRIPT_DIR/entitlements.plist" \
  --identifier com.cardinal.one.findersync \
  "$OUTPUT_DIR"
