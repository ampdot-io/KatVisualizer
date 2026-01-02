use wasm_bindgen::prelude::*;
use std::f64::consts::PI;

#[wasm_bindgen]
pub struct MusicVisualizer {
    analyzer: BetterAnalyzer,
    gain: f64,
    listening_volume: Option<f64>,
}

#[wasm_bindgen]
impl MusicVisualizer {
    #[wasm_bindgen(constructor)]
    pub fn new(resolution: usize, sample_rate: f32) -> Self {
        console_error_panic_hook::set_once();

        let config = BetterAnalyzerConfiguration {
            resolution,
            start_frequency: 20.0,
            end_frequency: 20000.0,
            erb_frequency_scale: true,
            sample_rate,
            q_time_resolution: 17.30993,
            erb_time_resolution: true,
            erb_bandwidth_divisor: 2.0,
            time_resolution_clamp: (0.0, 37.0),
            nc_method: true,
            masking: true,
        };

        Self {
            analyzer: BetterAnalyzer::new(config),
            gain: 0.0,
            listening_volume: Some(86.0),
        }
    }

    pub fn analyze(&mut self, samples: &[f32]) -> Vec<f32> {
        let samples_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();

        self.analyzer.analyze(samples_f64.into_iter(), self.listening_volume);

        let spectrum = self.analyzer.raw_analysis();

        let mut result = Vec::with_capacity(spectrum.len() * 2);
        let gain_linear = dbfs_to_amplitude(self.gain);

        for &amp in spectrum {
            // Convert amplitude to dB with gain applied
            let amp_with_gain = amp * 2.0 * gain_linear;
            let vol_db = if amp_with_gain > 0.0 {
                amplitude_to_dbfs(amp_with_gain)
            } else {
                -100.0
            };

            // Normalize to 0.0-1.0 range
            // Expected range is roughly -80 dB to +20 dB
            let normalized = ((vol_db + 80.0) / 100.0).max(0.0).min(1.0) as f32;

            result.push(0.0);  // pan (mono for now)
            result.push(normalized);
        }

        result
    }

    pub fn get_frequencies(&self) -> Vec<f32> {
        self.analyzer
            .frequencies()
            .iter()
            .map(|(_, center, _)| *center as f32)
            .collect()
    }

    pub fn get_frequency_count(&self) -> usize {
        self.analyzer.frequencies().len()
    }

    pub fn set_gain(&mut self, gain_db: f64) {
        self.gain = gain_db;
    }
}

// Core analyzer implementation
#[derive(Clone)]
struct BetterAnalyzerConfiguration {
    resolution: usize,
    start_frequency: f64,
    end_frequency: f64,
    erb_frequency_scale: bool,
    sample_rate: f32,
    q_time_resolution: f64,
    erb_time_resolution: bool,
    erb_bandwidth_divisor: f64,
    time_resolution_clamp: (f64, f64),
    nc_method: bool,
    masking: bool,
}

#[derive(Clone)]
struct BetterAnalyzer {
    config: BetterAnalyzerConfiguration,
    transform: VQsDFT,
    masker: Masker,
    masking: Vec<f64>,
    frequency_bands: Vec<(f64, f64, f64)>,
}

impl BetterAnalyzer {
    fn new(config: BetterAnalyzerConfiguration) -> Self {
        let frequency_scale = if config.erb_frequency_scale {
            FrequencyScale::Erb
        } else {
            FrequencyScale::Logarithmic
        };

        let frequency_bands = frequency_scale.generate_bands(
            config.resolution,
            config.start_frequency,
            config.end_frequency,
            |center| {
                if config.erb_time_resolution {
                    (24.7 * (0.00437 * center + 1.0)) / config.erb_bandwidth_divisor
                } else {
                    center / config.q_time_resolution
                }
                .min(1.0 / (config.time_resolution_clamp.0 / 1000.0))
                .max(1.0 / (config.time_resolution_clamp.1 / 1000.0))
            },
        );

        let transform = VQsDFT::new(
            &frequency_bands,
            HANN_WINDOW,
            1000.0,
            1.0,
            1000.0,
            config.sample_rate as f64,
            config.nc_method,
        );

        let masker = Masker::new(&frequency_bands);

        let frequency_bands: Vec<_> = frequency_bands
            .iter()
            .map(|band| (band.low, band.center, band.high))
            .collect();

        Self {
            config,
            masker,
            masking: vec![0.0; frequency_bands.len()],
            transform,
            frequency_bands,
        }
    }

