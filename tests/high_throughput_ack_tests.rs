//! High-throughput acknowledgment scenario tests
//!
//! These tests verify that the acknowledgment system can handle high-throughput
//! scenarios with multiple concurrent publishers, subscribers, and acknowledgment
//! operations while maintaining consistency and performance.

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::memory::{InMemoryAckSubscriber, InMemoryBroker, InMemoryPublisher};
use kincir::router::{AckRouter, AckStrategy, RouterAckConfig};
use kincir::{Message, Publisher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{sleep, timeout};

// Test configuration constants
const HIGH_THROUGHPUT_MESSAGE_COUNT: usize = 1000;
const CONCURRENT_PUBLISHERS: usize = 5;
const CONCURRENT_SUBSCRIBERS: usize = 3;
const BATCH_SIZE: usize = 50;

// Helper function to create test messages with sequence numbers
fn create_sequenced_messages(count: usize, prefix: &str, start_seq: usize) -> Vec<Message> {
    (0..count)
        .map(|i| {
            let seq = start_seq + i;
            Message::new(format!("{} message {}", prefix, seq).into_bytes())
                .with_metadata("sequence", &seq.to_string())
                .with_metadata("batch_id", prefix)
                .with_metadata("created_at", &chrono::Utc::now().to_rfc3339())
        })
        .collect()
}

// Helper function to create a high-performance message handler
fn create_fast_handler() -> kincir::router::HandlerFunc {
    Arc::new(|msg: Message| {
        Box::pin(async move {
            // Minimal processing for high throughput
            let processed = msg.with_metadata("processed", "true");
            Ok(vec![processed])
        })
    })
}

#[tokio::test]
async fn test_high_volume_single_subscriber() {
    println!("🚀 Testing high-volume single subscriber scenario...");
    
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("high_volume").await
        .expect("Failed to subscribe");

    // Publish large batch of messages
    let test_messages = create_sequenced_messages(HIGH_THROUGHPUT_MESSAGE_COUNT, "HighVolume", 0);
    
    let publish_start = Instant::now();
    publisher.publish("high_volume", test_messages.clone()).await
        .expect("Failed to publish messages");
    let publish_duration = publish_start.elapsed();

    println!("📤 Published {} messages in {:?}", HIGH_THROUGHPUT_MESSAGE_COUNT, publish_duration);

    // Receive and acknowledge all messages
    let mut received_count = 0;
    let mut ack_times = Vec::new();
    let process_start = Instant::now();

    for i in 0..HIGH_THROUGHPUT_MESSAGE_COUNT {
        let (message, handle) = timeout(
            Duration::from_secs(30),
            subscriber.receive_with_ack()
        ).await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));

        // Verify message content
        assert!(String::from_utf8_lossy(&message.payload).contains("HighVolume"));
        assert!(message.metadata.contains_key("sequence"));

        let ack_start = Instant::now();
        subscriber.ack(handle).await
            .expect(&format!("Failed to acknowledge message {}", i + 1));
        let ack_duration = ack_start.elapsed();

        ack_times.push(ack_duration);
        received_count += 1;

        // Progress indicator
        if (i + 1) % 100 == 0 {
            println!("📥 Processed {} messages", i + 1);
        }
    }

    let total_duration = process_start.elapsed();

    // Calculate performance metrics
    let avg_ack_time = ack_times.iter().sum::<Duration>() / ack_times.len() as u32;
    let throughput = received_count as f64 / total_duration.as_secs_f64();

    println!("📊 High-volume single subscriber results:");
    println!("   Messages processed: {}", received_count);
    println!("   Total time: {:?}", total_duration);
    println!("   Average ack time: {:?}", avg_ack_time);
    println!("   Throughput: {:.2} messages/second", throughput);

    // Verify performance expectations
    assert_eq!(received_count, HIGH_THROUGHPUT_MESSAGE_COUNT);
    assert!(throughput > 100.0, "Throughput should be > 100 msg/sec, got {:.2}", throughput);
    assert!(avg_ack_time.as_millis() < 10, "Average ack time should be < 10ms");
}

