---
layout: default
title: Backends
nav_order: 4
has_children: true
---

# Message Broker Backends
{: .no_toc }

Kincir provides a unified interface for working with multiple message broker backends. Each backend implements the same `Publisher` and `Subscriber` traits, allowing you to switch between different brokers with minimal code changes.

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Available Backends

### In-Memory Broker
{: .d-inline-block }

Production Ready
{: .label .label-green }

The in-memory broker provides a lightweight, high-performance message broker that runs entirely within your application's memory space. Perfect for testing, development, and lightweight production scenarios.

**Key Features:**
- Zero external dependencies
- Sub-millisecond latency
- Message ordering and TTL support
- Health monitoring and statistics
- Thread-safe concurrent operations

[View In-Memory Documentation]({{ '/docs/backends/in-memory/' | relative_url }}){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }

### Kafka
{: .d-inline-block }

Coming Soon
{: .label .label-yellow }

Apache Kafka backend for high-throughput, distributed message streaming.

**Key Features:**
- High throughput and low latency
- Distributed and fault-tolerant
- Message persistence and replay
- Consumer groups and partitioning

### RabbitMQ
{: .d-inline-block }

Coming Soon
{: .label .label-yellow }

RabbitMQ backend for reliable message queuing with advanced routing.

**Key Features:**
- Reliable message delivery
- Flexible routing and exchanges
- Message acknowledgments
- Dead letter queues

### NATS
{: .d-inline-block }

Planned
{: .label .label-blue }

NATS backend for cloud-native messaging.

**Key Features:**
- Lightweight and fast
- Cloud-native design
- Subject-based messaging
- JetStream persistence

## Choosing a Backend

### Development and Testing

For development and testing scenarios, the **in-memory broker** is the ideal choice:

```rust
use kincir::memory::{InMemoryBroker, InMemoryConfig};

// Perfect for tests
let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));

// Great for development
let broker = Arc::new(InMemoryBroker::with_default_config());
```

**Advantages:**
- No external dependencies to set up
- Fast test execution
- Deterministic behavior
- Easy debugging with built-in statistics

### Production Scenarios

#### Lightweight Applications

For lightweight production applications with moderate message volumes:

```rust
use kincir::memory::{InMemoryBroker, InMemoryConfig};

let config = InMemoryConfig::high_performance()
    .with_message_ttl(Some(Duration::from_secs(1800))) // 30-minute TTL
    .with_max_queue_size(Some(10000));

let broker = Arc::new(InMemoryBroker::new(config));
```

#### High-Throughput Applications

For high-throughput, distributed applications, consider Kafka:

```rust
// Coming soon
use kincir::kafka::KafkaPublisher;
let publisher = KafkaPublisher::new("localhost:9092");
```

#### Reliable Message Queuing

For applications requiring guaranteed message delivery, consider RabbitMQ:

```rust
// Coming soon
use kincir::rabbitmq::RabbitMQPublisher;
let publisher = RabbitMQPublisher::new("amqp://localhost:5672");
```

## Backend Comparison

| Feature | In-Memory | Kafka | RabbitMQ | NATS |
|---------|-----------|-------|----------|------|
| **Setup Complexity** | None | Medium | Medium | Low |
| **External Dependencies** | None | Zookeeper | None | None |
| **Persistence** | Memory only | Disk | Disk | Optional |
| **Distribution** | Single process | Multi-node | Multi-node | Multi-node |
| **Throughput** | Very High | Very High | High | Very High |
| **Latency** | Sub-ms | Low | Low | Very Low |
| **Message Ordering** | ✅ | ✅ | ✅ | ✅ |
| **Message TTL** | ✅ | ✅ | ✅ | ✅ |
| **Health Monitoring** | ✅ | ✅ | ✅ | ✅ |
| **Best For** | Testing, Dev, Lightweight | Big Data, Streaming | Enterprise, Reliability | Cloud-native, IoT |

## Unified API

All backends implement the same core traits, making it easy to switch between them:

```rust
use kincir::{Publisher, Subscriber, Message};

// Generic function that works with any backend
async fn send_message<P: Publisher>(
    publisher: &P, 
    topic: &str, 
    data: &[u8]
) -> Result<(), P::Error> {
    let message = Message::new(data.to_vec());
    publisher.publish(topic, vec![message]).await
}

// Works with in-memory broker
let in_memory_publisher = InMemoryPublisher::new(broker);
send_message(&in_memory_publisher, "topic", b"data").await?;

// Will work with Kafka (when available)
// let kafka_publisher = KafkaPublisher::new("localhost:9092");
// send_message(&kafka_publisher, "topic", b"data").await?;
```

## Migration Between Backends

Switching between backends typically requires only configuration changes:

```rust
// Development with in-memory broker
#[cfg(debug_assertions)]
fn create_publisher() -> impl Publisher {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    InMemoryPublisher::new(broker)
}

// Production with Kafka
#[cfg(not(debug_assertions))]
fn create_publisher() -> impl Publisher {
    // KafkaPublisher::new("production-kafka:9092")
    // Coming soon...
    let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::high_performance()));
    InMemoryPublisher::new(broker)
}
```

## Getting Started

1. **Start with the in-memory broker** for development and testing
2. **Choose the appropriate backend** for your production requirements
3. **Use the unified API** to keep your code backend-agnostic
4. **Migrate easily** when your requirements change

[Get Started with In-Memory Broker]({{ '/docs/backends/in-memory/' | relative_url }}){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View All Documentation]({{ '/docs/' | relative_url }}){: .btn .btn-outline .fs-5 .mb-4 .mb-md-0 }
