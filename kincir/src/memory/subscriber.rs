use super::{InMemoryBroker, InMemoryError};
use crate::{Message, Subscriber};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;

/// Subscriber implementation for the in-memory message broker
#[derive(Debug)]
pub struct InMemorySubscriber {
    broker: Arc<InMemoryBroker>,
    state: Arc<Mutex<SubscriberState>>,
}

#[derive(Debug)]
struct SubscriberState {
    receiver: Option<mpsc::UnboundedReceiver<Message>>,
    subscribed_topic: Option<String>,
}

impl InMemorySubscriber {
    /// Create a new in-memory subscriber with the given broker
    pub fn new(broker: Arc<InMemoryBroker>) -> Self {
        Self {
            broker,
            state: Arc::new(Mutex::new(SubscriberState {
                receiver: None,
                subscribed_topic: None,
            })),
        }
    }

    /// Get reference to the underlying broker
    pub fn broker(&self) -> &Arc<InMemoryBroker> {
        &self.broker
    }

    /// Check if the subscriber is connected (broker not shutdown)
    pub fn is_connected(&self) -> bool {
        !self.broker.is_shutdown()
    }

    /// Check if the subscriber is subscribed to a topic
    pub fn is_subscribed(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.subscribed_topic.is_some() && state.receiver.is_some()
    }

    /// Get the currently subscribed topic
    pub fn subscribed_topic(&self) -> Option<String> {
        let state = self.state.lock().unwrap();
        state.subscribed_topic.clone()
    }

    /// Get broker statistics if enabled
    pub fn stats(&self) -> Option<super::StatsSnapshot> {
        self.broker
            .stats()
            .map(|stats| super::StatsSnapshot::from(stats.as_ref()))
    }

    /// Unsubscribe from the current topic
    pub fn unsubscribe(&self) {
        let mut state = self.state.lock().unwrap();
        state.receiver = None;
        state.subscribed_topic = None;

        if let Some(stats) = self.broker.stats() {
            stats.increment_subscribers_disconnected();
        }
    }

    /// Try to receive a message without blocking
    /// Returns None if no message is available
    pub fn try_receive(&self) -> Result<Option<Message>, InMemoryError> {
        if !self.is_connected() {
            return Err(InMemoryError::BrokerShutdown);
        }

        let mut state = self.state.lock().unwrap();
        if let Some(receiver) = &mut state.receiver {
            match receiver.try_recv() {
                Ok(message) => {
                    if let Some(stats) = self.broker.stats() {
                        stats.increment_messages_consumed(1);
                    }
                    Ok(Some(message))
                }
                Err(mpsc::error::TryRecvError::Empty) => Ok(None),
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    Err(InMemoryError::channel_receive_error("Channel disconnected"))
                }
            }
        } else {
            Err(InMemoryError::NotSubscribed)
        }
    }

    /// Receive multiple messages up to the specified limit
    /// Returns immediately with whatever messages are available
    pub fn try_receive_batch(&self, max_messages: usize) -> Result<Vec<Message>, InMemoryError> {
        if !self.is_connected() {
            return Err(InMemoryError::BrokerShutdown);
        }

        if max_messages == 0 {
            return Ok(Vec::new());
        }

        let mut messages = Vec::with_capacity(max_messages);

        for _ in 0..max_messages {
            match self.try_receive()? {
                Some(message) => messages.push(message),
                None => break,
            }
        }

        Ok(messages)
    }
}

#[async_trait]
impl Subscriber for InMemorySubscriber {
    type Error = InMemoryError;

    /// Subscribe to messages from the specified topic
    ///
    /// This method will:
    /// 1. Validate the topic name
    /// 2. Check broker shutdown status
    /// 3. Create topic if it doesn't exist (subject to limits)
    /// 4. Register subscriber with the topic
    /// 5. Set up message receiver channel
    /// 6. Update statistics (if enabled)
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name to subscribe to
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If subscription was successful
    /// * `Err(InMemoryError)` - If subscription failed
    ///
    /// # Errors
    ///
    /// * `BrokerShutdown` - If the broker has been shut down
    /// * `InvalidTopicName` - If the topic name is invalid
    /// * `MaxTopicsReached` - If topic limit would be exceeded
    /// * `SubscriberAlreadyExists` - If subscriber limit would be exceeded
    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        if !self.is_connected() {
            return Err(InMemoryError::BrokerShutdown);
        }

        // Unsubscribe from previous topic if any
        if self.is_subscribed() {
            self.unsubscribe();
        }

        // Subscribe to new topic through broker
        let receiver = self.broker.subscribe(topic)?;

        // Store subscription details
        let mut state = self.state.lock().unwrap();
        state.receiver = Some(receiver);
        state.subscribed_topic = Some(topic.to_string());

