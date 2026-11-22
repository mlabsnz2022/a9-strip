use nih_plug::prelude::*;
use nih_plug_egui::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use std::f32::consts::PI;

pub const BG_CHASSIS: Color32 = Color32::from_rgb(20, 21, 24);
pub const BG_PANEL: Color32 = Color32::from_rgb(28, 30, 35);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(46, 49, 58);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(232, 235, 240);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(142, 148, 160);
pub const PIONEER_BLUE: Color32 = Color32::from_rgb(40, 168, 255);
pub const PIONEER_AMBER: Color32 = Color32::from_rgb(255, 168, 38);
pub const PIONEER_RED: Color32 = Color32::from_rgb(245, 56, 56);
pub const PIONEER_GREEN: Color32 = Color32::from_rgb(46, 214, 102);

#[derive(Clone, Copy)]
pub enum KnobStyle {
    /// Small grey-capped Trim / Parameter knob
    Utility,
    /// Standard Pioneer DJM-A9 EQ/ISO knob (dark body, crisp white indicator)
    Eq,
    /// Large signature silver-ringed Sound Color FX Filter knob
    ColorFilter,
}

/// Persistent per-drag state for the DJM-A9 Center Lock mechanism.
#[derive(Clone, Copy, Default)]
struct CenterLockDragState {
    /// Side the knob started on when drag began: -1 (left), 0 (center), +1 (right)
    start_side: i8,
    /// Whether the knob has hit and locked at 12 o'clock during this drag
    locked_at_center: bool,
}

