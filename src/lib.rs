//! AuraVerb — CLAP Reverb Plugin with Vizia UI mimicking Valhalla VintageVerb.

pub mod dsp;
pub mod editor;
pub mod params;
pub mod preset;

use std::sync::Arc;
use nih_plug::prelude::*;
use dsp::{DattorroReverb, ReverbConfig};
use params::AuraVerbParams;

pub struct AuraVerb {
    params: Arc<AuraVerbParams>,
    reverb: DattorroReverb,
    sample_rate: f32,
    silent_frames_count: usize,
    tail_active: bool,
}

impl Default for AuraVerb {
    fn default() -> Self {
        Self {
            params: Arc::new(AuraVerbParams::default()),
            reverb: DattorroReverb::new(),
            sample_rate: 44100.0,
            silent_frames_count: 0,
            tail_active: false,
        }
    }
}

impl Plugin for AuraVerb {
    const NAME: &'static str = "AuraVintageVerb";
    const VENDOR: &'static str = "Aura Bridge";
    const URL: &'static str = "https://github.com/aura";
    const EMAIL: &'static str = "info@aurabridge.org";
    const VERSION: &'static str = "1.0.0";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        // 1. Stereo In -> Stereo Out
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        // 2. Mono In -> Stereo Out (Typical reverb send configuration)
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        // 3. Mono In -> Mono Out (Pure mono tracks)
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.reverb.set_sample_rate(buffer_config.sample_rate);
        self.silent_frames_count = 0;
        self.tail_active = false;
        true
    }

    fn reset(&mut self) {
        self.reverb.reset();
        self.silent_frames_count = 0;
        self.tail_active = false;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let p = &self.params;
        let num_samples = buffer.samples();
        let num_channels = buffer.channels();

        // 1. Silence Detection: Measure input peak
        let mut input_peak = 0.0f32;
        for channel in buffer.as_slice() {
            for &sample in channel.iter() {
                input_peak = input_peak.max(sample.abs());
            }
        }

        let is_input_silent = input_peak < 1e-5; // Threshold ~ -100 dB

        if !is_input_silent {
            self.silent_frames_count = 0;
            self.tail_active = true;
        } else {
            self.silent_frames_count += num_samples;
        }

        // 2. Sleep mode: If both input and tail are silent, bypass processing entirely (0.00% CPU)
        if !self.tail_active {
            for channel in buffer.as_slice() {
                channel.fill(0.0);
            }
            return ProcessStatus::Normal;
        }

        // 3. Active processing
        let cfg = ReverbConfig {
            mix: p.mix.value() * 0.01,
            predelay_ms: p.predelay.value(),
            decay_s: p.decay.value(),
            damping_high_freq: p.damping_high_freq.value(),
            damping_high_shelf: p.damping_high_shelf.value(),
            damping_bass_freq: p.damping_bass_freq.value(),
            damping_bass_mult: p.damping_bass_mult.value(),
            size: p.size.value() * 0.01,
            attack: p.attack.value() * 0.01,
            diff_early: p.diff_early.value() * 0.01,
            diff_late: p.diff_late.value() * 0.01,
            mod_rate: p.mod_rate.value(),
            mod_depth: p.mod_depth.value() * 0.01,
            eq_high_cut: p.eq_high_cut.value(),
            eq_low_cut: p.eq_low_cut.value(),
            reverb_mode: p.reverb_mode.value(),
            color_mode: p.color_mode.value(),
        };

        self.reverb.prepare_block(&cfg);

        let mut output_peak = 0.0f32;

        if num_channels == 1 {
            // Mono In -> Mono Out
            for mut channel_samples in buffer.iter_samples() {
                let in_mono = *channel_samples.get_mut(0).unwrap();
                let (out_l, out_r) = self.reverb.process_sample_fast(in_mono, in_mono);
                let out_mono = 0.5 * (out_l + out_r);

                output_peak = output_peak.max(out_mono.abs());
                *channel_samples.get_mut(0).unwrap() = out_mono;
            }
        } else {
            // Stereo Out (either Stereo In or Mono In)
            for mut channel_samples in buffer.iter_samples() {
                let in_l = *channel_samples.get_mut(0).unwrap();
                // If input has only 1 channel, duplicate mono to right channel
                let in_r = channel_samples.get_mut(1).map(|v| *v).unwrap_or(in_l);

                let (out_l, out_r) = self.reverb.process_sample_fast(in_l, in_r);

                output_peak = output_peak.max(out_l.abs().max(out_r.abs()));

                *channel_samples.get_mut(0).unwrap() = out_l;
                if let Some(r) = channel_samples.get_mut(1) {
                    *r = out_r;
                }
            }
        }

        // 4. Tail evaluation: if input is silent and reverb output has decayed below -100 dB
        //    Use a dynamic threshold proportional to decay time (min 0.5s, max 10s) to avoid
        //    cutting off long reverb tails prematurely.
        let silence_threshold_s = (p.decay.value() * 2.0).clamp(0.5, 10.0);
        let silence_threshold_samples = (self.sample_rate * silence_threshold_s) as usize;
        if is_input_silent && output_peak < 1e-5 && self.silent_frames_count > silence_threshold_samples {
            self.tail_active = false;
            self.reverb.reset();
            for channel in buffer.as_slice() {
                channel.fill(0.0);
            }
            return ProcessStatus::Normal;
        }

        // 5. Tail still active: report remaining tail samples to DAW host for CLAP sleep-management
        let max_tail_samples = (p.decay.value() * self.sample_rate).min(70.0 * 192000.0) as u32;
        ProcessStatus::Tail(max_tail_samples)
    }
}

impl ClapPlugin for AuraVerb {
    const CLAP_ID: &'static str = "com.aura.vintageverb";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Valhalla VintageVerb emulation based on Jon Dattorro MVerb algorithm");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Reverb,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

nih_export_clap!(AuraVerb);
