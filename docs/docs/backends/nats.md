---
layout: default
title: NATS
parent: Backends
nav_order: 4
---

# NATS Backend
{: .no_toc }

NATS is a lightweight, high-performance message queue system ideal for cloud-native applications.

## Installation

Enable the NATS feature:

```toml
[dependencies]
kincir = { version = "0.2.0", features = ["nats"] }
```

## Usage

```rust
use kincir::nats::{NatsPublisher, NatsSubscriber};
use kincir::{Publisher, Subscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create publisher
    let publisher = NatsPublisher::new("nats://localhost:4222").await?;
    
    // Publish messages
    publisher.publish("orders", vec![
        Message::new(b"Order #1".to_vec())
    ]).await?;
    
    // Create subscriber
    let mut subscriber = NatsSubscriber::new("nats://localhost:4222").await?;
    subscriber.subscribe("orders").await?;
    
    // Receive message
    let msg = subscriber.receive().await?;
    println!("Received: {:?}", String::from_utf8_lossy(&msg.payload));
    
    Ok(())
}
```

## Configuration

NATS supports various connection options:

```rust
// With authentication
let publisher = NatsPublisher::new("nats://user:pass@localhost:4222").await?;

// With token
let publisher = NatsPublisher::new("nats://mytoken@localhost:4222").await?;
```

## Features

- **Lightweight** - Minimal resource usage
- **High performance** - Sub-millisecond latency
- **Cloud-native** - Built for modern distributed systems
- **Subject-based** - Flexible pub/sub messaging
- **No persistence** - Fire-and-forget by default (use JetStream for persistence)

## Error Handling

```rust
use kincir::nats::NatsError;

match publisher.publish("topic", messages).await {
    Ok(_) => println!("Published"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## See Also

- [NATS Documentation](https://docs.nats.io/)
- [async-nats crate](https://crates.io/crates/async-nats)
