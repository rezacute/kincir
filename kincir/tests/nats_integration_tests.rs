//! Integration tests for NATS backend

#[cfg(test)]
mod tests {
    use kincir::nats::NatsError;
    use kincir::Message;

    #[test]
    fn test_nats_error_display() {
        let err = NatsError::ConnectionError("connection failed".to_string());
        assert_eq!(err.to_string(), "NATS connection error: connection failed");
        
        let err = NatsError::PublishError("publish failed".to_string());
        assert_eq!(err.to_string(), "NATS publish error: publish failed");
        
        let err = NatsError::SubscribeError("subscribe failed".to_string());
        assert_eq!(err.to_string(), "NATS subscribe error: subscribe failed");
        
        let err = NatsError::ReceiveError("receive failed".to_string());
        assert_eq!(err.to_string(), "NATS receive error: receive failed");
    }

    #[test]
    fn test_message_creation() {
        let payload = b"test message".to_vec();
        let message = Message::new(payload.clone());
        
        assert!(!message.uuid.is_empty());
        assert_eq!(message.payload, payload);
    }
}


/// Tests that require a running NATS server. Enabled with the
/// `broker-integration` feature so they are compile-checked but do not run in
/// the default test suite. Run with:
///   cargo test -p kincir --features broker-integration --test nats_integration_tests
#[cfg(feature = "broker-integration")]
mod broker_tests {
    use kincir::nats::{NatsPublisher, NatsSubscriber};
    use kincir::{Message, Publisher, Subscriber};
    use std::time::Duration;
    use tokio::time::timeout;

    fn nats_url() -> String {
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string())
    }

    #[tokio::test]
    async fn test_nats_publish_subscribe_roundtrip() {
        let url = nats_url();
        let publisher = NatsPublisher::new(&url).await.expect("connect publisher");
        let mut subscriber = NatsSubscriber::new(&url).await.expect("connect subscriber");

        let subject = "kincir.test.roundtrip";
        subscriber.subscribe(subject).await.expect("subscribe");

        // Give the subscription a moment to register on the server before publishing.
        tokio::time::sleep(Duration::from_millis(100)).await;

        let payload = b"hello nats".to_vec();
        publisher
            .publish(subject, vec![Message::new(payload.clone())])
            .await
            .expect("publish");

        let received = timeout(Duration::from_secs(5), subscriber.receive())
            .await
            .expect("receive timed out")
            .expect("receive failed");

        assert_eq!(received.payload, payload);
        assert_eq!(
            received.metadata.get("nats_subject").map(|s| s.as_str()),
            Some(subject)
        );
    }

    #[tokio::test]
    async fn test_nats_receive_without_subscribe_errors() {
        let url = nats_url();
        let mut subscriber = NatsSubscriber::new(&url).await.expect("connect subscriber");

        // Receiving before subscribing should return an error, not panic.
        assert!(subscriber.receive().await.is_err());
    }
}
