# Message Acknowledgments Example

Message acknowledgments are crucial for reliable message processing. Kincir v0.2.0 introduces comprehensive acknowledgment support across RabbitMQ, Kafka, and MQTT backends, ensuring messages are processed exactly once and handling failures gracefully.

## Overview

Acknowledgments provide:
- **Reliability**: Messages are not lost if processing fails
- **At-least-once delivery**: Messages are redelivered if not acknowledged
- **Error handling**: Failed messages can be rejected and requeued
- **Flow control**: Prevents overwhelming consumers

## RabbitMQ Acknowledgments

### Basic Acknowledgment Pattern

```rust
use kincir::rabbitmq::RabbitMQAckSubscriber;
use kincir::{AckSubscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut subscriber = RabbitMQAckSubscriber::new("amqp://localhost:5672", "orders-queue");
    subscriber.subscribe("orders").await?;

    println!("Processing orders with acknowledgments...");

    loop {
        // Receive message with acknowledgment handle
        let (message, ack_handle) = subscriber.receive_with_ack().await?;
        
        // Process the message
        match process_order(&message).await {
            Ok(()) => {
                println!("Order processed successfully: {}", 
                        String::from_utf8_lossy(&message.payload));
                // Acknowledge successful processing
                ack_handle.ack().await?;
            }
            Err(e) => {
                eprintln!("Failed to process order: {}", e);
                // Reject and requeue the message for retry
                ack_handle.nack(true).await?;
            }
        }
    }
}

async fn process_order(message: &Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let order_data = String::from_utf8_lossy(&message.payload);
    println!("Processing order: {}", order_data);
    
    // Simulate order validation
    if let Some(order_id) = message.get_metadata("order_id") {
        if order_id == "INVALID" {
            return Err("Invalid order ID".into());
        }
    }
    
    // Simulate processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Simulate random failures (10% chance)
    if rand::random::<f64>() < 0.1 {
        return Err("Random processing failure".into());
    }
    
    Ok(())
}
```

### Advanced Error Handling with Dead Letter Queue

```rust
use kincir::rabbitmq::{RabbitMQAckSubscriber, RabbitMQPublisher, RabbitMQConfig};
use kincir::{AckSubscriber, Publisher, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure subscriber with dead letter queue
    let config = RabbitMQConfig {
        connection_url: "amqp://localhost:5672".to_string(),
        exchange_name: "orders".to_string(),
        queue_name: "orders-processing".to_string(),
        dead_letter_exchange: Some("orders-dlq".to_string()),
        max_retries: Some(3),
        retry_delay_ms: Some(5000),
    };
    
    let mut subscriber = RabbitMQAckSubscriber::with_config(config);
    let dlq_publisher = RabbitMQPublisher::new("amqp://localhost:5672");
    
    subscriber.subscribe("orders").await?;
    
    loop {
        let (message, ack_handle) = subscriber.receive_with_ack().await?;
        
        // Track retry count
        let retry_count = message.get_metadata("retry_count")
            .and_then(|c| c.parse::<u32>().ok())
            .unwrap_or(0);
        
        match process_order_with_retries(&message, retry_count).await {
            Ok(()) => {
                println!("Order processed after {} retries", retry_count);
                ack_handle.ack().await?;
            }
            Err(ProcessingError::Retryable(e)) if retry_count < 3 => {
                println!("Retryable error (attempt {}): {}", retry_count + 1, e);
                
                // Add retry metadata and requeue
                let mut retry_message = message.clone();
                retry_message.set_metadata("retry_count", &(retry_count + 1).to_string());
                retry_message.set_metadata("last_error", &e.to_string());
                
                // Reject and requeue for retry
                ack_handle.nack(true).await?;
            }
            Err(ProcessingError::Fatal(e)) | Err(ProcessingError::Retryable(e)) => {
                println!("Fatal error or max retries exceeded: {}", e);
                
                // Send to dead letter queue
                let mut dlq_message = message.clone();
                dlq_message.set_metadata("failure_reason", &e.to_string());
                dlq_message.set_metadata("failed_at", &chrono::Utc::now().to_rfc3339());
                dlq_message.set_metadata("retry_count", &retry_count.to_string());
                
                dlq_publisher.publish("orders-dlq", vec![dlq_message]).await?;
                
                // Acknowledge to remove from main queue
                ack_handle.ack().await?;
            }
        }
    }
}

#[derive(Debug)]
enum ProcessingError {
    Retryable(String),
    Fatal(String),
}

impl std::fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ProcessingError::Retryable(msg) => write!(f, "Retryable: {}", msg),
            ProcessingError::Fatal(msg) => write!(f, "Fatal: {}", msg),
        }
    }
}

impl std::error::Error for ProcessingError {}

async fn process_order_with_retries(message: &Message, retry_count: u32) -> Result<(), ProcessingError> {
    let order_data = String::from_utf8_lossy(&message.payload);
    
    // Simulate different types of errors
    if order_data.contains("NETWORK_ERROR") {
        return Err(ProcessingError::Retryable("Network timeout".to_string()));
    }
    
    if order_data.contains("INVALID_FORMAT") {
        return Err(ProcessingError::Fatal("Invalid order format".to_string()));
    }
    
    // Simulate transient failures that might succeed on retry
    if retry_count < 2 && rand::random::<f64>() < 0.3 {
        return Err(ProcessingError::Retryable("Temporary service unavailable".to_string()));
    }
    
    println!("Successfully processed order: {}", order_data);
    Ok(())
}
```

