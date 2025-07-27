---
layout: docs
title: Integration Testing
description: Testing message flows and broker integrations in Kincir applications
---

# Integration Testing with Kincir

This guide demonstrates comprehensive integration testing strategies for Kincir applications, including testing message flows, broker integrations, and end-to-end scenarios.

## Table of Contents

- [Test Environment Setup](#test-environment-setup)
- [Message Flow Testing](#message-flow-testing)
- [Multi-Broker Testing](#multi-broker-testing)
- [Performance Testing](#performance-testing)
- [Error Scenario Testing](#error-scenario-testing)

## Test Environment Setup

### Docker Test Environment

```yaml
# docker-compose.test.yml
version: '3.8'
services:
  rabbitmq:
    image: rabbitmq:3-management
    ports:
      - "5672:5672"
      - "15672:15672"
    environment:
      RABBITMQ_DEFAULT_USER: test
      RABBITMQ_DEFAULT_PASS: test
    healthcheck:
      test: ["CMD", "rabbitmq-diagnostics", "ping"]
      interval: 30s
      timeout: 10s
      retries: 5

  kafka:
    image: confluentinc/cp-kafka:latest
    ports:
      - "9092:9092"
    environment:
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://localhost:9092
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
    depends_on:
      - zookeeper
    healthcheck:
      test: ["CMD", "kafka-topics", "--bootstrap-server", "localhost:9092", "--list"]
      interval: 30s
      timeout: 10s
      retries: 5

  zookeeper:
    image: confluentinc/cp-zookeeper:latest
    ports:
      - "2181:2181"
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181
      ZOOKEEPER_TICK_TIME: 2000

  mosquitto:
    image: eclipse-mosquitto:latest
    ports:
      - "1883:1883"
      - "9001:9001"
    volumes:
      - ./mosquitto.conf:/mosquitto/config/mosquitto.conf
```

### Test Configuration

```rust
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

pub struct IntegrationTestConfig {
    pub rabbitmq_url: String,
    pub kafka_brokers: String,
    pub mqtt_url: String,
    pub test_timeout: Duration,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            rabbitmq_url: "amqp://test:test@localhost:5672".to_string(),
            kafka_brokers: "localhost:9092".to_string(),
            mqtt_url: "mqtt://localhost:1883".to_string(),
            test_timeout: Duration::from_secs(30),
        }
    }
}

pub struct TestEnvironment {
    config: IntegrationTestConfig,
}

impl TestEnvironment {
    pub fn new() -> Self {
        Self {
            config: IntegrationTestConfig::default(),
        }
    }
    
    pub async fn wait_for_services(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Waiting for test services to be ready...");
        
        // Wait for RabbitMQ
        self.wait_for_rabbitmq().await?;
        
        // Wait for Kafka
        self.wait_for_kafka().await?;
        
        // Wait for MQTT
        self.wait_for_mqtt().await?;
        
        println!("All test services are ready");
        Ok(())
    }
    
    async fn wait_for_rabbitmq(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut attempts = 0;
        let max_attempts = 30;
        
        while attempts < max_attempts {
            match kincir::rabbitmq::RabbitMQPublisher::new(&self.config.rabbitmq_url) {
                publisher => {
                    // Try to publish a test message
                    let test_message = Message::new(b"health-check".to_vec());
                    if publisher.publish("health-check", vec![test_message]).await.is_ok() {
                        return Ok(());
                    }
                }
            }
            
            attempts += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        
        Err("RabbitMQ not ready after 30 seconds".into())
    }
    
    async fn wait_for_kafka(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Similar implementation for Kafka
        tokio::time::sleep(Duration::from_secs(2)).await; // Simplified for example
        Ok(())
    }
    
    async fn wait_for_mqtt(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Similar implementation for MQTT
        tokio::time::sleep(Duration::from_secs(1)).await; // Simplified for example
        Ok(())
    }
}
```

## Message Flow Testing

### End-to-End Message Flow

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
    use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
    use tokio_test;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_in_memory_message_flow() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker);
        
        let topic = "integration-test";
        subscriber.subscribe(topic).await.unwrap();
        
        // Test single message
        let test_message = Message::new(b"test payload".to_vec())
            .with_metadata("test_id", "001");
        
        publisher.publish(topic, vec![test_message.clone()]).await.unwrap();
        
        let received = timeout(Duration::from_secs(5), subscriber.receive())
            .await
            .expect("Timeout waiting for message")
            .expect("Failed to receive message");
        
        assert_eq!(received.payload, test_message.payload);
        assert_eq!(received.metadata.get("test_id"), Some(&"001".to_string()));
    }

    #[tokio::test]
    async fn test_rabbitmq_message_flow() {
        let test_env = TestEnvironment::new();
        test_env.wait_for_services().await.unwrap();
        
        let publisher = RabbitMQPublisher::new(&test_env.config.rabbitmq_url);
        let mut subscriber = RabbitMQSubscriber::new(&test_env.config.rabbitmq_url, "test-queue");
        
        let topic = "integration-test";
        subscriber.subscribe(topic).await.unwrap();
        
        // Allow some time for subscription to be established
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let test_message = Message::new(b"rabbitmq test".to_vec())
            .with_metadata("broker", "rabbitmq");
        
        publisher.publish(topic, vec![test_message.clone()]).await.unwrap();
        
        let received = timeout(Duration::from_secs(10), subscriber.receive())
            .await
            .expect("Timeout waiting for RabbitMQ message")
            .expect("Failed to receive RabbitMQ message");
        
        assert_eq!(received.payload, test_message.payload);
        assert_eq!(received.metadata.get("broker"), Some(&"rabbitmq".to_string()));
    }

    #[tokio::test]
    async fn test_message_ordering() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker);
        
        let topic = "ordering-test";
        subscriber.subscribe(topic).await.unwrap();
        
        // Send messages in order
        let messages: Vec<Message> = (0..10)
            .map(|i| Message::new(format!("message-{}", i).into_bytes())
                .with_metadata("sequence", &i.to_string()))
            .collect();
        
        publisher.publish(topic, messages.clone()).await.unwrap();
        
        // Verify order
        for (i, expected_message) in messages.iter().enumerate() {
            let received = timeout(Duration::from_secs(5), subscriber.receive())
                .await
                .expect("Timeout waiting for ordered message")
                .expect("Failed to receive ordered message");
            
            assert_eq!(received.payload, expected_message.payload);
            assert_eq!(received.metadata.get("sequence"), Some(&i.to_string()));
        }
    }
}
```

## Multi-Broker Testing

### Cross-Broker Message Routing

```rust
pub struct CrossBrokerTest {
    in_memory_broker: Arc<InMemoryBroker>,
    rabbitmq_url: String,
}

