use std::time::Duration;

/// Configuration for the in-memory message broker
#[derive(Debug, Clone)]
pub struct InMemoryConfig {
    /// Maximum number of messages per topic queue (None = unlimited)
    pub max_queue_size: Option<usize>,
    
    /// Maximum number of topics allowed (None = unlimited)
    pub max_topics: Option<usize>,
    
    /// Whether to enable message persistence during application lifetime
    pub enable_persistence: bool,
    
    /// Whether to maintain message ordering within topics
    pub maintain_order: bool,
    
    /// Default timeout for blocking operations
    pub default_timeout: Duration,
    
    /// Whether to enable broker statistics collection
    pub enable_stats: bool,
    
    /// Maximum number of subscribers per topic (None = unlimited)
    pub max_subscribers_per_topic: Option<usize>,
}

impl Default for InMemoryConfig {
    fn default() -> Self {
        Self {
            max_queue_size: Some(10000), // Default to 10k messages per topic
            max_topics: Some(1000),      // Default to 1k topics
            enable_persistence: true,
            maintain_order: true,
            default_timeout: Duration::from_secs(30),
            enable_stats: false,
            max_subscribers_per_topic: Some(100), // Default to 100 subscribers per topic
        }
    }
}

impl InMemoryConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set maximum queue size per topic
    pub fn with_max_queue_size(mut self, size: Option<usize>) -> Self {
        self.max_queue_size = size;
        self
    }
    
    /// Set maximum number of topics
    pub fn with_max_topics(mut self, max: Option<usize>) -> Self {
        self.max_topics = max;
        self
    }
    
    /// Enable or disable message persistence
    pub fn with_persistence(mut self, enabled: bool) -> Self {
        self.enable_persistence = enabled;
        self
    }
    
    /// Enable or disable message ordering
    pub fn with_ordering(mut self, enabled: bool) -> Self {
        self.maintain_order = enabled;
        self
    }
    
    /// Set default timeout for operations
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }
    
    /// Enable or disable statistics collection
    pub fn with_stats(mut self, enabled: bool) -> Self {
        self.enable_stats = enabled;
        self
    }
    
    /// Set maximum subscribers per topic
    pub fn with_max_subscribers_per_topic(mut self, max: Option<usize>) -> Self {
        self.max_subscribers_per_topic = max;
        self
    }
    
    /// Create configuration optimized for testing
    pub fn for_testing() -> Self {
        Self {
            max_queue_size: Some(100),
            max_topics: Some(10),
            enable_persistence: true,
            maintain_order: true,
            default_timeout: Duration::from_millis(100),
            enable_stats: true,
            max_subscribers_per_topic: Some(5),
        }
    }
    
    /// Create configuration optimized for high performance
    pub fn for_high_performance() -> Self {
        Self {
            max_queue_size: None, // Unlimited
            max_topics: None,     // Unlimited
            enable_persistence: true,
            maintain_order: false, // Disable ordering for better performance
            default_timeout: Duration::from_secs(1),
            enable_stats: false,   // Disable stats for better performance
            max_subscribers_per_topic: None, // Unlimited
        }
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if let Some(queue_size) = self.max_queue_size {
            if queue_size == 0 {
                return Err("max_queue_size cannot be zero".to_string());
            }
        }
        
        if let Some(max_topics) = self.max_topics {
            if max_topics == 0 {
                return Err("max_topics cannot be zero".to_string());
            }
        }
        
        if let Some(max_subs) = self.max_subscribers_per_topic {
            if max_subs == 0 {
                return Err("max_subscribers_per_topic cannot be zero".to_string());
            }
        }
        
        if self.default_timeout.is_zero() {
            return Err("default_timeout cannot be zero".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = InMemoryConfig::default();
        assert_eq!(config.max_queue_size, Some(10000));
        assert_eq!(config.max_topics, Some(1000));
        assert!(config.enable_persistence);
        assert!(config.maintain_order);
        assert!(!config.enable_stats);
    }
    
    #[test]
    fn test_config_builder() {
        let config = InMemoryConfig::new()
            .with_max_queue_size(Some(5000))
            .with_max_topics(Some(500))
            .with_persistence(false)
            .with_stats(true);
            
        assert_eq!(config.max_queue_size, Some(5000));
        assert_eq!(config.max_topics, Some(500));
        assert!(!config.enable_persistence);
        assert!(config.enable_stats);
    }
    
    #[test]
    fn test_testing_config() {
        let config = InMemoryConfig::for_testing();
        assert_eq!(config.max_queue_size, Some(100));
        assert_eq!(config.max_topics, Some(10));
        assert!(config.enable_stats);
        assert_eq!(config.default_timeout, Duration::from_millis(100));
    }
    
    #[test]
    fn test_high_performance_config() {
        let config = InMemoryConfig::for_high_performance();
        assert_eq!(config.max_queue_size, None);
        assert_eq!(config.max_topics, None);
        assert!(!config.maintain_order);
        assert!(!config.enable_stats);
    }
    
    #[test]
    fn test_config_validation() {
        // Valid config
        let config = InMemoryConfig::default();
        assert!(config.validate().is_ok());
        
        // Invalid queue size
        let config = InMemoryConfig::default().with_max_queue_size(Some(0));
        assert!(config.validate().is_err());
        
        // Invalid max topics
        let config = InMemoryConfig::default().with_max_topics(Some(0));
        assert!(config.validate().is_err());
        
        // Invalid timeout
        let config = InMemoryConfig::default().with_timeout(Duration::ZERO);
        assert!(config.validate().is_err());
    }
}
