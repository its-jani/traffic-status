pub mod session;

use crossbeam_channel::Receiver;
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use std::time::{Duration, Instant};

use crate::app::session::{AppEffect, SessionStore, SESSION_TTL};
use crate::config::{self, AppConfig, WindowPlacement};
use crate::types::{IpcCommand, LightState};

/// Capsule width in design points at `scale == 1.0`.
const CAPSULE_WIDTH: f32 = 104.0;
/// Capsule height in design points at `scale == 1.0`.
const CAPSULE_HEIGHT: f32 = 276.0;
/// Extra width reserved for the multi-session tab rail.
const TAB_RAIL_WIDTH: f32 = 32.0;
/// Outer window height (capsule plus shadow breathing room).
const WINDOW_HEIGHT: f32 = 280.0;

/// 1.6 s attention pulse for the red lamp.
const PULSE_PERIOD_SECS: f32 = 1.6;
const PULSE_SPEED: f32 = std::f32::consts::TAU / PULSE_PERIOD_SECS;
/// Pulse oscillates between 82% and 100% brightness.
const PULSE_MID: f32 = 0.91;
const PULSE_AMPLITUDE: f32 = 0.09;

pub struct TrafficStatusApp {
    rx: Receiver<IpcCommand>,
    store: SessionStore,
    config: AppConfig,
    pulse_phase: f32,
    last_frame_time: Instant,
    last_size: Vec2,
    last_prune: Instant,
    last_position_save: Instant,
    /// True while the OS is forwarding clicks straight through the capsule.
    passthrough: bool,
    // Smooth transition weights for the three lamps [red, yellow, green] in range [0.0, 1.0]
    red_weight: f32,
    yellow_weight: f32,
    green_weight: f32,
}

