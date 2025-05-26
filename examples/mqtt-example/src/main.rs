use kincir::{Message, Publisher, Subscriber};
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber, MQTTError, QoS}; 
use std::sync::Arc;
use tokio;

#[cfg(feature = "logging")]
use tracing::{info, error, debug, warn, Level};
#[cfg(feature = "logging")]
use tracing_subscriber;

// Custom warn for no_logging to avoid unused import and macro error
#[cfg(not(feature = "logging"))]
macro_rules! warn {
    ($($arg:tt)*) => {
        println!($($arg)*);
    };
}


#[tokio::main]
async fn main() -> Result<(), Box<MQTTError>> {
    // Initialize tracing subscriber for logging
    #[cfg(feature = "logging")]
    tracing_subscriber::fmt().with_max_level(Level::DEBUG).init();

    let broker_url = "mqtt://localhost:1883";
    let topic = "kincir/test/mqtt_example_revised";
    let qos = QoS::AtLeastOnce; // Define QoS for the subscriber

    // --- Create MQTTPublisher ---
    #[cfg(feature = "logging")]
    info!(publisher.broker = broker_url, publisher.topic = topic, "Creating MQTTPublisher");
    let publisher = match MQTTPublisher::new(broker_url, topic) {
        Ok(p) => Arc::new(p), 
        Err(e) => {
            #[cfg(feature = "logging")]
            error!("Failed to create MQTTPublisher: {:?}", e);
            return Err(Box::new(e));
        }
    };
    #[cfg(feature = "logging")]
    info!("MQTTPublisher created successfully.");

    // --- Create MQTTSubscriber ---
    #[cfg(feature = "logging")]
    info!(subscriber.broker = broker_url, subscriber.topic = topic, "Creating MQTTSubscriber");
    let mut subscriber = match MQTTSubscriber::new(broker_url, topic, qos) { // Pass qos
        Ok(s) => s,
        Err(e) => {
            #[cfg(feature = "logging")]
            error!("Failed to create MQTTSubscriber: {:?}", e);
            return Err(Box::new(e));
        }
    };
    #[cfg(feature = "logging")]
    info!("MQTTSubscriber created successfully.");

    // --- MQTTSubscriber subscribes to the topic ---
    #[cfg(feature = "logging")]
    info!("MQTTSubscriber subscribing to topic '{}'...", topic);
    match subscriber.subscribe(topic).await { 
        Ok(_) => {
            #[cfg(feature = "logging")]
            info!("MQTTSubscriber subscribed successfully to topic '{}'", topic);
        }
        Err(e) => {
            #[cfg(feature = "logging")]
            error!("MQTTSubscriber failed to subscribe: {:?}", e);
            return Err(Box::new(MQTTError::Other(e.to_string())));
        }
    }

    // --- Spawn a Tokio task for the subscriber to receive messages ---
    let _subscriber_topic = topic.to_string(); // Prefixed with underscore as it's only used when logging feature is enabled
    tokio::spawn(async move {
        #[cfg(feature = "logging")]
        info!(topic = _subscriber_topic.as_str(), "Subscriber task started. Listening for messages...");
        loop {
            match subscriber.receive().await {
                Ok(message) => {
                    let payload_str = String::from_utf8_lossy(&message.payload);
                    #[cfg(feature = "logging")]
                    info!(
                        topic = _subscriber_topic.as_str(),
                        message.uuid = message.uuid.as_str(),
                        message.payload = payload_str.as_ref(),
                        "Received message"
                    );
                    #[cfg(not(feature = "logging"))]
                    println!(
                        "Subscriber task received message (UUID: {}): Payload: {}",
                        message.uuid, payload_str
                    );
                }
                Err(e) => {
                    #[cfg(feature = "logging")]
                    error!(topic = _subscriber_topic.as_str(), "Subscriber receive error: {:?}", e);
                    warn!("Subscriber loop will attempt to continue after error: {:?}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await; 
                }
            }
        }
    });

    // Allow subscriber task to start and subscribe
    #[cfg(feature = "logging")]
    debug!("Sleeping for 1 second to allow subscriber task to initialize...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // --- Create a kincir::Message ---
    let message_payload_str = "Hello, Kincir MQTT (revised example)!";
    let message_payload_bytes = message_payload_str.as_bytes().to_vec();
    let kincir_message = Message::new(message_payload_bytes);

    #[cfg(feature = "logging")]
    info!(
        publisher.topic = topic,
        message.uuid = kincir_message.uuid.as_str(),
        message.payload = message_payload_str,
        "Publishing message"
    );
    
    // --- MQTTPublisher publishes the message ---
    match publisher.publish(topic, vec![kincir_message.clone()]).await {
        Ok(_) => {
            #[cfg(feature = "logging")]
            info!("Message published successfully!");
        }
        Err(e) => {
            #[cfg(feature = "logging")]
            error!("Failed to publish message: {:?}", e);
            return Err(Box::new(MQTTError::Other(e.to_string())));
        }
    }

    // Give some time for the message to be processed by the subscriber task
    #[cfg(feature = "logging")]
    debug!("Sleeping for 2 seconds to allow message processing...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    #[cfg(feature = "logging")]
    info!("MQTT example finished execution.");
    Ok(())
}
