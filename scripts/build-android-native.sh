#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ANDROID_MODULE="$ROOT_DIR/tantivy-android/src/main/jniLibs"
ANDROID_ABIS="${ANDROID_ABIS:-arm64-v8a armeabi-v7a x86 x86_64}"

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo "ANDROID_NDK_HOME is required" >&2
  exit 1
fi

TOOLCHAIN_DIR="$(find "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [[ -z "$TOOLCHAIN_DIR" ]]; then
  echo "Unable to find Android NDK LLVM prebuilt toolchain" >&2
  exit 1
fi

target_for_abi() {
  case "$1" in
    arm64-v8a) echo "aarch64-linux-android" ;;
    armeabi-v7a) echo "armv7-linux-androideabi" ;;
    x86) echo "i686-linux-android" ;;
    x86_64) echo "x86_64-linux-android" ;;
    *)
      echo "Unsupported Android ABI: $1" >&2
      return 1
      ;;
  esac
}

cc_for_abi() {
  case "$1" in
    arm64-v8a) echo "$TOOLCHAIN_DIR/bin/aarch64-linux-android23-clang" ;;
    armeabi-v7a) echo "$TOOLCHAIN_DIR/bin/armv7a-linux-androideabi23-clang" ;;
    x86) echo "$TOOLCHAIN_DIR/bin/i686-linux-android23-clang" ;;
    x86_64) echo "$TOOLCHAIN_DIR/bin/x86_64-linux-android23-clang" ;;
    *)
      echo "Unsupported Android ABI: $1" >&2
      return 1
      ;;
  esac
}

env_key_for_target() {
  echo "$1" | tr '[:lower:]-' '[:lower:]_'
}

rm -rf "$ANDROID_MODULE"

for abi in $ANDROID_ABIS; do
  target="$(target_for_abi "$abi")"
  target_key="$(env_key_for_target "$target")"
  rustup target add "$target"

  export "AR_${target_key}=$TOOLCHAIN_DIR/bin/llvm-ar"
  export "CC_${target_key}=$(cc_for_abi "$abi")"

  cargo build --release --target "$target"

  mkdir -p "$ANDROID_MODULE/$abi"
  cp "$ROOT_DIR/target/$target/release/libtantivy_jni.so" "$ANDROID_MODULE/$abi/libtantivy_jni.so"
done
