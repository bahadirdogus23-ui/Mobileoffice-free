// Runs the UniFFI-generated Kotlin bindings against the host build of libmo_ffi on a plain JVM,
// so the Rust ⇄ Kotlin boundary is tested without an Android device or SDK.
//   (cd rust && cargo build -p mo-ffi) && tools/jvm-smoke/gradlew -p tools/jvm-smoke test
pluginManagement {
    repositories { gradlePluginPortal(); mavenCentral() }
}
dependencyResolutionManagement {
    repositories { mavenCentral() }
}
rootProject.name = "jvm-smoke"
