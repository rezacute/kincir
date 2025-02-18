use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
use kincir::router::{Logger, Router, StdLogger};
use kincir::Message;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    let logger = Arc::new(StdLogger::new(true, true));

    // Example configuration for RabbitMQ
    let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
    let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);

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
        "input-exchange".to_string(),
        "output-exchange".to_string(),
        subscriber,
        publisher,
        handler,
    );

    router.run().await
}
