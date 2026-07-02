# Changelog

All notable changes to this project will be documented in this file.

This project follows semantic versioning after the first stable `1.0.0` release.

## Unreleased

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
