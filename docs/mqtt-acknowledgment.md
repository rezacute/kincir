# MQTT Acknowledgment Implementation

This document provides comprehensive documentation for the MQTT acknowledgment handling implementation in Kincir, which enables reliable message processing with QoS-aware acknowledgment control.

## Overview

The MQTT acknowledgment implementation provides:

- **QoS-Aware Acknowledgment**: Different acknowledgment behavior based on MQTT Quality of Service levels
- **Persistent Sessions**: Support for persistent sessions to enable message redelivery
- **Packet ID Tracking**: Track MQTT packet identifiers for QoS > 0 messages
- **Connection Recovery**: Handle connection recovery scenarios with proper message redelivery
- **MQTT-Specific Semantics**: Native MQTT acknowledgment behavior and protocol compliance

## Core Components

### MQTTAckHandle

The `MQTTAckHandle` represents an acknowledgment handle for a specific message received from an MQTT broker.

```rust
use kincir::mqtt::{MQTTAckHandle, QoS};
use kincir::ack::AckHandle;

// Handle provides message information
let message_id = handle.message_id();
let topic = handle.topic();
let delivery_count = handle.delivery_count();
let is_retry = handle.is_retry();
let timestamp = handle.timestamp();

// MQTT-specific information
let qos = handle.qos();
let packet_id = handle.packet_id();
let requires_ack = handle.requires_ack();
let handle_id = handle.handle_id();
```

#### Key Properties

- **Message ID**: Unique identifier for the message
- **Topic**: MQTT topic name where the message was received
- **QoS Level**: MQTT Quality of Service level (0, 1, or 2)
- **Packet ID**: MQTT packet identifier for QoS > 0 messages
- **Delivery Count**: Number of delivery attempts (1 for first delivery)
- **Timestamp**: When the handle was created
- **Requires Ack**: Whether this message requires acknowledgment based on QoS
- **Handle ID**: Unique identifier for the handle itself

### MQTTAckSubscriber

The `MQTTAckSubscriber` provides MQTT subscription with manual acknowledgment support.

```rust
use kincir::mqtt::{MQTTAckSubscriber, QoS};
use kincir::ack::AckSubscriber;

// Create subscriber
let broker_url = "127.0.0.1";
let client_id = Some("my-client-id".to_string());
let mut subscriber = MQTTAckSubscriber::new(broker_url, client_id).await?;

// Subscribe to a topic with specific QoS
subscriber.subscribe_with_qos("sensors/temperature", QoS::AtLeastOnce).await?;

// Receive message with acknowledgment handle
let (message, ack_handle) = subscriber.receive_with_ack().await?;

// Process message and acknowledge
subscriber.ack(ack_handle).await?;
```

## MQTT Quality of Service Levels

MQTT defines three QoS levels that affect acknowledgment behavior:

### QoS 0 - At Most Once (Fire and Forget)

```rust
// Subscribe with QoS 0
subscriber.subscribe_with_qos("notifications", QoS::AtMostOnce).await?;

let (message, ack_handle) = subscriber.receive_with_ack().await?;

// QoS 0 messages don't require acknowledgment
assert_eq!(ack_handle.qos(), QoS::AtMostOnce);
assert!(!ack_handle.requires_ack());
assert_eq!(ack_handle.packet_id(), None);

// Acknowledging is a no-op for QoS 0
subscriber.ack(ack_handle).await?;
```

### QoS 1 - At Least Once

```rust
// Subscribe with QoS 1
subscriber.subscribe_with_qos("orders", QoS::AtLeastOnce).await?;

let (message, ack_handle) = subscriber.receive_with_ack().await?;

// QoS 1 messages require acknowledgment
assert_eq!(ack_handle.qos(), QoS::AtLeastOnce);
assert!(ack_handle.requires_ack());
assert!(ack_handle.packet_id().is_some());

// Acknowledge sends PUBACK to broker
subscriber.ack(ack_handle).await?;
```

### QoS 2 - Exactly Once

```rust
// Subscribe with QoS 2
subscriber.subscribe_with_qos("payments", QoS::ExactlyOnce).await?;

let (message, ack_handle) = subscriber.receive_with_ack().await?;

// QoS 2 messages require acknowledgment
assert_eq!(ack_handle.qos(), QoS::ExactlyOnce);
assert!(ack_handle.requires_ack());
assert!(ack_handle.packet_id().is_some());

// Acknowledge completes PUBREC/PUBREL/PUBCOMP handshake
subscriber.ack(ack_handle).await?;
```

## Basic Usage

### 1. Setup and Subscription

