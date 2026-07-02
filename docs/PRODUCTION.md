# Production Readiness

This project is pre-1.0. Treat each release as production-ready only after the checks below pass for the exact commit and tag being shipped.

## Required Gates

Run locally or in CI:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo audit
cargo deny check
cargo cyclonedx --format json --spec-version 1.5
./gradlew :tantivy-android:detekt
./gradlew :tantivy-android:lintRelease
./gradlew :tantivy-android:dokkaGenerate
./gradlew apiCheck
./gradlew :tantivy-android:testDebugUnitTest
./gradlew :sample-app:assembleRelease
```

Run with an Android SDK, NDK, and emulator:

```bash
scripts/build-android-native.sh
scripts/verify-android-native.sh
./gradlew :tantivy-android:connectedDebugAndroidTest
./gradlew :sample-app:connectedDebugAndroidTest
```

## Release Candidate Flow

1. Tag an RC, for example `v0.1.0-rc1`.
2. Confirm `.github/workflows/release.yml` builds all configured Android ABIs.
3. Download the generated AAR, JNI archives, Maven repository archive, metadata, and checksums.
4. Verify the Rust and Gradle CycloneDX SBOM files are present in the release assets.
5. Verify checksums before distributing artifacts.
6. Consume the AAR from a separate Android project, not only from the in-repo sample.
7. Promote to a final tag only after the separate app can index, commit, refresh, search, close, and reopen an index.

## Compatibility Matrix

Current tested targets:

- Android min SDK 23
- Android compile SDK 36
- Android NDK 27.2.12479018
- Android ABIs: `arm64-v8a`, `armeabi-v7a`, `x86`, `x86_64`
- Rust 1.96.1
- Gradle wrapper from this repository

## API Stability

The high-level Kotlin coroutine API should remain source-compatible across patch releases. Any source-incompatible change must be documented in `CHANGELOG.md`.

The following surfaces may change before 1.0:

- APIs annotated with `@AdvancedTantivyApi`
- native JSON contracts
- release artifact filenames
- internal Rust module layout

## Native Safety Expectations

Native operations are synchronous once entered. Kotlin cancellation is cooperative between JNI calls, especially between indexing batches. Closing an index prevents new operations from starting, while an operation that has already entered native code may finish.

Rust keeps native resources behind opaque handles and validates handles on every entry. JNI catches Rust panics and maps native errors to typed Kotlin exceptions.

## Consumer App Proof

The `sample-app` module is a consumer smoke test. It must keep building as a release APK with R8 enabled. This verifies:

- published consumer ProGuard rules preserve JNI entry points
- app code can use coroutine APIs from UI code
- the AAR packaging model works for Android application consumers
- the sample app launches on an emulator with packaged JNI libraries

Before a stable 1.0 release, also maintain at least one external sample or fixture project that depends on the published artifact instead of `project(":tantivy-android")`.
