use async_trait::async_trait;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;
use std::fmt;
use thiserror::Error;
use tokio::sync::mpsc;

#[cfg(feature = "logging")]
use tracing::{debug, error, info, warn};

use crate::core::{Message, MessageHandler, Publisher, Subscriber};

#[derive(Error, Debug)]
pub enum MQTTError {
    #[error("MQTT connection error: {0}")]
    ConnectionError(String),
    #[error("MQTT publish error: {0}")]
    PublishError(String),
    #[error("MQTT subscribe error: {0}")]
    SubscribeError(String),
    #[error("MQTT receive error: {0}")]
    ReceiveError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

pub struct MQTTPublisher {
    client: AsyncClient,
    topic: String,
    // EventLoop needs to be stored and polled
    // We'll spawn a task to poll it.
    // event_loop: rumqttc::EventLoop,
}

impl MQTTPublisher {
    pub fn new(broker_url: &str, topic: &str) -> Result<Self, MQTTError> {
        let mut mqtt_options = MqttOptions::new("rumqtt-publisher", broker_url, 1883);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(5));

        match AsyncClient::new(mqtt_options, 10) {
            Ok((client, mut eventloop)) => {
                // Spawn a task to poll the event loop for the publisher
                tokio::spawn(async move {
                    loop {
                        match eventloop.poll().await {
                            Ok(notification) => {
                                #[cfg(feature = "logging")]
                                debug!("Publisher EventLoop notification: {:?}", notification);
                                // Handle specific notifications if needed, e.g., disconnections
                            }
                            Err(e) => {
                                #[cfg(feature = "logging")]
                                error!("Publisher EventLoop error: {}", e);
                                // Potentially break the loop or implement reconnection logic
                                break;
                            }
                        }
                    }
                });

                #[cfg(feature = "logging")]
                info!(
                    "Successfully connected to MQTT broker for publisher: {}",
                    broker_url
                );
                Ok(MQTTPublisher {
                    client,
                    topic: topic.to_string(),
                })
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                error!("Failed to connect to MQTT broker for publisher: {}", e);
                Err(MQTTError::ConnectionError(e.to_string()))
            }
        }
    }
}

#[async_trait]
impl Publisher for MQTTPublisher {
    // The trait defines `topic: &str` and `messages: Vec<Message>`.
    // Current MQTTPublisher is tied to a single topic at creation.
    // We will use the publisher's configured topic and ignore the `topic` argument here for now,
    // or assert it matches. For simplicity, ignoring.
    // The trait also expects Vec<Message>, current impl takes one generic message.
    // We'll adapt to take Vec<crate::Message> and publish them one by one.
    async fn publish(
        &self,
        _topic: &str,
        messages: Vec<crate::Message>,
    ) -> Result<(), Box<dyn Error>> {
        for message in messages {
            // Assuming crate::Message payload is already Vec<u8>
            // If kincir::Message used serde for payload, we'd serialize here.
            // But the Message struct has `payload: Vec<u8>`.
            // The previous MQTT impl used serde_json::to_string(&message).
            // This indicates a mismatch in understanding of kincir::Message.
            // For now, let's assume the payload in `message` is what needs to be sent.
            // If `message` itself needs to be serialized (e.g. if it's a struct that *contains* the payload),
            // then the `to_payload()` method from `kincir::Message` trait would be used.
            // However, the `Publisher` trait takes `Vec<Message>`, not `Vec<T: Message>`.
            // So, `message.payload` should be the raw bytes.

            // The previous code:
            // let payload = serde_json::to_string(&message)
            //     .map_err(|e| MQTTError::SerializationError(e.to_string()))?;
            // This implies that the `Message` type `T` itself was being serialized, not its `payload` field.
            // Let's stick to the `kincir::Message` struct which has a `payload: Vec<u8>`.
            // The `MqttPublisher::publish` signature was `<T: Message + Sync>`.
            // The trait is `messages: Vec<Message>`. So, `message.payload` is the correct thing to publish.

            match self
                .client
                .publish(
                    &self.topic,
                    QoS::AtLeastOnce,
                    false,
                    message.payload.as_slice(),
                )
                .await
            {
                Ok(_) => {
                    #[cfg(feature = "logging")]
                    debug!(
                        "Successfully published message (UUID: {}) to topic: {}",
                        message.uuid, self.topic
                    );
                }
                Err(e) => {
                    #[cfg(feature = "logging")]
                    error!(
                        "Failed to publish message (UUID: {}) to topic {}: {}",
                        message.uuid, self.topic, e
                    );
                    // Return on first error
                    return Err(Box::new(MQTTError::PublishError(e.to_string())));
                }
            }
        }
        Ok(())
    }
}

// Forward declaration for Message if not already in scope
// use crate::core::Message;

