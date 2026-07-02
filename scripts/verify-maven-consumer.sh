#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_NAME="${VERSION_NAME:-0.1.0-SNAPSHOT}"
PUBLISH_ARTIFACT="${PUBLISH_ARTIFACT:-true}"

if [[ "$PUBLISH_ARTIFACT" == "true" ]]; then
  "$ROOT_DIR/gradlew" \
    --no-daemon \
    --console=plain \
    -PVERSION_NAME="$VERSION_NAME" \
    :tantivy-android:publishReleasePublicationToReleaseRepository
fi

"$ROOT_DIR/gradlew" \
  --no-daemon \
  --console=plain \
  -p "$ROOT_DIR/fixtures/maven-consumer" \
  -PVERSION_NAME="$VERSION_NAME" \
  :app:assembleRelease
