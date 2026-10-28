//! Jon Dattorro Figure-of-Eight Reverb Tank (MVerb Expansion).
//!
//! High-performance, ultra-lightweight implementation:
//! - Stereo SIMD parallel filter processing (both channels computed simultaneously).
//! - Block-level / Lazy coefficient updates.
//! - Parallel 2-channel parabolic LFO modulation.
//! - Precomputed tap indices and delay buffer lengths.
//! - Fully compatible with Stereo and Mono track configurations.

use nih_plug::prelude::Enum;

use super::color::{ChannelColorProcessor, ColorMode};
use super::delay_line::{AllpassFilter, DelayLine};
use super::filters::{StereoBiquadFilter, StereoOnePoleFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum ReverbMode {
    #[name = "Concert Hall"]
    ConcertHall,
    #[name = "Bright Hall"]
    BrightHall,
    #[name = "Plate"]
    Plate,
    #[name = "Room"]
    Room,
}

impl Default for ReverbMode {
    fn default() -> Self {
        Self::ConcertHall
    }
}

/// Parameters passed to the DSP engine per process block or sample.
#[derive(Clone, Copy, PartialEq)]
pub struct ReverbConfig {
    pub mix: f32,             // 0.0 .. 1.0 (0% .. 100%)
    pub predelay_ms: f32,     // 0.0 .. 500.0 ms
    pub decay_s: f32,         // 0.2 .. 70.0 s
    pub damping_high_freq: f32, // 1000 .. 20000 Hz
    pub damping_high_shelf: f32, // -24.0 .. 0.0 dB
    pub damping_bass_freq: f32,  // 10 .. 1000 Hz
    pub damping_bass_mult: f32,  // 0.25 .. 4.0 X
    pub size: f32,            // 0.0 .. 1.0 (0% .. 100%)
    pub attack: f32,          // 0.0 .. 1.0 (0% .. 100%)
    pub diff_early: f32,      // 0.0 .. 1.0
    pub diff_late: f32,       // 0.0 .. 1.0
    pub mod_rate: f32,        // 0.05 .. 10.0 Hz
    pub mod_depth: f32,       // 0.0 .. 1.0
    pub eq_high_cut: f32,     // 1000 .. 20000 Hz
    pub eq_low_cut: f32,      // 10 .. 1000 Hz
    pub reverb_mode: ReverbMode,
    pub color_mode: ColorMode,
}

/// Stereo 2-channel parabolic sine approximation executed in parallel registers.
#[inline(always)]
pub fn fast_sin_stereo(phase_l: f32, phase_r: f32) -> (f32, f32) {
    let xl = 2.0 * phase_l - 1.0;
    let xr = 2.0 * phase_r - 1.0;

    let yl = 4.0 * xl * (1.0 - xl.abs());
    let yr = 4.0 * xr * (1.0 - xr.abs());

    let out_l = 0.225 * (yl * yl.abs() - yl) + yl;
    let out_r = 0.225 * (yr * yr.abs() - yr) + yr;

    (out_l, out_r)
}

pub struct DattorroReverb {
    sample_rate: f32,

    // PreDelay (up to 500ms at 192kHz)
    predelay_l: DelayLine,
    predelay_r: DelayLine,

    // Input Diffusion: 4 allpass filters per channel
    input_ap1_l: AllpassFilter,
    input_ap2_l: AllpassFilter,
    input_ap3_l: AllpassFilter,
    input_ap4_l: AllpassFilter,

    input_ap1_r: AllpassFilter,
    input_ap2_r: AllpassFilter,
    input_ap3_r: AllpassFilter,
    input_ap4_r: AllpassFilter,

    // --- Left Tank Half ---
    tank_mod_delay_l: DelayLine,
    tank_ap1_l: AllpassFilter,
    tank_delay1_l: DelayLine,
    tank_ap2_l: AllpassFilter,
    tank_delay2_l: DelayLine,
    tank_color_l: ChannelColorProcessor,

    // --- Right Tank Half ---
    tank_mod_delay_r: DelayLine,
    tank_ap1_r: AllpassFilter,
    tank_delay1_r: DelayLine,
    tank_ap2_r: AllpassFilter,
    tank_delay2_r: DelayLine,
    tank_color_r: ChannelColorProcessor,

    // --- Stereo Parallel Damping Filters ---
    tank_high_damp: StereoOnePoleFilter,
    tank_high_shelf: StereoBiquadFilter,
    tank_bass_damp: StereoOnePoleFilter,

    // --- Stereo Parallel Output EQ ---
    eq_high_cut: StereoBiquadFilter,
    eq_low_cut: StereoBiquadFilter,

    // Modulation LFO phases
    lfo_phase_l: f32,
    lfo_phase_r: f32,

    // Cross-feedback storage
    last_tank_out_l: f32,
    last_tank_out_r: f32,

    // --- Cached Block Precomputed Parameters ---
    cached_cfg: Option<ReverbConfig>,
    predelay_samples: f32,
    attack: f32,
    decay_gain: f32,
    lfo_inc: f32,
    max_mod_samples: f32,
    mod_delay_base_l: f32,
    mod_delay_base_r: f32,
    mix: f32,
    bass_mult: f32,

    del1_len_l: usize,
    del2_len_l: usize,
    ap2_del_l: usize,

    del1_len_r: usize,
    del2_len_r: usize,
    ap2_del_r: usize,

    // Precomputed 14 tap sample offsets
    tap_del1_r_266: usize,
    tap_del1_r_2974: usize,
    tap_ap2_r_1913: usize,
    tap_del2_r_1996: usize,
    tap_del1_l_1990: usize,
    tap_ap2_l_187: usize,
    tap_del2_l_1066: usize,

    tap_del1_l_353: usize,
    tap_del1_l_3627: usize,
    tap_ap2_l_1228: usize,
    tap_del2_l_2673: usize,
    tap_del1_r_2111: usize,
    tap_ap2_r_335: usize,
    tap_del2_r_121: usize,
}

impl DattorroReverb {
    pub fn new() -> Self {
        let max_sr_scale = 192000.0 / 29761.0 * 1.6;

        Self {
            sample_rate: 44100.0,

            predelay_l: DelayLine::new(131072),
            predelay_r: DelayLine::new(131072),

            input_ap1_l: AllpassFilter::new((142.0 * max_sr_scale) as usize, 0.75),
            input_ap2_l: AllpassFilter::new((107.0 * max_sr_scale) as usize, 0.75),
            input_ap3_l: AllpassFilter::new((379.0 * max_sr_scale) as usize, 0.625),
            input_ap4_l: AllpassFilter::new((277.0 * max_sr_scale) as usize, 0.625),

            input_ap1_r: AllpassFilter::new((151.0 * max_sr_scale) as usize, 0.75),
            input_ap2_r: AllpassFilter::new((113.0 * max_sr_scale) as usize, 0.75),
            input_ap3_r: AllpassFilter::new((389.0 * max_sr_scale) as usize, 0.625),
            input_ap4_r: AllpassFilter::new((283.0 * max_sr_scale) as usize, 0.625),

            tank_mod_delay_l: DelayLine::new((1200.0 * max_sr_scale) as usize),
            tank_ap1_l: AllpassFilter::new((4500.0 * max_sr_scale) as usize, -0.7),
            tank_delay1_l: DelayLine::new((4000.0 * max_sr_scale) as usize),
            tank_ap2_l: AllpassFilter::new((2000.0 * max_sr_scale) as usize, 0.5),
            tank_delay2_l: DelayLine::new((3000.0 * max_sr_scale) as usize),
            tank_color_l: ChannelColorProcessor::new(12345),

            tank_mod_delay_r: DelayLine::new((1500.0 * max_sr_scale) as usize),
            tank_ap1_r: AllpassFilter::new((4300.0 * max_sr_scale) as usize, -0.7),
            tank_delay1_r: DelayLine::new((3500.0 * max_sr_scale) as usize),
            tank_ap2_r: AllpassFilter::new((3000.0 * max_sr_scale) as usize, 0.5),
            tank_delay2_r: DelayLine::new((4200.0 * max_sr_scale) as usize),
            tank_color_r: ChannelColorProcessor::new(67890),

            tank_high_damp: StereoOnePoleFilter::new(),
            tank_high_shelf: StereoBiquadFilter::new(),
            tank_bass_damp: StereoOnePoleFilter::new(),

            eq_high_cut: StereoBiquadFilter::new(),
            eq_low_cut: StereoBiquadFilter::new(),

            lfo_phase_l: 0.0,
            lfo_phase_r: 0.25,

            last_tank_out_l: 0.0,
            last_tank_out_r: 0.0,

            cached_cfg: None,
            predelay_samples: 0.0,
            attack: 0.5,
            decay_gain: 0.8,
            lfo_inc: 0.001,
            max_mod_samples: 20.0,
            mod_delay_base_l: 672.0,
            mod_delay_base_r: 908.0,
            mix: 1.0,
            bass_mult: 1.0,

            del1_len_l: 3720,
            del2_len_l: 2656,
            ap2_del_l: 1800,

            del1_len_r: 3163,
            del2_len_r: 3996,
            ap2_del_r: 2770,

            tap_del1_r_266: 266,
            tap_del1_r_2974: 2974,
            tap_ap2_r_1913: 1913,
            tap_del2_r_1996: 1996,
            tap_del1_l_1990: 1990,
            tap_ap2_l_187: 187,
            tap_del2_l_1066: 1066,

            tap_del1_l_353: 353,
            tap_del1_l_3627: 3627,
            tap_ap2_l_1228: 1228,
            tap_del2_l_2673: 2673,
            tap_del1_r_2111: 2111,
            tap_ap2_r_335: 335,
            tap_del2_r_121: 121,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sr = if sample_rate.is_finite() && sample_rate > 1000.0 && sample_rate < 1000000.0 {
            sample_rate
        } else {
            44100.0
        };
        self.sample_rate = sr;
        self.tank_color_l.set_sample_rate(sr);
        self.tank_color_r.set_sample_rate(sr);
        self.cached_cfg = None;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.predelay_l.reset();
        self.predelay_r.reset();

        self.input_ap1_l.reset();
        self.input_ap2_l.reset();
        self.input_ap3_l.reset();
        self.input_ap4_l.reset();

        self.input_ap1_r.reset();
        self.input_ap2_r.reset();
        self.input_ap3_r.reset();
        self.input_ap4_r.reset();

        self.tank_mod_delay_l.reset();
        self.tank_ap1_l.reset();
        self.tank_delay1_l.reset();
        self.tank_ap2_l.reset();
        self.tank_delay2_l.reset();
        self.tank_color_l.reset();

        self.tank_mod_delay_r.reset();
        self.tank_ap1_r.reset();
        self.tank_delay1_r.reset();
        self.tank_ap2_r.reset();
        self.tank_delay2_r.reset();
        self.tank_color_r.reset();

        self.tank_high_damp.reset();
        self.tank_high_shelf.reset();
        self.tank_bass_damp.reset();

        self.eq_high_cut.reset();
        self.eq_low_cut.reset();

        self.lfo_phase_l = 0.0;
        self.lfo_phase_r = 0.25;
        self.last_tank_out_l = 0.0;
        self.last_tank_out_r = 0.0;
        self.cached_cfg = None;
    }

    /// Prepare coefficients and parameters once per audio block.
    /// Skips all math if parameters have not changed.
    pub fn prepare_block(&mut self, cfg: &ReverbConfig) {
        if let Some(ref cached) = self.cached_cfg {
            if cached == cfg {
                return;
            }
        }
        self.cached_cfg = Some(*cfg);

        let sr = self.sample_rate;
        let sr_scale = sr / 29761.0;

        let mode_size_mult = match cfg.reverb_mode {
            ReverbMode::ConcertHall => 1.0,
            ReverbMode::BrightHall => 0.9,
            ReverbMode::Plate => 0.75,
            ReverbMode::Room => 0.55,
        };
        let size_scale = (0.35 + 1.0 * cfg.size.clamp(0.0, 1.0)) * mode_size_mult;
        let tank_scale = sr_scale * size_scale;

        // Color Mode
        self.tank_color_l.set_mode(cfg.color_mode);
        self.tank_color_r.set_mode(cfg.color_mode);

        // PreDelay
        let max_predelay = (self.predelay_l.capacity() - 4) as f32;
        self.predelay_samples = (cfg.predelay_ms.max(0.0) * 0.001 * sr).clamp(0.0, max_predelay);

        // Attack & Diffusion
        self.attack = cfg.attack.clamp(0.0, 1.0);
        let early_diff = cfg.diff_early.clamp(0.0, 1.0);

        self.input_ap1_l.gain = 0.75 * early_diff;
        self.input_ap2_l.gain = 0.75 * early_diff;
        self.input_ap3_l.gain = 0.625 * early_diff;
        self.input_ap4_l.gain = 0.625 * early_diff;
        self.input_ap1_r.gain = 0.75 * early_diff;
        self.input_ap2_r.gain = 0.75 * early_diff;
        self.input_ap3_r.gain = 0.625 * early_diff;
        self.input_ap4_r.gain = 0.625 * early_diff;

        self.input_ap1_l.set_delay((142.0 * sr_scale) as usize);
        self.input_ap2_l.set_delay((107.0 * sr_scale) as usize);
        self.input_ap3_l.set_delay((379.0 * sr_scale) as usize);
        self.input_ap4_l.set_delay((277.0 * sr_scale) as usize);
        self.input_ap1_r.set_delay((151.0 * sr_scale) as usize);
        self.input_ap2_r.set_delay((113.0 * sr_scale) as usize);
        self.input_ap3_r.set_delay((389.0 * sr_scale) as usize);
        self.input_ap4_r.set_delay((283.0 * sr_scale) as usize);

        // Decay Gain: g = exp(-6.907755 * T_loop / T_60)
        let loop_time = 0.15 * size_scale;
        self.decay_gain = (-6.907755 * loop_time / cfg.decay_s.max(0.1)).exp().clamp(0.0, 0.995);

        // LFO
        self.lfo_inc = cfg.mod_rate.max(0.01) / sr;
        self.max_mod_samples = 24.0 * sr_scale * cfg.mod_depth.clamp(0.0, 1.0);
        self.mod_delay_base_l = 672.0 * tank_scale;
        self.mod_delay_base_r = 908.0 * tank_scale;

        // Tank Allpass gains & delays
        let late_diff = cfg.diff_late.clamp(0.0, 1.0);
        self.tank_ap1_l.gain = -0.7 * late_diff;
        self.tank_ap2_l.gain = 0.5 * late_diff;
        self.tank_ap1_r.gain = -0.7 * late_diff;
        self.tank_ap2_r.gain = 0.5 * late_diff;

        let ap1_del_l = (4453.0 * tank_scale) as usize;
        self.del1_len_l = (3720.0 * tank_scale) as usize;
        self.ap2_del_l = (1800.0 * tank_scale) as usize;
        self.del2_len_l = (2656.0 * tank_scale) as usize;

        let ap1_del_r = (4217.0 * tank_scale) as usize;
        self.del1_len_r = (3163.0 * tank_scale) as usize;
        self.ap2_del_r = (2770.0 * tank_scale) as usize;
        self.del2_len_r = (3996.0 * tank_scale) as usize;

        self.tank_ap1_l.set_delay(ap1_del_l);
        self.tank_ap2_l.set_delay(self.ap2_del_l);
        self.tank_ap1_r.set_delay(ap1_del_r);
        self.tank_ap2_r.set_delay(self.ap2_del_r);

        // Damping filters (Parallel Stereo SIMD)
        self.tank_high_damp.set_cutoff(cfg.damping_high_freq, sr);
        self.tank_high_shelf.set_high_shelf(cfg.damping_high_freq, cfg.damping_high_shelf, sr);
        self.tank_bass_damp.set_cutoff(cfg.damping_bass_freq, sr);
        self.bass_mult = cfg.damping_bass_mult;

        // Precompute 14 Tap offsets (clamped to max_len - 1 to avoid reading the oldest/wrap sample)
        let tap = |val: f32, max_len: usize| ((val * tank_scale) as usize).min(max_len.saturating_sub(1));

        self.tap_del1_r_266 = tap(266.0, self.del1_len_r);
        self.tap_del1_r_2974 = tap(2974.0, self.del1_len_r);
        self.tap_ap2_r_1913 = tap(1913.0, self.ap2_del_r);
        self.tap_del2_r_1996 = tap(1996.0, self.del2_len_r);
        self.tap_del1_l_1990 = tap(1990.0, self.del1_len_l);
        self.tap_ap2_l_187 = tap(187.0, self.ap2_del_l);
        self.tap_del2_l_1066 = tap(1066.0, self.del2_len_l);

        self.tap_del1_l_353 = tap(353.0, self.del1_len_l);
        self.tap_del1_l_3627 = tap(3627.0, self.del1_len_l);
        self.tap_ap2_l_1228 = tap(1228.0, self.ap2_del_l);
        self.tap_del2_l_2673 = tap(2673.0, self.del2_len_l);
        self.tap_del1_r_2111 = tap(2111.0, self.del1_len_r);
        self.tap_ap2_r_335 = tap(335.0, self.ap2_del_r);
        self.tap_del2_r_121 = tap(121.0, self.del2_len_r);

        // Output HighCut & LowCut EQ (Parallel Stereo SIMD)
        self.eq_high_cut.set_lowpass(cfg.eq_high_cut, 0.7071, sr);
        self.eq_low_cut.set_highpass(cfg.eq_low_cut, 0.7071, sr);

        self.mix = cfg.mix.clamp(0.0, 1.0);
    }

    /// Backwards-compatible process_sample that ensures block parameters are precomputed.
    #[inline(always)]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32, cfg: &ReverbConfig) -> (f32, f32) {
        self.prepare_block(cfg);
        self.process_sample_fast(in_l, in_r)
    }

    /// Ultra-fast, zero-allocation sample loop with Stereo SIMD execution.
    #[inline(always)]
    pub fn process_sample_fast(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let in_l = if in_l.is_finite() { in_l.clamp(-10.0, 10.0) } else { 0.0 };
        let in_r = if in_r.is_finite() { in_r.clamp(-10.0, 10.0) } else { 0.0 };

        // 1. PreDelay
        self.predelay_l.push(in_l);
        self.predelay_r.push(in_r);
        let pd_l = self.predelay_l.read_linear(self.predelay_samples);
        let pd_r = self.predelay_r.read_linear(self.predelay_samples);

        // 2. Input Diffusion Cascade
        let diff_l = self.input_ap4_l.process(
            self.input_ap3_l.process(
                self.input_ap2_l.process(
                    self.input_ap1_l.process(pd_l)
                )
            )
        );
        let diff_r = self.input_ap4_r.process(
            self.input_ap3_r.process(
                self.input_ap2_r.process(
                    self.input_ap1_r.process(pd_r)
                )
            )
        );

        let input_to_tank_l = pd_l * (1.0 - self.attack) + diff_l * self.attack;
        let input_to_tank_r = pd_r * (1.0 - self.attack) + diff_r * self.attack;

        // 3. Stereo Parallel LFO Step
        self.lfo_phase_l = (self.lfo_phase_l + self.lfo_inc).fract();
        self.lfo_phase_r = (self.lfo_phase_r + self.lfo_inc).fract();

        let (lfo_val_l, lfo_val_r) = fast_sin_stereo(self.lfo_phase_l, self.lfo_phase_r);

        let mod_delay_l = self.mod_delay_base_l + lfo_val_l * self.max_mod_samples;
        let mod_delay_r = self.mod_delay_base_r + lfo_val_r * self.max_mod_samples;

        // 4. Modulated Delay Reads
        let tank_in_l = input_to_tank_l + self.last_tank_out_r * self.decay_gain;
        self.tank_mod_delay_l.push(tank_in_l);
        let mod_out_l = self.tank_mod_delay_l.read_hermite(mod_delay_l);

        let tank_in_r = input_to_tank_r + self.last_tank_out_l * self.decay_gain;
        self.tank_mod_delay_r.push(tank_in_r);
        let mod_out_r = self.tank_mod_delay_r.read_hermite(mod_delay_r);

        // 5. Parallel Stereo Damping (Lowpass + HighShelf + BassDamp)
        let damped_lp = self.tank_high_damp.process_lowpass([mod_out_l, mod_out_r]);
        let shelved = self.tank_high_shelf.process(damped_lp);

        let (bass_l, bass_r) = if self.bass_mult < 1.0 {
            // Bass Cut: crossfade between original and highpassed signal
            let hp = self.tank_bass_damp.process_highpass(shelved);
            let mult = self.bass_mult;
            let inv_mult = 1.0 - mult;
            (
                shelved[0] * mult + hp[0] * inv_mult,
                shelved[1] * mult + hp[1] * inv_mult,
            )
        } else if self.bass_mult > 1.0 {
            // Bass Boost: add lowpassed signal scaled by boost amount
            let lp = self.tank_bass_damp.process_lowpass(shelved);
            let boost = self.bass_mult - 1.0;
            (
                shelved[0] + lp[0] * boost,
                shelved[1] + lp[1] * boost,
            )
        } else {
            // bass_mult == 1.0: no change, still tick the filter to keep state consistent
            let _ = self.tank_bass_damp.process_lowpass(shelved);
            (shelved[0], shelved[1])
        };

        // 6. Tank Allpass and Fixed Delays
        let ap1_out_l = self.tank_ap1_l.process(bass_l);
        self.tank_delay1_l.push(ap1_out_l);
        let del1_out_l = self.tank_delay1_l.read(self.del1_len_l);

        let ap2_out_l = self.tank_ap2_l.process(del1_out_l);
        self.tank_delay2_l.push(ap2_out_l);
        let del2_out_l = self.tank_delay2_l.read(self.del2_len_l);
        let tank_colored_out_l = self.tank_color_l.process_sample(del2_out_l);
        self.last_tank_out_l = tank_colored_out_l;

        let ap1_out_r = self.tank_ap1_r.process(bass_r);
        self.tank_delay1_r.push(ap1_out_r);
        let del1_out_r = self.tank_delay1_r.read(self.del1_len_r);

        let ap2_out_r = self.tank_ap2_r.process(del1_out_r);
        self.tank_delay2_r.push(ap2_out_r);
        let del2_out_r = self.tank_delay2_r.read(self.del2_len_r);
        let tank_colored_out_r = self.tank_color_r.process_sample(del2_out_r);
        self.last_tank_out_r = tank_colored_out_r;

        // 7. Multi-Tap Readout with Precomputed Indices
        let wet_l = 0.6 * (
            self.tank_delay1_r.read(self.tap_del1_r_266)
            + self.tank_delay1_r.read(self.tap_del1_r_2974)
            - self.tank_ap2_r.read_tap(self.tap_ap2_r_1913)
            + self.tank_delay2_r.read(self.tap_del2_r_1996)
            - self.tank_delay1_l.read(self.tap_del1_l_1990)
            - self.tank_ap2_l.read_tap(self.tap_ap2_l_187)
            - self.tank_delay2_l.read(self.tap_del2_l_1066)
        );

        let wet_r = 0.6 * (
            self.tank_delay1_l.read(self.tap_del1_l_353)
            + self.tank_delay1_l.read(self.tap_del1_l_3627)
            - self.tank_ap2_l.read_tap(self.tap_ap2_l_1228)
            + self.tank_delay2_l.read(self.tap_del2_l_2673)
            - self.tank_delay1_r.read(self.tap_del1_r_2111)
            - self.tank_ap2_r.read_tap(self.tap_ap2_r_335)
            - self.tank_delay2_r.read(self.tap_del2_r_121)
        );

        // 8. Output HighCut & LowCut EQ (Stereo SIMD)
        let eq_out = self.eq_low_cut.process(self.eq_high_cut.process([wet_l, wet_r]));

        // 9. Dry / Wet Blend
        let out_l = in_l * (1.0 - self.mix) + eq_out[0] * self.mix;
        let out_r = in_r * (1.0 - self.mix) + eq_out[1] * self.mix;

        let safe_l = if out_l.is_finite() { out_l.clamp(-10.0, 10.0) } else { 0.0 };
        let safe_r = if out_r.is_finite() { out_r.clamp(-10.0, 10.0) } else { 0.0 };

        (safe_l, safe_r)
    }
}
