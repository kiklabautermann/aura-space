//! Preset management for AuraVerb (JSON save/load and factory presets).

use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use nih_plug::prelude::*;

use crate::dsp::{ColorMode, ReverbMode};
use crate::params::AuraVerbParams;

/// Serializable snapshot of all AuraVerb parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AuraVerbPreset {
    pub name: String,
    pub mix: f32,
    pub predelay: f32,
    pub decay: f32,
    pub damping_high_freq: f32,
    pub damping_high_shelf: f32,
    pub damping_bass_freq: f32,
    pub damping_bass_mult: f32,
    pub size: f32,
    pub attack: f32,
    pub diff_early: f32,
    pub diff_late: f32,
    pub mod_rate: f32,
    pub mod_depth: f32,
    pub eq_high_cut: f32,
    pub eq_low_cut: f32,
    pub reverb_mode: usize,
    pub color_mode: usize,
}

impl Default for AuraVerbPreset {
    fn default() -> Self {
        Self::factory_presets()[0].clone()
    }
}

impl AuraVerbPreset {
    /// Capture current parameter values into a preset.
    pub fn from_params(name: &str, p: &AuraVerbParams) -> Self {
        let rev_mode = match p.reverb_mode.value() {
            ReverbMode::ConcertHall => 0,
            ReverbMode::BrightHall => 1,
            ReverbMode::Plate => 2,
            ReverbMode::Room => 3,
        };
        let col_mode = match p.color_mode.value() {
            ColorMode::Vintage1970s => 0,
            ColorMode::Vintage1980s => 1,
            ColorMode::Now => 2,
        };

        Self {
            name: name.to_string(),
            mix: p.mix.value(),
            predelay: p.predelay.value(),
            decay: p.decay.value(),
            damping_high_freq: p.damping_high_freq.value(),
            damping_high_shelf: p.damping_high_shelf.value(),
            damping_bass_freq: p.damping_bass_freq.value(),
            damping_bass_mult: p.damping_bass_mult.value(),
            size: p.size.value(),
            attack: p.attack.value(),
            diff_early: p.diff_early.value(),
            diff_late: p.diff_late.value(),
            mod_rate: p.mod_rate.value(),
            mod_depth: p.mod_depth.value(),
            eq_high_cut: p.eq_high_cut.value(),
            eq_low_cut: p.eq_low_cut.value(),
            reverb_mode: rev_mode,
            color_mode: col_mode,
        }
    }

    /// Apply preset values to plugin parameters via an emit closure.
    pub fn apply_to_params(&self, p: &AuraVerbParams, mut emit_normalized: impl FnMut(ParamPtr, f32)) {
        let mut set = |param: &FloatParam, plain: f32| {
            let norm = param.preview_normalized(plain);
            emit_normalized(param.as_ptr(), norm);
        };

        set(&p.mix, self.mix);
        set(&p.predelay, self.predelay);
        set(&p.decay, self.decay);
        set(&p.damping_high_freq, self.damping_high_freq);
        set(&p.damping_high_shelf, self.damping_high_shelf);
        set(&p.damping_bass_freq, self.damping_bass_freq);
        set(&p.damping_bass_mult, self.damping_bass_mult);
        set(&p.size, self.size);
        set(&p.attack, self.attack);
        set(&p.diff_early, self.diff_early);
        set(&p.diff_late, self.diff_late);
        set(&p.mod_rate, self.mod_rate);
        set(&p.mod_depth, self.mod_depth);
        set(&p.eq_high_cut, self.eq_high_cut);
        set(&p.eq_low_cut, self.eq_low_cut);

        // Enum parameters: 0..N-1 mapped into normalized range
        let rev_norm = (self.reverb_mode as f32) / 3.0;
        emit_normalized(p.reverb_mode.as_ptr(), rev_norm.clamp(0.0, 1.0));

        let col_norm = (self.color_mode as f32) / 2.0;
        emit_normalized(p.color_mode.as_ptr(), col_norm.clamp(0.0, 1.0));
    }

