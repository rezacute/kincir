#!/usr/bin/env python3
import http.server
import socketserver
import os
import sys
import markdown
import re
from pathlib import Path

PORT = 8080
DOCS_DIR = "/home/ubuntu/code/kincir/docs"
PROJECT_DIR = "/home/ubuntu/code/kincir"

class DocsHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=DOCS_DIR, **kwargs)
    
    def do_GET(self):
        # Handle root path
        if self.path == '/':
            self.path = '/index.html'
        
        # Convert .md requests to .html
        if self.path.endswith('.md'):
            self.path = self.path[:-3] + '.html'
        
        # Try to serve the file
        try:
            # Check if it's a markdown file that needs conversion
            md_path = os.path.join(DOCS_DIR, self.path[1:].replace('.html', '.md'))
            html_path = os.path.join(DOCS_DIR, self.path[1:])
            
            if os.path.exists(md_path) and self.path.endswith('.html'):
                # Convert markdown to HTML
                self.serve_markdown(md_path)
                return
            elif os.path.exists(html_path):
                # Serve existing HTML file
                super().do_GET()
                return
            else:
                # Try to serve from project root for README
                if self.path == '/README.html':
                    readme_path = os.path.join(PROJECT_DIR, 'README.md')
                    if os.path.exists(readme_path):
                        self.serve_markdown(readme_path)
                        return
                
                # File not found
                self.send_error(404, "File not found")
                
        except Exception as e:
            self.send_error(500, f"Server error: {str(e)}")
    
    def serve_markdown(self, md_path):
        try:
            with open(md_path, 'r', encoding='utf-8') as f:
                content = f.read()
            
            # Remove Jekyll front matter
            content = re.sub(r'^---\n.*?\n---\n', '', content, flags=re.DOTALL)
            
            # Convert markdown to HTML
            md = markdown.Markdown(extensions=['codehilite', 'fenced_code', 'tables', 'toc'])
            html_content = md.convert(content)
            
            # Create a simple HTML template
            full_html = f"""
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kincir Documentation</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            color: #333;
        }}
        h1, h2, h3, h4, h5, h6 {{
            color: #2c3e50;
            margin-top: 2em;
        }}
        h1 {{
            border-bottom: 2px solid #3498db;
            padding-bottom: 10px;
        }}
        code {{
            background-color: #f8f9fa;
            padding: 2px 4px;
            border-radius: 3px;
            font-family: 'Monaco', 'Consolas', monospace;
        }}
        pre {{
            background-color: #f8f9fa;
            padding: 15px;
            border-radius: 5px;
            overflow-x: auto;
            border-left: 4px solid #3498db;
        }}
        pre code {{
            background: none;
            padding: 0;
        }}
        blockquote {{
            border-left: 4px solid #3498db;
            margin: 0;
            padding-left: 20px;
            color: #666;
        }}
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 20px 0;
        }}
        th, td {{
            border: 1px solid #ddd;
            padding: 12px;
            text-align: left;
        }}
        th {{
            background-color: #f8f9fa;
            font-weight: bold;
        }}
        .nav {{
            background-color: #2c3e50;
            color: white;
            padding: 10px 0;
            margin: -20px -20px 20px -20px;
            text-align: center;
        }}
        .nav a {{
            color: white;
            text-decoration: none;
            margin: 0 15px;
        }}
        .nav a:hover {{
            text-decoration: underline;
        }}
        .badge {{
            display: inline-block;
            padding: 2px 8px;
            background-color: #3498db;
            color: white;
            border-radius: 12px;
            font-size: 0.8em;
            margin: 2px;
        }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/">Home</a>
        <a href="/README.html">README</a>
        <a href="https://github.com/rezacute/kincir">GitHub</a>
        <a href="https://crates.io/crates/kincir">Crates.io</a>
    </div>
    {html_content}
</body>
</html>
"""
            
            self.send_response(200)
            self.send_header('Content-type', 'text/html; charset=utf-8')
            self.end_headers()
            self.wfile.write(full_html.encode('utf-8'))
            
        except Exception as e:
            self.send_error(500, f"Error processing markdown: {str(e)}")

def main():
    # Check if running as root (required for port 80)
    if PORT < 1024 and os.geteuid() != 0:
        print(f"Error: This script must be run as root to bind to port {PORT}")
        print("Please run: sudo python3 simple_docs_server.py")
        sys.exit(1)
    
    # Change to docs directory
    os.chdir(DOCS_DIR)
    
    print(f"Starting Kincir documentation server on port {PORT}")
    print(f"Serving from: {DOCS_DIR}")
    print(f"Access the documentation at: http://localhost:{PORT}")
    print("Press Ctrl+C to stop the server")
    
    try:
        with socketserver.TCPServer(("", PORT), DocsHandler) as httpd:
            httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nServer stopped.")
    except PermissionError:
        print(f"Error: Permission denied to bind to port {PORT}")
        print("Please run as root: sudo python3 simple_docs_server.py")
    except Exception as e:
        print(f"Error starting server: {e}")

if __name__ == "__main__":
    main()