impl TrafficStatusApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        rx: Receiver<IpcCommand>,
        config: AppConfig,
    ) -> Self {
        let config = config.sanitized();
        let passthrough = config.ghost;
        Self {
            rx,
            store: SessionStore::with_default_session(),
            config,
            pulse_phase: 0.0,
            last_frame_time: Instant::now(),
            last_size: Vec2::ZERO,
            last_prune: Instant::now(),
            last_position_save: Instant::now(),
            passthrough,
            red_weight: 0.0,
            yellow_weight: 0.0,
            green_weight: 1.0, // Default active session starts in Green
        }
    }

    fn process_ipc_events(&mut self, ctx: &egui::Context) {
        while let Ok(cmd) = self.rx.try_recv() {
            if let IpcCommand::UpdateConfig { scale, ghost } = cmd {
                self.apply_config(scale, ghost);
                continue;
            }
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

    /// Apply and persist `/config` changes coming from the CLI or an agent hook.
    fn apply_config(&mut self, scale: Option<f32>, ghost: Option<bool>) {
        if let Some(scale) = scale {
            self.config.scale = config::clamp_scale(scale);
            // Force the window to re-measure on the next frame.
            self.last_size = Vec2::ZERO;
        }
        if let Some(ghost) = ghost {
            self.config.ghost = ghost;
        }
        self.persist_config();
    }

    fn persist_config(&mut self) {
        if let Err(e) = self.config.save() {
            eprintln!("[Traffic Status] config: {e}");
        }
    }

    /// Garbage-collect sessions that went quiet (closed terminal, crashed CLI).
    fn maintain_sessions(&mut self, ctx: &egui::Context) {
        if self.last_prune.elapsed() < Duration::from_secs(5) {
            return;
        }
        self.last_prune = Instant::now();
        for effect in self.store.prune_stale(SESSION_TTL) {
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

    fn widget_size(&self) -> Vec2 {
        let width = if self.store.order().len() > 1 {
            CAPSULE_WIDTH + TAB_RAIL_WIDTH
        } else {
            CAPSULE_WIDTH
        };
        Vec2::new(width, WINDOW_HEIGHT) * self.config.scale
    }

    /// Persist `(x, y)` whenever the OS reports a new, settled window position.
    fn persist_position_if_moved(&mut self, ctx: &egui::Context) {
        let Some(rect) = ctx.input(|i| i.viewport().outer_rect) else {
            return;
        };
        let pointer_down = ctx.input(|i| i.pointer.any_down());
        if pointer_down || self.last_position_save.elapsed() < Duration::from_millis(300) {
            return;
        }

        let pos = (rect.min.x, rect.min.y);
        let moved = self
            .config
            .window
            .as_pos()
            .is_none_or(|(x, y)| (x - pos.0).abs() > 0.5 || (y - pos.1).abs() > 0.5);
        if moved {
            self.config.window = WindowPlacement::from_pos(pos.0, pos.1);
            self.last_position_save = Instant::now();
            self.persist_config();
        }
    }

    /// Toggle click-through mode, suspending it while `Alt` is held so the
    /// capsule can still be grabbed.
    fn update_passthrough(&mut self, ctx: &egui::Context) {
        let want_passthrough = self.config.ghost && !alt_held(ctx);
        if want_passthrough != self.passthrough {
            self.passthrough = want_passthrough;
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(want_passthrough));
        }
        if self.config.ghost {
            // Keep polling `Alt` even though clicks are forwarded elsewhere.
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }
}

impl eframe::App for TrafficStatusApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist_config();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_ipc_events(ctx);
        self.maintain_sessions(ctx);
        self.update_passthrough(ctx);
        self.persist_position_if_moved(ctx);

        let needed = self.widget_size();
        if (needed - self.last_size).length() > 0.5 {
            self.last_size = needed;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(needed));
        }

        let now = Instant::now();
        let dt = now
            .duration_since(self.last_frame_time)
            .as_secs_f32()
            .min(0.1);
        self.last_frame_time = now;

        self.store.ensure_active();
        // Multi-agent multiplexing: the bulb always shows the most severe
        // session state (Red > Yellow > Green), not just the last tab.
        let current_state = self.store.resolved_state();

        // Target weights based on the resolved state
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

        if self.store.has_active_animation() || is_animating_transition {
            self.pulse_phase = (self.pulse_phase + dt * PULSE_SPEED) % std::f32::consts::TAU;
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(500));
        }

        // Slow sinusoidal breathing so a blocked permission prompt catches
        // peripheral attention without jarring flashes.
        let red_pulse = PULSE_MID + PULSE_AMPLITUDE * self.pulse_phase.sin();

        let scale = self.config.scale;
        let has_multiple_sessions = self.store.order().len() > 1;

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
                            ui.add_space(24.0 * scale);
                            for (idx, session_id) in order_snapshot.iter().enumerate() {
                                if let Some(sess) = self.store.sessions().get(session_id) {
                                    let is_selected =
                                        self.store.active_id() == Some(session_id.as_str());
                                    let tab_dot_color = lamp_color(sess.state);

                                    let tab_btn = egui::Button::new(
                                        egui::RichText::new(format!("{}", idx + 1))
                                            .size(10.0 * scale)
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
                                    .rounding(Rounding::same(8.0 * scale));

                                    let hover_text = format!(
                                        "{} ({})\nStatus: {}\n{}",
                                        sess.label,
                                        sess.source.as_deref().unwrap_or("agent"),
                                        sess.state.display_name(),
                                        sess.message.as_deref().unwrap_or("No message")
                                    );

                                    if ui.add(tab_btn).on_hover_text(hover_text).clicked() {
                                        self.store.select(session_id);
                                    }

                                    ui.add_space(3.0 * scale);
                                }
                            }
                        });
                    }

                    // High-Fidelity Hardware Traffic Light Housing
                    let (capsule_rect, _) = ui.allocate_exact_size(
                        Vec2::new(CAPSULE_WIDTH * scale, CAPSULE_HEIGHT * scale),
                        egui::Sense::hover(),
                    );

                    let painter = ui.painter_at(capsule_rect);

                    render_authentic_traffic_light(
                        &painter,
                        capsule_rect,
                        self.red_weight,
                        self.yellow_weight,
                        self.green_weight,
                        red_pulse,
                    );

                    // Discreet hover controls (Close '✕' and Minimize '—')
                    if is_hovered {
                        let close_rect = Rect::from_center_size(
                            Pos2::new(
                                capsule_rect.max.x - 14.0 * scale,
                                capsule_rect.min.y + 14.0 * scale,
                            ),
                            Vec2::splat(14.0 * scale),
                        );
                        let min_rect = Rect::from_center_size(
                            Pos2::new(
                                capsule_rect.min.x + 14.0 * scale,
                                capsule_rect.min.y + 14.0 * scale,
                            ),
                            Vec2::splat(14.0 * scale),
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
                            egui::FontId::proportional(9.0 * scale),
                            close_color,
                        );
                        painter.text(
                            min_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "—",
                            egui::FontId::proportional(9.0 * scale),
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

/// Lamp color for one session, reused by the tab rail.
fn lamp_color(state: LightState) -> Color32 {
    match state {
        LightState::Red => Color32::from_rgb(229, 72, 77),
        LightState::Yellow => Color32::from_rgb(242, 163, 60),
        LightState::Green => Color32::from_rgb(47, 204, 110),
        LightState::Off => Color32::from_rgb(80, 85, 100),
    }
}

/// Is `Alt` currently held? On Windows this is polled from the OS so it keeps
/// working while the window is click-through and therefore unfocused.
#[cfg(target_os = "windows")]
fn alt_held(_ctx: &egui::Context) -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn GetAsyncKeyState(v_key: i32) -> i16;
    }
    const VK_MENU: i32 = 0x12;
    unsafe { (GetAsyncKeyState(VK_MENU) as u16 & 0x8000) != 0 }
}

