use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Always declare the cfg to silence unexpected_cfgs warnings under -D warnings.
    println!("cargo:rustc-check-cfg=cfg(oxinum_simd)");

    let simd_requested = env::var("CARGO_FEATURE_SIMD").is_ok();
    if !simd_requested {
        return;
    }

    let rustc = env::var("RUSTC").unwrap_or_else(|_| String::from("rustc"));
    let is_nightly = Command::new(rustc)
        .arg("-vV")
        .output()
        .map(|out| {
            let version = String::from_utf8_lossy(&out.stdout);
            version.contains("nightly") || version.contains("-dev")
        })
        .unwrap_or(false);

    if is_nightly {
        println!("cargo:rustc-cfg=oxinum_simd");
    }
}
