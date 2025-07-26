---
layout: default
title: In-Memory Broker Quick Start
parent: Quick Start
nav_order: 1
---

# In-Memory Broker Quick Start
{: .no_toc }

Get up and running with Kincir's in-memory message broker in under 5 minutes.

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Installation

Add Kincir to your `Cargo.toml`:

```toml
[dependencies]
kincir = "0.1.6"
tokio = { version = "1.0", features = ["full"] }
```

## Your First Message

Create a new Rust file and add this code:

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create the broker
    let broker = Arc::new(InMemoryBroker::with_default_config());
    
    // 2. Create publisher and subscriber
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    // 3. Subscribe to a topic
    subscriber.subscribe("hello").await?;
    
    // 4. Publish a message
    let message = Message::new(b"Hello, Kincir!".to_vec());
    publisher.publish("hello", vec![message]).await?;
    
    // 5. Receive the message
    let received = subscriber.receive().await?;
    println!("Received: {}", String::from_utf8_lossy(&received.payload));
    
    Ok(())
}
```

Run it:

```bash
cargo run
```

You should see:
```
Received: Hello, Kincir!
```

🎉 **Congratulations!** You've sent your first message with Kincir!

## Adding Metadata

Messages can carry metadata for routing, filtering, and processing:

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    subscriber.subscribe("orders").await?;
    
    // Create message with metadata
    let order = Message::new(b"Order #1234: 2x Coffee".to_vec())
        .with_metadata("customer", "Alice")
        .with_metadata("priority", "high")
        .with_metadata("total", "15.50");
    
    publisher.publish("orders", vec![order]).await?;
    
    let received = subscriber.receive().await?;
    println!("Order: {}", String::from_utf8_lossy(&received.payload));
    println!("Customer: {}", received.metadata.get("customer").unwrap());
    println!("Priority: {}", received.metadata.get("priority").unwrap());
    
    Ok(())
}
```

## Multiple Topics

Handle different types of messages with multiple topics:

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    
    // Create subscribers for different topics
    let mut orders_sub = InMemorySubscriber::new(broker.clone());
    let mut notifications_sub = InMemorySubscriber::new(broker.clone());
    
    orders_sub.subscribe("orders").await?;
    notifications_sub.subscribe("notifications").await?;
    
    // Publish to different topics
    publisher.publish("orders", vec![
        Message::new(b"New order received".to_vec())
    ]).await?;
    
    publisher.publish("notifications", vec![
        Message::new(b"Welcome to our service!".to_vec())
    ]).await?;
    
    // Receive from different topics
    let order = orders_sub.receive().await?;
    let notification = notifications_sub.receive().await?;
    
    println!("Order: {}", String::from_utf8_lossy(&order.payload));
    println!("Notification: {}", String::from_utf8_lossy(&notification.payload));
    
    Ok(())
}
```

## Configuration

Customize the broker for your needs:

```rust
use kincir::memory::{InMemoryBroker, InMemoryConfig};
use std::time::Duration;

// High-performance configuration
let config = InMemoryConfig::high_performance();
let broker = InMemoryBroker::new(config);

// Custom configuration
let config = InMemoryConfig::new()
    .with_max_queue_size(Some(1000))      // Limit queue size
    .with_stats(true)                     // Enable statistics
    .with_maintain_order(true)            // Message ordering
    .with_message_ttl(Some(Duration::from_secs(300))); // 5-minute TTL

let broker = InMemoryBroker::new(config);
```

## Error Handling

Handle errors gracefully:

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemoryConfig, InMemoryError};
use kincir::{Publisher, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create broker with small queue limit
    let config = InMemoryConfig::new().with_max_queue_size(Some(1));
    let broker = Arc::new(InMemoryBroker::new(config));
    let publisher = InMemoryPublisher::new(broker);
    
    // First message succeeds
    publisher.publish("limited", vec![
        Message::new(b"Message 1".to_vec())
    ]).await?;
    
    // Second message fails due to queue limit
    match publisher.publish("limited", vec![
        Message::new(b"Message 2".to_vec())
    ]).await {
        Ok(()) => println!("Published successfully"),
        Err(InMemoryError::QueueFull { topic }) => {
            println!("Queue full for topic: {}", topic);
        },
        Err(e) => println!("Other error: {}", e),
    }
    
    Ok(())
}
```

## Testing

Perfect for unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
    use kincir::{Publisher, Subscriber, Message};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_message_flow() {
        // Use testing configuration
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

## Next Steps

Now that you have the basics down, explore more advanced features:

- **[Configuration Guide]({{ '/docs/backends/in-memory/configuration' | relative_url }})** - Customize the broker for your use case
- **[Examples]({{ '/docs/backends/in-memory/examples' | relative_url }})** - See practical examples and patterns
- **[Full Documentation]({{ '/docs/backends/in-memory/' | relative_url }})** - Complete feature reference

## Why In-Memory?

The in-memory broker is perfect for:

✅ **Development** - No external dependencies to set up  
✅ **Testing** - Fast, deterministic test execution  
✅ **Prototyping** - Quick experimentation with message patterns  
✅ **Lightweight Production** - Simple deployments without external brokers  
✅ **Learning** - Understand messaging concepts without complexity  

Ready to build something amazing? [Check out the examples]({{ '/docs/backends/in-memory/examples' | relative_url }}) for inspiration!
