# Task 5: Unit & Integration Tests for Stability

## Overview
Establish comprehensive testing infrastructure to ensure stability and reliability of all v0.2 features. This includes unit tests, integration tests, property-based tests, and end-to-end testing scenarios.

## Requirements

### Functional Requirements
- Unit tests for all new modules and features
- Integration tests for cross-component interactions
- End-to-end tests for complete message flows
- Property-based testing for edge cases
- Test coverage > 80% for all core modules
- Automated test execution in CI/CD

### Non-Functional Requirements
- Fast test execution (< 5 minutes for full suite)
- Reliable and deterministic tests
- Clear test failure reporting
- Easy test environment setup
- Parallel test execution support

## Technical Design

### Test Categories

#### 1. Unit Tests
- Individual component testing
- Mock-based isolation
- Edge case validation
- Error condition testing

#### 2. Integration Tests
- Cross-component interactions
- Backend integration testing
- Router pipeline testing
- Feature combination testing

#### 3. End-to-End Tests
- Complete message flow validation
- Multi-backend scenarios
- Performance under load
- Failure recovery testing

#### 4. Property-Based Tests
- Message invariant validation
- Correlation ID consistency
- Ack/Nack behavior verification
- Concurrent access safety

### Test Infrastructure
```rust
// Test utilities
pub struct TestEnvironment {
    pub brokers: HashMap<BackendType, Box<dyn TestBroker>>,
    pub temp_dir: TempDir,
    pub config: TestConfig,
}

pub trait TestBroker: Send + Sync {
    async fn start(&mut self) -> Result<(), TestError>;
    async fn stop(&mut self) -> Result<(), TestError>;
    async fn reset(&mut self) -> Result<(), TestError>;
    fn connection_string(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub timeout: Duration,
    pub max_retries: u32,
    pub parallel_tests: bool,
    pub cleanup_on_failure: bool,
}
```

## Implementation Tasks

### Phase 1: Test Infrastructure (Day 1)
- [ ] Set up test utilities and helpers
- [ ] Create mock implementations for all backends
- [ ] Implement test environment management
- [ ] Add test configuration system
- [ ] Create test data generators

### Phase 2: Unit Tests - Core Components (Day 1-2)
- [ ] Message struct tests (creation, metadata, serialization)
- [ ] Correlation ID generation and propagation tests
- [ ] In-memory broker unit tests
- [ ] Ack/Nack handle tests
- [ ] Error handling tests

### Phase 3: Unit Tests - Backend Implementations (Day 2-3)
- [ ] Kafka publisher/subscriber unit tests
- [ ] RabbitMQ publisher/subscriber unit tests
- [ ] MQTT publisher/subscriber unit tests
- [ ] Backend-specific error handling tests
- [ ] Connection management tests

### Phase 4: Integration Tests (Day 3-4)
- [ ] Router with different backends
- [ ] Cross-backend message flow
- [ ] Ack/Nack integration with Router
- [ ] Correlation ID propagation through Router
- [ ] Feature combination tests

### Phase 5: End-to-End Tests (Day 4-5)
- [ ] Complete message processing pipelines
- [ ] Multi-hop message routing
- [ ] Failure recovery scenarios
- [ ] Performance under concurrent load
- [ ] Long-running stability tests

### Phase 6: Property-Based Tests (Day 5)
- [ ] Message invariant properties
- [ ] Correlation ID uniqueness properties
- [ ] Ack/Nack consistency properties
- [ ] Concurrent access safety properties
- [ ] Serialization round-trip properties

### Phase 7: CI/CD Integration (Day 6-7)
- [ ] GitHub Actions workflow setup
- [ ] Test result reporting
- [ ] Coverage reporting integration
- [ ] Performance regression detection
- [ ] Flaky test detection and handling

## Testing Strategy

### Unit Tests Structure

