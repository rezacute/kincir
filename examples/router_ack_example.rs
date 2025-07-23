//! Router Acknowledgment Example
//!
//! This example demonstrates how to use the acknowledgment-aware router for reliable
//! message processing with automatic acknowledgment handling, error recovery, and
//! comprehensive statistics tracking.
//!
//! Run with: cargo run --example router_ack_example

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::memory::{InMemoryAckSubscriber, InMemoryBroker, InMemoryPublisher};
use kincir::router::{AckRouter, AckStrategy, RouterAckConfig};
use kincir::{Message, Publisher};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 Router Acknowledgment Example");
    println!("=================================");

    // Configuration
    let broker = Arc::new(InMemoryBroker::with_default_config());
    
    // Example 1: Basic acknowledgment-aware routing
    println!("\n" + "=".repeat(60).as_str());
    println!("📋 Example 1: Basic Acknowledgment-Aware Routing");
    println!("=".repeat(60));
    
    basic_ack_routing_example(broker.clone()).await?;

    // Example 2: Error handling and retry logic
    println!("\n" + "=".repeat(60).as_str());
    println!("📋 Example 2: Error Handling and Retry Logic");
    println!("=".repeat(60));
    
    error_handling_example(broker.clone()).await?;

    // Example 3: Different acknowledgment strategies
    println!("\n" + "=".repeat(60).as_str());
    println!("📋 Example 3: Different Acknowledgment Strategies");
    println!("=".repeat(60));
    
    acknowledgment_strategies_example(broker.clone()).await?;

    // Example 4: Processing timeout handling
    println!("\n" + "=".repeat(60).as_str());
    println!("📋 Example 4: Processing Timeout Handling");
    println!("=".repeat(60));
    
    timeout_handling_example(broker.clone()).await?;

    // Example 5: Statistics and monitoring
    println!("\n" + "=".repeat(60).as_str());
    println!("📋 Example 5: Statistics and Monitoring");
    println!("=".repeat(60));
    
    statistics_example(broker.clone()).await?;

    println!("\n🎉 All examples completed successfully!");
    println!("   The acknowledgment-aware router provides reliable message processing");
    println!("   with comprehensive error handling and monitoring capabilities.");

    Ok(())
}

async fn basic_ack_routing_example(
    broker: Arc<InMemoryBroker>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Setting up basic acknowledgment-aware router...");

    // Create components
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let output_subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));

    // Create a simple message processing handler
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            println!("  🔄 Processing message: {}", String::from_utf8_lossy(&msg.payload));
            
            // Simple transformation: add processed metadata and modify payload
            let processed_payload = format!("PROCESSED: {}", String::from_utf8_lossy(&msg.payload));
            let processed_msg = Message::new(processed_payload.into_bytes())
                .with_metadata("processed", "true")
                .with_metadata("processed_at", &chrono::Utc::now().to_rfc3339())
                .with_metadata("original_id", &msg.uuid);
            
            Ok(vec![processed_msg])
        })
    });

    // Create router with default configuration
    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(true, false)); // Enable info, disable debug
        AckRouter::with_default_config(
            logger,
            "input".to_string(),
            "output".to_string(),
            subscriber,
            output_publisher,
            handler,
        )
    };

    #[cfg(not(feature = "logging"))]
    let router = AckRouter::with_default_config(
        "input".to_string(),
        "output".to_string(),
        subscriber,
        output_publisher,
        handler,
    );

    // Subscribe to output topic to see processed messages
    {
        let mut output_sub = output_subscriber.lock().await;
        output_sub.subscribe("output").await?;
    }

    // Publish test messages
    let test_messages = vec![
        Message::new(b"Hello Router!".to_vec()).with_metadata("priority", "high"),
        Message::new(b"Message 2".to_vec()).with_metadata("priority", "normal"),
        Message::new(b"Message 3".to_vec()).with_metadata("priority", "low"),
    ];

    for (i, message) in test_messages.iter().enumerate() {
        input_publisher.publish("input", vec![message.clone()]).await?;
        println!("✅ Published message {}: {}", i + 1, String::from_utf8_lossy(&message.payload));
    }

    // Process messages one by one
    for i in 0..test_messages.len() {
        println!("📥 Processing message {}...", i + 1);
        let result = timeout(Duration::from_secs(5), router.process_single_message()).await;
        
        match result {
            Ok(Ok(_)) => println!("✅ Message {} processed successfully", i + 1),
            Ok(Err(e)) => println!("❌ Message {} processing failed: {}", i + 1, e),
            Err(_) => println!("⏰ Message {} processing timed out", i + 1),
        }

        // Check for processed message in output
        let mut output_sub = output_subscriber.lock().await;
        match timeout(Duration::from_millis(100), output_sub.receive_with_ack()).await {
            Ok(Ok((processed_msg, handle))) => {
                println!("  📤 Received processed message: {}", String::from_utf8_lossy(&processed_msg.payload));
                println!("     Metadata: {:?}", processed_msg.metadata);
                output_sub.ack(handle).await?;
            }
            _ => println!("  ⚠️  No processed message received (might be expected)"),
        }
    }

    // Display statistics
    let stats = router.stats().await;
    println!("\n📊 Processing Statistics:");
    println!("   Messages Processed: {}", stats.messages_processed);
    println!("   Messages Acknowledged: {}", stats.messages_acked);
    println!("   Messages Nacked: {}", stats.messages_nacked);
    println!("   Acknowledgment Rate: {:.1}%", stats.ack_rate());
    println!("   Average Processing Time: {:.2}ms", stats.avg_processing_time_ms);

    Ok(())
}

