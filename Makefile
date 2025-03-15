# Kincir Makefile
# Supports build, test, and Docker operations

# Variables
CARGO := cargo
DOCKER := docker
DOCKER_COMPOSE := docker-compose
CARGO_EDIT := cargo-edit
VERSION := $(shell grep -m 1 'version = ' kincir/Cargo.toml | cut -d '"' -f 2)
EXAMPLES := kafka-example rabbitmq-example

# Colors for terminal output
YELLOW := \033[1;33m
GREEN := \033[1;32m
RED := \033[1;31m
NC := \033[0m # No Color

# Default target
.PHONY: all
all: build

# Help command
.PHONY: help
help:
	@echo "$(YELLOW)Kincir Makefile$(NC)"
	@echo "$(YELLOW)================$(NC)"
	@echo "Available commands:"
	@echo "  $(GREEN)make$(NC)                   - Build the project"
	@echo "  $(GREEN)make help$(NC)              - Show this help message"
	@echo ""
	@echo "$(YELLOW)Build Commands:$(NC)"
	@echo "  $(GREEN)make build$(NC)             - Build the project"
	@echo "  $(GREEN)make build-release$(NC)     - Build the project in release mode"
	@echo "  $(GREEN)make clean$(NC)             - Clean build artifacts"
	@echo ""
	@echo "$(YELLOW)Test Commands:$(NC)"
	@echo "  $(GREEN)make test$(NC)              - Run all tests"
	@echo "  $(GREEN)make test-unit$(NC)         - Run unit tests"
	@echo "  $(GREEN)make test-integration$(NC)  - Run integration tests"
	@echo ""
	@echo "$(YELLOW)Benchmark Commands:$(NC)"
	@echo "  $(GREEN)make bench$(NC)             - Run all benchmarks"
	@echo "  $(GREEN)make bench-kafka$(NC)       - Run Kafka benchmarks"
	@echo "  $(GREEN)make bench-rabbitmq$(NC)    - Run RabbitMQ benchmarks"
	@echo ""
	@echo "$(YELLOW)Documentation Commands:$(NC)"
	@echo "  $(GREEN)make docs$(NC)              - Generate documentation"
	@echo "  $(GREEN)make docs-open$(NC)         - Generate and open documentation in browser"
	@echo ""
	@echo "$(YELLOW)Verification Commands:$(NC)"
	@echo "  $(GREEN)make verify$(NC)            - Run all verification checks"
	@echo "  $(GREEN)make fmt$(NC)               - Format code with rustfmt"
	@echo "  $(GREEN)make lint$(NC)              - Run clippy linter"
	@echo "  $(GREEN)make check$(NC)             - Check for compilation errors"
	@echo ""
	@echo "$(YELLOW)Docker Commands:$(NC)"
	@echo "  $(GREEN)make docker-build$(NC)      - Build Docker image"
	@echo "  $(GREEN)make docker-up$(NC)         - Start all Docker services"
	@echo "  $(GREEN)make docker-down$(NC)       - Stop all Docker services"
	@echo "  $(GREEN)make docker-kafka$(NC)      - Run Kafka example in Docker"
	@echo "  $(GREEN)make docker-rabbitmq$(NC)   - Run RabbitMQ example in Docker"
	@echo ""
	@echo "$(YELLOW)Version Commands:$(NC)"
	@echo "  $(GREEN)make version$(NC)           - Show current version"
	@echo "  $(GREEN)make bump-major$(NC)        - Bump major version (x.0.0)"
	@echo "  $(GREEN)make bump-minor$(NC)        - Bump minor version (0.x.0)"
	@echo "  $(GREEN)make bump-patch$(NC)        - Bump patch version (0.0.x)"
	@echo "  $(GREEN)make set-version V=x.y.z$(NC) - Set specific version"
	@echo ""
	@echo "Current version: $(VERSION)"

# Build commands
.PHONY: build
build:
	@echo "$(YELLOW)Building project...$(NC)"
	$(CARGO) build
	@echo "$(GREEN)Build completed!$(NC)"

.PHONY: build-release
build-release:
	@echo "$(YELLOW)Building project in release mode...$(NC)"
	$(CARGO) build --release
	@echo "$(GREEN)Release build completed!$(NC)"

.PHONY: clean
clean:
	@echo "$(YELLOW)Cleaning build artifacts...$(NC)"
	$(CARGO) clean
	@echo "$(GREEN)Clean completed!$(NC)"

# Test commands
.PHONY: test
test: test-unit test-integration
	@echo "$(GREEN)All tests completed!$(NC)"

.PHONY: test-unit
test-unit:
	@echo "$(YELLOW)Running unit tests...$(NC)"
	$(CARGO) test --lib
	@echo "$(GREEN)Unit tests completed!$(NC)"

.PHONY: test-integration
test-integration:
	@echo "$(YELLOW)Running integration tests...$(NC)"
	$(CARGO) test --tests
	@echo "$(GREEN)Integration tests completed!$(NC)"

# Benchmark commands
.PHONY: bench
bench:
	@echo "$(YELLOW)Running benchmarks...$(NC)"
	./scripts/benchmark.sh --all
	@echo "$(GREEN)Benchmarks completed!$(NC)"

.PHONY: bench-kafka
bench-kafka:
	@echo "$(YELLOW)Running Kafka benchmarks...$(NC)"
	./scripts/benchmark.sh --kafka
	@echo "$(GREEN)Kafka benchmarks completed!$(NC)"

.PHONY: bench-rabbitmq
bench-rabbitmq:
	@echo "$(YELLOW)Running RabbitMQ benchmarks...$(NC)"
	./scripts/benchmark.sh --rabbitmq
	@echo "$(GREEN)RabbitMQ benchmarks completed!$(NC)"

# Documentation commands
.PHONY: docs
docs:
	@echo "$(YELLOW)Generating documentation...$(NC)"
	./scripts/generate_docs.sh
	@echo "$(GREEN)Documentation generated!$(NC)"

.PHONY: docs-open
docs-open:
	@echo "$(YELLOW)Generating and opening documentation...$(NC)"
	./scripts/generate_docs.sh --open
	@echo "$(GREEN)Documentation opened!$(NC)"

# Verification commands
.PHONY: verify
verify: fmt lint check
	@echo "$(GREEN)All verification checks passed!$(NC)"

.PHONY: fmt
fmt:
	@echo "$(YELLOW)Formatting code...$(NC)"
	$(CARGO) fmt --all
	@echo "$(GREEN)Formatting completed!$(NC)"

.PHONY: lint
lint:
	@echo "$(YELLOW)Running clippy linter...$(NC)"
	$(CARGO) clippy -- -D warnings
	@echo "$(GREEN)Linting completed!$(NC)"

.PHONY: check
check:
	@echo "$(YELLOW)Checking for compilation errors...$(NC)"
	$(CARGO) check --all-targets
	@echo "$(GREEN)Check completed!$(NC)"

# Docker commands
.PHONY: docker-build
docker-build:
	@echo "$(YELLOW)Building Docker image...$(NC)"
	$(DOCKER) build -t kincir:$(VERSION) .
	@echo "$(GREEN)Docker build completed!$(NC)"

.PHONY: docker-up
docker-up:
	@echo "$(YELLOW)Starting Docker services...$(NC)"
	$(DOCKER_COMPOSE) up -d
	@echo "$(GREEN)Docker services started!$(NC)"

.PHONY: docker-down
docker-down:
	@echo "$(YELLOW)Stopping Docker services...$(NC)"
	$(DOCKER_COMPOSE) down
	@echo "$(GREEN)Docker services stopped!$(NC)"

.PHONY: docker-kafka
docker-kafka:
	@echo "$(YELLOW)Running Kafka example in Docker...$(NC)"
	$(DOCKER_COMPOSE) --profile kafka-demo up -d
	@echo "$(GREEN)Kafka example started!$(NC)"

.PHONY: docker-rabbitmq
docker-rabbitmq:
	@echo "$(YELLOW)Running RabbitMQ example in Docker...$(NC)"
	$(DOCKER_COMPOSE) --profile rabbitmq-demo up -d
	@echo "$(GREEN)RabbitMQ example started!$(NC)"

# Version commands
.PHONY: version
version:
	@echo "Current version: $(VERSION)"

.PHONY: check-cargo-edit
check-cargo-edit:
	@which cargo-set-version > /dev/null || (echo "$(RED)cargo-edit is not installed. Installing...$(NC)" && cargo install cargo-edit)

.PHONY: bump-major
bump-major: check-cargo-edit
	@echo "$(YELLOW)Bumping major version...$(NC)"
	@cd kincir && cargo set-version --bump major
	@$(MAKE) update-workspace-version
	@echo "$(GREEN)Version bumped to $$(grep -m 1 'version = ' kincir/Cargo.toml | cut -d '"' -f 2)$(NC)"

.PHONY: bump-minor
bump-minor: check-cargo-edit
	@echo "$(YELLOW)Bumping minor version...$(NC)"
	@cd kincir && cargo set-version --bump minor
	@$(MAKE) update-workspace-version
	@echo "$(GREEN)Version bumped to $$(grep -m 1 'version = ' kincir/Cargo.toml | cut -d '"' -f 2)$(NC)"

.PHONY: bump-patch
bump-patch: check-cargo-edit
	@echo "$(YELLOW)Bumping patch version...$(NC)"
	@cd kincir && cargo set-version --bump patch
	@$(MAKE) update-workspace-version
	@echo "$(GREEN)Version bumped to $$(grep -m 1 'version = ' kincir/Cargo.toml | cut -d '"' -f 2)$(NC)"

.PHONY: set-version
set-version: check-cargo-edit
	@if [ -z "$(V)" ]; then \
		echo "$(RED)Error: Version not specified. Use 'make set-version V=x.y.z'$(NC)"; \
		exit 1; \
	fi
	@echo "$(YELLOW)Setting version to $(V)...$(NC)"
	@cd kincir && cargo set-version $(V)
	@$(MAKE) update-workspace-version
	@echo "$(GREEN)Version set to $(V)$(NC)"

.PHONY: update-workspace-version
update-workspace-version:
	@NEW_VERSION=$$(grep -m 1 'version = ' kincir/Cargo.toml | cut -d '"' -f 2); \
	echo "$(YELLOW)Updating workspace dependencies to version $$NEW_VERSION...$(NC)"; \
	sed -i.bak -E "s/(kincir = \{ path = \"kincir\", version = \")([0-9]+\.[0-9]+\.[0-9]+)(\"\})/\1$$NEW_VERSION\3/" Cargo.toml || true; \
	rm -f Cargo.toml.bak || true; \
	for example in $(EXAMPLES); do \
		sed -i.bak -E "s/(kincir = \{ version = \")([0-9]+\.[0-9]+\.[0-9]+)(\"\})/\1$$NEW_VERSION\3/" examples/$$example/Cargo.toml || true; \
		rm -f examples/$$example/Cargo.toml.bak || true; \
	done
	@echo "$(GREEN)Workspace dependencies updated!$(NC)"

# Run examples
.PHONY: run-kafka-example
run-kafka-example:
	@echo "$(YELLOW)Running Kafka example...$(NC)"
	cd examples/kafka-example && $(CARGO) run
	@echo "$(GREEN)Kafka example completed!$(NC)"

.PHONY: run-rabbitmq-example
run-rabbitmq-example:
	@echo "$(YELLOW)Running RabbitMQ example...$(NC)"
	cd examples/rabbitmq-example && $(CARGO) run
	@echo "$(GREEN)RabbitMQ example completed!$(NC)" 