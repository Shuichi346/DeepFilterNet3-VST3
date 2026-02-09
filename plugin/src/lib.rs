use nih_plug::prelude::*;
use std::sync::{Arc, Mutex};

use df::tract::{DfParams, DfTract, RuntimeParams};
use ndarray::Array2;

// DfTract を Mutex でラップしてスレッドセーフに
struct DfWrapper(Mutex<Option<DfTract>>);

// Send と Sync を手動で実装（Mutex で保護されているため安全）
unsafe impl Send for DfWrapper {}
unsafe impl Sync for DfWrapper {}

struct DeepFilterPlugin {
    params: Arc<DeepFilterParams>,
    df_model: DfWrapper,
    input_buffer: Mutex<Vec<f32>>,
    output_buffer: Mutex<Vec<f32>>,
    hop_size: usize,
    is_initialized: bool,
}

#[derive(Params)]
struct DeepFilterParams {
    #[id = "atten_lim"]
    pub atten_lim: FloatParam,

    #[id = "mix"]
    pub mix: FloatParam,
}

impl Default for DeepFilterParams {
    fn default() -> Self {
        Self {
            atten_lim: FloatParam::new(
                "Attenuation Limit",
                100.0,
                FloatRange::Linear { min: 0.0, max: 100.0 },
            )
            .with_unit(" dB")
            .with_smoother(SmoothingStyle::Linear(50.0)),

            mix: FloatParam::new(
                "Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
        }
    }
}

impl Default for DeepFilterPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(DeepFilterParams::default()),
            df_model: DfWrapper(Mutex::new(None)),
            input_buffer: Mutex::new(Vec::new()),
            output_buffer: Mutex::new(Vec::new()),
            hop_size: 480,
            is_initialized: false,
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

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // DeepFilterNet は 48kHz のみサポート
        if (buffer_config.sample_rate - 48000.0).abs() > 1.0 {
            nih_log!("DeepFilterNet requires 48kHz. Current: {}Hz", buffer_config.sample_rate);
            return false;
        }

        let num_channels = audio_io_layout
            .main_input_channels
            .map(|c| c.get() as usize)
            .unwrap_or(1);

        match self.init_model(num_channels) {
            Ok(hop) => {
                self.hop_size = hop;
                self.is_initialized = true;
                nih_log!("DeepFilterNet initialized. hop_size={}", hop);
                true
            }
            Err(e) => {
                nih_log!("Failed to init DeepFilterNet: {:?}", e);
                false
            }
        }
    }

    fn reset(&mut self) {
        if let Ok(mut buf) = self.input_buffer.lock() {
            buf.clear();
        }
        if let Ok(mut buf) = self.output_buffer.lock() {
            buf.clear();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if !self.is_initialized {
            return ProcessStatus::Normal;
        }

        let mix = self.params.mix.smoothed.next();
        let atten = self.params.atten_lim.smoothed.next();
        let num_samples = buffer.samples();
        let num_channels = buffer.channels();
        let hop = self.hop_size;

        // 入力収集（モノラルにミックスダウン）
        {
            let mut input_buf = self.input_buffer.lock().unwrap();
            for i in 0..num_samples {
                let mut sum = 0.0f32;
                for channel in buffer.iter_samples().nth(i).unwrap() {
                    sum += *channel;
                }
                input_buf.push(sum / num_channels as f32);
            }
        }

        // DeepFilterNet でフレーム処理
        {
            let mut input_buf = self.input_buffer.lock().unwrap();
            let mut output_buf = self.output_buffer.lock().unwrap();
            let mut model_guard = self.df_model.0.lock().unwrap();

            if let Some(ref mut df_model) = *model_guard {
                df_model.set_atten_lim(atten);

                while input_buf.len() >= hop {
                    let mut in_frame = Array2::zeros((1, hop));
                    let mut out_frame = Array2::zeros((1, hop));

                    for (i, &s) in input_buf[..hop].iter().enumerate() {
                        in_frame[[0, i]] = s;
                    }

                    match df_model.process(in_frame.view(), out_frame.view_mut()) {
                        Ok(_) => {
                            for i in 0..hop {
                                output_buf.push(out_frame[[0, i]]);
                            }
                        }
                        Err(_) => {
                            output_buf.extend_from_slice(&input_buf[..hop]);
                        }
                    }

                    input_buf.drain(..hop);
                }
            }
        }

        // 出力書き込み
        {
            let mut output_buf = self.output_buffer.lock().unwrap();
            if output_buf.len() >= num_samples {
                for (sample_idx, channel_samples) in buffer.iter_samples().enumerate() {
                    let processed = output_buf[sample_idx];
                    for sample in channel_samples {
                        let dry = *sample;
                        *sample = dry * (1.0 - mix) + processed * mix;
                    }
                }
                output_buf.drain(..num_samples);
            }
        }

        ProcessStatus::Normal
    }
}

impl DeepFilterPlugin {
    fn init_model(&mut self, channels: usize) -> Result<usize, Box<dyn std::error::Error>> {
        let df_params = DfParams::default();
        let rt_params = RuntimeParams::default_with_ch(channels);
        let df = DfTract::new(df_params, &rt_params)?;
        let hop = df.hop_size;

        *self.df_model.0.lock().unwrap() = Some(df);
        *self.input_buffer.lock().unwrap() = Vec::with_capacity(hop * 4);
        *self.output_buffer.lock().unwrap() = Vec::with_capacity(hop * 4);

        Ok(hop)
    }
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

nih_export_clap!(DeepFilterPlugin);
nih_export_vst3!(DeepFilterPlugin);
