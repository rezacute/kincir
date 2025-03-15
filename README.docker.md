# Docker Support for Kincir

This document explains how to use Docker with the Kincir project.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)

## Getting Started

The project includes Docker support with a multi-service setup that includes:

- Kincir application
- Kafka and Zookeeper
- RabbitMQ
- Example services for both Kafka and RabbitMQ

## Basic Usage

### Building and Starting the Services

To build and start all services:

```bash
docker-compose up -d
```

This will start Kafka, Zookeeper, and RabbitMQ services, but not the example applications.

### Running the Kafka Example

To run the Kafka example:

```bash
docker-compose --profile kafka-demo up -d
```

This will start the Kafka example service along with its dependencies.

### Running the RabbitMQ Example

To run the RabbitMQ example:

```bash
docker-compose --profile rabbitmq-demo up -d
```

This will start the RabbitMQ example service along with its dependencies.

### Running Both Examples

To run both examples:

```bash
docker-compose --profile kafka-demo --profile rabbitmq-demo up -d
```

## Service Access

- **Kafka**: Available at `kafka:9092` within the Docker network, and at `localhost:9092` from the host
- **RabbitMQ**: Available at `rabbitmq:5672` within the Docker network, and at `localhost:5672` from the host
- **RabbitMQ Management UI**: Available at `http://localhost:15672` (username: guest, password: guest)

## Environment Variables

The example applications support the following environment variables:

- **Kafka Example**:
  - `KAFKA_BROKER`: Kafka broker address (default: `kafka:9092`)
  - `RUST_LOG`: Logging level (default: `info`)

- **RabbitMQ Example**:
  - `RABBITMQ_URL`: RabbitMQ connection URL (default: `amqp://guest:guest@rabbitmq:5672`)
  - `RUST_LOG`: Logging level (default: `info`)

## Custom Configuration

You can modify the `docker-compose.yml` file to customize the configuration of each service.

## Stopping the Services

To stop all services:

```bash
docker-compose down
```

To stop all services and remove volumes:

```bash
docker-compose down -v
```

## Troubleshooting

### Checking Logs

To check logs for a specific service:

```bash
docker-compose logs [service-name]
```

For example:

```bash
docker-compose logs kafka-example
```

### Connecting to a Container

To connect to a running container:

```bash
docker-compose exec [service-name] /bin/bash
```

For example:

```bash
docker-compose exec kincir /bin/bash
``` 