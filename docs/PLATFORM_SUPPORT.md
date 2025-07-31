# Platform Support and Dynamic Optimizations

This document describes how Locai handles platform-specific optimizations and cross-platform builds.

## Overview

Locai now uses dynamic platform detection to automatically apply the best optimizations for your target platform. This ensures optimal performance while maintaining compatibility across all supported architectures.

## Supported Platforms

### Officially Supported

| Platform | Architecture | Status | Notes |
|----------|-------------|--------|-------|
| Linux | x86_64 | ✅ Full Support | Optimized with LLD linker |
| Linux | aarch64 (ARM64) | ✅ Full Support | Native ARM64 optimizations |
| macOS | x86_64 (Intel) | ✅ Full Support | Intel Mac compatibility |
| macOS | aarch64 (Apple Silicon) | ✅ Full Support | Native M1/M2 optimizations |
| Windows | x86_64 | ✅ Full Support | MSVC toolchain |
| Windows | aarch64 | 🧪 Experimental | Windows ARM64 |

### Storage Engine Support

| Engine | x86_64 | aarch64 | Notes |
|--------|--------|---------|-------|
| RocksDB | ✅ | ✅ | Native builds on all platforms |
| SurrealDB Memory | ✅ | ✅ | Pure Rust, no native deps |
| SurrealDB Embedded | ✅ | ✅ | Uses RocksDB backend |

## How Dynamic Optimization Works

### 1. Cargo Configuration (`.cargo/config.toml`)

The project uses target-specific Cargo configuration sections that automatically apply based on your compilation target:

```toml
# Applied only when building for x86_64 Linux
[target.x86_64-unknown-linux-gnu.env]
ROCKSDB_LIB_DIR = "/usr/lib/x86_64-linux-gnu"

# Applied only when building for ARM64 macOS  
[target.aarch64-apple-darwin.env]
# macOS handles library paths automatically
```

### 2. Build Scripts (`build.rs`)

Each crate with native dependencies includes a build script that:

- Detects the target platform at compile time
- Configures RocksDB library paths dynamically
- Applies platform-specific linker optimizations
- Sets conditional compilation flags

Example detection logic:
```rust
let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

match (target_os, target_arch) {
    ("linux", "x86_64") => configure_x86_64_linux(),
    ("macos", "aarch64") => configure_apple_silicon(),
    // ... other platforms
}
```

### 3. Conditional Features

Platform-specific features are enabled automatically:

```rust
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn optimized_x86_64_path() {
    // x86_64-specific optimizations
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]  
fn apple_silicon_path() {
    // Apple Silicon optimizations
}
```

## Building for Different Platforms

### Native Builds

Simply run `cargo build` - optimizations are applied automatically:

```bash
# On Apple Silicon Mac
cargo build  # Automatically uses ARM64 optimizations

# On x86_64 Linux  
cargo build  # Automatically uses x86_64 optimizations
```

### Cross-Compilation

Install the target and build:

```bash
# ARM64 Linux from x86_64
rustup target add aarch64-unknown-linux-gnu
cargo build --target aarch64-unknown-linux-gnu

# Intel Mac from ARM64 Mac
rustup target add x86_64-apple-darwin  
cargo build --target x86_64-apple-darwin
```

### Docker Builds

Use the provided multi-arch Dockerfile:

```bash
# Build for current platform
docker build -t locai .

# Build for specific platform
docker build --platform linux/amd64 -t locai:amd64 .
docker build --platform linux/arm64 -t locai:arm64 .
```

## Platform-Specific Optimizations

### Linux x86_64
- Uses LLD linker for faster builds
- System RocksDB libraries when available
- AVX2 optimizations where supported

### Linux ARM64
- Native ARM64 RocksDB builds
- NEON SIMD optimizations
- LLD linker when available

### macOS (Both Intel and Apple Silicon)
- Uses system library paths (`/usr/local/lib`, `/opt/homebrew/lib`)
- Automatic framework linking
- Metal acceleration for AI workloads (when enabled)

### Windows
- MSVC toolchain optimizations
- vcpkg integration for dependencies
- Windows-specific async I/O

## Troubleshooting

### Build Fails on Mac ARM64

**Problem**: Getting x86_64 architecture errors  
**Solution**: The new configuration should fix this automatically. If issues persist:

```bash
# Clean and rebuild
cargo clean
cargo build
```

### RocksDB Library Not Found

**Problem**: `cannot find -lrocksdb`  
**Solution**: Install RocksDB for your platform:

```bash
# Ubuntu/Debian
sudo apt-get install librocksdb-dev

# macOS with Homebrew  
brew install rocksdb

# Or let the build script compile from source (slower but guaranteed to work)
cargo build --features rocksdb/static
```

### Slow Builds

**Problem**: Compilation takes too long  
**Solutions**:
- Use LLD linker (automatically enabled on supported platforms)
- Enable parallel compilation: `export CARGO_BUILD_JOBS=8`
- Use `sccache` for caching: `cargo install sccache && export RUSTC_WRAPPER=sccache`

### Cross-Compilation Issues

**Problem**: Can't cross-compile to target platform  
**Solution**: Install required linkers and libraries:

```bash
# For ARM64 Linux targets on x86_64
sudo apt-get install gcc-aarch64-linux-gnu

# Set the linker in ~/.cargo/config.toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

## Development Workflow

### Testing on Multiple Platforms

Use the provided scripts:

```bash
# Test current platform
cargo test

# Test specific platform (if cross-compilation is set up)
cargo test --target aarch64-unknown-linux-gnu

# Test in Docker for Linux platforms
docker run --rm -v $(pwd):/workspace -w /workspace rust:1.75 cargo test
```

### Adding New Platform Support

1. Add target-specific configuration to `.cargo/config.toml`
2. Update `build.rs` scripts with platform detection
3. Add platform to the support matrix in this document
4. Test the build on the target platform

### Debugging Build Issues

Enable verbose build output:

```bash
# See what the build script is doing
cargo build -vv

# Check what features are enabled
cargo build --message-format=json | jq '.features'

# See the exact commands being run
CARGO_LOG=cargo::core::compiler=debug cargo build
```

## Contributing

When adding platform-specific code:

1. Use conditional compilation (`#[cfg(...)]`) rather than runtime checks
2. Update both the Cargo configuration and build scripts
3. Add tests for the new platform if possible
4. Update this documentation

## See Also

- [Cargo Target Configuration](https://doc.rust-lang.org/cargo/reference/config.html#target)
- [Rust Cross-Compilation](https://rust-lang.github.io/rustup/cross-compilation.html)
- [RocksDB Installation Guide](https://github.com/facebook/rocksdb/blob/main/INSTALL.md)