//! Worker-only fixed-frame DeepFilterNet DSP pipeline.

use crate::model::{attenuation_is_effectively_zero, DfEngine, ModelInfo};
use crate::resampler::{RateConverter, RateError, RatePlan};

/// Live worker geometry and the checked latency reported to the host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DspInfo {
    pub(crate) model: ModelInfo,
    pub(crate) host_sample_rate: usize,
    pub(crate) host_quantum: usize,
    pub(crate) latency: LatencyBreakdown,
}

/// Latency components with their sample-rate domain encoded in each field name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LatencyBreakdown {
    pub(crate) host_to_model_output_delay_model: usize,
    pub(crate) model_algorithmic_delay_model: usize,
    pub(crate) model_delay_scaled_host: usize,
    pub(crate) model_to_host_output_delay_host: usize,
    pub(crate) core_delay_host: usize,
    pub(crate) runway_delay_host: usize,
    pub(crate) total_host: u32,
}

impl LatencyBreakdown {
    fn new(
        model: ModelInfo,
        host_sample_rate: usize,
        host_quantum: usize,
        host_to_model_output_delay_model: usize,
        model_to_host_output_delay_host: usize,
    ) -> Result<Self, DspError> {
        let model_delay_model = host_to_model_output_delay_model
            .checked_add(model.algorithmic_delay)
            .ok_or_else(|| DspError::new("model-domain latency overflowed"))?;
        let model_delay_scaled_host = round_ratio(model_delay_model, host_sample_rate, model.sample_rate)?;
        let core_delay_host = model_delay_scaled_host
            .checked_add(model_to_host_output_delay_host)
            .ok_or_else(|| DspError::new("host-domain core latency overflowed"))?;
        let runway_delay_host = host_quantum
            .checked_mul(2)
            .ok_or_else(|| DspError::new("host runway latency overflowed"))?;
        let total_host = core_delay_host
            .checked_add(runway_delay_host)
            .ok_or_else(|| DspError::new("total host latency overflowed"))?;
        let total_host = u32::try_from(total_host)
            .map_err(|_| DspError::new("total host latency does not fit u32"))?;

        Ok(Self {
            host_to_model_output_delay_model,
            model_algorithmic_delay_model: model.algorithmic_delay,
            model_delay_scaled_host,
            model_to_host_output_delay_host,
            core_delay_host,
            runway_delay_host,
            total_host,
        })
    }
}

/// Result that distinguishes recoverable model degradation from fatal DSP geometry errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DspProcessOutcome {
    pub(crate) model_faulted: bool,
}

/// One persistent host-quantum-to-host-quantum worker pipeline.
pub(crate) struct DspCore {
    engine: DfEngine,
    host_to_model: RateConverter,
    model_to_host: RateConverter,
    model_input: Vec<f32>,
    model_selected: Vec<f32>,
    raw_delay: Vec<f32>,
    raw_delay_cursor: usize,
    model_error_latched: bool,
    info: DspInfo,
}

impl DspCore {
    /// Construct all model-rate and host-rate state inside the worker thread.
    pub(crate) fn new(engine: DfEngine, host_sample_rate: usize) -> Result<Self, DspError> {
        let plan = RatePlan::preflight(host_sample_rate).map_err(DspError::from_rate)?;
        let model = engine.info();
        plan.verify_model(model).map_err(DspError::from_rate)?;

        let host_to_model = RateConverter::host_to_model(&plan).map_err(DspError::from_rate)?;
        let model_to_host = RateConverter::model_to_host(&plan).map_err(DspError::from_rate)?;
        if host_to_model.input_frames() != plan.host_quantum
            || host_to_model.output_frames() != model.hop_size
            || model_to_host.input_frames() != model.hop_size
            || model_to_host.output_frames() != plan.host_quantum
        {
            return Err(DspError::new("constructed converter sizes differ from model or host plan"));
        }

        let latency = LatencyBreakdown::new(
            model,
            plan.host_sample_rate,
            plan.host_quantum,
            host_to_model.output_delay(),
            model_to_host.output_delay(),
        )?;
        let raw_delay_len = model.algorithmic_delay;
        let info = DspInfo {
            model,
            host_sample_rate: plan.host_sample_rate,
            host_quantum: plan.host_quantum,
            latency,
        };

        Ok(Self {
            engine,
            host_to_model,
            model_to_host,
            model_input: vec![0.0; model.hop_size],
            model_selected: vec![0.0; model.hop_size],
            raw_delay: vec![0.0; raw_delay_len],
            raw_delay_cursor: 0,
            model_error_latched: false,
            info,
        })
    }

