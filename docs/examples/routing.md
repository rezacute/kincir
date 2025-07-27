---
layout: docs
title: Message Routing Example
description: Advanced message routing and transformation patterns
---

# Message Routing Example

Message routing is a powerful feature in Kincir that allows you to process and transform messages as they flow through your system. The Router component provides a flexible way to handle message processing with customizable handlers.

## Basic Router Setup

```rust
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::router::{Router, Logger, StdLogger};
use kincir::Message;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Configure message brokers
    let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672"));
    let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672", "input-queue"));

    // Define message handler
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            // Process the message
            let mut processed_msg = msg;
            processed_msg.set_metadata("processed", "true");
            processed_msg.set_metadata("processed_at", &chrono::Utc::now().to_rfc3339());
            Ok(vec![processed_msg])
        })
    });

    // Create and run router
    let router = Router::new(
        logger,
        "input-exchange".to_string(),
        "output-exchange".to_string(),
        subscriber,
        publisher,
        handler,
    );

    router.run().await
}
```

## Advanced Message Transformation

```rust
use kincir::router::{Router, MessageHandler};
use kincir::Message;
use serde_json::{Value, json};

// Custom message handler that transforms JSON messages
struct JsonTransformHandler;

impl MessageHandler for JsonTransformHandler {
    async fn handle(&self, message: Message) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        let payload = String::from_utf8_lossy(&message.payload);
        
        // Parse JSON
        let mut data: Value = serde_json::from_str(&payload)?;
        
        // Transform the data
        if let Some(obj) = data.as_object_mut() {
            // Add processing metadata
            obj.insert("processed_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
            obj.insert("processor_version".to_string(), json!("1.0"));
            
            // Transform specific fields
            if let Some(amount) = obj.get("amount").and_then(|v| v.as_f64()) {
                obj.insert("amount_cents".to_string(), json!((amount * 100.0) as i64));
            }
            
            // Add derived fields
            if let Some(user_id) = obj.get("user_id").and_then(|v| v.as_str()) {
                obj.insert("user_segment".to_string(), json!(determine_user_segment(user_id)));
            }
        }
        
        // Create transformed message
        let transformed_message = Message::new(data.to_string().into_bytes())
            .with_metadata("transformation", "json_enhanced")
            .with_metadata("original_uuid", &message.uuid);
        
        Ok(vec![transformed_message])
    }
}

fn determine_user_segment(user_id: &str) -> &'static str {
    // Simple segmentation logic
    match user_id.chars().last().unwrap_or('0') {
        '0'..='3' => "premium",
        '4'..='7' => "standard", 
        _ => "basic",
    }
}
```

## Multi-Output Routing

```rust
use kincir::router::MessageHandler;
use kincir::Message;

struct MultiOutputHandler;

impl MessageHandler for MultiOutputHandler {
    async fn handle(&self, message: Message) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        let mut outputs = Vec::new();
        
        // Original message goes to archive
        let archive_msg = message.clone()
            .with_metadata("destination", "archive")
            .with_metadata("archived_at", &chrono::Utc::now().to_rfc3339());
        outputs.push(archive_msg);
        
        // Extract metrics
        if let Some(event_type) = message.get_metadata("event_type") {
            let metric_msg = Message::new(
                {% raw %}format!(r#"{{"metric":"event_count","type":"{}","value":1}}"#, event_type).into_bytes(){% endraw %}
            ).with_metadata("destination", "metrics")
             .with_metadata("metric_type", "counter");
            outputs.push(metric_msg);
        }
        
        // Generate alerts for critical events
        if message.get_metadata("priority") == Some("critical") {
            let alert_msg = Message::new(
                format!("CRITICAL EVENT: {}", String::from_utf8_lossy(&message.payload)).into_bytes()
            ).with_metadata("destination", "alerts")
             .with_metadata("alert_level", "critical");
            outputs.push(alert_msg);
        }
        
        // Process for different user segments
        if let Some(user_segment) = message.get_metadata("user_segment") {
            let segment_msg = message.clone()
                .with_metadata("destination", &format!("segment_{}", user_segment))
                .with_metadata("segmented_at", &chrono::Utc::now().to_rfc3339());
            outputs.push(segment_msg);
        }
        
        Ok(outputs)
    }
}
```

## Conditional Routing

```rust
use kincir::router::MessageHandler;
use kincir::Message;

struct ConditionalRouter;

impl MessageHandler for ConditionalRouter {
    async fn handle(&self, message: Message) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        let event_type = message.get_metadata("event_type").unwrap_or("unknown");
        
        match event_type {
            "user_registration" => self.handle_user_registration(message).await,
            "order_placed" => self.handle_order_placed(message).await,
            "payment_processed" => self.handle_payment_processed(message).await,
            "error" => self.handle_error_event(message).await,
            _ => self.handle_unknown_event(message).await,
        }
    }
}

impl ConditionalRouter {
    async fn handle_user_registration(&self, message: Message) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        let mut outputs = Vec::new();
        
        // Send welcome email
        let email_msg = Message::new(b"Send welcome email".to_vec())
            .with_metadata("action", "send_email")
            .with_metadata("template", "welcome")
            .with_metadata("user_id", message.get_metadata("user_id").unwrap_or("unknown"));
        outputs.push(email_msg);
        
        // Update user analytics
        let analytics_msg = Message::new(b"New user registered".to_vec())
            .with_metadata("action", "update_analytics")
            .with_metadata("metric", "user_registration");
        outputs.push(analytics_msg);
        
        Ok(outputs)
    }
    
    async fn handle_order_placed(&self, message: Message) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        let mut outputs = Vec::new();
        
        // Process payment
        let payment_msg = Message::new(message.payload.clone())
            .with_metadata("action", "process_payment")
            .with_metadata("order_id", message.get_metadata("order_id").unwrap_or("unknown"));
        outputs.push(payment_msg);
        
        // Update inventory
        let inventory_msg = Message::new(b"Update inventory".to_vec())
            .with_metadata("action", "update_inventory")
            .with_metadata("order_id", message.get_metadata("order_id").unwrap_or("unknown"));
        outputs.push(inventory_msg);
        
        Ok(outputs)
    }
    
    async fn handle_payment_processed(&self, message: Message) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        let mut outputs = Vec::new();
        
        // Send confirmation email
        let email_msg = Message::new(b"Send order confirmation".to_vec())
            .with_metadata("action", "send_email")
            .with_metadata("template", "order_confirmation");
        outputs.push(email_msg);
        
        // Trigger fulfillment
        let fulfillment_msg = Message::new(b"Start order fulfillment".to_vec())
            .with_metadata("action", "start_fulfillment");
        outputs.push(fulfillment_msg);
        
        Ok(outputs)
    }
    
    async fn handle_error_event(&self, message: Message) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        // Send to error handling system
        let error_msg = message.clone()
            .with_metadata("destination", "error_handler")
            .with_metadata("requires_attention", "true");
        
        Ok(vec![error_msg])
    }
    
    async fn handle_unknown_event(&self, message: Message) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        // Log unknown events for analysis
        let log_msg = message.clone()
            .with_metadata("destination", "unknown_events_log")
            .with_metadata("needs_classification", "true");
        
        Ok(vec![log_msg])
    }
}
```

## Next Steps

- [Error Handling](/examples/error-handling.html) - Comprehensive error strategies
- [Performance Optimization](/examples/performance.html) - High-throughput patterns
- [Microservices Communication](/examples/microservices.html) - Service integration

## Resources

- [Kincir API Documentation](https://docs.rs/kincir)
- [Getting Started Guide](/docs/getting-started.html)
