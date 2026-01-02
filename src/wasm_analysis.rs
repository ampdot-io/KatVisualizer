
use crate::analyzer::{
    BetterAnalyzer, BetterAnalyzerConfiguration, BetterSpectrogram, BetterAnalysis
};
use crate::AnalysisMetrics;
use crate::common::*;
use crate::wasm_stft::CircularBuffer;
use parking_lot::{FairMutex, Mutex, RwLock};
use std::sync::Arc;
use web_time::{Duration, Instant};

#[allow(clippy::type_complexity)]
pub struct WasmAnalysisChain {
    pub chunker_left: CircularBuffer,
    pub chunker_right: CircularBuffer,
    left_analyzer: Arc<Mutex<(Vec<f64>, BetterAnalyzer)>>,
    right_analyzer: Arc<Mutex<(Vec<f64>, BetterAnalyzer)>>,
    gain: f64,
    internal_buffering: bool,
    update_rate: f64,
    listening_volume: Option<f64>,
    masking: bool,
    pub latency_samples: u32,
    additional_latency: Duration,
    sample_rate: f32,
    chunk_size: usize,
    chunk_duration: Duration,
    single_input: bool,
    pub frequencies: Arc<RwLock<Vec<(f32, f32, f32)>>>,
}

impl WasmAnalysisChain {
    pub fn new(
        config: &AnalysisChainConfig,
        sample_rate: f32,
        frequency_list_container: Arc<RwLock<Vec<(f32, f32, f32)>>>,
    ) -> Self {
        let analyzer = BetterAnalyzer::new(BetterAnalyzerConfiguration {
            resolution: config.resolution,
            start_frequency: config.start_frequency,
            end_frequency: config.end_frequency,
            erb_frequency_scale: config.erb_frequency_scale,
            sample_rate,
            erb_time_resolution: config.erb_time_resolution,
            erb_bandwidth_divisor: config.erb_bandwidth_divisor,
            time_resolution_clamp: config.time_resolution_clamp,
            q_time_resolution: config.q_time_resolution,
            nc_method: config.nc_method,
            masking: config.masking,
        });

        let chunk_size = (sample_rate as f64 / config.update_rate_hz).round() as usize;
        let chunker_left = CircularBuffer::new(chunk_size);
        let chunker_right = CircularBuffer::new(chunk_size);

        {
            let mut frequencies = frequency_list_container.write();
            frequencies.clear();
            frequencies.extend(
                analyzer
                    .frequencies()
                    .iter()
                    .map(|(a, b, c)| (*a as f32, *b as f32, *c as f32)),
            );
        }

        Self {
            sample_rate,
            internal_buffering: config.internal_buffering,
            latency_samples: chunker_left.latency_samples()
            + (config.latency_offset.as_secs_f64() * sample_rate as f64) as u32,
            additional_latency: config.latency_offset,
            chunker_left,
            chunker_right,
            frequencies: frequency_list_container,
            left_analyzer: Arc::new(Mutex::new((vec![0.0; chunk_size], analyzer.clone()))),
            right_analyzer: Arc::new(Mutex::new((vec![0.0; chunk_size], analyzer))),
            gain: config.gain,
            update_rate: config.update_rate_hz,
            listening_volume: if config.normalize_amplitude {
                Some(config.listening_volume)
            } else {
                None
            },
            masking: config.masking,
            chunk_size,
            chunk_duration: Duration::from_secs_f64(chunk_size as f64 / sample_rate as f64),
            single_input: false, // Assume stereo for now, or handle appropriately
        }
    }

    pub fn process(
        &mut self,
        input: &[f32],
        output: &FairMutex<(BetterSpectrogram, AnalysisMetrics)>,
    ) {
        // Input is interleaved stereo [L, R, L, R, ...]

        let mut finished = Instant::now();

        // Separate channels
        // Since CircularBuffer::process takes a slice, we might want to deinterleave first.
        // Or we can just push sample by sample.

        let frames = input.len() / 2;

        // This is a bit inefficient (looping and pushing one by one), but simpler to implement
        for i in 0..frames {
            let l = input[i * 2];
            let r = input[i * 2 + 1];

            // We need to capture the output trigger from the circular buffers.
            // Since we process L and R simultaneously (same rate), they should trigger at the same time.

            let l_chunk = self.chunker_left.push_sample(l as f64);
            let r_chunk = self.chunker_right.push_sample(r as f64);

            if let (Some(l_data), Some(r_data)) = (l_chunk, r_chunk) {
                // Analyze
                let listening_volume = self.listening_volume;
                let left_analyzer_ref = self.left_analyzer.clone();
                let right_analyzer_ref = self.right_analyzer.clone();

                // Run analysis synchronously (no threads on WASM for now)
                {
                     let mut lock = left_analyzer_ref.lock();
                     let (_, ref mut analyzer) = *lock;
                     analyzer.analyze(l_data.iter().copied(), listening_volume);
                }
                {
                     let mut lock = right_analyzer_ref.lock();
                     let (_, ref mut analyzer) = *lock;
                     analyzer.analyze(r_data.iter().copied(), listening_volume);
                }

                // Update spectrogram
                let (ref mut spectrogram, ref mut metrics) = *output.lock();

                let left_lock = left_analyzer_ref.lock();
                let right_lock = right_analyzer_ref.lock();
                let left_analyzer = &left_lock.1;
                let right_analyzer = &right_lock.1;

                spectrogram.update_fn(|analysis_output: &mut BetterAnalysis| {
                    analysis_output.update_stereo(
                        left_analyzer,
                        right_analyzer,
                        self.gain,
                        self.listening_volume,
                        self.chunk_duration,
                    );
                });

                 let now = Instant::now();
                 // Duration calculation might be tricky on WASM if Instant is not monotonic or has low res,
                 // but typically it works fine with performance.now shim.
                 metrics.processing = now.duration_since(finished);
                 metrics.finished = now;
                 finished = now;
            }
        }
    }