    pub(crate) fn info(&self) -> DspInfo {
        self.info
    }

    /// Convert and process one exact host quantum, returning delayed raw audio on model failure.
    pub(crate) fn process_chunk(
        &mut self,
        host_input: &[f32],
        requested_attenuation: f32,
        host_output: &mut [f32],
    ) -> Result<DspProcessOutcome, DspError> {
        if host_input.len() != self.info.host_quantum
            || host_output.len() != self.info.host_quantum
        {
            return Err(DspError::new("worker chunk does not match the negotiated host quantum"));
        }

        self.host_to_model
            .process_into_buffer(host_input, &mut self.model_input)
            .map_err(DspError::from_rate)?;
        self.push_raw_delay();

        if !self.model_error_latched {
            match self.engine.process_hop(&self.model_input, requested_attenuation) {
                Ok(model_output) if !attenuation_is_effectively_zero(requested_attenuation) => {
                    self.model_selected.copy_from_slice(model_output);
                }
                Ok(_) => {}
                Err(_) => self.model_error_latched = true,
            }
        }

        self.model_to_host
            .process_into_buffer(&self.model_selected, host_output)
            .map_err(DspError::from_rate)?;

        Ok(DspProcessOutcome {
            model_faulted: self.model_error_latched,
        })
    }

    /// Restore fresh model/resampler state before the worker acknowledges a generation reset.
    pub(crate) fn reset(&mut self) {
        self.engine.reset();
        self.host_to_model.reset();
        self.model_to_host.reset();
        self.model_input.fill(0.0);
        self.model_selected.fill(0.0);
        self.raw_delay.fill(0.0);
        self.raw_delay_cursor = 0;
        self.model_error_latched = false;
    }

    fn push_raw_delay(&mut self) {
        if self.raw_delay.is_empty() {
            self.model_selected.copy_from_slice(&self.model_input);
            return;
        }

        for (raw, delayed) in self
            .model_input
            .iter()
            .copied()
            .zip(self.model_selected.iter_mut())
        {
            *delayed = self.raw_delay[self.raw_delay_cursor];
            self.raw_delay[self.raw_delay_cursor] = raw;
            self.raw_delay_cursor = (self.raw_delay_cursor + 1) % self.raw_delay.len();
        }
    }
}

fn round_ratio(value: usize, numerator: usize, denominator: usize) -> Result<usize, DspError> {
    if denominator == 0 {
        return Err(DspError::new("model sample rate must not be zero"));
    }
    let scaled = value
        .checked_mul(numerator)
        .ok_or_else(|| DspError::new("latency scaling overflowed"))?;
    let half = denominator / 2;
    scaled
        .checked_add(half)
        .map(|rounded| rounded / denominator)
        .ok_or_else(|| DspError::new("latency rounding overflowed"))
}

/// Owned DSP construction or fatal conversion error.
#[derive(Debug)]
pub(crate) struct DspError(String);

impl DspError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    fn from_rate(error: RateError) -> Self {
        Self(error.to_string())
    }
}

