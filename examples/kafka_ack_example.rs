//! Kafka Acknowledgment Example
//!
//! This example demonstrates how to use Apache Kafka with manual acknowledgment handling.
//! It shows publishing messages, receiving them with acknowledgment handles, and
//! performing manual acknowledgment or negative acknowledgment operations.
//!
//! Prerequisites:
//! - Apache Kafka server running on localhost:9092
//! - Topic creation (topics will be auto-created if auto.create.topics.enable=true)
//!
//! Run with: cargo run --example kafka_ack_example

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::kafka::{KafkaAckSubscriber, KafkaPublisher};
use kincir::{Message, Publisher};
use std::time::Duration;
use tokio::time::{sleep, timeout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 Kafka Acknowledgment Example");
    println!("=================================");

    // Check if Kafka is available
    if !is_kafka_available().await {
        eprintln!("❌ Kafka is not available at localhost:9092");
        eprintln!("   Please start Kafka server and try again.");
        eprintln!("   Docker: docker run -d --name kafka -p 9092:9092 apache/kafka:latest");
        return Ok(());
    }

    println!("✅ Kafka connection available");

    // Configuration
    let brokers = vec!["127.0.0.1:9092".to_string()];
    let topic = "ack-example-topic";
    let consumer_group = "ack-example-group";

    // Create publisher
    println!("\n📤 Creating Kafka publisher...");
    let publisher = KafkaPublisher::new(brokers.clone())?;
    println!("✅ Publisher created successfully");

    // Create subscriber with acknowledgment support
    println!("\n📥 Creating Kafka acknowledgment subscriber...");
    let mut subscriber = KafkaAckSubscriber::new(brokers, consumer_group.to_string()).await?;
    println!("✅ Acknowledgment subscriber created successfully");
    println!("   Consumer Group: {}", subscriber.group_id());

    // Subscribe to topic
    println!("\n🔗 Subscribing to topic: {}", topic);
    subscriber.subscribe(topic).await?;
    println!("✅ Successfully subscribed to topic");

    // Example 1: Basic acknowledgment
    println!("\n" + "=".repeat(50).as_str());
    println!("📋 Example 1: Basic Message Acknowledgment");
    println!("=".repeat(50));
    
    basic_acknowledgment_example(&publisher, &mut subscriber, topic).await?;

    // Example 2: Negative acknowledgment with requeue
    println!("\n" + "=".repeat(50).as_str());
    println!("📋 Example 2: Negative Acknowledgment with Requeue");
    println!("=".repeat(50));
    
    negative_acknowledgment_example(&publisher, &mut subscriber, topic).await?;

    // Example 3: Batch acknowledgment
    println!("\n" + "=".repeat(50).as_str());
    println!("📋 Example 3: Batch Acknowledgment");
    println!("=".repeat(50));
    
    batch_acknowledgment_example(&publisher, &mut subscriber, topic).await?;

    // Example 4: Offset management and consumer groups
    println!("\n" + "=".repeat(50).as_str());
    println!("📋 Example 4: Offset Management and Consumer Groups");
    println!("=".repeat(50));
    
    offset_management_example(&publisher, &mut subscriber, topic).await?;

    println!("\n🎉 All examples completed successfully!");
    println!("   The Kafka acknowledgment system provides reliable message processing");
    println!("   with manual control over offset commits and consumer group coordination.");

    Ok(())
}

async fn basic_acknowledgment_example(
    publisher: &KafkaPublisher,
    subscriber: &mut KafkaAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing message for basic acknowledgment...");
    
    let message = Message::new(b"Hello from Kafka with acknowledgment!".to_vec())
        .with_metadata("example", "basic_ack")
        .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339());
    
    publisher.publish(topic, vec![message.clone()]).await?;
    println!("✅ Message published: {}", String::from_utf8_lossy(&message.payload));

    println!("📥 Receiving message with acknowledgment handle...");
    let (received_message, ack_handle) = timeout(
        Duration::from_secs(15),
        subscriber.receive_with_ack()
    ).await??;

    println!("✅ Message received:");
    println!("   ID: {}", received_message.uuid);
    println!("   Payload: {}", String::from_utf8_lossy(&received_message.payload));
    println!("   Metadata: {:?}", received_message.metadata);
    
    println!("📋 Acknowledgment handle details:");
    println!("   Message ID: {}", ack_handle.message_id());
    println!("   Topic: {}", ack_handle.topic());
    println!("   Partition: {}", ack_handle.partition());
    println!("   Offset: {}", ack_handle.offset());
    println!("   Delivery Count: {}", ack_handle.delivery_count());
    println!("   Is Retry: {}", ack_handle.is_retry());
    println!("   Handle ID: {}", ack_handle.handle_id());

    println!("✅ Acknowledging message (committing offset)...");
    subscriber.ack(ack_handle).await?;
    println!("✅ Message acknowledged successfully - offset committed");

    Ok(())
}

