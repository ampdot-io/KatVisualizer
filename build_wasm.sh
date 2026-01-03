#!/bin/bash
set -e
export PATH="$HOME/.cargo/bin:$PATH"
mkdir -p dist
cargo build --target wasm32-unknown-unknown --release --bin katvisualizer_wasm
wasm-bindgen target/wasm32-unknown-unknown/release/katvisualizer_wasm.wasm --out-dir dist --target web
cp web/index.html dist/
