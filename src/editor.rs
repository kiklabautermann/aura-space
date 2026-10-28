//! Vizia GUI for AuraVerb (Valhalla VintageVerb vector aesthetic with Preset handling).

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
use crate::params::AuraVerbParams;
use crate::preset::AuraVerbPreset;

// Vintage color palette matching vintagevwerb.png
pub const COLOR_BG: Color = Color::rgb(20, 4, 7);
pub const COLOR_PANEL_BG: Color = Color::rgb(26, 5, 9);

pub const COLOR_VALHALLA: Color = Color::rgb(243, 113, 33);
pub const COLOR_VINTAGE: Color = Color::rgb(255, 184, 77);

pub const COLOR_MIX_ORANGE: Color = Color::rgb(243, 111, 33);
pub const COLOR_DAMPING_AMBER: Color = Color::rgb(248, 190, 82);
pub const COLOR_SHAPE_ORANGE: Color = Color::rgb(245, 130, 41);
pub const COLOR_DIFF_REDORANGE: Color = Color::rgb(232, 86, 39);
pub const COLOR_MOD_CRIMSON: Color = Color::rgb(200, 43, 58);
pub const COLOR_EQ_DARKRED: Color = Color::rgb(138, 30, 40);

#[derive(Lens)]
pub struct GuiData {
    pub params: Arc<AuraVerbParams>,
    pub current_preset_name: String,
    pub preset_index: usize,
    pub presets: Vec<AuraVerbPreset>,
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
                let dir = AuraVerbPreset::preset_dir();
                let safe_name = self.current_preset_name.replace(['/', '\\', ' '], "_");
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&dir)
                    .set_file_name(&format!("{}.json", safe_name))
                    .add_filter("AuraVerb Preset (*.json)", &["json"])
                    .save_file()
                {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Custom Preset");
                    let new_preset = AuraVerbPreset::from_params(name, &self.params);
                    if let Ok(()) = new_preset.save_to_file(&path) {
                        self.current_preset_name = name.to_string();
                        self.presets = AuraVerbPreset::list_all_presets();
                        if let Some(idx) = self.presets.iter().position(|p| p.name == name) {
                            self.preset_index = idx;
                        }
                    }
                }
                meta.consume();
            }
            AppEvent::LoadPreset => {
                let dir = AuraVerbPreset::preset_dir();
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&dir)
                    .add_filter("AuraVerb Preset (*.json)", &["json"])
                    .pick_file()
                {
                    if let Ok(preset) = AuraVerbPreset::load_from_file(&path) {
                        self.current_preset_name = preset.name.clone();
                        preset.apply_to_params(&self.params, |ptr, val| {
                            cx.emit(RawParamEvent::BeginSetParameter(ptr));
                            cx.emit(RawParamEvent::SetParameterNormalized(ptr, val));
                            cx.emit(RawParamEvent::EndSetParameter(ptr));
                        });
                        cx.emit(RawParamEvent::ParametersChanged);
                        self.presets = AuraVerbPreset::list_all_presets();
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

/// Rotary knob disc view with flat vector aesthetic and angle notch.
pub struct KnobDisc {
    param_base: ParamWidgetBase,
    drag_active: bool,
    drag_start_y: f32,
    drag_start_val: f32,
    disc_color: vg::Color,
    radius: f32,
}

impl KnobDisc {
    pub fn new<L, Params, P, FMap>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
        disc_color: vg::Color,
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
            disc_color,
            radius,
        }
        .build(cx, |_| {})
    }
}

impl View for KnobDisc {
    fn element(&self) -> Option<&'static str> {
        Some("knob-disc")
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

        // 1. Draw flat circular disc
        let mut path = vg::Path::new();
        path.circle(center_x, center_y, r);
        canvas.fill_path(&path, &vg::Paint::color(self.disc_color));

        // 2. Draw notch indicator line
        let norm = self.param_base.unmodulated_normalized_value();
        let angle = -0.75 * PI + norm * 1.5 * PI;
        let sin_a = angle.sin();
        let cos_a = -angle.cos();

        let inner_r = r * 0.22;
        let outer_r = r * 0.95;

        let mut notch = vg::Path::new();
        notch.move_to(center_x + inner_r * sin_a, center_y + inner_r * cos_a);
        notch.line_to(center_x + outer_r * sin_a, center_y + outer_r * cos_a);

        let mut notch_paint = vg::Paint::color(vg::Color::rgb(20, 4, 7));
        notch_paint.set_line_width(2.5);
        canvas.stroke_path(&notch, &notch_paint);
    }
}

/// The Iconic 4-ring Concentric Decay Dial from Valhalla VintageVerb.
pub struct DecayDialDisc {
    param_base: ParamWidgetBase,
    drag_active: bool,
    drag_start_y: f32,
    drag_start_val: f32,
}

impl DecayDialDisc {
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

impl View for DecayDialDisc {
    fn element(&self) -> Option<&'static str> {
        Some("decay-dial-disc")
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

        let outer_r = (bounds.w * 0.5).min(bounds.h * 0.5) - 2.0;
        let ring_thickness = outer_r * 0.15;

