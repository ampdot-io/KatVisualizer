
use eframe::wasm_bindgen::prelude::*;
use katvisualizer::wasm_app::KatVisualizerApp;
use katvisualizer::wasm_analysis::WasmAnalysisChain;
use katvisualizer::common::AnalysisChainConfig;
use katvisualizer::analyzer::BetterSpectrogram;
use katvisualizer::AnalysisMetrics;
use katvisualizer::common::{SPECTROGRAM_SLICES, MAX_FREQUENCY_BINS};
use parking_lot::{FairMutex, Mutex, RwLock};
use std::sync::Arc;
use web_time::{Duration, Instant};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[wasm_bindgen]
pub async fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Check if console_log is available or just use println/eprintln which wasm-bindgen forwards to console
    // console_log::init_with_level(log::Level::Warn).expect("error initializing logger");
    // eframe initializes its own logger often, but we can stick to panic hook for now.

    // Audio Setup
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("failed to find input device");

    let config = device.default_input_config().unwrap();
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels();

    // Shared State
    let analysis_frequencies = Arc::new(RwLock::new(Vec::with_capacity(MAX_FREQUENCY_BINS)));
    let analysis_config = AnalysisChainConfig::default();

    let analysis_chain = Arc::new(Mutex::new(Some(WasmAnalysisChain::new(
        &analysis_config,
        sample_rate,
        analysis_frequencies.clone(),
    ))));

    let analysis_output = Arc::new(FairMutex::new((
        BetterSpectrogram::new(SPECTROGRAM_SLICES, MAX_FREQUENCY_BINS),
        AnalysisMetrics {
            processing: Duration::ZERO,
            finished: Instant::now(),
        },
    )));

    let chain_clone = analysis_chain.clone();
    let output_clone = analysis_output.clone();

    let err_fn = |err| {
        // Log error to console
        web_sys::console::error_1(&format!("an error occurred on stream: {}", err).into());
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                if let Some(chain) = chain_clone.lock().as_mut() {
                    chain.process(data, &output_clone);
                }
            },
            err_fn,
            None,
        ),
        sample_format => {
            return Err(format!("Unsupported sample format: {:?}", sample_format).into());
        }
    }
    .map_err(|e| format!("Failed to build input stream: {}", e))?;

    stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;
    Box::leak(Box::new(stream));

    let web_options = eframe::WebOptions::default();

    // Move Arcs into closure
    let analysis_chain = analysis_chain.clone();
    let analysis_output = analysis_output.clone();
    let analysis_frequencies = analysis_frequencies.clone();

    eframe::WebRunner::new()
        .start(
            canvas_id,
            web_options,
            Box::new(move |cc| {
                 Box::new(KatVisualizerApp::new(
                    cc,
                    analysis_chain,
                    analysis_output,
                    analysis_frequencies,
                    channels as u32,
                    0,
                    sample_rate,
                 )) as Box<dyn eframe::App>
            }),
        )
        .await
}

fn main() {
}