#[cfg(not(target_os = "windows"))]
fn alt_held(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.modifiers.alt)
}

/// Renders the reference video authentic hardware capsule housing, 3D visor cowls, and dotted LED lenses.
fn render_authentic_traffic_light(
    painter: &egui::Painter,
    rect: Rect,
    red_weight: f32,
    yellow_weight: f32,
    green_weight: f32,
    red_pulse: f32,
) {
    // Everything is authored against the 104 px design width, then scaled.
    let s = rect.width() / CAPSULE_WIDTH;
    let corner_rounding = Rounding::same(rect.width() * 0.5);

    // 1. Multi-Stage Realistic Drop Shadows (as defined in reference styles.css)
    // 0 14px 30px -10px rgba(0, 0, 0, 0.8)
    painter.rect_filled(
        rect.translate(Vec2::new(0.0, 8.0 * s)).expand(1.0 * s),
        corner_rounding,
        Color32::from_rgba_unmultiplied(0, 0, 0, 130),
    );
    // 0 44px 80px -40px rgba(0, 0, 0, 0.9)
    painter.rect_filled(
        rect.translate(Vec2::new(0.0, 18.0 * s)).expand(3.0 * s),
        corner_rounding,
        Color32::from_rgba_unmultiplied(0, 0, 0, 75),
    );

    // 2. Main Matte Hardware Housing (#191D23 from reference design)
    painter.rect_filled(rect, corner_rounding, Color32::from_rgb(25, 29, 35));

    // Subtle brushed metallic gradient overlay (100deg highlight on top left)
    painter.rect_stroke(
        rect,
        corner_rounding,
        Stroke::new(
            1.2_f32 * s,
            Color32::from_rgba_unmultiplied(255, 255, 255, 42),
        ),
    );
    // Dark bottom inset edge
    painter.rect_stroke(
        rect.shrink(1.0 * s),
        Rounding::same(rect.width() * 0.5 - 1.0 * s),
        Stroke::new(1.0_f32 * s, Color32::from_rgba_unmultiplied(0, 0, 0, 180)),
    );

    let center_x = rect.center().x;

    // 3. Hardware Screws (calc(var(--lamp-w) * 0.085) ~ 8.8px from top/bottom curve)
    draw_screw(painter, Pos2::new(center_x, rect.min.y + 11.5 * s), s);
    draw_screw(painter, Pos2::new(center_x, rect.max.y - 11.5 * s), s);

    // 4. Lamp Dimensions (72px diameter = 36px radius at scale 1.0)
    let lamp_radius = 36.0 * s;
    let start_y = rect.min.y + 24.5 * s + lamp_radius; // 60.5
    let step_y = 80.0 * s; // Spacing between centers

    let centers = [
        Pos2::new(center_x, start_y),                // RED (Top)
        Pos2::new(center_x, start_y + step_y),       // YELLOW (Middle)
        Pos2::new(center_x, start_y + step_y * 2.0), // GREEN (Bottom)
    ];

    // RED (Top) - #E5484D / #FF1744, breathing while a session is blocked
    draw_reference_lamp(
        painter,
        centers[0],
        lamp_radius,
        red_weight,
        Color32::from_rgb(229, 72, 77),
        Color32::from_rgb(46, 20, 24),
        Color32::from_rgb(255, 23, 68),
        red_pulse,
        s,
    );

    // YELLOW (Middle) - #F2A33C / #FFB300, steady after its fade-in
    draw_reference_lamp(
        painter,
        centers[1],
        lamp_radius,
        yellow_weight,
        Color32::from_rgb(242, 163, 60),
        Color32::from_rgb(48, 32, 16),
        Color32::from_rgb(255, 179, 0),
        1.0,
        s,
    );

    // GREEN (Bottom) - #2FCC6E / #00E676, steady after its fade-in
    draw_reference_lamp(
        painter,
        centers[2],
        lamp_radius,
        green_weight,
        Color32::from_rgb(47, 204, 110),
        Color32::from_rgb(18, 44, 28),
        Color32::from_rgb(0, 230, 118),
        1.0,
        s,
    );
}

