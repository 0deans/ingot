import java.io.File
import java.util.Properties

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

data class ReleaseSigningInput(
    val storeFile: File,
    val storePassword: String,
    val keyAlias: String,
    val keyPassword: String,
)

val releaseSigning: ReleaseSigningInput? = run {
    val envPath = System.getenv("ANDROID_KEYSTORE_PATH")?.takeIf { it.isNotBlank() }
    val envStorePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")?.takeIf { it.isNotBlank() }
    if (envPath != null && envStorePassword != null) {
        val envAlias = System.getenv("ANDROID_KEY_ALIAS")?.takeIf { it.isNotBlank() } ?: "ingot"
        val envKeyPassword =
            System.getenv("ANDROID_KEY_PASSWORD")?.takeIf { it.isNotBlank() } ?: envStorePassword
        return@run ReleaseSigningInput(
            storeFile = file(envPath),
            storePassword = envStorePassword,
            keyAlias = envAlias,
            keyPassword = envKeyPassword,
        )
    }

    val propFile = rootProject.file("keystore.properties")
    if (!propFile.exists()) {
        return@run null
    }
    val props = Properties().apply { propFile.inputStream().use { load(it) } }
    val propPath = props.getProperty("storeFile")?.takeIf { it.isNotBlank() } ?: return@run null
    val propStorePassword =
        props.getProperty("storePassword")?.takeIf { it.isNotBlank() } ?: return@run null
    val propAlias = props.getProperty("keyAlias")?.takeIf { it.isNotBlank() } ?: "ingot"
    val propKeyPassword =
        props.getProperty("keyPassword")?.takeIf { it.isNotBlank() } ?: propStorePassword
    ReleaseSigningInput(
        storeFile = rootProject.file(propPath),
        storePassword = propStorePassword,
        keyAlias = propAlias,
        keyPassword = propKeyPassword,
    )
}

android {
    compileSdk = 36
    namespace = "me.odean.ingot"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "me.odean.ingot"
        minSdk = 24
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    signingConfigs {
        releaseSigning?.let { signing ->
            create("release") {
                storeFile = signing.storeFile
                storePassword = signing.storePassword
                keyAlias = signing.keyAlias
                keyPassword = signing.keyPassword
            }
        }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            if (releaseSigning != null) {
                signingConfig = signingConfigs.getByName("release")
            }
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        buildConfig = true
    }
}

rust {
    rootDirRel = "../../../"
}

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

apply(from = "tauri.build.gradle.kts")