//! Integration tests for RabbitMQ acknowledgment handling
//!
//! These tests verify the RabbitMQ acknowledgment implementation works correctly
//! with manual acknowledgment, negative acknowledgment, and batch operations.

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::rabbitmq::{RabbitMQAckHandle, RabbitMQAckSubscriber, RabbitMQPublisher};
use kincir::{Message, Publisher};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

// Helper function to check if RabbitMQ is available
async fn is_rabbitmq_available() -> bool {
    match tokio::net::TcpStream::connect("127.0.0.1:5672").await {
        Ok(_) => true,
        Err(_) => false,
    }
}

// Skip test if RabbitMQ is not available
macro_rules! skip_if_no_rabbitmq {
    () => {
        if !is_rabbitmq_available().await {
            println!("Skipping test: RabbitMQ not available at 127.0.0.1:5672");
            return;
        }
    };
}

#[tokio::test]
async fn test_rabbitmq_ack_handle_properties() {
    let handle = RabbitMQAckHandle::new(
        "test-message-123".to_string(),
        "test-queue".to_string(),
        SystemTime::now(),
        1,
        42,
    );

    assert_eq!(handle.message_id(), "test-message-123");
    assert_eq!(handle.topic(), "test-queue");
    assert_eq!(handle.delivery_count(), 1);
    assert!(!handle.is_retry());
    assert_eq!(handle.delivery_tag(), 42);
    assert!(!handle.handle_id().is_empty());
}

#[tokio::test]
async fn test_rabbitmq_ack_handle_retry_detection() {
    let handle = RabbitMQAckHandle::new(
        "retry-message".to_string(),
        "retry-queue".to_string(),
        SystemTime::now(),
        3,
        100,
    );

    assert_eq!(handle.delivery_count(), 3);
    assert!(handle.is_retry());
}

#[tokio::test]
async fn test_rabbitmq_ack_subscriber_creation() {
    skip_if_no_rabbitmq!();

    let result = RabbitMQAckSubscriber::new("amqp://guest:guest@127.0.0.1:5672").await;
    assert!(result.is_ok(), "Failed to create RabbitMQ ack subscriber: {:?}", result.err());

    let subscriber = result.unwrap();
    assert!(!subscriber.is_subscribed().await);
    assert!(subscriber.subscribed_topic().await.is_none());
}

#[tokio::test]
async fn test_rabbitmq_ack_subscriber_subscription() {
    skip_if_no_rabbitmq!();

    let subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@127.0.0.1:5672")
        .await
        .expect("Failed to create subscriber");

    let topic = "test-ack-subscription";
    let result = subscriber.subscribe(topic).await;
    assert!(result.is_ok(), "Failed to subscribe: {:?}", result.err());

    assert!(subscriber.is_subscribed().await);
    assert_eq!(subscriber.subscribed_topic().await, Some(topic.to_string()));
}

