pub mod ack;
#[cfg(test)]
mod tests;

use async_trait::async_trait;
pub use rumqttc::QoS;
use rumqttc::{AsyncClient, MqttOptions}; // Removed QoS from here // Publicly re-export QoS
                                         // serde::{Deserialize, Serialize}; // Removed
                                         // serde_json; // Removed
                                         // std::error::Error; // Removed
                                         // std::fmt; // Removed
use thiserror::Error; // Keep thiserror for MQTTError definition
use tokio::sync::mpsc;

#[cfg(feature = "logging")]
use tracing::{debug, error, info, warn};

use crate::{Publisher, Subscriber}; // Message removed from this line

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
    #[error("Other MQTT error: {0}")]
    Other(String),
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
        let client_id = format!("kincir-mqtt-publisher-{}", uuid::Uuid::new_v4());
        let mut mqtt_options = MqttOptions::new(client_id, broker_url, 1883);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(5));

        let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10); // Direct assignment

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
            "MQTTPublisher: Initialized for broker_url: {}, topic: {}",
            broker_url, topic
        );
        Ok(MQTTPublisher {
            client,
            topic: topic.to_string(),
        })
    }
}

#[async_trait]
impl Publisher for MQTTPublisher {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // The trait defines `topic: &str` and `messages: Vec<Message>`.
    // Current MQTTPublisher is tied to a single topic at creation.
    // We will use the publisher's configured topic and ignore the `topic` argument here for now,
    // or assert it matches. For simplicity, ignoring.
    // The trait also expects Vec<crate::Message>, current impl takes one generic message.
    // We'll adapt to take Vec<crate::Message> and publish them one by one.
    async fn publish(
        &self,
        _topic: &str,
        messages: Vec<crate::Message>,
    ) -> Result<(), Self::Error> {
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
use uuid::Uuid; // Ensure Uuid is imported

pub struct MQTTSubscriber {
    client: AsyncClient, // Retain client for potential re-subscribe logic or other control operations
    topic: String,
    message_rx: mpsc::Receiver<Result<crate::Message, MQTTError>>,
    qos: QoS, // Added QoS field
}

impl MQTTSubscriber {
    pub fn new(broker_url: &str, topic_str: &str, qos: QoS) -> Result<Self, MQTTError> {
        // Added qos parameter
        let client_id = format!("kincir-mqtt-subscriber-{}", Uuid::new_v4());
        let mut mqtt_options = MqttOptions::new(client_id, broker_url, 1883);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(5));

        let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);
        let (message_tx, message_rx) = mpsc::channel(100); // Buffer size can be configured

        // Spawned task to handle the event loop and forward messages/errors
        tokio::spawn(async move {
            #[cfg(feature = "logging")]
            info!("MQTT EventLoop Task: Started for topic processing.");

            loop {
                match event_loop.poll().await {
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                        let k_message = crate::Message {
                            uuid: Uuid::new_v4().to_string(),
                            payload: publish.payload.to_vec(),
                            metadata: std::collections::HashMap::new(),
                        };
                        if message_tx.send(Ok(k_message)).await.is_err() {
                            #[cfg(feature = "logging")]
                            error!("MQTT EventLoop Task: Failed to send message to channel. Receiver dropped.");
                            break; // Exit loop if receiver is gone
                        }
                    }
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::Disconnect)) => {
                        #[cfg(feature = "logging")]
                        warn!("MQTT EventLoop Task: MQTT Disconnected. Sending error and exiting.");
                        // Attempt to send error, ignore if receiver is already dropped
                        let _ = message_tx
                            .send(Err(MQTTError::ConnectionError(
                                "MQTT Disconnected".to_string(),
                            )))
                            .await;
                        break; // Exit loop on disconnect
                    }
                    Ok(event) => {
                        #[cfg(feature = "logging")]
                        debug!("MQTT EventLoop Task: Received event: {:?}", event);
                        // Handle other events like ConnAck, PubAck, SubAck, PingResp etc. if necessary
                        // For now, just logging them.
                    }
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        error!("MQTT EventLoop Task: MQTT EventLoop error: {}. Sending error and exiting.", e);
                        // Attempt to send error, ignore if receiver is already dropped
                        let _ = message_tx
                            .send(Err(MQTTError::ReceiveError(e.to_string())))
                            .await;
                        break; // Exit loop on error
                    }
                }
            }
            #[cfg(feature = "logging")]
            info!("MQTT EventLoop Task: Exiting.");
        });

        #[cfg(feature = "logging")]
        info!(
            "MQTTSubscriber: Created for broker_url: {}, topic: {}",
            broker_url, topic_str
        );

        Ok(MQTTSubscriber {
            client,
            topic: topic_str.to_string(),
            message_rx,
            qos, // Store QoS
        })
    }
}

