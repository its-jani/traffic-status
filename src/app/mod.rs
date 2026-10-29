pub mod session;

use crossbeam_channel::Receiver;
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use std::time::{Duration, Instant};

use crate::app::session::{AppEffect, SessionStore};
use crate::types::{IpcCommand, LightState};

pub struct TrafficStatusApp {
    rx: Receiver<IpcCommand>,
    store: SessionStore,
    pulse_phase: f32,
    last_frame_time: Instant,
    last_width: f32,
}

impl TrafficStatusApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, rx: Receiver<IpcCommand>) -> Self {
        Self {
            rx,
            store: SessionStore::with_default_session(),
            pulse_phase: 0.0,
            last_frame_time: Instant::now(),
            last_width: 86.0,
        }
    }

    fn process_ipc_events(&mut self, ctx: &egui::Context) {
        while let Ok(cmd) = self.rx.try_recv() {
            for effect in self.store.apply(cmd) {
                match effect {
                    AppEffect::Repaint => ctx.request_repaint(),
                    AppEffect::Unminimize => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false))
                    }
                    AppEffect::FocusWindow => ctx.send_viewport_cmd(egui::ViewportCommand::Focus),
                    AppEffect::CloseWindow => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    AppEffect::ExitProcess => std::process::exit(0),
                }
            }
        }
    }

    fn has_active_animation(&self) -> bool {
        self.store.has_active_animation()
    }
}

