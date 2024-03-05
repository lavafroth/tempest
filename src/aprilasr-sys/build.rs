use cmake;
use std::{env, path::PathBuf};

/// Use p! to debug as println! is reserved in build files.
#[allow(unused_macros)]
macro_rules! p {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

/// Builds April ASR speech-to-text library in C using vendored
/// CMake file specifically designed for the task. With a known
/// working CMake build of libaprilasr the script will generate
/// low-level bindings for April ASR following Rust *-sys crate
/// conventions in order to decouple higher-level wrappers.
fn main() {
    // Allow users to set include path as a convenience.
    if let Ok(include_dir) = env::var("APRIL_INCLUDE_DIR") {
        println!("cargo:include={}", include_dir);
    }

    // Only re-build April ASR if the API changes.
    println!("cargo:rerun-if-changed=vendor/april-asr/april_api.h");
    println!("cargo:rerun-if-changed=wrapper.h");

    // Build April ASR from source using CMake.
    let dst = cmake::Config::new("vendor/april-asr").build().join("build");

    // Update linker search paths for local builds.
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=aprilasr");

    // Configure and generate April ASR bindgen bindings.
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Couldn't generate bindings!");

    // Write bindings to out dir.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!")
}
