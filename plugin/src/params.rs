use nice_plug::prelude::*;

#[derive(Params)]
pub(crate) struct DeepFilterParams {
    #[id = "atten_lim"]
    pub(crate) atten_lim: FloatParam,

    #[id = "mix"]
    pub(crate) mix: FloatParam,
}

impl Default for DeepFilterParams {
    fn default() -> Self {
        Self {
            atten_lim: FloatParam::new(
                "Attenuation Limit",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
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
