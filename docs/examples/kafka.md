---
layout: docs
title: Kafka Integration Example
description: High-throughput message streaming with Apache Kafka
---

# Kafka Integration Example

Apache Kafka is a distributed streaming platform designed for high-throughput, fault-tolerant message streaming. Kincir provides seamless integration with Kafka through its unified interface.

## Prerequisites

Before running these examples, ensure you have Kafka installed and running:

```bash
# Using Docker Compose
version: '3.8'
services:
  zookeeper:
    image: confluentinc/cp-zookeeper:latest
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181
      ZOOKEEPER_TICK_TIME: 2000
    ports:
      - "2181:2181"

  kafka:
    image: confluentinc/cp-kafka:latest
    depends_on:
      - zookeeper
    ports:
      - "9092:9092"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://localhost:9092
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1

# Start with: docker-compose up -d
```

## Basic Usage

### Simple Producer-Consumer

```rust
use kincir::kafka::{KafkaPublisher, KafkaSubscriber};
use kincir::{Publisher, Subscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create Kafka publisher and subscriber
    let publisher = KafkaPublisher::new("localhost:9092");
    let mut subscriber = KafkaSubscriber::new("localhost:9092", "my-consumer-group");

    // Subscribe to a topic
    subscriber.subscribe("user-events").await?;
    
    // Publish messages
    let messages = vec![
        Message::new(b"User registered: alice@example.com".to_vec())
            .with_metadata("event_type", "user_registration")
            .with_metadata("user_id", "alice123")
            .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339()),
            
        Message::new(b"User logged in: alice@example.com".to_vec())
            .with_metadata("event_type", "user_login")
            .with_metadata("user_id", "alice123")
            .with_metadata("ip_address", "192.168.1.100"),
    ];
    
    publisher.publish("user-events", messages).await?;
    
    // Consume messages
    for _ in 0..2 {
        let message = subscriber.receive().await?;
        println!("Received: {:?}", String::from_utf8_lossy(&message.payload));
        println!("Event type: {:?}", message.get_metadata("event_type"));
    }
    
    Ok(())
}
```

## High-Throughput Publishing

```rust
use kincir::kafka::{KafkaPublisher, KafkaConfig};
use kincir::{Publisher, Message};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure Kafka for high throughput
    let config = KafkaConfig {
        bootstrap_servers: "localhost:9092".to_string(),
        batch_size: 16384,
        linger_ms: 5,
        compression_type: "snappy".to_string(),
        acks: "1".to_string(),
        retries: 3,
        max_in_flight_requests: 5,
    };
    
    let publisher = KafkaPublisher::with_config(config);
    
    let message_count = 100_000;
    let batch_size = 1000;
    
    println!("Publishing {} messages in batches of {}", message_count, batch_size);
    
    let start = Instant::now();
    
    for batch in 0..(message_count / batch_size) {
        let mut messages = Vec::with_capacity(batch_size);
        
        for i in 0..batch_size {
            let message_id = batch * batch_size + i;
            let message = Message::new(format!("High throughput message #{}", message_id).into_bytes())
                .with_metadata("batch", &batch.to_string())
                .with_metadata("message_id", &message_id.to_string())
                .with_metadata("timestamp", &chrono::Utc::now().timestamp().to_string());
            
            messages.push(message);
        }
        
        publisher.publish("high-throughput-topic", messages).await?;
        
        if batch % 10 == 0 {
            println!("Published batch {} ({} messages)", batch, (batch + 1) * batch_size);
        }
    }
    
    let duration = start.elapsed();
    println!("Published {} messages in {:?}", message_count, duration);
    println!("Throughput: {:.2} messages/second", 
             message_count as f64 / duration.as_secs_f64());
    
    Ok(())
}
```

## Consumer Groups

