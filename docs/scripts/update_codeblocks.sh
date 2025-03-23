#!/bin/bash

# This script finds regular markdown code blocks and converts them to Jekyll highlight blocks
# Run from the root of the docs directory

# Function to convert a file
convert_file() {
    local file=$1
    local tempfile="${file}.temp"
    
    # Read file content
    content=$(cat "$file")
    
    # Use Python to handle the conversion (more reliable than complex bash regex)
    python3 -c "
import re
import sys

content = '''$content'''

# Pattern to match markdown code blocks
pattern = r'```(\w+)\n([\s\S]*?)```'

def replace_with_highlight(match):
    lang = match.group(1)
    code = match.group(2)
    return f'''<div class=\"highlight-wrapper\" style=\"position: relative;\" data-language=\"{lang.upper()}\">
{{% highlight {lang} %}}
{code.rstrip()}
{{% endhighlight %}}
<button class=\"copy-button manual-copy-btn\" onclick=\"copyCode(this)\" aria-label=\"Copy code to clipboard\">
  <svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"18\" height=\"18\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect x=\"9\" y=\"9\" width=\"13\" height=\"13\" rx=\"2\" ry=\"2\"></rect><path d=\"M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1\"></path></svg>
</button>
</div>'''

# Replace all code blocks
new_content = re.sub(pattern, replace_with_highlight, content)

# Output the new content
print(new_content)
" > "$tempfile"
    
    # Check if file was modified
    if ! cmp -s "$file" "$tempfile"; then
        mv "$tempfile" "$file"
        echo "Updated: $file"
    else
        rm "$tempfile"
        echo "Skipped: $file (no changes needed)"
    fi
}

# Find all markdown files
for file in $(find .. -name "*.md"); do
    # Skip files that already have highlight tags
    if grep -q "highlight-wrapper" "$file"; then
        echo "Skipped: $file (already processed)"
        continue
    fi
    
    # Skip files that don't have code blocks
    if ! grep -q "\`\`\`" "$file"; then
        echo "Skipped: $file (no code blocks)"
        continue
    fi
    
    convert_file "$file"
done

echo "Code block conversion complete! Refresh your browser to see the changes." 