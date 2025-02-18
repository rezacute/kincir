use crate::{Message, Publisher, Subscriber, Router, Logger};
use async_trait::async_trait;
use std::sync::Arc;
use tokio;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_message_creation() {
        let payload = b"test payload".to_vec();
        let msg = Message::new(payload.clone());

        assert_eq!(msg.payload, payload);
        assert!(msg.uuid.len() > 0);
        assert!(msg.metadata.is_empty());
    }

    #[test]
    fn test_message_with_metadata() {
        let payload = b"test payload".to_vec();
        let msg = Message::new(payload)
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");

        assert_eq!(msg.metadata.get("key1").unwrap(), "value1");
        assert_eq!(msg.metadata.get("key2").unwrap(), "value2");
        assert_eq!(msg.metadata.len(), 2);
    }

    // Mock Publisher for testing
    #[derive(Default)]
    struct MockPublisher {
        published_messages: Arc<tokio::sync::Mutex<Vec<(String, Vec<Message>)>>>
    }

    #[async_trait]
    impl Publisher for MockPublisher {
        type Error = Box<dyn std::error::Error + Send + Sync>;

        async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
            self.published_messages.lock().await.push((topic.to_string(), messages));
            Ok(())
        }
    }

    // Mock Subscriber for testing
    #[derive(Default)]
    struct MockSubscriber {
        messages: Arc<tokio::sync::Mutex<Vec<Message>>>,
        subscribed_topics: Arc<tokio::sync::Mutex<Vec<String>>>
    }

    #[async_trait]
    impl Subscriber for MockSubscriber {
        type Error = Box<dyn std::error::Error + Send + Sync>;

        async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
            self.subscribed_topics.lock().await.push(topic.to_string());
            Ok(())
        }

        async fn receive(&self) -> Result<Message, Self::Error> {
            let messages = self.messages.lock().await;
            if messages.is_empty() {
                return Err("No messages available".into());
            }
            Ok(messages[0].clone())
        }
    }

    // Mock Logger for testing
    #[derive(Default)]
    struct MockLogger {
        logs: Arc<tokio::sync::Mutex<Vec<(String, String)>>>
    }

    #[async_trait]
    impl Logger for MockLogger {
        async fn info(&self, msg: &str) {
            self.logs.lock().await.push(("info".to_string(), msg.to_string()));
        }

        async fn error(&self, msg: &str) {
            self.logs.lock().await.push(("error".to_string(), msg.to_string()));
        }
    }

    #[tokio::test]
    async fn test_router_message_flow() {
        let logger = Arc::new(MockLogger::default());
        let publisher = Arc::new(MockPublisher::default());
        let subscriber = Arc::new(MockSubscriber::default());

        // Add a test message to the subscriber
        let test_message = Message::new(b"test data".to_vec())
            .with_metadata("original", "true");
        subscriber.messages.lock().await.push(test_message);

        // Create a handler that adds a processed flag
        let handler = Arc::new(|msg: Message| {
            Box::pin(async move {
                let mut processed_msg = msg;
                processed_msg.metadata.insert("processed".to_string(), "true".to_string());
                Ok(vec![processed_msg])
            })
        });

        let router = Router::new(
            logger.clone(),
            "input-topic".to_string(),
            "output-topic".to_string(),
            subscriber.clone(),
            publisher.clone(),
            handler
        );

        // Run the router for one message
        tokio::spawn(async move {
            let _ = router.run().await;
        });

        // Give some time for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify the message flow
        let published = publisher.published_messages.lock().await;
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].0, "output-topic");
        assert_eq!(published[0].1[0].metadata.get("processed").unwrap(), "true");

        // Verify logging
        let logs = logger.logs.lock().await;
        assert!(logs.iter().any(|(level, msg)| level == "info" && msg.contains("Starting router")));
    }
}