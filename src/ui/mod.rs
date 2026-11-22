pub mod widgets;

use crate::dsp::meter::MeterState;
use crate::params::{A9StripParams, EqMode};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{
    self, Color32, ColorImage, CornerRadius, FontId, Frame, Margin, Stroke, TextureHandle,
    TextureOptions, Vec2,
};
use nih_plug_egui::create_egui_editor;
use std::sync::Arc;
use widgets::{
    illuminated_button, led_level_meter, pioneer_fader, pioneer_knob, KnobStyle, BG_CHASSIS,
    BG_PANEL, BORDER_SUBTLE, PIONEER_AMBER, PIONEER_BLUE, PIONEER_RED, TEXT_MUTED, TEXT_PRIMARY,
};

const LOGO_PNG_BYTES: &[u8] = include_bytes!("../../assets/mlabs_logo.png");

fn load_logo_texture(ctx: &egui::Context) -> Option<TextureHandle> {
    let mut decoder = png::Decoder::new(LOGO_PNG_BYTES);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let bytes = &buf[..info.buffer_size()];

    let image = match info.color_type {
        png::ColorType::Rgba => ColorImage::from_rgba_unmultiplied(
            [info.width as usize, info.height as usize],
            bytes,
        ),
        png::ColorType::Rgb => {
            ColorImage::from_rgb([info.width as usize, info.height as usize], bytes)
        }
        _ => return None,
    };

    Some(ctx.load_texture("mlabs_logo", image, TextureOptions::LINEAR))
}

