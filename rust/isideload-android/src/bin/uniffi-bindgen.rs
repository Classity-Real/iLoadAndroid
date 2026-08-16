fn main() {
    // UniFFI 0.27+ exposes the CLI entrypoint as `uniffi::cli::main()`.
    // The older `uniffi_bindgen_main()` symbol no longer exists.
    uniffi::cli::main();
}

// Usage, once the crate is built for the host (or run through cargo-ndk):
//
//   cargo run --bin uniffi-bindgen -- generate \
//       --library target/debug/libisideload_android.so \
//       --language kotlin \
//       --out-dir ../android/app/src/main/java/generated
//
// This produces the Kotlin binding file(s) that wrap AuthSession below.
