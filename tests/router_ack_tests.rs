//! Integration tests for router acknowledgment handling
//!
//! These tests verify the acknowledgment-aware router implementation works correctly
//! with different acknowledgment strategies, error handling, and statistics tracking.

use kincir::ack::{AckHandle, AckSubscriber};
use kincir::memory::{InMemoryAckHandle, InMemoryAckSubscriber, InMemoryBroker, InMemoryPublisher};
use kincir::router::{AckRouter, AckStrategy, RouterAckConfig, RouterAckStats};
use kincir::{Message, Publisher};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

// Helper function to create a test message
fn create_test_message(content: &str) -> Message {
    Message::new(content.as_bytes().to_vec())
        .with_metadata("test", "true")
        .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339())
}

// Helper function to create a test handler that always succeeds
fn create_success_handler() -> kincir::router::HandlerFunc {
    Arc::new(|msg: Message| {
        Box::pin(async move {
            // Simple transformation: add processed metadata
            let processed_msg = msg.with_metadata("processed", "true");
            Ok(vec![processed_msg])
        })
    })
}

// Helper function to create a test handler that always fails
fn create_failure_handler() -> kincir::router::HandlerFunc {
    Arc::new(|_msg: Message| {
        Box::pin(async move {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Simulated processing failure",
            )) as Box<dyn std::error::Error + Send + Sync>)
        })
    })
}

// Helper function to create a test handler that fails conditionally
fn create_conditional_handler() -> kincir::router::HandlerFunc {
    Arc::new(|msg: Message| {
        Box::pin(async move {
            // Fail if message contains "fail" in payload
            let payload_str = String::from_utf8_lossy(&msg.payload);
            if payload_str.contains("fail") {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Conditional processing failure",
                )) as Box<dyn std::error::Error + Send + Sync>)
            } else {
                let processed_msg = msg.with_metadata("processed", "true");
                Ok(vec![processed_msg])
            }
        })
    })
}

#[tokio::test]
async fn test_router_ack_config_default() {
    let config = RouterAckConfig::default();
    assert_eq!(config.strategy, AckStrategy::AutoAckOnSuccess);
    assert_eq!(config.processing_timeout, Some(Duration::from_secs(30)));
    assert_eq!(config.max_retries, 3);
    assert!(config.requeue_on_failure);
    assert_eq!(config.batch_size, None);
}

#[tokio::test]
async fn test_router_ack_stats_calculations() {
    let mut stats = RouterAckStats::default();
    
    // Test initial state
    assert_eq!(stats.ack_rate(), 0.0);
    assert_eq!(stats.nack_rate(), 0.0);
    assert_eq!(stats.avg_processing_time_ms, 0.0);

    // Update with some data
    stats.messages_processed = 10;
    stats.messages_acked = 8;
    stats.messages_nacked = 2;
    stats.update_processing_time(Duration::from_millis(100));
    stats.update_processing_time(Duration::from_millis(200));

    assert_eq!(stats.ack_rate(), 80.0);
    assert_eq!(stats.nack_rate(), 20.0);
    assert!(stats.avg_processing_time_ms > 0.0);
}

#[tokio::test]
async fn test_ack_router_creation_with_default_config() {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let handler = create_success_handler();

    #[cfg(feature = "logging")]
    {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false)); // Disable logging for tests
        
        let router = AckRouter::with_default_config(
            logger,
            "input".to_string(),
            "output".to_string(),
            subscriber,
            publisher,
            handler,
        );

        let stats = router.stats().await;
        assert_eq!(stats.messages_processed, 0);
    }

    #[cfg(not(feature = "logging"))]
    {
        let router = AckRouter::with_default_config(
            "input".to_string(),
            "output".to_string(),
            subscriber,
            publisher,
            handler,
        );

        let stats = router.stats().await;
        assert_eq!(stats.messages_processed, 0);
    }
}

