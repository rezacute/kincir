---
layout: docs
title: Configuration
---

# Configuration

Kincir provides flexible configuration options that allow you to customize its behavior for your specific use case. This guide covers the various configuration options available for publishers, subscribers, and routers.

## Publisher Configuration

Each message broker backend in Kincir has its own configuration options for publishers. Here's how to configure the most common backends:

### Kafka Publisher Configuration

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;

// Create a Kafka publisher configuration
let config = KafkaPublisherConfig::new()
    // Required: Bootstrap servers (comma-separated list)
    .bootstrap_servers("localhost:9092")
    
    // Optional: Client ID
    .client_id("my-application")
    
    // Optional: Connection timeout (in milliseconds)
    .connection_timeout(5000)
    
    // Optional: Request timeout (in milliseconds)
    .request_timeout(30000)
    
    // Optional: Set maximum number of in-flight requests
    .max_in_flight_requests(5)
    
    // Optional: Configure compression (none, gzip, snappy, lz4, zstd)
    .compression(CompressionType::Snappy)
    
    // Optional: Set security protocol
    .security_protocol(SecurityProtocol::Ssl)
    
    // Optional: SSL configuration
    .ssl_ca_location("/path/to/ca.pem")
    .ssl_certificate_location("/path/to/cert.pem")
    .ssl_key_location("/path/to/key.pem")
    
    // Build the configuration
    .build()?;
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

### RabbitMQ Publisher Configuration

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;

// Create a RabbitMQ publisher configuration
let config = RabbitMqPublisherConfig::new()
    // Required: Connection URI
    .uri("amqp://guest:guest@localhost:5672/%2f")
    
    // Optional: Exchange name (default is "amq.topic")
    .exchange("my-exchange")
    
    // Optional: Exchange type (direct, fanout, topic, headers)
    .exchange_type(ExchangeType::Topic)
    
    // Optional: Connection timeout (in milliseconds)
    .connection_timeout(5000)
    
    // Optional: Enable publisher confirms
    .enable_publisher_confirms(true)
    
    // Build the configuration
    .build()?;
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Subscriber Configuration

Similar to publishers, each message broker backend has specific configuration options for subscribers:

### Kafka Subscriber Configuration

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;

// Create a Kafka subscriber configuration
let config = KafkaSubscriberConfig::new()
    // Required: Bootstrap servers (comma-separated list)
    .bootstrap_servers("localhost:9092")
    
    // Required: Consumer group ID
    .group_id("my-consumer-group")
    
    // Optional: Auto-commit interval (in milliseconds)
    .auto_commit_interval(5000)
    
    // Optional: Auto offset reset (earliest, latest, none)
    .auto_offset_reset(AutoOffsetReset::Earliest)
    
    // Optional: Max poll interval (in milliseconds)
    .max_poll_interval(300000)
    
    // Optional: Session timeout (in milliseconds)
    .session_timeout(30000)
    
    // Optional: Heartbeat interval (in milliseconds)
    .heartbeat_interval(3000)
    
    // Optional: Set security protocol
    .security_protocol(SecurityProtocol::Ssl)
    
    // Optional: SSL configuration
    .ssl_ca_location("/path/to/ca.pem")
    .ssl_certificate_location("/path/to/cert.pem")
    .ssl_key_location("/path/to/key.pem")
    
    // Build the configuration
    .build()?;
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

### RabbitMQ Subscriber Configuration

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;

// Create a RabbitMQ subscriber configuration
let config = RabbitMqSubscriberConfig::new()
    // Required: Connection URI
    .uri("amqp://guest:guest@localhost:5672/%2f")
    
    // Optional: Exchange name (default is "amq.topic")
    .exchange("my-exchange")
    
    // Optional: Exchange type (direct, fanout, topic, headers)
    .exchange_type(ExchangeType::Topic)
    
    // Optional: Queue name (default is auto-generated)
    .queue("my-queue")
    
    // Optional: Whether to make the queue durable
    .durable(true)
    
    // Optional: Whether to auto-delete the queue
    .auto_delete(false)
    
    // Optional: Prefetch count (max unacknowledged messages)
    .prefetch_count(10)
    
    // Build the configuration
    .build()?;
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Router Configuration

The Kincir router provides several configuration options to customize its behavior:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;

// Create a router with custom configuration
let router = Router::builder()
    // Optional: Set the maximum number of concurrent handlers
    .max_concurrent_handlers(100)
    
    // Optional: Enable message tracing
    .tracing_enabled(true)
    
    // Optional: Set a custom router name (useful for metrics)
    .name("my-application-router")
    
    // Optional: Set the default channel capacity
    .channel_capacity(1000)
    
    // Optional: Configure error handling strategy
    .error_strategy(ErrorStrategy::RetryWithBackoff {
        max_retries: 3,
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(5),
    })
    
    // Build the router
    .build();
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Environment Variables Configuration

