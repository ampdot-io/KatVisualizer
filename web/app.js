// Import WASM module
let wasm;
let visualizer = null;
let audioContext = null;
let audioSource = null;
let audioBuffer = null;
let startTime = 0;
let pauseTime = 0;
let isPlaying = false;
let isPaused = false;
let animationId = null;
let scriptProcessor = null;

// Canvas and rendering
const canvas = document.getElementById('visualizer');
const ctx = canvas.getContext('2d');

// Settings
const settings = {
    resolution: 512,
    gain: 0,
    listeningVolume: 86,
    erbBandwidthDiv: 2.0,
    erbFreqScale: true,
    erbTimeRes: true,
    ncMethod: true,
    masking: true,
    normalizeAmplitude: true,
    colorHue: 180,
    brightness: 1.0,
    barSpacing: 2,
    showMasking: false,
};

// FPS tracking
let lastFrameTime = performance.now();
let frameCount = 0;
let fpsUpdateTime = performance.now();

// Initialize
async function init() {
    try {
        const { WasmVisualizer } = await import('./katvisualizer.js');
        wasm = { WasmVisualizer };

        setupCanvas();
        setupEventListeners();
        hideLoading();

        console.log('KatVisualizer WASM module loaded successfully');
    } catch (error) {
        console.error('Failed to load WASM module:', error);
        document.getElementById('loadingOverlay').innerHTML = `
            <p style="color: #ff006e; font-size: 1.2em;">Failed to load WASM module</p>
            <p style="color: #a0a0b0; margin-top: 10px;">Please ensure the WASM files are built and in the pkg/ directory</p>
            <p style="color: #a0a0b0; margin-top: 10px; font-size: 0.9em;">Error: ${error.message}</p>
        `;
    }
}

function hideLoading() {
    document.getElementById('loadingOverlay').classList.add('hidden');
}

function setupCanvas() {
    const updateCanvasSize = () => {
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * window.devicePixelRatio;
        canvas.height = rect.height * window.devicePixelRatio;
        ctx.scale(window.devicePixelRatio, window.devicePixelRatio);
    };

    updateCanvasSize();
    window.addEventListener('resize', updateCanvasSize);
}

function setupEventListeners() {
    // File input
    document.getElementById('audioFile').addEventListener('change', handleFileSelect);

    // Playback controls
    document.getElementById('playButton').addEventListener('click', play);
    document.getElementById('pauseButton').addEventListener('click', pause);
    document.getElementById('stopButton').addEventListener('click', stop);

    // Settings
    document.getElementById('resolution').addEventListener('input', (e) => {
        settings.resolution = parseInt(e.target.value);
        document.getElementById('resolutionValue').textContent = settings.resolution;
        updateVisualizerConfig();
    });

    document.getElementById('gain').addEventListener('input', (e) => {
        settings.gain = parseFloat(e.target.value);
        document.getElementById('gainValue').textContent = settings.gain.toFixed(1);
        if (visualizer) visualizer.set_gain(settings.gain);
    });

    document.getElementById('listeningVolume').addEventListener('input', (e) => {
        settings.listeningVolume = parseFloat(e.target.value);
        document.getElementById('listeningVolumeValue').textContent = settings.listeningVolume.toFixed(1);
        if (visualizer) visualizer.set_listening_volume(settings.listeningVolume);
    });

    document.getElementById('erbBandwidthDiv').addEventListener('input', (e) => {
        settings.erbBandwidthDiv = parseFloat(e.target.value);
        document.getElementById('erbBandwidthDivValue').textContent = settings.erbBandwidthDiv.toFixed(1);
        updateVisualizerConfig();
    });

    document.getElementById('erbFreqScale').addEventListener('change', (e) => {
        settings.erbFreqScale = e.target.checked;
        updateVisualizerConfig();
    });

    document.getElementById('erbTimeRes').addEventListener('change', (e) => {
        settings.erbTimeRes = e.target.checked;
        updateVisualizerConfig();
    });

    document.getElementById('ncMethod').addEventListener('change', (e) => {
        settings.ncMethod = e.target.checked;
        updateVisualizerConfig();
    });

    document.getElementById('masking').addEventListener('change', (e) => {
        settings.masking = e.target.checked;
        updateVisualizerConfig();
    });

    document.getElementById('normalizeAmplitude').addEventListener('change', (e) => {
        settings.normalizeAmplitude = e.target.checked;
        updateVisualizerConfig();
    });

    // Render settings
    document.getElementById('colorHue').addEventListener('input', (e) => {
        settings.colorHue = parseInt(e.target.value);
        document.getElementById('colorHueValue').textContent = settings.colorHue;
    });

    document.getElementById('brightness').addEventListener('input', (e) => {
        settings.brightness = parseFloat(e.target.value);
        document.getElementById('brightnessValue').textContent = settings.brightness.toFixed(1);
    });

    document.getElementById('barSpacing').addEventListener('input', (e) => {
        settings.barSpacing = parseInt(e.target.value);
        document.getElementById('barSpacingValue').textContent = settings.barSpacing;
    });

    document.getElementById('showMasking').addEventListener('change', (e) => {
        settings.showMasking = e.target.checked;
    });
}

