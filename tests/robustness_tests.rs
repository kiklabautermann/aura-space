//! Forensic robustness, stress and fuzzing tests for AuraVerb.

use aura_space::dsp::color::ColorMode;
use aura_space::dsp::mverb::{DattorroReverb, ReverbConfig, ReverbMode};
use aura_space::preset::AuraSpacePreset;

#[test]
fn test_nan_and_infinity_resilience() {
    let mut verb = DattorroReverb::new();
    verb.set_sample_rate(44100.0);

    let cfg = ReverbConfig {
        mix: 0.5,
        predelay_ms: 10.0,
        decay_s: 4.0,
        damping_high_freq: 6000.0,
        damping_high_shelf: -12.0,
        damping_bass_freq: 700.0,
        damping_bass_mult: 1.5,
        size: 1.0,
        attack: 0.5,
        diff_early: 1.0,
        diff_late: 1.0,
        mod_rate: 2.5,
        mod_depth: 0.5,
        eq_high_cut: 8000.0,
        eq_low_cut: 20.0,
        reverb_mode: ReverbMode::ConcertHall,
        color_mode: ColorMode::Vintage1970s,
    };
    verb.prepare_block(&cfg);

    // 1. Bombard with NaNs and Infinities
    let evil_inputs = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e35,
        -1e35,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ];

    for &evil in &evil_inputs {
        let (out_l, out_r) = verb.process_sample(evil, evil, &cfg);
        assert!(!out_l.is_nan(), "Left channel produced NaN from evil input {}", evil);
        assert!(!out_r.is_nan(), "Right channel produced NaN from evil input {}", evil);
        assert!(!out_l.is_infinite(), "Left channel produced Inf from evil input {}", evil);
        assert!(!out_r.is_infinite(), "Right channel produced Inf from evil input {}", evil);
    }

    // 2. Verify that the reverb can immediately recover and play normal audio after NaN bombardment
    let (clean_l, clean_r) = verb.process_sample(0.5, 0.5, &cfg);
    assert!(clean_l.is_finite());
    assert!(clean_r.is_finite());
}

#[test]
fn test_extreme_sample_rates_safety() {
    let mut verb = DattorroReverb::new();

    let extreme_srs = [
        0.0,
        -48000.0,
        f32::NAN,
        f32::INFINITY,
        100.0,
        192000.0,
        384000.0,
    ];

    for &sr in &extreme_srs {
        verb.set_sample_rate(sr);
        let cfg = ReverbConfig {
            mix: 0.5,
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
            eq_high_cut: 8000.0,
            eq_low_cut: 20.0,
            reverb_mode: ReverbMode::ConcertHall,
            color_mode: ColorMode::Vintage1970s,
        };

        let (out_l, out_r) = verb.process_sample(0.5, 0.5, &cfg);
        assert!(out_l.is_finite(), "Sample rate {} caused non-finite out_l", sr);
        assert!(out_r.is_finite(), "Sample rate {} caused non-finite out_r", sr);
    }
}

#[test]
fn test_corrupt_preset_sanitization() {
    let mut corrupt_preset = AuraSpacePreset {
        name: "   ".to_string(), // Empty name
        mix: f32::NAN,
        predelay: -999.0,
        decay: 99999.0,
        damping_high_freq: -50.0,
        damping_high_shelf: 50.0,
        damping_bass_freq: f32::INFINITY,
        damping_bass_mult: -10.0,
        size: 5000.0,
        attack: -1.0,
        diff_early: 999.0,
        diff_late: f32::NAN,
        mod_rate: 0.0,
        mod_depth: -5.0,
        eq_high_cut: 50000.0,
        eq_low_cut: -100.0,
        reverb_mode: 99,
        color_mode: 99,
    };

    // Sanitize must clamp everything to legal ranges without panicking
    corrupt_preset.sanitize();

    assert_eq!(corrupt_preset.name, "Untitled");
    assert!(corrupt_preset.mix >= 0.0 && corrupt_preset.mix <= 100.0);
    assert!(corrupt_preset.predelay >= 0.0 && corrupt_preset.predelay <= 500.0);
    assert!(corrupt_preset.decay >= 0.2 && corrupt_preset.decay <= 70.0);
    assert!(corrupt_preset.damping_high_freq >= 1000.0 && corrupt_preset.damping_high_freq <= 20000.0);
    assert!(corrupt_preset.damping_high_shelf >= -24.0 && corrupt_preset.damping_high_shelf <= 0.0);
    assert!(corrupt_preset.damping_bass_freq >= 10.0 && corrupt_preset.damping_bass_freq <= 1000.0);
    assert!(corrupt_preset.damping_bass_mult >= 0.25 && corrupt_preset.damping_bass_mult <= 4.0);
    assert!(corrupt_preset.size >= 0.0 && corrupt_preset.size <= 100.0);
    assert!(corrupt_preset.attack >= 0.0 && corrupt_preset.attack <= 100.0);
    assert!(corrupt_preset.diff_early >= 0.0 && corrupt_preset.diff_early <= 100.0);
    assert!(corrupt_preset.diff_late >= 0.0 && corrupt_preset.diff_late <= 100.0);
    assert!(corrupt_preset.mod_rate >= 0.05 && corrupt_preset.mod_rate <= 10.0);
    assert!(corrupt_preset.mod_depth >= 0.0 && corrupt_preset.mod_depth <= 100.0);
    assert!(corrupt_preset.eq_high_cut >= 1000.0 && corrupt_preset.eq_high_cut <= 20000.0);
    assert!(corrupt_preset.eq_low_cut >= 10.0 && corrupt_preset.eq_low_cut <= 1000.0);
    assert!(corrupt_preset.reverb_mode <= 3);
    assert!(corrupt_preset.color_mode <= 2);
}

#[test]
fn test_rapid_parameter_modulation_stress() {
    let mut verb = DattorroReverb::new();
    verb.set_sample_rate(48000.0);

    // Randomize parameters every 10 samples for 10000 samples
    for i in 0..10_000 {
        let f = (i as f32 * 0.13).sin().abs();
        let cfg = ReverbConfig {
            mix: f,
            predelay_ms: f * 500.0,
            decay_s: 0.2 + f * 69.8,
            damping_high_freq: 1000.0 + f * 19000.0,
            damping_high_shelf: -24.0 + f * 24.0,
            damping_bass_freq: 10.0 + f * 990.0,
            damping_bass_mult: 0.25 + f * 3.75,
            size: f,
            attack: f,
            diff_early: f,
            diff_late: f,
            mod_rate: 0.05 + f * 9.95,
            mod_depth: f,
            eq_high_cut: 1000.0 + f * 19000.0,
            eq_low_cut: 10.0 + f * 990.0,
            reverb_mode: if f > 0.5 { ReverbMode::Plate } else { ReverbMode::ConcertHall },
            color_mode: if f > 0.6 { ColorMode::Vintage1970s } else { ColorMode::Now },
        };

        let (out_l, out_r) = verb.process_sample(0.2, -0.2, &cfg);
        assert!(out_l.is_finite(), "out_l went non-finite during modulation stress at sample {}", i);
        assert!(out_r.is_finite(), "out_r went non-finite during modulation stress at sample {}", i);
    }
}
