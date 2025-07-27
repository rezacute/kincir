---
layout: docs
title: Event Sourcing
description: Implementing event sourcing patterns with Kincir's message streaming capabilities
---

# Event Sourcing with Kincir

This guide demonstrates how to implement event sourcing patterns using Kincir's message streaming capabilities for building resilient, auditable systems.

## Table of Contents

- [Event Sourcing Basics](#event-sourcing-basics)
- [Event Store Implementation](#event-store-implementation)
- [Aggregate Pattern](#aggregate-pattern)
- [Event Replay](#event-replay)
- [Snapshots](#snapshots)

## Event Sourcing Basics

### Domain Events

```rust
use kincir::{Publisher, Subscriber, Message};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DomainEvent {
    pub event_id: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub event_data: serde_json::Value,
    pub version: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DomainEvent {
    pub fn new(aggregate_id: String, event_type: String, event_data: serde_json::Value, version: u64) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            aggregate_id,
            event_type,
            event_data,
            version,
            timestamp: chrono::Utc::now(),
        }
    }
}

// Order domain events
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderCreated {
    pub order_id: String,
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderItem {
    pub product_id: String,
    pub quantity: u32,
    pub price: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderShipped {
    pub order_id: String,
    pub tracking_number: String,
    pub carrier: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderCancelled {
    pub order_id: String,
    pub reason: String,
}
```

## Event Store Implementation

### Event Store with Kincir

```rust
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct EventStore {
    publisher: InMemoryPublisher,
    subscriber: InMemorySubscriber,
    events: Arc<RwLock<HashMap<String, Vec<DomainEvent>>>>,
}

impl EventStore {
    pub fn new(broker: Arc<InMemoryBroker>) -> Self {
        Self {
            publisher: InMemoryPublisher::new(broker.clone()),
            subscriber: InMemorySubscriber::new(broker),
            events: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn append_events(
        &self,
        aggregate_id: &str,
        events: Vec<DomainEvent>,
        expected_version: Option<u64>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Optimistic concurrency control
        {
            let mut store = self.events.write().await;
            let current_events = store.entry(aggregate_id.to_string()).or_insert_with(Vec::new);
            
            if let Some(expected) = expected_version {
                let current_version = current_events.len() as u64;
                if current_version != expected {
                    return Err(format!("Concurrency conflict: expected version {}, got {}", 
                                     expected, current_version).into());
                }
            }
            
            current_events.extend(events.clone());
        }
        
        // Publish events to message broker
        for event in events {
            let message = Message::new(serde_json::to_vec(&event)?)
                .with_metadata("event_type", &event.event_type)
                .with_metadata("aggregate_id", &event.aggregate_id)
                .with_metadata("version", &event.version.to_string());
            
            self.publisher.publish("domain_events", vec![message]).await?;
        }
        
        Ok(())
    }
    
    pub async fn get_events(
        &self,
        aggregate_id: &str,
        from_version: Option<u64>,
    ) -> Result<Vec<DomainEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let store = self.events.read().await;
        let events = store.get(aggregate_id).cloned().unwrap_or_default();
        
        let filtered_events = if let Some(from) = from_version {
            events.into_iter().filter(|e| e.version >= from).collect()
        } else {
            events
        };
        
        Ok(filtered_events)
    }
    
    pub async fn get_all_events_after(
        &self,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<DomainEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let store = self.events.read().await;
        let mut all_events = Vec::new();
        
        for events in store.values() {
            for event in events {
                if event.timestamp > timestamp {
                    all_events.push(event.clone());
                }
            }
        }
        
        all_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(all_events)
    }
}
```

## Aggregate Pattern

### Order Aggregate

```rust
#[derive(Debug, Clone)]
pub struct Order {
    pub id: String,
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub status: OrderStatus,
    pub total_amount: f64,
    pub version: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Created,
    Paid,
    Shipped,
    Delivered,
    Cancelled,
}

impl Order {
    pub fn new(id: String, customer_id: String, items: Vec<OrderItem>) -> Self {
        let total_amount = items.iter().map(|item| item.price * item.quantity as f64).sum();
        let now = chrono::Utc::now();
        
        Self {
            id,
            customer_id,
            items,
            status: OrderStatus::Created,
            total_amount,
            version: 0,
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn create_order(
        id: String,
        customer_id: String,
        items: Vec<OrderItem>,
    ) -> Result<(Self, Vec<DomainEvent>), Box<dyn std::error::Error + Send + Sync>> {
        if items.is_empty() {
            return Err("Order must have at least one item".into());
        }
        
        let order = Self::new(id.clone(), customer_id.clone(), items.clone());
        
        let event_data = serde_json::to_value(OrderCreated {
            order_id: id.clone(),
            customer_id,
            items,
            total_amount: order.total_amount,
        })?;
        
        let event = DomainEvent::new(id, "OrderCreated".to_string(), event_data, 1);
        
        Ok((order, vec![event]))
    }
    
    pub fn ship_order(
        &self,
        tracking_number: String,
        carrier: String,
    ) -> Result<Vec<DomainEvent>, Box<dyn std::error::Error + Send + Sync>> {
        if self.status != OrderStatus::Paid {
            return Err("Order must be paid before shipping".into());
        }
        
        let event_data = serde_json::to_value(OrderShipped {
            order_id: self.id.clone(),
            tracking_number,
            carrier,
        })?;
        
        let event = DomainEvent::new(
            self.id.clone(),
            "OrderShipped".to_string(),
            event_data,
            self.version + 1,
        );
        
        Ok(vec![event])
    }
    
    pub fn cancel_order(
        &self,
        reason: String,
    ) -> Result<Vec<DomainEvent>, Box<dyn std::error::Error + Send + Sync>> {
        if self.status == OrderStatus::Shipped || self.status == OrderStatus::Delivered {
            return Err("Cannot cancel shipped or delivered order".into());
        }
        
        let event_data = serde_json::to_value(OrderCancelled {
            order_id: self.id.clone(),
            reason,
        })?;
        
        let event = DomainEvent::new(
            self.id.clone(),
            "OrderCancelled".to_string(),
            event_data,
            self.version + 1,
        );
        
        Ok(vec![event])
    }
    
    pub fn apply_event(&mut self, event: &DomainEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event.event_type.as_str() {
            "OrderCreated" => {
                let data: OrderCreated = serde_json::from_value(event.event_data.clone())?;
                self.customer_id = data.customer_id;
                self.items = data.items;
                self.total_amount = data.total_amount;
                self.status = OrderStatus::Created;
            }
            "OrderShipped" => {
                self.status = OrderStatus::Shipped;
            }
            "OrderCancelled" => {
                self.status = OrderStatus::Cancelled;
            }
            _ => return Err(format!("Unknown event type: {}", event.event_type).into()),
        }
        
        self.version = event.version;
        self.updated_at = event.timestamp;
        Ok(())
    }
    
    pub fn from_events(events: Vec<DomainEvent>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if events.is_empty() {
            return Err("Cannot create aggregate from empty event stream".into());
        }
        
        let first_event = &events[0];
        if first_event.event_type != "OrderCreated" {
            return Err("First event must be OrderCreated".into());
        }
        
        let data: OrderCreated = serde_json::from_value(first_event.event_data.clone())?;
        let mut order = Self::new(data.order_id, data.customer_id, data.items);
        
        for event in events {
            order.apply_event(&event)?;
        }
        
        Ok(order)
    }
}
```

## Event Replay

### Event Replay System

```rust
pub struct EventReplayService {
    event_store: Arc<EventStore>,
    publisher: InMemoryPublisher,
}

impl EventReplayService {
    pub fn new(event_store: Arc<EventStore>, broker: Arc<InMemoryBroker>) -> Self {
        Self {
            event_store,
            publisher: InMemoryPublisher::new(broker),
        }
    }
    
    pub async fn replay_events_from(
        &self,
        from_timestamp: chrono::DateTime<chrono::Utc>,
        target_topic: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let events = self.event_store.get_all_events_after(from_timestamp).await?;
        let event_count = events.len() as u64;
        
        println!("Replaying {} events from {}", event_count, from_timestamp);
        
        for event in events {
            let message = Message::new(serde_json::to_vec(&event)?)
                .with_metadata("event_type", &event.event_type)
                .with_metadata("aggregate_id", &event.aggregate_id)
                .with_metadata("version", &event.version.to_string())
                .with_metadata("replayed", "true");
            
            self.publisher.publish(target_topic, vec![message]).await?;
        }
        
        println!("Replay completed: {} events replayed", event_count);
        Ok(event_count)
    }
    
    pub async fn rebuild_aggregate(
        &self,
        aggregate_id: &str,
    ) -> Result<Order, Box<dyn std::error::Error + Send + Sync>> {
        let events = self.event_store.get_events(aggregate_id, None).await?;
        Order::from_events(events)
    }
}
```

## Snapshots

### Snapshot System

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AggregateSnapshot {
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub version: u64,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct SnapshotStore {
    snapshots: Arc<RwLock<HashMap<String, AggregateSnapshot>>>,
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn save_snapshot(
        &self,
        aggregate_id: &str,
        aggregate: &Order,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let snapshot = AggregateSnapshot {
            aggregate_id: aggregate_id.to_string(),
            aggregate_type: "Order".to_string(),
            version: aggregate.version,
            data: serde_json::to_value(aggregate)?,
            timestamp: chrono::Utc::now(),
        };
        
        let mut store = self.snapshots.write().await;
        store.insert(aggregate_id.to_string(), snapshot);
        
        Ok(())
    }
    
    pub async fn get_snapshot(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<AggregateSnapshot>, Box<dyn std::error::Error + Send + Sync>> {
        let store = self.snapshots.read().await;
        Ok(store.get(aggregate_id).cloned())
    }
}

// Enhanced aggregate loading with snapshots
impl EventStore {
    pub async fn load_aggregate_with_snapshot(
        &self,
        aggregate_id: &str,
        snapshot_store: &SnapshotStore,
    ) -> Result<Order, Box<dyn std::error::Error + Send + Sync>> {
        // Try to load from snapshot first
        if let Some(snapshot) = snapshot_store.get_snapshot(aggregate_id).await? {
            let mut order: Order = serde_json::from_value(snapshot.data)?;
            
            // Load events after snapshot
            let events = self.get_events(aggregate_id, Some(snapshot.version + 1)).await?;
            
            // Apply events after snapshot
            for event in events {
                order.apply_event(&event)?;
            }
            
            return Ok(order);
        }
        
        // No snapshot, load from all events
        let events = self.get_events(aggregate_id, None).await?;
        Order::from_events(events)
    }
}
```

## Complete Example

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    let event_store = Arc::new(EventStore::new(broker.clone()));
    let snapshot_store = SnapshotStore::new();
    let replay_service = EventReplayService::new(event_store.clone(), broker);
    
    // Create an order
    let items = vec![
        OrderItem {
            product_id: "PROD-001".to_string(),
            quantity: 2,
            price: 29.99,
        },
        OrderItem {
            product_id: "PROD-002".to_string(),
            quantity: 1,
            price: 49.99,
        },
    ];
    
    let (order, events) = Order::create_order(
        "ORDER-001".to_string(),
        "CUSTOMER-123".to_string(),
        items,
    )?;
    
    // Store events
    event_store.append_events("ORDER-001", events, Some(0)).await?;
    
    // Ship the order
    let ship_events = order.ship_order(
        "TRACK-123456".to_string(),
        "UPS".to_string(),
    )?;
    
    event_store.append_events("ORDER-001", ship_events, Some(1)).await?;
    
    // Create snapshot
    let rebuilt_order = event_store.load_aggregate_with_snapshot("ORDER-001", &snapshot_store).await?;
    snapshot_store.save_snapshot("ORDER-001", &rebuilt_order).await?;
    
    // Replay events from a specific time
    let replay_from = chrono::Utc::now() - chrono::Duration::hours(1);
    let replayed_count = replay_service.replay_events_from(replay_from, "replayed_events").await?;
    
    println!("Order status: {:?}", rebuilt_order.status);
    println!("Replayed {} events", replayed_count);
    
    Ok(())
}
```

This event sourcing implementation provides a solid foundation for building resilient, auditable systems with Kincir's message streaming capabilities.
