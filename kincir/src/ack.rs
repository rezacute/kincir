//! Unified acknowledgment handling for message brokers
//! 
//! This module provides a consistent interface for acknowledging and negatively
//! acknowledging messages across different message broker backends.

use std::time::{Duration, SystemTime};
use async_trait::async_trait;

/// Configuration for acknowledgment behavior
#[derive(Debug, Clone)]
pub struct AckConfig {
    /// Acknowledgment mode
    pub mode: AckMode,
    /// Timeout for acknowledgment operations
    pub timeout: Duration,
    /// Maximum number of retries for failed messages
    pub max_retries: u32,
    /// Delay between retries
    pub retry_delay: Duration,
    /// Optional dead letter topic for failed messages
    pub dead_letter_topic: Option<String>,
}

impl Default for AckConfig {
    fn default() -> Self {
        Self {
            mode: AckMode::Manual,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            dead_letter_topic: None,
        }
    }
}

impl AckConfig {
    /// Create a new AckConfig with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the acknowledgment mode
    pub fn with_mode(mut self, mode: AckMode) -> Self {
        self.mode = mode;
        self
    }
    
    /// Set the acknowledgment timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
    
    /// Set the retry delay
    pub fn with_retry_delay(mut self, retry_delay: Duration) -> Self {
        self.retry_delay = retry_delay;
        self
    }
    
    /// Set the dead letter topic
    pub fn with_dead_letter_topic(mut self, topic: Option<String>) -> Self {
        self.dead_letter_topic = topic;
        self
    }
    
    /// Create configuration for automatic acknowledgment
    pub fn auto() -> Self {
        Self::new().with_mode(AckMode::Auto)
    }
    
    /// Create configuration for manual acknowledgment
    pub fn manual() -> Self {
        Self::new().with_mode(AckMode::Manual)
    }
    
    /// Create configuration for client-controlled automatic acknowledgment
    pub fn client_auto() -> Self {
        Self::new().with_mode(AckMode::ClientAuto)
    }
}

/// Acknowledgment mode determines when messages are acknowledged
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AckMode {
    /// Automatic acknowledgment immediately after receive
    Auto,
    /// Manual acknowledgment required by client
    Manual,
    /// Automatic acknowledgment after successful handler execution
    ClientAuto,
}

/// Generic trait for acknowledgment handles
/// 
/// An acknowledgment handle represents a received message that can be
/// acknowledged or negatively acknowledged.
pub trait AckHandle: Send + Sync + Clone {
    /// Get the unique message identifier
    fn message_id(&self) -> &str;
    
    /// Get the topic/queue name
    fn topic(&self) -> &str;
    
    /// Get the message timestamp
    fn timestamp(&self) -> SystemTime;
    
    /// Get the delivery attempt count
    fn delivery_count(&self) -> u32 {
        1
    }
    
    /// Check if this message has been retried
    fn is_retry(&self) -> bool {
        self.delivery_count() > 1
    }
}

/// Enhanced subscriber trait with acknowledgment support
#[async_trait]
pub trait AckSubscriber {
    /// The type of error that can occur during operations
    type Error;
    
    /// The type of acknowledgment handle for this backend
    type AckHandle: AckHandle;

    /// Subscribe to messages from the specified topic
    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error>;
    
    /// Receive a message with its acknowledgment handle
    /// 
    /// Returns a tuple of (Message, AckHandle) that allows the client
    /// to acknowledge or negatively acknowledge the message.
    async fn receive_with_ack(&mut self) -> Result<(crate::Message, Self::AckHandle), Self::Error>;
    
    /// Acknowledge successful processing of a message
    /// 
    /// This tells the broker that the message has been successfully processed
    /// and can be removed from the queue.
    async fn ack(&self, handle: Self::AckHandle) -> Result<(), Self::Error>;
    
    /// Negatively acknowledge a message
    /// 
    /// This tells the broker that the message could not be processed.
    /// The `requeue` parameter determines whether the message should be
    /// redelivered or sent to a dead letter queue.
    async fn nack(&self, handle: Self::AckHandle, requeue: bool) -> Result<(), Self::Error>;
    
    /// Acknowledge multiple messages in a batch
    /// 
    /// This is more efficient than individual ack calls for backends
    /// that support batch operations.
    async fn ack_batch(&self, handles: Vec<Self::AckHandle>) -> Result<(), Self::Error> {
        // Default implementation calls ack individually
        for handle in handles {
            self.ack(handle).await?;
        }
        Ok(())
    }
    
