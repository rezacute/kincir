#!/bin/bash

# This script updates all documentation pages to use the docs layout
# Run from the root of the docs directory

# Find all markdown files in the docs directory (excluding the main index.md)
find docs -name "*.md" ! -path "docs/index.md" | while read -r file; do
  # Check if the file uses the default layout
  if grep -q "layout: default" "$file"; then
    echo "Updating layout for $file"
    # Replace 'layout: default' with 'layout: docs'
    sed -i '' 's/layout: default/layout: docs/g' "$file"
    
    # Check if the file contains a docs-container-page div wrapper
    if grep -q "<div class=\"docs-container-page\">" "$file"; then
      echo "Removing docs-container-page wrapper from $file"
      # Remove the opening and closing div tags
      sed -i '' '/<div class="docs-container-page">/d' "$file"
      sed -i '' '/<\/div>$/d' "$file"
    fi
  fi
done

echo "Layout updates complete!" 