#[tokio::test]
async fn test_ack_router_with_custom_config() {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let handler = create_success_handler();

    let config = RouterAckConfig {
        strategy: AckStrategy::AlwaysAck,
        processing_timeout: Some(Duration::from_secs(10)),
        max_retries: 5,
        requeue_on_failure: false,
        batch_size: Some(10),
    };

    #[cfg(feature = "logging")]
    {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false));
        
        let router = AckRouter::new(
            logger,
            "input".to_string(),
            "output".to_string(),
            subscriber,
            publisher,
            handler,
            config.clone(),
        );

        // Test that config was applied
        let stats = router.stats().await;
        assert_eq!(stats.messages_processed, 0);
    }

    #[cfg(not(feature = "logging"))]
    {
        let router = AckRouter::new(
            "input".to_string(),
            "output".to_string(),
            subscriber,
            publisher,
            handler,
            config.clone(),
        );

        let stats = router.stats().await;
        assert_eq!(stats.messages_processed, 0);
    }
}

#[tokio::test]
async fn test_ack_router_successful_processing() {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let handler = create_success_handler();

    let config = RouterAckConfig {
        strategy: AckStrategy::AutoAckOnSuccess,
        processing_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false));
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

    // Publish a test message to input topic
    let test_message = create_test_message("Hello Router!");
    input_publisher.publish("input", vec![test_message]).await
        .expect("Failed to publish test message");

    // Process a single message
    let result = timeout(Duration::from_secs(2), router.process_single_message()).await;
    assert!(result.is_ok(), "Message processing timed out");
    assert!(result.unwrap().is_ok(), "Message processing failed");

    // Check statistics
    let stats = router.stats().await;
    assert_eq!(stats.messages_processed, 1);
    assert_eq!(stats.messages_acked, 1);
    assert_eq!(stats.messages_nacked, 0);
    assert_eq!(stats.ack_rate(), 100.0);
}

#[tokio::test]
async fn test_ack_router_processing_failure() {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let handler = create_failure_handler();

    let config = RouterAckConfig {
        strategy: AckStrategy::AutoAckOnSuccess,
        processing_timeout: Some(Duration::from_secs(5)),
        max_retries: 1,
        requeue_on_failure: false, // Don't requeue for this test
        ..Default::default()
    };

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false));
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

    // Publish a test message to input topic
    let test_message = create_test_message("This will fail");
    input_publisher.publish("input", vec![test_message]).await
        .expect("Failed to publish test message");

    // Process a single message (should fail)
    let result = timeout(Duration::from_secs(2), router.process_single_message()).await;
    assert!(result.is_ok(), "Message processing timed out");
    assert!(result.unwrap().is_ok(), "Router should handle processing failures gracefully");

    // Check statistics
    let stats = router.stats().await;
    assert_eq!(stats.messages_processed, 1);
    assert_eq!(stats.messages_acked, 0);
    assert_eq!(stats.messages_nacked, 1);
    assert_eq!(stats.nack_rate(), 100.0);
}

#[tokio::test]
async fn test_ack_router_conditional_processing() {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let handler = create_conditional_handler();

    let config = RouterAckConfig {
        strategy: AckStrategy::AutoAckOnSuccess,
        processing_timeout: Some(Duration::from_secs(5)),
        max_retries: 1,
        requeue_on_failure: false,
        ..Default::default()
    };

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false));
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

    // Publish a successful message
    let success_message = create_test_message("This will succeed");
    input_publisher.publish("input", vec![success_message]).await
        .expect("Failed to publish success message");

    // Process successful message
    let result = timeout(Duration::from_secs(2), router.process_single_message()).await;
    assert!(result.is_ok() && result.unwrap().is_ok());

    // Publish a failing message
    let fail_message = create_test_message("This will fail");
    input_publisher.publish("input", vec![fail_message]).await
        .expect("Failed to publish fail message");

    // Process failing message
    let result = timeout(Duration::from_secs(2), router.process_single_message()).await;
    assert!(result.is_ok() && result.unwrap().is_ok());

    // Check statistics
    let stats = router.stats().await;
    assert_eq!(stats.messages_processed, 2);
    assert_eq!(stats.messages_acked, 1);
    assert_eq!(stats.messages_nacked, 1);
    assert_eq!(stats.ack_rate(), 50.0);
    assert_eq!(stats.nack_rate(), 50.0);
}

