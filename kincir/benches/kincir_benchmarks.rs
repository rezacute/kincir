use criterion::{criterion_group, criterion_main, Criterion};
use kincir::Message;
use std::time::Duration;

// Benchmark message creation
fn bench_message_creation(c: &mut Criterion) {
    c.bench_function("message_creation", |b| {
        b.iter(|| {
            let payload = b"Hello, World!".to_vec();
            let _message = Message::new(payload);
        })
    });
}

// Benchmark message with metadata
fn bench_message_with_metadata(c: &mut Criterion) {
    c.bench_function("message_with_metadata", |b| {
        b.iter(|| {
            let payload = b"Hello, World!".to_vec();
            let _message = Message::new(payload)
                .with_metadata("content-type", "text/plain")
                .with_metadata("priority", "high");
        })
    });
}

// Benchmark message serialization
fn bench_message_serialization(c: &mut Criterion) {
    let payload = b"Hello, World!".to_vec();
    let message = Message::new(payload).with_metadata("content-type", "text/plain");

    c.bench_function("message_serialization", |b| {
        b.iter(|| {
            let _serialized = bincode::serialize(&message).unwrap();
        })
    });
}

// Benchmark message deserialization
fn bench_message_deserialization(c: &mut Criterion) {
    let payload = b"Hello, World!".to_vec();
    let message = Message::new(payload).with_metadata("content-type", "text/plain");
    let serialized = bincode::serialize(&message).unwrap();

    c.bench_function("message_deserialization", |b| {
        b.iter(|| {
            let _deserialized: Message = bincode::deserialize(&serialized).unwrap();
        })
    });
}

// Configure the benchmark group
fn configure_benchmarks() -> Criterion {
    Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(5))
        .configure_from_args()
}

criterion_group! {
    name = benches;
    config = configure_benchmarks();
    targets = bench_message_creation, bench_message_with_metadata,
              bench_message_serialization, bench_message_deserialization
}
criterion_main!(benches);
