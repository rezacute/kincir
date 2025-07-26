//! Acknowledgment handling for the in-memory message broker

use crate::ack::{AckHandle, AckSubscriber};
use crate::memory::{InMemoryBroker, InMemoryError};
use crate::Message;
use async_trait::async_trait;
use std::sync::{Arc, Weak};
use std::time::SystemTime;
use uuid::Uuid;

/// Acknowledgment handle for in-memory broker messages
#[derive(Debug, Clone)]
pub struct InMemoryAckHandle {
    /// Unique message identifier
    message_id: String,
    /// Topic name
    topic: String,
    /// Message timestamp
    timestamp: SystemTime,
    /// Delivery attempt count
    delivery_count: u32,
    /// Weak reference to the broker for acknowledgment operations
    broker_ref: Weak<InMemoryBroker>,
    /// Internal handle ID for tracking
    handle_id: String,
}

impl InMemoryAckHandle {
    /// Create a new acknowledgment handle
    pub fn new(
        message_id: String,
        topic: String,
        timestamp: SystemTime,
        delivery_count: u32,
        broker_ref: Weak<InMemoryBroker>,
    ) -> Self {
        Self {
            message_id,
            topic,
            timestamp,
            delivery_count,
            broker_ref,
            handle_id: Uuid::new_v4().to_string(),
        }
    }

    /// Get the internal handle ID
    pub fn handle_id(&self) -> &str {
        &self.handle_id
    }

    /// Get a weak reference to the broker
    pub fn broker_ref(&self) -> &Weak<InMemoryBroker> {
        &self.broker_ref
    }
}

impl AckHandle for InMemoryAckHandle {
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

/// In-memory subscriber with acknowledgment support
pub struct InMemoryAckSubscriber {
    /// Reference to the broker
    broker: Arc<InMemoryBroker>,
    /// Current subscription state
    state: tokio::sync::Mutex<SubscriberState>,
}

struct SubscriberState {
    /// Message receiver channel
    receiver: Option<tokio::sync::mpsc::UnboundedReceiver<(Message, InMemoryAckHandle)>>,
    /// Currently subscribed topic
    subscribed_topic: Option<String>,
}

impl InMemoryAckSubscriber {
    /// Create a new in-memory acknowledgment subscriber
    pub fn new(broker: Arc<InMemoryBroker>) -> Self {
        Self {
            broker,
            state: tokio::sync::Mutex::new(SubscriberState {
                receiver: None,
                subscribed_topic: None,
            }),
        }
    }

    /// Get the broker reference
    pub fn broker(&self) -> &Arc<InMemoryBroker> {
        &self.broker
    }

    /// Check if currently subscribed to a topic
    pub async fn is_subscribed(&self) -> bool {
        let state = self.state.lock().await;
        state.subscribed_topic.is_some()
    }

    /// Get the currently subscribed topic
    pub async fn subscribed_topic(&self) -> Option<String> {
        let state = self.state.lock().await;
        state.subscribed_topic.clone()
    }
}

#[async_trait]
impl AckSubscriber for InMemoryAckSubscriber {
    type Error = InMemoryError;
    type AckHandle = InMemoryAckHandle;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        if self.broker.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }

        // Validate topic name
        if topic.is_empty() || topic.contains('\0') {
            return Err(InMemoryError::invalid_topic_name(topic));
        }

        // Create acknowledgment-aware receiver
        let receiver = self.broker.subscribe_with_ack(topic)?;

        let mut state = self.state.lock().await;
        state.receiver = Some(receiver);
        state.subscribed_topic = Some(topic.to_string());

        Ok(())
    }

    async fn receive_with_ack(&mut self) -> Result<(Message, Self::AckHandle), Self::Error> {
        if self.broker.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }

        // Take the receiver out temporarily to avoid holding the lock during await
        let mut receiver = {
            let mut state = self.state.lock().await;
            state.receiver.take()
        };

        if let Some(ref mut rx) = receiver {
            let result = match rx.recv().await {
                Some((message, handle)) => {
                    // Update statistics
                    if let Some(stats) = self.broker.stats() {
                        stats.increment_messages_consumed(1);
                    }
                    Ok((message, handle))
                }
                None => {
                    // Channel closed
                    if let Some(stats) = self.broker.stats() {
                        stats.increment_consume_errors();
                    }
                    Err(InMemoryError::channel_receive_error("Channel closed"))
                }
            };

            // Put the receiver back
            let mut state = self.state.lock().await;
            state.receiver = receiver;

            result
        } else {
            if let Some(stats) = self.broker.stats() {
                stats.increment_consume_errors();
            }
            Err(InMemoryError::NotSubscribed)
        }
    }

    async fn ack(&self, handle: Self::AckHandle) -> Result<(), Self::Error> {
        if let Some(broker) = handle.broker_ref.upgrade() {
            broker.ack_message(&handle).await
        } else {
            Err(InMemoryError::BrokerShutdown)
        }
    }

    async fn nack(&self, handle: Self::AckHandle, requeue: bool) -> Result<(), Self::Error> {
        if let Some(broker) = handle.broker_ref.upgrade() {
            broker.nack_message(&handle, requeue).await
        } else {
            Err(InMemoryError::BrokerShutdown)
        }
    }

    async fn ack_batch(&self, handles: Vec<Self::AckHandle>) -> Result<(), Self::Error> {
        // For now, use the default implementation that calls ack individually
        // TODO: Implement proper batch optimization later
        for handle in handles {
            self.ack(handle).await?;
        }
        Ok(())
    }

    async fn nack_batch(
        &self,
        handles: Vec<Self::AckHandle>,
        requeue: bool,
    ) -> Result<(), Self::Error> {
        // For now, use the default implementation that calls nack individually
        // TODO: Implement proper batch optimization later
        for handle in handles {
            self.nack(handle, requeue).await?;
        }
        Ok(())
    }
}

// Implement Drop to clean up subscription
impl Drop for InMemoryAckSubscriber {
    fn drop(&mut self) {
        if let Some(stats) = self.broker.stats() {
            stats.increment_subscribers_disconnected();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{InMemoryBroker, InMemoryConfig};
    use std::time::Duration;

    #[tokio::test]
    async fn test_ack_handle_creation() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let broker_ref = Arc::downgrade(&broker);

        let handle = InMemoryAckHandle::new(
            "msg-123".to_string(),
            "test-topic".to_string(),
            SystemTime::now(),
            1,
            broker_ref,
        );

        assert_eq!(handle.message_id(), "msg-123");
        assert_eq!(handle.topic(), "test-topic");
        assert_eq!(handle.delivery_count(), 1);
        assert!(!handle.is_retry());
    }

    #[tokio::test]
    async fn test_ack_subscriber_creation() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let subscriber = InMemoryAckSubscriber::new(broker.clone());

        assert!(!subscriber.is_subscribed().await);
        assert_eq!(subscriber.subscribed_topic().await, None);
        assert_eq!(Arc::as_ptr(subscriber.broker()), Arc::as_ptr(&broker));
    }

    #[tokio::test]
    async fn test_subscribe_validation() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let subscriber = InMemoryAckSubscriber::new(broker);

        // Test empty topic name
        let result = subscriber.subscribe("").await;
        assert!(matches!(
            result,
            Err(InMemoryError::InvalidTopicName { .. })
        ));

        // Test topic with null character
        let result = subscriber.subscribe("test\0topic").await;
        assert!(matches!(
            result,
            Err(InMemoryError::InvalidTopicName { .. })
        ));
    }
}
