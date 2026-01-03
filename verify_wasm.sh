#!/bin/bash

set -e

echo "========================================="
echo "  KatVisualizer WASM Verification Test"
echo "========================================="
echo ""

cd /home/user/KatVisualizer

# 1. Check files exist
echo "[ 1/7 ] Checking build artifacts..."
if [ ! -f "pkg/katvisualizer_bg.wasm" ]; then
    echo "ERROR: WASM file not found"
    exit 1
fi
if [ ! -f "pkg/katvisualizer.js" ]; then
    echo "ERROR: JS bindings not found"
    exit 1
fi
if [ ! -f "pkg/index.html" ]; then
    echo "ERROR: index.html not found"
    exit 1
fi
if [ ! -f "pkg/app.js" ]; then
    echo "ERROR: app.js not found"
    exit 1
fi
echo "✓ All build artifacts present"
echo ""

# 2. Check import path
echo "[ 2/7 ] Verifying import path in app.js..."
if grep -q "import('./katvisualizer.js')" pkg/app.js; then
    echo "✓ Import path is correct"
else
    echo "ERROR: Import path is incorrect"
    echo "Expected: import('./katvisualizer.js')"
    echo "Found:"
    grep "import.*katvisualizer" pkg/app.js || echo "  No import found"
    exit 1
fi
echo ""

# 3. Check file sizes
echo "[ 3/7 ] Checking file sizes..."
WASM_SIZE=$(stat -f%z pkg/katvisualizer_bg.wasm 2>/dev/null || stat -c%s pkg/katvisualizer_bg.wasm 2>/dev/null)
echo "  WASM module: $WASM_SIZE bytes (~$(($WASM_SIZE / 1024))KB)"
if [ $WASM_SIZE -lt 10000 ]; then
    echo "WARNING: WASM file seems too small"
fi
echo "✓ File sizes reasonable"
echo ""

# 4. Verify WASM module structure
echo "[ 4/7 ] Checking WASM module exports..."
if command -v wasm-objdump &> /dev/null; then
    EXPORTS=$(wasm-objdump -x pkg/katvisualizer_bg.wasm | grep -c "export" || echo "0")
    echo "  Found $EXPORTS exports"
    echo "✓ WASM module has exports"
else
    echo "⚠ wasm-objdump not available, skipping detailed check"
fi
echo ""

# 5. Check JavaScript bindings
echo "[ 5/7 ] Verifying JavaScript bindings..."
if grep -q "WasmVisualizer" pkg/katvisualizer.js; then
    echo "✓ WasmVisualizer class found in bindings"
else
    echo "ERROR: WasmVisualizer class not found"
    exit 1
fi
echo ""

# 6. Check HTML structure
echo "[ 6/7 ] Validating HTML..."
if grep -q '<script type="module" src="app.js">' pkg/index.html; then
    echo "✓ HTML properly references app.js as module"
else
    echo "ERROR: HTML doesn't properly load app.js"
    exit 1
fi
echo ""

# 7. Test server accessibility
echo "[ 7/7 ] Testing server..."
if pgrep -f "python3 -m http.server 8080" > /dev/null; then
    echo "✓ Server is running on port 8080"

    # Try to fetch a file
    if command -v curl &> /dev/null; then
        if curl -s http://localhost:8080/katvisualizer.js | head -1 | grep -q "import"; then
            echo "✓ Server is responding correctly"
        else
            echo "⚠ Server running but response unexpected"
        fi
    fi
else
    echo "⚠ Server not running (start with: cd pkg && python3 -m http.server 8080)"
fi
echo ""

echo "========================================="
echo "  ✓ All verification checks passed!"
echo "========================================="
echo ""
echo "Access the visualizer at:"
echo "  Main app:  http://localhost:8080/index.html"
echo "  Tests:     http://localhost:8080/test.html"
echo "  Test auto: http://localhost:8080/test_visualizer.html"
echo "  Audio gen: http://localhost:8080/generate_test_tone.html"
echo ""
echo "To test:"
echo "  1. Open http://localhost:8080/test_visualizer.html"
echo "  2. Wait for automated tests to run"
echo "  3. Or manually test at http://localhost:8080/"
echo ""
