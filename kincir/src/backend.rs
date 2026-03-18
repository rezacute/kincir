//! Unified Backend API for Kincir
//!
//! This module provides a unified interface for creating and managing
//! different message broker backends through a common API.
//!
//! # Quick Start
//!
//! ```
//! use kincir::backend::{Backend, BackendBuilder};
//!
//! // Create a RabbitMQ backend
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let backend = BackendBuilder::create_rabbitmq("amqp://localhost:5672").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Supported Backends
//!
//! | Scheme | Backend |
//! |--------|---------|
//! | `amqp://` | RabbitMQ |
//! | `rabbitmq://` | RabbitMQ |
//! | `mqtt://` | MQTT |

use crate::mqtt::{MQTTPublisher, MQTTSubscriber, QoS};
use crate::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber, RabbitMQError};
use crate::{Publisher, Subscriber, Message};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;

/// Represents the type of backend to create
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    /// RabbitMQ broker
    RabbitMQ,
    /// MQTT broker
    MQTT,
}

impl BackendType {
    /// Parse backend type from connection string scheme
    pub fn from_scheme(scheme: &str) -> Option<Self> {
        match scheme.to_lowercase().as_str() {
            "amqp" | "rabbitmq" => Some(BackendType::RabbitMQ),
            "mqtt" => Some(BackendType::MQTT),
            _ => None,
        }
    }
}

/// Error type for backend operations
#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Backend error: {0}")]
    Other(String),

    #[error("Unsupported backend scheme: {0}")]
    UnsupportedScheme(String),

    #[error("RabbitMQ error: {0}")]
    RabbitMQError(String),

    #[error("MQTT error: {0}")]
    MQTTError(String),
}

/// Unified Backend trait that combines Publisher and Subscriber
#[async_trait]
pub trait Backend: Send + Sync {
    /// Get the backend type
    fn backend_type(&self) -> BackendType;

    /// Publish messages to a topic
    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), BackendError>;

    /// Subscribe to a topic
    async fn subscribe(&self, topic: &str) -> Result<(), BackendError>;

    /// Receive the next message
    async fn receive(&mut self) -> Result<Message, BackendError>;
}

/// RabbitMQ backend implementation
pub struct RabbitMQBackend {
    publisher: Arc<RabbitMQPublisher>,
    subscriber: Arc<Mutex<RabbitMQSubscriber>>,
}

impl RabbitMQBackend {
    pub async fn new(connection_string: &str) -> Result<Self, BackendError> {
        let publisher = RabbitMQPublisher::new(connection_string)
            .await
            .map_err(|e: RabbitMQError| BackendError::RabbitMQError(e.to_string()))?;
        let subscriber = RabbitMQSubscriber::new(connection_string)
            .await
            .map_err(|e: RabbitMQError| BackendError::RabbitMQError(e.to_string()))?;
        Ok(Self {
            publisher: Arc::new(publisher),
            subscriber: Arc::new(Mutex::new(subscriber))
        })
    }
}

#[async_trait]
impl Backend for RabbitMQBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::RabbitMQ
    }

    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), BackendError> {
        self.publisher.publish(topic, messages)
            .await
            .map_err(|e| BackendError::RabbitMQError(e.to_string()))
    }

    async fn subscribe(&self, topic: &str) -> Result<(), BackendError> {
        self.subscriber.lock().await.subscribe(topic)
            .await
            .map_err(|e| BackendError::RabbitMQError(e.to_string()))
    }

    async fn receive(&mut self) -> Result<Message, BackendError> {
        self.subscriber.lock().await.receive()
            .await
            .map_err(|e| BackendError::RabbitMQError(e.to_string()))
    }
}

/// MQTT backend implementation
pub struct MQTTBackend {
    publisher: Arc<MQTTPublisher>,
    subscriber: Arc<Mutex<MQTTSubscriber>>,
    topic: String,
}

impl MQTTBackend {
    pub fn new(broker_url: &str, topic: &str) -> Result<Self, BackendError> {
        let publisher = MQTTPublisher::new(broker_url, topic)
            .map_err(|e| BackendError::MQTTError(e.to_string()))?;
        let subscriber = MQTTSubscriber::new(broker_url, topic, QoS::AtLeastOnce)
            .map_err(|e| BackendError::MQTTError(e.to_string()))?;
        Ok(Self { 
            publisher: Arc::new(publisher), 
            subscriber: Arc::new(Mutex::new(subscriber)),
            topic: topic.to_string(),
        })
    }
}

#[async_trait]
impl Backend for MQTTBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::MQTT
    }

    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), BackendError> {
        self.publisher.publish(topic, messages)
            .await
            .map_err(|e| BackendError::MQTTError(e.to_string()))
    }

    async fn subscribe(&self, topic: &str) -> Result<(), BackendError> {
        self.subscriber.lock().await.subscribe(topic)
            .await
            .map_err(|e| BackendError::MQTTError(e.to_string()))
    }

    async fn receive(&mut self) -> Result<Message, BackendError> {
        self.subscriber.lock().await.receive()
            .await
            .map_err(|e| BackendError::MQTTError(e.to_string()))
    }
}

/// Backend builder for creating backends from connection strings
pub struct BackendBuilder;

impl BackendBuilder {
    /// Create a RabbitMQ backend from a connection string
    pub async fn create_rabbitmq(connection_string: &str) -> Result<Box<dyn Backend>, BackendError> {
        Ok(Box::new(RabbitMQBackend::new(connection_string).await?))
    }

    /// Create an MQTT backend
    pub fn create_mqtt(broker_url: &str, topic: &str) -> Result<Box<dyn Backend>, BackendError> {
        Ok(Box::new(MQTTBackend::new(broker_url, topic)?))
    }

    /// Create a backend from a connection string
    ///
    /// For RabbitMQ: use "amqp://" or "rabbitmq://" scheme
    /// For MQTT: use "mqtt://" scheme
    pub async fn from_connection_string(
        connection_string: &str,
        topic: &str,
    ) -> Result<Box<dyn Backend>, BackendError> {
        let url = url::Url::parse(connection_string)
            .map_err(|e| BackendError::Other(e.to_string()))?;
        let scheme = url.scheme();

        let backend_type = BackendType::from_scheme(scheme)
            .ok_or_else(|| BackendError::UnsupportedScheme(scheme.to_string()))?;

        match backend_type {
            BackendType::RabbitMQ => Ok(Box::new(RabbitMQBackend::new(connection_string).await?)),
            BackendType::MQTT => Ok(Box::new(MQTTBackend::new(connection_string, topic)?)),
        }
    }
}

#[cfg(test)]
mod tests;