#### Message Tests
```rust
#[cfg(test)]
mod message_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_message_creation() {
        let payload = b"test payload".to_vec();
        let message = Message::new(payload.clone());
        
        assert!(!message.uuid.is_empty());
        assert_eq!(message.payload, payload);
        assert!(message.metadata.is_empty());
        assert!(!message.correlation_id.is_empty());
        assert_eq!(message.hop_count, 0);
    }

    #[test]
    fn test_message_with_metadata() {
        let message = Message::new(b"test".to_vec())
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");
        
        assert_eq!(message.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(message.metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_child_message_creation() {
        let parent = Message::new(b"parent".to_vec());
        let child = parent.child_message(b"child".to_vec());
        
        assert_eq!(child.parent_correlation_id, Some(parent.correlation_id));
        assert_eq!(child.hop_count, parent.hop_count + 1);
        assert_ne!(child.uuid, parent.uuid);
    }

    #[test]
    fn test_message_serialization() {
        let original = Message::new(b"test".to_vec())
            .with_metadata("test", "value");
        
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(original.uuid, deserialized.uuid);
        assert_eq!(original.payload, deserialized.payload);
        assert_eq!(original.metadata, deserialized.metadata);
        assert_eq!(original.correlation_id, deserialized.correlation_id);
    }
}
```

#### In-Memory Broker Tests
```rust
#[cfg(test)]
mod inmemory_tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_basic_publish_subscribe() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::default()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        subscriber.subscribe("test-topic").await.unwrap();
        
        let message = Message::new(b"test message".to_vec());
        publisher.publish("test-topic", vec![message.clone()]).await.unwrap();
        
        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.payload, message.payload);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::default()));
        let publisher = InMemoryPublisher::new(broker.clone());
        
        let mut sub1 = InMemorySubscriber::new(broker.clone());
        let mut sub2 = InMemorySubscriber::new(broker.clone());
        
        sub1.subscribe("test-topic").await.unwrap();
        sub2.subscribe("test-topic").await.unwrap();
        
        let message = Message::new(b"broadcast".to_vec());
        publisher.publish("test-topic", vec![message.clone()]).await.unwrap();
        
        let received1 = sub1.receive().await.unwrap();
        let received2 = sub2.receive().await.unwrap();
        
        assert_eq!(received1.payload, message.payload);
        assert_eq!(received2.payload, message.payload);
    }

    #[tokio::test]
    async fn test_queue_size_limit() {
        let config = InMemoryConfig {
            max_queue_size: Some(2),
            ..Default::default()
        };
        let broker = Arc::new(InMemoryBroker::new(config));
        let publisher = InMemoryPublisher::new(broker.clone());
        
        // Fill queue to capacity
        let messages = vec![
            Message::new(b"msg1".to_vec()),
            Message::new(b"msg2".to_vec()),
        ];
        publisher.publish("test-topic", messages).await.unwrap();
        
        // This should fail due to queue size limit
        let overflow_msg = vec![Message::new(b"overflow".to_vec())];
        let result = publisher.publish("test-topic", overflow_msg).await;
        assert!(result.is_err());
    }
}
```

#### Ack/Nack Tests
```rust
#[cfg(test)]
mod ack_nack_tests {
    use super::*;

    #[tokio::test]
    async fn test_manual_acknowledgment() {
        let config = AckConfig {
            mode: AckMode::Manual,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            dead_letter_topic: None,
        };
        
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::default()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::with_ack_config(broker.clone(), config);
        
        subscriber.subscribe("test-topic").await.unwrap();
        
        let message = Message::new(b"test".to_vec());
        publisher.publish("test-topic", vec![message.clone()]).await.unwrap();
        
        let (received, ack_handle) = subscriber.receive_with_ack().await.unwrap();
        assert_eq!(received.payload, message.payload);
        
        // Acknowledge the message
        subscriber.ack(ack_handle).await.unwrap();
        
        // Message should not be redelivered
        let result = timeout(Duration::from_millis(100), subscriber.receive()).await;
        assert!(result.is_err()); // Timeout expected
    }

    #[tokio::test]
    async fn test_nack_redelivery() {
        let config = AckConfig {
            mode: AckMode::Manual,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_millis(10),
            dead_letter_topic: None,
        };
        
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::default()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::with_ack_config(broker.clone(), config);
        
        subscriber.subscribe("test-topic").await.unwrap();
        
        let message = Message::new(b"test".to_vec());
        publisher.publish("test-topic", vec![message.clone()]).await.unwrap();
        
        let (received1, ack_handle1) = subscriber.receive_with_ack().await.unwrap();
        
        // Nack with requeue
        subscriber.nack(ack_handle1, true).await.unwrap();
        
        // Message should be redelivered
        let (received2, ack_handle2) = subscriber.receive_with_ack().await.unwrap();
        assert_eq!(received1.uuid, received2.uuid);
        
        // Acknowledge the redelivered message
        subscriber.ack(ack_handle2).await.unwrap();
    }
}
```

