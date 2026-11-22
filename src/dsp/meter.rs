use atomic_float::AtomicF32;
use std::sync::atomic::Ordering;

/// Lock-free atomic stereo/mono peak meter + soft-clip activity indicator for the UI thread.
pub struct MeterState {
    pub peak_linear: AtomicF32,
    pub clip_activity: AtomicF32,
}

impl Default for MeterState {
    fn default() -> Self {
        Self {
            peak_linear: AtomicF32::new(0.0),
            clip_activity: AtomicF32::new(0.0),
        }
    }
}

impl MeterState {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn update(&self, peak: f32, clip: f32) {
        // Peak hold with smooth decay handled in DSP or UI; store max if higher, else decay
        let prev = self.peak_linear.load(Ordering::Relaxed);
        let next = if peak > prev { peak } else { prev * 0.92 + peak * 0.08 };
        self.peak_linear.store(next, Ordering::Relaxed);

        let prev_c = self.clip_activity.load(Ordering::Relaxed);
        let next_c = if clip > prev_c { clip } else { prev_c * 0.88 };
        self.clip_activity.store(next_c, Ordering::Relaxed);
    }

    #[inline]
    pub fn read_db(&self) -> f32 {
        let lin = self.peak_linear.load(Ordering::Relaxed);
        if lin <= 1e-6 {
            -96.0
        } else {
            20.0 * lin.log10()
        }
    }

    #[inline]
    pub fn read_clip(&self) -> f32 {
        self.clip_activity.load(Ordering::Relaxed)
    }
}
