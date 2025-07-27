---
layout: docs
title: Kincir - High-Performance Rust Message Streaming
description: Unified message streaming library for Rust with support for multiple broker backends
---

# Kincir

**Building event-driven applications the easy way in Rust**

Kincir is a unified message streaming library for Rust that provides a consistent interface for working with multiple message broker backends.

<div class="action-buttons">
  <a href="docs/getting-started.html" class="btn btn-primary">Get Started</a>
  <a href="https://github.com/rezacute/kincir" class="btn btn-secondary">View on GitHub</a>
  <a href="https://crates.io/crates/kincir" class="btn btn-secondary">Crates.io</a>
</div>

---

## Key Features

### 🔧 Unified Interface
A simple, consistent API for publishing and subscribing to messages across different messaging systems.

### 🚀 Multiple Backends
Support for Kafka, RabbitMQ, MQTT, and in-memory brokers with the same interface.

### 🔒 Message Acknowledgments
Comprehensive acknowledgment support across all backends for reliable message processing.

### 🎯 Event-Driven Architecture
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

### Basic Example

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

---

## Supported Backends

| Backend | Publisher | Subscriber | Acknowledgments | Status |
|---------|-----------|------------|-----------------|--------|
| **In-Memory** | ✅ | ✅ | ✅ | Stable |
| **RabbitMQ** | ✅ | ✅ | ✅ | Stable |
| **Kafka** | ✅ | ✅ | ✅ | Stable |
| **MQTT** | ✅ | ✅ | ✅ | Stable |

---

## Architecture

Kincir provides a unified interface that abstracts away the complexity of different message brokers:

```text
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│   Application   │    │    Kincir    │    │  Message Broker │
│                 │    │   Unified    │    │                 │
│  Publisher/     │◄──►│  Interface   │◄──►│  RabbitMQ/Kafka │
│  Subscriber     │    │              │    │  MQTT/Memory    │
└─────────────────┘    └──────────────┘    └─────────────────┘
```

---

## Why Kincir?

### vs. Direct Broker APIs
- **Unified Interface**: Switch between brokers without changing your application code
- **Simplified Development**: One API to learn instead of multiple broker-specific APIs
- **Future-Proof**: Add new brokers without changing existing code

### vs. Other Messaging Libraries
- **Rust-First**: Built specifically for Rust with zero-cost abstractions
- **Comprehensive**: Supports acknowledgments, routing, and advanced features
- **Performance**: No overhead compared to direct broker usage
- **Type Safety**: Leverages Rust's type system for safer message handling

### Comparison

- **vs. Watermill (Go)**: Similar feature set but with Rust's performance and safety
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
- ✅ Unit & integration tests for stability (65/65 tests passing)

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
- Stress testing and performance benchmarking  

### 🛠 v0.5 – Hardening & API Freeze  
- API finalization for stability  
- Cross-platform testing (Linux, macOS, Windows)  
- Memory optimization and async efficiency improvements  
- Comprehensive documentation and migration guide  

### 🚀 v1.0 – Production-Ready Release  
- High-performance, production-ready messaging library  
- Fully stable API with semantic versioning  
- Complete Watermill feature parity (middleware, observability, routing)  
- Extensive test coverage and robust CI/CD pipeline  
- Community engagement and ecosystem expansion  

---

## Getting Started

Ready to dive in? Check out our comprehensive documentation:

- **[Getting Started Guide](docs/getting-started.html)** - Complete setup and basic usage
- **[Examples](examples/)** - Comprehensive examples for all backends
- **[API Documentation](https://docs.rs/kincir)** - Full API reference

### Quick Links

- **Documentation**: [Getting Started](docs/getting-started.html)
- **Examples**: [Comprehensive Examples](examples/)
- **GitHub**: [Source Code](https://github.com/rezacute/kincir)
- **Crates.io**: [Package](https://crates.io/crates/kincir)
- **Issues**: [GitHub Issues](https://github.com/rezacute/kincir/issues)
- **Discussions**: [GitHub Discussions](https://github.com/rezacute/kincir/discussions)

---

## Ready to start building?

Check out the documentation to learn how to integrate Kincir into your Rust applications.

[Get Started](docs/getting-started.html) | [API Reference](https://docs.rs/kincir) | [Examples](examples/)

---

*Kincir is licensed under the Apache License, Version 2.0*
