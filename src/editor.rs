//! Vizia GUI for AuraVerb (Proprietary 'Aura Studio' Vector Aesthetic).
//!
//! Distinctive, modern, studio-grade visual design:
//! - Sleek brushed obsidian / dark graphite chassis (#0e1014 / #161a21).
//! - Custom Aura Orbital Decay Dial (radiant golden arc + precision cyan needle).
//! - Modern studio rack-mount panels with subtle 8px corners and colored header badges.
//! - Color-coded frequency and modulation sections (Cyan, Amber, Orange, Mint, Violet, Coral).
//! - 100% original visual design, fully legally independent and trade-dress safe.

use std::f32::consts::PI;
use std::sync::Arc;

use nih_plug::prelude::{Editor, Param};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util::ModifiersExt;
use nih_plug_vizia::widgets::{RawParamEvent, ResizeHandle};
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};

use crate::dsp::{ColorMode, ReverbMode};
use crate::params::AuraSpaceParams;
use crate::preset::AuraSpacePreset;

// =========================================================================
// Aura Studio Color Palette (100% Original Brand Identity)
// =========================================================================
pub const COLOR_BG: Color = Color::rgb(14, 16, 20);            // #0e1014 Deep Obsidian
pub const COLOR_PANEL_BG: Color = Color::rgb(22, 26, 33);      // #161a21 Studio Slate
pub const COLOR_PANEL_BORDER: Color = Color::rgb(42, 50, 64);  // #2a3240 Subtle Rack Divider

// Brand Accent Colors
pub const COLOR_AURA_CYAN: Color = Color::rgb(56, 189, 248);   // #38bdf8 Vibrant Sky Cyan
pub const COLOR_AURA_GOLD: Color = Color::rgb(245, 158, 11);   // #f59e0b Radiant Solar Gold
pub const COLOR_TEXT_WHITE: Color = Color::rgb(241, 245, 249); // #f1f5f9 Crisp Label Text
pub const COLOR_TEXT_MUTED: Color = Color::rgb(148, 163, 184); // #94a3b8 Soft Subtitle Slate

// Section Accent Colors (Modern Hardware Rack Identity)
pub const COLOR_ACCENT_MAIN: Color = Color::rgb(56, 189, 248);  // Cyan (Mix / PreDelay)
pub const COLOR_ACCENT_DAMP: Color = Color::rgb(245, 158, 11);  // Gold (Damping)
pub const COLOR_ACCENT_SHAPE: Color = Color::rgb(249, 115, 22); // Solar Orange (Shape)
pub const COLOR_ACCENT_DIFF: Color = Color::rgb(16, 185, 129);  // Mint Emerald (Diffusion)
pub const COLOR_ACCENT_MOD: Color = Color::rgb(168, 85, 247);   // Cosmic Violet (Modulation)
pub const COLOR_ACCENT_EQ: Color = Color::rgb(244, 63, 94);     // Studio Coral / Rose (Output EQ)

#[derive(Lens)]
pub struct GuiData {
    pub params: Arc<AuraSpaceParams>,
    pub current_preset_name: String,
    pub preset_index: usize,
    pub presets: Vec<AuraSpacePreset>,
}

#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    NextPreset,
    PrevPreset,
    SavePreset,
    LoadPreset,
}

