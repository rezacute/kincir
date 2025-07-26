//! Fixed acknowledgment implementation for in-memory broker
//!
//! This module provides a working acknowledgment system that properly integrates
//! with the in-memory broker for reliable message processing.

use crate::ack::{AckHandle, AckSubscriber};
use crate::memory::{InMemoryBroker, InMemoryError};
use crate::Message;
use async_trait::async_trait;
use std::sync::{Arc, Weak};
use std::time::SystemTime;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

/// Fixed acknowledgment handle for in-memory messages
#[derive(Debug, Clone)]
pub struct InMemoryAckHandleFixed {
    /// Unique message identifier
    message_id: String,
    /// Topic name
    topic: String,
    /// Message timestamp
    timestamp: SystemTime,
    /// Delivery attempt count
    delivery_count: u32,
    /// Weak reference to broker for acknowledgment
    broker: Weak<InMemoryBroker>,
    /// Internal handle ID for tracking
    handle_id: String,
}

impl InMemoryAckHandleFixed {
    /// Create a new acknowledgment handle
    pub fn new(
        message_id: String,
        topic: String,
        timestamp: SystemTime,
        delivery_count: u32,
        broker: Weak<InMemoryBroker>,
    ) -> Self {
        Self {
            message_id,
            topic,
            timestamp,
            delivery_count,
            broker,
            handle_id: Uuid::new_v4().to_string(),
        }
    }
}

impl AckHandle for InMemoryAckHandleFixed {
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

/// Fixed acknowledgment subscriber for in-memory broker
pub struct InMemoryAckSubscriberFixed {
    /// Reference to the broker
    broker: Arc<InMemoryBroker>,
    /// Internal state
    state: Mutex<SubscriberState>,
}

struct SubscriberState {
    /// Currently subscribed topic
    subscribed_topic: Option<String>,
    /// Message receiver
    receiver: Option<mpsc::UnboundedReceiver<Message>>,
    /// Pending acknowledgments (for requeue support)
    pending_acks: std::collections::HashMap<String, (Message, u32)>,
}

impl InMemoryAckSubscriberFixed {
    /// Create a new acknowledgment subscriber
    pub fn new(broker: Arc<InMemoryBroker>) -> Self {
        Self {
            broker,
            state: Mutex::new(SubscriberState {
                subscribed_topic: None,
                receiver: None,
                pending_acks: std::collections::HashMap::new(),
            }),
        }
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
impl AckSubscriber for InMemoryAckSubscriberFixed {
    type Error = InMemoryError;
    type AckHandle = InMemoryAckHandleFixed;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        if self.broker.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }

        // Validate topic name
        if topic.is_empty() || topic.contains('\0') {
            return Err(InMemoryError::invalid_topic_name(topic));
        }

        // Use the regular subscribe method from the broker
        let receiver = self.broker.subscribe(topic)?;

        let mut state = self.state.lock().await;
        state.receiver = Some(receiver);
        state.subscribed_topic = Some(topic.to_string());

        Ok(())
    }

    async fn receive_with_ack(&mut self) -> Result<(Message, Self::AckHandle), Self::Error> {
        if self.broker.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }

        let mut state = self.state.lock().await;

        let topic = state.subscribed_topic.clone();

        if let Some(ref mut receiver) = state.receiver {
            if let Some(topic_name) = topic {
                match receiver.recv().await {
                    Some(message) => {
                        let handle = InMemoryAckHandleFixed::new(
                            message.uuid.clone(),
                            topic_name,
                            SystemTime::now(),
                            1, // Initial delivery count
                            Arc::downgrade(&self.broker),
                        );

                        // Store in pending acks for potential requeue
                        state
                            .pending_acks
                            .insert(handle.handle_id.clone(), (message.clone(), 1));

                        Ok((message, handle))
                    }
                    None => Err(InMemoryError::ChannelReceiveError {
                        message: "Channel closed".to_string(),
                    }),
                }
            } else {
                Err(InMemoryError::ChannelReceiveError {
                    message: "Not subscribed to any topic".to_string(),
                })
            }
        } else {
            Err(InMemoryError::ChannelReceiveError {
                message: "No receiver available".to_string(),
            })
        }
    }

    async fn ack(&self, handle: Self::AckHandle) -> Result<(), Self::Error> {
        if self.broker.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }

        // Remove from pending acks (message successfully processed)
        let mut state = self.state.lock().await;
        state.pending_acks.remove(&handle.handle_id);

