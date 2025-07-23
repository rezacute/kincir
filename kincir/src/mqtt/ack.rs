//! Acknowledgment handling for MQTT message broker

use crate::ack::{AckHandle, AckSubscriber};
use crate::mqtt::MQTTError;
use crate::Message;
use async_trait::async_trait;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, Publish, QoS};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

/// Acknowledgment handle for MQTT messages
#[derive(Debug, Clone)]
pub struct MQTTAckHandle {
    /// Unique message identifier
    message_id: String,
    /// Topic name
    topic: String,
    /// Message timestamp
    timestamp: SystemTime,
    /// Delivery attempt count
    delivery_count: u32,
    /// MQTT Quality of Service level
    qos: QoS,
    /// MQTT packet identifier (for QoS > 0)
    packet_id: Option<u16>,
    /// Internal handle ID for tracking
    handle_id: String,
    /// Whether this message requires acknowledgment
    requires_ack: bool,
}

impl MQTTAckHandle {
    /// Create a new MQTT acknowledgment handle
    pub fn new(
        message_id: String,
        topic: String,
        timestamp: SystemTime,
        delivery_count: u32,
        qos: QoS,
        packet_id: Option<u16>,
    ) -> Self {
        let requires_ack = matches!(qos, QoS::AtLeastOnce | QoS::ExactlyOnce);
        
        Self {
            message_id,
            topic,
            timestamp,
            delivery_count,
            qos,
            packet_id,
            handle_id: Uuid::new_v4().to_string(),
            requires_ack,
        }
    }
    
    /// Get the MQTT Quality of Service level
    pub fn qos(&self) -> QoS {
        self.qos
    }
    
    /// Get the MQTT packet identifier (for QoS > 0)
    pub fn packet_id(&self) -> Option<u16> {
        self.packet_id
    }
    
    /// Get the internal handle ID
    pub fn handle_id(&self) -> &str {
        &self.handle_id
    }
    
    /// Check if this message requires acknowledgment
    pub fn requires_ack(&self) -> bool {
        self.requires_ack
    }
}

impl AckHandle for MQTTAckHandle {
    fn message_id(&self) -> &str {
        &self.message_id
    }
    
    fn topic(&self) -> &str {
        &self.topic
    }
    
    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    
    fn delivery_count(&self) -> u32 {
        self.delivery_count
    }
}

/// MQTT subscriber with acknowledgment support
pub struct MQTTAckSubscriber {
    /// MQTT client
    client: AsyncClient,
    /// MQTT event loop
    event_loop: Arc<Mutex<EventLoop>>,
    /// Current subscription state
    state: Mutex<SubscriberState>,
    /// Message receiver channel
    message_rx: Arc<Mutex<mpsc::Receiver<(Message, MQTTAckHandle)>>>,
    /// Logger for debugging (optional)
    #[cfg(feature = "logging")]
    logger: Arc<dyn crate::logging::Logger>,
}

struct SubscriberState {
    /// Current topic subscriptions with QoS
    subscriptions: HashMap<String, QoS>,
    /// Pending acknowledgments (packet_id -> handle)
    pending_acks: HashMap<u16, MQTTAckHandle>,
    /// Message delivery tracking
    delivery_counts: HashMap<String, u32>,
}

impl MQTTAckSubscriber {
    /// Create a new MQTT acknowledgment subscriber
    #[cfg(not(feature = "logging"))]
    pub async fn new(
        broker_url: &str,
        client_id: Option<String>,
    ) -> Result<Self, MQTTError> {
        let client_id = client_id.unwrap_or_else(|| {
            format!("kincir-mqtt-ack-{}", Uuid::new_v4())
        });
        
        let mut mqtt_options = MqttOptions::new(client_id, broker_url, 1883);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(30));
        mqtt_options.set_clean_session(false); // Persistent session for QoS > 0
        
        let (client, event_loop) = AsyncClient::new(mqtt_options, 100);
        let (message_tx, message_rx) = mpsc::channel(1000);
        
        // Start event loop handler
        let event_loop_handle = Arc::new(Mutex::new(event_loop));
        let event_loop_clone = event_loop_handle.clone();
        let client_clone = client.clone();
        
        tokio::spawn(async move {
            Self::handle_event_loop(event_loop_clone, message_tx, client_clone).await;
        });

