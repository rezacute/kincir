---
layout: default
title: In-Memory Broker Example
description: Zero-dependency, high-performance message broker for testing and development
---

# In-Memory Broker Example

The in-memory broker is Kincir's zero-dependency, high-performance message broker perfect for testing, development, and lightweight production scenarios.

## Features

- **Zero Dependencies**: No external broker software required
- **High Performance**: Sub-millisecond message delivery (2-3µs average)
- **Thread Safe**: Concurrent publishers and subscribers with deadlock resolution
- **Feature Rich**: Message ordering, TTL, health monitoring, and statistics
- **Testing Friendly**: Perfect for unit tests and development

## Basic Usage

### Simple Publisher-Subscriber

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create broker with default configuration
    let broker = Arc::new(InMemoryBroker::with_default_config());
    
    // Create publisher and subscriber
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());

    // Subscribe to a topic
    subscriber.subscribe("events").await?;
    
    // Publish a message
    let message = Message::new(b"Hello, Kincir!".to_vec());
    publisher.publish("events", vec![message]).await?;
    
    // Receive the message
    let received = subscriber.receive().await?;
    println!("Received: {:?}", String::from_utf8_lossy(&received.payload));
    
    Ok(())
}
```

## Advanced Configuration

### Custom Broker Configuration

```rust
use kincir::memory::{InMemoryBrokerConfig, InMemoryBroker};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create custom configuration
    let config = InMemoryBrokerConfig {
        max_messages_per_topic: 10000,
        message_ttl_seconds: Some(3600), // 1 hour TTL
        enable_message_ordering: true,
        enable_statistics: true,
        enable_health_monitoring: true,
    };

    // Create broker with custom config
    let broker = Arc::new(InMemoryBroker::with_config(config));
    
    // Use the broker...
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    Ok(())
}
```

## Multiple Topics Example

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    
    // Create multiple subscribers for different topics
    let mut user_subscriber = InMemorySubscriber::new(broker.clone());
    let mut order_subscriber = InMemorySubscriber::new(broker.clone());
    let mut notification_subscriber = InMemorySubscriber::new(broker.clone());
    
    // Subscribe to different topics
    user_subscriber.subscribe("users").await?;
    order_subscriber.subscribe("orders").await?;
    notification_subscriber.subscribe("notifications").await?;
    
    // Publish messages to different topics
    let user_msg = Message::new(b"User registered: john@example.com".to_vec())
        .with_metadata("event_type", "user_registration")
        .with_metadata("user_id", "12345");
    
    let order_msg = Message::new(b"Order placed: #ORD-001".to_vec())
        .with_metadata("event_type", "order_placed")
        .with_metadata("order_id", "ORD-001")
        .with_metadata("amount", "99.99");
    
    let notification_msg = Message::new(b"Welcome email sent".to_vec())
        .with_metadata("event_type", "email_sent")
        .with_metadata("recipient", "john@example.com");
    
    // Publish all messages
    publisher.publish("users", vec![user_msg]).await?;
    publisher.publish("orders", vec![order_msg]).await?;
    publisher.publish("notifications", vec![notification_msg]).await?;
    
    // Receive messages from each subscriber
    println!("User event: {:?}", user_subscriber.receive().await?);
    println!("Order event: {:?}", order_subscriber.receive().await?);
    println!("Notification event: {:?}", notification_subscriber.receive().await?);
    
    Ok(())
}
```

## Concurrent Publishers and Subscribers

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    
    // Spawn multiple publishers
    let mut publisher_handles = vec![];
    for i in 0..5 {
        let broker_clone = broker.clone();
        let handle = task::spawn(async move {
            let publisher = InMemoryPublisher::new(broker_clone);
            for j in 0..10 {
                let message = Message::new(format!("Message from publisher {} - {}", i, j).into_bytes())
                    .with_metadata("publisher_id", &i.to_string())
                    .with_metadata("message_number", &j.to_string());
                
                publisher.publish("concurrent_topic", vec![message]).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });
        publisher_handles.push(handle);
    }
    
    // Spawn multiple subscribers
    let mut subscriber_handles = vec![];
    for i in 0..3 {
        let broker_clone = broker.clone();
        let handle = task::spawn(async move {
            let mut subscriber = InMemorySubscriber::new(broker_clone);
            subscriber.subscribe("concurrent_topic").await.unwrap();
            
            for _ in 0..15 { // Each subscriber will receive some messages
                match subscriber.receive().await {
                    Ok(message) => {
                        println!("Subscriber {} received: {:?}", i, 
                            String::from_utf8_lossy(&message.payload));
                    }
                    Err(e) => {
                        println!("Subscriber {} error: {}", i, e);
                        break;
                    }
                }
            }
        });
        subscriber_handles.push(handle);
    }
    
    // Wait for all publishers to complete
    for handle in publisher_handles {
        handle.await?;
    }
    
    // Give subscribers time to process remaining messages
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Cancel subscriber tasks
    for handle in subscriber_handles {
        handle.abort();
    }
    
    Ok(())
}
```

## Message Metadata and Filtering

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    subscriber.subscribe("events").await?;
    
    // Publish messages with rich metadata
    let messages = vec![
        Message::new(b"User login".to_vec())
            .with_metadata("event_type", "authentication")
            .with_metadata("action", "login")
            .with_metadata("user_id", "user123")
            .with_metadata("ip_address", "192.168.1.100")
            .with_metadata("timestamp", "2025-07-25T21:00:00Z"),
            
        Message::new(b"Order created".to_vec())
            .with_metadata("event_type", "commerce")
            .with_metadata("action", "order_created")
            .with_metadata("order_id", "ORD-456")
            .with_metadata("customer_id", "user123")
            .with_metadata("amount", "149.99")
            .with_metadata("currency", "USD"),
            
        Message::new(b"Payment processed".to_vec())
            .with_metadata("event_type", "payment")
            .with_metadata("action", "payment_success")
            .with_metadata("transaction_id", "TXN-789")
            .with_metadata("order_id", "ORD-456")
            .with_metadata("method", "credit_card"),
    ];
    
    publisher.publish("events", messages).await?;
    
    // Process messages with metadata
    for _ in 0..3 {
        let message = subscriber.receive().await?;
        println!("Event: {}", String::from_utf8_lossy(&message.payload));
        println!("  Type: {:?}", message.get_metadata("event_type"));
        println!("  Action: {:?}", message.get_metadata("action"));
        println!("  UUID: {}", message.uuid);
        println!("  All metadata: {:?}", message.metadata);
        println!("---");
    }
    
    Ok(())
}
```

