---
layout: default
title: Examples
parent: In-Memory Broker
grand_parent: Backends
nav_order: 3
---

# In-Memory Broker Examples
{: .no_toc }

Practical examples demonstrating various use cases and patterns with the in-memory message broker.

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Basic Examples

### Simple Publisher-Subscriber

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create broker
    let broker = Arc::new(InMemoryBroker::with_default_config());
    
    // Create publisher and subscriber
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    // Subscribe to topic
    subscriber.subscribe("notifications").await?;
    
    // Publish message
    let message = Message::new(b"Hello, World!".to_vec());
    publisher.publish("notifications", vec![message]).await?;
    
    // Receive message
    let received = subscriber.receive().await?;
    println!("Received: {}", String::from_utf8_lossy(&received.payload));
    
    Ok(())
}
```

### Multiple Topics

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    
    // Create multiple subscribers for different topics
    let mut orders_subscriber = InMemorySubscriber::new(broker.clone());
    let mut notifications_subscriber = InMemorySubscriber::new(broker.clone());
    
    orders_subscriber.subscribe("orders").await?;
    notifications_subscriber.subscribe("notifications").await?;
    
    // Publish to different topics
    publisher.publish("orders", vec![
        Message::new(b"Order #1234".to_vec())
            .with_metadata("customer", "Alice")
    ]).await?;
    
    publisher.publish("notifications", vec![
        Message::new(b"Welcome Alice!".to_vec())
            .with_metadata("type", "welcome")
    ]).await?;
    
    // Receive from different topics
    let order = orders_subscriber.receive().await?;
    let notification = notifications_subscriber.receive().await?;
    
    println!("Order: {}", String::from_utf8_lossy(&order.payload));
    println!("Notification: {}", String::from_utf8_lossy(&notification.payload));
    
    Ok(())
}
```

## Advanced Examples

### Message Ordering with Sequence Numbers

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable message ordering
    let config = InMemoryConfig::new().with_maintain_order(true);
    let broker = Arc::new(InMemoryBroker::new(config));
    
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    subscriber.subscribe("ordered-events").await?;
    
    // Publish multiple messages
    let messages = vec![
        Message::new(b"Event 1".to_vec()),
        Message::new(b"Event 2".to_vec()),
        Message::new(b"Event 3".to_vec()),
    ];
    
    publisher.publish("ordered-events", messages).await?;
    
    // Receive messages in order
    for i in 1..=3 {
        let message = subscriber.receive().await?;
        let sequence = message.metadata.get("_sequence").unwrap();
        println!("Received message {} with sequence {}", 
                 String::from_utf8_lossy(&message.payload), sequence);
    }
    
    Ok(())
}
```

### Message TTL and Cleanup

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure TTL
    let config = InMemoryConfig::new()
        .with_message_ttl(Some(Duration::from_secs(2)))  // 2-second TTL
        .with_cleanup_interval(Duration::from_secs(1));  // Check every second
    
    let broker = Arc::new(InMemoryBroker::new(config));
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    subscriber.subscribe("temp-messages").await?;
    
    // Publish message
    publisher.publish("temp-messages", vec![
        Message::new(b"This message will expire".to_vec())
    ]).await?;
    
    println!("Message published, waiting for expiration...");
    
    // Wait for message to expire
    sleep(Duration::from_secs(3)).await;
    
    // Try to consume - should be empty due to TTL
    match broker.consume("temp-messages") {
        Ok(Some(msg)) => println!("Message still available: {}", 
                                String::from_utf8_lossy(&msg.payload)),
        Ok(None) => println!("Message expired and cleaned up"),
        Err(e) => println!("Error: {}", e),
    }
    
    Ok(())
}
```

