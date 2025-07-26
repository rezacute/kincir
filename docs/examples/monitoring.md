# Monitoring and Observability with Kincir

This guide demonstrates how to add comprehensive monitoring, metrics, and distributed tracing to Kincir applications.

## Table of Contents

- [Metrics Collection](#metrics-collection)
- [Health Checks](#health-checks)
- [Distributed Tracing](#distributed-tracing)
- [Logging](#logging)
- [Alerting](#alerting)

## Metrics Collection

### Prometheus Metrics

```rust
use prometheus::{Counter, Histogram, Gauge, Registry, Encoder, TextEncoder};
use kincir::{Publisher, Subscriber, Message};
use std::sync::Arc;
use std::time::Instant;

pub struct MetricsCollector {
    messages_published: Counter,
    messages_received: Counter,
    message_processing_duration: Histogram,
    active_connections: Gauge,
    error_count: Counter,
    registry: Registry,
}

impl MetricsCollector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let registry = Registry::new();
        
        let messages_published = Counter::new(
            "kincir_messages_published_total",
            "Total number of messages published"
        )?;
        
        let messages_received = Counter::new(
            "kincir_messages_received_total", 
            "Total number of messages received"
        )?;
        
        let message_processing_duration = Histogram::new(
            "kincir_message_processing_duration_seconds",
            "Time spent processing messages"
        )?;
        
        let active_connections = Gauge::new(
            "kincir_active_connections",
            "Number of active connections"
        )?;
        
        let error_count = Counter::new(
            "kincir_errors_total",
            "Total number of errors"
        )?;
        
        registry.register(Box::new(messages_published.clone()))?;
        registry.register(Box::new(messages_received.clone()))?;
        registry.register(Box::new(message_processing_duration.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(error_count.clone()))?;
        
        Ok(Self {
            messages_published,
            messages_received,
            message_processing_duration,
            active_connections,
            error_count,
            registry,
        })
    }
    
    pub fn record_message_published(&self) {
        self.messages_published.inc();
    }
    
    pub fn record_message_received(&self) {
        self.messages_received.inc();
    }
    
    pub fn record_processing_time(&self, duration: std::time::Duration) {
        self.message_processing_duration.observe(duration.as_secs_f64());
    }
    
    pub fn set_active_connections(&self, count: i64) {
        self.active_connections.set(count as f64);
    }
    
    pub fn record_error(&self) {
        self.error_count.inc();
    }
    
    pub fn export_metrics(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

// Instrumented Publisher
pub struct InstrumentedPublisher<P> {
    publisher: P,
    metrics: Arc<MetricsCollector>,
}

impl<P> InstrumentedPublisher<P>
where
    P: Publisher,
{
    pub fn new(publisher: P, metrics: Arc<MetricsCollector>) -> Self {
        Self { publisher, metrics }
    }
}

#[async_trait::async_trait]
impl<P> Publisher for InstrumentedPublisher<P>
where
    P: Publisher + Send + Sync,
{
    type Error = P::Error;
    
    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        let start = Instant::now();
        let message_count = messages.len();
        
        match self.publisher.publish(topic, messages).await {
            Ok(()) => {
                for _ in 0..message_count {
                    self.metrics.record_message_published();
                }
                self.metrics.record_processing_time(start.elapsed());
                Ok(())
            }
            Err(e) => {
                self.metrics.record_error();
                Err(e)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let metrics = Arc::new(MetricsCollector::new()?);
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let base_publisher = kincir::memory::InMemoryPublisher::new(broker);
    let instrumented_publisher = InstrumentedPublisher::new(base_publisher, metrics.clone());
    
    // Publish some messages
    for i in 0..100 {
        let message = Message::new(format!("Message {}", i).into_bytes());
        instrumented_publisher.publish("test-topic", vec![message]).await?;
    }
    
    // Export metrics
    println!("Metrics:\n{}", metrics.export_metrics()?);
    
    Ok(())
}
```

## Health Checks

### Application Health Monitoring

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub last_checked: String,
    pub response_time_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthReport {
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub timestamp: String,
}

pub struct HealthMonitor {
    checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn register_check<F, Fut>(&self, name: String, check_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String, String>> + Send,
    {
        let checks = self.checks.clone();
        let check_name = name.clone();
        
        tokio::spawn(async move {
            loop {
                let start = Instant::now();
                let (status, message) = match check_fn().await {
                    Ok(msg) => (HealthStatus::Healthy, msg),
                    Err(err) => (HealthStatus::Unhealthy, err),
                };
                
                let health_check = HealthCheck {
                    name: check_name.clone(),
                    status,
                    message,
                    last_checked: chrono::Utc::now().to_rfc3339(),
                    response_time_ms: start.elapsed().as_millis() as u64,
                };
                
                {
                    let mut checks_map = checks.write().await;
                    checks_map.insert(check_name.clone(), health_check);
                }
                
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }
    
    pub async fn get_health_report(&self) -> HealthReport {
        let checks_map = self.checks.read().await;
        let checks: Vec<HealthCheck> = checks_map.values().cloned().collect();
        
        let overall_status = if checks.iter().any(|c| matches!(c.status, HealthStatus::Unhealthy)) {
            HealthStatus::Unhealthy
        } else if checks.iter().any(|c| matches!(c.status, HealthStatus::Degraded)) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        
        HealthReport {
            overall_status,
            checks,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// Kincir-specific health checks
pub struct KincirHealthChecks {
    broker: Arc<kincir::memory::InMemoryBroker>,
}

impl KincirHealthChecks {
    pub fn new(broker: Arc<kincir::memory::InMemoryBroker>) -> Self {
        Self { broker }
    }
    
    pub async fn check_broker_health(&self) -> Result<String, String> {
        // Check if broker is responsive
        let publisher = kincir::memory::InMemoryPublisher::new(self.broker.clone());
        let test_message = Message::new(b"health-check".to_vec());
        
        match publisher.publish("health-check", vec![test_message]).await {
            Ok(_) => Ok("Broker is healthy".to_string()),
            Err(e) => Err(format!("Broker health check failed: {}", e)),
        }
    }
    
    pub async fn check_memory_usage(&self) -> Result<String, String> {
        // Simple memory usage check (in a real implementation, you'd use proper memory monitoring)
        let stats = self.broker.get_statistics().await;
        let total_messages = stats.total_messages_published + stats.total_messages_received;
        
        if total_messages > 1_000_000 {
            Err("High message volume detected".to_string())
        } else {
            Ok(format!("Memory usage normal (total messages: {})", total_messages))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let health_monitor = HealthMonitor::new();
    let kincir_checks = KincirHealthChecks::new(broker.clone());
    
    // Register health checks
    health_monitor.register_check(
        "broker_health".to_string(),
        {
            let kincir_checks = kincir_checks.clone();
            move || kincir_checks.check_broker_health()
        }
    ).await;
    
    health_monitor.register_check(
        "memory_usage".to_string(),
        {
            let kincir_checks = kincir_checks.clone();
            move || kincir_checks.check_memory_usage()
        }
    ).await;
    
    // Wait for health checks to run
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Get health report
    let report = health_monitor.get_health_report().await;
    println!("Health Report: {}", serde_json::to_string_pretty(&report)?);
    
    Ok(())
}
```

## Distributed Tracing

### OpenTelemetry Integration

```rust
use opentelemetry::{
    global,
    trace::{TraceContextExt, Tracer},
    Context, KeyValue,
};
use opentelemetry_jaeger::new_agent_pipeline;
use tracing::{info, instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub struct TracedPublisher<P> {
    publisher: P,
    tracer: Box<dyn Tracer + Send + Sync>,
}

impl<P> TracedPublisher<P>
where
    P: Publisher,
{
    pub fn new(publisher: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let tracer = new_agent_pipeline()
            .with_service_name("kincir-publisher")
            .install_simple()?;
        
        Ok(Self {
            publisher,
            tracer: Box::new(tracer),
        })
    }
}

#[async_trait::async_trait]
impl<P> Publisher for TracedPublisher<P>
where
    P: Publisher + Send + Sync,
{
    type Error = P::Error;
    
    #[instrument(skip(self, messages))]
    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        let span = self.tracer
            .start("kincir.publish")
            .with_attributes(vec![
                KeyValue::new("messaging.destination", topic.to_string()),
                KeyValue::new("messaging.message_count", messages.len() as i64),
            ]);
        
        let cx = Context::current_with_span(span);
        let _guard = cx.attach();
        
        // Add trace context to messages
        let traced_messages: Vec<Message> = messages
            .into_iter()
            .map(|mut msg| {
                let span_context = Span::current().context().span().span_context();
                msg.set_metadata("trace_id", &span_context.trace_id().to_string());
                msg.set_metadata("span_id", &span_context.span_id().to_string());
                msg
            })
            .collect();
        
        info!("Publishing {} messages to topic '{}'", traced_messages.len(), topic);
        
        match self.publisher.publish(topic, traced_messages).await {
            Ok(()) => {
                info!("Successfully published messages");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to publish messages: {:?}", e);
                Err(e)
            }
        }
    }
}

// Traced message processor
pub struct TracedMessageProcessor {
    tracer: Box<dyn Tracer + Send + Sync>,
}

impl TracedMessageProcessor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let tracer = new_agent_pipeline()
            .with_service_name("kincir-processor")
            .install_simple()?;
        
        Ok(Self {
            tracer: Box::new(tracer),
        })
    }
    
    #[instrument(skip(self, message))]
    pub async fn process_message(&self, message: Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Extract trace context from message
        let trace_id = message.metadata.get("trace_id");
        let span_id = message.metadata.get("span_id");
        
        let span = self.tracer
            .start("kincir.process_message")
            .with_attributes(vec![
                KeyValue::new("message.uuid", message.uuid.clone()),
                KeyValue::new("message.size", message.payload.len() as i64),
            ]);
        
        if let (Some(trace_id), Some(span_id)) = (trace_id, span_id) {
            span.add_event(
                "trace_context_extracted",
                vec![
                    KeyValue::new("parent.trace_id", trace_id.clone()),
                    KeyValue::new("parent.span_id", span_id.clone()),
                ],
            );
        }
        
        let cx = Context::current_with_span(span);
        let _guard = cx.attach();
        
        info!("Processing message: {}", message.uuid);
        
        // Simulate message processing
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        info!("Message processed successfully");
        Ok(())
    }
}
```

## Logging

### Structured Logging

```rust
use tracing::{info, warn, error, debug, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use serde_json::json;

pub struct StructuredLogger;

impl StructuredLogger {
    pub fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "kincir=debug,info".into()),
            )
            .with(tracing_subscriber::fmt::layer().json())
            .try_init()?;
        
        Ok(())
    }
}

pub struct LoggingPublisher<P> {
    publisher: P,
    service_name: String,
}

impl<P> LoggingPublisher<P>
where
    P: Publisher,
{
    pub fn new(publisher: P, service_name: String) -> Self {
        Self { publisher, service_name }
    }
}

#[async_trait::async_trait]
impl<P> Publisher for LoggingPublisher<P>
where
    P: Publisher + Send + Sync,
{
    type Error = P::Error;
    
    #[instrument(skip(self, messages), fields(service = %self.service_name, topic = %topic, message_count = messages.len()))]
    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        debug!("Starting message publish operation");
        
        let start = std::time::Instant::now();
        
        match self.publisher.publish(topic, messages.clone()).await {
            Ok(()) => {
                let duration = start.elapsed();
                info!(
                    duration_ms = duration.as_millis(),
                    "Messages published successfully"
                );
                
                // Log individual message details in debug mode
                for (i, message) in messages.iter().enumerate() {
                    debug!(
                        message_index = i,
                        message_uuid = %message.uuid,
                        message_size = message.payload.len(),
                        metadata = ?message.metadata,
                        "Message details"
                    );
                }
                
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    duration_ms = start.elapsed().as_millis(),
                    "Failed to publish messages"
                );
                Err(e)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    StructuredLogger::init()?;
    
    let broker = Arc::new(kincir::memory::InMemoryBroker::with_default_config());
    let base_publisher = kincir::memory::InMemoryPublisher::new(broker);
    let logging_publisher = LoggingPublisher::new(base_publisher, "order-service".to_string());
    
    // Publish messages with structured logging
    for i in 0..5 {
        let message = Message::new(format!("Order {}", i).into_bytes())
            .with_metadata("order_id", &format!("ORD-{:03}", i))
            .with_metadata("customer_id", &format!("CUST-{}", i % 3));
        
        logging_publisher.publish("orders", vec![message]).await?;
    }
    
    Ok(())
}
```

## Alerting

### Alert Manager Integration

```rust
use serde::{Serialize, Deserialize};
use reqwest::Client;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Alert {
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub starts_at: String,
    pub ends_at: Option<String>,
}

pub struct AlertManager {
    client: Client,
    webhook_url: String,
}

impl AlertManager {
    pub fn new(webhook_url: String) -> Self {
        Self {
            client: Client::new(),
            webhook_url,
        }
    }
    
    pub async fn send_alert(&self, alert: Alert) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let alerts = vec![alert];
        
        let response = self.client
            .post(&self.webhook_url)
            .json(&alerts)
            .send()
            .await?;
        
        if response.status().is_success() {
            println!("Alert sent successfully");
        } else {
            eprintln!("Failed to send alert: {}", response.status());
        }
        
        Ok(())
    }
    
    pub fn create_high_error_rate_alert(&self, service: &str, error_rate: f64) -> Alert {
        let mut labels = HashMap::new();
        labels.insert("alertname".to_string(), "HighErrorRate".to_string());
        labels.insert("service".to_string(), service.to_string());
        labels.insert("severity".to_string(), "critical".to_string());
        
        let mut annotations = HashMap::new();
        annotations.insert(
            "summary".to_string(),
            format!("High error rate detected in {}", service),
        );
        annotations.insert(
            "description".to_string(),
            format!("Error rate is {:.2}% which exceeds the threshold", error_rate * 100.0),
        );
        
        Alert {
            labels,
            annotations,
            starts_at: chrono::Utc::now().to_rfc3339(),
            ends_at: None,
        }
    }
}

// Monitoring service that triggers alerts
pub struct MonitoringService {
    metrics: Arc<MetricsCollector>,
    alert_manager: AlertManager,
    error_rate_threshold: f64,
}

impl MonitoringService {
    pub fn new(
        metrics: Arc<MetricsCollector>,
        alert_manager: AlertManager,
        error_rate_threshold: f64,
    ) -> Self {
        Self {
            metrics,
            alert_manager,
            error_rate_threshold,
        }
    }
    
    pub async fn check_and_alert(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This would typically get metrics from Prometheus
        // For demo purposes, we'll simulate checking error rate
        let error_rate = 0.15; // 15% error rate
        
        if error_rate > self.error_rate_threshold {
            let alert = self.alert_manager.create_high_error_rate_alert("kincir-service", error_rate);
            self.alert_manager.send_alert(alert).await?;
        }
        
        Ok(())
    }
}
```

This monitoring guide provides comprehensive observability for Kincir applications in production environments.