    pub fn config(&self) -> AnalysisChainConfig {
        let analyzer = self.left_analyzer.lock();
        let analyzer_config = analyzer.1.config();

        AnalysisChainConfig {
            gain: self.gain,
            listening_volume: self
                .listening_volume
                .unwrap_or(AnalysisChainConfig::default().listening_volume),
            normalize_amplitude: self.listening_volume.is_some(),
            masking: self.masking,
            internal_buffering: self.internal_buffering,
            update_rate_hz: self.update_rate,
            latency_offset: self.additional_latency,
            resolution: analyzer_config.resolution,
            start_frequency: analyzer_config.start_frequency,
            end_frequency: analyzer_config.end_frequency,
            erb_frequency_scale: analyzer_config.erb_frequency_scale,
            erb_time_resolution: analyzer_config.erb_time_resolution,
            erb_bandwidth_divisor: analyzer_config.erb_bandwidth_divisor,
            time_resolution_clamp: analyzer_config.time_resolution_clamp,
            q_time_resolution: analyzer_config.q_time_resolution,
            nc_method: analyzer_config.nc_method,

             // Defaults for unsupported features
            output_osc: false,
            osc_socket_address: "".to_string(),
            osc_resource_address_frequencies: "".to_string(),
            osc_resource_address_stats: "".to_string(),
            output_midi: false,
            midi_max_simultaneous_tones: 24,
            midi_tone_amplitude_threshold: 0.0,
            midi_pressure_min_amplitude: 0.0,
            midi_pressure_max_amplitude: 0.0,
        }
    }

    pub fn update_config(&mut self, config: &AnalysisChainConfig) {
        self.gain = config.gain;
        self.listening_volume = if config.normalize_amplitude {
            Some(config.listening_volume)
        } else {
            None
        };
        self.masking = config.masking;

        let old_left_analyzer = self.left_analyzer.lock();
        let old_analyzer_config = old_left_analyzer.1.config();

        // Skip OSC/MIDI updates as they are not supported

        if self.update_rate != config.update_rate_hz {
            self.chunk_size = (self.sample_rate as f64 / config.update_rate_hz).round() as usize;
            self.chunker_left.set_size(self.chunk_size);
            self.chunker_right.set_size(self.chunk_size);

            self.additional_latency = config.latency_offset;
            self.latency_samples = self.chunker_left.latency_samples() + (self.additional_latency.as_secs_f64()
                * self.sample_rate as f64) as u32;
            self.chunk_duration =
                Duration::from_secs_f64(self.chunk_size as f64 / self.sample_rate as f64);
        }

         if old_analyzer_config.resolution != config.resolution
            || old_analyzer_config.start_frequency != config.start_frequency
            || old_analyzer_config.end_frequency != config.end_frequency
            || old_analyzer_config.erb_frequency_scale != config.erb_frequency_scale
            || old_analyzer_config.erb_time_resolution != config.erb_time_resolution
            || old_analyzer_config.time_resolution_clamp != config.time_resolution_clamp
            || old_analyzer_config.erb_bandwidth_divisor != config.erb_bandwidth_divisor
            || old_analyzer_config.q_time_resolution != config.q_time_resolution
            || old_analyzer_config.nc_method != config.nc_method
            || old_analyzer_config.masking != config.masking
        {
             let analyzer = BetterAnalyzer::new(BetterAnalyzerConfiguration {
                resolution: config.resolution,
                start_frequency: config.start_frequency,
                end_frequency: config.end_frequency,
                erb_frequency_scale: config.erb_frequency_scale,
                sample_rate: self.sample_rate,
                erb_time_resolution: config.erb_time_resolution,
                erb_bandwidth_divisor: config.erb_bandwidth_divisor,
                time_resolution_clamp: config.time_resolution_clamp,
                q_time_resolution: config.q_time_resolution,
                nc_method: config.nc_method,
                masking: config.masking,
            });
            drop(old_left_analyzer); // release lock

            let mut frequencies = self.frequencies.write();
            frequencies.clear();
            frequencies.extend(
                analyzer
                    .frequencies()
                    .iter()
                    .map(|(a, b, c)| (*a as f32, *b as f32, *c as f32)),
            );

            self.left_analyzer =
                Arc::new(Mutex::new((vec![0.0; self.chunk_size], analyzer.clone())));
            self.right_analyzer = Arc::new(Mutex::new((vec![0.0; self.chunk_size], analyzer)));
        }

        self.internal_buffering = config.internal_buffering;
        self.update_rate = config.update_rate_hz;
    }
}
