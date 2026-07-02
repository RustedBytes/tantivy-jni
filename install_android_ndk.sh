#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

NDK_VERSION="${NDK_VERSION:-27.2.12479018}"
NDK_RELEASE="${NDK_RELEASE:-r27c}"
INSTALL_ROOT="${ANDROID_NDK_INSTALL_ROOT:-$ROOT_DIR/.android/ndk}"
DOWNLOAD_DIR="${ANDROID_NDK_DOWNLOAD_DIR:-$ROOT_DIR/.android/downloads}"
ENV_FILE="${ANDROID_NDK_ENV_FILE:-$ROOT_DIR/.android/ndk.env}"
FORCE=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--force] [--help]

Downloads Android NDK $NDK_VERSION ($NDK_RELEASE) into:
  $INSTALL_ROOT/$NDK_VERSION

Environment overrides:
  NDK_VERSION                 Android NDK package revision. Default: $NDK_VERSION
  NDK_RELEASE                 Android NDK archive release name. Default: $NDK_RELEASE
  ANDROID_NDK_INSTALL_ROOT    Install root. Default: $ROOT_DIR/.android/ndk
  ANDROID_NDK_DOWNLOAD_DIR    Archive cache dir. Default: $ROOT_DIR/.android/downloads
  ANDROID_NDK_ENV_FILE        Env file written after install. Default: $ROOT_DIR/.android/ndk.env
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force)
      FORCE=1
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
  shift
done

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required command not found: $1" >&2
    exit 1
  fi
}

download_file() {
  local url="$1"
  local output="$2"

  if command -v curl >/dev/null 2>&1; then
    curl --fail --location --retry 3 --continue-at - --output "$output" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget --continue --output-document="$output" "$url"
  else
    echo "Either curl or wget is required to download the NDK" >&2
    exit 1
  fi
}

sha1_file() {
  local file="$1"

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 1 "$file" | awk '{print $1}'
  elif command -v sha1sum >/dev/null 2>&1; then
    sha1sum "$file" | awk '{print $1}'
  else
    return 1
  fi
}

detect_package() {
  local os
  os="$(uname -s)"

  case "$os" in
    Darwin)
      # Google documents this zip URL pattern for automated macOS downloads.
      PACKAGE_NAME="android-ndk-$NDK_RELEASE-darwin.zip"
      EXPECTED_SHA1="${ANDROID_NDK_SHA1:-}"
      ;;
    Linux)
      PACKAGE_NAME="android-ndk-$NDK_RELEASE-linux.zip"
      EXPECTED_SHA1="${ANDROID_NDK_SHA1:-090e8083a715fdb1a3e402d0763c388abb03fb4e}"
      ;;
    *)
      echo "Unsupported OS for direct NDK install: $os" >&2
      echo "Install Android NDK $NDK_VERSION manually and set ANDROID_NDK_HOME." >&2
      exit 1
      ;;
  esac
}

verify_checksum() {
  local archive="$1"

  if [[ -z "$EXPECTED_SHA1" ]]; then
    echo "No SHA-1 checksum configured for $PACKAGE_NAME; relying on HTTPS download."
    return 0
  fi

  local actual_sha1
  if ! actual_sha1="$(sha1_file "$archive")"; then
    echo "No SHA-1 tool found; unable to verify $PACKAGE_NAME" >&2
    exit 1
  fi

  if [[ "$actual_sha1" != "$EXPECTED_SHA1" ]]; then
    echo "Checksum mismatch for $archive" >&2
    echo "Expected: $EXPECTED_SHA1" >&2
    echo "Actual:   $actual_sha1" >&2
    exit 1
  fi
}

validate_install() {
  local ndk_home="$1"
  local source_properties="$ndk_home/source.properties"

  if [[ ! -f "$source_properties" ]]; then
    echo "Invalid NDK install: missing $source_properties" >&2
    exit 1
  fi

  local installed_version
  installed_version="$(sed -n 's/^Pkg.Revision *= *//p' "$source_properties" | head -n 1)"
  if [[ "$installed_version" != "$NDK_VERSION" ]]; then
    echo "Installed NDK version mismatch" >&2
    echo "Expected: $NDK_VERSION" >&2
    echo "Actual:   $installed_version" >&2
    exit 1
  fi

  if ! find "$ndk_home/toolchains/llvm/prebuilt" -mindepth 1 -maxdepth 1 -type d | grep -q .; then
    echo "Invalid NDK install: missing LLVM prebuilt toolchain" >&2
    exit 1
  fi
}

write_env_file() {
  local ndk_home="$1"
  mkdir -p "$(dirname "$ENV_FILE")"
  {
    printf 'export ANDROID_NDK_HOME=%q\n' "$ndk_home"
    printf 'export ANDROID_NDK_ROOT="${ANDROID_NDK_HOME}"\n'
  } > "$ENV_FILE"
}

detect_package
require_command unzip

NDK_HOME="$INSTALL_ROOT/$NDK_VERSION"

if [[ -d "$NDK_HOME" && "$FORCE" -eq 0 ]]; then
  validate_install "$NDK_HOME"
  write_env_file "$NDK_HOME"
  echo "Android NDK already installed: $NDK_HOME"
  echo "To use it in this shell: source \"$ENV_FILE\""
  exit 0
fi

mkdir -p "$DOWNLOAD_DIR" "$INSTALL_ROOT"

ARCHIVE="$DOWNLOAD_DIR/$PACKAGE_NAME"
URL="https://dl.google.com/android/repository/$PACKAGE_NAME"

if [[ "$FORCE" -eq 1 ]]; then
  rm -f "$ARCHIVE"
fi

if [[ ! -s "$ARCHIVE" || "$FORCE" -eq 1 ]]; then
  echo "Downloading $PACKAGE_NAME"
  download_file "$URL" "$ARCHIVE"
fi

verify_checksum "$ARCHIVE"

TMP_DIR="$ROOT_DIR/.android/tmp/install-ndk"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "Extracting $PACKAGE_NAME"
unzip -q "$ARCHIVE" -d "$TMP_DIR"

EXTRACTED_DIR="$TMP_DIR/android-ndk-$NDK_RELEASE"
if [[ ! -d "$EXTRACTED_DIR" ]]; then
  EXTRACTED_DIR="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -type d -name 'android-ndk-*' | head -n 1)"
fi

if [[ -z "${EXTRACTED_DIR:-}" || ! -d "$EXTRACTED_DIR" ]]; then
  echo "Unable to find extracted NDK directory in $TMP_DIR" >&2
  exit 1
fi

rm -rf "$NDK_HOME"
mv "$EXTRACTED_DIR" "$NDK_HOME"
rm -rf "$TMP_DIR"

validate_install "$NDK_HOME"
write_env_file "$NDK_HOME"

echo "Installed Android NDK $NDK_VERSION at $NDK_HOME"
echo "To use it in this shell: source \"$ENV_FILE\""
echo "Example: ANDROID_NDK_HOME=\"$NDK_HOME\" scripts/build-android-native.sh"
