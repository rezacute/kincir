#!/bin/bash

echo "=== Kincir Documentation Server Status ==="
echo

echo "1. Checking nginx status:"
sudo systemctl is-active nginx
echo

echo "2. Checking kincir-docs service status:"
sudo systemctl is-active kincir-docs
echo

echo "3. Checking port 80 (nginx):"
ss -tlnp | grep :80 | head -1
echo

echo "4. Checking port 8080 (docs server):"
ss -tlnp | grep :8080 | head -1
echo

echo "5. Testing HTTP response on port 80:"
curl -s -o /dev/null -w "HTTP Status: %{http_code}\n" http://localhost:80/
echo

echo "6. Testing README page:"
curl -s -o /dev/null -w "HTTP Status: %{http_code}\n" http://localhost:80/README.html
echo

echo "7. Public IP address:"
curl -s http://checkip.amazonaws.com/
echo

echo "=== Documentation is available at: ==="
echo "Local: http://localhost:80/"
echo "Public: http://$(curl -s http://checkip.amazonaws.com/):80/"
echo
echo "Available pages:"
echo "- Main page: /"
echo "- README: /README.html"
echo "- Index: /index.html"
