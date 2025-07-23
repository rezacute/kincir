# Task 3: Correlation ID Tracking

## Overview
Implement correlation ID tracking throughout the message flow to enable distributed tracing, request tracking, and debugging across service boundaries. This feature will help track messages through complex processing pipelines.

## Requirements

### Functional Requirements
- Add correlation ID support to `Message` struct
- Automatic correlation ID generation and propagation
- Support for custom correlation ID injection
- Integration with logging and tracing systems
- Correlation ID preservation across Router hops
- Support for parent-child relationship tracking

### Non-Functional Requirements
- Minimal performance overhead
- Thread-safe correlation ID operations
- Backward compatibility with existing messages
- Integration with popular tracing frameworks

## Technical Design

### Enhanced Message Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub uuid: String,
    pub payload: Vec<u8>,
    pub metadata: HashMap<String, String>,
    
    // New correlation tracking fields
    pub correlation_id: String,
    pub parent_correlation_id: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub created_at: SystemTime,
    pub hop_count: u32,
}
```

### Correlation Context
```rust
#[derive(Debug, Clone)]
pub struct CorrelationContext {
    pub correlation_id: String,
    pub parent_correlation_id: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub service_name: String,
    pub operation_name: String,
    pub baggage: HashMap<String, String>,
}

impl CorrelationContext {
    pub fn new(service_name: &str, operation_name: &str) -> Self;
    pub fn child(&self, operation_name: &str) -> Self;
    pub fn with_trace_id(self, trace_id: String) -> Self;
    pub fn with_baggage(self, key: String, value: String) -> Self;
}
```

### Correlation Manager
```rust
pub struct CorrelationManager {
    service_name: String,
    id_generator: Box<dyn IdGenerator + Send + Sync>,
    context_propagator: Box<dyn ContextPropagator + Send + Sync>,
}

pub trait IdGenerator: Send + Sync {
    fn generate_correlation_id(&self) -> String;
    fn generate_trace_id(&self) -> String;
    fn generate_span_id(&self) -> String;
}

pub trait ContextPropagator: Send + Sync {
    fn inject(&self, context: &CorrelationContext, metadata: &mut HashMap<String, String>);
    fn extract(&self, metadata: &HashMap<String, String>) -> Option<CorrelationContext>;
}
```

### Integration with Tracing
```rust
#[cfg(feature = "tracing")]
pub struct TracingIntegration {
    tracer: opentelemetry::global::Tracer,
}

#[cfg(feature = "tracing")]
impl TracingIntegration {
    pub fn start_span(&self, context: &CorrelationContext) -> tracing::Span;
    pub fn end_span(&self, span: tracing::Span, result: Result<(), Box<dyn Error>>);
    pub fn add_event(&self, span: &tracing::Span, event: &str, attributes: HashMap<String, String>);
}
```

## Implementation Tasks

### Phase 1: Core Infrastructure (Day 1)
- [ ] Extend `Message` struct with correlation fields
- [ ] Implement `CorrelationContext` struct
- [ ] Create `IdGenerator` trait with UUID-based implementation
- [ ] Add backward compatibility for existing messages
- [ ] Implement message migration utilities

### Phase 2: Context Propagation (Day 1-2)
- [ ] Implement `ContextPropagator` trait
- [ ] Create W3C Trace Context propagator
- [ ] Create custom Kincir propagator format
- [ ] Add context injection/extraction utilities
- [ ] Implement baggage propagation

### Phase 3: Message Enhancement (Day 2)
- [ ] Update `Message::new()` to generate correlation IDs
- [ ] Add `Message::with_correlation()` method
- [ ] Add `Message::child_message()` for creating related messages
- [ ] Implement correlation ID validation
- [ ] Add hop count tracking

### Phase 4: Router Integration (Day 2-3)
- [ ] Update Router to propagate correlation context
- [ ] Add correlation ID to handler function signature
- [ ] Implement automatic parent-child relationships
- [ ] Add correlation-aware logging in Router
- [ ] Support correlation ID filtering and routing

### Phase 5: Backend Integration (Day 3)
- [ ] Update all Publishers to include correlation metadata
- [ ] Update all Subscribers to extract correlation context
- [ ] Ensure correlation IDs survive serialization/deserialization
- [ ] Add backend-specific correlation optimizations
- [ ] Test correlation propagation across different backends

## Testing Strategy

### Unit Tests
- [ ] Test correlation ID generation and uniqueness
- [ ] Test context propagation and extraction
- [ ] Test parent-child relationship creation
- [ ] Test backward compatibility with old messages
- [ ] Test hop count tracking

### Integration Tests
- [ ] Test correlation ID propagation through Router
- [ ] Test cross-backend correlation preservation
- [ ] Test tracing integration (if enabled)
- [ ] Test high-throughput correlation tracking
- [ ] Test correlation ID filtering and routing

### Performance Tests
- [ ] Measure overhead of correlation tracking
- [ ] Test memory usage with correlation data
- [ ] Benchmark correlation ID generation
- [ ] Test impact on message serialization

## File Structure
```
kincir/src/
├── correlation/
│   ├── mod.rs
│   ├── context.rs
│   ├── manager.rs
│   ├── propagator.rs
│   ├── generator.rs
│   └── tracing.rs (optional)
├── lib.rs (updated Message struct)
├── router.rs (updated with correlation)
└── */mod.rs (all backends updated)
```

## Example Usage

### Basic Correlation Tracking
```rust
use kincir::{Message, CorrelationContext, CorrelationManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let correlation_manager = CorrelationManager::new("my-service");
    
    // Create message with automatic correlation ID
    let message = Message::new(b"Hello".to_vec());
    println!("Correlation ID: {}", message.correlation_id);
    
    // Create child message
    let child_message = message.child_message(b"Child".to_vec());
    println!("Parent ID: {:?}", child_message.parent_correlation_id);
    
    // Create message with custom correlation context
    let context = CorrelationContext::new("my-service", "process-order")
        .with_trace_id("trace-123".to_string());
    
    let message = Message::with_correlation(b"Order".to_vec(), context);
    
    Ok(())
}
```

### Router with Correlation
```rust
use kincir::router::Router;
use kincir::correlation::CorrelationContext;

