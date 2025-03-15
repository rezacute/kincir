#!/bin/bash

# Script to manage Docker environment for Kincir
# Usage: ./scripts/docker_env.sh [start|stop|restart|status|logs|kafka|rabbitmq|both]

set -e

# Colors for terminal output
YELLOW='\033[1;33m'
GREEN='\033[1;32m'
RED='\033[1;31m'
BLUE='\033[1;34m'
NC='\033[0m' # No Color

# Function to display help
show_help() {
    echo -e "${YELLOW}Kincir Docker Environment Manager${NC}"
    echo -e "${YELLOW}=============================${NC}"
    echo "Usage: ./scripts/docker_env.sh [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  start       Start the Docker environment"
    echo "  stop        Stop the Docker environment"
    echo "  restart     Restart the Docker environment"
    echo "  status      Show status of Docker containers"
    echo "  logs        Show logs from Docker containers"
    echo "  kafka       Start Kafka example environment"
    echo "  rabbitmq    Start RabbitMQ example environment"
    echo "  both        Start both Kafka and RabbitMQ example environments"
    echo "  build       Build Docker images"
    echo "  clean       Remove Docker containers, networks, and volumes"
    echo "  help        Display this help message"
    echo ""
    echo "Examples:"
    echo "  ./scripts/docker_env.sh start"
    echo "  ./scripts/docker_env.sh logs"
    echo "  ./scripts/docker_env.sh kafka"
}

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is not installed or not in PATH${NC}"
    exit 1
fi

# Check if Docker Compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}Error: Docker Compose is not installed or not in PATH${NC}"
    exit 1
fi

# Parse command line arguments
if [ "$#" -eq 0 ]; then
    show_help
    exit 1
fi

COMMAND="$1"
CONTAINER="$2"

# Function to start Docker environment
start_env() {
    echo -e "${YELLOW}Starting Docker environment...${NC}"
    docker-compose up -d
    echo -e "${GREEN}Docker environment started!${NC}"
}

# Function to stop Docker environment
stop_env() {
    echo -e "${YELLOW}Stopping Docker environment...${NC}"
    docker-compose down
    echo -e "${GREEN}Docker environment stopped!${NC}"
}

# Function to restart Docker environment
restart_env() {
    echo -e "${YELLOW}Restarting Docker environment...${NC}"
    docker-compose restart
    echo -e "${GREEN}Docker environment restarted!${NC}"
}

# Function to show status of Docker containers
show_status() {
    echo -e "${YELLOW}Docker container status:${NC}"
    docker-compose ps
}

# Function to show logs from Docker containers
show_logs() {
    if [ -z "$CONTAINER" ]; then
        echo -e "${YELLOW}Showing logs from all containers:${NC}"
        docker-compose logs
    else
        echo -e "${YELLOW}Showing logs from ${CONTAINER}:${NC}"
        docker-compose logs "$CONTAINER"
    fi
}

# Function to start Kafka example environment
start_kafka() {
    echo -e "${YELLOW}Starting Kafka example environment...${NC}"
    docker-compose --profile kafka-demo up -d
    echo -e "${GREEN}Kafka example environment started!${NC}"
    echo -e "${BLUE}Kafka is available at localhost:9092${NC}"
}

# Function to start RabbitMQ example environment
start_rabbitmq() {
    echo -e "${YELLOW}Starting RabbitMQ example environment...${NC}"
    docker-compose --profile rabbitmq-demo up -d
    echo -e "${GREEN}RabbitMQ example environment started!${NC}"
    echo -e "${BLUE}RabbitMQ is available at localhost:5672${NC}"
    echo -e "${BLUE}RabbitMQ Management UI is available at http://localhost:15672${NC}"
    echo -e "${BLUE}Username: guest, Password: guest${NC}"
}

# Function to start both Kafka and RabbitMQ example environments
start_both() {
    echo -e "${YELLOW}Starting both Kafka and RabbitMQ example environments...${NC}"
    docker-compose --profile kafka-demo --profile rabbitmq-demo up -d
    echo -e "${GREEN}Both example environments started!${NC}"
    echo -e "${BLUE}Kafka is available at localhost:9092${NC}"
    echo -e "${BLUE}RabbitMQ is available at localhost:5672${NC}"
    echo -e "${BLUE}RabbitMQ Management UI is available at http://localhost:15672${NC}"
    echo -e "${BLUE}Username: guest, Password: guest${NC}"
}

# Function to build Docker images
build_images() {
    echo -e "${YELLOW}Building Docker images...${NC}"
    docker-compose build
    echo -e "${GREEN}Docker images built!${NC}"
}

# Function to clean Docker environment
clean_env() {
    echo -e "${YELLOW}Cleaning Docker environment...${NC}"
    docker-compose down -v
    echo -e "${GREEN}Docker environment cleaned!${NC}"
}

# Main execution
case "$COMMAND" in
    start)
        start_env
        ;;
    stop)
        stop_env
        ;;
    restart)
        restart_env
        ;;
    status)
        show_status
        ;;
    logs)
        show_logs
        ;;
    kafka)
        start_kafka
        ;;
    rabbitmq)
        start_rabbitmq
        ;;
    both)
        start_both
        ;;
    build)
        build_images
        ;;
    clean)
        clean_env
        ;;
    help)
        show_help
        ;;
    *)
        echo -e "${RED}Unknown command: $COMMAND${NC}"
        show_help
        exit 1
        ;;
esac 