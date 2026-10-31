pub mod app;
pub mod cli;
pub mod installer;
pub mod ipc;
pub mod types;

use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;

use crate::app::TrafficStatusApp;
use crate::cli::handle_cli_args;
use crate::ipc::start_ipc_server;
use crate::types::IpcCommand;

fn main() -> eframe::Result<()> {
    install_panic_hook();

    let args: Vec<String> = std::env::args().collect();
    if handle_cli_args(&args) {
        return Ok(());
    }

    let port: u16 = std::env::var("TRAFFIC_STATUS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8765);

    // Test if already running before opening window
    if cli::is_daemon_alive(port) {
        println!("🚦 Traffic Status daemon is already running on port {port}.");
        return Ok(());
    }

    let (tx, rx): (Sender<IpcCommand>, Receiver<IpcCommand>) = unbounded();

    // Start background Tokio IPC server
    let server_tx = tx.clone();
    std::thread::Builder::new()
        .name("ipc-server".to_string())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                if let Err(e) = start_ipc_server(port, server_tx).await {
                    eprintln!("[Traffic Status] IPC Server error on port {port}: {e}");
                }
            });
        })
        .expect("Failed to spawn IPC thread");

    // Eframe window options: Authentic vertical floating widget at top-right corner
    // Outer dimensions: 104x280px to accommodate 72px aperture lamps and 3D hoods
    let initial_pos = get_top_right_pos(104.0);
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([104.0, 280.0])
            .with_min_inner_size([90.0, 200.0])
            .with_position(initial_pos)
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_taskbar(false)
            .with_title("Traffic Status"),
        ..Default::default()
    };

    eframe::run_native(
        "Traffic Status",
        native_options,
        Box::new(|cc| Ok(Box::new(TrafficStatusApp::new(cc, rx)))),
    )
}

#[cfg(target_os = "windows")]
fn get_top_right_pos(window_width: f32) -> [f32; 2] {
    #[link(name = "user32")]
    extern "system" {
        fn GetSystemMetrics(n_index: i32) -> i32;
    }
    // SM_CXSCREEN = 0
    let screen_w = unsafe { GetSystemMetrics(0) };
    let margin = 28.0;
    let x = (screen_w as f32 - window_width - margin).max(0.0);
    [x, margin]
}

#[cfg(not(target_os = "windows"))]
fn get_top_right_pos(_window_width: f32) -> [f32; 2] {
    [1200.0, 28.0]
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic"
        };
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        eprintln!("🔥 [Traffic Status Panic] {payload} at {location}");
        default_hook(panic_info);
    }));
}
