//! Integration tests for NATS backend

#[cfg(test)]
mod tests {
    use kincir::nats::{NatsError, NatsPublisher, NatsSubscriber};
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