## Kafka Acknowledgments

### Manual Commit Pattern

```rust
use kincir::kafka::{KafkaAckSubscriber, KafkaConsumerConfig};
use kincir::{AckSubscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure Kafka consumer for manual acknowledgments
    let config = KafkaConsumerConfig {
        bootstrap_servers: "localhost:9092".to_string(),
        group_id: "manual-ack-group".to_string(),
        enable_auto_commit: false, // Disable auto-commit
        auto_offset_reset: "earliest".to_string(),
        session_timeout_ms: 30000,
        max_poll_records: 100,
    };
    
    let mut subscriber = KafkaAckSubscriber::with_config(config);
    subscriber.subscribe("events").await?;
    
    println!("Processing events with manual acknowledgments...");
    
    let mut batch_count = 0;
    let batch_size = 10;
    
    loop {
        let (message, ack_handle) = subscriber.receive_with_ack().await?;
        
        match process_event(&message).await {
            Ok(()) => {
                batch_count += 1;
                
                // Acknowledge individual message
                ack_handle.ack().await?;
                
                // Commit offsets in batches for better performance
                if batch_count % batch_size == 0 {
                    subscriber.commit_offsets().await?;
                    println!("Committed batch of {} messages", batch_size);
                }
            }
            Err(e) => {
                eprintln!("Failed to process event: {}", e);
                // Don't acknowledge failed messages
                // They will be reprocessed when the consumer restarts
            }
        }
    }
}

async fn process_event(message: &Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event_data = String::from_utf8_lossy(&message.payload);
    println!("Processing event: {}", event_data);
    
    // Simulate processing
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    
    // Simulate occasional failures
    if rand::random::<f64>() < 0.05 {
        return Err("Random processing failure".into());
    }
    
    Ok(())
}
```

### Transactional Processing