```rust
use kincir::kafka::{KafkaSubscriber, KafkaConsumerConfig};
use kincir::{Subscriber, Message};
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create multiple consumers in the same group
    let consumer_group = "order-processing-group";
    let topic = "orders";
    
    let mut consumer_handles = vec![];
    
    for consumer_id in 0..3 {
        let handle = task::spawn(async move {
            let config = KafkaConsumerConfig {
                bootstrap_servers: "localhost:9092".to_string(),
                group_id: consumer_group.to_string(),
                auto_offset_reset: "earliest".to_string(),
                enable_auto_commit: true,
                auto_commit_interval_ms: 1000,
                session_timeout_ms: 30000,
                max_poll_records: 500,
            };
            
            let mut subscriber = KafkaSubscriber::with_config(config);
            subscriber.subscribe(topic).await.unwrap();
            
            let mut message_count = 0;
            loop {
                match subscriber.receive().await {
                    Ok(message) => {
                        message_count += 1;
                        let order_data = String::from_utf8_lossy(&message.payload);
                        println!("Consumer {} processed order: {} (total: {})", 
                                consumer_id, order_data, message_count);
                        
                        // Simulate processing time
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        eprintln!("Consumer {} error: {}", consumer_id, e);
                        break;
                    }
                }
            }
        });
        consumer_handles.push(handle);
    }
    
    // Let consumers run for a while
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    
    // Cancel consumers
    for handle in consumer_handles {
        handle.abort();
    }
    
    Ok(())
}
```

## Partitioned Topics

```rust
use kincir::kafka::{KafkaPublisher, KafkaPartitioner};
use kincir::{Publisher, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let publisher = KafkaPublisher::new("localhost:9092");
    
    // Publish messages with specific partitioning
    let user_ids = vec!["user1", "user2", "user3", "user4", "user5"];
    
    for user_id in user_ids {
        let messages = vec![
            Message::new(format!("Login event for {}", user_id).into_bytes())
                .with_metadata("user_id", user_id)
                .with_metadata("event_type", "login")
                .with_metadata("partition_key", user_id), // Use user_id for partitioning
                
            Message::new(format!("Purchase event for {}", user_id).into_bytes())
                .with_metadata("user_id", user_id)
                .with_metadata("event_type", "purchase")
                .with_metadata("partition_key", user_id),
        ];
        
        // Messages with the same partition_key will go to the same partition
        publisher.publish_with_key("user-events", messages, Some(user_id)).await?;
    }
    
    println!("Published events for all users with partitioning");
    Ok(())
}
```

## Exactly-Once Semantics

```rust
use kincir::kafka::{KafkaPublisher, KafkaTransactionalConfig};
use kincir::{Publisher, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure for exactly-once semantics
    let config = KafkaTransactionalConfig {
        bootstrap_servers: "localhost:9092".to_string(),
        transactional_id: "order-processor-1".to_string(),
        enable_idempotence: true,
        acks: "all".to_string(),
        retries: i32::MAX,
        max_in_flight_requests: 1,
    };
    
    let publisher = KafkaPublisher::with_transactional_config(config);
    
    // Begin transaction
    publisher.begin_transaction().await?;
    
    match process_order_batch().await {
        Ok(processed_orders) => {
            // Publish all processed orders in a single transaction
            for order in processed_orders {
                let message = Message::new(order.to_json().into_bytes())
                    .with_metadata("order_id", &order.id)
                    .with_metadata("status", "processed");
                
                publisher.publish("processed-orders", vec![message]).await?;
            }
            
            // Commit transaction
            publisher.commit_transaction().await?;
            println!("Successfully processed and published order batch");
        }
        Err(e) => {
            // Abort transaction on error
            publisher.abort_transaction().await?;
            eprintln!("Failed to process orders, transaction aborted: {}", e);
        }
    }
    
    Ok(())
}

#[derive(Debug)]
struct Order {
    id: String,
    amount: f64,
    customer_id: String,
}

impl Order {
    fn to_json(&self) -> String {
        {% raw %}format!(r#"{{"id":"{}","amount":{},"customer_id":"{}"}}"#, 
                self.id, self.amount, self.customer_id){% endraw %}
    }
}

async fn process_order_batch() -> Result<Vec<Order>, Box<dyn std::error::Error + Send + Sync>> {
    // Simulate order processing
    let orders = vec![
        Order { id: "ORD-001".to_string(), amount: 99.99, customer_id: "CUST-123".to_string() },
        Order { id: "ORD-002".to_string(), amount: 149.50, customer_id: "CUST-456".to_string() },
        Order { id: "ORD-003".to_string(), amount: 75.25, customer_id: "CUST-789".to_string() },
    ];
    
    // Simulate processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    Ok(orders)
}
```

## Stream Processing