impl Model for GuiData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event: &AppEvent, meta| match app_event {
            AppEvent::NextPreset => {
                if !self.presets.is_empty() {
                    self.preset_index = (self.preset_index + 1) % self.presets.len();
                    if let Some(preset) = self.presets.get(self.preset_index).cloned() {
                        self.current_preset_name = preset.name.clone();
                        preset.apply_to_params(&self.params, |ptr, val| {
                            cx.emit(RawParamEvent::BeginSetParameter(ptr));
                            cx.emit(RawParamEvent::SetParameterNormalized(ptr, val));
                            cx.emit(RawParamEvent::EndSetParameter(ptr));
                        });
                        cx.emit(RawParamEvent::ParametersChanged);
                    }
                }
                meta.consume();
            }
            AppEvent::PrevPreset => {
                if !self.presets.is_empty() {
                    self.preset_index = if self.preset_index == 0 || self.preset_index >= self.presets.len() {
                        self.presets.len() - 1
                    } else {
                        self.preset_index - 1
                    };
                    if let Some(preset) = self.presets.get(self.preset_index).cloned() {
                        self.current_preset_name = preset.name.clone();
                        preset.apply_to_params(&self.params, |ptr, val| {
                            cx.emit(RawParamEvent::BeginSetParameter(ptr));
                            cx.emit(RawParamEvent::SetParameterNormalized(ptr, val));
                            cx.emit(RawParamEvent::EndSetParameter(ptr));
                        });
                        cx.emit(RawParamEvent::ParametersChanged);
                    }
                }
                meta.consume();
            }
            AppEvent::SavePreset => {
                let dir = AuraSpacePreset::preset_dir();
                let safe_name = self.current_preset_name.replace(['/', '\\', ' '], "_");
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&dir)
                    .set_file_name(&format!("{}.json", safe_name))
                    .add_filter("AuraSpace Preset (*.json)", &["json"])
                    .save_file()
                {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Custom Preset");
                    let new_preset = AuraSpacePreset::from_params(name, &self.params);
                    if let Ok(()) = new_preset.save_to_file(&path) {
                        self.current_preset_name = name.to_string();
                        self.presets = AuraSpacePreset::list_all_presets();
                        if let Some(idx) = self.presets.iter().position(|p| p.name == name) {
                            self.preset_index = idx;
                        }
                    }
                }
                meta.consume();
            }
            AppEvent::LoadPreset => {
                let dir = AuraSpacePreset::preset_dir();
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&dir)
                    .add_filter("AuraSpace Preset (*.json)", &["json"])
                    .pick_file()
                {
                    if let Ok(preset) = AuraSpacePreset::load_from_file(&path) {
                        self.current_preset_name = preset.name.clone();
                        preset.apply_to_params(&self.params, |ptr, val| {
                            cx.emit(RawParamEvent::BeginSetParameter(ptr));
                            cx.emit(RawParamEvent::SetParameterNormalized(ptr, val));
                            cx.emit(RawParamEvent::EndSetParameter(ptr));
                        });
                        cx.emit(RawParamEvent::ParametersChanged);
                        self.presets = AuraSpacePreset::list_all_presets();
                        if let Some(idx) = self.presets.iter().position(|p| p.name == preset.name) {
                            self.preset_index = idx;
                        }
                    }
                }
                meta.consume();
            }
        });
    }
}

/// Rotary knob disc view with modern studio bezel and high-contrast indicator notch.
pub struct StudioKnobDisc {
    param_base: ParamWidgetBase,
    drag_active: bool,
    drag_start_y: f32,
    drag_start_val: f32,
    accent_color: vg::Color,
    radius: f32,
}

impl StudioKnobDisc {
    pub fn new<L, Params, P, FMap>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
        accent_color: vg::Color,
        radius: f32,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params, params_to_param),
            drag_active: false,
            drag_start_y: 0.0,
            drag_start_val: 0.0,
            accent_color,
            radius,
        }
        .build(cx, |_| {})
    }
}

