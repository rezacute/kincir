use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kincir::memory::{
    InMemoryAckSubscriberFixed, InMemoryBroker, InMemoryPublisher, InMemorySubscriber,
};
use kincir::{AckSubscriber, Message, Publisher, Subscriber};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Benchmark configuration for different scenarios
#[derive(Clone)]
struct BenchmarkConfig {
    message_count: usize,
    message_size: usize,
    concurrent_publishers: usize,
    concurrent_subscribers: usize,
}

impl BenchmarkConfig {
    fn small() -> Self {
        Self {
            message_count: 100,
            message_size: 64,
            concurrent_publishers: 1,
            concurrent_subscribers: 1,
        }
    }

    fn medium() -> Self {
        Self {
            message_count: 1000,
            message_size: 1024,
            concurrent_publishers: 2,
            concurrent_subscribers: 2,
        }
    }

    fn large() -> Self {
        Self {
            message_count: 5000,
            message_size: 4096,
            concurrent_publishers: 4,
            concurrent_subscribers: 4,
        }
    }
}

/// Create test messages for benchmarking
fn create_test_messages(count: usize, size: usize) -> Vec<Message> {
    (0..count)
        .map(|i| {
            let payload = vec![0u8; size];
            let mut msg = Message::new(payload);
            msg = msg.with_metadata("benchmark_id", &i.to_string());
            msg = msg.with_metadata(
                "timestamp",
                &std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .to_string(),
            );
            msg
        })
        .collect()
}

