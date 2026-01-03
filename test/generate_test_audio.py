#!/usr/bin/env python3
"""
Generate test audio files for the visualizer
"""

import numpy as np
import wave
import struct
import os

def generate_sine_wave(filename, frequency=440, duration=5, sample_rate=48000):
    """Generate a pure sine wave"""
    t = np.linspace(0, duration, int(sample_rate * duration))
    samples = np.sin(2 * np.pi * frequency * t)
    samples = (samples * 32767).astype(np.int16)

    with wave.open(filename, 'w') as wav:
        wav.setnchannels(1)  # mono
        wav.setsampwidth(2)  # 16-bit
        wav.setframerate(sample_rate)
        wav.writeframes(samples.tobytes())
    print(f"Generated {filename}: {frequency}Hz sine wave, {duration}s")

def generate_chord(filename, frequencies=[261.63, 329.63, 392.00], duration=5, sample_rate=48000):
    """Generate a chord (multiple frequencies)"""
    t = np.linspace(0, duration, int(sample_rate * duration))
    samples = np.zeros_like(t)

    for freq in frequencies:
        samples += np.sin(2 * np.pi * freq * t)

    samples = samples / len(frequencies)  # normalize
    samples = (samples * 32767).astype(np.int16)

    with wave.open(filename, 'w') as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(sample_rate)
        wav.writeframes(samples.tobytes())
    print(f"Generated {filename}: Chord {frequencies}, {duration}s")

def generate_sweep(filename, start_freq=20, end_freq=20000, duration=10, sample_rate=48000):
    """Generate a frequency sweep"""
    t = np.linspace(0, duration, int(sample_rate * duration))
    # Exponential sweep for better coverage
    freq = start_freq * (end_freq / start_freq) ** (t / duration)
    phase = 2 * np.pi * np.cumsum(freq) / sample_rate
    samples = np.sin(phase)
    samples = (samples * 32767).astype(np.int16)

    with wave.open(filename, 'w') as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(sample_rate)
        wav.writeframes(samples.tobytes())
    print(f"Generated {filename}: Sweep {start_freq}Hz-{end_freq}Hz, {duration}s")

def generate_stereo_test(filename, left_freq=440, right_freq=880, duration=5, sample_rate=48000):
    """Generate stereo test with different frequencies in each channel"""
    t = np.linspace(0, duration, int(sample_rate * duration))
    left = np.sin(2 * np.pi * left_freq * t)
    right = np.sin(2 * np.pi * right_freq * t)

    left = (left * 32767).astype(np.int16)
    right = (right * 32767).astype(np.int16)

    # Interleave left and right channels
    stereo = np.empty((left.size + right.size,), dtype=np.int16)
    stereo[0::2] = left
    stereo[1::2] = right

    with wave.open(filename, 'w') as wav:
        wav.setnchannels(2)  # stereo
        wav.setsampwidth(2)
        wav.setframerate(sample_rate)
        wav.writeframes(stereo.tobytes())
    print(f"Generated {filename}: Stereo L={left_freq}Hz R={right_freq}Hz, {duration}s")

if __name__ == '__main__':
    os.makedirs('test_audio', exist_ok=True)

    # Generate various test files
    generate_sine_wave('test_audio/440hz_sine.wav', 440, 5)
    generate_sine_wave('test_audio/1000hz_sine.wav', 1000, 5)
    generate_chord('test_audio/c_major_chord.wav', [261.63, 329.63, 392.00], 5)  # C-E-G
    generate_sweep('test_audio/frequency_sweep.wav', 20, 20000, 10)
    generate_stereo_test('test_audio/stereo_test.wav', 440, 880, 5)

    print("\nAll test audio files generated in test_audio/")