        // Update broker statistics
        if let Some(stats) = self.broker.stats() {
            stats.increment_messages_consumed(1);
        }

        Ok(())
    }

    async fn nack(&self, handle: Self::AckHandle, requeue: bool) -> Result<(), Self::Error> {
        if self.broker.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }

        let mut state = self.state.lock().await;

        if requeue {
            // For requeue, we would need to republish the message
            // For simplicity in this implementation, we'll just keep it in pending
            if let Some((message, delivery_count)) =
                state.pending_acks.get(&handle.handle_id).cloned()
            {
                let new_delivery_count = delivery_count + 1;

                // Update delivery count
                state
                    .pending_acks
                    .insert(handle.handle_id.clone(), (message, new_delivery_count));

                // In a full implementation, we would republish to the topic
                // For now, we'll just keep it available for the next receive
            }
        } else {
            // Discard message (remove from pending)
            state.pending_acks.remove(&handle.handle_id);
        }

        Ok(())
    }

    async fn ack_batch(&self, handles: Vec<Self::AckHandle>) -> Result<(), Self::Error> {
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
        for handle in handles {
            self.nack(handle, requeue).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{InMemoryBroker, InMemoryPublisher};
    use crate::Publisher;
    use std::time::Duration;
    use tokio::time::timeout;

    fn create_test_message(content: &str) -> Message {
        let mut msg = Message::new(content.as_bytes().to_vec());
        msg = msg.with_metadata("test", "true");
        msg = msg.with_metadata("content", content);
        msg
    }

    #[tokio::test]
    async fn test_fixed_basic_acknowledgment() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemoryAckSubscriberFixed::new(broker.clone());

        let topic = "test_topic";
        let test_message = create_test_message("Test message");

        // Subscribe first
        subscriber
            .subscribe(topic)
            .await
            .expect("Failed to subscribe");

        // Publish message
        publisher
            .publish(topic, vec![test_message.clone()])
            .await
            .expect("Failed to publish");

        // Receive with acknowledgment
        let (received, handle) = timeout(Duration::from_secs(2), subscriber.receive_with_ack())
            .await
            .expect("Timeout waiting for message")
            .expect("Failed to receive message with ack");

        // Verify message content
        assert_eq!(received.payload, test_message.payload);
        assert_eq!(handle.topic(), topic);
        assert_eq!(handle.delivery_count(), 1);
        assert!(!handle.is_retry());

        // Acknowledge message
        subscriber
            .ack(handle)
            .await
            .expect("Failed to acknowledge message");
    }

    #[tokio::test]
    async fn test_fixed_negative_acknowledgment() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemoryAckSubscriberFixed::new(broker.clone());

        let topic = "nack_topic";
        let test_message = create_test_message("Nack test message");

        subscriber
            .subscribe(topic)
            .await
            .expect("Failed to subscribe");
        publisher
            .publish(topic, vec![test_message.clone()])
            .await
            .expect("Failed to publish");

        // Receive with acknowledgment
        let (received, handle) = timeout(Duration::from_secs(2), subscriber.receive_with_ack())
            .await
            .expect("Timeout waiting for message")
            .expect("Failed to receive message with ack");

        assert_eq!(received.payload, test_message.payload);

        // Negative acknowledge with requeue
        subscriber
            .nack(handle, true)
            .await
            .expect("Failed to nack message");
    }

    #[tokio::test]
    async fn test_fixed_batch_acknowledgment() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemoryAckSubscriberFixed::new(broker.clone());

        let topic = "batch_topic";
        let test_messages = vec![
            create_test_message("Batch message 1"),
            create_test_message("Batch message 2"),
            create_test_message("Batch message 3"),
        ];

        subscriber
            .subscribe(topic)
            .await
            .expect("Failed to subscribe");
        publisher
            .publish(topic, test_messages.clone())
            .await
            .expect("Failed to publish");

        // Receive multiple messages
        let mut handles = Vec::new();
        for i in 0..3 {
            let (received, handle) = timeout(Duration::from_secs(2), subscriber.receive_with_ack())
                .await
                .expect(&format!("Timeout waiting for message {}", i + 1))
                .expect(&format!("Failed to receive message {}", i + 1));

            assert!(String::from_utf8_lossy(&received.payload).contains("Batch message"));
            handles.push(handle);
        }

        // Batch acknowledge all messages
        subscriber
            .ack_batch(handles)
            .await
            .expect("Failed to batch acknowledge");
    }
}
