//! Example demonstrating acknowledgment handling with the in-memory broker
//!
//! This example shows how to use the new acknowledgment functionality to ensure
//! reliable message processing with manual acknowledgment control.

use kincir::ack::{AckConfig, AckHandle, AckSubscriber};
use kincir::memory::{InMemoryBroker, InMemoryConfig, InMemoryPublisher, InMemoryAckSubscriber};
use kincir::{Publisher, Message};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Kincir Acknowledgment Example");
    println!("================================");
    
    // Create broker with statistics enabled
    let config = InMemoryConfig::new()
        .with_stats(true)
        .with_maintain_order(true);
    let broker = Arc::new(InMemoryBroker::new(config));
    
    // Create publisher and acknowledgment-aware subscriber
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut ack_subscriber = InMemoryAckSubscriber::new(broker.clone());
    
    println!("\n📡 Setting up acknowledgment-based message processing...");
    
    // Subscribe to a topic
    ack_subscriber.subscribe("orders").await?;
    println!("✅ Subscribed to 'orders' topic");
    
    // Publish some test messages
    let messages = vec![
        Message::new(b"Order #1001: 2x Coffee".to_vec())
            .with_metadata("customer", "Alice")
            .with_metadata("priority", "high"),
        Message::new(b"Order #1002: 1x Tea".to_vec())
            .with_metadata("customer", "Bob")
            .with_metadata("priority", "normal"),
        Message::new(b"Order #1003: 3x Sandwich".to_vec())
            .with_metadata("customer", "Charlie")
            .with_metadata("priority", "low"),
    ];
    
    publisher.publish("orders", messages).await?;
    println!("📤 Published 3 orders");
    
    // Process messages with acknowledgment
    println!("\n🔄 Processing messages with acknowledgment...");
    
    for i in 1..=3 {
        // Receive message with acknowledgment handle
        let (message, ack_handle) = ack_subscriber.receive_with_ack().await?;
        
        let order_text = String::from_utf8_lossy(&message.payload);
        let customer = message
            .metadata
            .get("customer")
            .map(|s| s.as_str())
            .unwrap_or("Unknown");
        let priority = message
            .metadata
            .get("priority")
            .map(|s| s.as_str())
            .unwrap_or("normal");
        
        println!("📨 Received: {}", order_text);
        println!("   Customer: {}, Priority: {}", customer, priority);
        println!("   Message ID: {}", ack_handle.message_id());
        println!("   Topic: {}", ack_handle.topic());
        println!("   Delivery Count: {}", ack_handle.delivery_count());
        
        // Simulate processing time
        sleep(Duration::from_millis(100)).await;
        
        // Demonstrate different acknowledgment scenarios
        match i {
            1 => {
                // Successful processing - acknowledge
                ack_subscriber.ack(ack_handle).await?;
                println!("✅ Order processed successfully - ACK sent");
            }
            2 => {
                // Processing failed but recoverable - nack with requeue
                ack_subscriber.nack(ack_handle, true).await?;
                println!("⚠️  Processing failed - NACK sent (requeued)");
            }
            3 => {
                // Processing failed permanently - nack without requeue
                ack_subscriber.nack(ack_handle, false).await?;
                println!("❌ Processing failed permanently - NACK sent (dead letter)");
            }
            _ => {}
        }
        
        println!();
    }
    
    // Demonstrate batch acknowledgment
    println!("📦 Demonstrating batch acknowledgment...");
    
    // Publish more messages for batch processing
    let batch_messages = vec![
        Message::new(b"Batch Order #2001".to_vec()),
        Message::new(b"Batch Order #2002".to_vec()),
        Message::new(b"Batch Order #2003".to_vec()),
    ];
    
    publisher.publish("orders", batch_messages).await?;
    
    // Collect handles for batch acknowledgment
    let mut handles = Vec::new();
    for _ in 1..=3 {
        let (message, handle) = ack_subscriber.receive_with_ack().await?;
        println!("📨 Batched: {}", String::from_utf8_lossy(&message.payload));
        handles.push(handle);
    }
    
    // Acknowledge all messages in batch
    ack_subscriber.ack_batch(handles).await?;
    println!("✅ Batch acknowledgment completed");
    
    // Show broker statistics
    if let Some(stats) = publisher.stats() {
        println!("\n📊 Broker Statistics:");
        println!("   Messages Published: {}", stats.messages_published);
        println!("   Messages Consumed: {}", stats.messages_consumed);
        println!("   Active Topics: {}", stats.active_topics);
        println!("   Uptime: {:?}", stats.uptime);
    }
    
    // Show broker health
    let health = broker.health_check();
    println!("\n🏥 Broker Health:");
    println!("   Healthy: {}", health.is_healthy);
    println!("   Topics: {}", health.topic_count);
    println!("   Queued Messages: {}", health.total_queued_messages);
    println!("   Memory Usage: {} bytes", health.memory_usage_estimate);
    
    println!("\n🎉 Acknowledgment example completed successfully!");
    
    Ok(())
}

/// Example showing different acknowledgment configurations
#[allow(dead_code)]
async fn demonstrate_ack_configs() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚙️  Acknowledgment Configuration Examples:");
    
    // Manual acknowledgment (default)
    let manual_config = AckConfig::manual()
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(3);
    println!("📋 Manual ACK: {:?}", manual_config.mode);
    
    // Automatic acknowledgment
    let auto_config = AckConfig::auto()
        .with_timeout(Duration::from_secs(10));
    println!("📋 Auto ACK: {:?}", auto_config.mode);
    
    // Client-controlled automatic acknowledgment
    let client_auto_config = AckConfig::client_auto()
        .with_max_retries(5)
        .with_retry_delay(Duration::from_secs(2))
        .with_dead_letter_topic(Some("failed-orders".to_string()));
    println!("📋 Client Auto ACK: {:?}", client_auto_config.mode);
    
    Ok(())
}

/// Example showing error handling with acknowledgments
#[allow(dead_code)]
async fn demonstrate_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    use kincir::memory::InMemoryError;
    
    println!("\n🚨 Error Handling Examples:");
    
    let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());
    
    // Try to receive without subscribing
    match subscriber.receive_with_ack().await {
        Err(InMemoryError::NotSubscribed) => {
            println!("❌ Expected error: Not subscribed to any topic");
        }
        _ => println!("⚠️  Unexpected result"),
    }
    
    // Try operations on shutdown broker
    broker.shutdown()?;
    
    match subscriber.subscribe("test").await {
        Err(InMemoryError::BrokerShutdown) => {
            println!("❌ Expected error: Broker is shutdown");
        }
        _ => println!("⚠️  Unexpected result"),
    }
    
    Ok(())
}
