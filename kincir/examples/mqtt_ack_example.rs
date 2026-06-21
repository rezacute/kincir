//! MQTT Acknowledgment Example
//!
//! This example demonstrates how to use MQTT with manual acknowledgment handling.
//! It shows publishing messages, receiving them with acknowledgment handles, and
//! performing manual acknowledgment operations with different QoS levels.
//!
//! Prerequisites:
//! - MQTT broker running on localhost:1883 (e.g., Mosquitto)
//!
//! Run with: cargo run --example mqtt_ack_example

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::mqtt::{MQTTAckSubscriber, MQTTPublisher, QoS};
use kincir::{Message, Publisher};
use std::time::Duration;
use tokio::time::{sleep, timeout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 MQTT Acknowledgment Example");
    println!("===============================");

    // Check if MQTT broker is available
    if !is_mqtt_available().await {
        eprintln!("❌ MQTT broker is not available at localhost:1883");
        eprintln!("   Please start an MQTT broker and try again.");
        eprintln!("   Docker: docker run -d --name mosquitto -p 1883:1883 eclipse-mosquitto:latest");
        return Ok(());
    }

    println!("✅ MQTT broker connection available");

    // Configuration
    let broker_url = "127.0.0.1";
    let base_topic = "kincir/ack/example";

    // Create publisher
    println!("\n📤 Creating MQTT publisher...");
    let publisher = MQTTPublisher::new(broker_url, base_topic)?;
    println!("✅ Publisher created successfully");

    // Create subscriber with acknowledgment support
    println!("\n📥 Creating MQTT acknowledgment subscriber...");
    let mut subscriber = MQTTAckSubscriber::new(broker_url, Some("ack-example-client".to_string())).await?;
    println!("✅ Acknowledgment subscriber created successfully");

    // Example 1: QoS 0 - Fire and forget (no acknowledgment needed)
    println!("\n{}", "=".repeat(60));
    println!("📋 Example 1: QoS 0 - Fire and Forget (No Acknowledgment)");
    println!("{}", "=".repeat(60));
    
    qos0_example(&publisher, &mut subscriber, &format!("{}/qos0", base_topic)).await?;

    // Example 2: QoS 1 - At least once (acknowledgment required)
    println!("\n{}", "=".repeat(60));
    println!("📋 Example 2: QoS 1 - At Least Once (Acknowledgment Required)");
    println!("{}", "=".repeat(60));
    
    qos1_example(&publisher, &mut subscriber, &format!("{}/qos1", base_topic)).await?;

    // Example 3: QoS 2 - Exactly once (acknowledgment required)
    println!("\n{}", "=".repeat(60));
    println!("📋 Example 3: QoS 2 - Exactly Once (Acknowledgment Required)");
    println!("{}", "=".repeat(60));
    
    qos2_example(&publisher, &mut subscriber, &format!("{}/qos2", base_topic)).await?;

    // Example 4: Negative acknowledgment and requeue behavior
    println!("\n{}", "=".repeat(60));
    println!("📋 Example 4: Negative Acknowledgment and Requeue Behavior");
    println!("{}", "=".repeat(60));
    
    negative_acknowledgment_example(&publisher, &mut subscriber, &format!("{}/nack", base_topic)).await?;

    // Example 5: Batch operations
    println!("\n{}", "=".repeat(60));
    println!("📋 Example 5: Batch Acknowledgment Operations");
    println!("{}", "=".repeat(60));
    
    batch_operations_example(&publisher, &mut subscriber, &format!("{}/batch", base_topic)).await?;

    println!("\n🎉 All examples completed successfully!");
    println!("   The MQTT acknowledgment system provides reliable message processing");
    println!("   with QoS-aware acknowledgment control and MQTT-specific semantics.");

    Ok(())
}

