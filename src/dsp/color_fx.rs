use crate::dsp::color_filter::ColorFilterChannel;
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFxMode {
    Space,
    DubEcho,
    Sweep,
    Noise,
    Crush,
    Filter,
}

/// 2-pole Cytomic TPT Zero-Delay Feedback SVF with LP, HP, BP, and Notch outputs (`f64`).
#[derive(Debug, Clone, Copy, Default)]
struct MultiSvf {
    ic1eq: f64,
    ic2eq: f64,
}

impl MultiSvf {
    fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }

    /// Returns `(lp, bp, hp, notch)`.
    #[inline]
    fn process(&mut self, v0: f64, g: f64, k: f64) -> (f64, f64, f64, f64) {
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        let v3 = v0 - self.ic2eq;
        let v1 = a1 * self.ic1eq + a2 * v3;
        let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;

        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        let lp = v2;
        let bp = v1;
        let hp = v0 - k * v1 - v2;
        let notch = lp + hp;
        (lp, bp, hp, notch)
    }
}

/// Pre-allocated comb filter with 1-pole low-pass damping for SPACE reverb.
struct DampedComb {
    buf: Vec<f64>,
    idx: usize,
    filter_store: f64,
}

impl DampedComb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            idx: 0,
            filter_store: 0.0,
        }
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filter_store = 0.0;
    }

    #[inline]
    fn process(&mut self, input: f64, feedback: f64, damp: f64) -> f64 {
        let out = self.buf[self.idx];
        self.filter_store = out * (1.0 - damp) + self.filter_store * damp;
        self.buf[self.idx] = input + self.filter_store * feedback;
        self.idx += 1;
        if self.idx >= self.buf.len() {
            self.idx = 0;
        }
        out
    }
}

/// Pre-allocated Schroeder all-pass diffuser for SPACE reverb.
struct AllPassDiffuser {
    buf: Vec<f64>,
    idx: usize,
}

impl AllPassDiffuser {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            idx: 0,
        }
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
    }

    #[inline]
    fn process(&mut self, input: f64) -> f64 {
        let buf_out = self.buf[self.idx];
        let out = -input + buf_out;
        self.buf[self.idx] = input + buf_out * 0.5;
        self.idx += 1;
        if self.idx >= self.buf.len() {
            self.idx = 0;
        }
        out
    }
}

/// Stereo SPACE reverb tank (Freeverb/Schroeder-Moorer architecture in `f64`).
struct SpaceReverb {
    sample_rate: f64,
    combs_l: Vec<DampedComb>,
    combs_r: Vec<DampedComb>,
    ap_l: Vec<AllPassDiffuser>,
    ap_r: Vec<AllPassDiffuser>,
    send_filter_l: MultiSvf,
    send_filter_r: MultiSvf,
}