    fn frequencies(&self) -> &[(f64, f64, f64)] {
        &self.frequency_bands
    }

    fn analyze(&mut self, samples: impl Iterator<Item = f64>, listening_volume: Option<f64>) {
        self.transform.analyze(samples);

        if self.config.masking {
            self.masker.calculate_masking_threshold(
                self.transform.spectrum_data.iter().copied(),
                listening_volume,
                &mut self.masking,
            );
            self.transform
                .spectrum_data
                .iter_mut()
                .zip(self.masking.iter().copied())
                .for_each(|(amplitude, masking_amplitude)| {
                    *amplitude = amplitude.max(masking_amplitude)
                });
        }
    }

    fn raw_analysis(&self) -> &[f64] {
        &self.transform.spectrum_data
    }
}

// Frequency scaling
enum FrequencyScale {
    Logarithmic,
    Erb,
    Bark,
}

impl FrequencyScale {
    fn scale(&self, x: f64) -> f64 {
        match self {
            Self::Logarithmic => x.log2(),
            Self::Erb => (1.0 + 0.00437 * x).log2(),
            Self::Bark => (26.81 * x) / (1960.0 + x) - 0.53,
        }
    }

    fn inv_scale(&self, x: f64) -> f64 {
        match self {
            Self::Logarithmic => 2.0_f64.powf(x),
            Self::Erb => (1.0 / 0.00437) * ((2.0_f64.powf(x)) - 1.0),
            Self::Bark => 1960.0 / (26.81 / (x + 0.53) - 1.0),
        }
    }

    fn generate_bands<F>(&self, n: usize, low: f64, high: f64, bandwidth: F) -> Vec<FrequencyBand>
    where
        F: Fn(f64) -> f64,
    {
        (0..n)
            .map(|i| {
                let i = i as f64;
                let target_max = (n - 1) as f64;

                let center = self.inv_scale(map_value_f64(
                    i,
                    0.0,
                    target_max,
                    self.scale(low),
                    self.scale(high),
                ));
                let lower = self.inv_scale(map_value_f64(
                    i - 0.5,
                    0.0,
                    target_max,
                    self.scale(low),
                    self.scale(high),
                ));
                let higher = self.inv_scale(map_value_f64(
                    i + 0.5,
                    0.0,
                    target_max,
                    self.scale(low),
                    self.scale(high),
                ));
                let bandwidth = bandwidth(center);

                FrequencyBand {
                    low: (center - (bandwidth / 2.0)).min(lower),
                    center,
                    high: (center + (bandwidth / 2.0)).max(higher),
                }
            })
            .collect()
    }
}

// VQsDFT implementation
const HANN_WINDOW: &[f64] = &[1.0, 0.5];

#[derive(Clone)]
struct VQsDFT {
    coeffs: Vec<VQsDFTCoeffs>,
    #[allow(dead_code)]
    gains: Vec<f64>,  // Used in non-NC method (not implemented in WASM version)
    buffer: Vec<f64>,
    buffer_index: usize,
    spectrum_data: Vec<f64>,
    use_nc: bool,
}

#[derive(Clone)]
struct VQsDFTCoeffs {
    period: f64,
    kernel: Vec<VQsDFTCoeff>,
}

#[derive(Clone, Copy)]
struct VQsDFTCoeff {
    twiddle: (f64, f64),
    fiddle: (f64, f64),
    reson: f64,
    coeff1: (f64, f64),
    coeff2: (f64, f64),
    coeff3: (f64, f64),
    coeff4: (f64, f64),
    coeff5: (f64, f64),
}

#[derive(Clone, Copy)]
struct FrequencyBand {
    low: f64,
    center: f64,
    high: f64,
}

