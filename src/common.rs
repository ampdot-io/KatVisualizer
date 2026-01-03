use std::time::Duration;
use parking_lot::RwLock;
use color::{ColorSpaceTag, DynamicColor, Flags, Rgba8, Srgb};

#[cfg(target_arch = "wasm32")]
use eframe::egui::Color32;
#[cfg(not(target_arch = "wasm32"))]
use nih_plug_egui::egui::Color32;

use crate::analyzer::{BetterSpectrogram, map_value_f32, AnalysisChainConfig};

pub const MAX_FREQUENCY_BINS: usize = 2048;
pub const SPECTROGRAM_SLICES: usize = 8192;
pub const MAX_OSC_FREQUENCY_BINS: usize = 320;

#[derive(Clone, Copy)]
pub struct RenderSettings {
    pub left_hue: f32,
    pub right_hue: f32,
    pub minimum_lightness: f32,
    pub maximum_lightness: f32,
    pub maximum_chroma: f32,
    pub automatic_gain: bool,
    pub agc_duration: Duration,
    pub agc_above_masking: f32,
    pub agc_below_masking: f32,
    pub agc_minimum: f32,
    pub agc_maximum: f32,
    pub min_db: f32,
    pub max_db: f32,
    pub bargraph_height: f32,
    pub spectrogram_duration: Duration,
    pub bargraph_averaging: Duration,
    pub show_performance: bool,
    pub show_format: bool,
    pub show_hover: bool,
    pub show_masking: bool,
    pub masking_color: Color32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            left_hue: 195.0,
            right_hue: 328.0,
            minimum_lightness: 0.13,
            maximum_lightness: 0.82,
            maximum_chroma: 0.09,
            automatic_gain: true,
            agc_duration: Duration::from_secs_f64(1.0),
            agc_above_masking: 40.0,
            agc_below_masking: 0.0,
            agc_minimum: 3.0 - AnalysisChainConfig::default().listening_volume as f32,
            agc_maximum: 100.0 - AnalysisChainConfig::default().listening_volume as f32,
            min_db: 20.0 - AnalysisChainConfig::default().listening_volume as f32,
            max_db: 80.0 - AnalysisChainConfig::default().listening_volume as f32,
            bargraph_height: 0.33,
            spectrogram_duration: Duration::from_secs_f64(0.67),
            bargraph_averaging: Duration::from_secs_f64(0.004),
            show_performance: true,
            show_format: false,
            show_hover: true,
            show_masking: true,
            masking_color: Color32::from_rgb(33, 0, 4),
        }
    }
}

pub struct ColorTable {
    pub table: Vec<(u8, u8, u8)>,
    pub size: (usize, usize),
    pub max: (f32, f32),
}

const COLOR_TABLE_CHROMA_SIZE: usize = 512;
const COLOR_TABLE_LIGHTNESS_SIZE: usize = 1024;

impl ColorTable {
    pub fn new() -> Self {
        Self {
            table: vec![(0, 0, 0); COLOR_TABLE_CHROMA_SIZE * COLOR_TABLE_LIGHTNESS_SIZE],
            size: (COLOR_TABLE_CHROMA_SIZE, COLOR_TABLE_LIGHTNESS_SIZE),
            max: (
                (COLOR_TABLE_CHROMA_SIZE - 1) as f32,
                (COLOR_TABLE_LIGHTNESS_SIZE - 1) as f32,
            ),
        }
    }
    pub fn build(
        &mut self,
        left_hue: f32,
        right_hue: f32,
        min_lightness: f32,
        max_lightness: f32,
        max_chroma: f32,
    ) {
        let left_color = DynamicColor {
            cs: ColorSpaceTag::Oklch,
            flags: Flags::default(),
            components: [max_lightness, max_chroma, left_hue, 1.0],
        };
        let right_color = DynamicColor {
            cs: ColorSpaceTag::Oklch,
            flags: Flags::default(),
            components: [max_lightness, max_chroma, right_hue, 1.0],
        };

        for split_index in 0..self.size.0 {
            let split = map_value_f32(split_index as f32, 0.0, self.max.0, -1.0, 1.0);
            for intensity_index in 0..self.size.1 {
                if intensity_index == 0 {
                    self.table[split_index * self.size.1] = (0, 0, 0);
                    continue;
                }

                let intensity = map_value_f32(intensity_index as f32, 0.0, self.max.1, 0.0, 1.0);

                let mut color = if split >= 0.0 {
                    let mut color = right_color;
                    color.components[1] = map_value_f32(split, 0.0, 1.0, 0.0, color.components[1]);
                    color
                } else {
                    let mut color = left_color;
                    color.components[1] = map_value_f32(-split, 0.0, 1.0, 0.0, color.components[1]);
                    color
                };

                color.components[0] =
                    map_value_f32(intensity, 0.0, 1.0, min_lightness, color.components[0]);

                let converted: Rgba8 = color.to_alpha_color::<Srgb>().to_rgba8();

                self.table[(split_index * self.size.1) + intensity_index] =
                    (converted.r, converted.g, converted.b);
            }
        }
    }
    pub fn lookup(&self, split: f32, intensity: f32) -> Color32 {
        let location = (
            map_value_f32(split, -1.0, 1.0, 0.0, self.max.0)
                .round()
                .clamp(0.0, self.max.0) as usize,
            map_value_f32(intensity, 0.0, 1.0, 0.0, self.max.1)
                .round()
                .clamp(0.0, self.max.1) as usize,
        );

        let color = unsafe {
            *self
                .table
                .get_unchecked((location.0 * self.size.1) + location.1)
        };

        Color32::from_rgb(color.0, color.1, color.2)
    }
}

pub fn calculate_volume_min_max(
    settings: &RenderSettings,
    spectrogram: &BetterSpectrogram,
) -> (f32, f32) {
    if !settings.automatic_gain || !spectrogram.data[0].masking_mean.is_finite() {
        return (settings.min_db, settings.max_db);
    }

    let mut elapsed = Duration::ZERO;

    let mut masking_sum = 0.0;
    let mut rows: usize = 0;

    for row in &spectrogram.data {
        elapsed += row.duration;
        if elapsed > settings.agc_duration {
            break;
        }

        if row.masking_mean.is_finite() {
            masking_sum += row.masking_mean as f64;
            rows += 1;
        }
    }

    let masking = (masking_sum / rows as f64) as f32;

    (
        (masking - settings.agc_below_masking).clamp(settings.agc_minimum, settings.agc_maximum),
        (masking + settings.agc_above_masking).clamp(settings.agc_minimum, settings.agc_maximum),
    )
}
