---
layout: default
title: Configuration
parent: In-Memory Broker
grand_parent: Backends
nav_order: 2
---

# In-Memory Broker Configuration
{: .no_toc }

Comprehensive guide to configuring the in-memory message broker for different use cases.

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Configuration Builder

The `InMemoryConfig` struct provides a fluent builder API for configuring the broker:

```rust
use kincir::memory::InMemoryConfig;
use std::time::Duration;

let config = InMemoryConfig::new()
    .with_max_queue_size(Some(1000))
    .with_max_topics(Some(50))
    .with_stats(true)
    .with_maintain_order(true)
    .with_message_ttl(Some(Duration::from_secs(300)))
    .with_cleanup_interval(Duration::from_secs(60));
```

## Configuration Options

### Queue Management

#### Max Queue Size
Limits the number of messages that can be queued per topic:

```rust
// Unlimited queue size (default)
let config = InMemoryConfig::new().with_max_queue_size(None);

// Limited to 1000 messages per topic
let config = InMemoryConfig::new().with_max_queue_size(Some(1000));
```

**Use cases:**
- **Development**: Set low limits to catch queue overflow issues early
- **Testing**: Use moderate limits to prevent test memory bloat
- **Production**: Set based on expected message volume and available memory

#### Max Topics
Limits the total number of topics that can be created:

```rust
// Unlimited topics (default)
let config = InMemoryConfig::new().with_max_topics(None);

// Limited to 100 topics
let config = InMemoryConfig::new().with_max_topics(Some(100));
```

### Message Persistence

#### Enable Persistence
Controls whether messages are stored in topic queues:

```rust
// Enable persistence (default) - messages stored in queues
let config = InMemoryConfig::new().with_persistence(true);

// Disable persistence - messages only broadcast to active subscribers
let config = InMemoryConfig::new().with_persistence(false);
```

**When to disable:**
- Pure pub/sub scenarios (no message queuing needed)
- Maximum performance requirements
- Memory-constrained environments

### Message Ordering

#### Maintain Order
Controls whether messages maintain order and receive sequence numbers:

```rust
// Enable ordering (default) - messages get sequence numbers
let config = InMemoryConfig::new().with_maintain_order(true);

// Disable ordering - better performance, no sequence guarantees
let config = InMemoryConfig::new().with_maintain_order(false);
```

**Ordering adds:**
- `_sequence` metadata to each message
- Slight performance overhead
- Guaranteed message ordering within topics

### Statistics Collection

#### Enable Stats
Controls comprehensive statistics collection:

```rust
// Enable statistics collection
let config = InMemoryConfig::new().with_stats(true);

// Disable statistics (default) - better performance
let config = InMemoryConfig::new().with_stats(false);
```

**Statistics include:**
- Message publish/consume counts
- Topic creation/deletion counts
- Performance timing metrics
- Error counts
- Uptime tracking

### Message Time-To-Live (TTL)

#### Default Message TTL
Sets automatic message expiration:

```rust
// No TTL (default) - messages never expire
let config = InMemoryConfig::new().with_message_ttl(None);

// 5-minute TTL - messages expire after 5 minutes
let config = InMemoryConfig::new()
    .with_message_ttl(Some(Duration::from_secs(300)));
```

#### Cleanup Interval
Controls how often expired messages are cleaned up:

```rust
// Check for expired messages every minute (default)
let config = InMemoryConfig::new()
    .with_cleanup_interval(Duration::from_secs(60));

// More frequent cleanup - every 10 seconds
let config = InMemoryConfig::new()
    .with_cleanup_interval(Duration::from_secs(10));
```

## Pre-configured Profiles

### Default Configuration
Balanced configuration suitable for most use cases:

```rust
let config = InMemoryConfig::default();
// Equivalent to:
let config = InMemoryConfig::new()
    .with_max_queue_size(None)
    .with_max_topics(None)
    .with_persistence(true)
    .with_maintain_order(true)
    .with_stats(false)
    .with_message_ttl(None)
    .with_cleanup_interval(Duration::from_secs(60));
```

### High Performance Configuration
Optimized for maximum throughput and minimum latency:

```rust
let config = InMemoryConfig::high_performance();
// Equivalent to:
let config = InMemoryConfig::new()
    .with_max_queue_size(Some(10000))  // Large queues
    .with_max_topics(Some(1000))       // Many topics
    .with_persistence(true)
    .with_maintain_order(false)        // No ordering overhead
    .with_stats(false)                 // No stats overhead
    .with_message_ttl(None)            // No TTL overhead
    .with_cleanup_interval(Duration::from_secs(300)); // Less frequent cleanup
```

### Testing Configuration
Optimized for unit and integration tests:

