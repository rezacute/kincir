---
layout: docs
title: MQTT Integration
description: IoT messaging with MQTT and Quality of Service handling
---

# MQTT Integration Example

MQTT (Message Queuing Telemetry Transport) is a lightweight messaging protocol designed for IoT devices and low-bandwidth, high-latency networks. Kincir provides full MQTT support with Quality of Service (QoS) handling.

## Prerequisites

Before running these examples, ensure you have an MQTT broker running:

```bash
# Using Mosquitto with Docker
docker run -it -p 1883:1883 -p 9001:9001 eclipse-mosquitto

# Or install locally (Ubuntu/Debian)
sudo apt-get install mosquitto mosquitto-clients
sudo systemctl start mosquitto
```

## Basic Usage

### Simple Publisher-Subscriber

```rust
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber};
use kincir::{Publisher, Subscriber, Message};
use rumqttc::QoS;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create MQTT publisher and subscriber
    let publisher = MQTTPublisher::new("mqtt://localhost:1883", "sensor-publisher");
    let mut subscriber = MQTTSubscriber::new("mqtt://localhost:1883", "sensor-subscriber");

    // Subscribe to sensor topics
    subscriber.subscribe("sensors/temperature").await?;
    subscriber.subscribe("sensors/humidity").await?;
    
    // Publish sensor data
    let temp_message = Message::new(b"23.5".to_vec())
        .with_metadata("sensor_id", "temp_001")
        .with_metadata("unit", "celsius")
        .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339());
    
    let humidity_message = Message::new(b"65.2".to_vec())
        .with_metadata("sensor_id", "hum_001")
        .with_metadata("unit", "percent")
        .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339());
    
    // Publish with different QoS levels
    publisher.publish_with_qos("sensors/temperature", vec![temp_message], QoS::AtLeastOnce).await?;
    publisher.publish_with_qos("sensors/humidity", vec![humidity_message], QoS::ExactlyOnce).await?;
    
    // Receive messages
    for _ in 0..2 {
        let message = subscriber.receive().await?;
        println!("Received sensor data: {:?}", String::from_utf8_lossy(&message.payload));
        println!("Sensor ID: {:?}", message.get_metadata("sensor_id"));
    }
    
    Ok(())
}
```

## Quality of Service (QoS) Levels

```rust
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber};
use kincir::{Publisher, Subscriber, Message};
use rumqttc::QoS;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let publisher = MQTTPublisher::new("mqtt://localhost:1883", "qos-publisher");
    let mut subscriber = MQTTSubscriber::new("mqtt://localhost:1883", "qos-subscriber");
    
    // Subscribe to different topics with different QoS levels
    subscriber.subscribe_with_qos("alerts/critical", QoS::ExactlyOnce).await?;
    subscriber.subscribe_with_qos("alerts/warning", QoS::AtLeastOnce).await?;
    subscriber.subscribe_with_qos("alerts/info", QoS::AtMostOnce).await?;
    
    // Publish messages with appropriate QoS levels
    
    // Critical alerts - must be delivered exactly once
    let critical_alert = Message::new(b"CRITICAL: System temperature exceeded 80°C".to_vec())
        .with_metadata("alert_level", "critical")
        .with_metadata("system_id", "server_001");
    publisher.publish_with_qos("alerts/critical", vec![critical_alert], QoS::ExactlyOnce).await?;
    
    // Warning alerts - should be delivered at least once
    let warning_alert = Message::new(b"WARNING: High memory usage detected".to_vec())
        .with_metadata("alert_level", "warning")
        .with_metadata("memory_usage", "85%");
    publisher.publish_with_qos("alerts/warning", vec![warning_alert], QoS::AtLeastOnce).await?;
    
    // Info alerts - fire and forget
    let info_alert = Message::new(b"INFO: System backup completed".to_vec())
        .with_metadata("alert_level", "info")
        .with_metadata("backup_size", "2.3GB");
    publisher.publish_with_qos("alerts/info", vec![info_alert], QoS::AtMostOnce).await?;
    
    // Process alerts
    for _ in 0..3 {
        let message = subscriber.receive().await?;
        let alert_level = message.get_metadata("alert_level").unwrap_or("unknown");
        println!("Received {} alert: {}", alert_level, String::from_utf8_lossy(&message.payload));
    }
    
    Ok(())
}
```

## IoT Device Simulation

