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
}

impl TopicData {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            subscribers: Vec::new(),
            created_at: Instant::now(),
            total_published: 0,
            total_consumed: 0,
        }
    }
    
    fn add_message(&mut self, message: Message, max_queue_size: Option<usize>) -> Result<(), InMemoryError> {
        if let Some(max_size) = max_queue_size {
            if self.queue.len() >= max_size {
                return Err(InMemoryError::queue_full("topic"));
            }
        }
        
        self.queue.push_back(message);
        self.total_published += 1;
        Ok(())
    }
    
    fn take_message(&mut self) -> Option<Message> {
        let message = self.queue.pop_front();
        if message.is_some() {
            self.total_consumed += 1;
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
        Ok(())
    }
    
    fn remove_subscriber(&mut self, sender_id: &mpsc::UnboundedSender<Message>) {
        self.subscribers.retain(|s| !std::ptr::eq(s, sender_id));
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
        
        Ok(())
    }
    
    fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
    
    fn queue_size(&self) -> usize {
        self.queue.len()
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
        
        Self {
            topics: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats,
            shutdown: Arc::new(RwLock::new(false)),
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
    
    /// Shutdown the broker
    pub fn shutdown(&self) -> Result<(), InMemoryError> {
        let mut shutdown = self.shutdown.write().unwrap();
        *shutdown = true;
        
        // Clear all topics and notify subscribers
        let mut topics = self.topics.write().unwrap();
        topics.clear();
        
        Ok(())
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
        
        // Get or create topic
        let topic_data = topics.entry(topic.to_string()).or_insert_with(|| {
            if let Some(stats) = &self.stats {
                stats.increment_topics_created();
            }
            TopicData::new()
        });
        
        // Process messages
        for message in messages {
            // Add to queue if persistence is enabled
            if self.config.enable_persistence {
                topic_data.add_message(message.clone(), self.config.max_queue_size)?;
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
}

impl TopicInfo {
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
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
    
    #[test]
    fn test_broker_creation() {
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
