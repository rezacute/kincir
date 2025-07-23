# Task 4: Performance Profiling & Benchmarks

## Overview
Establish comprehensive performance benchmarking and profiling infrastructure to measure, monitor, and optimize Kincir's performance across all backends and features. This will provide baseline metrics and identify performance bottlenecks.

## Requirements

### Functional Requirements
- Comprehensive benchmark suite covering all major operations
- Performance regression detection
- Memory usage profiling
- Latency and throughput measurements
- Backend comparison benchmarks
- Feature impact analysis (correlation IDs, ack/nack)

### Non-Functional Requirements
- Automated benchmark execution in CI/CD
- Historical performance tracking
- Reproducible benchmark results
- Clear performance reporting
- Integration with existing Criterion setup

## Technical Design

### Benchmark Categories

#### 1. Core Operations
- Message creation and serialization
- Publisher throughput
- Subscriber latency
- Router processing speed
- Memory allocation patterns

#### 2. Backend Comparisons
- Kafka vs RabbitMQ vs In-Memory performance
- Connection establishment overhead
- Message persistence costs
- Concurrent access patterns

#### 3. Feature Impact
- Correlation ID overhead
- Ack/Nack processing cost
- Metadata serialization impact
- Logging performance impact

### Benchmark Infrastructure
```rust
// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub message_sizes: Vec<usize>,
    pub batch_sizes: Vec<usize>,
    pub concurrent_clients: Vec<usize>,
    pub duration: Duration,
    pub warmup_duration: Duration,
    pub backends: Vec<BackendType>,
}

// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub throughput_msgs_per_sec: f64,
    pub latency_p50: Duration,
    pub latency_p95: Duration,
    pub latency_p99: Duration,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub error_rate: f64,
}

// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub backend: BackendType,
    pub config: BenchmarkConfig,
    pub metrics: PerformanceMetrics,
    pub timestamp: SystemTime,
    pub git_commit: String,
}
```

## Implementation Tasks

### Phase 1: Benchmark Infrastructure (Day 1)
- [ ] Extend existing Criterion setup with comprehensive benchmarks
- [ ] Create benchmark configuration system
- [ ] Implement performance metrics collection
- [ ] Add memory profiling utilities
- [ ] Create benchmark result storage and reporting

### Phase 2: Core Operation Benchmarks (Day 1-2)
- [ ] Message creation and manipulation benchmarks
- [ ] Serialization/deserialization performance tests
- [ ] UUID generation performance
- [ ] Metadata operations benchmarks
- [ ] Memory allocation profiling

### Phase 3: Backend Performance Benchmarks (Day 2-3)
- [ ] In-memory broker benchmarks
- [ ] Kafka publisher/subscriber benchmarks
- [ ] RabbitMQ publisher/subscriber benchmarks
- [ ] MQTT publisher/subscriber benchmarks
- [ ] Cross-backend comparison suite

### Phase 4: Feature Impact Benchmarks (Day 3)
- [ ] Correlation ID tracking overhead
- [ ] Ack/Nack processing performance
- [ ] Router processing benchmarks
- [ ] Logging impact measurements
- [ ] Concurrent access benchmarks

### Phase 5: Advanced Profiling (Day 4)
- [ ] Memory leak detection
- [ ] CPU profiling integration
- [ ] Async runtime performance analysis
- [ ] Connection pooling efficiency
- [ ] Garbage collection impact (if applicable)

## Testing Strategy

### Benchmark Categories

#### 1. Throughput Benchmarks
```rust
// Publisher throughput
fn bench_publisher_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("publisher_throughput");
    
    for backend in [BackendType::InMemory, BackendType::RabbitMQ, BackendType::Kafka] {
        for batch_size in [1, 10, 100, 1000] {
            for message_size in [100, 1024, 10240] {
                group.bench_with_input(
                    BenchmarkId::from_parameter(format!("{:?}_{}_{}", backend, batch_size, message_size)),
                    &(backend, batch_size, message_size),
                    |b, &(backend, batch_size, message_size)| {
                        b.to_async(Runtime::new().unwrap()).iter(|| async {
                            let publisher = create_publisher(backend).await;
                            let messages = create_messages(batch_size, message_size);
                            publisher.publish("test-topic", messages).await.unwrap();
                        });
                    },
                );
            }
        }
    }
    
    group.finish();
}
```

#### 2. Latency Benchmarks
```rust
// End-to-end latency
fn bench_end_to_end_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end_latency");
    
    for backend in [BackendType::InMemory, BackendType::RabbitMQ] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", backend)),
            &backend,
            |b, &backend| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let (publisher, mut subscriber) = create_pub_sub_pair(backend).await;
                    
                    let start = Instant::now();
                    let message = Message::new(b"test".to_vec());
                    publisher.publish("test-topic", vec![message.clone()]).await.unwrap();
                    
                    let received = subscriber.receive().await.unwrap();
                    let latency = start.elapsed();
                    
                    assert_eq!(received.payload, message.payload);
                    latency
                });
            },
        );
    }
    
    group.finish();
}
```

#### 3. Memory Usage Benchmarks
```rust
// Memory allocation patterns
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    group.bench_function("message_creation", |b| {
        b.iter(|| {
            let messages: Vec<Message> = (0..1000)
                .map(|i| Message::new(format!("message_{}", i).into_bytes()))
                .collect();
            black_box(messages);
        });
    });
    
    group.bench_function("correlation_tracking", |b| {
        b.iter(|| {
            let messages: Vec<Message> = (0..1000)
                .map(|i| {
                    let mut msg = Message::new(format!("message_{}", i).into_bytes());
                    msg = msg.with_correlation_id("test-correlation");
                    msg
                })
                .collect();
            black_box(messages);
        });
    });
    
    group.finish();
}
```

