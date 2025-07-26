# Kincir

[![Crates.io](https://img.shields.io/crates/v/kincir.svg)](https://crates.io/crates/kincir)
[![Documentation](https://docs.rs/kincir/badge.svg)](https://docs.rs/kincir)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Kincir is a high-performance Rust library that provides a unified interface for message streaming with support for multiple message broker backends. It offers a simple, consistent API for publishing and subscribing to messages across different messaging systems, with advanced routing capabilities and comprehensive acknowledgment support.

**[📚 Online Documentation](https://rezacute.github.io/kincir/) | [🦀 Crates.io](https://crates.io/crates/kincir) | [💻 GitHub Repository](https://github.com/rezacute/kincir)**

## Features

- **In-Memory Message Broker** - Zero-dependency, high-performance broker for testing and lightweight production
- **Message Acknowledgments** - Comprehensive acknowledgment support across RabbitMQ, Kafka, and MQTT backends
- **MQTT Support** - Full MQTT implementation with Quality of Service (QoS) handling
- **Unified messaging interface** with support for multiple backends (Kafka, RabbitMQ, MQTT)
- **Message routing** with customizable handlers
- **Advanced message features** - Message ordering, TTL (Time-To-Live), health monitoring
- **Thread-safe operations** - Concurrent publishers and subscribers with deadlock resolution
- **Built-in logging support** (optional via feature flag)
- **Message UUID generation** for tracking and identification
- **Customizable message metadata** support
- **Async/await support**
- **Type-safe error handling**

## Installation

Add kincir to your `Cargo.toml`:

```toml
[dependencies]
kincir = "0.2.0"
```

### Feature Flags

Kincir provides feature flags to customize the library:

```toml
[dependencies]
# Default features (includes logging)
kincir = "0.2.0"

# Without logging
kincir = { version = "0.2.0", default-features = false }

# Explicitly enable logging
kincir = { version = "0.2.0", features = ["logging"] }

# With Protocol Buffers support
kincir = { version = "0.2.0", features = ["protobuf"] }

# With both logging and Protocol Buffers
kincir = { version = "0.2.0", features = ["logging", "protobuf"] }
```

## Quick Start

### In-Memory Broker (Zero Dependencies)

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
    publisher.publish("orders", vec![Message::new(b"Order #1234".to_vec())]).await?;
    let message = subscriber.receive().await?;
    
    println!("Received: {:?}", String::from_utf8_lossy(&message.payload));
    Ok(())
}
```

### Message Acknowledgments

```rust
use kincir::rabbitmq::RabbitMQAckSubscriber;
use kincir::{AckSubscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut subscriber = RabbitMQAckSubscriber::new("amqp://localhost:5672", "my-queue");
    subscriber.subscribe("orders").await?;

    let (message, ack_handle) = subscriber.receive_with_ack().await?;
    println!("Processing: {:?}", message);

    // Acknowledge successful processing
    ack_handle.ack().await?;
    // Or reject and requeue on error: ack_handle.nack(true).await?;
    
    Ok(())
}
```

### MQTT Support

```rust
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber};
use kincir::{Publisher, Subscriber, Message};
use rumqttc::QoS;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let publisher = MQTTPublisher::new("mqtt://localhost:1883", "client-pub");
    let mut subscriber = MQTTSubscriber::new("mqtt://localhost:1883", "client-sub");

    subscriber.subscribe("sensors/temperature").await?;
    publisher.publish_with_qos("sensors/temperature", 
        vec![Message::new(b"25.5".to_vec())], QoS::AtLeastOnce).await?;
    
    let message = subscriber.receive().await?;
    println!("Temperature: {:?}", String::from_utf8_lossy(&message.payload));
    
    Ok(())
}
```

## Build and Development

### Using Make

The project includes a Makefile to simplify common development tasks:

```bash
# Build the project
make build

# Run tests
make test

# Format code and run linters
make verify

# Generate documentation
make docs

# Run benchmarks
make bench

# Show all available commands
make help
```

### Using Docker

The project includes Docker support for development and testing:

```bash
# Start the Docker environment
./scripts/docker_env.sh start

# Run the Kafka example
./scripts/docker_env.sh kafka

# Run the RabbitMQ example
./scripts/docker_env.sh rabbitmq

# Show all available commands
./scripts/docker_env.sh help
```

For more details on Docker usage, see [README.docker.md](README.docker.md).

## Advanced Usage

### Message Router with Logging

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::logging::{Logger, StdLogger};
use kincir::router::Router;
use kincir::Message;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let logger = Arc::new(StdLogger::new(true, true));
    let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
    let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);

    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            let mut processed_msg = msg;
            processed_msg.set_metadata("processed", "true");
            Ok(vec![processed_msg])
        })
    });

    let router = Router::new(
        logger,
        "input-exchange".to_string(),
        "output-exchange".to_string(),
        subscriber,
        publisher,
        handler,
    );

    router.run().await
}
```

### Protocol Buffers Support

When the `protobuf` feature is enabled:

```rust
#[cfg(feature = "protobuf")]
use kincir::{Message, MessageCodec, ProtobufCodec};

let message = Message::new(b"Hello, Protocol Buffers!".to_vec())
    .with_metadata("encoding", "protobuf");

let codec = ProtobufCodec::new();
let encoded = codec.encode(&message).unwrap();
let decoded = codec.decode(&encoded).unwrap();
```

## Backend Implementations

### Kafka

```rust
use kincir::kafka::{KafkaPublisher, KafkaSubscriber};

let publisher = KafkaPublisher::new("localhost:9092");
let subscriber = KafkaSubscriber::new("localhost:9092", "consumer-group-id");
```

### RabbitMQ

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};

let publisher = RabbitMQPublisher::new("amqp://localhost:5672");
let subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "my-queue");
```

### MQTT

```rust
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber};

