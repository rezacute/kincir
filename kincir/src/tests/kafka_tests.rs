use crate::kafka::{KafkaPublisher, KafkaSubscriber, KafkaError};
use crate::Message;
use tokio;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kafka_publisher_creation() {
        let result = KafkaPublisher::new("invalid:9092").await;
        assert!(result.is_err());
        match result {
            Err(KafkaError::Connection(_)) => assert!(true),
            _ => assert!(false, "Expected Kafka connection error")
        }
    }

    #[tokio::test]
    async fn test_kafka_subscriber_creation() {
        let result = KafkaSubscriber::new("invalid:9092", "test-group").await;
        assert!(result.is_err());
        match result {
            Err(KafkaError::Connection(_)) => assert!(true),
            _ => assert!(false, "Expected Kafka connection error")
        }
    }

    #[tokio::test]
    async fn test_kafka_message_serialization() {
        let msg = Message::new(b"test data".to_vec())
            .with_metadata("test_key", "test_value");
        
        // Test serialization
        let serialized = serde_json::to_vec(&msg).unwrap();
        let deserialized: Message = serde_json::from_slice(&serialized).unwrap();
        
        assert_eq!(deserialized.payload, msg.payload);
        assert_eq!(deserialized.metadata, msg.metadata);
        assert_eq!(deserialized.uuid, msg.uuid);
    }

    // Integration test - requires running Kafka instance
    #[tokio::test]
    #[ignore]
    async fn test_kafka_publish_subscribe() {
        let publisher = KafkaPublisher::new("localhost:9092").await.unwrap();
        // Changed to mut subscriber as receive() now takes &mut self
        let mut subscriber = KafkaSubscriber::new("localhost:9092", "test-group").await.unwrap();
        
        let test_topic = "test-topic";
        let test_message = Message::new(b"integration test".to_vec())
            .with_metadata("test", "true");
        
        // Subscribe first
        subscriber.subscribe(test_topic).await.unwrap();
        
        // Publish message
        publisher.publish(test_topic, vec![test_message.clone()]).await.unwrap();
        
        // Receive and verify
        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.payload, test_message.payload);
        assert_eq!(received.metadata, test_message.metadata);
    }

    #[tokio::test]
    async fn test_kafka_batch_publish() {
        let publisher = KafkaPublisher::new("localhost:9092").await.unwrap();
        let messages = vec![
            Message::new(b"batch1".to_vec()).with_metadata("batch", "1"),
            Message::new(b"batch2".to_vec()).with_metadata("batch", "2"),
        ];

        let result = publisher.publish("test-topic", messages).await;
        assert!(result.is_err()); // Should fail due to no connection
    }
}