/// Draws an authentic metallic countersunk screw matching the reference styles.css
fn draw_screw(painter: &egui::Painter, pos: Pos2, s: f32) {
    let screw_radius = 3.2 * s;
    // Outer shadow
    painter.circle_filled(
        Pos2::new(pos.x, pos.y + 0.5 * s),
        screw_radius,
        Color32::from_rgba_unmultiplied(0, 0, 0, 160),
    );
    // Outer screw rim (#3A424C -> #101317)
    painter.circle_filled(pos, screw_radius, Color32::from_rgb(42, 48, 56));
    painter.circle_stroke(
        pos,
        screw_radius,
        Stroke::new(
            0.6_f32 * s,
            Color32::from_rgba_unmultiplied(255, 255, 255, 48),
        ),
    );
    // Inner socket recess
    painter.circle_filled(pos, 2.2 * s, Color32::from_rgb(16, 19, 23));
    // Center screw slot
    painter.line_segment(
        [
            Pos2::new(pos.x - 1.5 * s, pos.y),
            Pos2::new(pos.x + 1.5 * s, pos.y),
        ],
        Stroke::new(0.8_f32 * s, Color32::from_rgba_unmultiplied(0, 0, 0, 200)),
    );
}

/// Draws a lamp matching the reference video design:
/// - Recessed seating socket (#2E353E to #0B0E12)
/// - Crisp optical lens with all lighting strictly contained inside the lamp aperture
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
    s: f32,
) {
    let effective_weight = weight * pulse;

    // 1. Recessed Bezel Ring Socket (Dark mounting frame)
    let socket_radius = radius + 2.0 * s;
    painter.circle_filled(center, socket_radius, Color32::from_rgb(12, 15, 19));
    painter.circle_stroke(
        center,
        socket_radius,
        Stroke::new(1.0_f32 * s, Color32::from_rgba_unmultiplied(0, 0, 0, 230)),
    );
    // Outer subtle bezel highlight on upper rim
    painter.circle_stroke(
        center,
        socket_radius - 0.5 * s,
        Stroke::new(
            0.6_f32 * s,
            Color32::from_rgba_unmultiplied(255, 255, 255, 18),
        ),
    );

    // 2. Base Glass Lens Color (Smoothly blended from unlit 16% baseline to 100% lit)
    let t = effective_weight.clamp(0.0, 1.0);
    let r = ((1.0 - t) * unlit_color.r() as f32 + t * lit_color.r() as f32) as u8;
    let g = ((1.0 - t) * unlit_color.g() as f32 + t * lit_color.g() as f32) as u8;
    let b = ((1.0 - t) * unlit_color.b() as f32 + t * lit_color.b() as f32) as u8;
    let lens_color = Color32::from_rgb(r, g, b);

    // Lens glass body - STRICTLY CONFINED TO RADIUS
    painter.circle_filled(center, radius, lens_color);

    // 3. Internal Lens Illumination Core (Strictly confined inside the lens - NEVER outside radius)
    if effective_weight > 0.02 {
        // Inner luminous glow core (radius * 0.82)
        let alpha_core = (105.0 * effective_weight) as u8;
        painter.circle_filled(
            center,
            radius * 0.82,
            Color32::from_rgba_unmultiplied(
                glow_color.r(),
                glow_color.g(),
                glow_color.b(),
                alpha_core,
            ),
        );
        // Saturated center bloom (radius * 0.52)
        let alpha_mid = (150.0 * effective_weight) as u8;
        painter.circle_filled(
            center,
            radius * 0.52,
            Color32::from_rgba_unmultiplied(
                glow_color.r(),
                glow_color.g(),
                glow_color.b(),
                alpha_mid,
            ),
        );
        // Center hot-spot (radius * 0.28)
        let alpha_hot = (180.0 * effective_weight) as u8;
        painter.circle_filled(
            center,
            radius * 0.28,
            Color32::from_rgba_unmultiplied(255, 255, 255, alpha_hot),
        );
    }

    // 4. Fine-grained Dotted LED Matrix Grid Texture (inside radius - 2.8)
    draw_grid_led_matrix(painter, center, radius, effective_weight, s);

    // 5. Downward Visor Cast Shadow on upper half of lens
    let shadow_rect = Rect::from_min_max(
        Pos2::new(center.x - radius + 1.5 * s, center.y - radius),
        Pos2::new(center.x + radius - 1.5 * s, center.y + 3.0 * s),
    );
    painter.rect_filled(
        shadow_rect,
        Rounding::same(radius * 0.35),
        Color32::from_rgba_unmultiplied(0, 0, 0, 85),
    );

    // 6. Crisp Lens Glass Outer Rim Stroke (Defines the exact circular boundary)
    let rim_highlight_alpha = (30.0 + 45.0 * effective_weight) as u8;
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            1.2_f32 * s,
            Color32::from_rgba_unmultiplied(255, 255, 255, rim_highlight_alpha),
        ),
    );
    // Dark inner rim ring right inside the edge
    painter.circle_stroke(
        center,
        radius - 0.8 * s,
        Stroke::new(0.8_f32 * s, Color32::from_rgba_unmultiplied(0, 0, 0, 160)),
    );

    // 7. 3D Cylindrical Visor Cowl (Sun Shade Dome)
    draw_3d_visor_cowl(painter, center, radius, s);
}

