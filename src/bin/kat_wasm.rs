#[cfg(target_arch = "wasm32")]
mod wasm_app {
    use std::sync::Arc;
    use std::time::{Duration};
    use web_time::Instant;
    use parking_lot::{RwLock, Mutex};
    use eframe::wasm_bindgen::prelude::*;
    use eframe::wasm_bindgen::JsCast;
    use eframe::{WebOptions};
    use eframe::egui;
    use katvisualizer::analyzer::{BetterAnalyzer, BetterAnalyzerConfiguration, BetterSpectrogram, AnalysisChainConfig};
    use katvisualizer::common::{RenderSettings, ColorTable, SPECTROGRAM_SLICES, MAX_FREQUENCY_BINS};
    use katvisualizer::ui::{SharedState, draw_ui};
    use std::sync::mpsc::{channel, Receiver, Sender};
    use wasm_bindgen_futures::JsFuture;

    struct AudioState {
        context: Option<web_sys::AudioContext>,
        source: Option<web_sys::AudioBufferSourceNode>,
        processor: Option<web_sys::ScriptProcessorNode>,
        closure: Option<Closure<dyn FnMut(web_sys::AudioProcessingEvent)>>,
    }

    #[wasm_bindgen]
    pub struct WebVisualizer {
        shared_state: SharedState,
        analyzer: Arc<Mutex<BetterAnalyzer>>,
        spectrogram: Arc<RwLock<BetterSpectrogram>>,
        analysis_config: AnalysisChainConfig,
        frequencies: Vec<(f32, f32, f32)>,

        audio_state_container: Option<std::rc::Rc<std::cell::RefCell<AudioState>>>,

        config_tx: Sender<AnalysisChainConfig>,
        config_rx: Receiver<AnalysisChainConfig>,
    }

    impl WebVisualizer {
        pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
             let settings = RenderSettings::default();
             let mut color_table = ColorTable::new();
             color_table.build(
                settings.left_hue,
                settings.right_hue,
                settings.minimum_lightness,
                settings.maximum_lightness,
                settings.maximum_chroma,
            );

            let analysis_config = AnalysisChainConfig::default();

            let analyzer_config = BetterAnalyzerConfiguration {
                 resolution: analysis_config.resolution,
                 start_frequency: analysis_config.start_frequency,
                 end_frequency: analysis_config.end_frequency,
                 erb_frequency_scale: analysis_config.erb_frequency_scale,
                 sample_rate: 44100.0,
                 erb_time_resolution: analysis_config.erb_time_resolution,
                 erb_bandwidth_divisor: analysis_config.erb_bandwidth_divisor,
                 time_resolution_clamp: analysis_config.time_resolution_clamp,
                 q_time_resolution: analysis_config.q_time_resolution,
                 nc_method: analysis_config.nc_method,
                 masking: analysis_config.masking,
            };

            let analyzer = BetterAnalyzer::new(analyzer_config);
            let frequencies = analyzer.frequencies().iter().map(|(a,b,c)| (*a as f32, *b as f32, *c as f32)).collect();

            let shared_state = SharedState {
                settings: RwLock::new(settings),
                last_frame: Mutex::new(Instant::now()),
                color_table: RwLock::new(color_table),
                cached_analysis_settings: Mutex::new(analysis_config.clone()),
                spectrogram_texture: Arc::new(RwLock::new(None)),
            };

            let (config_tx, config_rx) = channel();

            Self {
                shared_state,
                analyzer: Arc::new(Mutex::new(analyzer)),
                spectrogram: Arc::new(RwLock::new(BetterSpectrogram::new(SPECTROGRAM_SLICES, MAX_FREQUENCY_BINS))),
                analysis_config,
                frequencies,
                audio_state_container: None,
                config_tx,
                config_rx,
            }
        }

        fn update_analyzer(&mut self) {
             let analyzer_config = BetterAnalyzerConfiguration {
                 resolution: self.analysis_config.resolution,
                 start_frequency: self.analysis_config.start_frequency,
                 end_frequency: self.analysis_config.end_frequency,
                 erb_frequency_scale: self.analysis_config.erb_frequency_scale,
                 sample_rate: self.analyzer.lock().config().sample_rate,
                 erb_time_resolution: self.analysis_config.erb_time_resolution,
                 erb_bandwidth_divisor: self.analysis_config.erb_bandwidth_divisor,
                 time_resolution_clamp: self.analysis_config.time_resolution_clamp,
                 q_time_resolution: self.analysis_config.q_time_resolution,
                 nc_method: self.analysis_config.nc_method,
                 masking: self.analysis_config.masking,
            };

            let mut analyzer = self.analyzer.lock();
            let old_config = analyzer.config();
             if old_config.resolution != analyzer_config.resolution
                || old_config.start_frequency != analyzer_config.start_frequency
                || old_config.end_frequency != analyzer_config.end_frequency
                || old_config.erb_frequency_scale != analyzer_config.erb_frequency_scale
                || old_config.erb_time_resolution != analyzer_config.erb_time_resolution
                || old_config.time_resolution_clamp != analyzer_config.time_resolution_clamp
                || old_config.erb_bandwidth_divisor != analyzer_config.erb_bandwidth_divisor
                || old_config.q_time_resolution != analyzer_config.q_time_resolution
                || old_config.nc_method != analyzer_config.nc_method
                || old_config.masking != analyzer_config.masking
            {
                 *analyzer = BetterAnalyzer::new(analyzer_config);
                 self.frequencies = analyzer.frequencies().iter().map(|(a,b,c)| (*a as f32, *b as f32, *c as f32)).collect();
            }
        }

