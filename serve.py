#!/usr/bin/env python3
import http.server
import socketserver
import os
import sys
import socket

# Try port 8080 instead of 8000
PORT = 8080
DIRECTORY = "_site"

def find_available_port(start_port):
    """Find an available port by incrementing from the start port"""
    port = start_port
    max_port = start_port + 100  # Don't search forever
    
    while port < max_port:
        try:
            # Try to create a socket on the port
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.bind(('', port))
                return port
        except OSError:
            # Port is in use, try the next one
            port += 1
    
    raise RuntimeError(f"Could not find an available port in range {start_port}-{max_port}")

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=DIRECTORY, **kwargs)
    
    def end_headers(self):
        # Add CORS headers
        self.send_header('Access-Control-Allow-Origin', '*')
        super().end_headers()

if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    
    if not os.path.isdir(DIRECTORY):
        print(f"Error: Directory '{DIRECTORY}' does not exist.")
        sys.exit(1)
    
    # Find an available port
    try:
        port = find_available_port(PORT)
    except RuntimeError as e:
        print(f"Error: {e}")
        sys.exit(1)
        
    handler = Handler
    
    try:
        with socketserver.TCPServer(("", port), handler) as httpd:
            print(f"Serving at http://localhost:{port}")
            print(f"Serving directory: {os.path.abspath(DIRECTORY)}")
            print("Press Ctrl+C to stop the server")
            
            try:
                httpd.serve_forever()
            except KeyboardInterrupt:
                print("\nServer stopped.")
                httpd.server_close()
    except OSError as e:
        print(f"Error starting server: {e}")
        sys.exit(1) 