/// Draws the realistic 3D visor cowl matching reference video:
/// 3D layered cylindrical shroud framing the top aperture, casting inner shadows and catching rim light.
fn draw_3d_visor_cowl(painter: &egui::Painter, center: Pos2, radius: f32, s: f32) {
    let visor_r = radius + 3.0 * s;

    // Outer Visor Body: Layered arcs creating 3D cylindrical projection
    // Shroud top body (#2B323B down to #0E1115)
    for i in 0..4 {
        let offset_y = -(i as f32) * 1.0 * s;
        let arc_r = visor_r + (i as f32) * 0.4 * s;
        let c = Pos2::new(center.x, center.y + offset_y);
        let shade = (38 - i * 7) as u8;
        draw_arc(
            painter,
            c,
            arc_r,
            -2.92,
            -0.22,
            Stroke::new(2.8_f32 * s, Color32::from_rgb(shade, shade + 5, shade + 12)),
        );
    }

    // Top outer visor rim highlight (catches subtle top directional ambient light)
    draw_arc(
        painter,
        Pos2::new(center.x, center.y - 3.2 * s),
        visor_r + 1.2 * s,
        -2.75,
        -0.39,
        Stroke::new(
            0.9_f32 * s,
            Color32::from_rgba_unmultiplied(180, 195, 215, 60),
        ),
    );

    // Inner cowl lip (deep dark shadow defining the underside)
    draw_arc(
        painter,
        center,
        radius + 0.8 * s,
        -3.00,
        -0.14,
        Stroke::new(2.4_f32 * s, Color32::from_rgba_unmultiplied(8, 10, 14, 240)),
    );
}

/// Draws fine-grained dotted LED matrix grid (matching reference CSS background-size: 11% 11%)
fn draw_grid_led_matrix(painter: &egui::Painter, center: Pos2, radius: f32, weight: f32, s: f32) {
    let grid_step = 6.2 * s; // ~9x9 grid across the 72px aperture
    let dot_r = 1.05 * s;
    let radius_sq = (radius - 2.8 * s) * (radius - 2.8 * s);

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
                    let alpha = ((140.0 + 115.0 * center_boost) * weight).clamp(0.0, 255.0) as u8;
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
