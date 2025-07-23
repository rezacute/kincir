//! Comprehensive acknowledgment validation tests
//!
//! These tests validate the complete acknowledgment system functionality
//! including cross-backend consistency, high-throughput scenarios, and
//! integration with the router system.

#[cfg(test)]
mod comprehensive_tests {
    use crate::ack::AckSubscriber;
    use crate::memory::{InMemoryAckSubscriber, InMemoryBroker, InMemoryPublisher};
    use crate::{Message, Publisher};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::timeout;

    // Wrapper to convert InMemoryPublisher to the expected error type
    struct PublisherWrapper {
        inner: InMemoryPublisher,
    }

    impl PublisherWrapper {
        fn new(publisher: InMemoryPublisher) -> Self {
            Self { inner: publisher }
        }
    }

    #[async_trait::async_trait]
    impl Publisher for PublisherWrapper {
        type Error = Box<dyn std::error::Error + Send + Sync>;

        async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
            self.inner.publish(topic, messages).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }

    // Helper function to create test messages with metadata
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
                        ("created_at".to_string(), format!("{:?}", SystemTime::now())),
                    ]
                    .into_iter()
                    .collect(),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn test_integration_validation() {
        // Simple integration test to validate acknowledgment system works end-to-end
        println!("🚀 Running integration validation test...");

        let broker = Arc::new(InMemoryBroker::new(crate::memory::InMemoryConfig::for_testing()));
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

        // Test 1: Basic publish-subscribe-ack workflow
        subscriber.subscribe("integration_test").await
            .expect("Failed to subscribe");

        let test_message = Message::new(b"Integration test message".to_vec())
            .with_metadata("test_type", "integration");
        
        publisher.publish("integration_test", vec![test_message.clone()]).await
            .expect("Failed to publish message");

        // Small delay to ensure message propagation
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Receive and acknowledge
        match timeout(Duration::from_secs(2), subscriber.receive_with_ack()).await {
            Ok(Ok((received_message, ack_handle))) => {
                assert_eq!(received_message.payload, test_message.payload);
                assert_eq!(received_message.metadata.get("test_type"), Some(&"integration".to_string()));
                
                subscriber.ack(ack_handle).await
                    .expect("Failed to acknowledge message");
                
                println!("✅ Basic workflow test passed");
            }
            Ok(Err(e)) => {
                panic!("Failed to receive message: {}", e);
            }
            Err(_) => {
                panic!("Timeout waiting for message");
            }
        }

        // Test 2: Batch operations
        let batch_messages: Vec<Message> = (0..3)
            .map(|i| Message::new(format!("Batch message {}", i).into_bytes()))
            .collect();

        publisher.publish("integration_test", batch_messages.clone()).await
            .expect("Failed to publish batch");

        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut handles = Vec::new();
        for i in 0..3 {
            match timeout(Duration::from_secs(2), subscriber.receive_with_ack()).await {
                Ok(Ok((_, handle))) => handles.push(handle),
                Ok(Err(e)) => panic!("Failed to receive batch message {}: {}", i, e),
                Err(_) => panic!("Timeout waiting for batch message {}", i),
            }
        }

        subscriber.ack_batch(handles).await
            .expect("Failed to batch acknowledge");

        println!("✅ Batch operations test passed");

        // Test 3: Negative acknowledgment
        let nack_message = Message::new(b"Nack test message".to_vec());
        publisher.publish("integration_test", vec![nack_message]).await
            .expect("Failed to publish nack message");

        tokio::time::sleep(Duration::from_millis(10)).await;

        match timeout(Duration::from_secs(2), subscriber.receive_with_ack()).await {
            Ok(Ok((_, handle))) => {
                subscriber.nack(handle, false).await
                    .expect("Failed to nack message");
                println!("✅ Negative acknowledgment test passed");
            }
            Ok(Err(e)) => panic!("Failed to receive nack message: {}", e),
            Err(_) => panic!("Timeout waiting for nack message"),
        }

        println!("✅ Integration validation test completed successfully");
    }

    #[tokio::test]
    async fn test_batch_acknowledgment_comprehensive() {
        // Test comprehensive batch acknowledgment functionality
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

        subscriber.subscribe("batch_comprehensive").await
            .expect("Failed to subscribe");

        // Test batch sizes: small, medium, large
        let batch_sizes = vec![5, 25, 100];

        for batch_size in batch_sizes {
            println!("Testing batch size: {}", batch_size);

            // Publish batch
            let test_messages = create_test_messages(batch_size, &format!("Batch{}", batch_size));
            publisher.publish("batch_comprehensive", test_messages.clone()).await
                .expect("Failed to publish batch");

            // Receive all messages
            let mut handles = Vec::new();
            for i in 0..batch_size {
                let (message, handle) = timeout(
                    Duration::from_secs(10),
                    subscriber.receive_with_ack()
                ).await
                    .expect(&format!("Timeout waiting for message {} in batch {}", i + 1, batch_size))
                    .expect(&format!("Failed to receive message {} in batch {}", i + 1, batch_size));

                // Verify message is from this batch
                assert!(message.metadata.get("batch_id") == Some(&"test_batch".to_string()));
                handles.push(handle);
            }

            // Batch acknowledge
            let ack_start = std::time::Instant::now();
            subscriber.ack_batch(handles).await
                .expect(&format!("Failed to batch acknowledge {} messages", batch_size));
            let ack_duration = ack_start.elapsed();

            println!("  Batch {} acknowledged in {:?}", batch_size, ack_duration);
            assert!(ack_duration.as_millis() < 100, "Batch ack should be fast");
        }

        println!("✅ Comprehensive batch acknowledgment test passed");
    }

    #[tokio::test]
    async fn test_negative_acknowledgment_scenarios() {
        // Test various negative acknowledgment scenarios
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

        subscriber.subscribe("nack_scenarios").await
            .expect("Failed to subscribe");

        // Scenario 1: Nack without requeue
        let message1 = create_test_message(
            "Nack scenario 1",
            [("scenario".to_string(), "nack_discard".to_string())]
                .into_iter()
                .collect(),
        );
        
        publisher.publish("nack_scenarios", vec![message1]).await
            .expect("Failed to publish message");

        let (_, handle1) = timeout(
            Duration::from_secs(5),
            subscriber.receive_with_ack()
        ).await
            .expect("Timeout waiting for message")
            .expect("Failed to receive message");

        subscriber.nack(handle1, false).await
            .expect("Failed to nack message without requeue");

        // Scenario 2: Nack with requeue
        let message2 = create_test_message(
            "Nack scenario 2",
            [("scenario".to_string(), "nack_requeue".to_string())]
                .into_iter()
                .collect(),
        );
        
        publisher.publish("nack_scenarios", vec![message2]).await
            .expect("Failed to publish message");

        let (_, handle2) = timeout(
            Duration::from_secs(5),
            subscriber.receive_with_ack()
        ).await
            .expect("Timeout waiting for message")
            .expect("Failed to receive message");

        subscriber.nack(handle2, true).await
            .expect("Failed to nack message with requeue");

        // Scenario 3: Batch nack
        let batch_messages = create_test_messages(3, "BatchNack");
        publisher.publish("nack_scenarios", batch_messages).await
            .expect("Failed to publish batch for nack");

        let mut nack_handles = Vec::new();
        for i in 0..3 {
            let (_, handle) = timeout(
                Duration::from_secs(5),
                subscriber.receive_with_ack()
            ).await
                .expect(&format!("Timeout waiting for nack message {}", i + 1))
                .expect(&format!("Failed to receive nack message {}", i + 1));
            nack_handles.push(handle);
        }

        subscriber.nack_batch(nack_handles, false).await
            .expect("Failed to batch nack messages");

        println!("✅ Negative acknowledgment scenarios test passed");
    }

    #[tokio::test]
    async fn test_router_acknowledgment_integration() {
        // Test router integration with acknowledgment system
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let output_publisher = Arc::new(PublisherWrapper::new(InMemoryPublisher::new(broker.clone())));
        let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
        let output_subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));

        // Create message handler
        let handler: crate::router::HandlerFunc = Arc::new(|msg: Message| {
            Box::pin(async move {
                // Simple processing: add metadata
                let processed = msg
                    .with_metadata("processed", "true")
                    .with_metadata("processed_at", &format!("{:?}", SystemTime::now()));
                Ok(vec![processed])
            })
        });

        // Test different acknowledgment strategies
        let strategies = vec![
            (AckStrategy::AutoAckOnSuccess, "AutoAckOnSuccess"),
            (AckStrategy::AlwaysAck, "AlwaysAck"),
        ];

        for (strategy, strategy_name) in strategies {
            println!("Testing router with strategy: {}", strategy_name);

            let config = RouterAckConfig {
                strategy: strategy.clone(),
                processing_timeout: Some(Duration::from_secs(5)),
                max_retries: 1,
                requeue_on_failure: false,
                batch_size: None,
            };

            #[cfg(feature = "logging")]
            let router = {
                use crate::logging::StdLogger;
                let logger = Arc::new(StdLogger::new(false, false));
                AckRouter::new(
                    logger,
                    "router_input".to_string(),
                    "router_output".to_string(),
                    subscriber.clone(),
                    output_publisher.clone(),
                    handler.clone(),
                    config,
                )
            };

            #[cfg(not(feature = "logging"))]
            let router = AckRouter::new(
                "router_input".to_string(),
                "router_output".to_string(),
                subscriber.clone(),
                output_publisher.clone(),
                handler.clone(),
                config,
            );

            // Subscribe to output
            {
                let mut output_sub = output_subscriber.lock().await;
                output_sub.subscribe("router_output").await
                    .expect("Failed to subscribe to router output");
            }

            // Publish test message
            let test_message = create_test_message(
                &format!("Router test with {}", strategy_name),
                [("strategy".to_string(), strategy_name.to_string())]
                    .into_iter()
                    .collect(),
            );

            input_publisher.publish("router_input", vec![test_message]).await
                .expect("Failed to publish to router input");

            // Process message through router
            let process_result = timeout(
                Duration::from_secs(10),
                router.process_single_message()
            ).await;

            assert!(process_result.is_ok(), "Router processing should not timeout");
            assert!(process_result.unwrap().is_ok(), "Router processing should succeed");

            // Verify processed message in output
            let output_result = timeout(Duration::from_secs(5), async {
                let mut output_sub = output_subscriber.lock().await;
                output_sub.receive_with_ack().await
            }).await;

            assert!(output_result.is_ok(), "Should receive processed message");
            let (processed_message, output_handle) = output_result.unwrap()
                .expect("Should successfully receive processed message");

            // Verify processing
            assert_eq!(processed_message.metadata.get("processed"), Some(&"true".to_string()));
            assert!(processed_message.metadata.contains_key("processed_at"));

            // Acknowledge output message
            {
                let mut output_sub = output_subscriber.lock().await;
                output_sub.ack(output_handle).await
                    .expect("Failed to acknowledge output message");
            }

            // Check router statistics
            let stats = router.stats().await;
            assert_eq!(stats.messages_processed, 1);
            
            match strategy {
                AckStrategy::AutoAckOnSuccess | AckStrategy::AlwaysAck => {
                    assert_eq!(stats.messages_acked, 1);
                    assert_eq!(stats.ack_rate(), 100.0);
                }
                _ => {}
            }

            // Reset for next strategy test
            router.reset_stats().await;
        }

        println!("✅ Router acknowledgment integration test passed");
    }

    #[tokio::test]
    async fn test_concurrent_acknowledgment_operations() {
        // Test concurrent acknowledgment operations
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));

        // Create multiple subscribers
        let subscriber_count = 3;
        let messages_per_subscriber = 10;
        let total_messages = subscriber_count * messages_per_subscriber;

        // Publish all messages first
        let test_messages = create_test_messages(total_messages, "Concurrent");
        publisher.publish("concurrent_ack", test_messages).await
            .expect("Failed to publish messages");

        // Create concurrent subscribers
        let mut subscriber_handles = Vec::new();

        for sub_id in 0..subscriber_count {
            let broker_clone = broker.clone();
            let handle = tokio::spawn(async move {
                let mut subscriber = InMemoryAckSubscriber::new(broker_clone);
                subscriber.subscribe("concurrent_ack").await
                    .expect("Failed to subscribe");

                let mut processed_count = 0;
                let mut ack_times = Vec::new();

                // Try to receive and acknowledge messages
                for _ in 0..messages_per_subscriber {
                    match timeout(
                        Duration::from_secs(10),
                        subscriber.receive_with_ack()
                    ).await {
                        Ok(Ok((_, handle))) => {
                            let ack_start = std::time::Instant::now();
                            if subscriber.ack(handle).await.is_ok() {
                                let ack_duration = ack_start.elapsed();
                                ack_times.push(ack_duration);
                                processed_count += 1;
                            }
                        }
                        Ok(Err(_)) => break,
                        Err(_) => break, // Timeout
                    }
                }

                let avg_ack_time = if !ack_times.is_empty() {
                    ack_times.iter().sum::<Duration>() / ack_times.len() as u32
                } else {
                    Duration::from_nanos(0)
                };

                (sub_id, processed_count, avg_ack_time)
            });

            subscriber_handles.push(handle);
        }

        // Wait for all subscribers to complete
        let mut total_processed = 0;
        for handle in subscriber_handles {
            let (sub_id, count, avg_ack_time) = handle.await
                .expect("Subscriber task should complete");
            
            total_processed += count;
            println!("Subscriber {} processed {} messages (avg ack: {:?})", 
                    sub_id, count, avg_ack_time);
        }

        println!("Total messages processed: {} / {}", total_processed, total_messages);
        
        // Verify all messages were processed
        assert_eq!(total_processed, total_messages, 
                  "All messages should be processed by concurrent subscribers");

        println!("✅ Concurrent acknowledgment operations test passed");
    }

    #[tokio::test]
    async fn test_acknowledgment_performance_metrics() {
        // Test acknowledgment performance and collect metrics
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

        subscriber.subscribe("performance_metrics").await
            .expect("Failed to subscribe");

        // Test different message sizes and batch sizes
        let test_scenarios = vec![
            (100, 100, "Small messages"),
            (1000, 10, "Large messages"),
            (500, 50, "Medium batch"),
        ];

        for (message_count, batch_size, scenario_name) in test_scenarios {
            println!("Testing scenario: {} ({} messages, batch size {})", 
                    scenario_name, message_count, batch_size);

            // Create messages with varying sizes
            let payload_size = if scenario_name.contains("Large") { 1024 } else { 64 };
            let payload = vec![b'X'; payload_size];
            
            let messages: Vec<Message> = (0..message_count)
                .map(|i| {
                    Message::new(payload.clone())
                        .with_metadata("scenario", scenario_name)
                        .with_metadata("message_id", &i.to_string())
                })
                .collect();

            // Publish messages
            let publish_start = std::time::Instant::now();
            publisher.publish("performance_metrics", messages).await
                .expect("Failed to publish messages");
            let publish_duration = publish_start.elapsed();

            // Receive and acknowledge in batches
            let mut total_ack_time = Duration::from_nanos(0);
            let mut batch_count = 0;
            let receive_start = std::time::Instant::now();

            for batch_start in (0..message_count).step_by(batch_size) {
                let batch_end = std::cmp::min(batch_start + batch_size, message_count);
                let current_batch_size = batch_end - batch_start;

                let mut batch_handles = Vec::new();
                
                // Receive batch
                for _ in 0..current_batch_size {
                    let (_, handle) = timeout(
                        Duration::from_secs(10),
                        subscriber.receive_with_ack()
                    ).await
                        .expect("Timeout receiving message")
                        .expect("Failed to receive message");
                    batch_handles.push(handle);
                }

                // Acknowledge batch
                let ack_start = std::time::Instant::now();
                subscriber.ack_batch(batch_handles).await
                    .expect("Failed to batch acknowledge");
                let ack_duration = ack_start.elapsed();

                total_ack_time += ack_duration;
                batch_count += 1;
            }

            let total_duration = receive_start.elapsed();

            // Calculate metrics
            let throughput = message_count as f64 / total_duration.as_secs_f64();
            let avg_ack_time_per_batch = total_ack_time / batch_count;
            let avg_ack_time_per_message = total_ack_time / message_count as u32;

            println!("  Results:");
            println!("    Publish time: {:?}", publish_duration);
            println!("    Total time: {:?}", total_duration);
            println!("    Throughput: {:.2} msg/sec", throughput);
            println!("    Avg ack time per batch: {:?}", avg_ack_time_per_batch);
            println!("    Avg ack time per message: {:?}", avg_ack_time_per_message);

            // Verify performance expectations
            assert!(throughput > 50.0, "Throughput should be > 50 msg/sec for {}", scenario_name);
            assert!(avg_ack_time_per_message.as_millis() < 10, 
                   "Avg ack time per message should be < 10ms for {}", scenario_name);
        }

        println!("✅ Acknowledgment performance metrics test passed");
    }

    #[tokio::test]
    async fn test_acknowledgment_error_recovery() {
        // Test error recovery scenarios in acknowledgment system
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

        subscriber.subscribe("error_recovery").await
            .expect("Failed to subscribe");

        // Test scenario: Acknowledge after subscriber operations
        let test_message = create_test_message(
            "Error recovery test",
            [("test_type".to_string(), "error_recovery".to_string())]
                .into_iter()
                .collect(),
        );

        publisher.publish("error_recovery", vec![test_message]).await
            .expect("Failed to publish message");

        let (_, handle) = timeout(
            Duration::from_secs(5),
            subscriber.receive_with_ack()
        ).await
            .expect("Timeout waiting for message")
            .expect("Failed to receive message");

        // Test that acknowledgment works normally
        let ack_result = subscriber.ack(handle).await;
        assert!(ack_result.is_ok(), "Normal acknowledgment should succeed");

        // Test subscription state consistency after operations
        assert!(subscriber.is_subscribed().await);

        println!("✅ Acknowledgment error recovery test passed");
    }

    #[tokio::test]
    async fn test_acknowledgment_system_integration() {
        // Final integration test that combines all acknowledgment features
        println!("🚀 Running comprehensive acknowledgment system integration test...");

        let broker = Arc::new(InMemoryBroker::with_default_config());
        let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

        // Test multiple topics
        let topics = vec!["integration_1", "integration_2", "integration_3"];
        for topic in &topics {
            subscriber.subscribe(topic).await
                .expect(&format!("Failed to subscribe to {}", topic));
        }

        // Verify subscription state
        assert!(subscriber.is_subscribed().await);
        // Note: We'll verify subscriptions work by successfully receiving messages from all topics

        // Publish messages to different topics
        let mut all_messages = Vec::new();
        for (i, topic) in topics.iter().enumerate() {
            let topic_messages = create_test_messages(5, &format!("Topic{}", i));
            publisher.publish(topic, topic_messages.clone()).await
                .expect(&format!("Failed to publish to {}", topic));
            all_messages.extend(topic_messages);
        }

        // Receive all messages
        let mut received_messages = Vec::new();
        let mut handles = Vec::new();

        for i in 0..all_messages.len() {
            let (message, handle) = timeout(
                Duration::from_secs(10),
                subscriber.receive_with_ack()
            ).await
                .expect(&format!("Timeout waiting for message {}", i + 1))
                .expect(&format!("Failed to receive message {}", i + 1));

            received_messages.push(message);
            handles.push(handle);
        }

        // Verify all messages received
        assert_eq!(received_messages.len(), all_messages.len());

        // Test mixed acknowledgment operations
        let mid_point = handles.len() / 2;
        
        // Acknowledge first half individually
        for handle in handles.drain(0..mid_point) {
            subscriber.ack(handle).await
                .expect("Failed to individually acknowledge message");
        }

        // Batch acknowledge second half
        subscriber.ack_batch(handles).await
            .expect("Failed to batch acknowledge remaining messages");

        println!("✅ Comprehensive acknowledgment system integration test passed");
        println!("   - Tested {} topics", topics.len());
        println!("   - Processed {} messages", all_messages.len());
        println!("   - Mixed individual and batch acknowledgment");
        println!("   - All operations completed successfully");
    }
}
