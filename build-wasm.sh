#!/bin/bash

set -e

echo "Building KatVisualizer for WASM..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack is not installed"
    echo "Please install it with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

# Build with wasm-pack
echo "Compiling Rust to WASM..."
wasm-pack build --target web --no-default-features --features wasm

# Copy web files to pkg directory for easy serving
echo "Copying web files..."
cp web/index.html pkg/
cp web/style.css pkg/
cp web/app.js pkg/

echo ""
echo "Build complete!"
echo "To run the visualizer:"
echo "  1. Install a local server (e.g., 'python3 -m http.server' or 'npx http-server')"
echo "  2. Navigate to the pkg/ directory"
echo "  3. Start the server: python3 -m http.server 8080"
echo "  4. Open http://localhost:8080 in your browser"
echo ""
