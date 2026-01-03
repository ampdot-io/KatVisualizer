# KatVisualizer - WASM Web Version

This is the web-based version of KatVisualizer, compiled to WebAssembly for use in modern web browsers.

## Features

- **Audio File Support**: Load and visualize any audio file supported by your browser
- **Real-time Analysis**: Advanced audio analysis using ERB-scale VQT with spectral reassignment
- **Interactive Controls**: Adjust visualization parameters in real-time
- **Beautiful Rendering**: Gradient-based frequency visualization with configurable colors

## Building

### Prerequisites

1. **Rust**: Install from [rust-lang.org](https://rust-lang.org/tools/install/)
2. **wasm-pack**: Install with:
   ```bash
   curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
   ```

### Build Steps

1. Build the WASM module:
   ```bash
   chmod +x build-wasm.sh
   ./build-wasm.sh
   ```

2. Start a local web server (required due to WASM security restrictions):
   ```bash
   chmod +x test-wasm.sh
   ./test-wasm.sh
   ```

   Or manually:
   ```bash
   cd pkg
   python3 -m http.server 8080
   ```

3. Open your browser to `http://localhost:8080`

## Usage

1. **Load Audio**: Click "Choose Audio File" and select an audio file (MP3, WAV, OGG, etc.)
2. **Play**: Click the "Play" button to start playback and visualization
3. **Adjust Settings**: Open the "Analysis Settings" and "Render Settings" panels to customize the visualization

### Analysis Settings

- **Resolution**: Number of frequency bins (128-1024)
- **Gain**: Input gain adjustment in dB
- **Listening Volume**: Reference listening level for equal-loudness contours (dB SPL)
- **ERB Bandwidth Divisor**: Controls frequency resolution (lower = higher resolution)
- **ERB Frequency Scale**: Use ERB (Equivalent Rectangular Bandwidth) frequency spacing
- **ERB Time Resolution**: Use ERB-based time resolution
- **NC Method Windowing**: Enable Nuttall-Connes windowing method
- **Simultaneous Masking**: Apply psychoacoustic masking thresholds
- **Normalize Amplitude**: Apply ISO 226:2023 equal loudness normalization

### Render Settings

- **Color Hue**: Base color hue for the visualization (0-360)
- **Brightness**: Brightness multiplier
- **Bar Spacing**: Spacing between frequency bars in pixels
- **Show Masking Thresholds**: Display simultaneous masking thresholds

## Supported Audio Formats

The visualizer supports any audio format that your browser can decode, typically including:
- MP3
- WAV
- OGG/Vorbis
- M4A/AAC
- FLAC (in some browsers)

## Performance

The WASM version is optimized for performance but may be slower than the native plugin version. For best performance:

- Use a modern browser (Chrome, Firefox, Edge, Safari)
- Keep resolution at 512 or below for real-time visualization
- Disable masking if performance is poor
- Close other browser tabs to free up CPU

## Technical Details

The web version uses:
- **Rust compiled to WASM** for audio analysis
- **Web Audio API** for audio decoding and playback
- **HTML5 Canvas** for rendering
- **wasm-bindgen** for Rust-JavaScript interop

The same core audio analysis engine is used as in the VST3/CLAP plugin version, ensuring identical analysis quality.

## Troubleshooting

### WASM Module Won't Load

- Ensure you're serving the files over HTTP (not file://)
- Check browser console for errors
- Verify all files are in the pkg/ directory

### Audio Won't Play

- Check that your browser supports the audio file format
- Verify audio file isn't corrupted
- Check browser audio permissions

### Poor Performance

- Reduce resolution setting
- Disable simultaneous masking
- Try a different browser
- Close other applications

## Development

To modify the WASM version:

1. Edit Rust code in `src/wasm.rs` or core analyzer in `src/analyzer.rs`
2. Edit web interface in `web/` directory
3. Rebuild with `./build-wasm.sh`
4. Test with `./test-wasm.sh`

## License

Same license as the main KatVisualizer project.
