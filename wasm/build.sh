#!/bin/bash
# Build script for KatVisualizer WASM

set -e

echo "Building KatVisualizer for WASM..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

# Build for web target
wasm-pack build --target web --out-dir pkg

echo "Build complete! Open index.html in a web server to run."
echo "For example: python3 -m http.server 8080"
