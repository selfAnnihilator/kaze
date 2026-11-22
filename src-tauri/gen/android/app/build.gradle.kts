import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 37
    namespace = "org.intelligentmusicplayer.app"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "org.intelligentmusicplayer.app"
        minSdk = 24
        targetSdk = 37
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {
                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            optimization {
               enable = true
            }
            proguardFiles(
                *fileTree(".") {
                  include("**/*.pro")
                  exclude("build/**")
                }.files.toTypedArray()
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    buildFeatures {
        buildConfig = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_1_8
    }
}

rust {
    rootDirRel = "../../../"
}

// The Rust backend links libc++_shared.so (oboe is C++), so ship the NDK copy.
val copyLibcxx = tasks.register("copyLibcxxShared") {
    val ndkDir = System.getenv("NDK_HOME")?.let { file(it) } ?: android.ndkDirectory
    val outDir = layout.projectDirectory.dir("src/main/jniLibs")
    val abis = mapOf(
        "arm64-v8a" to "aarch64-linux-android",
        "armeabi-v7a" to "arm-linux-androideabi",
        "x86" to "i686-linux-android",
        "x86_64" to "x86_64-linux-android",
    )
    doLast {
        val prebuilt = File(ndkDir, "toolchains/llvm/prebuilt").listFiles()?.firstOrNull()
            ?: throw GradleException("NDK prebuilt toolchain not found in $ndkDir")
        abis.forEach { (abi, triple) ->
            val lib = File(prebuilt, "sysroot/usr/lib/$triple/libc++_shared.so")
            if (lib.exists()) {
                lib.copyTo(outDir.dir(abi).asFile.apply { mkdirs() }.resolve(lib.name), overwrite = true)
            }
        }
    }
}
tasks.named("preBuild") { dependsOn(copyLibcxx) }

dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = file("tauri.build.gradle.kts"))
