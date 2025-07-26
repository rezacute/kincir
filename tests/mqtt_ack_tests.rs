//! Integration tests for MQTT acknowledgment handling
//!
//! These tests verify the MQTT acknowledgment implementation works correctly
//! with different QoS levels, manual acknowledgment, and MQTT-specific features.

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::mqtt::{MQTTAckHandle, MQTTAckSubscriber, MQTTPublisher, QoS};
use kincir::{Message, Publisher};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

// Helper function to check if MQTT broker is available
async fn is_mqtt_available() -> bool {
    match tokio::net::TcpStream::connect("127.0.0.1:1883").await {
        Ok(_) => true,
        Err(_) => false,
    }
}

// Skip test if MQTT broker is not available
macro_rules! skip_if_no_mqtt {
    () => {
        if !is_mqtt_available().await {
            println!("Skipping test: MQTT broker not available at 127.0.0.1:1883");
            return;
        }
    };
}

#[tokio::test]
async fn test_mqtt_ack_handle_properties() {
    let handle = MQTTAckHandle::new(
        "test-message-123".to_string(),
        "test/topic".to_string(),
        SystemTime::now(),
        1,
        QoS::AtLeastOnce,
        Some(42),
    );

    assert_eq!(handle.message_id(), "test-message-123");
    assert_eq!(handle.topic(), "test/topic");
    assert_eq!(handle.delivery_count(), 1);
    assert!(!handle.is_retry());
    assert_eq!(handle.qos(), QoS::AtLeastOnce);
    assert_eq!(handle.packet_id(), Some(42));
    assert!(handle.requires_ack());
    assert!(!handle.handle_id().is_empty());
}

#[tokio::test]
async fn test_mqtt_ack_handle_qos_levels() {
    // QoS 0 - Fire and forget
    let handle_qos0 = MQTTAckHandle::new(
        "msg-qos0".to_string(),
        "test/topic".to_string(),
        SystemTime::now(),
        1,
        QoS::AtMostOnce,
        None,
    );
    
    assert_eq!(handle_qos0.qos(), QoS::AtMostOnce);
    assert!(!handle_qos0.requires_ack());
    assert_eq!(handle_qos0.packet_id(), None);

    // QoS 1 - At least once
    let handle_qos1 = MQTTAckHandle::new(
        "msg-qos1".to_string(),
        "test/topic".to_string(),
        SystemTime::now(),
        1,
        QoS::AtLeastOnce,
        Some(123),
    );
    
    assert_eq!(handle_qos1.qos(), QoS::AtLeastOnce);
    assert!(handle_qos1.requires_ack());
    assert_eq!(handle_qos1.packet_id(), Some(123));

    // QoS 2 - Exactly once
    let handle_qos2 = MQTTAckHandle::new(
        "msg-qos2".to_string(),
        "test/topic".to_string(),
        SystemTime::now(),
        1,
        QoS::ExactlyOnce,
        Some(456),
    );
    
    assert_eq!(handle_qos2.qos(), QoS::ExactlyOnce);
    assert!(handle_qos2.requires_ack());
    assert_eq!(handle_qos2.packet_id(), Some(456));
}

#[tokio::test]
async fn test_mqtt_ack_handle_retry_detection() {
    let handle = MQTTAckHandle::new(
        "retry-message".to_string(),
        "retry/topic".to_string(),
        SystemTime::now(),
        3,
        QoS::AtLeastOnce,
        Some(789),
    );

    assert_eq!(handle.delivery_count(), 3);
    assert!(handle.is_retry());
}

#[tokio::test]
async fn test_mqtt_ack_subscriber_creation() {
    skip_if_no_mqtt!();

    let broker_url = "127.0.0.1";
    let client_id = Some("test-ack-client".to_string());
    
    let result = MQTTAckSubscriber::new(broker_url, client_id).await;
    assert!(result.is_ok(), "Failed to create MQTT ack subscriber: {:?}", result.err());

    let subscriber = result.unwrap();
    assert!(!subscriber.is_subscribed().await);
    assert!(subscriber.subscribed_topics().await.is_empty());
}