/// Draws a Pioneer DJM-A9 styled rotary knob bound to a `FloatParam`.
#[allow(clippy::too_many_arguments)]
pub fn pioneer_knob(
    ui: &mut egui::Ui,
    param: &FloatParam,
    setter: &ParamSetter,
    label: &str,
    left_mark: &str,
    right_mark: &str,
    diameter: f32,
    bipolar: bool,
    style: KnobStyle,
    center_lock: bool,
) {
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = 1.5;
        ui.label(
            egui::RichText::new(label)
                .font(FontId::proportional(10.0))
                .strong()
                .color(TEXT_PRIMARY),
        );

        let total_w = (diameter + 26.0).max(68.0);
        let total_h = diameter + 10.0;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(total_w, total_h), Sense::click_and_drag());
        let center = Pos2::new(rect.center().x, rect.top() + diameter * 0.5 + 3.0);
        let radius = diameter * 0.5;

        let lock_id = response.id.with("center_lock_state");

        // Double-click resets to default (12 o'clock)
        if response.double_clicked() {
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, param.default_normalized_value());
            setter.end_set_parameter(param);
        }

        if response.drag_started() {
            setter.begin_set_parameter(param);
            if center_lock && bipolar {
                let norm = param.modulated_normalized_value();
                let start_side = if norm < 0.495 {
                    -1
                } else if norm > 0.505 {
                    1
                } else {
                    0
                };
                ui.memory_mut(|m| {
                    m.data.insert_temp(
                        lock_id,
                        CenterLockDragState {
                            start_side,
                            locked_at_center: false,
                        },
                    );
                });
            }
        }

        if response.dragged() {
            let fine = ui.input(|i| i.modifiers.shift);
            let sensitivity = if fine { 0.0012 } else { 0.0045 };
            let delta = -response.drag_delta().y * sensitivity;
            let cur = param.modulated_normalized_value();
            let mut next = (cur + delta).clamp(0.0, 1.0);

            if center_lock && bipolar {
                let mut state = ui
                    .memory(|m| m.data.get_temp::<CenterLockDragState>(lock_id))
                    .unwrap_or_default();

                if state.locked_at_center {
                    next = 0.5;
                } else if (state.start_side == -1 && next >= 0.5)
                    || (state.start_side == 1 && next <= 0.5)
                {
                    next = 0.5;
                    state.locked_at_center = true;
                    ui.memory_mut(|m| m.data.insert_temp(lock_id, state));
                }
            }

            setter.set_parameter_normalized(param, next);
        }

        if response.drag_stopped() {
            setter.end_set_parameter(param);
        }

        // Scroll wheel support when hovered
        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.1 {
                let step = if ui.input(|i| i.modifiers.shift) {
                    0.005
                } else {
                    0.02
                };
                let cur = param.modulated_normalized_value();
                let next = (cur + scroll.signum() * step).clamp(0.0, 1.0);
                setter.begin_set_parameter(param);
                setter.set_parameter_normalized(param, next);
                setter.end_set_parameter(param);
            }
        }

        let norm = param.modulated_normalized_value();
        let painter = ui.painter();

        // Sweep angle: -135 deg (7:30) to +135 deg (4:30), with 0 deg at 12 o'clock (-PI/2 in screen coords)
        let start_angle = -0.75 * PI - PI * 0.5;
        let end_angle = 0.75 * PI - PI * 0.5;
        let val_angle = start_angle + norm * (end_angle - start_angle);
        let center_angle = -PI * 0.5;

        // Draw scale ticks around the knob
        let tick_outer = radius + 6.0;
        let tick_inner = radius + 2.5;
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let a = start_angle + t * (end_angle - start_angle);
            let is_center = i == 5;
            let len = if is_center { 5.0 } else { 3.0 };
            let col = if is_center {
                TEXT_PRIMARY
            } else {
                Color32::from_rgb(95, 100, 112)
            };
            let p1 = center + Vec2::new(a.cos(), a.sin()) * (tick_inner + if is_center { -0.5 } else { 0.5 });
            let p2 = center + Vec2::new(a.cos(), a.sin()) * (tick_inner + len);
            painter.line_segment([p1, p2], Stroke::new(if is_center { 1.6_f32 } else { 1.0_f32 }, col));
        }

        // Draw active value arc
        let arc_r = radius + 3.5;
        let arc_color = match style {
            KnobStyle::ColorFilter => {
                if (norm - 0.5).abs() <= 0.008 {
                    Color32::from_rgb(70, 75, 86)
                } else if norm < 0.5 {
                    PIONEER_BLUE
                } else {
                    PIONEER_AMBER
                }
            }
            KnobStyle::Eq => PIONEER_BLUE,
            KnobStyle::Utility => Color32::from_rgb(175, 182, 196),
        };

        let (a_from, a_to) = if bipolar {
            (center_angle.min(val_angle), center_angle.max(val_angle))
        } else {
            (start_angle, val_angle)
        };
        if (a_to - a_from).abs() > 0.02 {
            let steps = 28;
            let mut pts = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let a = a_from + (i as f32 / steps as f32) * (a_to - a_from);
                pts.push(center + Vec2::new(a.cos(), a.sin()) * arc_r);
            }
            painter.add(egui::Shape::line(pts, Stroke::new(2.2_f32, arc_color)));
        }

        // Outer knob shadow & knurled rim
        painter.circle_filled(
            center + Vec2::new(0.0, 2.0),
            radius,
            Color32::from_rgba_unmultiplied(0, 0, 0, 130),
        );

        match style {
            KnobStyle::ColorFilter => {
                // Signature DJM-A9 metallic silver outer ring + dark rubber center
                painter.circle_filled(center, radius, Color32::from_rgb(152, 158, 170));
                painter.circle_stroke(
                    center,
                    radius,
                    Stroke::new(1.2_f32, Color32::from_rgb(205, 210, 220)),
                );
                painter.circle_filled(center, radius * 0.78, Color32::from_rgb(26, 28, 33));
                painter.circle_stroke(
                    center,
                    radius * 0.78,
                    Stroke::new(1.0_f32, Color32::from_rgb(60, 64, 74)),
                );
            }
            KnobStyle::Eq => {
                painter.circle_filled(center, radius, Color32::from_rgb(36, 39, 46));
                painter.circle_stroke(
                    center,
                    radius,
                    Stroke::new(1.2_f32, Color32::from_rgb(76, 80, 92)),
                );
                painter.circle_filled(center, radius * 0.76, Color32::from_rgb(22, 24, 28));
            }
            KnobStyle::Utility => {
                painter.circle_filled(center, radius, Color32::from_rgb(56, 60, 68));
                painter.circle_stroke(
                    center,
                    radius,
                    Stroke::new(1.0_f32, Color32::from_rgb(102, 108, 120)),
                );
                painter.circle_filled(center, radius * 0.72, Color32::from_rgb(38, 41, 48));
            }
        }

        // Crisp white pointer indicator line
        let ptr_start = center + Vec2::new(val_angle.cos(), val_angle.sin()) * (radius * 0.25);
        let ptr_end = center + Vec2::new(val_angle.cos(), val_angle.sin()) * (radius * 0.90);
        painter.line_segment(
            [ptr_start, ptr_end],
            Stroke::new(2.4_f32, Color32::from_rgb(245, 247, 250)),
        );

        // Left / Right silk-screened markings
        let mark_y = center.y + tick_outer * 0.78;
        if !left_mark.is_empty() {
            painter.text(
                Pos2::new(center.x - radius - 4.0, mark_y),
                Align2::RIGHT_CENTER,
                left_mark,
                FontId::proportional(8.5),
                TEXT_MUTED,
            );
        }
        if !right_mark.is_empty() {
            painter.text(
                Pos2::new(center.x + radius + 4.0, mark_y),
                Align2::LEFT_CENTER,
                right_mark,
                FontId::proportional(8.5),
                TEXT_MUTED,
            );
        }

        // Live readout value below knob
        let readout = param.to_string();
        ui.label(
            egui::RichText::new(readout)
                .font(FontId::monospace(9.5))
                .color(if response.dragged() || response.hovered() {
                    TEXT_PRIMARY
                } else {
                    TEXT_MUTED
                }),
        );
    });
}

