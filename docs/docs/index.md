---
layout: default
title: Documentation
---

<div class="docs-container-page">
  <h1>Kincir Documentation</h1>

  <p class="intro-text">Welcome to the Kincir documentation. Kincir is a unified message streaming library for Rust that provides a consistent interface for working with multiple message broker backends.</p>

  <div class="docs-sections">
    <div class="docs-section">
      <h2>Getting Started</h2>
      <div class="docs-links">
        <a href="{{ '/docs/installation/index' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">📥</div>
          <div class="docs-link-content">
            <h3>Installation</h3>
            <p>Learn how to install Kincir in your Rust project</p>
          </div>
        </a>
        <a href="{{ '/docs/quick-start/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">🚀</div>
          <div class="docs-link-content">
            <h3>Quick Start</h3>
            <p>Start using Kincir in your application quickly</p>
          </div>
        </a>
        <a href="{{ '/docs/configuration/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">⚙️</div>
          <div class="docs-link-content">
            <h3>Configuration</h3>
            <p>Configure Kincir for your specific needs</p>
          </div>
        </a>
      </div>
    </div>

    <div class="docs-section">
      <h2>Core Concepts</h2>
      <div class="docs-links">
        <a href="{{ '/docs/core-concepts/publishers/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">📢</div>
          <div class="docs-link-content">
            <h3>Publishers</h3>
            <p>Learn about message publishing</p>
          </div>
        </a>
        <a href="{{ '/docs/core-concepts/subscribers/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">👂</div>
          <div class="docs-link-content">
            <h3>Subscribers</h3>
            <p>Learn about message subscription</p>
          </div>
        </a>
        <a href="{{ '/docs/core-concepts/routing/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">🔄</div>
          <div class="docs-link-content">
            <h3>Message Routing</h3>
            <p>Understand how messages are routed</p>
          </div>
        </a>
        <a href="{{ '/docs/core-concepts/error-handling/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">⚠️</div>
          <div class="docs-link-content">
            <h3>Error Handling</h3>
            <p>Handle errors in your message processing</p>
          </div>
        </a>
      </div>
    </div>

    <div class="docs-section">
      <h2>Message Broker Backends</h2>
      <div class="docs-links">
        <a href="{{ '/docs/backends/in-memory/' | relative_url }}" class="docs-link featured-link">
          <div class="docs-link-icon">⚡</div>
          <div class="docs-link-content">
            <h3>In-Memory Broker <span class="new-badge">NEW</span></h3>
            <p>Zero-dependency, high-performance message broker for testing and lightweight production</p>
          </div>
        </a>
        <a href="{{ '/docs/backends/kafka/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">🔗</div>
          <div class="docs-link-content">
            <h3>Kafka</h3>
            <p>Using Kincir with Apache Kafka</p>
          </div>
        </a>
        <a href="{{ '/docs/backends/rabbitmq/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">🐇</div>
          <div class="docs-link-content">
            <h3>RabbitMQ</h3>
            <p>Using Kincir with RabbitMQ</p>
          </div>
        </a>
 
      </div>
    </div>

    <div class="docs-section">
      <h2>Advanced Usage</h2>
      <div class="docs-links">
        <a href="{{ '/docs/advanced/middleware/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">🔌</div>
          <div class="docs-link-content">
            <h3>Middleware</h3>
            <p>Add custom behavior to your message processing</p>
          </div>
        </a>
        <a href="{{ '/docs/advanced/testing/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">🧪</div>
          <div class="docs-link-content">
            <h3>Testing</h3>
            <p>Test your Kincir applications</p>
          </div>
        </a>
        <a href="{{ '/docs/advanced/performance/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">⚡</div>
          <div class="docs-link-content">
            <h3>Performance Tuning</h3>
            <p>Optimize Kincir for performance</p>
          </div>
        </a>
        <a href="{{ '/docs/advanced/deployment/' | relative_url }}" class="docs-link">
          <div class="docs-link-icon">🚀</div>
          <div class="docs-link-content">
            <h3>Deployment</h3>
            <p>Deploy Kincir in production</p>
          </div>
        </a>
      </div>
    </div>
  </div>

    <div class="docs-section">
      <h2>MQTT to RabbitMQ Tunnel</h2>
      <p>The `MqttToRabbitMQTunnel` provides a way to forward messages from MQTT topics to a RabbitMQ message broker.</p>
      <h3>Configuration</h3>
      <p>To use the tunnel, you need to set up two configuration structs:</p>
      <ol>
        <li>
          <strong>`MqttTunnelConfig`</strong>:
          <ul>
            <li>`broker_url`: The URL of your MQTT broker (e.g., "mqtt://localhost:1883").</li>
            <li>`topics`: A `Vec&lt;String&gt;` of MQTT topics to subscribe to.</li>
            <li>`qos`: The Quality of Service level (u8) for MQTT subscriptions (0, 1, or 2).</li>
          </ul>
        </li>
        <li>
          <strong>`RabbitMQTunnelConfig`</strong>:
          <ul>
            <li>`uri`: The connection URI for your RabbitMQ instance (e.g., "amqp://guest:guest@localhost:5672/%2f").</li>
            <li>`routing_key`: The RabbitMQ routing key to which messages will be published. This is often the name of a queue if using the default exchange, or a routing key that matches a binding on an exchange.</li>
          </ul>
        </li>
      </ol>
      <h3>Example Usage</h3>
      <div class="highlight-wrapper" style="position: relative;" data-language="rust">
      {% highlight rust %}
          use kincir::tunnel::{MqttTunnelConfig, RabbitMQTunnelConfig, MqttToRabbitMQTunnel};
          use std::env;

          async fn run_tunnel() -> Result<(), Box<dyn std::error::Error>> {
          let mqtt_broker_url = "mqtt://localhost:1883";
          let mqtt_topics = vec!["data/source".to_string()];
          let mqtt_qos = 1;
          let mqtt_config = MqttTunnelConfig::new(&mqtt_broker_url, mqtt_topics, mqtt_qos);

          let rabbitmq_uri = "amqp://localhost:5672";
          let rabbitmq_routing_key = "iot_data_queue";
          let rabbitmq_config = RabbitMQTunnelConfig::new(&rabbitmq_uri, &rabbitmq_routing_key);

          let mut tunnel = MqttToRabbitMQTunnel::new(mqtt_config, rabbitmq_config);

          if let Err(e) = tunnel.run().await {
              eprintln!("Tunnel encountered an error: {}", e);
          }
          Ok(())
          }
      {% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>
      <p>For a complete runnable example, please see the `examples/mqtt-to-rabbitmq-example` directory in the repository.</p>
      <h3>Error Handling</h3>
      <p>The `run` method of the tunnel returns a `ResultVec&lt;(), TunnelError&gt;`. You should handle potential errors such as connection issues, configuration problems, or runtime errors during message processing.</p>
    </div>

  <div class="docs-footer">

  </div>
</div>