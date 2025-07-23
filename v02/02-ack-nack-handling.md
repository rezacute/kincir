# Task 2: Unified Ack/Nack Handling

## Overview
Implement unified acknowledgment (Ack) and negative acknowledgment (Nack) handling across all message broker backends. This ensures reliable message processing and consistent error handling patterns.

## Requirements

### Functional Requirements
- Extend `Subscriber` trait with Ack/Nack methods
- Implement Ack/Nack for all existing backends (Kafka, RabbitMQ, MQTT, In-Memory)
- Support automatic and manual acknowledgment modes
- Handle message redelivery on Nack
- Provide configurable retry policies
- Support dead letter queue patterns

### Non-Functional Requirements
- Consistent behavior across all backends
- Minimal performance impact
- Thread-safe operations
- Backward compatibility with existing code

## Technical Design

### Enhanced Subscriber Trait
```rust
#[async_trait]
pub trait Subscriber {
    type Error;
    type AckHandle;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error>;
    
    // Enhanced receive method returns message with ack handle
    async fn receive(&mut self) -> Result<(Message, Self::AckHandle), Self::Error>;
    
    // New acknowledgment methods
    async fn ack(&self, handle: Self::AckHandle) -> Result<(), Self::Error>;
    async fn nack(&self, handle: Self::AckHandle, requeue: bool) -> Result<(), Self::Error>;
    
    // Batch operations for efficiency
    async fn ack_batch(&self, handles: Vec<Self::AckHandle>) -> Result<(), Self::Error>;
    async fn nack_batch(&self, handles: Vec<Self::AckHandle>, requeue: bool) -> Result<(), Self::Error>;
}
```

### Acknowledgment Handle Types
```rust
// Generic acknowledgment handle
pub trait AckHandle: Send + Sync + Clone {
    fn message_id(&self) -> &str;
    fn topic(&self) -> &str;
    fn timestamp(&self) -> std::time::SystemTime;
}

// Backend-specific implementations
pub struct KafkaAckHandle {
    partition: i32,
    offset: i64,
    topic: String,
    message_id: String,
    timestamp: SystemTime,
}

pub struct RabbitMQAckHandle {
    delivery_tag: u64,
    channel_id: u16,
    topic: String,
    message_id: String,
    timestamp: SystemTime,
}

pub struct InMemoryAckHandle {
    message_id: String,
    topic: String,
    timestamp: SystemTime,
    broker_ref: Weak<InMemoryBroker>,
}
```

### Configuration
```rust
#[derive(Debug, Clone)]
pub struct AckConfig {
    pub mode: AckMode,
    pub timeout: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub dead_letter_topic: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AckMode {
    Auto,           // Automatic acknowledgment after receive
    Manual,         // Manual acknowledgment required
    ClientAuto,     // Auto-ack after successful handler execution
}
```

## Implementation Tasks

### Phase 1: Core Infrastructure (Day 1-2)
- [ ] Design and implement enhanced `Subscriber` trait
- [ ] Create `AckHandle` trait and backend-specific implementations
- [ ] Implement `AckConfig` and `AckMode` types
- [ ] Add acknowledgment-related error types
- [ ] Create backward compatibility layer for existing `receive()` method

### Phase 2: In-Memory Backend Implementation (Day 2-3)
- [ ] Implement `InMemoryAckHandle`
- [ ] Add acknowledgment tracking to `InMemoryBroker`
- [ ] Implement message redelivery logic
- [ ] Add timeout handling for unacknowledged messages
- [ ] Implement dead letter queue support

### Phase 3: RabbitMQ Backend Implementation (Day 3-4)
- [ ] Implement `RabbitMQAckHandle`
- [ ] Update `RabbitMQSubscriber` with ack/nack methods
- [ ] Handle RabbitMQ-specific acknowledgment semantics
- [ ] Implement batch acknowledgment optimization
- [ ] Add dead letter exchange configuration

### Phase 4: Kafka Backend Implementation (Day 4-5)
- [ ] Implement `KafkaAckHandle`
- [ ] Update `KafkaSubscriber` with manual commit support
- [ ] Handle offset management for ack/nack operations
- [ ] Implement retry topic patterns
- [ ] Add consumer group coordination for acknowledgments

### Phase 5: MQTT Backend Implementation (Day 5-6)
- [ ] Implement `MQTTAckHandle`
- [ ] Add QoS-aware acknowledgment handling
- [ ] Implement message persistence for QoS > 0
- [ ] Handle connection recovery scenarios
- [ ] Add MQTT-specific retry mechanisms

