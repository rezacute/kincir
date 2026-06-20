//! Middleware framework for Kincir
//!
//! This module provides middleware components that can intercept and modify
//! message processing operations.
//!
//! # Quick Start
//!
//! ```
//! use kincir::middleware::{Middleware, LoggingMiddleware, RetryMiddleware};
//! use kincir::Message;
//! ```
//!
//! # Available Middleware
//!
//! - [`LoggingMiddleware`] - Logs all messages
//! - [`RetryMiddleware`] - Retries failed operations
//! - [`CorrelationMiddleware`] - Adds correlation IDs

use crate::Message;
use async_trait::async_trait;
use std::sync::Arc;

/// Context passed to middleware methods
#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    /// Topic or queue name
    pub topic: String,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl MiddlewareContext {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Middleware trait for intercepting message operations
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Called before publishing messages
    async fn before_publish(&self, _context: &MiddlewareContext, _messages: &mut [Message]) {}

    /// Called after publishing messages
    async fn after_publish(&self, _context: &MiddlewareContext, _messages: &[Message]) {}

    /// Called before subscribing to a topic
    async fn before_subscribe(&self, _context: &MiddlewareContext) {}

    /// Called after subscribing to a topic
    async fn after_subscribe(&self, _context: &MiddlewareContext) {}

    /// Called before receiving a message
    async fn before_receive(&self, _context: &MiddlewareContext) {}

    /// Called after receiving a message
    async fn after_receive(&self, _context: &MiddlewareContext, _message: &Message) {}
}

/// Chain of middleware handlers
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    #[allow(clippy::should_implement_trait)] // `add` is an idiomatic builder method here, not arithmetic
    pub fn add<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    pub async fn before_publish(&self, context: &MiddlewareContext, messages: &mut [Message]) {
        for mw in &self.middlewares {
            mw.before_publish(context, messages).await;
        }
    }

    pub async fn after_publish(&self, context: &MiddlewareContext, messages: &[Message]) {
        for mw in &self.middlewares {
            mw.after_publish(context, messages).await;
        }
    }

    pub async fn before_subscribe(&self, context: &MiddlewareContext) {
        for mw in &self.middlewares {
            mw.before_subscribe(context).await;
        }
    }

    pub async fn after_subscribe(&self, context: &MiddlewareContext) {
        for mw in &self.middlewares {
            mw.after_subscribe(context).await;
        }
    }

    pub async fn before_receive(&self, context: &MiddlewareContext) {
        for mw in &self.middlewares {
            mw.before_receive(context).await;
        }
    }

    pub async fn after_receive(&self, context: &MiddlewareContext, message: &Message) {
        for mw in &self.middlewares {
            mw.after_receive(context, message).await;
        }
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "logging")]
mod logging_middleware {
    use super::*;
    use tracing::{debug, info};

    /// Middleware that logs all message operations
    #[derive(Debug, Clone)]
    pub struct LoggingMiddleware {
        name: String,
        include_payload: bool,
    }

    impl LoggingMiddleware {
        /// Create a new logging middleware with a label used to identify its
        /// output in the logs.
        pub fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                include_payload: false,
            }
        }

        pub fn with_payload(mut self) -> Self {
            self.include_payload = true;
            self
        }

        /// Get the configured label for this middleware.
        pub fn name(&self) -> &str {
            &self.name
        }
    }

    impl Default for LoggingMiddleware {
        fn default() -> Self {
            Self::new("kincir")
        }
    }

    #[async_trait]
    impl Middleware for LoggingMiddleware {
        async fn before_publish(&self, context: &MiddlewareContext, messages: &mut [Message]) {
            info!(
                middleware = %self.name,
                topic = %context.topic,
                count = messages.len(),
                "Publishing messages"
            );
            if self.include_payload {
                for msg in messages.iter().take(3) {
                    debug!(uuid = %msg.uuid, payload_len = msg.payload.len(), "Message payload");
                }
            }
        }

        async fn after_publish(&self, context: &MiddlewareContext, messages: &[Message]) {
            info!(
                middleware = %self.name,
                topic = %context.topic,
                count = messages.len(),
                "Published messages successfully"
            );
        }

        async fn before_subscribe(&self, context: &MiddlewareContext) {
            info!(middleware = %self.name, topic = %context.topic, "Subscribing to topic");
        }

        async fn after_subscribe(&self, context: &MiddlewareContext) {
            info!(middleware = %self.name, topic = %context.topic, "Subscribed successfully");
        }

        async fn before_receive(&self, context: &MiddlewareContext) {
            debug!(middleware = %self.name, topic = %context.topic, "Waiting for message");
        }

        async fn after_receive(&self, context: &MiddlewareContext, message: &Message) {
            info!(
                middleware = %self.name,
                topic = %context.topic,
                uuid = %message.uuid,
                "Received message"
            );
        }
    }
}