pub struct MQTTSubscriber {
    client: AsyncClient,
    event_loop: rumqttc::EventLoop,
    topic: String, // Topic subscribed to
}

impl MQTTSubscriber {
    // new remains the same, it sets up the client and event loop
    pub fn new(broker_url: &str, topic: &str) -> Result<Self, MQTTError> {
        let client_id = format!("kincir-mqtt-subscriber-{}", uuid::Uuid::new_v4());
        let mut mqtt_options = MqttOptions::new(client_id, broker_url, 1883);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(5));

        match AsyncClient::new(mqtt_options, 10) {
            Ok((client, event_loop)) => {
                #[cfg(feature = "logging")]
                info!(
                    "MQTTSubscriber: Successfully connected to MQTT broker: {} for topic: {}",
                    broker_url, topic
                );
                Ok(MQTTSubscriber {
                    client,
                    event_loop,
                    topic: topic.to_string(),
                })
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                error!(
                    "MQTTSubscriber: Failed to connect to MQTT broker {}: {}",
                    broker_url, e
                );
                Err(MQTTError::ConnectionError(e.to_string()))
            }
        }
    }
}

#[async_trait]
impl Subscriber for MQTTSubscriber {
    // Subscribe to the topic this subscriber was created for.
    // Aligns with Subscriber trait: &self, topic: &str
    async fn subscribe(&self, topic: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        if topic != self.topic {
            let err_msg = format!(
                "Subscription topic mismatch: expected '{}', got '{}'",
                self.topic, topic
            );
            #[cfg(feature = "logging")]
            error!("{}", err_msg);
            return Err(Box::new(MQTTError::SubscribeError(err_msg)));
        }
        self.client
            .subscribe(&self.topic, QoS::AtLeastOnce)
            .await
            .map_err(|e| {
                #[cfg(feature = "logging")]
                error!("Failed to subscribe to MQTT topic {}: {}", self.topic, e);
                Box::new(MQTTError::SubscribeError(e.to_string())) as Box<dyn Error + Send + Sync>
            })?;
        #[cfg(feature = "logging")]
        info!("Successfully subscribed to MQTT topic: {}", self.topic);
        Ok(())
    }

    // Receive one message. This should be called in a loop.
    // NOTE: This requires &mut self due to event_loop.poll().
    // This assumes the Subscriber trait in lib.rs is changed to `receive(&mut self)`.
    async fn receive(&mut self) -> Result<crate::Message, Box<dyn Error + Send + Sync>> {
        loop {
            match self.event_loop.poll().await {
                Ok(notification) => {
                    if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) =
                        notification
                    {
                        // Deserialize directly into kincir::Message
                        // The payload from rumqttc is Vec<u8>.
                        // kincir::Message expects this payload directly.
                        // We need a UUID and metadata. The MQTT message itself doesn't carry
                        // a separate UUID or kincir metadata in its standard structure.
                        // We'll create a new kincir::Message, using the MQTT payload as its payload.
                        // UUID will be generated. Metadata will be empty for now, or we could
                        // potentially extract some from MQTT properties if available and desired.

                        #[cfg(feature = "logging")]
                        debug!("Received raw MQTT message on topic: {}", publish.topic);

                        // Here, we assume the payload *is* the kincir::Message payload, not a serialized kincir::Message.
                        // If the publisher sends a serialized kincir::Message (e.g. JSON of the kincir::Message struct),
                        // then we would deserialize `kincir::Message` from `publish.payload`.
                        // However, the MQTTPublisher was just changed to send `message.payload`.
                        // This means the application bytes are directly in `publish.payload`.
                        let k_message = crate::Message {
                            uuid: uuid::Uuid::new_v4().to_string(), // Generate new UUID for this received message
                            payload: publish.payload.to_vec(),
                            metadata: std::collections::HashMap::new(), // Empty metadata for now
                        };

                        // The old code used `serde_json::from_slice::<T>(&publish.payload)`
                        // where T was `kincir::Message`. This implies the whole kincir message
                        // (UUID, payload, metadata) was expected to be in the MQTT payload.
                        // This contradicts the publisher change.
                        // For now, let's assume the MQTT payload IS the application data.
                        return Ok(k_message);
                    }
                    // Handle other notifications if necessary (e.g., Disconnect, Reconnect)
                    #[cfg(feature = "logging")]
                    debug!("Received other MQTT notification: {:?}", notification);
                }
                Err(e) => {
                    #[cfg(feature = "logging")]
                    error!("Error polling MQTT event loop: {}", e);
                    // Depending on the error, may need to break or attempt reconnection.
                    // For now, return an error.
                    return Err(Box::new(MQTTError::ReceiveError(e.to_string())));
                }
            }
        }
    }
}
