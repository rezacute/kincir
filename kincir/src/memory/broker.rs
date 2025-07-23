use crate::Message;
use super::{InMemoryConfig, InMemoryError, BrokerStats};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock, Weak};
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

/// A subscriber handle for managing message delivery
pub type SubscriberSender = mpsc::UnboundedSender<Message>;

/// Topic information and message queue
#[derive(Debug)]
struct TopicData {
    /// Message queue for this topic
    queue: VecDeque<Message>,
    /// Active subscribers for this topic
    subscribers: Vec<SubscriberSender>,
    /// Topic creation timestamp
    created_at: Instant,
    /// Total messages published to this topic
    total_published: u64,
    /// Total messages consumed from this topic
    total_consumed: u64,
    /// Message sequence number for ordering
    next_sequence: u64,
    /// Last activity timestamp
    last_activity: Instant,
}

impl TopicData {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            queue: VecDeque::new(),
            subscribers: Vec::new(),
            created_at: now,
            total_published: 0,
            total_consumed: 0,
            next_sequence: 1,
            last_activity: now,
        }
    }
    
    fn add_message(&mut self, mut message: Message, max_queue_size: Option<usize>, add_metadata: bool) -> Result<(), InMemoryError> {
        if let Some(max_size) = max_queue_size {
            if self.queue.len() >= max_size {
                return Err(InMemoryError::queue_full("topic"));
            }
        }
        
        // Add sequence number and timestamp only if requested
        if add_metadata {
            message.metadata.insert("_sequence".to_string(), self.next_sequence.to_string());
            self.next_sequence += 1;
            
            let now_millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            message.metadata.insert("_enqueued_at".to_string(), now_millis.to_string());
        }
        
        self.queue.push_back(message);
        self.total_published += 1;
        self.last_activity = Instant::now();
        Ok(())
    }
    
    fn cleanup_expired_messages(&mut self, ttl: Duration) -> usize {
        let now_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        
        let ttl_millis = ttl.as_millis();
        let mut removed_count = 0;
        
        // Remove expired messages from the front of the queue
        while let Some(message) = self.queue.front() {
            if let Some(enqueued_at_str) = message.metadata.get("_enqueued_at") {
                if let Ok(enqueued_at) = enqueued_at_str.parse::<u128>() {
                    if now_millis.saturating_sub(enqueued_at) > ttl_millis {
                        self.queue.pop_front();
                        removed_count += 1;
                        continue;
                    }
                }
            }
            break; // Stop at first non-expired message (FIFO order)
        }
        
        if removed_count > 0 {
            self.last_activity = Instant::now();
        }
        
        removed_count
    }
    
    fn take_message(&mut self) -> Option<Message> {
        let message = self.queue.pop_front();
        if message.is_some() {
            self.total_consumed += 1;
            self.last_activity = Instant::now();
        }
        message
    }
    
    fn add_subscriber(&mut self, sender: SubscriberSender, max_subscribers: Option<usize>) -> Result<(), InMemoryError> {
        if let Some(max_subs) = max_subscribers {
            if self.subscribers.len() >= max_subs {
                return Err(InMemoryError::subscriber_already_exists("topic"));
            }
        }
        
        self.subscribers.push(sender);
        self.last_activity = Instant::now();
        Ok(())
    }
    
    fn remove_subscriber(&mut self, sender_id: &mpsc::UnboundedSender<Message>) {
        self.subscribers.retain(|s| !std::ptr::eq(s, sender_id));
        self.last_activity = Instant::now();
    }
    
    fn broadcast_message(&mut self, message: &Message) -> Result<(), InMemoryError> {
        // Remove closed subscribers
        self.subscribers.retain(|sender| !sender.is_closed());
        
        // Send message to all active subscribers
        for sender in &self.subscribers {
            if let Err(e) = sender.send(message.clone()) {
                // Log error but continue with other subscribers
                #[cfg(feature = "logging")]
                tracing::warn!("Failed to send message to subscriber: {}", e);
            }
        }
        
        self.last_activity = Instant::now();
        Ok(())
    }
    
    fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
    
    fn queue_size(&self) -> usize {
        self.queue.len()
    }
    
    fn is_idle(&self, idle_threshold: Duration) -> bool {
        self.last_activity.elapsed() > idle_threshold
    }
    
    fn next_sequence_number(&self) -> u64 {
        self.next_sequence
    }
}