async fn negative_acknowledgment_example(
    publisher: &KafkaPublisher,
    subscriber: &mut KafkaAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing message for negative acknowledgment...");
    
    let message = Message::new(b"Message to be negatively acknowledged".to_vec())
        .with_metadata("example", "nack")
        .with_metadata("should_process", "false");
    
    publisher.publish(topic, vec![message.clone()]).await?;
    println!("✅ Message published: {}", String::from_utf8_lossy(&message.payload));

    println!("📥 Receiving message...");
    let (received_message, ack_handle) = timeout(
        Duration::from_secs(15),
        subscriber.receive_with_ack()
    ).await??;

    println!("✅ Message received: {}", String::from_utf8_lossy(&received_message.payload));
    println!("   Partition: {}, Offset: {}", ack_handle.partition(), ack_handle.offset());

    // Simulate processing failure
    let should_process = received_message.metadata.get("should_process")
        .map(|v| v == "true")
        .unwrap_or(false);

    if !should_process {
        println!("❌ Processing failed - negatively acknowledging with requeue...");
        subscriber.nack(ack_handle, true).await?;
        println!("✅ Message negatively acknowledged (offset NOT committed)");
        println!("💡 Message will be redelivered on consumer restart or rebalance");
        println!("   In Kafka, 'requeue' means we don't commit the offset");
    } else {
        println!("✅ Processing successful - acknowledging...");
        subscriber.ack(ack_handle).await?;
    }

    Ok(())
}

async fn batch_acknowledgment_example(
    publisher: &KafkaPublisher,
    subscriber: &mut KafkaAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing batch of messages...");
    
    let messages = vec![
        Message::new(b"Batch message 1".to_vec()).with_metadata("batch_id", "1"),
        Message::new(b"Batch message 2".to_vec()).with_metadata("batch_id", "2"),
        Message::new(b"Batch message 3".to_vec()).with_metadata("batch_id", "3"),
    ];
    
    publisher.publish(topic, messages.clone()).await?;
    println!("✅ Published {} messages", messages.len());

    println!("📥 Receiving messages and collecting acknowledgment handles...");
    let mut ack_handles = Vec::new();
    
    for i in 0..messages.len() {
        let (received_message, ack_handle) = timeout(
            Duration::from_secs(15),
            subscriber.receive_with_ack()
        ).await??;
        
        println!("✅ Received message {}: {} (partition: {}, offset: {})", 
                i + 1, 
                String::from_utf8_lossy(&received_message.payload),
                ack_handle.partition(),
                ack_handle.offset());
        
        ack_handles.push(ack_handle);
    }

    println!("📋 Performing batch acknowledgment for {} messages...", ack_handles.len());
    println!("   Kafka will commit the highest offset for each partition");
    
    subscriber.ack_batch(ack_handles).await?;
    println!("✅ Batch acknowledgment completed successfully");
    println!("💡 All messages in the batch were acknowledged with optimized offset commits");

    Ok(())
}

async fn offset_management_example(
    publisher: &KafkaPublisher,
    subscriber: &mut KafkaAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing messages for offset management demonstration...");
    
    let messages = vec![
        Message::new(b"Offset message 1".to_vec()).with_metadata("process", "true"),
        Message::new(b"Offset message 2".to_vec()).with_metadata("process", "false"),
        Message::new(b"Offset message 3".to_vec()).with_metadata("process", "true"),
    ];
    
    publisher.publish(topic, messages.clone()).await?;
    println!("✅ Published {} messages for offset management", messages.len());

    println!("📥 Processing messages with selective acknowledgment...");
    
    for i in 0..messages.len() {
        let (received_message, ack_handle) = timeout(
            Duration::from_secs(15),
            subscriber.receive_with_ack()
        ).await??;

        println!("🔄 Processing message {} at offset {}", i + 1, ack_handle.offset());
        
        // Simulate processing based on metadata
        let should_process = received_message.metadata.get("process")
            .map(|v| v == "true")
            .unwrap_or(false);

        if should_process {
            println!("✅ Processing successful - acknowledging offset {}", ack_handle.offset());
            subscriber.ack(ack_handle).await?;
        } else {
            println!("❌ Processing failed - discarding message at offset {}", ack_handle.offset());
            // Discard by committing offset (skip this message)
            subscriber.nack(ack_handle, false).await?;
        }
        
        // Small delay to demonstrate sequential processing
        sleep(Duration::from_millis(100)).await;
    }

    println!("📊 Offset management completed:");
    println!("   Consumer group: {}", subscriber.group_id());
    println!("   Topic: {}", topic);
    println!("💡 Kafka tracks committed offsets per consumer group and partition");
    println!("   This enables reliable message processing and consumer failover");

    Ok(())
}

async fn is_kafka_available() -> bool {
    match tokio::net::TcpStream::connect("127.0.0.1:9092").await {
        Ok(_) => true,
        Err(_) => false,
    }
}