impl View for StudioKnobDisc {
    fn element(&self) -> Option<&'static str> {
        Some("studio-knob-disc")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.drag_active = true;
                self.drag_start_y = cx.mouse().cursory;
                self.drag_start_val = self.param_base.unmodulated_normalized_value();
                self.param_base.begin_set_parameter(cx);
                cx.capture();
                cx.focus();
                meta.consume();
            }
            WindowEvent::MouseMove(_x, y) => {
                if self.drag_active {
                    let delta_y = self.drag_start_y - *y;
                    let sensitivity = if cx.modifiers().shift() { 0.001 } else { 0.004 };
                    let new_val = (self.drag_start_val + delta_y * sensitivity).clamp(0.0, 1.0);
                    self.param_base.set_normalized_value(cx, new_val);
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_active {
                    self.drag_active = false;
                    self.param_base.end_set_parameter(cx);
                    cx.release();
                    meta.consume();
                }
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                self.param_base.begin_set_parameter(cx);
                self.param_base
                    .set_normalized_value(cx, self.param_base.default_normalized_value());
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }
            WindowEvent::MouseScroll(_x, y) => {
                let current = self.param_base.unmodulated_normalized_value();
                let step = *y * 0.02;
                self.param_base.begin_set_parameter(cx);
                self.param_base
                    .set_normalized_value(cx, (current + step).clamp(0.0, 1.0));
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let center_x = bounds.x + bounds.w * 0.5;
        let center_y = bounds.y + bounds.h * 0.5;
        let r = self.radius.min(bounds.w * 0.5 - 2.0);

        // 1. Outer metallic bezel ring
        let mut bezel = vg::Path::new();
        bezel.circle(center_x, center_y, r);
        let mut bezel_paint = vg::Paint::color(vg::Color::rgb(45, 55, 72));
        bezel_paint.set_line_width(1.5);
        canvas.stroke_path(&bezel, &bezel_paint);

        // 2. Inner slate knob face
        let mut face = vg::Path::new();
        face.circle(center_x, center_y, r - 1.0);
        canvas.fill_path(&face, &vg::Paint::color(vg::Color::rgb(28, 34, 44)));

        // 3. Precision pointer needle in section accent color
        let norm = self.param_base.unmodulated_normalized_value();
        let angle = -0.75 * PI + norm * 1.5 * PI;
        let sin_a = angle.sin();
        let cos_a = -angle.cos();

        let inner_r = r * 0.22;
        let outer_r = r * 0.92;

        let mut notch = vg::Path::new();
        notch.move_to(center_x + inner_r * sin_a, center_y + inner_r * cos_a);
        notch.line_to(center_x + outer_r * sin_a, center_y + outer_r * cos_a);

        let mut notch_paint = vg::Paint::color(self.accent_color);
        notch_paint.set_line_width(2.5);
        notch_paint.set_line_cap(vg::LineCap::Round);
        canvas.stroke_path(&notch, &notch_paint);

        // 4. Center hub
        let mut hub = vg::Path::new();
        hub.circle(center_x, center_y, 3.0);
        canvas.fill_path(&hub, &vg::Paint::color(vg::Color::rgb(14, 16, 20)));
    }
}

/// Custom Aura Orbital Decay Dial (Glowing golden halo arc + precision cyan indicator).
pub struct AuraDecayDialDisc {
    param_base: ParamWidgetBase,
    drag_active: bool,
    drag_start_y: f32,
    drag_start_val: f32,
}

impl AuraDecayDialDisc {
    pub fn new<L, Params, P, FMap>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params, params_to_param),
            drag_active: false,
            drag_start_y: 0.0,
            drag_start_val: 0.0,
        }
        .build(cx, |_| {})
    }
}