/// Pioneer-style illuminated hardware push button bound to a `BoolParam`.
pub fn illuminated_button(
    ui: &mut egui::Ui,
    param: &BoolParam,
    setter: &ParamSetter,
    label: &str,
    active_color: Color32,
    size: Vec2,
) {
    let is_on = param.modulated_plain_value();
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if response.clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, !is_on);
        setter.end_set_parameter(param);
    }

    let painter = ui.painter();
    let bg = if is_on {
        Color32::from_rgba_unmultiplied(active_color.r(), active_color.g(), active_color.b(), 42)
    } else {
        Color32::from_rgb(26, 28, 33)
    };
    let border = if is_on {
        active_color
    } else if response.hovered() {
        Color32::from_rgb(100, 106, 120)
    } else {
        BORDER_SUBTLE
    };
    let text_col = if is_on { active_color } else { TEXT_MUTED };

    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.2_f32, border), StrokeKind::Inside);

    // Small status LED dot on left
    let led_center = Pos2::new(rect.left() + 10.0, rect.center().y);
    let led_col = if is_on {
        active_color
    } else {
        Color32::from_rgb(48, 52, 60)
    };
    painter.circle_filled(led_center, 3.0, led_col);

    painter.text(
        Pos2::new(rect.center().x + 4.0, rect.center().y),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(10.0),
        text_col,
    );
}

/// Pioneer DJM-A9 vertical 15-segment LED level meter + clip LED.
pub fn led_level_meter(ui: &mut egui::Ui, level_db: f32, clip_activity: f32, height: f32) {
    let width = 26.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, 3.0, Color32::from_rgb(12, 13, 16));
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, BORDER_SUBTLE), StrokeKind::Inside);

    // 15 segments from -24 dB to +6 dB (top segment = CLIP)
    let thresholds: [f32; 15] = [
        -24.0, -21.0, -18.0, -15.0, -12.0, -9.0, -6.0, -4.0, -2.0, -1.0, 0.0, 1.0, 2.0, 4.0, 6.0,
    ];

    let pad = 4.0;
    let gap = 2.0;
    let seg_count = thresholds.len() as f32;
    let seg_h = ((rect.height() - pad * 2.0) - gap * (seg_count - 1.0)) / seg_count;
    let seg_w = rect.width() - pad * 2.0;

    for (i, &thresh) in thresholds.iter().enumerate() {
        let rev_i = (thresholds.len() - 1 - i) as f32;
        let y = rect.top() + pad + rev_i * (seg_h + gap);
        let seg_rect = Rect::from_min_size(Pos2::new(rect.left() + pad, y), Vec2::new(seg_w, seg_h));

        let lit = if i == thresholds.len() - 1 {
            level_db >= thresh || clip_activity > 0.08
        } else {
            level_db >= thresh
        };

        let on_color = if i >= 13 {
            PIONEER_RED
        } else if i >= 10 {
            PIONEER_AMBER
        } else {
            PIONEER_GREEN
        };

        let off_color = Color32::from_rgb(28, 30, 36);
        painter.rect_filled(seg_rect, 1.5, if lit { on_color } else { off_color });
    }
}