## Performance Testing

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    subscriber.subscribe("performance_test").await?;
    
    let message_count = 10_000;
    let message_size = 1024; // 1KB messages
    let payload = vec![0u8; message_size];
    
    // Prepare messages
    let messages: Vec<Message> = (0..message_count)
        .map(|i| Message::new(payload.clone())
            .with_metadata("sequence", &i.to_string()))
        .collect();
    
    println!("Starting performance test with {} messages of {} bytes each", 
             message_count, message_size);
    
    // Measure publishing time
    let start = Instant::now();
    publisher.publish("performance_test", messages).await?;
    let publish_duration = start.elapsed();
    
    println!("Published {} messages in {:?}", message_count, publish_duration);
    println!("Publishing rate: {:.2} messages/second", 
             message_count as f64 / publish_duration.as_secs_f64());
    
    // Measure receiving time
    let start = Instant::now();
    for i in 0..message_count {
        let _message = subscriber.receive().await?;
        if i % 1000 == 0 {
            println!("Received {} messages", i + 1);
        }
    }
    let receive_duration = start.elapsed();
    
    println!("Received {} messages in {:?}", message_count, receive_duration);
    println!("Receiving rate: {:.2} messages/second", 
             message_count as f64 / receive_duration.as_secs_f64());
    
    // Get broker statistics
    if let Some(stats) = broker.get_statistics() {
        println!("Broker statistics: {:?}", stats);
    }
    
    Ok(())
}
```

## Error Handling

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message, KincirError};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    // Error handling for subscription
    match subscriber.subscribe("test_topic").await {
        Ok(()) => println!("Successfully subscribed to test_topic"),
        Err(e) => {
            eprintln!("Failed to subscribe: {}", e);
            return Err(e.into());
        }
    }
    
    // Error handling for publishing
    let message = Message::new(b"Test message".to_vec());
    match publisher.publish("test_topic", vec![message]).await {
        Ok(()) => println!("Message published successfully"),
        Err(KincirError::BrokerError(e)) => {
            eprintln!("Broker error: {}", e);
        }
        Err(KincirError::SerializationError(e)) => {
            eprintln!("Serialization error: {}", e);
        }
        Err(e) => {
            eprintln!("Other error: {}", e);
        }
    }
    
    // Error handling for receiving
    match subscriber.receive().await {
        Ok(message) => {
            println!("Received: {:?}", String::from_utf8_lossy(&message.payload));
        }
        Err(KincirError::TimeoutError) => {
            println!("No message received within timeout");
        }
        Err(KincirError::BrokerError(e)) => {
            eprintln!("Broker error while receiving: {}", e);
        }
        Err(e) => {
            eprintln!("Error receiving message: {}", e);
        }
    }
    
    Ok(())
}
```

## Testing with In-Memory Broker

The in-memory broker is perfect for unit testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
    use kincir::{Publisher, Subscriber, Message};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_basic_pub_sub() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        subscriber.subscribe("test").await.unwrap();
        
        let message = Message::new(b"test message".to_vec());
        publisher.publish("test", vec![message]).await.unwrap();
        
        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.payload, b"test message");
    }
    
    #[tokio::test]
    async fn test_message_metadata() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        subscriber.subscribe("test").await.unwrap();
        
        let message = Message::new(b"test".to_vec())
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");
        
        publisher.publish("test", vec![message]).await.unwrap();
        
        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.get_metadata("key1"), Some("value1"));
        assert_eq!(received.get_metadata("key2"), Some("value2"));
    }
}
```

## Next Steps

- [RabbitMQ Integration](/examples/rabbitmq.html) - Production message queuing
- [Kafka Integration](/examples/kafka.html) - High-throughput streaming
- [Message Acknowledgments](/examples/acknowledgments.html) - Reliable processing
- [Performance Optimization](/examples/performance.html) - Tuning for high load

## Resources

- [Getting Started Guide](/docs/getting-started.html)
- [API Documentation](https://docs.rs/kincir)
- [GitHub Repository](https://github.com/rezacute/kincir)
