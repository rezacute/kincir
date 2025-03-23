---
layout: docs
title: Kafka Backend
---

# Kafka Backend

This guide explains how to use Kincir with Apache Kafka as a message broker backend.

## System Dependencies

If you're using the Kafka backend (`kafka` feature), you'll need:

- librdkafka 1.8.0 or later
- OpenSSL development libraries (for secure connections)

### Ubuntu/Debian:

<div class="highlight-wrapper" style="position: relative;" data-language="bash">
{% highlight bash %}
sudo apt-get install librdkafka-dev libssl-dev
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

### macOS (using Homebrew):

<div class="highlight-wrapper" style="position: relative;" data-language="bash">
{% highlight bash %}
brew install librdkafka openssl
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Configuration

To use Kafka with Kincir, you need to enable the `kafka` feature in your `Cargo.toml`:

<div class="highlight-wrapper" style="position: relative;" data-language="toml">
{% highlight toml %}
[dependencies]
kincir = { version = "0.1.0", features = ["kafka"] }
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Publisher Example

Here's an example of publishing messages to a Kafka topic:

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

## Subscriber Example

Here's how to create a Kafka subscriber:

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

## Advanced Configuration

Kincir allows you to set additional Kafka-specific configuration options:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a Kafka publisher with advanced configuration
    let config = KafkaPublisherConfig::new()
        .bootstrap_servers("kafka1:9092,kafka2:9092,kafka3:9092")
        .client_id("advanced-publisher")
        .message_timeout(Duration::from_secs(30))
        .security_protocol(SecurityProtocol::Ssl)
        .ssl_ca_location("/path/to/ca.pem")
        .ssl_certificate_location("/path/to/cert.pem")
        .ssl_key_location("/path/to/key.pem")
        .build()?;
    
    let publisher = KafkaPublisher::new(config)?;
    
    // Use the publisher...
    
    Ok(())
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Error Handling

Kincir provides specific error types for Kafka-related errors:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use kincir::kafka::error::KafkaError;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = KafkaPublisherConfig::new()
        .bootstrap_servers("localhost:9092")
        .build()?;
    
    let publisher = match KafkaPublisher::new(config) {
        Ok(publisher) => publisher,
        Err(err) => {
            if let Some(kafka_err) = err.downcast_ref::<KafkaError>() {
                match kafka_err {
                    KafkaError::Configuration(conf_err) => {
                        eprintln!("Kafka configuration error: {}", conf_err);
                    },
                    KafkaError::Connection(conn_err) => {
                        eprintln!("Kafka connection error: {}", conn_err);
                    },
                    // Handle other specific errors
                    _ => eprintln!("Other Kafka error: {}", kafka_err),
                }
            }
            return Err(err);
        }
    };
    
    // Use the publisher...
    
    Ok(())
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Next Steps

Learn more about other available backends in Kincir:

- [RabbitMQ Backend](/docs/backends/rabbitmq/) 