async fn error_handling_example(
    broker: Arc<InMemoryBroker>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Setting up error handling example...");

    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));

    // Handler that fails for messages containing "fail"
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            let payload_str = String::from_utf8_lossy(&msg.payload);
            println!("  🔄 Processing message: {}", payload_str);
            
            if payload_str.contains("fail") {
                println!("  ❌ Simulating processing failure");
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Simulated processing failure",
                )) as Box<dyn std::error::Error + Send + Sync>)
            } else {
                let processed_msg = msg.with_metadata("processed", "true");
                Ok(vec![processed_msg])
            }
        })
    });

    // Configure with retry logic
    let config = RouterAckConfig {
        strategy: AckStrategy::AutoAckOnSuccess,
        processing_timeout: Some(Duration::from_secs(5)),
        max_retries: 2,
        requeue_on_failure: false, // Don't requeue for this example
        batch_size: None,
    };

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(true, false));
        AckRouter::new(
            logger,
            "input".to_string(),
            "output".to_string(),
            subscriber,
            output_publisher,
            handler,
            config,
        )
    };

    #[cfg(not(feature = "logging"))]
    let router = AckRouter::new(
        "input".to_string(),
        "output".to_string(),
        subscriber,
        output_publisher,
        handler,
        config,
    );

    // Test messages - some will succeed, some will fail
    let test_messages = vec![
        Message::new(b"Success message 1".to_vec()),
        Message::new(b"This will fail".to_vec()),
        Message::new(b"Success message 2".to_vec()),
        Message::new(b"Another fail message".to_vec()),
    ];

    for (i, message) in test_messages.iter().enumerate() {
        input_publisher.publish("input", vec![message.clone()]).await?;
        println!("✅ Published message {}: {}", i + 1, String::from_utf8_lossy(&message.payload));
    }

    // Process all messages
    for i in 0..test_messages.len() {
        println!("📥 Processing message {}...", i + 1);
        let result = timeout(Duration::from_secs(5), router.process_single_message()).await;
        
        match result {
            Ok(Ok(_)) => println!("✅ Message {} handled successfully", i + 1),
            Ok(Err(e)) => println!("❌ Message {} handling failed: {}", i + 1, e),
            Err(_) => println!("⏰ Message {} processing timed out", i + 1),
        }
    }

    // Display error handling statistics
    let stats = router.stats().await;
    println!("\n📊 Error Handling Statistics:");
    println!("   Messages Processed: {}", stats.messages_processed);
    println!("   Messages Acknowledged: {}", stats.messages_acked);
    println!("   Messages Nacked: {}", stats.messages_nacked);
    println!("   Acknowledgment Rate: {:.1}%", stats.ack_rate());
    println!("   Negative Acknowledgment Rate: {:.1}%", stats.nack_rate());

    Ok(())
}

