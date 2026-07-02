#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist}"
RELEASE_TAG_NAME="${RELEASE_TAG_NAME:-${GITHUB_REF_NAME:-local}}"
RELEASE_VERSION="${RELEASE_VERSION:-${RELEASE_TAG_NAME#v}}"

mkdir -p "$DIST_DIR"

cargo metadata --format-version 1 --locked > "$DIST_DIR/cargo-metadata-${RELEASE_TAG_NAME}.json"

"$ROOT_DIR/gradlew" \
  --no-daemon \
  --console=plain \
  :tantivy-android:dependencies \
  --configuration releaseRuntimeClasspath \
  > "$DIST_DIR/gradle-release-runtime-dependencies-${RELEASE_TAG_NAME}.txt"

cat > "$DIST_DIR/build-info-${RELEASE_TAG_NAME}.json" <<EOF
{
  "name": "tantivy-jni",
  "version": "$RELEASE_VERSION",
  "gitRef": "${GITHUB_REF:-}",
  "gitSha": "${GITHUB_SHA:-$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || true)}",
  "rustc": "$(rustc --version)",
  "cargo": "$(cargo --version)",
  "gradle": "$("$ROOT_DIR/gradlew" --version | awk '/^Gradle / { print $2; exit }')",
  "androidNdkVersion": "${ANDROID_NDK_VERSION:-}",
  "androidAbis": "${ANDROID_ABIS:-}"
}
EOF
