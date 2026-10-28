plugins {
    kotlin("jvm") version "2.4.21"
}

val repoRoot = rootDir.resolve("../..")
val bindings = layout.buildDirectory.dir("generated/uniffi")
val nativeDir = repoRoot.resolve("rust/target/debug")

// Generate the bindings from the host library, as scripts/build-native.sh does for Android.
val generateBindings by tasks.registering(Exec::class) {
    workingDir = repoRoot.resolve("rust")
    inputs.file(nativeDir.resolve("libmo_ffi.so"))
    inputs.files(repoRoot.resolve("rust/mo-ffi/uniffi.toml"), repoRoot.resolve("rust/mo-ffi/uniffi-jvm.toml"))
    outputs.dir(bindings)
    commandLine(
        "cargo", "run", "-q", "-p", "uniffi-bindgen", "--", "generate",
        "--library", "target/debug/libmo_ffi.so", "--language", "kotlin", "--no-format",
        "--config", "mo-ffi/uniffi-jvm.toml",
        "--out-dir", bindings.get().asFile.absolutePath,
    )
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

kotlin {
    compilerOptions.jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    sourceSets["main"].kotlin.srcDir(generateBindings)
}

dependencies {
    implementation("net.java.dev.jna:jna:5.19.1")
    testImplementation(kotlin("test"))
}

tasks.test {
    useJUnitPlatform()
    systemProperty("jna.library.path", nativeDir.absolutePath)
}
