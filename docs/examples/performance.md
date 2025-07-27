---
layout: docs
title: Performance Optimization
description: Techniques and best practices for optimizing Kincir performance in high-load scenarios
---

# Performance Optimization in Kincir

This guide covers techniques and best practices for optimizing Kincir performance in high-load production scenarios.

## Table of Contents

- [Benchmarking Basics](#benchmarking-basics)
- [In-Memory Broker Optimization](#in-memory-broker-optimization)
- [Batch Processing](#batch-processing)
- [Connection Pooling](#connection-pooling)
- [Memory Management](#memory-management)
- [Async Optimization](#async-optimization)
- [Monitoring Performance](#monitoring-performance)

## Benchmarking Basics

### Simple Throughput Test

```rust
use kincir::{Publisher, Subscriber, Message};
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker);
    
    subscriber.subscribe("benchmark").await?;
    
    let message_count = 100_000;
    let message_size = 1024; // 1KB messages
    
    // Create test messages
    let messages: Vec<Message> = (0..message_count)
        .map(|i| {
            let payload = vec![0u8; message_size];
            Message::new(payload).with_metadata("id", &i.to_string())
        })
        .collect();
    
    println!("Starting throughput test with {} messages of {} bytes each", 
             message_count, message_size);
    
    // Measure publish throughput
    let start = Instant::now();
    publisher.publish("benchmark", messages).await?;
    let publish_duration = start.elapsed();
    
    let publish_throughput = message_count as f64 / publish_duration.as_secs_f64();
    println!("Publish throughput: {:.2} messages/second", publish_throughput);
    
    // Measure receive throughput
    let start = Instant::now();
    for _ in 0..message_count {
        let _ = subscriber.receive().await?;
    }
    let receive_duration = start.elapsed();
    
    let receive_throughput = message_count as f64 / receive_duration.as_secs_f64();
    println!("Receive throughput: {:.2} messages/second", receive_throughput);
    
    Ok(())
}
```

## In-Memory Broker Optimization

### High-Performance Configuration

```rust
use kincir::memory::{InMemoryBroker, BrokerConfig};
use std::sync::Arc;
use std::time::Duration;

fn create_optimized_broker() -> Arc<InMemoryBroker> {
    let config = BrokerConfig {
        max_queue_size: 1_000_000,        // Large queue for high throughput
        enable_message_ordering: false,    // Disable if ordering not needed
        enable_ttl: false,                // Disable TTL for better performance
        enable_health_monitoring: false,  // Disable monitoring in production
        cleanup_interval: Duration::from_secs(300), // Less frequent cleanup
        max_subscribers_per_topic: 1000,
        enable_statistics: false,         // Disable stats collection
    };
    
    Arc::new(InMemoryBroker::new(config))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = create_optimized_broker();
    
    // Use the optimized broker
    let publisher = kincir::memory::InMemoryPublisher::new(broker.clone());
    let mut subscriber = kincir::memory::InMemorySubscriber::new(broker);
    
    // Your high-performance application logic here
    
    Ok(())
}
```

### Concurrent Publishers and Subscribers

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use tokio::task::JoinSet;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let num_publishers = 10;
    let num_subscribers = 5;
    let messages_per_publisher = 10_000;
    
    let mut tasks = JoinSet::new();
    
    // Spawn multiple publishers
    for publisher_id in 0..num_publishers {
        let broker = broker.clone();
        tasks.spawn(async move {
            let publisher = InMemoryPublisher::new(broker);
            let start = Instant::now();
            
            for i in 0..messages_per_publisher {
                let message = Message::new(
                    format!("Publisher {} - Message {}", publisher_id, i).into_bytes()
                );
                publisher.publish("high-throughput", vec![message]).await?;
            }
            
            let duration = start.elapsed();
            println!("Publisher {} completed in {:?}", publisher_id, duration);
            
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
    }
    
    // Spawn multiple subscribers
    for subscriber_id in 0..num_subscribers {
        let broker = broker.clone();
        tasks.spawn(async move {
            let mut subscriber = InMemorySubscriber::new(broker);
            subscriber.subscribe("high-throughput").await?;
            
            let mut received_count = 0;
            let start = Instant::now();
            
            // Each subscriber processes messages for a fixed duration
            let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
            tokio::pin!(timeout);
            
            loop {
                tokio::select! {
                    result = subscriber.receive() => {
                        match result {
                            Ok(_) => received_count += 1,
                            Err(e) => eprintln!("Subscriber {} error: {}", subscriber_id, e),
                        }
                    }
                    _ = &mut timeout => break,
                }
            }
            
            let duration = start.elapsed();
            let throughput = received_count as f64 / duration.as_secs_f64();
            println!("Subscriber {} processed {} messages ({:.2} msg/s)", 
                     subscriber_id, received_count, throughput);
            
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
    }
    
    // Wait for all tasks to complete
    while let Some(result) = tasks.join_next().await {
        if let Err(e) = result? {
            eprintln!("Task error: {}", e);
        }
    }
    
    Ok(())
}
```

## Batch Processing

### Efficient Batch Publishing

```rust
use kincir::{Publisher, Message};
use kincir::memory::InMemoryPublisher;
use std::sync::Arc;
use std::time::Instant;

pub struct BatchPublisher {
    publisher: InMemoryPublisher,
    batch_size: usize,
    batch_timeout: std::time::Duration,
}

impl BatchPublisher {
    pub fn new(
        broker: Arc<kincir::memory::InMemoryBroker>,
        batch_size: usize,
        batch_timeout: std::time::Duration,
    ) -> Self {
        Self {
            publisher: InMemoryPublisher::new(broker),
            batch_size,
            batch_timeout,
        }
    }
    
    pub async fn publish_batch(
        &self,
        topic: &str,
        messages: Vec<Message>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let start = Instant::now();
        
        // Process messages in batches
        for chunk in messages.chunks(self.batch_size) {
            self.publisher.publish(topic, chunk.to_vec()).await?;
            
            // Optional: small delay between batches to prevent overwhelming
            if chunk.len() == self.batch_size {
                tokio::time::sleep(std::time::Duration::from_micros(100)).await;
            }
        }
        
        let duration = start.elapsed();
        println!("Published {} messages in {} batches ({:?})", 
                 messages.len(), 
                 (messages.len() + self.batch_size - 1) / self.batch_size,
                 duration);
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let batch_publisher = BatchPublisher::new(
        broker,
        1000, // Batch size
        std::time::Duration::from_millis(100), // Batch timeout
    );
    
    // Create a large number of messages
    let messages: Vec<Message> = (0..50_000)
        .map(|i| Message::new(format!("Batch message {}", i).into_bytes()))
        .collect();
    
    batch_publisher.publish_batch("batch-topic", messages).await?;
    
    Ok(())
}
```

### Batch Message Processing

```rust
use kincir::{Subscriber, Message};
use kincir::memory::InMemorySubscriber;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;

pub struct BatchProcessor {
    subscriber: InMemorySubscriber,
    batch_size: usize,
    batch_timeout: Duration,
}

impl BatchProcessor {
    pub fn new(
        broker: Arc<kincir::memory::InMemoryBroker>,
        batch_size: usize,
        batch_timeout: Duration,
    ) -> Self {
        Self {
            subscriber: InMemorySubscriber::new(broker),
            batch_size,
            batch_timeout,
        }
    }
    
    pub async fn subscribe(&mut self, topic: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.subscriber.subscribe(topic).await
    }
    
    pub async fn process_batches<F>(&mut self, mut processor: F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(Vec<Message>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>,
    {
        let mut batch = Vec::with_capacity(self.batch_size);
        let mut last_batch_time = Instant::now();
        
        loop {
            // Try to receive a message with timeout
            let receive_timeout = Duration::from_millis(100);
            match timeout(receive_timeout, self.subscriber.receive()).await {
                Ok(Ok(message)) => {
                    batch.push(message);
                    
                    // Process batch if it's full or timeout exceeded
                    if batch.len() >= self.batch_size || 
                       last_batch_time.elapsed() >= self.batch_timeout {
                        if !batch.is_empty() {
                            let batch_to_process = std::mem::take(&mut batch);
                            processor(batch_to_process).await?;
                            last_batch_time = Instant::now();
                        }
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Error receiving message: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(_) => {
                    // Timeout - process any pending messages
                    if !batch.is_empty() && last_batch_time.elapsed() >= self.batch_timeout {
                        let batch_to_process = std::mem::take(&mut batch);
                        processor(batch_to_process).await?;
                        last_batch_time = Instant::now();
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let mut batch_processor = BatchProcessor::new(
        broker,
        100, // Process 100 messages at a time
        Duration::from_millis(500), // Or every 500ms
    );
    
    batch_processor.subscribe("batch-topic").await?;
    
    batch_processor.process_batches(|batch| {
        Box::pin(async move {
            println!("Processing batch of {} messages", batch.len());
            
            // Simulate batch processing
            for (i, message) in batch.iter().enumerate() {
                println!("  Message {}: {:?}", i, String::from_utf8_lossy(&message.payload));
            }
            
            // Simulate processing time
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            Ok(())
        })
    }).await?;
    
    Ok(())
}
```

## Connection Pooling

### RabbitMQ Connection Pool

```rust
use kincir::rabbitmq::RabbitMQPublisher;
use std::sync::Arc;
use tokio::sync::Semaphore;
use std::collections::VecDeque;
use tokio::sync::Mutex;

pub struct RabbitMQConnectionPool {
    publishers: Arc<Mutex<VecDeque<RabbitMQPublisher>>>,
    semaphore: Arc<Semaphore>,
    connection_string: String,
    max_connections: usize,
}

impl RabbitMQConnectionPool {
    pub fn new(connection_string: String, max_connections: usize) -> Self {
        Self {
            publishers: Arc::new(Mutex::new(VecDeque::new())),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            connection_string,
            max_connections,
        }
    }
    
    pub async fn get_publisher(&self) -> Result<PooledPublisher, Box<dyn std::error::Error + Send + Sync>> {
        // Acquire semaphore permit
        let permit = self.semaphore.clone().acquire_owned().await?;
        
        // Try to get existing publisher from pool
        let mut publishers = self.publishers.lock().await;
        if let Some(publisher) = publishers.pop_front() {
            return Ok(PooledPublisher {
                publisher: Some(publisher),
                pool: self.publishers.clone(),
                _permit: permit,
            });
        }
        drop(publishers);
        
        // Create new publisher if pool is empty
        let publisher = RabbitMQPublisher::new(&self.connection_string);
        Ok(PooledPublisher {
            publisher: Some(publisher),
            pool: self.publishers.clone(),
            _permit: permit,
        })
    }
}

pub struct PooledPublisher {
    publisher: Option<RabbitMQPublisher>,
    pool: Arc<Mutex<VecDeque<RabbitMQPublisher>>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl PooledPublisher {
    pub fn get(&self) -> &RabbitMQPublisher {
        self.publisher.as_ref().unwrap()
    }
}

impl Drop for PooledPublisher {
    fn drop(&mut self) {
        if let Some(publisher) = self.publisher.take() {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                let mut publishers = pool.lock().await;
                publishers.push_back(publisher);
            });
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = Arc::new(RabbitMQConnectionPool::new(
        "amqp://localhost:5672".to_string(),
        10, // Max 10 connections
    ));
    
    // Use the connection pool
    let pooled_publisher = pool.get_publisher().await?;
    let message = kincir::Message::new(b"Pooled message".to_vec());
    
    pooled_publisher.get().publish("test-topic", vec![message]).await?;
    
    // Publisher is automatically returned to pool when dropped
    
    Ok(())
}
```

## Memory Management

### Memory-Efficient Message Handling

```rust
use kincir::{Message, Publisher, Subscriber};
use std::sync::Arc;
use bytes::Bytes;

// Use Bytes for zero-copy message payloads
pub struct EfficientMessage {
    pub uuid: String,
    pub payload: Bytes, // Zero-copy byte buffer
    pub metadata: std::collections::HashMap<String, String>,
}

impl EfficientMessage {
    pub fn new(payload: Bytes) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4().to_string(),
            payload,
            metadata: std::collections::HashMap::new(),
        }
    }
    
    pub fn from_slice(data: &[u8]) -> Self {
        Self::new(Bytes::copy_from_slice(data))
    }
    
    pub fn from_static(data: &'static [u8]) -> Self {
        Self::new(Bytes::from_static(data))
    }
}

// Memory pool for message reuse
pub struct MessagePool {
    pool: Arc<tokio::sync::Mutex<Vec<Message>>>,
    max_size: usize,
}

impl MessagePool {
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(max_size))),
            max_size,
        }
    }
    
    pub async fn get_message(&self, payload: Vec<u8>) -> Message {
        let mut pool = self.pool.lock().await;
        
        if let Some(mut message) = pool.pop() {
            // Reuse existing message
            message.payload = payload;
            message.metadata.clear();
            message.uuid = uuid::Uuid::new_v4().to_string();
            message
        } else {
            // Create new message
            Message::new(payload)
        }
    }
    
    pub async fn return_message(&self, message: Message) {
        let mut pool = self.pool.lock().await;
        
        if pool.len() < self.max_size {
            pool.push(message);
        }
        // If pool is full, message is dropped and memory is freed
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message_pool = Arc::new(MessagePool::new(1000));
    
    // Use the message pool
    let message = message_pool.get_message(b"Pooled message".to_vec()).await;
    
    // Process the message...
    
    // Return to pool when done
    message_pool.return_message(message).await;
    
    Ok(())
}
```

## Async Optimization

### Efficient Async Patterns

```rust
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use tokio::sync::mpsc;
use futures::stream::{StreamExt, FuturesUnordered};

// Producer-consumer pattern with channels
pub struct AsyncMessageProcessor {
    sender: mpsc::Sender<Message>,
    receiver: Option<mpsc::Receiver<Message>>,
}

impl AsyncMessageProcessor {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer_size);
        Self {
            sender,
            receiver: Some(receiver),
        }
    }
    
    pub async fn send_message(&self, message: Message) -> Result<(), mpsc::error::SendError<Message>> {
        self.sender.send(message).await
    }
    
    pub async fn process_messages<F>(&mut self, mut processor: F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(Message) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>,
    {
        let mut receiver = self.receiver.take().unwrap();
        
        while let Some(message) = receiver.recv().await {
            processor(message).await?;
        }
        
        Ok(())
    }
}

// Parallel message processing
pub async fn process_messages_parallel<F>(
    messages: Vec<Message>,
    concurrency: usize,
    processor: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: Fn(Message) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync + 'static,
{
    let processor = Arc::new(processor);
    let mut futures = FuturesUnordered::new();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    
    for message in messages {
        let processor = processor.clone();
        let semaphore = semaphore.clone();
        
        futures.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await?;
            processor(message).await
        }));
    }
    
    while let Some(result) = futures.next().await {
        result??;
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Example: Process 1000 messages with max 10 concurrent tasks
    let messages: Vec<Message> = (0..1000)
        .map(|i| Message::new(format!("Message {}", i).into_bytes()))
        .collect();
    
    process_messages_parallel(
        messages,
        10, // Max concurrency
        |message| {
            Box::pin(async move {
                // Simulate processing
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                println!("Processed: {:?}", String::from_utf8_lossy(&message.payload));
                Ok(())
            })
        },
    ).await?;
    
    Ok(())
}
```

## Monitoring Performance

### Performance Metrics Collection

```rust
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub messages_published: u64,
    pub messages_received: u64,
    pub total_publish_time: Duration,
    pub total_receive_time: Duration,
    pub error_count: u64,
    pub start_time: Instant,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            messages_published: 0,
            messages_received: 0,
            total_publish_time: Duration::ZERO,
            total_receive_time: Duration::ZERO,
            error_count: 0,
            start_time: Instant::now(),
        }
    }
    
    pub fn publish_throughput(&self) -> f64 {
        if self.total_publish_time.is_zero() {
            0.0
        } else {
            self.messages_published as f64 / self.total_publish_time.as_secs_f64()
        }
    }
    
    pub fn receive_throughput(&self) -> f64 {
        if self.total_receive_time.is_zero() {
            0.0
        } else {
            self.messages_received as f64 / self.total_receive_time.as_secs_f64()
        }
    }
    
    pub fn overall_throughput(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            0.0
        } else {
            (self.messages_published + self.messages_received) as f64 / elapsed
        }
    }
    
    pub fn error_rate(&self) -> f64 {
        let total_operations = self.messages_published + self.messages_received;
        if total_operations == 0 {
            0.0
        } else {
            self.error_count as f64 / total_operations as f64
        }
    }
}

pub struct MetricsCollector {
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(PerformanceMetrics::new())),
        }
    }
    
    pub async fn record_publish(&self, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.messages_published += 1;
        metrics.total_publish_time += duration;
    }
    
    pub async fn record_receive(&self, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.messages_received += 1;
        metrics.total_receive_time += duration;
    }
    
    pub async fn record_error(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.error_count += 1;
    }
    
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }
    
    pub async fn print_summary(&self) {
        let metrics = self.get_metrics().await;
        
        println!("=== Performance Summary ===");
        println!("Messages Published: {}", metrics.messages_published);
        println!("Messages Received: {}", metrics.messages_received);
        println!("Publish Throughput: {:.2} msg/s", metrics.publish_throughput());
        println!("Receive Throughput: {:.2} msg/s", metrics.receive_throughput());
        println!("Overall Throughput: {:.2} msg/s", metrics.overall_throughput());
        println!("Error Rate: {:.4}%", metrics.error_rate() * 100.0);
        println!("Total Runtime: {:?}", metrics.start_time.elapsed());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let metrics = Arc::new(MetricsCollector::new());
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let publisher = kincir::memory::InMemoryPublisher::new(broker.clone());
    let mut subscriber = kincir::memory::InMemorySubscriber::new(broker);
    
    subscriber.subscribe("perf-test").await?;
    
    // Publish messages with timing
    for i in 0..1000 {
        let start = Instant::now();
        let message = Message::new(format!("Message {}", i).into_bytes());
        
        match publisher.publish("perf-test", vec![message]).await {
            Ok(_) => {
                metrics.record_publish(start.elapsed()).await;
            }
            Err(_) => {
                metrics.record_error().await;
            }
        }
    }
    
    // Receive messages with timing
    for _ in 0..1000 {
        let start = Instant::now();
        
        match subscriber.receive().await {
            Ok(_) => {
                metrics.record_receive(start.elapsed()).await;
            }
            Err(_) => {
                metrics.record_error().await;
            }
        }
    }
    
    // Print performance summary
    metrics.print_summary().await;
    
    Ok(())
}
```

## Best Practices Summary

1. **Use appropriate batch sizes** - Balance latency vs throughput
2. **Configure broker settings** - Disable unnecessary features for production
3. **Implement connection pooling** - Reuse connections for external brokers
4. **Monitor memory usage** - Use object pools for high-frequency operations
5. **Optimize async patterns** - Use channels and semaphores for concurrency control
6. **Measure everything** - Collect metrics to identify bottlenecks
7. **Test under load** - Benchmark with realistic workloads
8. **Profile your application** - Use tools like `perf` and `flamegraph`

## Running Performance Tests

```bash
# Clone the repository
git clone https://github.com/rezacute/kincir.git
cd kincir

# Run performance benchmarks
cargo run --release --example performance-benchmark

# Profile with flamegraph (requires cargo-flamegraph)
cargo install flamegraph
cargo flamegraph --example performance-benchmark

# Memory profiling with valgrind (Linux)
valgrind --tool=massif cargo run --release --example performance-benchmark
```

This performance guide provides the foundation for building high-throughput, low-latency applications with Kincir.
