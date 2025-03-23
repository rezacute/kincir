# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.6] - 2024-03-23

### Added
- Protocol Buffers (protobuf) encoding/decoding support via `protobuf` feature flag
- New `MessageCodec` trait for message encoding and decoding 
- `ProtobufCodec` implementation for efficient binary serialization
- Example application for Protocol Buffers usage
- Default implementations for various structs to follow Rust conventions

### Fixed
- Various Clippy warnings across the codebase
- Proper implementation of ProstMessage trait for protocol buffer structs

## [0.1.5] - 2024-03-22

### Added
- Optional logging with feature flag support
- Default features now include logging
- Improved documentation of feature flags

### Changed
- Refactored router implementation to work with or without logging
- Made logging feature optional and configurable

## [0.1.4] - 2024-03-20

### Added
- Makefile for common development tasks
- Docker and docker-compose support for development and testing
- Scripts for managing Docker environment
- Scripts for upgrading dependencies
- Scripts for releasing new versions
- CHANGELOG.md file

### Changed
- Updated example applications to work in Docker environment
- Updated README.md with build and Docker information

### Fixed
- Clippy warning in integration test

## [0.1.0] - 2023-02-18

### Added
- Initial release
- Basic message router implementation
- Support for Kafka and RabbitMQ backends
- Message UUID generation
- Metadata support
- Logging support
- Example applications
