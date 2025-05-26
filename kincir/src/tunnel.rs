// kincir/src/tunnel.rs

use crate::kafka::KafkaPublisher;
use crate::mqtt::MQTTSubscriber;
use crate::rabbitmq::RabbitMQPublisher;
use crate::Publisher; // The trait
use crate::Subscriber; // The trait
use rumqttc; // Ensure rumqttc is available for QoS enum
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinHandle;
#[cfg(feature = "logging")]
use tracing::{debug, error, info, warn};

// Re-export or define necessary MQTT and Kafka types if not directly accessible
// For now, assume MqttConfig and KafkaConfig will be defined here.
// May need to adjust imports based on actual kincir::mqtt and kincir::kafka modules.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttTunnelConfig {
    pub broker_url: String,
    pub topics: Vec<String>,
    pub qos: u8, // Changed back to u8 to avoid Serde issues with rumqttc::QoS
                 // Add fields for authentication later if needed
}

impl MqttTunnelConfig {
    pub fn new(broker_url: &str, topics: Vec<String>, qos: u8) -> Self {
        Self {
            broker_url: broker_url.to_string(),
            topics,
            qos,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaTunnelConfig {
    pub broker_urls: Vec<String>,
    pub topic: String,
    // Add fields for authentication later if needed
}

impl KafkaTunnelConfig {
    pub fn new(broker_urls: Vec<String>, topic: &str) -> Self {
        Self {
            broker_urls,
            topic: topic.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RabbitMQTunnelConfig {
    pub uri: String,
    pub routing_key: String,
    // Add fields for authentication later if needed
}

impl RabbitMQTunnelConfig {
    pub fn new(uri: &str, routing_key: &str) -> Self {
        Self {
            uri: uri.to_string(),
            routing_key: routing_key.to_string(),
        }
    }
}

#[derive(Error, Debug)]
pub enum TunnelError {
    #[error("MQTT client error: {0}")]
    MqttClientError(String),
    #[error("Kafka client error: {0}")]
    KafkaClientError(String),
    #[error("Message processing error: {0}")]
    MessageProcessingError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Tunnel runtime error: {0}")]
    RuntimeError(String),
    #[error("RabbitMQ client error: {0}")]
    RabbitMQClientError(String),
}

pub struct MqttToRabbitMQTunnel {
    mqtt_config: MqttTunnelConfig,
    rabbitmq_config: RabbitMQTunnelConfig,
}

impl MqttToRabbitMQTunnel {
    pub fn new(mqtt_config: MqttTunnelConfig, rabbitmq_config: RabbitMQTunnelConfig) -> Self {
        Self {
            mqtt_config,
            rabbitmq_config,
        }
    }

    pub async fn run(&mut self) -> Result<(), TunnelError> {
        #[cfg(feature = "logging")]
        info!(
            "MqttToRabbitMQTunnel starting up for {} MQTT topics...",
            self.mqtt_config.topics.len()
        );

        if self.mqtt_config.topics.is_empty() {
            #[cfg(feature = "logging")]
            error!("No MQTT topics configured for the tunnel.");
            return Err(TunnelError::ConfigurationError(
                "No MQTT topics provided".to_string(),
            ));
        }

        // Create RabbitMQ publisher once and wrap in Arc
        let rabbitmq_publisher = RabbitMQPublisher::new(&self.rabbitmq_config.uri)
            .await // Ensure new() is awaited
            .map_err(|e| {
                TunnelError::RabbitMQClientError(format!(
                    "Failed to create RabbitMQ publisher: {}",
                    e
                ))
            })?;
        let rabbitmq_publisher_arc = Arc::new(rabbitmq_publisher); // Wrap in Arc

        let mut task_handles: Vec<JoinHandle<Result<(), TunnelError>>> = Vec::new();

        for mqtt_topic in &self.mqtt_config.topics {
            let topic_clone = mqtt_topic.clone(); // Clone topic string for the task
            let mqtt_broker_url = self.mqtt_config.broker_url.clone();
            let qos_u8 = self.mqtt_config.qos;

            let publisher_clone = Arc::clone(&rabbitmq_publisher_arc);
            let rabbitmq_routing_key = self.rabbitmq_config.routing_key.clone();

            let task = tokio::spawn(async move {
                let rumqttc_qos = match qos_u8 {
                    0 => rumqttc::QoS::AtMostOnce,
                    1 => rumqttc::QoS::AtLeastOnce,
                    2 => rumqttc::QoS::ExactlyOnce,
                    _ => {
                        #[cfg(feature = "logging")]
                        warn!("Task for {}: Invalid QoS value {} configured, defaulting to AtLeastOnce.", topic_clone, qos_u8);
                        rumqttc::QoS::AtLeastOnce
                    }
                };

                #[cfg(feature = "logging")]
                info!(
                    "Task for {}: Initializing MQTT subscriber for broker_url: {}, qos: {:?}",
                    topic_clone, mqtt_broker_url, rumqttc_qos
                );

                // Create MQTT subscriber for this specific topic
                let mut mqtt_subscriber =
                    MQTTSubscriber::new(&mqtt_broker_url, &topic_clone, rumqttc_qos).map_err(
                        |e| {
                            #[cfg(feature = "logging")]
                            error!(
                                "Task for {}: Failed to create MQTT subscriber: {}",
                                topic_clone, e
                            );
                            TunnelError::MqttClientError(format!(
                                "Task {}: MQTT subscriber creation failed: {}",
                                topic_clone, e
                            ))
                        },
                    )?;

                // Subscribe to the MQTT topic
                match mqtt_subscriber.subscribe(&topic_clone).await {
                    Ok(_) => {
                        #[cfg(feature = "logging")]
                        info!(
                            "Task for {}: Successfully subscribed to MQTT topic.",
                            topic_clone
                        );
                    }
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        error!(
                            "Task for {}: Failed to subscribe to MQTT topic: {}",
                            topic_clone, e
                        );
                        return Err(TunnelError::MqttClientError(format!(
                            "Task {}: MQTT subscription failed: {}",
                            topic_clone, e
                        )));
                    }
                }

                #[cfg(feature = "logging")]
                info!(
                    "Task for {}: Starting message forwarding loop to RabbitMQ routing key {}.",
                    topic_clone, rabbitmq_routing_key
                );

                loop {
                    match mqtt_subscriber.receive().await {
                        Ok(kincir_message) => {
                            #[cfg(feature = "logging")]
                            debug!(
                                "Task for {}: Received message UUID {} from MQTT.",
                                topic_clone, kincir_message.uuid
                            );

                            match publisher_clone
                                .publish(&rabbitmq_routing_key, vec![kincir_message.clone()])
                                .await
                            {
                                Ok(_) => {
                                    #[cfg(feature = "logging")]
                                    debug!("Task for {}: Successfully published message UUID {} to RabbitMQ routing key {}.", topic_clone, kincir_message.uuid, rabbitmq_routing_key);
                                }
                                Err(e) => {
                                    #[cfg(feature = "logging")]
                                    error!("Task for {}: Failed to publish message UUID {} to RabbitMQ: {}. Message might be lost.", topic_clone, kincir_message.uuid, e);
                                    // Decide on error handling for publish failure. For now, log and continue.
                                    // To make it more robust, this task could return an error:
                                    return Err(TunnelError::RabbitMQClientError(format!("Task {}: RabbitMQ publish failed: {}", topic_clone, e)));
                                }
                            }
                        }
                        Err(e) => {
                            #[cfg(feature = "logging")]
                            error!(
                                "Task for {}: Error receiving message from MQTT: {}.",
                                topic_clone, e
                            );
                            // This error might be critical (e.g., connection lost).
                            // The task should probably exit and report the error.
                            return Err(TunnelError::MqttClientError(format!(
                                "Task {}: MQTT receive error: {}",
                                topic_clone, e
                            )));
                        }
                    }
                }
            });
            task_handles.push(task);
        }

        // Wait for all tasks to complete
        for handle in task_handles {
            match handle.await {
                Ok(Ok(_)) => { /* Task completed successfully */ }
                Ok(Err(e)) => {
                    // One of the tasks failed
                    #[cfg(feature = "logging")]
                    error!("A tunnel task failed: {:?}", e);
                    return Err(e); // Return the first error encountered
                }
                Err(e) => {
                    // Task panicked or was cancelled
                    #[cfg(feature = "logging")]
                    error!("A tunnel task panicked or was cancelled: {:?}", e);
                    return Err(TunnelError::RuntimeError(format!(
                        "Task execution failed: {}",
                        e
                    )));
                }
            }
        }

        #[cfg(feature = "logging")]
        info!("All MqttToRabbitMQTunnel tasks completed. Shutting down (or indicates an issue if tasks exited unexpectedly).");
        Ok(())
    }
}

pub struct MqttToKafkaTunnel {
    mqtt_config: MqttTunnelConfig,
    kafka_config: KafkaTunnelConfig,
    // May need to store client instances if they are created early
    // Or, they could be created within the run() method.
}

impl MqttToKafkaTunnel {
    pub fn new(mqtt_config: MqttTunnelConfig, kafka_config: KafkaTunnelConfig) -> Self {
        Self {
            mqtt_config,
            kafka_config,
        }
    }

    pub async fn run(&mut self) -> Result<(), TunnelError> {
        #[cfg(feature = "logging")]
        info!(
            "MqttToKafkaTunnel starting up for {} MQTT topics...",
            self.mqtt_config.topics.len()
        );

        if self.mqtt_config.topics.is_empty() {
            #[cfg(feature = "logging")]
            error!("No MQTT topics configured for the tunnel.");
            return Err(TunnelError::ConfigurationError(
                "No MQTT topics provided".to_string(),
            ));
        }

        // Create Kafka publisher once. FutureProducer from rdkafka is cloneable.
        let kafka_publisher =
            KafkaPublisher::new(self.kafka_config.broker_urls.clone()).map_err(|e| {
                TunnelError::KafkaClientError(format!("Failed to create Kafka publisher: {}", e))
            })?;

        let mut task_handles: Vec<JoinHandle<Result<(), TunnelError>>> = Vec::new();

        for mqtt_topic in &self.mqtt_config.topics {
            let topic_clone = mqtt_topic.clone(); // Clone topic string for the task
            let mqtt_broker_url = self.mqtt_config.broker_url.clone();
            let qos_u8 = self.mqtt_config.qos;

            let kafka_publisher_clone = kafka_publisher.clone(); // Clone FutureProducer
            let kafka_target_topic = self.kafka_config.topic.clone();

            let task = tokio::spawn(async move {
                let rumqttc_qos = match qos_u8 {
                    0 => rumqttc::QoS::AtMostOnce,
                    1 => rumqttc::QoS::AtLeastOnce,
                    2 => rumqttc::QoS::ExactlyOnce,
                    _ => {
                        #[cfg(feature = "logging")]
                        warn!("Task for {}: Invalid QoS value {} configured, defaulting to AtLeastOnce.", topic_clone, qos_u8);
                        rumqttc::QoS::AtLeastOnce
                    }
                };

                #[cfg(feature = "logging")]
                info!(
                    "Task for {}: Initializing MQTT subscriber for broker_url: {}, qos: {:?}",
                    topic_clone, mqtt_broker_url, rumqttc_qos
                );

                // Create MQTT subscriber for this specific topic
                let mut mqtt_subscriber =
                    MQTTSubscriber::new(&mqtt_broker_url, &topic_clone, rumqttc_qos).map_err(
                        |e| {
                            #[cfg(feature = "logging")]
                            error!(
                                "Task for {}: Failed to create MQTT subscriber: {}",
                                topic_clone, e
                            );
                            TunnelError::MqttClientError(format!(
                                "Task {}: MQTT subscriber creation failed: {}",
                                topic_clone, e
                            ))
                        },
                    )?;

                // Subscribe to the MQTT topic
                match mqtt_subscriber.subscribe(&topic_clone).await {
                    Ok(_) => {
                        #[cfg(feature = "logging")]
                        info!(
                            "Task for {}: Successfully subscribed to MQTT topic.",
                            topic_clone
                        );
                    }
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        error!(
                            "Task for {}: Failed to subscribe to MQTT topic: {}",
                            topic_clone, e
                        );
                        return Err(TunnelError::MqttClientError(format!(
                            "Task {}: MQTT subscription failed: {}",
                            topic_clone, e
                        )));
                    }
                }

                #[cfg(feature = "logging")]
                info!(
                    "Task for {}: Starting message forwarding loop to Kafka topic {}.",
                    topic_clone, kafka_target_topic
                );

                loop {
                    match mqtt_subscriber.receive().await {
                        Ok(kincir_message) => {
                            #[cfg(feature = "logging")]
                            debug!(
                                "Task for {}: Received message UUID {} from MQTT.",
                                topic_clone, kincir_message.uuid
                            );

                            match kafka_publisher_clone
                                .publish(&kafka_target_topic, vec![kincir_message.clone()])
                                .await
                            {
                                Ok(_) => {
                                    #[cfg(feature = "logging")]
                                    debug!("Task for {}: Successfully published message UUID {} to Kafka topic {}.", topic_clone, kincir_message.uuid, kafka_target_topic);
                                }
                                Err(e) => {
                                    #[cfg(feature = "logging")]
                                    error!("Task for {}: Failed to publish message UUID {} to Kafka: {}. Message might be lost.", topic_clone, kincir_message.uuid, e);
                                    // Decide on error handling for publish failure. For now, log and continue.
                                    // To make it more robust, this task could return an error:
                                    // return Err(TunnelError::KafkaClientError(format!("Task {}: Kafka publish failed: {}", topic_clone, e)));
                                }
                            }
                        }
                        Err(e) => {
                            #[cfg(feature = "logging")]
                            error!(
                                "Task for {}: Error receiving message from MQTT: {}.",
                                topic_clone, e
                            );
                            // This error might be critical (e.g., connection lost).
                            // The task should probably exit and report the error.
                            return Err(TunnelError::MqttClientError(format!(
                                "Task {}: MQTT receive error: {}",
                                topic_clone, e
                            )));
                        }
                    }
                }
            });
            task_handles.push(task);
        }

        // Wait for all tasks to complete
        for handle in task_handles {
            match handle.await {
                Ok(Ok(_)) => { /* Task completed successfully */ }
                Ok(Err(e)) => {
                    // One of the tasks failed
                    #[cfg(feature = "logging")]
                    error!("A tunnel task failed: {:?}", e);
                    return Err(e); // Return the first error encountered
                }
                Err(e) => {
                    // Task panicked or was cancelled
                    #[cfg(feature = "logging")]
                    error!("A tunnel task panicked or was cancelled: {:?}", e);
                    return Err(TunnelError::RuntimeError(format!(
                        "Task execution failed: {}",
                        e
                    )));
                }
            }
        }

        #[cfg(feature = "logging")]
        info!("All MqttToKafkaTunnel tasks completed. Shutting down (or indicates an issue if tasks exited unexpectedly).");
        Ok(())
    }
}
