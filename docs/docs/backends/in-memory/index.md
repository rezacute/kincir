---
layout: default
title: In-Memory Broker
parent: Backends
nav_order: 1
---

# In-Memory Message Broker
{: .no_toc }

The in-memory broker provides a lightweight, high-performance message broker implementation that runs entirely in memory. It's perfect for testing, development, and lightweight production scenarios where external broker dependencies are not desired.

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

The in-memory broker is a complete implementation of Kincir's `Publisher` and `Subscriber` traits that operates entirely within your application's memory space. It provides:

- **Zero external dependencies** - No need for Kafka, RabbitMQ, or other external brokers
- **High performance** - Sub-millisecond message delivery latency
- **Thread-safe** - Concurrent publishers and subscribers supported
- **Feature-rich** - Message ordering, TTL, health monitoring, and more
- **Testing-friendly** - Perfect for unit and integration tests

## Quick Start

Add Kincir to your `Cargo.toml`:

```toml
[dependencies]
kincir = "0.1.6"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create broker with default configuration
    let broker = Arc::new(InMemoryBroker::with_default_config());
    
    // Create publisher and subscriber
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    // Subscribe to a topic
    subscriber.subscribe("orders").await?;
    
    // Publish a message
    let message = Message::new(b"Order #1234".to_vec())
        .with_metadata("customer", "Alice")
        .with_metadata("priority", "high");
    
    publisher.publish("orders", vec![message]).await?;
    
    // Receive the message
    let received = subscriber.receive().await?;
    println!("Received: {}", String::from_utf8_lossy(&received.payload));
    
    Ok(())
}
```

## Configuration

The in-memory broker supports extensive configuration through `InMemoryConfig`:

### Basic Configuration

```rust
use kincir::memory::InMemoryConfig;
use std::time::Duration;

let config = InMemoryConfig::new()
    .with_max_queue_size(Some(1000))      // Limit queue size per topic
    .with_max_topics(Some(100))           // Limit number of topics
    .with_stats(true)                     // Enable statistics collection
    .with_maintain_order(true)            // Enable message ordering
    .with_message_ttl(Some(Duration::from_secs(300))); // 5-minute TTL
```

### Pre-configured Profiles

```rust
// High-performance configuration
let config = InMemoryConfig::high_performance();

// Testing configuration (optimized for tests)
let config = InMemoryConfig::for_testing();

// Default configuration
let config = InMemoryConfig::default();
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_queue_size` | `Option<usize>` | `None` | Maximum messages per topic queue |
| `max_topics` | `Option<usize>` | `None` | Maximum number of topics |
| `enable_persistence` | `bool` | `true` | Enable message persistence in queues |
| `maintain_order` | `bool` | `true` | Maintain message ordering |
| `enable_stats` | `bool` | `false` | Enable statistics collection |
| `default_message_ttl` | `Option<Duration>` | `None` | Default message time-to-live |
| `cleanup_interval` | `Duration` | `60s` | TTL cleanup task interval |

## Advanced Features

### Message Ordering

When `maintain_order` is enabled, messages are automatically assigned sequence numbers:

```rust
let config = InMemoryConfig::new().with_maintain_order(true);
let broker = Arc::new(InMemoryBroker::new(config));

// Messages will have _sequence metadata: "1", "2", "3", etc.
```

### Message Time-To-Live (TTL)

Configure automatic message expiration:

```rust
use std::time::Duration;

let config = InMemoryConfig::new()
    .with_message_ttl(Some(Duration::from_secs(300))) // 5 minutes
    .with_cleanup_interval(Duration::from_secs(60));  // Check every minute

let broker = Arc::new(InMemoryBroker::new(config));
```

### Health Monitoring

Monitor broker health and performance:

```rust
// Get broker health information
let health = broker.health_check();
println!("Healthy: {}", health.is_healthy);
println!("Topics: {}", health.topic_count);
println!("Queued messages: {}", health.total_queued_messages);
println!("Memory usage: {} bytes", health.memory_usage_estimate);

// Get detailed topic information
let topics = broker.list_topic_info();
for topic in topics {
    println!("Topic: {}, Messages: {}, Age: {:?}", 
             topic.name, topic.queue_size, topic.age());
}
```

