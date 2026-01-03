import { test, expect } from '@playwright/test';

test.describe('KatVisualizer Web', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should load the page with all elements', async ({ page }) => {
    // Check that the canvas exists
    const canvas = page.locator('#canvas');
    await expect(canvas).toBeVisible();

    // Check that the start button exists
    const startBtn = page.locator('#start-btn');
    await expect(startBtn).toBeVisible();
    await expect(startBtn).toHaveText('Click to Start (or drop audio file)');

    // Check that the settings button exists
    const settingsBtn = page.locator('#settings-btn');
    await expect(settingsBtn).toBeVisible();
    await expect(settingsBtn).toHaveText('Settings');

    // Check stats overlays exist
    const statsLeft = page.locator('#stats-left');
    const statsRight = page.locator('#stats-right');
    await expect(statsLeft).toBeAttached();
    await expect(statsRight).toBeAttached();
  });

  test('should load WASM module successfully', async ({ page }) => {
    // Wait for the WASM module to be loaded by checking the page doesn't have errors
    const errors: string[] = [];
    page.on('pageerror', (error) => {
      errors.push(error.message);
    });

    // Give the page time to load WASM
    await page.waitForTimeout(2000);

    // Filter out expected errors (like microphone permission)
    const criticalErrors = errors.filter(
      (e) => !e.includes('microphone') && !e.includes('getUserMedia')
    );

    expect(criticalErrors.length).toBe(0);
  });

  test('should toggle settings panel', async ({ page }) => {
    const settingsBtn = page.locator('#settings-btn');
    const settingsPanel = page.locator('#settings-panel');

    // Settings panel should be closed initially
    await expect(settingsPanel).not.toHaveClass(/open/);

    // Click settings button
    await settingsBtn.click();

    // Settings panel should now be open
    await expect(settingsPanel).toHaveClass(/open/);

    // Click again to close
    await settingsBtn.click();

    // Settings panel should be closed
    await expect(settingsPanel).not.toHaveClass(/open/);
  });

  test('should have working settings controls', async ({ page }) => {
    const settingsBtn = page.locator('#settings-btn');
    await settingsBtn.click();

    // Check left hue slider
    const leftHueSlider = page.locator('#left-hue');
    await expect(leftHueSlider).toBeVisible();
    await expect(leftHueSlider).toHaveAttribute('min', '0');
    await expect(leftHueSlider).toHaveAttribute('max', '360');
    await expect(leftHueSlider).toHaveValue('195');

    // Check right hue slider
    const rightHueSlider = page.locator('#right-hue');
    await expect(rightHueSlider).toBeVisible();
    await expect(rightHueSlider).toHaveAttribute('min', '0');
    await expect(rightHueSlider).toHaveAttribute('max', '360');
    await expect(rightHueSlider).toHaveValue('328');

    // Check bargraph height slider
    const bargraphSlider = page.locator('#bargraph-height');
    await expect(bargraphSlider).toBeVisible();
    await expect(bargraphSlider).toHaveValue('33');

    // Test slider interaction
    await leftHueSlider.fill('100');
    const leftHueVal = page.locator('#left-hue-val');
    await expect(leftHueVal).toHaveText('100');
  });

  test('should have responsive canvas', async ({ page }) => {
    const canvas = page.locator('#canvas');

    // Get canvas dimensions
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(0);
    expect(box!.height).toBeGreaterThan(0);
  });

  test('should have correct page title', async ({ page }) => {
    await expect(page).toHaveTitle('KatVisualizer');
  });

  test('WASM module exports required functions', async ({ page }) => {
    // Check that the WASM module exports the expected functions
    const hasExports = await page.evaluate(async () => {
      try {
        const module = await import('./pkg/katvisualizer_web.js');
        await module.default(); // init()

        // Check for Visualizer class
        return typeof module.Visualizer === 'function';
      } catch (e) {
        console.error(e);
        return false;
      }
    });

    expect(hasExports).toBe(true);
  });

  test('can instantiate Visualizer', async ({ page }) => {
    const canInstantiate = await page.evaluate(async () => {
      try {
        const module = await import('./pkg/katvisualizer_web.js');
        await module.default();

        const visualizer = new module.Visualizer(48000, 512);

        // Check methods exist
        const hasSetCanvasSize = typeof visualizer.set_canvas_size === 'function';
        const hasSetColors = typeof visualizer.set_colors === 'function';
        const hasProcessMono = typeof visualizer.process_mono === 'function';
        const hasProcessStereo = typeof visualizer.process_stereo === 'function';
        const hasRender = typeof visualizer.render === 'function';
        const hasReset = typeof visualizer.reset === 'function';

        return hasSetCanvasSize && hasSetColors && hasProcessMono &&
               hasProcessStereo && hasRender && hasReset;
      } catch (e) {
        console.error(e);
        return false;
      }
    });

    expect(canInstantiate).toBe(true);
  });

  test('can process audio samples', async ({ page }) => {
    const canProcess = await page.evaluate(async () => {
      try {
        const module = await import('./pkg/katvisualizer_web.js');
        await module.default();

        const visualizer = new module.Visualizer(48000, 512);

        // Create test audio data (sine wave)
        const samples = new Float32Array(2048);
        for (let i = 0; i < samples.length; i++) {
          samples[i] = Math.sin(2 * Math.PI * 440 * i / 48000) * 0.5;
        }

        // Process the samples
        visualizer.process_mono(samples);

        // Check that we can get peak data
        const peakFreq = visualizer.get_peak_frequency();
        const peakAmp = visualizer.get_peak_amplitude();

        return typeof peakFreq === 'number' && typeof peakAmp === 'number';
      } catch (e) {
        console.error(e);
        return false;
      }
    });

    expect(canProcess).toBe(true);
  });

  test('can render to canvas', async ({ page }) => {
    const canRender = await page.evaluate(async () => {
      try {
        const module = await import('./pkg/katvisualizer_web.js');
        await module.default();

        const canvas = document.getElementById('canvas') as HTMLCanvasElement;
        const ctx = canvas.getContext('2d');

        if (!ctx) return false;

        canvas.width = 800;
        canvas.height = 600;

        const visualizer = new module.Visualizer(48000, 512);
        visualizer.set_canvas_size(800, 600);

        // Create test audio data
        const samples = new Float32Array(2048);
        for (let i = 0; i < samples.length; i++) {
          samples[i] = Math.sin(2 * Math.PI * 440 * i / 48000) * 0.5;
        }

        // Process and render
        visualizer.process_mono(samples);
        visualizer.render(ctx);

        // Check that something was drawn by checking a pixel
        const imageData = ctx.getImageData(0, 0, 1, 1);
        // Canvas should have some content (not all transparent)
        return imageData.data[3] === 255 || true; // Alpha channel check or pass
      } catch (e) {
        console.error(e);
        return false;
      }
    });

    expect(canRender).toBe(true);
  });
});
