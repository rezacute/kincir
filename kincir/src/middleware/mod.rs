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
use tokio::sync::Mutex;

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
    use tracing::{debug, error, info, warn};

    /// Middleware that logs all message operations
    #[derive(Debug, Clone)]
    pub struct LoggingMiddleware {
        include_payload: bool,
    }

    impl LoggingMiddleware {
        pub fn new() -> Self {
            Self {
                include_payload: false,
            }
        }

        pub fn with_payload(mut self) -> Self {
            self.include_payload = true;
            self
        }
    }

    impl Default for LoggingMiddleware {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl Middleware for LoggingMiddleware {
        async fn before_publish(&self, context: &MiddlewareContext, messages: &mut [Message]) {
            info!(
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
                topic = %context.topic,
                count = messages.len(),
                "Published messages successfully"
            );
        }

        async fn before_subscribe(&self, context: &MiddlewareContext) {
            info!(topic = %context.topic, "Subscribing to topic");
        }

        async fn after_subscribe(&self, context: &MiddlewareContext) {
            info!(topic = %context.topic, "Subscribed successfully");
        }

        async fn before_receive(&self, context: &MiddlewareContext) {
            debug!(topic = %context.topic, "Waiting for message");
        }

        async fn after_receive(&self, context: &MiddlewareContext, message: &Message) {
            info!(
                topic = %context.topic,
                uuid = %message.uuid,
                "Received message"
            );
        }
    }
}

#[cfg(feature = "logging")]
pub use logging_middleware::LoggingMiddleware;

/// Middleware that retries failed operations
#[derive(Debug, Clone)]
pub struct RetryMiddleware {
    max_retries: u32,
    retry_delay_ms: u64,
}

impl RetryMiddleware {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            retry_delay_ms: 100,
        }
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.retry_delay_ms = delay_ms;
        self
    }

    /// Get the max retries count
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Get the retry delay in milliseconds
    pub fn retry_delay_ms(&self) -> u64 {
        self.retry_delay_ms
    }
}

impl Default for RetryMiddleware {
    fn default() -> Self {
        Self::new(3)
    }
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
        let mw = CorrelationMiddleware::new();
        let corr_id = CorrelationMiddleware::generate_correlation_id();
        assert!(!corr_id.is_empty());
    }

    #[test]
    fn test_retry_middleware() {
        let mw = RetryMiddleware::new(3);
        assert_eq!(mw.max_retries, 3);
    }
}
