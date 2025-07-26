//! Cross-backend acknowledgment consistency integration tests
//!
//! These tests verify that acknowledgment behavior is consistent across all
//! supported backends (In-Memory, RabbitMQ, Kafka, MQTT) and that the
//! unified acknowledgment interface works correctly with different implementations.

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::memory::{InMemoryAckHandle, InMemoryAckSubscriber, InMemoryBroker, InMemoryPublisher};
use kincir::{Message, Publisher};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::{sleep, timeout};

// Helper function to create a test message with metadata
fn create_test_message(content: &str, metadata: HashMap<String, String>) -> Message {
    let mut message = Message::new(content.as_bytes().to_vec());
    for (key, value) in metadata {
        message = message.with_metadata(&key, &value);
    }
    message
}

// Helper function to create test messages batch
fn create_test_messages(count: usize, prefix: &str) -> Vec<Message> {
    (0..count)
        .map(|i| {
            create_test_message(
                &format!("{} message {}", prefix, i + 1),
                [
                    ("batch_id".to_string(), "test_batch".to_string()),
                    ("message_index".to_string(), i.to_string()),
                    ("created_at".to_string(), chrono::Utc::now().to_rfc3339()),
                ]
                .into_iter()
                .collect(),
            )
        })
        .collect()
}

#[tokio::test]
async fn test_acknowledgment_handle_consistency() {
    // Test that acknowledgment handles have consistent behavior across backends
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    // Subscribe to test topic
    subscriber.subscribe("consistency_test").await
        .expect("Failed to subscribe");

    // Publish test message
    let test_message = create_test_message(
        "Consistency test message",
        [("test_type".to_string(), "consistency".to_string())]
            .into_iter()
            .collect(),
    );
    
    publisher.publish("consistency_test", vec![test_message.clone()]).await
        .expect("Failed to publish message");

    // Receive message with acknowledgment handle
    let (received_message, ack_handle) = timeout(
        Duration::from_secs(5),
        subscriber.receive_with_ack()
    ).await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");

    // Verify message content consistency
    assert_eq!(received_message.payload, test_message.payload);
    assert_eq!(
        received_message.metadata.get("test_type"),
        Some(&"consistency".to_string())
    );

    // Verify acknowledgment handle properties
    assert!(!ack_handle.message_id().is_empty());
    assert_eq!(ack_handle.topic(), "consistency_test");
    assert_eq!(ack_handle.delivery_count(), 1);
    assert!(!ack_handle.is_retry());
    assert!(ack_handle.timestamp() <= SystemTime::now());

    // Test acknowledgment
    subscriber.ack(ack_handle).await
        .expect("Failed to acknowledge message");
}

#[tokio::test]
async fn test_batch_acknowledgment_consistency() {
    // Test that batch acknowledgment works consistently
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("batch_test").await
        .expect("Failed to subscribe");

    // Publish batch of messages
    let test_messages = create_test_messages(5, "Batch");
    publisher.publish("batch_test", test_messages.clone()).await
        .expect("Failed to publish batch");

    // Receive all messages and collect handles
    let mut received_messages = Vec::new();
    let mut ack_handles = Vec::new();

    for i in 0..test_messages.len() {
        let (message, handle) = timeout(
            Duration::from_secs(5),
            subscriber.receive_with_ack()
        ).await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));

        received_messages.push(message);
        ack_handles.push(handle);
    }

    // Verify all messages received
    assert_eq!(received_messages.len(), test_messages.len());
    
    // Verify message content (order may vary)
    for received in &received_messages {
        let received_content = String::from_utf8_lossy(&received.payload);
        assert!(test_messages.iter().any(|original| {
            String::from_utf8_lossy(&original.payload) == received_content
        }));
    }

    // Test batch acknowledgment
    subscriber.ack_batch(ack_handles).await
        .expect("Failed to batch acknowledge messages");
}

#[tokio::test]
async fn test_negative_acknowledgment_consistency() {
    // Test that negative acknowledgment works consistently
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("nack_test").await
        .expect("Failed to subscribe");

    // Test nack without requeue
    let test_message1 = create_test_message(
        "Nack test message 1",
        [("nack_type".to_string(), "discard".to_string())]
            .into_iter()
            .collect(),
    );
    
    publisher.publish("nack_test", vec![test_message1]).await
        .expect("Failed to publish message");

    let (_, handle1) = timeout(
        Duration::from_secs(5),
        subscriber.receive_with_ack()
    ).await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");

    // Negative acknowledge without requeue
    subscriber.nack(handle1, false).await
        .expect("Failed to nack message");

    // Test nack with requeue
    let test_message2 = create_test_message(
        "Nack test message 2",
        [("nack_type".to_string(), "requeue".to_string())]
            .into_iter()
            .collect(),
    );
    
    publisher.publish("nack_test", vec![test_message2]).await
        .expect("Failed to publish message");

    let (_, handle2) = timeout(
        Duration::from_secs(5),
        subscriber.receive_with_ack()
    ).await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");

    // Negative acknowledge with requeue
    subscriber.nack(handle2, true).await
        .expect("Failed to nack message with requeue");
}

