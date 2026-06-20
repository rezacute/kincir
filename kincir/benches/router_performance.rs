//! Benchmarks for the message routing pipeline.
//!
//! NOTE: `kincir::router::Router` is generic over backends whose associated
//! `Error` type is `Box<dyn Error + Send + Sync>`, which the in-memory backend
//! (whose error type is `InMemoryError`) does not satisfy. To benchmark the
//! routing work against the dependency-free in-memory broker, these benchmarks
//! drive the equivalent pipeline directly: subscribe -> publish -> receive ->
//! run the handler -> publish to the output topic -> receive from the output
//! topic. This mirrors exactly what `Router::run` does per message.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::router::HandlerFunc;
use kincir::{Message, Publisher, Subscriber};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_router_messages(count: usize, size: usize) -> Vec<Message> {
    let payload = vec![b'R'; size];
    (0..count)
        .map(|i| {
            Message::new(payload.clone())
                .with_metadata("router_id", &i.to_string())
                .with_metadata("benchmark", "router")
        })
        .collect()
}

// Simple pass-through handler
fn create_passthrough_handler() -> HandlerFunc {
    Arc::new(|msg: Message| Box::pin(async move { Ok(vec![msg]) }))
}

// Processing handler that adds metadata
fn create_processing_handler() -> HandlerFunc {
    Arc::new(|msg: Message| {
        Box::pin(async move {
            let processed = msg.with_metadata("processed", "true").with_metadata(
                "processed_at",
                &std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    .to_string(),
            );
            Ok(vec![processed])
        })
    })
}

// Transformation handler that modifies payload
fn create_transform_handler() -> HandlerFunc {
    Arc::new(|msg: Message| {
        Box::pin(async move {
            let mut new_payload = msg.payload.clone();
            new_payload.extend_from_slice(b" [TRANSFORMED]");
            let transformed = Message::new(new_payload).with_metadata("transformed", "true");
            Ok(vec![transformed])
        })
    })
}

/// Drive `count` messages through the routing pipeline and return the messages
/// received on the output topic.
async fn route_messages(
    broker: Arc<InMemoryBroker>,
    handler: &HandlerFunc,
    input_topic: &str,
    output_topic: &str,
    messages: Vec<Message>,
) -> Vec<Message> {
    let input_publisher = InMemoryPublisher::new(broker.clone());
    let output_publisher = InMemoryPublisher::new(broker.clone());
    let mut input_subscriber = InMemorySubscriber::new(broker.clone());
    let mut output_subscriber = InMemorySubscriber::new(broker.clone());

    // Subscribers must be registered before publishing so they receive the broadcast.
    input_subscriber.subscribe(input_topic).await.unwrap();
    output_subscriber.subscribe(output_topic).await.unwrap();

    let count = messages.len();
    input_publisher
        .publish(input_topic, messages)
        .await
        .unwrap();

    // Route each message: receive -> handler -> publish processed output.
    for _ in 0..count {
        let msg = input_subscriber.receive().await.unwrap();
        let processed = (handler)(msg).await.unwrap();
        if !processed.is_empty() {
            output_publisher
                .publish(output_topic, processed)
                .await
                .unwrap();
        }
    }

    // Drain the output topic.
    let mut received = Vec::with_capacity(count);
    for _ in 0..count {
        received.push(output_subscriber.receive().await.unwrap());
    }
    received
}

fn bench_router_passthrough(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("router_passthrough");

    for message_count in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*message_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}msg", message_count)),
            message_count,
            |b, &count| {
                let handler = create_passthrough_handler();
                b.to_async(&rt).iter(|| {
                    let handler = handler.clone();
                    async move {
                        let broker = Arc::new(InMemoryBroker::with_default_config());
                        let messages = create_router_messages(count, 128);
                        let received = route_messages(
                            broker,
                            &handler,
                            "input",
                            "output",
                            black_box(messages),
                        )
                        .await;
                        black_box(received);
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_router_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("router_processing");

    for message_count in [100, 500].iter() {
        group.throughput(Throughput::Elements(*message_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}msg", message_count)),
            message_count,
            |b, &count| {
                let handler = create_processing_handler();
                b.to_async(&rt).iter(|| {
                    let handler = handler.clone();
                    async move {
                        let broker = Arc::new(InMemoryBroker::with_default_config());
                        let messages = create_router_messages(count, 256);
                        let received = route_messages(
                            broker,
                            &handler,
                            "input",
                            "output",
                            black_box(messages),
                        )
                        .await;

                        for msg in &received {
                            assert_eq!(msg.metadata.get("processed"), Some(&"true".to_string()));
                        }
                        black_box(received);
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_router_transformation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("router_transformation");

    for message_size in [64, 256, 1024].iter() {
        group.throughput(Throughput::Bytes(*message_size as u64 * 100));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}bytes", message_size)),
            message_size,
            |b, &size| {
                let handler = create_transform_handler();
                b.to_async(&rt).iter(|| {
                    let handler = handler.clone();
                    async move {
                        let broker = Arc::new(InMemoryBroker::with_default_config());
                        let messages = create_router_messages(100, size);
                        let received = route_messages(
                            broker,
                            &handler,
                            "input",
                            "output",
                            black_box(messages),
                        )
                        .await;

                        for msg in &received {
                            assert!(String::from_utf8_lossy(&msg.payload).contains("[TRANSFORMED]"));
                        }
                        black_box(received);
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_router_concurrent_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("router_concurrent");

    for router_count in [1, 2, 4].iter() {
        let messages_per_router = 200;
        let total_messages = router_count * messages_per_router;

        group.throughput(Throughput::Elements(total_messages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}routers", router_count)),
            router_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async move {
                    let broker = Arc::new(InMemoryBroker::with_default_config());

                    let mut handles = Vec::new();
                    for router_id in 0..count {
                        let broker_clone = broker.clone();
                        let handle = tokio::spawn(async move {
                            let input_topic = format!("input_{}", router_id);
                            let output_topic = format!("output_{}", router_id);
                            let handler = create_processing_handler();

                            let messages = create_router_messages(messages_per_router, 128);
                            route_messages(
                                broker_clone,
                                &handler,
                                &input_topic,
                                &output_topic,
                                messages,
                            )
                            .await
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        let received = handle.await.unwrap();
                        black_box(received);
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_router_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("router_latency");
    group.sample_size(500);

    group.bench_function("single_message_latency", |b| {
        let handler = create_passthrough_handler();
        b.to_async(&rt).iter(|| {
            let handler = handler.clone();
            async move {
                let broker = Arc::new(InMemoryBroker::with_default_config());
                let message = Message::new(b"latency_test".to_vec());
                let received = route_messages(
                    broker,
                    &handler,
                    "input",
                    "output",
                    vec![black_box(message)],
                )
                .await;
                black_box(received);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_router_passthrough,
    bench_router_processing,
    bench_router_transformation,
    bench_router_concurrent_processing,
    bench_router_latency
);
criterion_main!(benches);
