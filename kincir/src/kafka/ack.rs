//! Acknowledgment handling for Apache Kafka message broker

use crate::ack::{AckHandle, AckSubscriber};
use crate::kafka::KafkaError;
use crate::Message;
use async_trait::async_trait;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::{ClientConfig, TopicPartitionList, Message as KafkaMessage};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Acknowledgment handle for Kafka messages
#[derive(Debug, Clone)]
pub struct KafkaAckHandle {
    /// Unique message identifier
    message_id: String,
    /// Topic name
    topic: String,
    /// Message timestamp
    timestamp: SystemTime,
    /// Delivery attempt count
    delivery_count: u32,
    /// Kafka partition
    partition: i32,
    /// Kafka offset
    offset: i64,
    /// Internal handle ID for tracking
    handle_id: String,
}

impl KafkaAckHandle {
    /// Create a new Kafka acknowledgment handle
    pub fn new(
        message_id: String,
        topic: String,
        timestamp: SystemTime,
        delivery_count: u32,
        partition: i32,
        offset: i64,
    ) -> Self {
        Self {
            message_id,
            topic,
            timestamp,
            delivery_count,
            partition,
            offset,
            handle_id: Uuid::new_v4().to_string(),
        }
    }
    
    /// Get the Kafka partition
    pub fn partition(&self) -> i32 {
        self.partition
    }
    
    /// Get the Kafka offset
    pub fn offset(&self) -> i64 {
        self.offset
    }
    
    /// Get the internal handle ID
    pub fn handle_id(&self) -> &str {
        &self.handle_id
    }
}

impl AckHandle for KafkaAckHandle {
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

/// Kafka subscriber with acknowledgment support
pub struct KafkaAckSubscriber {
    /// Kafka consumer
    consumer: StreamConsumer,
    /// Current subscription state
    state: Mutex<SubscriberState>,
    /// Consumer group ID
    group_id: String,
    /// Broker addresses
    brokers: Vec<String>,
    /// Logger for debugging (optional)
    #[cfg(feature = "logging")]
    logger: Arc<dyn crate::logging::Logger>,
}

struct SubscriberState {
    /// Current topic subscriptions
    topics: Vec<String>,
    /// Pending acknowledgments (offset -> handle)
    pending_acks: HashMap<(String, i32, i64), KafkaAckHandle>,
    /// Last committed offsets per partition
    committed_offsets: HashMap<(String, i32), i64>,
}

impl KafkaAckSubscriber {
    /// Create a new Kafka acknowledgment subscriber
    #[cfg(not(feature = "logging"))]
    pub async fn new(
        brokers: Vec<String>,
        group_id: String,
    ) -> Result<Self, KafkaError> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &group_id)
            .set("bootstrap.servers", brokers.join(","))
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false") // Manual commit for acknowledgment control
            .set("auto.offset.reset", "earliest")
            .create()
            .map_err(KafkaError::Kafka)?;