async fn qos0_example(
    publisher: &MQTTPublisher,
    subscriber: &mut MQTTAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing QoS 0 message (fire and forget)...");
    
    // Subscribe with QoS 0
    subscriber.subscribe_with_qos(topic, QoS::AtMostOnce).await?;
    println!("✅ Subscribed to {} with QoS 0", topic);
    
    // Give time for subscription to be established
    sleep(Duration::from_millis(100)).await;
    
    let message = Message::new(b"Hello from MQTT QoS 0!".to_vec())
        .with_metadata("qos", "0")
        .with_metadata("timestamp", chrono::Utc::now().to_rfc3339());
    
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
    println!("   QoS: {:?}", ack_handle.qos());
    println!("   Packet ID: {:?}", ack_handle.packet_id());
    println!("   Requires Ack: {}", ack_handle.requires_ack());
    println!("   Handle ID: {}", ack_handle.handle_id());

    println!("✅ Acknowledging message (no-op for QoS 0)...");
    subscriber.ack(ack_handle).await?;
    println!("✅ Message acknowledged (QoS 0 requires no actual acknowledgment)");

    Ok(())
}

async fn qos1_example(
    publisher: &MQTTPublisher,
    subscriber: &mut MQTTAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing QoS 1 message (at least once)...");
    
    // Subscribe with QoS 1
    subscriber.subscribe_with_qos(topic, QoS::AtLeastOnce).await?;
    println!("✅ Subscribed to {} with QoS 1", topic);
    
    // Give time for subscription to be established
    sleep(Duration::from_millis(100)).await;
    
    let message = Message::new(b"Hello from MQTT QoS 1!".to_vec())
        .with_metadata("qos", "1")
        .with_metadata("delivery_guarantee", "at_least_once");
    
    publisher.publish(topic, vec![message.clone()]).await?;
    println!("✅ Message published: {}", String::from_utf8_lossy(&message.payload));

    println!("📥 Receiving message with acknowledgment handle...");
    let (received_message, ack_handle) = timeout(
        Duration::from_secs(10),
        subscriber.receive_with_ack()
    ).await??;

    println!("✅ Message received:");
    println!("   Payload: {}", String::from_utf8_lossy(&received_message.payload));
    
    println!("📋 QoS 1 acknowledgment details:");
    println!("   QoS: {:?}", ack_handle.qos());
    println!("   Packet ID: {:?}", ack_handle.packet_id());
    println!("   Requires Ack: {}", ack_handle.requires_ack());

    println!("✅ Acknowledging QoS 1 message...");
    subscriber.ack(ack_handle).await?;
    println!("✅ Message acknowledged - PUBACK sent to broker");
    println!("💡 QoS 1 ensures at-least-once delivery with acknowledgment");

    Ok(())
}

async fn qos2_example(
    publisher: &MQTTPublisher,
    subscriber: &mut MQTTAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing QoS 2 message (exactly once)...");
    
    // Subscribe with QoS 2
    subscriber.subscribe_with_qos(topic, QoS::ExactlyOnce).await?;
    println!("✅ Subscribed to {} with QoS 2", topic);
    
    // Give time for subscription to be established
    sleep(Duration::from_millis(100)).await;
    
    let message = Message::new(b"Hello from MQTT QoS 2!".to_vec())
        .with_metadata("qos", "2")
        .with_metadata("delivery_guarantee", "exactly_once");
    
    publisher.publish(topic, vec![message.clone()]).await?;
    println!("✅ Message published: {}", String::from_utf8_lossy(&message.payload));

    println!("📥 Receiving message with acknowledgment handle...");
    let (received_message, ack_handle) = timeout(
        Duration::from_secs(10),
        subscriber.receive_with_ack()
    ).await??;

    println!("✅ Message received:");
    println!("   Payload: {}", String::from_utf8_lossy(&received_message.payload));
    
    println!("📋 QoS 2 acknowledgment details:");
    println!("   QoS: {:?}", ack_handle.qos());
    println!("   Packet ID: {:?}", ack_handle.packet_id());
    println!("   Requires Ack: {}", ack_handle.requires_ack());

    println!("✅ Acknowledging QoS 2 message...");
    subscriber.ack(ack_handle).await?;
    println!("✅ Message acknowledged - PUBREC/PUBREL/PUBCOMP handshake completed");
    println!("💡 QoS 2 ensures exactly-once delivery with 4-way handshake");

    Ok(())
}