```rust
use kincir::kafka::{KafkaSubscriber, KafkaPublisher};
use kincir::{Subscriber, Publisher, Message};
use serde_json::{Value, json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Input stream: raw events
    let mut input_subscriber = KafkaSubscriber::new("localhost:9092", "stream-processor");
    input_subscriber.subscribe("raw-events").await?;
    
    // Output streams: processed events
    let metrics_publisher = KafkaPublisher::new("localhost:9092");
    let alerts_publisher = KafkaPublisher::new("localhost:9092");
    
    println!("Starting stream processor...");
    
    loop {
        match input_subscriber.receive().await {
            Ok(message) => {
                // Parse incoming event
                if let Ok(event) = parse_event(&message) {
                    // Process event and generate outputs
                    if let Some(metric) = generate_metric(&event) {
                        metrics_publisher.publish("metrics", vec![metric]).await?;
                    }
                    
                    if let Some(alert) = check_for_alert(&event) {
                        alerts_publisher.publish("alerts", vec![alert]).await?;
                    }
                    
                    // Log processing
                    println!("Processed event: {}", event["event_type"]);
                }
            }
            Err(e) => {
                eprintln!("Stream processing error: {}", e);
                // Implement error handling strategy
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

fn parse_event(message: &Message) -> Result<Value, serde_json::Error> {
    let payload = String::from_utf8_lossy(&message.payload);
    serde_json::from_str(&payload)
}

fn generate_metric(event: &Value) -> Option<Message> {
    if let Some(event_type) = event["event_type"].as_str() {
        let metric = json!({
            "metric_name": format!("{}_count", event_type),
            "value": 1,
            "timestamp": chrono::Utc::now().timestamp(),
            "tags": {
                "event_type": event_type
            }
        });
        
        Some(Message::new(metric.to_string().into_bytes())
            .with_metadata("metric_type", "counter")
            .with_metadata("source", "stream_processor"))
    } else {
        None
    }
}

fn check_for_alert(event: &Value) -> Option<Message> {
    // Check for error events
    if event["event_type"] == "error" {
        let alert = json!({
            "alert_type": "error_detected",
            "severity": "high",
            "message": format!("Error event detected: {}", event["message"]),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "source_event": event
        });
        
        Some(Message::new(alert.to_string().into_bytes())
            .with_metadata("alert_level", "high")
            .with_metadata("alert_type", "error_detected"))
    } else {
        None
    }
}
```

## Kafka Streams Integration

```rust
use kincir::kafka::{KafkaStreamsBuilder, KafkaSubscriber, KafkaPublisher};
use kincir::{Subscriber, Publisher, Message};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Word count stream processing example
    let word_counts: Arc<Mutex<HashMap<String, u64>>> = Arc::new(Mutex::new(HashMap::new()));
    
    let mut subscriber = KafkaSubscriber::new("localhost:9092", "word-count-processor");
    subscriber.subscribe("text-input").await?;
    
    let publisher = KafkaPublisher::new("localhost:9092");
    
    println!("Starting word count stream processor...");
    
    loop {
        match subscriber.receive().await {
            Ok(message) => {
                let text = String::from_utf8_lossy(&message.payload);
                
                // Process text and count words
                let words: Vec<&str> = text.split_whitespace().collect();
                let mut counts = word_counts.lock().unwrap();
                
                for word in words {
                    let word = word.to_lowercase();
                    *counts.entry(word.clone()).or_insert(0) += 1;
                    
                    // Publish updated count
                    let count_message = Message::new(counts[&word].to_string().into_bytes())
                        .with_metadata("word", &word)
                        .with_metadata("count", &counts[&word].to_string())
                        .with_metadata("timestamp", &chrono::Utc::now().timestamp().to_string());
                    
                    publisher.publish("word-counts", vec![count_message]).await?;
                }
                
                println!("Processed text with {} words", words.len());
            }
            Err(e) => {
                eprintln!("Stream processing error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}
```

## Performance Monitoring

