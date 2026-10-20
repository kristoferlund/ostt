//! Derives the string reported by `ostt --version`.
//!
//! GPU builds carry a suffix so a user can tell which build is running. The
//! suffix used to be keyed on the Cargo feature alone, but the CUDA 12 and
//! CUDA 13 builds share the `cuda` feature -- they differ only in the
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
            if env::var_os("CARGO_FEATURE_CUDA").is_some() {
                "cuda".to_owned()
            } else if env::var_os("CARGO_FEATURE_VULKAN").is_some() {
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

    if env::var_os("CARGO_FEATURE_CUDA").is_some() {
        link_cuda_libraries();
    }
}

/// transcribe-cpp-sys 0.4.0 links the static ggml-cuda archive but, unlike its
/// Vulkan and ROCm backends, does not list the CUDA libraries that archive
/// needs, so the final link fails with undefined cuda*/cublas*/cu* symbols.
fn link_cuda_libraries() {
    println!("cargo:rerun-if-env-changed=CUDA_HOME");
    println!("cargo:rerun-if-env-changed=CUDA_PATH");

    let root = env::var("CUDA_HOME")
        .or_else(|_| env::var("CUDA_PATH"))
        .unwrap_or_else(|_| {
            ["/usr/local/cuda", "/opt/cuda"]
                .into_iter()
                .find(|p| std::path::Path::new(p).is_dir())
                .unwrap_or("/usr/local/cuda")
                .to_owned()
        });
    // stubs/ provides libcuda.so for build machines without an NVIDIA driver;
    // the binary still records libcuda.so.1, which the driver supplies at runtime.
    for dir in ["lib64", "lib", "lib64/stubs", "lib/stubs"] {
        println!("cargo:rustc-link-search=native={root}/{dir}");
    }
    for lib in ["cudart", "cublas", "cuda"] {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
}
