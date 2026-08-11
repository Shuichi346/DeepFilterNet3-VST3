//! Persistent mono conversion between the host timeline and the model timeline.

use rubato::{FftFixedOut, Resampler};

use crate::model::{ModelInfo, MODEL_HOP_SIZE, MODEL_SAMPLE_RATE};
use crate::worker::MAX_HOST_QUANTUM;

/// Exact frame geometry derived without constructing a resampler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RatePlan {
    pub(crate) host_sample_rate: usize,
    pub(crate) host_quantum: usize,
    host_to_model: ConverterPlan,
    model_to_host: ConverterPlan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ConverterPlan {
    input_frames: usize,
    output_frames: usize,
    output_delay: usize,
}

impl RatePlan {
    /// Validate the supported exact fixed-frame rate geometry without allocating rubato state.
    pub(crate) fn preflight(host_sample_rate: usize) -> Result<Self, RateError> {
        if host_sample_rate == 0 {
            return Err(RateError::new("host sample rate must not be zero"));
        }

        if host_sample_rate == MODEL_SAMPLE_RATE {
            return Ok(Self {
                host_sample_rate,
                host_quantum: MODEL_HOP_SIZE,
                host_to_model: ConverterPlan {
                    input_frames: MODEL_HOP_SIZE,
                    output_frames: MODEL_HOP_SIZE,
                    output_delay: 0,
                },
                model_to_host: ConverterPlan {
                    input_frames: MODEL_HOP_SIZE,
                    output_frames: MODEL_HOP_SIZE,
                    output_delay: 0,
                },
            });
        }

        let host_to_model = fixed_out_plan(host_sample_rate, MODEL_SAMPLE_RATE, MODEL_HOP_SIZE)?;
        let host_quantum = host_to_model.input_frames;
        if host_quantum == 0 || host_quantum > MAX_HOST_QUANTUM {
            return Err(RateError::new("host resampling quantum is unsupported"));
        }

        let model_to_host = fixed_out_plan(MODEL_SAMPLE_RATE, host_sample_rate, host_quantum)?;
        if host_to_model.output_frames != MODEL_HOP_SIZE
            || model_to_host.input_frames != MODEL_HOP_SIZE
            || model_to_host.output_frames != host_quantum
        {
            return Err(RateError::new("host rate does not produce exact fixed resampler frames"));
        }

        Ok(Self {
            host_sample_rate,
            host_quantum,
            host_to_model,
            model_to_host,
        })
    }

    pub(crate) fn verify_model(&self, info: ModelInfo) -> Result<(), RateError> {
        if info.sample_rate != MODEL_SAMPLE_RATE || info.hop_size != MODEL_HOP_SIZE {
            return Err(RateError::new("embedded model rate or hop differs from supported geometry"));
        }
        Ok(())
    }
}

/// A converter whose buffers and rubato state never cross the worker boundary.
pub(crate) enum RateConverter {
    Identity {
        input_frames: usize,
        output_frames: usize,
    },
    Rubato {
        resampler: FftFixedOut<f32>,
        input: Vec<f32>,
        output: Vec<f32>,
        expected: ConverterPlan,
    },
}

impl RateConverter {
    pub(crate) fn host_to_model(plan: &RatePlan) -> Result<Self, RateError> {
        Self::new(plan, plan.host_to_model, plan.host_sample_rate, MODEL_SAMPLE_RATE)
    }

    pub(crate) fn model_to_host(plan: &RatePlan) -> Result<Self, RateError> {
        Self::new(plan, plan.model_to_host, MODEL_SAMPLE_RATE, plan.host_sample_rate)
    }