```rust
use kincir::kafka::{KafkaTransactionalProcessor, KafkaTransactionalConfig};
use kincir::{TransactionalProcessor, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = KafkaTransactionalConfig {
        bootstrap_servers: "localhost:9092".to_string(),
        consumer_group_id: "transactional-processor".to_string(),
        producer_transactional_id: "processor-1".to_string(),
        input_topics: vec!["raw-events".to_string()],
        output_topics: vec!["processed-events".to_string(), "metrics".to_string()],
    };
    
    let mut processor = KafkaTransactionalProcessor::with_config(config);
    
    println!("Starting transactional event processor...");
    
    loop {
        // Begin transaction
        let transaction = processor.begin_transaction().await?;
        
        match process_event_batch(&mut processor, &transaction).await {
            Ok(processed_count) => {
                // Commit transaction
                transaction.commit().await?;
                println!("Successfully processed {} events in transaction", processed_count);
            }
            Err(e) => {
                // Abort transaction
                transaction.abort().await?;
                eprintln!("Transaction aborted due to error: {}", e);
                
                // Wait before retrying
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

async fn process_event_batch(
    processor: &mut KafkaTransactionalProcessor,
    transaction: &KafkaTransaction,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut processed_count = 0;
    let batch_size = 100;
    
    for _ in 0..batch_size {
        if let Some((message, ack_handle)) = processor.try_receive_with_ack().await? {
            // Process the event
            let processed_event = transform_event(&message)?;
            let metric = generate_metric(&message)?;
            
            // Publish results within the transaction
            transaction.publish("processed-events", vec![processed_event]).await?;
            transaction.publish("metrics", vec![metric]).await?;
            
            // Acknowledge the input message
            ack_handle.ack().await?;
            
            processed_count += 1;
        } else {
            // No more messages in this batch
            break;
        }
    }
    
    Ok(processed_count)
}

fn transform_event(message: &Message) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
    let input_data = String::from_utf8_lossy(&message.payload);
    
    // Transform the event data
    let processed_data = format!("PROCESSED: {}", input_data);
    
    Ok(Message::new(processed_data.into_bytes())
        .with_metadata("processed_at", &chrono::Utc::now().to_rfc3339())
        .with_metadata("processor_id", "processor-1")
        .with_metadata("original_uuid", &message.uuid))
}

fn generate_metric(message: &Message) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
    let metric_data = serde_json::json!({
        "metric_name": "events_processed",
        "value": 1,
        "timestamp": chrono::Utc::now().timestamp(),
        "tags": {
            "processor": "processor-1",
            "source_topic": "raw-events"
        }
    });
    
    Ok(Message::new(metric_data.to_string().into_bytes())
        .with_metadata("metric_type", "counter"))
}
```

## MQTT Acknowledgments

### QoS-based Acknowledgments

```rust
use kincir::mqtt::{MQTTAckSubscriber, MQTTPublisher};
use kincir::{AckSubscriber, Publisher, Message};
use rumqttc::QoS;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut subscriber = MQTTAckSubscriber::new("mqtt://localhost:1883", "ack-subscriber");
    let publisher = MQTTPublisher::new("mqtt://localhost:1883", "ack-publisher");
    
    // Subscribe with QoS 2 (exactly once delivery)
    subscriber.subscribe_with_qos("commands/device/+", QoS::ExactlyOnce).await?;
    
    println!("Processing device commands with acknowledgments...");
    
    loop {
        let (message, ack_handle) = subscriber.receive_with_ack().await?;
        
        let device_id = extract_device_id(&message);
        let command = String::from_utf8_lossy(&message.payload);
        
        println!("Received command for device {}: {}", device_id, command);
        
        match execute_device_command(&device_id, &command).await {
            Ok(result) => {
                println!("Command executed successfully: {}", result);
                
                // Send success response
                let response = Message::new(format!("SUCCESS: {}", result).into_bytes())
                    .with_metadata("device_id", &device_id)
                    .with_metadata("command_id", &message.uuid)
                    .with_metadata("status", "success");
                
                let response_topic = format!("responses/device/{}", device_id);
                publisher.publish_with_qos(&response_topic, vec![response], QoS::AtLeastOnce).await?;
                
                // Acknowledge the command
                ack_handle.ack().await?;
            }
            Err(e) => {
                eprintln!("Command execution failed: {}", e);
                
                // Send error response
                let error_response = Message::new(format!("ERROR: {}", e).into_bytes())
                    .with_metadata("device_id", &device_id)
                    .with_metadata("command_id", &message.uuid)
                    .with_metadata("status", "error");
                
                let response_topic = format!("responses/device/{}", device_id);
                publisher.publish_with_qos(&response_topic, vec![error_response], QoS::AtLeastOnce).await?;
                
                // Decide whether to acknowledge or reject based on error type
                if is_retryable_error(&e) {
                    // Don't acknowledge - message will be redelivered
                    println!("Retryable error - message will be redelivered");
                } else {
                    // Acknowledge to prevent infinite retries
                    ack_handle.ack().await?;
                }
            }
        }
    }
}

fn extract_device_id(message: &Message) -> String {
    message.get_metadata("device_id")
        .unwrap_or("unknown")
        .to_string()
}

async fn execute_device_command(device_id: &str, command: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    println!("Executing command '{}' on device '{}'", command, device_id);
    
    // Simulate command execution
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    // Simulate different outcomes
    match command {
        "reboot" => Ok("Device rebooted successfully".to_string()),
        "status" => Ok("Device is online and healthy".to_string()),
        "update" => {
            if rand::random::<f64>() < 0.8 {
                Ok("Firmware updated successfully".to_string())
            } else {
                Err("Update failed: insufficient storage".into())
            }
        }
        "invalid" => Err("Unknown command".into()),
        _ => Err("Command execution timeout".into()),
    }
}

fn is_retryable_error(error: &Box<dyn std::error::Error + Send + Sync>) -> bool {
    let error_msg = error.to_string().to_lowercase();
    error_msg.contains("timeout") || error_msg.contains("network") || error_msg.contains("temporary")
}
```

