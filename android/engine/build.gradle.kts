// The Rust engines (libmo_ffi.so) and their UniFFI-generated Kotlin API.
// Both are build outputs of scripts/build-native.sh; run it before building this module.
plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "org.mobileofficefree.engine"
    compileSdk = 37
    defaultConfig {
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // UniFFI's Kotlin bindings call native code through JNA.
    implementation(libs.jna) { artifact { type = "aar" } }
}

val nativeOutputs = listOf("src/main/jniLibs", "src/main/java/org/mobileofficefree/engine/mo_ffi.kt")
tasks.named("preBuild") {
    val missing = nativeOutputs.map { layout.projectDirectory.file(it).asFile }.filterNot { it.exists() }
    doFirst {
        check(missing.isEmpty()) {
            "Native engine outputs are missing: ${missing.joinToString()}.\nRun scripts/build-native.sh first."
        }
    }
}
