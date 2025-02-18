use crate::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber, RabbitMQError};
use crate::Message;
use tokio;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rabbitmq_publisher_creation() {
        let result = RabbitMQPublisher::new("amqp://non-existent:5672").await;
        assert!(result.is_err());
        match result {
            Err(RabbitMQError::RabbitMQ(_)) => assert!(true),
            _ => assert!(false, "Expected RabbitMQ connection error")
        }
    }

    #[tokio::test]
    async fn test_rabbitmq_subscriber_creation() {
        let result = RabbitMQSubscriber::new("amqp://non-existent:5672").await;
        assert!(result.is_err());
        match result {
            Err(RabbitMQError::RabbitMQ(_)) => assert!(true),
            _ => assert!(false, "Expected RabbitMQ connection error")
        }
    }

    #[tokio::test]
    async fn test_rabbitmq_message_serialization() {
        let msg = Message::new(b"test data".to_vec())
            .with_metadata("test_key", "test_value");
        
        // Test serialization
        let serialized = serde_json::to_vec(&msg).unwrap();
        let deserialized: Message = serde_json::from_slice(&serialized).unwrap();
        
        assert_eq!(deserialized.payload, msg.payload);
        assert_eq!(deserialized.metadata, msg.metadata);
        assert_eq!(deserialized.uuid, msg.uuid);
    }

    // Integration test - requires running RabbitMQ instance
    #[tokio::test]
    #[ignore]
    async fn test_rabbitmq_publish_subscribe() {
        let publisher = RabbitMQPublisher::new("amqp://localhost:5672").await.unwrap();
        let subscriber = RabbitMQSubscriber::new("amqp://localhost:5672").await.unwrap();
        
        let test_queue = "test-queue";
        let test_message = Message::new(b"integration test".to_vec())
            .with_metadata("test", "true");
        
        // Subscribe first
        subscriber.subscribe(test_queue).await.unwrap();
        
        // Publish message
        publisher.publish(test_queue, vec![test_message.clone()]).await.unwrap();
        
        // Receive and verify
        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.payload, test_message.payload);
        assert_eq!(received.metadata, test_message.metadata);
    }
}