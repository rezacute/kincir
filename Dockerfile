FROM rust:1.76-slim as builder

WORKDIR /usr/src/kincir

# Copy the entire project
COPY . .

# Build the project in release mode
RUN cargo build --release

# Create a smaller runtime image
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binaries from the builder stage
COPY --from=builder /usr/src/kincir/target/release/kincir /app/
COPY --from=builder /usr/src/kincir/target/release/kafka-example /app/
COPY --from=builder /usr/src/kincir/target/release/rabbitmq-example /app/

# Set the default command
CMD ["./kincir"] 