```rust
use kincir::kafka::{KafkaPublisher, KafkaSubscriber, KafkaMetrics};
use kincir::{Publisher, Subscriber, Message};
use std::time::{Instant, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let publisher = KafkaPublisher::new("localhost:9092");
    let mut subscriber = KafkaSubscriber::new("localhost:9092", "perf-test-group");
    
    subscriber.subscribe("performance-test").await?;
    
    // Performance test parameters
    let message_count = 10_000;
    let message_size = 1024; // 1KB messages
    let payload = vec![0u8; message_size];
    
    println!("Starting Kafka performance test...");
    println!("Messages: {}, Size: {} bytes each", message_count, message_size);
    
    // Measure publishing performance
    let start = Instant::now();
    let mut messages = Vec::with_capacity(100);
    
    for i in 0..message_count {
        let message = Message::new(payload.clone())
            .with_metadata("sequence", &i.to_string())
            .with_metadata("timestamp", &chrono::Utc::now().timestamp_nanos().to_string());
        
        messages.push(message);
        
        // Publish in batches of 100
        if messages.len() == 100 || i == message_count - 1 {
            publisher.publish("performance-test", messages.clone()).await?;
            messages.clear();
        }
        
        if i % 1000 == 0 {
            println!("Published {} messages", i + 1);
        }
    }
    
    let publish_duration = start.elapsed();
    println!("Publishing completed in {:?}", publish_duration);
    println!("Publishing rate: {:.2} messages/second", 
             message_count as f64 / publish_duration.as_secs_f64());
    
    // Measure consuming performance
    let start = Instant::now();
    let mut received_count = 0;
    
    while received_count < message_count {
        match subscriber.receive().await {
            Ok(_message) => {
                received_count += 1;
                if received_count % 1000 == 0 {
                    println!("Received {} messages", received_count);
                }
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
        }
    }
    
    let consume_duration = start.elapsed();
    println!("Consuming completed in {:?}", consume_duration);
    println!("Consuming rate: {:.2} messages/second", 
             received_count as f64 / consume_duration.as_secs_f64());
    
    // Get Kafka metrics
    if let Ok(metrics) = publisher.get_metrics().await {
        println!("Publisher metrics: {:?}", metrics);
    }
    
    if let Ok(metrics) = subscriber.get_metrics().await {
        println!("Consumer metrics: {:?}", metrics);
    }
    
    Ok(())
}
```

## Error Handling and Resilience

```rust
use kincir::kafka::{KafkaPublisher, KafkaSubscriber, KafkaError};
use kincir::{Publisher, Subscriber, Message};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut retry_count = 0;
    let max_retries = 5;
    
    loop {
        match run_kafka_client().await {
            Ok(()) => {
                println!("Kafka client completed successfully");
                break;
            }
            Err(KafkaError::BrokerNotAvailable) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    eprintln!("Max retries reached. Kafka broker not available.");
                    return Err("Kafka broker not available".into());
                }
                
                println!("Kafka broker not available (attempt {}). Retrying in 10 seconds...", retry_count);
                sleep(Duration::from_secs(10)).await;
            }
            Err(KafkaError::TopicNotFound(topic)) => {
                println!("Topic '{}' not found. Creating topic...", topic);
                // In a real application, you might want to create the topic here
                sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                eprintln!("Non-recoverable Kafka error: {}", e);
                return Err(e.into());
            }
        }
    }
    
    Ok(())
}

async fn run_kafka_client() -> Result<(), KafkaError> {
    let publisher = KafkaPublisher::new("localhost:9092");
    let mut subscriber = KafkaSubscriber::new("localhost:9092", "resilient-consumer");
    
    subscriber.subscribe("resilient-topic").await?;
    
    let message = Message::new(b"Resilient message".to_vec());
    publisher.publish("resilient-topic", vec![message]).await?;
    
    let received = subscriber.receive().await?;
    println!("Received: {:?}", String::from_utf8_lossy(&received.payload));
    
    Ok(())
}
```

## Testing with Kafka

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::{clients::Cli, images::kafka::Kafka, Container};

    #[tokio::test]
    async fn test_kafka_integration() {
        // Start Kafka container for testing
        let docker = Cli::default();
        let kafka_container = docker.run(Kafka::default());
        let bootstrap_servers = format!(
            "127.0.0.1:{}",
            kafka_container.get_host_port_ipv4(9092)
        );
        
        let publisher = KafkaPublisher::new(&bootstrap_servers);
        let mut subscriber = KafkaSubscriber::new(&bootstrap_servers, "test-group");
        
        subscriber.subscribe("test-topic").await.unwrap();
        
        let message = Message::new(b"integration test message".to_vec());
        publisher.publish("test-topic", vec![message]).await.unwrap();
        
        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.payload, b"integration test message");
    }
}
```

## Next Steps

- [MQTT Support](/examples/mqtt.html) - IoT messaging
- [Message Acknowledgments](/examples/acknowledgments.html) - Reliable processing
- [Stream Processing](/examples/stream-processing.html) - Real-time data processing
- [Performance Optimization](/examples/performance.html) - High-throughput tuning

## Resources

- [Apache Kafka Documentation](https://kafka.apache.org/documentation/)
- [Kafka Streams](https://kafka.apache.org/documentation/streams/)
- [Kincir API Documentation](https://docs.rs/kincir)
- [Getting Started Guide](/docs/getting-started.html)
