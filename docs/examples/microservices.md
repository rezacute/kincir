---
layout: docs
title: Microservices Communication
description: Using Kincir for inter-service communication in microservices architecture
---

# Microservices Communication with Kincir

This guide demonstrates how to use Kincir for inter-service communication in a microservices architecture.

## Table of Contents

- [Service Architecture](#service-architecture)
- [Event-Driven Communication](#event-driven-communication)
- [Request-Response Pattern](#request-response-pattern)
- [Service Discovery](#service-discovery)
- [Load Balancing](#load-balancing)

## Service Architecture

### Order Processing System

```rust
use kincir::{Publisher, Subscriber, Message};
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
pub struct OrderEvent {
    pub order_id: String,
    pub customer_id: String,
    pub amount: f64,
    pub status: OrderStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OrderStatus {
    Created,
    PaymentProcessed,
    Shipped,
    Delivered,
    Cancelled,
}

// Order Service
pub struct OrderService {
    publisher: InMemoryPublisher,
    subscriber: InMemorySubscriber,
}

impl OrderService {
    pub fn new(broker: Arc<InMemoryBroker>) -> Self {
        Self {
            publisher: InMemoryPublisher::new(broker.clone()),
            subscriber: InMemorySubscriber::new(broker),
        }
    }
    
    pub async fn create_order(&self, order: OrderEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Order Service: Creating order {}", order.order_id);
        
        let message = Message::new(serde_json::to_vec(&order)?)
            .with_metadata("event_type", "order_created")
            .with_metadata("service", "order_service");
        
        self.publisher.publish("order_events", vec![message]).await?;
        Ok(())
    }
    
    pub async fn listen_for_events(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.subscriber.subscribe("payment_events").await?;
        self.subscriber.subscribe("shipping_events").await?;
        
        loop {
            let message = self.subscriber.receive().await?;
            self.handle_event(message).await?;
        }
    }
    
    async fn handle_event(&self, message: Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event_type = message.metadata.get("event_type").unwrap_or(&"unknown".to_string());
        
        match event_type.as_str() {
            "payment_processed" => {
                let order_event: OrderEvent = serde_json::from_slice(&message.payload)?;
                println!("Order Service: Payment processed for order {}", order_event.order_id);
                
                // Update order status and publish event
                let updated_order = OrderEvent {
                    status: OrderStatus::PaymentProcessed,
                    ..order_event
                };
                
                let response_message = Message::new(serde_json::to_vec(&updated_order)?)
                    .with_metadata("event_type", "order_updated")
                    .with_metadata("service", "order_service");
                
                self.publisher.publish("order_events", vec![response_message]).await?;
            }
            "shipment_created" => {
                let order_event: OrderEvent = serde_json::from_slice(&message.payload)?;
                println!("Order Service: Shipment created for order {}", order_event.order_id);
            }
            _ => {
                println!("Order Service: Unknown event type: {}", event_type);
            }
        }
        
        Ok(())
    }
}

// Payment Service
pub struct PaymentService {
    publisher: InMemoryPublisher,
    subscriber: InMemorySubscriber,
}

impl PaymentService {
    pub fn new(broker: Arc<InMemoryBroker>) -> Self {
        Self {
            publisher: InMemoryPublisher::new(broker.clone()),
            subscriber: InMemorySubscriber::new(broker),
        }
    }
    
    pub async fn listen_for_orders(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.subscriber.subscribe("order_events").await?;
        
        loop {
            let message = self.subscriber.receive().await?;
            if let Some(event_type) = message.metadata.get("event_type") {
                if event_type == "order_created" {
                    self.process_payment(message).await?;
                }
            }
        }
    }
    
    async fn process_payment(&self, message: Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let order: OrderEvent = serde_json::from_slice(&message.payload)?;
        println!("Payment Service: Processing payment for order {} (${:.2})", 
                 order.order_id, order.amount);
        
        // Simulate payment processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let payment_event = OrderEvent {
            status: OrderStatus::PaymentProcessed,
            ..order
        };
        
        let response_message = Message::new(serde_json::to_vec(&payment_event)?)
            .with_metadata("event_type", "payment_processed")
            .with_metadata("service", "payment_service");
        
        self.publisher.publish("payment_events", vec![response_message]).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(InMemoryBroker::with_default_config());
    
    // Start services
    let order_service = Arc::new(OrderService::new(broker.clone()));
    let payment_service = Arc::new(PaymentService::new(broker.clone()));
    
    // Start event listeners
    let mut order_listener = OrderService::new(broker.clone());
    let mut payment_listener = PaymentService::new(broker.clone());
    
    tokio::spawn(async move {
        if let Err(e) = order_listener.listen_for_events().await {
            eprintln!("Order listener error: {}", e);
        }
    });
    
    tokio::spawn(async move {
        if let Err(e) = payment_listener.listen_for_orders().await {
            eprintln!("Payment listener error: {}", e);
        }
    });
    
    // Simulate order creation
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    let order = OrderEvent {
        order_id: "ORD-001".to_string(),
        customer_id: "CUST-123".to_string(),
        amount: 99.99,
        status: OrderStatus::Created,
    };
    
    order_service.create_order(order).await?;
    
    // Keep the application running
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    
    Ok(())
}
```

## Event-Driven Communication

### Saga Pattern Implementation

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SagaStep {
    pub step_id: String,
    pub service: String,
    pub action: String,
    pub compensation_action: String,
    pub status: StepStatus,
    pub data: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StepStatus {
    Pending,
    Completed,
    Failed,
    Compensated,
}

pub struct SagaOrchestrator {
    publisher: InMemoryPublisher,
    subscriber: InMemorySubscriber,
    active_sagas: HashMap<String, Vec<SagaStep>>,
}

impl SagaOrchestrator {
    pub fn new(broker: Arc<InMemoryBroker>) -> Self {
        Self {
            publisher: InMemoryPublisher::new(broker.clone()),
            subscriber: InMemorySubscriber::new(broker),
            active_sagas: HashMap::new(),
        }
    }
    
    pub async fn start_saga(&mut self, saga_id: String, steps: Vec<SagaStep>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Starting saga: {}", saga_id);
        self.active_sagas.insert(saga_id.clone(), steps.clone());
        
        // Execute first step
        if let Some(first_step) = steps.first() {
            self.execute_step(&saga_id, first_step).await?;
        }
        
        Ok(())
    }
    
    async fn execute_step(&self, saga_id: &str, step: &SagaStep) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Executing step {} for saga {}", step.step_id, saga_id);
        
        let message = Message::new(serde_json::to_vec(step)?)
            .with_metadata("saga_id", saga_id)
            .with_metadata("step_id", &step.step_id)
            .with_metadata("action", &step.action);
        
        let topic = format!("{}_commands", step.service);
        self.publisher.publish(&topic, vec![message]).await?;
        
        Ok(())
    }
}
```

## Request-Response Pattern

```rust
use tokio::sync::oneshot;
use std::collections::HashMap;
use uuid::Uuid;

pub struct RequestResponseService {
    publisher: InMemoryPublisher,
    subscriber: InMemorySubscriber,
    pending_requests: Arc<tokio::sync::Mutex<HashMap<String, oneshot::Sender<Message>>>>,
}

impl RequestResponseService {
    pub fn new(broker: Arc<InMemoryBroker>, service_name: &str) -> Self {
        let mut subscriber = InMemorySubscriber::new(broker.clone());
        
        Self {
            publisher: InMemoryPublisher::new(broker),
            subscriber,
            pending_requests: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn request(&self, target_service: &str, request_data: Vec<u8>) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
        let request_id = Uuid::new_v4().to_string();
        let (sender, receiver) = oneshot::channel();
        
        // Store the pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), sender);
        }
        
        // Send the request
        let request_message = Message::new(request_data)
            .with_metadata("request_id", &request_id)
            .with_metadata("reply_to", "response_topic");
        
        let topic = format!("{}_requests", target_service);
        self.publisher.publish(&topic, vec![request_message]).await?;
        
        // Wait for response
        let response = receiver.await?;
        Ok(response)
    }
    
    pub async fn listen_for_responses(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.subscriber.subscribe("response_topic").await?;
        
        loop {
            let message = self.subscriber.receive().await?;
            
            if let Some(request_id) = message.metadata.get("request_id") {
                let mut pending = self.pending_requests.lock().await;
                if let Some(sender) = pending.remove(request_id) {
                    let _ = sender.send(message);
                }
            }
        }
    }
}
```

This microservices guide demonstrates the core patterns for building distributed systems with Kincir.
