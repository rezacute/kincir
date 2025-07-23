//! Integration tests for Kafka acknowledgment handling
//!
//! These tests verify the Kafka acknowledgment implementation works correctly
//! with manual acknowledgment, negative acknowledgment, and batch operations.

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::kafka::{KafkaAckHandle, KafkaAckSubscriber, KafkaPublisher};
use kincir::{Message, Publisher};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

// Helper function to check if Kafka is available
async fn is_kafka_available() -> bool {
    match tokio::net::TcpStream::connect("127.0.0.1:9092").await {
        Ok(_) => true,
        Err(_) => false,
    }
}

// Skip test if Kafka is not available
macro_rules! skip_if_no_kafka {
    () => {
        if !is_kafka_available().await {
            println!("Skipping test: Kafka not available at 127.0.0.1:9092");
            return;
        }
    };
}

#[tokio::test]
async fn test_kafka_ack_handle_properties() {
    let handle = KafkaAckHandle::new(
        "test-message-123".to_string(),
        "test-topic".to_string(),
        SystemTime::now(),
        1,
        0,
        12345,
    );

    assert_eq!(handle.message_id(), "test-message-123");
    assert_eq!(handle.topic(), "test-topic");
    assert_eq!(handle.delivery_count(), 1);
    assert!(!handle.is_retry());
    assert_eq!(handle.partition(), 0);
    assert_eq!(handle.offset(), 12345);
    assert!(!handle.handle_id().is_empty());
}

#[tokio::test]
async fn test_kafka_ack_handle_retry_detection() {
    let handle = KafkaAckHandle::new(
        "retry-message".to_string(),
        "retry-topic".to_string(),
        SystemTime::now(),
        3,
        1,
        67890,
    );

    assert_eq!(handle.delivery_count(), 3);
    assert!(handle.is_retry());
    assert_eq!(handle.partition(), 1);
    assert_eq!(handle.offset(), 67890);
}

#[tokio::test]
async fn test_kafka_ack_subscriber_creation() {
    skip_if_no_kafka!();

    let brokers = vec!["127.0.0.1:9092".to_string()];
    let group_id = "test-ack-group".to_string();
    
    let result = KafkaAckSubscriber::new(brokers.clone(), group_id.clone()).await;
    assert!(result.is_ok(), "Failed to create Kafka ack subscriber: {:?}", result.err());

    let subscriber = result.unwrap();
    assert_eq!(subscriber.group_id(), "test-ack-group");
    assert_eq!(subscriber.brokers(), &brokers);
    assert!(!subscriber.is_subscribed().await);
    assert!(subscriber.subscribed_topics().await.is_empty());
}

#[tokio::test]
async fn test_kafka_ack_subscriber_subscription() {
    skip_if_no_kafka!();

    let brokers = vec!["127.0.0.1:9092".to_string()];
    let group_id = "test-subscription-group".to_string();
    
    let subscriber = KafkaAckSubscriber::new(brokers, group_id)
        .await
        .expect("Failed to create subscriber");

    let topic = "test-ack-subscription";
    let result = subscriber.subscribe(topic).await;
    assert!(result.is_ok(), "Failed to subscribe: {:?}", result.err());

    assert!(subscriber.is_subscribed().await);
    let topics = subscriber.subscribed_topics().await;
    assert!(topics.contains(&topic.to_string()));
}

#[tokio::test]
async fn test_kafka_manual_acknowledgment() {
    skip_if_no_kafka!();

    let topic = "test-manual-ack";
    let brokers = vec!["127.0.0.1:9092".to_string()];
    
    // Create publisher
    let publisher = KafkaPublisher::new(brokers.clone())
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = KafkaAckSubscriber::new(brokers, "test-ack-group".to_string())
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic
    subscriber.subscribe(topic).await.expect("Failed to subscribe");

    // Publish a test message
    let test_message = Message::new(b"Test acknowledgment message".to_vec());
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
    assert!(!ack_handle.is_retry());

    // Acknowledge the message
    let ack_result = subscriber.ack(ack_handle).await;
    assert!(ack_result.is_ok(), "Failed to acknowledge message: {:?}", ack_result.err());
}

#[tokio::test]
async fn test_kafka_negative_acknowledgment_with_requeue() {
    skip_if_no_kafka!();

    let topic = "test-nack-requeue";
    let brokers = vec!["127.0.0.1:9092".to_string()];
    
    // Create publisher
    let publisher = KafkaPublisher::new(brokers.clone())
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = KafkaAckSubscriber::new(brokers, "test-nack-group".to_string())
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic
    subscriber.subscribe(topic).await.expect("Failed to subscribe");

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

    // Negatively acknowledge with requeue (don't commit offset)
    let nack_result = subscriber.nack(ack_handle, true).await;
    assert!(nack_result.is_ok(), "Failed to nack message: {:?}", nack_result.err());

    // Note: In Kafka, requeue means the offset is not committed,
    // so the message will be redelivered on consumer restart or rebalance
}

#[tokio::test]
async fn test_kafka_negative_acknowledgment_without_requeue() {
    skip_if_no_kafka!();

    let topic = "test-nack-no-requeue";
    let brokers = vec!["127.0.0.1:9092".to_string()];
    
    // Create publisher
    let publisher = KafkaPublisher::new(brokers.clone())
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = KafkaAckSubscriber::new(brokers, "test-nack-discard-group".to_string())
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic
    subscriber.subscribe(topic).await.expect("Failed to subscribe");

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

    // Negatively acknowledge without requeue (commit offset to discard)
    let nack_result = subscriber.nack(ack_handle, false).await;
    assert!(nack_result.is_ok(), "Failed to nack message: {:?}", nack_result.err());
}

