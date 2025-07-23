use kincir::memory::{InMemoryBroker, InMemoryConfig, InMemoryPublisher, InMemorySubscriber};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting In-Memory Broker Example");
    
    // Create broker with custom configuration
    let config = InMemoryConfig::new()
        .with_max_queue_size(Some(1000))
        .with_max_topics(Some(10))
        .with_stats(true);
    
    let broker = Arc::new(InMemoryBroker::new(config));
    info!("✅ Created in-memory broker with statistics enabled");
    
    // Create publisher and subscriber
    let publisher = InMemoryPublisher::new(broker.clone());
    let mut subscriber = InMemorySubscriber::new(broker.clone());
    
    // Subscribe to a topic
    subscriber.subscribe("orders").await?;
    info!("📡 Subscribed to 'orders' topic");
    
    // Publish some sample orders
    let orders = vec![
        Message::new(b"Order #1001: 2x Coffee, 1x Croissant".to_vec())
            .with_metadata("customer_id", "alice123")
            .with_metadata("priority", "high")
            .with_metadata("total", "15.50"),
        
        Message::new(b"Order #1002: 1x Tea, 2x Muffin".to_vec())
            .with_metadata("customer_id", "bob456")
            .with_metadata("priority", "normal")
            .with_metadata("total", "12.00"),
        
        Message::new(b"Order #1003: 3x Sandwich, 1x Juice".to_vec())
            .with_metadata("customer_id", "charlie789")
            .with_metadata("priority", "low")
            .with_metadata("total", "22.75"),
    ];
    
    info!("📤 Publishing {} orders...", orders.len());
    publisher.publish("orders", orders).await?;
    
    // Process the orders
    info!("📥 Processing orders...");
    for i in 1..=3 {
        let order = subscriber.receive().await?;
        
        let order_text = String::from_utf8_lossy(&order.payload);
        let customer_id = order.metadata.get("customer_id").map(|s| s.as_str()).unwrap_or("unknown");
        let priority = order.metadata.get("priority").map(|s| s.as_str()).unwrap_or("normal");
        let total = order.metadata.get("total").map(|s| s.as_str()).unwrap_or("0.00");
        
        info!("🍽️  Order {}: {}", i, order_text);
        info!("   👤 Customer: {}", customer_id);
        info!("   ⚡ Priority: {}", priority);
        info!("   💰 Total: ${}", total);
        info!("   🆔 Message ID: {}", order.uuid);
        
        // Simulate processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    // Show broker statistics
    if let Some(stats) = publisher.stats() {
        info!("📊 Broker Statistics:");
        info!("   📨 Messages Published: {}", stats.messages_published);
        info!("   📬 Messages Consumed: {}", stats.messages_consumed);
        info!("   📂 Active Topics: {}", stats.active_topics);
        info!("   👥 Active Subscribers: {}", stats.active_subscribers);
        info!("   ⏱️  Uptime: {:?}", stats.uptime);
        
        if let Some(avg_publish_time) = stats.average_publish_time_ns {
            info!("   📤 Avg Publish Time: {}ns", avg_publish_time);
        }
        
        if let Some(avg_consume_time) = stats.average_consume_time_ns {
            info!("   📥 Avg Consume Time: {}ns", avg_consume_time);
        }
    }
    
    // Demonstrate error handling
    info!("🧪 Testing error scenarios...");
    
    // Try to publish to invalid topic
    let invalid_result = publisher.publish("", vec![Message::new(b"test".to_vec())]).await;
    if let Err(e) = invalid_result {
        warn!("❌ Expected error for invalid topic: {}", e);
    }
    
    // Show topic information
    if let Ok(topic_info) = broker.topic_info("orders") {
        info!("📋 Topic 'orders' info:");
        info!("   📦 Queue Size: {}", topic_info.queue_size);
        info!("   👥 Subscribers: {}", topic_info.subscriber_count);
        info!("   📨 Total Published: {}", topic_info.total_published);
        info!("   📬 Total Consumed: {}", topic_info.total_consumed);
        info!("   🕐 Age: {:?}", topic_info.age());
    }
    
    // Demonstrate multiple subscribers
    info!("🔄 Testing multiple subscribers...");
    let mut subscriber2 = InMemorySubscriber::new(broker.clone());
    subscriber2.subscribe("orders").await?;
    
    // Publish one more order
    let final_order = vec![
        Message::new(b"Order #1004: 1x Pizza".to_vec())
            .with_metadata("customer_id", "diana101")
            .with_metadata("priority", "urgent")
            .with_metadata("total", "18.99")
    ];
    
    publisher.publish("orders", final_order).await?;
    
    // Both subscribers should receive it
    let order1 = subscriber.receive().await?;
    let order2 = subscriber2.receive().await?;
    
    info!("📨 Subscriber 1 received: {}", String::from_utf8_lossy(&order1.payload));
    info!("📨 Subscriber 2 received: {}", String::from_utf8_lossy(&order2.payload));
    
    // Clean shutdown
    info!("🛑 Shutting down broker...");
    broker.shutdown()?;
    
    info!("✅ Example completed successfully!");
    
    Ok(())
}