#[tokio::test]
async fn test_ack_router_always_ack_strategy() {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let handler = create_failure_handler(); // Always fails

    let config = RouterAckConfig {
        strategy: AckStrategy::AlwaysAck, // Always acknowledge even on failure
        processing_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false));
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

    // Publish a test message
    let test_message = create_test_message("This will fail but be acked");
    input_publisher.publish("input", vec![test_message]).await
        .expect("Failed to publish test message");

    // Process message (should fail but still be acknowledged due to AlwaysAck strategy)
    let result = timeout(Duration::from_secs(2), router.process_single_message()).await;
    assert!(result.is_ok() && result.unwrap().is_ok());

    // Check statistics - should show ack even though processing failed
    let stats = router.stats().await;
    assert_eq!(stats.messages_processed, 1);
    assert_eq!(stats.messages_acked, 1); // Should be acked due to AlwaysAck strategy
    assert_eq!(stats.ack_rate(), 100.0);
}

#[tokio::test]
async fn test_ack_router_never_ack_strategy() {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let handler = create_success_handler(); // Always succeeds

    let config = RouterAckConfig {
        strategy: AckStrategy::NeverAck, // Never acknowledge even on success
        processing_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false));
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

    // Publish a test message
    let test_message = create_test_message("This will succeed but not be acked");
    input_publisher.publish("input", vec![test_message]).await
        .expect("Failed to publish test message");

    // Process message (should succeed but not be acknowledged due to NeverAck strategy)
    let result = timeout(Duration::from_secs(2), router.process_single_message()).await;
    assert!(result.is_ok() && result.unwrap().is_ok());

    // Check statistics - should show no acks even though processing succeeded
    let stats = router.stats().await;
    assert_eq!(stats.messages_processed, 1);
    assert_eq!(stats.messages_acked, 0); // Should not be acked due to NeverAck strategy
    assert_eq!(stats.messages_nacked, 0);
    assert_eq!(stats.ack_rate(), 0.0);
}

#[tokio::test]
async fn test_ack_router_processing_timeout() {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    
    // Handler that takes longer than the timeout
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            sleep(Duration::from_millis(200)).await; // Longer than timeout
            Ok(vec![msg])
        })
    });

    let config = RouterAckConfig {
        strategy: AckStrategy::AutoAckOnSuccess,
        processing_timeout: Some(Duration::from_millis(100)), // Short timeout
        max_retries: 1,
        requeue_on_failure: false,
        ..Default::default()
    };

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false));
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

    // Publish a test message
    let test_message = create_test_message("This will timeout");
    input_publisher.publish("input", vec![test_message]).await
        .expect("Failed to publish test message");

    // Process message (should timeout)
    let result = timeout(Duration::from_secs(2), router.process_single_message()).await;
    assert!(result.is_ok() && result.unwrap().is_ok());

    // Check statistics - should show timeout
    let stats = router.stats().await;
    assert_eq!(stats.messages_processed, 1);
    assert_eq!(stats.messages_timed_out, 1);
    assert_eq!(stats.messages_nacked, 1); // Timed out messages are nacked
}

#[tokio::test]
async fn test_ack_router_stats_reset() {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
    let subscriber = Arc::new(Mutex::new(InMemoryAckSubscriber::new(broker.clone())));
    let handler = create_success_handler();

    #[cfg(feature = "logging")]
    let router = {
        use kincir::logging::StdLogger;
        let logger = Arc::new(StdLogger::new(false, false));
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

    // Process a message to generate some stats
    let test_message = create_test_message("Test message");
    input_publisher.publish("input", vec![test_message]).await
        .expect("Failed to publish test message");

    let result = timeout(Duration::from_secs(2), router.process_single_message()).await;
    assert!(result.is_ok() && result.unwrap().is_ok());

    // Verify stats are not zero
    let stats = router.stats().await;
    assert_eq!(stats.messages_processed, 1);
    assert_eq!(stats.messages_acked, 1);

    // Reset stats
    router.reset_stats().await;

    // Verify stats are reset
    let stats = router.stats().await;
    assert_eq!(stats.messages_processed, 0);
    assert_eq!(stats.messages_acked, 0);
    assert_eq!(stats.messages_nacked, 0);
    assert_eq!(stats.ack_rate(), 0.0);
}

// Note: Additional integration tests for batch processing and full router.run() 
// would require more complex setup and potentially running in separate tasks.
// These tests focus on the core acknowledgment logic and statistics tracking.