/// Pioneer DJM-A9 vertical Channel Fader with P-Lock style cap and silk-screened scale ticks.
pub fn pioneer_fader(
    ui: &mut egui::Ui,
    param: &FloatParam,
    setter: &ParamSetter,
    height: f32,
) {
    let width = 46.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());

    let cap_h = 26.0;
    let cap_w = 32.0;
    let track_top = rect.top() + cap_h * 0.5 + 4.0;
    let track_bottom = rect.bottom() - cap_h * 0.5 - 4.0;
    let track_h = (track_bottom - track_top).max(1.0);

    // Double-click resets to full up (0.0 dB)
    if response.double_clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter_normalized(param, param.default_normalized_value());
        setter.end_set_parameter(param);
    }

    if response.drag_started() {
        setter.begin_set_parameter(param);
    }

    if response.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        let next = if fine {
            let cur = param.modulated_normalized_value();
            (cur - response.drag_delta().y / (track_h * 4.0)).clamp(0.0, 1.0)
        } else if let Some(pointer) = response.interact_pointer_pos() {
            (1.0 - (pointer.y - track_top) / track_h).clamp(0.0, 1.0)
        } else {
            let cur = param.modulated_normalized_value();
            (cur - response.drag_delta().y / track_h).clamp(0.0, 1.0)
        };
        setter.set_parameter_normalized(param, next);
    } else if response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let next = (1.0 - (pointer.y - track_top) / track_h).clamp(0.0, 1.0);
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, next);
            setter.end_set_parameter(param);
        }
    }

    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }

    // Scroll wheel support when hovered
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.1 {
            let step = if ui.input(|i| i.modifiers.shift) {
                0.005
            } else {
                0.025
            };
            let cur = param.modulated_normalized_value();
            let next = (cur + scroll.signum() * step).clamp(0.0, 1.0);
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, next);
            setter.end_set_parameter(param);
        }
    }

    let norm = param.modulated_normalized_value();
    let painter = ui.painter();
    let cx = rect.center().x;

    // Recessed fader panel background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(16, 17, 21));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, BORDER_SUBTLE), StrokeKind::Inside);

    // Pioneer-style horizontal scale ticks (0..10) on both sides of the slot
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        let y = track_bottom - t * track_h;
        let major = i % 2 == 0;
        let tick_w = if major { 8.0 } else { 4.5 };
        let col = if i == 10 || i == 0 {
            TEXT_PRIMARY
        } else if major {
            Color32::from_rgb(130, 136, 148)
        } else {
            Color32::from_rgb(68, 72, 82)
        };
        let stroke = Stroke::new(if major { 1.2_f32 } else { 0.8_f32 }, col);
        painter.line_segment([Pos2::new(cx - 5.0 - tick_w, y), Pos2::new(cx - 5.0, y)], stroke);
        painter.line_segment([Pos2::new(cx + 5.0, y), Pos2::new(cx + 5.0 + tick_w, y)], stroke);
    }

    // Recessed center slot
    let slot_rect = Rect::from_min_max(
        Pos2::new(cx - 2.5, track_top),
        Pos2::new(cx + 2.5, track_bottom),
    );
    painter.rect_filled(slot_rect, 2.0, Color32::from_rgb(8, 9, 11));

    // Active blue level line from bottom up to fader cap
    let cap_y = track_bottom - norm * track_h;
    if norm > 0.005 {
        let fill_rect = Rect::from_min_max(
            Pos2::new(cx - 1.5, cap_y),
            Pos2::new(cx + 1.5, track_bottom),
        );
        painter.rect_filled(fill_rect, 1.0, PIONEER_BLUE);
    }

    // Sculpted Pioneer P-Lock fader cap
    let cap_rect = Rect::from_center_size(Pos2::new(cx, cap_y), Vec2::new(cap_w, cap_h));
    // Drop shadow
    painter.rect_filled(
        cap_rect.translate(Vec2::new(0.0, 2.5)),
        3.0,
        Color32::from_rgba_unmultiplied(0, 0, 0, 150),
    );
    // Main cap body
    painter.rect_filled(cap_rect, 3.0, Color32::from_rgb(38, 41, 48));
    painter.rect_stroke(
        cap_rect,
        3.0,
        Stroke::new(
            1.2_f32,
            if response.dragged() || response.hovered() {
                Color32::from_rgb(130, 138, 152)
            } else {
                Color32::from_rgb(86, 92, 104)
            },
        ),
        StrokeKind::Inside,
    );
    // Top & bottom sculpted bevel highlights
    let inner_cap = cap_rect.shrink2(Vec2::new(3.0, 4.0));
    painter.rect_filled(inner_cap, 1.5, Color32::from_rgb(24, 26, 31));

    // Crisp white center hairline (signature Pioneer fader indicator)
    painter.line_segment(
        [Pos2::new(cap_rect.left() + 3.0, cap_y), Pos2::new(cap_rect.right() - 3.0, cap_y)],
        Stroke::new(2.2_f32, Color32::from_rgb(242, 245, 250)),
    );
}
