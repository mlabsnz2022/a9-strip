pub mod dsp;
pub mod params;
pub mod ui;

use dsp::meter::MeterState;
use dsp::A9ChannelStripDsp;
use nih_plug::prelude::*;
use params::{fader_to_gain, trim_knob_to_gain, A9StripParams, EqMode};
use std::sync::Arc;

pub struct A9Strip {
    params: Arc<A9StripParams>,
    dsp: A9ChannelStripDsp,
    meter: Arc<MeterState>,
}

impl Default for A9Strip {
    fn default() -> Self {
        Self {
            params: Arc::new(A9StripParams::default()),
            dsp: A9ChannelStripDsp::new(48000.0),
            meter: Arc::new(MeterState::new()),
        }
    }
}

impl Plugin for A9Strip {
    const NAME: &'static str = "A9 Strip";
    const VENDOR: &'static str = "m-labs";
    const URL: &'static str = "https://github.com/mlabsnz2022/a9-strip";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone(), self.meter.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.dsp.set_sample_rate(buffer_config.sample_rate as f64);
        true
    }

    fn reset(&mut self) {
        self.dsp.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let iso_mode = self.params.eq_mode.value() == EqMode::Isolator;
        let filter_on = self.params.filter_on.value();
        let soft_clip_on = self.params.soft_clip.value();

        let mut block_peak = 0.0_f64;
        let mut block_clip = 0.0_f64;

        for mut channel_samples in buffer.iter_samples() {
            let trim_gain = trim_knob_to_gain(self.params.trim.smoothed.next());
            let eq_hi = self.params.eq_hi.smoothed.next() as f64;
            let eq_mid = self.params.eq_mid.smoothed.next() as f64;
            let eq_low = self.params.eq_low.smoothed.next() as f64;
            let filter_pos = self.params.filter.smoothed.next() as f64;
            let filter_res = self.params.filter_res.smoothed.next() as f64;
            let fader_gain = fader_to_gain(self.params.fader.smoothed.next());

            let mut iter = channel_samples.iter_mut();
            let left = iter.next();
            let right = iter.next();

            if let (Some(l_ref), Some(r_ref)) = (left, right) {
                let (out_l, out_r, peak, clip) = self.dsp.process_stereo(
                    *l_ref as f64,
                    *r_ref as f64,
                    trim_gain,
                    eq_low,
                    eq_mid,
                    eq_hi,
                    iso_mode,
                    filter_pos,
                    filter_res,
                    filter_on,
                    fader_gain,
                    soft_clip_on,
                );
                *l_ref = out_l as f32;
                *r_ref = out_r as f32;
                if peak > block_peak {
                    block_peak = peak;
                }
                if clip > block_clip {
                    block_clip = clip;
                }
            }
        }

        if self.params.editor_state.is_open() {
            self.meter.update(block_peak as f32, block_clip as f32);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for A9Strip {
    const CLAP_ID: &'static str = "com.a9strip.pioneer-a9-channel-strip";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Pioneer DJM-A9 Channel Strip (Trim, EQ/Isolator, Sound Color FX Filter + Resonance)");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Equalizer,
        ClapFeature::Filter,
        ClapFeature::Mixing,
    ];
}

impl Vst3Plugin for A9Strip {
    const VST3_CLASS_ID: [u8; 16] = *b"A9StripChannelFX";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Eq,
        Vst3SubCategory::Filter,
        Vst3SubCategory::Tools,
    ];
}

nih_export_clap!(A9Strip);
nih_export_vst3!(A9Strip);
