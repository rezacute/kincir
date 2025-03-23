# Kincir

[![Crates.io](https://img.shields.io/crates/v/kincir.svg)](https://crates.io/crates/kincir)
[![Documentation](https://docs.rs/kincir/badge.svg)](https://docs.rs/kincir)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Kincir is a Rust library that provides a unified interface for message streaming with support for multiple message broker backends. It offers a simple, consistent API for publishing and subscribing to messages across different messaging systems, with advanced routing capabilities.

**[📚 Online Documentation](https://rezacute.github.io/kincir/) | [🦀 Crates.io](https://crates.io/crates/kincir) | [💻 GitHub Repository](https://github.com/rezacute/kincir)**

## Features

- Unified messaging interface with support for multiple backends (Kafka, RabbitMQ)
- Message routing with customizable handlers
- Built-in logging support (optional via feature flag)
- Message UUID generation for tracking and identification
- Customizable message metadata support
- Async/await support
- Type-safe error handling

## Installation

Add kincir to your `Cargo.toml`:

```toml
[dependencies]
kincir = "0.1.6"
```

### Feature Flags

Kincir provides feature flags to customize the library:

```toml
[dependencies]
# Default features (includes logging)
kincir = "0.1.6"

# Without logging
kincir = { version = "0.1.6", default-features = false }

# Explicitly enable logging
kincir = { version = "0.1.6", features = ["logging"] }

# With Protocol Buffers support
kincir = { version = "0.1.6", features = ["protobuf"] }

# With both logging and Protocol Buffers
kincir = { version = "0.1.6", features = ["logging", "protobuf"] }
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

## Usage

### Basic Message Creation

```rust
use kincir::Message;

// Create a new message with payload
let payload = b"Hello, World!".to_vec();
let message = Message::new(payload);

// Add metadata to the message
let message = message.with_metadata("content-type", "text/plain");
```

### Setting Up a Message Router

The Router is a central component that handles message flow between publishers and subscribers.

#### With Logging (Default)

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::logging::{Logger, StdLogger};
use kincir::router::Router;
use kincir::Message;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Configure message brokers
    let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
    let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);

    // Define message handler
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            // Process the message
            let mut processed_msg = msg;
            processed_msg.set_metadata("processed", "true");
            Ok(vec![processed_msg])
        })
    });

    // Create and run router with logger
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

#### Without Logging

When the `logging` feature is disabled, the Router is used without a logger:

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::router::Router;
use kincir::Message;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure message brokers
    let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
    let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);

    // Define message handler
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            // Process the message
            let mut processed_msg = msg;
            processed_msg.set_metadata("processed", "true");
            Ok(vec![processed_msg])
        })
    });

    // Create and run router without logger
    let router = Router::new(
        "input-exchange".to_string(),
        "output-exchange".to_string(),
        subscriber,
        publisher,
        handler,
    );

    router.run().await
}
```

### Publishing Messages

```rust
use kincir::Publisher;

// Create messages to publish
let messages = vec![Message::new(b"Message 1".to_vec()), Message::new(b"Message 2".to_vec())];

// Publish messages to a topic
async fn publish_example<P: Publisher>(publisher: &P) -> Result<(), P::Error> {
    publisher.publish("my-topic", messages).await
}
```

### Subscribing to Messages

```rust
use kincir::Subscriber;

// Subscribe and receive messages
async fn subscribe_example<S: Subscriber>(subscriber: &S) -> Result<(), S::Error> {
    // Subscribe to a topic
    subscriber.subscribe("my-topic").await?;
    
    // Receive messages
    loop {
        let message = subscriber.receive().await?;
        println!("Received message: {:?}", message);
    }
}
```

## Backend Implementations

### Kafka

Kincir provides Kafka support through the `kafka` module:

```rust
use kincir::kafka::{KafkaPublisher, KafkaSubscriber};
use tokio::sync::mpsc;

// Set up channels
let (tx, rx) = mpsc::channel(100);

// Configure Kafka publisher and subscriber (default with logging)
let publisher = KafkaPublisher::new(
    vec!["localhost:9092".to_string()],
    tx,
    logger.clone(), // Only needed with logging feature
);

let subscriber = KafkaSubscriber::new(
    vec!["localhost:9092".to_string()],
    "consumer-group-id".to_string(),
    rx,
    logger, // Only needed with logging feature
);

// Without logging feature, the logger parameter is not needed
```

### RabbitMQ

RabbitMQ support is available through the `rabbitmq` module:

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};

// Configure RabbitMQ components
let publisher = RabbitMQPublisher::new("amqp://localhost:5672").await?;
let subscriber = RabbitMQSubscriber::new("amqp://localhost:5672").await?;

// With logging feature, you can optionally add a logger
// publisher = publisher.with_logger(logger.clone());
// subscriber = subscriber.with_logger(logger);
```

## Message Structure

Each message in Kincir consists of:

- `uuid`: A unique identifier for the message
- `payload`: The actual message content as a byte vector
- `metadata`: A hash map of string key-value pairs for additional message information

## Message Encoding and Decoding

### Protocol Buffers Support

When the `protobuf` feature is enabled, Kincir provides Protocol Buffers encoding/decoding capabilities:

```rust
use kincir::Message;
use kincir::protobuf::{MessageCodec, ProtobufCodec};

// Create a protobuf codec
let codec = ProtobufCodec::new();

// Create a message
let message = Message::new(b"Hello".to_vec())
    .with_metadata("content-type", "text/plain");

// Encode the message to send over the wire
let encoded = codec.encode(&message).unwrap();

// Later, decode the message
let decoded = codec.decode(&encoded).unwrap();
```

## Message Handler

Message handlers are async functions that process incoming messages and can produce zero or more output messages:

```rust
use kincir::Message;

// Define a message handler
let handler = |msg: Message| {
    Box::pin(async move {
        // Process the message
        let mut processed_msg = msg;
        processed_msg.set_metadata("processed", "true");
        Ok(vec![processed_msg])
    })
};
```

### Protocol Buffers Support

When the `protobuf` feature flag is enabled, Kincir provides support for encoding and decoding messages using Protocol Buffers through the `MessageCodec` trait:

```rust
#[cfg(feature = "protobuf")]
use kincir::{Message, MessageCodec, ProtobufCodec};

// Create a message
let message = Message::new(b"Hello, Protocol Buffers!".to_vec())
    .with_metadata("encoding", "protobuf");

// Create a Protocol Buffers codec
let codec = ProtobufCodec::new();

// Encode the message to Protocol Buffers binary format
let encoded = codec.encode(&message).unwrap();

// Decode the binary data back to a Message
let decoded = codec.decode(&encoded).unwrap();

assert_eq!(message.uuid, decoded.uuid);
assert_eq!(message.payload, decoded.payload);
assert_eq!(message.metadata, decoded.metadata);
```

This is particularly useful when you need:
- Smaller message size compared to JSON
- Stricter schema validation
- Better performance for serialization and deserialization
- Language-agnostic message exchange

## Roadmap to v1.0 🚀  

Kincir is evolving towards **feature parity with Watermill (Golang)** while leveraging Rust's performance and safety. Below is our roadmap:

### ✅ **v0.2 – Core Enhancements**  
- In-memory message broker for local testing  
- Unified Ack/Nack handling across backends  
- Correlation ID tracking for tracing  
- Performance profiling and initial benchmarks  
- Unit & integration tests for stability  

### 🔄 **v0.3 – Middleware & Backend Expansion**  
- Middleware framework: logging, retry, recovery, correlation  
- Additional broker support (e.g., NATS, AWS SQS)  
- Optimized async pipeline for lower latency  
- Integration tests for middleware + new backends  

### 📊 **v0.4 – Distributed Tracing & Monitoring**  
- OpenTelemetry-based tracing for message flows  
- Prometheus metrics for message processing  
- Poison queue (dead-letter handling)  
- Throttling & backpressure support  
- Stress testing and performance benchmarking  

### 🛠 **v0.5 – Hardening & API Freeze**  
- API finalization for stability  
- Cross-platform testing (Linux, macOS, Windows)  
- Memory optimization and async efficiency improvements