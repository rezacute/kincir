//! Unit tests for the backend module

#[cfg(test)]
mod tests {
    use crate::backend::{BackendBuilder, BackendError, BackendType};

    #[test]
    fn test_parse_amqp_scheme() {
        assert_eq!(BackendType::from_scheme("amqp"), Some(BackendType::RabbitMQ));
    }

    #[test]
    fn test_parse_rabbitmq_scheme() {
        assert_eq!(BackendType::from_scheme("rabbitmq"), Some(BackendType::RabbitMQ));
    }

    #[test]
    fn test_parse_mqtt_scheme() {
        assert_eq!(BackendType::from_scheme("mqtt"), Some(BackendType::MQTT));
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert_eq!(BackendType::from_scheme("AMQP"), Some(BackendType::RabbitMQ));
        assert_eq!(BackendType::from_scheme("MQTT"), Some(BackendType::MQTT));
    }

    #[test]
    fn test_parse_invalid_scheme() {
        assert_eq!(BackendType::from_scheme("http"), None);
        assert_eq!(BackendType::from_scheme("redis"), None);
        assert_eq!(BackendType::from_scheme(""), None);
    }

    #[test]
    fn test_kafka_not_supported_via_scheme() {
        assert_eq!(BackendType::from_scheme("kafka"), None);
    }

    #[test]
    fn test_other_error() {
        let err = BackendError::Other("test error".to_string());
        assert_eq!(err.to_string(), "Backend error: test error");
    }

    #[test]
    fn test_unsupported_scheme_error() {
        let err = BackendError::UnsupportedScheme("http".to_string());
        assert_eq!(err.to_string(), "Unsupported backend scheme: http");
    }

    #[test]
    fn test_rabbitmq_error() {
        let err = BackendError::RabbitMQError("connection failed".to_string());
        assert_eq!(err.to_string(), "RabbitMQ error: connection failed");
    }

    #[test]
    fn test_mqtt_error() {
        let err = BackendError::MQTTError("broker unreachable".to_string());
        assert_eq!(err.to_string(), "MQTT error: broker unreachable");
    }
}
