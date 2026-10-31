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
    // Smooth transition weights for the three lamps [red, yellow, green] in range [0.0, 1.0]
    red_weight: f32,
    yellow_weight: f32,
    green_weight: f32,
}

impl TrafficStatusApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, rx: Receiver<IpcCommand>) -> Self {
        Self {
            rx,
            store: SessionStore::with_default_session(),
            pulse_phase: 0.0,
            last_frame_time: Instant::now(),
            last_width: 104.0,
            red_weight: 0.0,
            yellow_weight: 0.0,
            green_weight: 1.0, // Default active session starts in Green
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
        let needed_width = if has_multiple_sessions { 136.0 } else { 104.0 };
        let needed_height = 280.0;

        if (needed_width - self.last_width).abs() > 0.5 {
            self.last_width = needed_width;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                needed_width,
                needed_height,
            )));
        }

        let now = Instant::now();
        let dt = now
            .duration_since(self.last_frame_time)
            .as_secs_f32()
            .min(0.1);
        self.last_frame_time = now;

        self.store.ensure_active();
        let active_session = self.store.active_session().cloned();
        let current_state = active_session
            .as_ref()
            .map(|s| s.state)
            .unwrap_or(LightState::Off);

        // Target weights based on current state
        let target_red = if current_state == LightState::Red {
            1.0
        } else {
            0.0
        };
        let target_yellow = if current_state == LightState::Yellow {
            1.0
        } else {
            0.0
        };
        let target_green = if current_state == LightState::Green {
            1.0
        } else {
            0.0
        };

        // 250ms smooth exponential blend (speed ~ 12.0)
        let blend_speed = 12.0 * dt;
        self.red_weight += (target_red - self.red_weight) * blend_speed.min(1.0);
        self.yellow_weight += (target_yellow - self.yellow_weight) * blend_speed.min(1.0);
        self.green_weight += (target_green - self.green_weight) * blend_speed.min(1.0);

        let is_animating_transition = (target_red - self.red_weight).abs() > 0.01
            || (target_yellow - self.yellow_weight).abs() > 0.01
            || (target_green - self.green_weight).abs() > 0.01;

        if self.has_active_animation() || is_animating_transition {
            self.pulse_phase = (self.pulse_phase + dt * 4.2) % std::f32::consts::TAU;
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(500));
        }

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
                            ui.add_space(24.0);
                            for (idx, session_id) in order_snapshot.iter().enumerate() {
                                if let Some(sess) = self.store.sessions().get(session_id) {
                                    let is_selected =
                                        self.store.active_id() == Some(session_id.as_str());
                                    let tab_dot_color = match sess.state {
                                        LightState::Red => Color32::from_rgb(229, 72, 77),
                                        LightState::Yellow => Color32::from_rgb(242, 163, 60),
                                        LightState::Green => Color32::from_rgb(47, 204, 110),
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

                    // High-Fidelity Hardware Traffic Light Housing
                    let (capsule_rect, _) =
                        ui.allocate_exact_size(Vec2::new(104.0, 276.0), egui::Sense::hover());

                    let painter = ui.painter_at(capsule_rect);

                    render_authentic_traffic_light(
                        &painter,
                        capsule_rect,
                        self.red_weight,
                        self.yellow_weight,
                        self.green_weight,
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

/// Renders the reference video authentic hardware capsule housing, 3D visor cowls, and dotted LED lenses.
fn render_authentic_traffic_light(
    painter: &egui::Painter,
    rect: Rect,
    red_weight: f32,
    yellow_weight: f32,
    green_weight: f32,
    pulse_phase: f32,
) {
    let corner_rounding = Rounding::same(rect.width() * 0.5);

    // 1. Multi-Stage Realistic Drop Shadows (as defined in reference styles.css)
    // 0 14px 30px -10px rgba(0, 0, 0, 0.8)
    painter.rect_filled(
        rect.translate(Vec2::new(0.0, 8.0)).expand(1.0),
        corner_rounding,
        Color32::from_rgba_unmultiplied(0, 0, 0, 130),
    );
    // 0 44px 80px -40px rgba(0, 0, 0, 0.9)
    painter.rect_filled(
        rect.translate(Vec2::new(0.0, 18.0)).expand(3.0),
        corner_rounding,
        Color32::from_rgba_unmultiplied(0, 0, 0, 75),
    );

    // 2. Main Matte Hardware Housing (#191D23 from reference design)
    painter.rect_filled(rect, corner_rounding, Color32::from_rgb(25, 29, 35));

    // Subtle brushed metallic gradient overlay (100deg highlight on top left)
    painter.rect_stroke(
        rect,
        corner_rounding,
        Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 42)),
    );
    // Dark bottom inset edge
    painter.rect_stroke(
        rect.shrink(1.0),
        Rounding::same(rect.width() * 0.5 - 1.0),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 180)),
    );

    let center_x = rect.center().x;

    // 3. Hardware Screws (calc(var(--lamp-w) * 0.085) ~ 8.8px from top/bottom curve)
    draw_screw(painter, Pos2::new(center_x, rect.min.y + 11.5));
    draw_screw(painter, Pos2::new(center_x, rect.max.y - 11.5));

    // 4. Lamp Dimensions (72px diameter = 36px radius)
    let lamp_radius = 36.0;
    let start_y = rect.min.y + 24.5 + lamp_radius; // 60.5
    let step_y = 80.0; // Spacing between centers

    let centers = [
        Pos2::new(center_x, start_y),                // RED (Top)
        Pos2::new(center_x, start_y + step_y),       // YELLOW (Middle)
        Pos2::new(center_x, start_y + step_y * 2.0), // GREEN (Bottom)
    ];

    // Pulse multiplier for yellow when active
    let yellow_pulse = if yellow_weight > 0.05 {
        ((pulse_phase.sin() + 1.0) / 2.0) * 0.35 + 0.65
    } else {
        1.0
    };

    // RED (Top) - #E5484D / #FF1744
    draw_reference_lamp(
        painter,
        centers[0],
        lamp_radius,
        red_weight,
        Color32::from_rgb(229, 72, 77),
        Color32::from_rgb(46, 20, 24),
        Color32::from_rgb(255, 23, 68),
        1.0,
    );

    // YELLOW (Middle) - #F2A33C / #FFB300
    draw_reference_lamp(
        painter,
        centers[1],
        lamp_radius,
        yellow_weight,
        Color32::from_rgb(242, 163, 60),
        Color32::from_rgb(48, 32, 16),
        Color32::from_rgb(255, 179, 0),
        yellow_pulse,
    );

    // GREEN (Bottom) - #2FCC6E / #00E676
    draw_reference_lamp(
        painter,
        centers[2],
        lamp_radius,
        green_weight,
        Color32::from_rgb(47, 204, 110),
        Color32::from_rgb(18, 44, 28),
        Color32::from_rgb(0, 230, 118),
        1.0,
    );
}

