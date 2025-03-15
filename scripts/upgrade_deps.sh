#!/bin/bash

# Script to upgrade dependencies in the Kincir project
# Usage: ./scripts/upgrade_deps.sh [--check|--update|--major]

set -e

# Colors for terminal output
YELLOW='\033[1;33m'
GREEN='\033[1;32m'
RED='\033[1;31m'
NC='\033[0m' # No Color

# Check if cargo-edit is installed
if ! command -v cargo-upgrade &> /dev/null; then
    echo -e "${RED}cargo-edit is not installed. Installing...${NC}"
    cargo install cargo-edit
fi

# Function to display help
show_help() {
    echo -e "${YELLOW}Kincir Dependency Upgrade Script${NC}"
    echo -e "${YELLOW}===============================${NC}"
    echo "Usage: ./scripts/upgrade_deps.sh [OPTION]"
    echo ""
    echo "Options:"
    echo "  --check       Check for outdated dependencies without upgrading"
    echo "  --update      Update dependencies to latest compatible versions (default)"
    echo "  --major       Update dependencies including major version bumps"
    echo "  --help        Display this help message"
    echo ""
    echo "Examples:"
    echo "  ./scripts/upgrade_deps.sh --check"
    echo "  ./scripts/upgrade_deps.sh --update"
    echo "  ./scripts/upgrade_deps.sh --major"
}

# Parse command line arguments
MODE="update"
if [ "$#" -gt 0 ]; then
    case "$1" in
        --check)
            MODE="check"
            ;;
        --update)
            MODE="update"
            ;;
        --major)
            MODE="major"
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

# Function to process a Cargo.toml file
process_cargo_toml() {
    local cargo_toml="$1"
    local dir=$(dirname "$cargo_toml")
    
    echo -e "${YELLOW}Processing $cargo_toml...${NC}"
    
    cd "$dir"
    
    if [ "$MODE" == "check" ]; then
        echo -e "${YELLOW}Checking for outdated dependencies...${NC}"
        cargo outdated
    elif [ "$MODE" == "update" ]; then
        echo -e "${YELLOW}Updating dependencies (compatible versions)...${NC}"
        cargo upgrade --incompatible ignore
    elif [ "$MODE" == "major" ]; then
        echo -e "${YELLOW}Updating dependencies (including major versions)...${NC}"
        cargo upgrade
    fi
    
    cd - > /dev/null
}

# Main script execution
echo -e "${YELLOW}Starting dependency upgrade process...${NC}"

# Process main crate
process_cargo_toml "kincir/Cargo.toml"

# Process examples
for example in examples/*/; do
    if [ -f "${example}Cargo.toml" ]; then
        process_cargo_toml "${example}Cargo.toml"
    fi
done

# Process workspace Cargo.toml
process_cargo_toml "Cargo.toml"

echo -e "${GREEN}Dependency upgrade process completed!${NC}"

# If we updated dependencies, suggest running tests
if [ "$MODE" != "check" ]; then
    echo -e "${YELLOW}Recommended next steps:${NC}"
    echo -e "  1. Run ${GREEN}make test${NC} to verify everything still works"
    echo -e "  2. Run ${GREEN}make verify${NC} to check for any linting issues"
    echo -e "  3. Commit the changes with a message like 'chore: update dependencies'"
fi 