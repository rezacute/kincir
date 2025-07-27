---
layout: docs
title: RabbitMQ Integration
description: Enterprise-grade message queuing with RabbitMQ and Kincir
---

# RabbitMQ Integration Example

RabbitMQ is a robust, enterprise-grade message broker that provides reliable message queuing with advanced routing capabilities. Kincir provides seamless integration with RabbitMQ through its unified interface.

## Prerequisites

Before running these examples, ensure you have RabbitMQ installed and running:

```bash
# Using Docker
docker run -d --name rabbitmq -p 5672:5672 -p 15672:15672 rabbitmq:3-management

# Or install locally (Ubuntu/Debian)
sudo apt-get install rabbitmq-server
sudo systemctl start rabbitmq-server
```

## Basic Usage

### Simple Publisher-Subscriber

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::{Publisher, Subscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create RabbitMQ publisher and subscriber
    let publisher = RabbitMQPublisher::new("amqp://localhost:5672");
    let mut subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "my-queue");

    // Subscribe to an exchange
    subscriber.subscribe("user-events").await?;
    
    // Publish a message
    let message = Message::new(b"User registered: john@example.com".to_vec())
        .with_metadata("event_type", "user_registration")
        .with_metadata("user_id", "12345")
        .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339());
    
    publisher.publish("user-events", vec![message]).await?;
    
    // Receive and process the message
    let received = subscriber.receive().await?;
    println!("Received: {:?}", String::from_utf8_lossy(&received.payload));
    println!("Event type: {:?}", received.get_metadata("event_type"));
    
    Ok(())
}
```

## Message Acknowledgments

RabbitMQ supports message acknowledgments for reliable processing:

```rust
use kincir::rabbitmq::RabbitMQAckSubscriber;
use kincir::{AckSubscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut subscriber = RabbitMQAckSubscriber::new("amqp://localhost:5672", "orders-queue");
    subscriber.subscribe("orders").await?;

    loop {
        // Receive message with acknowledgment handle
        let (message, ack_handle) = subscriber.receive_with_ack().await?;
        
        // Process the message
        match process_order(&message).await {
            Ok(()) => {
                println!("Order processed successfully");
                // Acknowledge successful processing
                ack_handle.ack().await?;
            }
            Err(e) => {
                eprintln!("Failed to process order: {}", e);
                // Reject and requeue the message
                ack_handle.nack(true).await?;
            }
        }
    }
}

async fn process_order(message: &Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Simulate order processing
    println!("Processing order: {:?}", String::from_utf8_lossy(&message.payload));
    
    // Simulate potential failure
    if message.get_metadata("order_id") == Some("FAIL") {
        return Err("Simulated processing failure".into());
    }
    
    // Simulate processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    Ok(())
}
```

## Advanced Configuration

### Custom RabbitMQ Configuration

```rust
use kincir::rabbitmq::{RabbitMQConfig, RabbitMQPublisher, RabbitMQSubscriber};
use kincir::{Publisher, Subscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create custom configuration
    let config = RabbitMQConfig {
        connection_url: "amqp://user:password@localhost:5672/vhost".to_string(),
        exchange_name: "my-exchange".to_string(),
        exchange_type: "topic".to_string(),
        durable: true,
        auto_delete: false,
        routing_key: Some("events.user.*".to_string()),
        queue_options: Some(QueueOptions {
            durable: true,
            exclusive: false,
            auto_delete: false,
            arguments: HashMap::new(),
        }),
    };

    // Create publisher and subscriber with custom config
    let publisher = RabbitMQPublisher::with_config(config.clone());
    let mut subscriber = RabbitMQSubscriber::with_config(config, "user-events-queue");
    
    // Use with routing keys
    subscriber.subscribe("events.user.registered").await?;
    
    let message = Message::new(b"User John registered".to_vec());
    publisher.publish_with_routing_key("events.user.registered", vec![message]).await?;
    
    let received = subscriber.receive().await?;
    println!("Received: {:?}", String::from_utf8_lossy(&received.payload));
    
    Ok(())
}
```

## Topic-Based Routing

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::{Publisher, Subscriber, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let publisher = RabbitMQPublisher::new("amqp://localhost:5672");
    
    // Create subscribers for different routing patterns
    let mut user_subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "user-events");
    let mut order_subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "order-events");
    let mut all_subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "all-events");
    
    // Subscribe to different routing patterns
    user_subscriber.subscribe("events.user.*").await?;
    order_subscriber.subscribe("events.order.*").await?;
    all_subscriber.subscribe("events.*").await?;
    
    // Publish messages with different routing keys
    let user_message = Message::new(b"User registered".to_vec());
    let order_message = Message::new(b"Order placed".to_vec());
    let system_message = Message::new(b"System maintenance".to_vec());
    
    publisher.publish_with_routing_key("events.user.registered", vec![user_message]).await?;
    publisher.publish_with_routing_key("events.order.placed", vec![order_message]).await?;
    publisher.publish_with_routing_key("events.system.maintenance", vec![system_message]).await?;
    
    // Each subscriber will receive relevant messages
    println!("User subscriber: {:?}", user_subscriber.receive().await?);
    println!("Order subscriber: {:?}", order_subscriber.receive().await?);
    
    // All subscriber receives all messages
    for _ in 0..3 {
        println!("All subscriber: {:?}", all_subscriber.receive().await?);
    }
    
    Ok(())
}
```

