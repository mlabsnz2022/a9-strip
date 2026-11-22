use std::f64::consts::PI;

/// 2nd-order biquad filter in Transposed Direct Form II (`f64` internal precision).
#[derive(Debug, Clone, Copy, Default)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    pub fn new() -> Self {
        Self {
            b0: 1.0,
            ..Default::default()
        }
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    pub fn set_low_shelf(&mut self, sample_rate: f64, freq: f64, slope: f64, gain_db: f64) {
        if gain_db.abs() < 1e-4 {
            self.set_unity();
            return;
        }
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let a = 10.0_f64.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = (sin_w0 / 2.0) * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).max(0.0).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;

        self.set_normalized(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_high_shelf(&mut self, sample_rate: f64, freq: f64, slope: f64, gain_db: f64) {
        if gain_db.abs() < 1e-4 {
            self.set_unity();
            return;
        }
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let a = 10.0_f64.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = (sin_w0 / 2.0) * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).max(0.0).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;

        self.set_normalized(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_peaking(&mut self, sample_rate: f64, freq: f64, q: f64, gain_db: f64) {
        if gain_db.abs() < 1e-4 {
            self.set_unity();
            return;
        }
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let a = 10.0_f64.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        self.set_normalized(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_butterworth_lp(&mut self, sample_rate: f64, freq: f64) {
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let w0 = 2.0 * PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * std::f64::consts::FRAC_1_SQRT_2);

        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.set_normalized(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_butterworth_hp(&mut self, sample_rate: f64, freq: f64) {
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let w0 = 2.0 * PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * std::f64::consts::FRAC_1_SQRT_2);

        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.set_normalized(b0, b1, b2, a0, a1, a2);
    }

    /// 2nd-order All-Pass filter with Q = 1/sqrt(2) matching the phase shift of an LR4 crossover.
    pub fn set_lr4_allpass(&mut self, sample_rate: f64, freq: f64) {
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let w0 = 2.0 * PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * std::f64::consts::FRAC_1_SQRT_2);

        let b0 = 1.0 - alpha;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 + alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.set_normalized(b0, b1, b2, a0, a1, a2);
    }

    fn set_unity(&mut self) {
        self.b0 = 1.0;
        self.b1 = 0.0;
        self.b2 = 0.0;
        self.a1 = 0.0;
        self.a2 = 0.0;
    }

    fn set_normalized(&mut self, b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) {
        let inv_a0 = 1.0 / a0;
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }
}

/// Maps a bipolar knob position (`-1.0..=1.0`, center `0.0`) to EQ mode dB (`-26.0..=+6.0`).
#[inline]
pub fn knob_to_eq_db(pos: f64) -> f64 {
    let p = pos.clamp(-1.0, 1.0);
    if p >= 0.0 {
        p * 6.0
    } else {
        // Gentle curve near 12 o'clock, reaching -26 dB at full CCW
        -26.0 * (-p).powf(1.35)
    }
}

/// Maps a bipolar knob position (`-1.0..=1.0`, center `0.0`) to Isolator linear gain (`0.0..=~1.995`).
#[inline]
pub fn knob_to_iso_gain(pos: f64) -> f64 {
    let p = pos.clamp(-1.0, 1.0);
    if p >= 0.0 {
        10.0_f64.powf((p * 6.0) / 20.0)
    } else if p <= -0.999 {
        0.0
    } else {
        // Progressive Pioneer A9 isolator taper: smooth near center, full kill (-inf) at -1.0
        (1.0 + p).powf(2.2)
    }
}

/// Single-channel Pioneer DJM-A9 switchable 3-band EQ / Isolator.
pub struct EqIsoChannel {
    sample_rate: f64,

    // EQ mode filters (70 Hz Low Shelf, 1 kHz Mid Peak, 13 kHz High Shelf)
    eq_low: Biquad,
    eq_mid: Biquad,
    eq_high: Biquad,

    // Last EQ gains to avoid redundant trig recalculation when knobs are stationary
    last_eq_low_db: f64,
    last_eq_mid_db: f64,
    last_eq_high_db: f64,

    // ISO mode: Phase-aligned 3-band Linkwitz-Riley 4th-order (24 dB/oct) crossover
    // Split 1 @ 280 Hz (Low vs Mid+High)
    iso_lp1_a: Biquad,
    iso_lp1_b: Biquad,
    iso_hp1_a: Biquad,
    iso_hp1_b: Biquad,

    // Split 2 @ 3800 Hz (Mid vs High)
    iso_lp2_a: Biquad,
    iso_lp2_b: Biquad,
    iso_hp2_a: Biquad,
    iso_hp2_b: Biquad,

    // All-pass phase alignment on Low band @ 3800 Hz so Low + Mid + High sums mathematically flat
    iso_low_ap: Biquad,

    // Smooth crossfade between EQ (0.0) and ISO (1.0)
    mode_mix: f64,
    mode_coeff: f64,
}

impl EqIsoChannel {
    pub const EQ_LOW_FREQ: f64 = 70.0;
    pub const EQ_MID_FREQ: f64 = 1000.0;
    pub const EQ_HIGH_FREQ: f64 = 13000.0;

    pub const ISO_LOW_XOVER: f64 = 280.0;
    pub const ISO_HIGH_XOVER: f64 = 3800.0;

    pub fn new(sample_rate: f64) -> Self {
        let mut ch = Self {
            sample_rate: sample_rate.max(8000.0),
            eq_low: Biquad::new(),
            eq_mid: Biquad::new(),
            eq_high: Biquad::new(),
            last_eq_low_db: f64::NAN,
            last_eq_mid_db: f64::NAN,
            last_eq_high_db: f64::NAN,
            iso_lp1_a: Biquad::new(),
            iso_lp1_b: Biquad::new(),
            iso_hp1_a: Biquad::new(),
            iso_hp1_b: Biquad::new(),
            iso_lp2_a: Biquad::new(),
            iso_lp2_b: Biquad::new(),
            iso_hp2_a: Biquad::new(),
            iso_hp2_b: Biquad::new(),
            iso_low_ap: Biquad::new(),
            mode_mix: 1.0,
            mode_coeff: 0.01,
        };
        ch.set_sample_rate(sample_rate);
        ch
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(8000.0);
        self.last_eq_low_db = f64::NAN;
        self.last_eq_mid_db = f64::NAN;
        self.last_eq_high_db = f64::NAN;

        // Configure fixed LR4 crossover network
        self.iso_lp1_a.set_butterworth_lp(self.sample_rate, Self::ISO_LOW_XOVER);
        self.iso_lp1_b.set_butterworth_lp(self.sample_rate, Self::ISO_LOW_XOVER);
        self.iso_hp1_a.set_butterworth_hp(self.sample_rate, Self::ISO_LOW_XOVER);
        self.iso_hp1_b.set_butterworth_hp(self.sample_rate, Self::ISO_LOW_XOVER);

        self.iso_lp2_a.set_butterworth_lp(self.sample_rate, Self::ISO_HIGH_XOVER);
        self.iso_lp2_b.set_butterworth_lp(self.sample_rate, Self::ISO_HIGH_XOVER);
        self.iso_hp2_a.set_butterworth_hp(self.sample_rate, Self::ISO_HIGH_XOVER);
        self.iso_hp2_b.set_butterworth_hp(self.sample_rate, Self::ISO_HIGH_XOVER);

        self.iso_low_ap.set_lr4_allpass(self.sample_rate, Self::ISO_HIGH_XOVER);

        // ~8 ms mode crossfade time constant
        self.mode_coeff = 1.0 - (-1.0 / (0.008 * self.sample_rate)).exp();
        self.reset();
    }

    pub fn reset(&mut self) {
        self.eq_low.reset();
        self.eq_mid.reset();
        self.eq_high.reset();
        self.iso_lp1_a.reset();
        self.iso_lp1_b.reset();
        self.iso_hp1_a.reset();
        self.iso_hp1_b.reset();
        self.iso_lp2_a.reset();
        self.iso_lp2_b.reset();
        self.iso_hp2_a.reset();
        self.iso_hp2_b.reset();
        self.iso_low_ap.reset();
    }

    #[inline]
    pub fn process(
        &mut self,
        x: f64,
        low_pos: f64,
        mid_pos: f64,
        high_pos: f64,
        iso_mode: bool,
    ) -> f64 {
        let target_mix = if iso_mode { 1.0 } else { 0.0 };
        self.mode_mix += self.mode_coeff * (target_mix - self.mode_mix);

        // Always update both topologies' states so switching modes mid-song is click-free
        let low_db = knob_to_eq_db(low_pos);
        let mid_db = knob_to_eq_db(mid_pos);
        let high_db = knob_to_eq_db(high_pos);

        if (low_db - self.last_eq_low_db).abs() > 1e-4 || self.last_eq_low_db.is_nan() {
            self.eq_low
                .set_low_shelf(self.sample_rate, Self::EQ_LOW_FREQ, 0.8, low_db);
            self.last_eq_low_db = low_db;
        }
        if (mid_db - self.last_eq_mid_db).abs() > 1e-4 || self.last_eq_mid_db.is_nan() {
            self.eq_mid
                .set_peaking(self.sample_rate, Self::EQ_MID_FREQ, 0.72, mid_db);
            self.last_eq_mid_db = mid_db;
        }
        if (high_db - self.last_eq_high_db).abs() > 1e-4 || self.last_eq_high_db.is_nan() {
            self.eq_high
                .set_high_shelf(self.sample_rate, Self::EQ_HIGH_FREQ, 0.7, high_db);
            self.last_eq_high_db = high_db;
        }

        let eq_out = self.eq_high.process(self.eq_mid.process(self.eq_low.process(x)));

        // LR4 Isolator split: Low + Mid + High
        let low_raw = self.iso_lp1_b.process(self.iso_lp1_a.process(x));
        let low_aligned = self.iso_low_ap.process(low_raw);

        let mid_high = self.iso_hp1_b.process(self.iso_hp1_a.process(x));
        let mid_band = self.iso_lp2_b.process(self.iso_lp2_a.process(mid_high));
        let high_band = self.iso_hp2_b.process(self.iso_hp2_a.process(mid_high));

        let g_low = knob_to_iso_gain(low_pos);
        let g_mid = knob_to_iso_gain(mid_pos);
        let g_high = knob_to_iso_gain(high_pos);

        let iso_out = low_aligned * g_low + mid_band * g_mid + high_band * g_high;

        eq_out * (1.0 - self.mode_mix) + iso_out * self.mode_mix
    }
}