impl VQsDFT {
    fn new(
        freq_bands: &[FrequencyBand],
        window: &[f64],
        time_res: f64,
        bandwidth: f64,
        max_time_res: f64,
        sample_rate: f64,
        use_nc: bool,
    ) -> Self {
        let buffer_size = (sample_rate * max_time_res / 1000.0).round() as usize;
        let buffer_size_f64 = buffer_size as f64;

        let (min_idx, max_idx) = if use_nc {
            (0, 2)
        } else {
            (-(window.len() as isize) + 1, window.len() as isize)
        };
        let items_per_band = (max_idx - min_idx) as usize;

        let gains = if use_nc {
            vec![0.0; 2]
        } else {
            (min_idx..max_idx)
                .map(|i| window[i.unsigned_abs()] * (-((i as f64).abs() % 2.0) * 2.0 + 1.0))
                .collect()
        };

        let k_offset = if use_nc { -0.5 } else { 0.0 };

        VQsDFT {
            spectrum_data: vec![0.0; freq_bands.len()],
            gains,
            coeffs: freq_bands
                .iter()
                .map(|x| {
                    let mut fiddles = Vec::with_capacity(items_per_band);
                    let mut twiddles = Vec::with_capacity(items_per_band);
                    let mut reson_coeffs = Vec::with_capacity(items_per_band);

                    let period = (f64::min(
                        buffer_size_f64,
                        sample_rate
                            / (bandwidth * (x.high - x.low).abs() + 1.0 / (time_res / 1000.0)),
                    ))
                    .trunc();

                    for i in min_idx..max_idx {
                        let i = i as f64;

                        let k = (x.center * period) / sample_rate + i + k_offset;
                        let fid = -2.0 * PI * k;
                        let twid = (2.0 * PI * k) / period;
                        let reson = 2.0 * f64::cos(twid);

                        fiddles.push((f64::cos(fid), f64::sin(fid)));
                        twiddles.push((f64::cos(twid), f64::sin(twid)));
                        reson_coeffs.push(reson);
                    }

                    VQsDFTCoeffs {
                        period,
                        kernel: twiddles
                            .into_iter()
                            .zip(fiddles.into_iter().zip(reson_coeffs))
                            .map(|(twiddle, (fiddle, reson))| VQsDFTCoeff {
                                twiddle,
                                fiddle,
                                reson,
                                coeff1: (0.0, 0.0),
                                coeff2: (0.0, 0.0),
                                coeff3: (0.0, 0.0),
                                coeff4: (0.0, 0.0),
                                coeff5: (0.0, 0.0),
                            })
                            .collect(),
                    }
                })
                .collect(),
            buffer: vec![0.0; buffer_size + 1],
            buffer_index: buffer_size,
            use_nc,
        }
    }

    fn analyze(&mut self, samples: impl Iterator<Item = f64>) -> &[f64] {
        self.spectrum_data.fill(0.0);

        let buffer_len = self.buffer.len();
        let buffer_len_int = buffer_len as isize;

        if self.use_nc {
            for sample in samples {
                self.buffer_index =
                    (((self.buffer_index + 1) % buffer_len) + buffer_len) % buffer_len;
                self.buffer[self.buffer_index] = sample;
                let latest = sample;

                for (coeffs, spectrum_data) in
                    self.coeffs.iter_mut().zip(self.spectrum_data.iter_mut())
                {
                    let oldest_idx = (((self.buffer_index as isize - coeffs.period as isize)
                        % buffer_len_int)
                        + buffer_len_int) as usize
                        % buffer_len;
                    let oldest = self.buffer[oldest_idx];

                    let period = coeffs.period;

                    for coeff in coeffs.kernel.iter_mut() {
                        let comb_x = latest * coeff.fiddle.0 - oldest;
                        let comb_y = latest * coeff.fiddle.1;

                        coeff.coeff1.0 =
                            comb_x * coeff.twiddle.0 - comb_y * coeff.twiddle.1 - coeff.coeff2.0;
                        coeff.coeff1.1 =
                            comb_x * coeff.twiddle.1 + comb_y * coeff.twiddle.0 - coeff.coeff2.1;

                        coeff.coeff2.0 = comb_x;
                        coeff.coeff2.1 = comb_y;

                        coeff.coeff3.0 =
                            coeff.coeff1.0 + coeff.reson * coeff.coeff4.0 - coeff.coeff5.0;
                        coeff.coeff3.1 =
                            coeff.coeff1.1 + coeff.reson * coeff.coeff4.1 - coeff.coeff5.1;

                        coeff.coeff5 = coeff.coeff4;
                        coeff.coeff4 = coeff.coeff3;
                    }

                    let first_coeff3 = coeffs.kernel[0].coeff3;
                    let second_coeff3 = coeffs.kernel[1].coeff3;

                    *spectrum_data = spectrum_data.max(
                        -(first_coeff3.0 / period * second_coeff3.0 / period)
                            - (first_coeff3.1 / period * second_coeff3.1 / period),
                    );
                }
            }
        }

        self.spectrum_data.iter_mut().for_each(|x| *x = x.sqrt());

        &self.spectrum_data
    }
}