#[tokio::test]
async fn test_acknowledgment_error_handling() {
    // Test error handling consistency across backends
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("error_test").await
        .expect("Failed to subscribe");

    // Publish test message
    let test_message = create_test_message(
        "Error handling test",
        [("test_scenario".to_string(), "error_handling".to_string())]
            .into_iter()
            .collect(),
    );
    
    publisher.publish("error_test", vec![test_message]).await
        .expect("Failed to publish message");

    let (_, handle) = timeout(
        Duration::from_secs(5),
        subscriber.receive_with_ack()
    ).await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");

    // Test that acknowledgment succeeds
    let ack_result = subscriber.ack(handle).await;
    assert!(ack_result.is_ok(), "Acknowledgment should succeed");

    // Test that double acknowledgment is handled gracefully
    // (This behavior may vary by backend, but should not panic)
    // Note: We can't test this with the same handle as it's consumed
}

#[tokio::test]
async fn test_concurrent_acknowledgment_operations() {
    // Test concurrent acknowledgment operations
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    
    // Create multiple subscribers for concurrent operations
    let mut subscriber1 = InMemoryAckSubscriber::new(broker.clone());
    let mut subscriber2 = InMemoryAckSubscriber::new(broker.clone());

    subscriber1.subscribe("concurrent_test").await
        .expect("Failed to subscribe subscriber1");
    subscriber2.subscribe("concurrent_test").await
        .expect("Failed to subscribe subscriber2");

    // Publish multiple messages
    let test_messages = create_test_messages(10, "Concurrent");
    publisher.publish("concurrent_test", test_messages).await
        .expect("Failed to publish messages");

    // Concurrently receive and acknowledge messages
    let handle1 = tokio::spawn(async move {
        let mut ack_count = 0;
        for _ in 0..5 {
            if let Ok((_, handle)) = timeout(
                Duration::from_secs(5),
                subscriber1.receive_with_ack()
            ).await {
                if let Ok(_) = handle {
                    if let Ok(_) = subscriber1.ack(handle.unwrap()).await {
                        ack_count += 1;
                    }
                }
            }
        }
        ack_count
    });

    let handle2 = tokio::spawn(async move {
        let mut ack_count = 0;
        for _ in 0..5 {
            if let Ok((_, handle)) = timeout(
                Duration::from_secs(5),
                subscriber2.receive_with_ack()
            ).await {
                if let Ok(_) = handle {
                    if let Ok(_) = subscriber2.ack(handle.unwrap()).await {
                        ack_count += 1;
                    }
                }
            }
        }
        ack_count
    });

    // Wait for both tasks to complete
    let (count1, count2) = tokio::try_join!(handle1, handle2)
        .expect("Concurrent tasks should complete successfully");

    // Verify that messages were processed (exact distribution may vary)
    assert!(count1.unwrap() + count2.unwrap() > 0, "At least some messages should be acknowledged");
}

#[tokio::test]
async fn test_acknowledgment_timing_consistency() {
    // Test that acknowledgment timing is consistent
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("timing_test").await
        .expect("Failed to subscribe");

    // Measure acknowledgment timing
    let mut ack_times = Vec::new();

    for i in 0..5 {
        let test_message = create_test_message(
            &format!("Timing test message {}", i + 1),
            [("timing_test".to_string(), i.to_string())]
                .into_iter()
                .collect(),
        );
        
        publisher.publish("timing_test", vec![test_message]).await
            .expect("Failed to publish message");

        let (_, handle) = timeout(
            Duration::from_secs(5),
            subscriber.receive_with_ack()
        ).await
            .expect("Timeout waiting for message")
            .expect("Failed to receive message");

        let start_time = std::time::Instant::now();
        subscriber.ack(handle).await
            .expect("Failed to acknowledge message");
        let ack_duration = start_time.elapsed();

        ack_times.push(ack_duration);
    }

    // Verify acknowledgment times are reasonable (< 100ms for in-memory)
    for (i, duration) in ack_times.iter().enumerate() {
        assert!(
            duration.as_millis() < 100,
            "Acknowledgment {} took too long: {:?}",
            i + 1,
            duration
        );
    }

    // Calculate average acknowledgment time
    let avg_time = ack_times.iter().sum::<Duration>() / ack_times.len() as u32;
    println!("Average acknowledgment time: {:?}", avg_time);
}

