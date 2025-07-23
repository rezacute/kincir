# Task 1: In-Memory Message Broker

## Overview
Implement an in-memory message broker for local testing and development. This will provide a lightweight, fast alternative to external brokers during development and testing.

## Requirements

### Functional Requirements
- Implement `Publisher` and `Subscriber` traits for in-memory operations
- Support multiple topics/queues simultaneously
- Thread-safe operations for concurrent access
- Message persistence during application lifetime
- Support for message ordering within topics
- Configurable queue size limits

### Non-Functional Requirements
- High performance (minimal latency)
- Memory efficient
- Thread-safe
- No external dependencies

## Technical Design

### Core Components

#### 1. InMemoryBroker
```rust
pub struct InMemoryBroker {
    topics: Arc<RwLock<HashMap<String, VecDeque<Message>>>>,
    subscribers: Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Message>>>>>,
    config: InMemoryConfig,
}

pub struct InMemoryConfig {
    pub max_queue_size: Option<usize>,
    pub max_topics: Option<usize>,
    pub enable_persistence: bool,
}
```

#### 2. InMemoryPublisher
```rust
pub struct InMemoryPublisher {
    broker: Arc<InMemoryBroker>,
}

impl Publisher for InMemoryPublisher {
    type Error = InMemoryError;
    
    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error>;
}
```

#### 3. InMemorySubscriber
```rust
pub struct InMemorySubscriber {
    broker: Arc<InMemoryBroker>,
    receiver: Option<mpsc::UnboundedReceiver<Message>>,
    subscribed_topic: Option<String>,
}

impl Subscriber for InMemorySubscriber {
    type Error = InMemoryError;
    
    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error>;
    async fn receive(&mut self) -> Result<Message, Self::Error>;
}
```

### Error Handling
```rust
#[derive(Debug, thiserror::Error)]
pub enum InMemoryError {
    #[error("Topic not found: {topic}")]
    TopicNotFound { topic: String },
    
    #[error("Queue full for topic: {topic}")]
    QueueFull { topic: String },
    
    #[error("Not subscribed to any topic")]
    NotSubscribed,
    
    #[error("Broker shutdown")]
    BrokerShutdown,
}
```

## Implementation Tasks

### Phase 1: Core Infrastructure (Day 1-2)
- [ ] Create `InMemoryBroker` struct with basic topic management
- [ ] Implement thread-safe topic storage using `Arc<RwLock<HashMap>>`
- [ ] Add configuration struct for broker settings
- [ ] Implement basic error types

### Phase 2: Publisher Implementation (Day 2-3)
- [ ] Implement `InMemoryPublisher` struct
- [ ] Implement `Publisher` trait for `InMemoryPublisher`
- [ ] Add message validation and topic creation
- [ ] Implement queue size limits
- [ ] Add metrics collection (message count, topic count)

### Phase 3: Subscriber Implementation (Day 3-4)
- [ ] Implement `InMemorySubscriber` struct
- [ ] Implement `Subscriber` trait for `InMemorySubscriber`
- [ ] Add subscription management
- [ ] Implement message delivery using channels
- [ ] Handle subscriber disconnection cleanup

### Phase 4: Advanced Features (Day 4-5)
- [ ] Add message ordering guarantees
- [ ] Implement topic pattern matching (optional)
- [ ] Add broker statistics and monitoring
- [ ] Implement graceful shutdown
- [ ] Add message TTL support (optional)

## Testing Strategy

### Unit Tests
- [ ] Test basic publish/subscribe operations
- [ ] Test concurrent access scenarios
- [ ] Test queue size limits
- [ ] Test error conditions
- [ ] Test subscriber cleanup

### Integration Tests
- [ ] Test with Router component
- [ ] Test message ordering
- [ ] Test high-throughput scenarios
- [ ] Test memory usage patterns

### Benchmarks
- [ ] Publish throughput benchmarks
- [ ] Subscribe latency benchmarks
- [ ] Memory usage benchmarks
- [ ] Concurrent access benchmarks

## File Structure
```
kincir/src/
├── memory/
│   ├── mod.rs
│   ├── broker.rs
│   ├── publisher.rs
│   ├── subscriber.rs
│   └── error.rs
└── lib.rs (add memory module export)
```

## Example Usage
```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber, InMemoryConfig};
use kincir::{Publisher, Subscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create broker with configuration
    let config = InMemoryConfig {
        max_queue_size: Some(1000),
        max_topics: Some(100),
        enable_persistence: true,
    };
    
    let broker = Arc::new(InMemoryBroker::new(config));
    
    // Create publisher and subscriber
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    // Subscribe to topic
    subscriber.subscribe("test-topic").await?;
    
    // Publish messages
    let messages = vec![Message::new(b"Hello, World!".to_vec())];
    publisher.publish("test-topic", messages).await?;
    
    // Receive message
    let received = subscriber.receive().await?;
    println!("Received: {:?}", received);
    
    Ok(())
}
```

## Success Criteria
- [ ] All Publisher/Subscriber trait methods implemented
- [ ] Thread-safe concurrent access
- [ ] Memory usage stays bounded
- [ ] Performance benchmarks show < 1ms latency for basic operations
- [ ] Integration with existing Router works seamlessly
- [ ] Comprehensive test coverage (>90%)

## Dependencies
- `tokio` (already in Cargo.toml)
- `std::collections::HashMap, VecDeque`
- `std::sync::{Arc, RwLock}`
- `tokio::sync::mpsc`

## Documentation Requirements
- [ ] API documentation for all public types
- [ ] Usage examples in module documentation
- [ ] Performance characteristics documentation
- [ ] Configuration options documentation
- [ ] Update main README with in-memory broker example
