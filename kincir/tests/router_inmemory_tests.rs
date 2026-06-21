//! End-to-end test demonstrating that the in-memory broker can be used with
//! `kincir::router::Router` via the boxed-error adapters.
//!
//! Previously the in-memory backend could not be routed because its associated
//! `Error` type (`InMemoryError`) did not match the `Box<dyn Error + Send + Sync>`
//! required by `Router`. The `PublisherExt::boxed` / `SubscriberExt::boxed`
//! adapters bridge that gap.

use kincir::adapter::{PublisherExt, SubscriberExt};
use kincir::logging::NoOpLogger;
use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
use kincir::router::{HandlerFunc, Router};
use kincir::{Message, Publisher, Subscriber};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

#[tokio::test]
async fn test_router_with_in_memory_backend() {
    let broker = Arc::new(InMemoryBroker::with_default_config());

    // Router input/output backends, adapted to the boxed error type.
    let router_subscriber = Arc::new(Mutex::new(
        InMemorySubscriber::new(broker.clone()).boxed(),
    ));
    let router_publisher = Arc::new(InMemoryPublisher::new(broker.clone()).boxed());

    // Handler appends a marker to the payload.
    let handler: HandlerFunc = Arc::new(|msg: Message| {
        Box::pin(async move {
            let mut payload = msg.payload.clone();
            payload.extend_from_slice(b"-routed");
            Ok(vec![Message::new(payload).with_metadata("routed", "true")])
        })
    });

    let router = Router::new(
        Arc::new(NoOpLogger),
        "input".to_string(),
        "output".to_string(),
        router_subscriber,
        router_publisher,
        handler,
    );

    // An external consumer of the routed output. Subscribe BEFORE the router
    // publishes anything to "output".
    let mut output_consumer = InMemorySubscriber::new(broker.clone());
    output_consumer.subscribe("output").await.unwrap();

    let input_publisher = InMemoryPublisher::new(broker.clone());

    // The router's `run()` loop is infinite, so we drive it concurrently with a
    // driver future on the same task (the router is not `Send`, so it cannot be
    // spawned) and finish as soon as the driver collects the routed messages.
    let driver = async {
        // Give the router time to subscribe to "input" before we publish to it.
        sleep(Duration::from_millis(150)).await;

        input_publisher
            .publish(
                "input",
                vec![
                    Message::new(b"alpha".to_vec()),
                    Message::new(b"beta".to_vec()),
                ],
            )
            .await
            .unwrap();

        let first = output_consumer.receive().await.unwrap();
        let second = output_consumer.receive().await.unwrap();
        (first, second)
    };

    let (first, second) = timeout(Duration::from_secs(10), async {
        tokio::select! {
            result = driver => result,
            run_result = router.run() => panic!("router exited unexpectedly: {:?}", run_result),
        }
    })
    .await
    .expect("timed out waiting for routed messages");

    let mut payloads = vec![
        String::from_utf8_lossy(&first.payload).to_string(),
        String::from_utf8_lossy(&second.payload).to_string(),
    ];
    payloads.sort();

    assert_eq!(payloads, vec!["alpha-routed", "beta-routed"]);
    assert_eq!(first.metadata.get("routed"), Some(&"true".to_string()));
}
