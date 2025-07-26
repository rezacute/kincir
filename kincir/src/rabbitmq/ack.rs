//! Acknowledgment handling for RabbitMQ message broker

use crate::ack::{AckHandle, AckSubscriber};
use crate::rabbitmq::RabbitMQError;
use crate::Message;
use async_trait::async_trait;
use futures::StreamExt;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Connection, Consumer};
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

/// Acknowledgment handle for RabbitMQ messages
#[derive(Debug, Clone)]
pub struct RabbitMQAckHandle {
    /// Unique message identifier
    message_id: String,
    /// Topic/queue name
    topic: String,
    /// Message timestamp
    timestamp: SystemTime,
    /// Delivery attempt count
    delivery_count: u32,
    /// RabbitMQ delivery tag for acknowledgment
    delivery_tag: u64,
    /// Internal handle ID for tracking
    handle_id: String,
}

impl RabbitMQAckHandle {
    /// Create a new RabbitMQ acknowledgment handle
    pub fn new(
        message_id: String,
        topic: String,
        timestamp: SystemTime,
        delivery_count: u32,
        delivery_tag: u64,
    ) -> Self {
        Self {
            message_id,
            topic,
            timestamp,
            delivery_count,
            delivery_tag,
            handle_id: Uuid::new_v4().to_string(),
        }
    }

    /// Get the RabbitMQ delivery tag
    pub fn delivery_tag(&self) -> u64 {
        self.delivery_tag
    }

    /// Get the internal handle ID
    pub fn handle_id(&self) -> &str {
        &self.handle_id
    }
}

impl AckHandle for RabbitMQAckHandle {
    fn message_id(&self) -> &str {
        &self.message_id
    }

    fn topic(&self) -> &str {
        &self.topic
    }

    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    fn delivery_count(&self) -> u32 {
        self.delivery_count
    }
}

/// RabbitMQ subscriber with acknowledgment support
pub struct RabbitMQAckSubscriber {
    /// RabbitMQ connection
    connection: Connection,
    /// Current subscription state
    state: tokio::sync::Mutex<SubscriberState>,
    /// Logger for debugging (optional)
    #[cfg(feature = "logging")]
    logger: Arc<dyn crate::logging::Logger>,
}

#[derive(Debug)]
struct SubscriberState {
    /// Current topic subscription
    topic: Option<String>,
    /// RabbitMQ consumer
    consumer: Option<Consumer>,
    /// Channel for acknowledgment operations
    channel: Option<lapin::Channel>,
}

impl RabbitMQAckSubscriber {
    /// Create a new RabbitMQ acknowledgment subscriber
    #[cfg(not(feature = "logging"))]
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(uri, lapin::ConnectionProperties::default())
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        Ok(Self {
            connection,
            state: tokio::sync::Mutex::new(SubscriberState {
                topic: None,
                consumer: None,
                channel: None,
            }),
        })
    }

    /// Create a new RabbitMQ acknowledgment subscriber with logging
    #[cfg(feature = "logging")]
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(uri, lapin::ConnectionProperties::default())
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        let logger = Arc::new(crate::logging::NoOpLogger::new());

        Ok(Self {
            connection,
            state: tokio::sync::Mutex::new(SubscriberState {
                topic: None,
                consumer: None,
                channel: None,
            }),
            logger,
        })
    }

    /// Set a logger for the subscriber (only available with the "logging" feature)
    #[cfg(feature = "logging")]
    pub fn with_logger(mut self, logger: Arc<dyn crate::logging::Logger>) -> Self {
        self.logger = logger;
        self
    }

    /// Get the currently subscribed topic
    pub async fn subscribed_topic(&self) -> Option<String> {
        let state = self.state.lock().await;
        state.topic.clone()
    }

    /// Check if currently subscribed to a topic
    pub async fn is_subscribed(&self) -> bool {
        let state = self.state.lock().await;
        state.topic.is_some()
    }

    /// Acknowledge a message using its delivery information
    async fn ack_delivery(&self, delivery_tag: u64) -> Result<(), RabbitMQError> {
        let state = self.state.lock().await;
        if let Some(channel) = &state.channel {
            channel
                .basic_ack(delivery_tag, BasicAckOptions::default())
                .await
                .map_err(RabbitMQError::RabbitMQ)?;
        }
        Ok(())
    }

    /// Negatively acknowledge a message using its delivery information
    async fn nack_delivery(&self, delivery_tag: u64, requeue: bool) -> Result<(), RabbitMQError> {
        let state = self.state.lock().await;
        if let Some(channel) = &state.channel {
            channel
                .basic_nack(
                    delivery_tag,
                    BasicNackOptions {
                        multiple: false,
                        requeue,
                    },
                )
                .await
                .map_err(RabbitMQError::RabbitMQ)?;
        }
        Ok(())
    }
}

