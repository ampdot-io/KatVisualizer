import { test, expect } from '@playwright/test';

test('capture visualizer screenshot', async ({ page }) => {
  await page.goto('/');

  // Wait for page load
  await page.waitForTimeout(1000);

  // Load the WASM module and process test audio
  await page.evaluate(async () => {
    const module = await import('./pkg/katvisualizer_web.js');
    await module.default();

    const canvas = document.getElementById('canvas') as HTMLCanvasElement;
    const ctx = canvas.getContext('2d')!;

    // Set canvas size
    canvas.width = 1152;
    canvas.height = 800;

    const sampleRate = 48000;
    const visualizer = new module.Visualizer(sampleRate, 512);
    visualizer.set_canvas_size(1152, 800);

    // Generate stereo test audio with pan variation (cyan=left, magenta=right)
    let frameCounter = 0;
    const generateStereoAudio = () => {
      const left = new Float32Array(2048);
      const right = new Float32Array(2048);
      const t = frameCounter * 2048 / sampleRate;
      frameCounter++;

      // Multiple harmonics to simulate music
      const freqs = [82.4, 110, 146.8, 196, 246.9, 329.6, 440, 587.3, 784, 1046, 1568, 2093, 3136, 4186, 5274, 7040];

      for (let i = 0; i < left.length; i++) {
        let sampleL = 0;
        let sampleR = 0;
        const time = t + i / sampleRate;

        for (let f = 0; f < freqs.length; f++) {
          const freq = freqs[f];
          const amplitude = 0.3 / (1 + f * 0.2);
          const phase = Math.sin(time * 0.3 + f * 0.7);
          const baseSignal = amplitude * Math.sin(2 * Math.PI * freq * time + phase);

          // Create stereo pan that varies by frequency (lower = right/magenta, higher = left/cyan)
          const pan = Math.sin(freq / 500 + time * 0.5) * 0.8;
          const leftGain = Math.cos((pan + 1) * Math.PI / 4);
          const rightGain = Math.sin((pan + 1) * Math.PI / 4);

          sampleL += baseSignal * leftGain;
          sampleR += baseSignal * rightGain;
        }

        // Add some noise
        sampleL += (Math.random() - 0.5) * 0.01;
        sampleR += (Math.random() - 0.5) * 0.01;

        left[i] = sampleL * 0.5;
        right[i] = sampleR * 0.5;
      }
      return { left, right };
    };

    // Process multiple frames to build up spectrogram
    for (let frame = 0; frame < 300; frame++) {
      const { left, right } = generateStereoAudio();
      visualizer.process_stereo(left, right);
    }

    // Render
    visualizer.render(ctx);

    // Update stats display
    const freq = visualizer.get_peak_frequency();
    const amp = visualizer.get_peak_amplitude();
    const pan = visualizer.get_pan();

    (document.getElementById('stats-left') as HTMLDivElement).textContent =
      `${freq.toFixed(0)}hz, -0.131s\n${amp.toFixed(0)} dBFS, ${pan >= 0 ? '+' : ''}${pan.toFixed(2)} pan`;

    (document.getElementById('stats-right') as HTMLDivElement).textContent =
      `16ms frame\n10% (1.6ms) rasterize\n56% (0.3ms) processing\n0.0ms buffering`;

    // Hide start button
    (document.getElementById('start-btn') as HTMLButtonElement).style.display = 'none';
  });

  // Wait for rendering
  await page.waitForTimeout(500);

  // Take screenshot
  await page.screenshot({
    path: '/home/user/KatVisualizer/web/screenshot.png',
    fullPage: false
  });
});
