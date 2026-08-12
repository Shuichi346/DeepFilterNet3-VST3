use std::sync::Arc;

use egui::{
    Align, Color32, CornerRadius, CursorIcon, Frame, Key, Layout, Margin, Rect, RichText,
    Sense, Stroke, TextEdit, TextStyle, Visuals, pos2, vec2,
};
use nice_plug::editor::dpi::LogicalSize;
use nice_plug::prelude::{Editor, FloatParam, Param, ParamSetter};
use nice_plug_egui::{EguiNiceSettings, EguiState, create_egui_editor};

use crate::params::DeepFilterParams;

pub(crate) const EDITOR_WIDTH: f32 = 420.0;
pub(crate) const EDITOR_HEIGHT: f32 = 190.0;

const BACKGROUND: Color32 = Color32::from_rgb(15, 18, 24);
const CARD_BACKGROUND: Color32 = Color32::from_rgb(22, 27, 36);
const CARD_BORDER: Color32 = Color32::from_rgb(39, 47, 61);
const TRACK_BACKGROUND: Color32 = Color32::from_rgb(43, 51, 65);
const ACCENT: Color32 = Color32::from_rgb(103, 142, 239);
const ACCENT_HOVERED: Color32 = Color32::from_rgb(124, 158, 244);
const PRIMARY_TEXT: Color32 = Color32::from_rgb(232, 236, 244);
const SECONDARY_TEXT: Color32 = Color32::from_rgb(148, 158, 177);
const FIELD_BACKGROUND: Color32 = Color32::from_rgb(12, 15, 21);
const FIELD_BORDER: Color32 = Color32::from_rgb(54, 65, 83);
const INVALID: Color32 = Color32::from_rgb(232, 100, 108);

const TRACK_HEIGHT: f32 = 10.0;
const ATTENUATION_PRESENTATION: ValuePresentation = ValuePresentation {
    scale: 1.0,
    decimals: 1,
    unit: "dB",
    field_width: 58.0,
};
const MIX_PRESENTATION: ValuePresentation = ValuePresentation {
    scale: 100.0,
    decimals: 0,
    unit: "%",
    field_width: 46.0,
};

#[derive(Clone, Copy)]
struct ValuePresentation {
    scale: f32,
    decimals: usize,
    unit: &'static str,
    field_width: f32,
}

#[derive(Default)]
struct EditorUiState {
    attenuation: ParameterControlState,
    mix: ParameterControlState,
}

#[derive(Default)]
struct ParameterControlState {
    input: String,
    invalid_input: bool,
    dragging: bool,
    drag_start_normalized: f32,
    drag_start_x: f32,
}

pub(crate) fn default_state() -> Arc<EguiState> {
    EguiState::from_size(LogicalSize::new(EDITOR_WIDTH, EDITOR_HEIGHT))
}

pub(crate) fn create(
    params: Arc<DeepFilterParams>,
    state: Arc<EguiState>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        state,
        EditorUiState::default(),
        EguiNiceSettings {
            title: String::from("DeepFilter Noise Reduction"),
            ..Default::default()
        },
        |context, _commands, _state| {
            let mut visuals = Visuals::dark();
            visuals.panel_fill = BACKGROUND;
            visuals.window_fill = BACKGROUND;
            visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, PRIMARY_TEXT);
            visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, PRIMARY_TEXT);
            visuals.widgets.active.fg_stroke = Stroke::new(1.0, PRIMARY_TEXT);
            visuals.selection.bg_fill = ACCENT;
            visuals.selection.stroke = Stroke::new(1.0, PRIMARY_TEXT);
            context.set_visuals(visuals);
        },
        move |ui, setter, _commands, state| {
            Frame::new()
                .fill(BACKGROUND)
                .inner_margin(Margin::symmetric(20, 15))
                .show(ui, |ui| {
                    ui.set_width(EDITOR_WIDTH - 40.0);
                    parameter_control(
                        ui,
                        "Attenuation Limit",
                        &params.atten_lim,
                        setter,
                        &mut state.attenuation,
                        ATTENUATION_PRESENTATION,
                    );
                    ui.add_space(12.0);
                    parameter_control(
                        ui,
                        "Mix",
                        &params.mix,
                        setter,
                        &mut state.mix,
                        MIX_PRESENTATION,
                    );
                });
        },
    )
}

