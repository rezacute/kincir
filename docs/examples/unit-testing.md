# Unit Testing with Kincir

This guide demonstrates best practices for unit testing Kincir-based applications, including mocking, test fixtures, and integration testing strategies.

## Table of Contents

- [Basic Testing Setup](#basic-testing-setup)
- [Testing Publishers](#testing-publishers)
- [Testing Subscribers](#testing-subscribers)
- [Mocking External Dependencies](#mocking-external-dependencies)
- [Integration Testing](#integration-testing)
- [Test Utilities](#test-utilities)

## Basic Testing Setup

### Test Dependencies

Add these dependencies to your `Cargo.toml`:

```toml
[dev-dependencies]
tokio-test = "0.4"
mockall = "0.11"
assert_matches = "1.5"
```

### Simple Publisher Test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher};
    use kincir::{Publisher, Message};
    use std::sync::Arc;
    use tokio_test;

    #[tokio::test]
    async fn test_publish_message() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker);
        
        let message = Message::new(b"test message".to_vec());
        let result = publisher.publish("test-topic", vec![message]).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_multiple_messages() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker);
        
        let messages = vec![
            Message::new(b"message 1".to_vec()),
            Message::new(b"message 2".to_vec()),
            Message::new(b"message 3".to_vec()),
        ];
        
        let result = publisher.publish("test-topic", messages).await;
        assert!(result.is_ok());
    }
}
```

## Testing Publishers

### Publisher with Error Handling

```rust
use kincir::{Publisher, Message};
use std::sync::Arc;

pub struct OrderPublisher {
    publisher: Box<dyn Publisher<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
}

impl OrderPublisher {
    pub fn new(publisher: Box<dyn Publisher<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>) -> Self {
        Self { publisher }
    }
    
    pub async fn publish_order(&self, order_id: &str, order_data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = Message::new(order_data.to_vec())
            .with_metadata("order_id", order_id)
            .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339());
        
        self.publisher.publish("orders", vec![message]).await
    }
}

#[cfg(test)]
mod publisher_tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher};
    use mockall::{predicate::*, mock};

    mock! {
        TestPublisher {}
        
        #[async_trait::async_trait]
        impl Publisher for TestPublisher {
            type Error = Box<dyn std::error::Error + Send + Sync>;
            
            async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error>;
        }
    }

    #[tokio::test]
    async fn test_order_publisher_success() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = Box::new(InMemoryPublisher::new(broker));
        let order_publisher = OrderPublisher::new(publisher);
        
        let result = order_publisher.publish_order("ORDER-123", b"order data").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_order_publisher_with_mock() {
        let mut mock_publisher = MockTestPublisher::new();
        
        mock_publisher
            .expect_publish()
            .with(eq("orders"), function(|messages: &Vec<Message>| {
                messages.len() == 1 && 
                messages[0].metadata.get("order_id") == Some(&"ORDER-123".to_string())
            }))
            .times(1)
            .returning(|_, _| Ok(()));
        
        let order_publisher = OrderPublisher::new(Box::new(mock_publisher));
        let result = order_publisher.publish_order("ORDER-123", b"order data").await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_order_publisher_error_handling() {
        let mut mock_publisher = MockTestPublisher::new();
        
        mock_publisher
            .expect_publish()
            .times(1)
            .returning(|_, _| Err("Publisher error".into()));
        
        let order_publisher = OrderPublisher::new(Box::new(mock_publisher));
        let result = order_publisher.publish_order("ORDER-123", b"order data").await;
        
        assert!(result.is_err());
    }
}
```

## Testing Subscribers

### Subscriber Message Processing

```rust
use kincir::{Subscriber, Message};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OrderEvent {
    pub order_id: String,
    pub amount: f64,
    pub status: String,
}

pub struct OrderProcessor {
    subscriber: Box<dyn Subscriber<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
}

impl OrderProcessor {
    pub fn new(subscriber: Box<dyn Subscriber<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>) -> Self {
        Self { subscriber }
    }
    