async fn acknowledgment_strategies_example(
    broker: Arc<InMemoryBroker>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Testing different acknowledgment strategies...");

    let strategies = vec![
        (AckStrategy::AutoAckOnSuccess, "Auto Ack on Success"),
        (AckStrategy::AlwaysAck, "Always Ack"),
        (AckStrategy::NeverAck, "Never Ack"),
    ];

    for (strategy, strategy_name) in strategies {
        println!("\n🔧 Testing strategy: {}", strategy_name);

        let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
        let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));

        // Handler that always fails for this test
        let handler = Arc::new(|_msg: Message| {
            Box::pin(async move {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Intentional failure for strategy testing",
                )) as Box<dyn std::error::Error + Send + Sync>)
            })
        });

        let config = RouterAckConfig {
            strategy: strategy.clone(),
            processing_timeout: Some(Duration::from_secs(5)),
            max_retries: 1,
            requeue_on_failure: false,
            batch_size: None,
        };

        #[cfg(feature = "logging")]
        let router = {
            use kincir::logging::StdLogger;
            let logger = Arc::new(StdLogger::new(false, false)); // Disable logging for cleaner output
            AckRouter::new(
                logger,
                "input".to_string(),
                "output".to_string(),
                subscriber,
                output_publisher,
                handler,
                config,
            )
        };

        #[cfg(not(feature = "logging"))]
        let router = AckRouter::new(
            "input".to_string(),
            "output".to_string(),
            subscriber,
            output_publisher,
            handler,
            config,
        );

        // Publish test message
        let test_message = Message::new(format!("Test message for {}", strategy_name).into_bytes());
        input_publisher.publish("input", vec![test_message]).await?;

        // Process message
        let result = timeout(Duration::from_secs(5), router.process_single_message()).await;
        match result {
            Ok(Ok(_)) => println!("  ✅ Message handled successfully"),
            Ok(Err(e)) => println!("  ❌ Message handling failed: {}", e),
            Err(_) => println!("  ⏰ Message processing timed out"),
        }

        // Display strategy-specific statistics
        let stats = router.stats().await;
        println!("  📊 Strategy Results:");
        println!("     Messages Processed: {}", stats.messages_processed);
        println!("     Messages Acknowledged: {}", stats.messages_acked);
        println!("     Messages Nacked: {}", stats.messages_nacked);
        
        match strategy {
            AckStrategy::AutoAckOnSuccess => {
                println!("     Expected: Nack on failure (processing failed)");
                assert_eq!(stats.messages_nacked, 1);
            }
            AckStrategy::AlwaysAck => {
                println!("     Expected: Ack even on failure");
                assert_eq!(stats.messages_acked, 1);
            }
            AckStrategy::NeverAck => {
                println!("     Expected: No ack/nack operations");
                assert_eq!(stats.messages_acked, 0);
                assert_eq!(stats.messages_nacked, 0);
            }
            AckStrategy::Manual => {
                println!("     Expected: Manual handling (not tested here)");
            }
        }
    }

    Ok(())
}

async fn timeout_handling_example(
    broker: Arc<InMemoryBroker>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Setting up timeout handling example...");

    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));

    // Handler that takes longer than the timeout
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            let payload_str = String::from_utf8_lossy(&msg.payload);
            println!("  🔄 Starting slow processing: {}", payload_str);
            
            // Simulate slow processing
            sleep(Duration::from_millis(300)).await;
            
            println!("  ✅ Slow processing completed");
            Ok(vec![msg.with_metadata("slow_processed", "true")])
        })
    });

    let config = RouterAckConfig {
        strategy: AckStrategy::AutoAckOnSuccess,
        processing_timeout: Some(Duration::from_millis(100)), // Short timeout
        max_retries: 1,
        requeue_on_failure: false,
        batch_size: None,
    };

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(true, false));
        AckRouter::new(
            logger,
            "input".to_string(),
            "output".to_string(),
            subscriber,
            output_publisher,
            handler,
            config,
        )
    };

    #[cfg(not(feature = "logging"))]
    let router = AckRouter::new(
        "input".to_string(),
        "output".to_string(),
        subscriber,
        output_publisher,
        handler,
        config,
    );

    // Publish test message
    let test_message = Message::new(b"Slow processing message".to_vec());
    input_publisher.publish("input", vec![test_message]).await?;
    println!("✅ Published slow processing message");

    // Process message (should timeout)
    println!("📥 Processing message (will timeout)...");
    let result = timeout(Duration::from_secs(5), router.process_single_message()).await;
    
    match result {
        Ok(Ok(_)) => println!("✅ Message handled successfully (timeout was handled)"),
        Ok(Err(e)) => println!("❌ Message handling failed: {}", e),
        Err(_) => println!("⏰ Router processing timed out"),
    }

    // Display timeout statistics
    let stats = router.stats().await;
    println!("\n📊 Timeout Handling Statistics:");
    println!("   Messages Processed: {}", stats.messages_processed);
    println!("   Messages Timed Out: {}", stats.messages_timed_out);
    println!("   Messages Nacked: {}", stats.messages_nacked);
    println!("   Expected: 1 timeout and 1 nack");

    Ok(())
}

