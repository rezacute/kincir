# Getting Started with Kincir

Welcome to Kincir! This guide will help you get up and running with Kincir's unified message streaming interface.

## Installation

Add Kincir to your `Cargo.toml`:

```toml
[dependencies]
kincir = "0.2.0"
```

### Feature Flags

Kincir supports optional features that you can enable based on your needs:

```toml
[dependencies]
kincir = { version = "0.2.0", features = ["rabbitmq", "kafka", "mqtt", "logging"] }
```

Available features:
- `rabbitmq` - RabbitMQ backend support
- `kafka` - Apache Kafka backend support  
- `mqtt` - MQTT broker support
- `logging` - Built-in logging capabilities
- `protobuf` - Protocol Buffers message serialization

## Quick Start

### 1. In-Memory Broker (Recommended for Testing)

The in-memory broker is perfect for testing and development:

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

### 2. RabbitMQ Backend

For production use with RabbitMQ:

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::{Publisher, Subscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create RabbitMQ publisher and subscriber
    let publisher = RabbitMQPublisher::new("amqp://localhost:5672");
    let mut subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "my-queue");

    // Subscribe to an exchange
    subscriber.subscribe("user-events").await?;
    
    // Publish a message
    let message = Message::new(b"User registered".to_vec())
        .with_metadata("event_type", "user_registration")
        .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339());
    
    publisher.publish("user-events", vec![message]).await?;
    
    // Receive and process messages
    loop {
        let message = subscriber.receive().await?;
        println!("Processing: {:?}", message);
        // Process your message here
    }
}
```

### 3. Kafka Backend

For high-throughput scenarios with Kafka:

```rust
use kincir::kafka::{KafkaPublisher, KafkaSubscriber};
use kincir::{Publisher, Subscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create Kafka publisher and subscriber
    let publisher = KafkaPublisher::new("localhost:9092");
    let mut subscriber = KafkaSubscriber::new("localhost:9092", "consumer-group");

    // Subscribe to a topic
    subscriber.subscribe("orders").await?;
    
    // Publish messages
    let messages = vec![
        Message::new(b"Order #1001".to_vec()),
        Message::new(b"Order #1002".to_vec()),
        Message::new(b"Order #1003".to_vec()),
    ];
    
    publisher.publish("orders", messages).await?;
    
    // Process messages
    loop {
        let message = subscriber.receive().await?;
        println!("Processing order: {:?}", message);
    }
}
```

## Message Structure

Every message in Kincir has the following structure:

```rust
pub struct Message {
    pub uuid: String,           // Unique identifier
    pub payload: Vec<u8>,       // Message content
    pub metadata: HashMap<String, String>, // Additional metadata
}
```

### Creating Messages

```rust
use kincir::Message;

// Basic message
let msg = Message::new(b"Hello".to_vec());

// Message with metadata
let msg = Message::new(b"User data".to_vec())
    .with_metadata("user_id", "12345")
    .with_metadata("action", "login")
    .with_metadata("timestamp", "2025-07-25T20:00:00Z");

// Access message properties
println!("Message ID: {}", msg.uuid);
println!("Payload: {:?}", String::from_utf8_lossy(&msg.payload));
println!("User ID: {:?}", msg.get_metadata("user_id"));
```

## Error Handling

Kincir provides comprehensive error handling:

```rust
use kincir::{Publisher, KincirError};

async fn publish_with_error_handling() -> Result<(), KincirError> {
    let publisher = InMemoryPublisher::new(broker);
    let message = Message::new(b"test".to_vec());
    
    match publisher.publish("topic", vec![message]).await {
        Ok(()) => println!("Message published successfully"),
        Err(KincirError::ConnectionError(e)) => {
            eprintln!("Connection failed: {}", e);
            // Handle connection error
        },
        Err(KincirError::SerializationError(e)) => {
            eprintln!("Serialization failed: {}", e);
            // Handle serialization error
        },
        Err(e) => {
            eprintln!("Other error: {}", e);
            // Handle other errors
        }
    }
    
    Ok(())
}
```

## Configuration

### In-Memory Broker Configuration

```rust
use kincir::memory::{InMemoryBrokerConfig, InMemoryBroker};

let config = InMemoryBrokerConfig {
    max_messages_per_topic: 10000,
    message_ttl_seconds: Some(3600), // 1 hour TTL
    enable_message_ordering: true,
    enable_statistics: true,
};

let broker = InMemoryBroker::with_config(config);
```

### RabbitMQ Configuration

```rust
use kincir::rabbitmq::{RabbitMQConfig, RabbitMQPublisher};

let config = RabbitMQConfig {
    connection_url: "amqp://user:pass@localhost:5672".to_string(),
    exchange_name: "my-exchange".to_string(),
    exchange_type: "topic".to_string(),
    durable: true,
    auto_delete: false,
};

let publisher = RabbitMQPublisher::with_config(config);
```

## Next Steps

- [Examples](/examples/) - See more detailed examples
- [API Reference](https://docs.rs/kincir) - Complete API documentation
- [Architecture](/docs/architecture.html) - Learn about Kincir's design
- [Best Practices](/docs/best-practices.html) - Production deployment tips

## Need Help?

- [GitHub Issues](https://github.com/rezacute/kincir/issues) - Report bugs or request features
- [GitHub Discussions](https://github.com/rezacute/kincir/discussions) - Ask questions and share ideas
- [Documentation](https://docs.rs/kincir) - Complete API reference