impl eframe::App for TrafficStatusApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_ipc_events(ctx);

        let has_multiple_sessions = self.store.order().len() > 1;
        let needed_width = if has_multiple_sessions { 118.0 } else { 86.0 };
        let needed_height = 236.0;

        if (needed_width - self.last_width).abs() > 0.5 {
            self.last_width = needed_width;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                needed_width,
                needed_height,
            )));
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        if self.has_active_animation() {
            self.pulse_phase = (self.pulse_phase + dt * 4.2) % std::f32::consts::TAU;
            ctx.request_repaint_after(Duration::from_millis(33));
        } else {
            ctx.request_repaint_after(Duration::from_millis(1000));
        }

        self.store.ensure_active();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                // Drag the entire widget by clicking & dragging anywhere
                let full_rect = ui.max_rect();
                let drag_response =
                    ui.interact(full_rect, ui.id().with("window_drag"), egui::Sense::drag());
                if drag_response.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                let is_hovered = drag_response.hovered();

                ui.horizontal(|ui| {
                    // Multi-session side tabs column
                    if has_multiple_sessions {
                        let order_snapshot = self.store.order().to_vec();
                        ui.vertical(|ui| {
                            ui.add_space(20.0);
                            for (idx, session_id) in order_snapshot.iter().enumerate() {
                                if let Some(sess) = self.store.sessions().get(session_id) {
                                    let is_selected =
                                        self.store.active_id() == Some(session_id.as_str());
                                    let tab_dot_color = match sess.state {
                                        LightState::Red => Color32::from_rgb(255, 60, 60),
                                        LightState::Yellow => Color32::from_rgb(255, 210, 40),
                                        LightState::Green => Color32::from_rgb(50, 230, 110),
                                        LightState::Off => Color32::from_rgb(80, 85, 100),
                                    };

                                    let tab_btn = egui::Button::new(
                                        egui::RichText::new(format!("{}", idx + 1))
                                            .size(10.0)
                                            .color(if is_selected {
                                                Color32::WHITE
                                            } else {
                                                Color32::from_rgb(140, 145, 165)
                                            })
                                            .strong(),
                                    )
                                    .fill(if is_selected {
                                        Color32::from_rgba_unmultiplied(35, 40, 55, 240)
                                    } else {
                                        Color32::from_rgba_unmultiplied(16, 18, 24, 180)
                                    })
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        if is_selected {
                                            tab_dot_color
                                        } else {
                                            Color32::from_rgba_unmultiplied(255, 255, 255, 20)
                                        },
                                    ))
                                    .rounding(Rounding::same(8.0));

                                    let hover_text = format!(
                                        "{}: {}\nStatus: {}",
                                        sess.label,
                                        sess.state.display_name(),
                                        sess.message.as_deref().unwrap_or("No message")
                                    );

                                    if ui.add(tab_btn).on_hover_text(hover_text).clicked() {
                                        self.store.select(session_id);
                                    }

                                    ui.add_space(3.0);
                                }
                            }
                        });
                    }

                    // Authentic Traffic Light Physical Housing
                    let active_session = self.store.active_session().cloned();
                    let current_state = active_session
                        .as_ref()
                        .map(|s| s.state)
                        .unwrap_or(LightState::Off);

                    let (capsule_rect, _) =
                        ui.allocate_exact_size(Vec2::new(84.0, 232.0), egui::Sense::hover());

                    let painter = ui.painter_at(capsule_rect);

                    render_authentic_traffic_light(
                        &painter,
                        capsule_rect,
                        current_state,
                        self.pulse_phase,
                    );

                    // Hover tooltips for active session
                    if let Some(session) = &active_session {
                        let elapsed = session.last_updated.elapsed().as_secs();
                        let time_str = if elapsed < 60 {
                            format!("{elapsed}s ago")
                        } else {
                            format!("{}m ago", elapsed / 60)
                        };
                        let tooltip_text = format!(
                            "{}\nSession: {}\nUpdated: {}\nMessage: {}",
                            session.state.display_name(),
                            session.label,
                            time_str,
                            session.message.as_deref().unwrap_or("Ready")
                        );
                        drag_response.on_hover_text(tooltip_text);
                    } else {
                        drag_response.on_hover_text("Traffic Status (Standby)");
                    }

                    // Discreet hover controls (Close '✕' and Minimize '—')
                    if is_hovered {
                        let close_rect = Rect::from_center_size(
                            Pos2::new(capsule_rect.max.x - 14.0, capsule_rect.min.y + 14.0),
                            Vec2::new(14.0, 14.0),
                        );
                        let min_rect = Rect::from_center_size(
                            Pos2::new(capsule_rect.min.x + 14.0, capsule_rect.min.y + 14.0),
                            Vec2::new(14.0, 14.0),
                        );

                        let close_resp = ui.interact(
                            close_rect,
                            ui.id().with("close_btn"),
                            egui::Sense::click(),
                        );
                        let min_resp =
                            ui.interact(min_rect, ui.id().with("min_btn"), egui::Sense::click());

                        let close_color = if close_resp.hovered() {
                            Color32::from_rgb(255, 90, 90)
                        } else {
                            Color32::from_rgba_unmultiplied(160, 165, 180, 160)
                        };
                        let min_color = if min_resp.hovered() {
                            Color32::WHITE
                        } else {
                            Color32::from_rgba_unmultiplied(160, 165, 180, 160)
                        };

                        painter.text(
                            close_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "✕",
                            egui::FontId::proportional(9.0),
                            close_color,
                        );
                        painter.text(
                            min_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "—",
                            egui::FontId::proportional(9.0),
                            min_color,
                        );

                        if close_resp.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if min_resp.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    }
                });
            });
    }
}