    /// Save preset to JSON file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialization error: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("File write error: {}", e))?;
        Ok(())
    }

    /// Load preset from JSON file with safety sanitization against corrupt values.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("File read error: {}", e))?;
        let mut preset: Self = serde_json::from_str(&json)
            .map_err(|e| format!("Parse error: {}", e))?;
        preset.sanitize();
        Ok(preset)
    }

    /// Sanitize all parameter ranges to guarantee validity and prevent crashes or audio blowups.
    pub fn sanitize(&mut self) {
        if self.name.trim().is_empty() {
            self.name = "Untitled".to_string();
        }
        let check = |val: f32, min: f32, max: f32, default: f32| {
            if val.is_finite() {
                val.clamp(min, max)
            } else {
                default
            }
        };

        self.mix = check(self.mix, 0.0, 100.0, 100.0);
        self.predelay = check(self.predelay, 0.0, 500.0, 20.0);
        self.decay = check(self.decay, 0.2, 70.0, 4.0);
        self.damping_high_freq = check(self.damping_high_freq, 1000.0, 20000.0, 6000.0);
        self.damping_high_shelf = check(self.damping_high_shelf, -24.0, 0.0, -24.0);
        self.damping_bass_freq = check(self.damping_bass_freq, 10.0, 1000.0, 700.0);
        self.damping_bass_mult = check(self.damping_bass_mult, 0.25, 4.0, 1.5);
        self.size = check(self.size, 0.0, 100.0, 100.0);
        self.attack = check(self.attack, 0.0, 100.0, 50.0);
        self.diff_early = check(self.diff_early, 0.0, 100.0, 100.0);
        self.diff_late = check(self.diff_late, 0.0, 100.0, 100.0);
        self.mod_rate = check(self.mod_rate, 0.05, 10.0, 2.53);
        self.mod_depth = check(self.mod_depth, 0.0, 100.0, 38.0);
        self.eq_high_cut = check(self.eq_high_cut, 1000.0, 20000.0, 8000.0);
        self.eq_low_cut = check(self.eq_low_cut, 10.0, 1000.0, 10.0);
        self.reverb_mode = self.reverb_mode.min(3);
        self.color_mode = self.color_mode.min(2);
    }

    /// Default user preset directory: ~/.config/aura_verb/presets/
    pub fn preset_dir() -> PathBuf {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("aura_verb")
            .join("presets");
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
        }
        dir
    }

    /// Built-in Factory Presets (cached via LazyLock, allocated only once)
    pub fn factory_presets() -> &'static [AuraVerbPreset] {
        static FACTORY: LazyLock<Vec<AuraVerbPreset>> = LazyLock::new(|| {
            vec![
                AuraVerbPreset {
                    name: "Concert Hall".to_string(),
                    mix: 100.0,
                    predelay: 20.0,
                    decay: 4.0,
                    damping_high_freq: 6000.0,
                    damping_high_shelf: -24.0,
                    damping_bass_freq: 700.0,
                    damping_bass_mult: 1.5,
                    size: 100.0,
                    attack: 50.0,
                    diff_early: 100.0,
                    diff_late: 100.0,
                    mod_rate: 2.53,
                    mod_depth: 38.0,
                    eq_high_cut: 8000.0,
                    eq_low_cut: 10.0,
                    reverb_mode: 0, // Concert Hall
                    color_mode: 0,  // 1970s
                },
                AuraVerbPreset {
                    name: "Vintage Plate".to_string(),
                    mix: 100.0,
                    predelay: 0.0,
                    decay: 2.2,
                    damping_high_freq: 9500.0,
                    damping_high_shelf: -10.0,
                    damping_bass_freq: 400.0,
                    damping_bass_mult: 0.9,
                    size: 75.0,
                    attack: 85.0,
                    diff_early: 100.0,
                    diff_late: 100.0,
                    mod_rate: 1.8,
                    mod_depth: 25.0,
                    eq_high_cut: 12000.0,
                    eq_low_cut: 120.0,
                    reverb_mode: 2, // Plate
                    color_mode: 1,  // 1980s
                },
                AuraVerbPreset {
                    name: "Warm Room".to_string(),
                    mix: 100.0,
                    predelay: 8.0,
                    decay: 1.1,
                    damping_high_freq: 4500.0,
                    damping_high_shelf: -18.0,
                    damping_bass_freq: 500.0,
                    damping_bass_mult: 1.2,
                    size: 50.0,
                    attack: 30.0,
                    diff_early: 80.0,
                    diff_late: 60.0,
                    mod_rate: 1.2,
                    mod_depth: 15.0,
                    eq_high_cut: 7000.0,
                    eq_low_cut: 40.0,
                    reverb_mode: 3, // Room
                    color_mode: 0,  // 1970s
                },
                AuraVerbPreset {
                    name: "Cathedral Cavern".to_string(),
                    mix: 100.0,
                    predelay: 45.0,
                    decay: 12.0,
                    damping_high_freq: 5000.0,
                    damping_high_shelf: -15.0,
                    damping_bass_freq: 800.0,
                    damping_bass_mult: 1.8,
                    size: 100.0,
                    attack: 70.0,
                    diff_early: 100.0,
                    diff_late: 100.0,
                    mod_rate: 0.8,
                    mod_depth: 45.0,
                    eq_high_cut: 6500.0,
                    eq_low_cut: 15.0,
                    reverb_mode: 0, // Concert Hall
                    color_mode: 1,  // 1980s
                },
                AuraVerbPreset {
                    name: "1970s Dark Space".to_string(),
                    mix: 100.0,
                    predelay: 35.0,
                    decay: 6.5,
                    damping_high_freq: 3200.0,
                    damping_high_shelf: -24.0,
                    damping_bass_freq: 600.0,
                    damping_bass_mult: 2.0,
                    size: 95.0,
                    attack: 80.0,
                    diff_early: 100.0,
                    diff_late: 90.0,
                    mod_rate: 3.2,
                    mod_depth: 55.0,
                    eq_high_cut: 5000.0,
                    eq_low_cut: 20.0,
                    reverb_mode: 0, // Concert Hall
                    color_mode: 0,  // 1970s
                },
                AuraVerbPreset {
                    name: "80s Vocal Hall".to_string(),
                    mix: 100.0,
                    predelay: 24.0,
                    decay: 3.4,
                    damping_high_freq: 8000.0,
                    damping_high_shelf: -8.0,
                    damping_bass_freq: 350.0,
                    damping_bass_mult: 0.8,
                    size: 85.0,
                    attack: 60.0,
                    diff_early: 90.0,
                    diff_late: 95.0,
                    mod_rate: 2.2,
                    mod_depth: 30.0,
                    eq_high_cut: 10000.0,
                    eq_low_cut: 100.0,
                    reverb_mode: 1, // Bright Hall
                    color_mode: 1,  // 1980s
                },
                AuraVerbPreset {
                    name: "Clean Modern Studio".to_string(),
                    mix: 100.0,
                    predelay: 12.0,
                    decay: 1.8,
                    damping_high_freq: 14000.0,
                    damping_high_shelf: -4.0,
                    damping_bass_freq: 250.0,
                    damping_bass_mult: 1.0,
                    size: 65.0,
                    attack: 45.0,
                    diff_early: 80.0,
                    diff_late: 85.0,
                    mod_rate: 1.0,
                    mod_depth: 10.0,
                    eq_high_cut: 18000.0,
                    eq_low_cut: 30.0,
                    reverb_mode: 1, // Bright Hall
                    color_mode: 2,  // Now
                },
                AuraVerbPreset {
                    name: "Ambient Cloud".to_string(),
                    mix: 100.0,
                    predelay: 50.0,
                    decay: 28.0,
                    damping_high_freq: 6500.0,
                    damping_high_shelf: -12.0,
                    damping_bass_freq: 650.0,
                    damping_bass_mult: 1.4,
                    size: 100.0,
                    attack: 95.0,
                    diff_early: 100.0,
                    diff_late: 100.0,
                    mod_rate: 0.45,
                    mod_depth: 75.0,
                    eq_high_cut: 7500.0,
                    eq_low_cut: 25.0,
                    reverb_mode: 0, // Concert Hall
                    color_mode: 1,  // 1980s
                },
            ]
        });
        &FACTORY
    }

    /// Synchronize user preset directory with factory presets.
    pub fn ensure_factory_presets() {
        let dir = Self::preset_dir();
        for p in Self::factory_presets() {
            let filename = format!("{}.json", p.name.replace(['/', '\\', ' '], "_"));
            let path = dir.join(filename);
            if !path.exists() {
                let _ = p.save_to_file(&path);
            }
        }
    }

    /// List all presets found in preset_dir(), plus factory presets as fallback.
    pub fn list_all_presets() -> Vec<AuraVerbPreset> {
        Self::ensure_factory_presets();
        let dir = Self::preset_dir();
        let mut list = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(preset) = Self::load_from_file(&path) {
                        list.push(preset);
                    }
                }
            }
        }

        if list.is_empty() {
            list = Self::factory_presets().to_vec();
        } else {
            list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }

        list
    }
}