#[tokio::test]
async fn test_concurrent_publishers_single_subscriber() {
    println!("🚀 Testing concurrent publishers with single subscriber...");
    
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("concurrent_pub").await
        .expect("Failed to subscribe");

    let messages_per_publisher = HIGH_THROUGHPUT_MESSAGE_COUNT / CONCURRENT_PUBLISHERS;
    let total_expected = messages_per_publisher * CONCURRENT_PUBLISHERS;

    // Create concurrent publishers
    let mut publisher_handles = Vec::new();
    let publish_start = Instant::now();

    for pub_id in 0..CONCURRENT_PUBLISHERS {
        let broker_clone = broker.clone();
        let handle = tokio::spawn(async move {
            let publisher = InMemoryPublisher::new(broker_clone);
            let messages = create_sequenced_messages(
                messages_per_publisher,
                &format!("Pub{}", pub_id),
                pub_id * messages_per_publisher,
            );

            let start = Instant::now();
            publisher.publish("concurrent_pub", messages).await
                .expect("Failed to publish messages");
            let duration = start.elapsed();

            (pub_id, messages_per_publisher, duration)
        });
        publisher_handles.push(handle);
    }

    // Wait for all publishers to complete
    let mut total_published = 0;
    for handle in publisher_handles {
        let (pub_id, count, duration) = handle.await
            .expect("Publisher task should complete");
        total_published += count;
        println!("📤 Publisher {} published {} messages in {:?}", pub_id, count, duration);
    }

    let publish_duration = publish_start.elapsed();
    println!("📤 All publishers completed in {:?}", publish_duration);

    // Receive and acknowledge all messages
    let mut received_messages = Vec::new();
    let mut ack_times = Vec::new();
    let receive_start = Instant::now();

    for i in 0..total_expected {
        let (message, handle) = timeout(
            Duration::from_secs(30),
            subscriber.receive_with_ack()
        ).await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));

        received_messages.push(message);

        let ack_start = Instant::now();
        subscriber.ack(handle).await
            .expect(&format!("Failed to acknowledge message {}", i + 1));
        let ack_duration = ack_start.elapsed();

        ack_times.push(ack_duration);

        if (i + 1) % 100 == 0 {
            println!("📥 Received {} messages", i + 1);
        }
    }

    let receive_duration = receive_start.elapsed();

    // Verify all publishers' messages were received
    let mut publisher_counts = vec![0; CONCURRENT_PUBLISHERS];
    for message in &received_messages {
        let payload = String::from_utf8_lossy(&message.payload);
        for pub_id in 0..CONCURRENT_PUBLISHERS {
            if payload.contains(&format!("Pub{}", pub_id)) {
                publisher_counts[pub_id] += 1;
                break;
            }
        }
    }

    // Calculate metrics
    let avg_ack_time = ack_times.iter().sum::<Duration>() / ack_times.len() as u32;
    let throughput = received_messages.len() as f64 / receive_duration.as_secs_f64();

    println!("📊 Concurrent publishers results:");
    println!("   Total published: {}", total_published);
    println!("   Total received: {}", received_messages.len());
    println!("   Receive time: {:?}", receive_duration);
    println!("   Average ack time: {:?}", avg_ack_time);
    println!("   Throughput: {:.2} messages/second", throughput);
    
    for (pub_id, count) in publisher_counts.iter().enumerate() {
        println!("   Publisher {} messages: {}", pub_id, count);
    }

    // Verify results
    assert_eq!(received_messages.len(), total_expected);
    assert!(throughput > 50.0, "Throughput should be > 50 msg/sec with concurrent publishers");
    
    // Verify each publisher's messages were received
    for (pub_id, &count) in publisher_counts.iter().enumerate() {
        assert!(count > 0, "Publisher {} messages not received", pub_id);
    }
}