/// Renders the authentic traffic light capsule housing, screws, deep cowls, and dotted LED lenses.
fn render_authentic_traffic_light(
    painter: &egui::Painter,
    rect: Rect,
    state: LightState,
    pulse_phase: f32,
) {
    let corner_rounding = Rounding::same(36.0);

    // 1. Soft Outer Ambient Shadow
    painter.rect_filled(
        rect.expand(2.0),
        Rounding::same(38.0),
        Color32::from_rgba_unmultiplied(0, 0, 0, 75),
    );

    // 2. Main Outer Housing (Deep Matte Obsidian/Graphite Polycarbonate)
    painter.rect_filled(
        rect,
        corner_rounding,
        Color32::from_rgba_unmultiplied(15, 17, 22, 252),
    );

    // 3. Molded Bevel Rim (Subtle double edge seen on authentic traffic light casings)
    painter.rect_stroke(
        rect,
        corner_rounding,
        Stroke::new(1.4_f32, Color32::from_rgba_unmultiplied(46, 51, 64, 210)),
    );
    painter.rect_stroke(
        rect.shrink(1.2),
        Rounding::same(35.0),
        Stroke::new(0.8_f32, Color32::from_rgba_unmultiplied(10, 12, 16, 240)),
    );

    let center_x = rect.center().x;

    // 4. Miniature Hardware Screws (Top & Bottom Center)
    draw_screw(painter, Pos2::new(center_x, rect.min.y + 13.0));
    draw_screw(painter, Pos2::new(center_x, rect.max.y - 13.0));

    // 5. Lamp Centers (Symmetrically spaced)
    let bulb_radius = 20.5;
    let start_y = rect.min.y + 40.5 + bulb_radius; // 61.0
    let step_y = bulb_radius * 2.0 + 14.0; // 55.0

    let centers = [
        Pos2::new(center_x, start_y),                // RED (Top)
        Pos2::new(center_x, start_y + step_y),       // YELLOW (Middle)
        Pos2::new(center_x, start_y + step_y * 2.0), // GREEN (Bottom)
    ];

    // 6. Draw each lamp: Socket, Cowl/Visor, LED Matrix, and Bloom Glow
    // RED (Top)
    let is_red = state == LightState::Red;
    draw_realistic_lamp(
        painter,
        centers[0],
        bulb_radius,
        is_red,
        Color32::from_rgb(255, 30, 30),
        Color32::from_rgb(24, 11, 13),
        Color32::from_rgb(255, 45, 45),
        1.0,
    );

    // YELLOW (Middle) - with smooth rhythmic pulse
    let is_yellow = state == LightState::Yellow;
    let yellow_pulse = if is_yellow {
        ((pulse_phase.sin() + 1.0) / 2.0) * 0.45 + 0.55
    } else {
        1.0
    };
    draw_realistic_lamp(
        painter,
        centers[1],
        bulb_radius,
        is_yellow,
        Color32::from_rgb(255, 192, 0),
        Color32::from_rgb(24, 20, 8),
        Color32::from_rgb(255, 205, 25),
        yellow_pulse,
    );

    // GREEN (Bottom)
    let is_green = state == LightState::Green;
    draw_realistic_lamp(
        painter,
        centers[2],
        bulb_radius,
        is_green,
        Color32::from_rgb(0, 230, 80),
        Color32::from_rgb(8, 24, 14),
        Color32::from_rgb(25, 245, 100),
        1.0,
    );
}

/// Draws an authentic metallic countersunk screw at the midline.
fn draw_screw(painter: &egui::Painter, pos: Pos2) {
    // Outer countersunk recess
    painter.circle_filled(pos, 2.5, Color32::from_rgb(36, 40, 50));
    // Inner socket hole
    painter.circle_filled(pos, 1.8, Color32::from_rgb(10, 11, 14));
    // Center slot highlight
    painter.line_segment(
        [Pos2::new(pos.x - 1.2, pos.y), Pos2::new(pos.x + 1.2, pos.y)],
        Stroke::new(0.8_f32, Color32::from_rgb(52, 58, 70)),
    );
}

