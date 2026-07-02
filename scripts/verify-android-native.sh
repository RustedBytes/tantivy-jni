#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JNI_LIBS_DIR="${JNI_LIBS_DIR:-$ROOT_DIR/tantivy-android/src/main/jniLibs}"
ANDROID_ABIS="${ANDROID_ABIS:-arm64-v8a armeabi-v7a x86 x86_64}"

required_symbols=(
  Java_com_rustedbytes_tantivy_NativeTantivy_nativeOpenIndex
  Java_com_rustedbytes_tantivy_NativeTantivy_nativeCloseIndex
  Java_com_rustedbytes_tantivy_NativeTantivy_nativeAddDocuments
  Java_com_rustedbytes_tantivy_NativeTantivy_nativeDeleteTerm
  Java_com_rustedbytes_tantivy_NativeTantivy_nativeCommit
  Java_com_rustedbytes_tantivy_NativeTantivy_nativeRefresh
  Java_com_rustedbytes_tantivy_NativeTantivy_nativeCommitAndRefresh
  Java_com_rustedbytes_tantivy_NativeTantivy_nativeSchemaInfo
  Java_com_rustedbytes_tantivy_NativeTantivy_nativeSearch
)

nm_tool="nm"
if [[ -n "${ANDROID_NDK_HOME:-}" ]]; then
  toolchain_dir="$(find "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
  if [[ -n "$toolchain_dir" && -x "$toolchain_dir/bin/llvm-nm" ]]; then
    nm_tool="$toolchain_dir/bin/llvm-nm"
  fi
fi

for abi in $ANDROID_ABIS; do
  library="$JNI_LIBS_DIR/$abi/libtantivy_jni.so"
  if [[ ! -s "$library" ]]; then
    echo "Missing JNI library for $abi: $library" >&2
    exit 1
  fi

  symbols="$("$nm_tool" -D --defined-only "$library")"
  for symbol in "${required_symbols[@]}"; do
    if ! grep -q "$symbol" <<<"$symbols"; then
      echo "Missing symbol $symbol in $library" >&2
      exit 1
    fi
  done
done
