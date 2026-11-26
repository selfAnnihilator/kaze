pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }
dependencyResolutionManagement { repositories { google(); mavenCentral() } }
rootProject.name = "KazeLifecycleQA"
include(":app", ":tauri-plugin-kaze-media", ":tauri-android")
val repo = settingsDir.resolve("../../..").canonicalFile
project(":tauri-plugin-kaze-media").projectDir = repo.resolve("plugins/kaze-media/android")
val tauriSettings = repo.resolve("src-tauri/gen/android/tauri.settings.gradle").readText()
val tauriPath = Regex("project\\(':tauri-android'\\).projectDir = new File\\(\"([^\"]+)\"\\)")
    .find(tauriSettings)?.groupValues?.get(1) ?: error("Initialize the project's Android target first")
project(":tauri-android").projectDir = file(tauriPath)
