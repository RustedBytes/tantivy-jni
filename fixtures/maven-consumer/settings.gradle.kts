pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        maven {
            url = uri("../../tantivy-android/build/repository")
        }
        google()
        mavenCentral()
    }
}

rootProject.name = "tantivy-maven-consumer"
include(":app")
