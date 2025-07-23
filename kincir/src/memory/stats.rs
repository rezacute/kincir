use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

/// Statistics for the in-memory broker
#[derive(Debug)]
pub struct BrokerStats {
    // Message statistics
    pub messages_published: AtomicU64,
    pub messages_consumed: AtomicU64,
    pub messages_pending: AtomicU64,
    
    // Topic statistics
    pub topics_created: AtomicUsize,
    pub topics_deleted: AtomicUsize,
    pub active_topics: AtomicUsize,
    
    // Subscriber statistics
    pub subscribers_connected: AtomicUsize,
    pub subscribers_disconnected: AtomicUsize,
    pub active_subscribers: AtomicUsize,
    
    // Performance statistics
    pub total_publish_time_ns: AtomicU64,
    pub total_consume_time_ns: AtomicU64,
    pub peak_memory_usage: AtomicU64,
    
    // Error statistics
    pub publish_errors: AtomicU64,
    pub consume_errors: AtomicU64,
    pub queue_full_errors: AtomicU64,
    
    // Timing
    pub created_at: SystemTime,
    pub last_reset: AtomicU64, // Unix timestamp in nanoseconds
}

impl Default for BrokerStats {
    fn default() -> Self {
        Self::new()
    }
}

impl BrokerStats {
    pub fn new() -> Self {
        let now = SystemTime::now();
        let now_nanos = now.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
            
        Self {
            messages_published: AtomicU64::new(0),
            messages_consumed: AtomicU64::new(0),
            messages_pending: AtomicU64::new(0),
            topics_created: AtomicUsize::new(0),
            topics_deleted: AtomicUsize::new(0),
            active_topics: AtomicUsize::new(0),
            subscribers_connected: AtomicUsize::new(0),
            subscribers_disconnected: AtomicUsize::new(0),
            active_subscribers: AtomicUsize::new(0),
            total_publish_time_ns: AtomicU64::new(0),
            total_consume_time_ns: AtomicU64::new(0),
            peak_memory_usage: AtomicU64::new(0),
            publish_errors: AtomicU64::new(0),
            consume_errors: AtomicU64::new(0),
            queue_full_errors: AtomicU64::new(0),
            created_at: now,
            last_reset: AtomicU64::new(now_nanos),
        }
    }
    
    // Message statistics methods
    pub fn increment_messages_published(&self, count: u64) {
        self.messages_published.fetch_add(count, Ordering::Relaxed);
    }
    
    pub fn increment_messages_consumed(&self, count: u64) {
        self.messages_consumed.fetch_add(count, Ordering::Relaxed);
    }
    
    pub fn set_messages_pending(&self, count: u64) {
        self.messages_pending.store(count, Ordering::Relaxed);
    }
    
    pub fn get_messages_published(&self) -> u64 {
        self.messages_published.load(Ordering::Relaxed)
    }
    
    pub fn get_messages_consumed(&self) -> u64 {
        self.messages_consumed.load(Ordering::Relaxed)
    }
    
    pub fn get_messages_pending(&self) -> u64 {
        self.messages_pending.load(Ordering::Relaxed)
    }
    
