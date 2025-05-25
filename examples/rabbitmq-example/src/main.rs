use kincir::logging::StdLogger;
use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::{HandlerFunc, Message, Router};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Get RabbitMQ connection string from environment variable or use default
    let rabbitmq_url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@rabbitmq:5672".to_string());

    // Example configuration for RabbitMQ
    let publisher = Arc::new(RabbitMQPublisher::new(&rabbitmq_url).await?);
    // subscriber needs to be Arc<Mutex<dyn Subscriber>> for the Router
    let rabbitmq_subscriber = RabbitMQSubscriber::new(&rabbitmq_url).await?;
    let subscriber = Arc::new(tokio::sync::Mutex::new(rabbitmq_subscriber));

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
        "input-exchange".to_string(),
        "output-exchange".to_string(),
        subscriber,
        publisher,
        handler,
    );

    router.run().await
}
