use kincir::Message;

#[test]
fn test_message_creation() {
    let payload = b"Hello, World!".to_vec();
    let message = Message::new(payload.clone());

    assert!(!message.uuid.is_empty(), "Message UUID should not be empty");
    assert_eq!(message.payload, payload);
}

#[test]
fn test_message_metadata() {
    let payload = b"Hello, World!".to_vec();
    let message = Message::new(payload)
        .with_metadata("content-type", "text/plain")
        .with_metadata("priority", "high");

    assert_eq!(
        message.metadata.get("content-type"),
        Some(&"text/plain".to_string())
    );
    assert_eq!(message.metadata.get("priority"), Some(&"high".to_string()));
    assert!(!message.metadata.contains_key("non-existent"));
}

#[cfg(test)]
mod mqtt_to_rabbitmq_tunnel_tests {
    use kincir::tunnel::{MqttTunnelConfig, RabbitMQTunnelConfig, MqttToRabbitMQTunnel};
    use rumqttc::{AsyncClient as MqttClient, MqttOptions, QoS as MqttQoS};
    use lapin::{Connection, ConnectionProperties, options::*, types::FieldTable, Consumer}; // Removed Channel
    use tokio::time::{timeout, Duration};
    use futures::StreamExt; // Replaced tokio::stream::StreamExt
    // use futures_util::stream::StreamExt as _; // Removed unused import
    // use tokio_amqp::LapinTokioExt; // Removed as with_tokio() will be replaced
    use uuid::Uuid;
    use kincir::Message as KincirMessage;
    // use serde_json; // Removed
    use std::sync::Once;

    static INIT_LOG: Once = Once::new();

    fn setup_logging() {
        INIT_LOG.call_once(|| {
            // Ensure RUST_LOG is set, e.g., "info,kincir=debug,lapin=warn,rumqttc=warn"
            // You might need to set this environment variable when running tests,
            // or use a more sophisticated logger setup if your test runner doesn't inherit it.
            if std::env::var("RUST_LOG").is_err() {
                std::env::set_var("RUST_LOG", "warn,kincir=info"); // Default if not set
            }
            tracing_subscriber::fmt::try_init().ok();
        });
    }