/// In-memory message broker that provides thread-safe topic management
#[derive(Debug)]
pub struct InMemoryBroker {
    /// Topic storage with thread-safe access
    topics: Arc<RwLock<HashMap<String, TopicData>>>,
    /// Broker configuration
    config: InMemoryConfig,
    /// Broker statistics (optional)
    stats: Option<Arc<BrokerStats>>,
    /// Shutdown flag
    shutdown: Arc<RwLock<bool>>,
}

impl InMemoryBroker {
    /// Create a new in-memory broker with the given configuration
    pub fn new(config: InMemoryConfig) -> Self {
        // Validate configuration
        if let Err(e) = config.validate() {
            panic!("Invalid InMemoryConfig: {}", e);
        }
        
        let stats = if config.enable_stats {
            Some(Arc::new(BrokerStats::new()))
        } else {
            None
        };
        
        let topics = Arc::new(RwLock::new(HashMap::new()));
        let shutdown = Arc::new(RwLock::new(false));
        
        // Start cleanup task if TTL is enabled
        if let Some(ttl) = config.default_message_ttl {
            let topics_clone = Arc::clone(&topics);
            let shutdown_clone = Arc::clone(&shutdown);
            let cleanup_interval = config.cleanup_interval;
            
            tokio::spawn(async move {
                Self::cleanup_task(topics_clone, shutdown_clone, cleanup_interval, ttl).await;
            });
        }
        
        Self {
            topics,
            config,
            stats,
            shutdown,
        }
    }
    
    /// Background task for cleaning up expired messages
    async fn cleanup_task(
        topics: Arc<RwLock<HashMap<String, TopicData>>>,
        shutdown: Arc<RwLock<bool>>,
        interval: Duration,
        ttl: Duration,
    ) {
        let mut cleanup_interval = tokio::time::interval(interval);
        
        loop {
            cleanup_interval.tick().await;
            
            // Check if broker is shutdown
            if *shutdown.read().unwrap() {
                break;
            }
            
            // Clean up expired messages
            let mut topics_guard = topics.write().unwrap();
            let mut total_removed = 0;
            
            for topic_data in topics_guard.values_mut() {
                total_removed += topic_data.cleanup_expired_messages(ttl);
            }
            
            if total_removed > 0 {
                #[cfg(feature = "logging")]
                tracing::debug!("Cleaned up {} expired messages", total_removed);
            }
        }
    }
    
    /// Create a new broker with default configuration
    pub fn with_default_config() -> Self {
        Self::new(InMemoryConfig::default())
    }
    
    /// Get broker configuration
    pub fn config(&self) -> &InMemoryConfig {
        &self.config
    }
    
    /// Get broker statistics (if enabled)
    pub fn stats(&self) -> Option<Arc<BrokerStats>> {
        self.stats.clone()
    }
    
    /// Check if broker is shutdown
    pub fn is_shutdown(&self) -> bool {
        *self.shutdown.read().unwrap()
    }
    
