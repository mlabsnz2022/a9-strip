pub mod color_filter;
pub mod eq_iso;
pub mod meter;
pub mod soft_clip;

use color_filter::ColorFilterChannel;
use eq_iso::EqIsoChannel;
use soft_clip::SoftClipper;

/// Complete stereo Pioneer DJM-A9 Channel Strip DSP processor.
/// Signal Flow: Input -> Trim -> 3-Band EQ/ISO -> Sound Color FX Filter -> Soft Clipper -> Output
pub struct A9ChannelStripDsp {
    eq_iso: [EqIsoChannel; 2],
    filter: [ColorFilterChannel; 2],
    clipper: SoftClipper,
}

impl A9ChannelStripDsp {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            eq_iso: [EqIsoChannel::new(sample_rate), EqIsoChannel::new(sample_rate)],
            filter: [
                ColorFilterChannel::new(sample_rate),
                ColorFilterChannel::new(sample_rate),
            ],
            clipper: SoftClipper::new(),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        for ch in &mut self.eq_iso {
            ch.set_sample_rate(sample_rate);
        }
        for ch in &mut self.filter {
            ch.set_sample_rate(sample_rate);
        }
    }

    pub fn reset(&mut self) {
        for ch in &mut self.eq_iso {
            ch.reset();
        }
        for ch in &mut self.filter {
            ch.reset();
        }
    }

    /// Processes a stereo frame `(left, right)` in `f64` precision.
    /// Returns `(out_l, out_r, meter_peak, clip_activity)`.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn process_stereo(
        &mut self,
        left: f64,
        right: f64,
        trim_gain: f64,
        low_pos: f64,
        mid_pos: f64,
        high_pos: f64,
        iso_mode: bool,
        filter_pos: f64,
        filter_res: f64,
        filter_enabled: bool,
        fader_gain: f64,
        soft_clip_enabled: bool,
    ) -> (f64, f64, f64, f64) {
        // 1. Trim (Overall Gain)
        let l1 = left * trim_gain;
        let r1 = right * trim_gain;

        // 2. 3-Band EQ / Isolator
        let l2 = self.eq_iso[0].process(l1, low_pos, mid_pos, high_pos, iso_mode);
        let r2 = self.eq_iso[1].process(r1, low_pos, mid_pos, high_pos, iso_mode);

        // 3. Sound Color FX: Filter
        let l3 = self.filter[0].process(l2, filter_pos, filter_res, filter_enabled);
        let r3 = self.filter[1].process(r2, filter_pos, filter_res, filter_enabled);

        // 4. Channel Fader
        let l4 = l3 * fader_gain;
        let r4 = r3 * fader_gain;

        // 5. Subtle Output Soft Clipper
        let (l_out, clip_l) = self.clipper.process(l4, soft_clip_enabled);
        let (r_out, clip_r) = self.clipper.process(r4, soft_clip_enabled);

        let peak = l_out.abs().max(r_out.abs());
        let clip = clip_l.max(clip_r);

        (l_out, r_out, peak, clip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn measure_sine_gain_db(
        dsp_fn: &mut impl FnMut(f64) -> f64,
        sample_rate: f64,
        freq: f64,
    ) -> f64 {
        let settle_samples = (sample_rate * 0.25) as usize;
        let measure_samples = (sample_rate * 0.25) as usize;
        let mut phase: f64 = 0.0;
        let inc = 2.0 * PI * freq / sample_rate;

        for _ in 0..settle_samples {
            dsp_fn(phase.sin());
            phase += inc;
        }

        let mut in_sq = 0.0;
        let mut out_sq = 0.0;
        for _ in 0..measure_samples {
            let x = phase.sin();
            let y = dsp_fn(x);
            in_sq += x * x;
            out_sq += y * y;
            phase += inc;
        }

        if out_sq <= 1e-30 {
            -300.0
        } else {
            10.0 * (out_sq / in_sq).log10()
        }
    }

    #[test]
    fn iso_mode_sums_mathematically_flat() {
        let sr = 48000.0;
        for &freq in &[40.0, 100.0, 280.0, 600.0, 1500.0, 3800.0, 8000.0, 15000.0] {
            let mut ch = EqIsoChannel::new(sr);
            let db = measure_sine_gain_db(&mut |x| ch.process(x, 0.0, 0.0, 0.0, true), sr, freq);
            assert!(
                db.abs() < 0.05,
                "ISO mode not flat at {freq} Hz: measured {db:.4} dB"
            );
        }
    }

    #[test]
    fn iso_mode_full_kill_is_silent() {
        let sr = 48000.0;
        let mut ch = EqIsoChannel::new(sr);
        let db = measure_sine_gain_db(&mut |x| ch.process(x, -1.0, -1.0, -1.0, true), sr, 1000.0);
        assert!(db < -120.0, "ISO full kill should be silent, got {db:.2} dB");
    }

    #[test]
    fn eq_mode_range_matches_specs() {
        let sr = 48000.0;

        // Low cut at 35 Hz (well inside 70 Hz shelf) should be near -26 dB
        let mut ch = EqIsoChannel::new(sr);
        // Settle mode mix to EQ
        for _ in 0..4000 {
            ch.process(0.0, -1.0, 0.0, 0.0, false);
        }
        let low_cut = measure_sine_gain_db(&mut |x| ch.process(x, -1.0, 0.0, 0.0, false), sr, 25.0);
        assert!(
            low_cut < -20.0 && low_cut > -27.5,
            "EQ low cut out of range: {low_cut:.2} dB"
        );

        // Mid boost at 1 kHz should be near +6 dB
        let mut ch2 = EqIsoChannel::new(sr);
        for _ in 0..4000 {
            ch2.process(0.0, 0.0, 1.0, 0.0, false);
        }
        let mid_boost =
            measure_sine_gain_db(&mut |x| ch2.process(x, 0.0, 1.0, 0.0, false), sr, 1000.0);
        assert!(
            (mid_boost - 6.0).abs() < 0.5,
            "EQ mid boost should be ~+6 dB, got {mid_boost:.2} dB"
        );
    }

    #[test]
    fn color_filter_center_is_transparent() {
        let sr = 48000.0;
        let mut f = ColorFilterChannel::new(sr);
        for i in 0..1000 {
            let x = (i as f64 * 0.1).sin() * 0.7;
            let y = f.process(x, 0.0, 1.0, true);
            assert!((y - x).abs() < 1e-12, "Center filter should be bit-transparent");
        }
    }

    #[test]
    fn color_filter_sweep_is_stable_at_max_resonance() {
        for &sr in &[44100.0, 48000.0, 96000.0] {
            let mut f = ColorFilterChannel::new(sr);
            let n = (sr * 0.5) as usize;
            for i in 0..n {
                let t = i as f64 / n as f64;
                // Rapidly sweep from -1.0 (LPF) across 0.0 to +1.0 (HPF)
                let pos = -1.0 + 2.0 * t;
                let x = (2.0 * PI * 440.0 * (i as f64 / sr)).sin() * 0.5;
                let y = f.process(x, pos, 1.0, true);
                assert!(y.is_finite(), "Filter produced NaN/Inf at sr={sr}, pos={pos}");
                assert!(
                    y.abs() < 20.0,
                    "Filter blew up at sr={sr}, pos={pos}: y={y}"
                );
            }
        }
    }

    #[test]
    fn soft_clipper_is_linear_below_threshold_and_bounds_peaks() {
        let clip = SoftClipper::new();
        let (y_lin, act_lin) = clip.process(0.5, true);
        assert!((y_lin - 0.5).abs() < 1e-12);
        assert_eq!(act_lin, 0.0);

        let (y_hot, act_hot) = clip.process(3.0, true);
        assert!(y_hot > 0.84 && y_hot < 1.0, "Hot signal should be softly bounded: {y_hot}");
        assert!(act_hot > 0.5);
    }

    #[test]
    fn color_filter_lpf_hpf_and_resonance_behave_as_expected() {
        let sr = 48000.0;

        // LPF at pos = -0.5 (~700 Hz cutoff) should heavily attenuate 8 kHz while passing 100 Hz
        let mut lpf_low = ColorFilterChannel::new(sr);
        let pass_db = measure_sine_gain_db(&mut |x| lpf_low.process(x, -0.5, 0.0, true), sr, 100.0);
        let mut lpf_high = ColorFilterChannel::new(sr);
        let stop_db = measure_sine_gain_db(&mut |x| lpf_high.process(x, -0.5, 0.0, true), sr, 8000.0);
        assert!(pass_db > -1.0, "LPF should pass 100 Hz, got {pass_db:.2} dB");
        assert!(stop_db < -30.0, "LPF should strongly cut 8 kHz, got {stop_db:.2} dB");

        // HPF at pos = +0.5 (~600 Hz cutoff) should heavily attenuate 60 Hz while passing 5 kHz
        let mut hpf_low = ColorFilterChannel::new(sr);
        let hpf_stop = measure_sine_gain_db(&mut |x| hpf_low.process(x, 0.5, 0.0, true), sr, 60.0);
        let mut hpf_high = ColorFilterChannel::new(sr);
        let hpf_pass = measure_sine_gain_db(&mut |x| hpf_high.process(x, 0.5, 0.0, true), sr, 5000.0);
        assert!(hpf_stop < -30.0, "HPF should strongly cut 60 Hz, got {hpf_stop:.2} dB");
        assert!(hpf_pass > -1.0, "HPF should pass 5 kHz, got {hpf_pass:.2} dB");

        // Resonance (PARAMETER = 0.85) should create a clear boost at the cutoff frequency vs PARAMETER = 0.0
        let cutoff = lpf_low.cutoff_hz(-0.5);
        let mut flat_f = ColorFilterChannel::new(sr);
        let flat_at_fc = measure_sine_gain_db(&mut |x| flat_f.process(x, -0.5, 0.0, true), sr, cutoff);
        let mut res_f = ColorFilterChannel::new(sr);
        let res_at_fc = measure_sine_gain_db(&mut |x| res_f.process(x, -0.5, 0.85, true), sr, cutoff);
        assert!(
            res_at_fc > flat_at_fc + 6.0,
            "High PARAMETER resonance should boost cutoff: flat={flat_at_fc:.2} dB, res={res_at_fc:.2} dB"
        );
    }
}
