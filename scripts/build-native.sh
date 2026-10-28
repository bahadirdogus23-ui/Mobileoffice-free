#!/usr/bin/env bash
# Builds libmo_ffi.so for Android and generates its Kotlin bindings into the :engine module.
#
#   scripts/build-native.sh [--debug] [ABI ...]     (default ABIs: arm64-v8a armeabi-v7a x86_64)
#
# Needs: Rust (rustup targets for the ABIs), cargo-ndk, and an Android NDK (ANDROID_NDK_HOME,
# or the newest one under $ANDROID_HOME/ndk).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
profile=release
if [[ "${1:-}" == "--debug" ]]; then profile=debug; shift; fi
abis=("$@")
[[ ${#abis[@]} -eq 0 ]] && abis=(arm64-v8a armeabi-v7a x86_64)

engine="$root/android/engine/src/main"
min_sdk=26

if [[ -z "${ANDROID_NDK_HOME:-}" && -n "${ANDROID_HOME:-}" && -d "$ANDROID_HOME/ndk" ]]; then
  ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$(ls "$ANDROID_HOME/ndk" | sort -V | tail -1)"
  export ANDROID_NDK_HOME
fi

cd "$root/rust"
targs=()
for abi in "${abis[@]}"; do targs+=(-t "$abi"); done
release_flag=()
[[ $profile == release ]] && release_flag=(--release)

echo "==> libmo_ffi.so ($profile) for ${abis[*]}"
rm -rf "$engine/jniLibs"
cargo ndk "${targs[@]}" --platform "$min_sdk" -o "$engine/jniLibs" build -p mo-ffi "${release_flag[@]}"

echo "==> Kotlin bindings"
cargo build -p mo-ffi
host_lib="target/debug/libmo_ffi.so"
[[ -f $host_lib ]] || host_lib="target/debug/libmo_ffi.dylib"
rm -rf "$engine/java/uniffi" "$engine/java/org/mobileofficefree/engine/mo_ffi.kt"
cargo run -q -p uniffi-bindgen -- generate --library "$host_lib" --language kotlin --no-format --out-dir "$engine/java"

python3 "$root/scripts/check_elf_alignment.py" "$engine/jniLibs"
