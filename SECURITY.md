# Security Policy

## Supported Versions

This project is pre-1.0. Security fixes are applied to the latest released version only until a stable compatibility policy is published.

## Reporting A Vulnerability

Please report vulnerabilities privately through GitHub security advisories if available for the repository. If advisories are not enabled, contact the maintainers privately before opening a public issue.

Include:

- affected version or commit
- Android API level and ABI, if relevant
- whether the issue is in Kotlin, JNI, Rust, packaging, or release artifacts
- minimal reproduction steps
- expected and actual impact

## Security Checks

The project uses:

- `cargo audit` for RustSec vulnerability checks
- `cargo deny check` for Rust advisory, license, source, and duplicate dependency policy
- Detekt for Kotlin static analysis
- Android Lint in CI
- JNI symbol verification for release artifacts
- Gradle wrapper validation in CI

Release artifacts include checksums and dependency/build metadata.
