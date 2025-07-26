use thiserror::Error;

/// Errors that can occur during in-memory broker operations
#[derive(Debug, Error, Clone)]
pub enum InMemoryError {
    #[error("Topic not found: {topic}")]
    TopicNotFound { topic: String },

    #[error("Queue full for topic: {topic}")]
    QueueFull { topic: String },

    #[error("Not subscribed to any topic")]
    NotSubscribed,

    #[error("Broker shutdown")]
    BrokerShutdown,

    #[error("Maximum number of topics reached: {max}")]
    MaxTopicsReached { max: usize },

    #[error("Invalid topic name: {topic}")]
    InvalidTopicName { topic: String },

    #[error("Subscriber already exists for topic: {topic}")]
    SubscriberAlreadyExists { topic: String },

    #[error("Channel send error: {message}")]
    ChannelSendError { message: String },

    #[error("Channel receive error: {message}")]
    ChannelReceiveError { message: String },
}

impl InMemoryError {
    pub fn topic_not_found(topic: impl Into<String>) -> Self {
        Self::TopicNotFound {
            topic: topic.into(),
        }
    }

    pub fn queue_full(topic: impl Into<String>) -> Self {
        Self::QueueFull {
            topic: topic.into(),
        }
    }

    pub fn max_topics_reached(max: usize) -> Self {
        Self::MaxTopicsReached { max }
    }

    pub fn invalid_topic_name(topic: impl Into<String>) -> Self {
        Self::InvalidTopicName {
            topic: topic.into(),
        }
    }

    pub fn subscriber_already_exists(topic: impl Into<String>) -> Self {
        Self::SubscriberAlreadyExists {
            topic: topic.into(),
        }
    }

    pub fn channel_send_error(message: impl Into<String>) -> Self {
        Self::ChannelSendError {
            message: message.into(),
        }
    }

    pub fn channel_receive_error(message: impl Into<String>) -> Self {
        Self::ChannelReceiveError {
            message: message.into(),
        }
    }
}

// Implement Send + Sync for error handling in async contexts
unsafe impl Send for InMemoryError {}
unsafe impl Sync for InMemoryError {}
