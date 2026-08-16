fn main() {
    uniffi::uniffi_bindgen_main();
}

// Usage, once the crate is built for the host (or run through cargo-ndk):
//
//   cargo run --bin uniffi-bindgen -- generate \
//       --library target/debug/libisideload_android.so \
//       --language kotlin \
//       --out-dir ../android/app/src/main/java/generated
//
// This produces the Kotlin binding file(s) that wrap AuthSession below.
