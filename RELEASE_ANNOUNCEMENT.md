# Kincir v0.2.0 Released

We're excited to announce the release of Kincir v0.2.0, a major update that brings significant improvements to our Rust messaging library.

## What's New

This release introduces several features that make Kincir much more practical for real-world use:

**In-Memory Message Broker**: You can now use Kincir without any external dependencies. This is perfect for testing, development, or lightweight applications that don't need the complexity of external message brokers.

**Message Acknowledgments**: We've added comprehensive acknowledgment support across all backends (RabbitMQ, Kafka, and MQTT). This ensures reliable message delivery and proper error handling in production environments.

**MQTT Support**: Full MQTT implementation with Quality of Service (QoS) handling, making it easier to integrate IoT devices and services.

**Better Performance**: We've significantly improved performance with sub-millisecond message delivery latency and throughput exceeding 100,000 messages per second.

## Why This Matters

Before this release, using Kincir required setting up external message brokers and dealing with complex configuration. Now you can get started immediately with the in-memory broker, then seamlessly switch to production-grade brokers like RabbitMQ or Kafka when you're ready to scale.

The acknowledgment system means you can trust that your messages are being delivered reliably, which is crucial for any serious application.

## Getting Started

Install Kincir v0.2.0 with:

```bash
cargo add kincir
```

Here's a simple example using the new in-memory broker:

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

let broker = Arc::new(InMemoryBroker::with_default_config());
let publisher = InMemoryPublisher::new(broker.clone());
let mut subscriber = InMemorySubscriber::new(broker.clone());

subscriber.subscribe("orders").await?;
publisher.publish("orders", vec![Message::new(b"Order #1234".to_vec())]).await?;
let message = subscriber.receive().await?;
```

## What's Next

We're working toward v1.0 with plans for middleware support, additional message brokers, and distributed tracing. Our goal is to provide a messaging library that's as easy to use as it is powerful.

You can find the full documentation at [docs.rs/kincir](https://docs.rs/kincir) and the source code on [GitHub](https://github.com/rezacute/kincir).

Thanks to everyone who provided feedback and helped make this release possible. We're looking forward to seeing what you build with Kincir.
