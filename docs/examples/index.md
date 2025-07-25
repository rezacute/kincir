# Kincir Examples

This section contains practical examples demonstrating various Kincir features and use cases.

## Basic Examples

### [In-Memory Broker Example](/examples/in-memory.html)
Learn how to use the high-performance in-memory broker for testing and lightweight production scenarios.

### [RabbitMQ Integration](/examples/rabbitmq.html)
Complete example of integrating Kincir with RabbitMQ for reliable message queuing.

### [Kafka Integration](/examples/kafka.html)
High-throughput message streaming with Apache Kafka backend.

### [MQTT IoT Example](/examples/mqtt.html)
IoT device communication using MQTT with Quality of Service levels.

## Advanced Examples

### [Message Acknowledgments](/examples/acknowledgments.html)
Reliable message processing with acknowledgment patterns across different backends.

### [Message Routing](/examples/routing.html)
Advanced message routing with custom handlers and middleware.

### [Error Handling](/examples/error-handling.html)
Comprehensive error handling strategies for production applications.

### [Performance Optimization](/examples/performance.html)
Tips and techniques for optimizing Kincir performance in high-load scenarios.

## Real-World Use Cases

### [Microservices Communication](/examples/microservices.html)
Using Kincir for inter-service communication in a microservices architecture.

### [Event Sourcing](/examples/event-sourcing.html)
Implementing event sourcing patterns with Kincir's message streaming capabilities.

### [CQRS Implementation](/examples/cqrs.html)
Command Query Responsibility Segregation using Kincir for command and event handling.

### [IoT Data Pipeline](/examples/iot-pipeline.html)
Building a complete IoT data processing pipeline with MQTT and message routing.

## Integration Examples

### [Web Application Integration](/examples/web-integration.html)
Integrating Kincir with web frameworks like Axum, Warp, and Actix-web.

### [Database Integration](/examples/database.html)
Combining Kincir with databases for event-driven data processing.

### [Monitoring and Observability](/examples/monitoring.html)
Adding monitoring, metrics, and distributed tracing to Kincir applications.

## Testing Examples

### [Unit Testing](/examples/unit-testing.html)
Best practices for unit testing Kincir-based applications.

### [Integration Testing](/examples/integration-testing.html)
Testing message flows and broker integrations.

### [Load Testing](/examples/load-testing.html)
Performance testing strategies for Kincir applications.

---

## Running the Examples

All examples are available in the [GitHub repository](https://github.com/rezacute/kincir/tree/main/examples).

To run an example:

```bash
git clone https://github.com/rezacute/kincir.git
cd kincir/examples/basic-usage
cargo run
```

## Prerequisites

Most examples require:
- Rust 1.70 or later
- Tokio async runtime
- Specific broker software (RabbitMQ, Kafka, etc.) for backend examples

## Contributing Examples

We welcome contributions of new examples! Please see our [Contributing Guide](https://github.com/rezacute/kincir/blob/main/CONTRIBUTING.md) for guidelines.
