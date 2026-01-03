# How the Visual Tests Now Detect Rendering Failures

## The Problem

The original visual tests were **too lenient** and would pass even when the visualizer rendered nothing. A black screen with no bars would still pass the tests.

## Root Cause

Old test logic:
```javascript
// OLD - Too lenient
const hasColor = avgBrightness > 15 && colorPixels > (totalPixels * 0.01);
```

Problem: If the canvas background is `#0a0a0a` (brightness ~10), and nothing renders, avgBrightness would be ~10-12. But with just the base lightness of 30% in the color formula, even bars with value=0 would create pixels slightly brighter than background, barely pushing avgBrightness above 15.

**The test would pass even with no actual bars drawn.**

## The Fix

### 1. Canvas Comparison Test (Test 11) - THE CRITICAL ONE

```javascript
const blankCanvas = /* background only */;
const testCanvas = /* after rendering */;

if (blankCanvas.toDataURL() === testCanvas.toDataURL()) {
    FAIL: "Canvas unchanged after rendering!"
}
```

**This catches the bug immediately.** If nothing renders, the canvases are identical.

### 2. Stricter Pixel Analysis

```javascript
// NEW - Much stricter
const BACKGROUND_BRIGHTNESS = 10;
const THRESHOLD_ABOVE_BG = 30;  // Must be 30+ brighter than background

// Count pixels significantly above background
if (brightness > BACKGROUND_BRIGHTNESS + THRESHOLD_ABOVE_BG) {
    nonBackgroundPixels++;
}

const hasColor = nonBackgroundPixels > (totalPixels * 0.005); // 0.5% minimum
const hasSignificantContent = maxBrightness > 80; // Must have bright peaks
```

Now requires:
- Pixels must be at least **40** brightness (not just 15)
- At least **0.5%** of pixels must be visible bars
- Maximum brightness must exceed **80** (has peaks)

### 3. Multi-Criteria Validation (Test 13)

```javascript
if (!sinePixels.hasColor) {
    ERROR: "NO visible bars above background!"
}
if (!sinePixels.hasSignificantContent) {
    ERROR: "No bright pixels detected (max brightness < 80)!"
}
if (sinePixels.brightPixelCount < 100) {
    ERROR: "Only N bright pixels - too few for visible signal!"
}
```

**THREE checks, all must pass:**
1. Visible bars exist (above background)
2. Bright pixels exist (peaks)
3. Enough bright pixels (not just a few)

### 4. Detailed Logging

```javascript
logData('Sine wave pixel analysis', {
    avgBrightness: 12.34,
    maxBrightness: 8.56,        // ← Shows it's too dim!
    nonBackgroundRatio: '0.02%', // ← Shows almost nothing rendered!
    brightPixelCount: 5,         // ← Shows very few pixels!
    hasSignificantContent: false // ← FAIL flag
});
```

When rendering fails, you'll see:
- maxBrightness: Very low (< 80)
- nonBackgroundRatio: Near 0%
- brightPixelCount: Near 0
- hasSignificantContent: false

## What Gets Caught Now

### Scenario 1: Nothing renders (all zeros)
```
✗ FAIL: Canvas unchanged after rendering! [Test 11]
✗ FAIL: Sine wave shows NO visible bars above background! [Test 13]
✗ FAIL: No bright pixels detected (max brightness < 80)! [Test 13]
✗ FAIL: Only 0 bright pixels - too few! [Test 13]
```

### Scenario 2: Very weak rendering (values too low)
```
✓ PASS: Canvas modified [Test 11]
✗ FAIL: Only 23 bright pixels - too few for visible signal! [Test 13]
✗ FAIL: No bright pixels detected (max brightness < 80)! [Test 13]
```

### Scenario 3: Bars have height = 0
```
✗ FAIL: Canvas unchanged after rendering! [Test 11]
✗ FAIL: NO visible bars above background! [Test 13]
```

## Rendering Check Page

New file: `wasm/render-check.html`

Standalone page with 5 comprehensive tests:
1. **Canvas Comparison** - Are canvases identical?
2. **Pixel Analysis** - How many pixels changed?
3. **Bar Heights** - Do bars have visible height?
4. **Spectrum Values** - Is data non-zero?
5. **Final Verdict** - Overall pass/fail

Example output when broken:
```
✗ FAIL: Canvas changes after rendering
✗ FAIL: Pixels are modified (0 changed)
✗ FAIL: Bright pixels exist (max RGB = 10,10,10)
✗ FAIL: Bars have height (0 bars > 5px)
✗ FAIL: Spectrum has signal (max value = 0.0001)

CRITICAL: Only 0/5 tests passed - Visualizer is BROKEN
```

## Thresholds

| Metric | Old | New | Reason |
|--------|-----|-----|--------|
| avgBrightness | >15 | - | Too lenient, removed |
| colorPixels | >1% | - | Too lenient, removed |
| nonBackgroundPixels | - | >0.5% | Must be significantly above BG |
| maxBrightness | - | >80 | Must have bright peaks |
| brightPixelCount | - | >100 | Minimum visible pixels |
| canvasModified | - | TRUE | Must change from blank |

## How to Use

### Quick Check
```bash
cd wasm
python3 -m http.server 8080
# Open http://localhost:8080/render-check.html
```

If rendering works: "ALL TESTS PASSED ✓✓✓"
If broken: "CRITICAL: Visualizer is BROKEN"

### Full Test Suite
```bash
# Open http://localhost:8080/test.html
```

Look for Test 11 first:
- ✓ "Canvas modified by rendering" → Baseline OK
- ✗ "Canvas unchanged after rendering!" → **BROKEN**

## Summary

The tests now detect rendering failures through:

1. **Direct comparison** - Canvas must change
2. **Strict thresholds** - Much higher brightness requirements
3. **Multiple criteria** - All must pass, not just one
4. **Clear messages** - Explains exactly what failed
5. **Detailed metrics** - Shows why it failed

**Result: The "everything is black" bug can no longer pass the tests.**
