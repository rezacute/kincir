//! Message routing functionality for the Kincir messaging system.
//!
//! This module provides the core routing capabilities that enable message processing
//! and forwarding between different topics/queues. It includes:
//!
//! - A flexible `Router` implementation for message handling and routing
//! - Type definitions for message handler functions
//! - Acknowledgment-aware routing with `AckRouter` for reliable message processing
//!
//! When the "logging" feature is enabled, it also includes integration with the
//! `Logger` trait from the logging module.
//!
//! # Example
//!
//! ```rust,no_run
//! use kincir::router::Router;
//! use kincir::Message;
//! use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
//! use std::sync::Arc;
//! use std::pin::Pin;
//! use std::future::Future;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! # // The following part will only be compiled when the "logging" feature is enabled
//! # #[cfg(feature = "logging")]
//! # {
//! # use kincir::logging::{Logger, StdLogger};
//! # let logger = Arc::new(StdLogger::new(true, true));
//! # }
//!     let handler = Arc::new(|msg: Message| -> Pin<Box<dyn Future<Output = Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>>> + Send>> {
//!         Box::pin(async move {
//!             // Process message here
//!             Ok(vec![msg])
//!         })
//!     });
//!
//!     // Set up router with RabbitMQ backend
//!     let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
//!     let subscriber_instance = RabbitMQSubscriber::new("amqp://localhost:5672").await?;
//!
//! # // Create the router differently based on feature flags
//! # #[cfg(feature = "logging")]
//! # {
//!     // With the "logging" feature enabled, include a logger
//! # use kincir::logging::Logger;
//! # use tokio::sync::Mutex;
//! # let logger = Arc::new(kincir::logging::StdLogger::new(true, true));
//!     let router = Router::new(
//!         logger,
//!         "input-queue".to_string(),
//!         "output-queue".to_string(),
//!         Arc::new(Mutex::new(subscriber_instance)),
//!         publisher,
//!         handler,
//!     );
//!     
//!     // Run the router (with logging)
//!     router.run().await
//! # }
//! # #[cfg(not(feature = "logging"))]
//! # {
//!     // Without the "logging" feature, don't include a logger
//! # use tokio::sync::Mutex;
//!     let router = Router::new(
//!         "input-queue".to_string(),
//!         "output-queue".to_string(),
//!         Arc::new(Mutex::new(subscriber_instance)),
//!         publisher,
//!         handler,
//!     );
//!     
//!     // Run the router (without logging)
//!     router.run().await
//! # }
//! # }

pub mod ack;

use crate::Message;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[cfg(feature = "logging")]
use crate::logging::Logger;

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

/// The Router struct handles message flow between publishers and subscribers.
///
/// # Example
///
/// ```rust,no_run
/// use kincir::rabbitmq::{RabbitMQPublisher, RabbitMQSubscriber};
/// use kincir::router::Router;
/// use kincir::Message;
/// use std::sync::Arc;
/// use std::pin::Pin;
/// use std::future::Future;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// # // The following part will only be compiled when the "logging" feature is enabled
/// # #[cfg(feature = "logging")]
/// # {
/// # use kincir::logging::{Logger, StdLogger};
/// # let logger = Arc::new(StdLogger::new(true, true));
/// # }
///     let handler = Arc::new(|msg: Message| -> Pin<Box<dyn Future<Output = Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>>> + Send>> {
///         Box::pin(async move {
///             // Process message here
///             Ok(vec![msg])
///         })
///     });
///
///     // Set up router with RabbitMQ backend
///     let publisher = Arc::new(RabbitMQPublisher::new("amqp://localhost:5672").await?);
///     let subscriber_instance = RabbitMQSubscriber::new("amqp://localhost:5672").await?;
///
/// # // Create the router differently based on feature flags
/// # #[cfg(feature = "logging")]
/// # {
///     // With the "logging" feature enabled, include a logger
/// # use kincir::logging::Logger;
/// # use tokio::sync::Mutex;
/// # let logger = Arc::new(kincir::logging::StdLogger::new(true, true));
///     let router = Router::new(
///         logger,
///         "input-queue".to_string(),
///         "output-queue".to_string(),
///         Arc::new(Mutex::new(subscriber_instance)),
///         publisher,
///         handler,
///     );
///     
///     // Run the router (with logging)
///     router.run().await
/// # }
/// # #[cfg(not(feature = "logging"))]
/// # {
///     // Without the "logging" feature, don't include a logger
/// # use tokio::sync::Mutex;
///     let router = Router::new(
///         "input-queue".to_string(),
///         "output-queue".to_string(),
///         Arc::new(Mutex::new(subscriber_instance)),
///         publisher,
///         handler,
///     );
///     
///     // Run the router (without logging)
///     router.run().await
/// # }
/// # }
use tokio::sync::Mutex; // Add this import