    /// Negatively acknowledge multiple messages in a batch
    async fn nack_batch(&self, handles: Vec<Self::AckHandle>, requeue: bool) -> Result<(), Self::Error> {
        // Default implementation calls nack individually
        for handle in handles {
            self.nack(handle, requeue).await?;
        }
        Ok(())
    }
}

/// Wrapper that provides backward compatibility for existing Subscriber implementations
/// 
/// This allows existing code to continue working while new code can opt into
/// acknowledgment handling.
pub struct CompatSubscriber<S> {
    inner: S,
    config: AckConfig,
}

impl<S> CompatSubscriber<S> {
    /// Create a new compatibility wrapper
    pub fn new(subscriber: S, config: AckConfig) -> Self {
        Self {
            inner: subscriber,
            config,
        }
    }
    
    /// Get the acknowledgment configuration
    pub fn config(&self) -> &AckConfig {
        &self.config
    }
    
    /// Get a reference to the inner subscriber
    pub fn inner(&self) -> &S {
        &self.inner
    }
    
    /// Get a mutable reference to the inner subscriber
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }
    
    /// Consume the wrapper and return the inner subscriber
    pub fn into_inner(self) -> S {
        self.inner
    }
}

#[async_trait]
impl<S> crate::Subscriber for CompatSubscriber<S>
where
    S: crate::Subscriber + Send + Sync,
{
    type Error = S::Error;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        self.inner.subscribe(topic).await
    }

    async fn receive(&mut self) -> Result<crate::Message, Self::Error> {
        // For backward compatibility, we just receive the message
        // without acknowledgment handling
        self.inner.receive().await
    }
}

/// Acknowledgment statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct AckStats {
    /// Total messages acknowledged
    pub acked: u64,
    /// Total messages negatively acknowledged
    pub nacked: u64,
    /// Total messages requeued
    pub requeued: u64,
    /// Total messages sent to dead letter queue
    pub dead_lettered: u64,
    /// Total acknowledgment timeouts
    pub timeouts: u64,
    /// Total acknowledgment errors
    pub errors: u64,
}

impl AckStats {
    /// Create new acknowledgment statistics
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Increment acknowledged message count
    pub fn increment_acked(&mut self) {
        self.acked += 1;
    }
    
    /// Increment negatively acknowledged message count
    pub fn increment_nacked(&mut self) {
        self.nacked += 1;
    }
    
    /// Increment requeued message count
    pub fn increment_requeued(&mut self) {
        self.requeued += 1;
    }
    
    /// Increment dead lettered message count
    pub fn increment_dead_lettered(&mut self) {
        self.dead_lettered += 1;
    }
    
    /// Increment timeout count
    pub fn increment_timeouts(&mut self) {
        self.timeouts += 1;
    }
    
    /// Increment error count
    pub fn increment_errors(&mut self) {
        self.errors += 1;
    }
    
    /// Get total messages processed
    pub fn total_processed(&self) -> u64 {
        self.acked + self.nacked
    }
    
    /// Get acknowledgment success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total_processed();
        if total == 0 {
            0.0
        } else {
            self.acked as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ack_config_builder() {
        let config = AckConfig::new()
            .with_mode(AckMode::Auto)
            .with_timeout(Duration::from_secs(60))
            .with_max_retries(5)
            .with_retry_delay(Duration::from_secs(2))
            .with_dead_letter_topic(Some("dead-letters".to_string()));
        
        assert_eq!(config.mode, AckMode::Auto);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_delay, Duration::from_secs(2));
        assert_eq!(config.dead_letter_topic, Some("dead-letters".to_string()));
    }
    
    #[test]
    fn test_ack_config_presets() {
        let auto_config = AckConfig::auto();
        assert_eq!(auto_config.mode, AckMode::Auto);
        
        let manual_config = AckConfig::manual();
        assert_eq!(manual_config.mode, AckMode::Manual);
        
        let client_auto_config = AckConfig::client_auto();
        assert_eq!(client_auto_config.mode, AckMode::ClientAuto);
    }
    
    #[test]
    fn test_ack_stats() {
        let mut stats = AckStats::new();
        
        stats.increment_acked();
        stats.increment_acked();
        stats.increment_nacked();
        
        assert_eq!(stats.acked, 2);
        assert_eq!(stats.nacked, 1);
        assert_eq!(stats.total_processed(), 3);
        assert!((stats.success_rate() - 0.6666666666666666).abs() < f64::EPSILON);
    }
}
