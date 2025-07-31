//! Build script for locai-server with platform-specific optimizations

use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    
    println!("cargo:rerun-if-changed=build.rs");
    
    // Print build information
    println!("cargo:warning=Building locai-server for {} {}", target_os, target_arch);
    
    // Platform-specific server optimizations
    match (&target_os[..], &target_arch[..]) {
        ("linux", "x86_64") => {
            // Try to use LLD linker for faster linking
            if which::which("ld.lld").is_ok() {
                println!("cargo:rustc-link-arg=-fuse-ld=lld");
            }
        }
        
        ("linux", "aarch64") => {
            println!("cargo:warning=ARM64 Linux server build");
            if which::which("ld.lld").is_ok() {
                println!("cargo:rustc-link-arg=-fuse-ld=lld");
            }
        }
        
        ("macos", "aarch64") => {
            println!("cargo:warning=Apple Silicon server build");
        }
        
        ("macos", "x86_64") => {
            println!("cargo:warning=Intel Mac server build");
        }
        
        _ => {
            println!("cargo:warning=Server build for {}/{}", target_os, target_arch);
        }
    }
}