#[async_trait]
impl Subscriber for MQTTSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Subscribe to the topic this subscriber was created for.
    // Aligns with Subscriber trait: &self, topic: &str
    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        if topic != self.topic {
            let err_msg = format!(
                "Subscription topic mismatch: expected '{}', got '{}'. Current subscriber is for topic '{}'.",
                self.topic, topic, self.topic
            );
            #[cfg(feature = "logging")]
            error!("{}", err_msg);
            // It's important that an instance of MQTTSubscriber is tied to a single topic from its MPSC queue.
            // If the user wants to subscribe to a different topic, they should create a new MQTTSubscriber.
            // So, this check is crucial.
            return Err(Box::new(MQTTError::SubscribeError(err_msg)));
        }

        // The spawned task in `new` is already polling the event loop associated with `self.client`.
        // When `self.client.subscribe` is called here and succeeds, the event loop will
        // start receiving messages for this subscription, which will then be processed by the spawned task.

        let client_clone = self.client.clone();
        let topic_to_subscribe = self.topic.clone(); // Use self.topic, as this instance is fixed to it.

        #[cfg(feature = "logging")]
        info!(
            "MQTTSubscriber::subscribe - Attempting to subscribe client to topic: {}",
            topic_to_subscribe
        );

        client_clone
            .subscribe(&topic_to_subscribe, self.qos) // Use self.qos
            .await
            .map_err(|e| {
                #[cfg(feature = "logging")]
                error!(
                    "MQTTSubscriber::subscribe - Failed to subscribe to MQTT topic {}: {}",
                    topic_to_subscribe, e
                );
                Box::new(MQTTError::SubscribeError(e.to_string())) as Self::Error
            })?;

        #[cfg(feature = "logging")]
        info!(
            "MQTTSubscriber::subscribe - Successfully sent SUBSCRIBE packet for topic: {}",
            topic_to_subscribe
        );
        Ok(())
    }

    // Receive one message. This should be called in a loop.
    // NOTE: This requires &mut self due to event_loop.poll().
    // This assumes the Subscriber trait in lib.rs is changed to `receive(&mut self)`.
    async fn receive(&mut self) -> Result<crate::Message, Self::Error> {
        match self.message_rx.recv().await {
            Some(Ok(message)) => {
                #[cfg(feature = "logging")]
                debug!(
                    "MQTTSubscriber::receive - Received message from channel: {}",
                    message.uuid
                );
                Ok(message)
            }
            Some(Err(mqtt_error)) => {
                #[cfg(feature = "logging")]
                error!(
                    "MQTTSubscriber::receive - Received error from event loop task: {:?}",
                    mqtt_error
                );
                Err(Box::new(mqtt_error) as Self::Error)
            }
            None => {
                // Channel has been closed. This implies the event loop task has terminated.
                #[cfg(feature = "logging")]
                warn!("MQTTSubscriber::receive - Message channel closed. Event loop task likely terminated.");
                Err(Box::new(MQTTError::ReceiveError(
                    "Message channel closed. Event loop task terminated.".to_string(),
                )) as Self::Error)
            }
        }
    }
}

// Final adjustment for error returning in receive:
// The line: return Err(Box::new(MQTTError::ReceiveError(e.to_string())));
// should ensure it's cast to Self::Error if not directly compatible.
// Since Self::Error is Box<dyn std::error::Error + Send + Sync>,
// and MQTTError implements std::error::Error, this should be fine.
// The casting `as Box<dyn Error + Send + Sync>` in the .map_err for subscribe was already correct.
// The one in publish also needs to be `as Self::Error` or ensure the Box::new matches.

// Let's ensure all Box::new(MQTTError::...) are cast or match Self::Error.
// For publish error: `return Err(Box::new(MQTTError::PublishError(e.to_string())) as Self::Error);`
// For subscribe error: `Box::new(MQTTError::SubscribeError(e.to_string())) as Self::Error` (already done)
// For receive error: `return Err(Box::new(MQTTError::ReceiveError(e.to_string())) as Self::Error);`
// No, the direct Box::new(MQTTError::...) is already compatible with Box<dyn Error ...>`.
// The `as Self::Error` is only needed if the types were different and one implemented the other.
// Here, they are effectively the same type alias.
// The current code `return Err(Box::new(MQTTError::PublishError(e.to_string())));` is fine
// because `Box<MQTTError>` can be coerced to `Box<dyn Error ...>`.

// Re-export acknowledgment types
pub use ack::{MQTTAckHandle, MQTTAckSubscriber};
