//! Integration tests for memory lifecycle tracking
//!
//! Tests the lifecycle tracking configuration and behavior integration.

use locai::config::LifecycleTrackingConfig;

#[test]
fn test_lifecycle_config_default() {
    let config = LifecycleTrackingConfig::default();

    assert!(config.enabled);
    assert!(config.update_on_get);
    assert!(!config.update_on_search);
    assert!(!config.update_on_list);
    assert!(!config.blocking);
    assert!(config.batched);
    assert_eq!(config.flush_interval_secs, 60);
    assert_eq!(config.flush_threshold_count, 100);
}

#[test]
fn test_lifecycle_config_validation_valid() {
    let config = LifecycleTrackingConfig {
        enabled: true,
        update_on_get: true,
        update_on_search: false,
        update_on_list: false,
        blocking: false,
        batched: true,
        flush_interval_secs: 60,
        flush_threshold_count: 100,
    };

    assert!(config.validate().is_ok());
}

#[test]
fn test_lifecycle_config_validation_zero_interval() {
    let config = LifecycleTrackingConfig {
        flush_interval_secs: 0,
        ..LifecycleTrackingConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_lifecycle_config_validation_zero_threshold() {
    let config = LifecycleTrackingConfig {
        flush_threshold_count: 0,
        ..LifecycleTrackingConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_lifecycle_config_disabled() {
    let config = LifecycleTrackingConfig {
        enabled: false,
        ..LifecycleTrackingConfig::default()
    };

    assert!(!config.enabled);
    // Should still validate even when disabled
    assert!(config.validate().is_ok());
}

#[test]
fn test_lifecycle_config_blocking_mode() {
    let config = LifecycleTrackingConfig {
        enabled: true,
        blocking: true,
        batched: false,
        ..LifecycleTrackingConfig::default()
    };

    assert!(config.enabled);
    assert!(config.blocking);
    assert!(!config.batched);
}

#[test]
fn test_lifecycle_config_batched_mode() {
    let config = LifecycleTrackingConfig {
        enabled: true,
        update_on_get: true,
        update_on_search: false,
        update_on_list: false,
        batched: true,
        blocking: false,
        flush_interval_secs: 120,
        flush_threshold_count: 200,
    };

    assert!(config.enabled);
    assert!(config.batched);
    assert!(!config.blocking);
    assert_eq!(config.flush_interval_secs, 120);
    assert_eq!(config.flush_threshold_count, 200);
}

#[test]
fn test_lifecycle_config_async_mode() {
    let config = LifecycleTrackingConfig {
        enabled: true,
        update_on_get: true,
        update_on_search: false,
        update_on_list: false,
        batched: false,
        blocking: false,
        ..LifecycleTrackingConfig::default()
    };

    assert!(config.enabled);
    assert!(!config.batched);
    assert!(!config.blocking);
}

#[test]
fn test_lifecycle_config_track_all_operations() {
    let config = LifecycleTrackingConfig {
        enabled: true,
        update_on_get: true,
        update_on_search: true,
        update_on_list: true,
        ..LifecycleTrackingConfig::default()
    };

    assert!(config.update_on_get);
    assert!(config.update_on_search);
    assert!(config.update_on_list);
}

#[test]
fn test_lifecycle_config_track_only_get() {
    let config = LifecycleTrackingConfig {
        enabled: true,
        update_on_get: true,
        update_on_search: false,
        update_on_list: false,
        ..LifecycleTrackingConfig::default()
    };

    assert!(config.update_on_get);
    assert!(!config.update_on_search);
    assert!(!config.update_on_list);
}