        Ok(Self {
            client,
            event_loop: event_loop_handle,
            state: Mutex::new(SubscriberState {
                subscriptions: HashMap::new(),
                pending_acks: HashMap::new(),
                delivery_counts: HashMap::new(),
            }),
            message_rx: Arc::new(Mutex::new(message_rx)),
        })
    }

    /// Create a new MQTT acknowledgment subscriber with logging
    #[cfg(feature = "logging")]
    pub async fn new(
        broker_url: &str,
        client_id: Option<String>,
    ) -> Result<Self, MQTTError> {
        let client_id = client_id.unwrap_or_else(|| {
            format!("kincir-mqtt-ack-{}", Uuid::new_v4())
        });
        
        let mut mqtt_options = MqttOptions::new(client_id, broker_url, 1883);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(30));
        mqtt_options.set_clean_session(false); // Persistent session for QoS > 0
        
        let (client, event_loop) = AsyncClient::new(mqtt_options, 100);
        let (message_tx, message_rx) = mpsc::channel(1000);
        
        let logger = Arc::new(crate::logging::NoOpLogger::new());
        
        // Start event loop handler
        let event_loop_handle = Arc::new(Mutex::new(event_loop));
        let event_loop_clone = event_loop_handle.clone();
        let client_clone = client.clone();
        let logger_clone = logger.clone();
        
        tokio::spawn(async move {
            Self::handle_event_loop_with_logging(event_loop_clone, message_tx, client_clone, logger_clone).await;
        });

        Ok(Self {
            client,
            event_loop: event_loop_handle,
            state: Mutex::new(SubscriberState {
                subscriptions: HashMap::new(),
                pending_acks: HashMap::new(),
                delivery_counts: HashMap::new(),
            }),
            message_rx: Arc::new(Mutex::new(message_rx)),
            logger,
        })
    }

    /// Set a logger for the subscriber (only available with the "logging" feature)
    #[cfg(feature = "logging")]
    pub fn with_logger(mut self, logger: Arc<dyn crate::logging::Logger>) -> Self {
        self.logger = logger;
        self
    }

    /// Get the currently subscribed topics
    pub async fn subscribed_topics(&self) -> Vec<String> {
        let state = self.state.lock().await;
        state.subscriptions.keys().cloned().collect()
    }

    /// Check if currently subscribed to any topics
    pub async fn is_subscribed(&self) -> bool {
        let state = self.state.lock().await;
        !state.subscriptions.is_empty()
    }

    /// Handle MQTT event loop without logging
    async fn handle_event_loop(
        event_loop: Arc<Mutex<EventLoop>>,
        message_tx: mpsc::Sender<(Message, MQTTAckHandle)>,
        _client: AsyncClient,
    ) {
        loop {
            let event_result = {
                let mut event_loop_guard = event_loop.lock().await;
                event_loop_guard.poll().await
            };

            match event_result {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    if let Some((message, handle)) = Self::process_publish(publish) {
                        if message_tx.send((message, handle)).await.is_err() {
                            break; // Channel closed
                        }
                    }
                }
                Ok(Event::Incoming(Packet::PubAck(_))) => {
                    // Handle QoS 1 acknowledgment
                }
                Ok(Event::Incoming(Packet::PubRec(_))) => {
                    // Handle QoS 2 acknowledgment (part 1)
                }
                Ok(Event::Incoming(Packet::PubComp(_))) => {
                    // Handle QoS 2 acknowledgment (part 2)
                }
                Ok(_) => {
                    // Other events
                }
                Err(_) => {
                    // Connection error - could implement reconnection logic here
                    break;
                }
            }
        }
    }

    /// Handle MQTT event loop with logging
    #[cfg(feature = "logging")]
    async fn handle_event_loop_with_logging(
        event_loop: Arc<Mutex<EventLoop>>,
        message_tx: mpsc::Sender<(Message, MQTTAckHandle)>,
        _client: AsyncClient,
        logger: Arc<dyn crate::logging::Logger>,
    ) {
        logger.info("MQTT acknowledgment event loop started").await;
        
        loop {
            let event_result = {
                let mut event_loop_guard = event_loop.lock().await;
                event_loop_guard.poll().await
            };

            match event_result {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    logger.info(&format!(
                        "Received MQTT publish: topic={}, qos={:?}, packet_id={:?}",
                        publish.topic, publish.qos, publish.pkid
                    )).await;
                    
                    if let Some((message, handle)) = Self::process_publish(publish) {
                        if message_tx.send((message, handle)).await.is_err() {
                            logger.error("Message channel closed").await;
                            break;
                        }
                    }
                }
                Ok(Event::Incoming(Packet::PubAck(puback))) => {
                    logger.info(&format!("Received PUBACK for packet_id: {}", puback.pkid)).await;
                }
                Ok(Event::Incoming(Packet::PubRec(pubrec))) => {
                    logger.info(&format!("Received PUBREC for packet_id: {}", pubrec.pkid)).await;
                }
                Ok(Event::Incoming(Packet::PubComp(pubcomp))) => {
                    logger.info(&format!("Received PUBCOMP for packet_id: {}", pubcomp.pkid)).await;
                }
                Ok(event) => {
                    logger.info(&format!("MQTT event: {:?}", event)).await;
                }
                Err(e) => {
                    logger.error(&format!("MQTT connection error: {:?}", e)).await;
                    break;
                }
            }
        }
    }

    /// Process MQTT publish packet into message and handle
    fn process_publish(publish: Publish) -> Option<(Message, MQTTAckHandle)> {
        // Try to deserialize as JSON first, fallback to raw payload
        let message = match serde_json::from_slice::<Message>(&publish.payload) {
            Ok(msg) => msg,
            Err(_) => Message::new(publish.payload.to_vec()),
        };

        let handle = MQTTAckHandle::new(
            message.uuid.clone(),
            publish.topic.clone(),
            SystemTime::now(),
            1, // TODO: Track actual delivery count
            publish.qos,
            Some(publish.pkid),
        );

        Some((message, handle))
    }

    /// Subscribe to a topic with specific QoS
    pub async fn subscribe_with_qos(&self, topic: &str, qos: QoS) -> Result<(), MQTTError> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Subscribing to MQTT topic {} with QoS {:?}", topic, qos))
            .await;

        self.client
            .subscribe(topic, qos)
            .await
            .map_err(|e| MQTTError::SubscribeError(e.to_string()))?;

        // Update state
        let mut state = self.state.lock().await;
        state.subscriptions.insert(topic.to_string(), qos);

        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Successfully subscribed to MQTT topic {}", topic))
            .await;

        Ok(())
    }
}

