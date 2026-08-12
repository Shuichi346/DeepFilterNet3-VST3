#[cfg(all(feature = "model-ll", feature = "model-standard"))]
compile_error!("exactly one DeepFilterNet model feature must be enabled");

#[cfg(not(any(feature = "model-ll", feature = "model-standard")))]
compile_error!("exactly one DeepFilterNet model feature must be enabled");

mod bridge;
mod dsp;
mod editor;
mod model;
mod params;
mod resampler;
mod worker;

use std::sync::Arc;

use bridge::{BridgeConfig, HostBridge};
use nice_plug::prelude::*;
use params::DeepFilterParams;
use resampler::RatePlan;
use worker::WorkerHandle;

enum ProcessingState {
    Active(HostBridge),
    Bypass,
}

struct DeepFilterPlugin {
    params: Arc<DeepFilterParams>,
    editor_state: Arc<nice_plug_egui::EguiState>,
    processing: ProcessingState,
}

impl Default for DeepFilterPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(DeepFilterParams::default()),
            editor_state: editor::default_state(),
            processing: ProcessingState::Bypass,
        }
    }
}

impl Plugin for DeepFilterPlugin {
    const NAME: &'static str = "DeepFilter Noise Reduction";
    const VENDOR: &'static str = "DeepFilterNet";
    const URL: &'static str = "https://github.com/Rikorose/DeepFilterNet";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    const HARD_REALTIME_ONLY: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.editor_state.clone())
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.shutdown_active();
        context.set_latency_samples(0);

        let Some(channels) = selected_channels(audio_io_layout) else {
            return true;
        };
        let Some(sample_rate) = valid_sample_rate(buffer_config.sample_rate) else {
            return true;
        };
        let Ok(rate_plan) = RatePlan::preflight(sample_rate) else {
            return true;
        };
        let Ok(queue_capacity) = HostBridge::queue_capacity(
            buffer_config.max_buffer_size,
            rate_plan.host_quantum,
        ) else {
            return true;
        };
        let Ok(worker) = WorkerHandle::start(
            queue_capacity,
            sample_rate,
            0,
            self.params.atten_lim.value(),
        ) else {
            return true;
        };
        let dsp_info = worker.dsp_info();
        let config = BridgeConfig {
            channels,
            max_buffer_size: buffer_config.max_buffer_size,
            host_quantum: rate_plan.host_quantum,
            reported_latency: dsp_info.latency.total_host,
            process_mode: buffer_config.process_mode,
            queue_capacity,
        };
        let Ok(bridge) = HostBridge::new(worker, config) else {
            return true;
        };

        context.set_latency_samples(bridge.reported_latency());
        self.processing = ProcessingState::Active(bridge);
        true
    }

    fn reset(&mut self) {
        let attenuation = self.params.atten_lim.value();
        self.params.atten_lim.smoothed.reset(attenuation);
        self.params.mix.smoothed.reset(self.params.mix.value());

        if let ProcessingState::Active(bridge) = &mut self.processing {
            bridge.set_attenuation(attenuation);
            bridge.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let attenuation = self.params.atten_lim.smoothed.next();
        let mix = &self.params.mix;
        let channels = buffer.as_slice();

        if let ProcessingState::Active(bridge) = &mut self.processing {
            bridge.set_attenuation(attenuation);

            match channels.len() {
                1 => {
                    if let Some(mono) = channels.first_mut() {
                        let _ = bridge.process_mono(&mut **mono, || mix.smoothed.next());
                    }
                }
                2 => {
                    let (left, right) = channels.split_at_mut(1);
                    if let (Some(left), Some(right)) = (left.first_mut(), right.first_mut()) {
                        let _ = bridge.process_stereo(
                            &mut **left,
                            &mut **right,
                            || mix.smoothed.next(),
                        );
                    }
                }
                _ => {}
            }
        }

        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {
        self.shutdown_active();
    }
}

impl DeepFilterPlugin {
    fn shutdown_active(&mut self) {
        let processing = std::mem::replace(&mut self.processing, ProcessingState::Bypass);
        if let ProcessingState::Active(mut bridge) = processing {
            bridge.shutdown();
        }
    }
}

fn selected_channels(layout: &AudioIOLayout) -> Option<usize> {
    let input = usize::try_from(layout.main_input_channels?.get()).ok()?;
    let output = usize::try_from(layout.main_output_channels?.get()).ok()?;
    if input == output && (input == 1 || input == 2) {
        Some(input)
    } else {
        None
    }
}

fn valid_sample_rate(sample_rate: f32) -> Option<usize> {
    if !sample_rate.is_finite()
        || sample_rate <= 0.0
        || sample_rate.fract() != 0.0
        || sample_rate >= usize::MAX as f32
    {
        return None;
    }

    Some(sample_rate as usize)
}

impl ClapPlugin for DeepFilterPlugin {
    const CLAP_ID: &'static str = "com.deepfilter.noise-reduction";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Noise reduction using DeepFilterNet3");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for DeepFilterPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"DeepFilterNR001\0";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Restoration,
    ];
}

