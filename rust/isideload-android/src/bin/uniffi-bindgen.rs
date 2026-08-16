fn main() {
    uniffi::uniffi_bindgen_main();
}

// Usage, once the crate is built (or run through cargo-ndk):
//
//   cargo run --bin uniffi-bindgen -- generate \
//       --library target/debug/libisideload_android.so \
//       --language kotlin \
//       --out-dir ../android/app/src/main/java/generated
//
// Requires the uniffi "cli" feature (see Cargo.toml) -- without it this
// binary fails to compile with "cannot find `cli` in `uniffi`" / "cannot
// find function `uniffi_bindgen_main`", since both are feature-gated.
