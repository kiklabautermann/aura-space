//! Automated DSP tests for AuraVerb / MVerb Dattorro engine.

use aura_verb::dsp::color::{ChannelColorProcessor, ColorMode};
use aura_verb::dsp::delay_line::{AllpassFilter, DelayLine};
use aura_verb::dsp::filters::{BiquadFilter, OnePoleFilter};
use aura_verb::dsp::mverb::{DattorroReverb, ReverbConfig, ReverbMode};

#[test]
fn test_delay_line_fifo_and_interpolation() {
    let mut dl = DelayLine::new(100);
    dl.push(1.0);
    dl.push(2.0);
    dl.push(3.0);
    dl.push(4.0);

    // Read integer delays
    assert_eq!(dl.read(1), 4.0);
    assert_eq!(dl.read(2), 3.0);
    assert_eq!(dl.read(3), 2.0);
    assert_eq!(dl.read(4), 1.0);

    // Read linear interpolation between delay 2 (3.0) and delay 3 (2.0)
    let lin = dl.read_linear(2.5);
    assert!((lin - 2.5).abs() < 1e-5);

    // Read Hermite interpolation
    let herm = dl.read_hermite(2.5);
    assert!((herm - 2.5).abs() < 0.1);

    // Reset
    dl.reset();
    assert_eq!(dl.read(1), 0.0);
}

#[test]
fn test_allpass_filter_stability() {
    let mut ap = AllpassFilter::new(50, 0.7);
    ap.set_delay(20);

    // Input impulse
    let out0 = ap.process(1.0);
    assert!(!out0.is_nan() && !out0.is_infinite());

    // Process silence and verify decay
    let mut max_abs = 0.0f32;
    for _ in 0..500 {
        let out = ap.process(0.0);
        assert!(!out.is_nan() && !out.is_infinite());
        max_abs = max_abs.max(out.abs());
    }
    // After 500 samples, allpass impulse response should have decayed
    let final_out = ap.process(0.0);
    assert!(final_out.abs() < 1e-4);
}

#[test]
fn test_one_pole_and_biquad_filters() {
    let mut op = OnePoleFilter::new();
    op.set_cutoff(1000.0, 44100.0);

    // DC gain of lowpass should be 1.0
    let mut val = 0.0;
    for _ in 0..1000 {
        val = op.process_lowpass(1.0);
    }
    assert!((val - 1.0).abs() < 0.01);

    // Biquad Lowpass
    let mut bq = BiquadFilter::new();
    bq.set_lowpass(1000.0, 0.7071, 44100.0);
    let mut bq_val = 0.0;
    for _ in 0..1000 {
        bq_val = bq.process(1.0);
    }
    assert!((bq_val - 1.0).abs() < 0.01);
}

#[test]
fn test_color_engine_modes() {
    let mut color = ChannelColorProcessor::new(42);
    color.set_sample_rate(44100.0);

    // NOW mode should be transparent
    color.set_mode(ColorMode::Now);
    let input = 0.42;
    assert_eq!(color.process_sample(input), input);

    // 1970s mode: should apply saturation and quantization
    color.set_mode(ColorMode::Vintage1970s);
    let out_70 = color.process_sample(input);
    assert!(!out_70.is_nan());
    assert!(!out_70.is_infinite());
    let out_loud = color.process_sample(2.0);
    assert!(out_loud < 1.5);

    // 1980s mode
    color.set_mode(ColorMode::Vintage1980s);
    let out_80 = color.process_sample(input);
    assert!(!out_80.is_nan());
}

#[test]
fn test_dattorro_reverb_impulse_response_and_decay() {
    let mut verb = DattorroReverb::new();
    verb.set_sample_rate(44100.0);

    let cfg = ReverbConfig {
        mix: 1.0, // 100% wet
        predelay_ms: 10.0,
        decay_s: 2.0,
        damping_high_freq: 6000.0,
        damping_high_shelf: -12.0,
        damping_bass_freq: 700.0,
        damping_bass_mult: 1.0,
        size: 1.0,
        attack: 0.5,
        diff_early: 1.0,
        diff_late: 1.0,
        mod_rate: 2.0,
        mod_depth: 0.5,
        eq_high_cut: 12000.0,
        eq_low_cut: 20.0,
        reverb_mode: ReverbMode::ConcertHall,
        color_mode: ColorMode::Vintage1970s,
    };

    let (first_l, first_r) = verb.process_sample(1.0, 0.0, &cfg);
    assert!(first_l.abs() < 0.1);
    assert!(first_r.abs() < 0.1);

    let mut energy_sum = 0.0f32;
    let mut max_output = 0.0f32;

    for _ in 0..88200 {
        let (out_l, out_r) = verb.process_sample_fast(0.0, 0.0);
        assert!(!out_l.is_nan(), "Left channel NaN detected");
        assert!(!out_r.is_nan(), "Right channel NaN detected");
        assert!(!out_l.is_infinite(), "Left channel Inf detected");
        assert!(!out_r.is_infinite(), "Right channel Inf detected");

        let mag = out_l.abs() + out_r.abs();
        energy_sum += mag;
        max_output = max_output.max(mag);
    }

    assert!(energy_sum > 1.0, "Reverb tail was silent: energy={}", energy_sum);
    assert!(max_output > 0.01, "Reverb peak was too low: peak={}", max_output);

    let (tail_l, tail_r) = verb.process_sample_fast(0.0, 0.0);
    assert!(tail_l.abs() < max_output * 0.1, "Tail did not decay: {}", tail_l);
    assert!(tail_r.abs() < max_output * 0.1, "Tail did not decay: {}", tail_r);
}

#[test]
fn test_dsp_performance_throughput() {
    let mut verb = DattorroReverb::new();
    verb.set_sample_rate(48000.0);

    let cfg = ReverbConfig {
        mix: 0.5,
        predelay_ms: 25.0,
        decay_s: 4.0,
        damping_high_freq: 6000.0,
        damping_high_shelf: -12.0,
        damping_bass_freq: 700.0,
        damping_bass_mult: 1.5,
        size: 1.0,
        attack: 0.5,
        diff_early: 1.0,
        diff_late: 1.0,
        mod_rate: 2.53,
        mod_depth: 0.38,
        eq_high_cut: 8000.0,
        eq_low_cut: 10.0,
        reverb_mode: ReverbMode::ConcertHall,
        color_mode: ColorMode::Vintage1970s,
    };

    verb.prepare_block(&cfg);

    let start = std::time::Instant::now();
    let num_frames = 480_000; // 10 seconds of audio at 48kHz

    let mut sum = 0.0f32;
    for i in 0..num_frames {
        let input = if i == 0 { 1.0 } else { 0.0 };
        let (l, r) = verb.process_sample_fast(input, input);
        sum += l + r;
    }

    let elapsed = start.elapsed();
    let audio_seconds = num_frames as f64 / 48000.0;
    let real_time_factor = audio_seconds / elapsed.as_secs_f64();

    let min_speedup = if cfg!(debug_assertions) { 4.0 } else { 20.0 };
    assert!(real_time_factor > min_speedup, "DSP was too slow!");
    assert!(sum.is_finite());
}
