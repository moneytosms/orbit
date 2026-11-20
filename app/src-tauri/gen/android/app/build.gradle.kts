import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

// Version lockstep: versionName is the semver in tauri.conf.json (scripts/release.sh bumps it);
// versionCode is derived from it (major*10000 + minor*100 + patch), so every release is newer.
val appVersion = Regex("\"version\"\\s*:\\s*\"([^\"]+)\"")
    .find(file("../../../tauri.conf.json").readText())!!.groupValues[1]
val appVersionCode = appVersion.split(".").map { it.toInt() }
    .let { (major, minor, patch) -> major * 10000 + minor * 100 + patch }

// Release signing: keystore outside the repo. Env wins over ~/.config/orbit/release.properties.
// Keys: storeFile, storePassword, keyAlias, keyPassword. Absent -> release APK is debug-signed.
val releaseProps = Properties().apply {
    val propFile = File(System.getProperty("user.home"), ".config/orbit/release.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}
fun releaseSecret(env: String, key: String): String? = System.getenv(env) ?: releaseProps.getProperty(key)
val releaseStoreFile = releaseSecret("ORBIT_KEYSTORE", "storeFile")
val releaseStorePassword = releaseSecret("ORBIT_KEYSTORE_PASSWORD", "storePassword")
val releaseKeyAlias = releaseSecret("ORBIT_KEY_ALIAS", "keyAlias")
val releaseKeyPassword = releaseSecret("ORBIT_KEY_PASSWORD", "keyPassword")
val hasReleaseKey = releaseStoreFile != null && File(releaseStoreFile).isFile &&
    releaseStorePassword != null && releaseKeyAlias != null && releaseKeyPassword != null

android {
    compileSdk = 37
    namespace = "dev.orbit.app"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "dev.orbit.app"
        minSdk = 31
        targetSdk = 37
        versionCode = appVersionCode
        versionName = appVersion
    }
    signingConfigs {
        if (hasReleaseKey) {
            create("release") {
                storeFile = File(releaseStoreFile!!)
                storePassword = releaseStorePassword
                keyAlias = releaseKeyAlias
                keyPassword = releaseKeyPassword
            }
        }
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
            signingConfig = signingConfigs.getByName(if (hasReleaseKey) "release" else "debug")
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

dependencies {
    implementation("androidx.work:work-runtime-ktx:2.10.1")
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
