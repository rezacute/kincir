//! Message routing and logging functionality for the Kincir messaging system.
//!
//! This module provides the core routing capabilities that enable message processing
//! and forwarding between different topics/queues. It includes:
//!
//! - A flexible `Router` implementation for message handling and routing
//! - A `Logger` trait for customizable logging
//! - A standard logger implementation (`StdLogger`)
//! - Type definitions for message handler functions
//!
//! # Example
//!
//! ```rust,no_run
//! use kincir::router::{Router, Logger, StdLogger};
//! use kincir::Message;
//! use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
//! use std::sync::Arc;
//! use std::pin::Pin;
//! use std::future::Future;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let logger = Arc::new(StdLogger::new(true, true));
//!     let handler = Arc::new(|msg: Message| -> Pin<Box<dyn Future<Output = Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>>> + Send>> {
//!         Box::pin(async move {
//!             // Process message here
//!             Ok(vec![msg])
//!         })
//!     });
//!
//!     // Set up router with RabbitMQ backend
//!     let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
//!     let subscriber = Arc::new(RabbitMQSubscriber::new("amqp://localhost:5672").await?);
//!
//!     let router = Router::new(
//!         logger,
//!         "input-queue".to_string(),
//!         "output-queue".to_string(),
//!         subscriber,
//!         publisher,
//!         handler,
//!     );
//!
//!     router.run().await
//! }
//! ```

use crate::Message;
use async_trait::async_trait;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Defines the interface for logging operations in the router.
#[async_trait]
pub trait Logger: Send + Sync {
    /// Logs an informational message.
    async fn info(&self, msg: &str);
    /// Logs an error message.
    async fn error(&self, msg: &str);
}

/// A standard implementation of the Logger trait that writes to stdout/stderr.
///
/// This logger provides basic logging capabilities with configurable output levels.
pub struct StdLogger {
    /// Whether to show informational messages
    show_info: bool,
    /// Whether to show error messages
    show_error: bool,
}

impl StdLogger {
    /// Creates a new StdLogger with configurable output levels.
    ///
    /// # Arguments
    ///
    /// * `show_info` - Whether to display informational messages
    /// * `show_error` - Whether to display error messages
    pub fn new(show_info: bool, show_error: bool) -> Self {
        Self {
            show_info,
            show_error,
        }
    }
}

#[async_trait]
impl Logger for StdLogger {
    async fn info(&self, msg: &str) {
        if self.show_info {
            println!("[INFO] {}", msg);
        }
    }

    async fn error(&self, msg: &str) {
        if self.show_error {
            eprintln!("[ERROR] {}", msg);
        }
    }
}

/// Type alias for message handler functions.
///
/// Handler functions take a message as input and return a Future that resolves to
/// a Result containing a vector of processed messages or an error.
pub type HandlerFunc = Arc<
    dyn Fn(
            Message,
        ) -> Pin<
            Box<dyn Future<Output = Result<Vec<Message>, Box<dyn Error + Send + Sync>>> + Send>,
        > + Send
        + Sync,
>;

/// The main router component that manages message flow between topics/queues.
///
/// The Router is responsible for:
/// - Subscribing to input topics
/// - Processing messages using the provided handler
/// - Publishing processed messages to output topics
/// - Logging operations and errors
pub struct Router {
    logger: Arc<dyn Logger>,
    consume_topic: String,
    publish_topic: String,
    subscriber: Arc<dyn crate::Subscriber<Error = Box<dyn Error + Send + Sync>>>,
    publisher: Arc<dyn crate::Publisher<Error = Box<dyn Error + Send + Sync>>>,
    handler: HandlerFunc,
}

impl Router {
    /// Creates a new Router instance.
    ///
    /// # Arguments
    ///
    /// * `logger` - The logger implementation to use
    /// * `consume_topic` - The topic/queue to consume messages from
    /// * `publish_topic` - The topic/queue to publish processed messages to
    /// * `subscriber` - The subscriber implementation
    /// * `publisher` - The publisher implementation
    /// * `handler` - The message processing function
    pub fn new(
        logger: Arc<dyn Logger>,
        consume_topic: String,
        publish_topic: String,
        subscriber: Arc<dyn crate::Subscriber<Error = Box<dyn Error + Send + Sync>>>,
        publisher: Arc<dyn crate::Publisher<Error = Box<dyn Error + Send + Sync>>>,
        handler: HandlerFunc,
    ) -> Self {
        Self {
            logger,
            consume_topic,
            publish_topic,
            subscriber,
            publisher,
            handler,
        }
    }

    /// Starts the router's message processing loop.
    ///
    /// This method will:
    /// 1. Subscribe to the input topic
    /// 2. Continuously receive messages
    /// 3. Process messages using the handler
    /// 4. Publish processed messages to the output topic
    pub async fn run(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.logger.info("Starting router...").await;
        self.subscriber.subscribe(&self.consume_topic).await?;

        loop {
            match self.subscriber.receive().await {
                Ok(msg) => {
                    self.logger
                        .info(&format!("Received message: {}", msg.uuid))
                        .await;

                    match (self.handler)(msg).await {
                        Ok(processed_msgs) => {
                            if !processed_msgs.is_empty() {
                                self.publisher
                                    .publish(&self.publish_topic, processed_msgs)
                                    .await?
                            }
                        }
                        Err(e) => {
                            self.logger
                                .error(&format!("Error processing message: {}", e))
                                .await;
                        }
                    }
                }
                Err(e) => {
                    self.logger
                        .error(&format!("Error receiving message: {}", e))
                        .await;
                }
            }
        }
    }
}