    pub async fn process_order(&self, message: Message) -> Result<OrderEvent, Box<dyn std::error::Error + Send + Sync>> {
        let order_event: OrderEvent = serde_json::from_slice(&message.payload)?;
        
        // Validate order
        if order_event.amount <= 0.0 {
            return Err("Invalid order amount".into());
        }
        
        // Process order logic here
        println!("Processing order: {}", order_event.order_id);
        
        Ok(order_event)
    }
}

#[cfg(test)]
mod subscriber_tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemorySubscriber};
    use assert_matches::assert_matches;

    #[tokio::test]
    async fn test_process_valid_order() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let subscriber = Box::new(InMemorySubscriber::new(broker));
        let processor = OrderProcessor::new(subscriber);
        
        let order = OrderEvent {
            order_id: "ORDER-123".to_string(),
            amount: 99.99,
            status: "pending".to_string(),
        };
        
        let message = Message::new(serde_json::to_vec(&order).unwrap());
        let result = processor.process_order(message).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), order);
    }

    #[tokio::test]
    async fn test_process_invalid_order_amount() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let subscriber = Box::new(InMemorySubscriber::new(broker));
        let processor = OrderProcessor::new(subscriber);
        
        let order = OrderEvent {
            order_id: "ORDER-123".to_string(),
            amount: -10.0, // Invalid amount
            status: "pending".to_string(),
        };
        
        let message = Message::new(serde_json::to_vec(&order).unwrap());
        let result = processor.process_order(message).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid order amount"));
    }

    #[tokio::test]
    async fn test_process_malformed_message() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let subscriber = Box::new(InMemorySubscriber::new(broker));
        let processor = OrderProcessor::new(subscriber);
        
        let message = Message::new(b"invalid json".to_vec());
        let result = processor.process_order(message).await;
        
        assert!(result.is_err());
    }
}
```

## Mocking External Dependencies

### Mock RabbitMQ Publisher

```rust
use mockall::{predicate::*, mock};
use async_trait::async_trait;

mock! {
    RabbitMQPublisher {}
    
    #[async_trait]
    impl Publisher for RabbitMQPublisher {
        type Error = Box<dyn std::error::Error + Send + Sync>;
        
        async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error>;
    }
}

pub struct MessageService {
    publisher: Box<dyn Publisher<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
}

impl MessageService {
    pub fn new(publisher: Box<dyn Publisher<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>) -> Self {
        Self { publisher }
    }
    
    pub async fn send_notification(&self, user_id: &str, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let notification = Message::new(message.as_bytes().to_vec())
            .with_metadata("user_id", user_id)
            .with_metadata("type", "notification");
        
        self.publisher.publish("notifications", vec![notification]).await
    }
}

#[cfg(test)]
mod service_tests {
    use super::*;

