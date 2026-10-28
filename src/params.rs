//! Plugin parameters for AuraVerb / Rust Vintage Verb.

use std::sync::Arc;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;

use crate::dsp::{ColorMode, ReverbMode};

#[derive(Params)]
pub struct AuraSpaceParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    // Column 1: Mix & PreDelay
    #[id = "mix"]
    pub mix: FloatParam,

    #[id = "predelay"]
    pub predelay: FloatParam,

    // Column 2: Decay (Iconic Concentric Ring Dial)
    #[id = "decay"]
    pub decay: FloatParam,

    // Column 3: Damping (4 Knobs)
    #[id = "damp_hfreq"]
    pub damping_high_freq: FloatParam,

    #[id = "damp_hshelf"]
    pub damping_high_shelf: FloatParam,

    #[id = "damp_bfreq"]
    pub damping_bass_freq: FloatParam,

    #[id = "damp_bmult"]
    pub damping_bass_mult: FloatParam,

    // Column 4: Shape (2 Knobs)
    #[id = "shape_size"]
    pub size: FloatParam,

    #[id = "shape_attack"]
    pub attack: FloatParam,

    // Column 5: Diffusion (2 Knobs)
    #[id = "diff_early"]
    pub diff_early: FloatParam,

    #[id = "diff_late"]
    pub diff_late: FloatParam,

    // Column 6: Modulation (2 Knobs)
    #[id = "mod_rate"]
    pub mod_rate: FloatParam,

    #[id = "mod_depth"]
    pub mod_depth: FloatParam,

    // Column 7: Output EQ (2 Knobs)
    #[id = "eq_high_cut"]
    pub eq_high_cut: FloatParam,

    #[id = "eq_low_cut"]
    pub eq_low_cut: FloatParam,

    // Bottom Bar Selectors
    #[id = "mode"]
    pub reverb_mode: EnumParam<ReverbMode>,

    #[id = "color"]
    pub color_mode: EnumParam<ColorMode>,
}

impl Default for AuraSpaceParams {
    fn default() -> Self {
        Self {
            editor_state: ViziaState::new(|| (1020, 410)),

            mix: FloatParam::new(
                "Mix",
                100.0,
                FloatRange::Linear { min: 0.0, max: 100.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(1))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            predelay: FloatParam::new(
                "PreDelay",
                20.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 500.0,
                    factor: 0.4,
                },
            )
            .with_unit(" ms")
            .with_value_to_string(formatters::v2s_f32_rounded(2))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            decay: FloatParam::new(
                "Decay",
                4.0,
                FloatRange::Skewed {
                    min: 0.2,
                    max: 70.0,
                    factor: 0.35,
                },
            )
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(2))
            .with_smoother(SmoothingStyle::Linear(30.0)),

            damping_high_freq: FloatParam::new(
                "HighFreq",
                6000.0,
                FloatRange::Skewed {
                    min: 1000.0,
                    max: 20000.0,
                    factor: 0.3,
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            damping_high_shelf: FloatParam::new(
                "HighShelf",
                -24.0,
                FloatRange::Linear { min: -24.0, max: 0.0 },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(2))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            damping_bass_freq: FloatParam::new(
                "BassFreq",
                700.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 1000.0,
                    factor: 0.3,
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            damping_bass_mult: FloatParam::new(
                "BassMult",
                1.5,
                FloatRange::Skewed {
                    min: 0.25,
                    max: 4.0,
                    factor: 0.5,
                },
            )
            .with_unit(" X")
            .with_value_to_string(formatters::v2s_f32_rounded(2))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            size: FloatParam::new(
                "Size",
                100.0,
                FloatRange::Linear { min: 0.0, max: 100.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(1))
            .with_smoother(SmoothingStyle::Linear(25.0)),

            attack: FloatParam::new(
                "Attack",
                50.0,
                FloatRange::Linear { min: 0.0, max: 100.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(1))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            diff_early: FloatParam::new(
                "Early",
                100.0,
                FloatRange::Linear { min: 0.0, max: 100.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(1))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            diff_late: FloatParam::new(
                "Late",
                100.0,
                FloatRange::Linear { min: 0.0, max: 100.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(1))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            mod_rate: FloatParam::new(
                "Rate",
                2.53,
                FloatRange::Skewed {
                    min: 0.05,
                    max: 10.0,
                    factor: 0.4,
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(2))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            mod_depth: FloatParam::new(
                "Depth",
                38.0,
                FloatRange::Linear { min: 0.0, max: 100.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(1))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            eq_high_cut: FloatParam::new(
                "HighCut",
                8000.0,
                FloatRange::Skewed {
                    min: 1000.0,
                    max: 20000.0,
                    factor: 0.3,
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            eq_low_cut: FloatParam::new(
                "LowCut",
                10.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 1000.0,
                    factor: 0.3,
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(20.0)),

            reverb_mode: EnumParam::new("Mode", ReverbMode::ConcertHall),
            color_mode: EnumParam::new("Color", ColorMode::Vintage1970s),
        }
    }
}
