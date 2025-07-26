# Router Acknowledgment Implementation

This document provides comprehensive documentation for the acknowledgment-aware router implementation in Kincir, which enables reliable message processing with automatic acknowledgment handling, error recovery, and comprehensive monitoring.

## Overview

The acknowledgment-aware router (`AckRouter`) provides:

- **Automatic Acknowledgment Handling**: Configurable strategies for message acknowledgment based on processing results
- **Error Recovery**: Retry logic with configurable maximum attempts and requeue behavior
- **Processing Timeouts**: Configurable timeouts to prevent hanging message processing
- **Comprehensive Statistics**: Detailed metrics for monitoring and debugging
- **Flexible Configuration**: Multiple acknowledgment strategies and processing options
- **Integration**: Seamless integration with any `AckSubscriber` implementation

## Core Components

### AckRouter

The `AckRouter` is the main component that processes messages from an acknowledgment-capable subscriber, applies message handlers, publishes results, and manages acknowledgments based on configurable strategies.

```rust
use kincir::router::{AckRouter, AckStrategy, RouterAckConfig};
use kincir::memory::{InMemoryAckSubscriber, InMemoryBroker, InMemoryPublisher};
use std::sync::Arc;
use tokio::sync::Mutex;

// Create components
let broker = Arc::new(InMemoryBroker::with_default_config());
let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));

// Create message handler
let handler = Arc::new(|msg: Message| {
    Box::pin(async move {
        // Process message
        let processed = msg.with_metadata("processed", "true");
        Ok(vec![processed])
    })
});

// Create router
#[cfg(feature = "logging")]
let router = {
    use kincir::logging::StdLogger;
    let logger = Arc::new(StdLogger::new(true, false));
    AckRouter::with_default_config(
        logger,
        "input".to_string(),
        "output".to_string(),
        subscriber,
        publisher,
        handler,
    )
};

#[cfg(not(feature = "logging"))]
let router = AckRouter::with_default_config(
    "input".to_string(),
    "output".to_string(),
    subscriber,
    publisher,
    handler,
);
```

### RouterAckConfig

Configuration for acknowledgment behavior and processing options:

```rust
use kincir::router::{RouterAckConfig, AckStrategy};
use std::time::Duration;

let config = RouterAckConfig {
    strategy: AckStrategy::AutoAckOnSuccess,
    processing_timeout: Some(Duration::from_secs(30)),
    max_retries: 3,
    requeue_on_failure: true,
    batch_size: None,
};
```

#### Configuration Options

