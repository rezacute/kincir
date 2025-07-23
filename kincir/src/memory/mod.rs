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
//!
//! # Example
//!
//! ```rust,no_run
//! use kincir::memory::{InMemoryBroker, InMemoryConfig};
//! use kincir::Message;
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
//!     // Subscribe to a topic
//!     let mut receiver = broker.subscribe("my-topic")?;
//!     
//!     // Publish messages
//!     let messages = vec![
//!         Message::new(b"Hello".to_vec()),
//!         Message::new(b"World".to_vec()),
//!     ];
//!     broker.publish("my-topic", messages)?;
//!     
//!     // Receive messages
//!     while let Ok(message) = receiver.recv().await {
//!         println!("Received: {:?}", String::from_utf8_lossy(&message.payload));
//!     }
//!     
//!     Ok(())
//! }
//! ```

mod broker;
mod config;
mod error;
mod stats;

// Re-export public types
pub use broker::{InMemoryBroker, TopicInfo};
pub use config::InMemoryConfig;
pub use error::InMemoryError;
pub use stats::{BrokerStats, StatsSnapshot};

// Publisher and Subscriber implementations will be added in Phase 2
// pub use publisher::InMemoryPublisher;
// pub use subscriber::InMemorySubscriber;
