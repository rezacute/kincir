use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::router::{Router, HandlerFunc};
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
    Arc::new(|msg: Message| {
        Box::pin(async move {
            Ok(vec![msg])
        })
    })
}

// Processing handler that adds metadata
fn create_processing_handler() -> HandlerFunc {
    Arc::new(|msg: Message| {
        Box::pin(async move {
            let processed = msg
                .with_metadata("processed", "true")
                .with_metadata("processed_at", &std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    .to_string());
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
            let transformed = Message::new(new_payload)
                .with_metadata("transformed", "true");
            Ok(vec![transformed])
        })
    })
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
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
                    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
                    let subscriber = Arc::new(InMemorySubscriber::new(broker.clone()));
                    let mut output_subscriber = InMemorySubscriber::new(broker.clone());
                    
                    #[cfg(feature = "logging")]
                    let router = {
                        use kincir::logging::StdLogger;
                        let logger = Arc::new(StdLogger::new(false, false));
                        Router::new(
                            logger,
                            "input".to_string(),
                            "output".to_string(),
                            subscriber,
                            output_publisher,
                            create_passthrough_handler(),
                        )
                    };
                    
                    #[cfg(not(feature = "logging"))]
                    let router = Router::new(
                        "input".to_string(),
                        "output".to_string(),
                        subscriber,
                        output_publisher,
                        create_passthrough_handler(),
                    );
                    
                    output_subscriber.subscribe("output").await.unwrap();
                    
                    let messages = create_router_messages(count, 128);
                    input_publisher.publish("input", black_box(messages)).await.unwrap();
                    
                    // Process messages through router
                    for _ in 0..count {
                        router.process_single_message().await.unwrap();
                    }
                    
                    // Verify output
                    for _ in 0..count {
                        let _ = black_box(output_subscriber.receive().await.unwrap());
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
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
                    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
                    let subscriber = Arc::new(InMemorySubscriber::new(broker.clone()));
                    let mut output_subscriber = InMemorySubscriber::new(broker.clone());
                    
                    #[cfg(feature = "logging")]
                    let router = {
                        use kincir::logging::StdLogger;
                        let logger = Arc::new(StdLogger::new(false, false));
                        Router::new(
                            logger,
                            "input".to_string(),
                            "output".to_string(),
                            subscriber,
                            output_publisher,
                            create_processing_handler(),
                        )
                    };
                    
                    #[cfg(not(feature = "logging"))]
                    let router = Router::new(
                        "input".to_string(),
                        "output".to_string(),
                        subscriber,
                        output_publisher,
                        create_processing_handler(),
                    );
                    
                    output_subscriber.subscribe("output").await.unwrap();
                    
                    let messages = create_router_messages(count, 256);
                    input_publisher.publish("input", black_box(messages)).await.unwrap();
                    
                    // Process messages through router
                    for _ in 0..count {
                        router.process_single_message().await.unwrap();
                    }
                    
                    // Verify output
                    for _ in 0..count {
                        let msg = output_subscriber.receive().await.unwrap();
                        assert_eq!(msg.metadata.get("processed"), Some(&"true".to_string()));
                        black_box(msg);
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
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
                    let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
                    let subscriber = Arc::new(InMemorySubscriber::new(broker.clone()));
                    let mut output_subscriber = InMemorySubscriber::new(broker.clone());
                    
                    #[cfg(feature = "logging")]
                    let router = {
                        use kincir::logging::StdLogger;
                        let logger = Arc::new(StdLogger::new(false, false));
                        Router::new(
                            logger,
                            "input".to_string(),
                            "output".to_string(),
                            subscriber,
                            output_publisher,
                            create_transform_handler(),
                        )
                    };
                    
                    #[cfg(not(feature = "logging"))]
                    let router = Router::new(
                        "input".to_string(),
                        "output".to_string(),
                        subscriber,
                        output_publisher,
                        create_transform_handler(),
                    );
                    
                    output_subscriber.subscribe("output").await.unwrap();
                    
                    let messages = create_router_messages(100, size);
                    input_publisher.publish("input", black_box(messages)).await.unwrap();
                    
                    // Process messages through router
                    for _ in 0..100 {
                        router.process_single_message().await.unwrap();
                    }
                    
                    // Verify output
                    for _ in 0..100 {
                        let msg = output_subscriber.receive().await.unwrap();
                        assert!(String::from_utf8_lossy(&msg.payload).contains("[TRANSFORMED]"));
                        black_box(msg);
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
                b.to_async(&rt).iter(|| async {
                    let broker = Arc::new(InMemoryBroker::with_default_config());
                    
                    let mut handles = Vec::new();
                    for router_id in 0..count {
                        let broker_clone = broker.clone();
                        let handle = tokio::spawn(async move {
                            let input_topic = format!("input_{}", router_id);
                            let output_topic = format!("output_{}", router_id);
                            
                            let input_publisher = Arc::new(InMemoryPublisher::new(broker_clone.clone()));
                            let output_publisher = Arc::new(InMemoryPublisher::new(broker_clone.clone()));
                            let subscriber = Arc::new(InMemorySubscriber::new(broker_clone.clone()));
                            let mut output_subscriber = InMemorySubscriber::new(broker_clone);
                            
                            #[cfg(feature = "logging")]
                            let router = {
                                use kincir::logging::StdLogger;
                                let logger = Arc::new(StdLogger::new(false, false));
                                Router::new(
                                    logger,
                                    input_topic.clone(),
                                    output_topic.clone(),
                                    subscriber,
                                    output_publisher,
                                    create_processing_handler(),
                                )
                            };
                            
                            #[cfg(not(feature = "logging"))]
                            let router = Router::new(
                                input_topic.clone(),
                                output_topic.clone(),
                                subscriber,
                                output_publisher,
                                create_processing_handler(),
                            );
                            
                            output_subscriber.subscribe(&output_topic).await.unwrap();
                            
                            let messages = create_router_messages(messages_per_router, 128);
                            input_publisher.publish(&input_topic, messages).await.unwrap();
                            
                            // Process messages
                            for _ in 0..messages_per_router {
                                router.process_single_message().await.unwrap();
                            }
                            
                            // Verify output
                            for _ in 0..messages_per_router {
                                output_subscriber.receive().await.unwrap();
                            }
                        });
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        handle.await.unwrap();
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
        b.to_async(&rt).iter(|| async {
            let broker = Arc::new(InMemoryBroker::with_default_config());
            let input_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
            let output_publisher = Arc::new(InMemoryPublisher::new(broker.clone()));
            let subscriber = Arc::new(InMemorySubscriber::new(broker.clone()));
            let mut output_subscriber = InMemorySubscriber::new(broker.clone());
            
            #[cfg(feature = "logging")]
            let router = {
                use kincir::logging::StdLogger;
                let logger = Arc::new(StdLogger::new(false, false));
                Router::new(
                    logger,
                    "input".to_string(),
                    "output".to_string(),
                    subscriber,
                    output_publisher,
                    create_passthrough_handler(),
                )
            };
            
            #[cfg(not(feature = "logging"))]
            let router = Router::new(
                "input".to_string(),
                "output".to_string(),
                subscriber,
                output_publisher,
                create_passthrough_handler(),
            );
            
            output_subscriber.subscribe("output").await.unwrap();
            
            let message = Message::new(b"latency_test".to_vec());
            input_publisher.publish("input", vec![black_box(message)]).await.unwrap();
            
            router.process_single_message().await.unwrap();
            let _ = black_box(output_subscriber.receive().await.unwrap());
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
