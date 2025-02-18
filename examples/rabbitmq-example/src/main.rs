use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::router::StdLogger;
use kincir::{Message, HandlerFunc, Router};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Example configuration for RabbitMQ
    let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
    let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);

    // Define message handler
    let handler: HandlerFunc = Arc::new(
        |msg: Message| {
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
        "input-exchange".to_string(),
        "output-exchange".to_string(),
        subscriber,
        publisher,
        handler,
    );

    router.run().await
}
