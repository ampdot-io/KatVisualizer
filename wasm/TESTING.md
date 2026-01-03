# Testing Guide for KatVisualizer WASM

## Running Tests

### 1. Build the WASM module

```bash
cd wasm
./build.sh
```

### 2. Serve the test files

```bash
python3 -m http.server 8080
```

### 3. Open test pages

- **Automated tests**: http://localhost:8080/test.html
- **Debug viewer**: http://localhost:8080/debug.html
- **Main app**: http://localhost:8080/index.html

## Test Pages

### render-check.html (NEW - RECOMMENDED)
**Quick rendering validation page** - Run this first!

5 comprehensive tests:
1. Canvas Comparison - Does rendering change the canvas?
2. Pixel Analysis - How many pixels are modified?
3. Bar Height Verification - Do bars have visible height?
4. Spectrum Data Validation - Is analyzer returning values?
5. Final Verdict - Overall pass/fail

**Expected output**: "ALL TESTS PASSED - Visualizer is working correctly! ✓✓✓"

**If broken**: "CRITICAL: Only 0/5 tests passed - Visualizer is BROKEN"

This page will immediately tell you if rendering works at all.

### test.html
Automated test suite that verifies:

**Data Tests (1-10):**
- WASM module loading
- Frequency bin generation
- Silence analysis (should return ~0 values)
- Sine wave analysis (should detect 440 Hz peak)
- Multi-frequency signal analysis
- Gain control
- Different resolutions (64, 256, 512 bins)
- Memory stability

**Visual Rendering Tests (11-18):**
- **Test 11**: Canvas comparison - CRITICAL baseline check
  - Compares blank canvas vs rendered canvas
  - FAILS immediately if nothing renders
  - This catches "everything is black" bugs
- **Test 12**: Silence renders as dark/black
- **Test 13**: Sine wave renders with visible bars - STRICT VALIDATION
  - Checks 3 criteria: hasColor, hasSignificantContent, brightPixelCount
  - Shows detailed pixel analysis
  - FAILS if max brightness < 80 or < 100 bright pixels
- **Test 14**: Peak appears at correct visual position
- **Test 15**: Rainbow color gradient across spectrum
- **Test 16**: Multiple frequencies show as separate visual peaks
- **Test 17**: Gain increases visual brightness
- **Test 18**: Frequency labels render at correct positions

**Expected output**: All tests should pass (green checkmarks) with visual spectrum renderings displayed

**Critical**: If Test 11 fails ("Canvas unchanged"), rendering is completely broken and all other visual tests will fail.

Each visual test creates a canvas showing the actual rendered visualization, allowing both automated verification and manual inspection.

See [TEST_IMPROVEMENTS.md](TEST_IMPROVEMENTS.md) for details on how tests detect rendering failures.

### debug.html
Interactive debug viewer with:
- Microphone input visualization
- Test signal generators (440 Hz sine, silence)
- Real-time spectrum display
- Debug log output

**How to use**:
1. Click "Test 440 Hz Sine" - should show a peak around 440 Hz
2. Click "Test Silence" - should show flat/near-zero spectrum
3. Click "Start" for microphone input (requires permission)

### index.html
Full application with:
- Real-time microphone visualization
- Gain and resolution controls
- FPS counter
- Frequency labels

## Common Issues

### No visualization appears
**Quick diagnosis**: Open http://localhost:8080/render-check.html

If Test 1 fails ("Canvas changes after rendering"):
- Rendering is completely broken
- Check WASM module built correctly
- Check browser console for errors
- Try rebuilding: `./build.sh`

If all render-check tests pass but index.html shows nothing:
- Check browser console for errors
- Verify microphone permissions
- Check Web Audio API initialization

### Spectrum shows all zeros
**Quick diagnosis**: Run render-check.html Test 4 ("Spectrum has signal")

If max spectrum value < 0.01:
- WASM analyzer is broken
- Check that normalization formula is correct
- Rebuild with `./build.sh` to get latest fixes

### Tests pass but visualization looks wrong
**Quick diagnosis**: Run test.html Tests 11-18

- Test 11 fails: Nothing renders at all
- Test 13 fails: Bars too dim or missing
- Test 14 fails: Peak in wrong position
- Test 15 fails: Colors not working

### Peak detection doesn't work
- Ensure you're using test signals with sufficient amplitude (0.3-0.5)
- Check that resolution is appropriate for frequency (higher resolution = better frequency accuracy)
- Run render-check.html to verify bars are drawn

### Performance issues
- Reduce resolution (try 128 or 64 bins)
- Close other tabs/applications
- Use Chrome/Edge for best WASM performance

## Manual Testing Checklist

**Build and Data Tests:**
- [ ] Build completes without errors
- [ ] test.html shows all data tests passing (Tests 1-10)

**Visual Rendering Tests (test.html):**
- [ ] Silence canvas shows dark/black bars
- [ ] 440 Hz sine canvas shows colored peak
- [ ] Peak appears in correct position (around middle of spectrum)
- [ ] Rainbow colors visible across spectrum (red-orange-yellow-green-blue)
- [ ] Multi-frequency canvas shows 2+ distinct peaks
- [ ] High gain canvas is brighter than normal gain
- [ ] Frequency labels visible and positioned correctly

**Interactive Tests (debug.html):**
- [ ] "Test 440 Hz Sine" shows visual peak near 440 Hz
- [ ] "Test Silence" shows flat dark spectrum
- [ ] Microphone input shows live visualization
- [ ] Debug console shows detailed output

**Main Application (index.html):**
- [ ] Real-time visualization displays with microphone
- [ ] Gain slider affects visualization brightness
- [ ] Resolution slider changes number of bars
- [ ] FPS counter shows ~60 fps
- [ ] Frequency labels appear at bottom
- [ ] Rainbow colors visible across spectrum
- [ ] No console errors during normal operation

## Debugging Tips

### Enable verbose logging
Open browser console (F12) and check for:
- WASM loading messages
- Error messages
- Performance warnings

### Check spectrum data
In console, after tests run:
```javascript
// In test.html or debug.html
const samples = new Float32Array(4096);
for (let i = 0; i < samples.length; i++) {
    samples[i] = Math.sin(2 * Math.PI * 440 * i / 48000) * 0.5;
}
const result = visualizer.analyze(samples);
console.log('Result length:', result.length);
console.log('Non-zero values:', result.filter(v => v > 0.01).length);
console.log('Max value:', Math.max(...result));
```

### Verify frequency bins
```javascript
const freqs = visualizer.get_frequencies();
console.log('Freq count:', freqs.length);
console.log('Range:', freqs[0], 'to', freqs[freqs.length-1]);
```
