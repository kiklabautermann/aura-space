//! Vintage Color Engine (1970s, 1980s, NOW) — Robust & Denormal-safe.

use crate::dsp::filters::BiquadFilter;
use nih_plug::prelude::Enum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum ColorMode {
    #[name = "1970s"]
    Vintage1970s,
    #[name = "1980s"]
    Vintage1980s,
    #[name = "NOW"]
    Now,
}

impl Default for ColorMode {
    fn default() -> Self {
        Self::Vintage1970s
    }
}

/// Ultra-fast Xorshift32 PRNG for realtime audio TPDF dither generation (zero-allocation).
#[derive(Clone)]
struct FastPrng {
    state: u32,
}

impl FastPrng {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x12345678 } else { seed },
        }
    }

    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        if x == 0 {
            x = 0x12345678;
        }
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform float in [-1.0, 1.0]
    #[inline(always)]
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32 / 2147483648.0) - 1.0
    }

    /// Triangular PDF dither in [-1.0, 1.0]
    #[inline(always)]
    fn next_tpdf(&mut self) -> f32 {
        let r1 = self.next_f32();
        let r2 = self.next_f32();
        0.5 * (r1 + r2)
    }
}

/// Color Processor for a single audio channel.
#[derive(Clone)]
pub struct ChannelColorProcessor {
    lp1: BiquadFilter,
    lp2: BiquadFilter,
    prng: FastPrng,
    current_mode: ColorMode,
    sample_rate: f32,
}

impl ChannelColorProcessor {
    pub fn new(seed: u32) -> Self {
        Self {
            lp1: BiquadFilter::new(),
            lp2: BiquadFilter::new(),
            prng: FastPrng::new(seed),
            current_mode: ColorMode::Vintage1970s,
            sample_rate: 44100.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let safe_sr = if sample_rate.is_finite() && sample_rate > 1000.0 { sample_rate } else { 44100.0 };
        self.sample_rate = safe_sr;
        self.update_filters();
    }

    pub fn set_mode(&mut self, mode: ColorMode) {
        if self.current_mode != mode {
            self.current_mode = mode;
            self.update_filters();
        }
    }

    pub fn reset(&mut self) {
        self.lp1.reset();
        self.lp2.reset();
    }

    fn update_filters(&mut self) {
        let sr = self.sample_rate;
        match self.current_mode {
            ColorMode::Vintage1970s => {
                let cutoff = 10500.0f32.min(sr * 0.45);
                self.lp1.set_lowpass(cutoff, 0.5412, sr);
                self.lp2.set_lowpass(cutoff, 1.3065, sr);
            }
            ColorMode::Vintage1980s => {
                let cutoff = 16000.0f32.min(sr * 0.45);
                self.lp1.set_lowpass(cutoff, 0.5412, sr);
                self.lp2.set_lowpass(cutoff, 1.3065, sr);
            }
            ColorMode::Now => {
                let cutoff = (sr * 0.49).min(22000.0);
                self.lp1.set_lowpass(cutoff, 0.7071, sr);
                self.lp2.set_lowpass(cutoff, 0.7071, sr);
            }
        }
    }

    /// Process sample through vintage bandwidth filtering, bit quantization and dither.
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let safe_in = if input.is_finite() { input } else { 0.0 };
        match self.current_mode {
            ColorMode::Vintage1970s => {
                let filtered = self.lp2.process(self.lp1.process(safe_in));

                const LEVELS_12BIT: f32 = 2048.0;
                let dither = self.prng.next_tpdf() / LEVELS_12BIT;
                let scaled = filtered + dither;
                let quantized = (scaled * LEVELS_12BIT).round() / LEVELS_12BIT;

                Self::soft_clip_1970s(quantized)
            }
            ColorMode::Vintage1980s => {
                let filtered = self.lp2.process(self.lp1.process(safe_in));

                const LEVELS_16BIT: f32 = 32768.0;
                let dither = self.prng.next_tpdf() / LEVELS_16BIT;
                let scaled = filtered + dither;
                let quantized = (scaled * LEVELS_16BIT).round() / LEVELS_16BIT;

                Self::soft_clip_1980s(quantized)
            }
            ColorMode::Now => safe_in,
        }
    }

    /// Smooth rational tanh-like soft clipping for 1970s warmth (robust against NaN and inf).
    #[inline(always)]
    pub fn soft_clip_1970s(x: f32) -> f32 {
        let safe_x = if x.is_finite() { x } else { 0.0 };
        let drive = 1.15;
        let xd = (safe_x * drive).clamp(-3.0, 3.0);
        let x2 = xd * xd;
        (xd * (27.0 + x2) / (27.0 + 9.0 * x2)) * (1.0 / drive)
    }

    /// Gentle saturation for 1980s mode.
    #[inline(always)]
    pub fn soft_clip_1980s(x: f32) -> f32 {
        let safe_x = if x.is_finite() { x } else { 0.0 };
        let drive = 1.05;
        let xd = (safe_x * drive).clamp(-3.0, 3.0);
        let x2 = xd * xd;
        (xd * (27.0 + x2) / (27.0 + 9.0 * x2)) * (1.0 / drive)
    }
}