    fn new(
        plan: &RatePlan,
        expected: ConverterPlan,
        input_rate: usize,
        output_rate: usize,
    ) -> Result<Self, RateError> {
        if plan.host_sample_rate == MODEL_SAMPLE_RATE {
            return Ok(Self::Identity {
                input_frames: expected.input_frames,
                output_frames: expected.output_frames,
            });
        }

        let resampler = FftFixedOut::<f32>::new(input_rate, output_rate, expected.output_frames, 1, 1)
            .map_err(|error| RateError::new(format!("could not construct resampler: {error}")))?;
        if resampler.input_frames_next() != expected.input_frames
            || resampler.output_frames_next() != expected.output_frames
            || resampler.output_delay() != expected.output_delay
        {
            return Err(RateError::new("rubato frame geometry differs from the checked rate plan"));
        }

        Ok(Self::Rubato {
            resampler,
            input: vec![0.0; expected.input_frames],
            output: vec![0.0; expected.output_frames],
            expected,
        })
    }

    pub(crate) fn input_frames(&self) -> usize {
        match self {
            Self::Identity { input_frames, .. } => *input_frames,
            Self::Rubato { expected, .. } => expected.input_frames,
        }
    }

    pub(crate) fn output_frames(&self) -> usize {
        match self {
            Self::Identity { output_frames, .. } => *output_frames,
            Self::Rubato { expected, .. } => expected.output_frames,
        }
    }

    pub(crate) fn output_delay(&self) -> usize {
        match self {
            Self::Identity { .. } => 0,
            Self::Rubato { expected, .. } => expected.output_delay,
        }
    }

    /// Convert exactly one checked frame group without using rubato's allocating API.
    pub(crate) fn process_into_buffer(
        &mut self,
        source: &[f32],
        destination: &mut [f32],
    ) -> Result<(), RateError> {
        if source.len() != self.input_frames() || destination.len() != self.output_frames() {
            return Err(RateError::new("resampler input or output length is not the negotiated size"));
        }

        match self {
            Self::Identity { .. } => destination.copy_from_slice(source),
            Self::Rubato {
                resampler,
                input,
                output,
                expected,
            } => {
                if resampler.input_frames_next() != expected.input_frames
                    || resampler.output_frames_next() != expected.output_frames
                {
                    return Err(RateError::new("rubato next-frame geometry changed unexpectedly"));
                }
                input.copy_from_slice(source);
                let input_channels = [&input[..]];
                let mut output_channels = [&mut output[..]];
                let (consumed, produced) = resampler
                    .process_into_buffer(&input_channels, &mut output_channels, None)
                    .map_err(|error| RateError::new(format!("rubato processing failed: {error}")))?;
                if consumed != expected.input_frames || produced != expected.output_frames {
                    return Err(RateError::new("rubato consumed or produced an unexpected frame count"));
                }
                if resampler.input_frames_next() != expected.input_frames
                    || resampler.output_frames_next() != expected.output_frames
                {
                    return Err(RateError::new(
                        "rubato fixed-frame geometry changed after processing",
                    ));
                }
                destination.copy_from_slice(output);
            }
        }
        Ok(())
    }

    pub(crate) fn reset(&mut self) {
        if let Self::Rubato {
            resampler,
            input,
            output,
            ..
        } = self
        {
            resampler.reset();
            input.fill(0.0);
            output.fill(0.0);
        }
    }
}

fn fixed_out_plan(
    input_rate: usize,
    output_rate: usize,
    output_frames: usize,
) -> Result<ConverterPlan, RateError> {
    let divisor = gcd(input_rate, output_rate);
    let minimum_output = output_rate
        .checked_div(divisor)
        .ok_or_else(|| RateError::new("invalid resampler rate divisor"))?;
    let fft_chunks = ceil_div(output_frames, minimum_output)?;
    let fft_size_out = fft_chunks
        .checked_mul(output_rate)
        .and_then(|value| value.checked_div(divisor))
        .ok_or_else(|| RateError::new("resampler output frame calculation overflowed"))?;
    let input_frames = fft_chunks
        .checked_mul(input_rate)
        .and_then(|value| value.checked_div(divisor))
        .ok_or_else(|| RateError::new("resampler input frame calculation overflowed"))?;
    if fft_size_out != output_frames {
        return Err(RateError::new(
            "host rate does not preserve a constant exact fixed-output quantum",
        ));
    }

    Ok(ConverterPlan {
        input_frames,
        output_frames,
        output_delay: fft_size_out / 2,
    })
}