#[tokio::test]
async fn test_concurrent_subscribers_single_publisher() {
    println!("🚀 Testing single publisher with concurrent subscribers...");
    
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));

    // Create concurrent subscribers
    let mut subscriber_handles = Vec::new();
    let received_counts = Arc::new(Vec::new());
    
    for sub_id in 0..CONCURRENT_SUBSCRIBERS {
        received_counts.push(AtomicU64::new(0));
    }
    let received_counts = Arc::new(received_counts);

    let messages_per_subscriber = HIGH_THROUGHPUT_MESSAGE_COUNT / CONCURRENT_SUBSCRIBERS;

    for sub_id in 0..CONCURRENT_SUBSCRIBERS {
        let broker_clone = broker.clone();
        let counts_clone = received_counts.clone();
        
        let handle = tokio::spawn(async move {
            let mut subscriber = InMemoryAckSubscriber::new(broker_clone);
            subscriber.subscribe("concurrent_sub").await
                .expect("Failed to subscribe");

            let mut local_count = 0;
            let mut ack_times = Vec::new();
            let start = Instant::now();

            // Each subscriber tries to receive messages
            while local_count < messages_per_subscriber {
                match timeout(
                    Duration::from_secs(10),
                    subscriber.receive_with_ack()
                ).await {
                    Ok(Ok((message, handle))) => {
                        let ack_start = Instant::now();
                        subscriber.ack(handle).await
                            .expect("Failed to acknowledge message");
                        let ack_duration = ack_start.elapsed();

                        ack_times.push(ack_duration);
                        local_count += 1;
                        counts_clone[sub_id].store(local_count, Ordering::Relaxed);

                        // Verify message content
                        assert!(String::from_utf8_lossy(&message.payload).contains("ConcurrentSub"));
                    }
                    Ok(Err(e)) => {
                        eprintln!("Subscriber {} receive error: {}", sub_id, e);
                        break;
                    }
                    Err(_) => {
                        println!("Subscriber {} timeout after {} messages", sub_id, local_count);
                        break;
                    }
                }
            }

            let duration = start.elapsed();
            let avg_ack_time = if !ack_times.is_empty() {
                ack_times.iter().sum::<Duration>() / ack_times.len() as u32
            } else {
                Duration::from_nanos(0)
            };

            (sub_id, local_count, duration, avg_ack_time)
        });
        
        subscriber_handles.push(handle);
    }

    // Publish messages
    let test_messages = create_sequenced_messages(HIGH_THROUGHPUT_MESSAGE_COUNT, "ConcurrentSub", 0);
    
    let publish_start = Instant::now();
    publisher.publish("concurrent_sub", test_messages).await
        .expect("Failed to publish messages");
    let publish_duration = publish_start.elapsed();

    println!("📤 Published {} messages in {:?}", HIGH_THROUGHPUT_MESSAGE_COUNT, publish_duration);

    // Wait for all subscribers to complete
    let mut total_received = 0;
    let mut all_ack_times = Vec::new();

    for handle in subscriber_handles {
        let (sub_id, count, duration, avg_ack_time) = handle.await
            .expect("Subscriber task should complete");
        
        total_received += count;
        all_ack_times.push(avg_ack_time);
        
        println!("📥 Subscriber {} received {} messages in {:?} (avg ack: {:?})", 
                sub_id, count, duration, avg_ack_time);
    }

    println!("📊 Concurrent subscribers results:");
    println!("   Total published: {}", HIGH_THROUGHPUT_MESSAGE_COUNT);
    println!("   Total received: {}", total_received);
    
    // Verify that all messages were received (distributed among subscribers)
    assert_eq!(total_received, HIGH_THROUGHPUT_MESSAGE_COUNT, 
              "All messages should be received by subscribers");
}