async fn negative_acknowledgment_example(
    publisher: &MQTTPublisher,
    subscriber: &mut MQTTAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing message for negative acknowledgment demonstration...");
    
    // Subscribe with QoS 1
    subscriber.subscribe_with_qos(topic, QoS::AtLeastOnce).await?;
    println!("✅ Subscribed to {} with QoS 1", topic);
    
    // Give time for subscription to be established
    sleep(Duration::from_millis(100)).await;
    
    let message = Message::new(b"Message for nack demonstration".to_vec())
        .with_metadata("should_process", "false")
        .with_metadata("error_simulation", "true");
    
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
        println!("❌ Processing failed - demonstrating negative acknowledgment...");
        
        println!("🔄 Option 1: Negative acknowledgment with requeue (don't acknowledge)");
        println!("   This will cause redelivery on reconnection for QoS > 0");
        subscriber.nack(ack_handle.clone(), true).await?;
        println!("✅ Message negatively acknowledged with requeue");
        
        println!("💡 In MQTT, 'requeue' means we don't send acknowledgment");
        println!("   The broker will redeliver on reconnection or session resumption");
        
        // For demonstration, let's also show discard behavior
        println!("\n🗑️  Option 2: Negative acknowledgment without requeue (discard)");
        println!("   This would acknowledge the message to prevent redelivery");
        // Note: We already used the handle above, so this is just for demonstration
        println!("   subscriber.nack(handle, false) would acknowledge and discard");
    } else {
        println!("✅ Processing successful - acknowledging...");
        subscriber.ack(ack_handle).await?;
    }

    Ok(())
}

async fn batch_operations_example(
    publisher: &MQTTPublisher,
    subscriber: &mut MQTTAckSubscriber,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Publishing batch of messages...");
    
    // Subscribe with QoS 1
    subscriber.subscribe_with_qos(topic, QoS::AtLeastOnce).await?;
    println!("✅ Subscribed to {} with QoS 1", topic);
    
    // Give time for subscription to be established
    sleep(Duration::from_millis(100)).await;
    
    let messages = [Message::new(b"Batch message 1".to_vec()).with_metadata("batch_id", "1"),
        Message::new(b"Batch message 2".to_vec()).with_metadata("batch_id", "2"),
        Message::new(b"Batch message 3".to_vec()).with_metadata("batch_id", "3")];
    
    // Publish messages individually (MQTT doesn't have native batch publish)
    for (i, message) in messages.iter().enumerate() {
        publisher.publish(topic, vec![message.clone()]).await?;
        println!("✅ Published message {}: {}", i + 1, String::from_utf8_lossy(&message.payload));
    }

    println!("📥 Receiving messages and collecting acknowledgment handles...");
    let mut ack_handles = Vec::new();
    
    for i in 0..messages.len() {
        let (received_message, ack_handle) = timeout(
            Duration::from_secs(10),
            subscriber.receive_with_ack()
        ).await??;
        
        println!("✅ Received message {}: {} (QoS: {:?}, Packet ID: {:?})", 
                i + 1, 
                String::from_utf8_lossy(&received_message.payload),
                ack_handle.qos(),
                ack_handle.packet_id());
        
        ack_handles.push(ack_handle);
    }

    println!("📋 Performing batch acknowledgment for {} messages...", ack_handles.len());
    println!("   Note: MQTT doesn't have native batch ack, so we process individually");
    
    subscriber.ack_batch(ack_handles).await?;
    println!("✅ Batch acknowledgment completed successfully");
    println!("💡 Each message was individually acknowledged to the MQTT broker");

    Ok(())
}

async fn is_mqtt_available() -> bool {
    (tokio::net::TcpStream::connect("127.0.0.1:1883").await).is_ok()
}
