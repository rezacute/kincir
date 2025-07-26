# Error Handling in Kincir

This guide demonstrates comprehensive error handling strategies for production Kincir applications, including retry mechanisms, dead letter queues, and graceful degradation patterns.

## Table of Contents

- [Basic Error Handling](#basic-error-handling)
- [Retry Mechanisms](#retry-mechanisms)
- [Dead Letter Queues](#dead-letter-queues)
- [Circuit Breaker Pattern](#circuit-breaker-pattern)
- [Graceful Degradation](#graceful-degradation)
- [Error Monitoring](#error-monitoring)

## Basic Error Handling

### Publisher Error Handling

```rust
use kincir::{Publisher, Message};
use kincir::memory::InMemoryPublisher;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker);
    
    let message = Message::new(b"Important data".to_vec());
    
    // Basic error handling with retry
    let mut attempts = 0;
    let max_attempts = 3;
    
    loop {
        match publisher.publish("orders", vec![message.clone()]).await {
            Ok(_) => {
                println!("Message published successfully");
                break;
            }
            Err(e) => {
                attempts += 1;
                if attempts >= max_attempts {
                    eprintln!("Failed to publish after {} attempts: {}", max_attempts, e);
                    return Err(e.into());
                }
                
                println!("Publish attempt {} failed: {}. Retrying...", attempts, e);
                sleep(Duration::from_millis(100 * attempts as u64)).await;
            }
        }
    }
    
    Ok(())
}
```

### Subscriber Error Handling

```rust
use kincir::{Subscriber, AckSubscriber, Message};
use kincir::memory::{InMemorySubscriber, InMemoryAckSubscriber};
use std::sync::Arc;

async fn handle_message_with_error_recovery(
    message: Message
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Simulate message processing that might fail
    if message.payload.len() < 5 {
        return Err("Message too short".into());
    }
    
    // Process the message
    println!("Processing message: {:?}", String::from_utf8_lossy(&message.payload));
    
    // Simulate potential processing error
    if message.payload.starts_with(b"ERROR") {
        return Err("Processing failed for error message".into());
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let mut subscriber = InMemoryAckSubscriber::new(broker);
    
    subscriber.subscribe("orders").await?;
    
    loop {
        match subscriber.receive_with_ack().await {
            Ok((message, ack_handle)) => {
                match handle_message_with_error_recovery(message).await {
                    Ok(_) => {
                        // Acknowledge successful processing
                        if let Err(e) = ack_handle.ack().await {
                            eprintln!("Failed to acknowledge message: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Message processing failed: {}", e);
                        // Reject and requeue for retry
                        if let Err(ack_err) = ack_handle.nack(true).await {
                            eprintln!("Failed to nack message: {}", ack_err);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to receive message: {}", e);
                // Implement backoff strategy
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }
}
```

## Retry Mechanisms

### Exponential Backoff Retry

```rust
use tokio::time::{sleep, Duration};
use std::cmp::min;

pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    config: RetryConfig,
) -> Result<T, E>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    let mut delay = config.initial_delay;
    
    loop {
        attempts += 1;
        
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempts >= config.max_attempts {
                    return Err(e);
                }
                
                println!("Attempt {} failed: {}. Retrying in {:?}...", attempts, e, delay);
                sleep(delay).await;
                
                // Calculate next delay with exponential backoff
                delay = min(
                    Duration::from_millis(
                        (delay.as_millis() as f64 * config.backoff_multiplier) as u64
                    ),
                    config.max_delay,
                );
            }
        }
    }
}

// Usage example
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let publisher = InMemoryPublisher::new(broker);
    let message = Message::new(b"Retry example".to_vec());
    
    let result = retry_with_backoff(
        || {
            let publisher = publisher.clone();
            let message = message.clone();
            Box::pin(async move {
                publisher.publish("orders", vec![message]).await
            })
        },
        RetryConfig::default(),
    ).await?;
    
    println!("Message published successfully after retries");
    Ok(())
}
```

## Dead Letter Queues

### Dead Letter Queue Implementation

```rust
use kincir::{Message, Publisher, AckSubscriber};
use kincir::memory::{InMemoryPublisher, InMemoryAckSubscriber};
use std::sync::Arc;
use std::collections::HashMap;

pub struct DeadLetterHandler {
    dlq_publisher: InMemoryPublisher,
    max_retries: u32,
}

impl DeadLetterHandler {
    pub fn new(broker: Arc<kincir::memory::InMemoryBroker>, max_retries: u32) -> Self {
        Self {
            dlq_publisher: InMemoryPublisher::new(broker),
            max_retries,
        }
    }
    
    pub async fn handle_failed_message(
        &self,
        mut message: Message,
        error: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Get current retry count
        let retry_count: u32 = message
            .metadata
            .get("retry_count")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        if retry_count >= self.max_retries {
            // Send to dead letter queue
            message.set_metadata("dlq_reason", error);
            message.set_metadata("final_retry_count", &retry_count.to_string());
            message.set_metadata("dlq_timestamp", &chrono::Utc::now().to_rfc3339());
            
            self.dlq_publisher
                .publish("dead_letter_queue", vec![message])
                .await?;
            
            println!("Message sent to dead letter queue after {} retries", retry_count);
            return Ok(false); // Don't retry
        }
        
        // Increment retry count and allow retry
        message.set_metadata("retry_count", &(retry_count + 1).to_string());
        message.set_metadata("last_error", error);
        
        println!("Message will be retried (attempt {})", retry_count + 1);
        Ok(true) // Allow retry
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());
    let dlq_handler = DeadLetterHandler::new(broker.clone(), 3);
    
    subscriber.subscribe("orders").await?;
    
    loop {
        match subscriber.receive_with_ack().await {
            Ok((message, ack_handle)) => {
                // Simulate processing that might fail
                let processing_result = process_message(&message).await;
                
                match processing_result {
                    Ok(_) => {
                        ack_handle.ack().await?;
                        println!("Message processed successfully");
                    }
                    Err(e) => {
                        let should_retry = dlq_handler
                            .handle_failed_message(message, &e.to_string())
                            .await?;
                        
                        if should_retry {
                            // Nack with requeue for retry
                            ack_handle.nack(true).await?;
                        } else {
                            // Ack to remove from main queue (already in DLQ)
                            ack_handle.ack().await?;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to receive message: {}", e);
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }
}

async fn process_message(message: &Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Simulate processing logic that might fail
    if message.payload.starts_with(b"FAIL") {
        return Err("Simulated processing failure".into());
    }
    
    println!("Processing: {:?}", String::from_utf8_lossy(&message.payload));
    Ok(())
}
```

## Circuit Breaker Pattern

```rust
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<Mutex<u32>>,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            last_failure_time: Arc::new(Mutex::new(None)),
            failure_threshold,
            recovery_timeout,
        }
    }
    
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        // Check if circuit should transition from Open to HalfOpen
        self.check_recovery_timeout();
        
        let current_state = {
            let state = self.state.lock().unwrap();
            state.clone()
        };
        
        match current_state {
            CircuitState::Open => {
                return Err(CircuitBreakerError::CircuitOpen);
            }
            CircuitState::HalfOpen | CircuitState::Closed => {
                match operation.await {
                    Ok(result) => {
                        self.on_success();
                        Ok(result)
                    }
                    Err(e) => {
                        self.on_failure();
                        Err(CircuitBreakerError::OperationFailed(e))
                    }
                }
            }
        }
    }
    
    fn check_recovery_timeout(&self) {
        let mut state = self.state.lock().unwrap();
        if let CircuitState::Open = *state {
            if let Some(last_failure) = *self.last_failure_time.lock().unwrap() {
                if last_failure.elapsed() >= self.recovery_timeout {
                    *state = CircuitState::HalfOpen;
                    println!("Circuit breaker transitioning to HalfOpen");
                }
            }
        }
    }
    
    fn on_success(&self) {
        let mut state = self.state.lock().unwrap();
        let mut failure_count = self.failure_count.lock().unwrap();
        
        *failure_count = 0;
        *state = CircuitState::Closed;
    }
    
    fn on_failure(&self) {
        let mut state = self.state.lock().unwrap();
        let mut failure_count = self.failure_count.lock().unwrap();
        let mut last_failure_time = self.last_failure_time.lock().unwrap();
        
        *failure_count += 1;
        *last_failure_time = Some(Instant::now());
        
        if *failure_count >= self.failure_threshold {
            *state = CircuitState::Open;
            println!("Circuit breaker opened after {} failures", *failure_count);
        }
    }
}

#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    CircuitOpen,
    OperationFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::OperationFailed(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CircuitBreakerError::OperationFailed(e) => Some(e),
            _ => None,
        }
    }
}
```

## Graceful Degradation

```rust
use kincir::{Publisher, Message};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

pub struct ResilientPublisher {
    primary_publisher: Arc<dyn Publisher<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
    fallback_publisher: Option<Arc<dyn Publisher<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>>,
    circuit_breaker: CircuitBreaker,
}

impl ResilientPublisher {
    pub fn new(
        primary: Arc<dyn Publisher<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
        fallback: Option<Arc<dyn Publisher<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>>,
    ) -> Self {
        Self {
            primary_publisher: primary,
            fallback_publisher: fallback,
            circuit_breaker: CircuitBreaker::new(5, Duration::from_secs(30)),
        }
    }
    
    pub async fn publish_with_fallback(
        &self,
        topic: &str,
        messages: Vec<Message>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Try primary publisher with timeout and circuit breaker
        let primary_result = timeout(
            Duration::from_secs(5),
            self.circuit_breaker.call(
                self.primary_publisher.publish(topic, messages.clone())
            )
        ).await;
        
        match primary_result {
            Ok(Ok(Ok(_))) => {
                println!("Published via primary publisher");
                return Ok(());
            }
            Ok(Ok(Err(CircuitBreakerError::CircuitOpen))) => {
                println!("Primary publisher circuit is open, trying fallback");
            }
            Ok(Ok(Err(CircuitBreakerError::OperationFailed(e)))) => {
                println!("Primary publisher failed: {}, trying fallback", e);
            }
            Ok(Err(_)) => {
                println!("Primary publisher operation failed, trying fallback");
            }
            Err(_) => {
                println!("Primary publisher timed out, trying fallback");
            }
        }
        
        // Try fallback publisher if available
        if let Some(fallback) = &self.fallback_publisher {
            match timeout(
                Duration::from_secs(5),
                fallback.publish(topic, messages)
            ).await {
                Ok(Ok(_)) => {
                    println!("Published via fallback publisher");
                    return Ok(());
                }
                Ok(Err(e)) => {
                    println!("Fallback publisher failed: {}", e);
                }
                Err(_) => {
                    println!("Fallback publisher timed out");
                }
            }
        }
        
        Err("All publishers failed".into())
    }
}
```

## Error Monitoring

```rust
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use serde_json::json;

pub struct ErrorMetrics {
    error_counts: Arc<RwLock<HashMap<String, u64>>>,
    last_errors: Arc<RwLock<HashMap<String, String>>>,
}

impl ErrorMetrics {
    pub fn new() -> Self {
        Self {
            error_counts: Arc::new(RwLock::new(HashMap::new())),
            last_errors: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn record_error(&self, error_type: &str, error_message: &str) {
        let mut counts = self.error_counts.write().await;
        *counts.entry(error_type.to_string()).or_insert(0) += 1;
        
        let mut last_errors = self.last_errors.write().await;
        last_errors.insert(error_type.to_string(), error_message.to_string());
        
        // Log error for monitoring systems
        println!("ERROR [{}]: {}", error_type, error_message);
    }
    
    pub async fn get_error_summary(&self) -> serde_json::Value {
        let counts = self.error_counts.read().await;
        let last_errors = self.last_errors.read().await;
        
        json!({
            "error_counts": *counts,
            "last_errors": *last_errors,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }
}

// Usage in application
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let metrics = Arc::new(ErrorMetrics::new());
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let mut subscriber = kincir::memory::InMemoryAckSubscriber::new(broker);
    
    subscriber.subscribe("orders").await?;
    
    loop {
        match subscriber.receive_with_ack().await {
            Ok((message, ack_handle)) => {
                match process_message(&message).await {
                    Ok(_) => {
                        ack_handle.ack().await?;
                    }
                    Err(e) => {
                        metrics.record_error("processing_error", &e.to_string()).await;
                        ack_handle.nack(true).await?;
                    }
                }
            }
            Err(e) => {
                metrics.record_error("receive_error", &e.to_string()).await;
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }
}
```

## Best Practices

1. **Always handle errors explicitly** - Don't ignore potential failure points
2. **Implement proper retry logic** - Use exponential backoff to avoid overwhelming systems
3. **Use dead letter queues** - Prevent poison messages from blocking processing
4. **Monitor error rates** - Track and alert on error patterns
5. **Implement circuit breakers** - Protect against cascading failures
6. **Use timeouts** - Prevent operations from hanging indefinitely
7. **Log errors appropriately** - Include context for debugging
8. **Test error scenarios** - Include error cases in your test suite

## Running the Examples

To run these error handling examples:

```bash
# Clone the repository
git clone https://github.com/rezacute/kincir.git
cd kincir

# Add dependencies to Cargo.toml
[dependencies]
kincir = "0.2.0"
tokio = { version = "1.0", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
serde_json = "1.0"

# Run the examples
cargo run --example error-handling
```

This comprehensive error handling guide provides the foundation for building robust, production-ready applications with Kincir.
