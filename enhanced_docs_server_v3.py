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
                
                # Configure markdown with basic extensions (no syntax highlighting)
                md = markdown.Markdown(extensions=[
                    'fenced_code', 
                    'tables',
                    'toc',
                    'footnotes',
                    'attr_list',
                    'def_list',
                    'abbr',
                    'admonition'
                ], extension_configs={
                    'toc': {
                        'permalink': True,
                        'permalink_title': 'Link to this section'
                    }
                })
                
                # Convert markdown to HTML
                html_content = md.convert(content)
                
                # Add copy buttons and Prism.js classes to code blocks
                html_content = self.enhance_code_blocks(html_content)
                
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
    
    def enhance_code_blocks(self, html_content):
        """Add copy buttons and Prism.js classes to code blocks"""
        # Pattern to match both fenced code blocks and highlight divs
        patterns = [
            # Pattern for simple fenced code blocks
            r'<pre><code([^>]*)>(.*?)</code></pre>',
            # Pattern for highlight divs (from Pygments)
            r'<div class="highlight"><pre><span></span><code([^>]*)>(.*?)</code></pre></div>'
        ]
        
        def enhance_code_block(match):
            if len(match.groups()) == 2:
                attrs = match.group(1)
                code_content = match.group(2)
            else:
                attrs = ""
                code_content = match.group(1)
            
            # Extract language from class attribute or detect from content
            language = "text"
            if 'class="language-' in attrs:
                lang_match = re.search(r'class="language-([^"]*)"', attrs)
                if lang_match:
                    language = lang_match.group(1)
            elif 'class="' in attrs:
                class_match = re.search(r'class="([^"]*)"', attrs)
                if class_match:
                    language = class_match.group(1)
            else:
                # Auto-detect language from code content
                if "fn main" in code_content or "use " in code_content or "let " in code_content or "async fn" in code_content:
                    language = "rust"
                elif "import " in code_content or "def " in code_content or "print(" in code_content:
                    language = "python"
                elif "function " in code_content or "const " in code_content or "var " in code_content:
                    language = "javascript"
                elif "curl " in code_content or "sudo " in code_content or "#!/bin/bash" in code_content:
                    language = "bash"
                elif "[dependencies]" in code_content or "cargo " in code_content:
                    language = "toml"
            
            # Extract plain text for copying
            plain_text = re.sub(r'<[^>]+>', '', code_content)
            plain_text = plain_text.replace('&lt;', '<').replace('&gt;', '>').replace('&amp;', '&').replace('&quot;', '"')
            
            return f'''<div class="code-block-container">
                <div class="code-block-header">
                    <span class="code-language">{language}</span>
                    <button class="copy-button" onclick="copyCode(this)" data-code="{plain_text.replace('"', '&quot;').replace("'", "&#39;")}">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                            <path d="m5 15-4-4 4-4"></path>
                        </svg>
                        Copy
                    </button>
                </div>
                <pre class="language-{language}"><code class="language-{language}">{code_content}</code></pre>
            </div>'''
        
        # Apply all patterns
        for pattern in patterns:
            html_content = re.sub(pattern, enhance_code_block, html_content, flags=re.DOTALL)
        
        return html_content
    
    def create_html_template(self, html_content, md_path):
        """Create enhanced HTML template with Prism.js syntax highlighting"""
        
        # Determine page title
        page_title = "Kincir Documentation"
        if "README" in md_path:
            page_title = "Kincir - README"
        elif "index" in md_path:
            page_title = "Kincir - High-Performance Rust Message Streaming"
        
        # Kincir logo SVG (inline for better performance)
        logo_svg = '''<svg width="32" height="32" viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
            <defs>
                <linearGradient x1="0%" y1="0%" x2="100%" y2="100%" id="windmillGradient">
                    <stop stop-color="#60a5fa" offset="0%"></stop>
                    <stop stop-color="#3b82f6" offset="100%"></stop>
                </linearGradient>
            </defs>
            <circle cx="100" cy="100" r="15" fill="#fbbf24"></circle>
            <g fill="url(#windmillGradient)">
                <path d="M100,85 L80,25 C65,30 60,40 65,50 L100,85 Z" transform="rotate(0 100 100)"></path>
                <path d="M100,85 L80,25 C65,30 60,40 65,50 L100,85 Z" transform="rotate(90 100 100)"></path>
                <path d="M100,85 L80,25 C65,30 60,40 65,50 L100,85 Z" transform="rotate(180 100 100)"></path>
                <path d="M100,85 L80,25 C65,30 60,40 65,50 L100,85 Z" transform="rotate(270 100 100)"></path>
            </g>
            <rect x="97" y="115" width="6" height="70" fill="#8b5cf6"></rect>
        </svg>'''
        
        return f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="Kincir - High-performance Rust library for unified message streaming across multiple broker backends">
    <meta name="keywords" content="rust, messaging, kafka, rabbitmq, mqtt, event-driven, microservices">
    <meta name="author" content="Kincir Team">
    <title>{page_title}</title>
    
    <!-- Prism.js CSS for syntax highlighting -->
    <link href="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism-tomorrow.min.css" rel="stylesheet">
    <link href="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/plugins/toolbar/prism-toolbar.min.css" rel="stylesheet">
    
    <!-- Google Fonts -->
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600&family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
    
    <style>
        :root {{
            --primary-color: #4169E1;
            --secondary-color: #1E3A8A;
            --accent-color: #3B82F6;
            --success-color: #10B981;
            --warning-color: #F59E0B;
            --background-color: #ffffff;
            --surface-color: #f8fafc;
            --text-color: #1e293b;
            --text-muted: #64748b;
            --border-color: #e2e8f0;
            --code-bg: #2d3748;
            --code-text: #e2e8f0;
            --shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
            --blue-gradient: linear-gradient(135deg, #4169E1, #1E3A8A);
        }}
        
        * {{
            box-sizing: border-box;
        }}
        
        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
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
            background: var(--blue-gradient);
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
            font-weight: 700;
            color: white;
            text-decoration: none;
            display: flex;
            align-items: center;
            gap: 12px;
        }}
        
        .nav-brand .logo-container {{
            width: 40px;
            height: 40px;
            background: white;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            border: 2px solid rgba(255, 255, 255, 0.3);
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
            transition: all 0.3s ease;
        }}
        
        .nav-brand:hover .logo-container {{
            transform: scale(1.05);
            border-color: rgba(255, 255, 255, 0.5);
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        }}
        
        .nav-brand svg {{
            width: 28px;
            height: 28px;
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
            border-radius: 6px;
            transition: all 0.3s ease;
            font-weight: 500;
        }}
        
        .nav a:hover {{
            background-color: rgba(255,255,255,0.2);
            transform: translateY(-1px);
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
            border-bottom: 3px solid var(--primary-color);
            padding-bottom: 15px;
            margin-top: 1em;
            background: var(--blue-gradient);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}
        
        h2 {{
            font-size: 2rem;
            border-bottom: 2px solid var(--border-color);
            padding-bottom: 10px;
            color: var(--primary-color);
        }}
        
        h3 {{
            font-size: 1.5rem;
            color: var(--accent-color);
        }}
        
        /* Links */
        a {{
            color: var(--primary-color);
            text-decoration: none;
            transition: color 0.3s ease;
        }}
        
        a:hover {{
            color: var(--secondary-color);
            text-decoration: underline;
        }}
        
        /* Enhanced code blocks with Prism.js */
        .code-block-container {{
            position: relative;
            margin: 1.5em 0;
            border-radius: 12px;
            overflow: hidden;
            box-shadow: var(--shadow);
            border: 1px solid #374151;
        }}
        
        .code-block-header {{
            background: linear-gradient(135deg, #374151, #1f2937);
            padding: 12px 20px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid #4b5563;
        }}
        
        .code-language {{
            color: #9ca3af;
            font-size: 0.85em;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}
        
        .copy-button {{
            background: rgba(59, 130, 246, 0.1);
            border: 1px solid rgba(59, 130, 246, 0.3);
            color: #60a5fa;
            padding: 8px 16px;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.85em;
            display: flex;
            align-items: center;
            gap: 8px;
            transition: all 0.3s ease;
            font-weight: 500;
        }}
        
        .copy-button:hover {{
            background: rgba(59, 130, 246, 0.2);
            border-color: rgba(59, 130, 246, 0.5);
            transform: translateY(-1px);
        }}
        
        .copy-button.copied {{
            background: rgba(16, 185, 129, 0.2);
            border-color: rgba(16, 185, 129, 0.5);
            color: #34d399;
        }}
        
        .copy-button svg {{
            width: 16px;
            height: 16px;
        }}
        
        /* Override Prism.js styles for better integration */
        pre[class*="language-"] {{
            margin: 0 !important;
            padding: 24px !important;
            background: #2d3748 !important;
            border-radius: 0 !important;
            font-family: 'JetBrains Mono', 'Fira Code', 'Monaco', 'Menlo', monospace !important;
            font-size: 0.9em !important;
            line-height: 1.6 !important;
        }}
        
        code[class*="language-"] {{
            font-family: 'JetBrains Mono', 'Fira Code', 'Monaco', 'Menlo', monospace !important;
            font-size: 0.9em !important;
        }}
        
        /* Inline code */
        code:not([class*="language-"]) {{
            background-color: #f1f5f9;
            padding: 4px 8px;
            border-radius: 6px;
            font-family: 'JetBrains Mono', 'Fira Code', 'Monaco', 'Menlo', monospace;
            font-size: 0.9em;
            border: 1px solid var(--border-color);
            color: var(--primary-color);
            font-weight: 500;
        }}
        
        /* Tables */
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 20px 0;
            box-shadow: var(--shadow);
            border-radius: 12px;
            overflow: hidden;
            border: 1px solid var(--border-color);
        }}
        
        th, td {{
            border: 1px solid var(--border-color);
            padding: 16px;
            text-align: left;
        }}
        
        th {{
            background: var(--blue-gradient);
            color: white;
            font-weight: 600;
        }}
        
        tr:nth-child(even) {{
            background-color: var(--surface-color);
        }}
        
        tr:hover {{
            background-color: #eff6ff;
        }}
        
        /* Blockquotes */
        blockquote {{
            border-left: 4px solid var(--primary-color);
            margin: 1.5em 0;
            padding: 20px 24px;
            background: linear-gradient(135deg, #eff6ff, #dbeafe);
            border-radius: 0 12px 12px 0;
            font-style: italic;
            color: var(--text-muted);
            box-shadow: var(--shadow);
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
            padding: 6px 16px;
            background: var(--blue-gradient);
            color: white;
            border-radius: 20px;
            font-size: 0.8em;
            font-weight: 600;
            margin: 2px;
            box-shadow: var(--shadow);
        }}
        
        .badge.success {{ background: linear-gradient(135deg, #10B981, #059669); }}
        .badge.warning {{ background: linear-gradient(135deg, #F59E0B, #D97706); }}
        .badge.danger {{ background: linear-gradient(135deg, #EF4444, #DC2626); }}
        
        /* Horizontal rules */
        hr {{
            border: none;
            height: 2px;
            background: var(--blue-gradient);
            margin: 2em 0;
            border-radius: 1px;
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
            
            .nav-brand .logo-container {{
                width: 36px;
                height: 36px;
            }}
            
            .nav-brand svg {{
                width: 24px;
                height: 24px;
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
                padding: 12px;
            }}
            
            .code-block-header {{
                padding: 10px 16px;
            }}
            
            .copy-button {{
                padding: 6px 12px;
                font-size: 0.8em;
            }}
            
            pre[class*="language-"] {{
                padding: 16px !important;
                font-size: 0.85em !important;
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
            border-radius: 8px;
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
                <div class="logo-container">
                    {logo_svg}
                </div>
                Kincir
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
            <p>⚡ Kincir Documentation • Generated on {time.strftime('%Y-%m-%d %H:%M UTC')}</p>
            <p>Licensed under the Apache License, Version 2.0</p>
        </div>
    </div>
    
    <div id="toast" class="toast"></div>
    
    <!-- Prism.js JavaScript for syntax highlighting -->
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-core.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/plugins/autoloader/prism-autoloader.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/plugins/toolbar/prism-toolbar.min.js"></script>
    
    <script>
        // Configure Prism.js autoloader
        Prism.plugins.autoloader.languages_path = 'https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/';
        
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
            
            // Initialize Prism.js highlighting
            if (typeof Prism !== 'undefined') {{
                Prism.highlightAll();
            }}
        }});
    </script>
</body>
</html>"""

def main():
    # Check if running as root (required for port 80)
    if PORT < 1024 and os.geteuid() != 0:
        print(f"Error: This script must be run as root to bind to port {PORT}")
        print("Please run: sudo python3 enhanced_docs_server_v3.py")
        sys.exit(1)
    
    # Change to docs directory
    os.chdir(DOCS_DIR)
    
    print(f"🌪️ Starting Kincir Enhanced Documentation Server v3 on port {PORT}")
    print(f"📁 Serving from: {DOCS_DIR}")
    print(f"💾 Cache directory: {CACHE_DIR}")
    print(f"🌐 Access the documentation at: http://13.215.22.189")
    print("✨ Features: Prism.js syntax highlighting, Copy buttons, Blue theme, Logo")
    print("Press Ctrl+C to stop the server")
    
    try:
        with socketserver.TCPServer(("", PORT), DocsHandler) as httpd:
            httpd.allow_reuse_address = True
            httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n🛑 Server stopped.")
    except PermissionError:
        print(f"❌ Error: Permission denied to bind to port {PORT}")
        print("Please run as root: sudo python3 enhanced_docs_server_v3.py")
    except Exception as e:
        print(f"❌ Error starting server: {e}")

if __name__ == "__main__":
    main()
