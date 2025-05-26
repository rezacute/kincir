# MQTT to Kafka Tunnel Example

This example demonstrates the Kincir library's MQTT to Kafka tunneling feature. It sets up a tunnel that subscribes to specified MQTT topics and republishes any received messages to a specified Kafka topic.

## Prerequisites

*   Rust (latest stable, as per the main project's requirements, e.g., 1.87.0)
*   Docker and Docker Compose (for running local MQTT and Kafka brokers)
*   An MQTT client (e.g., `mosquitto_pub`, `mosquitto_sub`, or an GUI client like MQTT Explorer) for testing.
*   A Kafka client (e.g., `kcat` or a Kafka tool) for verifying messages in Kafka (optional).

## Setup

1.  **Clone the Repository:**
    If you haven't already, clone the main Kincir repository. This example is located in the `examples/mqtt-to-kafka-example` directory.

2.  **Start Local Brokers:**
    Navigate to this example's directory (`examples/mqtt-to-kafka-example`) and start the local Mosquitto MQTT broker and Kafka cluster using Docker Compose:
    ```bash
    cd examples/mqtt-to-kafka-example
    docker-compose up -d
    ```
    This will start:
    *   Mosquitto MQTT broker on `localhost:1883`.
    *   Kafka broker accessible externally on `localhost:29092`.
    *   Zookeeper on `localhost:2181` (used by Kafka).

3.  **Configure Environment Variables:**
    Copy the example environment file to `.env`:
    ```bash
    cp .env_example .env
    ```
    Review and edit the `.env` file if you need to change default configurations:
    *   `MQTT_BROKER_URL`: URL of the MQTT broker. (Default: `mqtt://localhost:1883`)
    *   `MQTT_TOPICS`: Comma-separated list of MQTT topics to subscribe to. (Default: `source/topic,another/source,kincir/test`)
    *   `MQTT_QOS`: Quality of Service level for MQTT subscription (0, 1, or 2). (Default: `1`)
    *   `KAFKA_BROKER_URLS`: Comma-separated list of Kafka broker URLs. (Default: `localhost:29092` for the Docker Compose setup)
    *   `KAFKA_TOPIC`: The target Kafka topic to publish messages to. (Default: `destination_topic`)
    *   `RUST_LOG`: Logging configuration. (Default: `info,kincir=debug,mqtt_to_kafka_example=info`)

## Running the Example

1.  **Run the Tunnel Application:**
    From the root of the Kincir repository, run the example using Cargo:
    ```bash
    cargo run --example mqtt-to-kafka-example
    ```
    The application will connect to the configured MQTT and Kafka brokers and start forwarding messages. You should see log output in your terminal.

## Testing the Tunnel

1.  **Publish an MQTT Message:**
    Use an MQTT client to publish a message to one of the topics specified in `MQTT_TOPICS` (e.g., `source/topic`).

    Using `mosquitto_pub` (if installed):
    ```bash
    mosquitto_pub -h localhost -p 1883 -t "source/topic" -m "Hello from MQTT to Kafka Tunnel!"
    ```
    Or, for another configured topic:
    ```bash
    mosquitto_pub -h localhost -p 1883 -t "kincir/test" -m "Another test message"
    ```

2.  **Observe Logs:**
    You should see log output from the `mqtt-to-kafka-example` application indicating it received the message from MQTT and attempted to publish it to Kafka.

3.  **Verify Message in Kafka (Optional):**
    If you have a Kafka client, you can consume messages from the `KAFKA_TOPIC` (e.g., `destination_topic`) to verify the message was tunneled correctly.

    Using `kcat` (formerly `kafkacat`):
    ```bash
    kcat -b localhost:29092 -t destination_topic -C -e
    ```
    This will consume and print messages from the topic. You should see the message you published via MQTT.

## Stopping the Example and Brokers

*   To stop the tunnel application, press `Ctrl+C` in the terminal where it's running.
*   To stop and remove the Docker Compose services (MQTT and Kafka brokers):
    ```bash
    cd examples/mqtt-to-kafka-example
    docker-compose down
    ```
    If you also want to remove the persistent data volumes, use:
    ```bash
    docker-compose down -v
    ```

This example provides a basic demonstration. For production use, consider more robust error handling, reconnection strategies, security configurations, and monitoring.
