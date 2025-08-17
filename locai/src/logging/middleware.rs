//! HTTP middleware for logging requests and responses.
//!
//! This module provides middleware components that can be used with
//! Axum or other HTTP servers to log API requests and responses.

use std::time::Instant;
use tracing::{error, info};

// The HTTP middleware functionality is only available when the "http" feature is enabled
#[cfg(feature = "http")]
use tracing::Span;

#[cfg(feature = "http")]
use axum::{extract::Request, response::Response};

#[cfg(feature = "http")]
#[allow(dead_code)]
/// Create a tracing-enabled request logger middleware for Axum.
///
/// This function should be used with `tower::ServiceBuilder` to add
/// request logging to an Axum HTTP server.
///
/// # Example
///
/// ```
/// use tower::ServiceBuilder;
/// use locai::logging::middleware::trace_requests;
///
/// let app = axum::Router::new()
///     // Add routes...
///     .layer(ServiceBuilder::new().layer(trace_requests()));
/// ```
pub fn trace_requests() -> impl Clone {
    tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(|request: &Request| {
            // Extract useful information from request
            let method = request.method().to_string();
            let uri = request.uri().to_string();
            let version = format!("{:?}", request.version());

            // Create a span with this information
            tracing::info_span!(
                "request",
                method = %method,
                uri = %uri,
                version = %version,
                status = tracing::field::Empty,
                latency = tracing::field::Empty,
            )
        })
        .on_request(|request: &Request, _span: &Span| {
            // Log when request starts
            info!("Started {} request to {}", request.method(), request.uri());
        })
        .on_response(
            |response: &Response, latency: std::time::Duration, span: &Span| {
                // Record status and latency
                let status = response.status().as_u16();
                span.record("status", status);
                span.record("latency", format!("{} ms", latency.as_millis()));

                // Log based on status code
                if status < 400 {
                    info!("Completed request with status {} in {:?}", status, latency);
                } else if status < 500 {
                    tracing::warn!("Request error (client): status {} in {:?}", status, latency);
                } else {
                    error!("Request error (server): status {} in {:?}", status, latency);
                }
            },
        )
        .on_failure(
            |error: &(dyn std::fmt::Display + Send + Sync),
             _latency: std::time::Duration,
             _span: &Span| {
                // Log unhandled errors
                error!(
                    error = %error,
                    "Request processing failed"
                );
            },
        )
}

/// Helper struct for database operation logging.
#[allow(dead_code)]
pub struct DbOpLogger;

impl DbOpLogger {
    /// Create a new database operation logger.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }

    /// Log a database operation with timing information.
    #[allow(dead_code)]
    pub async fn log_operation<F, T, E>(
        &self,
        operation: &str,
        entity: &str,
        fut: F,
    ) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        // Create span for this database operation
        let span = tracing::info_span!("db_operation", operation = %operation, entity = %entity);
        let _enter = span.enter();

        // Record start time
        let start = Instant::now();

        // Execute the operation
        let result = fut.await;

        // Record end time and calculate duration
        let duration = start.elapsed();

        match &result {
            Ok(_) => {
                info!(
                    duration_ms = %duration.as_millis(),
                    "{} {} completed successfully",
                    operation, entity
                );
            }
            Err(e) => {
                error!(
                    duration_ms = %duration.as_millis(),
                    error = %e,
                    "{} {} failed",
                    operation, entity
                );
            }
        }

        result
    }
}
