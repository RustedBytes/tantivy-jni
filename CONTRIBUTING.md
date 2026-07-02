# Contributing

## Development Checks

Run these before opening a PR:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo audit
cargo deny check
./gradlew :tantivy-android:detekt
./gradlew :tantivy-android:dokkaGenerate
./gradlew apiCheck
./gradlew :tantivy-android:testDebugUnitTest
```

Android unit tests require a configured Android SDK. Either set `ANDROID_HOME` or add `sdk.dir` to `local.properties`.

## Native Android Build

Install the Android NDK version used by CI:

```bash
./install_android_ndk.sh
source .android/ndk.env
```

Build native libraries:

```bash
scripts/build-android-native.sh
```

The script writes generated `.so` files into `tantivy-android/src/main/jniLibs`, which is intentionally ignored by git.
By default it builds `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64`. To build a subset, set `ANDROID_ABIS`, for example:

```bash
ANDROID_ABIS="arm64-v8a x86_64" scripts/build-android-native.sh
```

Verify the generated JNI libraries and exported symbols:

```bash
scripts/verify-android-native.sh
```

## Release Process

1. Update `CHANGELOG.md`.
2. Ensure CI is green.
3. Create and push a tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release workflow builds JNI libraries, builds the release AAR, publishes a local Maven repository archive, publishes to GitHub Packages, and uploads all artifacts to the GitHub Release.
If `SIGNING_KEY` and `SIGNING_PASSWORD` secrets are configured, Gradle signs the release Maven publication with in-memory PGP keys.
Release artifacts include checksums, dependency metadata, and build metadata.

## API Stability

- High-level APIs are intended to evolve conservatively.
- APIs marked `@AdvancedTantivyApi` are opt-in and may change before `1.0.0`.
- Native cancellation is cooperative between JNI calls, not interruptible inside a single Tantivy operation.
- Public API changes must update the checked-in API snapshot with `./gradlew apiDump`.
