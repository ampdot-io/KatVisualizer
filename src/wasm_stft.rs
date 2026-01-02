
use std::collections::VecDeque;

#[derive(Clone)]
pub struct StftHelper {
    // Unused in WASM chain, kept for potential future use if we want to mimic nih_plug API closer.
}

// Rewriting StftHelper to be more useful for our specific case:
// We need a buffering struct that accepts a stream of samples and calls a callback when a chunk is ready.

pub struct CircularBuffer {
    buffer: VecDeque<f64>,
    chunk_size: usize,
    hop_size: usize,
}

impl CircularBuffer {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(chunk_size * 2),
            chunk_size,
            hop_size: chunk_size / 2, // 50% overlap
        }
    }

    pub fn set_size(&mut self, chunk_size: usize) {
        self.chunk_size = chunk_size;
        self.hop_size = chunk_size / 2;
    }

    pub fn latency_samples(&self) -> u32 {
        self.chunk_size as u32
    }

    pub fn push_sample(&mut self, sample: f64) -> Option<Vec<f64>> {
        self.buffer.push_back(sample);

        if self.buffer.len() >= self.chunk_size {
            let chunk: Vec<f64> = self.buffer.iter().take(self.chunk_size).copied().collect();
            // Drain hop_size
            self.buffer.drain(0..self.hop_size);
            Some(chunk)
        } else {
            None
        }
    }
}
