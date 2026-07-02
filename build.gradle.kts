import org.cyclonedx.gradle.CyclonedxAggregateTask
import org.cyclonedx.gradle.CyclonedxDirectTask

plugins {
    id("com.android.application") version "9.2.1" apply false
    id("com.android.library") version "9.2.0" apply false
    id("dev.detekt") version "2.0.0-alpha.5" apply false
    id("org.cyclonedx.bom") version "3.2.4"
    id("org.jetbrains.dokka") version "2.2.0" apply false
}

group = providers.gradleProperty("GROUP").get()
version = providers.gradleProperty("VERSION_NAME").get()

allprojects {
    group = rootProject.group
    version = rootProject.version

    tasks.withType<CyclonedxDirectTask>().configureEach {
        includeConfigs.set(listOf("releaseRuntimeClasspath"))
        includeMetadataResolution.set(false)
        includeBuildEnvironment.set(false)
    }
}

tasks.withType<CyclonedxAggregateTask>().configureEach {
    componentGroup.set(providers.gradleProperty("GROUP"))
    componentName.set(rootProject.name)
    componentVersion.set(providers.gradleProperty("VERSION_NAME"))
    jsonOutput.set(layout.buildDirectory.file("reports/cyclonedx/bom.json"))
    xmlOutput.set(layout.buildDirectory.file("reports/cyclonedx/bom.xml"))
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
