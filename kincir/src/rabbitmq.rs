use crate::Message;
use async_trait::async_trait;
use futures::StreamExt;
use lapin::options::{BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Connection, ConnectionProperties};
use serde_json;
use thiserror::Error;
use tokio_amqp::*;

#[derive(Error, Debug)]
pub enum RabbitMQError {
    #[error("RabbitMQ error: {0}")]
    RabbitMQ(#[from] lapin::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct RabbitMQPublisher {
    connection: Connection,
}

impl RabbitMQPublisher {
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(
            uri,
            ConnectionProperties::default().with_tokio(),
        )
        .await
        .map_err(RabbitMQError::RabbitMQ)?;

        Ok(Self { connection })
    }
}

#[async_trait]
impl super::Publisher for RabbitMQPublisher {
    type Error = RabbitMQError;

    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        let channel = self.connection.create_channel().await.map_err(RabbitMQError::RabbitMQ)?;

        channel
            .queue_declare(
                topic,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        for message in messages {
            let payload = serde_json::to_vec(&message)?;
            let confirm = channel
                .basic_publish(
                    "",
                    topic,
                    BasicPublishOptions::default(),
                    &payload,
                    BasicProperties::default(),
                )
                .await
                .map_err(RabbitMQError::RabbitMQ)?;
            let _ = confirm.await.map_err(RabbitMQError::RabbitMQ)?;
        }

        Ok(())
    }
}

pub struct RabbitMQSubscriber {
    connection: Connection,
}

impl RabbitMQSubscriber {
    pub async fn new(uri: &str) -> Result<Self, RabbitMQError> {
        let connection = Connection::connect(
            uri,
            ConnectionProperties::default().with_tokio(),
        )
        .await
        .map_err(RabbitMQError::RabbitMQ)?;

        Ok(Self { connection })
    }
}

#[async_trait]
impl super::Subscriber for RabbitMQSubscriber {
    type Error = RabbitMQError;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        let channel = self.connection.create_channel().await.map_err(RabbitMQError::RabbitMQ)?;

        channel
            .queue_declare(
                topic,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        Ok(())
    }

    async fn receive(&self) -> Result<Message, Self::Error> {
        let channel = self.connection.create_channel().await.map_err(RabbitMQError::RabbitMQ)?;
        let mut consumer = channel
            .basic_consume(
                "",
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(RabbitMQError::RabbitMQ)?;

        if let Some(delivery) = consumer.next().await {
            let delivery = delivery.map_err(RabbitMQError::RabbitMQ)?;
            let message = serde_json::from_slice(&delivery.data)?;
            delivery.ack(Default::default()).await.map_err(RabbitMQError::RabbitMQ)?;
            Ok(message)
        } else {
            Err(RabbitMQError::RabbitMQ(lapin::Error::InvalidChannelState(lapin::ChannelState::Error)))
        }
    }
}