impl SpaceReverb {
    // Classic stereo Freeverb delay lengths at 44.1 kHz
    const COMB_LENS_44K: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
    const AP_LENS_44K: [usize; 4] = [556, 441, 341, 225];
    const STEREO_SPREAD_44K: usize = 23;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            sample_rate: sample_rate.max(8000.0),
            combs_l: Vec::new(),
            combs_r: Vec::new(),
            ap_l: Vec::new(),
            ap_r: Vec::new(),
            send_filter_l: MultiSvf::default(),
            send_filter_r: MultiSvf::default(),
        };
        s.set_sample_rate(sample_rate);
        s
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(8000.0);
        let scale = self.sample_rate / 44100.0;
        let spread = ((Self::STEREO_SPREAD_44K as f64) * scale).round() as usize;

        self.combs_l = Self::COMB_LENS_44K
            .iter()
            .map(|&l| DampedComb::new(((l as f64) * scale).round() as usize))
            .collect();
        self.combs_r = Self::COMB_LENS_44K
            .iter()
            .map(|&l| DampedComb::new(((l as f64) * scale).round() as usize + spread))
            .collect();
        self.ap_l = Self::AP_LENS_44K
            .iter()
            .map(|&l| AllPassDiffuser::new(((l as f64) * scale).round() as usize))
            .collect();
        self.ap_r = Self::AP_LENS_44K
            .iter()
            .map(|&l| AllPassDiffuser::new(((l as f64) * scale).round() as usize + spread))
            .collect();
        self.send_filter_l.reset();
        self.send_filter_r.reset();
    }

    fn reset(&mut self) {
        for c in &mut self.combs_l {
            c.reset();
        }
        for c in &mut self.combs_r {
            c.reset();
        }
        for a in &mut self.ap_l {
            a.reset();
        }
        for a in &mut self.ap_r {
            a.reset();
        }
        self.send_filter_l.reset();
        self.send_filter_r.reset();
    }

    #[inline]
    fn process(&mut self, l: f64, r: f64, color: f64, param: f64, wet_enable: f64) -> (f64, f64) {
        let abs_c = color.abs();
        let send_amt = if abs_c <= 0.005 {
            0.0
        } else {
            ((abs_c - 0.005) / 0.995).clamp(0.0, 1.0).powf(0.85) * wet_enable
        };

        // Left of center: Mid-Low reverb; Right of center: Mid-High reverb
        let cutoff: f64 = if color < 0.0 {
            1600.0
        } else {
            650.0
        };
        let g = (PI * cutoff.min(self.sample_rate * 0.45) / self.sample_rate).tan();
        let (lp_l, _, hp_l, _) = self.send_filter_l.process(l, g, 1.4142);
        let (lp_r, _, hp_r, _) = self.send_filter_r.process(r, g, 1.4142);

        let send_l = (if color < 0.0 { lp_l } else { hp_l }) * send_amt;
        let send_r = (if color < 0.0 { lp_r } else { hp_r }) * send_amt;
        let mono_send = (send_l + send_r) * 0.14;

        // PARAMETER adjusts Feedback (tail length)
        let fb = 0.62 + param.clamp(0.0, 1.0) * 0.34;
        let damp = if color < 0.0 { 0.45 } else { 0.18 };

        let mut acc_l = 0.0;
        let mut acc_r = 0.0;
        for c in &mut self.combs_l {
            acc_l += c.process(mono_send, fb, damp);
        }
        for c in &mut self.combs_r {
            acc_r += c.process(mono_send, fb, damp);
        }
        for a in &mut self.ap_l {
            acc_l = a.process(acc_l);
        }
        for a in &mut self.ap_r {
            acc_r = a.process(acc_r);
        }

        // Keep dry signal intact and blend lush stereo reverb wash
        let wet_gain = 0.55;
        let dry_duck = 1.0 - send_amt * 0.25;
        (l * dry_duck + acc_l * wet_gain, r * dry_duck + acc_r * wet_gain)
    }
}

/// Stereo Pioneer DUB ECHO (375 ms dub tape delay with mid/high send and saturated feedback).
struct DubEcho {
    sample_rate: f64,
    buf_l: Vec<f64>,
    buf_r: Vec<f64>,
    idx_l: usize,
    idx_r: usize,
    send_svf_l: MultiSvf,
    send_svf_r: MultiSvf,
    fb_svf_l: MultiSvf,
    fb_svf_r: MultiSvf,
}

impl DubEcho {
    fn new(sample_rate: f64) -> Self {
        let mut d = Self {
            sample_rate: sample_rate.max(8000.0),
            buf_l: Vec::new(),
            buf_r: Vec::new(),
            idx_l: 0,
            idx_r: 0,
            send_svf_l: MultiSvf::default(),
            send_svf_r: MultiSvf::default(),
            fb_svf_l: MultiSvf::default(),
            fb_svf_r: MultiSvf::default(),
        };
        d.set_sample_rate(sample_rate);
        d
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(8000.0);
        // Classic ~375 ms dub delay (160 BPM 1/4 or 120 BPM dotted-8th feel) with subtle stereo spread
        let len_l = ((0.375 * self.sample_rate).round() as usize).max(64);
        let len_r = ((0.383 * self.sample_rate).round() as usize).max(64);
        self.buf_l = vec![0.0; len_l];
        self.buf_r = vec![0.0; len_r];
        self.idx_l = 0;
        self.idx_r = 0;
        self.send_svf_l.reset();
        self.send_svf_r.reset();
        self.fb_svf_l.reset();
        self.fb_svf_r.reset();
    }