    #[tokio::test]
    async fn test_send_notification() {
        let mut mock_publisher = MockRabbitMQPublisher::new();
        
        mock_publisher
            .expect_publish()
            .with(
                eq("notifications"),
                function(|messages: &Vec<Message>| {
                    messages.len() == 1 &&
                    messages[0].metadata.get("user_id") == Some(&"user123".to_string()) &&
                    messages[0].metadata.get("type") == Some(&"notification".to_string())
                })
            )
            .times(1)
            .returning(|_, _| Ok(()));
        
        let service = MessageService::new(Box::new(mock_publisher));
        let result = service.send_notification("user123", "Hello, World!").await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_notification_failure() {
        let mut mock_publisher = MockRabbitMQPublisher::new();
        
        mock_publisher
            .expect_publish()
            .times(1)
            .returning(|_, _| Err("Connection failed".into()));
        
        let service = MessageService::new(Box::new(mock_publisher));
        let result = service.send_notification("user123", "Hello, World!").await;
        
        assert!(result.is_err());
    }
}
```

## Integration Testing

### End-to-End Message Flow Test

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
    use kincir::{Publisher, Subscriber};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_end_to_end_message_flow() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker);
        
        // Subscribe to topic
        subscriber.subscribe("integration-test").await.unwrap();
        
        // Publish message
        let test_message = Message::new(b"integration test message".to_vec())
            .with_metadata("test_id", "test-001");
        
        publisher.publish("integration-test", vec![test_message.clone()]).await.unwrap();
        
        // Receive and verify message
        let received_message = timeout(Duration::from_secs(1), subscriber.receive())
            .await
            .expect("Timeout waiting for message")
            .expect("Failed to receive message");
        
        assert_eq!(received_message.payload, test_message.payload);
        assert_eq!(received_message.metadata.get("test_id"), Some(&"test-001".to_string()));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        
        let mut subscriber1 = InMemorySubscriber::new(broker.clone());
        let mut subscriber2 = InMemorySubscriber::new(broker.clone());
        
        // Both subscribers listen to the same topic
        subscriber1.subscribe("broadcast").await.unwrap();
        subscriber2.subscribe("broadcast").await.unwrap();
        
        // Publish message
        let message = Message::new(b"broadcast message".to_vec());
        publisher.publish("broadcast", vec![message]).await.unwrap();
        
        // Both subscribers should receive the message
        let msg1 = timeout(Duration::from_secs(1), subscriber1.receive()).await.unwrap().unwrap();
        let msg2 = timeout(Duration::from_secs(1), subscriber2.receive()).await.unwrap().unwrap();
        
        assert_eq!(msg1.payload, b"broadcast message");
        assert_eq!(msg2.payload, b"broadcast message");
    }
}
```

## Test Utilities

### Test Helper Functions

```rust
pub mod test_utils {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
    use std::sync::Arc;

    pub fn create_test_broker() -> Arc<InMemoryBroker> {
        Arc::new(InMemoryBroker::with_default_config())
    }

    pub fn create_test_message(payload: &str) -> Message {
        Message::new(payload.as_bytes().to_vec())
            .with_metadata("test", "true")
            .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339())
    }

    pub async fn publish_and_receive(
        topic: &str,
        message: Message,
    ) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
        let broker = create_test_broker();
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker);
        
        subscriber.subscribe(topic).await?;
        publisher.publish(topic, vec![message]).await?;
        
        let received = subscriber.receive().await?;
        Ok(received)
    }

    pub async fn assert_message_received(
        subscriber: &mut InMemorySubscriber,
        expected_payload: &[u8],
        timeout_secs: u64,
    ) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
        let message = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            subscriber.receive()
        ).await??;
        
        assert_eq!(message.payload, expected_payload);
        Ok(message)
    }
}

#[cfg(test)]
mod utility_tests {
    use super::test_utils::*;

    #[tokio::test]
    async fn test_publish_and_receive_utility() {
        let message = create_test_message("test payload");
        let received = publish_and_receive("test-topic", message.clone()).await.unwrap();
        
        assert_eq!(received.payload, message.payload);
        assert_eq!(received.metadata.get("test"), Some(&"true".to_string()));
    }

    #[tokio::test]
    async fn test_assert_message_received_utility() {
        let broker = create_test_broker();
        let publisher = kincir::memory::InMemoryPublisher::new(broker.clone());
        let mut subscriber = kincir::memory::InMemorySubscriber::new(broker);
        
        subscriber.subscribe("test-topic").await.unwrap();
        
        let message = create_test_message("utility test");
        publisher.publish("test-topic", vec![message]).await.unwrap();
        
        let received = assert_message_received(&mut subscriber, b"utility test", 1).await.unwrap();
        assert!(received.metadata.contains_key("test"));
    }
}
```

## Best Practices

1. **Use the in-memory broker for unit tests** - Fast and reliable
2. **Mock external dependencies** - Test your logic, not external systems
3. **Test error conditions** - Verify error handling works correctly
4. **Use timeouts** - Prevent tests from hanging indefinitely
5. **Test message serialization** - Ensure data integrity
6. **Verify metadata** - Check that metadata is correctly set and preserved
7. **Test concurrent scenarios** - Verify thread safety
8. **Use test utilities** - Create reusable helper functions

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_publish_message

# Run tests in parallel
cargo test --jobs 4

# Run integration tests only
cargo test --test integration_tests
```

This testing guide provides comprehensive examples for testing Kincir-based applications effectively.