fn gcd(mut left: usize, mut right: usize) -> usize {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn ceil_div(value: usize, divisor: usize) -> Result<usize, RateError> {
    if divisor == 0 {
        return Err(RateError::new("resampler divisor must not be zero"));
    }
    value
        .checked_add(divisor - 1)
        .map(|rounded| rounded / divisor)
        .ok_or_else(|| RateError::new("resampler frame calculation overflowed"))
}

/// Small owned construction and conversion error kept off the callback boundary.
#[derive(Debug)]
pub(crate) struct RateError(String);

impl RateError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for RateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_derives_exact_supported_host_quanta() {
        let expected = [
            (44_100, 441),
            (48_000, 480),
            (88_200, 882),
            (96_000, 960),
            (176_400, 1_764),
            (192_000, 1_920),
        ];
        for (sample_rate, host_quantum) in expected {
            let plan = RatePlan::preflight(sample_rate).expect("declared rate must preflight");
            assert_eq!(plan.host_sample_rate, sample_rate);
            assert_eq!(plan.host_quantum, host_quantum);
            assert_eq!(plan.host_to_model.output_frames, MODEL_HOP_SIZE);
            assert_eq!(plan.model_to_host.input_frames, MODEL_HOP_SIZE);
            assert_eq!(plan.model_to_host.output_frames, host_quantum);
        }
    }

    #[test]
    fn invalid_or_inexact_rates_fail_before_converter_construction() {
        assert!(RatePlan::preflight(0).is_err());
        assert!(RatePlan::preflight(1).is_err());
        assert!(RatePlan::preflight(usize::MAX).is_err());
    }

    #[test]
    fn model_geometry_must_match_the_embedded_contract() {
        let plan = RatePlan::preflight(MODEL_SAMPLE_RATE).expect("identity plan must preflight");
        let matching = ModelInfo {
            sample_rate: MODEL_SAMPLE_RATE,
            channels: 1,
            hop_size: MODEL_HOP_SIZE,
            fft_size: 960,
            lookahead: 0,
            algorithmic_delay: 480,
        };
        assert!(plan.verify_model(matching).is_ok());
        assert!(plan
            .verify_model(ModelInfo {
                sample_rate: 44_100,
                ..matching
            })
            .is_err());
        assert!(plan
            .verify_model(ModelInfo {
                hop_size: MODEL_HOP_SIZE / 2,
                ..matching
            })
            .is_err());
    }

    #[test]
    fn identity_converter_copies_exact_frames_and_resets_in_place() {
        let plan = RatePlan::preflight(MODEL_SAMPLE_RATE).expect("identity plan must preflight");
        let mut converter = RateConverter::host_to_model(&plan)
            .expect("identity converter must construct");
        let source: Vec<_> = (0..MODEL_HOP_SIZE).map(|index| index as f32).collect();
        let mut output = vec![0.0; MODEL_HOP_SIZE];
        converter
            .process_into_buffer(&source, &mut output)
            .expect("identity conversion must succeed");
        assert_eq!(output, source);
        converter.reset();
        output.fill(0.0);
        converter
            .process_into_buffer(&source, &mut output)
            .expect("reset identity conversion must succeed");
        assert_eq!(output, source);
        assert!(converter
            .process_into_buffer(&source[..MODEL_HOP_SIZE - 1], &mut output)
            .is_err());
    }

    #[test]
    fn integer_helpers_are_exact_and_checked() {
        assert_eq!(gcd(48_000, 44_100), 300);
        assert_eq!(gcd(48_000, 96_000), 48_000);
        assert_eq!(ceil_div(480, 160).expect("exact division must succeed"), 3);
        assert_eq!(ceil_div(481, 160).expect("rounded division must succeed"), 4);
        assert!(ceil_div(1, 0).is_err());
        assert!(ceil_div(usize::MAX, 2).is_err());
    }
}