    /// Publish messages to a topic
    pub fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), InMemoryError> {
        if self.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }
        
        let start_time = Instant::now();
        
        // Validate topic name
        if topic.is_empty() || topic.contains('\0') {
            return Err(InMemoryError::invalid_topic_name(topic));
        }
        
        // Single lock acquisition for the entire operation
        let mut topics = self.topics.write().unwrap();
        
        // Check topic limit
        if let Some(max_topics) = self.config.max_topics {
            if !topics.contains_key(topic) && topics.len() >= max_topics {
                if let Some(stats) = &self.stats {
                    stats.increment_publish_errors();
                }
                return Err(InMemoryError::max_topics_reached(max_topics));
            }
        }
        
        // Get or create topic data
        let topic_data = topics.entry(topic.to_string()).or_insert_with(|| {
            if let Some(stats) = &self.stats {
                stats.increment_topics_created();
            }
            TopicData::new()
        });
        
        for mut message in messages {
            // Add metadata for ordering and timestamps
            if self.config.maintain_order {
                message.metadata.insert("_sequence".to_string(), topic_data.next_sequence.to_string());
                topic_data.next_sequence += 1;
            }
            
            // Add processing timestamp
            let now_millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            message.metadata.insert("_enqueued_at".to_string(), now_millis.to_string());
            
            // Add to queue if persistence is enabled
            if self.config.enable_persistence {
                topic_data.add_message(message.clone(), self.config.max_queue_size, false)?; // Don't add metadata again
            }
            
            // Broadcast to subscribers
            topic_data.broadcast_message(&message)?;
            
            if let Some(stats) = &self.stats {
                stats.increment_messages_published(1);
            }
        }
        
        // Update performance stats
        if let Some(stats) = &self.stats {
            stats.add_publish_time(start_time.elapsed());
        }
        
        Ok(())
    }
    
    /// Subscribe to a topic
    pub fn subscribe(&self, topic: &str) -> Result<mpsc::UnboundedReceiver<Message>, InMemoryError> {
        if self.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }
        
        // Validate topic name
        if topic.is_empty() || topic.contains('\0') {
            return Err(InMemoryError::invalid_topic_name(topic));
        }
        
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let mut topics = self.topics.write().unwrap();
        
        // Check topic limit
        if let Some(max_topics) = self.config.max_topics {
            if !topics.contains_key(topic) && topics.len() >= max_topics {
                return Err(InMemoryError::max_topics_reached(max_topics));
            }
        }
        
        // Get or create topic
        let topic_data = topics.entry(topic.to_string()).or_insert_with(|| {
            if let Some(stats) = &self.stats {
                stats.increment_topics_created();
            }
            TopicData::new()
        });
        
        // Add subscriber
        topic_data.add_subscriber(sender, self.config.max_subscribers_per_topic)?;
        
        if let Some(stats) = &self.stats {
            stats.increment_subscribers_connected();
        }
        
        Ok(receiver)
    }
    
    /// Get a message from a topic queue (for pull-based consumption)
    pub fn consume(&self, topic: &str) -> Result<Option<Message>, InMemoryError> {
        if self.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }
        
        let start_time = Instant::now();
        
        let mut topics = self.topics.write().unwrap();
        
        let message = if let Some(topic_data) = topics.get_mut(topic) {
            topic_data.take_message()
        } else {
            return Err(InMemoryError::topic_not_found(topic));
        };
        
        if message.is_some() {
            if let Some(stats) = &self.stats {
                stats.increment_messages_consumed(1);
                stats.add_consume_time(start_time.elapsed());
            }
        }
        
        Ok(message)
    }
    
    /// Get topic information
    pub fn topic_info(&self, topic: &str) -> Result<TopicInfo, InMemoryError> {
        let topics = self.topics.read().unwrap();
        
        if let Some(topic_data) = topics.get(topic) {
            Ok(TopicInfo {
                name: topic.to_string(),
                queue_size: topic_data.queue_size(),
                subscriber_count: topic_data.subscriber_count(),
                total_published: topic_data.total_published,
                total_consumed: topic_data.total_consumed,
                created_at: topic_data.created_at,
                last_activity: topic_data.last_activity,
                next_sequence: topic_data.next_sequence_number(),
            })
        } else {
            Err(InMemoryError::topic_not_found(topic))
        }
    }
    
    /// List all topics
    pub fn list_topics(&self) -> Vec<String> {
        let topics = self.topics.read().unwrap();
        topics.keys().cloned().collect()
    }
    
    /// Get total number of topics
    pub fn topic_count(&self) -> usize {
        let topics = self.topics.read().unwrap();
        topics.len()
    }
    
    /// Delete a topic
    pub fn delete_topic(&self, topic: &str) -> Result<(), InMemoryError> {
        let mut topics = self.topics.write().unwrap();
        
        if topics.remove(topic).is_some() {
            if let Some(stats) = &self.stats {
                stats.increment_topics_deleted();
            }
            Ok(())
        } else {
            Err(InMemoryError::topic_not_found(topic))
        }
    }
    
    /// Clear all messages from a topic
    pub fn clear_topic(&self, topic: &str) -> Result<usize, InMemoryError> {
        let mut topics = self.topics.write().unwrap();
        
        if let Some(topic_data) = topics.get_mut(topic) {
            let cleared_count = topic_data.queue.len();
            topic_data.queue.clear();
            Ok(cleared_count)
        } else {
            Err(InMemoryError::topic_not_found(topic))
        }
    }
    
    /// Get weak reference to broker for use in handles
    pub fn downgrade(self: &Arc<Self>) -> Weak<Self> {
        Arc::downgrade(self)
    }
    
    /// Get all topic information
    pub fn list_topic_info(&self) -> Vec<TopicInfo> {
        let topics = self.topics.read().unwrap();
        topics.iter().map(|(name, data)| {
            TopicInfo {
                name: name.clone(),
                queue_size: data.queue_size(),
                subscriber_count: data.subscriber_count(),
                total_published: data.total_published,
                total_consumed: data.total_consumed,
                created_at: data.created_at,
                last_activity: data.last_activity,
                next_sequence: data.next_sequence_number(),
            }
        }).collect()
    }
    
    /// Clean up idle topics (topics with no activity for specified duration)
    pub fn cleanup_idle_topics(&self, idle_threshold: Duration) -> Result<Vec<String>, InMemoryError> {
        let mut topics = self.topics.write().unwrap();
        let mut removed_topics = Vec::new();
        
        // Find idle topics
        let idle_topic_names: Vec<String> = topics.iter()
            .filter(|(_, data)| data.is_idle(idle_threshold) && data.subscriber_count() == 0)
            .map(|(name, _)| name.clone())
            .collect();
        
        // Remove idle topics
        for topic_name in idle_topic_names {
            if topics.remove(&topic_name).is_some() {
                removed_topics.push(topic_name);
                if let Some(stats) = &self.stats {
                    stats.increment_topics_deleted();
                }
            }
        }
        
        Ok(removed_topics)
    }
    
    /// Get broker health information
    pub fn health_check(&self) -> BrokerHealth {
        let topics = self.topics.read().unwrap();
        let total_messages: usize = topics.values().map(|t| t.queue_size()).sum();
        let total_subscribers: usize = topics.values().map(|t| t.subscriber_count()).sum();
        
        // Calculate memory usage while we have the lock
        let memory_usage_estimate = self.estimate_memory_usage_with_lock(&topics);
        
        BrokerHealth {
            is_healthy: !self.is_shutdown(),
            topic_count: topics.len(),
            total_queued_messages: total_messages,
            total_subscribers,
            uptime: self.stats.as_ref().map(|s| s.uptime()).unwrap_or_default(),
            memory_usage_estimate,
        }
    }
    
    /// Estimate memory usage with existing lock (to avoid deadlock)
    fn estimate_memory_usage_with_lock(&self, topics: &std::collections::HashMap<String, TopicData>) -> usize {
        let mut total_size = 0;
        
        for (topic_name, topic_data) in topics.iter() {
            // Topic name size
            total_size += topic_name.len();
            
            // Messages in queue
            for message in &topic_data.queue {
                total_size += message.payload.len();
                total_size += message.uuid.len();
                for (k, v) in &message.metadata {
                    total_size += k.len() + v.len();
                }
            }
            
            // Subscriber channels (rough estimate)
            total_size += topic_data.subscribers.len() * 64; // Rough channel overhead
        }
        
        total_size
    }
    
    /// Estimate memory usage (rough calculation)
    fn estimate_memory_usage(&self) -> usize {
        let topics = self.topics.read().unwrap();
        self.estimate_memory_usage_with_lock(&topics)
    }
    
    /// Force shutdown (immediate)
    pub fn force_shutdown(&self) -> Result<(), InMemoryError> {
        let mut shutdown = self.shutdown.write().unwrap();
        *shutdown = true;
        
        // Clear all topics and notify subscribers
        let mut topics = self.topics.write().unwrap();
        topics.clear();
        
        Ok(())
    }
    
    /// Shutdown the broker (legacy method, same as force_shutdown)
    pub fn shutdown(&self) -> Result<(), InMemoryError> {
        self.force_shutdown()
    }
}

