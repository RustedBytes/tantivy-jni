# Changelog

All notable changes to this project will be documented in this file.

This project follows semantic versioning after the first stable `1.0.0` release.

## Unreleased

- Added a C ABI (`src/ffi_bridge.rs`) over the same core operations as the JNI bridge, for non-JVM consumers.
- Added `staticlib` to the crate output for Apple static linking.
- Added the `TantivyKit` Swift package (`tantivy-ios`) with a typed iOS/macOS API mirroring the Kotlin bindings, including `async`/`await` index and search operations.
- Added `scripts/build-ios-native.sh` to build the native `TantivyFFI.xcframework` (iOS device, iOS simulator, and macOS slices).
- Added a C header and module map (`include/`) for the Swift module.
- Added a Swift iOS CI workflow that builds the XCFramework and runs Swift tests.
- Added a tag-based iOS release workflow that publishes the zipped `TantivyFFI.xcframework` with a SwiftPM checksum for remote `binaryTarget` consumption.

## 0.1.0 - 2026-07-02

- Added Android Kotlin API backed by Rust/Tantivy through JNI.
- Added coroutine-native indexing and search APIs.
- Added typed schema, document, search, batch, and result models.
- Added advanced opt-in APIs for native search, schema info, and commit-and-refresh.
- Added Rust and Kotlin tests.
- Added Detekt static analysis.
- Added Dokka API documentation generation.
- Added checked-in Kotlin public API snapshot validation.
- Added consumer R8/ProGuard keep rules for the JNI bridge.
- Added optional in-memory PGP signing for release Maven publications.
- Added Gradle wrapper validation to Android and release workflows.
- Added GitHub Packages Maven publishing in the release workflow.
- Added Rust dependency policy checks with cargo-deny.
- Added security policy and release dependency/build metadata artifacts.
- Added GitHub Actions CI for Rust and Android/Kotlin checks.
- Added tag-based release workflow for Android JNI and AAR artifacts.
- Added Maven publishing configuration for the Android library.
- Added release SBOMs, artifact verification, and an external Maven consumer fixture.
- Added GitHub artifact attestations for release provenance and SBOMs.
