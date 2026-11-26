plugins { id("com.android.application"); id("org.jetbrains.kotlin.android") }
android {
    namespace = "org.intelligentmusicplayer.lifecycleqa"
    compileSdk = 37
    defaultConfig {
        applicationId = "org.intelligentmusicplayer.lifecycleqa"
        minSdk = 24
        targetSdk = 37
        versionCode = 1
        versionName = "1"
    }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_1_8; targetCompatibility = JavaVersion.VERSION_1_8 }
}
kotlin { compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_1_8) } }
dependencies {
    implementation(project(":tauri-plugin-kaze-media"))
    implementation(project(":tauri-android"))
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.media3:media3-common:1.4.1")
    implementation("androidx.media3:media3-session:1.4.1")
}