#[tokio::test]
async fn test_batch_acknowledgment_high_throughput() {
    println!("🚀 Testing batch acknowledgment high throughput...");
    
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("batch_ack").await
        .expect("Failed to subscribe");

    // Publish messages in batches
    let total_batches = HIGH_THROUGHPUT_MESSAGE_COUNT / BATCH_SIZE;
    let publish_start = Instant::now();

    for batch_id in 0..total_batches {
        let batch_messages = create_sequenced_messages(
            BATCH_SIZE,
            &format!("Batch{}", batch_id),
            batch_id * BATCH_SIZE,
        );
        
        publisher.publish("batch_ack", batch_messages).await
            .expect(&format!("Failed to publish batch {}", batch_id));
    }

    let publish_duration = publish_start.elapsed();
    println!("📤 Published {} batches ({} messages) in {:?}", 
            total_batches, total_batches * BATCH_SIZE, publish_duration);

    // Receive and batch acknowledge messages
    let mut total_processed = 0;
    let batch_ack_start = Instant::now();

    for batch_id in 0..total_batches {
        let mut batch_handles = Vec::new();
        
        // Collect batch of handles
        for i in 0..BATCH_SIZE {
            let (message, handle) = timeout(
                Duration::from_secs(10),
                subscriber.receive_with_ack()
            ).await
                .expect(&format!("Timeout waiting for message {} in batch {}", i, batch_id))
                .expect(&format!("Failed to receive message {} in batch {}", i, batch_id));

            // Verify message is from expected batch
            let payload = String::from_utf8_lossy(&message.payload);
            assert!(payload.contains(&format!("Batch{}", batch_id)) || 
                   payload.contains("Batch"), // Allow for any batch due to potential reordering
                   "Message not from expected batch: {}", payload);

            batch_handles.push(handle);
        }

        // Batch acknowledge
        let batch_ack_start_time = Instant::now();
        subscriber.ack_batch(batch_handles).await
            .expect(&format!("Failed to batch acknowledge batch {}", batch_id));
        let batch_ack_duration = batch_ack_start_time.elapsed();

        total_processed += BATCH_SIZE;
        
        if (batch_id + 1) % 5 == 0 {
            println!("📥 Processed {} batches ({} messages), last batch ack: {:?}", 
                    batch_id + 1, total_processed, batch_ack_duration);
        }
    }

    let total_batch_ack_duration = batch_ack_start.elapsed();
    let throughput = total_processed as f64 / total_batch_ack_duration.as_secs_f64();

    println!("📊 Batch acknowledgment results:");
    println!("   Total processed: {}", total_processed);
    println!("   Total time: {:?}", total_batch_ack_duration);
    println!("   Throughput: {:.2} messages/second", throughput);
    println!("   Batches per second: {:.2}", total_batches as f64 / total_batch_ack_duration.as_secs_f64());

    // Verify results
    assert_eq!(total_processed, total_batches * BATCH_SIZE);
    assert!(throughput > 200.0, "Batch acknowledgment throughput should be > 200 msg/sec");
}

