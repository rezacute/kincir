#!/bin/bash

# Build the site with an empty baseurl for local development
echo "Building Jekyll site with empty baseurl for local development..."
bundle exec jekyll build --baseurl ""

# Start the Python server
echo "Starting server at http://localhost:8080"
python3 serve.py 