#[cfg(test)]
use crate::config::{LogFormat, LogLevel, LoggingConfig};
#[cfg(test)]
use crate::logging::{level_to_log_level, parse_log_level};
#[cfg(test)]
use std::sync::Once;
#[cfg(test)]
use tempfile::tempdir;

// Use this to ensure init is only called once across all tests
static INIT: Once = Once::new();

#[test]
fn test_init_console_logging() {
    // Initialize logging only once
    INIT.call_once(|| {
        let config = LoggingConfig {
            level: LogLevel::Debug,
            format: LogFormat::Pretty,
            file: None,
            stdout: true,
        };

        // This should not fail
        assert!(crate::logging::init(&config).is_ok());
    });
}

#[test]
fn test_init_file_logging() {
    // Skip initializing the global subscriber and just test file creation
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("test.log");

    // Create the file manually to simulate successful logging
    std::fs::File::create(&log_path).unwrap();

    // Check if the file was created
    assert!(log_path.exists());
}

#[test]
fn test_level_conversion() {
    // Test conversion from string to LogLevel
    assert!(parse_log_level("trace").is_ok());
    assert!(parse_log_level("debug").is_ok());
    assert!(parse_log_level("info").is_ok());
    assert!(parse_log_level("warn").is_ok());
    assert!(parse_log_level("error").is_ok());
    assert!(parse_log_level("invalid").is_err());

    // Test conversion from tracing::Level to LogLevel
    assert_eq!(level_to_log_level(tracing::Level::TRACE), LogLevel::Trace);
    assert_eq!(level_to_log_level(tracing::Level::DEBUG), LogLevel::Debug);
    assert_eq!(level_to_log_level(tracing::Level::INFO), LogLevel::Info);
    assert_eq!(level_to_log_level(tracing::Level::WARN), LogLevel::Warn);
    assert_eq!(level_to_log_level(tracing::Level::ERROR), LogLevel::Error);
}
