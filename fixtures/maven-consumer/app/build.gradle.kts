plugins {
    id("com.android.application")
}

android {
    namespace = "com.rustedbytes.tantivy.fixture"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.rustedbytes.tantivy.fixture"
        minSdk = 23
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    compileOptions {
        isCoreLibraryDesugaringEnabled = true
    }
}

dependencies {
    val tantivyVersion = providers.gradleProperty("VERSION_NAME").orElse("0.1.0-SNAPSHOT")

    implementation("com.rustedbytes:tantivy-android:${tantivyVersion.get()}")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.5")
}