impl View for AuraDecayDialDisc {
    fn element(&self) -> Option<&'static str> {
        Some("aura-decay-dial-disc")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.drag_active = true;
                self.drag_start_y = cx.mouse().cursory;
                self.drag_start_val = self.param_base.unmodulated_normalized_value();
                self.param_base.begin_set_parameter(cx);
                cx.capture();
                cx.focus();
                meta.consume();
            }
            WindowEvent::MouseMove(_x, y) => {
                if self.drag_active {
                    let delta_y = self.drag_start_y - *y;
                    let sensitivity = if cx.modifiers().shift() { 0.001 } else { 0.003 };
                    let new_val = (self.drag_start_val + delta_y * sensitivity).clamp(0.0, 1.0);
                    self.param_base.set_normalized_value(cx, new_val);
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_active {
                    self.drag_active = false;
                    self.param_base.end_set_parameter(cx);
                    cx.release();
                    meta.consume();
                }
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                self.param_base.begin_set_parameter(cx);
                self.param_base
                    .set_normalized_value(cx, self.param_base.default_normalized_value());
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }
            WindowEvent::MouseScroll(_x, y) => {
                let current = self.param_base.unmodulated_normalized_value();
                let step = *y * 0.02;
                self.param_base.begin_set_parameter(cx);
                self.param_base
                    .set_normalized_value(cx, (current + step).clamp(0.0, 1.0));
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let center_x = bounds.x + bounds.w * 0.5;
        let center_y = bounds.y + bounds.h * 0.5;

        let outer_r = (bounds.w * 0.5).min(bounds.h * 0.5) - 3.0;

        let norm = self.param_base.unmodulated_normalized_value();
        let start_angle = -0.75 * PI;
        let current_angle = start_angle + norm * 1.5 * PI;
        let total_arc = 1.5 * PI;

        // 1. Dark outer background guide track (-135° to +135°)
        let track_steps = 32;
        let mut track_path = vg::Path::new();
        for i in 0..=track_steps {
            let t = i as f32 / track_steps as f32;
            let a = start_angle + t * total_arc;
            let px = center_x + a.sin() * outer_r;
            let py = center_y - a.cos() * outer_r;
            if i == 0 {
                track_path.move_to(px, py);
            } else {
                track_path.line_to(px, py);
            }
        }
        let mut track_paint = vg::Paint::color(vg::Color::rgb(35, 42, 54));
        track_paint.set_line_width(4.0);
        track_paint.set_line_cap(vg::LineCap::Round);
        canvas.stroke_path(&track_path, &track_paint);

        // 2. Active radiant solar gold progress arc
        if norm > 0.005 {
            let active_steps = (track_steps as f32 * norm).ceil() as usize;
            let mut arc_path = vg::Path::new();
            for i in 0..=active_steps {
                let t = (i as f32 / active_steps as f32) * norm;
                let a = start_angle + t * total_arc;
                let px = center_x + a.sin() * outer_r;
                let py = center_y - a.cos() * outer_r;
                if i == 0 {
                    arc_path.move_to(px, py);
                } else {
                    arc_path.line_to(px, py);
                }
            }
            let mut arc_paint = vg::Paint::color(vg::Color::rgb(245, 158, 11)); // Solar Gold
            arc_paint.set_line_width(5.5);
            arc_paint.set_line_cap(vg::LineCap::Round);
            canvas.stroke_path(&arc_path, &arc_paint);
        }

        // 3. Central disc body (modern studio slate)
        let inner_disc_r = outer_r - 16.0;
        let mut disc_path = vg::Path::new();
        disc_path.circle(center_x, center_y, inner_disc_r);
        canvas.fill_path(&disc_path, &vg::Paint::color(vg::Color::rgb(26, 32, 42)));

        let mut disc_border = vg::Path::new();
        disc_border.circle(center_x, center_y, inner_disc_r);
        let mut disc_paint = vg::Paint::color(vg::Color::rgb(50, 62, 80));
        disc_paint.set_line_width(1.5);
        canvas.stroke_path(&disc_border, &disc_paint);

        // 4. Precision glowing cyan needle indicator
        let sin_a = current_angle.sin();
        let cos_a = -current_angle.cos();

        let mut needle = vg::Path::new();
        needle.move_to(center_x + 10.0 * sin_a, center_y + 10.0 * cos_a);
        needle.line_to(center_x + (inner_disc_r - 2.0) * sin_a, center_y + (inner_disc_r - 2.0) * cos_a);

        let mut needle_paint = vg::Paint::color(vg::Color::rgb(56, 189, 248)); // Vibrant Cyan
        needle_paint.set_line_width(2.5);
        needle_paint.set_line_cap(vg::LineCap::Round);
        canvas.stroke_path(&needle, &needle_paint);

        // 5. Center dark hub
        let mut hub = vg::Path::new();
        hub.circle(center_x, center_y, 6.0);
        canvas.fill_path(&hub, &vg::Paint::color(vg::Color::rgb(14, 16, 20)));
    }
}

/// Helper function to build a complete StudioKnob unit: Label + StudioKnobDisc + Value.
fn build_studio_knob<L, Params, P, FMap>(
    cx: &mut Context,
    params: L,
    params_to_param: FMap,
    label_text: &'static str,
    accent_color: vg::Color,
    radius: f32,
) where
    L: Lens<Target = Params> + Clone + 'static,
    Params: 'static,
    P: Param + 'static,
    FMap: Fn(&Params) -> &P + Copy + 'static,
{
    ParamWidgetBase::build_view(params.clone(), params_to_param, move |cx, param_data| {
        VStack::new(cx, |cx| {
            // Parameter Name Header
            Label::new(cx, label_text)
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Bold)
                .font_size(11.5)
                .color(COLOR_TEXT_WHITE)
                .height(Pixels(18.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

            // Custom Modern Studio Knob Disc
            StudioKnobDisc::new(cx, params, params_to_param, accent_color, radius)
                .width(Pixels(radius * 2.0 + 6.0))
                .height(Pixels(radius * 2.0 + 6.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0));

            // Formatted Value Display
            let value_lens = param_data.make_lens(|param| {
                param.normalized_value_to_string(param.unmodulated_normalized_value(), true)
            });
            Label::new(cx, value_lens)
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_size(11.5)
                .color(Color::rgb(220, 226, 235))
                .height(Pixels(18.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));
        })
        .row_between(Pixels(5.0))
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0));
    })(cx);
}