### Integration Tests

#### Router Integration Tests
```rust
#[cfg(test)]
mod router_integration_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_router_with_inmemory_backend() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::default()));
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let subscriber = Arc::new(Mutex::new(InMemorySubscriber::new(broker.clone())));
        
        let processed_count = Arc::new(AtomicUsize::new(0));
        let count_clone = processed_count.clone();
        
        let handler = Arc::new(move |msg: Message| {
            let count = count_clone.clone();
            Box::pin(async move {
                count.fetch_add(1, Ordering::SeqCst);
                let processed = msg.with_metadata("processed", "true");
                Ok(vec![processed])
            })
        });
        
        let logger = Arc::new(StdLogger::new(false, false));
        let router = Router::new(
            logger,
            "input-topic".to_string(),
            "output-topic".to_string(),
            subscriber,
            publisher.clone(),
            handler,
        );
        
        // Start router in background
        let router_handle = tokio::spawn(async move {
            router.run().await
        });
        
        // Publish test messages
        let messages = vec![
            Message::new(b"msg1".to_vec()),
            Message::new(b"msg2".to_vec()),
            Message::new(b"msg3".to_vec()),
        ];
        
        publisher.publish("input-topic", messages).await.unwrap();
        
        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert_eq!(processed_count.load(Ordering::SeqCst), 3);
        
        // Cleanup
        router_handle.abort();
    }

    #[tokio::test]
    async fn test_correlation_id_propagation() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::default()));
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let subscriber = Arc::new(Mutex::new(InMemorySubscriber::new(broker.clone())));
        
        let correlation_ids = Arc::new(Mutex::new(Vec::new()));
        let ids_clone = correlation_ids.clone();
        
        let handler = Arc::new(move |msg: Message| {
            let ids = ids_clone.clone();
            Box::pin(async move {
                ids.lock().await.push(msg.correlation_id.clone());
                Ok(vec![msg.child_message(b"processed".to_vec())])
            })
        });
        
        let logger = Arc::new(StdLogger::new(false, false));
        let router = Router::new(
            logger,
            "input-topic".to_string(),
            "output-topic".to_string(),
            subscriber,
            publisher.clone(),
            handler,
        );
        
        let router_handle = tokio::spawn(async move {
            router.run().await
        });
        
        // Create message with specific correlation ID
        let mut message = Message::new(b"test".to_vec());
        message.correlation_id = "test-correlation-123".to_string();
        
        publisher.publish("input-topic", vec![message]).await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let ids = correlation_ids.lock().await;
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "test-correlation-123");
        
        router_handle.abort();
    }
}
```

