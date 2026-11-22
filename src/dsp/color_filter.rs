use std::f64::consts::PI;

/// 2-pole Zero-Delay Feedback (ZDF / Cytomic TPT) State-Variable Filter in `f64`.
/// Unconditionally stable under fast DJ filter sweeps and high resonance.
#[derive(Debug, Clone, Copy, Default)]
struct ZdfSvf {
    ic1eq: f64,
    ic2eq: f64,
}

impl ZdfSvf {
    fn new() -> Self {
        Self::default()
    }

    fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }

    /// Processes one sample and returns `(lowpass, highpass)`.
    #[inline]
    fn process(&mut self, v0: f64, g: f64, k: f64) -> (f64, f64) {
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        let v3 = v0 - self.ic2eq;
        let v1 = a1 * self.ic1eq + a2 * v3;
        let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;

        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        let lp = v2;
        let hp = v0 - k * v1 - v2;
        (lp, hp)
    }
}

/// Pioneer DJM-A9 Sound Color FX Filter (Center-Zero LPF/HPF + Parameter Resonance).
///
/// - `filter_pos` in `-1.0..=1.0`:
///   - `-1.0..0.0`: Low-Pass Filter sweeping logarithmically from 20 kHz down to 25 Hz
///   - `0.0`: True transparent bypass (with smooth center crossfade)
///   - `0.0..1.0`: High-Pass Filter sweeping logarithmically from 20 Hz up to 18 kHz
/// - `resonance` (`PARAMETER` knob) in `0.0..=1.0`:
///   - `0.0`: Smooth Butterworth (`Q = 0.707`, zero resonant peak)
///   - `1.0`: Aggressive classic Pioneer club resonance with musical soft saturation
pub struct ColorFilterChannel {
    sample_rate: f64,
    stage1: ZdfSvf,
    stage2: ZdfSvf,
    enable_mix: f64,
    enable_coeff: f64,
}

impl ColorFilterChannel {
    const MIN_FREQ_LPF: f64 = 25.0;
    const MAX_FREQ_LPF: f64 = 20000.0;
    const MIN_FREQ_HPF: f64 = 20.0;
    const MAX_FREQ_HPF: f64 = 18000.0;

    /// Small center deadzone where output is strictly dry.
    const CENTER_DEADZONE: f64 = 0.005;
    /// Width of the smooth dry->wet engagement ramp near 12 o'clock.
    const CENTER_RAMP_END: f64 = 0.045;

    pub fn new(sample_rate: f64) -> Self {
        let mut f = Self {
            sample_rate: sample_rate.max(8000.0),
            stage1: ZdfSvf::new(),
            stage2: ZdfSvf::new(),
            enable_mix: 1.0,
            enable_coeff: 0.01,
        };
        f.set_sample_rate(sample_rate);
        f
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(8000.0);
        // ~5 ms click-free enable/bypass ramp
        self.enable_coeff = 1.0 - (-1.0 / (0.005 * self.sample_rate)).exp();
        self.reset();
    }

    pub fn reset(&mut self) {
        self.stage1.reset();
        self.stage2.reset();
    }

    /// Computes cutoff frequency (Hz) for a given bipolar filter position (`-1.0..=1.0`).
    #[inline]
    pub fn cutoff_hz(&self, filter_pos: f64) -> f64 {
        let p = filter_pos.clamp(-1.0, 1.0);
        let nyquist_limit = (self.sample_rate * 0.45).min(20000.0);
        if p < 0.0 {
            // LPF: 0.0 -> 20 kHz, -1.0 -> 25 Hz
            let t = (-p).clamp(0.0, 1.0);
            let max_f = Self::MAX_FREQ_LPF.min(nyquist_limit);
            let ratio = Self::MIN_FREQ_LPF / max_f;
            (max_f * ratio.powf(t)).clamp(Self::MIN_FREQ_LPF, nyquist_limit)
        } else {
            // HPF: 0.0 -> 20 Hz, +1.0 -> 18 kHz
            let t = p.clamp(0.0, 1.0);
            let max_f = Self::MAX_FREQ_HPF.min(nyquist_limit);
            let ratio = max_f / Self::MIN_FREQ_HPF;
            (Self::MIN_FREQ_HPF * ratio.powf(t)).clamp(Self::MIN_FREQ_HPF, nyquist_limit)
        }
    }

    #[inline]
    pub fn process(&mut self, x: f64, filter_pos: f64, resonance: f64, enabled: bool) -> f64 {
        let target_enable = if enabled { 1.0 } else { 0.0 };
        self.enable_mix += self.enable_coeff * (target_enable - self.enable_mix);

        let p = filter_pos.clamp(-1.0, 1.0);
        let abs_p = p.abs();

        // Smooth wet engagement near 12 o'clock so leaving center never clicks or snaps
        let center_wet = if abs_p <= Self::CENTER_DEADZONE {
            0.0
        } else if abs_p < Self::CENTER_RAMP_END {
            let t = (abs_p - Self::CENTER_DEADZONE) / (Self::CENTER_RAMP_END - Self::CENTER_DEADZONE);
            // Smoothstep for silky engagement
            t * t * (3.0 - 2.0 * t)
        } else {
            1.0
        };

        let total_wet = center_wet * self.enable_mix;

        // If completely bypassed, gently decay internal filter state to zero and return exact dry
        if total_wet <= 1e-6 {
            self.stage1.ic1eq *= 0.99;
            self.stage1.ic2eq *= 0.99;
            self.stage2.ic1eq *= 0.99;
            self.stage2.ic2eq *= 0.99;
            return x;
        }

        let cutoff = self.cutoff_hz(p);
        let g = (PI * cutoff / self.sample_rate).tan();

        // Taper resonance down near 12 o'clock so there's no resonant bump at center
        let res_taper = ((abs_p - Self::CENTER_DEADZONE) / 0.14).clamp(0.0, 1.0);
        let res = resonance.clamp(0.0, 1.0) * res_taper;

        // Exponential Q mapping:
        // res = 0.0 -> Q = 0.7071 (Butterworth, flat, smooth)
        // res = 1.0 -> Stage 1 Q = 9.5, Stage 2 Q = 2.0 (classic Pioneer resonant club peak)
        let q1 = 0.70710678 * (13.4_f64).powf(res);
        let q2 = 0.70710678 * (2.8_f64).powf(res);
        let k1 = 1.0 / q1;
        let k2 = 1.0 / q2;

        let (lp1, hp1) = self.stage1.process(x, g, k1);
        let mid_sig = if p < 0.0 { lp1 } else { hp1 };

        // Musical soft saturation between stages when resonance is pushed high
        // Keeps screaming sweeps warm and prevents runaway levels
        let sat_drive = 1.0 + res * 0.45;
        let mid_sat = if res > 0.05 {
            (mid_sig * sat_drive).tanh() / sat_drive
        } else {
            mid_sig
        };

        let (lp2, hp2) = self.stage2.process(mid_sat, g, k2);
        let wet = if p < 0.0 { lp2 } else { hp2 };

        x * (1.0 - total_wet) + wet * total_wet
    }
}
