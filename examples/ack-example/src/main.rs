//! Example demonstrating acknowledgment handling infrastructure
//!
//! This example shows the acknowledgment configuration and basic functionality
//! that has been implemented so far.

use kincir::ack::{AckConfig, AckStats, AckHandle, AckSubscriber};
use kincir::memory::{InMemoryBroker, InMemoryConfig, InMemoryAckHandle, InMemoryAckSubscriber};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Kincir Acknowledgment Infrastructure Example");
    println!("===============================================");
    
    // Demonstrate acknowledgment configuration
    demonstrate_ack_configs().await?;
    
    // Demonstrate acknowledgment statistics
    demonstrate_ack_stats().await?;
    
    // Demonstrate acknowledgment handles
    demonstrate_ack_handles().await?;
    
    // Demonstrate broker acknowledgment operations
    demonstrate_broker_ack_operations().await?;
    
    println!("\n🎉 Acknowledgment infrastructure example completed!");
    
    Ok(())
}

/// Example showing different acknowledgment configurations
async fn demonstrate_ack_configs() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚙️  Acknowledgment Configuration Examples:");
    
    // Manual acknowledgment (default)
    let manual_config = AckConfig::manual()
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(3);
    println!("📋 Manual ACK Config:");
    println!("   Mode: {:?}", manual_config.mode);
    println!("   Timeout: {:?}", manual_config.timeout);
    println!("   Max Retries: {}", manual_config.max_retries);
    
    // Automatic acknowledgment
    let auto_config = AckConfig::auto()
        .with_timeout(Duration::from_secs(10));
    println!("\n📋 Auto ACK Config:");
    println!("   Mode: {:?}", auto_config.mode);
    println!("   Timeout: {:?}", auto_config.timeout);
    
    // Client-controlled automatic acknowledgment
    let client_auto_config = AckConfig::client_auto()
        .with_max_retries(5)
        .with_retry_delay(Duration::from_secs(2))
        .with_dead_letter_topic(Some("failed-orders".to_string()));
    println!("\n📋 Client Auto ACK Config:");
    println!("   Mode: {:?}", client_auto_config.mode);
    println!("   Max Retries: {}", client_auto_config.max_retries);
    println!("   Retry Delay: {:?}", client_auto_config.retry_delay);
    println!("   Dead Letter Topic: {:?}", client_auto_config.dead_letter_topic);
    
    Ok(())
}

/// Example showing acknowledgment statistics
async fn demonstrate_ack_stats() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Acknowledgment Statistics Example:");
    
    let mut stats = AckStats::new();
    
    // Simulate some acknowledgment operations
    stats.increment_acked();
    stats.increment_acked();
    stats.increment_acked();
    stats.increment_nacked();
    stats.increment_requeued();
    
    println!("   Messages Acknowledged: {}", stats.acked);
    println!("   Messages Nacked: {}", stats.nacked);
    println!("   Messages Requeued: {}", stats.requeued);
    println!("   Total Processed: {}", stats.total_processed());
    println!("   Success Rate: {:.2}%", stats.success_rate() * 100.0);
    
    Ok(())
}

/// Example showing acknowledgment handles
async fn demonstrate_ack_handles() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🏷️  Acknowledgment Handle Example:");
    
    let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
    let broker_ref = Arc::downgrade(&broker);
    
    let handle = InMemoryAckHandle::new(
        "msg-12345".to_string(),
        "orders".to_string(),
        SystemTime::now(),
        1,
        broker_ref,
    );
    
    println!("   Message ID: {}", handle.message_id());
    println!("   Topic: {}", handle.topic());
    println!("   Delivery Count: {}", handle.delivery_count());
    println!("   Is Retry: {}", handle.is_retry());
    println!("   Handle ID: {}", handle.handle_id());
    
    // Create a retry handle
    let retry_handle = InMemoryAckHandle::new(
        "msg-67890".to_string(),
        "orders".to_string(),
        SystemTime::now(),
        3,
        Arc::downgrade(&broker),
    );
    
    println!("\n   Retry Handle:");
    println!("   Message ID: {}", retry_handle.message_id());
    println!("   Delivery Count: {}", retry_handle.delivery_count());
    println!("   Is Retry: {}", retry_handle.is_retry());
    
    Ok(())
}

/// Example showing broker acknowledgment operations
async fn demonstrate_broker_ack_operations() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 Broker Acknowledgment Operations:");
    
    let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::new().with_stats(true)));
    
    // Create some test handles
    let handles = vec![
        InMemoryAckHandle::new(
            "msg-001".to_string(),
            "test-topic".to_string(),
            SystemTime::now(),
            1,
            Arc::downgrade(&broker),
        ),
        InMemoryAckHandle::new(
            "msg-002".to_string(),
            "test-topic".to_string(),
            SystemTime::now(),
            1,
            Arc::downgrade(&broker),
        ),
        InMemoryAckHandle::new(
            "msg-003".to_string(),
            "test-topic".to_string(),
            SystemTime::now(),
            2,
            Arc::downgrade(&broker),
        ),
    ];
    
    println!("   Testing individual acknowledgment operations...");
    
    // Test individual ack
    let result = broker.ack_message(&handles[0]).await;
    println!("   ✅ ACK result: {:?}", result);
    
    // Test individual nack with requeue
    let result = broker.nack_message(&handles[1], true).await;
    println!("   ⚠️  NACK (requeue) result: {:?}", result);
    
    // Test individual nack without requeue
    let result = broker.nack_message(&handles[2], false).await;
    println!("   ❌ NACK (dead letter) result: {:?}", result);
    
    println!("\n   Testing batch acknowledgment operations...");
    
    // Test batch ack
    let result = broker.ack_batch(&handles).await;
    println!("   ✅ Batch ACK result: {:?}", result);
    
    // Test batch nack
    let result = broker.nack_batch(&handles, true).await;
    println!("   ⚠️  Batch NACK result: {:?}", result);
    
    // Show broker statistics
    if let Some(stats) = broker.stats() {
        println!("\n📊 Broker Statistics After Operations:");
        println!("   Messages Consumed: {}", stats.messages_consumed.load(std::sync::atomic::Ordering::Relaxed));
        println!("   Publish Errors (requeues): {}", stats.publish_errors.load(std::sync::atomic::Ordering::Relaxed));
        println!("   Consume Errors (dead letters): {}", stats.consume_errors.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    // Test subscriber creation
    println!("\n👥 Acknowledgment Subscriber:");
    let subscriber = InMemoryAckSubscriber::new(broker.clone());
    println!("   Subscriber created successfully");
    println!("   Is subscribed: {}", subscriber.is_subscribed().await);
    println!("   Subscribed topic: {:?}", subscriber.subscribed_topic().await);
    
    // Test subscription validation
    println!("\n🔍 Subscription Validation:");
    match subscriber.subscribe("").await {
        Err(e) => println!("   ❌ Empty topic validation: {:?}", e),
        Ok(_) => println!("   ⚠️  Unexpected success"),
    }
    
    match subscriber.subscribe("valid-topic").await {
        Ok(_) => println!("   ✅ Valid topic subscription: Success"),
        Err(e) => println!("   ❌ Unexpected error: {:?}", e),
    }
    
    println!("   Is subscribed: {}", subscriber.is_subscribed().await);
    println!("   Subscribed topic: {:?}", subscriber.subscribed_topic().await);
    
    Ok(())
}