#[tokio::test]
async fn test_mqtt_ack_subscriber_subscription() {
    skip_if_no_mqtt!();

    let broker_url = "127.0.0.1";
    let client_id = Some("test-subscription-client".to_string());
    
    let subscriber = MQTTAckSubscriber::new(broker_url, client_id)
        .await
        .expect("Failed to create subscriber");

    let topic = "test/ack/subscription";
    let result = subscriber.subscribe(topic).await;
    assert!(result.is_ok(), "Failed to subscribe: {:?}", result.err());

    assert!(subscriber.is_subscribed().await);
    let topics = subscriber.subscribed_topics().await;
    assert!(topics.contains(&topic.to_string()));
}

#[tokio::test]
async fn test_mqtt_subscription_with_qos() {
    skip_if_no_mqtt!();

    let broker_url = "127.0.0.1";
    let client_id = Some("test-qos-client".to_string());
    
    let subscriber = MQTTAckSubscriber::new(broker_url, client_id)
        .await
        .expect("Failed to create subscriber");

    // Test different QoS levels
    let topic_qos0 = "test/qos0";
    let topic_qos1 = "test/qos1";
    let topic_qos2 = "test/qos2";

    subscriber.subscribe_with_qos(topic_qos0, QoS::AtMostOnce).await
        .expect("Failed to subscribe with QoS 0");
    
    subscriber.subscribe_with_qos(topic_qos1, QoS::AtLeastOnce).await
        .expect("Failed to subscribe with QoS 1");
    
    subscriber.subscribe_with_qos(topic_qos2, QoS::ExactlyOnce).await
        .expect("Failed to subscribe with QoS 2");

    let topics = subscriber.subscribed_topics().await;
    assert!(topics.contains(&topic_qos0.to_string()));
    assert!(topics.contains(&topic_qos1.to_string()));
    assert!(topics.contains(&topic_qos2.to_string()));
}

#[tokio::test]
async fn test_mqtt_manual_acknowledgment_qos1() {
    skip_if_no_mqtt!();

    let topic = "test/manual/ack/qos1";
    let broker_url = "127.0.0.1";
    
    // Create publisher
    let publisher = MQTTPublisher::new(broker_url, topic)
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = MQTTAckSubscriber::new(broker_url, Some("test-ack-qos1".to_string()))
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic with QoS 1
    subscriber.subscribe_with_qos(topic, QoS::AtLeastOnce).await
        .expect("Failed to subscribe");

    // Give some time for subscription to be established
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish a test message
    let test_message = Message::new(b"Test QoS 1 acknowledgment message".to_vec());
    publisher
        .publish(topic, vec![test_message.clone()])
        .await
        .expect("Failed to publish message");

    // Receive message with acknowledgment handle
    let receive_result = timeout(Duration::from_secs(10), subscriber.receive_with_ack()).await;
    assert!(receive_result.is_ok(), "Timeout waiting for message");

    let (received_message, ack_handle) = receive_result.unwrap().expect("Failed to receive message");
    
    // Verify message content
    assert_eq!(received_message.payload, test_message.payload);
    assert_eq!(ack_handle.topic(), topic);
    assert_eq!(ack_handle.qos(), QoS::AtLeastOnce);
    assert!(ack_handle.requires_ack());
    assert!(!ack_handle.is_retry());

    // Acknowledge the message
    let ack_result = subscriber.ack(ack_handle).await;
    assert!(ack_result.is_ok(), "Failed to acknowledge message: {:?}", ack_result.err());
}

#[tokio::test]
async fn test_mqtt_negative_acknowledgment_with_requeue() {
    skip_if_no_mqtt!();

    let topic = "test/nack/requeue";
    let broker_url = "127.0.0.1";
    
    // Create publisher
    let publisher = MQTTPublisher::new(broker_url, topic)
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = MQTTAckSubscriber::new(broker_url, Some("test-nack-requeue".to_string()))
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic with QoS 1
    subscriber.subscribe_with_qos(topic, QoS::AtLeastOnce).await
        .expect("Failed to subscribe");

    // Give some time for subscription to be established
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish a test message
    let test_message = Message::new(b"Test nack with requeue".to_vec());
    publisher
        .publish(topic, vec![test_message.clone()])
        .await
        .expect("Failed to publish message");

    // Receive message with acknowledgment handle
    let (received_message, ack_handle) = timeout(Duration::from_secs(10), subscriber.receive_with_ack())
        .await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");
    
    // Verify message content
    assert_eq!(received_message.payload, test_message.payload);
    assert_eq!(ack_handle.qos(), QoS::AtLeastOnce);

    // Negatively acknowledge with requeue
    let nack_result = subscriber.nack(ack_handle, true).await;
    assert!(nack_result.is_ok(), "Failed to nack message: {:?}", nack_result.err());

    // Note: In MQTT, requeue behavior depends on broker implementation
    // and connection persistence. The message may be redelivered on reconnection.
}

