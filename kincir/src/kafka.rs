use crate::{Message, router::Logger};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KafkaError {
    #[error("Channel send error")]
    ChannelSend,
    #[error("Channel receive error")]
    ChannelReceive,
}

pub struct KafkaPublisher {
    tx: mpsc::Sender<Message>,
    logger: Arc<dyn Logger>,
}

impl KafkaPublisher {
    pub fn new(
        _brokers: Vec<String>,
        tx: mpsc::Sender<Message>,
        logger: Arc<dyn Logger>,
    ) -> Self {
        Self { tx, logger }
    }
}

#[async_trait]
impl super::Publisher for KafkaPublisher {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn publish(&self, _topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        for message in messages {
            self.tx.send(message).await.map_err(|_| KafkaError::ChannelSend)?;
        }
        Ok(())
    }
}

pub struct KafkaSubscriber {
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Message>>>,
    logger: Arc<dyn Logger>,
}

impl KafkaSubscriber {
    pub fn new(
        _brokers: Vec<String>,
        _group_id: String,
        rx: mpsc::Receiver<Message>,
        logger: Arc<dyn Logger>,
    ) -> Self {
        Self { rx: Arc::new(tokio::sync::Mutex::new(rx)), logger }
    }
}

#[async_trait]
impl super::Subscriber for KafkaSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn subscribe(&self, _topic: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(&self) -> Result<Message, Self::Error> {
        let mut rx = self.rx.lock().await;
        rx.recv().await.ok_or_else(|| Box::new(KafkaError::ChannelReceive) as Box<dyn std::error::Error + Send + Sync>)
    }
}