## Work Queue Pattern

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let publisher = RabbitMQPublisher::new("amqp://localhost:5672");
    
    // Create multiple workers
    let mut worker_handles = vec![];
    for worker_id in 0..3 {
        let handle = task::spawn(async move {
            let mut subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "work-queue");
            subscriber.subscribe("tasks").await.unwrap();
            
            loop {
                match subscriber.receive().await {
                    Ok(message) => {
                        let task_data = String::from_utf8_lossy(&message.payload);
                        println!("Worker {} processing: {}", worker_id, task_data);
                        
                        // Simulate work
                        let work_duration = message.get_metadata("duration")
                            .and_then(|d| d.parse::<u64>().ok())
                            .unwrap_or(1000);
                        
                        tokio::time::sleep(tokio::time::Duration::from_millis(work_duration)).await;
                        println!("Worker {} completed: {}", worker_id, task_data);
                    }
                    Err(e) => {
                        eprintln!("Worker {} error: {}", worker_id, e);
                        break;
                    }
                }
            }
        });
        worker_handles.push(handle);
    }
    
    // Publish work tasks
    for i in 0..10 {
        let task = Message::new(format!("Task #{}", i).into_bytes())
            .with_metadata("task_id", &i.to_string())
            .with_metadata("duration", &(500 + i * 100).to_string()); // Varying work duration
        
        publisher.publish("tasks", vec![task]).await?;
        println!("Published task #{}", i);
    }
    
    // Let workers process for a while
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    
    // Cancel workers
    for handle in worker_handles {
        handle.abort();
    }
    
    Ok(())
}
```

## RPC Pattern

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::{Publisher, Subscriber, Message};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // RPC Server
    let server_handle = tokio::spawn(async {
        let mut subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "rpc-requests");
        let publisher = RabbitMQPublisher::new("amqp://localhost:5672");
        
        subscriber.subscribe("rpc_queue").await.unwrap();
        
        loop {
            match subscriber.receive().await {
                Ok(request) => {
                    let request_data = String::from_utf8_lossy(&request.payload);
                    println!("RPC Server received: {}", request_data);
                    
                    // Process request (simulate calculation)
                    let result = if let Some(num_str) = request.get_metadata("number") {
                        num_str.parse::<i32>().unwrap_or(0) * 2
                    } else {
                        0
                    };
                    
                    // Send response
                    if let Some(reply_to) = request.get_metadata("reply_to") {
                        if let Some(correlation_id) = request.get_metadata("correlation_id") {
                            let response = Message::new(result.to_string().into_bytes())
                                .with_metadata("correlation_id", correlation_id);
                            
                            publisher.publish(reply_to, vec![response]).await.unwrap();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("RPC Server error: {}", e);
                    break;
                }
            }
        }
    });
    
    // RPC Client
    let client_handle = tokio::spawn(async {
        let publisher = RabbitMQPublisher::new("amqp://localhost:5672");
        let mut subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "rpc-responses");
        let reply_queue = "rpc_reply_queue";
        
        subscriber.subscribe(reply_queue).await.unwrap();
        
        // Send RPC requests
        for i in 1..=5 {
            let correlation_id = Uuid::new_v4().to_string();
            
            let request = Message::new(format!("Calculate double of {}", i).into_bytes())
                .with_metadata("number", &i.to_string())
                .with_metadata("reply_to", reply_queue)
                .with_metadata("correlation_id", &correlation_id);
            
            publisher.publish("rpc_queue", vec![request]).await.unwrap();
            println!("RPC Client sent request for number: {}", i);
            
            // Wait for response
            let response = subscriber.receive().await.unwrap();
            if response.get_metadata("correlation_id") == Some(&correlation_id) {
                let result = String::from_utf8_lossy(&response.payload);
                println!("RPC Client received result: {}", result);
            }
        }
    });
    
    // Wait for both client and server
    tokio::select! {
        _ = server_handle => {},
        _ = client_handle => {},
    }
    
    Ok(())
}
```

