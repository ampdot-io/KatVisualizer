# KatVisualizer WASM

A realtime music visualizer ported to WebAssembly. This is a web version of KatVisualizer that runs entirely in your browser.

## Features

- **ERB-scale VQT (Variable Q Transform)** with ERB-based bandwidths
- **NC method windowing** and spectral reassignment
- **Simultaneous masking thresholds** calculation
- **Realtime microphone input** via Web Audio API
- **Pure WASM implementation** with minimal JavaScript

## Building

### Prerequisites

- Rust toolchain with rustup (install from https://rustup.rs/)
  - **Important**: Use rustup, not Homebrew Rust
  - Add wasm32 target: `rustup target add wasm32-unknown-unknown`
- wasm-pack (will be auto-installed by build script)

### Build Steps

1. Run the build script:
```bash
cd wasm
chmod +x build.sh
./build.sh
```

2. Serve the files with any HTTP server:
```bash
python3 -m http.server 8080
```

3. Run tests and verify functionality:
   - Open http://localhost:8080/test.html - automated test suite
   - Open http://localhost:8080/debug.html - interactive debug viewer
   - Open http://localhost:8080/index.html - full application

### Manual Build

```bash
cargo install wasm-pack
rustup target add wasm32-unknown-unknown
wasm-pack build --target web --out-dir pkg
```

## Testing

See [TESTING.md](TESTING.md) for detailed testing instructions.

Quick test:
1. Build the project
2. Serve with `python3 -m http.server 8080`
3. Open http://localhost:8080/test.html
4. All tests should pass (green checkmarks)

## Usage

1. Open the webpage in a modern browser (Chrome, Firefox, or Edge recommended)
2. Click "Start Microphone" and allow microphone access
3. Play some music or make sounds!
4. Adjust gain and resolution sliders to your preference

## Controls

- **Gain**: Adjust input amplification (-40 to +40 dB)
- **Resolution**: Number of frequency bins (64 to 1024)

## How It Works

The visualizer uses advanced psychoacoustic processing:

1. **Audio Input**: Captures microphone input via Web Audio API
2. **VQsDFT Transform**: Efficient Variable-Q Sliding DFT with NC method
3. **Masking**: Calculates simultaneous masking thresholds
4. **Rendering**: Draws frequency spectrum with perceptually-weighted colors

The entire DSP chain runs in WebAssembly for maximum performance.

## Browser Requirements

- Modern browser with WebAssembly support
- Microphone access
- Web Audio API support

## License

Same as parent project (see LICENSE file in root directory)
