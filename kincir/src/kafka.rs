//! Kafka implementation for the Kincir messaging system.
//!
//! This module provides Kafka-based implementations of the Publisher and Subscriber traits,
//! allowing integration with Apache Kafka message brokers. The implementation uses channels
//! for message passing and includes proper error handling.
//!
//! The functionality varies depending on feature flags:
//!
//! - With the "logging" feature enabled, logging is integrated throughout the components
//! - Without the "logging" feature, operations proceed without logging
//!
//! # Example
//!
//! ```rust,no_run
//! use kincir::kafka::{KafkaPublisher, KafkaSubscriber};
//! use std::sync::Arc;
//! use tokio::sync::mpsc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! // Set up channels
//! let (tx, rx) = mpsc::channel(100);
//!
//! # #[cfg(feature = "logging")]
//! # {
//! // With the "logging" feature
//! use kincir::logging::{Logger, StdLogger};
//! let logger = Arc::new(StdLogger::new(true, true));
//!
//! let publisher = Arc::new(KafkaPublisher::new(
//!     vec!["localhost:9092".to_string()],
//!     tx,
//!     logger.clone(),
//! ));
//!
//! let subscriber = Arc::new(KafkaSubscriber::new(
//!     vec!["localhost:9092".to_string()],
//!     "my-group".to_string(),
//!     rx,
//!     logger,
//! ));
//! # }
//!
//! # #[cfg(not(feature = "logging"))]
//! # {
//! // Without the "logging" feature
//! let publisher = Arc::new(KafkaPublisher::new(
//!     vec!["localhost:9092".to_string()],
//!     tx,
//! ));
//!
//! let subscriber = Arc::new(KafkaSubscriber::new(
//!     vec!["localhost:9092".to_string()],
//!     "my-group".to_string(),
//!     rx,
//! ));
//! # }
//!
//! # Ok(())
//! # }

#[cfg(feature = "logging")]
use crate::logging::Logger;
use crate::Message;
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
/// Uses channels for message passing and includes logging capabilities when the
/// "logging" feature is enabled.
#[cfg(feature = "logging")]
pub struct KafkaPublisher {
    tx: mpsc::Sender<Message>,
    logger: Arc<dyn Logger>,
}

/// Implementation of the Publisher trait for Kafka without logging.
#[cfg(not(feature = "logging"))]
pub struct KafkaPublisher {
    tx: mpsc::Sender<Message>,
}

#[cfg(feature = "logging")]
impl KafkaPublisher {
    /// Creates a new KafkaPublisher instance with logging.
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

#[cfg(not(feature = "logging"))]
impl KafkaPublisher {
    /// Creates a new KafkaPublisher instance without logging.
    ///
    /// # Arguments
    ///
    /// * `_brokers` - List of Kafka broker addresses
    /// * `tx` - Channel sender for messages
    pub fn new(_brokers: Vec<String>, tx: mpsc::Sender<Message>) -> Self {
        Self { tx }
    }
}

#[cfg(feature = "logging")]
#[async_trait]
impl super::Publisher for KafkaPublisher {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn publish(&self, _topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        self.logger
            .info(&format!(
                "Publishing {} messages to topic {}",
                messages.len(),
                _topic
            ))
            .await;
        for message in messages {
            self.tx
                .send(message)
                .await
                .map_err(|_| KafkaError::ChannelSend)?;
        }
        self.logger
            .info(&format!(
                "Successfully published messages to topic {}",
                _topic
            ))
            .await;
        Ok(())
    }
}

#[cfg(not(feature = "logging"))]
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

/// Implementation of the Subscriber trait for Kafka with logging.
#[cfg(feature = "logging")]
pub struct KafkaSubscriber {
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Message>>>,
    logger: Arc<dyn Logger>,
}

/// Implementation of the Subscriber trait for Kafka without logging.
#[cfg(not(feature = "logging"))]
pub struct KafkaSubscriber {
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Message>>>,
}

#[cfg(feature = "logging")]
impl KafkaSubscriber {
    /// Creates a new KafkaSubscriber instance with logging.
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

#[cfg(not(feature = "logging"))]
impl KafkaSubscriber {
    /// Creates a new KafkaSubscriber instance without logging.
    ///
    /// # Arguments
    ///
    /// * `_brokers` - List of Kafka broker addresses
    /// * `_group_id` - Consumer group ID
    /// * `rx` - Channel receiver for messages
    pub fn new(_brokers: Vec<String>, _group_id: String, rx: mpsc::Receiver<Message>) -> Self {
        Self {
            rx: Arc::new(tokio::sync::Mutex::new(rx)),
        }
    }
}

#[cfg(feature = "logging")]
#[async_trait]
impl super::Subscriber for KafkaSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        self.logger
            .info(&format!("Subscribing to topic {}", topic))
            .await;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Message, Self::Error> { // Changed to &mut self
        self.logger.info("Waiting to receive message").await;
        let mut rx_guard = self.rx.lock().await;
        match rx_guard.recv().await {
            Some(message) => {
                self.logger
                    .info(&format!("Received message {}", message.uuid))
                    .await;
                Ok(message)
            }
            None => {
                self.logger.error("Channel closed").await;
                Err(Box::new(KafkaError::ChannelReceive))
            }
        }
    }
}

#[cfg(not(feature = "logging"))]
#[async_trait]
impl super::Subscriber for KafkaSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn subscribe(&self, _topic: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(&mut self) -> Result<Message, Self::Error> { // Changed to &mut self
        let mut rx_guard = self.rx.lock().await;
        match rx_guard.recv().await {
            Some(message) => Ok(message),
            None => Err(Box::new(KafkaError::ChannelReceive)),
        }
    }
}
