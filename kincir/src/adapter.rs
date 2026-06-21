//! Adapters that bridge concrete-error backends to the trait-object interfaces
//! used by [`Router`](crate::router::Router).
//!
//! [`Router`](crate::router::Router) requires its publisher and subscriber to
//! expose `Box<dyn std::error::Error + Send + Sync>` as their associated `Error`
//! type. The RabbitMQ, Kafka and MQTT backends already use this boxed error
//! type, but backends with a concrete error type — most notably the in-memory
//! broker, whose error is [`InMemoryError`](crate::memory::InMemoryError) — do
//! not. [`BoxedPublisher`] and [`BoxedSubscriber`] wrap such backends so they
//! can be used with the router (or anywhere a uniform boxed error is required).
//!
//! # Example
//!
//! ```
//! use std::sync::Arc;
//! use kincir::adapter::{PublisherExt, SubscriberExt};
//! use kincir::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
//!
//! let broker = Arc::new(InMemoryBroker::with_default_config());
//!
//! // `InMemoryPublisher::Error` is `InMemoryError`; `.boxed()` turns it into a
//! // publisher whose error is `Box<dyn std::error::Error + Send + Sync>`.
//! let publisher = InMemoryPublisher::new(broker.clone()).boxed();
//! let subscriber = InMemorySubscriber::new(broker).boxed();
//!
//! // `publisher` / `subscriber` are now compatible with `kincir::router::Router`.
//! # let _ = (publisher, subscriber);
//! ```

use crate::{Message, Publisher, Subscriber};
use async_trait::async_trait;

/// The boxed error type expected by [`Router`](crate::router::Router).
pub type BoxedError = Box<dyn std::error::Error + Send + Sync>;

/// Wraps a [`Publisher`] with a concrete error type and re-exposes it with the
/// boxed error type [`BoxedError`].
///
/// Construct one directly with [`BoxedPublisher::new`] or ergonomically via
/// [`PublisherExt::boxed`].
pub struct BoxedPublisher<P> {
    inner: P,
}

impl<P> BoxedPublisher<P> {
    /// Wrap a publisher so its error is boxed.
    pub fn new(inner: P) -> Self {
        Self { inner }
    }

    /// Get a reference to the wrapped publisher.
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// Consume the wrapper and return the inner publisher.
    pub fn into_inner(self) -> P {
        self.inner
    }
}

#[async_trait]
impl<P> Publisher for BoxedPublisher<P>
where
    P: Publisher + Send + Sync,
    P::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = BoxedError;

    async fn publish(&self, topic: &str, messages: Vec<Message>) -> Result<(), Self::Error> {
        self.inner
            .publish(topic, messages)
            .await
            .map_err(|e| Box::new(e) as BoxedError)
    }
}

/// Wraps a [`Subscriber`] with a concrete error type and re-exposes it with the
/// boxed error type [`BoxedError`].
///
/// Construct one directly with [`BoxedSubscriber::new`] or ergonomically via
/// [`SubscriberExt::boxed`].
pub struct BoxedSubscriber<S> {
    inner: S,
}

impl<S> BoxedSubscriber<S> {
    /// Wrap a subscriber so its error is boxed.
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    /// Get a reference to the wrapped subscriber.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Get a mutable reference to the wrapped subscriber.
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consume the wrapper and return the inner subscriber.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

#[async_trait]
impl<S> Subscriber for BoxedSubscriber<S>
where
    S: Subscriber + Send + Sync,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = BoxedError;

    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error> {
        self.inner
            .subscribe(topic)
            .await
            .map_err(|e| Box::new(e) as BoxedError)
    }

    async fn receive(&mut self) -> Result<Message, Self::Error> {
        self.inner
            .receive()
            .await
            .map_err(|e| Box::new(e) as BoxedError)
    }
}

/// Extension trait adding [`boxed`](PublisherExt::boxed) to every [`Publisher`].
pub trait PublisherExt: Publisher + Sized {
    /// Wrap this publisher in a [`BoxedPublisher`] so its error becomes
    /// `Box<dyn std::error::Error + Send + Sync>`.
    fn boxed(self) -> BoxedPublisher<Self> {
        BoxedPublisher::new(self)
    }
}

impl<P: Publisher> PublisherExt for P {}

/// Extension trait adding [`boxed`](SubscriberExt::boxed) to every [`Subscriber`].
pub trait SubscriberExt: Subscriber + Sized {
    /// Wrap this subscriber in a [`BoxedSubscriber`] so its error becomes
    /// `Box<dyn std::error::Error + Send + Sync>`.
    fn boxed(self) -> BoxedSubscriber<Self> {
        BoxedSubscriber::new(self)
    }
}

impl<S: Subscriber> SubscriberExt for S {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{InMemoryBroker, InMemoryPublisher, InMemorySubscriber};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_boxed_publisher_subscriber_roundtrip() {
        let broker = Arc::new(InMemoryBroker::with_default_config());

        let publisher = InMemoryPublisher::new(broker.clone()).boxed();
        let mut subscriber = InMemorySubscriber::new(broker.clone()).boxed();

        subscriber.subscribe("test-topic").await.unwrap();

        let message = Message::new(b"hello".to_vec());
        publisher
            .publish("test-topic", vec![message.clone()])
            .await
            .unwrap();

        let received = subscriber.receive().await.unwrap();
        assert_eq!(received.payload, message.payload);
    }

    #[tokio::test]
    async fn test_boxed_types_satisfy_router_bounds() {
        // This compiles only if the boxed wrappers expose the boxed error type
        // expected by the router's trait objects.
        let broker = Arc::new(InMemoryBroker::with_default_config());

        let publisher: Arc<dyn Publisher<Error = BoxedError>> =
            Arc::new(InMemoryPublisher::new(broker.clone()).boxed());
        let subscriber: Arc<
            tokio::sync::Mutex<dyn Subscriber<Error = BoxedError> + Send + Sync>,
        > = Arc::new(tokio::sync::Mutex::new(
            InMemorySubscriber::new(broker).boxed(),
        ));

        let _ = (publisher, subscriber);
    }

    #[tokio::test]
    async fn test_into_inner() {
        let broker = Arc::new(InMemoryBroker::with_default_config());
        let wrapped = InMemoryPublisher::new(broker).boxed();
        let _inner: InMemoryPublisher = wrapped.into_inner();
    }
}