#[tokio::test]
async fn test_rabbitmq_manual_acknowledgment() {
    skip_if_no_rabbitmq!();

    let topic = "test-manual-ack";
    
    // Create publisher
    let publisher = RabbitMQPublisher::new("amqp://guest:guest@127.0.0.1:5672")
        .await
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@127.0.0.1:5672")
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
    let receive_result = timeout(Duration::from_secs(5), subscriber.receive_with_ack()).await;
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
async fn test_rabbitmq_negative_acknowledgment_with_requeue() {
    skip_if_no_rabbitmq!();

    let topic = "test-nack-requeue";
    
    // Create publisher
    let publisher = RabbitMQPublisher::new("amqp://guest:guest@127.0.0.1:5672")
        .await
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@127.0.0.1:5672")
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
    let (received_message, ack_handle) = timeout(Duration::from_secs(5), subscriber.receive_with_ack())
        .await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");
    
    // Verify message content
    assert_eq!(received_message.payload, test_message.payload);

    // Negatively acknowledge with requeue
    let nack_result = subscriber.nack(ack_handle, true).await;
    assert!(nack_result.is_ok(), "Failed to nack message: {:?}", nack_result.err());

    // The message should be requeued and available again
    // Note: In a real scenario, you might want to add a small delay here
    // to allow RabbitMQ to requeue the message
}

#[tokio::test]
async fn test_rabbitmq_negative_acknowledgment_without_requeue() {
    skip_if_no_rabbitmq!();

    let topic = "test-nack-no-requeue";
    
    // Create publisher
    let publisher = RabbitMQPublisher::new("amqp://guest:guest@127.0.0.1:5672")
        .await
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@127.0.0.1:5672")
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
    let (received_message, ack_handle) = timeout(Duration::from_secs(5), subscriber.receive_with_ack())
        .await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");
    
    // Verify message content
    assert_eq!(received_message.payload, test_message.payload);

    // Negatively acknowledge without requeue (message will be discarded)
    let nack_result = subscriber.nack(ack_handle, false).await;
    assert!(nack_result.is_ok(), "Failed to nack message: {:?}", nack_result.err());
}

#[tokio::test]
async fn test_rabbitmq_batch_acknowledgment() {
    skip_if_no_rabbitmq!();

    let topic = "test-batch-ack";
    
    // Create publisher
    let publisher = RabbitMQPublisher::new("amqp://guest:guest@127.0.0.1:5672")
        .await
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@127.0.0.1:5672")
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
        let (received_message, ack_handle) = timeout(Duration::from_secs(5), subscriber.receive_with_ack())
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
async fn test_rabbitmq_batch_negative_acknowledgment() {
    skip_if_no_rabbitmq!();

    let topic = "test-batch-nack";
    
    // Create publisher
    let publisher = RabbitMQPublisher::new("amqp://guest:guest@127.0.0.1:5672")
        .await
        .expect("Failed to create publisher");

    // Create subscriber with acknowledgment support
    let mut subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@127.0.0.1:5672")
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
        let (received_message, ack_handle) = timeout(Duration::from_secs(5), subscriber.receive_with_ack())
            .await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));
        
        // Verify message content
        assert_eq!(received_message.payload, messages[i].payload);
        ack_handles.push(ack_handle);
    }

    // Batch negatively acknowledge all messages without requeue
    let batch_nack_result = subscriber.nack_batch(ack_handles, false).await;
    assert!(batch_nack_result.is_ok(), "Failed to batch nack: {:?}", batch_nack_result.err());
}

#[tokio::test]
async fn test_rabbitmq_ack_handle_uniqueness() {
    let handle1 = RabbitMQAckHandle::new(
        "msg-1".to_string(),
        "topic-1".to_string(),
        SystemTime::now(),
        1,
        100,
    );

    let handle2 = RabbitMQAckHandle::new(
        "msg-2".to_string(),
        "topic-2".to_string(),
        SystemTime::now(),
        1,
        200,
    );

    // Each handle should have a unique handle ID
    assert_ne!(handle1.handle_id(), handle2.handle_id());
    
    // But different properties
    assert_ne!(handle1.message_id(), handle2.message_id());
    assert_ne!(handle1.delivery_tag(), handle2.delivery_tag());
}

#[cfg(feature = "logging")]
#[tokio::test]
async fn test_rabbitmq_ack_subscriber_with_logger() {
    skip_if_no_rabbitmq!();

    use kincir::logging::StdLogger;
    use std::sync::Arc;

    let logger = Arc::new(StdLogger::new(true, false)); // Enable info, disable debug
    
    let subscriber = RabbitMQAckSubscriber::new("amqp://guest:guest@127.0.0.1:5672")
        .await
        .expect("Failed to create subscriber")
        .with_logger(logger);

    let topic = "test-ack-with-logger";
    let result = subscriber.subscribe(topic).await;
    assert!(result.is_ok(), "Failed to subscribe with logger: {:?}", result.err());
}
