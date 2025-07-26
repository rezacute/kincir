use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kincir::ack::AckSubscriber;
use kincir::memory::{InMemoryAckSubscriber, InMemoryBroker, InMemoryPublisher};
use kincir::{Message, Publisher};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_ack_test_messages(count: usize, size: usize) -> Vec<Message> {
    let payload = vec![b'A'; size];
    (0..count)
        .map(|i| {
            Message::new(payload.clone())
                .with_metadata("ack_id", &i.to_string())
                .with_metadata("benchmark", "acknowledgment")
        })
        .collect()
}

fn bench_individual_acknowledgment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("acknowledgment_individual");

    for message_count in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*message_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}msg", message_count)),
            message_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemoryAckSubscriber::new(broker);

                    subscriber.subscribe("ack_individual").await.unwrap();

                    let messages = create_ack_test_messages(count, 128);
                    publisher.publish("ack_individual", messages).await.unwrap();

                    for _ in 0..count {
                        let (_, handle) = subscriber.receive_with_ack().await.unwrap();
                        subscriber.ack(black_box(handle)).await.unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_batch_acknowledgment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("acknowledgment_batch");

    for batch_size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_{}", batch_size)),
            batch_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemoryAckSubscriber::new(broker);

                    subscriber.subscribe("ack_batch").await.unwrap();

                    let messages = create_ack_test_messages(size, 128);
                    publisher.publish("ack_batch", messages).await.unwrap();

                    let mut handles = Vec::with_capacity(size);
                    for _ in 0..size {
                        let (_, handle) = subscriber.receive_with_ack().await.unwrap();
                        handles.push(handle);
                    }

                    subscriber.ack_batch(black_box(handles)).await.unwrap();
                });
            },
        );
    }
    group.finish();
}

fn bench_negative_acknowledgment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("acknowledgment_nack");

    for message_count in [100, 500].iter() {
        group.throughput(Throughput::Elements(*message_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}msg_nack", message_count)),
            message_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemoryAckSubscriber::new(broker);

                    subscriber.subscribe("ack_nack").await.unwrap();

                    let messages = create_ack_test_messages(count, 128);
                    publisher.publish("ack_nack", messages).await.unwrap();

                    for i in 0..count {
                        let (_, handle) = subscriber.receive_with_ack().await.unwrap();
                        // Alternate between ack and nack
                        if i % 2 == 0 {
                            subscriber.ack(black_box(handle)).await.unwrap();
                        } else {
                            subscriber.nack(black_box(handle), false).await.unwrap();
                        }
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_concurrent_acknowledgment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("acknowledgment_concurrent");

    for subscriber_count in [2, 4, 8].iter() {
        let messages_per_subscriber = 200;
        let total_messages = subscriber_count * messages_per_subscriber;

        group.throughput(Throughput::Elements(total_messages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}subscribers", subscriber_count)),
            subscriber_count,
            |b, &sub_count| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());

                    // Publish all messages first
                    let messages = create_ack_test_messages(total_messages, 128);
                    publisher.publish("ack_concurrent", messages).await.unwrap();

                    // Create concurrent subscribers
                    let mut handles = Vec::new();
                    for _ in 0..sub_count {
                        let broker_clone = broker.clone();
                        let handle = tokio::spawn(async move {
                            let mut subscriber = InMemoryAckSubscriber::new(broker_clone);
                            subscriber.subscribe("ack_concurrent").await.unwrap();

                            let mut ack_count = 0;
                            for _ in 0..messages_per_subscriber {
                                match subscriber.receive_with_ack().await {
                                    Ok((_, handle)) => {
                                        subscriber.ack(handle).await.unwrap();
                                        ack_count += 1;
                                    }
                                    Err(_) => break,
                                }
                            }
                            ack_count
                        });
                        handles.push(handle);
                    }

                    // Wait for all subscribers to complete
                    let mut total_acked = 0;
                    for handle in handles {
                        total_acked += handle.await.unwrap();
                    }

                    black_box(total_acked);
                });
            },
        );
    }
    group.finish();
}

fn bench_acknowledgment_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("acknowledgment_latency");
    group.sample_size(1000);

    group.bench_function("single_ack_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let broker = Arc::new(InMemoryBroker::with_default_config());
            let publisher = InMemoryPublisher::new(broker.clone());
            let mut subscriber = InMemoryAckSubscriber::new(broker);

            subscriber.subscribe("ack_latency").await.unwrap();

            let message = Message::new(b"latency_ack_test".to_vec());
            publisher
                .publish("ack_latency", vec![message])
                .await
                .unwrap();

            let (_, handle) = subscriber.receive_with_ack().await.unwrap();
            subscriber.ack(black_box(handle)).await.unwrap();
        });
    });

    group.bench_function("batch_ack_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let broker = Arc::new(InMemoryBroker::with_default_config());
            let publisher = InMemoryPublisher::new(broker.clone());
            let mut subscriber = InMemoryAckSubscriber::new(broker);

            subscriber.subscribe("ack_batch_latency").await.unwrap();

            let messages = create_ack_test_messages(10, 64);
            publisher
                .publish("ack_batch_latency", messages)
                .await
                .unwrap();

            let mut handles = Vec::new();
            for _ in 0..10 {
                let (_, handle) = subscriber.receive_with_ack().await.unwrap();
                handles.push(handle);
            }

            subscriber.ack_batch(black_box(handles)).await.unwrap();
        });
    });

    group.finish();
}

fn bench_acknowledgment_memory_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("acknowledgment_memory");

    // Test acknowledgment with different message sizes
    for message_size in [64, 512, 2048, 8192].iter() {
        group.throughput(Throughput::Bytes(*message_size as u64 * 100));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}bytes", message_size)),
            message_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let publisher = InMemoryPublisher::new(broker.clone());
                    let mut subscriber = InMemoryAckSubscriber::new(broker);

                    subscriber.subscribe("ack_memory").await.unwrap();

                    let messages = create_ack_test_messages(100, size);
                    publisher.publish("ack_memory", messages).await.unwrap();

                    for _ in 0..100 {
                        let (_, handle) = subscriber.receive_with_ack().await.unwrap();
                        subscriber.ack(black_box(handle)).await.unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_individual_acknowledgment,
    bench_batch_acknowledgment,
    bench_negative_acknowledgment,
    bench_concurrent_acknowledgment,
    bench_acknowledgment_latency,
    bench_acknowledgment_memory_patterns
);
criterion_main!(benches);