        fn load_audio(&mut self, data: Vec<u8>) {
            // Close previous context if exists
            if let Some(container) = &self.audio_state_container {
                let mut state = container.borrow_mut();
                if let Some(ctx) = state.context.take() {
                    let _ = ctx.close();
                }
                if let Some(source) = state.source.take() {
                    let _ = source.stop();
                    let _ = source.disconnect();
                }
                if let Some(processor) = state.processor.take() {
                    let _ = processor.disconnect();
                }
                state.closure = None;
            }

            let context = match web_sys::AudioContext::new() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to create AudioContext: {:?}", e);
                    return;
                }
            };

            let state_container = std::rc::Rc::new(std::cell::RefCell::new(AudioState {
                context: Some(context.clone()),
                source: None,
                processor: None,
                closure: None,
            }));

            self.audio_state_container = Some(state_container.clone());

            let analyzer = self.analyzer.clone();
            let spectrogram = self.spectrogram.clone();
            let config = self.analysis_config.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let array_buffer = js_sys::Uint8Array::from(&data[..]).buffer();
                // context is inside state_container

                let context_handle = {
                    if let Some(ref c) = state_container.borrow().context {
                        c.clone()
                    } else {
                        return;
                    }
                };

                let decoded_res = JsFuture::from(context_handle.decode_audio_data(&array_buffer).unwrap()).await;

                if let Ok(decoded) = decoded_res {
                    let buffer: web_sys::AudioBuffer = decoded.into();
                    start_playback(state_container, buffer, analyzer, spectrogram, config);
                } else {
                    log::error!("Failed to decode audio");
                }
            });
        }
    }

    impl eframe::App for WebVisualizer {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            ctx.request_repaint();

             while let Ok(new_config) = self.config_rx.try_recv() {
                 self.analysis_config = new_config;
                 self.update_analyzer();
             }

             egui::CentralPanel::default().show(ctx, |ui| {
                if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
                     let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
                     for file in dropped_files {
                         if let Some(bytes) = file.bytes {
                             self.load_audio(bytes.to_vec());
                         }
                     }
                }

                if self.audio_state_container.is_none() {
                     ui.centered_and_justified(|ui| {
                        ui.heading("Drag and drop audio file here");
                    });
                }

                let spectrogram = self.spectrogram.read();

                let tx_clone = self.config_tx.clone();
                let tx_clone2 = self.config_tx.clone();

                draw_ui(
                    ui,
                    ctx,
                    &self.shared_state,
                    &spectrogram,
                    Instant::now(),
                    Duration::ZERO,
                    &self.frequencies,
                    Some("WASM Visualizer".to_string()),
                    move |settings| {
                        let _ = tx_clone.send(settings.clone());
                    },
                    move |settings| {
                         let _ = tx_clone2.send(settings.clone());
                    }
                );
             });
        }
    }

    fn start_playback(
        state_container: std::rc::Rc<std::cell::RefCell<AudioState>>,
        buffer: web_sys::AudioBuffer,
        analyzer: Arc<Mutex<BetterAnalyzer>>,
        spectrogram: Arc<RwLock<BetterSpectrogram>>,
        config: AnalysisChainConfig
    ) {
        let mut state = state_container.borrow_mut();

        // If context was closed or removed, abort
        if state.context.is_none() { return; }
        let context = state.context.as_ref().unwrap();

        let source_res = context.create_buffer_source();
        if source_res.is_err() { return; }
        let source = source_res.unwrap();

        source.set_buffer(Some(&buffer));

        let buffer_size = 2048;
        let processor_res = context.create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
             buffer_size, 1, 1);
        if processor_res.is_err() { return; }
        let processor = processor_res.unwrap();

        let sample_rate = context.sample_rate();

        let closure = Closure::wrap(Box::new(move |event: web_sys::AudioProcessingEvent| {
             let input_buffer = event.input_buffer().unwrap();
             let input_data = input_buffer.get_channel_data(0).unwrap();
             let samples: Vec<f64> = input_data.iter().map(|&s| s as f64).collect();

             let mut analyzer = analyzer.lock();
             analyzer.analyze(samples.into_iter(), None);

             let mut spectrogram = spectrogram.write();
             let chunk_duration = Duration::from_secs_f64(buffer_size as f64 / sample_rate as f64);

             spectrogram.update_fn(|analysis| {
                 analysis.update_mono(&analyzer, config.gain, if config.normalize_amplitude { Some(config.listening_volume) } else { None }, chunk_duration);
             });

         }) as Box<dyn FnMut(_)>);

         processor.set_onaudioprocess(Some(closure.as_ref().unchecked_ref()));

         let _ = source.connect_with_audio_node(&processor);
         let _ = processor.connect_with_audio_node(&context.destination());

         let _ = source.start();

         state.source = Some(source);
         state.processor = Some(processor);
         state.closure = Some(closure);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast;
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("the_canvas_id").unwrap();
        let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(wasm_app::WebVisualizer::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
