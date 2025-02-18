use kincir::kafka::{KafkaPublisher, KafkaSubscriber};
use kincir::router::{Logger, Router, StdLogger};
use kincir::Message;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Set up channels for Kafka communication
    let (tx, rx) = mpsc::channel(100);

    // Example configuration for Kafka
    let publisher = Arc::new(KafkaPublisher::new(
        vec!["localhost:9092".to_string()],
        tx,
        logger.clone(),
    ));

    let subscriber = Arc::new(KafkaSubscriber::new(
        vec!["localhost:9092".to_string()],
        "example-group".to_string(),
        rx,
        logger.clone(),
    ));

    // Define message handler
    let handler = Arc::new(
        |msg: Message| -> Pin<
            Box<
                dyn Future<Output = Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>>>
                    + Send,
            >,
        > {
            Box::pin(async move {
                // Example message transformation
                let processed_msg = msg.with_metadata("processed", "true");
                Ok(vec![processed_msg])
            })
        },
    );

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
