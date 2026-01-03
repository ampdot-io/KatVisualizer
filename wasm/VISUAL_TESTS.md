# Visual Rendering Tests

This document describes the comprehensive visual testing added to verify that the WASM visualizer actually renders correctly to canvas.

## Why Visual Tests?

The original tests only checked **data output** - they verified numbers were correct but didn't check if anything was actually **drawn on screen**. A bug could make all the data correct but render a black screen.

Visual tests solve this by:
1. Actually rendering to canvas
2. Reading back pixel data
3. Verifying colors, brightness, and positions
4. Allowing manual inspection

## Test Coverage

### Test 11: Silence Rendering ✓
**Verifies**: Silence shows as dark/black canvas

```javascript
- Renders silence (all zeros) to canvas
- Checks average pixel brightness < 15
- Ensures no unexpected bright pixels
```

**Pass criteria**: Dark canvas, avg brightness < 15

### Test 12: Sine Wave Rendering ✓
**Verifies**: 440 Hz sine shows visible color

```javascript
- Renders 440 Hz sine wave
- Checks pixels have color (not all black)
- Verifies brightness matches signal strength
```

**Pass criteria**: Visible colors, avg brightness > 15

### Test 13: Peak Position Verification ✓
**Verifies**: Peak appears at correct visual location

```javascript
- Finds peak bin in data
- Maps to pixel X coordinate
- Checks that region has high brightness
```

**Pass criteria**: Peak region brightness > 100

### Test 14: Color Gradient Verification ✓
**Verifies**: Rainbow colors across spectrum

```javascript
- Samples colors at 10 points across width
- Calculates color variance
- Counts unique colors
```

**Pass criteria**:
- Color variance > 100 (gradient detected)
- Multiple unique colors

### Test 15: Multi-Frequency Visual Separation ✓
**Verifies**: Multiple peaks show as separate visual peaks

```javascript
- Renders 220 Hz + 880 Hz mix
- Finds local maxima in pixel brightness
- Counts distinct visual peaks
```

**Pass criteria**: ≥ 2 visual peaks detected

### Test 16: Gain Visual Effect ✓
**Verifies**: Gain slider increases brightness

```javascript
- Renders at 0 dB gain
- Renders same signal at +20 dB gain
- Compares average brightness
```

**Pass criteria**: High gain brightness > normal gain brightness

### Test 17: Frequency Label Positioning ✓
**Verifies**: Labels render at correct positions

```javascript
- Renders spectrum with frequency labels
- Labels: 20, 100, 200, 500, 1k, 2k, 5k, 10k, 20k
- Visual inspection required
```

**Pass criteria**: Labels visible and correctly positioned

## Visual Output

Each test creates a **visible canvas** showing:
- Test name as heading
- Actual rendered visualization
- Allows manual inspection

Example output structure:
```
┌─────────────────────────────┐
│ Silence Test                │
│ ┌─────────────────────────┐ │
│ │ [dark black bars]       │ │
│ │                         │ │
│ └─────────────────────────┘ │
└─────────────────────────────┘

✓ Silence renders correctly (avg brightness: 8.42)
```

## Helper Functions

### Canvas Creation
```javascript
createCanvas(id, width, height)
// Creates labeled canvas section in test results
```

### Rendering
```javascript
drawSpectrum(canvas, spectrum, frequencies, title)
// Draws spectrum bars with rainbow colors

drawSpectrumWithLabels(canvas, spectrum, frequencies)
// Adds frequency labels (20Hz, 1kHz, etc.)
```

### Pixel Analysis
```javascript
checkCanvasPixels(canvas)
// Returns: { avgBrightness, hasColor, colorPixelRatio }

checkRegionBrightness(canvas, xStart, xEnd)
// Returns average brightness of region

verifyColorGradient(canvas)
// Returns: { hasGradient, uniqueColors, avgRed, avgGreen, avgBlue }
```

### Peak Detection
```javascript
findPeakBin(spectrum)
// Finds highest value in spectrum data

findVisualPeaks(canvas, frequencies)
// Finds local maxima in rendered pixels
// Returns: [{ x, brightness }, ...]
```

## Running Visual Tests

```bash
# 1. Build
cd wasm
./build.sh

# 2. Serve
python3 -m http.server 8080

# 3. Open in browser
http://localhost:8080/test.html
```

## Expected Results

You should see:
1. **10 data tests** - All green checkmarks ✓
2. **7 visual tests** - All green checkmarks ✓
3. **7 canvas renderings** showing:
   - Silence: Dark/black
   - Sine: Colored peak around middle
   - Multi-freq: Two distinct peaks
   - Gain comparison: High gain brighter
   - Labels: Frequency markers visible

## What Gets Tested

| Aspect | How It's Tested |
|--------|----------------|
| Canvas renders | Pixel data exists |
| Silence is dark | Brightness < 15 |
| Signal has color | Brightness > 15, color variance |
| Peak position | Correct pixel region is bright |
| Rainbow colors | Color gradient detected |
| Multiple peaks | Local maxima found |
| Gain effect | Brightness increases |
| Labels | Visual inspection |

## Benefits

1. **Catches rendering bugs** - Would detect if canvas was all black
2. **Verifies colors** - Ensures rainbow gradient works
3. **Confirms positioning** - Peaks appear where expected
4. **Tests visual effects** - Gain slider actually changes brightness
5. **Manual inspection** - Can see what was rendered
6. **Regression prevention** - Visual output preserved for comparison

## Example Test Output

```
=== Visual Rendering Tests ===

✓ Testing visual rendering of silence...
✓ Silence renders correctly (avg brightness: 8.42)

✓ Testing visual rendering of 440 Hz sine wave...
✓ Sine wave renders with color (avg brightness: 64.23)

✓ Testing peak appears at correct visual position...
✓ Peak visual position: x=387, brightness=156.78
✓ Peak region shows expected brightness

✓ Testing rainbow color gradient across spectrum...
✓ Rainbow color gradient detected
Color distribution: {
  uniqueColors: 8,
  redAvg: 87.3,
  greenAvg: 62.1,
  blueAvg: 91.5
}

✓ Testing visual separation of multiple frequencies...
✓ Found 2 visual peaks
✓ Multiple peaks visible in rendering

✓ Testing gain affects visual brightness...
✓ Normal gain brightness: 64.23
✓ High gain brightness: 142.56
✓ Gain correctly increases brightness by 121.9%

✓ Testing frequency labels render at correct positions...
✓ Frequency labels rendered (visual inspection required)
```

## Integration with CI/CD

These tests can run in headless browsers:
- Puppeteer
- Playwright
- Selenium

Can capture screenshots for visual regression testing.

## Future Enhancements

Possible additions:
- [ ] Snapshot testing (compare to reference images)
- [ ] Color accuracy verification (specific HSL values)
- [ ] Animation frame testing
- [ ] Performance benchmarks (render time)
- [ ] Different resolutions side-by-side
- [ ] Stereo pan visualization (when implemented)