#[async_trait]
impl AckSubscriber for RabbitMQAckSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type AckHandle = RabbitMQAckHandle;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Subscribing to topic {} with acknowledgment support",
                topic
            ))
            .await;

        // Create a new channel for this subscription
        let channel = self.connection.create_channel().await.map_err(|e| {
            Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Declare the queue
        channel
            .queue_declare(topic, QueueDeclareOptions::default(), FieldTable::default())
            .await
            .map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

        // Create consumer with manual acknowledgment (no_ack = false)
        let consumer = channel
            .basic_consume(
                topic,
                "kincir-ack-consumer",
                BasicConsumeOptions {
                    no_local: false,
                    no_ack: false, // Manual acknowledgment
                    exclusive: false,
                    nowait: false,
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| {
                Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

        // Update state
        let mut state = self.state.lock().await;
        state.topic = Some(topic.to_string());
        state.consumer = Some(consumer);
        state.channel = Some(channel);

        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Successfully subscribed to topic {}", topic))
            .await;

        Ok(())
    }

    async fn receive_with_ack(&mut self) -> Result<(Message, Self::AckHandle), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info("Waiting to receive message with acknowledgment")
            .await;

        // Take the consumer out temporarily to avoid holding the lock during await
        let mut consumer = {
            let mut state = self.state.lock().await;
            state.consumer.take()
        };

        if let Some(ref mut consumer_ref) = consumer {
            if let Some(delivery_result) = consumer_ref.next().await {
                let delivery = delivery_result.map_err(|e| {
                    Box::new(RabbitMQError::RabbitMQ(e)) as Box<dyn std::error::Error + Send + Sync>
                })?;

                // Parse the message
                let message: Message = serde_json::from_slice(&delivery.data).map_err(|e| {
                    Box::new(RabbitMQError::Serialization(e))
                        as Box<dyn std::error::Error + Send + Sync>
                })?;

                // Get topic from state
                let topic = {
                    let state = self.state.lock().await;
                    state.topic.clone().unwrap_or_default()
                };

                // Create acknowledgment handle
                let ack_handle = RabbitMQAckHandle::new(
                    message.uuid.clone(),
                    topic,
                    SystemTime::now(),
                    1, // TODO: Track actual delivery count
                    delivery.delivery_tag,
                );

                // Put the consumer back
                let mut state = self.state.lock().await;
                state.consumer = consumer;

                #[cfg(feature = "logging")]
                self.logger
                    .info(&format!("Received message with ID: {}", message.uuid))
                    .await;

                Ok((message, ack_handle))
            } else {
                // Put the consumer back
                let mut state = self.state.lock().await;
                state.consumer = consumer;

                Err(
                    Box::new(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState(
                        lapin::ChannelState::Error,
                    ))) as Box<dyn std::error::Error + Send + Sync>,
                )
            }
        } else {
            Err(
                Box::new(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState(
                    lapin::ChannelState::Error,
                ))) as Box<dyn std::error::Error + Send + Sync>,
            )
        }
    }

    async fn ack(&self, handle: Self::AckHandle) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Acknowledging message: {}", handle.message_id()))
            .await;

        self.ack_delivery(handle.delivery_tag())
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn nack(&self, handle: Self::AckHandle, requeue: bool) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Negatively acknowledging message: {} (requeue: {})",
                handle.message_id(),
                requeue
            ))
            .await;

        self.nack_delivery(handle.delivery_tag(), requeue)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn ack_batch(&self, handles: Vec<Self::AckHandle>) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Batch acknowledging {} messages", handles.len()))
            .await;

        // RabbitMQ supports batch acknowledgment using the "multiple" flag
        // We'll acknowledge the highest delivery tag with multiple=true
        if let Some(max_handle) = handles.iter().max_by_key(|h| h.delivery_tag()) {
            let state = self.state.lock().await;
            if let Some(channel) = &state.channel {
                channel
                    .basic_ack(
                        max_handle.delivery_tag(),
                        BasicAckOptions { multiple: true },
                    )
                    .await
                    .map_err(|e| {
                        Box::new(RabbitMQError::RabbitMQ(e))
                            as Box<dyn std::error::Error + Send + Sync>
                    })?;
            }
        }

        Ok(())
    }

    async fn nack_batch(
        &self,
        handles: Vec<Self::AckHandle>,
        requeue: bool,
    ) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Batch negatively acknowledging {} messages (requeue: {})",
                handles.len(),
                requeue
            ))
            .await;

        // RabbitMQ supports batch nack using the "multiple" flag
        if let Some(max_handle) = handles.iter().max_by_key(|h| h.delivery_tag()) {
            let state = self.state.lock().await;
            if let Some(channel) = &state.channel {
                channel
                    .basic_nack(
                        max_handle.delivery_tag(),
                        BasicNackOptions {
                            multiple: true,
                            requeue,
                        },
                    )
                    .await
                    .map_err(|e| {
                        Box::new(RabbitMQError::RabbitMQ(e))
                            as Box<dyn std::error::Error + Send + Sync>
                    })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_rabbitmq_ack_handle_creation() {
        let handle = RabbitMQAckHandle::new(
            "msg-123".to_string(),
            "test-topic".to_string(),
            SystemTime::now(),
            1,
            12345,
        );

        assert_eq!(handle.message_id(), "msg-123");
        assert_eq!(handle.topic(), "test-topic");
        assert_eq!(handle.delivery_count(), 1);
        assert!(!handle.is_retry());
        assert_eq!(handle.delivery_tag(), 12345);
    }

    #[test]
    fn test_rabbitmq_ack_handle_retry() {
        let handle = RabbitMQAckHandle::new(
            "msg-456".to_string(),
            "test-topic".to_string(),
            SystemTime::now(),
            3,
            67890,
        );

        assert_eq!(handle.delivery_count(), 3);
        assert!(handle.is_retry());
    }

    // Note: Integration tests with actual RabbitMQ would require a running RabbitMQ instance
    // These would be added to a separate integration test suite
}