        // 4 Concentric Rings
        let rings = [
            (outer_r, vg::Color::rgb(248, 190, 82)),
            (outer_r - ring_thickness, vg::Color::rgb(245, 130, 41)),
            (outer_r - ring_thickness * 2.0, vg::Color::rgb(230, 71, 36)),
            (outer_r - ring_thickness * 3.0, vg::Color::rgb(140, 27, 34)),
        ];

        for &(r, color) in &rings {
            let mut path = vg::Path::new();
            path.circle(center_x, center_y, r);
            canvas.fill_path(&path, &vg::Paint::color(color));
        }

        // Inner hollow center (dark wine hole)
        let inner_hole_r = outer_r - ring_thickness * 4.0;
        let mut hole_path = vg::Path::new();
        hole_path.circle(center_x, center_y, inner_hole_r);
        canvas.fill_path(&hole_path, &vg::Paint::color(vg::Color::rgb(20, 4, 7)));

        // Cut / pointer line across all rings
        let norm = self.param_base.unmodulated_normalized_value();
        let angle = -0.75 * PI + norm * 1.5 * PI;
        let sin_a = angle.sin();
        let cos_a = -angle.cos();

        let mut slit = vg::Path::new();
        slit.move_to(
            center_x + (inner_hole_r - 1.0) * sin_a,
            center_y + (inner_hole_r - 1.0) * cos_a,
        );
        slit.line_to(
            center_x + (outer_r + 1.0) * sin_a,
            center_y + (outer_r + 1.0) * cos_a,
        );

        let mut slit_paint = vg::Paint::color(vg::Color::rgb(20, 4, 7));
        slit_paint.set_line_width(3.5);
        canvas.stroke_path(&slit, &slit_paint);
    }
}

