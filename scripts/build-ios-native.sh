#!/usr/bin/env bash
set -euo pipefail

# Builds the Rust crate as a static library for Apple platforms and packages the
# slices into an XCFramework consumed by the TantivyKit Swift package.
#
# Slices produced:
#   - ios-arm64                      (device)
#   - ios-arm64_x86_64-simulator     (arm64 + x86_64 simulator, lipo'd)
#   - macos-arm64_x86_64             (arm64 + x86_64, lipo'd; lets `swift test`
#                                     run on a Mac without a simulator)
#
# Requirements: Xcode command line tools, rustup with the Apple targets below.
# Override BUILD_MACOS=0 to skip the macОС slice.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB_NAME="libtantivy_jni.a"
INCLUDE_DIR="$ROOT_DIR/include"
OUT_DIR="$ROOT_DIR/tantivy-ios"
XCFRAMEWORK="$OUT_DIR/TantivyFFI.xcframework"
BUILD_MACOS="${BUILD_MACOS:-1}"

IOS_TARGETS=(aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios)
MACOS_TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)

# Minimum OS versions. Keep in sync with the `platforms` in tantivy-ios/Package.swift
# so the archives link cleanly against consumers built for these floors. rustc
# reads the platform-appropriate variable per target.
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-13.0}"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"

targets=("${IOS_TARGETS[@]}")
if [[ "$BUILD_MACOS" == "1" ]]; then
  targets+=("${MACOS_TARGETS[@]}")
fi

for target in "${targets[@]}"; do
  rustup target add "$target" >/dev/null
  echo "==> cargo build --release --target $target"
  cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml" --target "$target"
done

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

# iOS device slice (single arch, used directly).
DEVICE_LIB="$ROOT_DIR/target/aarch64-apple-ios/release/$LIB_NAME"

# iOS simulator fat slice.
SIM_LIB="$STAGE/ios-sim/$LIB_NAME"
mkdir -p "$(dirname "$SIM_LIB")"
lipo -create \
  "$ROOT_DIR/target/aarch64-apple-ios-sim/release/$LIB_NAME" \
  "$ROOT_DIR/target/x86_64-apple-ios/release/$LIB_NAME" \
  -output "$SIM_LIB"

CREATE_ARGS=(
  -library "$DEVICE_LIB" -headers "$INCLUDE_DIR"
  -library "$SIM_LIB" -headers "$INCLUDE_DIR"
)

# macОС fat slice (optional).
if [[ "$BUILD_MACOS" == "1" ]]; then
  MAC_LIB="$STAGE/macos/$LIB_NAME"
  mkdir -p "$(dirname "$MAC_LIB")"
  lipo -create \
    "$ROOT_DIR/target/aarch64-apple-darwin/release/$LIB_NAME" \
    "$ROOT_DIR/target/x86_64-apple-darwin/release/$LIB_NAME" \
    -output "$MAC_LIB"
  CREATE_ARGS+=(-library "$MAC_LIB" -headers "$INCLUDE_DIR")
fi

rm -rf "$XCFRAMEWORK"
mkdir -p "$OUT_DIR"
xcodebuild -create-xcframework "${CREATE_ARGS[@]}" -output "$XCFRAMEWORK"

echo "==> Wrote $XCFRAMEWORK"
