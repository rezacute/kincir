//! Integration example demonstrating the in-memory broker functionality
//! 
//! This example shows how to use the InMemoryBroker with Publisher and Subscriber
//! traits for complete message flow scenarios.

#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use crate::{Publisher, Subscriber, Message};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};
    
    #[tokio::test]
    async fn test_complete_message_flow() {
        // Create broker with statistics enabled
        let config = InMemoryConfig::new()
            .with_max_queue_size(Some(100))
            .with_stats(true);
        let broker = Arc::new(InMemoryBroker::new(config));
        
        // Create publisher and subscriber
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        // Subscribe to topic
        subscriber.subscribe("orders").await.unwrap();
        
        // Publish some orders
        let orders = vec![
            Message::new(b"Order #1: 2x Coffee".to_vec())
                .with_metadata("customer", "Alice")
                .with_metadata("priority", "high"),
            Message::new(b"Order #2: 1x Tea".to_vec())
                .with_metadata("customer", "Bob")
                .with_metadata("priority", "normal"),
            Message::new(b"Order #3: 3x Sandwich".to_vec())
                .with_metadata("customer", "Charlie")
                .with_metadata("priority", "low"),
        ];
        
        publisher.publish("orders", orders.clone()).await.unwrap();
        
        // Process orders
        for expected_order in &orders {
            let received_order = subscriber.receive().await.unwrap();
            
            assert_eq!(received_order.payload, expected_order.payload);
            
            // Check that all expected metadata is present (ignoring system metadata like _sequence, _enqueued_at)
            for (key, expected_value) in &expected_order.metadata {
                assert_eq!(
                    received_order.metadata.get(key),
                    Some(expected_value),
                    "Metadata key '{}' mismatch", key
                );
            }
            
            // Verify system metadata is present
            assert!(received_order.metadata.contains_key("_sequence"), "Missing _sequence metadata");
            assert!(received_order.metadata.contains_key("_enqueued_at"), "Missing _enqueued_at metadata");
            
            println!("Processed order: {} for customer: {}", 
                String::from_utf8_lossy(&received_order.payload),
                received_order.metadata.get("customer").unwrap_or(&"Unknown".to_string())
            );
        }
        
        // Check statistics
        let stats = publisher.stats().unwrap();
        assert_eq!(stats.messages_published, 3);
        assert_eq!(stats.messages_consumed, 3);
        assert_eq!(stats.active_topics, 1);
        assert_eq!(stats.active_subscribers, 1);
    }
    
    #[tokio::test]
    async fn test_multiple_topics_and_subscribers() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        
        // Create multiple publishers and subscribers
        let order_publisher = InMemoryPublisher::new(broker.clone());
        let notification_publisher = InMemoryPublisher::new(broker.clone());
        
        let mut order_processor = InMemorySubscriber::new(broker.clone());
        let mut notification_service = InMemorySubscriber::new(broker.clone());
        let mut audit_service = InMemorySubscriber::new(broker.clone());
        
        // Set up subscriptions
        order_processor.subscribe("orders").await.unwrap();
        notification_service.subscribe("notifications").await.unwrap();
        audit_service.subscribe("orders").await.unwrap(); // Also listens to orders
        
        // Publish to different topics
        let order = vec![Message::new(b"New order received".to_vec())];
        let notification = vec![Message::new(b"System maintenance scheduled".to_vec())];
        
        order_publisher.publish("orders", order.clone()).await.unwrap();
        notification_publisher.publish("notifications", notification.clone()).await.unwrap();
        
        // Receive messages
        let processed_order = order_processor.receive().await.unwrap();
        let audit_order = audit_service.receive().await.unwrap();
        let system_notification = notification_service.receive().await.unwrap();
        
        // Verify both order processor and audit service received the order
        assert_eq!(processed_order.payload, order[0].payload);
        assert_eq!(audit_order.payload, order[0].payload);
        assert_eq!(system_notification.payload, notification[0].payload);
        
        // Verify topic count
        assert_eq!(broker.topic_count(), 2);
    }
    
    #[tokio::test]
    async fn test_high_throughput_scenario() {
        let config = InMemoryConfig::new()
            .with_max_queue_size(Some(10000))
            .with_stats(true);
        let broker = Arc::new(InMemoryBroker::new(config));
        
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        subscriber.subscribe("high-volume").await.unwrap();
        
        // Generate many messages
        let message_count = 1000;
        let mut messages = Vec::with_capacity(message_count);
        for i in 0..message_count {
            messages.push(
                Message::new(format!("Message #{}", i).into_bytes())
                    .with_metadata("sequence", &i.to_string())
            );
        }
        
        // Publish in batches
        let batch_size = 100;
        for chunk in messages.chunks(batch_size) {
            publisher.publish("high-volume", chunk.to_vec()).await.unwrap();
        }
        
        // Receive all messages
        let mut received_count = 0;
        while received_count < message_count {
            let batch = subscriber.try_receive_batch(50).unwrap();
            if batch.is_empty() {
                // Wait a bit for more messages
                tokio::time::sleep(Duration::from_millis(1)).await;
                continue;
            }
            received_count += batch.len();
        }
        
        assert_eq!(received_count, message_count);
        
        // Check final statistics
        let stats = publisher.stats().unwrap();
        assert_eq!(stats.messages_published as usize, message_count);
        assert_eq!(stats.messages_consumed as usize, message_count);
    }
    
    #[tokio::test]
    async fn test_error_handling_scenarios() {
        let config = InMemoryConfig::new()
            .with_max_topics(Some(1))
            .with_max_queue_size(Some(2));
        let broker = Arc::new(InMemoryBroker::new(config));
        
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        // Test topic limit
        subscriber.subscribe("topic1").await.unwrap();
        let result = subscriber.subscribe("topic2").await;
        assert!(matches!(result, Err(InMemoryError::MaxTopicsReached { .. })));
        
        // Test queue limit
        let messages = vec![
            Message::new(b"Message 1".to_vec()),
            Message::new(b"Message 2".to_vec()),
            Message::new(b"Message 3".to_vec()), // This should fail
        ];
        
        let result = publisher.publish("topic1", messages).await;
        assert!(matches!(result, Err(InMemoryError::QueueFull { .. })));
        
        // Test invalid topic names
        let result = publisher.publish("", vec![Message::new(b"test".to_vec())]).await;
        assert!(matches!(result, Err(InMemoryError::InvalidTopicName { .. })));
        
        let result = subscriber.subscribe("test\0topic").await;
        assert!(matches!(result, Err(InMemoryError::InvalidTopicName { .. })));
    }
    
    #[tokio::test]
    async fn test_broker_lifecycle() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        // Normal operation
        subscriber.subscribe("lifecycle-test").await.unwrap();
        assert!(publisher.is_connected());
        assert!(subscriber.is_connected());
        
        let message = vec![Message::new(b"Before shutdown".to_vec())];
        publisher.publish("lifecycle-test", message).await.unwrap();
        
        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.payload, b"Before shutdown");
        
        // Shutdown broker
        broker.shutdown().unwrap();
        assert!(!publisher.is_connected());
        assert!(!subscriber.is_connected());
        
        // Operations should fail after shutdown
        let message = vec![Message::new(b"After shutdown".to_vec())];
        let result = publisher.publish("lifecycle-test", message).await;
        assert!(matches!(result, Err(InMemoryError::BrokerShutdown)));
        
        let result = subscriber.receive().await;
        assert!(matches!(result, Err(InMemoryError::BrokerShutdown)));
    }
    
    #[tokio::test]
    async fn test_concurrent_operations() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        
        // Create multiple publishers and subscribers
        let publishers: Vec<_> = (0..5)
            .map(|_| InMemoryPublisher::new(broker.clone()))
            .collect();
        
        let mut subscribers: Vec<_> = (0..3)
            .map(|_| InMemorySubscriber::new(broker.clone()))
            .collect();
        
        // Subscribe all subscribers to the same topic
        for subscriber in &mut subscribers {
            subscriber.subscribe("concurrent-test").await.unwrap();
        }
        
        // Publish messages concurrently from multiple publishers
        let publish_tasks: Vec<_> = publishers.into_iter().enumerate().map(|(i, publisher)| {
            tokio::spawn(async move {
                let messages = vec![
                    Message::new(format!("Message from publisher {}", i).into_bytes())
                ];
                publisher.publish("concurrent-test", messages).await.unwrap();
            })
        }).collect();
        
        // Wait for all publishes to complete
        for task in publish_tasks {
            task.await.unwrap();
        }
        
        // Each subscriber should receive all 5 messages
        for subscriber in &mut subscribers {
            for _ in 0..5 {
                let message = timeout(Duration::from_secs(1), subscriber.receive()).await.unwrap().unwrap();
                assert!(String::from_utf8_lossy(&message.payload).contains("Message from publisher"));
            }
        }
        
        assert_eq!(broker.topic_count(), 1);
    }
    
    #[tokio::test]
    async fn test_message_metadata_preservation() {
        let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::for_testing()));
        let publisher = InMemoryPublisher::new(broker.clone());
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        subscriber.subscribe("metadata-test").await.unwrap();
        
        // Create message with rich metadata
        let original_message = Message::new(b"Test payload".to_vec())
            .with_metadata("content-type", "application/json")
            .with_metadata("timestamp", "2024-01-15T10:30:00Z")
            .with_metadata("source", "order-service")
            .with_metadata("version", "1.0")
            .with_metadata("priority", "high");
        
        publisher.publish("metadata-test", vec![original_message.clone()]).await.unwrap();
        
        let received_message = subscriber.receive().await.unwrap();
        
        // Verify payload and UUID are preserved
        assert_eq!(received_message.payload, original_message.payload);
        assert_eq!(received_message.uuid, original_message.uuid);
        
        // Verify all original metadata is preserved (ignoring system metadata)
        for (key, expected_value) in &original_message.metadata {
            assert_eq!(
                received_message.metadata.get(key),
                Some(expected_value),
                "Metadata key '{}' mismatch", key
            );
        }
        
        // Verify system metadata is present
        assert!(received_message.metadata.contains_key("_sequence"), "Missing _sequence metadata");
        assert!(received_message.metadata.contains_key("_enqueued_at"), "Missing _enqueued_at metadata");
        
        // Verify specific metadata values
        assert_eq!(received_message.metadata.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(received_message.metadata.get("priority"), Some(&"high".to_string()));
        assert_eq!(received_message.metadata.get("source"), Some(&"order-service".to_string()));
    }
}
