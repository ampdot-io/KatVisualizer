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

# Generate JS bindings
echo "Generating bindings..."
wasm-bindgen target/wasm32-unknown-unknown/release/katvisualizer_wasm.wasm --out-dir . --target web

echo "Build complete. Serve the directory with a web server."