- **strategy**: Acknowledgment strategy (see [Acknowledgment Strategies](#acknowledgment-strategies))
- **processing_timeout**: Maximum time allowed for message processing
- **max_retries**: Maximum number of retry attempts for failed messages
- **requeue_on_failure**: Whether to requeue messages on processing failure
- **batch_size**: Optional batch size for batch processing mode

### RouterAckStats

Comprehensive statistics for monitoring router performance:

```rust
let stats = router.stats().await;
println!("Messages Processed: {}", stats.messages_processed);
println!("Messages Acknowledged: {}", stats.messages_acked);
println!("Messages Nacked: {}", stats.messages_nacked);
println!("Acknowledgment Rate: {:.1}%", stats.ack_rate());
println!("Average Processing Time: {:.2}ms", stats.avg_processing_time_ms);
```

#### Available Statistics

- **messages_processed**: Total number of messages processed
- **messages_acked**: Number of messages successfully acknowledged
- **messages_nacked**: Number of messages negatively acknowledged
- **messages_timed_out**: Number of messages that exceeded processing timeout
- **messages_max_retries_exceeded**: Number of messages that exceeded maximum retries
- **avg_processing_time_ms**: Average message processing time in milliseconds
- **ack_rate()**: Acknowledgment rate as a percentage
- **nack_rate()**: Negative acknowledgment rate as a percentage

## Acknowledgment Strategies

The router supports multiple acknowledgment strategies to handle different use cases:

### AutoAckOnSuccess (Default)

Automatically acknowledges messages on successful processing and negatively acknowledges on failure:

```rust
let config = RouterAckConfig {
    strategy: AckStrategy::AutoAckOnSuccess,
    ..Default::default()
};

// Behavior:
// - Success: Message is acknowledged
// - Failure: Message is negatively acknowledged (with requeue based on config)
```

### AlwaysAck

Always acknowledges messages regardless of processing result:

```rust
let config = RouterAckConfig {
    strategy: AckStrategy::AlwaysAck,
    ..Default::default()
};

// Behavior:
// - Success: Message is acknowledged
// - Failure: Message is still acknowledged (no redelivery)
```

### NeverAck

Never performs acknowledgment operations (for testing or special cases):

```rust
let config = RouterAckConfig {
    strategy: AckStrategy::NeverAck,
    ..Default::default()
};

// Behavior:
// - Success: No acknowledgment
// - Failure: No acknowledgment
// Note: Messages may be redelivered based on subscriber behavior
```

### Manual

Delegates acknowledgment handling to the message handler:

```rust
let config = RouterAckConfig {
    strategy: AckStrategy::Manual,
    ..Default::default()
};

// Handler must handle acknowledgments manually
let handler = Arc::new(|msg: Message| {
    Box::pin(async move {
        // Process message
        // Handler should call subscriber.ack() or subscriber.nack() as needed
        Ok(vec![processed_msg])
    })
});
```

## Basic Usage

### 1. Simple Message Processing

```rust
use kincir::router::{AckRouter, RouterAckConfig};
use kincir::memory::{InMemoryAckSubscriber, InMemoryBroker, InMemoryPublisher};
use kincir::{Message, Publisher};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));

    // Message handler
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            // Transform message
            let processed = Message::new(
                format!("PROCESSED: {}", String::from_utf8_lossy(&msg.payload)).into_bytes()
            ).with_metadata("processed", "true");
            
            Ok(vec![processed])
        })
    });

    // Create router
    let router = AckRouter::with_default_config(
        "input".to_string(),
        "output".to_string(),
        subscriber,
        output_publisher,
        handler,
    );

    // Publish test message
    let test_message = Message::new(b"Hello World!".to_vec());
    input_publisher.publish("input", vec![test_message]).await?;

    // Process single message
    router.process_single_message().await?;

    // Check statistics
    let stats = router.stats().await;
    println!("Processed: {}, Acked: {}", stats.messages_processed, stats.messages_acked);

    Ok(())
}
```

### 2. Error Handling and Retry Logic

```rust
// Handler that may fail
let handler = Arc::new(|msg: Message| {
    Box::pin(async move {
        let payload = String::from_utf8_lossy(&msg.payload);
        
        if payload.contains("error") {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Processing failed"
            )) as Box<dyn std::error::Error + Send + Sync>)
        } else {
            Ok(vec![msg.with_metadata("processed", "true")])
        }
    })
});

// Configure retry behavior
let config = RouterAckConfig {
    strategy: AckStrategy::AutoAckOnSuccess,
    max_retries: 3,
    requeue_on_failure: true,
    ..Default::default()
};

let router = AckRouter::new(
    "input".to_string(),
    "output".to_string(),
    subscriber,
    publisher,
    handler,
    config,
);
```

### 3. Processing Timeouts

```rust
// Handler with potential long processing
let handler = Arc::new(|msg: Message| {
    Box::pin(async move {
        // Simulate long processing
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok(vec![msg])
    })
});

// Configure timeout
let config = RouterAckConfig {
    processing_timeout: Some(Duration::from_secs(2)), // Shorter than handler time
    ..Default::default()
};

let router = AckRouter::new(
    "input".to_string(),
    "output".to_string(),
    subscriber,
    publisher,
    handler,
    config,
);

## Advanced Features

### Batch Processing

Process multiple messages in batches for improved throughput:

```rust
let config = RouterAckConfig {
    batch_size: Some(10), // Process 10 messages at a time
    ..Default::default()
};

let router = AckRouter::new(
    "input".to_string(),
    "output".to_string(),
    subscriber,
    publisher,
    handler,
    config,
);

// Use batch processing mode
router.run_with_batching().await?;
```

### Statistics Monitoring

Monitor router performance with comprehensive statistics:

```rust
// Get current statistics
let stats = router.stats().await;

println!("📊 Router Performance:");
println!("   Total Messages: {}", stats.messages_processed);
println!("   Success Rate: {:.1}%", stats.ack_rate());
println!("   Error Rate: {:.1}%", stats.nack_rate());
println!("   Timeout Rate: {:.1}%", 
    (stats.messages_timed_out as f64 / stats.messages_processed as f64) * 100.0);
println!("   Avg Processing Time: {:.2}ms", stats.avg_processing_time_ms);

// Reset statistics for new measurement period
router.reset_stats().await;
```

### Custom Configuration

Create routers with specific configurations for different use cases:

```rust
// High-reliability configuration
let reliable_config = RouterAckConfig {
    strategy: AckStrategy::AutoAckOnSuccess,
    processing_timeout: Some(Duration::from_secs(60)),
    max_retries: 5,
    requeue_on_failure: true,
    batch_size: None,
};

// High-throughput configuration
let throughput_config = RouterAckConfig {
    strategy: AckStrategy::AlwaysAck, // Don't retry failures
    processing_timeout: Some(Duration::from_secs(5)),
    max_retries: 0,
    requeue_on_failure: false,
    batch_size: Some(50),
};

// Testing configuration
let test_config = RouterAckConfig {
    strategy: AckStrategy::NeverAck,
    processing_timeout: Some(Duration::from_secs(1)),
    max_retries: 0,
    requeue_on_failure: false,
    batch_size: None,
};
```

## Integration with Different Backends

### In-Memory Backend

```rust
use kincir::memory::{InMemoryAckSubscriber, InMemoryBroker, InMemoryPublisher};

let broker = Arc::new(InMemoryBroker::with_default_config());
let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));

