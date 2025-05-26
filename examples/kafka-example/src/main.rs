use kincir::kafka::{KafkaPublisher, KafkaSubscriber};
use kincir::logging::StdLogger;
use kincir::{HandlerFunc, Message, Router, Publisher, Subscriber};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Set up channels for Kafka communication
    let (_tx, rx) = mpsc::channel(100); // tx is not used by the new KafkaPublisher

    // Get Kafka broker address from environment variable or use default
    let kafka_broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "kafka:9092".to_string());

    // Example configuration for Kafka
    let publisher_result = KafkaPublisher::new(vec![kafka_broker.clone()]);
    let publisher: Arc<dyn Publisher<Error = Box<dyn std::error::Error + Send + Sync>>> = match publisher_result {
        Ok(p) => Arc::new(p),
        Err(e) => {
            // Use logger if available, otherwise print to stderr
            #[cfg(feature = "logging")] 
            logger.error(&format!("Failed to create KafkaPublisher: {:?}", e)).await;
            #[cfg(not(feature = "logging"))]
            eprintln!("Failed to create KafkaPublisher: {:?}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
        }
    };
    
    // KafkaSubscriber needs to be wrapped in Arc<Mutex<...>> for the Router
    let kafka_subscriber = KafkaSubscriber::new(
        vec![kafka_broker],
        "example-group".to_string(),
        rx, 
        logger.clone(), 
    );
    let subscriber: Arc<tokio::sync::Mutex<dyn Subscriber<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>> 
        = Arc::new(tokio::sync::Mutex::new(kafka_subscriber));


    // Define message handler
    let handler: HandlerFunc = Arc::new(|msg: Message| {
        Box::pin(async move {
            // Example message transformation
            let processed_msg = msg.with_metadata("processed", "true");
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

    router.run().await?;
    Ok(())
}