/// Interactive clickable cycle button for enum parameters (Mode and Color).
pub struct EnumCycleButton {
    param_base: ParamWidgetBase,
    num_variants: usize,
}

impl EnumCycleButton {
    pub fn new<'a, L, Params, P, FMap, V>(
        cx: &'a mut Context,
        params: L,
        params_to_param: FMap,
        num_variants: usize,
        prefix_label: &'static str,
        value_lens: V,
        value_color: Color,
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = Params> + Clone + 'static,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
        V: Lens<Target = &'static str> + Clone + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params, params_to_param),
            num_variants,
        }
        .build(cx, move |cx| {
            HStack::new(cx, move |cx| {
                Label::new(cx, prefix_label)
                    .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                    .font_weight(FontWeightKeyword::Bold)
                    .font_size(13.5)
                    .color(COLOR_TEXT_MUTED)
                    .hoverable(false);

                Label::new(cx, value_lens)
                    .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                    .font_weight(FontWeightKeyword::Bold)
                    .font_size(13.5)
                    .color(value_color)
                    .hoverable(false);
            })
            .col_between(Pixels(6.0))
            .child_top(Stretch(1.0))
            .child_bottom(Stretch(1.0));
        })
        .cursor(CursorIcon::Hand)
    }
}

impl View for EnumCycleButton {
    fn element(&self) -> Option<&'static str> {
        Some("enum-cycle-button")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let current_norm = self.param_base.unmodulated_normalized_value();
                let step = 1.0 / self.num_variants.max(1) as f32;
                let next_norm = (current_norm + step + 0.001).fract();

                self.param_base.begin_set_parameter(cx);
                self.param_base.set_normalized_value(cx, next_norm);
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }
            _ => {}
        });
    }
}