async function handleFileSelect(event) {
    const file = event.target.files[0];
    if (!file) return;

    document.getElementById('fileName').textContent = file.name;

    try {
        const arrayBuffer = await file.arrayBuffer();

        if (!audioContext) {
            audioContext = new (window.AudioContext || window.webkitAudioContext)();
        }

        audioBuffer = await audioContext.decodeAudioData(arrayBuffer);

        // Initialize visualizer with correct sample rate
        if (visualizer) {
            visualizer.free();
        }
        visualizer = new wasm.WasmVisualizer(audioBuffer.sampleRate);
        updateVisualizerConfig();

        const binCount = visualizer.get_frequency_count();
        document.getElementById('binCount').textContent = binCount;

        // Enable playback controls
        document.getElementById('playButton').disabled = false;
        document.getElementById('pauseButton').disabled = false;
        document.getElementById('stopButton').disabled = false;

        console.log(`Loaded ${file.name}: ${audioBuffer.duration.toFixed(2)}s, ${audioBuffer.sampleRate}Hz`);
    } catch (error) {
        console.error('Error loading audio file:', error);
        alert('Failed to load audio file. Please try a different file.');
    }
}

function updateVisualizerConfig() {
    if (!visualizer || !audioBuffer) return;

    const config = {
        resolution: settings.resolution,
        start_frequency: 20.0,
        end_frequency: 20000.0,
        erb_frequency_scale: settings.erbFreqScale,
        sample_rate: audioBuffer.sampleRate,
        q_time_resolution: 17.30993,
        erb_time_resolution: settings.erbTimeRes,
        erb_bandwidth_divisor: settings.erbBandwidthDiv,
        time_resolution_clamp: [0.0, 37.0],
        nc_method: settings.ncMethod,
        masking: settings.masking,
        gain: settings.gain,
        listening_volume: settings.listeningVolume,
        normalize_amplitude: settings.normalizeAmplitude,
    };

    visualizer.update_config(config);
    const binCount = visualizer.get_frequency_count();
    document.getElementById('binCount').textContent = binCount;
}

function play() {
    if (!audioBuffer || !visualizer) return;

    if (isPaused) {
        // Resume from pause
        const offset = pauseTime;
        startTime = audioContext.currentTime - offset;
        isPaused = false;
    } else {
        // Start from beginning
        stop();
        startTime = audioContext.currentTime;
    }

    audioSource = audioContext.createBufferSource();
    audioSource.buffer = audioBuffer;

    // Create script processor for analysis
    const bufferSize = 4096;
    scriptProcessor = audioContext.createScriptProcessor(bufferSize, audioBuffer.numberOfChannels, audioBuffer.numberOfChannels);

    scriptProcessor.onaudioprocess = (event) => {
        if (!isPlaying) return;

        const inputBuffer = event.inputBuffer;
        const outputBuffer = event.outputBuffer;

        // Copy input to output (pass-through)
        for (let channel = 0; channel < outputBuffer.numberOfChannels; channel++) {
            const inputData = inputBuffer.getChannelData(channel);
            const outputData = outputBuffer.getChannelData(channel);
            outputData.set(inputData);
        }

        // Process audio for visualization
        if (inputBuffer.numberOfChannels === 1) {
            const samples = inputBuffer.getChannelData(0);
            visualizer.process_mono(samples);
        } else {
            const leftSamples = inputBuffer.getChannelData(0);
            const rightSamples = inputBuffer.getChannelData(1);
            visualizer.process_stereo(leftSamples, rightSamples);
        }
    };

    audioSource.connect(scriptProcessor);
    scriptProcessor.connect(audioContext.destination);
    audioSource.connect(audioContext.destination);

    const offset = isPaused ? pauseTime : 0;
    audioSource.start(0, offset);

    audioSource.onended = () => {
        if (isPlaying) {
            stop();
        }
    };

    isPlaying = true;
    startAnimation();
}

function pause() {
    if (!isPlaying) return;

    pauseTime = audioContext.currentTime - startTime;
    if (audioSource) {
        audioSource.stop();
        audioSource.disconnect();
    }
    if (scriptProcessor) {
        scriptProcessor.disconnect();
    }

    isPlaying = false;
    isPaused = true;
    stopAnimation();
}

