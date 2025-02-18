//! Kincir is a unified message streaming library that provides a consistent interface for
//! working with multiple message broker backends. It simplifies the process of building
//! message-driven applications by offering:
//!
//! - A unified API for publishing and subscribing to messages
//! - Support for multiple message broker backends (Kafka, RabbitMQ)
//! - Message routing with customizable handlers
//! - Built-in logging capabilities
//! - Message tracking with UUID generation
//! - Extensible message metadata
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
//! use kincir::router::{Router, Logger, StdLogger};
//! use kincir::Message;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     // Initialize components
//!     let logger = Arc::new(StdLogger::new(true, true));
//!     let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
//!     let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);
//!
//!     // Create message handler
//!     let handler = Arc::new(|msg: Message| {
//!         Box::pin(async move {
//!             Ok(vec![msg])
//!         })
//!     });
//!
//!     // Set up and run the router
//!     let router = Router::new(
//!         logger,
//!         "input-queue".to_string(),
//!         "output-queue".to_string(),
//!         subscriber,
//!         publisher,
//!         handler,
//!     );
//!
//!     router.run().await
//! }
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

// Re-export commonly used types
pub use router::{Logger, StdLogger, Router, HandlerFunc};
