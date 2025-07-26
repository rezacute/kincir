use super::{InMemoryBroker, InMemoryError};
use crate::{Message, Publisher};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

/// Publisher implementation for the in-memory message broker
#[derive(Debug, Clone)]
pub struct InMemoryPublisher {
    broker: Arc<InMemoryBroker>,
}

impl InMemoryPublisher {
    /// Create a new in-memory publisher with the given broker
    pub fn new(broker: Arc<InMemoryBroker>) -> Self {
        Self { broker }
    }

    /// Get reference to the underlying broker
    pub fn broker(&self) -> &Arc<InMemoryBroker> {
        &self.broker
    }

    /// Check if the publisher is connected (broker not shutdown)
    pub fn is_connected(&self) -> bool {
        !self.broker.is_shutdown()
    }

    /// Get broker statistics if enabled
    pub fn stats(&self) -> Option<super::StatsSnapshot> {
        self.broker
            .stats()
            .map(|stats| super::StatsSnapshot::from(stats.as_ref()))
    }
}

#[async_trait]
impl Publisher for InMemoryPublisher {
    type Error = InMemoryError;

    /// Publish messages to the specified topic
    ///
    /// This method will:
    /// 1. Validate the topic name
    /// 2. Check broker shutdown status
    /// 3. Create topic if it doesn't exist (subject to limits)
    /// 4. Add messages to topic queue (if persistence enabled)
    /// 5. Broadcast messages to all subscribers
    /// 6. Update statistics (if enabled)
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name to publish to
    /// * `messages` - Vector of messages to publish
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If all messages were published successfully
    /// * `Err(InMemoryError)` - If publishing failed
    ///
    /// # Errors
    ///
    /// * `BrokerShutdown` - If the broker has been shut down
    /// * `InvalidTopicName` - If the topic name is invalid
    /// * `MaxTopicsReached` - If topic limit would be exceeded
    /// * `QueueFull` - If the topic queue is full
    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        let start_time = Instant::now();

        // Validate input
        if messages.is_empty() {
            return Ok(()); // Nothing to publish
        }

        // Delegate to broker's publish method
        let result = self.broker.publish(topic, messages);

        // Update error statistics if publishing failed
        if result.is_err() {
            if let Some(stats) = self.broker.stats() {
                stats.increment_publish_errors();
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{InMemoryBroker, InMemoryConfig};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_publisher_creation() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());

        assert!(publisher.is_connected());
        assert_eq!(Arc::ptr_eq(&publisher.broker, &broker), true);
    }

    #[tokio::test]
    async fn test_basic_publish() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());

        let messages = vec![
            Message::new(b"message 1".to_vec()),
            Message::new(b"message 2".to_vec()),
        ];

        let result = publisher.publish("test-topic", messages).await;
        assert!(result.is_ok());

        // Verify topic was created
        assert_eq!(broker.topic_count(), 1);
        let topics = broker.list_topics();
        assert!(topics.contains(&"test-topic".to_string()));
    }

    #[tokio::test]
    async fn test_publish_empty_messages() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker);

        let result = publisher.publish("test-topic", vec![]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_to_multiple_topics() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());

        let message1 = vec![Message::new(b"topic1 message".to_vec())];
        let message2 = vec![Message::new(b"topic2 message".to_vec())];

        publisher.publish("topic1", message1).await.unwrap();
        publisher.publish("topic2", message2).await.unwrap();

        assert_eq!(broker.topic_count(), 2);
        let topics = broker.list_topics();
        assert!(topics.contains(&"topic1".to_string()));
        assert!(topics.contains(&"topic2".to_string()));
    }

    #[tokio::test]
    async fn test_publish_with_topic_limit() {
        let config = InMemoryConfig::for_testing().with_max_topics(Some(1));
        let broker = Arc::new(InMemoryBroker::new(config));
        let publisher = InMemoryPublisher::new(broker);

        let message = vec![Message::new(b"test".to_vec())];

        // First topic should succeed
        publisher.publish("topic1", message.clone()).await.unwrap();

        // Second topic should fail due to limit
        let result = publisher.publish("topic2", message).await;
        assert!(matches!(
            result,
            Err(InMemoryError::MaxTopicsReached { .. })
        ));
    }

    #[tokio::test]
    async fn test_publish_with_queue_limit() {
        let config = InMemoryConfig::for_testing().with_max_queue_size(Some(1));
        let broker = Arc::new(InMemoryBroker::new(config));
        let publisher = InMemoryPublisher::new(broker);

        let message1 = vec![Message::new(b"message1".to_vec())];
        let message2 = vec![Message::new(b"message2".to_vec())];

        // First message should succeed
        publisher.publish("test-topic", message1).await.unwrap();

        // Second message should fail due to queue limit
        let result = publisher.publish("test-topic", message2).await;
        assert!(matches!(result, Err(InMemoryError::QueueFull { .. })));
    }

    #[tokio::test]
    async fn test_publish_invalid_topic_name() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker);

        let message = vec![Message::new(b"test".to_vec())];

        // Empty topic name
        let result = publisher.publish("", message.clone()).await;
        assert!(matches!(
            result,
            Err(InMemoryError::InvalidTopicName { .. })
        ));

        // Topic name with null character
        let result = publisher.publish("test\0topic", message).await;
        assert!(matches!(
            result,
            Err(InMemoryError::InvalidTopicName { .. })
        ));
    }

    #[tokio::test]
    async fn test_publish_after_shutdown() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());

        // Shutdown broker
        broker.shutdown().unwrap();
        assert!(!publisher.is_connected());

        let message = vec![Message::new(b"test".to_vec())];
        let result = publisher.publish("test-topic", message).await;
        assert!(matches!(result, Err(InMemoryError::BrokerShutdown)));
    }

    #[tokio::test]
    async fn test_publisher_with_statistics() {
        let config = InMemoryConfig::for_testing().with_stats(true);
        let broker = Arc::new(InMemoryBroker::new(config));
        let publisher = InMemoryPublisher::new(broker.clone());

        let messages = vec![
            Message::new(b"message 1".to_vec()),
            Message::new(b"message 2".to_vec()),
        ];

        publisher.publish("test-topic", messages).await.unwrap();

        // Check statistics
        let stats_snapshot = publisher.stats().unwrap();
        assert_eq!(stats_snapshot.messages_published, 2);
        assert_eq!(stats_snapshot.active_topics, 1);
    }

    #[tokio::test]
    async fn test_concurrent_publishers() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher1 = InMemoryPublisher::new(broker.clone());
        let publisher2 = InMemoryPublisher::new(broker.clone());

        let messages1 = vec![Message::new(b"from publisher 1".to_vec())];
        let messages2 = vec![Message::new(b"from publisher 2".to_vec())];

        // Publish concurrently
        let (result1, result2) = tokio::join!(
            publisher1.publish("topic1", messages1),
            publisher2.publish("topic2", messages2)
        );

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(broker.topic_count(), 2);
    }

    #[tokio::test]
    async fn test_publisher_clone() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher1 = InMemoryPublisher::new(broker.clone());
        let publisher2 = publisher1.clone();

        let message = vec![Message::new(b"test".to_vec())];

        // Both publishers should work
        publisher1.publish("topic1", message.clone()).await.unwrap();
        publisher2.publish("topic2", message).await.unwrap();

        assert_eq!(broker.topic_count(), 2);
    }
}
