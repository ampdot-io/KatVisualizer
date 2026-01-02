# KatVisualizer - Web Version

This directory contains a WebAssembly port of KatVisualizer that runs entirely in your web browser.

## Quick Start

```bash
cd wasm
./build.sh
python3 -m http.server 8080
```

Then open http://localhost:8080 in your browser.

## What's Included

The web version includes:

- **Full DSP pipeline** ported to WASM (ERB-scale VQT, masking, NC method)
- **Realtime microphone input** via Web Audio API
- **Interactive controls** for gain and resolution
- **Minimal JavaScript** - ~150 lines total
- **Single HTML page** - no build tools required for deployment
- **55KB WASM module** - optimized for fast loading

## Features Ported

✅ ERB-scale Variable Q Transform (VQsDFT)
✅ NC method windowing and spectral reassignment
✅ Simultaneous masking threshold calculation
✅ Perceptually-weighted frequency analysis
✅ Realtime processing (~60 FPS)

## Technical Details

### Architecture

```
┌─────────────┐
│ Web Audio   │ Microphone/File Input
│    API      │
└──────┬──────┘
       │ Float32Array samples
       ▼
┌─────────────┐
│    WASM     │ VQsDFT + Masking
│  Analyzer   │ (Rust compiled to WASM)
└──────┬──────┘
       │ Spectrum data
       ▼
┌─────────────┐
│  Canvas     │ HTML5 Canvas
│  Renderer   │ Rainbow colorization
└─────────────┘
```

### Performance

- **WASM module size**: 55KB (uncompressed)
- **Typical FPS**: 60 fps on modern hardware
- **Latency**: < 100ms (depends on buffer size)
- **Memory**: ~2MB typical usage

### Browser Compatibility

Tested on:
- Chrome 90+
- Firefox 88+
- Edge 90+
- Safari 14+ (with limitations)

## Differences from Plugin Version

The web version has some simplifications:

- **Mono only** - No stereo panning (Web Audio provides mono mic input)
- **No MIDI output** - Web MIDI API not integrated yet
- **No OSC output** - Network restrictions in browsers
- **Simplified controls** - Just gain and resolution sliders
- **Fixed update rate** - Determined by requestAnimationFrame (~60 Hz)

## Building

See `wasm/README.md` for detailed build instructions.

## License

Same as main project.
