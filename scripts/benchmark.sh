#!/bin/bash

# Script to run benchmarks for the Kincir library
# Usage: ./scripts/benchmark.sh [--all|--kafka|--rabbitmq]

set -e

# Colors for terminal output
YELLOW='\033[1;33m'
GREEN='\033[1;32m'
RED='\033[1;31m'
BLUE='\033[1;34m'
NC='\033[0m' # No Color

# Function to display help
show_help() {
    echo -e "${YELLOW}Kincir Benchmark Script${NC}"
    echo -e "${YELLOW}======================${NC}"
    echo "Usage: ./scripts/benchmark.sh [OPTION]"
    echo ""
    echo "Options:"
    echo "  --all        Run all benchmarks (default)"
    echo "  --kafka      Run only Kafka benchmarks"
    echo "  --rabbitmq   Run only RabbitMQ benchmarks"
    echo "  --help       Display this help message"
    echo ""
    echo "Examples:"
    echo "  ./scripts/benchmark.sh --all"
    echo "  ./scripts/benchmark.sh --kafka"
}

# Check if cargo-criterion is installed
if ! command -v cargo-criterion &> /dev/null; then
    echo -e "${YELLOW}cargo-criterion is not installed. Installing...${NC}"
    cargo install cargo-criterion
fi

# Parse command line arguments
BENCHMARK_TYPE="all"
if [ "$#" -gt 0 ]; then
    case "$1" in
        --all)
            BENCHMARK_TYPE="all"
            ;;
        --kafka)
            BENCHMARK_TYPE="kafka"
            ;;
        --rabbitmq)
            BENCHMARK_TYPE="rabbitmq"
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            show_help
            exit 1
            ;;
    esac
fi

# Function to run benchmarks
run_benchmarks() {
    local benchmark_type="$1"
    
    echo -e "${YELLOW}Running ${benchmark_type} benchmarks...${NC}"
    
    # Make sure Docker services are running if needed
    if [ "$benchmark_type" = "kafka" ] || [ "$benchmark_type" = "all" ]; then
        echo -e "${BLUE}Making sure Kafka is running...${NC}"
        docker-compose up -d zookeeper kafka
        sleep 5
    fi
    
    if [ "$benchmark_type" = "rabbitmq" ] || [ "$benchmark_type" = "all" ]; then
        echo -e "${BLUE}Making sure RabbitMQ is running...${NC}"
        docker-compose up -d rabbitmq
        sleep 5
    fi
    
    # Run the benchmarks
    if [ "$benchmark_type" = "all" ]; then
        cargo criterion --bench kincir_benchmarks
    elif [ "$benchmark_type" = "kafka" ]; then
        cargo criterion --bench kincir_benchmarks -- kafka
    elif [ "$benchmark_type" = "rabbitmq" ]; then
        cargo criterion --bench kincir_benchmarks -- rabbitmq
    fi
    
    echo -e "${GREEN}Benchmarks completed!${NC}"
}

# Main execution
run_benchmarks "$BENCHMARK_TYPE"

# Show benchmark results location
echo -e "${YELLOW}Benchmark results are available in target/criterion/${NC}"
echo -e "${YELLOW}You can open target/criterion/report/index.html in your browser to view the results.${NC}" 