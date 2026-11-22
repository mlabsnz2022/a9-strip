/// Subtle, transparent headroom soft-clipper modeled on modern club mixer output staging.
///
/// - Below `THRESHOLD` (-1.5 dBFS ≈ 0.841): strictly linear, zero distortion.
/// - Above `THRESHOLD`: C1-continuous `tanh` soft knee asymptotically approaching `CEILING` (~0.995).
#[derive(Debug, Clone, Copy)]
pub struct SoftClipper {
    threshold: f64,
    ceiling: f64,
}

impl Default for SoftClipper {
    fn default() -> Self {
        Self {
            // -1.5 dBFS
            threshold: 0.841395141645195,
            // -0.04 dBFS
            ceiling: 0.995,
        }
    }
}

impl SoftClipper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Processes a single sample and returns `(output, engaged_amount)` where
    /// `engaged_amount` is in `0.0..=1.0` indicating soft-clipper activity.
    #[inline]
    pub fn process(&self, x: f64, enabled: bool) -> (f64, f64) {
        if !enabled {
            return (x, 0.0);
        }

        let abs_x = x.abs();
        if abs_x <= self.threshold {
            return (x, 0.0);
        }

        let headroom = self.ceiling - self.threshold;
        let over = abs_x - self.threshold;
        // C1-continuous soft knee: derivative at over = 0 is 1.0
        let shaped = self.threshold + headroom * (over / headroom).tanh();
        let out = shaped.copysign(x);
        let activity = (over / (headroom * 2.0)).clamp(0.0, 1.0);
        (out, activity)
    }
}
