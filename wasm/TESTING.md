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

### test.html
Automated test suite that verifies:

**Data Tests:**
- WASM module loading
- Frequency bin generation
- Silence analysis (should return ~0 values)
- Sine wave analysis (should detect 440 Hz peak)
- Multi-frequency signal analysis
- Gain control
- Different resolutions (64, 256, 512 bins)
- Memory stability

**Visual Rendering Tests:**
- Canvas renders silence as dark/black
- Canvas renders sine wave with visible color
- Peak appears at correct visual position
- Rainbow color gradient across spectrum
- Multiple frequencies show as separate visual peaks
- Gain increases visual brightness
- Frequency labels render at correct positions

**Expected output**: All tests should pass (green checkmarks) with visual spectrum renderings displayed

Each visual test creates a canvas showing the actual rendered visualization, allowing both automated verification and manual inspection

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
- Check browser console for errors
- Verify WASM module loaded (look for "Running" button status)
- Check microphone permissions

### Spectrum shows all zeros
- This was fixed - normalization formula was incorrect
- Rebuild with `./build.sh` to get latest fixes

### Peak detection doesn't work
- Ensure you're using test signals with sufficient amplitude (0.3-0.5)
- Check that resolution is appropriate for frequency (higher resolution = better frequency accuracy)

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