/// Information about a topic
#[derive(Debug, Clone)]
pub struct TopicInfo {
    pub name: String,
    pub queue_size: usize,
    pub subscriber_count: usize,
    pub total_published: u64,
    pub total_consumed: u64,
    pub created_at: Instant,
    pub last_activity: Instant,
    pub next_sequence: u64,
}

impl TopicInfo {
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
    
    pub fn idle_time(&self) -> Duration {
        self.last_activity.elapsed()
    }
    
    pub fn is_active(&self) -> bool {
        self.subscriber_count > 0 || self.queue_size > 0
    }
    
    pub fn throughput_rate(&self) -> f64 {
        let age_secs = self.age().as_secs_f64();
        if age_secs > 0.0 {
            self.total_published as f64 / age_secs
        } else {
            0.0
        }
    }
}

/// Broker health information
#[derive(Debug, Clone)]
pub struct BrokerHealth {
    pub is_healthy: bool,
    pub topic_count: usize,
    pub total_queued_messages: usize,
    pub total_subscribers: usize,
    pub uptime: Duration,
    pub memory_usage_estimate: usize,
}

impl BrokerHealth {
    pub fn is_overloaded(&self, max_topics: usize, max_messages: usize) -> bool {
        self.topic_count > max_topics || self.total_queued_messages > max_messages
    }
    