impl CrossBrokerTest {
    pub fn new(rabbitmq_url: String) -> Self {
        Self {
            in_memory_broker: Arc::new(InMemoryBroker::with_default_config()),
            rabbitmq_url,
        }
    }
    
    pub async fn test_in_memory_to_rabbitmq_routing(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Set up in-memory publisher
        let in_memory_publisher = InMemoryPublisher::new(self.in_memory_broker.clone());
        let mut in_memory_subscriber = InMemorySubscriber::new(self.in_memory_broker.clone());
        
        // Set up RabbitMQ publisher
        let rabbitmq_publisher = RabbitMQPublisher::new(&self.rabbitmq_url);
        let mut rabbitmq_subscriber = RabbitMQSubscriber::new(&self.rabbitmq_url, "cross-broker-test");
        
        // Subscribe to topics
        in_memory_subscriber.subscribe("source-topic").await?;
        rabbitmq_subscriber.subscribe("destination-topic").await?;
        
        // Create a message router
        let router_publisher = rabbitmq_publisher.clone();
        let router_subscriber = in_memory_subscriber;
        
        // Start routing task
        let routing_task = tokio::spawn(async move {
            loop {
                match router_subscriber.receive().await {
                    Ok(message) => {
                        let routed_message = Message::new(message.payload)
                            .with_metadata("routed", "true")
                            .with_metadata("source", "in-memory");
                        
                        if let Err(e) = router_publisher.publish("destination-topic", vec![routed_message]).await {
                            eprintln!("Routing error: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Receive error: {}", e);
                        break;
                    }
                }
            }
        });
        
        // Send test message
        let test_message = Message::new(b"cross-broker test".to_vec())
            .with_metadata("test_id", &Uuid::new_v4().to_string());
        
        in_memory_publisher.publish("source-topic", vec![test_message.clone()]).await?;
        
        // Verify message arrives at RabbitMQ
        let received = timeout(Duration::from_secs(10), rabbitmq_subscriber.receive()).await??;
        
        assert_eq!(received.payload, test_message.payload);
        assert_eq!(received.metadata.get("routed"), Some(&"true".to_string()));
        assert_eq!(received.metadata.get("source"), Some(&"in-memory".to_string()));
        
        routing_task.abort();
        Ok(())
    }
}

#[tokio::test]
async fn test_cross_broker_routing() {
    let test_env = TestEnvironment::new();
    test_env.wait_for_services().await.unwrap();
    
    let cross_broker_test = CrossBrokerTest::new(test_env.config.rabbitmq_url);
    cross_broker_test.test_in_memory_to_rabbitmq_routing().await.unwrap();
}
```

## Performance Testing

### Throughput and Latency Testing

```rust
use std::time::Instant;
use tokio::sync::mpsc;

pub struct PerformanceTest {
    broker: Arc<InMemoryBroker>,
}

impl PerformanceTest {
    pub fn new() -> Self {
        Self {
            broker: Arc::new(InMemoryBroker::with_default_config()),
        }
    }
    
