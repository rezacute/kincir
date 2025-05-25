#![cfg(test)]

use super::*; // This should bring in kincir::Message, Publisher, Subscriber
use crate::mqtt::{MQTTPublisher, MQTTSubscriber}; // Specific MQTT implementations
use std::sync::Arc;
use tokio::time::{timeout, Duration, sleep};
use uuid::Uuid;

const MQTT_BROKER_URL: &str = "mqtt://localhost:1883";
const TEST_TIMEOUT: Duration = Duration::from_secs(10); // Overall timeout for tests
const SHORT_TIMEOUT: Duration = Duration::from_secs(5); // Shorter timeout for single operations

fn generate_unique_topic(prefix: &str) -> String {
    format!("kincir/tests/{}-{}", prefix, Uuid::new_v4())
}

#[tokio::test]
async fn test_mqtt_publisher_subscriber_creation() {
    let topic = generate_unique_topic("creation");
    println!("Testing creation with topic: {}", topic);

    let publisher_result = MQTTPublisher::new(MQTT_BROKER_URL, &topic);
    assert!(publisher_result.is_ok(), "Publisher creation failed: {:?}", publisher_result.err());

    let subscriber_result = MQTTSubscriber::new(MQTT_BROKER_URL, &topic);
    assert!(subscriber_result.is_ok(), "Subscriber creation failed: {:?}", subscriber_result.err());
}

#[tokio::test]
async fn test_creation_with_invalid_broker_url() {
    let topic = generate_unique_topic("invalid-url");
    let invalid_broker_url = "mqtt://invalid-host-that-does-not-exist:1883";
    println!("Testing creation with invalid broker URL: {}", invalid_broker_url);

    let publisher_result = MQTTPublisher::new(invalid_broker_url, &topic);
    assert!(publisher_result.is_err(), "Publisher creation should fail with invalid URL");
    println!("Publisher creation failed as expected: {:?}", publisher_result.err().unwrap());

    let subscriber_result = MQTTSubscriber::new(invalid_broker_url, &topic);
    assert!(subscriber_result.is_err(), "Subscriber creation should fail with invalid URL");
    println!("Subscriber creation failed as expected: {:?}", subscriber_result.err().unwrap());
}

#[tokio::test]
async fn test_publish_single_message_receive() {
    let topic = generate_unique_topic("single-msg");
    println!("Testing single message on topic: {}", topic);

    let publisher = Arc::new(MQTTPublisher::new(MQTT_BROKER_URL, &topic).expect("Publisher creation failed"));
    let mut subscriber = MQTTSubscriber::new(MQTT_BROKER_URL, &topic).expect("Subscriber creation failed");

    // Subscribe
    timeout(SHORT_TIMEOUT, subscriber.subscribe(&topic))
        .await
        .expect("Subscriber subscribe timed out")
        .expect("Subscriber subscribe failed");
    println!("Subscriber subscribed to topic: {}", topic);

    // Spawn subscriber receive loop
    let received_message_payload = Arc::new(tokio::sync::Mutex::new(None::<Vec<u8>>));
    let received_message_payload_clone = received_message_payload.clone();
    
    tokio::spawn(async move {
        match timeout(SHORT_TIMEOUT, subscriber.receive()).await {
            Ok(Ok(msg)) => {
                println!("Subscriber received message with UUID: {}", msg.uuid);
                *received_message_payload_clone.lock().await = Some(msg.payload);
            }
            Ok(Err(e)) => {
                eprintln!("Subscriber receive error: {:?}", e);
            }
            Err(_) => {
                eprintln!("Subscriber receive timed out");
            }
        }
    });

    // Give subscriber time to start polling
    sleep(Duration::from_millis(500)).await;

    // Publish message
    let payload_str = "Hello Kincir MQTT single message!";
    let message = Message::new(payload_str.as_bytes().to_vec());
    println!("Publisher sending message with UUID: {}", message.uuid);
    timeout(SHORT_TIMEOUT, publisher.publish(&topic, vec![message.clone()]))
        .await
        .expect("Publisher publish timed out")
        .expect("Publisher publish failed");
    println!("Publisher message sent.");

    // Wait for message to be received
    sleep(Duration::from_secs(2)).await; // Wait for async operations

    let received = received_message_payload.lock().await;
    assert!(received.is_some(), "Message not received");
    assert_eq!(received.as_ref().unwrap(), &payload_str.as_bytes().to_vec(), "Message payload mismatch");
    println!("Verified single message received successfully.");
}

