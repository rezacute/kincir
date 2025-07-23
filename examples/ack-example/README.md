# Acknowledgment Example

This example demonstrates the new acknowledgment handling functionality in Kincir's in-memory broker.

## Features Demonstrated

- **Manual Acknowledgment**: Explicit control over message acknowledgment
- **Batch Operations**: Acknowledging multiple messages at once
- **Error Handling**: Proper handling of acknowledgment failures
- **Configuration Options**: Different acknowledgment modes and settings
- **Statistics Integration**: Monitoring acknowledgment operations

## Running the Example

```bash
cd examples/ack-example
cargo run
```

## What You'll See

The example will:

1. Set up an in-memory broker with acknowledgment support
2. Publish several test messages
3. Demonstrate different acknowledgment scenarios:
   - Successful processing (ACK)
   - Recoverable failure (NACK with requeue)
   - Permanent failure (NACK without requeue)
4. Show batch acknowledgment operations
5. Display broker statistics and health information
6. Demonstrate configuration options
7. Show error handling scenarios

## Key Concepts

### Acknowledgment Modes

- **Manual**: Explicit acknowledgment required
- **Auto**: Automatic acknowledgment after receive
- **ClientAuto**: Automatic acknowledgment after handler success

### Message Handling

- **ACK**: Message processed successfully
- **NACK with requeue**: Message failed but can be retried
- **NACK without requeue**: Message failed permanently (dead letter)

### Batch Operations

- Process multiple messages efficiently
- Reduce network overhead in distributed scenarios
- Maintain transactional consistency

## Integration with Existing Code

The acknowledgment functionality is designed to be backward compatible. Existing code using the standard `Subscriber` trait will continue to work unchanged, while new code can opt into acknowledgment handling using the `AckSubscriber` trait.