    fn reset(&mut self) {
        self.buf_l.fill(0.0);
        self.buf_r.fill(0.0);
        self.idx_l = 0;
        self.idx_r = 0;
        self.send_svf_l.reset();
        self.send_svf_r.reset();
        self.fb_svf_l.reset();
        self.fb_svf_r.reset();
    }

    #[inline]
    fn process(&mut self, l: f64, r: f64, color: f64, param: f64, wet_enable: f64) -> (f64, f64) {
        let abs_c = color.abs();
        let send_amt = if abs_c <= 0.005 {
            0.0
        } else {
            ((abs_c - 0.005) / 0.995).clamp(0.0, 1.0) * wet_enable
        };

        // Left: Mid-frequency echo (~900 Hz BP); Right: High-frequency echo (~2.2 kHz HP)
        let send_freq: f64 = if color < 0.0 { 950.0 } else { 2200.0 };
        let g_send = (PI * send_freq.min(self.sample_rate * 0.45) / self.sample_rate).tan();
        let (_, bp_l, hp_l, _) = self.send_svf_l.process(l, g_send, 0.9);
        let (_, bp_r, hp_r, _) = self.send_svf_r.process(r, g_send, 0.9);

        let in_l = (if color < 0.0 { bp_l } else { hp_l }) * send_amt;
        let in_r = (if color < 0.0 { bp_r } else { hp_r }) * send_amt;

        let delayed_l = self.buf_l[self.idx_l];
        let delayed_r = self.buf_r[self.idx_r];

        // Warm dub tape tone shaping inside feedback loop
        let fb_freq: f64 = if color < 0.0 { 1100.0 } else { 2600.0 };
        let g_fb = (PI * fb_freq.min(self.sample_rate * 0.45) / self.sample_rate).tan();
        let (_, fb_bp_l, fb_hp_l, _) = self.fb_svf_l.process(delayed_l, g_fb, 0.85);
        let (_, fb_bp_r, fb_hp_r, _) = self.fb_svf_r.process(delayed_r, g_fb, 0.85);

        let fb_sig_l = if color < 0.0 { fb_bp_l } else { fb_hp_l };
        let fb_sig_r = if color < 0.0 { fb_bp_r } else { fb_hp_r };

        // PARAMETER adjusts Feedback amount (0.25 .. 0.88)
        let fb = 0.25 + param.clamp(0.0, 1.0) * 0.63;

        // Soft-saturated ping-pong/stereo dub feedback
        self.buf_l[self.idx_l] = (in_l + fb_sig_l * fb * 0.85 + fb_sig_r * fb * 0.15).tanh();
        self.buf_r[self.idx_r] = (in_r + fb_sig_r * fb * 0.85 + fb_sig_l * fb * 0.15).tanh();

        self.idx_l = (self.idx_l + 1) % self.buf_l.len();
        self.idx_r = (self.idx_r + 1) % self.buf_r.len();

        (l + delayed_l * 0.9, r + delayed_r * 0.9)
    }
}

/// Stereo Pioneer SWEEP (CCW: widening Notch sweep, CW: narrowing Band-Pass sweep).
struct SweepFx {
    sample_rate: f64,
    svf_l1: MultiSvf,
    svf_l2: MultiSvf,
    svf_r1: MultiSvf,
    svf_r2: MultiSvf,
}

