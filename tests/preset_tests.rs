//! Unit tests for AuraVerb Preset System (serialization, apply, file I/O).

use std::sync::Arc;
use aura_verb::params::AuraVerbParams;
use aura_verb::preset::AuraVerbPreset;

#[test]
fn test_percentage_parameter_formatting() {
    let params = AuraVerbParams::default();

    // Verify mix (default 100.0) displays "100.0 %", NOT "10000.0 %"
    let mix_str = params.mix.to_string();
    assert_eq!(mix_str, "100.0 %", "Mix formatting failed: expected '100.0 %', got '{}'", mix_str);

    // Verify attack (default 50.0) displays "50.0 %"
    let attack_str = params.attack.to_string();
    assert_eq!(attack_str, "50.0 %", "Attack formatting failed: expected '50.0 %', got '{}'", attack_str);

    // Verify mod depth (default 38.0) displays "38.0 %"
    let depth_str = params.mod_depth.to_string();
    assert_eq!(depth_str, "38.0 %", "Depth formatting failed: expected '38.0 %', got '{}'", depth_str);

    // Verify size (default 100.0) displays "100.0 %"
    let size_str = params.size.to_string();
    assert_eq!(size_str, "100.0 %", "Size formatting failed: expected '100.0 %', got '{}'", size_str);

    // Verify diff early & late display "100.0 %"
    assert_eq!(params.diff_early.to_string(), "100.0 %");
    assert_eq!(params.diff_late.to_string(), "100.0 %");
}

#[test]
fn test_ensure_factory_presets() {
    AuraVerbPreset::ensure_factory_presets();
    let dir = AuraVerbPreset::preset_dir();
    assert!(dir.exists(), "Preset directory should exist");

    let list = AuraVerbPreset::list_all_presets();
    assert!(list.len() >= 8, "Expected at least 8 presets in directory");
}

#[test]
fn test_preset_from_params_and_serialization() {
    let params = Arc::new(AuraVerbParams::default());
    let preset = AuraVerbPreset::from_params("My Test Preset", &params);

    assert_eq!(preset.name, "My Test Preset");
    assert_eq!(preset.mix, 100.0);
    assert_eq!(preset.predelay, 20.0);
    assert_eq!(preset.decay, 4.0);

    let json = serde_json::to_string_pretty(&preset).expect("Serialization failed");
    let loaded: AuraVerbPreset = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(preset, loaded);
}

#[test]
fn test_preset_apply_to_params() {
    let params = Arc::new(AuraVerbParams::default());
    let presets = AuraVerbPreset::factory_presets();
    let cathedral = presets
        .iter()
        .find(|p| p.name == "Cathedral Cavern")
        .expect("Cathedral Cavern preset not found");

    let mut applied_count = 0;
    cathedral.apply_to_params(&params, |_ptr, norm| {
        assert!(
            norm >= 0.0 && norm <= 1.0,
            "Normalized param value out of [0, 1]: {}",
            norm
        );
        applied_count += 1;
    });

    assert_eq!(applied_count, 17);
}

#[test]
fn test_preset_file_io() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("aura_verb_test_preset.json");

    let original = AuraVerbPreset {
        name: "File IO Test".to_string(),
        mix: 85.0,
        predelay: 15.0,
        decay: 3.5,
        damping_high_freq: 5500.0,
        damping_high_shelf: -12.0,
        damping_bass_freq: 450.0,
        damping_bass_mult: 1.1,
        size: 80.0,
        attack: 60.0,
        diff_early: 90.0,
        diff_late: 90.0,
        mod_rate: 1.5,
        mod_depth: 30.0,
        eq_high_cut: 9000.0,
        eq_low_cut: 25.0,
        reverb_mode: 1,
        color_mode: 0,
    };

    original.save_to_file(&file_path).expect("Failed to save preset to file");
    assert!(file_path.exists());

    let loaded = AuraVerbPreset::load_from_file(&file_path).expect("Failed to load preset from file");
    assert_eq!(original, loaded);

    let _ = std::fs::remove_file(file_path);
}
