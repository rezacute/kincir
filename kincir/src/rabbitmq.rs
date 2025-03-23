//! RabbitMQ implementation for the Kincir messaging system.
//!
//! This module provides RabbitMQ-based implementations of the Publisher and Subscriber traits,
//! allowing integration with RabbitMQ message brokers. The implementation uses the `lapin`
//! library for RabbitMQ communication and includes proper error handling.
//!
//! # Example
//!
//! ```rust,no_run
//! use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
//! use kincir::logging::StdLogger;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let logger = Arc::new(StdLogger::new(true, true));
//!
//!     // Initialize RabbitMQ components
//!     let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
//!     let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);
//!
//!     Ok(())
//! }
//! ```

#[cfg(feature = "logging")]
use crate::logging::Logger;
use crate::Message;
use async_trait::async_trait;
use futures::StreamExt;
use lapin::options::{BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Connection, ConnectionProperties};
use serde_json;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RabbitMQError {
    /// Error when interacting with RabbitMQ
    #[error("RabbitMQ error: {0}")]
    RabbitMQ(#[from] lapin::Error),
    /// Error when serializing/deserializing messages
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Implementation of the Publisher trait for RabbitMQ.
///
/// Uses the lapin library for RabbitMQ communication.
pub struct RabbitMQPublisher {
    connection: Connection,
    #[cfg(feature = "logging")]
    logger: Arc<dyn Logger>,
}

impl RabbitMQPublisher {
    /// Creates a new RabbitMQPublisher instance.
    ///
    /// # Arguments
    ///
    /// * `uri` - The RabbitMQ connection URI (e.g., "amqp://localhost:5672")
    #[cfg(not(feature = "logging"))]
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(uri, ConnectionProperties::default())
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        Ok(Self { connection })
    }

    /// Creates a new RabbitMQPublisher instance with logging.
    ///
    /// # Arguments
    ///
    /// * `uri` - The RabbitMQ connection URI (e.g., "amqp://localhost:5672")
    /// * `logger` - The logger implementation to use
    #[cfg(feature = "logging")]
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(uri, ConnectionProperties::default())
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        // Create a default NoOpLogger
        let logger = Arc::new(crate::logging::NoOpLogger::new());

        Ok(Self { connection, logger })
    }

    /// Sets a logger for the publisher (only available with the "logging" feature).
    #[cfg(feature = "logging")]
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = logger;
        self
    }
}

#[cfg(feature = "logging")]
#[async_trait]
impl super::Publisher for RabbitMQPublisher {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        self.logger
            .info(&format!(
                "Publishing {} messages to {}",
                messages.len(),
                topic
            ))
            .await;

        let channel = self.connection.create_channel().await.map_err(|e| {
            Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
        })?;

        channel
            .queue_declare(topic, QueueDeclareOptions::default(), FieldTable::default())
            .await
            .map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

        for message in messages {
            let payload = serde_json::to_vec(&message).map_err(|e| {
                Box::new(RabbitMQError::Serialization(e))
                    as Box<dyn std::error::Error + Send + Sync>
            })?;
            let confirm = channel
                .basic_publish(
                    "",
                    topic,
                    BasicPublishOptions::default(),
                    &payload,
                    BasicProperties::default(),
                )
                .await
                .map_err(|e| {
                    Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
                })?;
            let _ = confirm.await.map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            self.logger
                .info(&format!("Published message {} to {}", message.uuid, topic))
                .await;
        }

        Ok(())
    }
}

#[cfg(not(feature = "logging"))]
#[async_trait]
impl super::Publisher for RabbitMQPublisher {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        let channel = self.connection.create_channel().await.map_err(|e| {
            Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
        })?;

        channel
            .queue_declare(topic, QueueDeclareOptions::default(), FieldTable::default())
            .await
            .map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

        for message in messages {
            let payload = serde_json::to_vec(&message).map_err(|e| {
                Box::new(RabbitMQError::Serialization(e))
                    as Box<dyn std::error::Error + Send + Sync>
            })?;
            let confirm = channel
                .basic_publish(
                    "",
                    topic,
                    BasicPublishOptions::default(),
                    &payload,
                    BasicProperties::default(),
                )
                .await
                .map_err(|e| {
                    Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
                })?;
            let _ = confirm.await.map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;
        }

        Ok(())
    }
}

/// Implementation of the Subscriber trait for RabbitMQ.
///
/// Uses the lapin library for RabbitMQ communication.
pub struct RabbitMQSubscriber {
    connection: Connection,
    topic: Arc<tokio::sync::Mutex<Option<String>>>,
    consumer: Arc<tokio::sync::Mutex<Option<lapin::Consumer>>>,
    #[cfg(feature = "logging")]
    logger: Arc<dyn Logger>,
}

