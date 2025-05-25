# Contributing to Kincir

Welcome to Kincir! We're excited that you're interested in contributing. This guide provides instructions to help you get started with the development setup and contribution process.

## Build and Development

### Using Make

The project includes a Makefile to simplify common development tasks:

```bash
# Build the project
make build

# Run tests
make test

# Format code and run linters
make verify

# Generate documentation
make docs

# Run benchmarks
make bench

# Show all available commands
make help
```

### Using Docker

The project includes Docker support for development and testing:

```bash
# Start the Docker environment
./scripts/docker_env.sh start

# Run the Kafka example
./scripts/docker_env.sh kafka

# Run the RabbitMQ example
./scripts/docker_env.sh rabbitmq

# Show all available commands
./scripts/docker_env.sh help
```

For more details on Docker usage, see [README.docker.md](README.docker.md).
