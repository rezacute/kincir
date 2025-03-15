#!/bin/bash

# Script to release a new version of the Kincir library
# Usage: ./scripts/release.sh [major|minor|patch|x.y.z]

set -e

# Colors for terminal output
YELLOW='\033[1;33m'
GREEN='\033[1;32m'
RED='\033[1;31m'
NC='\033[0m' # No Color

# Check if we're in the root directory of the project
if [ ! -f "Cargo.toml" ] || [ ! -d "kincir" ]; then
    echo -e "${RED}Error: This script must be run from the root directory of the project${NC}"
    exit 1
fi

# Function to display help
show_help() {
    echo -e "${YELLOW}Kincir Release Script${NC}"
    echo -e "${YELLOW}===================${NC}"
    echo "Usage: ./scripts/release.sh [VERSION_TYPE]"
    echo ""
    echo "Version types:"
    echo "  major        Bump major version (x.0.0)"
    echo "  minor        Bump minor version (0.x.0)"
    echo "  patch        Bump patch version (0.0.x)"
    echo "  x.y.z        Set specific version"
    echo "  --help       Display this help message"
    echo ""
    echo "Examples:"
    echo "  ./scripts/release.sh major"
    echo "  ./scripts/release.sh minor"
    echo "  ./scripts/release.sh patch"
    echo "  ./scripts/release.sh 1.2.3"
}

# Parse command line arguments
if [ "$#" -eq 0 ]; then
    show_help
    exit 1
fi

VERSION_TYPE="$1"
if [ "$VERSION_TYPE" == "--help" ]; then
    show_help
    exit 0
fi

# Get current version
CURRENT_VERSION=$(grep -m 1 'version = ' kincir/Cargo.toml | cut -d '"' -f 2)
echo -e "${YELLOW}Current version: ${CURRENT_VERSION}${NC}"

# Function to check if all tests pass
run_tests() {
    echo -e "${YELLOW}Running tests...${NC}"
    if ! make test; then
        echo -e "${RED}Tests failed. Aborting release.${NC}"
        exit 1
    fi
    echo -e "${GREEN}All tests passed!${NC}"
}

# Function to check if code passes verification
run_verification() {
    echo -e "${YELLOW}Running verification checks...${NC}"
    if ! make verify; then
        echo -e "${RED}Verification failed. Aborting release.${NC}"
        exit 1
    fi
    echo -e "${GREEN}All verification checks passed!${NC}"
}

# Function to update version
update_version() {
    local version_type="$1"
    
    echo -e "${YELLOW}Updating version...${NC}"
    
    if [[ "$version_type" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        # Set specific version
        make set-version V="$version_type"
        NEW_VERSION="$version_type"
    else
        # Bump version
        make bump-"$version_type"
        NEW_VERSION=$(grep -m 1 'version = ' kincir/Cargo.toml | cut -d '"' -f 2)
    fi
    
    echo -e "${GREEN}Version updated to ${NEW_VERSION}${NC}"
}

# Function to update CHANGELOG.md
update_changelog() {
    local new_version="$1"
    local changelog_file="CHANGELOG.md"
    
    # Create changelog file if it doesn't exist
    if [ ! -f "$changelog_file" ]; then
        echo -e "${YELLOW}Creating new CHANGELOG.md file...${NC}"
        cat > "$changelog_file" << EOF
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

EOF
    fi
    
    echo -e "${YELLOW}Updating CHANGELOG.md...${NC}"
    
    # Get the date in YYYY-MM-DD format
    local today=$(date +%Y-%m-%d)
    
    # Create a temporary file
    local temp_file=$(mktemp)
    
    # Add new version section at the top of the changelog
    cat > "$temp_file" << EOF
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [${new_version}] - ${today}

### Added
- TODO: Add new features

### Changed
- TODO: Add changes in existing functionality

### Deprecated
- TODO: Add soon-to-be removed features

### Removed
- TODO: Add removed features

### Fixed
- TODO: Add bug fixes

### Security
- TODO: Add security improvements

EOF
    
    # Append the rest of the original changelog (skipping the header)
    tail -n +6 "$changelog_file" >> "$temp_file"
    
    # Replace the original changelog with the new one
    mv "$temp_file" "$changelog_file"
    
    echo -e "${GREEN}CHANGELOG.md updated!${NC}"
    echo -e "${YELLOW}Please edit CHANGELOG.md to add the actual changes for version ${new_version}${NC}"
}

# Function to create a git tag
create_git_tag() {
    local version="$1"
    
    echo -e "${YELLOW}Creating git tag v${version}...${NC}"
    
    git add .
    git commit -m "chore: release version ${version}"
    git tag -a "v${version}" -m "Version ${version}"
    
    echo -e "${GREEN}Git tag created!${NC}"
    echo -e "${YELLOW}To push the changes and tag, run:${NC}"
    echo -e "  git push origin main"
    echo -e "  git push origin v${version}"
}

# Main execution
run_tests
run_verification
update_version "$VERSION_TYPE"
update_changelog "$NEW_VERSION"

# Prompt user to edit CHANGELOG.md
echo -e "${YELLOW}Please edit CHANGELOG.md now to add the actual changes for version ${NEW_VERSION}${NC}"
echo -e "${YELLOW}Press Enter when done...${NC}"
read

# Confirm with user
echo -e "${YELLOW}Ready to commit and tag version ${NEW_VERSION}. Continue? [y/N]${NC}"
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    create_git_tag "$NEW_VERSION"
    
    echo -e "${GREEN}Release process completed!${NC}"
    echo -e "${YELLOW}Next steps:${NC}"
    echo -e "  1. Push the changes: ${GREEN}git push origin main${NC}"
    echo -e "  2. Push the tag: ${GREEN}git push origin v${NEW_VERSION}${NC}"
    echo -e "  3. Publish to crates.io: ${GREEN}cd kincir && cargo publish${NC}"
else
    echo -e "${YELLOW}Release process aborted.${NC}"
fi 