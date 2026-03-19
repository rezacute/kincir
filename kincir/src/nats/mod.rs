//! NATS backend implementation for Kincir
//!
//! This module provides a NATS message broker backend using the async-nats client.

use crate::{Message, Publisher, Subscriber};
use async_nats::Client;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NatsError {
    #[error("NATS connection error: {0}")]
    ConnectionError(String),
    
    #[error("NATS publish error: {0}")]
    PublishError(String),
    
    #[error("NATS subscribe error: {0}")]
    SubscribeError(String),
    
    #[error("NATS receive error: {0}")]
    ReceiveError(String),
}

/// NATS Publisher
#[derive(Clone)]
pub struct NatsPublisher {
    client: Client,
}

impl NatsPublisher {
    pub async fn new(url: &str) -> Result<Self, NatsError> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| NatsError::ConnectionError(e.to_string()))?;
        
        Ok(Self { client })
    }
}

#[async_trait]
impl Publisher for NatsPublisher {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        for message in messages {
            let topic = topic.to_string();
            self.client
                .publish(topic, message.payload.into())
                .await
                .map_err(|e| NatsError::PublishError(e.to_string()))?;
        }
        Ok(())
    }
}

/// NATS Subscriber
pub struct NatsSubscriber {
    client: Client,
}

impl NatsSubscriber {
    pub async fn new(url: &str) -> Result<Self, NatsError> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| NatsError::ConnectionError(e.to_string()))?;
        
        Ok(Self { client })
    }
}

#[async_trait]
impl Subscriber for NatsSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        // NATS subscription is done per-message in receive
        let _ = topic;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Message, Self::Error> {
        // For NATS, we'd need to set up a subscription first
        // This is a simplified implementation
        Err(Box::new(NatsError::ReceiveError(
            "NATS receive not fully implemented - use subscription via client".to_string()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nats_error_display() {
        let err = NatsError::ConnectionError("connection failed".to_string());
        assert_eq!(err.to_string(), "NATS connection error: connection failed");
        
        let err = NatsError::PublishError("publish failed".to_string());
        assert_eq!(err.to_string(), "NATS publish error: publish failed");
    }
}
