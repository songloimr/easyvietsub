#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
LAUNCHER_MANIFEST="$ROOT_DIR/src-tauri/sidecar-launcher/Cargo.toml"
TARGET_TRIPLE="${1:-$(rustc -vV | awk '/host:/ {print $2}')}"
BIN_DIR="$ROOT_DIR/src-tauri/binaries"

mkdir -p "$BIN_DIR"

cargo build --manifest-path "$LAUNCHER_MANIFEST" --release --target "$TARGET_TRIPLE"

OUTPUT_DIR="$ROOT_DIR/src-tauri/sidecar-launcher/target/$TARGET_TRIPLE/release"
EXT=""

case "$TARGET_TRIPLE" in
  *windows*) EXT=".exe" ;;
esac

cp "$OUTPUT_DIR/easyvietsub-sidecar-launcher$EXT" "$BIN_DIR/ffmpeg-sidecar-$TARGET_TRIPLE$EXT"
cp "$OUTPUT_DIR/easyvietsub-sidecar-launcher$EXT" "$BIN_DIR/ffprobe-sidecar-$TARGET_TRIPLE$EXT"

chmod +x "$BIN_DIR/ffmpeg-sidecar-$TARGET_TRIPLE$EXT" "$BIN_DIR/ffprobe-sidecar-$TARGET_TRIPLE$EXT" || true

echo "Built sidecars for $TARGET_TRIPLE into $BIN_DIR"
