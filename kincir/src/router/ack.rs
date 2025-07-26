//! Acknowledgment-aware router implementation for the Kincir messaging system.
//!
//! This module provides router functionality that integrates with acknowledgment-capable
//! subscribers, enabling reliable message processing with automatic acknowledgment handling.

use crate::ack::{AckHandle, AckSubscriber};
use crate::router::HandlerFunc;
use crate::{Message, Publisher};
use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::timeout;

/// Configuration for acknowledgment behavior in the router
#[derive(Debug, Clone)]
pub struct RouterAckConfig {
    /// Automatic acknowledgment strategy
    pub strategy: AckStrategy,
    /// Timeout for message processing before considering it failed
    pub processing_timeout: Option<Duration>,
    /// Maximum number of retries for failed messages
    pub max_retries: u32,
    /// Whether to requeue messages on processing failure
    pub requeue_on_failure: bool,
    /// Batch size for acknowledgment operations
    pub batch_size: Option<usize>,
}

impl Default for RouterAckConfig {
    fn default() -> Self {
        Self {
            strategy: AckStrategy::AutoAckOnSuccess,
            processing_timeout: Some(Duration::from_secs(30)),
            max_retries: 3,
            requeue_on_failure: true,
            batch_size: None,
        }
    }
}

/// Strategy for automatic acknowledgment handling
#[derive(Debug, Clone, PartialEq)]
pub enum AckStrategy {
    /// Automatically acknowledge on successful processing, nack on failure
    AutoAckOnSuccess,
    /// Manually handle acknowledgments in the handler function
    Manual,
    /// Always acknowledge regardless of processing result
    AlwaysAck,
    /// Never acknowledge (for testing or special cases)
    NeverAck,
}

/// Statistics for acknowledgment operations
#[derive(Debug, Clone, Default)]
pub struct RouterAckStats {
    /// Total messages processed
    pub messages_processed: u64,
    /// Messages successfully acknowledged
    pub messages_acked: u64,
    /// Messages negatively acknowledged
    pub messages_nacked: u64,
    /// Messages that timed out during processing
    pub messages_timed_out: u64,
    /// Messages that exceeded max retries
    pub messages_max_retries_exceeded: u64,
    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,
    /// Total processing time for average calculation
    total_processing_time_ms: u64,
}

impl RouterAckStats {
    /// Update statistics with a new processing time
    pub fn update_processing_time(&mut self, duration: Duration) {
        self.total_processing_time_ms += duration.as_millis() as u64;
        self.avg_processing_time_ms = if self.messages_processed > 0 {
            self.total_processing_time_ms as f64 / self.messages_processed as f64
        } else {
            0.0
        };
    }