#[async_trait]
impl AckSubscriber for MQTTAckSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type AckHandle = MQTTAckHandle;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        // Default to QoS 1 for acknowledgment support
        self.subscribe_with_qos(topic, QoS::AtLeastOnce).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn receive_with_ack(&mut self) -> Result<(Message, Self::AckHandle), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger.info("Waiting to receive MQTT message with acknowledgment").await;

        let mut message_rx = self.message_rx.lock().await;
        let (message, mut ack_handle) = message_rx
            .recv()
            .await
            .ok_or_else(|| {
                Box::new(MQTTError::ReceiveError("Message channel closed".to_string()))
                    as Box<dyn std::error::Error + Send + Sync>
            })?;

        // Update delivery count
        let mut state = self.state.lock().await;
        let delivery_count = state
            .delivery_counts
            .entry(message.uuid.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        
        ack_handle.delivery_count = *delivery_count;

        // Store pending acknowledgment if required
        if ack_handle.requires_ack() {
            if let Some(packet_id) = ack_handle.packet_id() {
                state.pending_acks.insert(packet_id, ack_handle.clone());
            }
        }

        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Received MQTT message: topic={}, qos={:?}, requires_ack={}, message_id={}",
                ack_handle.topic(),
                ack_handle.qos(),
                ack_handle.requires_ack(),
                message.uuid
            ))
            .await;

        Ok((message, ack_handle))
    }

    async fn ack(&self, handle: Self::AckHandle) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Acknowledging MQTT message: topic={}, qos={:?}, packet_id={:?}, message_id={}",
                handle.topic(),
                handle.qos(),
                handle.packet_id(),
                handle.message_id()
            ))
            .await;

        // For MQTT, acknowledgment is handled automatically by the client library
        // for QoS > 0 messages. We just need to remove from pending acks.
        if let Some(packet_id) = handle.packet_id() {
            let mut state = self.state.lock().await;
            state.pending_acks.remove(&packet_id);
        }

        // For QoS 0, no acknowledgment is needed
        if !handle.requires_ack() {
            #[cfg(feature = "logging")]
            self.logger
                .info("QoS 0 message - no acknowledgment required")
                .await;
        }

        Ok(())
    }

    async fn nack(&self, handle: Self::AckHandle, requeue: bool) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Negatively acknowledging MQTT message: topic={}, qos={:?}, packet_id={:?}, message_id={}, requeue={}",
                handle.topic(),
                handle.qos(),
                handle.packet_id(),
                handle.message_id(),
                requeue
            ))
            .await;

        // For MQTT, negative acknowledgment behavior depends on QoS level:
        // - QoS 0: Fire and forget, no redelivery possible
        // - QoS 1: Can disconnect to trigger redelivery
        // - QoS 2: More complex, but similar to QoS 1

        if requeue && handle.requires_ack() {
            #[cfg(feature = "logging")]
            self.logger
                .info("MQTT requeue requested - message will be redelivered on reconnection")
                .await;
            
            // For MQTT, we don't explicitly acknowledge, which will cause
            // redelivery on reconnection for QoS > 0 messages
        } else {
            // Acknowledge to prevent redelivery
            self.ack(handle).await?;
        }

        Ok(())
    }

    async fn ack_batch(&self, handles: Vec<Self::AckHandle>) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Batch acknowledging {} MQTT messages", handles.len()))
            .await;

        // MQTT doesn't have native batch acknowledgment, so we process individually
        for handle in handles {
            self.ack(handle).await?;
        }

        Ok(())
    }

    async fn nack_batch(&self, handles: Vec<Self::AckHandle>, requeue: bool) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Batch negatively acknowledging {} MQTT messages (requeue: {})",
                handles.len(),
                requeue
            ))
            .await;

        // MQTT doesn't have native batch nack, so we process individually
        for handle in handles {
            self.nack(handle, requeue).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_mqtt_ack_handle_creation() {
        let handle = MQTTAckHandle::new(
            "msg-123".to_string(),
            "test/topic".to_string(),
            SystemTime::now(),
            1,
            QoS::AtLeastOnce,
            Some(42),
        );
        
        assert_eq!(handle.message_id(), "msg-123");
        assert_eq!(handle.topic(), "test/topic");
        assert_eq!(handle.delivery_count(), 1);
        assert!(!handle.is_retry());
        assert_eq!(handle.qos(), QoS::AtLeastOnce);
        assert_eq!(handle.packet_id(), Some(42));
        assert!(handle.requires_ack());
        assert!(!handle.handle_id().is_empty());
    }
    
    #[test]
    fn test_mqtt_ack_handle_qos_levels() {
        // QoS 0 - no acknowledgment required
        let handle_qos0 = MQTTAckHandle::new(
            "msg-qos0".to_string(),
            "test/topic".to_string(),
            SystemTime::now(),
            1,
            QoS::AtMostOnce,
            None,
        );
        
        assert_eq!(handle_qos0.qos(), QoS::AtMostOnce);
        assert!(!handle_qos0.requires_ack());
        assert_eq!(handle_qos0.packet_id(), None);

        // QoS 1 - acknowledgment required
        let handle_qos1 = MQTTAckHandle::new(
            "msg-qos1".to_string(),
            "test/topic".to_string(),
            SystemTime::now(),
            1,
            QoS::AtLeastOnce,
            Some(123),
        );
        
        assert_eq!(handle_qos1.qos(), QoS::AtLeastOnce);
        assert!(handle_qos1.requires_ack());
        assert_eq!(handle_qos1.packet_id(), Some(123));

        // QoS 2 - acknowledgment required
        let handle_qos2 = MQTTAckHandle::new(
            "msg-qos2".to_string(),
            "test/topic".to_string(),
            SystemTime::now(),
            1,
            QoS::ExactlyOnce,
            Some(456),
        );
        
        assert_eq!(handle_qos2.qos(), QoS::ExactlyOnce);
        assert!(handle_qos2.requires_ack());
        assert_eq!(handle_qos2.packet_id(), Some(456));
    }
    
    #[test]
    fn test_mqtt_ack_handle_retry() {
        let handle = MQTTAckHandle::new(
            "msg-retry".to_string(),
            "test/topic".to_string(),
            SystemTime::now(),
            3,
            QoS::AtLeastOnce,
            Some(789),
        );
        
        assert_eq!(handle.delivery_count(), 3);
        assert!(handle.is_retry());
    }
    
    // Note: Integration tests with actual MQTT broker would require a running MQTT instance
    // These would be added to a separate integration test suite
}
