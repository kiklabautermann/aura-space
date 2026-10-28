//! One-pole, Biquad, and Stereo SIMD-parallel filters for Reverb Damping and EQ (Robust & Denormal-safe).

use std::f32::consts::PI;

/// Simple, ultra-fast 1-pole Low-Pass / High-Pass filter.
#[derive(Clone, Default)]
pub struct OnePoleFilter {
    state: f32,
    alpha: f32,
}

impl OnePoleFilter {
    pub fn new() -> Self {
        Self {
            state: 0.0,
            alpha: 0.5,
        }
    }

    pub fn reset(&mut self) {
        self.state = 0.0;
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32, sample_rate: f32) {
        let safe_sr = if sample_rate.is_finite() && sample_rate > 1000.0 { sample_rate } else { 44100.0 };
        let safe_cutoff = if cutoff_hz.is_finite() { cutoff_hz } else { 1000.0 };
        let norm_freq = (safe_cutoff / safe_sr).clamp(0.0001, 0.499);
        let costh = (2.0 * PI * norm_freq).cos();
        let diff = (2.0 - costh).powi(2) - 1.0;
        self.alpha = 2.0 - costh - diff.max(0.0).sqrt();
        self.alpha = self.alpha.clamp(0.0, 1.0);
    }

    #[inline(always)]
    pub fn process_lowpass(&mut self, input: f32) -> f32 {
        let safe_in = if input.is_finite() { input } else { 0.0 };
        self.state += self.alpha * (safe_in - self.state);
        if !self.state.is_finite() || self.state.abs() < 1e-15 {
            self.state = 0.0;
        }
        self.state
    }

    #[inline(always)]
    pub fn process_highpass(&mut self, input: f32) -> f32 {
        let safe_in = if input.is_finite() { input } else { 0.0 };
        let lp = self.process_lowpass(safe_in);
        safe_in - lp
    }
}

/// Stereo 2-channel 1-pole filter processed simultaneously in SIMD register format.
#[derive(Clone, Default)]
pub struct StereoOnePoleFilter {
    state: [f32; 2],
    alpha: f32,
}

impl StereoOnePoleFilter {
    pub fn new() -> Self {
        Self {
            state: [0.0; 2],
            alpha: 0.5,
        }
    }

    pub fn reset(&mut self) {
        self.state = [0.0; 2];
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32, sample_rate: f32) {
        let safe_sr = if sample_rate.is_finite() && sample_rate > 1000.0 { sample_rate } else { 44100.0 };
        let safe_cutoff = if cutoff_hz.is_finite() { cutoff_hz } else { 1000.0 };
        let norm_freq = (safe_cutoff / safe_sr).clamp(0.0001, 0.499);
        let costh = (2.0 * PI * norm_freq).cos();
        let diff = (2.0 - costh).powi(2) - 1.0;
        self.alpha = 2.0 - costh - diff.max(0.0).sqrt();
        self.alpha = self.alpha.clamp(0.0, 1.0);
    }

    #[inline(always)]
    pub fn process_lowpass(&mut self, input: [f32; 2]) -> [f32; 2] {
        let in0 = if input[0].is_finite() { input[0] } else { 0.0 };
        let in1 = if input[1].is_finite() { input[1] } else { 0.0 };

        self.state[0] += self.alpha * (in0 - self.state[0]);
        self.state[1] += self.alpha * (in1 - self.state[1]);

        if !self.state[0].is_finite() || self.state[0].abs() < 1e-15 { self.state[0] = 0.0; }
        if !self.state[1].is_finite() || self.state[1].abs() < 1e-15 { self.state[1] = 0.0; }

        self.state
    }

    #[inline(always)]
    pub fn process_highpass(&mut self, input: [f32; 2]) -> [f32; 2] {
        let in0 = if input[0].is_finite() { input[0] } else { 0.0 };
        let in1 = if input[1].is_finite() { input[1] } else { 0.0 };
        let lp = self.process_lowpass([in0, in1]);
        [in0 - lp[0], in1 - lp[1]]
    }
}

