//! RabbitMQ Acknowledgment Example
//!
//! This example demonstrates how to use RabbitMQ with manual acknowledgment handling.
//! It shows publishing messages, receiving them with acknowledgment handles, and
//! performing manual acknowledgment or negative acknowledgment operations.
//!
//! Prerequisites:
//! - RabbitMQ server running on localhost:5672
//! - Default guest/guest credentials
//!
//! Run with: cargo run --example rabbitmq_ack_example

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::rabbitmq::{RabbitMQAckSubscriber, RabbitMQPublisher};
use kincir::{Message, Publisher};
use std::time::Duration;
use tokio::time::{sleep, timeout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 RabbitMQ Acknowledgment Example");
    println!("=====================================");

    // Check if RabbitMQ is available
    if !is_rabbitmq_available().await {
        eprintln!("❌ RabbitMQ is not available at localhost:5672");
        eprintln!("   Please start RabbitMQ server and try again.");
        eprintln!("   Docker: docker run -d --name rabbitmq -p 5672:5672 -p 15672:15672 rabbitmq:3-management");
        return Ok(());
    }

    println!("✅ RabbitMQ connection available");

    // Configuration
    let rabbitmq_uri = "amqp://guest:guest@127.0.0.1:5672";
    let topic = "ack-example-queue";

    // Create publisher
    println!("\n📤 Creating RabbitMQ publisher...");
    let publisher = RabbitMQPublisher::new(rabbitmq_uri).await?;
    println!("✅ Publisher created successfully");

    // Create subscriber with acknowledgment support
    println!("\n📥 Creating RabbitMQ acknowledgment subscriber...");
    let mut subscriber = RabbitMQAckSubscriber::new(rabbitmq_uri).await?;
    println!("✅ Acknowledgment subscriber created successfully");

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

    // Example 4: Error handling and retry logic
    println!("\n" + "=".repeat(50).as_str());
    println!("📋 Example 4: Error Handling and Retry Logic");
    println!("=".repeat(50));
    
    error_handling_example(&publisher, &mut subscriber, topic).await?;

    println!("\n🎉 All examples completed successfully!");
    println!("   The RabbitMQ acknowledgment system provides reliable message processing");
    println!("   with manual control over message acknowledgment and requeue behavior.");

    Ok(())
}

async fn basic_acknowledgment_example(
    publisher: &RabbitMQPublisher,
    subscriber: &mut RabbitMQAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing message for basic acknowledgment...");
    
    let message = Message::new(b"Hello from RabbitMQ with acknowledgment!".to_vec())
        .with_metadata("example", "basic_ack")
        .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339());
    
    publisher.publish(topic, vec![message.clone()]).await?;
    println!("✅ Message published: {}", String::from_utf8_lossy(&message.payload));

    println!("📥 Receiving message with acknowledgment handle...");
    let (received_message, ack_handle) = timeout(
        Duration::from_secs(10),
        subscriber.receive_with_ack()
    ).await??;

    println!("✅ Message received:");
    println!("   ID: {}", received_message.uuid);
    println!("   Payload: {}", String::from_utf8_lossy(&received_message.payload));
    println!("   Metadata: {:?}", received_message.metadata);
    
    println!("📋 Acknowledgment handle details:");
    println!("   Message ID: {}", ack_handle.message_id());
    println!("   Topic: {}", ack_handle.topic());
    println!("   Delivery Count: {}", ack_handle.delivery_count());
    println!("   Is Retry: {}", ack_handle.is_retry());
    println!("   Handle ID: {}", ack_handle.handle_id());

    println!("✅ Acknowledging message...");
    subscriber.ack(ack_handle).await?;
    println!("✅ Message acknowledged successfully");

    Ok(())
}

async fn negative_acknowledgment_example(
    publisher: &RabbitMQPublisher,
    subscriber: &mut RabbitMQAckSubscriber,
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
        Duration::from_secs(10),
        subscriber.receive_with_ack()
    ).await??;

    println!("✅ Message received: {}", String::from_utf8_lossy(&received_message.payload));

    // Simulate processing failure
    let should_process = received_message.metadata.get("should_process")
        .map(|v| v == "true")
        .unwrap_or(false);

    if !should_process {
        println!("❌ Processing failed - negatively acknowledging with requeue...");
        subscriber.nack(ack_handle, true).await?;
        println!("✅ Message negatively acknowledged and requeued");
        
        // The message is now back in the queue and could be received again
        println!("💡 Message has been requeued and is available for reprocessing");
    } else {
        println!("✅ Processing successful - acknowledging...");
        subscriber.ack(ack_handle).await?;
    }

    Ok(())
}

async fn batch_acknowledgment_example(
    publisher: &RabbitMQPublisher,
    subscriber: &mut RabbitMQAckSubscriber,
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
            Duration::from_secs(10),
            subscriber.receive_with_ack()
        ).await??;
        
        println!("✅ Received message {}: {}", 
                i + 1, 
                String::from_utf8_lossy(&received_message.payload));
        
        ack_handles.push(ack_handle);
    }

    println!("📋 Performing batch acknowledgment for {} messages...", ack_handles.len());
    subscriber.ack_batch(ack_handles).await?;
    println!("✅ Batch acknowledgment completed successfully");
    println!("💡 All messages in the batch were acknowledged with a single operation");

    Ok(())
}

async fn error_handling_example(
    publisher: &RabbitMQPublisher,
    subscriber: &mut RabbitMQAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing message for error handling demonstration...");
    
    let message = Message::new(b"Message for error handling".to_vec())
        .with_metadata("simulate_error", "true")
        .with_metadata("max_retries", "3");
    
    publisher.publish(topic, vec![message.clone()]).await?;
    println!("✅ Message published: {}", String::from_utf8_lossy(&message.payload));

    println!("📥 Receiving message and simulating processing with retries...");
    
    let mut retry_count = 0;
    let max_retries = 3;
    
    loop {
        let (received_message, ack_handle) = timeout(
            Duration::from_secs(10),
            subscriber.receive_with_ack()
        ).await??;

        retry_count += 1;
        println!("🔄 Processing attempt {} of {}", retry_count, max_retries);
        
        // Simulate processing
        let simulate_error = received_message.metadata.get("simulate_error")
            .map(|v| v == "true")
            .unwrap_or(false);

        if simulate_error && retry_count < max_retries {
            println!("❌ Processing failed (simulated) - attempt {}", retry_count);
            println!("🔄 Negatively acknowledging with requeue for retry...");
            subscriber.nack(ack_handle, true).await?;
            
            // Add delay before retry
            sleep(Duration::from_millis(500)).await;
            continue;
        } else if simulate_error && retry_count >= max_retries {
            println!("❌ Max retries exceeded - negatively acknowledging without requeue");
            println!("💀 Message will be discarded or sent to dead letter queue");
            subscriber.nack(ack_handle, false).await?;
            break;
        } else {
            println!("✅ Processing successful - acknowledging message");
            subscriber.ack(ack_handle).await?;
            break;
        }
    }

    println!("📊 Error handling completed:");
    println!("   Total attempts: {}", retry_count);
    println!("   Max retries: {}", max_retries);
    println!("💡 This demonstrates how to implement retry logic with acknowledgments");

    Ok(())
}

async fn is_rabbitmq_available() -> bool {
    match tokio::net::TcpStream::connect("127.0.0.1:5672").await {
        Ok(_) => true,
        Err(_) => false,
    }
}
