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
//!     // tx, // This argument is no longer needed for the new KafkaPublisher
//!     // logger.clone(), // This argument is no longer needed
//! ).unwrap()); // Add unwrap() or error handling
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
//!     // tx, // No longer needed
//! ).unwrap()); // Add unwrap() or error handling
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

pub mod ack;
#[cfg(test)]
mod tests;

#[cfg(feature = "logging")]
use crate::logging::Logger;
use crate::Message;
use async_trait::async_trait;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord}; // Removed Producer
use rdkafka::util::Timeout;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

/// Represents possible errors that can occur in Kafka operations.
#[derive(Error, Debug)]
pub enum KafkaError {
    #[error("Kafka client creation error: {0}")]
    ClientCreation(String),
    #[error("Kafka publish error: {0}")]
    PublishError(String),
    #[error("Kafka configuration error: {0}")]
    ConfigurationError(String),
    #[error("Kafka receive error: {0}")]
    ReceiveError(String),
    #[error("Kafka error: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Implementation of the Publisher trait for Kafka.
#[derive(Clone)] // Added Clone derive
pub struct KafkaPublisher {
    producer: FutureProducer,
}

impl KafkaPublisher {
    pub fn new(broker_urls: Vec<String>) -> Result<Self, KafkaError> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", broker_urls.join(","))
            .set("message.timeout.ms", "5000")
            .create()
            .map_err(|e| KafkaError::ClientCreation(e.to_string()))?;

        Ok(Self { producer })
    }
}

#[async_trait]
impl crate::Publisher for KafkaPublisher {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn publish(&self, topic: &str, messages: Vec<crate::Message>) -> Result<(), Self::Error> {
        for message in messages {
            let record = FutureRecord::to(topic)
                .payload(&message.payload)
                .key(&message.uuid);

            match self.producer.send(record, Timeout::Never).await {
                Ok(_delivery_status) => {
                    // delivery_status is (partition, offset)
                }
                Err((e, _owned_message)) => {
                    return Err(Box::new(KafkaError::PublishError(e.to_string())));
                }
            }
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

    async fn receive(&mut self) -> Result<Message, Self::Error> {
        // Changed to &mut self

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
                Err(Box::new(KafkaError::ReceiveError(
                    "Channel closed".to_string(),
                )))
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

    async fn receive(&self) -> Result<Message, Self::Error> {
        let mut rx_guard = self.rx.lock().await;
        match rx_guard.recv().await {
            Some(message) => Ok(message),
            None => Err(Box::new(KafkaError::ReceiveError(
                "Channel closed".to_string(),
            ))),
        }
    }
}

// Re-export acknowledgment types
pub use ack::{KafkaAckHandle, KafkaAckSubscriber};