/// Draws an authentic metallic countersunk screw matching the reference styles.css
fn draw_screw(painter: &egui::Painter, pos: Pos2) {
    let screw_radius = 3.2;
    // Outer shadow
    painter.circle_filled(
        Pos2::new(pos.x, pos.y + 0.5),
        screw_radius,
        Color32::from_rgba_unmultiplied(0, 0, 0, 160),
    );
    // Outer screw rim (#3A424C -> #101317)
    painter.circle_filled(pos, screw_radius, Color32::from_rgb(42, 48, 56));
    painter.circle_stroke(
        pos,
        screw_radius,
        Stroke::new(0.6_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 48)),
    );
    // Inner socket recess
    painter.circle_filled(pos, 2.2, Color32::from_rgb(16, 19, 23));
    // Center screw slot
    painter.line_segment(
        [Pos2::new(pos.x - 1.5, pos.y), Pos2::new(pos.x + 1.5, pos.y)],
        Stroke::new(0.8_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 200)),
    );
}

/// Draws a lamp matching the reference video design:
/// - Recessed seating socket (#2E353E to #0B0E12)
/// - Multi-stage soft radial bloom
/// - Glass lens with unlit baseline reflection (16% color) blending to 100% lit
/// - 3D visor cowl with downward shadow
/// - Dotted LED emitter grid
#[allow(clippy::too_many_arguments)]
fn draw_reference_lamp(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    weight: f32,
    lit_color: Color32,
    unlit_color: Color32,
    glow_color: Color32,
    pulse: f32,
) {
    let effective_weight = weight * pulse;

    // 1. Recessed Conical Socket (.lamp__u::before in reference CSS)
    let socket_radius = radius + 2.0;
    painter.circle_filled(center, socket_radius, Color32::from_rgb(11, 14, 18));
    painter.circle_stroke(
        center,
        socket_radius,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 210)),
    );

    // 2. Multi-Stage Soft Radial Bloom Glow (Surrounding active lamps)
    if effective_weight > 0.02 {
        let alpha_outer = (45.0 * effective_weight) as u8;
        let alpha_mid = (90.0 * effective_weight) as u8;

        // Outer ambient glow (radius * 1.8)
        painter.circle_filled(
            center,
            radius * 1.8,
            Color32::from_rgba_unmultiplied(
                glow_color.r(),
                glow_color.g(),
                glow_color.b(),
                alpha_outer,
            ),
        );
        // Saturated core bloom (radius * 1.35)
        painter.circle_filled(
            center,
            radius * 1.35,
            Color32::from_rgba_unmultiplied(
                glow_color.r(),
                glow_color.g(),
                glow_color.b(),
                alpha_mid,
            ),
        );
    }

    // 3. Base Glass Lens Color (Smoothly blended from unlit 16% baseline to 100% lit)
    let t = effective_weight.clamp(0.0, 1.0);
    let r = ((1.0 - t) * unlit_color.r() as f32 + t * lit_color.r() as f32) as u8;
    let g = ((1.0 - t) * unlit_color.g() as f32 + t * lit_color.g() as f32) as u8;
    let b = ((1.0 - t) * unlit_color.b() as f32 + t * lit_color.b() as f32) as u8;
    let lens_color = Color32::from_rgb(r, g, b);

    painter.circle_filled(center, radius, lens_color);

    // Lens rim highlights (inner bottom shadow, inner top rim highlight)
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(255, 255, 255, (20.0 + 40.0 * effective_weight) as u8),
        ),
    );

    // 4. Fine-grained Dotted LED Matrix Grid Texture (as in reference styles.css)
    draw_grid_led_matrix(painter, center, radius, effective_weight);

    // 5. Downward Visor Cast Shadow on upper half of lens
    let shadow_rect = Rect::from_min_max(
        Pos2::new(center.x - radius, center.y - radius),
        Pos2::new(center.x + radius, center.y + 4.0),
    );
    painter.rect_filled(
        shadow_rect,
        Rounding::ZERO,
        Color32::from_rgba_unmultiplied(0, 0, 0, 85),
    );

    // 6. 3D Cylindrical Visor Cowl (Sun Shade Dome)
    draw_3d_visor_cowl(painter, center, radius, effective_weight, glow_color);
}

