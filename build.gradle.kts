plugins {
    id("com.android.library") version "9.2.0" apply false
    id("dev.detekt") version "2.0.0-alpha.5" apply false
    id("org.jetbrains.dokka") version "2.2.0" apply false
}

tasks.register<Exec>("apiDump") {
    description = "Updates the checked-in Kotlin public API snapshot."
    group = "verification"
    commandLine(
        "python3",
        "scripts/kotlin-api-dump.py",
        "--source",
        "tantivy-android/src/main/kotlin",
        "--output",
        "api/tantivy-android.api",
    )
}

tasks.register<Exec>("apiCheck") {
    description = "Checks that the Kotlin public API snapshot is up to date."
    group = "verification"
    commandLine(
        "python3",
        "scripts/kotlin-api-dump.py",
        "--source",
        "tantivy-android/src/main/kotlin",
        "--check",
        "api/tantivy-android.api",
    )
}
