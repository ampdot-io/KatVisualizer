#!/bin/bash
# Build script for KatVisualizer WASM

set -e

echo "Building KatVisualizer for WASM..."

# Check if rustup is being used
if ! command -v rustup &> /dev/null; then
    echo "WARNING: rustup not found. You may need to install Rust via rustup."
    echo "Visit https://rustup.rs/"
    exit 1
fi

# Check if wasm32 target is installed
if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

# Build for web target
echo "Compiling to WASM..."
wasm-pack build --target web --out-dir pkg

echo ""
echo "✓ Build complete!"
echo ""
echo "Next steps:"
echo "  1. python3 -m http.server 8080"
echo "  2. Open http://localhost:8080/test.html to run tests"
echo "  3. Open http://localhost:8080/index.html for the app"
echo ""