## Error Handling and Reconnection

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::{Publisher, Subscriber, Message, KincirError};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut retry_count = 0;
    let max_retries = 5;
    
    loop {
        match run_rabbitmq_client().await {
            Ok(()) => {
                println!("RabbitMQ client completed successfully");
                break;
            }
            Err(KincirError::ConnectionError(e)) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    eprintln!("Max retries reached. Giving up. Error: {}", e);
                    return Err(e.into());
                }
                
                println!("Connection failed (attempt {}). Retrying in 5 seconds...", retry_count);
                sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                eprintln!("Non-recoverable error: {}", e);
                return Err(e.into());
            }
        }
    }
    
    Ok(())
}

async fn run_rabbitmq_client() -> Result<(), KincirError> {
    let publisher = RabbitMQPublisher::new("amqp://localhost:5672");
    let mut subscriber = RabbitMQSubscriber::new("amqp://localhost:5672", "test-queue");
    
    // This will fail if RabbitMQ is not running
    subscriber.subscribe("test-exchange").await?;
    
    let message = Message::new(b"Test message".to_vec());
    publisher.publish("test-exchange", vec![message]).await?;
    
    let received = subscriber.receive().await?;
    println!("Received: {:?}", String::from_utf8_lossy(&received.payload));
    
    Ok(())
}
```

## Performance Considerations

### Connection Pooling

```rust
use kincir::rabbitmq::{RabbitMQConnectionPool, RabbitMQPublisher};
use kincir::{Publisher, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create connection pool for better performance
    let pool = Arc::new(RabbitMQConnectionPool::new("amqp://localhost:5672", 10).await?);
    
    // Create multiple publishers sharing the pool
    let mut handles = vec![];
    for i in 0..5 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let publisher = RabbitMQPublisher::with_pool(pool_clone);
            
            for j in 0..100 {
                let message = Message::new(format!("Message {}-{}", i, j).into_bytes());
                publisher.publish("high-throughput", vec![message]).await.unwrap();
            }
        });
        handles.push(handle);
    }
    
    // Wait for all publishers to complete
    for handle in handles {
        handle.await?;
    }
    
    println!("Published 500 messages using connection pool");
    Ok(())
}
```

## Testing with RabbitMQ

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::{clients::Cli, images::rabbitmq::RabbitMq, Container};

    #[tokio::test]
    async fn test_rabbitmq_integration() {
        // Start RabbitMQ container for testing
        let docker = Cli::default();
        let rabbitmq_container = docker.run(RabbitMq::default());
        let connection_string = format!(
            "amqp://guest:guest@127.0.0.1:{}",
            rabbitmq_container.get_host_port_ipv4(5672)
        );
        
        let publisher = RabbitMQPublisher::new(&connection_string);
        let mut subscriber = RabbitMQSubscriber::new(&connection_string, "test-queue");
        
        subscriber.subscribe("test-exchange").await.unwrap();
        
        let message = Message::new(b"integration test message".to_vec());
        publisher.publish("test-exchange", vec![message]).await.unwrap();
        
        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.payload, b"integration test message");
    }
}
```

## Next Steps

- [Kafka Integration](/examples/kafka.html) - High-throughput streaming
- [MQTT Support](/examples/mqtt.html) - IoT messaging
- [Message Acknowledgments](/examples/acknowledgments.html) - Reliable processing
- [Microservices Communication](/examples/microservices.html) - Service integration

## Resources

- [RabbitMQ Documentation](https://www.rabbitmq.com/documentation.html)
- [AMQP Protocol](https://www.amqp.org/)
- [Kincir API Documentation](https://docs.rs/kincir)
- [Getting Started Guide](/docs/getting-started.html)
