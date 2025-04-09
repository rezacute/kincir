#!/bin/bash

# This script allows you to manually trigger the GitHub Actions workflow
# You need to have the GitHub CLI installed and be authenticated

# Set these variables
REPO="rezacute/kincir"
WORKFLOW="jekyll.yml"  # or static-docs.yml if you want to deploy the static files

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "Triggering workflow ${WORKFLOW} in ${REPO}..."

# Check if GitHub CLI is installed
if ! command -v gh &> /dev/null; then
    echo -e "${RED}GitHub CLI (gh) is not installed. Please install it first.${NC}"
    echo "Visit https://cli.github.com/ for installation instructions."
    exit 1
fi

# Check if user is authenticated
if ! gh auth status &> /dev/null; then
    echo -e "${RED}You are not authenticated with GitHub CLI.${NC}"
    echo "Run 'gh auth login' to authenticate."
    exit 1
fi

# Trigger the workflow
if [ "$WORKFLOW" = "static-docs.yml" ]; then
    # For static-docs.yml we need to pass the deploy_static input parameter
    gh workflow run $WORKFLOW -R $REPO --ref main -f deploy_static=true
else
    # For jekyll.yml we don't need additional parameters
    gh workflow run $WORKFLOW -R $REPO --ref main
fi

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Workflow triggered successfully!${NC}"
    echo "You can check the status on GitHub Actions dashboard:"
    echo "https://github.com/${REPO}/actions/workflows/${WORKFLOW}"
else
    echo -e "${RED}Failed to trigger workflow.${NC}"
    exit 1
fi 