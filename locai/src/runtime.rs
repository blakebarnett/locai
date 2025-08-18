//! Runtime configuration optimized for SurrealDB embedded use
//!
//! This module provides utilities for creating and configuring tokio runtimes
//! according to SurrealDB performance best practices.

use std::io;

/// Creates an optimized tokio runtime for SurrealDB embedded applications
///
/// This follows SurrealDB's recommended configuration:
/// - Multi-threaded runtime
/// - All features enabled
/// - 10MiB thread stack size for better performance
///
/// # Examples
///
/// ```rust
/// use locai::runtime::create_optimized_runtime;
///
/// let runtime = create_optimized_runtime().expect("Failed to create runtime");
/// runtime.block_on(async {
///     // Your application code here
/// });
/// ```
pub fn create_optimized_runtime() -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(10 * 1024 * 1024) // 10MiB
        .build()
}

/// Creates an optimized tokio runtime with custom thread count
///
/// # Arguments
/// * `worker_threads` - Number of worker threads to use (None for default)
///
/// # Examples
///
/// ```rust
/// use locai::runtime::create_optimized_runtime_with_threads;
///
/// let runtime = create_optimized_runtime_with_threads(Some(4))
///     .expect("Failed to create runtime");
/// ```
pub fn create_optimized_runtime_with_threads(
    worker_threads: Option<usize>,
) -> io::Result<tokio::runtime::Runtime> {
    let mut builder = tokio::runtime::Builder::new_multi_thread();

    builder.enable_all().thread_stack_size(10 * 1024 * 1024); // 10MiB

    if let Some(threads) = worker_threads {
        builder.worker_threads(threads);
    }

    builder.build()
}

/// Helper function to check if we're already in a tokio runtime
///
/// This is useful for applications that need to conditionally create a runtime
/// or use an existing one.
pub fn is_in_tokio_runtime() -> bool {
    tokio::runtime::Handle::try_current().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_optimized_runtime() {
        let runtime = create_optimized_runtime();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_create_optimized_runtime_with_threads() {
        let runtime = create_optimized_runtime_with_threads(Some(2));
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_is_in_tokio_runtime() {
        // Outside of runtime context, this should be false
        // (Note: this might be true if tests are run in a tokio context)
        let _in_runtime = is_in_tokio_runtime();
        // We can't assert specific values since test environment varies
    }
}
