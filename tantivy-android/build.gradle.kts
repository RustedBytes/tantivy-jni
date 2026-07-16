plugins {
    id("com.android.library")
    id("dev.detekt")
    id("maven-publish")
    id("org.jetbrains.dokka")
    id("signing")
    id("jacoco")
}

android {
    namespace = "com.rustedbytes.tantivy"
    compileSdk = 36

    defaultConfig {
        minSdk = 23
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
        }
    }

    compileOptions {
        isCoreLibraryDesugaringEnabled = true
    }

    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }
}

dependencies {
    api("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.5")

    testImplementation("org.jetbrains.kotlin:kotlin-test-junit:2.4.10")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.11.0")
    testImplementation("org.json:json:20260522")

    androidTestImplementation("org.jetbrains.kotlin:kotlin-test-junit:2.4.10")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    androidTestImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.11.0")
}

detekt {
    toolVersion = "2.0.0-alpha.5"
    buildUponDefaultConfig = true
    allRules = false
    config.setFrom(files("$rootDir/config/detekt/detekt.yml"))
    source.setFrom(
        files(
            "src/main/kotlin",
            "src/test/kotlin",
            "src/androidTest/kotlin",
        ),
    )
}

tasks.register<JacocoReport>("jacocoTestReport") {
    dependsOn("testDebugUnitTest")
    reports {
        xml.required.set(true)
        html.required.set(true)
    }
    classDirectories.setFrom(
        fileTree("build/intermediates/built_in_kotlinc/debug/compileDebugKotlin/classes")
    )
    sourceDirectories.setFrom(
        files("src/main/kotlin")
    )
    executionData.setFrom(
        files("build/outputs/unit_test_code_coverage/debugUnitTest/testDebugUnitTest.exec", "build/jacoco/testDebugUnitTest.exec")
    )
}

publishing {
    publications {
        register<MavenPublication>("release") {
            groupId = providers.gradleProperty("GROUP").get()
            artifactId = providers.gradleProperty("POM_ARTIFACT_ID").get()
            version = providers.gradleProperty("VERSION_NAME").get()

            afterEvaluate {
                from(components["release"])
            }

            pom {
                name.set(providers.gradleProperty("POM_NAME"))
                description.set(providers.gradleProperty("POM_DESCRIPTION"))
                url.set(providers.gradleProperty("POM_URL"))
                licenses {
                    license {
                        name.set("Apache License 2.0")
                        url.set("https://www.apache.org/licenses/LICENSE-2.0")
                    }
                }
                developers {
                    developer {
                        id.set("rustedbytes")
                        name.set("Rusted Bytes")
                    }
                }
                scm {
                    connection.set("scm:git:https://github.com/rustedbytes/tantivy-jni.git")
                    developerConnection.set("scm:git:ssh://git@github.com/rustedbytes/tantivy-jni.git")
                    url.set("https://github.com/rustedbytes/tantivy-jni")
                }
            }
        }
    }

    repositories {
        maven {
            name = "release"
            url = uri(layout.buildDirectory.dir("repository").get().asFile)
        }
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/${System.getenv("GITHUB_REPOSITORY") ?: "rustedbytes/tantivy-jni"}")
            credentials {
                username = providers.gradleProperty("gpr.user")
                    .orElse(providers.environmentVariable("GITHUB_ACTOR"))
                    .orNull
                password = providers.gradleProperty("gpr.key")
                    .orElse(providers.environmentVariable("GITHUB_TOKEN"))
                    .orNull
            }
        }
    }
}

signing {
    val signingKey = providers.gradleProperty("SIGNING_KEY")
        .orElse(providers.environmentVariable("SIGNING_KEY"))
    val signingPassword = providers.gradleProperty("SIGNING_PASSWORD")
        .orElse(providers.environmentVariable("SIGNING_PASSWORD"))
    val shouldSign = signingKey.isPresent && signingPassword.isPresent

    setRequired { shouldSign && gradle.taskGraph.allTasks.any { it.name.contains("ReleasePublication") } }

    if (shouldSign) {
        useInMemoryPgpKeys(signingKey.get(), signingPassword.get())
        sign(publishing.publications["release"])
    }
}
