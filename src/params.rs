use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum EqMode {
    #[name = "ISO"]
    Isolator,
    #[name = "EQ"]
    Eq,
}

/// Maps bipolar Trim knob position (`-1.0..=1.0`, center `0.0 = 0 dB`) to linear gain.
/// `-1.0` -> `0.0` (-inf dB), `0.0` -> `1.0` (0 dB), `+1.0` -> `+9.0 dB`.
#[inline]
pub fn trim_knob_to_gain(pos: f32) -> f64 {
    let p = (pos as f64).clamp(-1.0, 1.0);
    if p >= 0.0 {
        10.0_f64.powf((p * 9.0) / 20.0)
    } else if p <= -0.999 {
        0.0
    } else {
        // Smooth taper from 0 dB down to -inf dB
        (1.0 + p).powf(2.0)
    }
}

/// Maps channel fader position (`0.0..=1.0`, top `1.0 = 0 dB`) to linear gain.
/// Uses a smooth club-mixer taper reaching exact `-∞ dB` (`0.0`) at the bottom.
#[inline]
pub fn fader_to_gain(pos: f32) -> f64 {
    let p = (pos as f64).clamp(0.0, 1.0);
    if p <= 0.001 {
        0.0
    } else {
        p * p
    }
}

#[derive(Params)]
pub struct A9StripParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    /// Overall channel input Trim (`-∞ dB` to `+9 dB`, 0 dB at 12 o'clock).
    #[id = "trim"]
    pub trim: FloatParam,

    /// Switchable 3-band EQ (-26/+6 dB) vs full-kill Isolator (-∞/+6 dB).
    #[id = "eq_mode"]
    pub eq_mode: EnumParam<EqMode>,

    /// High band knob (`-1.0..=1.0`, center `0.0 = 0 dB`).
    #[id = "eq_hi"]
    pub eq_hi: FloatParam,

    /// Mid band knob (`-1.0..=1.0`, center `0.0 = 0 dB`).
    #[id = "eq_mid"]
    pub eq_mid: FloatParam,

    /// Low band knob (`-1.0..=1.0`, center `0.0 = 0 dB`).
    #[id = "eq_low"]
    pub eq_low: FloatParam,

    /// Sound Color FX Filter Enable button.
    #[id = "filter_on"]
    pub filter_on: BoolParam,

    /// Center-zero Filter knob (`-1.0` LPF .. `0.0` Bypass .. `+1.0` HPF).
    #[id = "filter"]
    pub filter: FloatParam,

    /// Sound Color FX Parameter knob (Filter Resonance / Q, `0.0..=1.0`).
    #[id = "filter_res"]
    pub filter_res: FloatParam,

    /// Signature DJM-A9 Center Lock switch for the Filter knob.
    #[id = "center_lock"]
    pub center_lock: BoolParam,

    /// Subtle output headroom soft clipper.
    #[id = "soft_clip"]
    pub soft_clip: BoolParam,

    /// Pioneer-style vertical channel volume fader (`0.0` = -∞ dB .. `1.0` = 0.0 dB).
    #[id = "fader"]
    pub fader: FloatParam,
}

impl Default for A9StripParams {
    fn default() -> Self {
        let eq_formatter = Arc::new(|v: f32| {
            if v.abs() < 0.005 {
                "0.0 dB".to_string()
            } else if v > 0.0 {
                format!("+{:.1} dB", v * 6.0)
            } else if v <= -0.995 {
                "CUT / -∞".to_string()
            } else {
                format!("{:.0}%", v * 100.0)
            }
        });

        Self {
            editor_state: EguiState::from_size(330, 680),

            trim: FloatParam::new(
                "Trim",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_value_to_string(Arc::new(|v| {
                let g = trim_knob_to_gain(v);
                if g <= 1e-5 {
                    "-∞ dB".to_string()
                } else {
                    let db = 20.0 * g.log10();
                    if db.abs() < 0.05 {
                        "0.0 dB".to_string()
                    } else {
                        format!("{db:+.1} dB")
                    }
                }
            })),

            eq_mode: EnumParam::new("EQ / ISO Mode", EqMode::Isolator),

            eq_hi: FloatParam::new(
                "High",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(6.0))
            .with_value_to_string(eq_formatter.clone()),

            eq_mid: FloatParam::new(
                "Mid",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(6.0))
            .with_value_to_string(eq_formatter.clone()),

            eq_low: FloatParam::new(
                "Low",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(6.0))
            .with_value_to_string(eq_formatter),

            filter_on: BoolParam::new("Filter On", true),

            filter: FloatParam::new(
                "Filter",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(6.0))
            .with_value_to_string(Arc::new(|v| {
                if v.abs() <= 0.005 {
                    "CENTER".to_string()
                } else if v < 0.0 {
                    let t = (-v as f64).clamp(0.0, 1.0);
                    let hz = 20000.0 * (25.0_f64 / 20000.0).powf(t);
                    if hz >= 1000.0 {
                        format!("LPF {:.1} kHz", hz / 1000.0)
                    } else {
                        format!("LPF {:.0} Hz", hz)
                    }
                } else {
                    let t = (v as f64).clamp(0.0, 1.0);
                    let hz = 20.0 * (18000.0_f64 / 20.0).powf(t);
                    if hz >= 1000.0 {
                        format!("HPF {:.1} kHz", hz / 1000.0)
                    } else {
                        format!("HPF {:.0} Hz", hz)
                    }
                }
            })),

            filter_res: FloatParam::new(
                "Parameter (Resonance)",
                0.35,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(8.0))
            .with_unit("%")
            .with_value_to_string(Arc::new(|v| format!("{:.0}", v * 100.0))),

            center_lock: BoolParam::new("Center Lock", true),

            soft_clip: BoolParam::new("Soft Clip", true),

            fader: FloatParam::new(
                "Channel Fader",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_value_to_string(Arc::new(|v| {
                let g = fader_to_gain(v);
                if g <= 1e-5 {
                    "-∞ dB".to_string()
                } else {
                    let db = 20.0 * g.log10();
                    if db.abs() < 0.05 {
                        "0.0 dB".to_string()
                    } else {
                        format!("{db:.1} dB")
                    }
                }
            })),
        }
    }
}
