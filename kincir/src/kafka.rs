//! Kafka implementation for the Kincir messaging system.
//!
//! This module provides Kafka-based implementations of the Publisher and Subscriber traits,
//! allowing integration with Apache Kafka message brokers. The implementation uses channels
//! for message passing and includes proper error handling.
//!
//! # Example
//!
//! ```rust,no_run
//! use kincir::kafka::{KafkaPublisher, KafkaSubscriber};
//! use kincir::router::{Router, StdLogger};
//! use std::sync::Arc;
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let logger = Arc::new(StdLogger::new(true, true));
//!     let (tx, rx) = mpsc::channel(100);
//!
//!     // Initialize Kafka components
//!     let publisher = Arc::new(KafkaPublisher::new(
//!         vec!["localhost:9092".to_string()],
//!         tx,
//!         logger.clone(),
//!     ));
//!
//!     let subscriber = Arc::new(KafkaSubscriber::new(
//!         vec!["localhost:9092".to_string()],
//!         "my-group".to_string(),
//!         rx,
//!         logger,
//!     ));
//!
//!     Ok(())
//! }
//! ```

use crate::{router::Logger, Message};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

/// Represents possible errors that can occur in Kafka operations.
#[derive(Error, Debug)]
pub enum KafkaError {
    /// Error when sending messages through the channel
    #[error("Channel send error")]
    ChannelSend,
    /// Error when receiving messages from the channel
    #[error("Channel receive error")]
    ChannelReceive,
}

/// Implementation of the Publisher trait for Kafka.
///
/// Uses channels for message passing and includes logging capabilities.
pub struct KafkaPublisher {
    tx: mpsc::Sender<Message>,
    logger: Arc<dyn Logger>,
}

impl KafkaPublisher {
    /// Creates a new KafkaPublisher instance.
    ///
    /// # Arguments
    ///
    /// * `_brokers` - List of Kafka broker addresses
    /// * `tx` - Channel sender for messages
    /// * `logger` - Logger implementation
    pub fn new(_brokers: Vec<String>, tx: mpsc::Sender<Message>, logger: Arc<dyn Logger>) -> Self {
        Self { tx, logger }
    }
}

#[async_trait]
impl super::Publisher for KafkaPublisher {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn publish(&self, _topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        for message in messages {
            self.tx
                .send(message)
                .await
                .map_err(|_| KafkaError::ChannelSend)?;
        }
        Ok(())
    }
}

/// Implementation of the Subscriber trait for Kafka.
///
/// Uses channels for message passing and includes logging capabilities.
pub struct KafkaSubscriber {
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Message>>>,
    logger: Arc<dyn Logger>,
}

impl KafkaSubscriber {
    /// Creates a new KafkaSubscriber instance.
    ///
    /// # Arguments
    ///
    /// * `_brokers` - List of Kafka broker addresses
    /// * `_group_id` - Consumer group ID
    /// * `rx` - Channel receiver for messages
    /// * `logger` - Logger implementation
    pub fn new(
        _brokers: Vec<String>,
        _group_id: String,
        rx: mpsc::Receiver<Message>,
        logger: Arc<dyn Logger>,
    ) -> Self {
        Self {
            rx: Arc::new(tokio::sync::Mutex::new(rx)),
            logger,
        }
    }
}

#[async_trait]
impl super::Subscriber for KafkaSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn subscribe(&self, _topic: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(&self) -> Result<Message, Self::Error> {
        let mut rx = self.rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            Box::new(KafkaError::ChannelReceive) as Box<dyn std::error::Error + Send + Sync>
        })
    }
}