#[tokio::test]
async fn test_kafka_batch_acknowledgment() {
    skip_if_no_kafka!();

    let topic = "test-batch-ack";
    let brokers = vec!["127.0.0.1:9092".to_string()];
    
    // Create publisher
    let publisher = KafkaPublisher::new(brokers.clone())
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = KafkaAckSubscriber::new(brokers, "test-batch-ack-group".to_string())
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic
    subscriber.subscribe(topic).await.expect("Failed to subscribe");

    // Publish multiple test messages
    let messages = vec![
        Message::new(b"Batch message 1".to_vec()),
        Message::new(b"Batch message 2".to_vec()),
        Message::new(b"Batch message 3".to_vec()),
    ];
    
    publisher
        .publish(topic, messages.clone())
        .await
        .expect("Failed to publish messages");

    // Receive messages and collect acknowledgment handles
    let mut ack_handles = Vec::new();
    for i in 0..3 {
        let (received_message, ack_handle) = timeout(Duration::from_secs(10), subscriber.receive_with_ack())
            .await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));
        
        // Verify message content
        assert_eq!(received_message.payload, messages[i].payload);
        ack_handles.push(ack_handle);
    }

    // Batch acknowledge all messages
    let batch_ack_result = subscriber.ack_batch(ack_handles).await;
    assert!(batch_ack_result.is_ok(), "Failed to batch acknowledge: {:?}", batch_ack_result.err());
}

#[tokio::test]
async fn test_kafka_batch_negative_acknowledgment() {
    skip_if_no_kafka!();

    let topic = "test-batch-nack";
    let brokers = vec!["127.0.0.1:9092".to_string()];
    
    // Create publisher
    let publisher = KafkaPublisher::new(brokers.clone())
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = KafkaAckSubscriber::new(brokers, "test-batch-nack-group".to_string())
        .await
        .expect("Failed to create ack subscriber");

    // Subscribe to topic
    subscriber.subscribe(topic).await.expect("Failed to subscribe");

    // Publish multiple test messages
    let messages = vec![
        Message::new(b"Batch nack message 1".to_vec()),
        Message::new(b"Batch nack message 2".to_vec()),
    ];
    
    publisher
        .publish(topic, messages.clone())
        .await
        .expect("Failed to publish messages");

    // Receive messages and collect acknowledgment handles
    let mut ack_handles = Vec::new();
    for i in 0..2 {
        let (received_message, ack_handle) = timeout(Duration::from_secs(10), subscriber.receive_with_ack())
            .await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));
        
        // Verify message content
        assert_eq!(received_message.payload, messages[i].payload);
        ack_handles.push(ack_handle);
    }

    // Batch negatively acknowledge all messages without requeue (discard)
    let batch_nack_result = subscriber.nack_batch(ack_handles, false).await;
    assert!(batch_nack_result.is_ok(), "Failed to batch nack: {:?}", batch_nack_result.err());
}

#[tokio::test]
async fn test_kafka_ack_handle_uniqueness() {
    let handle1 = KafkaAckHandle::new(
        "msg-1".to_string(),
        "topic-1".to_string(),
        SystemTime::now(),
        1,
        0,
        100,
    );

    let handle2 = KafkaAckHandle::new(
        "msg-2".to_string(),
        "topic-2".to_string(),
        SystemTime::now(),
        1,
        1,
        200,
    );

    // Each handle should have a unique handle ID
    assert_ne!(handle1.handle_id(), handle2.handle_id());
    
    // But different properties
    assert_ne!(handle1.message_id(), handle2.message_id());
    assert_ne!(handle1.partition(), handle2.partition());
    assert_ne!(handle1.offset(), handle2.offset());
}

#[tokio::test]
async fn test_kafka_offset_tracking() {
    let handle1 = KafkaAckHandle::new(
        "msg-1".to_string(),
        "test-topic".to_string(),
        SystemTime::now(),
        1,
        0,
        100,
    );

    let handle2 = KafkaAckHandle::new(
        "msg-2".to_string(),
        "test-topic".to_string(),
        SystemTime::now(),
        1,
        0,
        101,
    );

    // Same topic and partition, different offsets
    assert_eq!(handle1.topic(), handle2.topic());
    assert_eq!(handle1.partition(), handle2.partition());
    assert_ne!(handle1.offset(), handle2.offset());
    assert!(handle2.offset() > handle1.offset());
}

#[cfg(feature = "logging")]
#[tokio::test]
async fn test_kafka_ack_subscriber_with_logger() {
    skip_if_no_kafka!();

    use kincir::logging::StdLogger;
    use std::sync::Arc;

    let logger = Arc::new(StdLogger::new(true, false)); // Enable info, disable debug
    let brokers = vec!["127.0.0.1:9092".to_string()];
    
    let subscriber = KafkaAckSubscriber::new(brokers, "test-logger-group".to_string())
        .await
        .expect("Failed to create subscriber")
        .with_logger(logger);

    let topic = "test-ack-with-logger";
    let result = subscriber.subscribe(topic).await;
    assert!(result.is_ok(), "Failed to subscribe with logger: {:?}", result.err());
}

#[tokio::test]
async fn test_kafka_consumer_group_properties() {
    skip_if_no_kafka!();

    let brokers = vec!["127.0.0.1:9092".to_string()];
    let group_id = "test-consumer-group-123".to_string();
    
    let subscriber = KafkaAckSubscriber::new(brokers.clone(), group_id.clone())
        .await
        .expect("Failed to create subscriber");

    assert_eq!(subscriber.group_id(), "test-consumer-group-123");
    assert_eq!(subscriber.brokers(), &brokers);
}

// Note: These tests require a running Kafka instance at localhost:9092
// To run with Docker: docker run -d --name kafka -p 9092:9092 apache/kafka:latest
