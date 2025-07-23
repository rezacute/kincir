use crate::{Message, Publisher, Subscriber, Router, Logger};
use async_trait::async_trait;
use std::sync::Arc;
use tokio;

#[cfg(test)]
mod kafka_tests;
#[cfg(test)]
mod rabbitmq_tests;
#[cfg(test)]
mod mqtt_tests; // Added this line
#[cfg(test)]
mod backend_unit_tests; // Added comprehensive backend unit tests

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
        messages: Arc<tokio::sync::Mutex<Vec<Message>>>, // Stores messages to be "received"
        actual_received_messages: Arc<tokio::sync::Mutex<Vec<Message>>>, // Stores messages "sent" to it by publish
        subscribed_topics: Arc<tokio::sync::Mutex<Vec<String>>>
    }
    
    impl MockSubscriber {
        fn new() -> Self {
            Self {
                messages: Arc::new(tokio::sync::Mutex::new(Vec::new())),
                actual_received_messages: Arc::new(tokio::sync::Mutex::new(Vec::new())),
                subscribed_topics: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }
    
        // Helper to preload messages for the receive method
        async fn preload_message(&self, msg: Message) {
            self.messages.lock().await.push(msg);
        }
    }

    #[async_trait]
    impl Subscriber for MockSubscriber {
        type Error = Box<dyn std::error::Error + Send + Sync>;

        async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
            self.subscribed_topics.lock().await.push(topic.to_string());
            Ok(())
        }

        // Changed to &mut self to align with the Subscriber trait
        async fn receive(&mut self) -> Result<Message, Self::Error> {
            let mut messages = self.messages.lock().await;
            if messages.is_empty() {
                // To prevent tight loops in tests if receive is called more than expected
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                return Err("No messages available in MockSubscriber".into());
            }
            // Simulate actual consumption
            let msg = messages.remove(0);
            self.actual_received_messages.lock().await.push(msg.clone());
            Ok(msg)
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
        // Instantiate MockSubscriber using new()
        let mock_subscriber = MockSubscriber::new();

        // Add a test message to the subscriber's message queue for it to "receive"
        let test_message = Message::new(b"test data".to_vec())
            .with_metadata("original", "true");
        mock_subscriber.preload_message(test_message.clone()).await; // Use helper

        // Wrap the MockSubscriber in Arc<Mutex<...>> for the Router
        let subscriber_for_router = Arc::new(tokio::sync::Mutex::new(mock_subscriber));

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
            subscriber_for_router.clone(), // Pass the Arc<Mutex<MockSubscriber>>
            publisher.clone(),
            handler
        );

        // Run the router. It should process one message.
        // The router's run is a loop, so we run it in a task and give it time to process.
        let router_handle = tokio::spawn(async move {
            // We expect the router to receive one message, process it, and publish it.
            // Then it would loop trying to receive again.
            // The MockSubscriber will return an error if receive is called when no messages are preloaded.
            // Let the router run and expect it to complete one cycle.
            if let Err(e) = router.run().await {
                // We might get "No messages available" if it loops, which is fine for this test after one message.
                if e.to_string().contains("No messages available in MockSubscriber") {
                    // This is an expected way for the loop to terminate in this test.
                } else {
                    panic!("Router run failed: {:?}", e);
                }
            }
        });

        // Give some time for processing the single message
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Verify the message flow: check what was published
        let published_messages = publisher.published_messages.lock().await;
        assert_eq!(published_messages.len(), 1, "Should have published one message");
        assert_eq!(published_messages[0].0, "output-topic", "Published to wrong topic");
        assert_eq!(published_messages[0].1.len(), 1, "Should have published one message in the batch");
        let processed_msg = &published_messages[0].1[0];
        assert_eq!(processed_msg.metadata.get("original").unwrap(), "true");
        assert_eq!(processed_msg.metadata.get("processed").unwrap(), "true");

        // Abort the router task as its work is done for this test
        router_handle.abort();

        // Verify logging
        let logs = logger.logs.lock().await;
        assert!(logs.iter().any(|(level, msg)| level == "info" && msg.contains("Starting router")));
    }
}