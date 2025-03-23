# Kincir

<!-- 
GitHub Pages Setup:
1. Go to repository Settings > Pages
2. Under "Build and deployment" > "Source", select "GitHub Actions"
3. This will enable GitHub Pages for this repository
-->

[![Crates.io](https://img.shields.io/crates/v/kincir.svg)](https://crates.io/crates/kincir)
[![Documentation](https://docs.rs/kincir/badge.svg)](https://docs.rs/kincir)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Kincir is a Rust library that provides a unified interface for message streaming with support for multiple message broker backends. It offers a simple, consistent API for publishing and subscribing to messages across different messaging systems, with advanced routing capabilities.

## Features

- Unified messaging interface with support for multiple backends (Kafka, RabbitMQ)
- Message routing with customizable handlers
- Built-in logging support
- Message UUID generation for tracking and identification
- Customizable message metadata support
- Async/await support
- Type-safe error handling

## Installation

Add kincir to your `Cargo.toml`:

```toml
[dependencies]
kincir = "0.1.0"
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

The Router is a central component that handles message flow between publishers and subscribers:

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::router::{Router, Logger, StdLogger};
use kincir::Message;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Configure message brokers
    let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672"));
    let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672", "my-queue"));

    // Define message handler
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            // Process the message
            let mut processed_msg = msg;
            processed_msg.set_metadata("processed", "true");
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

// Configure Kafka publisher
let publisher = KafkaPublisher::new("localhost:9092");

// Configure Kafka subscriber
let subscriber = KafkaSubscriber::new("localhost:9092", "consumer-group-id");
```

### RabbitMQ

RabbitMQ support is available through the `rabbitmq` module:

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};

// Configure RabbitMQ publisher
let publisher = RabbitMQPublisher::new("amqp://localhost:5672");

// Configure RabbitMQ subscriber
let subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "my-queue");
```

## Message Structure

Each message in Kincir consists of:

- `uuid`: A unique identifier for the message
- `payload`: The actual message content as a byte vector
- `metadata`: A hash map of string key-value pairs for additional message information

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
Here's a **short and concise roadmap** for Kincir to be displayed in the README file:

---

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
- Comprehensive documentation and migration guide  

### 🚀 **v1.0 – Production-Ready Release**  
- High-performance, production-ready messaging library  
- Fully stable API with semantic versioning  
- Complete Watermill feature parity (middleware, observability, routing)  
- Extensive test coverage and robust CI/CD pipeline  
- Community engagement and ecosystem expansion  

For more details, visit [our roadmap](https://github.com/rezacute/kincir/projects) or contribute to the discussion!  


## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License

This project is licensed under the Apache License, Version 2.0 - see the [LICENSE](LICENSE) file for details.

Copyright 2024 Riza Alaudin Syah

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.