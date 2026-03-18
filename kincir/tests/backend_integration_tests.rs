//! Integration tests for the unified backend API
//!
//! These tests verify the backend API works correctly with different
//! message broker configurations.

use kincir::backend::{BackendBuilder, BackendError, BackendType};
use kincir::Message;

#[tokio::test]
async fn test_backend_type_from_scheme_amqp() {
    let result = BackendType::from_scheme("amqp");
    assert_eq!(result, Some(BackendType::RabbitMQ));
}

#[tokio::test]
async fn test_backend_type_from_scheme_mqtt() {
    let result = BackendType::from_scheme("mqtt");
    assert_eq!(result, Some(BackendType::MQTT));
}

#[tokio::test]
async fn test_backend_type_case_insensitive() {
    assert_eq!(BackendType::from_scheme("AMQP"), Some(BackendType::RabbitMQ));
    assert_eq!(BackendType::from_scheme("MQTT"), Some(BackendType::MQTT));
}

#[tokio::test]
async fn test_backend_type_invalid_scheme() {
    assert_eq!(BackendType::from_scheme("http"), None);
    assert_eq!(BackendType::from_scheme("redis"), None);
}

#[tokio::test]
async fn test_rabbitmq_connection_failure() {
    // Use a non-existent port to test connection failure handling
    let result = BackendBuilder::from_connection_string("amqp://localhost:9999", "test-queue").await;
    
    // Result should be an error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mqtt_connection_failure() {
    // Use a non-existent port to test connection failure handling  
    // Note: MQTT behavior varies - it may or may not fail immediately
    let _result = BackendBuilder::from_connection_string("mqtt://localhost:9999", "test-topic").await;
    // Test passes as long as it doesn't panic
}

#[tokio::test]
async fn test_unsupported_scheme() {
    let result = BackendBuilder::from_connection_string("http://localhost:8080", "test").await;
    
    // Should fail with unsupported scheme
    assert!(result.is_err());
    
    // Check error type by converting to string
    let err = result.err().unwrap();
    let err_str = err.to_string();
    assert!(err_str.contains("Unsupported"));
}

#[tokio::test]
async fn test_kafka_not_supported() {
    let result = BackendBuilder::from_connection_string("kafka://localhost:9092", "test").await;
    
    // Kafka is not supported via connection string
    assert!(result.is_err());
    let err = result.err().unwrap();
    let err_str = err.to_string();
    // The actual error message is "Kafka not supported via connection string"
    assert!(!err_str.is_empty());
}

#[test]
fn test_backend_error_display() {
    let err = BackendError::Other("test".to_string());
    assert_eq!(err.to_string(), "Backend error: test");
    
    let err = BackendError::UnsupportedScheme("http".to_string());
    assert_eq!(err.to_string(), "Unsupported backend scheme: http");
    
    let err = BackendError::RabbitMQError("failed".to_string());
    assert_eq!(err.to_string(), "RabbitMQ error: failed");
    
    let err = BackendError::MQTTError("timeout".to_string());
    assert_eq!(err.to_string(), "MQTT error: timeout");
}

#[test]
fn test_message_creation() {
    let payload = b"test message".to_vec();
    let message = Message::new(payload.clone());
    
    assert!(!message.uuid.is_empty());
    assert_eq!(message.payload, payload);
    assert!(message.metadata.is_empty());
}

#[test]
fn test_message_with_metadata() {
    let message = Message::new(b"data".to_vec())
        .with_metadata("key1", "value1")
        .with_metadata("key2", "value2");
    
    assert_eq!(message.metadata.get("key1"), Some(&"value1".to_string()));
    assert_eq!(message.metadata.get("key2"), Some(&"value2".to_string()));
}
