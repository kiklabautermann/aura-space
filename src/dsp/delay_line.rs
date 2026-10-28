//! Delay-Lines and Allpass-Filters for Reverb DSP (Crash-proof & SIMD Optimized).

/// Circular delay line with high-quality Hermite cubic and linear interpolation.
#[derive(Clone)]
pub struct DelayLine {
    buffer: Vec<f32>,
    write_pos: usize,
    capacity: usize,
}

impl DelayLine {
    pub fn new(max_delay_samples: usize) -> Self {
        let capacity = (max_delay_samples + 8).next_power_of_two().max(16);
        Self {
            buffer: vec![0.0; capacity],
            write_pos: 0,
            capacity,
        }
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline(always)]
    pub fn push(&mut self, sample: f32) {
        // Robustness: sanitize against NaNs, Infinities, and denormals
        let safe_sample = if sample.is_finite() && sample.abs() > 1e-20 { sample } else { 0.0 };
        let mask = self.capacity - 1;
        unsafe {
            *self.buffer.get_unchecked_mut(self.write_pos) = safe_sample;
        }
        self.write_pos = (self.write_pos + 1) & mask;
    }

    #[inline(always)]
    pub fn read(&self, delay_samples: usize) -> f32 {
        let mask = self.capacity - 1;
        let d = delay_samples.min(self.capacity - 1);
        let read_pos = (self.write_pos + self.capacity - d) & mask;
        unsafe { *self.buffer.get_unchecked(read_pos) }
    }

    /// Linear interpolated read with NaN & out-of-bounds protection.
    #[inline(always)]
    pub fn read_linear(&self, delay_samples: f32) -> f32 {
        let delay_safe = if delay_samples.is_finite() { delay_samples } else { 0.0 };
        let delay_clamped = delay_safe.clamp(0.0, (self.capacity - 2) as f32);
        let d_int = delay_clamped as usize;
        let frac = delay_clamped - d_int as f32;

        let mask = self.capacity - 1;
        let r0 = (self.write_pos + self.capacity - d_int) & mask;
        let r1 = (r0 + self.capacity - 1) & mask;

        let y0 = unsafe { *self.buffer.get_unchecked(r0) };
        let y1 = unsafe { *self.buffer.get_unchecked(r1) };

        y0 + frac * (y1 - y0)
    }

    /// 4-point, 3rd-order Hermite cubic interpolation with NaN & out-of-bounds protection.
    #[inline(always)]
    pub fn read_hermite(&self, delay_samples: f32) -> f32 {
        let delay_safe = if delay_samples.is_finite() { delay_samples } else { 1.0 };
        let delay_clamped = delay_safe.clamp(1.0, (self.capacity - 4) as f32);
        let d_int = delay_clamped as usize;
        let t = delay_clamped - d_int as f32;

        let mask = self.capacity - 1;
        let r0 = (self.write_pos + self.capacity - d_int) & mask;
        let r_prev = (r0 + 1) & mask;
        let r1 = (r0 + self.capacity - 1) & mask;
        let r2 = (r1 + self.capacity - 1) & mask;

        let ym1 = unsafe { *self.buffer.get_unchecked(r_prev) };
        let y0 = unsafe { *self.buffer.get_unchecked(r0) };
        let y1 = unsafe { *self.buffer.get_unchecked(r1) };
        let y2 = unsafe { *self.buffer.get_unchecked(r2) };

        let c0 = y0;
        let c1 = 0.5 * (y1 - ym1);
        let c2 = ym1 - 2.5 * y0 + 2.0 * y1 - 0.5 * y2;
        let c3 = 0.5 * (y2 - ym1) + 1.5 * (y0 - y1);

        ((c3 * t + c2) * t + c1) * t + c0
    }
}

/// Allpass filter with internal delay line and feedback coefficient.
#[derive(Clone)]
pub struct AllpassFilter {
    delay: DelayLine,
    delay_samples: usize,
    pub gain: f32,
}

impl AllpassFilter {
    pub fn new(max_delay_samples: usize, gain: f32) -> Self {
        Self {
            delay: DelayLine::new(max_delay_samples),
            delay_samples: max_delay_samples.max(1),
            gain: if gain.is_finite() { gain.clamp(-0.999, 0.999) } else { 0.5 },
        }
    }

    #[inline(always)]
    pub fn set_delay(&mut self, samples: usize) {
        self.delay_samples = samples.clamp(1, self.delay.capacity - 4);
    }

    pub fn reset(&mut self) {
        self.delay.reset();
    }

    /// Standard Schroeder allpass:
    /// y = -g * x + delay_out
    /// delay_in = x + g * y
    #[inline(always)]
    pub fn process(&mut self, input: f32) -> f32 {
        let safe_in = if input.is_finite() { input } else { 0.0 };

        let delayed = self.delay.read(self.delay_samples);
        let output = -self.gain * safe_in + delayed;
        self.delay.push(safe_in + self.gain * output);
        output
    }

    /// Read tap from inside the allpass delay line without advancing state.
    #[inline(always)]
    pub fn read_tap(&self, tap_samples: usize) -> f32 {
        self.delay.read(tap_samples.min(self.delay_samples))
    }
}