nice_export_clap!(DeepFilterPlugin);
nice_export_vst3!(DeepFilterPlugin);

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    static REAL_MODEL_TEST_ACTIVE: AtomicBool = AtomicBool::new(false);
    const REAL_MODEL_TEST_WAIT: Duration = Duration::from_secs(30);

    pub(crate) struct RealModelTestGuard;

    pub(crate) fn serialize_real_model() -> RealModelTestGuard {
        let deadline = Instant::now() + REAL_MODEL_TEST_WAIT;
        loop {
            if REAL_MODEL_TEST_ACTIVE
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return RealModelTestGuard;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting to serialize real-model tests"
            );
            std::thread::yield_now();
        }
    }

    impl Drop for RealModelTestGuard {
        fn drop(&mut self) {
            REAL_MODEL_TEST_ACTIVE.store(false, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    struct TestInitContext {
        latency: Cell<u32>,
    }

    impl TestInitContext {
        fn new() -> Self {
            Self {
                latency: Cell::new(u32::MAX),
            }
        }
    }

    impl InitContext<DeepFilterPlugin> for TestInitContext {
        fn plugin_api(&self) -> PluginApi {
            PluginApi::Vst3
        }

        fn execute(&self, _task: ()) {}

        fn set_latency_samples(&self, samples: u32) {
            self.latency.set(samples);
        }

        fn set_current_voice_capacity(&self, _capacity: u32) {}
    }

    struct TestProcessContext {
        transport: Transport,
    }

    impl TestProcessContext {
        fn new(sample_rate: f32) -> Self {
            Self {
                transport: Transport::new(sample_rate),
            }
        }
    }

    impl ProcessContext<DeepFilterPlugin> for TestProcessContext {
        fn plugin_api(&self) -> PluginApi {
            PluginApi::Vst3
        }

        fn execute_background(&self, _task: ()) {}

        fn execute_gui(&self, _task: ()) {}

        fn transport(&self) -> &Transport {
            &self.transport
        }

        fn next_event(&mut self) -> Option<PluginNoteEvent<DeepFilterPlugin>> {
            None
        }

        fn send_event(&mut self, _event: PluginNoteEvent<DeepFilterPlugin>) {}

        fn set_latency_samples(&self, _samples: u32) {}

        fn set_current_voice_capacity(&self, _capacity: u32) {}
    }

    fn assert_initialization_falls_back_to_direct_bypass(config: BufferConfig) {
        let mut plugin = DeepFilterPlugin::default();
        let context = TestInitContext::new();
        let mut context = context;
        assert!(plugin.initialize(
            &DeepFilterPlugin::AUDIO_IO_LAYOUTS[0],
            &config,
            &mut context,
        ));
        assert!(matches!(plugin.processing, ProcessingState::Bypass));
        assert_eq!(context.latency.get(), 0);

        let expected = vec![-0.75, -0.25, 0.0, 0.5, 1.0];
        let mut samples = expected.clone();
        let mut buffer = Buffer::default();
        unsafe {
            buffer.set_slices(samples.len(), |slices| {
                slices.clear();
                slices.push(samples.as_mut_slice());
            });
        }
        let mut aux_inputs = [];
        let mut aux_outputs = [];
        let mut auxiliary = AuxiliaryBuffers {
            inputs: &mut aux_inputs,
            outputs: &mut aux_outputs,
        };
        let mut process_context = TestProcessContext::new(config.sample_rate);
        assert_eq!(
            plugin.process(&mut buffer, &mut auxiliary, &mut process_context),
            ProcessStatus::Normal,
        );
        drop(buffer);
        assert_eq!(samples, expected);
    }

    #[test]
    fn invalid_rate_selects_zero_latency_direct_bypass() {
        assert_initialization_falls_back_to_direct_bypass(BufferConfig {
            sample_rate: f32::NAN,
            min_buffer_size: None,
            max_buffer_size: 512,
            process_mode: ProcessMode::Realtime,
        });
    }

    #[test]
    fn oversized_worker_queue_selects_zero_latency_direct_bypass() {
        assert_initialization_falls_back_to_direct_bypass(BufferConfig {
            sample_rate: 48_000.0,
            min_buffer_size: None,
            max_buffer_size: u32::MAX,
            process_mode: ProcessMode::Realtime,
        });
    }

    #[test]
    fn custom_editor_is_fixed_and_compact() {
        let mut plugin = DeepFilterPlugin::default();
        let executor = AsyncExecutor::new(Arc::new(|_| {}), Arc::new(|_| {}));
        let editor = plugin.editor(executor).expect("custom editor should exist");
        let size = editor.size().to_logical::<f32>(1.0);

        assert_eq!(size.width, editor::EDITOR_WIDTH);
        assert_eq!(size.height, editor::EDITOR_HEIGHT);
        assert!(!editor.resize_hint().can_resize);
    }
}
