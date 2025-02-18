use kincir::kafka::{KafkaPublisher, KafkaSubscriber};
use kincir::router::{Router, Logger, StdLogger};
use kincir::Message;
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Example configuration for Kafka
    let publisher = Arc::new(KafkaPublisher::new("localhost:9092"));
    let subscriber = Arc::new(KafkaSubscriber::new("localhost:9092", "example-group"));

    // Define message handler
    let handler = Arc::new(|msg: Message| {
        Box::pin(async move {
            // Example message transformation
            let mut processed_msg = msg;
            processed_msg.set_metadata("processed", "true");
            Ok(vec![processed_msg])
        })
    });

    // Create and run router
    let router = Router::new(
        logger,
        "input-topic".to_string(),
        "output-topic".to_string(),
        subscriber,
        publisher,
        handler,
    );

    router.run().await
}