impl RabbitMQSubscriber {
    /// Creates a new RabbitMQSubscriber instance.
    ///
    /// # Arguments
    ///
    /// * `uri` - The RabbitMQ connection URI (e.g., "amqp://localhost:5672")
    #[cfg(not(feature = "logging"))]
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(uri, ConnectionProperties::default())
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        Ok(Self {
            connection,
            topic: Arc::new(tokio::sync::Mutex::new(None)),
            consumer: Arc::new(tokio::sync::Mutex::new(None)),
        })
    }

    /// Creates a new RabbitMQSubscriber instance with logging.
    ///
    /// # Arguments
    ///
    /// * `uri` - The RabbitMQ connection URI (e.g., "amqp://localhost:5672")
    #[cfg(feature = "logging")]
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(uri, ConnectionProperties::default())
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        // Create a default NoOpLogger
        let logger = Arc::new(crate::logging::NoOpLogger::new());

        Ok(Self {
            connection,
            topic: Arc::new(tokio::sync::Mutex::new(None)),
            consumer: Arc::new(tokio::sync::Mutex::new(None)),
            logger,
        })
    }

    /// Sets a logger for the subscriber (only available with the "logging" feature).
    #[cfg(feature = "logging")]
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = logger;
        self
    }
}

#[cfg(feature = "logging")]
#[async_trait]
impl super::Subscriber for RabbitMQSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        self.logger
            .info(&format!("Subscribing to topic {}", topic))
            .await;

        let channel = self.connection.create_channel().await.map_err(|e| {
            Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
        })?;

        channel
            .queue_declare(topic, QueueDeclareOptions::default(), FieldTable::default())
            .await
            .map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

        let mut topic_guard = self.topic.lock().await;
        *topic_guard = Some(topic.to_string());

        // Create a consumer for the topic
        let mut consumer_guard = self.consumer.lock().await;
        *consumer_guard = Some(
            channel
                .basic_consume(
                    topic,
                    &format!("consumer-{}", uuid::Uuid::new_v4()),
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
                .map_err(|e| {
                    Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
                })?,
        );

        self.logger
            .info(&format!("Successfully subscribed to topic {}", topic))
            .await;

        Ok(())
    }

    async fn receive(&self) -> Result<Message, Self::Error> {
        self.logger.info("Waiting to receive message").await;

        let topic_guard = self.topic.lock().await;
        let _topic = topic_guard.as_ref().ok_or_else(|| {
            Box::new(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState(
                lapin::ChannelState::Error,
            ))) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let mut consumer_guard = self.consumer.lock().await;
        let consumer = consumer_guard.as_mut().ok_or_else(|| {
            Box::new(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState(
                lapin::ChannelState::Error,
            ))) as Box<dyn std::error::Error + Send + Sync>
        })?;

        if let Some(delivery) = consumer.next().await {
            let delivery = delivery.map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;
            let message: Message = serde_json::from_slice(&delivery.data).map_err(|e| {
                Box::new(RabbitMQError::Serialization(e))
                    as Box<dyn std::error::Error + Send + Sync>
            })?;
            delivery.ack(Default::default()).await.map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            self.logger
                .info(&format!("Received message {}", message.uuid))
                .await;

            Ok(message)
        } else {
            self.logger
                .error("Consumer stream ended unexpectedly")
                .await;

            Err(
                Box::new(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState(
                    lapin::ChannelState::Error,
                ))) as Box<dyn std::error::Error + Send + Sync>,
            )
        }
    }
}

#[cfg(not(feature = "logging"))]
#[async_trait]
impl super::Subscriber for RabbitMQSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        let channel = self.connection.create_channel().await.map_err(|e| {
            Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
        })?;

        channel
            .queue_declare(topic, QueueDeclareOptions::default(), FieldTable::default())
            .await
            .map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

        let mut topic_guard = self.topic.lock().await;
        *topic_guard = Some(topic.to_string());

        // Create a consumer for the topic
        let mut consumer_guard = self.consumer.lock().await;
        *consumer_guard = Some(
            channel
                .basic_consume(
                    topic,
                    &format!("consumer-{}", uuid::Uuid::new_v4()),
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
                .map_err(|e| {
                    Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
                })?,
        );

        Ok(())
    }

    async fn receive(&self) -> Result<Message, Self::Error> {
        let topic_guard = self.topic.lock().await;
        let _topic = topic_guard.as_ref().ok_or_else(|| {
            Box::new(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState(
                lapin::ChannelState::Error,
            ))) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let mut consumer_guard = self.consumer.lock().await;
        let consumer = consumer_guard.as_mut().ok_or_else(|| {
            Box::new(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState(
                lapin::ChannelState::Error,
            ))) as Box<dyn std::error::Error + Send + Sync>
        })?;

        if let Some(delivery) = consumer.next().await {
            let delivery = delivery.map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;
            let message: Message = serde_json::from_slice(&delivery.data).map_err(|e| {
                Box::new(RabbitMQError::Serialization(e))
                    as Box<dyn std::error::Error + Send + Sync>
            })?;
            delivery.ack(Default::default()).await.map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;
            Ok(message)
        } else {
            Err(
                Box::new(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState(
                    lapin::ChannelState::Error,
                ))) as Box<dyn std::error::Error + Send + Sync>,
            )
        }
    }
}