Kincir supports configuration through environment variables. This is useful for containerized environments and follows the 12-factor app methodology.

<div class="highlight-wrapper" style="position: relative;" data-language="bash">
{% highlight bash %}
# Kafka configuration
export KINCIR_KAFKA_BOOTSTRAP_SERVERS=localhost:9092
export KINCIR_KAFKA_CLIENT_ID=my-application
export KINCIR_KAFKA_GROUP_ID=my-consumer-group
export KINCIR_KAFKA_AUTO_OFFSET_RESET=earliest

# RabbitMQ configuration
export KINCIR_RABBITMQ_URI=amqp://guest:guest@localhost:5672/%2f
export KINCIR_RABBITMQ_EXCHANGE=my-exchange
export KINCIR_RABBITMQ_QUEUE=my-queue

# General Kincir configuration
export KINCIR_MAX_CONCURRENT_HANDLERS=100
export KINCIR_TRACING_ENABLED=true
export KINCIR_LOG_LEVEL=info
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

You can then load the configuration from environment variables:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;

// Load Kafka publisher configuration from environment variables
let config = KafkaPublisherConfig::from_env()?;

// Load RabbitMQ subscriber configuration from environment variables
let config = RabbitMqSubscriberConfig::from_env()?;

// Load router configuration from environment variables
let router = Router::from_env()?;
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Configuration Files

For more complex configurations, Kincir supports loading configuration from YAML or JSON files:

<div class="highlight-wrapper" style="position: relative;" data-language="yaml">
{% highlight yaml %}
# config.yaml
kafka:
  bootstrap_servers: localhost:9092
  client_id: my-application
  group_id: my-consumer-group
  auto_offset_reset: earliest
  security:
    protocol: ssl
    ssl_ca_location: /path/to/ca.pem
    ssl_certificate_location: /path/to/cert.pem
    ssl_key_location: /path/to/key.pem

rabbitmq:
  uri: amqp://guest:guest@localhost:5672/%2f
  exchange: my-exchange
  queue: my-queue
  prefetch_count: 10

router:
  max_concurrent_handlers: 100
  tracing_enabled: true
  name: my-application-router
  channel_capacity: 1000
  error_strategy:
    type: retry_with_backoff
    max_retries: 3
    initial_backoff_ms: 100
    max_backoff_ms: 5000
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

Loading configuration from a file:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use std::path::Path;

// Load Kafka publisher configuration from a YAML file
let config = KafkaPublisherConfig::from_file(Path::new("config.yaml"))?;

// Load RabbitMQ subscriber configuration from a YAML file
let config = RabbitMqSubscriberConfig::from_file(Path::new("config.yaml"))?;

// Load router configuration from a YAML file
let router = Router::from_file(Path::new("config.yaml"))?;
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Advanced Configuration

### Serialization Configuration

Kincir supports different serialization formats. By default, it uses JSON, but you can configure it to use Protocol Buffers or other formats:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use kincir::serialization::{Serializer, Deserializer};

// Configure a Kafka publisher to use Protocol Buffers
let config = KafkaPublisherConfig::new()
    .bootstrap_servers("localhost:9092")
    .serializer(protobuf::ProtobufSerializer::new())
    .build()?;

// Configure a Kafka subscriber to use Protocol Buffers
let config = KafkaSubscriberConfig::new()
    .bootstrap_servers("localhost:9092")
    .group_id("my-consumer-group")
    .deserializer(protobuf::ProtobufDeserializer::new())
    .build()?;
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

### Metrics Configuration

Kincir provides built-in metrics for monitoring your application:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use kincir::metrics::{MetricsReporter, PrometheusReporter};

// Configure a Kafka publisher with metrics reporting
let config = KafkaPublisherConfig::new()
    .bootstrap_servers("localhost:9092")
    .metrics_reporter(PrometheusReporter::new("my_application"))
    .build()?;

// Configure a router with metrics reporting
let router = Router::builder()
    .metrics_reporter(PrometheusReporter::new("my_application"))
    .build();
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

### Tracing Configuration

Kincir integrates with the tracing ecosystem for observability:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

// Initialize a tracing subscriber
let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::INFO)
    .finish();
tracing::subscriber::set_global_default(subscriber)
    .expect("Failed to set subscriber");

// Configure a router with tracing enabled
let router = Router::builder()
    .tracing_enabled(true)
    .build();
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>



<script>
function copyCode(button) {
  const codeBlock = button.previousElementSibling;
  const code = codeBlock.innerText;
  
  navigator.clipboard.writeText(code).then(function() {
    // Visual feedback
    button.classList.add('copied');
    button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>';
    
    // Reset after 2 seconds
    setTimeout(function() {
      button.classList.remove('copied');
      button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
    }, 2000);
  }).catch(function(err) {
    console.error('Could not copy text: ', err);
  });
}
</script> 