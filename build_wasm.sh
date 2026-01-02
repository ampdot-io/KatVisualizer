#!/bin/bash
set -e

# Install wasm-bindgen-cli if not present
# Always ensure we have the version matching our dependency
if ! command -v wasm-bindgen &> /dev/null || [[ "$(wasm-bindgen --version)" != "wasm-bindgen 0.2.106" ]]; then
    echo "Installing wasm-bindgen-cli 0.2.106..."
    cargo install -f wasm-bindgen-cli --version 0.2.106
fi

# Build the WASM binary
echo "Building WASM binary..."
cargo build --release --target wasm32-unknown-unknown --bin katvisualizer_wasm --features wasm --no-default-features

# Prepare dist directory
rm -rf dist
mkdir -p dist

# Generate JS bindings
echo "Generating bindings..."
wasm-bindgen target/wasm32-unknown-unknown/release/katvisualizer_wasm.wasm --out-dir dist --target web

# Copy index.html
cp index.html dist/

echo "Build complete. Serve the 'dist' directory with a web server."