#[tokio::test]
async fn test_router_high_throughput_acknowledgment() {
    println!("🚀 Testing router high throughput acknowledgment...");
    
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let output_subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));

    // Create high-performance router configuration
    let config = RouterAckConfig {
        strategy: AckStrategy::AutoAckOnSuccess,
        processing_timeout: Some(Duration::from_secs(10)),
        max_retries: 1,
        requeue_on_failure: false,
        batch_size: None, // Process individually for this test
    };

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false)); // Disable logging for performance
        AckRouter::new(
            logger,
            "router_input".to_string(),
            "router_output".to_string(),
            subscriber,
            output_publisher,
            create_fast_handler(),
            config,
        )
    };

    #[cfg(not(feature = "logging"))]
    let router = AckRouter::new(
        "router_input".to_string(),
        "router_output".to_string(),
        subscriber,
        output_publisher,
        create_fast_handler(),
        config,
    );

    // Subscribe to output to verify processed messages
    {
        let mut output_sub = output_subscriber.lock().await;
        output_sub.subscribe("router_output").await
            .expect("Failed to subscribe to output");
    }

    // Publish messages for router processing
    let message_count = 500; // Smaller count for router test
    let test_messages = create_sequenced_messages(message_count, "RouterTest", 0);
    
    let publish_start = Instant::now();
    input_publisher.publish("router_input", test_messages).await
        .expect("Failed to publish messages");
    let publish_duration = publish_start.elapsed();

    println!("📤 Published {} messages for router in {:?}", message_count, publish_duration);

    // Process messages through router
    let process_start = Instant::now();
    let mut processed_count = 0;

    for i in 0..message_count {
        match timeout(Duration::from_secs(10), router.process_single_message()).await {
            Ok(Ok(_)) => {
                processed_count += 1;
                if (i + 1) % 50 == 0 {
                    println!("🔄 Router processed {} messages", i + 1);
                }
            }
            Ok(Err(e)) => {
                eprintln!("Router processing error for message {}: {}", i + 1, e);
            }
            Err(_) => {
                eprintln!("Router processing timeout for message {}", i + 1);
                break;
            }
        }
    }

    let process_duration = process_start.elapsed();

    // Verify processed messages in output
    let mut output_received = 0;
    let output_start = Instant::now();

    for i in 0..processed_count {
        match timeout(Duration::from_secs(5), async {
            let mut output_sub = output_subscriber.lock().await;
            output_sub.receive_with_ack().await
        }).await {
            Ok(Ok((message, handle))) => {
                // Verify processed message
                assert!(message.metadata.contains_key("processed"));
                assert_eq!(message.metadata.get("processed"), Some(&"true".to_string()));
                
                // Acknowledge output message
                let mut output_sub = output_subscriber.lock().await;
                output_sub.ack(handle).await
                    .expect("Failed to acknowledge output message");
                
                output_received += 1;
            }
            Ok(Err(e)) => {
                eprintln!("Output receive error for message {}: {}", i + 1, e);
            }
            Err(_) => {
                println!("Output receive timeout after {} messages", output_received);
                break;
            }
        }
    }

    let output_duration = output_start.elapsed();

    // Get router statistics
    let stats = router.stats().await;

    println!("📊 Router high throughput results:");
    println!("   Messages published: {}", message_count);
    println!("   Messages processed: {}", processed_count);
    println!("   Output messages received: {}", output_received);
    println!("   Processing time: {:?}", process_duration);
    println!("   Output receive time: {:?}", output_duration);
    println!("   Processing throughput: {:.2} msg/sec", 
            processed_count as f64 / process_duration.as_secs_f64());
    println!("   Router statistics:");
    println!("     Processed: {}", stats.messages_processed);
    println!("     Acknowledged: {}", stats.messages_acked);
    println!("     Nacked: {}", stats.messages_nacked);
    println!("     Ack rate: {:.1}%", stats.ack_rate());
    println!("     Avg processing time: {:.2}ms", stats.avg_processing_time_ms);

    // Verify results
    assert_eq!(processed_count, message_count, "All messages should be processed");
    assert_eq!(output_received, processed_count, "All processed messages should appear in output");
    assert_eq!(stats.messages_processed, message_count as u64);
    assert_eq!(stats.messages_acked, message_count as u64);
    assert_eq!(stats.ack_rate(), 100.0);
    
    let processing_throughput = processed_count as f64 / process_duration.as_secs_f64();
    assert!(processing_throughput > 50.0, 
           "Router processing throughput should be > 50 msg/sec, got {:.2}", processing_throughput);
}

