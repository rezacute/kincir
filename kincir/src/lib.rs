use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub uuid: String,
    pub payload: Vec<u8>,
    pub metadata: HashMap<String, String>,
}

impl Message {
    pub fn new(payload: Vec<u8>) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            payload,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[async_trait]
pub trait Publisher {
    type Error;

    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait Subscriber {
    type Error;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error>;
    async fn receive(&self) -> Result<Message, Self::Error>;
}

pub mod kafka;
pub mod rabbitmq;
pub mod router;

// Re-export commonly used types
pub use router::{Logger, StdLogger, Router, HandlerFunc};