#[cfg(feature = "logging")]
pub use logging_middleware::LoggingMiddleware;

/// Configuration for [`RetryMiddleware`].
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Delay between retry attempts, in milliseconds.
    pub retry_delay_ms: u64,
}

impl RetryConfig {
    /// Create a new retry configuration with the default delay (100ms).
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            retry_delay_ms: 100,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::new(3)
    }
}

/// Middleware that retries failed operations
#[derive(Debug, Clone)]
pub struct RetryMiddleware {
    config: RetryConfig,
}

impl RetryMiddleware {
    pub fn new(max_retries: u32) -> Self {
        Self {
            config: RetryConfig::new(max_retries),
        }
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.config.retry_delay_ms = delay_ms;
        self
    }

    /// Get the retry configuration.
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Get the max retries count
    pub fn max_retries(&self) -> u32 {
        self.config.max_retries
    }

    /// Get the retry delay in milliseconds
    pub fn retry_delay_ms(&self) -> u64 {
        self.config.retry_delay_ms
    }
}

impl Default for RetryMiddleware {
    fn default() -> Self {
        Self::new(3)
    }
}

#[async_trait]
impl Middleware for RetryMiddleware {
    // The retry policy is exposed via `config()` for callers (e.g. routers) that
    // implement the retry loop. The middleware hooks themselves are no-ops so the
    // middleware can participate in a `MiddlewareChain` without altering messages.
}

/// Middleware that adds correlation IDs to messages
#[derive(Debug, Clone)]
pub struct CorrelationMiddleware {
    header_key: String,
}

impl CorrelationMiddleware {
    pub fn new() -> Self {
        Self {
            header_key: "correlation-id".to_string(),
        }
    }

    pub fn with_header_key(mut self, key: impl Into<String>) -> Self {
        self.header_key = key.into();
        self
    }

    pub fn generate_correlation_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

impl Default for CorrelationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for CorrelationMiddleware {
    async fn before_publish(&self, _context: &MiddlewareContext, messages: &mut [Message]) {
        let correlation_id = Self::generate_correlation_id();
        for msg in messages.iter_mut() {
            msg.metadata.insert(self.header_key.clone(), correlation_id.clone());
        }
    }

    async fn after_receive(&self, _context: &MiddlewareContext, message: &Message) {
        // Correlation ID is already in the message metadata
        if let Some(corr_id) = message.metadata.get(&self.header_key) {
            tracing::debug!(correlation_id = %corr_id, "Message correlation ID");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_context() {
        let ctx = MiddlewareContext::new("test-topic");
        assert_eq!(ctx.topic, "test-topic");

        let ctx = ctx.with_metadata("key", "value");
        assert_eq!(ctx.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_middleware_chain() {
        let chain = MiddlewareChain::new();
        // Chain can be created without middleware
        assert!(chain.middlewares.is_empty());
    }

    #[test]
    fn test_correlation_middleware() {
        let _mw = CorrelationMiddleware::new();
        let corr_id = CorrelationMiddleware::generate_correlation_id();
        assert!(!corr_id.is_empty());
    }

    #[test]
    fn test_retry_middleware() {
        let mw = RetryMiddleware::new(3);
        assert_eq!(mw.max_retries(), 3);
        assert_eq!(mw.config().max_retries, 3);
    }
}
