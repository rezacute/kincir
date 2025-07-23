# Kincir v0.2 - Core Enhancements

This directory contains the detailed task breakdown for Kincir v0.2 milestone, focusing on core stability and testing infrastructure.

## v0.2 Goals

✅ **Core Enhancements**
- In-memory message broker for local testing
- Unified Ack/Nack handling across backends
- Correlation ID tracking for tracing
- Performance profiling and initial benchmarks
- Unit & integration tests for stability

## Task Categories

### 1. In-Memory Message Broker
- **File**: `01-in-memory-broker.md`
- **Priority**: High
- **Dependencies**: None
- **Estimated Effort**: 3-5 days

### 2. Unified Ack/Nack Handling
- **File**: `02-ack-nack-handling.md`
- **Priority**: High
- **Dependencies**: None
- **Estimated Effort**: 4-6 days

### 3. Correlation ID Tracking
- **File**: `03-correlation-id-tracking.md`
- **Priority**: Medium
- **Dependencies**: None
- **Estimated Effort**: 2-3 days

### 4. Performance Profiling & Benchmarks
- **File**: `04-performance-benchmarks.md`
- **Priority**: Medium
- **Dependencies**: In-memory broker
- **Estimated Effort**: 3-4 days

### 5. Unit & Integration Tests
- **File**: `05-testing-infrastructure.md`
- **Priority**: High
- **Dependencies**: All above features
- **Estimated Effort**: 5-7 days

## Development Order

1. **Phase 1** (Parallel): In-memory broker + Correlation ID tracking
2. **Phase 2**: Unified Ack/Nack handling (requires testing with in-memory broker)
3. **Phase 3**: Performance benchmarks (requires stable core features)
4. **Phase 4**: Comprehensive testing (validates all features)

## Success Criteria

- [ ] In-memory broker passes all Publisher/Subscriber trait tests
- [ ] All backends support consistent Ack/Nack operations
- [ ] Correlation IDs are properly propagated through message flows
- [ ] Benchmark suite covers all major operations
- [ ] Test coverage > 80% for core modules
- [ ] All examples work with new features
- [ ] Documentation updated for new capabilities

## Timeline

**Target Duration**: 3-4 weeks
**Target Release**: End of current sprint + 1 month

## Notes

- Focus on backward compatibility
- Maintain existing API surface where possible
- Add feature flags for new functionality if needed
- Ensure all changes are well-documented
