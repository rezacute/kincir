use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kincir::memory::{InMemoryBroker, InMemoryConfig, InMemoryPublisher, InMemorySubscriber};
use kincir::{Message, Publisher, Subscriber};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_test_messages(count: usize, size: usize) -> Vec<Message> {
    let payload = vec![b'X'; size];
    (0..count)
        .map(|i| {
            Message::new(payload.clone())
                .with_metadata("message_id", &i.to_string())
                .with_metadata("benchmark", "memory_broker")
        })
        .collect()
}

fn bench_publish_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_broker_publish");

    for message_count in [100, 1000, 10000].iter() {
        for message_size in [64, 1024, 4096].iter() {
            group.throughput(Throughput::Elements(*message_count as u64));
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}msg_{}bytes", message_count, message_size)),
                &(*message_count, *message_size),
                |b, &(count, size)| {
                    b.to_async(&rt).iter(|| async {
                        let config = InMemoryConfig::new()
                            .with_max_queue_size(Some(count * 2))
                            .with_stats(false); // Disable stats for pure performance
                        let broker = Arc::new(InMemoryBroker::new(config));
                        let publisher = InMemoryPublisher::new(broker);

                        let messages = create_test_messages(count, size);

                        publisher
                            .publish("benchmark_topic", black_box(messages))
                            .await
                            .unwrap();
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_subscribe_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_broker_subscribe");

    for message_count in [100, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*message_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}msg", message_count)),
            message_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let config = InMemoryConfig::new()
                        .with_max_queue_size(Some(count * 2))
                        .with_stats(false);
                    let broker = Arc::new(InMemoryBroker::new(config));
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemorySubscriber::new(broker);

                    subscriber.subscribe("benchmark_topic").await.unwrap();

                    let messages = create_test_messages(count, 256);
                    publisher
                        .publish("benchmark_topic", messages)
                        .await
                        .unwrap();

                    for _ in 0..count {
                        let _ = black_box(subscriber.receive().await.unwrap());
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_broker_concurrent");

    for publisher_count in [1, 2, 4, 8].iter() {
        for subscriber_count in [1, 2, 4].iter() {
            let messages_per_publisher = 1000;
            group.throughput(Throughput::Elements(
                (publisher_count * messages_per_publisher) as u64,
            ));
            group.bench_with_input(
                BenchmarkId::from_parameter(format!(
                    "{}pub_{}sub",
                    publisher_count, subscriber_count
                )),
                &(*publisher_count, *subscriber_count),
                |b, &(pub_count, sub_count)| {
                    b.to_async(&rt).iter(|| async {
                        let config = InMemoryConfig::new()
                            .with_max_queue_size(Some(pub_count * messages_per_publisher * 2))
                            .with_stats(false);
                        let broker = Arc::new(InMemoryBroker::new(config));

                        // Create publishers
                        let mut publisher_handles = Vec::new();
                        for pub_id in 0..pub_count {
                            let broker_clone = broker.clone();
                            let handle = tokio::spawn(async move {
                                let publisher = InMemoryPublisher::new(broker_clone);
                                let messages = create_test_messages(messages_per_publisher, 128);
                                publisher
                                    .publish("concurrent_topic", messages)
                                    .await
                                    .unwrap();
                            });
                            publisher_handles.push(handle);
                        }

                        // Create subscribers
                        let mut subscriber_handles = Vec::new();
                        let messages_per_subscriber =
                            (pub_count * messages_per_publisher) / sub_count;

                        for _ in 0..sub_count {
                            let broker_clone = broker.clone();
                            let handle = tokio::spawn(async move {
                                let mut subscriber = InMemorySubscriber::new(broker_clone);
                                subscriber.subscribe("concurrent_topic").await.unwrap();

                                for _ in 0..messages_per_subscriber {
                                    let _ = subscriber.receive().await.unwrap();
                                }
                            });
                            subscriber_handles.push(handle);
                        }

                        // Wait for all operations to complete
                        for handle in publisher_handles {
                            handle.await.unwrap();
                        }
                        for handle in subscriber_handles {
                            handle.await.unwrap();
                        }
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_message_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_broker_latency");
    group.sample_size(1000);

    group.bench_function("single_message_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let broker = Arc::new(InMemoryBroker::with_default_config());
            let publisher = InMemoryPublisher::new(broker.clone());
            let mut subscriber = InMemorySubscriber::new(broker);

            subscriber.subscribe("latency_topic").await.unwrap();

            let message = Message::new(b"latency_test".to_vec());
            publisher
                .publish("latency_topic", vec![black_box(message)])
                .await
                .unwrap();

            let _ = black_box(subscriber.receive().await.unwrap());
        });
    });

    group.finish();
}

fn bench_memory_usage_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_broker_patterns");

    // Test different queue sizes
    for queue_size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("queue_size_{}", queue_size)),
            queue_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let config = InMemoryConfig::new()
                        .with_max_queue_size(Some(size))
                        .with_stats(false);
                    let broker = Arc::new(InMemoryBroker::new(config));
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemorySubscriber::new(broker);

                    subscriber.subscribe("pattern_topic").await.unwrap();

                    // Fill queue to capacity
                    let messages = create_test_messages(size / 2, 64);
                    publisher
                        .publish("pattern_topic", black_box(messages))
                        .await
                        .unwrap();

                    // Consume half the messages
                    for _ in 0..(size / 4) {
                        let _ = subscriber.receive().await.unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_publish_throughput,
    bench_subscribe_throughput,
    bench_concurrent_operations,
    bench_message_latency,
    bench_memory_usage_patterns
);
criterion_main!(benches);
