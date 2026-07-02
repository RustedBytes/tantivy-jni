#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_VERSION="${RELEASE_VERSION:-${1:-${GITHUB_REF_NAME:-}}}"
RELEASE_VERSION="${RELEASE_VERSION#v}"

if [[ -z "$RELEASE_VERSION" ]]; then
  echo "Release version is required. Pass RELEASE_VERSION, GITHUB_REF_NAME, or an argument." >&2
  exit 1
fi

if [[ "$RELEASE_VERSION" == *SNAPSHOT* ]]; then
  echo "Release version must not contain SNAPSHOT: $RELEASE_VERSION" >&2
  exit 1
fi

if [[ ! "$RELEASE_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "Release version must be semantic version like 0.1.0 or 0.1.0-rc1: $RELEASE_VERSION" >&2
  exit 1
fi

BASE_VERSION="${RELEASE_VERSION%%-*}"
CARGO_VERSION="$(awk -F ' *= *' '/^version *=/ { gsub(/"/, "", $2); print $2; exit }' "$ROOT_DIR/Cargo.toml")"

if [[ "$CARGO_VERSION" != "$BASE_VERSION" ]]; then
  echo "Cargo.toml version ($CARGO_VERSION) must match release base version ($BASE_VERSION)." >&2
  exit 1
fi

if ! grep -Eq "^##[[:space:]]+(\\[$BASE_VERSION\\]|$BASE_VERSION)([[:space:]]|$)" "$ROOT_DIR/CHANGELOG.md"; then
  echo "CHANGELOG.md must contain a release heading for $BASE_VERSION." >&2
  echo "Expected a heading like '## $BASE_VERSION' or '## [$BASE_VERSION]'." >&2
  exit 1
fi
