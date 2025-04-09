---
layout: docs
title: Installation
---

# Installation

Getting started with Kincir is straightforward. This guide will walk you through the steps to add Kincir to your Rust project.

## Requirements

Kincir requires:

- Rust 1.56 or later
- Cargo (Rust's package manager)
- Depending on which backends you enable, you may need additional system dependencies

## Adding Kincir to Your Project

Add Kincir to your `Cargo.toml` file:

<div class="highlight-wrapper" style="position: relative;" data-language="toml">
{% highlight toml %}
[dependencies]
kincir = "0.1.0"
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

### Enabling Specific Message Broker Backends

Kincir uses feature flags to keep dependencies minimal. Enable only the backends you need:

<div class="highlight-wrapper" style="position: relative;" data-language="toml">
{% highlight toml %}
[dependencies]
kincir = { version = "0.1.0", features = ["kafka", "rabbitmq"] }
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

Available features include:

- `kafka`: Enables Apache Kafka support
- `rabbitmq`: Enables RabbitMQ support
- `redis`: Enables Redis Streams support
- `nats`: Enables NATS support
- `protobuf`: Adds Protocol Buffers serialization support
- `json`: Adds JSON serialization support (enabled by default)
- `tracing`: Enables tracing integration
- `metrics`: Enables metrics collection

## Verifying Installation

To verify that Kincir is correctly installed, create a simple Rust program:

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::prelude::*;

fn main() {
    println!("Kincir version: {}", kincir::VERSION);
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

Compile and run the program:

<div class="highlight-wrapper" style="position: relative;" data-language="bash">
{% highlight bash %}
cargo run
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

If everything is set up correctly, you should see the Kincir version printed to the console.

## System Dependencies

Depending on which message broker backends you enable, you might need to install additional system libraries.

### Kafka Backend

If you're using the Kafka backend (`kafka` feature), you'll need:

- librdkafka 1.8.0 or later
- OpenSSL development libraries (for secure connections)

#### Ubuntu/Debian:

<div class="highlight-wrapper" style="position: relative;" data-language="bash">
{% highlight bash %}
sudo apt-get install librdkafka-dev libssl-dev
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

#### macOS (using Homebrew):

<div class="highlight-wrapper" style="position: relative;" data-language="bash">
{% highlight bash %}
brew install librdkafka openssl
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

### RabbitMQ Backend

If you're using the RabbitMQ backend (`rabbitmq` feature), you'll need:

- AMQP client libraries

#### Ubuntu/Debian:

<div class="highlight-wrapper" style="position: relative;" data-language="bash">
{% highlight bash %}
sudo apt-get install librabbitmq-dev
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

#### macOS (using Homebrew):

<div class="highlight-wrapper" style="position: relative;" data-language="bash">
{% highlight bash %}
brew install rabbitmq-c
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Next Steps

Once you have Kincir installed, you can proceed to the [Quick Start](/docs/quick-start/) guide to learn how to create publishers and subscribers. 