## Cross-Backend Acknowledgment Patterns

### Reliable Message Bridge

```rust
use kincir::rabbitmq::{RabbitMQAckSubscriber, RabbitMQPublisher};
use kincir::kafka::{KafkaPublisher};
use kincir::{AckSubscriber, Publisher, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // RabbitMQ input with acknowledgments
    let mut rabbitmq_subscriber = RabbitMQAckSubscriber::new("amqp://localhost:5672", "bridge-input");
    
    // Kafka output
    let kafka_publisher = KafkaPublisher::new("localhost:9092");
    
    // RabbitMQ dead letter queue for failed messages
    let dlq_publisher = RabbitMQPublisher::new("amqp://localhost:5672");
    
    rabbitmq_subscriber.subscribe("events").await?;
    
    println!("Starting reliable message bridge: RabbitMQ -> Kafka");
    
    loop {
        let (message, ack_handle) = rabbitmq_subscriber.receive_with_ack().await?;
        
        // Transform message for Kafka
        let kafka_message = Message::new(message.payload.clone())
            .with_metadata("source", "rabbitmq")
            .with_metadata("bridge_timestamp", &chrono::Utc::now().to_rfc3339())
            .with_metadata("original_uuid", &message.uuid);
        
        // Copy original metadata
        for (key, value) in &message.metadata {
            kafka_message.set_metadata(key, value);
        }
        
        // Attempt to publish to Kafka
        match kafka_publisher.publish("bridged-events", vec![kafka_message]).await {
            Ok(()) => {
                println!("Successfully bridged message {}", message.uuid);
                // Acknowledge only after successful Kafka publish
                ack_handle.ack().await?;
            }
            Err(e) => {
                eprintln!("Failed to publish to Kafka: {}", e);
                
                // Check if this is a retryable error
                if is_kafka_retryable_error(&e) {
                    println!("Retryable Kafka error - rejecting message for retry");
                    ack_handle.nack(true).await?; // Requeue for retry
                } else {
                    println!("Fatal Kafka error - sending to dead letter queue");
                    
                    // Send to dead letter queue
                    let dlq_message = message.clone()
                        .with_metadata("failure_reason", &e.to_string())
                        .with_metadata("failed_at", &chrono::Utc::now().to_rfc3339());
                    
                    dlq_publisher.publish("bridge-dlq", vec![dlq_message]).await?;
                    
                    // Acknowledge to remove from main queue
                    ack_handle.ack().await?;
                }
            }
        }
    }
}

fn is_kafka_retryable_error(error: &kincir::KincirError) -> bool {
    // Implement logic to determine if Kafka error is retryable
    match error {
        kincir::KincirError::ConnectionError(_) => true,
        kincir::KincirError::TimeoutError => true,
        kincir::KincirError::BrokerError(msg) if msg.contains("retriable") => true,
        _ => false,
    }
}
```

