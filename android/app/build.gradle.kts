plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "org.mobileofficefree.app"
    compileSdk = 37

    defaultConfig {
        // Working id; it can still change before the first store release (docs/RAPOR-VE-PLAN.md §16).
        applicationId = "org.mobileofficefree.app"
        minSdk = 26
        targetSdk = 36
        // CI builds count up, so each test build installs over the previous one.
        val build = providers.environmentVariable("GITHUB_RUN_NUMBER").orNull?.toIntOrNull()
        versionCode = build ?: 1
        versionName = if (build != null) "0.1.0-dev.$build" else "0.1.0"
    }

    signingConfigs {
        // A fixed debug key in the repository, so test builds from any machine update each other.
        // It signs nothing but debug builds; store releases use a private key kept out of the repo.
        getByName("debug") {
            storeFile = file("debug.keystore")
            storePassword = "android"
            keyAlias = "androiddebugkey"
            keyPassword = "android"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        compose = true
    }
    packaging {
        // Keep .so files uncompressed and page-aligned so they load straight from the APK.
        jniLibs.useLegacyPackaging = false
    }
}

dependencies {
    implementation(project(":engine"))
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.kotlinx.coroutines.android)
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.ui)
    implementation(libs.compose.material3)
    implementation(libs.compose.ui.tooling.preview)
    debugImplementation(libs.compose.ui.tooling)
    testImplementation(libs.junit)
}