```rust
let config = InMemoryConfig::for_testing();
// Equivalent to:
let config = InMemoryConfig::new()
    .with_max_queue_size(Some(100))    // Small queues
    .with_max_topics(Some(10))         // Few topics
    .with_persistence(true)
    .with_maintain_order(true)         // Predictable ordering
    .with_stats(true)                  // Enable debugging
    .with_message_ttl(None)            // No TTL for test stability
    .with_cleanup_interval(Duration::from_secs(3600)); // Infrequent cleanup
```

## Use Case Configurations

### Development Environment

```rust
let config = InMemoryConfig::new()
    .with_max_queue_size(Some(500))    // Catch overflow issues
    .with_max_topics(Some(20))         // Reasonable limit
    .with_stats(true)                  // Debug information
    .with_maintain_order(true)         // Predictable behavior
    .with_message_ttl(Some(Duration::from_secs(600))); // 10-minute cleanup
```

### Unit Testing

```rust
let config = InMemoryConfig::for_testing()
    .with_max_queue_size(Some(10))     // Small queues for fast tests
    .with_max_topics(Some(5));         // Few topics per test
```

### Integration Testing

```rust
let config = InMemoryConfig::for_testing()
    .with_max_queue_size(Some(100))    // Moderate queues
    .with_max_topics(Some(20))         // More topics for complex scenarios
    .with_stats(true);                 // Detailed test metrics
```

### Lightweight Production

```rust
let config = InMemoryConfig::new()
    .with_max_queue_size(Some(5000))   // Reasonable production limit
    .with_max_topics(Some(100))        // Production topic limit
    .with_maintain_order(true)         // Business requirement
    .with_message_ttl(Some(Duration::from_secs(1800))) // 30-minute TTL
    .with_cleanup_interval(Duration::from_secs(300));  // 5-minute cleanup
```

### High-Throughput Scenarios

```rust
let config = InMemoryConfig::high_performance()
    .with_max_queue_size(Some(50000))  // Very large queues
    .with_stats(false)                 // No performance overhead
    .with_maintain_order(false);       // Maximum throughput
```

### Memory-Constrained Environments

```rust
let config = InMemoryConfig::new()
    .with_max_queue_size(Some(100))    // Small queues
    .with_max_topics(Some(10))         // Few topics
    .with_persistence(false)           // No message storage
    .with_stats(false)                 // No stats overhead
    .with_message_ttl(Some(Duration::from_secs(60))) // Quick cleanup
    .with_cleanup_interval(Duration::from_secs(10)); // Frequent cleanup
```

## Configuration Validation

The broker validates configuration at startup:

```rust
use kincir::memory::{InMemoryBroker, InMemoryConfig, InMemoryError};

let config = InMemoryConfig::new()
    .with_max_queue_size(Some(0));  // Invalid: zero queue size

match InMemoryBroker::new(config) {
    Ok(broker) => println!("Broker created successfully"),
    Err(e) => println!("Configuration error: {}", e),
}
```

## Runtime Configuration Changes

Some configuration aspects can be modified at runtime:

```rust
let broker = Arc::new(InMemoryBroker::with_default_config());

// Check current configuration
let current_config = broker.config();
println!("Max queue size: {:?}", current_config.max_queue_size);

// Runtime operations
broker.cleanup_idle_topics(Duration::from_secs(300))?; // Manual cleanup
let health = broker.health_check(); // Check current state
```

## Performance Tuning

### For Maximum Throughput
- Disable message ordering (`maintain_order: false`)
- Disable statistics collection (`stats: false`)
- Use large queue sizes
- Reduce cleanup frequency
- Disable TTL if not needed

### For Minimum Latency
- Use small queue sizes
- Enable persistence for immediate delivery
- Disable unnecessary features
- Use high-performance profile

### For Memory Efficiency
- Set strict queue and topic limits
- Enable TTL with frequent cleanup
- Disable persistence if possible
- Monitor memory usage with health checks

## Monitoring Configuration

Enable comprehensive monitoring:

```rust
let config = InMemoryConfig::new()
    .with_stats(true);  // Enable statistics

let broker = Arc::new(InMemoryBroker::new(config));

// Monitor broker health
let health = broker.health_check();
println!("Memory usage: {} bytes", health.memory_usage_estimate);

// Monitor topic information
let topics = broker.list_topic_info();
for topic in topics {
    println!("Topic: {}, Queue: {}, Subscribers: {}", 
             topic.name, topic.queue_size, topic.subscriber_count);
}
```

## Configuration Best Practices

1. **Start with profiles**: Use pre-configured profiles as starting points
2. **Test configurations**: Validate configurations in development
3. **Monitor in production**: Enable health monitoring for production deployments
4. **Set reasonable limits**: Prevent resource exhaustion with appropriate limits
5. **Consider use case**: Optimize configuration for your specific requirements
6. **Document choices**: Document configuration decisions for team understanding