pub fn create(
    params: Arc<A9StripParams>,
    meter: Arc<MeterState>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        None::<TextureHandle>,
        |egui_ctx, state| {
            *state = load_logo_texture(egui_ctx);
        },
        move |egui_ctx, setter, logo_tex| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = BG_CHASSIS;
            egui_ctx.set_visuals(visuals);

            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(BG_CHASSIS).inner_margin(Margin::same(8)))
                .show(egui_ctx, |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;

                    // Header: Pioneer DJM-A9 style channel badge + m-labs logo
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("A9 STRIP")
                                .font(FontId::proportional(13.0))
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(tex) = logo_tex.as_ref() {
                                let tex_size = tex.size_vec2();
                                let target_h = 18.0;
                                let target_w = target_h * (tex_size.x / tex_size.y.max(1.0));
                                ui.add(
                                    egui::Image::new((tex.id(), Vec2::new(target_w, target_h)))
                                        .tint(Color32::from_rgb(235, 238, 244)),
                                );
                            }
                        });
                    });
                    ui.separator();

                    // Main two-column body: Left = Controls, Right = Segmented LED Meter
                    ui.horizontal_top(|ui| {
                        // Left column: Trim, EQ/ISO, Sound Color FX Filter, Soft Clip
                        ui.vertical(|ui| {
                            ui.set_width(210.0);
                            ui.spacing_mut().item_spacing.y = 5.0;

                            // --- 1. TRIM SECTION ---
                            Frame::NONE
                                .fill(BG_PANEL)
                                .corner_radius(CornerRadius::same(5))
                                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                .inner_margin(Margin::symmetric(8, 5))
                                .show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        pioneer_knob(
                                            ui,
                                            &params.trim,
                                            setter,
                                            "TRIM",
                                            "-∞",
                                            "+9",
                                            34.0,
                                            true,
                                            KnobStyle::Utility,
                                            false,
                                        );
                                    });
                                });

                            // --- 2. 3-BAND EQ / ISOLATOR SECTION ---
                            Frame::NONE
                                .fill(BG_PANEL)
                                .corner_radius(CornerRadius::same(5))
                                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                .inner_margin(Margin::symmetric(8, 5))
                                .show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.spacing_mut().item_spacing.y = 3.0;

                                        // EQ / ISO mode switcher
                                        let cur_mode = params.eq_mode.value();
                                        ui.horizontal(|ui| {
                                            ui.add_space(28.0);
                                            for (mode, title) in
                                                [(EqMode::Isolator, "ISO"), (EqMode::Eq, "EQ")]
                                            {
                                                let active = cur_mode == mode;
                                                let btn = egui::Button::new(
                                                    egui::RichText::new(title)
                                                        .font(FontId::proportional(9.5))
                                                        .strong()
                                                        .color(if active {
                                                            Color32::BLACK
                                                        } else {
                                                            TEXT_MUTED
                                                        }),
                                                )
                                                .fill(if active {
                                                    PIONEER_BLUE
                                                } else {
                                                    Color32::from_rgb(22, 24, 28)
                                                })
                                                .stroke(Stroke::new(
                                                    1.0_f32,
                                                    if active { PIONEER_BLUE } else { BORDER_SUBTLE },
                                                ))
                                                .min_size(Vec2::new(62.0, 17.0));

                                                if ui.add(btn).clicked() && !active {
                                                    setter.begin_set_parameter(&params.eq_mode);
                                                    setter.set_parameter(&params.eq_mode, mode);
                                                    setter.end_set_parameter(&params.eq_mode);
                                                }
                                            }
                                        });

                                        let cut_mark = match cur_mode {
                                            EqMode::Isolator => "-∞",
                                            EqMode::Eq => "-26",
                                        };

                                        pioneer_knob(
                                            ui,
                                            &params.eq_hi,
                                            setter,
                                            "HI",
                                            cut_mark,
                                            "+6",
                                            40.0,
                                            true,
                                            KnobStyle::Eq,
                                            false,
                                        );
                                        pioneer_knob(
                                            ui,
                                            &params.eq_mid,
                                            setter,
                                            "MID",
                                            cut_mark,
                                            "+6",
                                            40.0,
                                            true,
                                            KnobStyle::Eq,
                                            false,
                                        );
                                        pioneer_knob(
                                            ui,
                                            &params.eq_low,
                                            setter,
                                            "LOW",
                                            cut_mark,
                                            "+6",
                                            40.0,
                                            true,
                                            KnobStyle::Eq,
                                            false,
                                        );
                                    });
                                });

                            // --- 3. SOUND COLOR FX: FILTER SECTION ---
                            Frame::NONE
                                .fill(BG_PANEL)
                                .corner_radius(CornerRadius::same(5))
                                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                .inner_margin(Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.spacing_mut().item_spacing.y = 4.0;

                                        ui.label(
                                            egui::RichText::new("SOUND COLOR FX")
                                                .font(FontId::proportional(9.0))
                                                .strong()
                                                .color(TEXT_MUTED),
                                        );

                                        // Filter On/Off + Center Lock row
                                        ui.horizontal(|ui| {
                                            illuminated_button(
                                                ui,
                                                &params.filter_on,
                                                setter,
                                                "FILTER",
                                                PIONEER_BLUE,
                                                Vec2::new(88.0, 20.0),
                                            );
                                            ui.add_space(6.0);
                                            illuminated_button(
                                                ui,
                                                &params.center_lock,
                                                setter,
                                                "CTR LOCK",
                                                PIONEER_AMBER,
                                                Vec2::new(96.0, 20.0),
                                            );
                                        });

                                        // Signature large center-zero FILTER knob
                                        let lock_enabled = params.center_lock.modulated_plain_value();
                                        pioneer_knob(
                                            ui,
                                            &params.filter,
                                            setter,
                                            "COLOR / FILTER",
                                            "LPF",
                                            "HPF",
                                            50.0,
                                            true,
                                            KnobStyle::ColorFilter,
                                            lock_enabled,
                                        );

                                        // PARAMETER (Resonance) knob
                                        pioneer_knob(
                                            ui,
                                            &params.filter_res,
                                            setter,
                                            "PARAMETER (RES)",
                                            "MIN",
                                            "MAX",
                                            34.0,
                                            false,
                                            KnobStyle::Utility,
                                            false,
                                        );
                                    });
                                });

                            // --- 4. OUTPUT SOFT CLIPPER ---
                            ui.horizontal(|ui| {
                                illuminated_button(
                                    ui,
                                    &params.soft_clip,
                                    setter,
                                    "SOFT CLIP",
                                    PIONEER_RED,
                                    Vec2::new(112.0, 22.0),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let clip_act = meter.read_clip();
                                        let col = if clip_act > 0.05 {
                                            PIONEER_RED
                                        } else {
                                            TEXT_MUTED
                                        };
                                        ui.label(
                                            egui::RichText::new(if clip_act > 0.05 {
                                                "HEADROOM"
                                            } else {
                                                "CLEAN"
                                            })
                                            .font(FontId::monospace(8.5))
                                            .color(col),
                                        );
                                    },
                                );
                            });
                        });

                        ui.add_space(6.0);

                        // Middle column: Pioneer vertical Channel Fader
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("LEVEL")
                                    .font(FontId::proportional(7.5))
                                    .strong()
                                    .color(TEXT_MUTED),
                            );
                            let fader_h = (ui.available_height() - 18.0).clamp(200.0, 620.0);
                            pioneer_fader(ui, &params.fader, setter, fader_h);
                            ui.label(
                                egui::RichText::new(params.fader.to_string())
                                    .font(FontId::monospace(8.5))
                                    .color(TEXT_MUTED),
                            );
                        });

                        ui.add_space(6.0);

                        // Right column: Pioneer 15-segment LED meter sized to fit the window
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("CLIP")
                                    .font(FontId::proportional(7.5))
                                    .strong()
                                    .color(if meter.read_clip() > 0.05 {
                                        PIONEER_RED
                                    } else {
                                        TEXT_MUTED
                                    }),
                            );
                            let meter_h = (ui.available_height() - 18.0).clamp(200.0, 620.0);
                            led_level_meter(ui, meter.read_db(), meter.read_clip(), meter_h);
                            ui.label(
                                egui::RichText::new("dB")
                                    .font(FontId::monospace(8.5))
                                    .color(TEXT_MUTED),
                            );
                        });
                    });
                });
        },
    )
}