fn parameter_control(
    ui: &mut egui::Ui,
    label: &'static str,
    param: &FloatParam,
    setter: &ParamSetter,
    state: &mut ParameterControlState,
    presentation: ValuePresentation,
) {
    Frame::new()
        .fill(CARD_BACKGROUND)
        .stroke(Stroke::new(1.0, CARD_BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(13, 9))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(label)
                        .size(13.5)
                        .strong()
                        .color(PRIMARY_TEXT),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(presentation.unit)
                            .size(12.0)
                            .color(SECONDARY_TEXT),
                    );
                    value_editor(ui, label, param, setter, state, presentation);
                });
            });

            ui.add_space(7.0);
            parameter_track(ui, param, setter, state);
        });
}

fn value_editor(
    ui: &mut egui::Ui,
    label: &'static str,
    param: &FloatParam,
    setter: &ParamSetter,
    state: &mut ParameterControlState,
    presentation: ValuePresentation,
) {
    let edit_id = ui.make_persistent_id(("parameter-value", label));
    let focused_before = ui.memory(|memory| memory.has_focus(edit_id));
    if !focused_before {
        state.input = format_numeric_value(param.modulated_plain_value(), presentation);
        state.invalid_input = false;
    }

    let border_color = if state.invalid_input {
        INVALID
    } else if focused_before {
        ACCENT
    } else {
        FIELD_BORDER
    };
    let edit_frame = Frame::new()
        .fill(FIELD_BACKGROUND)
        .stroke(Stroke::new(1.0, border_color))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(7, 3));

    let response = ui.add(
        TextEdit::singleline(&mut state.input)
            .id(edit_id)
            .font(TextStyle::Monospace)
            .horizontal_align(Align::RIGHT)
            .desired_width(presentation.field_width)
            .frame(edit_frame),
    );

    if response.changed() {
        sanitize_numeric_input(&mut state.input);
        state.invalid_input = false;
    }

    let escape_pressed = response.has_focus() && ui.input(|input| input.key_pressed(Key::Escape));
    if escape_pressed {
        state.input = format_numeric_value(param.modulated_plain_value(), presentation);
        state.invalid_input = false;
        ui.memory_mut(|memory| memory.surrender_focus(edit_id));
        return;
    }

    let enter_pressed = response.has_focus() && ui.input(|input| input.key_pressed(Key::Enter));
    if enter_pressed {
        if commit_numeric_value(param, setter, &state.input) {
            state.invalid_input = false;
            ui.memory_mut(|memory| memory.surrender_focus(edit_id));
        } else {
            state.invalid_input = true;
        }
    } else if response.lost_focus() {
        if !commit_numeric_value(param, setter, &state.input) {
            state.input = format_numeric_value(param.modulated_plain_value(), presentation);
        }
        state.invalid_input = false;
    }
}

fn parameter_track(
    ui: &mut egui::Ui,
    param: &FloatParam,
    setter: &ParamSetter,
    state: &mut ParameterControlState,
) {
    let desired_size = vec2(ui.available_width(), TRACK_HEIGHT);
    let response = ui.allocate_response(desired_size, Sense::click_and_drag());
    if response.hovered() || state.dragging {
        ui.output_mut(|output| output.cursor_icon = CursorIcon::PointingHand);
    }

    let reset_requested = response.double_clicked()
        || (response.clicked() && ui.input(|input| input.modifiers.command));
    if reset_requested {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, param.default_plain_value());
        setter.end_set_parameter(param);
        state.dragging = false;
    } else {
        if response.drag_started() {
            setter.begin_set_parameter(param);
            state.dragging = true;
            state.drag_start_normalized = param.modulated_normalized_value();
            state.drag_start_x = response
                .interact_pointer_pos()
                .map_or(response.rect.left(), |position| position.x);
        }

        if state.dragging && response.dragged() {
            if let Some(position) = response.interact_pointer_pos() {
                let normalized = if ui.input(|input| input.modifiers.shift) {
                    state.drag_start_normalized + (position.x - state.drag_start_x) * 0.0015
                } else {
                    normalized_from_pointer(response.rect, position.x)
                };
                set_normalized_value(param, setter, normalized);
            }
        }

        if state.dragging && response.drag_stopped() {
            if let Some(position) = response.interact_pointer_pos() {
                let normalized = if ui.input(|input| input.modifiers.shift) {
                    state.drag_start_normalized + (position.x - state.drag_start_x) * 0.0015
                } else {
                    normalized_from_pointer(response.rect, position.x)
                };
                set_normalized_value(param, setter, normalized);
            }
            setter.end_set_parameter(param);
            state.dragging = false;
        } else if response.clicked() {
            if let Some(position) = response.interact_pointer_pos() {
                setter.begin_set_parameter(param);
                set_normalized_value(
                    param,
                    setter,
                    normalized_from_pointer(response.rect, position.x),
                );
                setter.end_set_parameter(param);
            }
        }
    }

    if ui.is_rect_visible(response.rect) {
        let normalized = param.modulated_normalized_value().clamp(0.0, 1.0);
        let accent = if response.hovered() || state.dragging {
            ACCENT_HOVERED
        } else {
            ACCENT
        };
        ui.painter().rect_filled(
            response.rect,
            CornerRadius::same(5),
            TRACK_BACKGROUND,
        );

        if normalized > 0.0 {
            let filled_right = egui::lerp(response.rect.x_range(), normalized);
            let filled_rect = Rect::from_min_max(
                response.rect.min,
                pos2(filled_right, response.rect.bottom()),
            );
            ui.painter()
                .rect_filled(filled_rect, CornerRadius::same(5), accent);
        }

        let thumb_x = egui::lerp(response.rect.x_range(), normalized);
        ui.painter()
            .circle_filled(pos2(thumb_x, response.rect.center().y), 5.0, PRIMARY_TEXT);
        ui.painter().circle_stroke(
            pos2(thumb_x, response.rect.center().y),
            5.0,
            Stroke::new(1.5, accent),
        );
    }
}