/// Draws the realistic 3D visor cowl matching reference video:
/// 3D layered cylindrical shroud framing the top aperture, casting inner shadows and catching rim light.
fn draw_3d_visor_cowl(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    weight: f32,
    glow_color: Color32,
) {
    let visor_r = radius + 3.0;

    // Outer Visor Body: Layered arcs creating 3D cylindrical projection
    // Shroud top body (#2B323B down to #0E1115)
    for i in 0..4 {
        let offset_y = -(i as f32) * 1.0;
        let arc_r = visor_r + (i as f32) * 0.4;
        let c = Pos2::new(center.x, center.y + offset_y);
        let shade = (38 - i * 7) as u8;
        draw_arc(
            painter,
            c,
            arc_r,
            -2.92,
            -0.22,
            Stroke::new(2.8_f32, Color32::from_rgb(shade, shade + 5, shade + 12)),
        );
    }

    // Top outer visor rim highlight (catches subtle top directional ambient light)
    draw_arc(
        painter,
        Pos2::new(center.x, center.y - 3.2),
        visor_r + 1.2,
        -2.75,
        -0.39,
        Stroke::new(0.9_f32, Color32::from_rgba_unmultiplied(180, 195, 215, 60)),
    );

    // Inner cowl lip (deep dark shadow defining the underside)
    draw_arc(
        painter,
        center,
        radius + 0.8,
        -3.00,
        -0.14,
        Stroke::new(2.4_f32, Color32::from_rgba_unmultiplied(8, 10, 14, 240)),
    );

    // Active visor inner crescent reflection when lit
    if weight > 0.05 {
        draw_arc(
            painter,
            center,
            radius + 1.6,
            -2.75,
            -0.39,
            Stroke::new(
                1.6_f32,
                Color32::from_rgba_unmultiplied(
                    glow_color.r(),
                    glow_color.g(),
                    glow_color.b(),
                    (190.0 * weight) as u8,
                ),
            ),
        );
    }
}

/// Draws fine-grained dotted LED matrix grid (matching reference CSS background-size: 11% 11%)
fn draw_grid_led_matrix(painter: &egui::Painter, center: Pos2, radius: f32, weight: f32) {
    let grid_step = 6.2; // ~9x9 grid across 72px aperture
    let dot_r = 1.05;
    let radius_sq = (radius - 2.8) * (radius - 2.8);

    let min_k = -5;
    let max_k = 5;

    for dy in min_k..=max_k {
        for dx in min_k..=max_k {
            let offset_x = dx as f32 * grid_step;
            let offset_y = dy as f32 * grid_step;
            let dist_sq = offset_x * offset_x + offset_y * offset_y;

            if dist_sq <= radius_sq {
                let pos = Pos2::new(center.x + offset_x, center.y + offset_y);
                let norm_dist = (dist_sq / radius_sq).sqrt();

                if weight > 0.05 {
                    // Active LED dot emitter: brighter at center, soft edge
                    let center_boost = (1.0 - norm_dist * 0.4).max(0.5);
                    let alpha = ((140.0 + 115.0 * center_boost) * weight) as u8;
                    painter.circle_filled(
                        pos,
                        dot_r * (0.95 + 0.25 * weight),
                        Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                    );
                } else {
                    // Unlit glass dot texture
                    painter.circle_filled(
                        pos,
                        dot_r * 0.8,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 28),
                    );
                }
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
    let steps = 28;
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