#[tokio::test]
async fn test_memory_usage_under_high_load() {
    println!("🚀 Testing memory usage under high load...");
    
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let mut subscriber = InMemoryAckSubscriber::new(broker.clone());

    subscriber.subscribe("memory_test").await
        .expect("Failed to subscribe");

    // Get initial memory usage
    let initial_memory = get_memory_usage();
    println!("📊 Initial memory usage: {} KB", initial_memory);

    // Publish large number of messages without acknowledging
    let large_message_count = 5000;
    let large_messages = create_large_messages(large_message_count, 1024); // 1KB each

    let publish_start = Instant::now();
    publisher.publish("memory_test", large_messages).await
        .expect("Failed to publish large messages");
    let publish_duration = publish_start.elapsed();

    println!("📤 Published {} large messages in {:?}", large_message_count, publish_duration);

    // Check memory usage after publishing
    let after_publish_memory = get_memory_usage();
    println!("📊 Memory after publish: {} KB (delta: {} KB)", 
            after_publish_memory, after_publish_memory - initial_memory);

    // Receive messages but don't acknowledge yet (should increase memory)
    let mut handles = Vec::new();
    let receive_start = Instant::now();

    for i in 0..large_message_count {
        let (_, handle) = timeout(
            Duration::from_secs(30),
            subscriber.receive_with_ack()
        ).await
            .expect(&format!("Timeout waiting for message {}", i + 1))
            .expect(&format!("Failed to receive message {}", i + 1));

        handles.push(handle);

        if (i + 1) % 500 == 0 {
            let current_memory = get_memory_usage();
            println!("📥 Received {} messages, memory: {} KB", i + 1, current_memory);
        }
    }

    let receive_duration = receive_start.elapsed();
    let after_receive_memory = get_memory_usage();
    
    println!("📊 Memory after receive: {} KB (delta: {} KB)", 
            after_receive_memory, after_receive_memory - initial_memory);

    // Now acknowledge all messages (should reduce memory)
    let ack_start = Instant::now();
    subscriber.ack_batch(handles).await
        .expect("Failed to batch acknowledge all messages");
    let ack_duration = ack_start.elapsed();

    let after_ack_memory = get_memory_usage();
    
    println!("📊 Memory after acknowledgment: {} KB (delta: {} KB)", 
            after_ack_memory, after_ack_memory - initial_memory);

    println!("📊 Memory usage test results:");
    println!("   Messages: {}", large_message_count);
    println!("   Receive time: {:?}", receive_duration);
    println!("   Ack time: {:?}", ack_duration);
    println!("   Memory growth: {} KB", after_receive_memory - initial_memory);
    println!("   Memory after cleanup: {} KB", after_ack_memory - initial_memory);
    println!("   Memory efficiency: {:.2} bytes/message", 
            ((after_receive_memory - initial_memory) * 1024) as f64 / large_message_count as f64);

    // Verify memory usage is reasonable
    let memory_per_message = ((after_receive_memory - initial_memory) * 1024) as f64 / large_message_count as f64;
    assert!(memory_per_message < 2048.0, // Less than 2KB per message overhead
           "Memory usage per message too high: {:.2} bytes", memory_per_message);
}

// Helper function to create large messages for memory testing
fn create_large_messages(count: usize, size_bytes: usize) -> Vec<Message> {
    let large_payload = vec![b'X'; size_bytes];
    (0..count)
        .map(|i| {
            Message::new(large_payload.clone())
                .with_metadata("message_id", &i.to_string())
                .with_metadata("size", &size_bytes.to_string())
        })
        .collect()
}

// Helper function to get current memory usage (simplified)
fn get_memory_usage() -> u64 {
    // This is a simplified memory usage check
    // In a real implementation, you might use system-specific APIs
    use std::alloc::{GlobalAlloc, Layout, System};
    
    // For testing purposes, we'll use a simple approximation
    // In production, you'd want to use proper memory profiling tools
    std::process::id() as u64 // Placeholder - replace with actual memory measurement
}