### Property-Based Tests
```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_message_uuid_uniqueness(payloads in prop::collection::vec(any::<Vec<u8>>(), 1..100)) {
            let messages: Vec<Message> = payloads.into_iter()
                .map(Message::new)
                .collect();
            
            let uuids: std::collections::HashSet<_> = messages.iter()
                .map(|m| &m.uuid)
                .collect();
            
            // All UUIDs should be unique
            prop_assert_eq!(uuids.len(), messages.len());
        }

        #[test]
        fn test_correlation_id_inheritance(payload in any::<Vec<u8>>()) {
            let parent = Message::new(payload.clone());
            let child = parent.child_message(payload);
            
            prop_assert_eq!(child.parent_correlation_id, Some(parent.correlation_id));
            prop_assert_eq!(child.hop_count, parent.hop_count + 1);
            prop_assert_ne!(child.uuid, parent.uuid);
        }

        #[test]
        fn test_message_serialization_roundtrip(
            payload in any::<Vec<u8>>(),
            metadata in prop::collection::hash_map(any::<String>(), any::<String>(), 0..10)
        ) {
            let mut original = Message::new(payload);
            for (k, v) in metadata {
                original = original.with_metadata(k, v);
            }
            
            let serialized = serde_json::to_string(&original).unwrap();
            let deserialized: Message = serde_json::from_str(&serialized).unwrap();
            
            prop_assert_eq!(original.uuid, deserialized.uuid);
            prop_assert_eq!(original.payload, deserialized.payload);
            prop_assert_eq!(original.metadata, deserialized.metadata);
            prop_assert_eq!(original.correlation_id, deserialized.correlation_id);
        }
    }
}
```

## File Structure
```
kincir/
├── src/
│   └── test_utils/
│       ├── mod.rs
│       ├── environment.rs
│       ├── mocks.rs
│       ├── generators.rs
│       └── assertions.rs
├── tests/
│   ├── integration_test.rs (existing, to be extended)
│   ├── end_to_end_tests.rs
│   ├── backend_integration_tests.rs
│   ├── router_integration_tests.rs
│   └── property_tests.rs
└── .github/
    └── workflows/
        └── tests.yml
```

## CI/CD Integration

### GitHub Actions Workflow
```yaml
name: Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      rabbitmq:
        image: rabbitmq:3-management
        ports:
          - 5672:5672
          - 15672:15672
        options: >-
          --health-cmd "rabbitmq-diagnostics -q ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      
      kafka:
        image: confluentinc/cp-kafka:latest
        ports:
          - 9092:9092
        env:
          KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
          KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://localhost:9092
          KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1

    steps:
      - uses: actions/checkout@v2
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
      
      - name: Cache dependencies
        uses: actions/cache@v2
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Run unit tests
        run: cargo test --lib
      
      - name: Run integration tests
        run: cargo test --test '*'
      
      - name: Run property tests
        run: cargo test --features proptest
      
      - name: Generate coverage report
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out xml --output-dir coverage
      
      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v1
        with:
          file: coverage/cobertura.xml
```

## Test Data Management

### Test Data Generators
```rust
pub struct MessageGenerator {
    correlation_counter: AtomicUsize,
}

impl MessageGenerator {
    pub fn new() -> Self {
        Self {
            correlation_counter: AtomicUsize::new(0),
        }
    }
    
    pub fn generate_message(&self, size: usize) -> Message {
        let payload = (0..size).map(|i| (i % 256) as u8).collect();
        Message::new(payload)
    }
    
    pub fn generate_batch(&self, count: usize, size: usize) -> Vec<Message> {
        (0..count).map(|_| self.generate_message(size)).collect()
    }
    
    pub fn generate_with_correlation(&self, size: usize) -> Message {
        let id = self.correlation_counter.fetch_add(1, Ordering::SeqCst);
        let mut message = self.generate_message(size);
        message.correlation_id = format!("test-correlation-{}", id);
        message
    }
}
```

## Success Criteria
- [ ] Unit test coverage > 80% for all core modules
- [ ] Integration tests cover all backend combinations
- [ ] Property-based tests validate critical invariants
- [ ] All tests pass consistently in CI/CD
- [ ] Test execution time < 5 minutes
- [ ] Flaky test rate < 1%
- [ ] Clear test failure reporting and debugging
- [ ] Test environment setup is automated

## Dependencies
- `tokio-test` for async testing utilities
- `proptest` for property-based testing
- `mockall` for mocking (if needed)
- `tempfile` for temporary test files
- `serde_json` for serialization testing
- `criterion` (already present) for performance testing

## Documentation Requirements
- [ ] Testing guide for contributors
- [ ] Test writing best practices
- [ ] CI/CD testing workflow documentation
- [ ] Test environment setup instructions
- [ ] Debugging failed tests guide
- [ ] Property-based testing examples