#### 4. Concurrent Access Benchmarks
```rust
// Concurrent publisher performance
fn bench_concurrent_publishers(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_publishers");
    
    for num_publishers in [1, 2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_publishers),
            &num_publishers,
            |b, &num_publishers| {
                b.to_async(Runtime::new().unwrap()).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::new(InMemoryConfig::default()));
                    
                    let handles: Vec<_> = (0..num_publishers)
                        .map(|i| {
                            let broker = broker.clone();
                            tokio::spawn(async move {
                                let publisher = InMemoryPublisher::new(broker);
                                let messages = create_messages(100, 1024);
                                publisher.publish(&format!("topic_{}", i), messages).await.unwrap();
                            })
                        })
                        .collect();
                    
                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            },
        );
    }
    
    group.finish();
}
```

## File Structure
```
kincir/
├── benches/
│   ├── kincir_benchmarks.rs (existing, to be extended)
│   ├── core_operations.rs
│   ├── backend_comparison.rs
│   ├── feature_impact.rs
│   ├── memory_profiling.rs
│   └── concurrent_access.rs
├── src/
│   └── bench_utils/
│       ├── mod.rs
│       ├── config.rs
│       ├── metrics.rs
│       ├── profiler.rs
│       └── reporter.rs
└── scripts/
    ├── run_benchmarks.sh
    ├── compare_results.py
    └── generate_report.py
```

## Benchmark Execution

### Local Development
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark category
cargo bench --bench core_operations

# Run with profiling
cargo bench --features profiling

# Generate HTML report
cargo bench -- --output-format html
```

### CI/CD Integration
```yaml
# .github/workflows/benchmarks.yml
name: Performance Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run benchmarks
        run: |
          cargo bench --bench kincir_benchmarks -- --output-format json > benchmark_results.json
      
      - name: Compare with baseline
        run: |
          python scripts/compare_results.py benchmark_results.json baseline_results.json
      
      - name: Upload results
        uses: actions/upload-artifact@v2
        with:
          name: benchmark-results
          path: benchmark_results.json
```

## Performance Targets

### Throughput Targets
- **In-Memory**: > 1M messages/sec for small messages (< 1KB)
- **RabbitMQ**: > 100K messages/sec for small messages
- **Kafka**: > 500K messages/sec for small messages

### Latency Targets
- **In-Memory**: < 1μs p99 latency
- **RabbitMQ**: < 10ms p99 latency
- **Kafka**: < 5ms p99 latency

### Memory Targets
- **Message overhead**: < 200 bytes per message
- **Correlation tracking**: < 5% memory overhead
- **Connection pooling**: < 1MB per connection

### Feature Impact Targets
- **Correlation ID**: < 2% throughput impact
- **Ack/Nack**: < 10% throughput impact
- **Logging**: < 5% throughput impact

## Profiling Tools Integration

### Memory Profiling
```rust
#[cfg(feature = "profiling")]
mod profiling {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    struct ProfilingAllocator;
    
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    
    unsafe impl GlobalAlloc for ProfilingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ret = System.alloc(layout);
            if !ret.is_null() {
                ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            }
            ret
        }
        
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout);
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
        }
    }
    
    #[global_allocator]
    static GLOBAL: ProfilingAllocator = ProfilingAllocator;
    
    pub fn current_memory_usage() -> usize {
        ALLOCATED.load(Ordering::SeqCst)
    }
}
```

### CPU Profiling
```rust
#[cfg(feature = "profiling")]
fn bench_with_cpu_profiling<F>(name: &str, f: F) 
where 
    F: FnOnce(),
{
    let guard = pprof::ProfilerGuard::new(100).unwrap();
    f();
    
    if let Ok(report) = guard.report().build() {
        let file = std::fs::File::create(format!("{}.pb", name)).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        report.pprof().unwrap().write_to_writer(&mut writer).unwrap();
    }
}
```

## Reporting and Analysis

### Benchmark Report Format
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "git_commit": "abc123def456",
  "rust_version": "1.75.0",
  "benchmarks": [
    {
      "name": "publisher_throughput_inmemory_100_1024",
      "backend": "InMemory",
      "metrics": {
        "throughput_msgs_per_sec": 1250000.0,
        "latency_p50": "0.8μs",
        "latency_p95": "1.2μs",
        "latency_p99": "2.1μs",
        "memory_usage_mb": 45.2,
        "cpu_usage_percent": 23.5,
        "error_rate": 0.0
      }
    }
  ]
}
```

### Performance Dashboard
- Historical performance trends
- Regression detection alerts
- Backend comparison charts
- Feature impact analysis
- Memory usage patterns

## Success Criteria
- [ ] Comprehensive benchmark suite covering all operations
- [ ] Performance baselines established for all backends
- [ ] Automated benchmark execution in CI/CD
- [ ] Performance regression detection system
- [ ] Memory profiling identifies optimization opportunities
- [ ] Benchmark results guide optimization priorities
- [ ] Performance documentation updated with targets

## Dependencies
- `criterion` (already in Cargo.toml)
- `pprof` (for CPU profiling)
- `jemallocator` (optional, for memory profiling)
- `tokio-test` (for async benchmarks)
- `sysinfo` (for system metrics)

## Documentation Requirements
- [ ] Benchmark execution guide
- [ ] Performance targets documentation
- [ ] Profiling tools usage guide
- [ ] Performance optimization recommendations
- [ ] CI/CD benchmark integration guide
- [ ] Performance regression investigation playbook
