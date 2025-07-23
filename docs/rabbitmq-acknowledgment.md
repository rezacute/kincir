# RabbitMQ Acknowledgment Implementation

This document provides comprehensive documentation for the RabbitMQ acknowledgment handling implementation in Kincir, which enables reliable message processing with manual acknowledgment control.

## Overview

The RabbitMQ acknowledgment implementation provides:

- **Manual Acknowledgment Control**: Explicit control over when messages are acknowledged
- **Negative Acknowledgment Support**: Ability to reject messages with optional requeue
- **Batch Operations**: Efficient batch acknowledgment and negative acknowledgment
- **Delivery Tracking**: Track delivery attempts and retry status
- **RabbitMQ Integration**: Native RabbitMQ acknowledgment semantics

## Core Components

### RabbitMQAckHandle

The `RabbitMQAckHandle` represents an acknowledgment handle for a specific message received from RabbitMQ.

```rust
use kincir::rabbitmq::RabbitMQAckHandle;
use kincir::ack::AckHandle;

// Handle provides message information
let message_id = handle.message_id();
let topic = handle.topic();
let delivery_count = handle.delivery_count();
let is_retry = handle.is_retry();
let timestamp = handle.timestamp();

// RabbitMQ-specific information
let delivery_tag = handle.delivery_tag();
let handle_id = handle.handle_id();
```

#### Key Properties

- **Message ID**: Unique identifier for the message
- **Topic**: Queue/topic name where the message was received
- **Delivery Count**: Number of delivery attempts (1 for first delivery)
- **Timestamp**: When the handle was created
- **Delivery Tag**: RabbitMQ delivery tag for acknowledgment operations
- **Handle ID**: Unique identifier for the handle itself

### RabbitMQAckSubscriber

The `RabbitMQAckSubscriber` provides RabbitMQ subscription with manual acknowledgment support.

```rust
use kincir::rabbitmq::RabbitMQAckSubscriber;
use kincir::ack::AckSubscriber;

// Create subscriber
let mut subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@localhost:5672").await?;

// Subscribe to a queue
subscriber.subscribe("my-queue").await?;

// Receive message with acknowledgment handle
let (message, ack_handle) = subscriber.receive_with_ack().await?;

// Process message and acknowledge
subscriber.ack(ack_handle).await?;
```

## Basic Usage

### 1. Setup and Subscription

```rust
use kincir::rabbitmq::{RabbitMQAckSubscriber, RabbitMQPublisher};
use kincir::ack::AckSubscriber;
use kincir::{Message, Publisher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create components
    let publisher = RabbitMQPublisher::new("amqp://guest:guest@localhost:5672").await?;
    let mut subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@localhost:5672").await?;
    
    // Subscribe to queue
    subscriber.subscribe("work-queue").await?;
    
    Ok(())
}
```

### 2. Publishing Messages

```rust
// Publish messages normally
let messages = vec![
    Message::new(b"Task 1".to_vec()),
    Message::new(b"Task 2".to_vec()),
];

publisher.publish("work-queue", messages).await?;
```

### 3. Receiving with Acknowledgment

```rust
// Receive message with acknowledgment handle
let (message, ack_handle) = subscriber.receive_with_ack().await?;

println!("Received: {}", String::from_utf8_lossy(&message.payload));
println!("Message ID: {}", ack_handle.message_id());
println!("Delivery count: {}", ack_handle.delivery_count());
```

## Acknowledgment Operations

### Positive Acknowledgment

Acknowledge successful message processing:

```rust
// Process message successfully
let result = process_message(&message).await;

if result.is_ok() {
    // Acknowledge the message
    subscriber.ack(ack_handle).await?;
    println!("Message processed and acknowledged");
}
```

### Negative Acknowledgment

Reject a message with optional requeue:

```rust
// Process message
let result = process_message(&message).await;

if result.is_err() {
    // Decide whether to requeue based on error type
    let should_requeue = is_retryable_error(&result);
    
    // Negatively acknowledge
    subscriber.nack(ack_handle, should_requeue).await?;
    
    if should_requeue {
        println!("Message rejected and requeued for retry");
    } else {
        println!("Message rejected and discarded");
    }
}
```

### Batch Operations

Process multiple messages efficiently:

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

## Error Handling and Retry Logic

### Basic Retry Pattern