/// Draws a realistic traffic light lamp with cylindrical visor hood, reflection, and LED emitter matrix.
#[allow(clippy::too_many_arguments)]
fn draw_realistic_lamp(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    is_active: bool,
    lit_color: Color32,
    unlit_color: Color32,
    glow_color: Color32,
    pulse: f32,
) {
    let socket_radius = radius + 2.5;

    // 1. Recessed dark lamp socket
    painter.circle_filled(center, socket_radius, Color32::from_rgb(10, 11, 14));
    painter.circle_stroke(
        center,
        socket_radius,
        Stroke::new(1.2_f32, Color32::from_rgb(24, 27, 34)),
    );

    // 2. Diffuse Bloom Glow (When lit)
    if is_active {
        // Broad outer ambient glow
        let glow_rad_1 = radius * (2.25 + (pulse - 1.0) * 0.35);
        painter.circle_filled(
            center,
            glow_rad_1,
            Color32::from_rgba_unmultiplied(
                glow_color.r(),
                glow_color.g(),
                glow_color.b(),
                (36.0 * pulse) as u8,
            ),
        );

        // Mid saturated bloom
        let glow_rad_2 = radius * (1.55 + (pulse - 1.0) * 0.2);
        painter.circle_filled(
            center,
            glow_rad_2,
            Color32::from_rgba_unmultiplied(
                glow_color.r(),
                glow_color.g(),
                glow_color.b(),
                (80.0 * pulse) as u8,
            ),
        );
    }

    // 3. Base lens disk
    let base_color = if is_active { lit_color } else { unlit_color };
    painter.circle_filled(center, radius, base_color);

    // 4. Dotted LED Matrix (Concentric emitter layout)
    draw_led_emitter_matrix(painter, center, is_active, pulse);

    // 5. Unlit Visor Shadow (Casts downward shadow on upper half of unlit lens)
    if !is_active {
        let shadow_rect = Rect::from_min_max(
            Pos2::new(center.x - radius, center.y - radius),
            Pos2::new(center.x + radius, center.y),
        );
        painter.rect_filled(
            shadow_rect,
            Rounding::ZERO,
            Color32::from_rgba_unmultiplied(0, 0, 0, 70),
        );
    }

    // 6. Cylindrical Visor Cowl (Sun shade projecting over upper half of bulb)
    let visor_radius = radius + 3.8;
    // Outer visor hood arch (-165 deg to -15 deg)
    draw_arc(
        painter,
        center,
        visor_radius,
        -2.88,
        -0.26,
        Stroke::new(3.6_f32, Color32::from_rgb(18, 20, 26)),
    );
    // Outer visor highlight trim
    draw_arc(
        painter,
        center,
        visor_radius + 1.2,
        -2.85,
        -0.29,
        Stroke::new(0.8_f32, Color32::from_rgba_unmultiplied(65, 72, 90, 120)),
    );

    // 7. Visor Inner Reflection (Highlighted crescent visible in the reference image when active)
    if is_active {
        draw_arc(
            painter,
            center,
            radius + 1.6,
            -2.75,
            -0.39,
            Stroke::new(
                1.8_f32,
                Color32::from_rgba_unmultiplied(
                    glow_color.r(),
                    glow_color.g(),
                    glow_color.b(),
                    (185.0 * pulse) as u8,
                ),
            ),
        );
    }

    // 8. Lens Edge Rim
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            1.0_f32,
            if is_active {
                Color32::from_rgba_unmultiplied(255, 255, 255, 75)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 14)
            },
        ),
    );
}

/// Draws concentric rings of miniature LED emitter dots matching modern LED traffic lights.
fn draw_led_emitter_matrix(painter: &egui::Painter, center: Pos2, is_active: bool, pulse: f32) {
    let rings: &[(f32, usize)] = &[(0.0, 1), (5.0, 6), (10.2, 12), (15.4, 18)];

    for &(ring_r, count) in rings {
        let dot_r = if ring_r == 0.0 {
            1.65
        } else if ring_r < 8.0 {
            1.4
        } else if ring_r < 12.0 {
            1.25
        } else {
            1.15
        };

        for i in 0..count {
            let angle = if count == 1 {
                0.0
            } else {
                (i as f32 / count as f32) * std::f32::consts::TAU
            };
            let pos = Pos2::new(
                center.x + ring_r * angle.cos(),
                center.y + ring_r * angle.sin(),
            );

            if is_active {
                // Illuminated LED emitter
                let alpha = if ring_r == 0.0 {
                    255
                } else if ring_r < 8.0 {
                    (235.0 * pulse) as u8
                } else if ring_r < 12.0 {
                    (195.0 * pulse) as u8
                } else {
                    (155.0 * pulse) as u8
                };

                painter.circle_filled(
                    pos,
                    dot_r * (1.0 + (1.0 - ring_r / 17.0) * 0.2),
                    Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                );
            } else {
                // Dim unlit LED dot texture
                painter.circle_filled(pos, dot_r, Color32::from_rgba_unmultiplied(30, 34, 44, 170));
                painter.circle_filled(
                    pos,
                    dot_r * 0.55,
                    Color32::from_rgba_unmultiplied(10, 12, 16, 210),
                );
            }
        }
    }
}

/// Helper to render a smooth stroked curved arc.
fn draw_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    stroke: Stroke,
) {
    let steps = 24;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        points.push(Pos2::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        ));
    }
    painter.add(egui::Shape::line(points, stroke));
}
