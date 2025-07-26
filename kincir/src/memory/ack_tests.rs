//! Tests for acknowledgment functionality

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::ack::{AckConfig, AckMode, AckSubscriber};
    use crate::memory::{InMemoryAckSubscriber, InMemoryBroker, InMemoryConfig};
    use crate::{Message, Publisher};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_ack_config_creation() {
        let config = AckConfig::new()
            .with_mode(AckMode::Manual)
            .with_timeout(Duration::from_secs(30))
            .with_max_retries(3);

        assert_eq!(config.mode, AckMode::Manual);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_ack_config_presets() {
        let auto_config = AckConfig::auto();
        assert_eq!(auto_config.mode, AckMode::Auto);

        let manual_config = AckConfig::manual();
        assert_eq!(manual_config.mode, AckMode::Manual);

        let client_auto_config = AckConfig::client_auto();
        assert_eq!(client_auto_config.mode, AckMode::ClientAuto);
    }

    #[tokio::test]
    async fn test_in_memory_ack_subscriber_creation() {
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

    #[tokio::test]
    async fn test_basic_ack_operations() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));

        // Test that ack methods can be called without errors
        let handle = crate::memory::ack::InMemoryAckHandle::new(
            "test-msg".to_string(),
            "test-topic".to_string(),
            std::time::SystemTime::now(),
            1,
            Arc::downgrade(&broker),
        );

        // Test ack
        let result = broker.ack_message(&handle).await;
        assert!(result.is_ok());

        // Test nack with requeue
        let result = broker.nack_message(&handle, true).await;
        assert!(result.is_ok());

        // Test nack without requeue
        let result = broker.nack_message(&handle, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_ack_operations() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));

        let handles = vec![
            crate::memory::ack::InMemoryAckHandle::new(
                "test-msg-1".to_string(),
                "test-topic".to_string(),
                std::time::SystemTime::now(),
                1,
                Arc::downgrade(&broker),
            ),
            crate::memory::ack::InMemoryAckHandle::new(
                "test-msg-2".to_string(),
                "test-topic".to_string(),
                std::time::SystemTime::now(),
                1,
                Arc::downgrade(&broker),
            ),
        ];

        // Test batch ack
        let result = broker.ack_batch(&handles).await;
        assert!(result.is_ok());

        // Test batch nack
        let result = broker.nack_batch(&handles, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ack_with_shutdown_broker() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));

        let handle = crate::memory::ack::InMemoryAckHandle::new(
            "test-msg".to_string(),
            "test-topic".to_string(),
            std::time::SystemTime::now(),
            1,
            Arc::downgrade(&broker),
        );

        // Shutdown the broker
        broker.shutdown().unwrap();

        // Operations should fail after shutdown
        let result = broker.ack_message(&handle).await;
        assert!(matches!(result, Err(InMemoryError::BrokerShutdown)));

        let result = broker.nack_message(&handle, true).await;
        assert!(matches!(result, Err(InMemoryError::BrokerShutdown)));
    }
}