## Testing Acknowledgment Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kincir::memory::{InMemoryBroker, InMemoryAckSubscriber, InMemoryPublisher};
    use kincir::{AckSubscriber, Publisher, Message};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_successful_acknowledgment() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemoryAckSubscriber::new(broker.clone());
        
        subscriber.subscribe("test").await.unwrap();
        
        let message = Message::new(b"test message".to_vec());
        publisher.publish("test", vec![message]).await.unwrap();
        
        let (received, ack_handle) = subscriber.receive_with_ack().await.unwrap();
        assert_eq!(received.payload, b"test message");
        
        // Acknowledge the message
        ack_handle.ack().await.unwrap();
        
        // Verify message is removed from queue
        assert!(subscriber.try_receive().await.is_err());
    }
    
    #[tokio::test]
    async fn test_message_redelivery_on_nack() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemoryAckSubscriber::new(broker.clone());
        
        subscriber.subscribe("test").await.unwrap();
        
        let message = Message::new(b"test message".to_vec());
        publisher.publish("test", vec![message]).await.unwrap();
        
        // Receive and reject message
        let (received, ack_handle) = subscriber.receive_with_ack().await.unwrap();
        ack_handle.nack(true).await.unwrap(); // Requeue
        
        // Message should be available again
        let (redelivered, ack_handle2) = subscriber.receive_with_ack().await.unwrap();
        assert_eq!(redelivered.payload, received.payload);
        
        // Acknowledge the redelivered message
        ack_handle2.ack().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_batch_acknowledgment() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemoryAckSubscriber::new(broker.clone());
        
        subscriber.subscribe("test").await.unwrap();
        
        // Publish multiple messages
        let messages: Vec<Message> = (0..5)
            .map(|i| Message::new(format!("message {}", i).into_bytes()))
            .collect();
        
        publisher.publish("test", messages).await.unwrap();
        
        // Process messages in batch
        let mut ack_handles = Vec::new();
        for _ in 0..5 {
            let (_, ack_handle) = subscriber.receive_with_ack().await.unwrap();
            ack_handles.push(ack_handle);
        }
        
        // Acknowledge all messages in batch
        for ack_handle in ack_handles {
            ack_handle.ack().await.unwrap();
        }
        
        // Verify all messages are processed
        assert!(subscriber.try_receive().await.is_err());
    }
}
```

## Best Practices

### 1. Choose Appropriate Acknowledgment Strategy
- **Auto-acknowledge**: For non-critical messages where loss is acceptable
- **Manual acknowledge**: For critical messages requiring guaranteed processing
- **Batch acknowledge**: For high-throughput scenarios

### 2. Handle Different Error Types
- **Transient errors**: Reject and requeue for retry
- **Permanent errors**: Acknowledge and send to dead letter queue
- **Poison messages**: Implement maximum retry limits

### 3. Monitor Acknowledgment Metrics
- Track acknowledgment rates
- Monitor redelivery counts
- Alert on high rejection rates

### 4. Implement Circuit Breakers
- Prevent cascading failures
- Implement backoff strategies
- Monitor downstream service health

## Next Steps

- [Error Handling](/examples/error-handling.html) - Comprehensive error strategies
- [Performance Optimization](/examples/performance.html) - High-throughput patterns
- [Monitoring](/examples/monitoring.html) - Observability and metrics
- [Testing](/examples/unit-testing.html) - Testing acknowledgment patterns

## Resources

- [RabbitMQ Acknowledgments](https://www.rabbitmq.com/confirms.html)
- [Kafka Consumer Acknowledgments](https://kafka.apache.org/documentation/#consumerconfigs)
- [MQTT QoS Levels](https://mqtt.org/mqtt-specification/)
- [Kincir API Documentation](https://docs.rs/kincir)
