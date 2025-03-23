//! Logging functionality for the Kincir messaging system.
//!
//! This module provides logging capabilities that can be used throughout
//! the Kincir library. It includes:
//!
//! - A `Logger` trait for customizable logging
//! - A standard logger implementation (`StdLogger`)
//!
//! # Example
//!
//! ```rust,no_run
//! use kincir::logging::{Logger, StdLogger};
//! use std::sync::Arc;
//!
//! // Create a logger that shows both info and error messages
//! let logger = Arc::new(StdLogger::new(true, true));
//!
//! // Use the logger
//! # async fn example(logger: Arc<StdLogger>) {
//! logger.info("This is an informational message").await;
//! logger.error("This is an error message").await;
//! # }
//! ```

use async_trait::async_trait;

/// Defines the interface for logging operations in the router.
#[async_trait]
pub trait Logger: Send + Sync {
    /// Logs an informational message.
    async fn info(&self, msg: &str);
    /// Logs an error message.
    async fn error(&self, msg: &str);
}

/// A standard implementation of the Logger trait that writes to stdout/stderr.
///
/// This logger provides basic logging capabilities with configurable output levels.
pub struct StdLogger {
    /// Whether to show informational messages
    show_info: bool,
    /// Whether to show error messages
    show_error: bool,
}

impl StdLogger {
    /// Creates a new StdLogger with configurable output levels.
    ///
    /// # Arguments
    ///
    /// * `show_info` - Whether to display informational messages
    /// * `show_error` - Whether to display error messages
    pub fn new(show_info: bool, show_error: bool) -> Self {
        Self {
            show_info,
            show_error,
        }
    }
}

#[async_trait]
impl Logger for StdLogger {
    async fn info(&self, msg: &str) {
        if self.show_info {
            println!("[INFO] {}", msg);
        }
    }

    async fn error(&self, msg: &str) {
        if self.show_error {
            eprintln!("[ERROR] {}", msg);
        }
    }
}

/// A no-op logger implementation that does nothing.
///
/// This logger can be used when logging is not required, and all logging
/// calls will be effectively no-ops.
pub struct NoOpLogger;

impl NoOpLogger {
    /// Creates a new NoOpLogger.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Logger for NoOpLogger {
    async fn info(&self, _msg: &str) {
        // Do nothing
    }

    async fn error(&self, _msg: &str) {
        // Do nothing
    }
} 