//! Structured logging infrastructure for Locai.
//!
//! This module provides a configurable logging system based on the tracing crate,
//! supporting different output formats, log levels, and integration with monitoring systems.

mod filters;
mod formatters;
mod middleware;
#[cfg(test)]
mod tests;

use crate::config::{LogFormat, LogLevel, LoggingConfig};
use std::path::Path;
use tracing::Level;
use tracing_appender::non_blocking::NonBlocking;

/// Error type for logging operations
#[derive(Debug)]
pub enum LogError {
    /// IO error occurred
    IoError(std::io::Error),

    /// Error parsing log level
    InvalidLogLevel(String),

    /// Error in subscriber setup
    SubscriberError(Box<dyn std::error::Error + Send + Sync>),

    /// General error
    Other(String),
}

impl From<std::io::Error> for LogError {
    fn from(err: std::io::Error) -> Self {
        LogError::IoError(err)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for LogError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        LogError::SubscriberError(err)
    }
}

/// Result type for logging operations
pub type Result<T> = std::result::Result<T, LogError>;

/// Initialize the logging system with the given configuration.
pub fn init(config: &LoggingConfig) -> Result<()> {
    // Convert LogLevel to tracing::Level
    let level = match config.level {
        LogLevel::Trace => Level::TRACE,
        LogLevel::Debug => Level::DEBUG,
        LogLevel::Info => Level::INFO,
        LogLevel::Warn => Level::WARN,
        LogLevel::Error => Level::ERROR,
    };

    // Create different types of subscribers based on format
    let result = match config.format {
        LogFormat::Json => init_json_logging(level, config),
        LogFormat::Compact => init_compact_logging(level, config),
        _ => init_pretty_logging(level, config),
    };

    // If the error is "already set", ignore it
    if let Err(LogError::SubscriberError(ref e)) = result
        && e.to_string().contains("SetGlobalDefaultError")
    {
        return Ok(());
    }

    result
}

/// Initialize logging with JSON formatting
fn init_json_logging(level: Level, config: &LoggingConfig) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_max_level(level)
        .with_level(true)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true);

    if let Some(file_path) = &config.file {
        let (writer, _guard) = create_non_blocking_file(file_path)?;

        if config.stdout {
            subscriber.with_writer(std::io::stdout).try_init()?;
            // Note: we can't easily log to both stdout and file with simple setup
            tracing::warn!("Configured for stdout only; file logging ignored");
        } else {
            subscriber.with_writer(writer).try_init()?;
        }
    } else if config.stdout {
        subscriber.try_init()?;
    }

    Ok(())
}

/// Initialize logging with compact formatting
fn init_compact_logging(level: Level, config: &LoggingConfig) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_max_level(level)
        .with_level(true)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true);

    if let Some(file_path) = &config.file {
        let (writer, _guard) = create_non_blocking_file(file_path)?;

        if config.stdout {
            subscriber.with_writer(std::io::stdout).try_init()?;
            // Note: we can't easily log to both stdout and file with simple setup
            tracing::warn!("Configured for stdout only; file logging ignored");
        } else {
            subscriber.with_writer(writer).try_init()?;
        }
    } else if config.stdout {
        subscriber.try_init()?;
    }

    Ok(())
}

/// Initialize logging with pretty formatting
fn init_pretty_logging(level: Level, config: &LoggingConfig) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_max_level(level)
        .with_level(true)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true);

    if let Some(file_path) = &config.file {
        let (writer, _guard) = create_non_blocking_file(file_path)?;

        if config.stdout {
            subscriber.with_writer(std::io::stdout).try_init()?;
            // Note: we can't easily log to both stdout and file with simple setup
            tracing::warn!("Configured for stdout only; file logging ignored");
        } else {
            subscriber.with_writer(writer).try_init()?;
        }
    } else if config.stdout {
        subscriber.try_init()?;
    }

    Ok(())
}

/// Create a non-blocking file writer.
fn create_non_blocking_file(
    path: impl AsRef<Path>,
) -> Result<(NonBlocking, tracing_appender::non_blocking::WorkerGuard)> {
    let path = path.as_ref();

    // Ensure the directory exists
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)?;
    }

    // Create a rolling file appender
    let file_appender = tracing_appender::rolling::never(
        path.parent().unwrap_or_else(|| Path::new(".")),
        path.file_name().unwrap_or_default(),
    );

    // Create a non-blocking writer
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    Ok((non_blocking, guard))
}

/// Parse a log level string into a LogLevel enum.
pub fn parse_log_level(level: &str) -> Result<LogLevel> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(LogLevel::Trace),
        "debug" => Ok(LogLevel::Debug),
        "info" => Ok(LogLevel::Info),
        "warn" => Ok(LogLevel::Warn),
        "error" => Ok(LogLevel::Error),
        _ => Err(LogError::InvalidLogLevel(level.to_string())),
    }
}

/// Convert a tracing::Level to a LogLevel enum.
pub fn level_to_log_level(level: Level) -> LogLevel {
    match level {
        Level::TRACE => LogLevel::Trace,
        Level::DEBUG => LogLevel::Debug,
        Level::INFO => LogLevel::Info,
        Level::WARN => LogLevel::Warn,
        Level::ERROR => LogLevel::Error,
    }
}

/// Set the log level at runtime.
pub fn set_log_level(level: LogLevel) -> Result<()> {
    // This is a placeholder - actual implementation would update the filter
    // on the global subscriber, which requires additional setup.
    // For now we'll just log a message.
    let level_name = match level {
        LogLevel::Trace => "TRACE",
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    };
    tracing::info!("Log level changed to {}", level_name);
    Ok(())
}

/// Helper macro for structured logging with additional fields.
#[macro_export]
macro_rules! log_with_fields {
    ($level:expr, $($fields:tt)+) => {
        tracing::event!($level, $($fields)+)
    };
}

/// Helper macro for logging errors with context.
#[macro_export]
macro_rules! log_error {
    ($err:expr, $msg:expr $(, $fields:tt)*) => {
        tracing::error!(
            error = $err.to_string(),
            message = $msg,
            $($fields)*
        )
    };
}

impl std::fmt::Display for LogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogError::IoError(e) => write!(f, "IO error: {}", e),
            LogError::SubscriberError(e) => write!(f, "Subscriber error: {}", e),
            LogError::InvalidLogLevel(s) => write!(f, "Invalid log level: {}", s),
            LogError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for LogError {}