    pub async fn test_throughput(&self, message_count: usize, message_size: usize) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let publisher = InMemoryPublisher::new(self.broker.clone());
        let mut subscriber = InMemorySubscriber::new(self.broker.clone());
        
        subscriber.subscribe("throughput-test").await?;
        
        // Create test messages
        let messages: Vec<Message> = (0..message_count)
            .map(|i| Message::new(vec![0u8; message_size])
                .with_metadata("id", &i.to_string()))
            .collect();
        
        // Start receiving task
        let (tx, mut rx) = mpsc::channel(1);
        let receive_task = tokio::spawn(async move {
            let start = Instant::now();
            
            for _ in 0..message_count {
                if subscriber.receive().await.is_err() {
                    break;
                }
            }
            
            let duration = start.elapsed();
            let _ = tx.send(duration).await;
        });
        
        // Small delay to ensure subscriber is ready
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Publish messages
        let publish_start = Instant::now();
        publisher.publish("throughput-test", messages).await?;
        let publish_duration = publish_start.elapsed();
        
        // Wait for all messages to be received
        let receive_duration = timeout(Duration::from_secs(30), rx.recv())
            .await?
            .ok_or("Receive task failed")?;
        
        receive_task.await?;
        
        let throughput = message_count as f64 / receive_duration.as_secs_f64();
        
        println!("Performance Test Results:");
        println!("  Messages: {}", message_count);
        println!("  Message Size: {} bytes", message_size);
        println!("  Publish Time: {:?}", publish_duration);
        println!("  Receive Time: {:?}", receive_duration);
        println!("  Throughput: {:.2} messages/second", throughput);
        
        Ok(throughput)
    }
    
    pub async fn test_latency(&self, sample_count: usize) -> Result<Vec<Duration>, Box<dyn std::error::Error + Send + Sync>> {
        let publisher = InMemoryPublisher::new(self.broker.clone());
        let mut subscriber = InMemorySubscriber::new(self.broker.clone());
        
        subscriber.subscribe("latency-test").await?;
        
        let mut latencies = Vec::new();
        
        for i in 0..sample_count {
            let start = Instant::now();
            
            let message = Message::new(format!("latency-test-{}", i).into_bytes())
                .with_metadata("timestamp", &start.elapsed().as_nanos().to_string());
            
            publisher.publish("latency-test", vec![message]).await?;
            
            let _received = subscriber.receive().await?;
            let latency = start.elapsed();
            
            latencies.push(latency);
            
            // Small delay between samples
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        
        // Calculate statistics
        let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
        let mut sorted_latencies = latencies.clone();
        sorted_latencies.sort();
        let p95_latency = sorted_latencies[(sample_count as f64 * 0.95) as usize];
        let p99_latency = sorted_latencies[(sample_count as f64 * 0.99) as usize];
        
        println!("Latency Test Results:");
        println!("  Samples: {}", sample_count);
        println!("  Average Latency: {:?}", avg_latency);
        println!("  P95 Latency: {:?}", p95_latency);
        println!("  P99 Latency: {:?}", p99_latency);
        
        Ok(latencies)
    }
}

#[tokio::test]
async fn test_performance_benchmarks() {
    let perf_test = PerformanceTest::new();
    
    // Test throughput with different message sizes
    let throughput_1kb = perf_test.test_throughput(10_000, 1024).await.unwrap();
    let throughput_10kb = perf_test.test_throughput(1_000, 10_240).await.unwrap();
    
    assert!(throughput_1kb > 1000.0, "Throughput should be > 1000 msg/s for 1KB messages");
    assert!(throughput_10kb > 100.0, "Throughput should be > 100 msg/s for 10KB messages");
    
    // Test latency
    let latencies = perf_test.test_latency(1000).await.unwrap();
    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    
    assert!(avg_latency < Duration::from_millis(10), "Average latency should be < 10ms");
}
```

## Error Scenario Testing

### Failure Recovery Testing

```rust
pub struct FailureRecoveryTest {
    broker: Arc<InMemoryBroker>,
}

impl FailureRecoveryTest {
    pub fn new() -> Self {
        Self {
            broker: Arc::new(InMemoryBroker::with_default_config()),
        }
    }
    
