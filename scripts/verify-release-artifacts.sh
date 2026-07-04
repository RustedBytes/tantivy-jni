#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist}"
RELEASE_TAG_NAME="${RELEASE_TAG_NAME:-${GITHUB_REF_NAME:-local}}"
ANDROID_ABIS="${ANDROID_ABIS:-arm64-v8a armeabi-v7a x86 x86_64}"

require_file() {
  local path="$1"
  if [[ ! -s "$path" ]]; then
    echo "Missing or empty release artifact: $path" >&2
    exit 1
  fi
}

require_zip_entry() {
  local archive="$1"
  local pattern="$2"
  if ! unzip -l "$archive" | grep -F "$pattern" >/dev/null; then
    echo "Archive $archive does not contain expected entry matching: $pattern" >&2
    exit 1
  fi
}

require_tar_entry() {
  local archive="$1"
  local pattern="$2"
  if ! tar -tzf "$archive" | grep -F "$pattern" >/dev/null; then
    echo "Archive $archive does not contain expected entry matching: $pattern" >&2
    exit 1
  fi
}

validate_json() {
  local path="$1"
  python3 -m json.tool "$path" >/dev/null
}

require_file "$DIST_DIR/tantivy-android-${RELEASE_TAG_NAME}.aar"
require_file "$DIST_DIR/sample-app-${RELEASE_TAG_NAME}-release.apk"
require_file "$DIST_DIR/tantivy-android-${RELEASE_TAG_NAME}-maven-repository.tar.gz"
require_file "$DIST_DIR/tantivy-jni-${RELEASE_TAG_NAME}-android-jni.tar.gz"
require_file "$DIST_DIR/tantivy-jni-${RELEASE_TAG_NAME}-android-jni.zip"
require_file "$DIST_DIR/cargo-metadata-${RELEASE_TAG_NAME}.json"
require_file "$DIST_DIR/rust-cyclonedx-${RELEASE_TAG_NAME}.json"
require_file "$DIST_DIR/gradle-cyclonedx-${RELEASE_TAG_NAME}.json"
require_file "$DIST_DIR/gradle-cyclonedx-${RELEASE_TAG_NAME}.xml"
require_file "$DIST_DIR/gradle-release-runtime-dependencies-${RELEASE_TAG_NAME}.txt"
require_file "$DIST_DIR/build-info-${RELEASE_TAG_NAME}.json"

validate_json "$DIST_DIR/cargo-metadata-${RELEASE_TAG_NAME}.json"
validate_json "$DIST_DIR/rust-cyclonedx-${RELEASE_TAG_NAME}.json"
validate_json "$DIST_DIR/gradle-cyclonedx-${RELEASE_TAG_NAME}.json"
validate_json "$DIST_DIR/build-info-${RELEASE_TAG_NAME}.json"

require_tar_entry "$DIST_DIR/tantivy-android-${RELEASE_TAG_NAME}-maven-repository.tar.gz" "tantivy-android"

for abi in $ANDROID_ABIS; do
  require_file "$DIST_DIR/libtantivy_jni-${RELEASE_TAG_NAME}-${abi}.so"
  require_zip_entry "$DIST_DIR/tantivy-android-${RELEASE_TAG_NAME}.aar" "jni/$abi/libtantivy_jni.so"
  require_zip_entry "$DIST_DIR/tantivy-jni-${RELEASE_TAG_NAME}-android-jni.zip" "jniLibs/$abi/libtantivy_jni.so"
  require_tar_entry "$DIST_DIR/tantivy-jni-${RELEASE_TAG_NAME}-android-jni.tar.gz" "jniLibs/$abi/libtantivy_jni.so"
done