fn normalized_from_pointer(rect: Rect, pointer_x: f32) -> f32 {
    egui::remap_clamp(pointer_x, rect.x_range(), 0.0..=1.0)
}

fn set_normalized_value(param: &FloatParam, setter: &ParamSetter, normalized: f32) {
    let plain = param.preview_plain(normalized.clamp(0.0, 1.0));
    if plain != param.modulated_plain_value() {
        setter.set_parameter(param, plain);
    }
}

fn commit_numeric_value(param: &FloatParam, setter: &ParamSetter, input: &str) -> bool {
    let Some(plain) = numeric_plain_value(param, input) else {
        return false;
    };

    setter.begin_set_parameter(param);
    if plain != param.modulated_plain_value() {
        setter.set_parameter(param, plain);
    }
    setter.end_set_parameter(param);
    true
}

fn numeric_plain_value(param: &FloatParam, input: &str) -> Option<f32> {
    let normalized = param.string_to_normalized_value(input)?;
    Some(param.preview_plain(normalized))
}

fn format_numeric_value(value: f32, presentation: ValuePresentation) -> String {
    let displayed = value * presentation.scale;
    format!("{displayed:.decimals$}", decimals = presentation.decimals)
}

fn sanitize_numeric_input(input: &mut String) {
    let mut decimal_seen = false;
    input.retain(|character| {
        if character.is_ascii_digit() {
            true
        } else if character == '.' && !decimal_seen {
            decimal_seen = true;
            true
        } else {
            false
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_value_formatting_is_concise_and_unit_free() {
        assert_eq!(
            format_numeric_value(57.49855, ATTENUATION_PRESENTATION),
            "57.5"
        );
        assert_eq!(format_numeric_value(0.86, MIX_PRESENTATION), "86");
        assert!(!format_numeric_value(100.0, ATTENUATION_PRESENTATION).contains("dB"));
        assert!(!format_numeric_value(1.0, MIX_PRESENTATION).contains('%'));
    }

    #[test]
    fn gui_value_input_keeps_only_a_single_decimal_number() {
        let mut attenuation = String::from("57.49855 dB");
        sanitize_numeric_input(&mut attenuation);
        assert_eq!(attenuation, "57.49855");

        let mut mix = String::from("86 %");
        sanitize_numeric_input(&mut mix);
        assert_eq!(mix, "86");

        let mut repeated_decimal = String::from("8.6.5");
        sanitize_numeric_input(&mut repeated_decimal);
        assert_eq!(repeated_decimal, "8.65");
    }

    #[test]
    fn gui_numeric_conversion_uses_parameter_ranges_and_formatters() {
        let params = DeepFilterParams::default();

        let attenuation = numeric_plain_value(&params.atten_lim, "57.49855").unwrap();
        assert!((attenuation - 57.49855).abs() < f32::EPSILON);

        let mix = numeric_plain_value(&params.mix, "86").unwrap();
        assert!((mix - 0.86).abs() < f32::EPSILON);

        assert_eq!(numeric_plain_value(&params.atten_lim, "999"), Some(100.0));
        assert_eq!(numeric_plain_value(&params.mix, "999"), Some(1.0));
        assert_eq!(numeric_plain_value(&params.mix, ""), None);
        assert_eq!(numeric_plain_value(&params.atten_lim, "."), None);
    }
}