    pub async fn test_publisher_failure_recovery(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let publisher = InMemoryPublisher::new(self.broker.clone());
        let mut subscriber = InMemorySubscriber::new(self.broker.clone());
        
        subscriber.subscribe("failure-test").await?;
        
        // Send normal message
        let message1 = Message::new(b"before failure".to_vec());
        publisher.publish("failure-test", vec![message1.clone()]).await?;
        
        let received1 = subscriber.receive().await?;
        assert_eq!(received1.payload, message1.payload);
        
        // Simulate failure by trying to publish to non-existent topic
        // (This would fail in real brokers, but in-memory broker is resilient)
        
        // Send recovery message
        let message2 = Message::new(b"after recovery".to_vec());
        publisher.publish("failure-test", vec![message2.clone()]).await?;
        
        let received2 = subscriber.receive().await?;
        assert_eq!(received2.payload, message2.payload);
        
        Ok(())
    }
    
    pub async fn test_subscriber_reconnection(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let publisher = InMemoryPublisher::new(self.broker.clone());
        let mut subscriber1 = InMemorySubscriber::new(self.broker.clone());
        
        subscriber1.subscribe("reconnection-test").await?;
        
        // Send message to first subscriber
        let message1 = Message::new(b"to first subscriber".to_vec());
        publisher.publish("reconnection-test", vec![message1.clone()]).await?;
        
        let received1 = subscriber1.receive().await?;
        assert_eq!(received1.payload, message1.payload);
        
        // Simulate subscriber disconnection by creating new subscriber
        let mut subscriber2 = InMemorySubscriber::new(self.broker.clone());
        subscriber2.subscribe("reconnection-test").await?;
        
        // Send message to new subscriber
        let message2 = Message::new(b"to reconnected subscriber".to_vec());
        publisher.publish("reconnection-test", vec![message2.clone()]).await?;
        
        let received2 = subscriber2.receive().await?;
        assert_eq!(received2.payload, message2.payload);
        
        Ok(())
    }
}

#[tokio::test]
async fn test_failure_scenarios() {
    let failure_test = FailureRecoveryTest::new();
    
    failure_test.test_publisher_failure_recovery().await.unwrap();
    failure_test.test_subscriber_reconnection().await.unwrap();
}
```

## Running Integration Tests

### Test Runner Script

```bash
#!/bin/bash
# run-integration-tests.sh

set -e

echo "Starting integration test environment..."

# Start test services
docker-compose -f docker-compose.test.yml up -d

# Wait for services to be ready
echo "Waiting for services to be ready..."
sleep 30

# Run integration tests
echo "Running integration tests..."
cargo test --test integration_tests -- --nocapture

# Cleanup
echo "Cleaning up test environment..."
docker-compose -f docker-compose.test.yml down

echo "Integration tests completed!"
```

### Cargo.toml Test Configuration

```toml
[[test]]
name = "integration_tests"
path = "tests/integration_tests.rs"

[dev-dependencies]
tokio-test = "0.4"
uuid = { version = "1.0", features = ["v4"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
```

This integration testing guide provides comprehensive strategies for testing Kincir applications across different scenarios and environments.
