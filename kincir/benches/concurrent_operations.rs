use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kincir::memory::{InMemoryBroker, InMemoryConfig, InMemoryPublisher, InMemorySubscriber};
use kincir::{Message, Publisher, Subscriber};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_concurrent_messages(count: usize, prefix: &str) -> Vec<Message> {
    (0..count)
        .map(|i| {
            Message::new(format!("{} message {}", prefix, i).into_bytes())
                .with_metadata("message_id", &i.to_string())
                .with_metadata("benchmark", "concurrent")
        })
        .collect()
}

fn bench_multiple_publishers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_publishers");
    
    for publisher_count in [2, 4, 8, 16].iter() {
        let messages_per_publisher = 500;
        let total_messages = publisher_count * messages_per_publisher;
        
        group.throughput(Throughput::Elements(total_messages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}publishers", publisher_count)),
            publisher_count,
            |b, &pub_count| {
                b.to_async(&rt).iter(|| async {
                    let config = InMemoryConfig::new()
                        .with_max_queue_size(Some(total_messages * 2))
                        .with_stats(false);
                    let broker = Arc::new(InMemoryBroker::new(config));
                    
                    let mut handles = Vec::new();
                    for pub_id in 0..pub_count {
                        let broker_clone = broker.clone();
                        let handle = tokio::spawn(async move {
                            let publisher = InMemoryPublisher::new(broker_clone);
                            let messages = create_concurrent_messages(
                                messages_per_publisher,
                                &format!("Pub{}", pub_id)
                            );
                            publisher.publish("concurrent_topic", black_box(messages)).await.unwrap();
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

fn bench_multiple_subscribers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_subscribers");
    
    for subscriber_count in [2, 4, 8].iter() {
        let total_messages = 1000;
        let messages_per_subscriber = total_messages / subscriber_count;
        
        group.throughput(Throughput::Elements(total_messages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}subscribers", subscriber_count)),
            subscriber_count,
            |b, &sub_count| {
                b.to_async(&rt).iter(|| async {
                    let config = InMemoryConfig::new()
                        .with_max_queue_size(Some(total_messages * 2))
                        .with_stats(false);
                    let broker = Arc::new(InMemoryBroker::new(config));
                    let publisher = InMemoryPublisher::new(broker.clone());
                    
                    // Publish all messages first
                    let messages = create_concurrent_messages(total_messages, "MultiSub");
                    publisher.publish("multi_sub_topic", messages).await.unwrap();
                    
                    let mut handles = Vec::new();
                    for _ in 0..sub_count {
                        let broker_clone = broker.clone();
                        let handle = tokio::spawn(async move {
                            let mut subscriber = InMemorySubscriber::new(broker_clone);
                            subscriber.subscribe("multi_sub_topic").await.unwrap();
                            
                            let mut received = 0;
                            for _ in 0..messages_per_subscriber {
                                match subscriber.receive().await {
                                    Ok(_) => received += 1,
                                    Err(_) => break,
                                }
                            }
                            received
                        });
                        handles.push(handle);
                    }
                    
                    let mut total_received = 0;
                    for handle in handles {
                        total_received += handle.await.unwrap();
                    }
                    black_box(total_received);
                });
            },
        );
    }
    group.finish();
}

fn bench_publisher_subscriber_pairs(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("pub_sub_pairs");
    
    for pair_count in [1, 2, 4, 8].iter() {
        let messages_per_pair = 200;
        let total_messages = pair_count * messages_per_pair;
        
        group.throughput(Throughput::Elements(total_messages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}pairs", pair_count)),
            pair_count,
            |b, &pairs| {
                b.to_async(&rt).iter(|| async {
                    let config = InMemoryConfig::new()
                        .with_max_queue_size(Some(total_messages * 2))
                        .with_stats(false);
                    let broker = Arc::new(InMemoryBroker::new(config));
                    
                    let mut handles = Vec::new();
                    for pair_id in 0..pairs {
                        let broker_clone = broker.clone();
                        let handle = tokio::spawn(async move {
                            let topic = format!("pair_topic_{}", pair_id);
                            
                            // Create publisher and subscriber for this pair
                            let publisher = InMemoryPublisher::new(broker_clone.clone());
                            let mut subscriber = InMemorySubscriber::new(broker_clone);
                            
                            subscriber.subscribe(&topic).await.unwrap();
                            
                            // Publish messages
                            let messages = create_concurrent_messages(messages_per_pair, &format!("Pair{}", pair_id));
                            publisher.publish(&topic, messages).await.unwrap();
                            
                            // Consume messages
                            for _ in 0..messages_per_pair {
                                subscriber.receive().await.unwrap();
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

fn bench_topic_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("topic_scaling");
    
    for topic_count in [1, 5, 10, 20].iter() {
        let messages_per_topic = 100;
        let total_messages = topic_count * messages_per_topic;
        
        group.throughput(Throughput::Elements(total_messages as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}topics", topic_count)),
            topic_count,
            |b, &topics| {
                b.to_async(&rt).iter(|| async {
                    let config = InMemoryConfig::new()
                        .with_max_queue_size(Some(total_messages * 2))
                        .with_stats(false);
                    let broker = Arc::new(InMemoryBroker::new(config));
                    
                    let mut handles = Vec::new();
                    for topic_id in 0..topics {
                        let broker_clone = broker.clone();
                        let handle = tokio::spawn(async move {
                            let topic = format!("scale_topic_{}", topic_id);
                            let publisher = InMemoryPublisher::new(broker_clone.clone());
                            let mut subscriber = InMemorySubscriber::new(broker_clone);
                            
                            subscriber.subscribe(&topic).await.unwrap();
                            
                            let messages = create_concurrent_messages(messages_per_topic, &format!("Topic{}", topic_id));
                            publisher.publish(&topic, black_box(messages)).await.unwrap();
                            
                            for _ in 0..messages_per_topic {
                                let _ = black_box(subscriber.receive().await.unwrap());
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

fn bench_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("mixed_workload");
    
    group.bench_function("realistic_workload", |b| {
        b.to_async(&rt).iter(|| async {
            let config = InMemoryConfig::new()
                .with_max_queue_size(Some(5000))
                .with_stats(false);
            let broker = Arc::new(InMemoryBroker::new(config));
            
            // Simulate a realistic mixed workload:
            // - 3 publishers with different message rates
            // - 2 subscribers consuming from different topics
            // - Some cross-topic communication
            
            let mut handles = Vec::new();
            
            // High-frequency publisher
            let broker_clone = broker.clone();
            handles.push(tokio::spawn(async move {
                let publisher = InMemoryPublisher::new(broker_clone);
                let messages = create_concurrent_messages(500, "HighFreq");
                publisher.publish("high_freq_topic", messages).await.unwrap();
            }));
            
            // Medium-frequency publisher
            let broker_clone = broker.clone();
            handles.push(tokio::spawn(async move {
                let publisher = InMemoryPublisher::new(broker_clone);
                let messages = create_concurrent_messages(200, "MedFreq");
                publisher.publish("med_freq_topic", messages).await.unwrap();
            }));
            
            // Low-frequency publisher
            let broker_clone = broker.clone();
            handles.push(tokio::spawn(async move {
                let publisher = InMemoryPublisher::new(broker_clone);
                let messages = create_concurrent_messages(100, "LowFreq");
                publisher.publish("low_freq_topic", messages).await.unwrap();
            }));
            
            // Fast subscriber (high-freq topic)
            let broker_clone = broker.clone();
            handles.push(tokio::spawn(async move {
                let mut subscriber = InMemorySubscriber::new(broker_clone);
                subscriber.subscribe("high_freq_topic").await.unwrap();
                for _ in 0..500 {
                    subscriber.receive().await.unwrap();
                }
            }));
            
            // Mixed subscriber (med + low freq topics)
            let broker_clone = broker.clone();
            handles.push(tokio::spawn(async move {
                let mut subscriber = InMemorySubscriber::new(broker_clone);
                subscriber.subscribe("med_freq_topic").await.unwrap();
                subscriber.subscribe("low_freq_topic").await.unwrap();
                for _ in 0..300 { // 200 + 100
                    subscriber.receive().await.unwrap();
                }
            }));
            
            for handle in handles {
                handle.await.unwrap();
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_multiple_publishers,
    bench_multiple_subscribers,
    bench_publisher_subscriber_pairs,
    bench_topic_scaling,
    bench_mixed_workload
);
criterion_main!(benches);
