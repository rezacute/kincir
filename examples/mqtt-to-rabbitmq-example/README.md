# MQTT to RabbitMQ Tunnel Example

This example demonstrates how to use the `MqttToRabbitMQTunnel` to forward messages from MQTT topics to a RabbitMQ queue/routing key.

## Prerequisites

- Rust toolchain
- Docker (optional, for running local MQTT and RabbitMQ brokers)

## Setup

1.  **Clone the repository.**
2.  **Navigate to this example's directory:**
    ```bash
    cd examples/mqtt-to-rabbitmq-example
    ```
3.  **Create a `.env` file** by copying the `.env_example`:
    ```bash
    cp .env_example .env
    ```
4.  **Edit the `.env` file** to configure your MQTT broker, topics, RabbitMQ URI, and routing key.

## Running the Example

### With Local Brokers (Docker Compose)

If you want to run local MQTT (Mosquitto) and RabbitMQ brokers using Docker:

1.  **Ensure Docker and Docker Compose are installed.**
2.  **Start the services:**
    From the `examples/mqtt-to-rabbitmq-example` directory:
    ```bash
    docker-compose up -d
    ```
    This will start Mosquitto on port 1883 and RabbitMQ on port 5672 (with management UI on 15672).
3.  **Run the example:**
    ```bash
    cargo run
    ```
4.  **To stop the local brokers:**
    ```bash
    docker-compose down
    ```

### With Existing Brokers

If you have existing MQTT and RabbitMQ brokers:

1.  **Ensure your `.env` file is configured to point to your brokers.**
2.  **Run the example:**
    ```bash
    cargo run
    ```

## How it Works

The application reads MQTT and RabbitMQ connection details from the `.env` file. It subscribes to the specified MQTT topics and forwards any incoming messages to the specified RabbitMQ routing key.

Check the console output for logs from the application. You can use tools like `mqttui` or `MQTT Explorer` to publish messages to the MQTT topics and RabbitMQ management plugin or `amqp-utils` to check messages in the RabbitMQ queue.
