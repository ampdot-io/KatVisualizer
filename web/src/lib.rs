use std::collections::VecDeque;
use std::f64::consts::PI;
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

// ============ Frequency Scale ============

#[derive(Clone, Copy)]
pub enum FrequencyScale {
    Logarithmic,
    Erb,
}

impl FrequencyScale {
    fn scale(&self, x: f64) -> f64 {
        match self {
            Self::Logarithmic => x.log2(),
            Self::Erb => (1.0 + 0.00437 * x).ln() / std::f64::consts::LN_2,
        }
    }

    fn inv_scale(&self, x: f64) -> f64 {
        match self {
            Self::Logarithmic => 2.0_f64.powf(x),
            Self::Erb => (1.0 / 0.00437) * (2.0_f64.powf(x) - 1.0),
        }
    }
}

fn map_value(x: f64, min: f64, max: f64, target_min: f64, target_max: f64) -> f64 {
    (x - min) / (max - min) * (target_max - target_min) + target_min
}

fn map_value_f32(x: f32, min: f32, max: f32, target_min: f32, target_max: f32) -> f32 {
    (x - min) / (max - min) * (target_max - target_min) + target_min
}

// ============ Frequency Band ============

#[derive(Clone, Copy)]
struct FrequencyBand {
    low: f64,
    center: f64,
    high: f64,
}

fn generate_frequency_bands(
    scale: FrequencyScale,
    n: usize,
    low: f64,
    high: f64,
    erb_bandwidth_divisor: f64,
) -> Vec<FrequencyBand> {
    (0..n)
        .map(|i| {
            let i = i as f64;
            let target_max = (n - 1) as f64;

            let center = scale.inv_scale(map_value(
                i,
                0.0,
                target_max,
                scale.scale(low),
                scale.scale(high),
            ));
            let lower = scale.inv_scale(map_value(
                i - 0.5,
                0.0,
                target_max,
                scale.scale(low),
                scale.scale(high),
            ));
            let higher = scale.inv_scale(map_value(
                i + 0.5,
                0.0,
                target_max,
                scale.scale(low),
                scale.scale(high),
            ));

            let bandwidth = (24.7 * (0.00437 * center + 1.0)) / erb_bandwidth_divisor;

            FrequencyBand {
                low: (center - (bandwidth / 2.0)).min(lower),
                center,
                high: (center + (bandwidth / 2.0)).max(higher),
            }
        })
        .collect()
}

// ============ VQsDFT ============

const HANN_WINDOW: &[f64] = &[1.0, 0.5];

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

#[derive(Clone)]
struct VQsDFTCoeffs {
    period: f64,
    kernel: Vec<VQsDFTCoeff>,
}

struct VQsDFT {
    coeffs: Vec<VQsDFTCoeffs>,
    gains: Vec<f64>,
    buffer: Vec<f64>,
    buffer_index: usize,
    spectrum_data: Vec<f64>,
}