/// Biquad filter (Direct Form I) for single channel.
#[derive(Clone, Default)]
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadFilter {
    pub fn new() -> Self {
        Self {
            b0: 1.0,
            ..Default::default()
        }
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    pub fn set_lowpass(&mut self, cutoff_hz: f32, q: f32, sample_rate: f32) {
        let safe_sr = if sample_rate.is_finite() && sample_rate > 1000.0 { sample_rate } else { 44100.0 };
        let safe_cutoff = if cutoff_hz.is_finite() { cutoff_hz } else { 1000.0 };
        let safe_q = if q.is_finite() { q.clamp(0.1, 20.0) } else { 0.7071 };

        let w0 = 2.0 * PI * (safe_cutoff / safe_sr).clamp(0.0001, 0.49);
        let alpha = w0.sin() / (2.0 * safe_q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_highpass(&mut self, cutoff_hz: f32, q: f32, sample_rate: f32) {
        let safe_sr = if sample_rate.is_finite() && sample_rate > 1000.0 { sample_rate } else { 44100.0 };
        let safe_cutoff = if cutoff_hz.is_finite() { cutoff_hz } else { 1000.0 };
        let safe_q = if q.is_finite() { q.clamp(0.1, 20.0) } else { 0.7071 };

        let w0 = 2.0 * PI * (safe_cutoff / safe_sr).clamp(0.0001, 0.49);
        let alpha = w0.sin() / (2.0 * safe_q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_high_shelf(&mut self, cutoff_hz: f32, gain_db: f32, sample_rate: f32) {
        let safe_sr = if sample_rate.is_finite() && sample_rate > 1000.0 { sample_rate } else { 44100.0 };
        let safe_cutoff = if cutoff_hz.is_finite() { cutoff_hz } else { 1000.0 };
        let safe_gain = if gain_db.is_finite() { gain_db.clamp(-48.0, 12.0) } else { 0.0 };

        let a = 10.0f32.powf(safe_gain / 40.0);
        let w0 = 2.0 * PI * (safe_cutoff / safe_sr).clamp(0.0001, 0.49);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * 0.7071);

        let a_sqrt = a.max(0.0).sqrt();
        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha;

        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    fn set_coeffs(&mut self, b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) {
        let safe_a0 = if a0.abs() > 1e-7 { a0 } else { 1.0 };
        let inv_a0 = 1.0 / safe_a0;
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }

    #[inline(always)]
    pub fn process(&mut self, input: f32) -> f32 {
        let safe_in = if input.is_finite() { input } else { 0.0 };
        let out = self.b0 * safe_in + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = safe_in;

        self.y2 = self.y1;
        self.y1 = if out.is_finite() && out.abs() > 1e-15 { out } else { 0.0 };
        self.y1
    }
}

/// Stereo 2-channel Biquad Filter processed simultaneously in SIMD register format.
#[derive(Clone, Default)]
pub struct StereoBiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: [f32; 2],
    x2: [f32; 2],
    y1: [f32; 2],
    y2: [f32; 2],
}

impl StereoBiquadFilter {
    pub fn new() -> Self {
        Self {
            b0: 1.0,
            ..Default::default()
        }
    }

    pub fn reset(&mut self) {
        self.x1 = [0.0; 2];
        self.x2 = [0.0; 2];
        self.y1 = [0.0; 2];
        self.y2 = [0.0; 2];
    }

    pub fn set_lowpass(&mut self, cutoff_hz: f32, q: f32, sample_rate: f32) {
        let safe_sr = if sample_rate.is_finite() && sample_rate > 1000.0 { sample_rate } else { 44100.0 };
        let safe_cutoff = if cutoff_hz.is_finite() { cutoff_hz } else { 1000.0 };
        let safe_q = if q.is_finite() { q.clamp(0.1, 20.0) } else { 0.7071 };

        let w0 = 2.0 * PI * (safe_cutoff / safe_sr).clamp(0.0001, 0.49);
        let alpha = w0.sin() / (2.0 * safe_q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_highpass(&mut self, cutoff_hz: f32, q: f32, sample_rate: f32) {
        let safe_sr = if sample_rate.is_finite() && sample_rate > 1000.0 { sample_rate } else { 44100.0 };
        let safe_cutoff = if cutoff_hz.is_finite() { cutoff_hz } else { 1000.0 };
        let safe_q = if q.is_finite() { q.clamp(0.1, 20.0) } else { 0.7071 };

        let w0 = 2.0 * PI * (safe_cutoff / safe_sr).clamp(0.0001, 0.49);
        let alpha = w0.sin() / (2.0 * safe_q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_high_shelf(&mut self, cutoff_hz: f32, gain_db: f32, sample_rate: f32) {
        let safe_sr = if sample_rate.is_finite() && sample_rate > 1000.0 { sample_rate } else { 44100.0 };
        let safe_cutoff = if cutoff_hz.is_finite() { cutoff_hz } else { 1000.0 };
        let safe_gain = if gain_db.is_finite() { gain_db.clamp(-48.0, 12.0) } else { 0.0 };

        let a = 10.0f32.powf(safe_gain / 40.0);
        let w0 = 2.0 * PI * (safe_cutoff / safe_sr).clamp(0.0001, 0.49);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * 0.7071);

        let a_sqrt = a.max(0.0).sqrt();
        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha;

        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    fn set_coeffs(&mut self, b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) {
        let safe_a0 = if a0.abs() > 1e-7 { a0 } else { 1.0 };
        let inv_a0 = 1.0 / safe_a0;
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }

    #[inline(always)]
    pub fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        let in0 = if input[0].is_finite() { input[0] } else { 0.0 };
        let in1 = if input[1].is_finite() { input[1] } else { 0.0 };

        let out0 = self.b0 * in0 + self.b1 * self.x1[0] + self.b2 * self.x2[0] - self.a1 * self.y1[0] - self.a2 * self.y2[0];
        let out1 = self.b0 * in1 + self.b1 * self.x1[1] + self.b2 * self.x2[1] - self.a1 * self.y1[1] - self.a2 * self.y2[1];

        self.x2 = self.x1;
        self.x1 = [in0, in1];

        let y1_0 = if out0.is_finite() && out0.abs() > 1e-15 { out0 } else { 0.0 };
        let y1_1 = if out1.is_finite() && out1.abs() > 1e-15 { out1 } else { 0.0 };

        self.y2 = self.y1;
        self.y1 = [y1_0, y1_1];

        self.y1
    }
}