impl SweepFx {
    fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate: sample_rate.max(8000.0),
            svf_l1: MultiSvf::default(),
            svf_l2: MultiSvf::default(),
            svf_r1: MultiSvf::default(),
            svf_r2: MultiSvf::default(),
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(8000.0);
        self.reset();
    }

    fn reset(&mut self) {
        self.svf_l1.reset();
        self.svf_l2.reset();
        self.svf_r1.reset();
        self.svf_r2.reset();
    }

    #[inline]
    fn process(&mut self, l: f64, r: f64, color: f64, param: f64, wet_enable: f64) -> (f64, f64) {
        let abs_c = color.abs();
        let center_wet = if abs_c <= 0.005 {
            0.0
        } else if abs_c < 0.05 {
            let t = (abs_c - 0.005) / 0.045;
            t * t * (3.0 - 2.0 * t)
        } else {
            1.0
        };
        let wet = center_wet * wet_enable;
        if wet <= 1e-6 {
            return (l, r);
        }

        let nyq = (self.sample_rate * 0.45).min(18000.0);
        let p = param.clamp(0.0, 1.0);

        if color < 0.0 {
            // CCW: Notch filter whose bandwidth gradually widens & sweeps downward
            let base_freq = 450.0 + p * 2200.0;
            let freq = (base_freq * (0.28_f64).powf(abs_c)).clamp(90.0, nyq);
            let g = (PI * freq / self.sample_rate).tan();
            // Lower Q = wider notch as you turn further CCW
            let q = (3.5 - abs_c * 2.85).clamp(0.45, 4.0);
            let k = 1.0 / q;

            let (_, _, _, n_l1) = self.svf_l1.process(l, g, k);
            let (_, _, _, n_l2) = self.svf_l2.process(n_l1, g, k);
            let (_, _, _, n_r1) = self.svf_r1.process(r, g, k);
            let (_, _, _, n_r2) = self.svf_r2.process(n_r1, g, k);

            (l * (1.0 - wet) + n_l2 * wet, r * (1.0 - wet) + n_r2 * wet)
        } else {
            // CW: Band-pass filter whose bandwidth narrows & rises in frequency
            let base_freq = 320.0 + p * 1200.0;
            let freq = (base_freq * (7.5_f64).powf(abs_c)).clamp(150.0, nyq);
            let g = (PI * freq / self.sample_rate).tan();
            let q = 0.7 + abs_c * (2.5 + p * 4.5);
            let k = 1.0 / q;

            let (_, bp_l1, _, _) = self.svf_l1.process(l, g, k);
            let (_, bp_l2, _, _) = self.svf_l2.process(bp_l1, g, k * 1.4);
            let (_, bp_r1, _, _) = self.svf_r1.process(r, g, k);
            let (_, bp_r2, _, _) = self.svf_r2.process(bp_r1, g, k * 1.4);

            // Compensate bandpass gain gently
            let comp = (1.0 + q * 0.25).min(2.2);
            let wl = (bp_l2 * comp).tanh();
            let wr = (bp_r2 * comp).tanh();
            (l * (1.0 - wet) + wl * wet, r * (1.0 - wet) + wr * wet)
        }
    }
}

/// Stereo Pioneer NOISE (uncorrelated stereo white noise + sweeping resonant LPF/HPF).
struct NoiseFx {
    sample_rate: f64,
    rng_l: u64,
    rng_r: u64,
    svf_l1: MultiSvf,
    svf_l2: MultiSvf,
    svf_r1: MultiSvf,
    svf_r2: MultiSvf,
}