let router = AckRouter::with_default_config(
    "input".to_string(),
    "output".to_string(),
    subscriber,
    publisher,
    handler,
);
```

### RabbitMQ Backend

```rust
use kincir::rabbitmq::{RabbitMQAckSubscriber, RabbitMQPublisher};

let subscriber = Arc::new(Mutex::new(
    RabbitMQAckSubscriber::new("amqp://localhost:5672", "consumer-group").await?
));
let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);

let router = AckRouter::with_default_config(
    "input-queue".to_string(),
    "output-queue".to_string(),
    subscriber,
    publisher,
    handler,
);
```

### Kafka Backend

```rust
use kincir::kafka::{KafkaAckSubscriber, KafkaPublisher};

let subscriber = Arc::new(Mutex::new(
    KafkaAckSubscriber::new("localhost:9092", "consumer-group").await?
));
let publisher = Arc::new(KafkaPublisher::new("localhost:9092").await?);

let router = AckRouter::with_default_config(
    "input-topic".to_string(),
    "output-topic".to_string(),
    subscriber,
    publisher,
    handler,
);
```

## Error Handling Patterns

### Retry with Exponential Backoff

```rust
use std::sync::atomic::{AtomicU32, Ordering};

let retry_count = Arc::new(AtomicU32::new(0));
let retry_count_clone = retry_count.clone();

let handler = Arc::new(move |msg: Message| {
    let retry_count = retry_count_clone.clone();
    Box::pin(async move {
        let attempt = retry_count.fetch_add(1, Ordering::SeqCst);
        
        // Simulate transient failures
        if attempt < 2 {
            tokio::time::sleep(Duration::from_millis(100 * (1 << attempt))).await;
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Transient failure (attempt {})", attempt + 1)
            )) as Box<dyn std::error::Error + Send + Sync>)
        } else {
            retry_count.store(0, Ordering::SeqCst);
            Ok(vec![msg.with_metadata("processed_after_retries", "true")])
        }
    })
});
```

### Dead Letter Queue Pattern

```rust
let handler = Arc::new(|msg: Message| {
    Box::pin(async move {
        // Try to process message
        match process_message(&msg).await {
            Ok(result) => Ok(vec![result]),
            Err(e) if is_retryable_error(&e) => {
                // Let router handle retry
                Err(e)
            }
            Err(e) => {
                // Send to dead letter queue
                let dead_letter_msg = msg
                    .with_metadata("error", &e.to_string())
                    .with_metadata("dead_letter", "true")
                    .with_metadata("failed_at", &chrono::Utc::now().to_rfc3339());
                
                // Don't return error - message will be acked
                Ok(vec![dead_letter_msg])
            }
        }
    })
});

async fn process_message(msg: &Message) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
    // Your processing logic here
    Ok(msg.clone())
}

fn is_retryable_error(error: &dyn std::error::Error) -> bool {
    // Determine if error is worth retrying
    error.to_string().contains("timeout") || error.to_string().contains("connection")
}
```

### Circuit Breaker Pattern

```rust
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::time::{Duration, Instant};

struct CircuitBreaker {
    failure_count: AtomicU32,
    is_open: AtomicBool,
    last_failure: Arc<Mutex<Option<Instant>>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

impl CircuitBreaker {
    fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            is_open: AtomicBool::new(false),
            last_failure: Arc::new(Mutex::new(None)),
            failure_threshold,
            recovery_timeout,
        }
    }

    async fn call<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        // Check if circuit is open
        if self.is_open.load(Ordering::SeqCst) {
            let last_failure = self.last_failure.lock().await;
            if let Some(last) = *last_failure {
                if last.elapsed() > self.recovery_timeout {
                    // Try to close circuit
                    self.is_open.store(false, Ordering::SeqCst);
                    self.failure_count.store(0, Ordering::SeqCst);
                } else {
                    return Err(/* circuit open error */);
                }
            }
        }

        match f() {
            Ok(result) => {
                // Reset on success
                self.failure_count.store(0, Ordering::SeqCst);
                Ok(result)
            }
            Err(e) => {
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if failures >= self.failure_threshold {
                    self.is_open.store(true, Ordering::SeqCst);
                    *self.last_failure.lock().await = Some(Instant::now());
                }
                Err(e)
            }
        }
    }
}
```

## Performance Optimization

### Tuning Configuration

```rust
// For high-throughput scenarios
let high_throughput_config = RouterAckConfig {
    strategy: AckStrategy::AutoAckOnSuccess,
    processing_timeout: Some(Duration::from_secs(5)), // Shorter timeout
    max_retries: 1, // Fewer retries
    requeue_on_failure: false, // Don't requeue to avoid blocking
    batch_size: Some(100), // Large batch size
};

