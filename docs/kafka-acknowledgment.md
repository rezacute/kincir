# Kafka Acknowledgment Implementation

This document provides comprehensive documentation for the Apache Kafka acknowledgment handling implementation in Kincir, which enables reliable message processing with manual offset commit control.

## Overview

The Kafka acknowledgment implementation provides:

- **Manual Offset Control**: Explicit control over when offsets are committed
- **Consumer Group Management**: Proper consumer group coordination and offset tracking
- **Batch Commit Optimization**: Efficient batch offset commits for high throughput
- **Partition-Aware Processing**: Handle messages from multiple partitions correctly
- **Requeue/Discard Logic**: Control message redelivery through offset management

## Core Components

### KafkaAckHandle

The `KafkaAckHandle` represents an acknowledgment handle for a specific message received from Kafka.

```rust
use kincir::kafka::KafkaAckHandle;
use kincir::ack::AckHandle;

// Handle provides message information
let message_id = handle.message_id();
let topic = handle.topic();
let delivery_count = handle.delivery_count();
let is_retry = handle.is_retry();
let timestamp = handle.timestamp();

// Kafka-specific information
let partition = handle.partition();
let offset = handle.offset();
let handle_id = handle.handle_id();
```

#### Key Properties

- **Message ID**: Unique identifier for the message
- **Topic**: Kafka topic name where the message was received
- **Partition**: Kafka partition number
- **Offset**: Kafka offset within the partition
- **Delivery Count**: Number of delivery attempts (1 for first delivery)
- **Timestamp**: When the handle was created
- **Handle ID**: Unique identifier for the handle itself

### KafkaAckSubscriber

The `KafkaAckSubscriber` provides Kafka subscription with manual offset commit support.

```rust
use kincir::kafka::KafkaAckSubscriber;
use kincir::ack::AckSubscriber;

// Create subscriber
let brokers = vec!["localhost:9092".to_string()];
let group_id = "my-consumer-group".to_string();
let mut subscriber = KafkaAckSubscriber::new(brokers, group_id).await?;

// Subscribe to a topic
subscriber.subscribe("my-topic").await?;

// Receive message with acknowledgment handle
let (message, ack_handle) = subscriber.receive_with_ack().await?;

// Process message and acknowledge (commit offset)
subscriber.ack(ack_handle).await?;
```

## Basic Usage

### 1. Setup and Subscription

```rust
use kincir::kafka::{KafkaAckSubscriber, KafkaPublisher};
use kincir::ack::AckSubscriber;
use kincir::{Message, Publisher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configuration
    let brokers = vec!["localhost:9092".to_string()];
    let consumer_group = "my-app-group".to_string();
    
    // Create components
    let publisher = KafkaPublisher::new(brokers.clone())?;
    let mut subscriber = KafkaAckSubscriber::new(brokers, consumer_group).await?;
    
    // Subscribe to topic
    subscriber.subscribe("events").await?;
    
    Ok(())
}
```

### 2. Publishing Messages

```rust
// Publish messages normally
let messages = vec![
    Message::new(b"Event 1".to_vec()),
    Message::new(b"Event 2".to_vec()),
];

publisher.publish("events", messages).await?;
```

### 3. Receiving with Acknowledgment

```rust
// Receive message with acknowledgment handle
let (message, ack_handle) = subscriber.receive_with_ack().await?;

println!("Received: {}", String::from_utf8_lossy(&message.payload));
println!("Topic: {}, Partition: {}, Offset: {}", 
         ack_handle.topic(), 
         ack_handle.partition(), 
         ack_handle.offset());
```

## Acknowledgment Operations

### Positive Acknowledgment (Commit Offset)

Acknowledge successful message processing by committing the offset:

```rust
// Process message successfully
let result = process_message(&message).await;

if result.is_ok() {
    // Acknowledge the message (commit offset)
    subscriber.ack(ack_handle).await?;
    println!("Message processed and offset committed");
}
```

### Negative Acknowledgment

Reject a message with control over redelivery:

```rust
// Process message
let result = process_message(&message).await;

if result.is_err() {
    // Decide whether to requeue based on error type
    let should_requeue = is_retryable_error(&result);
    
    if should_requeue {
        // Don't commit offset - message will be redelivered
        subscriber.nack(ack_handle, true).await?;
        println!("Message rejected - will be redelivered on restart/rebalance");
    } else {
        // Commit offset to discard message
        subscriber.nack(ack_handle, false).await?;
        println!("Message rejected and discarded");
    }
}
```

### Batch Operations

Process multiple messages efficiently with batch offset commits:

```rust
// Collect messages and handles
let mut messages = Vec::new();
let mut ack_handles = Vec::new();

// Receive batch of messages
for _ in 0..10 {
    let (message, handle) = subscriber.receive_with_ack().await?;
    messages.push(message);
    ack_handles.push(handle);
}

// Process all messages
let results = process_batch(&messages).await;

// Separate successful and failed handles
let (success_handles, failed_handles): (Vec<_>, Vec<_>) = 
    ack_handles.into_iter()
        .zip(results.iter())
        .partition(|(_, result)| result.is_ok());

// Batch acknowledge successful messages
if !success_handles.is_empty() {
    let handles: Vec<_> = success_handles.into_iter().map(|(h, _)| h).collect();
    subscriber.ack_batch(handles).await?;
}

// Batch reject failed messages
if !failed_handles.is_empty() {
    let handles: Vec<_> = failed_handles.into_iter().map(|(h, _)| h).collect();
    subscriber.nack_batch(handles, true).await?; // Requeue for retry
}
```

## Kafka-Specific Concepts

### Offset Management

In Kafka, acknowledgment is implemented through offset commits:

- **Positive Acknowledgment**: Commits the offset, marking the message as processed
- **Negative Acknowledgment with Requeue**: Doesn't commit offset, message redelivered later
- **Negative Acknowledgment without Requeue**: Commits offset to skip the message

```rust
// Understanding offset behavior
let (message, handle) = subscriber.receive_with_ack().await?;
println!("Current offset: {}", handle.offset());

// Acknowledge - commits offset + 1
subscriber.ack(handle).await?;
// Next consumer will start from offset + 1

// Or reject without requeue - also commits offset + 1
subscriber.nack(handle, false).await?;
// Message is effectively discarded

// Or reject with requeue - doesn't commit offset
subscriber.nack(handle, true).await?;
// Message will be redelivered on consumer restart/rebalance
```

### Consumer Groups

Kafka consumer groups provide load balancing and fault tolerance:

```rust
// Multiple consumers in the same group
let group_id = "order-processing-group".to_string();

// Consumer 1
let mut consumer1 = KafkaAckSubscriber::new(brokers.clone(), group_id.clone()).await?;
consumer1.subscribe("orders").await?;

// Consumer 2 (same group)
let mut consumer2 = KafkaAckSubscriber::new(brokers.clone(), group_id.clone()).await?;
consumer2.subscribe("orders").await?;

// Kafka will distribute partitions between consumers
// Each message is delivered to only one consumer in the group
```

### Partition Handling

Kafka topics are divided into partitions for scalability:

```rust
// Handle messages from multiple partitions
loop {
    let (message, handle) = subscriber.receive_with_ack().await?;
    
    println!("Processing message from partition {} at offset {}", 
             handle.partition(), handle.offset());
    
    // Process based on partition if needed
    match handle.partition() {
        0 => process_partition_0(&message).await?,
        1 => process_partition_1(&message).await?,
        _ => process_default(&message).await?,
    }
    
    subscriber.ack(handle).await?;
}
```

## Error Handling and Retry Logic

### Basic Retry Pattern

```rust
async fn process_with_retry(
    subscriber: &KafkaAckSubscriber,
    message: Message,
    ack_handle: KafkaAckHandle,
    max_retries: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let current_attempt = ack_handle.delivery_count();
    
    match process_message(&message).await {
        Ok(_) => {
            // Success - commit offset
            subscriber.ack(ack_handle).await?;
            println!("Message processed successfully on attempt {}", current_attempt);
        }
        Err(e) if current_attempt < max_retries => {
            // Retryable error - don't commit offset
            println!("Processing failed on attempt {} - will retry: {}", current_attempt, e);
            subscriber.nack(ack_handle, true).await?;
        }
        Err(e) => {
            // Max retries exceeded - discard
            println!("Max retries exceeded - discarding message: {}", e);
            subscriber.nack(ack_handle, false).await?;
        }
    }
    
    Ok(())
}
```

### Advanced Error Handling with Dead Letter Topics

```rust
async fn advanced_error_handling(
    subscriber: &KafkaAckSubscriber,
    dead_letter_publisher: &KafkaPublisher,
    message: Message,
    ack_handle: KafkaAckHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match process_message(&message).await {
        Ok(_) => {
            subscriber.ack(ack_handle).await?;
        }
        Err(e) => {
            match classify_error(&e) {
                ErrorType::Transient if ack_handle.delivery_count() < 5 => {
                    // Transient error - requeue for retry
                    subscriber.nack(ack_handle, true).await?;
                }
                ErrorType::Permanent | _ => {
                    // Permanent error or max retries - send to dead letter topic
                    let dead_letter_msg = message
                        .with_metadata("error", &e.to_string())
                        .with_metadata("original_topic", ack_handle.topic())
                        .with_metadata("original_partition", &ack_handle.partition().to_string())
                        .with_metadata("original_offset", &ack_handle.offset().to_string());
                    
                    dead_letter_publisher
                        .publish("dead-letter-topic", vec![dead_letter_msg])
                        .await?;
                    
                    // Commit original offset to avoid reprocessing
                    subscriber.nack(ack_handle, false).await?;
                }
            }
        }
    }
    
    Ok(())
}

enum ErrorType {
    Transient,
    Permanent,
}

fn classify_error(error: &dyn std::error::Error) -> ErrorType {
    // Classify error based on type, message, etc.
    // This is application-specific logic
    ErrorType::Transient
}
```

