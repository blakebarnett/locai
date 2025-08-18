//! Build script for dynamic platform-specific optimizations
//!
//! This script detects the target platform and applies appropriate
//! optimizations for RocksDB and other native dependencies.

use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=TARGET");

    // Print build information for debugging
    println!("cargo:warning=Building for target: {}", target);
    println!(
        "cargo:warning=Target OS: {}, Architecture: {}",
        target_os, target_arch
    );

    // Configure RocksDB based on platform
    configure_rocksdb(&target, &target_os, &target_arch);

    // Configure other platform-specific optimizations
    configure_platform_optimizations(&target, &target_os, &target_arch);

    // Set feature flags based on platform capabilities
    set_platform_features(&target_os, &target_arch);
}

fn configure_rocksdb(_target: &str, target_os: &str, target_arch: &str) {
    match (target_os, target_arch) {
        ("linux", "x86_64") => {
            // x86_64 Linux: Use system RocksDB if available
            if let Ok(lib_dir) = env::var("ROCKSDB_LIB_DIR") {
                println!("cargo:rustc-link-search=native={}", lib_dir);
            } else {
                // Try common locations
                for path in &["/usr/lib/x86_64-linux-gnu", "/usr/local/lib", "/usr/lib64"] {
                    if std::path::Path::new(path).exists() {
                        println!("cargo:rustc-env=ROCKSDB_LIB_DIR={}", path);
                        break;
                    }
                }
            }

            // Use LLD linker for faster linking on x86_64 Linux if available
            if which::which("ld.lld").is_ok() {
                println!("cargo:rustc-link-arg=-fuse-ld=lld");
                println!("cargo:warning=Using LLD linker for faster builds");
            } else {
                println!("cargo:warning=LLD not available, using default linker");
            }
        }

        ("linux", "aarch64") => {
            // ARM64 Linux: Usually auto-detects correctly
            println!("cargo:warning=ARM64 Linux detected - using default RocksDB configuration");

            // Try to use LLD if available for ARM64 Linux
            if which::which("ld.lld").is_ok() {
                println!("cargo:rustc-link-arg=-fuse-ld=lld");
                println!("cargo:warning=Using LLD linker for ARM64 builds");
            } else {
                println!("cargo:warning=LLD not available, using default linker");
            }
        }

        ("macos", _) => {
            // macOS: Use system libraries (both x86_64 and aarch64)
            println!("cargo:warning=macOS detected - using system RocksDB configuration");

            // macOS-specific optimizations
            if target_arch == "aarch64" {
                println!("cargo:warning=Apple Silicon detected - native ARM64 build");
            }

            // Use system library paths
            println!("cargo:rustc-link-search=native=/usr/local/lib");
            println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
        }

        ("windows", _) => {
            // Windows: Usually handled by vcpkg or bundled libraries
            println!("cargo:warning=Windows detected - using default RocksDB configuration");
        }

        _ => {
            println!(
                "cargo:warning=Unknown platform {}/{} - using default configuration",
                target_os, target_arch
            );
        }
    }
}

fn configure_platform_optimizations(_target: &str, target_os: &str, target_arch: &str) {
    match target_os {
        "linux" => {
            // Linux-specific optimizations
            match target_arch {
                "x86_64" => {
                    println!("cargo:rustc-cfg=feature=\"optimized_x86_64\"");
                }
                "aarch64" => {
                    println!("cargo:rustc-cfg=feature=\"optimized_aarch64\"");
                }
                _ => {}
            }
        }

        "macos" => {
            // macOS-specific optimizations
            println!("cargo:rustc-cfg=feature=\"macos_optimized\"");

            if target_arch == "aarch64" {
                println!("cargo:rustc-cfg=feature=\"apple_silicon\"");
            }
        }

        "windows" => {
            // Windows-specific optimizations
            println!("cargo:rustc-cfg=feature=\"windows_optimized\"");
        }

        _ => {}
    }
}

fn set_platform_features(target_os: &str, target_arch: &str) {
    // Enable SIMD optimizations where appropriate
    match target_arch {
        "x86_64" => {
            println!("cargo:rustc-cfg=feature=\"simd_x86_64\"");
        }
        "aarch64" => {
            println!("cargo:rustc-cfg=feature=\"simd_aarch64\"");
        }
        _ => {}
    }

    // Set custom feature flags (not built-in ones)
    match (target_os, target_arch) {
        ("linux", "x86_64") => {
            println!("cargo:rustc-cfg=feature=\"linux_x86_64\"");
        }
        ("linux", "aarch64") => {
            println!("cargo:rustc-cfg=feature=\"linux_aarch64\"");
        }
        ("macos", "aarch64") => {
            println!("cargo:rustc-cfg=feature=\"macos_arm64\"");
        }
        ("macos", "x86_64") => {
            println!("cargo:rustc-cfg=feature=\"macos_x86_64\"");
        }
        _ => {}
    }
}