    pub fn memory_usage_mb(&self) -> f64 {
        self.memory_usage_estimate as f64 / (1024.0 * 1024.0)
    }
}

// Additional methods for InMemoryBroker
impl InMemoryBroker {
    /// Subscribe to a topic with acknowledgment support
    /// 
    /// Returns a receiver that provides messages with acknowledgment handles
    pub fn subscribe_with_ack(&self, topic: &str) -> Result<tokio::sync::mpsc::UnboundedReceiver<(crate::Message, crate::memory::ack::InMemoryAckHandle)>, InMemoryError> {
        if self.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }
        
        // Validate topic name
        if topic.is_empty() || topic.contains('\0') {
            return Err(InMemoryError::invalid_topic_name(topic));
        }
        
        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        
        // For now, return the receiver - we'll implement the full logic later
        // TODO: Implement proper acknowledgment-aware message delivery
        
        if let Some(stats) = &self.stats {
            stats.increment_subscribers_connected();
        }
        
        Ok(rx)
    }
    
    /// Acknowledge a message
    pub async fn ack_message(&self, _handle: &crate::memory::ack::InMemoryAckHandle) -> Result<(), InMemoryError> {
        if self.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }
        
        // For in-memory broker, acknowledgment means the message was successfully processed
        if let Some(stats) = &self.stats {
            stats.increment_messages_consumed(1);
        }
        
        // TODO: Implement message tracking and cleanup if needed
        Ok(())
    }
    
    /// Negatively acknowledge a message
    pub async fn nack_message(&self, _handle: &crate::memory::ack::InMemoryAckHandle, requeue: bool) -> Result<(), InMemoryError> {
        if self.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }
        
        if requeue {
            // Requeue the message for redelivery
            if let Some(stats) = &self.stats {
                stats.increment_publish_errors(); // Track as requeue
            }
        } else {
            // Send to dead letter queue or discard
            if let Some(stats) = &self.stats {
                stats.increment_consume_errors(); // Track as dead letter
            }
        }
        
        Ok(())
    }
    
    /// Acknowledge multiple messages in batch
    pub async fn ack_batch(&self, handles: &[crate::memory::ack::InMemoryAckHandle]) -> Result<(), InMemoryError> {
        if self.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }
        
        // For batch operations, we can optimize by updating stats in bulk
        if let Some(stats) = &self.stats {
            stats.increment_messages_consumed(handles.len() as u64);
        }
        
        // TODO: Implement batch acknowledgment logic
        Ok(())
    }
    
    /// Negatively acknowledge multiple messages in batch
    pub async fn nack_batch(&self, _handles: &[crate::memory::ack::InMemoryAckHandle], requeue: bool) -> Result<(), InMemoryError> {
        if self.is_shutdown() {
            return Err(InMemoryError::BrokerShutdown);
        }
        
        if requeue {
            // Batch requeue operation
            if let Some(stats) = &self.stats {
                stats.increment_publish_errors(); // Track requeues
            }
        } else {
            // Batch dead letter operation
            if let Some(stats) = &self.stats {
                stats.increment_consume_errors(); // Track dead letters
            }
        }
        
        // TODO: Implement batch nack logic
        Ok(())
    }
}

