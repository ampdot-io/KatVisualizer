#!/bin/bash

set -e

echo "Building and testing KatVisualizer WASM..."

# Build
./build-wasm.sh

# Start a local server
echo ""
echo "Starting local server..."
echo "Open http://localhost:8080 in your browser"
echo "Press Ctrl+C to stop the server"
echo ""

cd pkg
python3 -m http.server 8080