        Ok(Self {
            consumer,
            state: Mutex::new(SubscriberState {
                topics: Vec::new(),
                pending_acks: HashMap::new(),
                committed_offsets: HashMap::new(),
            }),
            group_id,
            brokers,
        })
    }

    /// Create a new Kafka acknowledgment subscriber with logging
    #[cfg(feature = "logging")]
    pub async fn new(
        brokers: Vec<String>,
        group_id: String,
    ) -> Result<Self, KafkaError> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &group_id)
            .set("bootstrap.servers", brokers.join(","))
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false") // Manual commit for acknowledgment control
            .set("auto.offset.reset", "earliest")
            .create()
            .map_err(KafkaError::Kafka)?;

        let logger = Arc::new(crate::logging::NoOpLogger::new());

        Ok(Self {
            consumer,
            state: Mutex::new(SubscriberState {
                topics: Vec::new(),
                pending_acks: HashMap::new(),
                committed_offsets: HashMap::new(),
            }),
            group_id,
            brokers,
            logger,
        })
    }

    /// Set a logger for the subscriber (only available with the "logging" feature)
    #[cfg(feature = "logging")]
    pub fn with_logger(mut self, logger: Arc<dyn crate::logging::Logger>) -> Self {
        self.logger = logger;
        self
    }

    /// Get the consumer group ID
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    /// Get the broker addresses
    pub fn brokers(&self) -> &[String] {
        &self.brokers
    }

    /// Get the currently subscribed topics
    pub async fn subscribed_topics(&self) -> Vec<String> {
        let state = self.state.lock().await;
        state.topics.clone()
    }

    /// Check if currently subscribed to any topics
    pub async fn is_subscribed(&self) -> bool {
        let state = self.state.lock().await;
        !state.topics.is_empty()
    }

    /// Commit a specific offset for a topic/partition
    async fn commit_offset(&self, topic: &str, partition: i32, offset: i64) -> Result<(), KafkaError> {
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(offset + 1))
            .map_err(|e| KafkaError::ConfigurationError(format!("Failed to add partition offset: {:?}", e)))?;

        self.consumer
            .commit(&tpl, rdkafka::consumer::CommitMode::Sync)
            .map_err(KafkaError::Kafka)?;

        // Update committed offset tracking
        let mut state = self.state.lock().await;
        state.committed_offsets.insert((topic.to_string(), partition), offset);

        Ok(())
    }

    /// Commit multiple offsets in batch
    async fn commit_offsets_batch(&self, offsets: Vec<(String, i32, i64)>) -> Result<(), KafkaError> {
        if offsets.is_empty() {
            return Ok(());
        }

        let mut tpl = TopicPartitionList::new();
        for (topic, partition, offset) in &offsets {
            tpl.add_partition_offset(topic, *partition, rdkafka::Offset::Offset(offset + 1))
                .map_err(|e| KafkaError::ConfigurationError(format!("Failed to add partition offset: {:?}", e)))?;
        }

        self.consumer
            .commit(&tpl, rdkafka::consumer::CommitMode::Sync)
            .map_err(KafkaError::Kafka)?;

        // Update committed offset tracking
        let mut state = self.state.lock().await;
        for (topic, partition, offset) in offsets {
            state.committed_offsets.insert((topic, partition), offset);
        }

        Ok(())
    }

    /// Convert Kafka message to Kincir message
    fn convert_message(&self, kafka_msg: &BorrowedMessage) -> Result<Message, KafkaError> {
        let payload = kafka_msg.payload()
            .ok_or_else(|| KafkaError::Serialization("Empty message payload".to_string()))?
            .to_vec();

        // Try to deserialize as JSON first, fallback to raw payload
        match serde_json::from_slice::<Message>(&payload) {
            Ok(message) => Ok(message),
            Err(_) => {
                // Create a new message with the raw payload
                Ok(Message::new(payload))
            }
        }
    }
}

#[async_trait]
impl AckSubscriber for KafkaAckSubscriber {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type AckHandle = KafkaAckHandle;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Subscribing to Kafka topic {} with acknowledgment support", topic))
            .await;

        // Subscribe to the topic
        self.consumer
            .subscribe(&[topic])
            .map_err(|e| Box::new(KafkaError::Kafka(e)) as Box<dyn std::error::Error + Send + Sync>)?;

        // Update state
        let mut state = self.state.lock().await;
        if !state.topics.contains(&topic.to_string()) {
            state.topics.push(topic.to_string());
        }

        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Successfully subscribed to Kafka topic {}", topic))
            .await;

