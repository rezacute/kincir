//! NATS backend implementation for Kincir
//!
//! This module provides a NATS message broker backend using the async-nats client.

use crate::{Message, Publisher, Subscriber};
use async_nats::Client;
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

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
///
/// A subscription is created by [`Subscriber::subscribe`] and consumed
/// message-by-message via [`Subscriber::receive`]. The active subscription is
/// held behind a mutex so the `&self` `subscribe` and `&mut self` `receive`
/// methods can share it.
pub struct NatsSubscriber {
    client: Client,
    subscription: Arc<Mutex<Option<async_nats::Subscriber>>>,
}

impl NatsSubscriber {
    pub async fn new(url: &str) -> Result<Self, NatsError> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| NatsError::ConnectionError(e.to_string()))?;

        Ok(Self {
            client,
            subscription: Arc::new(Mutex::new(None)),
        })
    }
}

#[async_trait]
impl Subscriber for NatsSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        let subscription = self
            .client
            .subscribe(topic.to_string())
            .await
            .map_err(|e| NatsError::SubscribeError(e.to_string()))?;

        // Replace any previous subscription with the new one.
        *self.subscription.lock().await = Some(subscription);
        Ok(())
    }

    async fn receive(&mut self) -> Result<Message, Self::Error> {
        let mut guard = self.subscription.lock().await;
        let subscription = guard.as_mut().ok_or_else(|| {
            NatsError::ReceiveError("not subscribed: call subscribe() first".to_string())
        })?;

        match subscription.next().await {
            Some(nats_message) => {
                let message = Message::new(nats_message.payload.to_vec())
                    .with_metadata("nats_subject", nats_message.subject.to_string());
                Ok(message)
            }
            None => Err(Box::new(NatsError::ReceiveError(
                "subscription closed".to_string(),
            ))),
        }
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
