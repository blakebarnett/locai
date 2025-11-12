//! Example: Memory Lifecycle Tracking
//!
//! This example demonstrates Locai's memory lifecycle tracking configuration options.
//!
//! Topics covered:
//! - Configuring lifecycle tracking behavior
//! - Understanding three operating modes
//! - Use cases for lifecycle data
//! - Configuration validation

use locai::config::LifecycleTrackingConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║      Locai Memory Lifecycle Tracking Example                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // ===== Example 1: Default Configuration =====
    println!("┌─ EXAMPLE 1: Default Configuration ──────────────────────────────┐");
    let default_config = LifecycleTrackingConfig::default();
    println!("Lifecycle tracking enabled: {}", default_config.enabled);
    println!("Track memory retrieval: {}", default_config.update_on_get);
    println!(
        "Track search operations: {}",
        default_config.update_on_search
    );
    println!("Track list operations: {}", default_config.update_on_list);
    println!("Batching enabled: {}", default_config.batched);
    println!(
        "Flush interval: {} seconds",
        default_config.flush_interval_secs
    );
    println!(
        "Flush threshold: {} updates",
        default_config.flush_threshold_count
    );
    println!("└───────────────────────────────────────────────────────────────┘\n");

    // ===== Example 2: High-Performance Configuration =====
    println!("┌─ EXAMPLE 2: High-Performance Configuration ─────────────────────┐");
    let perf_config = LifecycleTrackingConfig {
        enabled: true,
        update_on_get: true,
        update_on_search: false,
        update_on_list: false,
        blocking: false,
        batched: true,
        flush_interval_secs: 120,
        flush_threshold_count: 200,
    };
    println!("✓ Optimized for throughput");
    println!(
        "  - Longer flush interval: {} seconds",
        perf_config.flush_interval_secs
    );
    println!(
        "  - Higher batch threshold: {} updates",
        perf_config.flush_threshold_count
    );
    println!("  - Async non-blocking: {}", !perf_config.blocking);
    println!("└───────────────────────────────────────────────────────────────┘\n");

    // ===== Example 3: Strict Consistency Configuration =====
    println!("┌─ EXAMPLE 3: Strict Consistency Configuration ───────────────────┐");
    let strict_config = LifecycleTrackingConfig {
        enabled: true,
        update_on_get: true,
        update_on_search: true,
        update_on_list: true,
        blocking: true,
        batched: false,
        flush_interval_secs: 60,
        flush_threshold_count: 100,
    };
    println!("✓ Ensures real-time accuracy");
    println!("  - Blocking updates: {}", strict_config.blocking);
    println!("  - No batching: {}", !strict_config.batched);
    println!(
        "  - Track all operations: get={}, search={}, list={}",
        strict_config.update_on_get, strict_config.update_on_search, strict_config.update_on_list
    );
    println!("└───────────────────────────────────────────────────────────────┘\n");

    // ===== Example 4: Configuration Validation =====
    println!("┌─ EXAMPLE 4: Configuration Validation ──────────────────────────┐");

    // Valid configuration
    let valid_config = LifecycleTrackingConfig {
        enabled: true,
        update_on_get: true,
        update_on_search: false,
        update_on_list: false,
        blocking: false,
        batched: true,
        flush_interval_secs: 60,
        flush_threshold_count: 100,
    };
    match valid_config.validate() {
        Ok(_) => println!("✓ Valid configuration passed validation"),
        Err(e) => println!("✗ Validation error: {}", e),
    }

    // Invalid configuration (zero interval)
    let invalid_config = LifecycleTrackingConfig {
        flush_interval_secs: 0,
        ..LifecycleTrackingConfig::default()
    };
    match invalid_config.validate() {
        Ok(_) => println!("✓ Invalid config passed (unexpected)"),
        Err(e) => println!("✓ Invalid config correctly rejected: {}", e),
    }

    // Invalid configuration (zero threshold)
    let invalid_config2 = LifecycleTrackingConfig {
        flush_threshold_count: 0,
        ..LifecycleTrackingConfig::default()
    };
    match invalid_config2.validate() {
        Ok(_) => println!("✓ Invalid config passed (unexpected)"),
        Err(e) => println!("✓ Invalid config correctly rejected: {}", e),
    }
    println!("└───────────────────────────────────────────────────────────────┘\n");

    // ===== Example 5: Use Cases for Lifecycle Data =====
    println!("┌─ EXAMPLE 5: Use Cases for Lifecycle Data ──────────────────────┐");
    println!("\nAccess count (update_on_get):");
    println!("  • Identify frequently-used memories");
    println!("  • Implement importance scoring");
    println!("  • Optimize memory retention");

    println!("\nLast accessed timestamp:");
    println!("  • Implement forgetting curves");
    println!("  • Identify stale memories");
    println!("  • Time-based memory decay");

    println!("\nConfiguration options:");
    println!("  • Disable tracking entirely (enabled: false)");
    println!("  • Track only get operations (other updates: false)");
    println!("  • Choose batching for performance");
    println!("  • Choose blocking for consistency");
    println!("└───────────────────────────────────────────────────────────────┘\n");

    // ===== Summary =====
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ Summary: Lifecycle Tracking Features                          ║");
    println!("╠════════════════════════════════════════════════════════════════╣");
    println!("║ ✓ Automatic metadata tracking:                                ║");
    println!("║   - access_count: incremented on each retrieval               ║");
    println!("║   - last_accessed: timestamp of most recent access            ║");
    println!("║                                                               ║");
    println!("║ ✓ Flexible configuration:                                     ║");
    println!("║   - Enable/disable globally                                   ║");
    println!("║   - Control which operations trigger tracking                 ║");
    println!("║   - Choose performance vs. consistency trade-off              ║");
    println!("║                                                               ║");
    println!("║ ✓ Three operating modes:                                      ║");
    println!("║   - Batched: Best for high-volume (default)                   ║");
    println!("║   - Async: Non-blocking background updates                    ║");
    println!("║   - Blocking: Real-time strict consistency                    ║");
    println!("║                                                               ║");
    println!("║ ✓ Use in your application:                                    ║");
    println!("║   - Calculate memory importance scores                        ║");
    println!("║   - Implement memory decay algorithms                         ║");
    println!("║   - Optimize memory management                                ║");
    println!("║   - Understand access patterns                                ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    Ok(())
}