    // Topic statistics methods
    pub fn increment_topics_created(&self) {
        self.topics_created.fetch_add(1, Ordering::Relaxed);
        self.active_topics.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_topics_deleted(&self) {
        self.topics_deleted.fetch_add(1, Ordering::Relaxed);
        self.active_topics.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn get_active_topics(&self) -> usize {
        self.active_topics.load(Ordering::Relaxed)
    }
    
    // Subscriber statistics methods
    pub fn increment_subscribers_connected(&self) {
        self.subscribers_connected.fetch_add(1, Ordering::Relaxed);
        self.active_subscribers.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_subscribers_disconnected(&self) {
        self.subscribers_disconnected.fetch_add(1, Ordering::Relaxed);
        self.active_subscribers.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn get_active_subscribers(&self) -> usize {
        self.active_subscribers.load(Ordering::Relaxed)
    }
    
    // Performance statistics methods
    pub fn add_publish_time(&self, duration: Duration) {
        self.total_publish_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }
    
    pub fn add_consume_time(&self, duration: Duration) {
        self.total_consume_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }
    
    pub fn update_peak_memory_usage(&self, usage: u64) {
        self.peak_memory_usage.fetch_max(usage, Ordering::Relaxed);
    }
    
    pub fn get_average_publish_time_ns(&self) -> Option<u64> {
        let total_time = self.total_publish_time_ns.load(Ordering::Relaxed);
        let total_messages = self.messages_published.load(Ordering::Relaxed);
        
        if total_messages > 0 {
            Some(total_time / total_messages)
        } else {
            None
        }
    }
    
    pub fn get_average_consume_time_ns(&self) -> Option<u64> {
        let total_time = self.total_consume_time_ns.load(Ordering::Relaxed);
        let total_messages = self.messages_consumed.load(Ordering::Relaxed);
        
        if total_messages > 0 {
            Some(total_time / total_messages)
        } else {
            None
        }
    }
    
    // Error statistics methods
    pub fn increment_publish_errors(&self) {
        self.publish_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_consume_errors(&self) {
        self.consume_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_queue_full_errors(&self) {
        self.queue_full_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    // Utility methods
    pub fn reset(&self) {
        self.messages_published.store(0, Ordering::Relaxed);
        self.messages_consumed.store(0, Ordering::Relaxed);
        self.messages_pending.store(0, Ordering::Relaxed);
        self.topics_created.store(0, Ordering::Relaxed);
        self.topics_deleted.store(0, Ordering::Relaxed);
        self.subscribers_connected.store(0, Ordering::Relaxed);
        self.subscribers_disconnected.store(0, Ordering::Relaxed);
        self.total_publish_time_ns.store(0, Ordering::Relaxed);
        self.total_consume_time_ns.store(0, Ordering::Relaxed);
        self.peak_memory_usage.store(0, Ordering::Relaxed);
        self.publish_errors.store(0, Ordering::Relaxed);
        self.consume_errors.store(0, Ordering::Relaxed);
        self.queue_full_errors.store(0, Ordering::Relaxed);
        
        let now_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.last_reset.store(now_nanos, Ordering::Relaxed);
    }
    
    pub fn uptime(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default()
    }
    
    pub fn time_since_reset(&self) -> Duration {
        let last_reset_nanos = self.last_reset.load(Ordering::Relaxed);
        let now_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
            
        Duration::from_nanos(now_nanos.saturating_sub(last_reset_nanos))
    }
}

/// Snapshot of broker statistics at a point in time
#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    pub messages_published: u64,
    pub messages_consumed: u64,
    pub messages_pending: u64,
    pub topics_created: usize,
    pub topics_deleted: usize,
    pub active_topics: usize,
    pub subscribers_connected: usize,
    pub subscribers_disconnected: usize,
    pub active_subscribers: usize,
    pub average_publish_time_ns: Option<u64>,
    pub average_consume_time_ns: Option<u64>,
    pub peak_memory_usage: u64,
    pub publish_errors: u64,
    pub consume_errors: u64,
    pub queue_full_errors: u64,
    pub uptime: Duration,
    pub time_since_reset: Duration,
    pub timestamp: SystemTime,
}

impl From<&BrokerStats> for StatsSnapshot {
    fn from(stats: &BrokerStats) -> Self {
        Self {
            messages_published: stats.get_messages_published(),
            messages_consumed: stats.get_messages_consumed(),
            messages_pending: stats.get_messages_pending(),
            topics_created: stats.topics_created.load(Ordering::Relaxed),
            topics_deleted: stats.topics_deleted.load(Ordering::Relaxed),
            active_topics: stats.get_active_topics(),
            subscribers_connected: stats.subscribers_connected.load(Ordering::Relaxed),
            subscribers_disconnected: stats.subscribers_disconnected.load(Ordering::Relaxed),
            active_subscribers: stats.get_active_subscribers(),
            average_publish_time_ns: stats.get_average_publish_time_ns(),
            average_consume_time_ns: stats.get_average_consume_time_ns(),
            peak_memory_usage: stats.peak_memory_usage.load(Ordering::Relaxed),
            publish_errors: stats.publish_errors.load(Ordering::Relaxed),
            consume_errors: stats.consume_errors.load(Ordering::Relaxed),
            queue_full_errors: stats.queue_full_errors.load(Ordering::Relaxed),
            uptime: stats.uptime(),
            time_since_reset: stats.time_since_reset(),
            timestamp: SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_stats_creation() {
        let stats = BrokerStats::new();
        assert_eq!(stats.get_messages_published(), 0);
        assert_eq!(stats.get_messages_consumed(), 0);
        assert_eq!(stats.get_active_topics(), 0);
        assert_eq!(stats.get_active_subscribers(), 0);
    }
    
    #[test]
    fn test_message_statistics() {
        let stats = BrokerStats::new();
        
        stats.increment_messages_published(5);
        stats.increment_messages_consumed(3);
        stats.set_messages_pending(2);
        
        assert_eq!(stats.get_messages_published(), 5);
        assert_eq!(stats.get_messages_consumed(), 3);
        assert_eq!(stats.get_messages_pending(), 2);
    }
    
    #[test]
    fn test_topic_statistics() {
        let stats = BrokerStats::new();
        
        stats.increment_topics_created();
        stats.increment_topics_created();
        assert_eq!(stats.get_active_topics(), 2);
        
        stats.increment_topics_deleted();
        assert_eq!(stats.get_active_topics(), 1);
    }
    
    #[test]
    fn test_performance_statistics() {
        let stats = BrokerStats::new();
        
        stats.increment_messages_published(2);
        stats.add_publish_time(Duration::from_nanos(1000));
        stats.add_publish_time(Duration::from_nanos(2000));
        
        assert_eq!(stats.get_average_publish_time_ns(), Some(1500));
    }
    
    #[test]
    fn test_stats_reset() {
        let stats = BrokerStats::new();
        
        stats.increment_messages_published(10);
        stats.increment_topics_created();
        
        assert_eq!(stats.get_messages_published(), 10);
        assert_eq!(stats.get_active_topics(), 1);
        
        stats.reset();
        
        assert_eq!(stats.get_messages_published(), 0);
        // Note: active_topics is not reset, only counters are
    }
    
    #[test]
    fn test_stats_snapshot() {
        let stats = BrokerStats::new();
        stats.increment_messages_published(5);
        stats.increment_topics_created();
        
        let snapshot = StatsSnapshot::from(&stats);
        
        assert_eq!(snapshot.messages_published, 5);
        assert_eq!(snapshot.active_topics, 1);
        assert!(snapshot.uptime.as_nanos() > 0);
    }
}