        Ok(())
    }

    /// Receive the next available message from the subscribed topic
    ///
    /// This method will block until a message is available or an error occurs.
    ///
    /// # Returns
    ///
    /// * `Ok(Message)` - The next message from the topic
    /// * `Err(InMemoryError)` - If receiving failed
    ///
    /// # Errors
    ///
    /// * `BrokerShutdown` - If the broker has been shut down
    /// * `NotSubscribed` - If not subscribed to any topic
    /// * `ChannelReceiveError` - If the message channel is disconnected
    async fn receive(&mut self) -> Result<Message, Self::Error> {
        let start_time = Instant::now();

        if !self.is_connected() {
            return Err(InMemoryError::BrokerShutdown);
        }

        // We need to take the receiver out temporarily to avoid holding the lock during await
        let mut receiver = {
            let mut state = self.state.lock().unwrap();
            state.receiver.take()
        };

        if let Some(ref mut rx) = receiver {
            let result = match rx.recv().await {
                Some(message) => {
                    // Update statistics
                    if let Some(stats) = self.broker.stats() {
                        stats.increment_messages_consumed(1);
                        stats.add_consume_time(start_time.elapsed());
                    }
                    Ok(message)
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
            let mut state = self.state.lock().unwrap();
            state.receiver = receiver;

            result
        } else {
            if let Some(stats) = self.broker.stats() {
                stats.increment_consume_errors();
            }
            Err(InMemoryError::NotSubscribed)
        }
    }
}

// Implement Drop to clean up subscription
impl Drop for InMemorySubscriber {
    fn drop(&mut self) {
        if self.is_subscribed() {
            if let Some(stats) = self.broker.stats() {
                stats.increment_subscribers_disconnected();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{InMemoryBroker, InMemoryConfig, InMemoryPublisher};
    use crate::Publisher;
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_subscriber_creation() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let subscriber = InMemorySubscriber::new(broker.clone());

        assert!(subscriber.is_connected());
        assert!(!subscriber.is_subscribed());
        assert_eq!(subscriber.subscribed_topic(), None);
        assert_eq!(Arc::ptr_eq(&subscriber.broker, &broker), true);
    }

    #[tokio::test]
    async fn test_basic_subscribe() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        let result = subscriber.subscribe("test-topic").await;
        assert!(result.is_ok());
        assert!(subscriber.is_subscribed());
        assert_eq!(
            subscriber.subscribed_topic(),
            Some("test-topic".to_string())
        );

        // Verify topic was created
        assert_eq!(broker.topic_count(), 1);
    }

    #[tokio::test]
    async fn test_subscribe_and_receive() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        // Subscribe first
        subscriber.subscribe("test-topic").await.unwrap();

        // Publish message
        let original_message = Message::new(b"Hello, World!".to_vec());
        publisher
            .publish("test-topic", vec![original_message.clone()])
            .await
            .unwrap();

        // Receive message
        let received_message = subscriber.receive().await.unwrap();
        assert_eq!(received_message.payload, original_message.payload);
        assert_eq!(received_message.uuid, original_message.uuid);
    }

    #[tokio::test]
    async fn test_receive_multiple_messages() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        subscriber.subscribe("test-topic").await.unwrap();

        // Publish multiple messages
        let messages = vec![
            Message::new(b"Message 1".to_vec()),
            Message::new(b"Message 2".to_vec()),
            Message::new(b"Message 3".to_vec()),
        ];
        publisher
            .publish("test-topic", messages.clone())
            .await
            .unwrap();

        // Receive all messages
        for (i, original) in messages.iter().enumerate() {
            let received = subscriber.receive().await.unwrap();
            assert_eq!(received.payload, original.payload);
            println!(
                "Received message {}: {:?}",
                i + 1,
                String::from_utf8_lossy(&received.payload)
            );
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber1 = InMemorySubscriber::new(broker.clone());
        let mut subscriber2 = InMemorySubscriber::new(broker.clone());

        // Both subscribe to same topic
        subscriber1.subscribe("broadcast-topic").await.unwrap();
        subscriber2.subscribe("broadcast-topic").await.unwrap();

        // Publish message
        let message = Message::new(b"Broadcast message".to_vec());
        publisher
            .publish("broadcast-topic", vec![message.clone()])
            .await
            .unwrap();

        // Both should receive the message
        let received1 = subscriber1.receive().await.unwrap();
        let received2 = subscriber2.receive().await.unwrap();

        assert_eq!(received1.payload, message.payload);
        assert_eq!(received2.payload, message.payload);
    }

    #[tokio::test]
    async fn test_try_receive() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        subscriber.subscribe("test-topic").await.unwrap();

        // No message available initially
        let result = subscriber.try_receive().unwrap();
        assert!(result.is_none());

        // Publish message
        let message = Message::new(b"Test message".to_vec());
        publisher
            .publish("test-topic", vec![message.clone()])
            .await
            .unwrap();

        // Message should be available now
        let received = subscriber.try_receive().unwrap().unwrap();
        assert_eq!(received.payload, message.payload);

        // No more messages
        let result = subscriber.try_receive().unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_try_receive_batch() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        subscriber.subscribe("test-topic").await.unwrap();

        // Publish multiple messages
        let messages = vec![
            Message::new(b"Message 1".to_vec()),
            Message::new(b"Message 2".to_vec()),
            Message::new(b"Message 3".to_vec()),
        ];
        publisher
            .publish("test-topic", messages.clone())
            .await
            .unwrap();

        // Receive batch
        let received_batch = subscriber.try_receive_batch(2).unwrap();
        assert_eq!(received_batch.len(), 2);
        assert_eq!(received_batch[0].payload, messages[0].payload);
        assert_eq!(received_batch[1].payload, messages[1].payload);

        // Receive remaining message
        let remaining = subscriber.try_receive_batch(5).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].payload, messages[2].payload);
    }

    #[tokio::test]
    async fn test_resubscribe() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        // Subscribe to first topic
        subscriber.subscribe("topic1").await.unwrap();
        assert_eq!(subscriber.subscribed_topic(), Some("topic1".to_string()));

        // Subscribe to second topic (should unsubscribe from first)
        subscriber.subscribe("topic2").await.unwrap();
        assert_eq!(subscriber.subscribed_topic(), Some("topic2".to_string()));

        assert_eq!(broker.topic_count(), 2);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        subscriber.subscribe("test-topic").await.unwrap();
        assert!(subscriber.is_subscribed());

        subscriber.unsubscribe();
        assert!(!subscriber.is_subscribed());
        assert_eq!(subscriber.subscribed_topic(), None);
    }

    #[tokio::test]
    async fn test_receive_not_subscribed() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let mut subscriber = InMemorySubscriber::new(broker);

        let result = subscriber.receive().await;
        assert!(matches!(result, Err(InMemoryError::NotSubscribed)));
    }

    #[tokio::test]
    async fn test_subscribe_invalid_topic_name() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let mut subscriber = InMemorySubscriber::new(broker);

        // Empty topic name
        let result = subscriber.subscribe("").await;
        assert!(matches!(
            result,
            Err(InMemoryError::InvalidTopicName { .. })
        ));

        // Topic name with null character
        let result = subscriber.subscribe("test\0topic").await;
        assert!(matches!(
            result,
            Err(InMemoryError::InvalidTopicName { .. })
        ));
    }

    #[tokio::test]
    async fn test_subscribe_after_shutdown() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        // Shutdown broker
        broker.shutdown().unwrap();
        assert!(!subscriber.is_connected());

        let result = subscriber.subscribe("test-topic").await;
        assert!(matches!(result, Err(InMemoryError::BrokerShutdown)));
    }

    #[tokio::test]
    async fn test_receive_after_shutdown() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        subscriber.subscribe("test-topic").await.unwrap();

        // Shutdown broker
        broker.shutdown().unwrap();

        let result = subscriber.receive().await;
        assert!(matches!(result, Err(InMemoryError::BrokerShutdown)));
    }

    #[tokio::test]
    async fn test_subscriber_with_statistics() {
        let config = InMemoryConfig::for_testing().with_stats(true);
        let broker = Arc::new(InMemoryBroker::new(config));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());

        subscriber.subscribe("test-topic").await.unwrap();

        let messages = vec![
            Message::new(b"Message 1".to_vec()),
            Message::new(b"Message 2".to_vec()),
        ];
        publisher.publish("test-topic", messages).await.unwrap();

        // Receive messages
        subscriber.receive().await.unwrap();
        subscriber.receive().await.unwrap();

        // Check statistics
        let stats_snapshot = subscriber.stats().unwrap();
        assert_eq!(stats_snapshot.messages_consumed, 2);
        assert_eq!(stats_snapshot.active_subscribers, 1);
    }

    #[tokio::test]
    async fn test_receive_timeout() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let mut subscriber = InMemorySubscriber::new(broker);

        subscriber.subscribe("test-topic").await.unwrap();

        // Try to receive with timeout (should timeout since no messages)
        let result = timeout(Duration::from_millis(100), subscriber.receive()).await;
        assert!(result.is_err()); // Timeout error
    }

    #[tokio::test]
    async fn test_concurrent_subscribers() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber1 = InMemorySubscriber::new(broker.clone());
        let mut subscriber2 = InMemorySubscriber::new(broker.clone());

        // Subscribe to different topics
        subscriber1.subscribe("topic1").await.unwrap();
        subscriber2.subscribe("topic2").await.unwrap();

        let message1 = vec![Message::new(b"Message for topic1".to_vec())];
        let message2 = vec![Message::new(b"Message for topic2".to_vec())];

        // Publish to both topics
        publisher.publish("topic1", message1.clone()).await.unwrap();
        publisher.publish("topic2", message2.clone()).await.unwrap();

        // Receive concurrently
        let (received1, received2) = tokio::join!(subscriber1.receive(), subscriber2.receive());

        assert!(received1.is_ok());
        assert!(received2.is_ok());
        assert_eq!(received1.unwrap().payload, message1[0].payload);
        assert_eq!(received2.unwrap().payload, message2[0].payload);
    }
}
