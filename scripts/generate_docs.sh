#!/bin/bash

# Script to generate documentation for the Kincir library
# Usage: ./scripts/generate_docs.sh [--open]

set -e

# Colors for terminal output
YELLOW='\033[1;33m'
GREEN='\033[1;32m'
RED='\033[1;31m'
BLUE='\033[1;34m'
NC='\033[0m' # No Color

# Parse command line arguments
OPEN_DOCS=false
if [ "$#" -gt 0 ] && [ "$1" == "--open" ]; then
    OPEN_DOCS=true
fi

# Function to check if cargo-doc is available
check_cargo_doc() {
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: cargo is not installed or not in PATH${NC}"
        exit 1
    fi
}

# Function to generate documentation
generate_docs() {
    echo -e "${YELLOW}Generating documentation...${NC}"
    
    # Generate documentation with all features enabled
    cargo doc --no-deps --all-features --workspace
    
    echo -e "${GREEN}Documentation generated!${NC}"
    echo -e "${BLUE}Documentation is available at target/doc/kincir/index.html${NC}"
}

# Function to open documentation in browser
open_docs() {
    echo -e "${YELLOW}Opening documentation in browser...${NC}"
    
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        xdg-open target/doc/kincir/index.html
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        open target/doc/kincir/index.html
    elif [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
        # Windows
        start target/doc/kincir/index.html
    else
        echo -e "${RED}Error: Unsupported operating system${NC}"
        echo -e "${BLUE}Documentation is available at target/doc/kincir/index.html${NC}"
        return 1
    fi
    
    echo -e "${GREEN}Documentation opened in browser!${NC}"
}

# Main execution
check_cargo_doc
generate_docs

if [ "$OPEN_DOCS" = true ]; then
    open_docs
fi 