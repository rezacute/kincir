#!/usr/bin/env python3
import http.server
import socketserver
import os
import sys
import markdown
import re
from pathlib import Path
import hashlib
import json
import time
from pygments import highlight
from pygments.lexers import get_lexer_by_name
from pygments.formatters import HtmlFormatter

PORT = 8080
DOCS_DIR = "/home/ubuntu/code/kincir/docs"
PROJECT_DIR = "/home/ubuntu/code/kincir"
CACHE_DIR = "/tmp/kincir_docs_cache"

# Create cache directory
os.makedirs(CACHE_DIR, exist_ok=True)

class DocsHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=DOCS_DIR, **kwargs)
    
    def do_GET(self):
        # Handle root path
        if self.path == '/':
            self.path = '/index.html'
        
        # Handle directory paths - add index.html
        if self.path.endswith('/') and self.path != '/':
            self.path = self.path + 'index.html'
        
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
    
    def get_cached_html(self, md_path):
        """Check if we have a cached version of the HTML"""
        try:
            # Create cache key from file path and modification time
            stat = os.stat(md_path)
            cache_key = hashlib.md5(f"{md_path}:{stat.st_mtime}".encode()).hexdigest()
            cache_file = os.path.join(CACHE_DIR, f"{cache_key}.json")
            
            if os.path.exists(cache_file):
                with open(cache_file, 'r') as f:
                    cache_data = json.load(f)
                    return cache_data['html']
            return None
        except:
            return None
    
    def cache_html(self, md_path, html_content):
        """Cache the converted HTML"""
        try:
            stat = os.stat(md_path)
            cache_key = hashlib.md5(f"{md_path}:{stat.st_mtime}".encode()).hexdigest()
            cache_file = os.path.join(CACHE_DIR, f"{cache_key}.json")
            
            cache_data = {
                'html': html_content,
                'timestamp': time.time(),
                'source_file': md_path
            }
            
            with open(cache_file, 'w') as f:
                json.dump(cache_data, f)
        except:
            pass  # Ignore cache errors
    
    def serve_markdown(self, md_path):
        try:
            # Check cache first
            cached_html = self.get_cached_html(md_path)
            if cached_html:
                html_content = cached_html
            else:
                # Read and process markdown
                with open(md_path, 'r', encoding='utf-8') as f:
                    content = f.read()
                
                # Remove Jekyll front matter
                content = re.sub(r'^---\n.*?\n---\n', '', content, flags=re.DOTALL)
                
                # Configure markdown with enhanced extensions
                md = markdown.Markdown(extensions=[
                    'codehilite',
                    'fenced_code', 
                    'tables',
                    'toc',
                    'footnotes',
                    'attr_list',
                    'def_list',
                    'abbr',
                    'admonition'
                ], extension_configs={
                    'codehilite': {
                        'css_class': 'highlight',
                        'use_pygments': True,
                        'pygments_style': 'github'
                    },
                    'toc': {
                        'permalink': True,
                        'permalink_title': 'Link to this section'
                    }
                })
                
                # Convert markdown to HTML
                html_content = md.convert(content)
                
                # Add copy buttons to code blocks
                html_content = self.add_copy_buttons(html_content)
                
                # Cache the result
                self.cache_html(md_path, html_content)
            
            # Create enhanced HTML template
            full_html = self.create_html_template(html_content, md_path)
            
            self.send_response(200)
            self.send_header('Content-type', 'text/html; charset=utf-8')
            self.send_header('Cache-Control', 'public, max-age=300')  # 5 minute cache
            self.end_headers()
            self.wfile.write(full_html.encode('utf-8'))
            
        except Exception as e:
            self.send_error(500, f"Error processing markdown: {str(e)}")
    
    def add_copy_buttons(self, html_content):
        """Add copy buttons to code blocks"""
        # Pattern to match code blocks with optional language specification
        code_block_pattern = r'<div class="codehilite"><pre><span></span><code[^>]*>(.*?)</code></pre></div>'
        
        def add_copy_button(match):
            code_content = match.group(1)
            # Extract plain text from HTML
            plain_text = re.sub(r'<[^>]+>', '', code_content)
            plain_text = plain_text.replace('&lt;', '<').replace('&gt;', '>').replace('&amp;', '&')
            
            return f'''<div class="code-block-container">
                <div class="code-block-header">
                    <button class="copy-button" onclick="copyCode(this)" data-code="{plain_text.replace('"', '&quot;')}">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                            <path d="m5 15-4-4 4-4"></path>
                        </svg>
                        Copy
                    </button>
                </div>
                <div class="codehilite"><pre><span></span><code{match.group(0).split('<code')[1].split('>')[0]}>{code_content}</code></pre></div>
            </div>'''
        
        return re.sub(code_block_pattern, add_copy_button, html_content, flags=re.DOTALL)
    
    def create_html_template(self, html_content, md_path):
        """Create enhanced HTML template with better styling and copy functionality"""
        
        # Determine page title
        page_title = "Kincir Documentation"
        if "README" in md_path:
            page_title = "Kincir - README"
        elif "index" in md_path:
            page_title = "Kincir - High-Performance Rust Message Streaming"
        
        return f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="Kincir - High-performance Rust library for unified message streaming across multiple broker backends">
    <meta name="keywords" content="rust, messaging, kafka, rabbitmq, mqtt, event-driven, microservices">
    <meta name="author" content="Kincir Team">
    <title>{page_title}</title>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism-tomorrow.min.css">
    <style>
        :root {{
            --primary-color: #3498db;
            --secondary-color: #2c3e50;
            --accent-color: #e74c3c;
            --success-color: #27ae60;
            --warning-color: #f39c12;
            --background-color: #ffffff;
            --surface-color: #f8f9fa;
            --text-color: #2c3e50;
            --text-muted: #6c757d;
            --border-color: #dee2e6;
            --code-bg: #2d3748;
            --shadow: 0 2px 4px rgba(0,0,0,0.1);
            --rust-orange: #ce422b;
            --rust-brown: #8b4513;
        }}
        
        * {{
            box-sizing: border-box;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', sans-serif;
            line-height: 1.7;
            max-width: 1200px;
            margin: 0 auto;
            padding: 0;
            color: var(--text-color);
            background-color: var(--background-color);
        }}
        
        .container {{
            padding: 20px;
        }}
        
        /* Navigation */
        .nav {{
            background: linear-gradient(135deg, var(--rust-brown), var(--rust-orange));
            color: white;
            padding: 15px 0;
            margin: 0;
            box-shadow: var(--shadow);
            position: sticky;
            top: 0;
            z-index: 100;
        }}
        
        .nav-content {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 0 20px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
        }}
        
        .nav-brand {{
            font-size: 1.5rem;
            font-weight: bold;
            color: white;
            text-decoration: none;
            display: flex;
            align-items: center;
            gap: 10px;
        }}
        
        .nav-links {{
            display: flex;
            gap: 20px;
            flex-wrap: wrap;
        }}
        
        .nav a {{
            color: white;
            text-decoration: none;
            padding: 8px 16px;
            border-radius: 4px;
            transition: background-color 0.3s ease;
        }}
        
        .nav a:hover {{
            background-color: rgba(255,255,255,0.2);
        }}
        
        /* Typography */
        h1, h2, h3, h4, h5, h6 {{
            color: var(--secondary-color);
            margin-top: 2em;
            margin-bottom: 0.5em;
            font-weight: 600;
        }}
        
        h1 {{
            font-size: 2.5rem;
            border-bottom: 3px solid var(--rust-orange);
            padding-bottom: 15px;
            margin-top: 1em;
        }}
        
        h2 {{
            font-size: 2rem;
            border-bottom: 2px solid var(--border-color);
            padding-bottom: 10px;
        }}
        
        h3 {{
            font-size: 1.5rem;
            color: var(--rust-orange);
        }}
        
        /* Links */
        a {{
            color: var(--rust-orange);
            text-decoration: none;
            transition: color 0.3s ease;
        }}
        
        a:hover {{
            color: var(--accent-color);
            text-decoration: underline;
        }}
        
        /* Code blocks with copy functionality */
        .code-block-container {{
            position: relative;
            margin: 1.5em 0;
            border-radius: 8px;
            overflow: hidden;
            box-shadow: var(--shadow);
        }}
        
        .code-block-header {{
            background: linear-gradient(135deg, #4a5568, #2d3748);
            padding: 10px 15px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid #4a5568;
        }}
        
        .copy-button {{
            background: rgba(255, 255, 255, 0.1);
            border: 1px solid rgba(255, 255, 255, 0.2);
            color: white;
            padding: 6px 12px;
            border-radius: 4px;
            cursor: pointer;
            font-size: 0.85em;
            display: flex;
            align-items: center;
            gap: 6px;
            transition: all 0.3s ease;
        }}
        
        .copy-button:hover {{
            background: rgba(255, 255, 255, 0.2);
            border-color: rgba(255, 255, 255, 0.3);
        }}
        
        .copy-button.copied {{
            background: var(--success-color);
            border-color: var(--success-color);
        }}
        
        .copy-button svg {{
            width: 14px;
            height: 14px;
        }}
        
        /* Enhanced code styling */
        .codehilite {{
            background-color: var(--code-bg);
            margin: 0;
        }}
        
        .codehilite pre {{
            background: none;
            margin: 0;
            padding: 20px;
            overflow-x: auto;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', 'Consolas', monospace;
            font-size: 0.9em;
            line-height: 1.5;
        }}
        
        .codehilite code {{
            background: none;
            padding: 0;
            border: none;
            color: #e2e8f0;
        }}
        
        /* Rust syntax highlighting */
        .highlight .k {{ color: #f56565; }} /* Keywords - red */
        .highlight .kn {{ color: #f56565; }} /* Keyword namespace */
        .highlight .kd {{ color: #f56565; }} /* Keyword declaration */
        .highlight .kt {{ color: #4299e1; }} /* Keyword type - blue */
        .highlight .s, .highlight .s2 {{ color: #68d391; }} /* Strings - green */
        .highlight .s1 {{ color: #68d391; }} /* Single quoted strings */
        .highlight .n {{ color: #e2e8f0; }} /* Names - light gray */
        .highlight .nc {{ color: #fbb6ce; }} /* Name class - pink */
        .highlight .nf {{ color: #90cdf4; }} /* Name function - light blue */
        .highlight .o {{ color: #f56565; }} /* Operators - red */
        .highlight .c, .highlight .c1 {{ color: #a0aec0; font-style: italic; }} /* Comments - gray */
        .highlight .cm {{ color: #a0aec0; font-style: italic; }} /* Multi-line comments */
        .highlight .mi {{ color: #fbb6ce; }} /* Numbers - pink */
        .highlight .mf {{ color: #fbb6ce; }} /* Float numbers */
        .highlight .nb {{ color: #4299e1; }} /* Name builtin - blue */
        .highlight .bp {{ color: #4299e1; }} /* Name builtin pseudo */
        
        /* Inline code */
        code {{
            background-color: #f7fafc;
            padding: 3px 6px;
            border-radius: 4px;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
            font-size: 0.9em;
            border: 1px solid var(--border-color);
            color: var(--rust-orange);
        }}
        
        /* Tables */
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 20px 0;
            box-shadow: var(--shadow);
            border-radius: 8px;
            overflow: hidden;
        }}
        
        th, td {{
            border: 1px solid var(--border-color);
            padding: 15px;
            text-align: left;
        }}
        
        th {{
            background: linear-gradient(135deg, var(--surface-color), #e9ecef);
            font-weight: 600;
            color: var(--secondary-color);
        }}
        
        tr:nth-child(even) {{
            background-color: var(--surface-color);
        }}
        
        tr:hover {{
            background-color: #fff5f5;
        }}
        
        /* Blockquotes */
        blockquote {{
            border-left: 4px solid var(--rust-orange);
            margin: 1.5em 0;
            padding: 15px 20px;
            background-color: var(--surface-color);
            border-radius: 0 8px 8px 0;
            font-style: italic;
            color: var(--text-muted);
        }}
        
        /* Lists */
        ul, ol {{
            padding-left: 30px;
            margin: 1em 0;
        }}
        
        li {{
            margin: 0.5em 0;
        }}
        
        /* Badges and status indicators */
        .badge {{
            display: inline-block;
            padding: 4px 12px;
            background-color: var(--rust-orange);
            color: white;
            border-radius: 20px;
            font-size: 0.8em;
            font-weight: 500;
            margin: 2px;
        }}
        
        .badge.success {{ background-color: var(--success-color); }}
        .badge.warning {{ background-color: var(--warning-color); }}
        .badge.danger {{ background-color: var(--accent-color); }}
        
        /* Horizontal rules */
        hr {{
            border: none;
            height: 2px;
            background: linear-gradient(90deg, var(--rust-orange), transparent);
            margin: 2em 0;
        }}
        
        /* Responsive design */
        @media (max-width: 768px) {{
            .container {{
                padding: 15px;
            }}
            
            .nav-content {{
                flex-direction: column;
                gap: 10px;
            }}
            
            .nav-links {{
                justify-content: center;
            }}
            
            h1 {{
                font-size: 2rem;
            }}
            
            h2 {{
                font-size: 1.5rem;
            }}
            
            table {{
                font-size: 0.9em;
            }}
            
            th, td {{
                padding: 10px;
            }}
            
            .code-block-header {{
                padding: 8px 12px;
            }}
            
            .copy-button {{
                padding: 4px 8px;
                font-size: 0.8em;
            }}
        }}
        
        /* Footer */
        .footer {{
            margin-top: 3em;
            padding: 2em 0;
            border-top: 1px solid var(--border-color);
            text-align: center;
            color: var(--text-muted);
            font-size: 0.9em;
        }}
        
        /* Toast notification for copy feedback */
        .toast {{
            position: fixed;
            top: 20px;
            right: 20px;
            background: var(--success-color);
            color: white;
            padding: 12px 20px;
            border-radius: 6px;
            box-shadow: var(--shadow);
            transform: translateX(100%);
            transition: transform 0.3s ease;
            z-index: 1000;
        }}
        
        .toast.show {{
            transform: translateX(0);
        }}
    </style>
</head>
<body>
    <nav class="nav">
        <div class="nav-content">
            <a href="/" class="nav-brand">
                🦀 Kincir
            </a>
            <div class="nav-links">
                <a href="/">Home</a>
                <a href="/docs/getting-started.html">Get Started</a>
                <a href="/examples/">Examples</a>
                <a href="/README.html">README</a>
                <a href="https://github.com/rezacute/kincir" target="_blank">GitHub</a>
                <a href="https://crates.io/crates/kincir" target="_blank">Crates.io</a>
                <a href="https://docs.rs/kincir" target="_blank">API Docs</a>
            </div>
        </div>
    </nav>
    
    <div class="container">
        {html_content}
        
        <div class="footer">
            <p>🦀 Kincir Documentation • Generated on {time.strftime('%Y-%m-%d %H:%M UTC')}</p>
            <p>Licensed under the Apache License, Version 2.0</p>
        </div>
    </div>
    
    <div id="toast" class="toast"></div>
    
    <script>
        function copyCode(button) {{
            const code = button.getAttribute('data-code');
            
            // Use the modern clipboard API if available
            if (navigator.clipboard && window.isSecureContext) {{
                navigator.clipboard.writeText(code).then(() => {{
                    showCopyFeedback(button);
                }}).catch(err => {{
                    console.error('Failed to copy: ', err);
                    fallbackCopyTextToClipboard(code, button);
                }});
            }} else {{
                fallbackCopyTextToClipboard(code, button);
            }}
        }}
        
        function fallbackCopyTextToClipboard(text, button) {{
            const textArea = document.createElement("textarea");
            textArea.value = text;
            textArea.style.top = "0";
            textArea.style.left = "0";
            textArea.style.position = "fixed";
            
            document.body.appendChild(textArea);
            textArea.focus();
            textArea.select();
            
            try {{
                const successful = document.execCommand('copy');
                if (successful) {{
                    showCopyFeedback(button);
                }}
            }} catch (err) {{
                console.error('Fallback: Oops, unable to copy', err);
            }}
            
            document.body.removeChild(textArea);
        }}
        
        function showCopyFeedback(button) {{
            // Update button
            const originalText = button.innerHTML;
            button.innerHTML = `
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="20,6 9,17 4,12"></polyline>
                </svg>
                Copied!
            `;
            button.classList.add('copied');
            
            // Show toast
            const toast = document.getElementById('toast');
            toast.textContent = 'Code copied to clipboard!';
            toast.classList.add('show');
            
            // Reset after 2 seconds
            setTimeout(() => {{
                button.innerHTML = originalText;
                button.classList.remove('copied');
                toast.classList.remove('show');
            }}, 2000);
        }}
        
        // Add smooth scrolling for anchor links
        document.addEventListener('DOMContentLoaded', function() {{
            const links = document.querySelectorAll('a[href^="#"]');
            links.forEach(link => {{
                link.addEventListener('click', function(e) {{
                    e.preventDefault();
                    const target = document.querySelector(this.getAttribute('href'));
                    if (target) {{
                        target.scrollIntoView({{
                            behavior: 'smooth',
                            block: 'start'
                        }});
                    }}
                }});
            }});
        }});
    </script>
</body>
</html>"""

def main():
    # Check if running as root (required for port 80)
    if PORT < 1024 and os.geteuid() != 0:
        print(f"Error: This script must be run as root to bind to port {PORT}")
        print("Please run: sudo python3 enhanced_docs_server.py")
        sys.exit(1)
    
    # Change to docs directory
    os.chdir(DOCS_DIR)
    
    print(f"🦀 Starting Kincir Enhanced Documentation Server on port {PORT}")
    print(f"📁 Serving from: {DOCS_DIR}")
    print(f"💾 Cache directory: {CACHE_DIR}")
    print(f"🌐 Access the documentation at: http://13.215.22.189")
    print("✨ Features: Copy buttons, Rust syntax highlighting, responsive design")
    print("Press Ctrl+C to stop the server")
    
    try:
        with socketserver.TCPServer(("", PORT), DocsHandler) as httpd:
            httpd.allow_reuse_address = True
            httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n🛑 Server stopped.")
    except PermissionError:
        print(f"❌ Error: Permission denied to bind to port {PORT}")
        print("Please run as root: sudo python3 enhanced_docs_server.py")
    except Exception as e:
        print(f"❌ Error starting server: {e}")

if __name__ == "__main__":
    main()
