#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ANDROID_MODULE="$ROOT_DIR/tantivy-android/src/main/jniLibs"

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo "ANDROID_NDK_HOME is required" >&2
  exit 1
fi

rustup target add aarch64-linux-android x86_64-linux-android

TOOLCHAIN_DIR="$(find "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [[ -z "$TOOLCHAIN_DIR" ]]; then
  echo "Unable to find Android NDK LLVM prebuilt toolchain" >&2
  exit 1
fi

export AR_aarch64_linux_android="$TOOLCHAIN_DIR/bin/llvm-ar"
export AR_x86_64_linux_android="$TOOLCHAIN_DIR/bin/llvm-ar"
export CC_aarch64_linux_android="$TOOLCHAIN_DIR/bin/aarch64-linux-android23-clang"
export CC_x86_64_linux_android="$TOOLCHAIN_DIR/bin/x86_64-linux-android23-clang"

cargo build --release --target aarch64-linux-android
cargo build --release --target x86_64-linux-android

mkdir -p "$ANDROID_MODULE/arm64-v8a" "$ANDROID_MODULE/x86_64"
cp "$ROOT_DIR/target/aarch64-linux-android/release/libtantivy_jni.so" "$ANDROID_MODULE/arm64-v8a/libtantivy_jni.so"
cp "$ROOT_DIR/target/x86_64-linux-android/release/libtantivy_jni.so" "$ANDROID_MODULE/x86_64/libtantivy_jni.so"