/// Benchmark in-memory broker publishing performance
fn bench_in_memory_publish(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("in_memory_publish");

    for config in [
        BenchmarkConfig::small(),
        BenchmarkConfig::medium(),
        BenchmarkConfig::large(),
    ] {
        group.throughput(Throughput::Elements(config.message_count as u64));

        group.bench_with_input(
            BenchmarkId::new(
                "single_publisher",
                format!("{}msg_{}bytes", config.message_count, config.message_size),
            ),
            &config,
            |b, config| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker);
                    let messages = create_test_messages(config.message_count, config.message_size);

                    let start = std::time::Instant::now();
                    publisher
                        .publish("benchmark_topic", messages)
                        .await
                        .unwrap();
                    let duration = start.elapsed();

                    black_box(duration);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark in-memory broker subscription performance
fn bench_in_memory_subscribe(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("in_memory_subscribe");

    for config in [BenchmarkConfig::small(), BenchmarkConfig::medium()] {
        group.throughput(Throughput::Elements(config.message_count as u64));

        group.bench_with_input(
            BenchmarkId::new(
                "single_subscriber",
                format!("{}msg_{}bytes", config.message_count, config.message_size),
            ),
            &config,
            |b, config| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemorySubscriber::new(broker);
                    let messages = create_test_messages(config.message_count, config.message_size);

                    subscriber.subscribe("benchmark_topic").await.unwrap();
                    publisher
                        .publish("benchmark_topic", messages)
                        .await
                        .unwrap();

                    let start = std::time::Instant::now();
                    for _ in 0..config.message_count {
                        let _msg = subscriber.receive().await.unwrap();
                    }
                    let duration = start.elapsed();

                    black_box(duration);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark acknowledgment performance
fn bench_acknowledgment_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("acknowledgment_performance");

    for config in [BenchmarkConfig::small(), BenchmarkConfig::medium()] {
        group.throughput(Throughput::Elements(config.message_count as u64));

        group.bench_with_input(
            BenchmarkId::new("ack_individual", format!("{}msg", config.message_count)),
            &config,
            |b, config| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemoryAckSubscriberFixed::new(broker);
                    let messages = create_test_messages(config.message_count, config.message_size);

                    subscriber.subscribe("ack_benchmark").await.unwrap();
                    publisher.publish("ack_benchmark", messages).await.unwrap();

                    let start = std::time::Instant::now();
                    for _ in 0..config.message_count {
                        let (_msg, handle) = subscriber.receive_with_ack().await.unwrap();
                        subscriber.ack(handle).await.unwrap();
                    }
                    let duration = start.elapsed();

                    black_box(duration);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ack_batch", format!("{}msg", config.message_count)),
            &config,
            |b, config| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemoryAckSubscriberFixed::new(broker);
                    let messages = create_test_messages(config.message_count, config.message_size);

                    subscriber.subscribe("ack_batch_benchmark").await.unwrap();
                    publisher
                        .publish("ack_batch_benchmark", messages)
                        .await
                        .unwrap();

                    let mut handles = Vec::new();
                    for _ in 0..config.message_count {
                        let (_msg, handle) = subscriber.receive_with_ack().await.unwrap();
                        handles.push(handle);
                    }

                    let start = std::time::Instant::now();
                    subscriber.ack_batch(handles).await.unwrap();
                    let duration = start.elapsed();

                    black_box(duration);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_operations");

    for config in [BenchmarkConfig::small(), BenchmarkConfig::medium()] {
        group.throughput(Throughput::Elements(
            (config.message_count * config.concurrent_publishers) as u64,
        ));

        group.bench_with_input(
            BenchmarkId::new(
                "concurrent_publish",
                format!(
                    "{}pub_{}msg",
                    config.concurrent_publishers, config.message_count
                ),
            ),
            &config,
            |b, config| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let messages = create_test_messages(config.message_count, config.message_size);

                    let start = std::time::Instant::now();

                    let mut handles = Vec::new();
                    for i in 0..config.concurrent_publishers {
                        let broker_clone = broker.clone();
                        let messages_clone = messages.clone();
                        let handle = tokio::spawn(async move {
                            let publisher = InMemoryPublisher::new(broker_clone);
                            publisher
                                .publish(&format!("concurrent_topic_{}", i), messages_clone)
                                .await
                                .unwrap();
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }

                    let duration = start.elapsed();
                    black_box(duration);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new(
                "concurrent_subscribe",
                format!(
                    "{}sub_{}msg",
                    config.concurrent_subscribers, config.message_count
                ),
            ),
            &config,
            |b, config| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let messages = create_test_messages(config.message_count, config.message_size);

                    // Pre-publish messages to all topics
                    for i in 0..config.concurrent_subscribers {
                        publisher
                            .publish(&format!("concurrent_sub_topic_{}", i), messages.clone())
                            .await
                            .unwrap();
                    }

                    let start = std::time::Instant::now();

                    let mut handles = Vec::new();
                    for i in 0..config.concurrent_subscribers {
                        let broker_clone = broker.clone();
                        let message_count = config.message_count;
                        let handle = tokio::spawn(async move {
                            let mut subscriber = InMemorySubscriber::new(broker_clone);
                            subscriber
                                .subscribe(&format!("concurrent_sub_topic_{}", i))
                                .await
                                .unwrap();

                            for _ in 0..message_count {
                                let _msg = subscriber.receive().await.unwrap();
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }

                    let duration = start.elapsed();
                    black_box(duration);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark message size impact
fn bench_message_size_impact(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("message_size_impact");

    let message_sizes = [64, 256, 1024, 4096, 16384]; // 64B to 16KB
    let message_count = 1000;

    for size in message_sizes {
        group.throughput(Throughput::Bytes((message_count * size) as u64));

        group.bench_with_input(
            BenchmarkId::new("publish_by_size", format!("{}bytes", size)),
            &size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker);
                    let messages = create_test_messages(message_count, size);

                    let start = std::time::Instant::now();
                    publisher
                        .publish("size_test_topic", messages)
                        .await
                        .unwrap();
                    let duration = start.elapsed();

                    black_box(duration);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("subscribe_by_size", format!("{}bytes", size)),
            &size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemorySubscriber::new(broker);
                    let messages = create_test_messages(message_count, size);

                    subscriber.subscribe("size_test_topic").await.unwrap();
                    publisher
                        .publish("size_test_topic", messages)
                        .await
                        .unwrap();

                    let start = std::time::Instant::now();
                    for _ in 0..message_count {
                        let _msg = subscriber.receive().await.unwrap();
                    }
                    let duration = start.elapsed();

                    black_box(duration);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark broker health and statistics impact
fn bench_broker_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("broker_overhead");

    let message_count = 1000;
    let message_size = 1024;

    group.bench_function("with_statistics", |b| {
        b.to_async(&rt).iter(|| async {
            let broker = Arc::new(InMemoryBroker::with_default_config());
            let publisher = InMemoryPublisher::new(broker.clone());
            let messages = create_test_messages(message_count, message_size);

            let start = std::time::Instant::now();
            publisher.publish("stats_topic", messages).await.unwrap();

            // Access statistics (simulating monitoring)
            let _stats = broker.stats();
            let _health = broker.health_check();

            let duration = start.elapsed();
            black_box(duration);
        });
    });

    group.bench_function("topic_operations", |b| {
        b.to_async(&rt).iter(|| async {
            let broker = Arc::new(InMemoryBroker::with_default_config());
            let publisher = InMemoryPublisher::new(broker.clone());
            let messages = create_test_messages(100, message_size); // Smaller batch for topic ops

            let start = std::time::Instant::now();

            // Multiple topic operations
            for i in 0..10 {
                publisher
                    .publish(&format!("topic_{}", i), messages.clone())
                    .await
                    .unwrap();
            }

            // Topic management operations
            let _topic_count = broker.topic_count();
            let _topics = broker.list_topics();

            let duration = start.elapsed();
            black_box(duration);
        });
    });

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_patterns");

    group.bench_function("memory_cleanup", |b| {
        b.to_async(&rt).iter(|| async {
            let broker = Arc::new(InMemoryBroker::with_default_config());
            let publisher = InMemoryPublisher::new(broker.clone());
            let mut subscriber = InMemorySubscriber::new(broker.clone());

            // Create and process messages
            subscriber.subscribe("cleanup_topic").await.unwrap();

            let start = std::time::Instant::now();

            // Simulate high-throughput with cleanup
            for batch in 0..10 {
                let messages = create_test_messages(100, 1024);
                publisher.publish("cleanup_topic", messages).await.unwrap();

                // Consume messages to trigger cleanup
                for _ in 0..100 {
                    let _msg = subscriber.receive().await.unwrap();
                }

                // Trigger cleanup operations
                if batch % 3 == 0 {
                    let _health = broker.health_check();
                }
            }

            let duration = start.elapsed();
            black_box(duration);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_in_memory_publish,
    bench_in_memory_subscribe,
    bench_acknowledgment_performance,
    bench_concurrent_operations,
    bench_message_size_impact,
    bench_broker_overhead,
    bench_memory_patterns
);

criterion_main!(benches);