        Ok(())
    }

    async fn receive_with_ack(&mut self) -> Result<(Message, Self::AckHandle), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger.info("Waiting to receive Kafka message with acknowledgment").await;

        use futures::StreamExt;

        let kafka_msg = self.consumer
            .stream()
            .next()
            .await
            .ok_or_else(|| {
                Box::new(KafkaError::Kafka(rdkafka::error::KafkaError::NoMessageReceived))
                    as Box<dyn std::error::Error + Send + Sync>
            })?
            .map_err(|e| {
                Box::new(KafkaError::Kafka(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

        // Convert Kafka message to Kincir message
        let message = self.convert_message(&kafka_msg)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Create acknowledgment handle
        let ack_handle = KafkaAckHandle::new(
            message.uuid.clone(),
            kafka_msg.topic().to_string(),
            SystemTime::now(),
            1, // TODO: Track actual delivery count from Kafka headers
            kafka_msg.partition(),
            kafka_msg.offset(),
        );

        // Store pending acknowledgment
        let mut state = self.state.lock().await;
        let key = (
            kafka_msg.topic().to_string(),
            kafka_msg.partition(),
            kafka_msg.offset(),
        );
        state.pending_acks.insert(key, ack_handle.clone());

        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Received Kafka message: topic={}, partition={}, offset={}, message_id={}",
                kafka_msg.topic(),
                kafka_msg.partition(),
                kafka_msg.offset(),
                message.uuid
            ))
            .await;

        Ok((message, ack_handle))
    }

    async fn ack(&self, handle: Self::AckHandle) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Acknowledging Kafka message: topic={}, partition={}, offset={}, message_id={}",
                handle.topic(),
                handle.partition(),
                handle.offset(),
                handle.message_id()
            ))
            .await;

        // Commit the offset
        self.commit_offset(handle.topic(), handle.partition(), handle.offset())
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Remove from pending acknowledgments
        let mut state = self.state.lock().await;
        let key = (handle.topic().to_string(), handle.partition(), handle.offset());
        state.pending_acks.remove(&key);

        Ok(())
    }

    async fn nack(&self, handle: Self::AckHandle, requeue: bool) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Negatively acknowledging Kafka message: topic={}, partition={}, offset={}, message_id={}, requeue={}",
                handle.topic(),
                handle.partition(),
                handle.offset(),
                handle.message_id(),
                requeue
            ))
            .await;

        if requeue {
            // For Kafka, "requeue" means we don't commit the offset
            // The message will be redelivered on next consumer restart or rebalance
            #[cfg(feature = "logging")]
            self.logger
                .info(&format!(
                    "Message will be redelivered: topic={}, partition={}, offset={}",
                    handle.topic(),
                    handle.partition(),
                    handle.offset()
                ))
                .await;
        } else {
            // Discard by committing the offset (skip this message)
            self.commit_offset(handle.topic(), handle.partition(), handle.offset())
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }

        // Remove from pending acknowledgments
        let mut state = self.state.lock().await;
        let key = (handle.topic().to_string(), handle.partition(), handle.offset());
        state.pending_acks.remove(&key);

        Ok(())
    }

    async fn ack_batch(&self, handles: Vec<Self::AckHandle>) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Batch acknowledging {} Kafka messages", handles.len()))
            .await;

        if handles.is_empty() {
            return Ok(());
        }

        // Group by topic/partition and find the highest offset for each
        let mut max_offsets: HashMap<(String, i32), i64> = HashMap::new();
        
        for handle in &handles {
            let key = (handle.topic().to_string(), handle.partition());
            let current_max = max_offsets.get(&key).copied().unwrap_or(-1);
            if handle.offset() > current_max {
                max_offsets.insert(key, handle.offset());
            }
        }

        // Commit the highest offsets
        let offsets: Vec<(String, i32, i64)> = max_offsets
            .into_iter()
            .map(|((topic, partition), offset)| (topic, partition, offset))
            .collect();

        self.commit_offsets_batch(offsets)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Remove from pending acknowledgments
        let mut state = self.state.lock().await;
        for handle in handles {
            let key = (handle.topic().to_string(), handle.partition(), handle.offset());
            state.pending_acks.remove(&key);
        }

        Ok(())
    }

    async fn nack_batch(&self, handles: Vec<Self::AckHandle>, requeue: bool) -> Result<(), Self::Error> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Batch negatively acknowledging {} Kafka messages (requeue: {})",
                handles.len(),
                requeue
            ))
            .await;

        if handles.is_empty() {
            return Ok(());
        }

        if !requeue {
            // Discard all messages by committing their offsets
            let offsets: Vec<(String, i32, i64)> = handles
                .iter()
                .map(|h| (h.topic().to_string(), h.partition(), h.offset()))
                .collect();

            self.commit_offsets_batch(offsets)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }
        // If requeue=true, we don't commit offsets, so messages will be redelivered

        // Remove from pending acknowledgments
        let mut state = self.state.lock().await;
        for handle in handles {
            let key = (handle.topic().to_string(), handle.partition(), handle.offset());
            state.pending_acks.remove(&key);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_kafka_ack_handle_creation() {
        let handle = KafkaAckHandle::new(
            "msg-123".to_string(),
            "test-topic".to_string(),
            SystemTime::now(),
            1,
            0,
            12345,
        );
        
        assert_eq!(handle.message_id(), "msg-123");
        assert_eq!(handle.topic(), "test-topic");
        assert_eq!(handle.delivery_count(), 1);
        assert!(!handle.is_retry());
        assert_eq!(handle.partition(), 0);
        assert_eq!(handle.offset(), 12345);
        assert!(!handle.handle_id().is_empty());
    }
    
    #[test]
    fn test_kafka_ack_handle_retry() {
        let handle = KafkaAckHandle::new(
            "msg-456".to_string(),
            "test-topic".to_string(),
            SystemTime::now(),
            3,
            1,
            67890,
        );
        
        assert_eq!(handle.delivery_count(), 3);
        assert!(handle.is_retry());
        assert_eq!(handle.partition(), 1);
        assert_eq!(handle.offset(), 67890);
    }
    
    // Note: Integration tests with actual Kafka would require a running Kafka instance
    // These would be added to a separate integration test suite
}
