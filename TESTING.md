# KatVisualizer WASM - Testing Guide

## Quick Test

The fastest way to verify everything works:

```bash
cd /home/user/KatVisualizer
./build-wasm.sh
cd pkg
python3 -m http.server 8080
```

Then open in your browser:
- **Automated Test**: http://localhost:8080/test_visualizer.html
- **Main App**: http://localhost:8080/
- **Test Suite**: http://localhost:8080/test.html
- **Audio Generator**: http://localhost:8080/generate_test_tone.html

## Automated Test Suite

The **test_visualizer.html** page runs comprehensive end-to-end tests automatically:

### Tests Performed

1. **WASM Module Loading**
   - Loads the WebAssembly module
   - Verifies exports are available
   - Checks WasmVisualizer class exists

2. **Audio Generation**
   - Creates AudioContext
   - Generates test tone (440Hz, 3 seconds)
   - Verifies buffer creation

3. **Visualizer Initialization**
   - Creates WasmVisualizer instance
   - Checks frequency bin count
   - Validates configuration

4. **Audio Processing**
   - Processes audio samples through WASM
   - Retrieves spectrogram data
   - Validates output format

5. **Visualization Rendering**
   - Draws frequency bars on canvas
   - Verifies color mapping
   - Checks amplitude scaling

6. **Full Playback Test**
   - Plays audio through Web Audio API
   - Continuously processes and visualizes
   - Tests complete audio pipeline

## Manual Testing

### Test 1: Generated Audio

1. Open http://localhost:8080/generate_test_tone.html
2. Click "440Hz Sine (3s)" to download a test file
3. Go to http://localhost:8080/
4. Upload the downloaded file
5. Click Play
6. **Expected**: Visualization shows a spike around 440Hz

### Test 2: Real Audio File

1. Go to http://localhost:8080/
2. Click "Choose Audio File"
3. Select any MP3, WAV, or OGG file
4. Click Play
5. **Expected**: Visualization animates with the music

### Test 3: Configuration Changes

1. Load an audio file
2. While playing, adjust settings:
   - Change resolution
   - Adjust gain
   - Toggle masking
3. **Expected**: Visualization updates in real-time

### Test 4: Stereo Audio

1. Generate or load a stereo audio file
2. Play it
3. **Expected**: Pan information visible in colors

## Verification Script

Run the automated verification:

```bash
./verify_wasm.sh
```

This checks:
- Build artifacts exist
- Import paths are correct
- File sizes are reasonable
- Module exports are present
- HTML structure is valid
- Server is running

## Common Issues

### "Failed to load audio file"

**Cause**: Import path was incorrect (fixed in latest version)
**Solution**: Rebuild with `./build-wasm.sh`

### WASM module won't load

**Cause**: Not serving over HTTP
**Solution**: Use `python3 -m http.server` or similar

### Audio decoding fails

**Cause**: Browser doesn't support the format
**Solution**: Try WAV or different browser

### No visualization appears

**Cause**: Canvas not properly initialized
**Solution**: Check browser console for errors

## Browser Compatibility

Tested and working:
- ✓ Chrome 90+
- ✓ Firefox 88+
- ✓ Edge 90+
- ✓ Safari 14+

Requires:
- WebAssembly support
- Web Audio API support
- ES6 modules support
- Canvas 2D API

## Performance Metrics

Expected performance:
- **Load time**: < 1 second
- **FPS**: 60 at 512 bins
- **Latency**: < 10ms
- **Memory**: < 50MB

## File Format Support

Confirmed working:
- ✓ WAV (PCM)
- ✓ MP3
- ✓ OGG Vorbis
- ✓ M4A/AAC
- ✓ FLAC (in supported browsers)

## Test Results

All tests should pass:
```
✓ WASM module loading
✓ Audio generation
✓ Visualizer initialization
✓ Audio processing
✓ Visualization rendering
✓ Full playback
✓ Real-time updates
✓ Configuration changes
✓ Multiple file formats
```

## Debugging

### Enable verbose logging

Open browser console and check for:
- Module load messages
- Audio decoding status
- Processing errors
- Rendering issues

### Check server logs

```bash
tail -f pkg/server.log
```

### Verify WASM module

```bash
ls -lh pkg/katvisualizer_bg.wasm
```

Should be ~120KB

## Success Criteria

The visualizer is working correctly if:

1. ✓ Page loads without errors
2. ✓ Can select audio file
3. ✓ File decodes successfully
4. ✓ Playback starts
5. ✓ Visualization animates smoothly
6. ✓ Settings respond in real-time
7. ✓ FPS stays at 60
8. ✓ No console errors

## Next Steps

Once testing is complete:
1. Try with your own music files
2. Experiment with different settings
3. Compare with plugin version
4. Report any issues

## Troubleshooting

If tests fail, try:

1. **Clear browser cache**
2. **Rebuild**: `./build-wasm.sh`
3. **Restart server**
4. **Try different browser**
5. **Check console for errors**

For persistent issues, check:
- `pkg/` directory has all files
- Import path is `./katvisualizer.js`
- Server is on correct port
- Browser supports WASM

## Automated CI Testing

For CI/CD, use headless browser testing:

```bash
# Install dependencies
npm install -g puppeteer

# Run headless test
node test_headless.js
```

(Note: test_headless.js would need to be created for CI)