impl NoiseFx {
    fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate: sample_rate.max(8000.0),
            rng_l: 0x9E37_79B9_7F4A_7C15,
            rng_r: 0xBF58_476D_1CE4_E5B9,
            svf_l1: MultiSvf::default(),
            svf_l2: MultiSvf::default(),
            svf_r1: MultiSvf::default(),
            svf_r2: MultiSvf::default(),
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(8000.0);
        self.reset();
    }

    fn reset(&mut self) {
        self.svf_l1.reset();
        self.svf_l2.reset();
        self.svf_r1.reset();
        self.svf_r2.reset();
    }

    #[inline]
    fn next_white(state: &mut u64) -> f64 {
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *state = x;
        // Convert to [-1.0, 1.0]
        ((x as f64) / (u64::MAX as f64)) * 2.0 - 1.0
    }

    #[inline]
    fn process(&mut self, l: f64, r: f64, color: f64, param: f64, wet_enable: f64) -> (f64, f64) {
        let abs_c = color.abs();
        if abs_c <= 0.005 || wet_enable <= 1e-6 {
            return (l, r);
        }

        let level_ramp = ((abs_c - 0.005) / 0.18).clamp(0.0, 1.0);
        // PARAMETER adjusts Noise Volume (from subtle 0.05 to full club riser 0.65)
        let noise_vol = (0.06 + param.clamp(0.0, 1.0) * 0.58) * level_ramp * wet_enable;

        let nyq = (self.sample_rate * 0.45).min(18000.0);
        let cutoff = if color < 0.0 {
            // CCW: lowers LPF cutoff from 12 kHz down to 120 Hz
            (12000.0 * (120.0_f64 / 12000.0).powf(abs_c)).clamp(120.0, nyq)
        } else {
            // CW: raises HPF cutoff from 250 Hz up to 12 kHz
            (250.0 * (12000.0_f64 / 250.0).powf(abs_c)).clamp(250.0, nyq)
        };

        let g = (PI * cutoff / self.sample_rate).tan();
        let k = 1.0 / 2.2; // Gentle club resonance on the noise sweep

        let wl = Self::next_white(&mut self.rng_l);
        let wr = Self::next_white(&mut self.rng_r);

        let (lp_l1, _, hp_l1, _) = self.svf_l1.process(wl, g, k);
        let (lp_r1, _, hp_r1, _) = self.svf_r1.process(wr, g, k);
        let s1_l = if color < 0.0 { lp_l1 } else { hp_l1 };
        let s1_r = if color < 0.0 { lp_r1 } else { hp_r1 };

        let (lp_l2, _, hp_l2, _) = self.svf_l2.process(s1_l, g, 1.2);
        let (lp_r2, _, hp_r2, _) = self.svf_r2.process(s1_r, g, 1.2);
        let n_l = if color < 0.0 { lp_l2 } else { hp_l2 };
        let n_r = if color < 0.0 { lp_r2 } else { hp_r2 };

        (l + n_l * noise_vol, r + n_r * noise_vol)
    }
}

/// Stereo Pioneer CRUSH (CCW: Bit-crush & sample-rate decimation; CW: Crushed + High-Pass Filter).
struct CrushFx {
    sample_rate: f64,
    phase: f64,
    hold_l: f64,
    hold_r: f64,
    hp_l: MultiSvf,
    hp_r: MultiSvf,
}

impl CrushFx {
    fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate: sample_rate.max(8000.0),
            phase: 0.0,
            hold_l: 0.0,
            hold_r: 0.0,
            hp_l: MultiSvf::default(),
            hp_r: MultiSvf::default(),
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(8000.0);
        self.reset();
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.hold_l = 0.0;
        self.hold_r = 0.0;
        self.hp_l.reset();
        self.hp_r.reset();
    }

    #[inline]
    fn process(&mut self, l: f64, r: f64, color: f64, param: f64, wet_enable: f64) -> (f64, f64) {
        let abs_c = color.abs();
        let center_wet = if abs_c <= 0.005 {
            0.0
        } else if abs_c < 0.04 {
            let t = (abs_c - 0.005) / 0.035;
            t * t * (3.0 - 2.0 * t)
        } else {
            1.0
        };
        let wet = center_wet * wet_enable;
        if wet <= 1e-6 {
            return (l, r);
        }

        let p = param.clamp(0.0, 1.0);
        let intensity = (abs_c * (0.65 + p * 0.65)).clamp(0.0, 1.0);

        // Sample-rate decimation from full SR down to ~1.2 kHz
        let target_sr = self.sample_rate * (1200.0 / self.sample_rate).powf(intensity * 0.92);
        let step = (target_sr / self.sample_rate).clamp(0.01, 1.0);
        self.phase += step;
        if self.phase >= 1.0 {
            self.phase -= 1.0;

            // Bit-depth reduction + slight pre-drive grit
            let drive = 1.0 + p * 1.8 * abs_c;
            let bits = 14.0 - intensity * 10.5; // 14 bits down to 3.5 bits
            let levels = 2.0_f64.powf(bits);
            let ql = ((l * drive).tanh() * levels).round() / levels;
            let qr = ((r * drive).tanh() * levels).round() / levels;
            self.hold_l = ql;
            self.hold_r = qr;
        }

        let (mut wl, mut wr) = (self.hold_l, self.hold_r);

        // CW: pass crushed signal through a rising High-Pass Filter
        if color > 0.0 {
            let nyq = (self.sample_rate * 0.45).min(16000.0);
            let hp_freq = (120.0 * (8500.0_f64 / 120.0).powf(abs_c)).clamp(120.0, nyq);
            let g = (PI * hp_freq / self.sample_rate).tan();
            let (_, _, hp_l, _) = self.hp_l.process(wl, g, 1.1);
            let (_, _, hp_r, _) = self.hp_r.process(wr, g, 1.1);
            wl = hp_l;
            wr = hp_r;
        }

        (l * (1.0 - wet) + wl * wet, r * (1.0 - wet) + wr * wet)
    }
}