### Statistics Collection

Enable comprehensive statistics:

```rust
let config = InMemoryConfig::new().with_stats(true);
let broker = Arc::new(InMemoryBroker::new(config));
let publisher = InMemoryPublisher::new(broker.clone());

// Publish some messages...

// Get statistics
if let Some(stats) = publisher.stats() {
    println!("Messages published: {}", stats.messages_published());
    println!("Messages consumed: {}", stats.messages_consumed());
    println!("Active topics: {}", stats.active_topics());
    println!("Uptime: {:?}", stats.uptime());
}
```

### Graceful Shutdown

Properly shutdown the broker:

```rust
// Graceful shutdown (waits for in-flight operations)
broker.shutdown()?;

// Force shutdown (immediate)
broker.force_shutdown()?;
```

## Error Handling

The in-memory broker provides comprehensive error handling:

```rust
use kincir::memory::InMemoryError;

match publisher.publish("topic", messages).await {
    Ok(()) => println!("Published successfully"),
    Err(InMemoryError::QueueFull { topic }) => {
        println!("Queue full for topic: {}", topic);
    },
    Err(InMemoryError::MaxTopicsReached { limit }) => {
        println!("Cannot create more topics, limit: {}", limit);
    },
    Err(InMemoryError::BrokerShutdown) => {
        println!("Broker is shutdown");
    },
    Err(e) => println!("Other error: {}", e),
}
```

## Performance Characteristics

The in-memory broker is optimized for high performance:

- **Latency**: Sub-millisecond message delivery
- **Throughput**: Thousands of messages per second
- **Memory**: Efficient memory usage with optional limits
- **Concurrency**: Thread-safe operations with minimal contention

### Benchmarks

Typical performance on modern hardware:

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Publish | < 0.1ms | 10,000+ msg/s |
| Subscribe | < 0.1ms | 10,000+ msg/s |
| Topic Creation | < 0.05ms | 20,000+ ops/s |

## Testing Integration

The in-memory broker is perfect for testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
    
    #[tokio::test]
    async fn test_message_flow() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        subscriber.subscribe("test").await.unwrap();
        
        let message = Message::new(b"test data".to_vec());
        publisher.publish("test", vec![message.clone()]).await.unwrap();
        
        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.payload, message.payload);
    }
}
```

## Migration from External Brokers

The in-memory broker implements the same `Publisher` and `Subscriber` traits as other Kincir backends, making migration seamless:

```rust
// Before: Using Kafka
let publisher = KafkaPublisher::new("localhost:9092");

// After: Using in-memory broker
let broker = Arc::new(InMemoryBroker::with_default_config());
let publisher = InMemoryPublisher::new(broker);

// Same API, no code changes needed!
publisher.publish("topic", messages).await?;
```

## Best Practices

### For Development
- Use `InMemoryConfig::for_testing()` in tests
- Enable statistics for debugging
- Set reasonable queue limits to catch issues early

### For Production
- Use `InMemoryConfig::high_performance()` for production
- Monitor memory usage with health checks
- Implement proper error handling
- Consider message TTL for long-running applications

### For High Throughput
- Disable ordering if not needed (`maintain_order: false`)
- Disable statistics collection in production
- Use larger queue sizes
- Batch messages when possible

## Limitations

While the in-memory broker is feature-rich, be aware of these limitations:

- **Persistence**: Messages are lost on application restart
- **Distribution**: Cannot share messages across multiple application instances  
- **Memory**: All messages stored in RAM
- **Durability**: No disk-based durability guarantees

For scenarios requiring persistence or distribution, consider using Kafka or RabbitMQ backends.

## API Reference

### InMemoryBroker

Core broker implementation with topic management and health monitoring.

### InMemoryPublisher

Publisher implementation for sending messages to topics.

### InMemorySubscriber  

Subscriber implementation for receiving messages from topics.

### InMemoryConfig

Configuration builder for customizing broker behavior.

### InMemoryError

Error types for comprehensive error handling.

For detailed API documentation, see the [Rust docs](https://docs.rs/kincir).
