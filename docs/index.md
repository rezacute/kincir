# Kincir

**Building event-driven applications the easy way in Rust**

Kincir is a unified message streaming library for Rust that provides a consistent interface for working with multiple message broker backends.

[Get Started](/docs/) | [View on GitHub](https://github.com/rezacute/kincir) | [Crates.io](https://crates.io/crates/kincir)

---

## Key Features

### 🔧 Unified Interface
A simple, consistent API for publishing and subscribing to messages across different messaging systems.

### 👥 Multiple Backends  
Support for Kafka, RabbitMQ, MQTT, and in-memory brokers with a single, consistent API.

### 📡 Message Routing
Powerful message routing capabilities with customizable handlers for complex event processing.

### ⚙️ Optional Features
Customize your build with optional feature flags for logging, Protocol Buffers support, and more.

### 🔄 Event-Driven Architecture
Build robust event-driven applications with reliable message passing and processing.

### 📊 High Performance
Designed for performance with Rust's safety guarantees and zero-cost abstractions.

---

## Quick Start

Add Kincir to your `Cargo.toml`:

```toml
[dependencies]
kincir = "0.2.0"
```

### Basic Usage

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create an in-memory broker
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());

    // Subscribe to a topic
    subscriber.subscribe("orders").await?;
    
    // Publish a message
    let message = Message::new(b"Order #1234".to_vec());
    publisher.publish("orders", vec![message]).await?;
    
    // Receive the message
    let received = subscriber.receive().await?;
    println!("Received: {:?}", received);
    
    Ok(())
}
```

---

## What's New in v0.2.0

### ✅ In-Memory Message Broker
- Zero-dependency, high-performance broker for testing and lightweight production
- Sub-millisecond message delivery latency (2-3µs average)
- Handles 100,000+ messages/second throughput
- Thread-safe concurrent operations with deadlock resolution

### ✅ Message Acknowledgments
- Comprehensive acknowledgment support across RabbitMQ, Kafka, and MQTT backends
- Reliable message processing with ack/nack capabilities
- Error handling and retry mechanisms

### ✅ MQTT Support
- Full MQTT implementation with Quality of Service (QoS) handling
- Perfect for IoT and real-time applications
- MQTT to RabbitMQ tunneling support

### ✅ Advanced Features
- Message ordering and TTL (Time-To-Live)
- Health monitoring and comprehensive statistics
- Built-in logging support with customizable levels
- Message UUID generation for tracking

---

## Supported Backends

| Backend | Status | Features |
|---------|--------|----------|
| **In-Memory** | ✅ Complete | High-performance, zero-dependency, testing-friendly |
| **RabbitMQ** | ✅ Complete | AMQP protocol, acknowledgments, routing |
| **Kafka** | ✅ Complete | High-throughput, partitioning, consumer groups |
| **MQTT** | ✅ Complete | IoT-focused, QoS levels, lightweight |
| **NATS** | 🔄 Planned | Cloud-native messaging |
| **AWS SQS** | 🔄 Planned | Managed queue service |

---

## Architecture

Kincir provides a unified interface that abstracts away the complexity of different message brokers:

```
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│   Application   │    │    Kincir    │    │  Message Broker │
│                 │    │   Unified    │    │                 │
│  Publisher/     │◄──►│  Interface   │◄──►│  RabbitMQ/Kafka │
│  Subscriber     │    │              │    │  MQTT/Memory    │
└─────────────────┘    └──────────────┘    └─────────────────┘
```

---

## Performance Benchmarks

### In-Memory Broker Performance
- **Latency**: 2-3µs average message delivery
- **Throughput**: 100,000+ messages/second
- **Memory Usage**: Minimal overhead with efficient data structures
- **Concurrency**: Full thread-safety with deadlock resolution

### Comparison with Other Solutions
- **vs. Watermill (Go)**: Feature parity with better performance
- **vs. Direct Broker APIs**: Simplified interface with no performance penalty
- **vs. Other Rust Libraries**: More comprehensive feature set

---

## Roadmap to v1.0 🚀

Kincir is evolving towards **feature parity with Watermill (Golang)** while leveraging Rust's performance and safety.

### ✅ v0.2 – Core Enhancements *(COMPLETED)*
- ✅ In-memory message broker for local testing  
- ✅ Advanced features: message ordering, TTL, health monitoring
- ✅ Comprehensive statistics and performance metrics
- ✅ Thread-safe concurrent operations with deadlock resolution

### 🔄 v0.3 – Middleware & Backend Expansion *(IN PROGRESS)*
- Middleware framework: logging, retry, recovery, correlation  
- Additional broker support (NATS, AWS SQS)  
- Optimized async pipeline for lower latency  

### 📊 v0.4 – Distributed Tracing & Monitoring
- OpenTelemetry-based tracing for message flows  
- Prometheus metrics for message processing  
- Poison queue (dead-letter handling)  
- Throttling & backpressure support  

### 🚀 v1.0 – Production-Ready Release
- High-performance, production-ready messaging library  
- Fully stable API with semantic versioning  
- Complete Watermill feature parity  
- Extensive test coverage and robust CI/CD pipeline  

---

## Community & Support

- **Documentation**: [docs.rs/kincir](https://docs.rs/kincir)
- **Repository**: [github.com/rezacute/kincir](https://github.com/rezacute/kincir)
- **Crate**: [crates.io/crates/kincir](https://crates.io/crates/kincir)
- **Issues**: [GitHub Issues](https://github.com/rezacute/kincir/issues)
- **Discussions**: [GitHub Discussions](https://github.com/rezacute/kincir/discussions)

---

## Ready to start building?

Check out the documentation to learn how to integrate Kincir into your Rust applications.

[Get Started](/docs/) | [API Reference](/docs/api/) | [Examples](/docs/examples/)

---

*Kincir is licensed under the Apache License, Version 2.0*
