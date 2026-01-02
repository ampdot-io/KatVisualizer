
use crate::analyzer::{BetterAnalyzerConfiguration, FrequencyScale};
use std::sync::Arc;
use std::time::Duration;

pub const MAX_FREQUENCY_BINS: usize = 2048;
pub const SPECTROGRAM_SLICES: usize = 8192;
pub const MAX_OSC_FREQUENCY_BINS: usize = 320;

#[derive(Clone)]
pub struct AnalysisChainConfig {
    pub gain: f64,
    pub listening_volume: f64,
    pub normalize_amplitude: bool,
    pub masking: bool,
    pub internal_buffering: bool,
    pub update_rate_hz: f64,
    pub latency_offset: Duration,

    pub output_osc: bool,
    pub osc_socket_address: String,
    pub osc_resource_address_frequencies: String,
    pub osc_resource_address_stats: String,
    pub output_midi: bool,
    pub midi_max_simultaneous_tones: usize,
    pub midi_tone_amplitude_threshold: f32,
    pub midi_pressure_min_amplitude: f32,
    pub midi_pressure_max_amplitude: f32,

    pub resolution: usize,
    pub start_frequency: f64,
    pub end_frequency: f64,
    pub erb_frequency_scale: bool,
    pub erb_time_resolution: bool,
    pub erb_bandwidth_divisor: f64,
    pub time_resolution_clamp: (f64, f64),
    pub q_time_resolution: f64,
    pub nc_method: bool,
}

impl Default for AnalysisChainConfig {
    fn default() -> Self {
        Self {
            gain: 0.0,
            listening_volume: 86.0,
            normalize_amplitude: true,
            masking: true,
            internal_buffering: true,
            update_rate_hz: 2048.0,
            resolution: 512,
            latency_offset: Duration::ZERO,

            output_osc: false,
            osc_socket_address: "127.0.0.1:8000".to_string(),
            osc_resource_address_frequencies: format!(
                "/katvisualizer/v{}/frequencies",
                env!("CARGO_PKG_VERSION")
            ),
            osc_resource_address_stats: format!(
                "/katvisualizer/v{}/stats",
                env!("CARGO_PKG_VERSION")
            ),
            output_midi: false,
            midi_max_simultaneous_tones: 24,
            midi_tone_amplitude_threshold: 30.0 - 86.0,
            midi_pressure_min_amplitude: 30.0 - 86.0,
            midi_pressure_max_amplitude: 70.0 - 86.0,

            start_frequency: BetterAnalyzerConfiguration::default().start_frequency,
            end_frequency: BetterAnalyzerConfiguration::default().end_frequency,
            erb_frequency_scale: BetterAnalyzerConfiguration::default().erb_frequency_scale,
            erb_time_resolution: BetterAnalyzerConfiguration::default().erb_time_resolution,
            erb_bandwidth_divisor: BetterAnalyzerConfiguration::default().erb_bandwidth_divisor,
            time_resolution_clamp: BetterAnalyzerConfiguration::default().time_resolution_clamp,
            q_time_resolution: BetterAnalyzerConfiguration::default().q_time_resolution,
            nc_method: BetterAnalyzerConfiguration::default().nc_method,
        }
    }
}