```rust
async fn process_with_retry(
    subscriber: &RabbitMQAckSubscriber,
    message: Message,
    ack_handle: RabbitMQAckHandle,
    max_retries: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let current_attempt = ack_handle.delivery_count();
    
    match process_message(&message).await {
        Ok(_) => {
            // Success - acknowledge
            subscriber.ack(ack_handle).await?;
            println!("Message processed successfully on attempt {}", current_attempt);
        }
        Err(e) if current_attempt < max_retries => {
            // Retryable error - requeue
            println!("Processing failed on attempt {} - requeuing: {}", current_attempt, e);
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

### Advanced Error Handling

```rust
async fn advanced_error_handling(
    subscriber: &RabbitMQAckSubscriber,
    message: Message,
    ack_handle: RabbitMQAckHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match process_message(&message).await {
        Ok(_) => {
            subscriber.ack(ack_handle).await?;
        }
        Err(e) => {
            match classify_error(&e) {
                ErrorType::Transient if ack_handle.delivery_count() < 5 => {
                    // Transient error - requeue with exponential backoff
                    let delay = Duration::from_millis(100 * 2_u64.pow(ack_handle.delivery_count()));
                    tokio::time::sleep(delay).await;
                    subscriber.nack(ack_handle, true).await?;
                }
                ErrorType::Permanent => {
                    // Permanent error - send to dead letter queue
                    send_to_dead_letter_queue(&message).await?;
                    subscriber.nack(ack_handle, false).await?;
                }
                _ => {
                    // Max retries or unknown error - discard
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
    Unknown,
}

fn classify_error(error: &dyn std::error::Error) -> ErrorType {
    // Classify error based on type, message, etc.
    // This is application-specific logic
    ErrorType::Transient
}
```

## Configuration and Best Practices

### Connection Configuration

```rust
// Basic connection
let subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@localhost:5672").await?;

// With custom connection parameters
let uri = "amqp://user:pass@rabbitmq.example.com:5672/vhost?heartbeat=30";
let subscriber = RabbitMQAckSubscriber::new(uri).await?;
```

### Logging Integration

```rust
#[cfg(feature = "logging")]
use kincir::logging::StdLogger;

#[cfg(feature = "logging")]
{
    let logger = Arc::new(StdLogger::new(true, true)); // info=true, debug=true
    let subscriber = RabbitMQAckSubscriber::new("amqp://localhost:5672")
        .await?
        .with_logger(logger);
}
```

### Performance Considerations

1. **Batch Processing**: Use batch acknowledgment for high-throughput scenarios
2. **Connection Pooling**: Reuse connections when possible
3. **Prefetch Settings**: Configure RabbitMQ prefetch for optimal performance
4. **Error Classification**: Quickly classify errors to avoid unnecessary retries

### Best Practices

1. **Always Handle Acknowledgments**: Never leave messages unacknowledged
2. **Implement Timeouts**: Use timeouts for message processing
3. **Monitor Delivery Counts**: Track retry attempts to prevent infinite loops
4. **Use Dead Letter Queues**: Configure dead letter exchanges for failed messages
5. **Log Acknowledgment Operations**: Log ack/nack operations for debugging

## Integration with Existing Code

### Migrating from Auto-Acknowledgment

```rust
// Before: Auto-acknowledgment
let mut subscriber = RabbitMQSubscriber::new("amqp://localhost:5672").await?;
subscriber.subscribe("queue").await?;
let message = subscriber.receive().await?;
// Message is automatically acknowledged

// After: Manual acknowledgment
let mut subscriber = RabbitMQAckSubscriber::new("amqp://localhost:5672").await?;
subscriber.subscribe("queue").await?;
let (message, ack_handle) = subscriber.receive_with_ack().await?;
// Process message
subscriber.ack(ack_handle).await?; // Manual acknowledgment
```

### Compatibility Layer

You can create a compatibility layer for gradual migration:

```rust
use kincir::ack::CompatSubscriber;

// Wrap acknowledgment subscriber for auto-ack behavior
let ack_subscriber = RabbitMQAckSubscriber::new("amqp://localhost:5672").await?;
let mut compat_subscriber = CompatSubscriber::new(ack_subscriber);

// Use like a regular subscriber (auto-acknowledges)
compat_subscriber.subscribe("queue").await?;
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
        let handle = RabbitMQAckHandle::new(
            "msg-123".to_string(),
            "test-queue".to_string(),
            SystemTime::now(),
            1,
            42,
        );
        
        assert_eq!(handle.message_id(), "msg-123");
        assert_eq!(handle.topic(), "test-queue");
        assert!(!handle.is_retry());
    }
}
```

### Integration Tests

Integration tests require a running RabbitMQ instance:

```bash
# Start RabbitMQ with Docker
docker run -d --name rabbitmq -p 5672:5672 -p 15672:15672 rabbitmq:3-management

# Run integration tests
cargo test rabbitmq_ack_tests --test rabbitmq_ack_tests
```

## Troubleshooting

### Common Issues

1. **Connection Failures**: Verify RabbitMQ is running and accessible
2. **Authentication Errors**: Check username/password in connection URI
3. **Queue Not Found**: Ensure queue exists or is declared properly
4. **Acknowledgment Timeouts**: Check for network issues or processing delays

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
- Message acknowledgment rate
- Negative acknowledgment rate
- Retry counts
- Processing latency
- Queue depth

## Examples

See the complete working example in `examples/rabbitmq_ack_example.rs`:

```bash
cargo run --example rabbitmq_ack_example
```

This example demonstrates:
- Basic acknowledgment
- Negative acknowledgment with requeue
- Batch operations
- Error handling and retry logic

## Conclusion

The RabbitMQ acknowledgment implementation provides robust, reliable message processing with full control over acknowledgment behavior. It integrates seamlessly with existing Kincir applications while providing advanced features for production use cases.

Key benefits:
- **Reliability**: Guaranteed message processing with acknowledgments
- **Flexibility**: Manual control over acknowledgment timing
- **Performance**: Batch operations for high-throughput scenarios
- **Integration**: Native RabbitMQ semantics and error handling
- **Monitoring**: Comprehensive tracking and logging capabilities