// For high-reliability scenarios
let high_reliability_config = RouterAckConfig {
    strategy: AckStrategy::AutoAckOnSuccess,
    processing_timeout: Some(Duration::from_secs(60)), // Longer timeout
    max_retries: 5, // More retries
    requeue_on_failure: true, // Requeue for retry
    batch_size: Some(10), // Smaller batch size for better error isolation
};
```

### Monitoring and Alerting

```rust
async fn monitor_router_health(router: &AckRouter<impl AckSubscriber, impl AckHandle>) {
    let stats = router.stats().await;
    
    // Alert on high error rate
    if stats.nack_rate() > 10.0 {
        eprintln!("⚠️  High error rate: {:.1}%", stats.nack_rate());
    }
    
    // Alert on high timeout rate
    let timeout_rate = if stats.messages_processed > 0 {
        (stats.messages_timed_out as f64 / stats.messages_processed as f64) * 100.0
    } else {
        0.0
    };
    
    if timeout_rate > 5.0 {
        eprintln!("⚠️  High timeout rate: {:.1}%", timeout_rate);
    }
    
    // Alert on slow processing
    if stats.avg_processing_time_ms > 1000.0 {
        eprintln!("⚠️  Slow processing: {:.2}ms average", stats.avg_processing_time_ms);
    }
}
```

## Testing

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_successful_processing() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
        
        let handler = Arc::new(|msg: Message| {
            Box::pin(async move {
                Ok(vec![msg.with_metadata("processed", "true")])
            })
        });

        let router = AckRouter::with_default_config(
            "input".to_string(),
            "output".to_string(),
            subscriber,
            publisher,
            handler,
        );

        // Publish test message
        let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let test_message = Message::new(b"test".to_vec());
        input_publisher.publish("input", vec![test_message]).await.unwrap();

        // Process message
        let result = timeout(Duration::from_secs(5), router.process_single_message()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());

        // Check statistics
        let stats = router.stats().await;
        assert_eq!(stats.messages_processed, 1);
        assert_eq!(stats.messages_acked, 1);
        assert_eq!(stats.ack_rate(), 100.0);
    }

    #[tokio::test]
    async fn test_processing_failure() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
        
        let handler = Arc::new(|_msg: Message| {
            Box::pin(async move {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Test failure"
                )) as Box<dyn std::error::Error + Send + Sync>)
            })
        });

        let config = RouterAckConfig {
            max_retries: 1,
            requeue_on_failure: false,
            ..Default::default()
        };

        let router = AckRouter::new(
            "input".to_string(),
            "output".to_string(),
            subscriber,
            publisher,
            handler,
            config,
        );

        // Publish test message
        let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let test_message = Message::new(b"test".to_vec());
        input_publisher.publish("input", vec![test_message]).await.unwrap();

        // Process message (should fail)
        let result = timeout(Duration::from_secs(5), router.process_single_message()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok()); // Router handles failures gracefully

        // Check statistics
        let stats = router.stats().await;
        assert_eq!(stats.messages_processed, 1);
        assert_eq!(stats.messages_nacked, 1);
        assert_eq!(stats.nack_rate(), 100.0);
    }
}
```

## Examples

See the complete working example in `examples/router_ack_example.rs`:

```bash
cargo run --example router_ack_example
```

This example demonstrates:
- Basic acknowledgment-aware routing
- Error handling and retry logic
- Different acknowledgment strategies
- Processing timeout handling
- Comprehensive statistics and monitoring

## Conclusion

The acknowledgment-aware router provides a robust foundation for reliable message processing with comprehensive error handling, monitoring, and flexible configuration options. It integrates seamlessly with any acknowledgment-capable subscriber and provides production-ready features for enterprise messaging workflows.

Key benefits:
- **Reliability**: Automatic acknowledgment handling with configurable retry logic
- **Flexibility**: Multiple acknowledgment strategies for different use cases
- **Monitoring**: Comprehensive statistics for performance tracking and alerting
- **Performance**: Optimized processing with batch support and timeout handling
- **Integration**: Works with any AckSubscriber implementation across different backends