#[tokio::test]
async fn test_mqtt_negative_acknowledgment_without_requeue() {
    skip_if_no_mqtt!();

    let topic = "test/nack/no-requeue";
    let broker_url = "127.0.0.1";
    
    // Create publisher
    let publisher = MQTTPublisher::new(broker_url, topic)
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = MQTTAckSubscriber::new(broker_url, Some("test-nack-discard".to_string()))
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic with QoS 1
    subscriber.subscribe_with_qos(topic, QoS::AtLeastOnce).await
        .expect("Failed to subscribe");

    // Give some time for subscription to be established
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish a test message
    let test_message = Message::new(b"Test nack without requeue".to_vec());
    publisher
        .publish(topic, vec![test_message.clone()])
        .await
        .expect("Failed to publish message");

    // Receive message with acknowledgment handle
    let (received_message, ack_handle) = timeout(Duration::from_secs(10), subscriber.receive_with_ack())
        .await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");
    
    // Verify message content
    assert_eq!(received_message.payload, test_message.payload);

    // Negatively acknowledge without requeue (discard)
    let nack_result = subscriber.nack(ack_handle, false).await;
    assert!(nack_result.is_ok(), "Failed to nack message: {:?}", nack_result.err());
}

#[tokio::test]
async fn test_mqtt_batch_acknowledgment() {
    skip_if_no_mqtt!();

    let topic = "test/batch/ack";
    let broker_url = "127.0.0.1";
    
    // Create publisher
    let publisher = MQTTPublisher::new(broker_url, topic)
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = MQTTAckSubscriber::new(broker_url, Some("test-batch-ack".to_string()))
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic with QoS 1
    subscriber.subscribe_with_qos(topic, QoS::AtLeastOnce).await
        .expect("Failed to subscribe");

    // Give some time for subscription to be established
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish multiple test messages
    let messages = vec![
        Message::new(b"Batch message 1".to_vec()),
        Message::new(b"Batch message 2".to_vec()),
        Message::new(b"Batch message 3".to_vec()),
    ];
    
    for message in &messages {
        publisher
            .publish(topic, vec![message.clone()])
            .await
            .expect("Failed to publish message");
    }

    // Receive messages and collect acknowledgment handles
    let mut ack_handles = Vec::new();
    for i in 0..3 {
        let (received_message, ack_handle) = timeout(Duration::from_secs(10), subscriber.receive_with_ack())
            .await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));
        
        // Verify message content (order may vary)
        assert!(messages.iter().any(|m| m.payload == received_message.payload));
        assert_eq!(ack_handle.qos(), QoS::AtLeastOnce);
        ack_handles.push(ack_handle);
    }

    // Batch acknowledge all messages
    let batch_ack_result = subscriber.ack_batch(ack_handles).await;
    assert!(batch_ack_result.is_ok(), "Failed to batch acknowledge: {:?}", batch_ack_result.err());
}

