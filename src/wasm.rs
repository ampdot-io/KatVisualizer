use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

use crate::analyzer::{BetterAnalyzer, BetterAnalyzerConfiguration, BetterSpectrogram};

const MAX_FREQUENCY_BINS: usize = 2048;
const SPECTROGRAM_SLICES: usize = 8192;

#[derive(Serialize, Deserialize)]
pub struct VisualizerConfig {
    pub resolution: usize,
    pub start_frequency: f64,
    pub end_frequency: f64,
    pub erb_frequency_scale: bool,
    pub sample_rate: f32,
    pub q_time_resolution: f64,
    pub erb_time_resolution: bool,
    pub erb_bandwidth_divisor: f64,
    pub time_resolution_clamp: (f64, f64),
    pub nc_method: bool,
    pub masking: bool,
    pub gain: f64,
    pub listening_volume: f64,
    pub normalize_amplitude: bool,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            resolution: 512,
            start_frequency: 20.0,
            end_frequency: 20000.0,
            erb_frequency_scale: true,
            sample_rate: 48000.0,
            q_time_resolution: 17.30993,
            erb_time_resolution: true,
            erb_bandwidth_divisor: 2.0,
            time_resolution_clamp: (0.0, 37.0),
            nc_method: true,
            masking: true,
            gain: 0.0,
            listening_volume: 86.0,
            normalize_amplitude: true,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FrequencyBin {
    pub lower: f32,
    pub center: f32,
    pub upper: f32,
    pub pan: f32,
    pub volume: f32,
    pub masking: f32,
}

#[derive(Serialize, Deserialize)]
pub struct SpectrogramData {
    pub frequencies: Vec<FrequencyBin>,
    pub mean: f32,
    pub max: f32,
    pub masking_mean: f32,
}

#[wasm_bindgen]
pub struct WasmVisualizer {
    left_analyzer: BetterAnalyzer,
    right_analyzer: Option<BetterAnalyzer>,
    spectrogram: BetterSpectrogram,
    config: VisualizerConfig,
    frequencies: Vec<(f32, f32, f32)>,
}

#[wasm_bindgen]
impl WasmVisualizer {
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f32) -> WasmVisualizer {
        console_error_panic_hook::set_once();

        let config = VisualizerConfig {
            sample_rate,
            ..Default::default()
        };

        let analyzer_config = BetterAnalyzerConfiguration {
            resolution: config.resolution,
            start_frequency: config.start_frequency,
            end_frequency: config.end_frequency,
            erb_frequency_scale: config.erb_frequency_scale,
            sample_rate: config.sample_rate,
            erb_time_resolution: config.erb_time_resolution,
            erb_bandwidth_divisor: config.erb_bandwidth_divisor,
            time_resolution_clamp: config.time_resolution_clamp,
            q_time_resolution: config.q_time_resolution,
            nc_method: config.nc_method,
            masking: config.masking,
        };

        let left_analyzer = BetterAnalyzer::new(analyzer_config.clone());
        let frequencies: Vec<(f32, f32, f32)> = left_analyzer
            .frequencies()
            .iter()
            .map(|(a, b, c)| (*a as f32, *b as f32, *c as f32))
            .collect();

        WasmVisualizer {
            left_analyzer,
            right_analyzer: Some(BetterAnalyzer::new(analyzer_config)),
            spectrogram: BetterSpectrogram::new(SPECTROGRAM_SLICES, MAX_FREQUENCY_BINS),
            config,
            frequencies,
        }
    }

    pub fn process_mono(&mut self, samples: &[f32]) {
        let listening_volume = if self.config.normalize_amplitude {
            Some(self.config.listening_volume)
        } else {
            None
        };

        self.left_analyzer.analyze(
            samples.iter().map(|s| *s as f64),
            listening_volume,
        );

        let chunk_duration = std::time::Duration::from_secs_f64(
            samples.len() as f64 / self.config.sample_rate as f64
        );

        self.spectrogram.update_fn(|analysis_output| {
            analysis_output.update_mono(
                &self.left_analyzer,
                self.config.gain,
                listening_volume,
                chunk_duration,
            );
        });
    }