    #[tokio::test]
    #[ignore] // Ignored by default as it requires running MQTT and RabbitMQ instances
    async fn test_mqtt_to_rabbitmq_tunnel_e2e() {
        setup_logging();

        let test_uuid = Uuid::new_v4().to_string();
        let mqtt_topic_name = format!("test/mqtt/{}", test_uuid);
        // Use the queue name as the routing key for direct publishing to the queue via default exchange
        let rabbitmq_queue_name = format!("test-queue-{}", test_uuid);

        tracing::info!(
            "Starting test_mqtt_to_rabbitmq_tunnel_e2e with MQTT topic: {} and RabbitMQ queue: {}",
            mqtt_topic_name,
            rabbitmq_queue_name
        );

        // Configure MQTT
        let mqtt_broker_url = std::env::var("TEST_MQTT_BROKER_URL").unwrap_or_else(|_| "mqtt://localhost:1883".to_string());
        let mqtt_config = MqttTunnelConfig::new(
            &mqtt_broker_url,
            vec![mqtt_topic_name.clone()],
            1 // QoS 1
        );

        // Configure RabbitMQ
        let rmq_uri = std::env::var("TEST_RABBITMQ_URI").unwrap_or_else(|_| "amqp://guest:guest@localhost:5672/%2f".to_string());
        let rmq_config = RabbitMQTunnelConfig::new(
            &rmq_uri,
            &rabbitmq_queue_name // Routing key is the queue name
        );

        // Instantiate Tunnel
        let mut tunnel = MqttToRabbitMQTunnel::new(mqtt_config, rmq_config);

        // Run tunnel in background
        let tunnel_handle = tokio::spawn(async move {
            tracing::info!("Tunnel task started.");
            if let Err(e) = tunnel.run().await {
                tracing::error!("Tunnel exited with error: {:?}", e);
                // Consider failing the test explicitly if the tunnel errors out
                // panic!("Tunnel run failed: {:?}", e); 
            }
            tracing::info!("Tunnel task finished.");
        });
        
        // Give the tunnel a moment to start up and subscribe
        tokio::time::sleep(Duration::from_secs(3)).await; // Increased sleep

        // Setup RabbitMQ consumer
        let rmq_conn = Connection::connect(
            &rmq_uri,
            ConnectionProperties::default(), // Rely on default executor, assuming lapins tokio-global-executor feature is enabled
        )
        .await
        .expect("Failed to connect to RabbitMQ for consumer setup");
        
        let rmq_channel = rmq_conn.create_channel().await.expect("Failed to create RMQ channel for consumer");

        let queue_declare_options = QueueDeclareOptions {
            exclusive: true,
            auto_delete: true,
            ..Default::default()
        };
        let _queue = rmq_channel
            .queue_declare(
                &rabbitmq_queue_name,
                queue_declare_options,
                FieldTable::default(),
            )
            .await
            .expect("Failed to declare RMQ queue");
        tracing::info!("Declared RabbitMQ queue: {}", rabbitmq_queue_name);
        
        // No explicit queue_bind needed due to direct-to-queue publishing strategy

        let mut consumer: Consumer = rmq_channel
            .basic_consume(
                &rabbitmq_queue_name,
                &format!("test_consumer_{}",test_uuid), // Unique consumer tag
                BasicConsumeOptions {
                    no_ack: true, 
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .expect("Failed to start RMQ consumer");

        tracing::info!("RabbitMQ consumer started on queue: {}", rabbitmq_queue_name);

        // Publish MQTT message
        let mqtt_client_id = format!("mqtt-publisher-{}", test_uuid);
        let mut mqtt_options = MqttOptions::new(&mqtt_client_id, mqtt_broker_url.replace("mqtt://", "").split(':').next().unwrap_or("localhost"), mqtt_broker_url.split(':').last().unwrap_or("1883").parse().unwrap_or(1883));
        mqtt_options.set_keep_alive(Duration::from_secs(5));
        let (mqtt_client, mut eventloop) = MqttClient::new(mqtt_options, 10);

        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(notification) => {
                        tracing::debug!("MQTT Client Event: {:?}", notification);
                    }
                    Err(e) => {
                        tracing::error!("MQTT Client event loop error: {:?}", e);
                        break;
                    }
                }
            }
            tracing::info!("MQTT client event loop finished.");
        });
        
        let original_payload_content = format!("Hello from MQTT test {}", test_uuid);
        let kincir_message_payload = original_payload_content.as_bytes().to_vec();

        mqtt_client
            .publish(
                mqtt_topic_name.clone(),
                MqttQoS::AtLeastOnce,
                false, 
                kincir_message_payload.clone(),
            )
            .await
            .expect("Failed to publish MQTT message");
        tracing::info!("Published test message to MQTT topic: {}", mqtt_topic_name);

        // Receive & Assert from RabbitMQ
        tracing::info!("Attempting to receive message from RabbitMQ queue: {}", rabbitmq_queue_name);
        
        let received_delivery_result = timeout(Duration::from_secs(15), consumer.next()).await; // Increased timeout

        match received_delivery_result {
            Ok(Some(Ok(delivery))) => {
                tracing::info!("Received a message from RabbitMQ.");
                let received_payload_bytes = delivery.data;
                match serde_json::from_slice::<KincirMessage>(&received_payload_bytes) {
                    Ok(received_kincir_message) => {
                        tracing::info!("Successfully deserialized KincirMessage from RabbitMQ: {:?}", received_kincir_message);
                        assert_eq!(received_kincir_message.payload, kincir_message_payload, "Payload content mismatch. Expected: {:?}, Got: {:?}", String::from_utf8_lossy(&kincir_message_payload), String::from_utf8_lossy(&received_kincir_message.payload));
                        tracing::info!("Payload content matches!");
                    }
                    Err(e) => {
                        panic!("Failed to deserialize KincirMessage from RabbitMQ payload. Error: {:?}. Payload (UTF-8 lossy): {}", e, String::from_utf8_lossy(&received_payload_bytes));
                    }
                }
            },
            Ok(Some(Err(e))) => panic!("Error receiving message from RabbitMQ: {:?}", e),
            Ok(None) => panic!("RabbitMQ consumer stream ended unexpectedly (queue deleted or channel closed prematurely?)."),
            Err(_) => panic!("Timeout waiting for message from RabbitMQ on queue {}. Ensure services are running and reachable, and tunnel is working.", rabbitmq_queue_name),
        };
        
        // Cleanup
        tunnel_handle.abort();
        match tunnel_handle.await {
            Ok(_) => tracing::info!("Tunnel task completed after abort."),
            Err(e) if e.is_cancelled() => tracing::info!("Tunnel task successfully cancelled."),
            Err(e) => tracing::warn!("Tunnel task panicked or had other error during abort: {:?}", e),
        }
        
        // Disconnect MQTT client
        if let Err(e) = mqtt_client.disconnect().await {
            tracing::warn!("Error disconnecting MQTT client: {:?}", e);
        }
        
        // Close RabbitMQ channel and connection
        // The queue is auto-delete, so no explicit delete needed.
        if let Err(e) = rmq_channel.close(200, "test complete").await {
             tracing::warn!("Error closing RabbitMQ channel: {:?}", e);
        }
        if let Err(e) = rmq_conn.close(200, "test complete").await {
            tracing::warn!("Error closing RabbitMQ connection: {:?}", e);
        }
        tracing::info!("Test finished for MQTT topic: {} and RabbitMQ queue: {}", mqtt_topic_name, rabbitmq_queue_name);
    }
}
