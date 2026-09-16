//! Derives the string reported by `ostt --version`.
//!
//! GPU builds carry a suffix so a user can tell which build is running. The
//! suffix used to be keyed on the Cargo feature alone, but the CUDA 12 and
//! CUDA 13 builds share the `whisper-cuda` feature -- they differ only in the
//! toolkit present at build time -- so the release workflow names the variant
//! explicitly through `OSTT_BUILD_VARIANT`. The feature remains the fallback
//! for local builds that do not set it.

use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=OSTT_BUILD_VARIANT");

    let variant = env::var("OSTT_BUILD_VARIANT")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            if env::var_os("CARGO_FEATURE_WHISPER_CUDA").is_some() {
                "cuda".to_owned()
            } else if env::var_os("CARGO_FEATURE_WHISPER_VULKAN").is_some() {
                "vulkan".to_owned()
            } else {
                String::new()
            }
        });

    let version = env::var("CARGO_PKG_VERSION").expect("cargo sets CARGO_PKG_VERSION");
    let reported = if variant.is_empty() {
        version
    } else {
        format!("{version}-{variant}")
    };
    println!("cargo:rustc-env=OSTT_VERSION={reported}");
}