    pub fn process_stereo(&mut self, left_samples: &[f32], right_samples: &[f32]) {
        let listening_volume = if self.config.normalize_amplitude {
            Some(self.config.listening_volume)
        } else {
            None
        };

        self.left_analyzer.analyze(
            left_samples.iter().map(|s| *s as f64),
            listening_volume,
        );

        if let Some(ref mut right_analyzer) = self.right_analyzer {
            right_analyzer.analyze(
                right_samples.iter().map(|s| *s as f64),
                listening_volume,
            );
        }

        let chunk_duration = std::time::Duration::from_secs_f64(
            left_samples.len() as f64 / self.config.sample_rate as f64
        );

        self.spectrogram.update_fn(|analysis_output| {
            if let Some(ref right_analyzer) = self.right_analyzer {
                analysis_output.update_stereo(
                    &self.left_analyzer,
                    right_analyzer,
                    self.config.gain,
                    listening_volume,
                    chunk_duration,
                );
            }
        });
    }

    pub fn get_spectrogram_data(&self) -> JsValue {
        let data = &self.spectrogram.data[0];

        let frequencies: Vec<FrequencyBin> = self.frequencies
            .iter()
            .zip(data.data.iter())
            .zip(data.masking.iter())
            .map(|(((lower, center, upper), (pan, volume)), (_, mask))| {
                FrequencyBin {
                    lower: *lower,
                    center: *center,
                    upper: *upper,
                    pan: *pan,
                    volume: *volume,
                    masking: *mask,
                }
            })
            .collect();

        let spectrogram_data = SpectrogramData {
            frequencies,
            mean: data.mean,
            max: data.max,
            masking_mean: data.masking_mean,
        };

        serde_wasm_bindgen::to_value(&spectrogram_data).unwrap()
    }

    pub fn get_frequency_count(&self) -> usize {
        self.frequencies.len()
    }

    pub fn update_config(&mut self, config_json: JsValue) {
        if let Ok(new_config) = serde_wasm_bindgen::from_value::<VisualizerConfig>(config_json) {
            let needs_reinit =
                self.config.resolution != new_config.resolution ||
                self.config.start_frequency != new_config.start_frequency ||
                self.config.end_frequency != new_config.end_frequency ||
                self.config.erb_frequency_scale != new_config.erb_frequency_scale ||
                self.config.sample_rate != new_config.sample_rate ||
                self.config.erb_time_resolution != new_config.erb_time_resolution ||
                self.config.erb_bandwidth_divisor != new_config.erb_bandwidth_divisor ||
                self.config.q_time_resolution != new_config.q_time_resolution ||
                self.config.nc_method != new_config.nc_method ||
                self.config.masking != new_config.masking;

            self.config = new_config;

            if needs_reinit {
                let analyzer_config = BetterAnalyzerConfiguration {
                    resolution: self.config.resolution,
                    start_frequency: self.config.start_frequency,
                    end_frequency: self.config.end_frequency,
                    erb_frequency_scale: self.config.erb_frequency_scale,
                    sample_rate: self.config.sample_rate,
                    erb_time_resolution: self.config.erb_time_resolution,
                    erb_bandwidth_divisor: self.config.erb_bandwidth_divisor,
                    time_resolution_clamp: self.config.time_resolution_clamp,
                    q_time_resolution: self.config.q_time_resolution,
                    nc_method: self.config.nc_method,
                    masking: self.config.masking,
                };

                self.left_analyzer = BetterAnalyzer::new(analyzer_config.clone());
                self.right_analyzer = Some(BetterAnalyzer::new(analyzer_config));

                self.frequencies = self.left_analyzer
                    .frequencies()
                    .iter()
                    .map(|(a, b, c)| (*a as f32, *b as f32, *c as f32))
                    .collect();
            }
        }
    }

    pub fn set_gain(&mut self, gain: f64) {
        self.config.gain = gain;
    }

    pub fn set_listening_volume(&mut self, volume: f64) {
        self.config.listening_volume = volume;
    }
}
