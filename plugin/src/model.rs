//! Worker-owned DeepFilterNet model state.
//!
//! `DfTract` is intentionally confined to this module and constructed by the
//! worker thread. The host callback only exchanges fixed-size audio messages.

use df::tract::{DfParams, DfTract, RuntimeParams};
use ndarray::Array2;

/// Fixed timing expected by the embedded official DeepFilterNet model.
pub(crate) const MODEL_SAMPLE_RATE: usize = 48_000;
pub(crate) const MODEL_HOP_SIZE: usize = 480;
const MIN_EFFECTIVE_ATTENUATION_DB: f32 = 0.01;

/// Immutable metadata derived from the constructed embedded model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ModelInfo {
    pub(crate) sample_rate: usize,
    pub(crate) channels: usize,
    pub(crate) hop_size: usize,
    pub(crate) fft_size: usize,
    pub(crate) lookahead: usize,
    pub(crate) algorithmic_delay: usize,
}

impl ModelInfo {
    fn from_model(model: &DfTract) -> Result<Self, ModelError> {
        if model.ch != 1 {
            return Err(ModelError::new("DeepFilterNet model must have one channel"));
        }
        if model.sr != MODEL_SAMPLE_RATE {
            return Err(ModelError::new("DeepFilterNet model must run at 48 kHz"));
        }
        if model.hop_size == 0 || model.fft_size == 0 || model.fft_size < model.hop_size {
            return Err(ModelError::new("DeepFilterNet model has inconsistent frame sizes"));
        }

        let analysis_delay = model
            .fft_size
            .checked_sub(model.hop_size)
            .ok_or_else(|| ModelError::new("DeepFilterNet analysis delay underflowed"))?;
        let lookahead_delay = model
            .lookahead
            .checked_mul(model.hop_size)
            .ok_or_else(|| ModelError::new("DeepFilterNet lookahead delay overflowed"))?;
        let algorithmic_delay = analysis_delay
            .checked_add(lookahead_delay)
            .ok_or_else(|| ModelError::new("DeepFilterNet algorithmic delay overflowed"))?;

        Ok(Self {
            sample_rate: model.sr,
            channels: model.ch,
            hop_size: model.hop_size,
            fft_size: model.fft_size,
            lookahead: model.lookahead,
            algorithmic_delay,
        })
    }
}

/// Small owned error type kept on the worker side of the audio boundary.
#[derive(Debug)]
pub(super) struct ModelError(String);

impl ModelError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A non-`Send` model engine that exists only on the persistent worker thread.
pub(super) struct DfEngine {
    pristine: DfTract,
    active: DfTract,
    input: Array2<f32>,
    output: Array2<f32>,
    info: ModelInfo,
    last_applied_attenuation: Option<f32>,
}

impl DfEngine {
    pub(super) fn new() -> Result<Self, ModelError> {
        let params = DfParams::default();
        let runtime = RuntimeParams::default_with_ch(1);
        let pristine = DfTract::new(params, &runtime)
            .map_err(|error| ModelError::new(format!("could not construct DeepFilterNet: {error}")))?;
        let info = ModelInfo::from_model(&pristine)?;
        let active = pristine.clone();
        let input = Array2::from_elem((1, info.hop_size), 0.0);
        let output = Array2::from_elem((1, info.hop_size), 0.0);

        Ok(Self {
            pristine,
            active,
            input,
            output,
            info,
            last_applied_attenuation: None,
        })
    }

    pub(super) fn info(&self) -> ModelInfo {
        self.info
    }

    /// Restore a pristine model and reusable frames before acknowledging reset.
    pub(super) fn reset(&mut self) {
        self.active = self.pristine.clone();
        self.input.fill(0.0);
        self.output.fill(0.0);
        self.last_applied_attenuation = None;
    }

    /// Process one exact mono model hop with the requested attenuation limit.
    pub(super) fn process_hop(
        &mut self,
        samples: &[f32],
        requested_attenuation: f32,
    ) -> Result<&[f32], ModelError> {
        if samples.len() != self.info.hop_size {
            return Err(ModelError::new("worker chunk does not match the model hop size"));
        }

        let effective_attenuation = effective_attenuation(requested_attenuation);
        if self.last_applied_attenuation != Some(effective_attenuation) {
            self.active.set_atten_lim(effective_attenuation);
            self.last_applied_attenuation = Some(effective_attenuation);
        }

        let input = self
            .input
            .as_slice_mut()
            .ok_or_else(|| ModelError::new("model input frame is not contiguous"))?;
        input.copy_from_slice(samples);
        self.active
            .process(self.input.view(), self.output.view_mut())
            .map_err(|error| ModelError::new(format!("DeepFilterNet hop processing failed: {error}")))?;

        self.output
            .as_slice()
            .ok_or_else(|| ModelError::new("model output frame is not contiguous"))
    }
}

pub(super) fn effective_attenuation(requested: f32) -> f32 {
    let requested = sanitized_attenuation(requested);
    requested.max(MIN_EFFECTIVE_ATTENUATION_DB)
}

/// Whether the user requested transparent enhancement while the model still advances.
pub(super) fn attenuation_is_effectively_zero(requested: f32) -> bool {
    sanitized_attenuation(requested) < MIN_EFFECTIVE_ATTENUATION_DB
}

fn sanitized_attenuation(requested: f32) -> f32 {
    if requested.is_finite() {
        requested.abs().min(100.0)
    } else {
        100.0
    }
}

#[cfg(all(test, feature = "model-ll"))]
mod tests {
    use super::*;

    fn model_fixture(sample_offset: usize) -> Vec<f32> {
        (0..MODEL_HOP_SIZE)
            .map(|index| {
                let phase = (sample_offset + index) as f32 / MODEL_SAMPLE_RATE as f32;
                0.15 * (phase * 440.0 * std::f32::consts::TAU).sin()
                    + 0.05 * (phase * 1_731.0 * std::f32::consts::TAU).sin()
            })
            .collect()
    }

    #[test]
    fn official_ll_metadata_and_one_channel_shape_are_live() {
        let _serial = crate::test_support::serialize_real_model();
        let mut engine = DfEngine::new().expect("official LL model must construct");
        assert_eq!(
            engine.info(),
            ModelInfo {
                sample_rate: 48_000,
                channels: 1,
                hop_size: 480,
                fft_size: 960,
                lookahead: 0,
                algorithmic_delay: 480,
            }
        );
        assert!(engine.process_hop(&[0.0; MODEL_HOP_SIZE - 1], 20.0).is_err());

        let mut observed_nonzero = false;
        for hop in 0..8 {
            let input = model_fixture(hop * MODEL_HOP_SIZE);
            let output = engine
                .process_hop(&input, 20.0)
                .expect("bounded one-channel inference must succeed");
            assert_eq!(output.len(), MODEL_HOP_SIZE);
            assert!(output.iter().all(|sample| sample.is_finite()));
            observed_nonzero |= output.iter().any(|sample| sample.abs() > 1.0e-8);
        }
        assert!(
            observed_nonzero,
            "bounded 20 dB model output must contain a non-silent sample"
        );
    }
}