pub fn create(
    params: Arc<AuraSpaceParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        // Register standard Noto Sans fonts
        assets::register_noto_sans_regular(cx);
        assets::register_noto_sans_bold(cx);

        let presets = AuraSpacePreset::list_all_presets();
        let current_preset_name = presets
            .first()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Concert Hall".to_string());

        GuiData {
            params: params.clone(),
            current_preset_name,
            preset_index: 0,
            presets,
        }
        .build(cx);

        // Main Background Window (1020 x 410)
        VStack::new(cx, |cx| {
            // -------------------------------------------------------------
            // TOP HEADER: Independent 'Aura Studio' Branding
            // -------------------------------------------------------------
            HStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, "AURA")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(26.0)
                        .color(COLOR_AURA_CYAN);
                    Label::new(cx, "SPACE")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(26.0)
                        .color(COLOR_AURA_GOLD);
                    Label::new(cx, " STUDIO")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(11.0)
                        .color(COLOR_TEXT_MUTED)
                        .top(Pixels(10.0));
                })
                .col_between(Pixels(2.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                Label::new(cx, "CLAP DSP Engine • v0.1.0")
                    .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                    .font_size(12.0)
                    .color(COLOR_TEXT_MUTED)
                    .child_right(Stretch(1.0))
                    .child_top(Stretch(1.0))
                    .child_bottom(Stretch(1.0));
            })
            .height(Pixels(46.0))
            .left(Pixels(32.0))
            .right(Pixels(32.0))
            .top(Pixels(8.0))
            .child_top(Stretch(1.0))
            .child_bottom(Stretch(1.0));

            // -------------------------------------------------------------
            // MIDDLE CONTROLS AREA (Studio Modular Rack Modules)
            // -------------------------------------------------------------
            HStack::new(cx, |cx| {
                // Column 1: MIX and PREDELAY (Aura Cyan)
                VStack::new(cx, |cx| {
                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.mix,
                        "MIX",
                        vg::Color::rgb(56, 189, 248),
                        20.0,
                    );
                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.predelay,
                        "PREDELAY",
                        vg::Color::rgb(56, 189, 248),
                        20.0,
                    );
                })
                .width(Pixels(96.0))
                .height(Pixels(290.0))
                .row_between(Pixels(32.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                // Column 2: DECAY (Aura Orbital Dial)
                ParamWidgetBase::build_view(
                    GuiData::params,
                    |p| &p.decay,
                    move |cx, param_data| {
                        VStack::new(cx, |cx| {
                            Label::new(cx, "DECAY")
                                .font_family(vec![FamilyOwned::Name(String::from(
                                    assets::NOTO_SANS,
                                ))])
                                .font_weight(FontWeightKeyword::Bold)
                                .font_size(14.5)
                                .color(COLOR_AURA_GOLD)
                                .height(Pixels(20.0))
                                .child_top(Stretch(1.0))
                                .child_bottom(Stretch(1.0));

                            AuraDecayDialDisc::new(cx, GuiData::params, |p| &p.decay)
                                .width(Pixels(124.0))
                                .height(Pixels(124.0))
                                .child_left(Stretch(1.0))
                                .child_right(Stretch(1.0));

                            let decay_val = param_data.make_lens(|param| {
                                param.normalized_value_to_string(
                                    param.unmodulated_normalized_value(),
                                    true,
                                )
                            });
                            Label::new(cx, decay_val)
                                .font_family(vec![FamilyOwned::Name(String::from(
                                    assets::NOTO_SANS,
                                ))])
                                .font_size(14.0)
                                .font_weight(FontWeightKeyword::Bold)
                                .color(COLOR_TEXT_WHITE)
                                .height(Pixels(20.0))
                                .child_top(Stretch(1.0))
                                .child_bottom(Stretch(1.0));
                        })
                        .width(Pixels(144.0))
                        .height(Pixels(290.0))
                        .row_between(Pixels(14.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Stretch(1.0));
                    },
                )(cx);

                // Column 3: DAMPING Module (Amber Gold)
                VStack::new(cx, |cx| {
                    Label::new(cx, "DAMPING")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(12.5)
                        .color(COLOR_ACCENT_DAMP)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        VStack::new(cx, |cx| {
                            build_studio_knob(
                                cx,
                                GuiData::params,
                                |p| &p.damping_high_freq,
                                "HighFreq",
                                vg::Color::rgb(245, 158, 11),
                                17.5,
                            );
                        })
                        .width(Pixels(92.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                        VStack::new(cx, |cx| {
                            build_studio_knob(
                                cx,
                                GuiData::params,
                                |p| &p.damping_high_shelf,
                                "HighShelf",
                                vg::Color::rgb(245, 158, 11),
                                17.5,
                            );
                        })
                        .width(Pixels(92.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));
                    })
                    .col_between(Pixels(6.0))
                    .child_left(Stretch(1.0))
                    .child_right(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        VStack::new(cx, |cx| {
                            build_studio_knob(
                                cx,
                                GuiData::params,
                                |p| &p.damping_bass_freq,
                                "BassFreq",
                                vg::Color::rgb(245, 158, 11),
                                17.5,
                            );
                        })
                        .width(Pixels(92.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                        VStack::new(cx, |cx| {
                            build_studio_knob(
                                cx,
                                GuiData::params,
                                |p| &p.damping_bass_mult,
                                "BassMult",
                                vg::Color::rgb(245, 158, 11),
                                17.5,
                            );
                        })
                        .width(Pixels(92.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));
                    })
                    .col_between(Pixels(6.0))
                    .child_left(Stretch(1.0))
                    .child_right(Stretch(1.0));
                })
                .width(Pixels(204.0))
                .height(Pixels(290.0))
                .border_color(COLOR_PANEL_BORDER)
                .border_width(Pixels(1.0))
                .border_radius(Pixels(8.0))
                .background_color(COLOR_PANEL_BG)
                .row_between(Pixels(22.0))
                .bottom(Pixels(12.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                // Column 4: SHAPE Module (Solar Orange)
                VStack::new(cx, |cx| {
                    Label::new(cx, "SHAPE")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(12.5)
                        .color(COLOR_ACCENT_SHAPE)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.size,
                        "Size",
                        vg::Color::rgb(249, 115, 22),
                        18.0,
                    );
                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.attack,
                        "Attack",
                        vg::Color::rgb(249, 115, 22),
                        18.0,
                    );
                })
                .width(Pixels(96.0))
                .height(Pixels(290.0))
                .border_color(COLOR_PANEL_BORDER)
                .border_width(Pixels(1.0))
                .border_radius(Pixels(8.0))
                .background_color(COLOR_PANEL_BG)
                .row_between(Pixels(26.0))
                .bottom(Pixels(12.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                // Column 5: DIFFUSION Module (Emerald Mint)
                VStack::new(cx, |cx| {
                    Label::new(cx, "DIFF")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(12.5)
                        .color(COLOR_ACCENT_DIFF)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.diff_early,
                        "Early",
                        vg::Color::rgb(16, 185, 129),
                        18.0,
                    );
                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.diff_late,
                        "Late",
                        vg::Color::rgb(16, 185, 129),
                        18.0,
                    );
                })
                .width(Pixels(96.0))
                .height(Pixels(290.0))
                .border_color(COLOR_PANEL_BORDER)
                .border_width(Pixels(1.0))
                .border_radius(Pixels(8.0))
                .background_color(COLOR_PANEL_BG)
                .row_between(Pixels(26.0))
                .bottom(Pixels(12.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                // Column 6: MODULATION Module (Cosmic Violet)
                VStack::new(cx, |cx| {
                    Label::new(cx, "MOD")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(12.5)
                        .color(COLOR_ACCENT_MOD)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.mod_rate,
                        "Rate",
                        vg::Color::rgb(168, 85, 247),
                        18.0,
                    );
                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.mod_depth,
                        "Depth",
                        vg::Color::rgb(168, 85, 247),
                        18.0,
                    );
                })
                .width(Pixels(96.0))
                .height(Pixels(290.0))
                .border_color(COLOR_PANEL_BORDER)
                .border_width(Pixels(1.0))
                .border_radius(Pixels(8.0))
                .background_color(COLOR_PANEL_BG)
                .row_between(Pixels(26.0))
                .bottom(Pixels(12.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                // Column 7: OUTPUT EQ Module (Studio Coral)
                VStack::new(cx, |cx| {
                    Label::new(cx, "EQ")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(12.5)
                        .color(COLOR_ACCENT_EQ)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.eq_high_cut,
                        "HighCut",
                        vg::Color::rgb(244, 63, 94),
                        18.0,
                    );
                    build_studio_knob(
                        cx,
                        GuiData::params,
                        |p| &p.eq_low_cut,
                        "LowCut",
                        vg::Color::rgb(244, 63, 94),
                        18.0,
                    );
                })
                .width(Pixels(96.0))
                .height(Pixels(290.0))
                .border_color(COLOR_PANEL_BORDER)
                .border_width(Pixels(1.0))
                .border_radius(Pixels(8.0))
                .background_color(COLOR_PANEL_BG)
                .row_between(Pixels(26.0))
                .bottom(Pixels(12.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));
            })
            .height(Pixels(290.0))
            .col_between(Pixels(16.0))
            .left(Pixels(28.0))
            .right(Pixels(28.0))
            .child_top(Stretch(1.0))
            .child_bottom(Stretch(1.0));

            // -------------------------------------------------------------
            // BOTTOM BAR: MODE, COLOR, PRESETS (Studio Rack Master Bar)
            // -------------------------------------------------------------
            HStack::new(cx, |cx| {
                // Interactive Mode Selector
                let mode_lens = GuiData::params.map(|p| match p.reverb_mode.value() {
                    ReverbMode::ConcertHall => "Concert Hall",
                    ReverbMode::BrightHall => "Bright Hall",
                    ReverbMode::Plate => "Plate",
                    ReverbMode::Room => "Room",
                });
                EnumCycleButton::new(
                    cx,
                    GuiData::params,
                    |p| &p.reverb_mode,
                    4,
                    "MODE:",
                    mode_lens,
                    COLOR_AURA_CYAN,
                );

                // Interactive Color Mode Selector
                let color_lens = GuiData::params.map(|p| match p.color_mode.value() {
                    ColorMode::Vintage1970s => "1970s",
                    ColorMode::Vintage1980s => "1980s",
                    ColorMode::Now => "NOW",
                });
                EnumCycleButton::new(
                    cx,
                    GuiData::params,
                    |p| &p.color_mode,
                    3,
                    "COLOR:",
                    color_lens,
                    COLOR_AURA_GOLD,
                );

                // Presets controls area
                HStack::new(cx, |cx| {
                    Label::new(cx, "PRESETS:")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(13.0)
                        .color(COLOR_TEXT_MUTED);

                    // Previous Button: ◀
                    Button::new(
                        cx,
                        |cx| cx.emit(AppEvent::PrevPreset),
                        |cx| {
                            Label::new(cx, "◀")
                                .font_family(vec![FamilyOwned::Name(String::from(
                                    assets::NOTO_SANS,
                                ))])
                                .font_size(11.0)
                                .color(COLOR_AURA_CYAN)
                        },
                    )
                    .width(Pixels(24.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(COLOR_PANEL_BORDER)
                    .border_width(Pixels(1.0))
                    .border_radius(Pixels(4.0))
                    .child_top(Stretch(1.0))
                    .child_bottom(Stretch(1.0))
                    .child_left(Stretch(1.0))
                    .child_right(Stretch(1.0));

                    // Current Preset Name (clickable to cycle next)
                    Button::new(
                        cx,
                        |cx| cx.emit(AppEvent::NextPreset),
                        |cx| {
                            Label::new(cx, GuiData::current_preset_name)
                                .font_family(vec![FamilyOwned::Name(String::from(
                                    assets::NOTO_SANS,
                                ))])
                                .font_weight(FontWeightKeyword::Bold)
                                .font_size(12.0)
                                .color(COLOR_TEXT_WHITE)
                        },
                    )
                    .width(Pixels(110.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(COLOR_PANEL_BORDER)
                    .border_width(Pixels(1.0))
                    .border_radius(Pixels(4.0))
                    .left(Pixels(2.0))
                    .right(Pixels(2.0))
                    .child_top(Stretch(1.0))
                    .child_bottom(Stretch(1.0))
                    .child_left(Stretch(1.0))
                    .child_right(Stretch(1.0));

                    // Next Button: ▶
                    Button::new(
                        cx,
                        |cx| cx.emit(AppEvent::NextPreset),
                        |cx| {
                            Label::new(cx, "▶")
                                .font_family(vec![FamilyOwned::Name(String::from(
                                    assets::NOTO_SANS,
                                ))])
                                .font_size(11.0)
                                .color(COLOR_AURA_CYAN)
                        },
                    )
                    .width(Pixels(24.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(COLOR_PANEL_BORDER)
                    .border_width(Pixels(1.0))
                    .border_radius(Pixels(4.0))
                    .child_top(Stretch(1.0))
                    .child_bottom(Stretch(1.0))
                    .child_left(Stretch(1.0))
                    .child_right(Stretch(1.0));

                    // SAVE Button
                    Button::new(
                        cx,
                        |cx| cx.emit(AppEvent::SavePreset),
                        |cx| {
                            Label::new(cx, "SAVE")
                                .font_family(vec![FamilyOwned::Name(String::from(
                                    assets::NOTO_SANS,
                                ))])
                                .font_weight(FontWeightKeyword::Bold)
                                .font_size(10.5)
                                .color(COLOR_AURA_GOLD)
                        },
                    )
                    .width(Pixels(42.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(COLOR_AURA_GOLD)
                    .border_width(Pixels(1.0))
                    .border_radius(Pixels(4.0))
                    .left(Pixels(4.0))
                    .child_top(Stretch(1.0))
                    .child_bottom(Stretch(1.0))
                    .child_left(Stretch(1.0))
                    .child_right(Stretch(1.0));

                    // LOAD Button
                    Button::new(
                        cx,
                        |cx| cx.emit(AppEvent::LoadPreset),
                        |cx| {
                            Label::new(cx, "LOAD")
                                .font_family(vec![FamilyOwned::Name(String::from(
                                    assets::NOTO_SANS,
                                ))])
                                .font_weight(FontWeightKeyword::Bold)
                                .font_size(10.5)
                                .color(COLOR_AURA_CYAN)
                        },
                    )
                    .width(Pixels(42.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(COLOR_AURA_CYAN)
                    .border_width(Pixels(1.0))
                    .border_radius(Pixels(4.0))
                    .left(Pixels(2.0))
                    .child_top(Stretch(1.0))
                    .child_bottom(Stretch(1.0))
                    .child_left(Stretch(1.0))
                    .child_right(Stretch(1.0));
                })
                .col_between(Pixels(3.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));
            })
            .height(Pixels(42.0))
            .left(Pixels(28.0))
            .right(Pixels(80.0))
            .col_between(Pixels(24.0))
            .child_top(Stretch(1.0))
            .child_bottom(Stretch(1.0));
        })
        .background_color(COLOR_BG)
        .width(Pixels(1020.0))
        .height(Pixels(410.0));

        ResizeHandle::new(cx);
    })
}