```rust
use kincir::mqtt::{MQTTAckSubscriber, MQTTPublisher, QoS};
use kincir::ack::AckSubscriber;
use kincir::{Message, Publisher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configuration
    let broker_url = "127.0.0.1";
    let client_id = Some("my-app-client".to_string());
    
    // Create components
    let publisher = MQTTPublisher::new(broker_url, "events")?;
    let mut subscriber = MQTTAckSubscriber::new(broker_url, client_id).await?;
    
    // Subscribe to topic with QoS 1
    subscriber.subscribe_with_qos("events", QoS::AtLeastOnce).await?;
    
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
println!("QoS: {:?}, Packet ID: {:?}, Requires Ack: {}", 
         ack_handle.qos(), 
         ack_handle.packet_id(),
         ack_handle.requires_ack());
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

Reject a message with control over redelivery:

```rust
// Process message
let result = process_message(&message).await;

if result.is_err() {
    // Decide whether to requeue based on error type
    let should_requeue = is_retryable_error(&result);
    
    if should_requeue {
        // Don't acknowledge - message will be redelivered on reconnection
        subscriber.nack(ack_handle, true).await?;
        println!("Message rejected - will be redelivered on reconnection");
    } else {
        // Acknowledge to prevent redelivery
        subscriber.nack(ack_handle, false).await?;
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

## MQTT-Specific Concepts

### Persistent Sessions

MQTT persistent sessions enable message redelivery:

```rust
// The MQTTAckSubscriber automatically uses persistent sessions
// (clean_session = false) to enable message redelivery for QoS > 0

let mut subscriber = MQTTAckSubscriber::new(broker_url, client_id).await?;

// Messages not acknowledged will be redelivered on reconnection
let (message, handle) = subscriber.receive_with_ack().await?;

// If we don't acknowledge and disconnect, the message will be redelivered
// when we reconnect with the same client ID
```

### Connection Recovery

Handle connection recovery scenarios:

```rust
// MQTT acknowledgment behavior during connection issues:

let (message, handle) = subscriber.receive_with_ack().await?;

// If connection is lost before acknowledgment:
// - QoS 0: Message is lost (fire and forget)
// - QoS 1: Message will be redelivered on reconnection
// - QoS 2: Message will be redelivered on reconnection

// Proper error handling
match subscriber.ack(handle).await {
    Ok(_) => println!("Message acknowledged successfully"),
    Err(e) => {
        println!("Acknowledgment failed: {}", e);
        // Handle connection recovery, retry logic, etc.
    }
}
```

### Topic Patterns and Wildcards

MQTT supports topic wildcards in subscriptions:

```rust
// Subscribe to multiple topics with wildcards
subscriber.subscribe_with_qos("sensors/+/temperature", QoS::AtLeastOnce).await?;
subscriber.subscribe_with_qos("alerts/#", QoS::ExactlyOnce).await?;

// Handle messages from different topics
let (message, handle) = subscriber.receive_with_ack().await?;
println!("Received from topic: {}", handle.topic());

// Topic-specific processing
match handle.topic() {
    topic if topic.starts_with("sensors/") => process_sensor_data(&message).await?,
    topic if topic.starts_with("alerts/") => process_alert(&message).await?,
    _ => process_default(&message).await?,
}

subscriber.ack(handle).await?;
```

## Error Handling and Retry Logic

### Basic Retry Pattern

```rust
async fn process_with_retry(
    subscriber: &MQTTAckSubscriber,
    message: Message,
    ack_handle: MQTTAckHandle,
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
            // Retryable error - don't acknowledge (will be redelivered)
            println!("Processing failed on attempt {} - will retry: {}", current_attempt, e);
            subscriber.nack(ack_handle, true).await?;
        }
        Err(e) => {
            // Max retries exceeded - acknowledge to prevent redelivery
            println!("Max retries exceeded - discarding message: {}", e);
            subscriber.nack(ack_handle, false).await?;
        }
    }
    
    Ok(())
}
```

### QoS-Aware Error Handling

```rust
async fn qos_aware_error_handling(
    subscriber: &MQTTAckSubscriber,
    message: Message,
    ack_handle: MQTTAckHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match process_message(&message).await {
        Ok(_) => {
            subscriber.ack(ack_handle).await?;
        }
        Err(e) => {
            match ack_handle.qos() {
                QoS::AtMostOnce => {
                    // QoS 0 - no redelivery possible, just log error
                    println!("QoS 0 message processing failed: {}", e);
                    subscriber.ack(ack_handle).await?; // No-op for QoS 0
                }
                QoS::AtLeastOnce | QoS::ExactlyOnce => {
                    // QoS 1/2 - can be redelivered
                    if is_retryable_error(&e) && ack_handle.delivery_count() < 3 {
                        println!("Retryable error - message will be redelivered: {}", e);
                        subscriber.nack(ack_handle, true).await?;
                    } else {
                        println!("Non-retryable error or max retries - discarding: {}", e);
                        subscriber.nack(ack_handle, false).await?;
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn is_retryable_error(error: &dyn std::error::Error) -> bool {
    // Classify error based on type, message, etc.
    // This is application-specific logic
    true
}
```

## Configuration and Best Practices

### Client Configuration

```rust
// The MQTTAckSubscriber uses these configurations:
// - clean_session = false (persistent session for message redelivery)
// - keep_alive = 30 seconds (connection keepalive)
// - Auto-reconnection handled by rumqttc library

let broker_url = "mqtt.example.com";
let client_id = Some("my-service-v1".to_string());
let subscriber = MQTTAckSubscriber::new(broker_url, client_id).await?;
```

### Logging Integration

```rust
#[cfg(feature = "logging")]
use kincir::logging::StdLogger;

#[cfg(feature = "logging")]
{
    let logger = Arc::new(StdLogger::new(true, true)); // info=true, debug=true
    let subscriber = MQTTAckSubscriber::new(broker_url, client_id)
        .await?
        .with_logger(logger);
}
```

### Performance Considerations

1. **QoS Selection**: Choose appropriate QoS level for your use case
   - QoS 0: Best performance, no delivery guarantee
   - QoS 1: Good performance, at-least-once delivery
   - QoS 2: Highest reliability, exactly-once delivery (slower)

2. **Client ID Management**: Use unique, persistent client IDs for session continuity

3. **Connection Pooling**: Reuse connections when possible

4. **Topic Design**: Design topic hierarchy for efficient routing

### Best Practices

1. **Always Handle Acknowledgments**: Never leave QoS > 0 messages unacknowledged
2. **Use Persistent Sessions**: Enable persistent sessions for reliable delivery
3. **Monitor Connection State**: Handle connection loss and recovery gracefully
4. **Implement Retry Logic**: Design appropriate retry strategies for your use case
5. **Use Appropriate QoS**: Match QoS level to message importance
6. **Handle Topic Wildcards**: Design for scalable topic patterns

## Integration with Existing Code

### Migrating from Auto-Acknowledgment

```rust
// Before: Auto-acknowledgment (hypothetical)
// This would be automatic acknowledgment in the regular MQTT subscriber

// After: Manual acknowledgment
let mut subscriber = MQTTAckSubscriber::new(broker_url, client_id).await?;
subscriber.subscribe_with_qos("topic", QoS::AtLeastOnce).await?;
let (message, ack_handle) = subscriber.receive_with_ack().await?;
// Process message
subscriber.ack(ack_handle).await?; // Manual acknowledgment
```

### Compatibility Layer

You can create a compatibility layer for gradual migration:

```rust
use kincir::ack::CompatSubscriber;

// Wrap acknowledgment subscriber for auto-ack behavior
let ack_subscriber = MQTTAckSubscriber::new(broker_url, client_id).await?;
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
        let handle = MQTTAckHandle::new(
            "msg-123".to_string(),
            "test/topic".to_string(),
            SystemTime::now(),
            1,
            QoS::AtLeastOnce,
            Some(42),
        );
        
        assert_eq!(handle.message_id(), "msg-123");
        assert_eq!(handle.topic(), "test/topic");
        assert_eq!(handle.qos(), QoS::AtLeastOnce);
        assert_eq!(handle.packet_id(), Some(42));
        assert!(handle.requires_ack());
    }
}
```

### Integration Tests

Integration tests require a running MQTT broker:

```bash
# Start MQTT broker with Docker
docker run -d --name mosquitto -p 1883:1883 eclipse-mosquitto:latest

# Run integration tests
cargo test mqtt_ack_tests --test mqtt_ack_tests
```

## Troubleshooting

### Common Issues

1. **Connection Failures**: Verify MQTT broker is running and accessible
2. **Authentication Issues**: Check username/password if required
3. **QoS Mismatch**: Ensure publisher and subscriber QoS levels are compatible
4. **Session Persistence**: Verify client ID consistency for persistent sessions
5. **Message Redelivery**: Check acknowledgment logic for QoS > 0 messages

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
- Message acknowledgment rate per QoS level
- Connection stability and reconnection frequency
- Message redelivery rates
- Processing latency by QoS level
- Topic subscription patterns

## Examples

See the complete working example in `examples/mqtt_ack_example.rs`:

```bash
cargo run --example mqtt_ack_example
```

This example demonstrates:
- QoS 0, 1, and 2 acknowledgment behavior
- Negative acknowledgment with requeue
- Batch operations
- MQTT-specific features and semantics

## Conclusion

The MQTT acknowledgment implementation provides reliable, QoS-aware message processing with full control over acknowledgment behavior. It integrates seamlessly with MQTT's quality of service model while providing advanced features for production use cases.

Key benefits:
- **Reliability**: QoS-aware message processing with proper acknowledgment
- **Flexibility**: Manual control over acknowledgment timing and redelivery
- **Performance**: Optimized for different QoS levels and use cases
- **Integration**: Native MQTT semantics and protocol compliance
- **Monitoring**: Comprehensive QoS tracking and logging capabilities