/// Complete Pioneer DJM-A9 Sound Color FX Stereo Processor (all 6 effects).
pub struct SoundColorFx {
    sample_rate: f64,
    space: SpaceReverb,
    dub_echo: DubEcho,
    sweep: SweepFx,
    noise: NoiseFx,
    crush: CrushFx,
    filter: [ColorFilterChannel; 2],
    enable_mix: f64,
    enable_coeff: f64,
    last_mode: ColorFxMode,
}

impl SoundColorFx {
    pub fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            sample_rate: sample_rate.max(8000.0),
            space: SpaceReverb::new(sample_rate),
            dub_echo: DubEcho::new(sample_rate),
            sweep: SweepFx::new(sample_rate),
            noise: NoiseFx::new(sample_rate),
            crush: CrushFx::new(sample_rate),
            filter: [
                ColorFilterChannel::new(sample_rate),
                ColorFilterChannel::new(sample_rate),
            ],
            enable_mix: 1.0,
            enable_coeff: 0.01,
            last_mode: ColorFxMode::Filter,
        };
        s.set_sample_rate(sample_rate);
        s
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(8000.0);
        self.space.set_sample_rate(self.sample_rate);
        self.dub_echo.set_sample_rate(self.sample_rate);
        self.sweep.set_sample_rate(self.sample_rate);
        self.noise.set_sample_rate(self.sample_rate);
        self.crush.set_sample_rate(self.sample_rate);
        for f in &mut self.filter {
            f.set_sample_rate(self.sample_rate);
        }
        self.enable_coeff = 1.0 - (-1.0 / (0.005 * self.sample_rate)).exp();
    }

    pub fn reset(&mut self) {
        self.space.reset();
        self.dub_echo.reset();
        self.sweep.reset();
        self.noise.reset();
        self.crush.reset();
        for f in &mut self.filter {
            f.reset();
        }
    }

    #[inline]
    pub fn process_stereo(
        &mut self,
        left: f64,
        right: f64,
        mode: ColorFxMode,
        color_pos: f64,
        param: f64,
        enabled: bool,
    ) -> (f64, f64) {
        if mode != self.last_mode {
            // Reset transient filter states when switching FX mode
            match mode {
                ColorFxMode::Sweep => self.sweep.reset(),
                ColorFxMode::Crush => self.crush.reset(),
                ColorFxMode::Noise => self.noise.reset(),
                ColorFxMode::Filter => {
                    self.filter[0].reset();
                    self.filter[1].reset();
                }
                _ => {}
            }
            self.last_mode = mode;
        }

        let target_en = if enabled { 1.0 } else { 0.0 };
        self.enable_mix += self.enable_coeff * (target_en - self.enable_mix);

        match mode {
            ColorFxMode::Space => {
                self.space
                    .process(left, right, color_pos, param, self.enable_mix)
            }
            ColorFxMode::DubEcho => {
                self.dub_echo
                    .process(left, right, color_pos, param, self.enable_mix)
            }
            ColorFxMode::Sweep => {
                self.sweep
                    .process(left, right, color_pos, param, self.enable_mix)
            }
            ColorFxMode::Noise => {
                self.noise
                    .process(left, right, color_pos, param, self.enable_mix)
            }
            ColorFxMode::Crush => {
                self.crush
                    .process(left, right, color_pos, param, self.enable_mix)
            }
            ColorFxMode::Filter => (
                self.filter[0].process(left, color_pos, param, enabled),
                self.filter[1].process(right, color_pos, param, enabled),
            ),
        }
    }
}