let publisher = MQTTPublisher::new("mqtt://localhost:1883", "client-pub");
let subscriber = MQTTSubscriber::new("mqtt://localhost:1883", "client-sub");
```

## Performance

Kincir v0.2.0 delivers exceptional performance:

- **153,000+ messages/second** publish throughput
- **100,000+ messages/second** acknowledgment throughput
- **Sub-millisecond latency** (2-3µs average acknowledgment)
- **Thread-safe concurrent operations** with deadlock resolution
- **Memory-efficient** operations with optimized async pipelines

## Testing

Kincir includes comprehensive testing:

- **138+ tests** covering all functionality
- **47 backend unit tests** for RabbitMQ, Kafka, and MQTT
- **10 integration tests** for cross-backend consistency
- **Performance benchmarking suite** with 7 test categories
- **CI/CD pipeline** with multi-platform testing

## Roadmap to v1.0

Kincir is evolving towards feature parity with Watermill (Golang):

### ✅ v0.2 – Core Enhancements (COMPLETED)
- In-memory message broker for local testing
- Advanced features: message ordering, TTL, health monitoring
- Comprehensive statistics and performance metrics
- Thread-safe concurrent operations with deadlock resolution
- Unit & integration tests for stability

### 🔄 v0.3 – Middleware & Backend Expansion
- Middleware framework: logging, retry, recovery, correlation
- Additional broker support (e.g., NATS, AWS SQS)
- Optimized async pipeline for lower latency
- Integration tests for middleware + new backends

### 📊 v0.4 – Distributed Tracing & Monitoring
- OpenTelemetry-based tracing for message flows
- Prometheus metrics for message processing
- Poison queue (dead-letter handling)
- Throttling & backpressure support

### 🛠 v0.5 – Hardening & API Freeze
- API finalization for stability
- Cross-platform testing (Linux, macOS, Windows)
- Memory optimization and async efficiency improvements

### 🚀 v1.0 – Production-Ready Release
- High-performance, production-ready messaging library
- Fully stable API with semantic versioning
- Complete Watermill feature parity
- Extensive test coverage and robust CI/CD pipeline