```rust
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber};
use kincir::{Publisher, Subscriber, Message};
use rumqttc::QoS;
use serde_json::json;
use std::sync::Arc;
use tokio::task;
use rand::Rng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Simulate multiple IoT devices
    let device_count = 5;
    let mut device_handles = vec![];
    
    for device_id in 0..device_count {
        let handle = task::spawn(async move {
            simulate_iot_device(device_id).await.unwrap();
        });
        device_handles.push(handle);
    }
    
    // Central monitoring system
    let monitor_handle = task::spawn(async {
        run_monitoring_system().await.unwrap();
    });
    
    // Let devices run for 30 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    
    // Stop devices
    for handle in device_handles {
        handle.abort();
    }
    monitor_handle.abort();
    
    Ok(())
}

async fn simulate_iot_device(device_id: u32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let publisher = MQTTPublisher::new("mqtt://localhost:1883", &format!("device_{}", device_id));
    let mut rng = rand::thread_rng();
    
    println!("Starting IoT device {}", device_id);
    
    loop {
        // Generate sensor readings
        let temperature = 20.0 + rng.gen::<f64>() * 15.0; // 20-35°C
        let humidity = 40.0 + rng.gen::<f64>() * 40.0;    // 40-80%
        let battery = 100.0 - rng.gen::<f64>() * 50.0;    // 50-100%
        
        // Create sensor data payload
        let sensor_data = json!({
            "device_id": device_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "temperature": temperature,
            "humidity": humidity,
            "battery_level": battery,
            "location": {
                "lat": 37.7749 + rng.gen::<f64>() * 0.01,
                "lon": -122.4194 + rng.gen::<f64>() * 0.01
            }
        });
        
        let message = Message::new(sensor_data.to_string().into_bytes())
            .with_metadata("device_id", &device_id.to_string())
            .with_metadata("data_type", "sensor_reading");
        
        // Publish to device-specific topic
        let topic = format!("devices/{}/sensors", device_id);
        publisher.publish_with_qos(&topic, vec![message], QoS::AtLeastOnce).await?;
        
        // Check for alerts
        if temperature > 30.0 {
            let alert = Message::new(format!("High temperature alert: {:.1}°C", temperature).into_bytes())
                .with_metadata("device_id", &device_id.to_string())
                .with_metadata("alert_type", "temperature")
                .with_metadata("severity", "warning");
            
            publisher.publish_with_qos("alerts/temperature", vec![alert], QoS::ExactlyOnce).await?;
        }
        
        if battery < 20.0 {
            let alert = Message::new(format!("Low battery alert: {:.1}%", battery).into_bytes())
                .with_metadata("device_id", &device_id.to_string())
                .with_metadata("alert_type", "battery")
                .with_metadata("severity", "critical");
            
            publisher.publish_with_qos("alerts/battery", vec![alert], QoS::ExactlyOnce).await?;
        }
        
        // Send data every 5 seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn run_monitoring_system() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut subscriber = MQTTSubscriber::new("mqtt://localhost:1883", "monitoring-system");
    
    // Subscribe to all device data and alerts
    subscriber.subscribe("devices/+/sensors").await?;
    subscriber.subscribe("alerts/+").await?;
    
    println!("Monitoring system started");
    
    loop {
        match subscriber.receive().await {
            Ok(message) => {
                let topic = message.get_metadata("topic").unwrap_or("unknown");
                
                if topic.contains("sensors") {
                    // Process sensor data
                    if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&message.payload) {
                        println!("Device {}: Temp={:.1}°C, Humidity={:.1}%, Battery={:.1}%",
                                data["device_id"], data["temperature"], data["humidity"], data["battery_level"]);
                    }
                } else if topic.contains("alerts") {
                    // Process alerts
                    let alert_msg = String::from_utf8_lossy(&message.payload);
                    let device_id = message.get_metadata("device_id").unwrap_or("unknown");
                    let severity = message.get_metadata("severity").unwrap_or("info");
                    
                    println!("🚨 ALERT [{}] Device {}: {}", severity.to_uppercase(), device_id, alert_msg);
                }
            }
            Err(e) => {
                eprintln!("Monitoring system error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}
```

## Retained Messages and Last Will

```rust
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber, MQTTConfig};
use kincir::{Publisher, Subscriber, Message};
use rumqttc::{QoS, LastWill};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure MQTT with Last Will Testament
    let last_will = LastWill::new(
        "devices/status/device_001",
        "offline",
        QoS::AtLeastOnce,
        true, // retain
    );
    
    let config = MQTTConfig {
        broker_url: "mqtt://localhost:1883".to_string(),
        client_id: "device_001".to_string(),
        keep_alive: 60,
        clean_session: false,
        last_will: Some(last_will),
        username: None,
        password: None,
    };
    
    let publisher = MQTTPublisher::with_config(config);
    let mut subscriber = MQTTSubscriber::new("mqtt://localhost:1883", "status-monitor");
    
    // Subscribe to device status
    subscriber.subscribe("devices/status/+").await?;
    
    // Publish device online status (retained)
    let online_message = Message::new(b"online".to_vec())
        .with_metadata("device_id", "device_001")
        .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339());
    
    publisher.publish_retained("devices/status/device_001", vec![online_message]).await?;
    
    // Publish device configuration (retained)
    let config_message = Message::new(br#"{"sampling_rate": 5, "alert_threshold": 30}"#.to_vec())
        .with_metadata("device_id", "device_001")
        .with_metadata("config_version", "1.0");
    
    publisher.publish_retained("devices/config/device_001", vec![config_message]).await?;
    
    // Monitor status messages
    println!("Monitoring device status...");
    for _ in 0..5 {
        let message = subscriber.receive().await?;
        let topic = message.get_metadata("topic").unwrap_or("unknown");
        let status = String::from_utf8_lossy(&message.payload);
        println!("Status update on {}: {}", topic, status);
    }
    
    // When this program exits, the Last Will message will be published automatically
    println!("Device going offline...");
    
    Ok(())
}
```

