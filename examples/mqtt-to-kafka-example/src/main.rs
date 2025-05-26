use dotenvy::dotenv;
use kincir::tunnel::{KafkaTunnelConfig, MqttToKafkaTunnel, MqttTunnelConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let filter = env::var("RUST_LOG").unwrap_or_else(|_| "info,kincir=debug".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    tracing::info!("Starting MQTT to Kafka Tunnel example...");

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

    let kafka_broker_urls_str =
        env::var("KAFKA_BROKER_URLS").unwrap_or_else(|_| "localhost:9092".to_string());
    let kafka_broker_urls: Vec<String> = kafka_broker_urls_str
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let kafka_topic = env::var("KAFKA_TOPIC").unwrap_or_else(|_| "destination_topic".to_string());

    tracing::info!("Kafka Broker URLs: {:?}", kafka_broker_urls);
    tracing::info!("Kafka Topic: {}", kafka_topic);

    let kafka_config = KafkaTunnelConfig::new(kafka_broker_urls, &kafka_topic);

    let mut tunnel = MqttToKafkaTunnel::new(mqtt_config, kafka_config);

    tracing::info!("Running the tunnel...");
    if let Err(e) = tunnel.run().await {
        tracing::error!("Tunnel encountered an error: {}", e);
        return Err(Box::new(e) as Box<dyn std::error::Error>);
    }

    tracing::info!("Tunnel finished or was interrupted.");
    Ok(())
}
