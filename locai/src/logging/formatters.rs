//! Custom formatters for the logging system.
//!
//! This module provides formatting options for structured logging,
//! including custom formats for different environments.

use std::fmt;
use tracing_subscriber::fmt::{format, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use time::{format_description, OffsetDateTime};

/// Custom formatter for development that emphasizes readability.
pub struct DevelopmentFormatter;

impl<S, N> FormatEvent<S, N> for DevelopmentFormatter
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        // Get current time
        let now = OffsetDateTime::now_local()
            .unwrap_or_else(|_| OffsetDateTime::now_utc());
        
        // Define time format
        let time_format = format_description::parse("[hour]:[minute]:[second].[subsecond]")
            .unwrap();
        
        // Format the time
        let time = now.format(&time_format).unwrap_or_default();
        
        // Get metadata
        let metadata = event.metadata();
        let level = *metadata.level();
        let target = metadata.target();
        
        // Format colored level
        let level_str = match level {
            tracing::Level::TRACE => "\x1b[36mTRACE\x1b[0m",
            tracing::Level::DEBUG => "\x1b[34mDEBUG\x1b[0m",
            tracing::Level::INFO => "\x1b[32mINFO \x1b[0m",
            tracing::Level::WARN => "\x1b[33mWARN \x1b[0m",
            tracing::Level::ERROR => "\x1b[31mERROR\x1b[0m",
        };
        
        // Format the event
        write!(writer, "{} {} [{}] ", time, level_str, target)?;
        
        // Get span context
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                write!(writer, "{}: ", span.name())?;
            }
        }
        
        // Format the fields
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        
        writeln!(writer)
    }
}

/// Formatter that produces detailed JSON output for production and analysis.
pub struct DetailedJsonFormatter;

impl<S, N> FormatEvent<S, N> for DetailedJsonFormatter
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        // Build JSON manually for complete control
        write!(writer, "{{")?;
        
        // Add timestamp
        let now = OffsetDateTime::now_utc();
        let time_format = format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]Z")
            .unwrap();
        let time = now.format(&time_format).unwrap_or_default();
        write!(writer, r#""timestamp":"{time}","#)?;
        
        // Add level
        let level = event.metadata().level();
        write!(writer, r#""level":"{}","#, level)?;
        
        // Add target (module path)
        let target = event.metadata().target();
        write!(writer, r#""target":"{}","#, target)?;
        
        // Add file and line if available
        if let Some(file) = event.metadata().file() {
            write!(writer, r#""file":"{}","#, file)?;
        }
        if let Some(line) = event.metadata().line() {
            write!(writer, r#""line":{},"#, line)?;
        }
        
        // Add span context
        if let Some(scope) = ctx.event_scope() {
            let spans: Vec<_> = scope.from_root().map(|span| span.name()).collect();
            if !spans.is_empty() {
                write!(writer, r#""spans":["#)?;
                for (i, span) in spans.iter().enumerate() {
                    if i > 0 {
                        write!(writer, ",")?;
                    }
                    write!(writer, r#""{}""#, span)?;
                }
                write!(writer, "],")?;
            }
        }
        
        // Add thread ID
        let thread_id = format!("{:?}", std::thread::current().id());
        write!(writer, r#""thread_id":"{}","#, thread_id)?;
        
        // Add fields (the actual log message content)
        write!(writer, r#""fields":"#)?;
        let mut buffer = String::new();
        let mut formatter = CustomFieldFormatter::new(&mut buffer);
        event.record(&mut formatter);
        // Escape any quotes or backslashes
        let escaped = buffer.replace('\\', "\\\\").replace('"', "\\\"");
        write!(writer, r#""{}""#, escaped)?;
        
        // Close the JSON object
        write!(writer, "}}\n")?;
        Ok(())
    }
}

/// Custom field formatter for JSON output
struct CustomFieldFormatter<'a> {
    buffer: &'a mut String,
}

impl<'a> CustomFieldFormatter<'a> {
    fn new(buffer: &'a mut String) -> Self {
        Self { buffer }
    }
}

impl<'a> tracing::field::Visit for CustomFieldFormatter<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if !self.buffer.is_empty() {
            self.buffer.push_str(", ");
        }
        self.buffer.push_str(&format!("{}={:?}", field.name(), value));
    }
} 