## MQTT to RabbitMQ Bridge

```rust
use kincir::mqtt::{MQTTSubscriber};
use kincir::rabbitmq::{RabbitMQPublisher};
use kincir::{Subscriber, Publisher, Message};
use rumqttc::QoS;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // MQTT subscriber for IoT data
    let mut mqtt_subscriber = MQTTSubscriber::new("mqtt://localhost:1883", "mqtt-bridge");
    
    // RabbitMQ publisher for backend processing
    let rabbitmq_publisher = RabbitMQPublisher::new("amqp://localhost:5672");
    
    // Subscribe to all IoT device topics
    mqtt_subscriber.subscribe("devices/+/+").await?;
    mqtt_subscriber.subscribe("alerts/+").await?;
    
    println!("MQTT to RabbitMQ bridge started");
    
    loop {
        match mqtt_subscriber.receive().await {
            Ok(mqtt_message) => {
                // Transform MQTT message for RabbitMQ
                let topic = mqtt_message.get_metadata("topic").unwrap_or("unknown");
                
                // Create RabbitMQ message with additional metadata
                let rabbitmq_message = Message::new(mqtt_message.payload.clone())
                    .with_metadata("source", "mqtt")
                    .with_metadata("original_topic", topic)
                    .with_metadata("bridge_timestamp", &chrono::Utc::now().to_rfc3339())
                    .with_metadata("message_id", &mqtt_message.uuid);
                
                // Copy original metadata
                for (key, value) in &mqtt_message.metadata {
                    if !key.starts_with("bridge_") {
                        rabbitmq_message.set_metadata(key, value);
                    }
                }
                
                // Route to appropriate RabbitMQ exchange based on topic
                let exchange = if topic.contains("sensors") {
                    "iot.sensors"
                } else if topic.contains("alerts") {
                    "iot.alerts"
                } else {
                    "iot.general"
                };
                
                // Publish to RabbitMQ
                match rabbitmq_publisher.publish(exchange, vec![rabbitmq_message]).await {
                    Ok(()) => {
                        println!("Bridged message from {} to {}", topic, exchange);
                    }
                    Err(e) => {
                        eprintln!("Failed to bridge message: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("MQTT bridge error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}
```

## Secure MQTT with TLS

```rust
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber, MQTTTlsConfig};
use kincir::{Publisher, Subscriber, Message};
use rumqttc::QoS;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure TLS connection
    let tls_config = MQTTTlsConfig {
        ca_cert_path: Some("/path/to/ca.crt".to_string()),
        client_cert_path: Some("/path/to/client.crt".to_string()),
        client_key_path: Some("/path/to/client.key".to_string()),
        server_name: "mqtt.example.com".to_string(),
        verify_server: true,
    };
    
    // Create secure MQTT connections
    let publisher = MQTTPublisher::with_tls("mqtts://mqtt.example.com:8883", "secure-publisher", tls_config.clone());
    let mut subscriber = MQTTSubscriber::with_tls("mqtts://mqtt.example.com:8883", "secure-subscriber", tls_config);
    
    // Subscribe to secure topics
    subscriber.subscribe("secure/data").await?;
    
    // Publish encrypted sensor data
    let encrypted_data = encrypt_sensor_data(b"sensitive sensor reading").await?;
    let message = Message::new(encrypted_data)
        .with_metadata("encryption", "aes256")
        .with_metadata("sensor_id", "secure_001");
    
    publisher.publish_with_qos("secure/data", vec![message], QoS::ExactlyOnce).await?;
    
    // Receive and decrypt
    let received = subscriber.receive().await?;
    let decrypted_data = decrypt_sensor_data(&received.payload).await?;
    println!("Decrypted data: {:?}", String::from_utf8_lossy(&decrypted_data));
    
    Ok(())
}

async fn encrypt_sensor_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Implement your encryption logic here
    // This is a placeholder
    Ok(data.to_vec())
}

async fn decrypt_sensor_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Implement your decryption logic here
    // This is a placeholder
    Ok(data.to_vec())
}
```

## MQTT Performance Testing