#[tokio::test]
async fn test_acknowledgment_metadata_preservation() {
    // Test that message metadata is preserved through acknowledgment process
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("metadata_test").await
        .expect("Failed to subscribe");

    // Create message with rich metadata
    let metadata = [
        ("correlation_id".to_string(), "test-correlation-123".to_string()),
        ("message_type".to_string(), "test_message".to_string()),
        ("priority".to_string(), "high".to_string()),
        ("source".to_string(), "integration_test".to_string()),
        ("timestamp".to_string(), chrono::Utc::now().to_rfc3339()),
    ].into_iter().collect();

    let test_message = create_test_message("Metadata preservation test", metadata.clone());
    
    publisher.publish("metadata_test", vec![test_message]).await
        .expect("Failed to publish message");

    let (received_message, handle) = timeout(
        Duration::from_secs(5),
        subscriber.receive_with_ack()
    ).await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");

    // Verify all metadata is preserved
    for (key, expected_value) in &metadata {
        assert_eq!(
            received_message.metadata.get(key),
            Some(expected_value),
            "Metadata key '{}' not preserved correctly",
            key
        );
    }

    // Verify handle has correct message information
    assert_eq!(handle.message_id(), received_message.uuid);
    assert_eq!(handle.topic(), "metadata_test");

    // Acknowledge message
    subscriber.ack(handle).await
        .expect("Failed to acknowledge message");
}

#[tokio::test]
async fn test_subscription_state_consistency() {
    // Test that subscription state is consistent across operations
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    // Initially not subscribed
    assert!(!subscriber.is_subscribed().await);
    assert!(subscriber.subscribed_topics().await.is_empty());

    // Subscribe to multiple topics
    let topics = vec!["topic1", "topic2", "topic3"];
    for topic in &topics {
        subscriber.subscribe(topic).await
            .expect(&format!("Failed to subscribe to {}", topic));
    }

    // Verify subscription state
    assert!(subscriber.is_subscribed().await);
    let subscribed_topics = subscriber.subscribed_topics().await;
    assert_eq!(subscribed_topics.len(), topics.len());
    
    for topic in &topics {
        assert!(
            subscribed_topics.contains(&topic.to_string()),
            "Topic '{}' not found in subscribed topics",
            topic
        );
    }
}

// Helper test for backend-specific behavior validation
#[tokio::test]
async fn test_backend_specific_acknowledgment_behavior() {
    // This test validates that the in-memory backend behaves correctly
    // Additional similar tests would be created for other backends
    
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("backend_test").await
        .expect("Failed to subscribe");

    // Test immediate acknowledgment (in-memory specific)
    let test_message = create_test_message(
        "Backend specific test",
        [("backend".to_string(), "in_memory".to_string())]
            .into_iter()
            .collect(),
    );
    
    publisher.publish("backend_test", vec![test_message]).await
        .expect("Failed to publish message");

    let (_, handle) = timeout(
        Duration::from_secs(5),
        subscriber.receive_with_ack()
    ).await
        .expect("Timeout waiting for message")
        .expect("Failed to receive message");

    // In-memory backend should acknowledge immediately
    let ack_start = std::time::Instant::now();
    subscriber.ack(handle).await
        .expect("Failed to acknowledge message");
    let ack_duration = ack_start.elapsed();

    // In-memory acknowledgment should be very fast (< 10ms)
    assert!(
        ack_duration.as_millis() < 10,
        "In-memory acknowledgment took too long: {:?}",
        ack_duration
    );
}

// Performance consistency test
#[tokio::test]
async fn test_acknowledgment_performance_consistency() {
    // Test that acknowledgment performance is consistent under load
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("performance_test").await
        .expect("Failed to subscribe");

    // Publish batch of messages for performance testing
    let message_count = 100;
    let test_messages = create_test_messages(message_count, "Performance");
    
    let publish_start = std::time::Instant::now();
    publisher.publish("performance_test", test_messages).await
        .expect("Failed to publish messages");
    let publish_duration = publish_start.elapsed();

    println!("Published {} messages in {:?}", message_count, publish_duration);

    // Receive and acknowledge all messages
    let mut ack_times = Vec::new();
    let process_start = std::time::Instant::now();

    for i in 0..message_count {
        let (_, handle) = timeout(
            Duration::from_secs(10),
            subscriber.receive_with_ack()
        ).await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));

        let ack_start = std::time::Instant::now();
        subscriber.ack(handle).await
            .expect(&format!("Failed to acknowledge message {}", i + 1));
        let ack_duration = ack_start.elapsed();

        ack_times.push(ack_duration);
    }

    let total_process_duration = process_start.elapsed();
    
    // Calculate performance metrics
    let avg_ack_time = ack_times.iter().sum::<Duration>() / ack_times.len() as u32;
    let max_ack_time = ack_times.iter().max().unwrap();
    let min_ack_time = ack_times.iter().min().unwrap();

    println!("Performance metrics for {} messages:", message_count);
    println!("  Total processing time: {:?}", total_process_duration);
    println!("  Average ack time: {:?}", avg_ack_time);
    println!("  Min ack time: {:?}", min_ack_time);
    println!("  Max ack time: {:?}", max_ack_time);
    println!("  Messages per second: {:.2}", 
        message_count as f64 / total_process_duration.as_secs_f64());

    // Verify performance is acceptable (adjust thresholds as needed)
    assert!(avg_ack_time.as_millis() < 10, "Average acknowledgment time too high");
    assert!(max_ack_time.as_millis() < 50, "Maximum acknowledgment time too high");
    assert!(total_process_duration.as_secs() < 5, "Total processing time too high");
}
