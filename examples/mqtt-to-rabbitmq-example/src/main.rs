use dotenvy::dotenv;
use kincir::tunnel::{MqttTunnelConfig, RabbitMQTunnelConfig, MqttToRabbitMQTunnel}; // Ensure MqttToRabbitMQTunnel is imported
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Initialize logging (e.g., using tracing_subscriber)
    let filter = env::var("RUST_LOG").unwrap_or_else(|_| "info,kincir=debug".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    tracing::info!("Starting MQTT to RabbitMQ Tunnel example...");

    // Load MQTT configuration from environment variables
    let mqtt_broker_url =
        env::var("MQTT_BROKER_URL").unwrap_or_else(|_| "mqtt://localhost:1883".to_string());
    let mqtt_topics_str =
        env::var("MQTT_TOPICS").unwrap_or_else(|_| "source/topic,another/source".to_string());
    let mqtt_topics: Vec<String> = mqtt_topics_str
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let mqtt_qos_str = env::var("MQTT_QOS").unwrap_or_else(|_| "1".to_string());
    let mqtt_qos: u8 = mqtt_qos_str.parse().unwrap_or(1);
    
    tracing::info!("MQTT Broker URL: {}", mqtt_broker_url);
    tracing::info!("MQTT Topics: {:?}", mqtt_topics);
    tracing::info!("MQTT QoS: {}", mqtt_qos);

    let mqtt_config = MqttTunnelConfig::new(&mqtt_broker_url, mqtt_topics, mqtt_qos);

    // Load RabbitMQ configuration from environment variables
    let rabbitmq_uri =
        env::var("RABBITMQ_URI").unwrap_or_else(|_| "amqp://localhost:5672".to_string());
    let rabbitmq_routing_key =
        env::var("RABBITMQ_ROUTING_KEY").unwrap_or_else(|_| "destination_queue".to_string());

    tracing::info!("RabbitMQ URI: {}", rabbitmq_uri);
    tracing::info!("RabbitMQ Routing Key: {}", rabbitmq_routing_key);
    
    let rabbitmq_config = RabbitMQTunnelConfig::new(&rabbitmq_uri, &rabbitmq_routing_key);

    // Create and run the tunnel
    let mut tunnel = MqttToRabbitMQTunnel::new(mqtt_config, rabbitmq_config);

    tracing::info!("Running the tunnel...");
    if let Err(e) = tunnel.run().await {
        tracing::error!("Tunnel encountered an error: {}", e);
        return Err(Box::new(e) as Box<dyn std::error::Error>);
    }

    tracing::info!("Tunnel finished or was interrupted.");
    Ok(())
}