### Health Monitoring and Statistics

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable statistics
    let config = InMemoryConfig::new().with_stats(true);
    let broker = Arc::new(InMemoryBroker::new(config));
    
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    subscriber.subscribe("monitored-topic").await?;
    
    // Publish some messages
    for i in 1..=5 {
        let message = Message::new(format!("Message {}", i).into_bytes());
        publisher.publish("monitored-topic", vec![message]).await?;
    }
    
    // Consume some messages
    for _ in 1..=3 {
        let _ = subscriber.receive().await?;
    }
    
    // Check broker health
    let health = broker.health_check();
    println!("Broker Health:");
    println!("  Healthy: {}", health.is_healthy);
    println!("  Topics: {}", health.topic_count);
    println!("  Queued messages: {}", health.total_queued_messages);
    println!("  Memory usage: {} bytes", health.memory_usage_estimate);
    
    // Get detailed statistics
    if let Some(stats) = publisher.stats() {
        println!("\nStatistics:");
        println!("  Messages published: {}", stats.messages_published());
        println!("  Messages consumed: {}", stats.messages_consumed());
        println!("  Active topics: {}", stats.active_topics());
        println!("  Uptime: {:?}", stats.uptime());
    }
    
    // Get topic information
    let topics = broker.list_topic_info();
    for topic in topics {
        println!("\nTopic: {}", topic.name);
        println!("  Queue size: {}", topic.queue_size);
        println!("  Subscribers: {}", topic.subscriber_count);
        println!("  Total published: {}", topic.total_published);
        println!("  Total consumed: {}", topic.total_consumed);
        println!("  Age: {:?}", topic.age());
    }
    
    Ok(())
}
```

## Concurrent Examples

### Multiple Publishers

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    subscriber.subscribe("concurrent-topic").await?;
    
    // Spawn multiple publisher tasks
    let mut handles = vec![];
    
    for i in 1..=3 {
        let broker_clone = broker.clone();
        let handle = task::spawn(async move {
            let publisher = InMemoryPublisher::new(broker_clone);
            
            for j in 1..=5 {
                let message = Message::new(
                    format!("Publisher {} - Message {}", i, j).into_bytes()
                );
                publisher.publish("concurrent-topic", vec![message]).await.unwrap();
            }
        });
        handles.push(handle);
    }
    
    // Wait for all publishers to complete
    for handle in handles {
        handle.await?;
    }
    
    // Receive all messages
    for _ in 1..=15 {  // 3 publishers × 5 messages each
        let message = subscriber.receive().await?;
        println!("Received: {}", String::from_utf8_lossy(&message.payload));
    }
    
    Ok(())
}
```

### Multiple Subscribers

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    
    // Spawn multiple subscriber tasks
    let mut handles = vec![];
    
    for i in 1..=3 {
        let broker_clone = broker.clone();
        let handle = task::spawn(async move {
            let mut subscriber = InMemorySubscriber::new(broker_clone);
            subscriber.subscribe("broadcast-topic").await.unwrap();
            
            // Each subscriber receives all messages
            for _ in 1..=5 {
                let message = subscriber.receive().await.unwrap();
                println!("Subscriber {} received: {}", 
                         i, String::from_utf8_lossy(&message.payload));
            }
        });
        handles.push(handle);
    }
    
    // Give subscribers time to subscribe
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Publish messages (will be broadcast to all subscribers)
    for i in 1..=5 {
        let message = Message::new(format!("Broadcast message {}", i).into_bytes());
        publisher.publish("broadcast-topic", vec![message]).await?;
    }
    
    // Wait for all subscribers to complete
    for handle in handles {
        handle.await?;
    }
    
    Ok(())
}
```

## Testing Examples

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
    use kincir::{Publisher, Subscriber, Message};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_basic_message_flow() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        subscriber.subscribe("test-topic").await.unwrap();
        
        let original_message = Message::new(b"test data".to_vec())
            .with_metadata("test", "value");
        
        publisher.publish("test-topic", vec![original_message.clone()]).await.unwrap();
        
        let received_message = subscriber.receive().await.unwrap();
        
        assert_eq!(received_message.payload, original_message.payload);
        assert_eq!(received_message.metadata.get("test"), Some(&"value".to_string()));
    }
    
    #[tokio::test]
    async fn test_queue_limits() {
        let config = InMemoryConfig::for_testing()
            .with_max_queue_size(Some(2));
        let broker = Arc::new(InMemoryBroker::new(config));
        let publisher = InMemoryPublisher::new(broker.clone());
        
        // First two messages should succeed
        let messages = vec![
            Message::new(b"Message 1".to_vec()),
            Message::new(b"Message 2".to_vec()),
        ];
        publisher.publish("test-topic", messages).await.unwrap();
        
        // Third message should fail due to queue limit
        let result = publisher.publish("test-topic", vec![
            Message::new(b"Message 3".to_vec())
        ]).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_message_ordering() {
        let config = InMemoryConfig::for_testing()
            .with_maintain_order(true);
        let broker = Arc::new(InMemoryBroker::new(config));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        subscriber.subscribe("ordered-topic").await.unwrap();
        
        let messages = vec![
            Message::new(b"First".to_vec()),
            Message::new(b"Second".to_vec()),
            Message::new(b"Third".to_vec()),
        ];
        
        publisher.publish("ordered-topic", messages).await.unwrap();
        
        // Verify sequence numbers
        for expected_seq in 1..=3 {
            let message = subscriber.receive().await.unwrap();
            let sequence = message.metadata.get("_sequence").unwrap();
            assert_eq!(sequence, &expected_seq.to_string());
        }
    }
}
```