### Phase 6: Router Integration (Day 6)
- [ ] Update `Router` to handle acknowledgment patterns
- [ ] Add automatic ack/nack based on handler success/failure
- [ ] Implement configurable acknowledgment strategies
- [ ] Add metrics for acknowledgment rates

## Testing Strategy

### Unit Tests
- [ ] Test ack/nack operations for each backend
- [ ] Test timeout and retry mechanisms
- [ ] Test dead letter queue functionality
- [ ] Test batch acknowledgment operations
- [ ] Test error scenarios and edge cases

### Integration Tests
- [ ] Test cross-backend acknowledgment consistency
- [ ] Test Router integration with ack/nack
- [ ] Test high-throughput acknowledgment scenarios
- [ ] Test connection recovery with pending acks

### Compatibility Tests
- [ ] Ensure backward compatibility with existing code
- [ ] Test migration path from old to new API
- [ ] Verify performance impact is minimal

## File Structure
```
kincir/src/
├── ack/
│   ├── mod.rs
│   ├── handle.rs
│   ├── config.rs
│   └── error.rs
├── memory/
│   └── subscriber.rs (updated)
├── rabbitmq.rs (updated)
├── kafka.rs (updated)
├── mqtt.rs (updated)
└── lib.rs (updated trait export)
```

## Example Usage

### Manual Acknowledgment
```rust
use kincir::{Subscriber, AckMode, AckConfig};
use kincir::rabbitmq::RabbitMQSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AckConfig {
        mode: AckMode::Manual,
        timeout: Duration::from_secs(30),
        max_retries: 3,
        retry_delay: Duration::from_secs(5),
        dead_letter_topic: Some("dead-letters".to_string()),
    };
    
    let mut subscriber = RabbitMQSubscriber::with_ack_config(
        "amqp://localhost:5672",
        config
    ).await?;
    
    subscriber.subscribe("my-topic").await?;
    
    loop {
        let (message, ack_handle) = subscriber.receive().await?;
        
        match process_message(message).await {
            Ok(_) => {
                subscriber.ack(ack_handle).await?;
                println!("Message processed successfully");
            }
            Err(e) => {
                println!("Processing failed: {}", e);
                subscriber.nack(ack_handle, true).await?; // requeue
            }
        }
    }
}
```

### Router with Auto-Ack
```rust
use kincir::router::Router;
use kincir::{AckMode, AckConfig};

let ack_config = AckConfig {
    mode: AckMode::ClientAuto,
    timeout: Duration::from_secs(60),
    max_retries: 5,
    retry_delay: Duration::from_secs(10),
    dead_letter_topic: Some("failed-messages".to_string()),
};

let router = Router::with_ack_config(
    logger,
    "input-topic".to_string(),
    "output-topic".to_string(),
    subscriber,
    publisher,
    handler,
    ack_config,
);

// Router will automatically ack on successful handler execution
// and nack on handler errors
router.run().await?;
```

## Migration Guide

### Backward Compatibility
The existing `receive()` method will be preserved with automatic acknowledgment:

```rust
// Old API (still works)
let message = subscriber.receive().await?;

// New API (recommended)
let (message, ack_handle) = subscriber.receive_with_ack().await?;
subscriber.ack(ack_handle).await?;
```

### Configuration Migration
```rust
// Before
let subscriber = RabbitMQSubscriber::new("amqp://localhost:5672").await?;

// After (with ack configuration)
let config = AckConfig::default().with_mode(AckMode::Manual);
let subscriber = RabbitMQSubscriber::with_ack_config("amqp://localhost:5672", config).await?;
```

## Success Criteria
- [ ] All backends implement consistent ack/nack behavior
- [ ] Backward compatibility maintained
- [ ] Performance impact < 5% for basic operations
- [ ] Dead letter queue functionality works across backends
- [ ] Comprehensive test coverage (>85%)
- [ ] Documentation covers all acknowledgment patterns
- [ ] Router integration handles ack/nack automatically

## Dependencies
- Existing backend dependencies (rdkafka, lapin, rumqttc)
- `std::time` for timeout handling
- `std::sync::Weak` for weak references
- `tokio::time` for async timeouts

## Documentation Requirements
- [ ] Update trait documentation with ack/nack examples
- [ ] Create acknowledgment patterns guide
- [ ] Document backend-specific behavior differences
- [ ] Add troubleshooting guide for acknowledgment issues
- [ ] Update README with acknowledgment examples
