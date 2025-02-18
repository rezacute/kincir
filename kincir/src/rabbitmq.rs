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
//! use kincir::router::{Router, StdLogger};
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

use crate::Message;
use async_trait::async_trait;
use futures::StreamExt;
use lapin::options::{BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Connection, ConnectionProperties};
use serde_json;
use thiserror::Error;
use tokio_amqp::*;

/// Represents possible errors that can occur in RabbitMQ operations.
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
}

impl RabbitMQPublisher {
    /// Creates a new RabbitMQPublisher instance.
    ///
    /// # Arguments
    ///
    /// * `uri` - The RabbitMQ connection URI (e.g., "amqp://localhost:5672")
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(
            uri,
            ConnectionProperties::default().with_tokio(),
        )
        .await
        .map_err(RabbitMQError::RabbitMQ)?;

        Ok(Self { connection })
    }
}

#[async_trait]
impl super::Publisher for RabbitMQPublisher {
    type Error = RabbitMQError;

    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        let channel = self.connection.create_channel().await.map_err(RabbitMQError::RabbitMQ)?;

        channel
            .queue_declare(
                topic,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        for message in messages {
            let payload = serde_json::to_vec(&message)?;
            let confirm = channel
                .basic_publish(
                    "",
                    topic,
                    BasicPublishOptions::default(),
                    &payload,
                    BasicProperties::default(),
                )
                .await
                .map_err(RabbitMQError::RabbitMQ)?;
            let _ = confirm.await.map_err(RabbitMQError::RabbitMQ)?;
        }

        Ok(())
    }
}

/// Implementation of the Subscriber trait for RabbitMQ.
///
/// Uses the lapin library for RabbitMQ communication.
pub struct RabbitMQSubscriber {
    connection: Connection,
}

impl RabbitMQSubscriber {
    /// Creates a new RabbitMQSubscriber instance.
    ///
    /// # Arguments
    ///
    /// * `uri` - The RabbitMQ connection URI (e.g., "amqp://localhost:5672")
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(
            uri,
            ConnectionProperties::default().with_tokio(),
        )
        .await
        .map_err(RabbitMQError::RabbitMQ)?;

        Ok(Self { connection })
    }
}

#[async_trait]
impl super::Subscriber for RabbitMQSubscriber {
    type Error = RabbitMQError;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        let channel = self.connection.create_channel().await.map_err(RabbitMQError::RabbitMQ)?;

        channel
            .queue_declare(
                topic,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        Ok(())
    }

    async fn receive(&self) -> Result<Message, Self::Error> {
        let channel = self.connection.create_channel().await.map_err(RabbitMQError::RabbitMQ)?;

        let mut consumer = channel
            .basic_consume(
                "",
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        if let Some(delivery) = consumer.next().await {
            let delivery = delivery.map_err(RabbitMQError::RabbitMQ)?;
            let message: Message = serde_json::from_slice(&delivery.data)?;
            delivery.ack(Default::default()).await.map_err(RabbitMQError::RabbitMQ)?;
            Ok(message)
        } else {
            Err(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState))
        }
    }
}