/// Helper function to build a complete VintageKnob unit: Label + KnobDisc + Value.
fn build_vintage_knob<L, Params, P, FMap>(
    cx: &mut Context,
    params: L,
    params_to_param: FMap,
    label_text: &'static str,
    disc_color: vg::Color,
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
                .font_size(12.0)
                .color(Color::rgb(245, 245, 245))
                .height(Pixels(18.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

            // Custom Flat Circular Knob Disc
            KnobDisc::new(cx, params, params_to_param, disc_color, radius)
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
                .color(Color::rgb(235, 235, 235))
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
                    .color(Color::rgb(220, 220, 220))
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
    params: Arc<AuraVerbParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        // Register standard Noto Sans fonts
        assets::register_noto_sans_regular(cx);
        assets::register_noto_sans_bold(cx);

        let presets = AuraVerbPreset::list_all_presets();
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
            // TOP HEADER: Title "Auraverb" & Actual Version
            // -------------------------------------------------------------
            HStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, "Aura")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(28.0)
                        .color(COLOR_VALHALLA);
                    Label::new(cx, "verb")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(28.0)
                        .color(COLOR_VINTAGE);
                })
                .col_between(Pixels(0.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                Label::new(cx, concat!("Version ", env!("CARGO_PKG_VERSION")))
                    .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                    .font_size(12.0)
                    .color(Color::rgb(180, 140, 130))
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
            // MIDDLE CONTROLS AREA (Columns 1-7 with enlarged 290px frame height)
            // -------------------------------------------------------------
            HStack::new(cx, |cx| {
                // Column 1: MIX and PREDELAY (Orange)
                VStack::new(cx, |cx| {
                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.mix,
                        "MIX",
                        vg::Color::rgb(243, 111, 33),
                        20.0,
                    );
                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.predelay,
                        "PREDELAY",
                        vg::Color::rgb(243, 111, 33),
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

                // Column 2: DECAY (Big Concentric Ring Dial)
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
                                .font_size(15.0)
                                .color(Color::rgb(240, 240, 240))
                                .height(Pixels(20.0))
                                .child_top(Stretch(1.0))
                                .child_bottom(Stretch(1.0));

                            DecayDialDisc::new(cx, GuiData::params, |p| &p.decay)
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
                                .color(Color::rgb(240, 240, 240))
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

                // Column 3: DAMPING Panel (Amber Border, 4 Knobs)
                VStack::new(cx, |cx| {
                    Label::new(cx, "DAMPING")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(13.0)
                        .color(COLOR_DAMPING_AMBER)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        VStack::new(cx, |cx| {
                            build_vintage_knob(
                                cx,
                                GuiData::params,
                                |p| &p.damping_high_freq,
                                "HighFreq",
                                vg::Color::rgb(248, 190, 82),
                                17.5,
                            );
                        })
                        .width(Pixels(92.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                        VStack::new(cx, |cx| {
                            build_vintage_knob(
                                cx,
                                GuiData::params,
                                |p| &p.damping_high_shelf,
                                "HighShelf",
                                vg::Color::rgb(248, 190, 82),
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
                            build_vintage_knob(
                                cx,
                                GuiData::params,
                                |p| &p.damping_bass_freq,
                                "BassFreq",
                                vg::Color::rgb(248, 190, 82),
                                17.5,
                            );
                        })
                        .width(Pixels(92.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                        VStack::new(cx, |cx| {
                            build_vintage_knob(
                                cx,
                                GuiData::params,
                                |p| &p.damping_bass_mult,
                                "BassMult",
                                vg::Color::rgb(248, 190, 82),
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
                .border_color(COLOR_DAMPING_AMBER)
                .border_width(Pixels(1.5))
                .border_radius(Pixels(16.0))
                .background_color(COLOR_PANEL_BG)
                .row_between(Pixels(22.0))
                .bottom(Pixels(12.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                // Column 4: SHAPE Panel (Orange Border: Size, Attack)
                VStack::new(cx, |cx| {
                    Label::new(cx, "SHAPE")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(13.0)
                        .color(COLOR_SHAPE_ORANGE)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.size,
                        "Size",
                        vg::Color::rgb(245, 130, 41),
                        18.0,
                    );
                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.attack,
                        "Attack",
                        vg::Color::rgb(245, 130, 41),
                        18.0,
                    );
                })
                .width(Pixels(96.0))
                .height(Pixels(290.0))
                .border_color(COLOR_SHAPE_ORANGE)
                .border_width(Pixels(1.5))
                .border_radius(Pixels(20.0))
                .background_color(COLOR_PANEL_BG)
                .row_between(Pixels(26.0))
                .bottom(Pixels(12.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                // Column 5: DIFF Panel (Red-Orange Border: Early, Late)
                VStack::new(cx, |cx| {
                    Label::new(cx, "DIFF")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(13.0)
                        .color(COLOR_DIFF_REDORANGE)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.diff_early,
                        "Early",
                        vg::Color::rgb(232, 86, 39),
                        18.0,
                    );
                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.diff_late,
                        "Late",
                        vg::Color::rgb(232, 86, 39),
                        18.0,
                    );
                })
                .width(Pixels(96.0))
                .height(Pixels(290.0))
                .border_color(COLOR_DIFF_REDORANGE)
                .border_width(Pixels(1.5))
                .border_radius(Pixels(20.0))
                .background_color(COLOR_PANEL_BG)
                .row_between(Pixels(26.0))
                .bottom(Pixels(12.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                // Column 6: MOD Panel (Crimson Border: Rate, Depth)
                VStack::new(cx, |cx| {
                    Label::new(cx, "MOD")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(13.0)
                        .color(COLOR_MOD_CRIMSON)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.mod_rate,
                        "Rate",
                        vg::Color::rgb(200, 43, 58),
                        18.0,
                    );
                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.mod_depth,
                        "Depth",
                        vg::Color::rgb(200, 43, 58),
                        18.0,
                    );
                })
                .width(Pixels(96.0))
                .height(Pixels(290.0))
                .border_color(COLOR_MOD_CRIMSON)
                .border_width(Pixels(1.5))
                .border_radius(Pixels(20.0))
                .background_color(COLOR_PANEL_BG)
                .row_between(Pixels(26.0))
                .bottom(Pixels(12.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0));

                // Column 7: EQ Panel (Dark Red Border: HighCut, LowCut)
                VStack::new(cx, |cx| {
                    Label::new(cx, "EQ")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(13.0)
                        .color(COLOR_EQ_DARKRED)
                        .top(Pixels(12.0))
                        .height(Pixels(20.0))
                        .child_left(Stretch(1.0))
                        .child_right(Stretch(1.0));

                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.eq_high_cut,
                        "HighCut",
                        vg::Color::rgb(138, 30, 40),
                        18.0,
                    );
                    build_vintage_knob(
                        cx,
                        GuiData::params,
                        |p| &p.eq_low_cut,
                        "LowCut",
                        vg::Color::rgb(138, 30, 40),
                        18.0,
                    );
                })
                .width(Pixels(96.0))
                .height(Pixels(290.0))
                .border_color(COLOR_EQ_DARKRED)
                .border_width(Pixels(1.5))
                .border_radius(Pixels(20.0))
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
            // BOTTOM BAR: MODE, COLOR, PRESETS (with Load/Save)
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
                    COLOR_MIX_ORANGE,
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
                    COLOR_MIX_ORANGE,
                );

                // Presets controls area
                HStack::new(cx, |cx| {
                    Label::new(cx, "PRESETS:")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(13.0)
                        .color(Color::rgb(220, 220, 220));

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
                                .color(COLOR_MIX_ORANGE)
                        },
                    )
                    .width(Pixels(24.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(COLOR_MIX_ORANGE)
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
                                .color(COLOR_VINTAGE)
                        },
                    )
                    .width(Pixels(110.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(Color::rgb(80, 40, 30))
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
                                .color(COLOR_MIX_ORANGE)
                        },
                    )
                    .width(Pixels(24.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(COLOR_MIX_ORANGE)
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
                                .color(COLOR_VINTAGE)
                        },
                    )
                    .width(Pixels(42.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(COLOR_VINTAGE)
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
                                .color(COLOR_MIX_ORANGE)
                        },
                    )
                    .width(Pixels(42.0))
                    .height(Pixels(24.0))
                    .background_color(COLOR_PANEL_BG)
                    .border_color(COLOR_MIX_ORANGE)
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