function stop() {
    if (audioSource) {
        try {
            audioSource.stop();
            audioSource.disconnect();
        } catch (e) {
            // Already stopped
        }
    }
    if (scriptProcessor) {
        scriptProcessor.disconnect();
        scriptProcessor = null;
    }

    isPlaying = false;
    isPaused = false;
    pauseTime = 0;
    startTime = 0;
    stopAnimation();

    // Reset progress bar
    document.getElementById('progressFill').style.width = '0%';
    document.getElementById('timeDisplay').textContent = '0:00 / ' + formatTime(audioBuffer ? audioBuffer.duration : 0);
}

function startAnimation() {
    if (animationId) return;

    const animate = () => {
        render();
        updateProgress();
        updateFPS();
        animationId = requestAnimationFrame(animate);
    };

    animate();
}

function stopAnimation() {
    if (animationId) {
        cancelAnimationFrame(animationId);
        animationId = null;
    }
}

function render() {
    if (!visualizer) return;

    const rect = canvas.getBoundingClientRect();
    const width = rect.width;
    const height = rect.height;

    // Clear canvas
    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, width, height);

    // Get spectrogram data
    const data = visualizer.get_spectrogram_data();

    if (!data || !data.frequencies || data.frequencies.length === 0) return;

    // Update info display
    document.getElementById('meanLevel').textContent = data.mean.toFixed(1) + ' dB';
    document.getElementById('peakLevel').textContent = data.max.toFixed(1) + ' dB';

    const barCount = data.frequencies.length;
    const barWidth = Math.max(1, (width - (barCount - 1) * settings.barSpacing) / barCount);

    // Draw frequency bars
    data.frequencies.forEach((bin, index) => {
        const x = index * (barWidth + settings.barSpacing);

        // Calculate bar height from volume (dB)
        // Map from typical range of -80dB to 0dB
        const normalizedVolume = Math.max(0, Math.min(1, (bin.volume + 80) / 80));
        const barHeight = normalizedVolume * height * 0.9;
        const y = height - barHeight;

        // Create color based on frequency (hue) and pan (saturation)
        // Use OkLCH-inspired colors
        const frequencyFactor = Math.log(bin.center / 20) / Math.log(20000 / 20);
        const hue = (settings.colorHue + frequencyFactor * 60) % 360;

        // Pan affects color intensity: center = full color, sides = more muted
        const panFactor = 1 - Math.abs(bin.pan) * 0.3;
        const saturation = 70 * panFactor;

        // Brightness based on volume
        const lightness = 30 + normalizedVolume * 40;

        ctx.fillStyle = `hsl(${hue}, ${saturation}%, ${lightness * settings.brightness}%)`;
        ctx.fillRect(x, y, barWidth, barHeight);

        // Draw masking threshold if enabled
        if (settings.showMasking && bin.masking) {
            const maskingNormalized = Math.max(0, Math.min(1, (bin.masking + 80) / 80));
            const maskingHeight = maskingNormalized * height * 0.9;
            const maskingY = height - maskingHeight;

            ctx.strokeStyle = 'rgba(255, 100, 100, 0.5)';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(x, maskingY);
            ctx.lineTo(x + barWidth, maskingY);
            ctx.stroke();
        }
    });

    // Draw frequency scale on bottom
    drawFrequencyScale(data.frequencies, width, height);
}

function drawFrequencyScale(frequencies, width, height) {
    if (!frequencies || frequencies.length === 0) return;

    ctx.fillStyle = 'rgba(160, 160, 176, 0.8)';
    ctx.font = '10px monospace';
    ctx.textAlign = 'center';

    // Draw labels for key frequencies
    const keyFreqs = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
    const barWidth = width / frequencies.length;

    keyFreqs.forEach(freq => {
        // Find closest bin
        let closestIdx = 0;
        let closestDiff = Math.abs(frequencies[0].center - freq);

        for (let i = 1; i < frequencies.length; i++) {
            const diff = Math.abs(frequencies[i].center - freq);
            if (diff < closestDiff) {
                closestDiff = diff;
                closestIdx = i;
            }
        }

        const x = closestIdx * (barWidth + settings.barSpacing);
        const label = freq >= 1000 ? (freq / 1000).toFixed(0) + 'k' : freq.toString();

        ctx.fillText(label, x + barWidth / 2, height - 5);
    });
}

function updateProgress() {
    if (!audioBuffer || !isPlaying) return;

    const currentTime = audioContext.currentTime - startTime;
    const duration = audioBuffer.duration;
    const progress = Math.min(100, (currentTime / duration) * 100);

    document.getElementById('progressFill').style.width = progress + '%';
    document.getElementById('timeDisplay').textContent =
        formatTime(currentTime) + ' / ' + formatTime(duration);
}

function updateFPS() {
    frameCount++;
    const now = performance.now();

    if (now - fpsUpdateTime >= 1000) {
        const fps = Math.round(frameCount * 1000 / (now - fpsUpdateTime));
        document.getElementById('fps').textContent = fps;
        frameCount = 0;
        fpsUpdateTime = now;
    }
}

function formatTime(seconds) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
}

// Initialize on load
init();
