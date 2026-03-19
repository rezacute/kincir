//! Integration tests for middleware framework

use kincir::middleware::{
    CorrelationMiddleware, LoggingMiddleware, Middleware, MiddlewareChain, MiddlewareContext,
    RetryMiddleware,
};
use kincir::Message;

#[tokio::test]
async fn test_middleware_integration_full_flow() {
    // Create middleware chain
    let chain = MiddlewareChain::new()
        .add(LoggingMiddleware::new("Test"))
        .add(CorrelationMiddleware::new())
        .add(RetryMiddleware::new(3));

    let ctx = MiddlewareContext::new("orders")
        .with_metadata("source", "integration-test");

    // Test publish flow
    let mut messages = vec![
        Message::new(b"Order #1".to_vec()),
        Message::new(b"Order #2".to_vec()),
    ];

    chain.before_publish(&ctx, &mut messages).await;
    chain.after_publish(&ctx, &messages).await;

    // Verify correlation IDs were added
    for msg in &messages {
        assert!(
            msg.metadata.contains_key("correlation-id"),
            "Message should have correlation ID"
        );
    }

    // Test subscribe flow
    chain.before_subscribe(&ctx).await;
    chain.after_subscribe(&ctx).await;

    // Test receive flow
    chain.before_receive(&ctx).await;
    let received_msg = Message::new(b"Received order".to_vec());
    chain.after_receive(&ctx, &received_msg).await;
}

#[tokio::test]
async fn test_multiple_correlation_ids() {
    let mw = CorrelationMiddleware::new();
    let ctx = MiddlewareContext::new("test");

    // First batch
    let mut batch1 = vec![Message::new(b"msg1".to_vec())];
    mw.before_publish(&ctx, &mut batch1).await;
    let corr_id1 = batch1[0].metadata.get("correlation-id").cloned();

    // Second batch - should get DIFFERENT correlation ID
    let mut batch2 = vec![Message::new(b"msg2".to_vec())];
    mw.before_publish(&ctx, &mut batch2).await;
    let corr_id2 = batch2[0].metadata.get("correlation-id").cloned();

    assert_ne!(corr_id1, corr_id2, "Different batches should have different correlation IDs");
}

#[tokio::test]
async fn test_middleware_chain_order() {
    // Create a simple test by using the middleware directly
    let logging = LoggingMiddleware::new("OrderTest");
    let correlation = CorrelationMiddleware::new();

    let chain = MiddlewareChain::new()
        .add(logging)
        .add(correlation);

    let ctx = MiddlewareContext::new("order-topic");
    let mut messages = vec![Message::new(b"test".to_vec())];

    // Execute
    chain.before_publish(&ctx, &mut messages).await;

    // Verify correlation was added after (in chain order)
    assert!(messages[0].metadata.contains_key("correlation-id"));
}

#[test]
fn test_middleware_context_clone() {
    let ctx = MiddlewareContext::new("test")
        .with_metadata("key", "value");

    let cloned = ctx.clone();

    assert_eq!(cloned.topic, "test");
    assert_eq!(cloned.metadata.get("key"), Some(&"value".to_string()));
}

#[test]
fn test_retry_config() {
    let config = RetryMiddleware::new(5).config().clone();
    assert_eq!(config.max_retries, 5);
}

#[tokio::test]
async fn test_empty_chain_operations() {
    let chain = MiddlewareChain::new();

    let ctx = MiddlewareContext::new("test");
    let mut messages = vec![Message::new(b"test".to_vec())];

    // All these should be no-ops with empty chain
    chain.before_publish(&ctx, &mut messages).await;
    chain.after_publish(&ctx, &messages).await;
    chain.before_subscribe(&ctx).await;
    chain.after_subscribe(&ctx).await;
    chain.before_receive(&ctx).await;
    chain.after_receive(&ctx, &messages[0]).await;
}

#[tokio::test]
async fn test_custom_header_key() {
    let mw = CorrelationMiddleware::new().with_header_key("x-request-id");
    let ctx = MiddlewareContext::new("test");
    let mut messages = vec![Message::new(b"test".to_vec())];

    mw.before_publish(&ctx, &mut messages).await;

    assert!(messages[0].metadata.contains_key("x-request-id"));
    assert!(!messages[0].metadata.contains_key("correlation-id"));
}
