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
        oscillator: Option<web_sys::OscillatorNode>,
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

        // Debug info
        last_audio_state: String,
        last_sample_rate: f32,
    }

    impl WebVisualizer {
        pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
             cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
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
                last_audio_state: "None".to_string(),
                last_sample_rate: 0.0,
            }
        }

        fn update_analyzer(&mut self) {
             let current_sample_rate = self.analyzer.lock().config().sample_rate;

             let analyzer_config = BetterAnalyzerConfiguration {
                 resolution: self.analysis_config.resolution,
                 start_frequency: self.analysis_config.start_frequency,
                 end_frequency: self.analysis_config.end_frequency,
                 erb_frequency_scale: self.analysis_config.erb_frequency_scale,
                 sample_rate: current_sample_rate,
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
                || old_config.sample_rate != analyzer_config.sample_rate
            {
                 *analyzer = BetterAnalyzer::new(analyzer_config);
                 self.frequencies = analyzer.frequencies().iter().map(|(a,b,c)| (*a as f32, *b as f32, *c as f32)).collect();
            }
        }

        fn stop_audio(&mut self) {
            if let Some(container) = &self.audio_state_container {
                let mut state = container.borrow_mut();
                if let Some(ctx) = state.context.take() {
                    let _ = ctx.close();
                }
                if let Some(source) = state.source.take() {
                    let _ = source.stop();
                    let _ = source.disconnect();
                }
                if let Some(osc) = state.oscillator.take() {
                    let _ = osc.stop();
                    let _ = osc.disconnect();
                }
                if let Some(processor) = state.processor.take() {
                    let _ = processor.disconnect();
                }
                state.closure = None;
            }
            self.audio_state_container = None;
        }

        fn init_audio_context(&mut self) -> Option<web_sys::AudioContext> {
            self.stop_audio();

            let context = match web_sys::AudioContext::new() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to create AudioContext: {:?}", e);
                    return None;
                }
            };

            // Resume context immediately (needed for some browsers policy)
            if context.state() == web_sys::AudioContextState::Suspended {
                let _ = context.resume();
            }

            let state_container = std::rc::Rc::new(std::cell::RefCell::new(AudioState {
                context: Some(context.clone()),
                source: None,
                processor: None,
                closure: None,
                oscillator: None,
            }));

            self.audio_state_container = Some(state_container);

            // Update analyzer sample rate
            {
                let mut analyzer = self.analyzer.lock();
                let mut config = analyzer.config().clone();
                if config.sample_rate != context.sample_rate() {
                    log::info!("Updating analyzer sample rate to {}", context.sample_rate());
                    config.sample_rate = context.sample_rate();
                    *analyzer = BetterAnalyzer::new(config);
                    self.frequencies = analyzer.frequencies().iter().map(|(a,b,c)| (*a as f32, *b as f32, *c as f32)).collect();
                }
            }

            Some(context)
        }

        fn play_test_tone(&mut self) {
            if let Some(context) = self.init_audio_context() {
                let container = self.audio_state_container.as_ref().unwrap().clone();
                let analyzer = self.analyzer.clone();
                let spectrogram = self.spectrogram.clone();
                let config = self.analysis_config.clone();

                let oscillator = context.create_oscillator().unwrap();
                oscillator.set_type(web_sys::OscillatorType::Sine);
                oscillator.frequency().set_value(440.0);

                start_processing(container.clone(), Some(oscillator), None, analyzer, spectrogram, config);
            }
        }

        fn load_audio(&mut self, data: Vec<u8>) {
            if let Some(_context) = self.init_audio_context() {
                let container = self.audio_state_container.as_ref().unwrap().clone();
                let analyzer = self.analyzer.clone();
                let spectrogram = self.spectrogram.clone();
                let config = self.analysis_config.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    let array_buffer = js_sys::Uint8Array::from(&data[..]).buffer();
                    let context_handle = {
                        if let Some(ref c) = container.borrow().context {
                            c.clone()
                        } else {
                            return;
                        }
                    };

                    let decoded_res = JsFuture::from(context_handle.decode_audio_data(&array_buffer).unwrap()).await;

                    if let Ok(decoded) = decoded_res {
                        let buffer: web_sys::AudioBuffer = decoded.into();
                        start_processing(container, None, Some(buffer), analyzer, spectrogram, config);
                    } else {
                        log::error!("Failed to decode audio");
                    }
                });
            }
        }
    }

    impl eframe::App for WebVisualizer {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            ctx.request_repaint();

             while let Ok(new_config) = self.config_rx.try_recv() {
                 self.analysis_config = new_config;
                 self.update_analyzer();
             }

             // Update status
             if let Some(container) = &self.audio_state_container {
                 let state = container.borrow();
                 if let Some(ctx) = &state.context {
                     self.last_audio_state = format!("{:?}", ctx.state());
                     self.last_sample_rate = ctx.sample_rate();
                 }
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
                        ui.vertical_centered(|ui| {
                            ui.heading("Drag and drop audio file here");
                            if ui.button("Play 440Hz Test Tone").clicked() {
                                self.play_test_tone();
                            }
                        });
                    });
                } else {
                    // Debug Overlay
                    ui.scope(|ui| {
                        ui.style_mut().visuals.widgets.noninteractive.bg_fill = egui::Color32::from_black_alpha(128);
                        let frame = egui::Frame::window(ui.style());
                        frame.show(ui, |ui| {
                            ui.label(format!("Audio: {}", self.last_audio_state));
                            ui.label(format!("SR: {:.0}Hz", self.last_sample_rate));
                        });
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

    fn start_processing(
        state_container: std::rc::Rc<std::cell::RefCell<AudioState>>,
        oscillator: Option<web_sys::OscillatorNode>,
        buffer: Option<web_sys::AudioBuffer>,
        analyzer: Arc<Mutex<BetterAnalyzer>>,
        spectrogram: Arc<RwLock<BetterSpectrogram>>,
        config: AnalysisChainConfig
    ) {
        let context = {
            let state = state_container.borrow();
            if state.context.is_none() { return; }
            state.context.as_ref().unwrap().clone()
        };

        let source_node: web_sys::AudioNode;

        if let Some(buf) = buffer {
            let source = context.create_buffer_source().unwrap();
            source.set_buffer(Some(&buf));
            let _ = source.start();
            state_container.borrow_mut().source = Some(source.clone());
            source_node = source.into();
        } else if let Some(osc) = oscillator {
            let _ = osc.start();
            state_container.borrow_mut().oscillator = Some(osc.clone());
            source_node = osc.into();
        } else {
            return;
        }

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

             // Pass through audio
             let output_buffer = event.output_buffer().unwrap();
             let mut output_data = output_buffer.get_channel_data(0).unwrap();
             output_data.copy_from_slice(&input_data); // Copy input to output to hear it

         }) as Box<dyn FnMut(_)>);

         processor.set_onaudioprocess(Some(closure.as_ref().unchecked_ref()));

         let _ = source_node.connect_with_audio_node(&processor);
         let _ = processor.connect_with_audio_node(&context.destination());

         let mut state = state_container.borrow_mut();
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