impl std::fmt::Display for DspError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DspError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "model-ll")]
    use crate::model::{MODEL_HOP_SIZE, MODEL_SAMPLE_RATE};

    fn ll_model() -> ModelInfo {
        ModelInfo {
            sample_rate: 48_000,
            channels: 1,
            hop_size: 480,
            fft_size: 960,
            lookahead: 0,
            algorithmic_delay: 480,
        }
    }

    #[test]
    fn latency_breakdown_keeps_rate_domains_explicit() {
        let cases = [
            (48_000, 480, 0, 0, 480, 480, 960, 1_440),
            (44_100, 441, 240, 220, 662, 882, 882, 1_764),
            (96_000, 960, 240, 480, 1_440, 1_920, 1_920, 3_840),
        ];
        for (
            host_rate,
            host_quantum,
            host_to_model_delay,
            model_to_host_delay,
            scaled_model_delay,
            core_delay,
            runway_delay,
            total,
        ) in cases
        {
            let latency = LatencyBreakdown::new(
                ll_model(),
                host_rate,
                host_quantum,
                host_to_model_delay,
                model_to_host_delay,
            )
            .expect("checked latency case must construct");
            assert_eq!(
                latency.host_to_model_output_delay_model,
                host_to_model_delay
            );
            assert_eq!(latency.model_algorithmic_delay_model, 480);
            assert_eq!(latency.model_delay_scaled_host, scaled_model_delay);
            assert_eq!(latency.model_to_host_output_delay_host, model_to_host_delay);
            assert_eq!(latency.core_delay_host, core_delay);
            assert_eq!(latency.runway_delay_host, runway_delay);
            assert_eq!(latency.total_host, total);
        }
    }

    #[test]
    fn ratio_rounding_is_nearest_and_overflow_checked() {
        assert_eq!(round_ratio(720, 44_100, 48_000).expect("ratio must fit"), 662);
        assert_eq!(round_ratio(720, 96_000, 48_000).expect("ratio must fit"), 1_440);
        assert!(round_ratio(1, 1, 0).is_err());
        assert!(round_ratio(usize::MAX, 2, 1).is_err());
    }

    #[test]
    fn latency_construction_rejects_overflow() {
        let mut model = ll_model();
        model.algorithmic_delay = usize::MAX;
        assert!(LatencyBreakdown::new(model, 48_000, 480, 1, 0).is_err());
        assert!(LatencyBreakdown::new(ll_model(), 48_000, usize::MAX, 0, 0).is_err());
    }

    #[cfg(feature = "model-ll")]
    fn real_fixture(chunks: usize, frequency: f32) -> Vec<f32> {
        (0..chunks * MODEL_HOP_SIZE)
            .map(|index| {
                let phase = index as f32 / MODEL_SAMPLE_RATE as f32;
                0.2 * (phase * frequency * std::f32::consts::TAU).sin()
                    + 0.03 * (phase * 2_113.0 * std::f32::consts::TAU).sin()
            })
            .collect()
    }

    #[cfg(feature = "model-ll")]
    fn render_real_core(core: &mut DspCore, input: &[f32], attenuation: f32) -> Vec<f32> {
        let quantum = core.info().host_quantum;
        assert_eq!(input.len() % quantum, 0);
        let mut rendered = Vec::with_capacity(input.len());
        for chunk in input.chunks_exact(quantum) {
            let mut output = vec![0.0; quantum];
            let outcome = core
                .process_chunk(chunk, attenuation, &mut output)
                .expect("bounded real DSP chunk must process");
            assert!(!outcome.model_faulted);
            assert!(output.iter().all(|sample| sample.is_finite()));
            rendered.extend_from_slice(&output);
        }
        rendered
    }

    #[cfg(feature = "model-ll")]
    #[test]
    fn real_core_reset_after_nonzero_audio_matches_its_fresh_run() {
        let _serial = crate::test_support::serialize_real_model();
        let engine = DfEngine::new().expect("official LL model must construct");
        let mut core = DspCore::new(engine, MODEL_SAMPLE_RATE)
            .expect("48 kHz real DSP core must construct");
        assert_eq!(core.info().model, ll_model());

        let fixture = real_fixture(8, 523.25);
        let fresh = render_real_core(&mut core, &fixture, 20.0);
        assert!(fresh.iter().any(|sample| sample.abs() > 1.0e-8));

        let dirty = real_fixture(5, 997.0);
        let dirty_output = render_real_core(&mut core, &dirty, 20.0);
        assert!(dirty_output.iter().any(|sample| sample.abs() > 1.0e-8));

        core.reset();
        let after_reset = render_real_core(&mut core, &fixture, 20.0);
        assert_eq!(after_reset, fresh);

        core.reset();
        let after_repeated_reset = render_real_core(&mut core, &fixture, 20.0);
        assert_eq!(after_repeated_reset, fresh);
    }
}
