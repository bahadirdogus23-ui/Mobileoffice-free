# Rules for contributors and coding agents

The plan is [docs/RAPOR-VE-PLAN.md](docs/RAPOR-VE-PLAN.md); decisions are recorded in [docs/adr/](docs/adr/).
User-facing text is Turkish first, English second; code, comments and commit messages are English.

## Layout

| Path | What |
|---|---|
| `rust/` | Cargo workspace. `mo-<format>` crates wrap one ArtCraft engine each; `mo-ffi` is the only `cdylib` and the only crate that knows about UniFFI |
| `android/` | Gradle project: `:app` (Compose UI) and `:engine` (generated Kotlin bindings + `.so`) |
| `scripts/build-native.sh` | Builds `libmo_ffi.so` for Android ABIs and generates the bindings; run before Gradle |
| `tools/jvm-smoke/` | Runs the generated Kotlin bindings against the host library on a JVM |

## Invariants

1. **No internet permission, no analytics, no Google Play services.** The manifest declares no permissions.
2. **Engines are reached only through `mo-*` bridges, and edits only through engine commands** (`execute(id, json)`).
   Do not reimplement in Kotlin what an engine command already does.
3. **Engine calls run on one thread per document** (see `SheetViewModel.engine`). Never call an engine from the UI thread.
4. **Never lose user data.** Opening never writes; saving writes the whole file with `"wt"`; anything the engine reports
   it could not keep is shown to the person.
5. **Upstream engines are pinned by commit** in `rust/Cargo.toml`. Bump them deliberately, in their own PR, with tests.
   Do not use ArtCraft names or logos anywhere in the app.
6. **Case conversions for identifiers use `Locale.ROOT`** (Turkish `i`/`İ`), never the default locale.

## Before pushing

```sh
cd rust && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cargo build -p mo-ffi && ../tools/jvm-smoke/gradlew -p ../tools/jvm-smoke test
```

The Android build needs the Android SDK and NDK (see README); CI builds the APK on every push.