#[cfg(feature = "logging")]
pub struct Router {
    logger: Arc<dyn Logger>,
    consume_topic: String,
    publish_topic: String,
    subscriber:
        Arc<Mutex<dyn crate::Subscriber<Error = Box<dyn Error + Send + Sync>> + Send + Sync>>,
    publisher: Arc<dyn crate::Publisher<Error = Box<dyn Error + Send + Sync>>>,
    handler: HandlerFunc,
}

#[cfg(not(feature = "logging"))]
pub struct Router {
    consume_topic: String,
    publish_topic: String,

    subscriber:
        Arc<Mutex<dyn crate::Subscriber<Error = Box<dyn Error + Send + Sync>> + Send + Sync>>,

    publisher: Arc<dyn crate::Publisher<Error = Box<dyn Error + Send + Sync>>>,
    handler: HandlerFunc,
}

#[cfg(feature = "logging")]
impl Router {
    /// Creates a new Router instance with logging.
    ///
    /// # Arguments
    ///
    /// * `logger` - The logger implementation to use
    /// * `consume_topic` - The topic/queue to consume messages from
    /// * `publish_topic` - The topic/queue to publish processed messages to
    /// * `subscriber` - The subscriber implementation (wrapped in Arc<Mutex<...>>)
    /// * `publisher` - The publisher implementation
    /// * `handler` - The message processing function
    pub fn new(
        logger: Arc<dyn Logger>,
        consume_topic: String,
        publish_topic: String,
        subscriber: Arc<
            Mutex<dyn crate::Subscriber<Error = Box<dyn Error + Send + Sync>> + Send + Sync>,
        >,
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
        // Lock the subscriber to call subscribe
        let subscriber_guard_for_subscribe = self.subscriber.lock().await; // Removed mut
        subscriber_guard_for_subscribe
            .subscribe(&self.consume_topic)
            .await?;
        drop(subscriber_guard_for_subscribe); // Release lock after subscribe

        loop {
            // Lock the subscriber for each receive operation
            let mut subscriber_guard = self.subscriber.lock().await;
            match subscriber_guard.receive().await {
                Ok(msg) => {
                    // Release the lock while processing the message if possible,
                    // especially if handler is long running.
                    // However, if receive() holds resources that must not be double-polled,
                    // it might be tricky. For now, hold lock through handler for simplicity.
                    // Re-evaluate if handler causes deadlocks or long lock holds.
                    // For now, we'll drop the guard *after* handling and publishing,
                    // which means the subscriber is locked for the whole message processing cycle.
                    // This might not be ideal for performance.
                    // A better pattern might be to receive, then drop guard, then handle, then re-acquire for next receive.
                    // But let's keep it simple first.

                    self.logger
                        .info(&format!("Received message: {}", msg.uuid))
                        .await;

                    match (self.handler)(msg).await {
                        Ok(processed_msgs) => {
                            if !processed_msgs.is_empty() {
                                self.logger
                                    .info(&format!("Publishing {} messages", processed_msgs.len()))
                                    .await;
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

#[cfg(not(feature = "logging"))]
impl Router {
    /// Creates a new Router instance without logging.
    ///
    /// # Arguments
    ///
    /// * `consume_topic` - The topic/queue to consume messages from
    /// * `publish_topic` - The topic/queue to publish processed messages to

    /// * `subscriber` - The subscriber implementation (wrapped in Arc<Mutex<...>>)

    /// * `publisher` - The publisher implementation
    /// * `handler` - The message processing function
    pub fn new(
        consume_topic: String,
        publish_topic: String,

        subscriber: Arc<
            Mutex<dyn crate::Subscriber<Error = Box<dyn Error + Send + Sync>> + Send + Sync>,
        >,

        publisher: Arc<dyn crate::Publisher<Error = Box<dyn Error + Send + Sync>>>,
        handler: HandlerFunc,
    ) -> Self {
        Self {
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
        // Lock the subscriber to call subscribe
        let subscriber_guard_for_subscribe = self.subscriber.lock().await; // Removed mut
        subscriber_guard_for_subscribe
            .subscribe(&self.consume_topic)
            .await?;
        drop(subscriber_guard_for_subscribe); // Release lock after subscribe

        loop {
            // Lock the subscriber for each receive operation
            let mut subscriber_guard = self.subscriber.lock().await;
            match subscriber_guard.receive().await {
                Ok(msg) => {
                    // Similar to the logging version, lock is held during handler.

                    match (self.handler)(msg).await {
                        Ok(processed_msgs) => {
                            if !processed_msgs.is_empty() {
                                self.publisher
                                    .publish(&self.publish_topic, processed_msgs)
                                    .await?
                            }
                        }
                        Err(_) => {
                            // Error handling without logging
                        }
                    }
                }
                Err(_) => {
                    // Error handling without logging
                }
            }
        }
    }
}

// Re-export acknowledgment router types
pub use ack::{AckRouter, AckStrategy, RouterAckConfig, RouterAckStats};