// Masking
const MAX_MASKING_DYNAMIC_RANGE: f64 = 100.0;

#[derive(Clone, Copy)]
struct MaskerCoeff {
    bark: f64,
    tonal_masking_threshold: f64,
    masking_coeff_1: f64,
    range: (usize, usize),
}

#[derive(Clone)]
struct Masker {
    coeffs: Vec<MaskerCoeff>,
    bark_set: Vec<f64>,
}

impl Masker {
    fn new(frequency_bands: &[FrequencyBand]) -> Self {
        let frequency_set: Vec<f64> = frequency_bands.iter().map(|f| f.center).collect();

        let bark_set: Vec<f64> = frequency_set
            .iter()
            .copied()
            .map(|f| FrequencyScale::Bark.scale(f))
            .collect();

        let band_count = frequency_bands.len();
        let range_indices = frequency_bands.iter().enumerate().map(|(i, f)| {
            let center_bark = FrequencyScale::Bark.scale(f.center);

            let min_masking_spread = (22.0 + (230.0 / f.center).min(10.0)).min(27.0);
            let bark_spread = MAX_MASKING_DYNAMIC_RANGE / min_masking_spread;

            let lower = (0..i.saturating_sub(1))
                .rev()
                .find(|i| {
                    FrequencyScale::Bark.scale(frequency_bands[*i].high)
                        <= (center_bark - bark_spread)
                })
                .unwrap_or(0);
            let upper = (i..band_count)
                .find(|i| {
                    FrequencyScale::Bark.scale(frequency_bands[*i].low)
                        >= (center_bark + bark_spread)
                })
                .unwrap_or(band_count - 1);

            ((lower + 1).min(i), upper.saturating_sub(1))
        });

        Self {
            coeffs: frequency_set
                .into_iter()
                .zip(bark_set.iter().copied().zip(range_indices))
                .map(|(frequency, (bark, range))| MaskerCoeff {
                    bark,
                    tonal_masking_threshold: -6.025 - (0.275 * bark),
                    masking_coeff_1: 22.0 + (230.0 / frequency).min(10.0),
                    range,
                })
                .collect(),
            bark_set,
        }
    }

    fn calculate_masking_threshold(
        &self,
        spectrum: impl Iterator<Item = f64>,
        listening_volume: Option<f64>,
        masking_threshold: &mut [f64],
    ) {
        masking_threshold.fill(0.0);

        let amplitude_correction_offset = if let Some(listening_volume) = listening_volume {
            listening_volume - 86.0
        } else {
            0.0
        };

        for (i, (component, coeff)) in spectrum.zip(self.coeffs.iter().copied()).enumerate() {
            let amplitude = component;
            let amplitude_db = amplitude_to_dbfs(component);

            if !amplitude.is_normal() {
                continue;
            }

            let (lower_spread, upper_spread, simultaneous) = (
                27.0,
                coeff.masking_coeff_1 - 0.2 * (amplitude_db + amplitude_correction_offset),
                27.0,
            );

            let offset = coeff.tonal_masking_threshold - simultaneous;
            let adjusted_amplitude = dbfs_to_amplitude(offset) * amplitude;

            for (t, b) in masking_threshold[coeff.range.0..i]
                .iter_mut()
                .zip(self.bark_set[coeff.range.0..i].iter().copied())
            {
                *t += dbfs_to_amplitude(-lower_spread * (coeff.bark - b)) * adjusted_amplitude;
            }

            for (t, b) in masking_threshold[i..=coeff.range.1]
                .iter_mut()
                .zip(self.bark_set[i..=coeff.range.1].iter().copied())
            {
                *t += dbfs_to_amplitude(-upper_spread * (b - coeff.bark)) * adjusted_amplitude;
            }
        }
    }
}

// Utility functions
#[inline(always)]
fn amplitude_to_dbfs(amplitude: f64) -> f64 {
    20.0 * f64::log10(amplitude)
}

#[inline(always)]
fn dbfs_to_amplitude(decibels: f64) -> f64 {
    10.0_f64.powf(decibels / 20.0)
}

#[inline(always)]
fn map_value_f64(x: f64, min: f64, max: f64, target_min: f64, target_max: f64) -> f64 {
    (x - min) / (max - min) * (target_max - target_min) + target_min
}