    /// Get acknowledgment rate as a percentage
    pub fn ack_rate(&self) -> f64 {
        if self.messages_processed > 0 {
            (self.messages_acked as f64 / self.messages_processed as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get negative acknowledgment rate as a percentage
    pub fn nack_rate(&self) -> f64 {
        if self.messages_processed > 0 {
            (self.messages_nacked as f64 / self.messages_processed as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Acknowledgment-aware router for reliable message processing
#[cfg(feature = "logging")]
pub struct AckRouter<S, H>
where
    S: AckSubscriber + Send + Sync,
    H: AckHandle + Send + Sync,
{
    /// Logger for debugging and monitoring
    logger: Arc<dyn crate::logging::Logger>,
    /// Topic to consume messages from
    consume_topic: String,
    /// Topic to publish processed messages to
    publish_topic: String,
    /// Acknowledgment-capable subscriber
    subscriber: Arc<Mutex<S>>,
    /// Publisher for processed messages
    publisher: Arc<dyn Publisher<Error = Box<dyn Error + Send + Sync>>>,
    /// Message processing handler
    handler: HandlerFunc,
    /// Acknowledgment configuration
    config: RouterAckConfig,
    /// Statistics tracking
    stats: Arc<Mutex<RouterAckStats>>,
    /// Phantom data for handle type
    _phantom: PhantomData<H>,
}

/// Acknowledgment-aware router without logging
#[cfg(not(feature = "logging"))]
pub struct AckRouter<S, H>
where
    S: AckSubscriber + Send + Sync,
    H: AckHandle + Send + Sync,
{
    /// Topic to consume messages from
    consume_topic: String,
    /// Topic to publish processed messages to
    publish_topic: String,
    /// Acknowledgment-capable subscriber
    subscriber: Arc<Mutex<S>>,
    /// Publisher for processed messages
    publisher: Arc<dyn Publisher<Error = Box<dyn Error + Send + Sync>>>,
    /// Message processing handler
    handler: HandlerFunc,
    /// Acknowledgment configuration
    config: RouterAckConfig,
    /// Statistics tracking
    stats: Arc<Mutex<RouterAckStats>>,
    /// Phantom data for handle type
    _phantom: PhantomData<H>,
}

#[cfg(feature = "logging")]
impl<S, H> AckRouter<S, H>
where
    S: AckSubscriber<AckHandle = H> + Send + Sync + 'static,
    H: AckHandle + Send + Sync + 'static,
    S::Error: Into<Box<dyn Error + Send + Sync>>,
{
    /// Create a new acknowledgment-aware router with logging
    pub fn new(
        logger: Arc<dyn crate::logging::Logger>,
        consume_topic: String,
        publish_topic: String,
        subscriber: Arc<Mutex<S>>,
        publisher: Arc<dyn Publisher<Error = Box<dyn Error + Send + Sync>>>,
        handler: HandlerFunc,
        config: RouterAckConfig,
    ) -> Self {
        Self {
            logger,
            consume_topic,
            publish_topic,
            subscriber,
            publisher,
            handler,
            config,
            stats: Arc::new(Mutex::new(RouterAckStats::default())),
            _phantom: PhantomData,
        }
    }

    /// Create a new acknowledgment-aware router with default configuration
    pub fn with_default_config(
        logger: Arc<dyn crate::logging::Logger>,
        consume_topic: String,
        publish_topic: String,
        subscriber: Arc<Mutex<S>>,
        publisher: Arc<dyn Publisher<Error = Box<dyn Error + Send + Sync>>>,
        handler: HandlerFunc,
    ) -> Self {
        Self::new(
            logger,
            consume_topic,
            publish_topic,
            subscriber,
            publisher,
            handler,
            RouterAckConfig::default(),
        )
    }
}

#[cfg(not(feature = "logging"))]
impl<S, H> AckRouter<S, H>
where
    S: AckSubscriber<AckHandle = H> + Send + Sync + 'static,
    H: AckHandle + Send + Sync + 'static,
    S::Error: Into<Box<dyn Error + Send + Sync>>,
{
    /// Create a new acknowledgment-aware router without logging
    pub fn new(
        consume_topic: String,
        publish_topic: String,
        subscriber: Arc<Mutex<S>>,
        publisher: Arc<dyn Publisher<Error = Box<dyn Error + Send + Sync>>>,
        handler: HandlerFunc,
        config: RouterAckConfig,
    ) -> Self {
        Self {
            consume_topic,
            publish_topic,
            subscriber,
            publisher,
            handler,
            config,
            stats: Arc::new(Mutex::new(RouterAckStats::default())),
            _phantom: PhantomData,
        }
    }

    /// Create a new acknowledgment-aware router with default configuration
    pub fn with_default_config(
        consume_topic: String,
        publish_topic: String,
        subscriber: Arc<Mutex<S>>,
        publisher: Arc<dyn Publisher<Error = Box<dyn Error + Send + Sync>>>,
        handler: HandlerFunc,
    ) -> Self {
        Self::new(
            consume_topic,
            publish_topic,
            subscriber,
            publisher,
            handler,
            RouterAckConfig::default(),
        )
    }
}

impl<S, H> AckRouter<S, H>
where
    S: AckSubscriber<AckHandle = H> + Send + Sync + 'static,
    H: AckHandle + Send + Sync + 'static,
    S::Error: Into<Box<dyn Error + Send + Sync>>,
{
    /// Get current acknowledgment statistics
    pub async fn stats(&self) -> RouterAckStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }

    /// Reset acknowledgment statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.lock().await;
        *stats = RouterAckStats::default();
    }

    /// Update acknowledgment configuration
    pub fn with_config(mut self, config: RouterAckConfig) -> Self {
        self.config = config;
        self
    }

    /// Run the acknowledgment-aware router
    pub async fn run(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        #[cfg(feature = "logging")]
        self.logger
            .info("Starting acknowledgment-aware router...")
            .await;

        // Subscribe to the consume topic
        {
            let subscriber = self.subscriber.lock().await;
            subscriber
                .subscribe(&self.consume_topic)
                .await
                .map_err(|e| e.into())?;
        }

        #[cfg(feature = "logging")]
        self.logger
            .info(&format!("Subscribed to topic: {}", self.consume_topic))
            .await;

        // Main processing loop
        loop {
            match self.process_single_message().await {
                Ok(_) => {
                    // Continue processing
                }
                Err(e) => {
                    #[cfg(feature = "logging")]
                    self.logger
                        .error(&format!("Error in message processing loop: {}", e))
                        .await;

                    // Continue processing despite errors
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Process a single message with acknowledgment handling (exposed for testing)
    pub async fn process_single_message(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let start_time = Instant::now();

        // Receive message with acknowledgment handle
        let (message, ack_handle) = {
            let mut subscriber = self.subscriber.lock().await;
            subscriber.receive_with_ack().await.map_err(|e| e.into())?
        };

        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Received message: id={}, topic={}, delivery_count={}",
                ack_handle.message_id(),
                ack_handle.topic(),
                ack_handle.delivery_count()
            ))
            .await;

        // Update statistics
        {
            let mut stats = self.stats.lock().await;
            stats.messages_processed += 1;
        }

        // Process message with timeout if configured
        let processing_result = if let Some(timeout_duration) = self.config.processing_timeout {
            match timeout(timeout_duration, (self.handler)(message.clone())).await {
                Ok(result) => result,
                Err(_) => {
                    #[cfg(feature = "logging")]
                    self.logger
                        .error(&format!(
                            "Message processing timed out after {:?} for message: {}",
                            timeout_duration,
                            ack_handle.message_id()
                        ))
                        .await;

                    // Update timeout statistics
                    {
                        let mut stats = self.stats.lock().await;
                        stats.messages_timed_out += 1;
                    }

                    return self
                        .handle_processing_failure(ack_handle, "Processing timeout")
                        .await;
                }
            }
        } else {
            (self.handler)(message.clone()).await
        };

        // Update processing time statistics
        let processing_time = start_time.elapsed();
        {
            let mut stats = self.stats.lock().await;
            stats.update_processing_time(processing_time);
        }

        // Handle processing result based on strategy
        match processing_result {
            Ok(processed_messages) => {
                self.handle_processing_success(message, ack_handle, processed_messages)
                    .await
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                self.logger
                    .error(&format!(
                        "Message processing failed for message {}: {}",
                        ack_handle.message_id(),
                        e
                    ))
                    .await;

                self.handle_processing_failure(ack_handle, &e.to_string())
                    .await
            }
        }
    }

    /// Handle successful message processing
    async fn handle_processing_success(
        &self,
        _original_message: Message,
        ack_handle: H,
        processed_messages: Vec<Message>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Publish processed messages if any
        if !processed_messages.is_empty() {
            let message_count = processed_messages.len();
            self.publisher
                .publish(&self.publish_topic, processed_messages)
                .await?;

            #[cfg(feature = "logging")]
            self.logger
                .info(&format!(
                    "Published {} processed messages to topic: {}",
                    message_count, self.publish_topic
                ))
                .await;
        }

        // Handle acknowledgment based on strategy
        match self.config.strategy {
            AckStrategy::AutoAckOnSuccess | AckStrategy::AlwaysAck => {
                let message_id = ack_handle.message_id().to_string();
                let subscriber = self.subscriber.lock().await;
                subscriber.ack(ack_handle).await.map_err(|e| e.into())?;

                #[cfg(feature = "logging")]
                self.logger
                    .info(&format!("Message acknowledged: {}", message_id))
                    .await;

                // Update statistics
                {
                    let mut stats = self.stats.lock().await;
                    stats.messages_acked += 1;
                }
            }
            AckStrategy::Manual => {
                #[cfg(feature = "logging")]
                self.logger
                    .info(&format!(
                        "Manual acknowledgment mode - handler should handle ack for message: {}",
                        ack_handle.message_id()
                    ))
                    .await;
            }
            AckStrategy::NeverAck => {
                #[cfg(feature = "logging")]
                self.logger
                    .info(&format!(
                        "Never acknowledge mode - message not acknowledged: {}",
                        ack_handle.message_id()
                    ))
                    .await;
            }
        }

        Ok(())
    }

    /// Handle failed message processing
    async fn handle_processing_failure(
        &self,
        ack_handle: H,
        error_message: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let delivery_count = ack_handle.delivery_count();
        let message_id = ack_handle.message_id().to_string();
        let should_retry = delivery_count <= self.config.max_retries;

        if should_retry && self.config.requeue_on_failure {
            // Negative acknowledge with requeue
            let subscriber = self.subscriber.lock().await;
            subscriber
                .nack(ack_handle, true)
                .await
                .map_err(|e| e.into())?;

            #[cfg(feature = "logging")]
            self.logger
                .info(&format!(
                    "Message negatively acknowledged with requeue (attempt {}): {} - {}",
                    delivery_count, message_id, error_message
                ))
                .await;
        } else {
            // Max retries exceeded or no requeue - discard message
            let subscriber = self.subscriber.lock().await;
            subscriber
                .nack(ack_handle, false)
                .await
                .map_err(|e| e.into())?;

            #[cfg(feature = "logging")]
            if delivery_count > self.config.max_retries {
                self.logger
                    .error(&format!(
                        "Max retries exceeded - message discarded: {} (attempts: {})",
                        message_id, delivery_count
                    ))
                    .await;

                // Update max retries statistics
                {
                    let mut stats = self.stats.lock().await;
                    stats.messages_max_retries_exceeded += 1;
                }
            } else {
                self.logger
                    .info(&format!(
                        "Message negatively acknowledged without requeue: {} - {}",
                        message_id, error_message
                    ))
                    .await;
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.lock().await;
            stats.messages_nacked += 1;
        }

        Ok(())
    }

    /// Run the router with batch processing (if configured)
    pub async fn run_with_batching(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if let Some(batch_size) = self.config.batch_size {
            self.run_batched(batch_size).await
        } else {
            self.run().await
        }
    }

    /// Run the router with batch acknowledgment processing
    async fn run_batched(&self, batch_size: usize) -> Result<(), Box<dyn Error + Send + Sync>> {
        #[cfg(feature = "logging")]
        self.logger
            .info(&format!(
                "Starting acknowledgment-aware router with batch size: {}",
                batch_size
            ))
            .await;

        // Subscribe to the consume topic
        {
            let subscriber = self.subscriber.lock().await;
            subscriber
                .subscribe(&self.consume_topic)
                .await
                .map_err(|e| e.into())?;
        }

        let mut batch_messages = Vec::new();
        let mut batch_handles = Vec::new();

        loop {
            // Collect batch of messages
            for _ in 0..batch_size {
                match self.receive_single_message().await {
                    Ok((message, handle)) => {
                        batch_messages.push(message);
                        batch_handles.push(handle);
                    }
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        self.logger
                            .error(&format!("Error receiving message: {}", e))
                            .await;
                        break;
                    }
                }
            }

            if !batch_messages.is_empty() {
                self.process_message_batch(batch_messages, batch_handles)
                    .await?;
                batch_messages = Vec::new();
                batch_handles = Vec::new();
            }
        }
    }

    /// Receive a single message (helper for batching)
    async fn receive_single_message(&self) -> Result<(Message, H), Box<dyn Error + Send + Sync>> {
        let mut subscriber = self.subscriber.lock().await;
        subscriber.receive_with_ack().await.map_err(|e| e.into())
    }

    /// Process a batch of messages
    async fn process_message_batch(
        &self,
        messages: Vec<Message>,
        handles: Vec<H>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut success_handles = Vec::new();
        let mut failed_handles = Vec::new();
        let mut all_processed_messages = Vec::new();

        // Process each message in the batch
        for (message, handle) in messages.into_iter().zip(handles.into_iter()) {
            match (self.handler)(message).await {
                Ok(mut processed_messages) => {
                    all_processed_messages.append(&mut processed_messages);
                    success_handles.push(handle);
                }
                Err(_) => {
                    failed_handles.push(handle);
                }
            }
        }

        // Publish all processed messages
        if !all_processed_messages.is_empty() {
            self.publisher
                .publish(&self.publish_topic, all_processed_messages)
                .await?;
        }

        // Batch acknowledge successful messages
        if !success_handles.is_empty() {
            let subscriber = self.subscriber.lock().await;
            subscriber
                .ack_batch(success_handles)
                .await
                .map_err(|e| e.into())?;
        }

        // Batch handle failed messages
        if !failed_handles.is_empty() {
            let subscriber = self.subscriber.lock().await;
            subscriber
                .nack_batch(failed_handles, self.config.requeue_on_failure)
                .await
                .map_err(|e| e.into())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ack_config_default() {
        let config = RouterAckConfig::default();
        assert_eq!(config.strategy, AckStrategy::AutoAckOnSuccess);
        assert_eq!(config.processing_timeout, Some(Duration::from_secs(30)));
        assert_eq!(config.max_retries, 3);
        assert!(config.requeue_on_failure);
        assert_eq!(config.batch_size, None);
    }

    #[test]
    fn test_ack_stats_calculations() {
        let mut stats = RouterAckStats::default();

        // Test initial state
        assert_eq!(stats.ack_rate(), 0.0);
        assert_eq!(stats.nack_rate(), 0.0);
        assert_eq!(stats.avg_processing_time_ms, 0.0);

        // Update with some data
        stats.messages_processed = 10;
        stats.messages_acked = 8;
        stats.messages_nacked = 2;
        stats.update_processing_time(Duration::from_millis(100));
        stats.update_processing_time(Duration::from_millis(200));

        assert_eq!(stats.ack_rate(), 80.0);
        assert_eq!(stats.nack_rate(), 20.0);
        assert!(stats.avg_processing_time_ms > 0.0);
    }

    #[test]
    fn test_ack_strategy_variants() {
        let strategies = vec![
            AckStrategy::AutoAckOnSuccess,
            AckStrategy::Manual,
            AckStrategy::AlwaysAck,
            AckStrategy::NeverAck,
        ];

        for strategy in strategies {
            let config = RouterAckConfig {
                strategy: strategy.clone(),
                ..Default::default()
            };
            assert_eq!(config.strategy, strategy);
        }
    }
}