### Integration Test Example

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
    use kincir::{Publisher, Subscriber, Message};
    use std::sync::Arc;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn test_high_throughput_scenario() {
        let config = InMemoryConfig::for_testing()
            .with_max_queue_size(Some(1000))
            .with_stats(true);
        let broker = Arc::new(InMemoryBroker::new(config));
        
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        subscriber.subscribe("high-throughput").await.unwrap();
        
        // Publish 100 messages
        let messages: Vec<Message> = (1..=100)
            .map(|i| Message::new(format!("Message {}", i).into_bytes()))
            .collect();
        
        let start = std::time::Instant::now();
        publisher.publish("high-throughput", messages).await.unwrap();
        let publish_duration = start.elapsed();
        
        // Consume all messages
        let start = std::time::Instant::now();
        for i in 1..=100 {
            let message = timeout(Duration::from_secs(1), subscriber.receive())
                .await
                .expect("Timeout waiting for message")
                .unwrap();
            
            let expected = format!("Message {}", i);
            assert_eq!(String::from_utf8_lossy(&message.payload), expected);
        }
        let consume_duration = start.elapsed();
        
        println!("Publish duration: {:?}", publish_duration);
        println!("Consume duration: {:?}", consume_duration);
        
        // Verify statistics
        if let Some(stats) = publisher.stats() {
            assert_eq!(stats.messages_published(), 100);
            assert_eq!(stats.messages_consumed(), 100);
        }
    }
    
    #[tokio::test]
    async fn test_concurrent_operations() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        
        // Spawn concurrent publishers and subscribers
        let mut handles = vec![];
        
        // Publishers
        for i in 1..=3 {
            let broker_clone = broker.clone();
            let handle = tokio::spawn(async move {
                let publisher = InMemoryPublisher::new(broker_clone);
                for j in 1..=10 {
                    let message = Message::new(
                        format!("P{}-M{}", i, j).into_bytes()
                    );
                    publisher.publish("concurrent", vec![message]).await.unwrap();
                }
            });
            handles.push(handle);
        }
        
        // Subscribers
        for i in 1..=2 {
            let broker_clone = broker.clone();
            let handle = tokio::spawn(async move {
                let mut subscriber = InMemorySubscriber::new(broker_clone);
                subscriber.subscribe("concurrent").await.unwrap();
                
                let mut received = 0;
                while received < 15 {  // Each subscriber gets all 30 messages
                    let _ = subscriber.receive().await.unwrap();
                    received += 1;
                }
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Verify broker state
        let health = broker.health_check();
        assert!(health.is_healthy);
        assert_eq!(health.topic_count, 1);
    }
}
```

## Error Handling Examples

### Comprehensive Error Handling

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig, InMemoryError};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = InMemoryConfig::new()
        .with_max_queue_size(Some(2))
        .with_max_topics(Some(1));
    
    let broker = Arc::new(InMemoryBroker::new(config));
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    // Test various error conditions
    
    // 1. Publishing to fill queue
    subscriber.subscribe("limited-topic").await?;
    
    let messages = vec![
        Message::new(b"Message 1".to_vec()),
        Message::new(b"Message 2".to_vec()),
    ];
    
    match publisher.publish("limited-topic", messages).await {
        Ok(()) => println!("Successfully published to queue"),
        Err(e) => println!("Publish error: {}", e),
    }
    
    // 2. Try to exceed queue limit
    match publisher.publish("limited-topic", vec![
        Message::new(b"Message 3".to_vec())
    ]).await {
        Ok(()) => println!("Unexpected success"),
        Err(InMemoryError::QueueFull { topic }) => {
            println!("Queue full for topic: {}", topic);
        },
        Err(e) => println!("Other error: {}", e),
    }
    
    // 3. Try to exceed topic limit
    match subscriber.subscribe("second-topic").await {
        Ok(()) => println!("Unexpected success"),
        Err(InMemoryError::MaxTopicsReached { limit }) => {
            println!("Cannot create more topics, limit: {}", limit);
        },
        Err(e) => println!("Other error: {}", e),
    }
    
    // 4. Try to receive without subscription
    let mut unsubscribed = InMemorySubscriber::new(broker.clone());
    match unsubscribed.receive().await {
        Ok(_) => println!("Unexpected success"),
        Err(InMemoryError::NotSubscribed) => {
            println!("Not subscribed to any topic");
        },
        Err(e) => println!("Other error: {}", e),
    }
    
    // 5. Operations after shutdown
    broker.shutdown()?;
    
    match publisher.publish("any-topic", vec![
        Message::new(b"After shutdown".to_vec())
    ]).await {
        Ok(()) => println!("Unexpected success"),
        Err(InMemoryError::BrokerShutdown) => {
            println!("Broker is shutdown");
        },
        Err(e) => println!("Other error: {}", e),
    }
    
    Ok(())
}
```

These examples demonstrate the full range of capabilities available with the in-memory message broker, from basic usage to advanced features and comprehensive error handling.
