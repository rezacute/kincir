//! In-memory message broker implementation
//!
//! This module provides a high-performance, thread-safe in-memory message broker
//! that implements the Publisher and Subscriber traits. It's designed for testing,
//! development, and scenarios where external message brokers are not required.
//!
//! # Features
//!
//! - Thread-safe concurrent access
//! - Configurable queue size limits
//! - Topic management with subscriber broadcasting
//! - Optional statistics collection
//! - Message ordering guarantees
//! - Graceful shutdown support
//! - Publisher and Subscriber trait implementations
//!
//! # Example
//!
//! ```rust,no_run
//! use kincir::memory::{InMemoryBroker, InMemoryConfig, InMemoryPublisher, InMemorySubscriber};
//! use kincir::{Publisher, Subscriber, Message};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create broker with custom configuration
//!     let config = InMemoryConfig::new()
//!         .with_max_queue_size(Some(1000))
//!         .with_stats(true);
//!     
//!     let broker = Arc::new(InMemoryBroker::new(config));
//!     
//!     // Create publisher and subscriber
//!     let publisher = InMemoryPublisher::new(broker.clone());
//!     let mut subscriber = InMemorySubscriber::new(broker.clone());
//!     
//!     // Subscribe to a topic
//!     subscriber.subscribe("my-topic").await?;
//!     
//!     // Publish messages
//!     let messages = vec![
//!         Message::new(b"Hello".to_vec()),
//!         Message::new(b"World".to_vec()),
//!     ];
//!     publisher.publish("my-topic", messages).await?;
//!     
//!     // Receive messages
//!     let message1 = subscriber.receive().await?;
//!     let message2 = subscriber.receive().await?;
//!     
//!     println!("Received: {:?}", String::from_utf8_lossy(&message1.payload));
//!     println!("Received: {:?}", String::from_utf8_lossy(&message2.payload));
//!     
//!     Ok(())
//! }
//! ```

mod advanced_tests;
mod broker;
mod config;
mod error;
mod example;
mod publisher;
mod stats;
mod subscriber;

// Re-export public types
pub use broker::{InMemoryBroker, TopicInfo, BrokerHealth};
pub use config::InMemoryConfig;
pub use error::InMemoryError;
pub use publisher::InMemoryPublisher;
pub use stats::{BrokerStats, StatsSnapshot};
pub use subscriber::InMemorySubscriber;
