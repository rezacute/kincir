//! Kincir is a unified message streaming library that provides a consistent interface for
//! working with multiple message broker backends. It simplifies the process of building
//! message-driven applications by offering:
//!
//! - A unified API for publishing and subscribing to messages
//! - Support for multiple message broker backends (Kafka, RabbitMQ)
//! - Message routing with customizable handlers
//! - Message tracking with UUID generation
//! - Extensible message metadata
//!
//! When the "logging" feature is enabled (default), Kincir also provides built-in logging capabilities.
//!
//! When the "protobuf" feature is enabled, Kincir provides Protocol Buffers encoding/decoding capabilities.
//!
//! # Example
//!
//! Basic usage with RabbitMQ backend:
//!
//! ```rust,no_run
//! use kincir::Message;
//! use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
//! use kincir::router::Router;
//! use std::sync::Arc;
//! use std::pin::Pin;
//! use std::future::Future;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! // Create and configure components
//! let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
//! let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);
//!
//! // Define message handler with explicit type signature
//! let handler = Arc::new(|msg: Message| -> Pin<Box<dyn Future<Output = Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>>> + Send>> {
//!     Box::pin(async move {
//!         let processed_msg = msg.with_metadata("processed", "true");
//!         Ok(vec![processed_msg])
//!     })
//! });
//!
//! // Set up and run router (with or without logging based on feature)
//! # #[cfg(feature = "logging")]
//! # {
//! // With logging (when "logging" feature is enabled)
//! use kincir::logging::{Logger, StdLogger};
//! let logger = Arc::new(StdLogger::new(true, true));
//! let router = Router::new(
//!     logger,
//!     "input-queue".to_string(),
//!     "output-queue".to_string(),
//!     subscriber.clone(),
//!     publisher.clone(),
//!     handler.clone(),
//! );
//!
//! router.run().await
//! # }
//! # #[cfg(not(feature = "logging"))]
//! # {
//! // Without logging (when "logging" feature is disabled)
//! let router = Router::new(
//!     "input-queue".to_string(),
//!     "output-queue".to_string(),
//!     subscriber,
//!     publisher,
//!     handler,
//! );
//!
//! router.run().await
//! # }
//! # }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a message in the system with unique identification, payload, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for the message
    pub uuid: String,
    /// The actual message content as bytes
    pub payload: Vec<u8>,
    /// Additional key-value pairs associated with the message
    pub metadata: HashMap<String, String>,
}

impl Message {
    /// Creates a new message with the given payload and a randomly generated UUID.
    ///
    /// # Examples
    ///
    /// ```
    /// use kincir::Message;
    ///
    /// let payload = b"Hello, World!".to_vec();
    /// let message = Message::new(payload);
    /// ```
    pub fn new(payload: Vec<u8>) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            payload,
            metadata: HashMap::new(),
        }
    }

    /// Adds metadata to the message and returns the modified message.
    ///
    /// # Examples
    ///
    /// ```
    /// use kincir::Message;
    ///
    /// let message = Message::new(b"Hello".to_vec())
    ///     .with_metadata("content-type", "text/plain")
    ///     .with_metadata("priority", "high");
    /// ```
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Defines the interface for publishing messages to a message broker.
#[async_trait]
pub trait Publisher {
    /// The type of error that can occur during publishing operations.
    type Error;

    /// Publishes a batch of messages to the specified topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - The destination topic or queue
    /// * `messages` - A vector of messages to publish
    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error>;
}

/// Defines the interface for subscribing to messages from a message broker.
#[async_trait]
pub trait Subscriber {
    /// The type of error that can occur during subscription operations.
    type Error;

    /// Subscribes to messages from the specified topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic or queue to subscribe to
    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error>;

    /// Receives the next available message from the subscribed topic.
    async fn receive(&self) -> Result<Message, Self::Error>;
}

pub mod kafka;
pub mod rabbitmq;
pub mod router;

#[cfg(feature = "logging")]
pub mod logging;

#[cfg(feature = "protobuf")]
pub mod protobuf;

// Re-export commonly used types
#[cfg(feature = "logging")]
pub use logging::{Logger, NoOpLogger, StdLogger};
#[cfg(feature = "protobuf")]
pub use protobuf::{MessageCodec, ProtobufCodec};
pub use router::HandlerFunc;
pub use router::Router;