#[tokio::test]
async fn test_publish_multiple_messages_receive() {
    let topic = generate_unique_topic("multi-msg");
    println!("Testing multiple messages on topic: {}", topic);

    let publisher = Arc::new(MQTTPublisher::new(MQTT_BROKER_URL, &topic).expect("Publisher creation failed"));
    let mut subscriber = MQTTSubscriber::new(MQTT_BROKER_URL, &topic).expect("Subscriber creation failed");

    timeout(SHORT_TIMEOUT, subscriber.subscribe(&topic))
        .await
        .expect("Subscriber subscribe timed out")
        .expect("Subscriber subscribe failed");
    println!("Subscriber subscribed to topic: {}", topic);

    let received_payloads = Arc::new(tokio::sync::Mutex::new(Vec::<Vec<u8>>::new()));
    let received_payloads_clone = received_payloads.clone();

    // Spawn subscriber to collect 3 messages
    tokio::spawn(async move {
        for i in 0..3 {
            match timeout(SHORT_TIMEOUT, subscriber.receive()).await {
                Ok(Ok(msg)) => {
                    println!("Subscriber received message {} with UUID: {}", i + 1, msg.uuid);
                    received_payloads_clone.lock().await.push(msg.payload);
                }
                Ok(Err(e)) => {
                    eprintln!("Subscriber receive error on message {}: {:?}", i + 1, e);
                    break; 
                }
                Err(_) => {
                    eprintln!("Subscriber receive timed out on message {}", i + 1);
                    break;
                }
            }
        }
    });

    sleep(Duration::from_millis(500)).await; // Give subscriber time to start

    let messages_to_send = vec![
        Message::new(b"MQTT Multi-Message 1".to_vec()),
        Message::new(b"MQTT Multi-Message 2".to_vec()),
        Message::new(b"MQTT Multi-Message 3".to_vec()),
    ];

    println!("Publisher sending {} messages...", messages_to_send.len());
    timeout(SHORT_TIMEOUT, publisher.publish(&topic, messages_to_send.clone()))
        .await
        .expect("Publisher publish (multiple) timed out")
        .expect("Publisher publish (multiple) failed");
    println!("Publisher messages sent.");
    
    sleep(Duration::from_secs(3)).await; // Wait for all messages

    let received = received_payloads.lock().await;
    assert_eq!(received.len(), 3, "Incorrect number of messages received");
    for (i, original_msg) in messages_to_send.iter().enumerate() {
        assert_eq!(&received[i], &original_msg.payload, "Message {} payload mismatch", i);
    }
    println!("Verified multiple messages received successfully.");
}

#[tokio::test]
async fn test_different_topics_isolation() {
    let topic1 = generate_unique_topic("isolation-topic1");
    let topic2 = generate_unique_topic("isolation-topic2");
    println!("Testing isolation with Topic 1: {}, Topic 2: {}", topic1, topic2);

    // Publisher and Subscriber for Topic 1
    let publisher1 = Arc::new(MQTTPublisher::new(MQTT_BROKER_URL, &topic1).expect("P1 creation failed"));
    let mut subscriber1 = MQTTSubscriber::new(MQTT_BROKER_URL, &topic1).expect("S1 creation failed");
    timeout(SHORT_TIMEOUT, subscriber1.subscribe(&topic1)).await.expect("S1 sub timed out").expect("S1 sub failed");

    // Publisher and Subscriber for Topic 2
    let publisher2 = Arc::new(MQTTPublisher::new(MQTT_BROKER_URL, &topic2).expect("P2 creation failed"));
    let mut subscriber2 = MQTTSubscriber::new(MQTT_BROKER_URL, &topic2).expect("S2 creation failed");
    timeout(SHORT_TIMEOUT, subscriber2.subscribe(&topic2)).await.expect("S2 sub timed out").expect("S2 sub failed");
    
    println!("Subscribers subscribed to respective topics.");

    // Spawn subscriber for Topic 1
    let s1_received_payload = Arc::new(tokio::sync::Mutex::new(None::<Vec<u8>>));
    let s1_received_payload_clone = s1_received_payload.clone();
    tokio::spawn(async move {
        if let Ok(Ok(msg)) = timeout(SHORT_TIMEOUT, subscriber1.receive()).await {
            println!("S1 received message with UUID: {}", msg.uuid);
            *s1_received_payload_clone.lock().await = Some(msg.payload);
        } else { eprintln!("S1 receive failed or timed out"); }
    });

    // Spawn subscriber for Topic 2
    let s2_received_payload = Arc::new(tokio::sync::Mutex::new(None::<Vec<u8>>));
    let s2_received_payload_clone = s2_received_payload.clone();
    tokio::spawn(async move {
        if let Ok(Ok(msg)) = timeout(SHORT_TIMEOUT, subscriber2.receive()).await {
            println!("S2 received message with UUID: {}", msg.uuid);
            *s2_received_payload_clone.lock().await = Some(msg.payload);
        } else { eprintln!("S2 receive failed or timed out"); }
    });

    sleep(Duration::from_millis(500)).await; // Allow subscribers to start

    // Publish to Topic 1
    let msg1_payload = b"Message for Topic 1".to_vec();
    let message1 = Message::new(msg1_payload.clone());
    println!("P1 sending message on Topic 1 (UUID: {})", message1.uuid);
    timeout(SHORT_TIMEOUT, publisher1.publish(&topic1, vec![message1.clone()])).await.expect("P1 pub timed out").expect("P1 pub failed");

    // Publish to Topic 2
    let msg2_payload = b"Message for Topic 2".to_vec();
    let message2 = Message::new(msg2_payload.clone());
    println!("P2 sending message on Topic 2 (UUID: {})", message2.uuid);
    timeout(SHORT_TIMEOUT, publisher2.publish(&topic2, vec![message2.clone()])).await.expect("P2 pub timed out").expect("P2 pub failed");

    sleep(Duration::from_secs(2)).await; // Wait for messages

    let s1_payload = s1_received_payload.lock().await;
    assert!(s1_payload.is_some(), "S1 did not receive message on Topic 1");
    assert_eq!(s1_payload.as_ref().unwrap(), &msg1_payload, "S1 payload mismatch");

    let s2_payload = s2_received_payload.lock().await;
    assert!(s2_payload.is_some(), "S2 did not receive message on Topic 2");
    assert_eq!(s2_payload.as_ref().unwrap(), &msg2_payload, "S2 payload mismatch");
    
    println!("Verified topic isolation successfully.");
}