```rust
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber};
use kincir::{Publisher, Subscriber, Message};
use rumqttc::QoS;
use std::time::Instant;
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message_count = 10_000;
    let payload_size = 256; // bytes
    let concurrent_publishers = 5;
    
    println!("MQTT Performance Test");
    println!("Messages: {}, Payload size: {} bytes, Publishers: {}", 
             message_count, payload_size, concurrent_publishers);
    
    // Start subscriber
    let mut subscriber = MQTTSubscriber::new("mqtt://localhost:1883", "perf-test-subscriber");
    subscriber.subscribe("perf/test").await?;
    
    let received_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let received_count_clone = received_count.clone();
    
    // Start receiving task
    let receive_handle = task::spawn(async move {
        let start = Instant::now();
        
        while received_count_clone.load(std::sync::atomic::Ordering::Relaxed) < message_count {
            match subscriber.receive().await {
                Ok(_) => {
                    let count = received_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    if count % 1000 == 0 {
                        println!("Received {} messages", count);
                    }
                }
                Err(e) => {
                    eprintln!("Receive error: {}", e);
                    break;
                }
            }
        }
        
        let duration = start.elapsed();
        println!("Receiving completed in {:?}", duration);
        println!("Receiving rate: {:.2} messages/second", 
                 message_count as f64 / duration.as_secs_f64());
    });
    
    // Start publishers
    let messages_per_publisher = message_count / concurrent_publishers;
    let payload = vec![0u8; payload_size];
    
    let start = Instant::now();
    let mut publisher_handles = vec![];
    
    for publisher_id in 0..concurrent_publishers {
        let payload_clone = payload.clone();
        let handle = task::spawn(async move {
            let publisher = MQTTPublisher::new("mqtt://localhost:1883", &format!("perf-publisher-{}", publisher_id));
            
            for i in 0..messages_per_publisher {
                let message = Message::new(payload_clone.clone())
                    .with_metadata("publisher_id", &publisher_id.to_string())
                    .with_metadata("sequence", &i.to_string());
                
                publisher.publish_with_qos("perf/test", vec![message], QoS::AtMostOnce).await.unwrap();
            }
            
            println!("Publisher {} completed", publisher_id);
        });
        publisher_handles.push(handle);
    }
    
    // Wait for all publishers to complete
    for handle in publisher_handles {
        handle.await?;
    }
    
    let publish_duration = start.elapsed();
    println!("Publishing completed in {:?}", publish_duration);
    println!("Publishing rate: {:.2} messages/second", 
             message_count as f64 / publish_duration.as_secs_f64());
    
    // Wait for receiving to complete
    receive_handle.await?;
    
    Ok(())
}
```

## Error Handling and Reconnection

```rust
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber, MQTTError};
use kincir::{Publisher, Subscriber, Message};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut retry_count = 0;
    let max_retries = 10;
    
    loop {
        match run_mqtt_client().await {
            Ok(()) => {
                println!("MQTT client completed successfully");
                break;
            }
            Err(MQTTError::ConnectionFailed) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    eprintln!("Max retries reached. MQTT broker not available.");
                    return Err("MQTT broker not available".into());
                }
                
                println!("MQTT connection failed (attempt {}). Retrying in 5 seconds...", retry_count);
                sleep(Duration::from_secs(5)).await;
            }
            Err(MQTTError::SubscriptionFailed(topic)) => {
                println!("Failed to subscribe to '{}'. Retrying...", topic);
                sleep(Duration::from_secs(2)).await;
            }
            Err(e) => {
                eprintln!("Non-recoverable MQTT error: {}", e);
                return Err(e.into());
            }
        }
    }
    
    Ok(())
}

async fn run_mqtt_client() -> Result<(), MQTTError> {
    let publisher = MQTTPublisher::new("mqtt://localhost:1883", "resilient-client");
    let mut subscriber = MQTTSubscriber::new("mqtt://localhost:1883", "resilient-subscriber");
    
    subscriber.subscribe("test/resilience").await?;
    
    let message = Message::new(b"Resilient MQTT message".to_vec());
    publisher.publish("test/resilience", vec![message]).await?;
    
    let received = subscriber.receive().await?;
    println!("Received: {:?}", String::from_utf8_lossy(&received.payload));
    
    Ok(())
}
```

## Next Steps

- [Message Acknowledgments](/examples/acknowledgments.html) - Reliable processing
- [IoT Data Pipeline](/examples/iot-pipeline.html) - Complete IoT solution
- [Microservices Communication](/examples/microservices.html) - Service integration
- [Performance Optimization](/examples/performance.html) - High-throughput tuning

## Resources

- [MQTT Specification](https://mqtt.org/mqtt-specification/)
- [Eclipse Mosquitto](https://mosquitto.org/)
- [Kincir API Documentation](https://docs.rs/kincir)
- [Getting Started Guide](/docs/getting-started.html)