async fn statistics_example(
    broker: Arc<InMemoryBroker>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📤 Setting up comprehensive statistics example...");

    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));

    // Handler with variable processing times
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            let payload_str = String::from_utf8_lossy(&msg.payload);
            
            // Variable processing time based on message content
            let processing_time = if payload_str.contains("fast") {
                Duration::from_millis(10)
            } else if payload_str.contains("slow") {
                Duration::from_millis(100)
            } else {
                Duration::from_millis(50)
            };
            
            sleep(processing_time).await;
            
            // Fail some messages for statistics
            if payload_str.contains("fail") {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Intentional failure for statistics",
                )) as Box<dyn std::error::Error + Send + Sync>)
            } else {
                Ok(vec![msg.with_metadata("processed", "true")])
            }
        })
    });

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false)); // Disable logging for cleaner output
        AckRouter::with_default_config(
            logger,
            "input".to_string(),
            "output".to_string(),
            subscriber,
            output_publisher,
            handler,
        )
    };

    #[cfg(not(feature = "logging"))]
    let router = AckRouter::with_default_config(
        "input".to_string(),
        "output".to_string(),
        subscriber,
        output_publisher,
        handler,
    );

    // Publish various test messages
    let test_messages = vec![
        Message::new(b"fast message 1".to_vec()),
        Message::new(b"normal message 1".to_vec()),
        Message::new(b"slow message 1".to_vec()),
        Message::new(b"fast message 2".to_vec()),
        Message::new(b"fail message 1".to_vec()),
        Message::new(b"normal message 2".to_vec()),
        Message::new(b"fail message 2".to_vec()),
        Message::new(b"slow message 2".to_vec()),
    ];

    for (i, message) in test_messages.iter().enumerate() {
        input_publisher.publish("input", vec![message.clone()]).await?;
        println!("✅ Published message {}: {}", i + 1, String::from_utf8_lossy(&message.payload));
    }

    // Process all messages
    println!("\n📥 Processing all messages...");
    for i in 0..test_messages.len() {
        let result = timeout(Duration::from_secs(5), router.process_single_message()).await;
        match result {
            Ok(Ok(_)) => print!("✅"),
            Ok(Err(_)) => print!("❌"),
            Err(_) => print!("⏰"),
        }
        if (i + 1) % 10 == 0 {
            println!();
        }
    }
    println!();

    // Display comprehensive statistics
    let stats = router.stats().await;
    println!("\n📊 Comprehensive Statistics:");
    println!("   Messages Processed: {}", stats.messages_processed);
    println!("   Messages Acknowledged: {}", stats.messages_acked);
    println!("   Messages Nacked: {}", stats.messages_nacked);
    println!("   Messages Timed Out: {}", stats.messages_timed_out);
    println!("   Max Retries Exceeded: {}", stats.messages_max_retries_exceeded);
    println!("   Acknowledgment Rate: {:.1}%", stats.ack_rate());
    println!("   Negative Acknowledgment Rate: {:.1}%", stats.nack_rate());
    println!("   Average Processing Time: {:.2}ms", stats.avg_processing_time_ms);

    // Reset statistics and verify
    println!("\n🔄 Resetting statistics...");
    router.reset_stats().await;
    let reset_stats = router.stats().await;
    println!("✅ Statistics reset - all counters should be zero:");
    println!("   Messages Processed: {}", reset_stats.messages_processed);
    println!("   Messages Acknowledged: {}", reset_stats.messages_acked);
    println!("   Messages Nacked: {}", reset_stats.messages_nacked);

    Ok(())
}
