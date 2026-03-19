---
layout: default
title: Middleware
nav_order: 5
has_children: true
---

# Middleware Framework
{: .no_toc }

Kincir v0.3 introduces a middleware framework that allows you to intercept and modify message processing operations. Middleware can be used for logging, retries, correlation tracking, and more.

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Quick Start

```rust
use kincir::middleware::{Middleware, MiddlewareChain, LoggingMiddleware, CorrelationMiddleware};

let chain = MiddlewareChain::new()
    .add(LoggingMiddleware::new("MyApp"))
    .add(CorrelationMiddleware::new());

// Use with message operations
let ctx = MiddlewareContext::new("orders");
let mut messages = vec![Message::new(b"Order #1".to_vec())];

chain.before_publish(&ctx, &mut messages).await;
```

## Middleware Trait

The core of the framework is the `Middleware` trait:

```rust
use kincir::middleware::Middleware;

#[async_trait]
pub trait Middleware: Send + Sync {
    async fn before_publish(&self, context: &MiddlewareContext, messages: &mut [Message]) {}
    async fn after_publish(&self, context: &MiddlewareContext, messages: &[Message]) {}
    async fn before_subscribe(&self, context: &MiddlewareContext) {}
    async fn after_subscribe(&self, context: &MiddlewareContext) {}
    async fn before_receive(&self, context: &MiddlewareContext) {}
    async fn after_receive(&self, context: &MiddlewareContext, message: &Message) {}
}
```

## Available Middleware

### LoggingMiddleware

Logs all message operations for debugging and monitoring.

```rust
use kincir::middleware::LoggingMiddleware;

let mw = LoggingMiddleware::new("MyApp")
    .with_payload()    // Include message payload in logs
    .with_metadata(); // Include metadata in logs
```

### CorrelationMiddleware

Adds correlation IDs to messages for distributed tracing.

```rust
use kincir::middleware::CorrelationMiddleware;

let mw = CorrelationMiddleware::new()
    .with_header_key("x-correlation-id"); // Custom header key

// Messages get a unique correlation ID
mw.before_publish(&ctx, &mut messages).await;
// messages[0].metadata["correlation-id"] is now set
```

### RetryMiddleware

Configures retry behavior for failed operations.

```rust
use kincir::middleware::RetryMiddleware;

let mw = RetryMiddleware::new(5)  // 5 retries
    .with_delay(200);             // 200ms base delay

// Supports exponential backoff
mw.calculate_delay(0); // 200ms
mw.calculate_delay(1); // 400ms  
mw.calculate_delay(2); // 800ms
```

## MiddlewareChain

Compose multiple middleware into a chain:

```rust
use kincir::middleware::MiddlewareChain;

let chain = MiddlewareChain::new()
    .add(LoggingMiddleware::new("App"))
    .add(CorrelationMiddleware::new())
    .add(RetryMiddleware::new(3));

// All middleware in chain are executed in order
chain.before_publish(&ctx, &mut messages).await;
```

## MiddlewareContext

Provides context to middleware:

```rust
use kincir::middleware::MiddlewareContext;

let ctx = MiddlewareContext::new("orders")
    .with_metadata("source", "api");
```

## Use Cases

### Logging All Messages

```rust
let logging = LoggingMiddleware::new("Orders")
    .with_payload()
    .with_metadata();

let chain = MiddlewareChain::new().add(logging);
```

### Distributed Tracing

```rust
let correlation = CorrelationMiddleware::new()
    .with_header_key("x-request-id");

let chain = MiddlewareChain::new().add(correlation);
```

### Retry with Logging

```rust
let chain = MiddlewareChain::new()
    .add(LoggingMiddleware::new("RetryApp"))
    .add(RetryMiddleware::new(5));
```

## Custom Middleware

Create your own middleware:

```rust
use kincir::middleware::{Middleware, MiddlewareContext};
use kincir::Message;
use async_trait::async_trait;

pub struct CustomMiddleware;

#[async_trait]
impl Middleware for CustomMiddleware {
    async fn before_publish(&self, context: &MiddlewareContext, messages: &mut [Message]) {
        // Your custom logic here
    }
}
```