impl VQsDFT {
    fn new(freq_bands: &[FrequencyBand], sample_rate: f64) -> Self {
        let buffer_size = (sample_rate * 1.0).round() as usize; // 1 second max buffer
        let buffer_size_f64 = buffer_size as f64;

        let (min_idx, max_idx) = (0, 2);
        let items_per_band = (max_idx - min_idx) as usize;

        let gains = vec![0.0; 2];
        let k_offset = -0.5;

        VQsDFT {
            spectrum_data: vec![0.0; freq_bands.len()],
            gains,
            coeffs: freq_bands
                .iter()
                .map(|x| {
                    let mut fiddles = Vec::with_capacity(items_per_band);
                    let mut twiddles = Vec::with_capacity(items_per_band);
                    let mut reson_coeffs = Vec::with_capacity(items_per_band);

                    let bandwidth = x.high - x.low;
                    let period = buffer_size_f64.min(sample_rate / (bandwidth + 1.0)).trunc();

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
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.buffer_index = self.buffer.len() - 1;
        self.coeffs.iter_mut().for_each(|coeffs| {
            coeffs.kernel.iter_mut().for_each(|coeff| {
                coeff.coeff1 = (0.0, 0.0);
                coeff.coeff2 = (0.0, 0.0);
                coeff.coeff3 = (0.0, 0.0);
                coeff.coeff4 = (0.0, 0.0);
                coeff.coeff5 = (0.0, 0.0);
            });
        });
    }

    fn analyze(&mut self, samples: &[f64]) {
        self.spectrum_data.fill(0.0);

        let buffer_len = self.buffer.len();
        let buffer_len_int = buffer_len as isize;

        for &sample in samples {
            self.buffer_index = (self.buffer_index + 1) % buffer_len;
            self.buffer[self.buffer_index] = sample;

            for (coeffs, spectrum_data) in
                self.coeffs.iter_mut().zip(self.spectrum_data.iter_mut())
            {
                let oldest_idx = ((self.buffer_index as isize - coeffs.period as isize)
                    % buffer_len_int
                    + buffer_len_int) as usize
                    % buffer_len;
                let oldest = self.buffer[oldest_idx];

                let period = coeffs.period;

                for (coeff, _) in coeffs.kernel.iter_mut().zip(self.gains.iter()) {
                    let comb_x = sample * coeff.fiddle.0 - oldest;
                    let comb_y = sample * coeff.fiddle.1;

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

                    coeff.coeff5.0 = coeff.coeff4.0;
                    coeff.coeff5.1 = coeff.coeff4.1;

                    coeff.coeff4.0 = coeff.coeff3.0;
                    coeff.coeff4.1 = coeff.coeff3.1;
                }

                let first_coeff3 = coeffs.kernel[0].coeff3;
                let second_coeff3 = coeffs.kernel[1].coeff3;

                *spectrum_data = spectrum_data.max(
                    -(first_coeff3.0 / period * second_coeff3.0 / period)
                        - (first_coeff3.1 / period * second_coeff3.1 / period),
                );
            }
        }

        self.spectrum_data.iter_mut().for_each(|x| *x = x.sqrt());
    }
}

// ============ Amplitude Conversion ============

#[inline(always)]
fn amplitude_to_dbfs(amplitude: f64) -> f64 {
    20.0 * amplitude.log10()
}

#[inline(always)]
fn dbfs_to_amplitude(decibels: f64) -> f64 {
    10.0_f64.powf(decibels / 20.0)
}

fn calculate_pan_and_volume(left: f64, right: f64) -> (f64, f64) {
    let ratio = left / right;

    let pan = if ratio == 1.0 {
        0.0
    } else if left == 0.0 && right > 0.0 {
        0.5
    } else if right == 0.0 && left > 0.0 {
        -0.5
    } else if ratio.is_nan() {
        0.0
    } else {
        (f64::atan(
            (-f64::sqrt(2.0) * f64::sqrt(ratio * ratio + 1.0) + ratio + 1.0) / (ratio - 1.0),
        ))
        .to_degrees()
            / 45.0
    };

    (pan, amplitude_to_dbfs(left + right))
}

// ============ Color Table (OkLCH) ============

fn oklch_to_srgb(l: f64, c: f64, h: f64) -> (u8, u8, u8) {
    let h_rad = h * PI / 180.0;
    let a = c * h_rad.cos();
    let b = c * h_rad.sin();

    // OkLab to linear sRGB
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let r = 4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3;
    let g = -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3;
    let b = -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3;

    // Linear to sRGB gamma
    fn linear_to_srgb(x: f64) -> f64 {
        if x <= 0.0031308 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        }
    }

    let r = (linear_to_srgb(r).clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (linear_to_srgb(g).clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (linear_to_srgb(b).clamp(0.0, 1.0) * 255.0).round() as u8;

    (r, g, b)
}

const COLOR_TABLE_CHROMA_SIZE: usize = 256;
const COLOR_TABLE_LIGHTNESS_SIZE: usize = 256;

struct ColorTable {
    table: Vec<(u8, u8, u8)>,
}

impl ColorTable {
    fn new() -> Self {
        Self {
            table: vec![(0, 0, 0); COLOR_TABLE_CHROMA_SIZE * COLOR_TABLE_LIGHTNESS_SIZE],
        }
    }

    fn build(
        &mut self,
        left_hue: f32,
        right_hue: f32,
        min_lightness: f32,
        max_lightness: f32,
        max_chroma: f32,
    ) {
        let max_split = (COLOR_TABLE_CHROMA_SIZE - 1) as f32;
        let max_intensity = (COLOR_TABLE_LIGHTNESS_SIZE - 1) as f32;

        for split_index in 0..COLOR_TABLE_CHROMA_SIZE {
            let split = map_value_f32(split_index as f32, 0.0, max_split, -1.0, 1.0);

            for intensity_index in 0..COLOR_TABLE_LIGHTNESS_SIZE {
                if intensity_index == 0 {
                    self.table[split_index * COLOR_TABLE_LIGHTNESS_SIZE] = (0, 0, 0);
                    continue;
                }

                let intensity =
                    map_value_f32(intensity_index as f32, 0.0, max_intensity, 0.0, 1.0);

                let (hue, chroma) = if split >= 0.0 {
                    (
                        right_hue,
                        map_value_f32(split, 0.0, 1.0, 0.0, max_chroma),
                    )
                } else {
                    (
                        left_hue,
                        map_value_f32(-split, 0.0, 1.0, 0.0, max_chroma),
                    )
                };

                let lightness =
                    map_value_f32(intensity, 0.0, 1.0, min_lightness, max_lightness);

                let color = oklch_to_srgb(lightness as f64, chroma as f64, hue as f64);
                self.table[(split_index * COLOR_TABLE_LIGHTNESS_SIZE) + intensity_index] = color;
            }
        }
    }

    fn lookup(&self, split: f32, intensity: f32) -> (u8, u8, u8) {
        let max_split = (COLOR_TABLE_CHROMA_SIZE - 1) as f32;
        let max_intensity = (COLOR_TABLE_LIGHTNESS_SIZE - 1) as f32;

        let x = map_value_f32(split, -1.0, 1.0, 0.0, max_split)
            .round()
            .clamp(0.0, max_split) as usize;
        let y = map_value_f32(intensity, 0.0, 1.0, 0.0, max_intensity)
            .round()
            .clamp(0.0, max_intensity) as usize;

        self.table[x * COLOR_TABLE_LIGHTNESS_SIZE + y]
    }
}

// ============ Analysis Result ============

#[derive(Clone)]
struct AnalysisSlice {
    data: Vec<(f32, f32)>, // (pan, volume)
    masking: Vec<(f32, f32)>,
    mean: f32,
    max: f32,
    masking_mean: f32,
}

impl AnalysisSlice {
    fn new(capacity: usize) -> Self {
        Self {
            data: vec![(0.0, f32::NEG_INFINITY); capacity],
            masking: vec![(0.0, f32::NEG_INFINITY); capacity],
            mean: f32::NEG_INFINITY,
            max: f32::NEG_INFINITY,
            masking_mean: f32::NEG_INFINITY,
        }
    }
}

// ============ Masking ============

fn bark_scale(f: f64) -> f64 {
    (26.81 * f) / (1960.0 + f) - 0.53
}

struct Masker {
    bark_set: Vec<f64>,
    tonal_thresholds: Vec<f64>,
    masking_coeffs: Vec<f64>,
    ranges: Vec<(usize, usize)>,
}

impl Masker {
    fn new(frequency_bands: &[FrequencyBand]) -> Self {
        let bark_set: Vec<f64> = frequency_bands
            .iter()
            .map(|f| bark_scale(f.center))
            .collect();

        let band_count = frequency_bands.len();

        let ranges: Vec<(usize, usize)> = frequency_bands
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let center_bark = bark_scale(f.center);
                let min_masking_spread = (22.0 + (230.0 / f.center).min(10.0)).min(27.0);
                let bark_spread = 100.0 / min_masking_spread;

                let lower = (0..i.saturating_sub(1))
                    .rev()
                    .find(|&j| bark_scale(frequency_bands[j].high) <= (center_bark - bark_spread))
                    .unwrap_or(0);
                let upper = (i..band_count)
                    .find(|&j| bark_scale(frequency_bands[j].low) >= (center_bark + bark_spread))
                    .unwrap_or(band_count - 1);

                ((lower + 1).min(i), upper.saturating_sub(1))
            })
            .collect();

        let tonal_thresholds: Vec<f64> = bark_set
            .iter()
            .map(|&bark| -6.025 - (0.275 * bark))
            .collect();

        let masking_coeffs: Vec<f64> = frequency_bands
            .iter()
            .map(|f| 22.0 + (230.0 / f.center).min(10.0))
            .collect();

        Self {
            bark_set,
            tonal_thresholds,
            masking_coeffs,
            ranges,
        }
    }

    fn calculate(&self, spectrum: &[f64], masking_out: &mut [f64]) {
        masking_out.fill(0.0);

        for (i, &amplitude) in spectrum.iter().enumerate() {
            if !amplitude.is_normal() || amplitude <= 0.0 {
                continue;
            }

            let amplitude_db = amplitude_to_dbfs(amplitude);
            let coeff_bark = self.bark_set[i];
            let (lower_spread, upper_spread) = (27.0, self.masking_coeffs[i] - 0.2 * amplitude_db);
            let offset = self.tonal_thresholds[i] - 27.0;
            let adjusted_amplitude = dbfs_to_amplitude(offset) * amplitude;

            let (range_start, range_end) = self.ranges[i];

            for j in range_start..i {
                let bark_diff = coeff_bark - self.bark_set[j];
                masking_out[j] += dbfs_to_amplitude(-lower_spread * bark_diff) * adjusted_amplitude;
            }

            for j in i..=range_end {
                let bark_diff = self.bark_set[j] - coeff_bark;
                masking_out[j] += dbfs_to_amplitude(-upper_spread * bark_diff) * adjusted_amplitude;
            }
        }
    }
}

// ============ Main Visualizer ============

#[wasm_bindgen]
pub struct Visualizer {
    left_analyzer: VQsDFT,
    right_analyzer: VQsDFT,
    masker: Masker,
    frequency_bands: Vec<FrequencyBand>,
    color_table: ColorTable,
    spectrogram: VecDeque<AnalysisSlice>,
    spectrogram_max_slices: usize,
    canvas_width: u32,
    canvas_height: u32,
    image_data: Vec<u8>,
    sample_rate: f64,
    resolution: usize,
    left_hue: f32,
    right_hue: f32,
    min_lightness: f32,
    max_lightness: f32,
    max_chroma: f32,
    min_db: f32,
    max_db: f32,
    bargraph_height_ratio: f32,
    masking_buffer: Vec<f64>,
    last_peak_frequency: f32,
    last_peak_amplitude: f32,
    last_pan: f32,
}

#[wasm_bindgen]
impl Visualizer {
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f64, resolution: usize) -> Self {
        let frequency_bands = generate_frequency_bands(
            FrequencyScale::Erb,
            resolution,
            20.0,
            20000.0,
            2.0,
        );

        let left_analyzer = VQsDFT::new(&frequency_bands, sample_rate);
        let right_analyzer = VQsDFT::new(&frequency_bands, sample_rate);
        let masker = Masker::new(&frequency_bands);

        let mut color_table = ColorTable::new();
        color_table.build(195.0, 328.0, 0.13, 0.82, 0.09);

        let spectrogram_max_slices = 512;
        let spectrogram: VecDeque<AnalysisSlice> = (0..spectrogram_max_slices)
            .map(|_| AnalysisSlice::new(resolution))
            .collect();

        Self {
            left_analyzer,
            right_analyzer,
            masker,
            frequency_bands,
            color_table,
            spectrogram,
            spectrogram_max_slices,
            canvas_width: 800,
            canvas_height: 600,
            image_data: vec![0; 800 * 600 * 4],
            sample_rate,
            resolution,
            left_hue: 195.0,
            right_hue: 328.0,
            min_lightness: 0.13,
            max_lightness: 0.82,
            max_chroma: 0.09,
            min_db: -60.0,
            max_db: -10.0,
            bargraph_height_ratio: 0.33,
            masking_buffer: vec![0.0; resolution],
            last_peak_frequency: 0.0,
            last_peak_amplitude: f32::NEG_INFINITY,
            last_pan: 0.0,
        }
    }

    pub fn set_canvas_size(&mut self, width: u32, height: u32) {
        self.canvas_width = width;
        self.canvas_height = height;
        self.image_data = vec![0; (width * height * 4) as usize];
    }

    pub fn set_colors(&mut self, left_hue: f32, right_hue: f32) {
        self.left_hue = left_hue;
        self.right_hue = right_hue;
        self.color_table.build(
            left_hue,
            right_hue,
            self.min_lightness,
            self.max_lightness,
            self.max_chroma,
        );
    }

    pub fn set_amplitude_range(&mut self, min_db: f32, max_db: f32) {
        self.min_db = min_db;
        self.max_db = max_db;
    }

    pub fn set_bargraph_height(&mut self, ratio: f32) {
        self.bargraph_height_ratio = ratio.clamp(0.0, 1.0);
    }

    pub fn process_stereo(&mut self, left: &[f32], right: &[f32]) {
        let left_f64: Vec<f64> = left.iter().map(|&x| x as f64).collect();
        let right_f64: Vec<f64> = right.iter().map(|&x| x as f64).collect();

        self.left_analyzer.analyze(&left_f64);
        self.right_analyzer.analyze(&right_f64);

        // Calculate masking from combined spectrum
        let combined: Vec<f64> = self
            .left_analyzer
            .spectrum_data
            .iter()
            .zip(self.right_analyzer.spectrum_data.iter())
            .map(|(&l, &r)| l + r)
            .collect();
        self.masker.calculate(&combined, &mut self.masking_buffer);

        // Create new analysis slice
        let mut slice = if let Some(s) = self.spectrogram.pop_back() {
            s
        } else {
            AnalysisSlice::new(self.resolution)
        };

        let mut sum = 0.0_f64;
        let mut max_vol = f32::NEG_INFINITY;
        let mut masking_sum = 0.0_f64;
        let mut peak_freq = 0.0_f32;
        let mut peak_amp = f32::NEG_INFINITY;
        let mut peak_pan = 0.0_f32;

        for i in 0..self.resolution {
            let left_amp = self.left_analyzer.spectrum_data[i];
            let right_amp = self.right_analyzer.spectrum_data[i];

            let (pan, volume) = calculate_pan_and_volume(left_amp, right_amp);
            let volume_f32 = volume as f32;
            let pan_f32 = (pan * 2.0) as f32;

            slice.data[i] = (pan_f32, volume_f32);

            let masking_vol = amplitude_to_dbfs(self.masking_buffer[i]) as f32;
            slice.masking[i] = (0.0, masking_vol);

            if volume.is_finite() {
                sum += dbfs_to_amplitude(volume);
            }
            if volume_f32 > max_vol {
                max_vol = volume_f32;
            }
            if masking_vol.is_finite() {
                masking_sum += dbfs_to_amplitude(masking_vol as f64);
            }
            if volume_f32 > peak_amp {
                peak_amp = volume_f32;
                peak_freq = self.frequency_bands[i].center as f32;
                peak_pan = pan_f32;
            }
        }

        slice.mean = amplitude_to_dbfs(sum / self.resolution as f64) as f32;
        slice.max = max_vol;
        slice.masking_mean = amplitude_to_dbfs(masking_sum / self.resolution as f64) as f32;

        self.last_peak_frequency = peak_freq;
        self.last_peak_amplitude = peak_amp;
        self.last_pan = peak_pan;

        self.spectrogram.push_front(slice);

        // Keep spectrogram bounded
        while self.spectrogram.len() > self.spectrogram_max_slices {
            self.spectrogram.pop_back();
        }
    }

    pub fn process_mono(&mut self, samples: &[f32]) {
        let samples_f64: Vec<f64> = samples.iter().map(|&x| x as f64).collect();

        self.left_analyzer.analyze(&samples_f64);

        // Calculate masking
        self.masker
            .calculate(&self.left_analyzer.spectrum_data, &mut self.masking_buffer);

        // Create new analysis slice
        let mut slice = if let Some(s) = self.spectrogram.pop_back() {
            s
        } else {
            AnalysisSlice::new(self.resolution)
        };

        let mut sum = 0.0_f64;
        let mut max_vol = f32::NEG_INFINITY;
        let mut masking_sum = 0.0_f64;
        let mut peak_freq = 0.0_f32;
        let mut peak_amp = f32::NEG_INFINITY;

        for i in 0..self.resolution {
            let amplitude = self.left_analyzer.spectrum_data[i] * 2.0;
            let volume = amplitude_to_dbfs(amplitude) as f32;

            slice.data[i] = (0.0, volume);

            let masking_vol = amplitude_to_dbfs(self.masking_buffer[i] * 2.0) as f32;
            slice.masking[i] = (0.0, masking_vol);

            if volume.is_finite() {
                sum += dbfs_to_amplitude(volume as f64);
            }
            if volume > max_vol {
                max_vol = volume;
            }
            if masking_vol.is_finite() {
                masking_sum += dbfs_to_amplitude(masking_vol as f64);
            }
            if volume > peak_amp {
                peak_amp = volume;
                peak_freq = self.frequency_bands[i].center as f32;
            }
        }

        slice.mean = amplitude_to_dbfs(sum / self.resolution as f64) as f32;
        slice.max = max_vol;
        slice.masking_mean = amplitude_to_dbfs(masking_sum / self.resolution as f64) as f32;

        self.last_peak_frequency = peak_freq;
        self.last_peak_amplitude = peak_amp;
        self.last_pan = 0.0;

        self.spectrogram.push_front(slice);

        while self.spectrogram.len() > self.spectrogram_max_slices {
            self.spectrogram.pop_back();
        }
    }

    pub fn get_peak_frequency(&self) -> f32 {
        self.last_peak_frequency
    }

    pub fn get_peak_amplitude(&self) -> f32 {
        self.last_peak_amplitude
    }

    pub fn get_pan(&self) -> f32 {
        self.last_pan
    }

    pub fn render(&mut self, ctx: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        let width = self.canvas_width as usize;
        let height = self.canvas_height as usize;

        let bargraph_height = (height as f32 * self.bargraph_height_ratio) as usize;
        let spectrogram_height = height - bargraph_height;

        self.image_data.fill(0);

        // Calculate dynamic range based on masking mean
        let (min_db, max_db) = if let Some(front) = self.spectrogram.front() {
            if front.masking_mean.is_finite() {
                let masking = front.masking_mean;
                (masking - 5.0, masking + 40.0)
            } else {
                (self.min_db, self.max_db)
            }
        } else {
            (self.min_db, self.max_db)
        };

        // Draw bargraph
        if bargraph_height > 0 {
            if let Some(front) = self.spectrogram.front() {
                let band_width = width as f32 / self.resolution as f32;

                for (i, &(pan, volume)) in front.data.iter().enumerate() {
                    let intensity =
                        map_value_f32(volume, min_db, max_db, 0.0, 1.0).clamp(0.0, 1.0);
                    let bar_height = (intensity * bargraph_height as f32) as usize;

                    let start_x = (i as f32 * band_width) as usize;
                    let end_x = ((i + 1) as f32 * band_width) as usize;

                    let (r, g, b) = self.color_table.lookup(pan, intensity);

                    for y in (bargraph_height - bar_height)..bargraph_height {
                        for x in start_x..end_x.min(width) {
                            let idx = (y * width + x) * 4;
                            self.image_data[idx] = r;
                            self.image_data[idx + 1] = g;
                            self.image_data[idx + 2] = b;
                            self.image_data[idx + 3] = 255;
                        }
                    }
                }

                // Draw masking threshold
                for (i, &(_, masking_vol)) in front.masking.iter().enumerate() {
                    let intensity =
                        map_value_f32(masking_vol, min_db, max_db, 0.0, 1.0).clamp(0.0, 1.0);
                    let mask_y = bargraph_height - (intensity * bargraph_height as f32) as usize;

                    let start_x = (i as f32 * band_width) as usize;
                    let end_x = ((i + 1) as f32 * band_width) as usize;

                    if mask_y < bargraph_height {
                        for y in mask_y..=(mask_y + 2).min(bargraph_height - 1) {
                            for x in start_x..end_x.min(width) {
                                let idx = (y * width + x) * 4;
                                self.image_data[idx] = 60;
                                self.image_data[idx + 1] = 0;
                                self.image_data[idx + 2] = 10;
                                self.image_data[idx + 3] = 255;
                            }
                        }
                    }
                }
            }
        }

        // Draw spectrogram
        if spectrogram_height > 0 {
            let spectrogram_rows = spectrogram_height.min(self.spectrogram.len());

            for (row, slice) in self.spectrogram.iter().take(spectrogram_rows).enumerate() {
                let y = bargraph_height + row;
                if y >= height {
                    break;
                }

                let band_width = width as f32 / slice.data.len() as f32;

                for (i, &(pan, volume)) in slice.data.iter().enumerate() {
                    let intensity =
                        map_value_f32(volume, min_db, max_db, 0.0, 1.0).clamp(0.0, 1.0);

                    let start_x = (i as f32 * band_width) as usize;
                    let end_x = ((i + 1) as f32 * band_width) as usize;

                    let (r, g, b) = self.color_table.lookup(pan, intensity);

                    for x in start_x..end_x.min(width) {
                        let idx = (y * width + x) * 4;
                        self.image_data[idx] = r;
                        self.image_data[idx + 1] = g;
                        self.image_data[idx + 2] = b;
                        self.image_data[idx + 3] = 255;
                    }
                }
            }
        }

        // Create ImageData and draw to canvas
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&self.image_data),
            self.canvas_width,
            self.canvas_height,
        )?;

        ctx.put_image_data(&image_data, 0.0, 0.0)?;

        Ok(())
    }

    pub fn reset(&mut self) {
        self.left_analyzer.reset();
        self.right_analyzer.reset();
        for slice in &mut self.spectrogram {
            slice.data.fill((0.0, f32::NEG_INFINITY));
            slice.masking.fill((0.0, f32::NEG_INFINITY));
            slice.mean = f32::NEG_INFINITY;
            slice.max = f32::NEG_INFINITY;
            slice.masking_mean = f32::NEG_INFINITY;
        }
    }
}
