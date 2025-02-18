# Kincir

Kincir is a unified message streaming library for Rust that provides a consistent interface for working with multiple message broker backends. It simplifies the process of building message-driven applications by offering a clean, unified API across different messaging systems.

## Features

- Unified API for publishing and subscribing to messages
- Support for multiple message broker backends:
  - Apache Kafka
  - RabbitMQ
- Message routing with customizable handlers
- Built-in logging capabilities
- Message tracking with UUID generation
- Extensible message metadata

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
kincir = "0.1.0"
```

## Quick Start

### Using with Kafka

```rust
use kincir::kafka::{KafkaPublisher, KafkaSubscriber};
use kincir::router::StdLogger;
use kincir::{HandlerFunc, Message, Router};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Set up channels for Kafka communication
    let (tx, rx) = mpsc::channel(100);

    // Configure Kafka components
    let publisher = Arc::new(KafkaPublisher::new(
        vec!["localhost:9092".to_string()],
        tx,
        logger.clone(),
    ));

    let subscriber = Arc::new(KafkaSubscriber::new(
        vec!["localhost:9092".to_string()],
        "example-group".to_string(),
        rx,
        logger.clone(),
    ));

    // Define message handler
    let handler: HandlerFunc = Arc::new(|msg: Message| {
        Box::pin(async move {
            let processed_msg = msg.with_metadata("processed", "true");
            Ok(vec![processed_msg])
        })
    });

    // Create and run router
    let router = Router::new(
        logger,
        "input-topic".to_string(),
        "output-topic".to_string(),
        subscriber,
        publisher,
        handler,
    );

    router.run().await
}
```

### Using with RabbitMQ

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::router::StdLogger;
use kincir::{HandlerFunc, Message, Router};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Configure RabbitMQ components
    let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
    let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);

    // Define message handler
    let handler: HandlerFunc = Arc::new(|msg: Message| {
        Box::pin(async move {
            let processed_msg = msg.with_metadata("processed", "true");
            Ok(vec![processed_msg])
        })
    });

    // Create and run router
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

## Core Components

### Message

The `Message` struct represents a message in the system with:
- Unique UUID
- Payload as bytes
- Extensible metadata

### Router

The `Router` is the central component that:
- Subscribes to input topics
- Processes messages using provided handlers
- Publishes processed messages to output topics
- Handles logging and error management

### Publisher/Subscriber

Traits that define the interface for message broker implementations:
- `Publisher`: For sending messages to topics
- `Subscriber`: For receiving messages from topics

## License

This project is licensed under the MIT License.