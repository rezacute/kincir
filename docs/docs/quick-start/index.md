---
layout: docs
title: Quick Start
---

# Quick Start Guide

This guide will walk you through creating a simple messaging application using Kincir. We'll cover how to create both a publisher and a subscriber.

## Basic Setup

First, make sure you have added Kincir to your project's dependencies as described in the [Installation](/docs/installation/index.md) guide.

## Creating a Publisher

Here's a simple example of creating a publisher that sends messages to a Kafka topic:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a Kafka publisher configuration
    let config = KafkaPublisherConfig::new()
        .bootstrap_servers("localhost:9092")
        .client_id("my-publisher")
        .build()?;
    
    // Create a publisher instance
    let publisher = KafkaPublisher::new(config)?;
    
    // Create a simple message
    let message = Message::new()
        .topic("my-topic")
        .key("user-123")
        .payload("Hello, Kincir!");
    
    // Publish the message
    let result = publisher.publish(message).await?;
    
    println!("Message published successfully: {:?}", result);
    
    Ok(())
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Creating a Subscriber

Now, let's create a subscriber that listens for messages on the same topic:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a Kafka subscriber configuration
    let config = KafkaSubscriberConfig::new()
        .bootstrap_servers("localhost:9092")
        .group_id("my-consumer-group")
        .auto_offset_reset(AutoOffsetReset::Earliest)
        .build()?;
    
    // Create a subscriber instance
    let subscriber = KafkaSubscriber::new(config)?;
    
    // Subscribe to a topic
    subscriber.subscribe("my-topic")?;
    
    println!("Waiting for messages...");
    
    // Process messages
    subscriber.start(|message| {
        println!("Received message: {:?}", message);
        
        // Return Ok to acknowledge the message
        Ok(())
    }).await?;
    
    Ok(())
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Using the Message Router

For more complex applications, you might want to use Kincir's router to handle different types of messages:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a Kafka subscriber configuration
    let config = KafkaSubscriberConfig::new()
        .bootstrap_servers("localhost:9092")
        .group_id("my-router-group")
        .build()?;
    
    // Create a subscriber instance
    let subscriber = KafkaSubscriber::new(config)?;
    
    // Create a router
    let mut router = Router::new();
    
    // Add routes based on topic patterns
    router.add_route(
        TopicMatcher::exact("user-created"),
        |message| {
            println!("New user created: {:?}", message);
            Ok(())
        },
    );
    
    router.add_route(
        TopicMatcher::prefix("order-"),
        |message| {
            println!("Order event received: {:?}", message);
            Ok(())
        },
    );
    
    // Subscribe to multiple topics
    subscriber.subscribe("user-created")?;
    subscriber.subscribe("order-created")?;
    subscriber.subscribe("order-updated")?;
    
    println!("Router started. Waiting for messages...");
    
    // Start the router with the subscriber
    router.start(subscriber).await?;
    
    Ok(())
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Running Multiple Subscribers

You can use Kincir to manage multiple subscribers concurrently:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use std::error::Error;
use tokio::join;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create subscribers for different topics
    let user_subscriber = create_subscriber("user-group", "user-events")?;
    let order_subscriber = create_subscriber("order-group", "order-events")?;
    
    // Start both subscribers concurrently
    let (user_result, order_result) = join!(
        user_subscriber.start(|msg| {
            println!("User event: {:?}", msg);
            Ok(())
        }),
        order_subscriber.start(|msg| {
            println!("Order event: {:?}", msg);
            Ok(())
        })
    );
    
    user_result?;
    order_result?;
    
    Ok(())
}

// Helper function to create a subscriber
fn create_subscriber(group: &str, topic: &str) -> Result<KafkaSubscriber, Box<dyn Error>> {
    let config = KafkaSubscriberConfig::new()
        .bootstrap_servers("localhost:9092")
        .group_id(group)
        .build()?;
    
    let subscriber = KafkaSubscriber::new(config)?;
    subscriber.subscribe(topic)?;
    
    Ok(subscriber)
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Next Steps

Now that you've seen the basics of using Kincir, you can explore more advanced topics:

- [Configuration options](/docs/configuration/)
- [Message routing in depth](/docs/core-concepts/routing/)
- [Working with different broker backends](/docs/backends/)
- [Error handling strategies](/docs/core-concepts/error-handling/)