// Handler with correlation context
let handler = Arc::new(|msg: Message, ctx: CorrelationContext| {
    Box::pin(async move {
        println!("Processing message {} in correlation {}", 
                msg.uuid, ctx.correlation_id);
        
        // Create child message for downstream processing
        let child_msg = msg.child_message(b"Processed".to_vec());
        
        Ok(vec![child_msg])
    })
});

let router = Router::with_correlation(
    logger,
    "input-topic".to_string(),
    "output-topic".to_string(),
    subscriber,
    publisher,
    handler,
    correlation_manager,
);

router.run().await?;
```

### Tracing Integration
```rust
#[cfg(feature = "tracing")]
use kincir::correlation::TracingIntegration;
use tracing::{info, error};

let tracing_integration = TracingIntegration::new();

let handler = Arc::new(move |msg: Message, ctx: CorrelationContext| {
    let tracing = tracing_integration.clone();
    Box::pin(async move {
        let span = tracing.start_span(&ctx);
        let _guard = span.enter();
        
        info!("Processing message with correlation ID: {}", ctx.correlation_id);
        
        match process_message(&msg).await {
            Ok(result) => {
                info!("Message processed successfully");
                tracing.end_span(span, Ok(()));
                Ok(vec![result])
            }
            Err(e) => {
                error!("Message processing failed: {}", e);
                tracing.end_span(span, Err(e.clone()));
                Err(e)
            }
        }
    })
});
```

### Custom Correlation Propagation
```rust
use kincir::correlation::{ContextPropagator, CorrelationContext};

struct CustomPropagator;

impl ContextPropagator for CustomPropagator {
    fn inject(&self, context: &CorrelationContext, metadata: &mut HashMap<String, String>) {
        metadata.insert("x-correlation-id".to_string(), context.correlation_id.clone());
        if let Some(parent) = &context.parent_correlation_id {
            metadata.insert("x-parent-correlation-id".to_string(), parent.clone());
        }
        if let Some(trace_id) = &context.trace_id {
            metadata.insert("x-trace-id".to_string(), trace_id.clone());
        }
    }
    
    fn extract(&self, metadata: &HashMap<String, String>) -> Option<CorrelationContext> {
        let correlation_id = metadata.get("x-correlation-id")?.clone();
        let parent_correlation_id = metadata.get("x-parent-correlation-id").cloned();
        let trace_id = metadata.get("x-trace-id").cloned();
        
        Some(CorrelationContext {
            correlation_id,
            parent_correlation_id,
            trace_id,
            span_id: None,
            service_name: "unknown".to_string(),
            operation_name: "unknown".to_string(),
            baggage: HashMap::new(),
        })
    }
}
```

## Configuration Options
```rust
#[derive(Debug, Clone)]
pub struct CorrelationConfig {
    pub service_name: String,
    pub auto_generate: bool,
    pub propagation_format: PropagationFormat,
    pub max_hop_count: u32,
    pub enable_tracing: bool,
    pub baggage_keys: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PropagationFormat {
    W3CTraceContext,
    KincirCustom,
    Custom(Box<dyn ContextPropagator + Send + Sync>),
}
```

## Migration Guide

### Backward Compatibility
Existing messages without correlation IDs will be automatically assigned one:

```rust
// Old messages are automatically enhanced
let old_message = Message {
    uuid: "existing-uuid".to_string(),
    payload: vec![1, 2, 3],
    metadata: HashMap::new(),
};

// Correlation ID is generated on first access
println!("Auto-generated correlation ID: {}", old_message.correlation_id);
```

### Gradual Adoption
```rust
// Phase 1: Enable correlation tracking
let config = CorrelationConfig::default()
    .with_service_name("my-service")
    .with_auto_generate(true);

// Phase 2: Add tracing integration
let config = config.with_tracing(true);

// Phase 3: Custom propagation format
let config = config.with_propagation_format(PropagationFormat::W3CTraceContext);
```

## Success Criteria
- [ ] All messages have correlation IDs
- [ ] Parent-child relationships are properly tracked
- [ ] Correlation context propagates across all backends
- [ ] Integration with tracing frameworks works
- [ ] Performance overhead < 2%
- [ ] Backward compatibility maintained
- [ ] Comprehensive test coverage (>90%)

## Dependencies
- `uuid` (already in Cargo.toml)
- `std::time::SystemTime`
- `serde` for serialization
- `opentelemetry` (optional, for tracing integration)
- `tracing` (optional, for logging integration)

## Documentation Requirements
- [ ] API documentation for all correlation types
- [ ] Correlation tracking guide with examples
- [ ] Tracing integration documentation
- [ ] Custom propagator implementation guide
- [ ] Performance characteristics documentation
- [ ] Update README with correlation examples