## Configuration and Best Practices

### Consumer Configuration

```rust
// The KafkaAckSubscriber uses these default configurations:
// - enable.auto.commit = false (manual commit for acknowledgment control)
// - auto.offset.reset = earliest (start from beginning if no committed offset)
// - session.timeout.ms = 6000 (consumer session timeout)
// - enable.partition.eof = false (don't signal end of partition)

let brokers = vec!["kafka1:9092".to_string(), "kafka2:9092".to_string()];
let group_id = "my-service-v1".to_string();
let subscriber = KafkaAckSubscriber::new(brokers, group_id).await?;
```

### Logging Integration

```rust
#[cfg(feature = "logging")]
use kincir::logging::StdLogger;

#[cfg(feature = "logging")]
{
    let logger = Arc::new(StdLogger::new(true, true)); // info=true, debug=true
    let subscriber = KafkaAckSubscriber::new(brokers, group_id)
        .await?
        .with_logger(logger);
}
```

### Performance Considerations

1. **Batch Processing**: Use batch acknowledgment for high-throughput scenarios
2. **Partition Assignment**: Ensure adequate partitions for parallelism
3. **Consumer Group Size**: Match consumer count to partition count for optimal distribution
4. **Offset Commit Frequency**: Balance between performance and durability

### Best Practices

1. **Always Handle Acknowledgments**: Never leave messages unacknowledged
2. **Use Consumer Groups**: Leverage Kafka's built-in load balancing
3. **Monitor Lag**: Track consumer lag to ensure timely processing
4. **Handle Rebalances**: Design for partition rebalancing scenarios
5. **Implement Dead Letter Topics**: Handle poison messages gracefully
6. **Use Batch Operations**: Optimize throughput with batch commits

## Integration with Existing Code

### Migrating from Auto-Commit

```rust
// Before: Auto-commit (not available in current implementation)
// This would be a hypothetical auto-commit subscriber

// After: Manual acknowledgment
let mut subscriber = KafkaAckSubscriber::new(brokers, group_id).await?;
subscriber.subscribe("topic").await?;
let (message, ack_handle) = subscriber.receive_with_ack().await?;
// Process message
subscriber.ack(ack_handle).await?; // Manual acknowledgment
```

### Compatibility Layer

You can create a compatibility layer for gradual migration:

```rust
use kincir::ack::CompatSubscriber;

// Wrap acknowledgment subscriber for auto-ack behavior
let ack_subscriber = KafkaAckSubscriber::new(brokers, group_id).await?;
let mut compat_subscriber = CompatSubscriber::new(ack_subscriber);

// Use like a regular subscriber (auto-acknowledges)
compat_subscriber.subscribe("topic").await?;
let message = compat_subscriber.receive().await?;
// Message is automatically acknowledged
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ack_handle_properties() {
        let handle = KafkaAckHandle::new(
            "msg-123".to_string(),
            "test-topic".to_string(),
            SystemTime::now(),
            1,
            0,
            42,
        );
        
        assert_eq!(handle.message_id(), "msg-123");
        assert_eq!(handle.topic(), "test-topic");
        assert_eq!(handle.partition(), 0);
        assert_eq!(handle.offset(), 42);
        assert!(!handle.is_retry());
    }
}
```

### Integration Tests

Integration tests require a running Kafka instance:

```bash
# Start Kafka with Docker
docker run -d --name kafka -p 9092:9092 apache/kafka:latest

# Run integration tests
cargo test kafka_ack_tests --test kafka_ack_tests
```

## Troubleshooting

### Common Issues

1. **Connection Failures**: Verify Kafka is running and accessible
2. **Consumer Group Conflicts**: Ensure unique consumer group IDs
3. **Offset Reset Issues**: Check auto.offset.reset configuration
4. **Partition Rebalancing**: Handle rebalance scenarios gracefully
5. **Commit Failures**: Check for network issues or broker problems

### Debugging

Enable logging to debug acknowledgment operations:

```rust
#[cfg(feature = "logging")]
{
    let logger = Arc::new(StdLogger::new(true, true));
    let subscriber = subscriber.with_logger(logger);
}
```

### Monitoring

Monitor key metrics:
- Consumer lag per partition
- Offset commit rate
- Message processing rate
- Error rates
- Rebalance frequency

## Examples

See the complete working example in `examples/kafka_ack_example.rs`:

```bash
cargo run --example kafka_ack_example
```

This example demonstrates:
- Basic acknowledgment
- Negative acknowledgment with requeue
- Batch operations
- Offset management and consumer groups

## Conclusion

The Kafka acknowledgment implementation provides robust, scalable message processing with full control over offset commits. It integrates seamlessly with Kafka's consumer group model while providing advanced features for production use cases.

Key benefits:
- **Reliability**: Guaranteed message processing with offset control
- **Scalability**: Leverages Kafka's partition-based architecture
- **Performance**: Batch operations for high-throughput scenarios
- **Integration**: Native Kafka semantics and consumer group support
- **Monitoring**: Comprehensive offset tracking and logging capabilities