// Implement Clone for InMemoryBroker to allow sharing
impl Clone for InMemoryBroker {
    fn clone(&self) -> Self {
        Self {
            topics: Arc::clone(&self.topics),
            config: self.config.clone(),
            stats: self.stats.clone(),
            shutdown: Arc::clone(&self.shutdown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_broker_creation() {
        let config = InMemoryConfig::for_testing();
        let broker = InMemoryBroker::new(config);
        
        assert_eq!(broker.topic_count(), 0);
        assert!(!broker.is_shutdown());
    }
    
    #[test]
    fn test_topic_creation_and_listing() {
        let broker = InMemoryBroker::with_default_config();
        
        // Subscribe to create topic
        let _receiver = broker.subscribe("test-topic").unwrap();
        
        assert_eq!(broker.topic_count(), 1);
        let topics = broker.list_topics();
        assert!(topics.contains(&"test-topic".to_string()));
    }
    
    #[test]
    fn test_publish_and_consume() {
        let broker = InMemoryBroker::with_default_config();
        
        let message = Message::new(b"test message".to_vec());
        broker.publish("test-topic", vec![message.clone()]).unwrap();
        
        let consumed = broker.consume("test-topic").unwrap();
        assert!(consumed.is_some());
        assert_eq!(consumed.unwrap().payload, message.payload);
    }
    
    #[test]
    fn test_subscribe_and_receive() {
        let broker = InMemoryBroker::with_default_config();
        let mut receiver = broker.subscribe("test-topic").unwrap();
        
        let message = Message::new(b"broadcast message".to_vec());
        broker.publish("test-topic", vec![message.clone()]).unwrap();
        
        // Should receive message immediately since it's broadcast
        let received = receiver.try_recv().unwrap();
        assert_eq!(received.payload, message.payload);
    }
    
    #[test]
    fn test_topic_limits() {
        let config = InMemoryConfig::new().with_max_topics(Some(1));
        let broker = InMemoryBroker::new(config);
        
        // First topic should succeed
        broker.subscribe("topic1").unwrap();
        
        // Second topic should fail
        let result = broker.subscribe("topic2");
        assert!(matches!(result, Err(InMemoryError::MaxTopicsReached { .. })));
    }
    
    #[test]
    fn test_queue_size_limits() {
        let config = InMemoryConfig::new().with_max_queue_size(Some(1));
        let broker = InMemoryBroker::new(config);
        
        // First message should succeed
        let message1 = Message::new(b"message1".to_vec());
        broker.publish("test-topic", vec![message1]).unwrap();
        
        // Second message should fail due to queue limit
        let message2 = Message::new(b"message2".to_vec());
        let result = broker.publish("test-topic", vec![message2]);
        assert!(matches!(result, Err(InMemoryError::QueueFull { .. })));
    }
    
    #[test]
    fn test_topic_info() {
        let broker = InMemoryBroker::with_default_config();
        let _receiver = broker.subscribe("test-topic").unwrap();
        
        let message = Message::new(b"test".to_vec());
        broker.publish("test-topic", vec![message]).unwrap();
        
        let info = broker.topic_info("test-topic").unwrap();
        assert_eq!(info.name, "test-topic");
        assert_eq!(info.queue_size, 1);
        assert_eq!(info.subscriber_count, 1);
        assert_eq!(info.total_published, 1);
    }
    
    #[test]
    fn test_broker_shutdown() {
        let broker = InMemoryBroker::with_default_config();
        
        assert!(!broker.is_shutdown());
        broker.shutdown().unwrap();
        assert!(broker.is_shutdown());
        
        // Operations should fail after shutdown
        let result = broker.publish("test", vec![Message::new(b"test".to_vec())]);
        assert!(matches!(result, Err(InMemoryError::BrokerShutdown)));
    }
    
    #[test]
    fn test_invalid_topic_names() {
        let broker = InMemoryBroker::with_default_config();
        
        // Empty topic name
        let result = broker.subscribe("");
        assert!(matches!(result, Err(InMemoryError::InvalidTopicName { .. })));
        
        // Topic name with null character
        let result = broker.subscribe("test\0topic");
        assert!(matches!(result, Err(InMemoryError::InvalidTopicName { .. })));
    }
}
