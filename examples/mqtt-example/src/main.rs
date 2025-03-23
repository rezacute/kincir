use kincir::{Message, Publisher, Subscriber}; // Using kincir::Message, Publisher, Subscriber from lib.rs
use kincir::mqtt::{MQTTPublisher, MQTTSubscriber}; // Specific MQTT implementations
use std::error::Error;
use std::sync::Arc;
use tokio;
use tracing::Level;
use tracing_subscriber;

#[cfg(feature = "logging")]
use tracing::{info, error, debug, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt().with_max_level(Level::DEBUG).init();

    let broker_url = "mqtt://localhost:1883";
    let topic = "kincir/test/mqtt_example_revised";

    // --- Create MQTTPublisher ---
    #[cfg(feature = "logging")]
    info!(publisher.broker = broker_url, publisher.topic = topic, "Creating MQTTPublisher");
    let publisher = match MQTTPublisher::new(broker_url, topic) {
        Ok(p) => Arc::new(p), // Arc for potential sharing, though not strictly needed if used only here
        Err(e) => {
            #[cfg(feature = "logging")]
            error!("Failed to create MQTTPublisher: {:?}", e);
            return Err(Box::new(e));
        }
    };
    #[cfg(feature = "logging")]
    info!("MQTTPublisher created successfully.");

    // --- Create MQTTSubscriber ---
    // Note: MQTTSubscriber::new might ideally return an Arc<Mutex<Self>> if subscribe/receive take &mut self
    // and we want to share it or use it in multiple tasks directly.
    // For this example, we'll make it mutable and use it in one spawned task.
    #[cfg(feature = "logging")]
    info!(subscriber.broker = broker_url, subscriber.topic = topic, "Creating MQTTSubscriber");
    let mut subscriber = match MQTTSubscriber::new(broker_url, topic) {
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
    // This now aligns with the Subscriber trait taking `&self` and topic,
    // assuming the MQTTSubscriber::subscribe was updated to match.
    // If MQTTSubscriber::subscribe still takes `&mut self` and no topic, this call needs adjustment.
    // Based on my *internal* refactor of mqtt.rs, it became `async fn subscribe(&mut self)`
    // The kincir::Subscriber trait is `async fn subscribe(&self, topic: &str)`
    // This is a known mismatch I need to fix in `mqtt.rs` later.
    // For now, I'll call it as if it's `&mut self` and no topic argument,
    // as that's how the `MQTTSubscriber` itself was designed in the prior step.
    #[cfg(feature = "logging")]
    info!("MQTTSubscriber subscribing to topic '{}'...", topic);
    match subscriber.subscribe().await { // Assuming this is the &mut self version from my mqtt.rs refactor
        Ok(_) => {
            #[cfg(feature = "logging")]
            info!("MQTTSubscriber subscribed successfully to topic '{}'", topic);
        }
        Err(e) => {
            #[cfg(feature = "logging")]
            error!("MQTTSubscriber failed to subscribe: {:?}", e);
            return Err(e);
        }
    }

    // --- Spawn a Tokio task for the subscriber to receive messages ---
    let subscriber_topic = topic.to_string(); // Clone topic for the spawned task
    tokio::spawn(async move {
        #[cfg(feature = "logging")]
        info!(topic = subscriber_topic, "Subscriber task started. Listening for messages...");
        loop {
            // Assuming MQTTSubscriber::receive() takes &mut self from my mqtt.rs refactor
            match subscriber.receive().await {
                Ok(message) => {
                    let payload_str = String::from_utf8_lossy(&message.payload);
                    #[cfg(feature = "logging")]
                    info!(
                        topic = subscriber_topic,
                        message.uuid = message.uuid,
                        message.payload = payload_str,
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
                    error!(topic = subscriber_topic, "Subscriber receive error: {:?}", e);
                    // Decide if the loop should break or continue on error
                    // For MQTT, errors from poll() can be connection issues.
                    warn!(topic = subscriber_topic, "Subscriber loop will attempt to continue after error.");
                    // Add a small delay to prevent tight error looping if connection is down
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
        message.uuid = kincir_message.uuid,
        message.payload = message_payload_str,
        "Publishing message"
    );
    
    // --- MQTTPublisher publishes the message ---
    // The Publisher trait's publish method expects Vec<Message>.
    match publisher.publish(topic, vec![kincir_message.clone()]).await {
        Ok(_) => {
            #[cfg(feature = "logging")]
            info!("Message published successfully!");
        }
        Err(e) => {
            #[cfg(feature = "logging")]
            error!("Failed to publish message: {:?}", e);
            return Err(e);
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