#[tokio::test]
async fn test_mqtt_batch_negative_acknowledgment() {
    skip_if_no_mqtt!();

    let topic = "test/batch/nack";
    let broker_url = "127.0.0.1";
    
    // Create publisher
    let publisher = MQTTPublisher::new(broker_url, topic)
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = MQTTAckSubscriber::new(broker_url, Some("test-batch-nack".to_string()))
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic with QoS 1
    subscriber.subscribe_with_qos(topic, QoS::AtLeastOnce).await
        .expect("Failed to subscribe");

    // Give some time for subscription to be established
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish multiple test messages
    let messages = vec![
        Message::new(b"Batch nack message 1".to_vec()),
        Message::new(b"Batch nack message 2".to_vec()),
    ];
    
    for message in &messages {
        publisher
            .publish(topic, vec![message.clone()])
            .await
            .expect("Failed to publish message");
    }

    // Receive messages and collect acknowledgment handles
    let mut ack_handles = Vec::new();
    for i in 0..2 {
        let (received_message, ack_handle) = timeout(Duration::from_secs(10), subscriber.receive_with_ack())
            .await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));
        
        // Verify message content (order may vary)
        assert!(messages.iter().any(|m| m.payload == received_message.payload));
        ack_handles.push(ack_handle);
    }

    // Batch negatively acknowledge all messages without requeue
    let batch_nack_result = subscriber.nack_batch(ack_handles, false).await;
    assert!(batch_nack_result.is_ok(), "Failed to batch nack: {:?}", batch_nack_result.err());
}

#[tokio::test]
async fn test_mqtt_qos0_no_acknowledgment_required() {
    skip_if_no_mqtt!();

    let topic = "test/qos0/no-ack";
    let broker_url = "127.0.0.1";
    
    // Create publisher
    let publisher = MQTTPublisher::new(broker_url, topic)
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = MQTTAckSubscriber::new(broker_url, Some("test-qos0-no-ack".to_string()))
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic with QoS 0
    subscriber.subscribe_with_qos(topic, QoS::AtMostOnce).await
        .expect("Failed to subscribe");

    // Give some time for subscription to be established
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish a test message
    let test_message = Message::new(b"Test QoS 0 message".to_vec());
    publisher
        .publish(topic, vec![test_message.clone()])
        .await
        .expect("Failed to publish message");

    // Receive message with acknowledgment handle
    let (received_message, ack_handle) = timeout(Duration::from_secs(10), subscriber.receive_with_ack())
        .await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");
    
    // Verify message content and QoS
    assert_eq!(received_message.payload, test_message.payload);
    assert_eq!(ack_handle.qos(), QoS::AtMostOnce);
    assert!(!ack_handle.requires_ack());
    assert_eq!(ack_handle.packet_id(), None);

    // Acknowledge the message (should be no-op for QoS 0)
    let ack_result = subscriber.ack(ack_handle).await;
    assert!(ack_result.is_ok(), "Failed to acknowledge QoS 0 message: {:?}", ack_result.err());
}

#[tokio::test]
async fn test_mqtt_ack_handle_uniqueness() {
    let handle1 = MQTTAckHandle::new(
        "msg-1".to_string(),
        "topic/1".to_string(),
        SystemTime::now(),
        1,
        QoS::AtLeastOnce,
        Some(100),
    );

    let handle2 = MQTTAckHandle::new(
        "msg-2".to_string(),
        "topic/2".to_string(),
        SystemTime::now(),
        1,
        QoS::AtLeastOnce,
        Some(200),
    );

    // Each handle should have a unique handle ID
    assert_ne!(handle1.handle_id(), handle2.handle_id());
    
    // But different properties
    assert_ne!(handle1.message_id(), handle2.message_id());
    assert_ne!(handle1.topic(), handle2.topic());
    assert_ne!(handle1.packet_id(), handle2.packet_id());
}

#[cfg(feature = "logging")]
#[tokio::test]
async fn test_mqtt_ack_subscriber_with_logger() {
    skip_if_no_mqtt!();

    use kincir::logging::StdLogger;
    use std::sync::Arc;

    let logger = Arc::new(StdLogger::new(true, false)); // Enable info, disable debug
    let broker_url = "127.0.0.1";
    
    let subscriber = MQTTAckSubscriber::new(broker_url, Some("test-logger-client".to_string()))
        .await
        .expect("Failed to create subscriber")
        .with_logger(logger);

    let topic = "test/ack/with-logger";
    let result = subscriber.subscribe(topic).await;
    assert!(result.is_ok(), "Failed to subscribe with logger: {:?}", result.err());
}

// Note: These tests require a running MQTT broker at localhost:1883
// To run with Docker: docker run -d --name mosquitto -p 1883:1883 eclipse-mosquitto:latest
