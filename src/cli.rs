use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{clamp_scale, AppConfig};
use crate::installer::{
    get_home_dir, get_stable_bin_path, install_global, install_project, run_doctor,
    uninstall_global, uninstall_project,
};

const DEFAULT_PORT: u16 = 8765;

pub fn handle_cli_args(args: &[String]) -> bool {
    if args.len() <= 1 {
        return false; // Launch GUI app
    }

    match args[1].as_str() {
        "install" => {
            let is_global = args.iter().any(|a| a == "--global" || a == "-g");
            let dry_run = args.iter().any(|a| a == "--dry-run");
            let force = args.iter().any(|a| a == "--force" || a == "-f");
            let project_idx = args.iter().position(|a| a == "--project" || a == "-p");

            if is_global {
                install_global(dry_run, force);
            } else if let Some(idx) = project_idx {
                let path = args.get(idx + 1).map(|s| s.as_str()).unwrap_or(".");
                install_project(Path::new(path), dry_run, force);
            } else if args.len() > 2 && !args[2].starts_with('-') {
                install_project(Path::new(&args[2]), dry_run, force);
            } else {
                install_project(Path::new("."), dry_run, force);
            }
            true
        }
        "uninstall" => {
            let is_global = args
                .iter()
                .any(|a| a == "--global" || a == "-g" || a == "--all");
            let dry_run = args.iter().any(|a| a == "--dry-run");
            let force = args.iter().any(|a| a == "--force" || a == "-f");
            let project_idx = args.iter().position(|a| a == "--project" || a == "-p");

            if is_global {
                uninstall_global(dry_run, force);
            } else if let Some(idx) = project_idx {
                let path = args.get(idx + 1).map(|s| s.as_str()).unwrap_or(".");
                uninstall_project(Path::new(path), dry_run, force);
            } else if args.len() > 2 && !args[2].starts_with('-') {
                uninstall_project(Path::new(&args[2]), dry_run, force);
            } else {
                uninstall_project(Path::new("."), dry_run, force);
            }
            true
        }
        "doctor" => {
            run_doctor();
            true
        }
        "on" => {
            let label = args
                .get(2)
                .filter(|arg| !arg.starts_with('-'))
                .map(|s| s.as_str())
                .unwrap_or("CLI-Session");
            apply_launch_flags(args);
            send_quick_command("on", label, args);
            true
        }
        "off" => {
            let port = get_configured_port();
            if is_daemon_alive(port) {
                // Only this workspace session goes away; the widget closes by
                // itself once the last session leaves.
                let _ = send_quick_command("off", "", args);
                println!("🛑 Traffic Status session dismissed.");
            } else {
                println!("ℹ️ Traffic Status is already off.");
            }
            true
        }
        "shutdown" => {
            let port = get_configured_port();
            if is_daemon_alive(port) {
                stop_daemon_if_running(false);
                println!("🛑 Traffic Status daemon stopped.");
            } else {
                println!("ℹ️ Traffic Status is already off.");
            }
            true
        }
        "ghost" => {
            set_ghost(args.get(2).map(|s| s.as_str()).unwrap_or("toggle"));
            true
        }
        "scale" => {
            set_scale(args.get(2).map(|s| s.as_str()));
            true
        }
        "green" | "yellow" | "red" => {
            let state = &args[1];
            let msg = args.get(2).map(|s| s.as_str()).unwrap_or("");
            send_quick_command(state, msg, args);
            true
        }
        "set" => {
            let state = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let msg = args.get(3).map(|s| s.as_str()).unwrap_or("");
            match state {
                "green" | "yellow" | "red" => {
                    send_quick_command(state, msg, args);
                }
                _ => {
                    eprintln!("Usage: traffic-status set <green|yellow|red> [MESSAGE]");
                }
            }
            true
        }
        "clear" => {
            let port = get_configured_port();
            if is_daemon_alive(port) {
                stop_daemon_if_running(false);
                println!("🛑 Traffic Status cleared and removed.");
            } else {
                println!("ℹ️ Traffic Status is already off.");
            }
            true
        }
        "--help" | "-h" | "help" => {
            print_help();
            true
        }
        "--version" | "-v" => {
            println!("traffic-status v{}", env!("CARGO_PKG_VERSION"));
            true
        }
        "run" | "gui" | "daemon" => {
            false // Handled by main.rs
        }
        flag if flag.starts_with('-') => {
            // Launch flags (`--scale`, `--ghost`, `--session`) only: persist
            // them and let main.rs open the window.
            apply_launch_flags(args);
            false
        }
        _ => {
            eprintln!("Unknown command: '{}'", args[1]);
            print_help();
            true
        }
    }
}

fn print_help() {
    println!("🚦 Traffic Status v{}", env!("CARGO_PKG_VERSION"));
    println!("A lightweight floating desktop traffic light for AI coding agents.\n");
    println!("USAGE:");
    println!("  traffic-status                           Launch the floating traffic status UI");
    println!("  traffic-status install --global          Install globally for all current and future projects");
    println!("  traffic-status install --project [PATH]  Install commands into a specific project");
    println!("  traffic-status uninstall --global        Remove global hooks & stable binary");
    println!("  traffic-status uninstall --project [PATH] Remove hooks from a specific project");
    println!(
        "  traffic-status doctor                    Diagnose installation, hooks, and port status"
    );
    println!("  traffic-status on [LABEL]                Activate session");
    println!("  traffic-status off                       Dismiss this workspace session");
    println!("  traffic-status shutdown                  Stop the widget and drop every session");
    println!("  traffic-status [set] <green|yellow|red> [MSG] Update status light");
    println!("  traffic-status ghost [on|off]            Toggle click-through mode (Alt to drag)");
    println!("  traffic-status scale <0.5-2.0>           Resize the widget capsule");
    println!("  traffic-status clear                     Clear all active sessions");
    println!("  traffic-status --help                    Show this help message");
    println!("  traffic-status --version                 Print version");
    println!("\nFLAGS:");
    println!("  --scale <N>                              Widget scale multiplier (default 1.0)");
    println!("  --ghost                                  Start with clicks passing through");
    println!("  --session <ID>                           Explicit session id (defaults to workspace hash)");
    println!("  --dry-run                                Preview changes without modifying files");
    println!(
        "  --force, -f                              Overwrite files even if marker is missing"
    );
}

/// Persist `--scale` / `--ghost` and push the change to a live daemon.
fn apply_launch_flags(args: &[String]) {
    let scale = flag_value(args, "--scale")
        .and_then(|value| value.parse::<f32>().ok())
        .map(clamp_scale);
    let ghost = args.iter().any(|a| a == "--ghost").then_some(true);
    if scale.is_none() && ghost.is_none() {
        return;
    }

    let mut config = AppConfig::load();
    if let Some(scale) = scale {
        config.scale = scale;
    }
    if let Some(ghost) = ghost {
        config.ghost = ghost;
    }
    if let Err(e) = config.save() {
        eprintln!("⚠️ {e}");
    }
    send_config_update(scale, ghost);
}

fn set_ghost(mode: &str) {
    let mode = mode.trim().to_lowercase();
    let mut config = AppConfig::load();
    let enabled = match mode.as_str() {
        "on" | "enable" | "true" | "yes" => true,
        "off" | "disable" | "false" | "no" => false,
        "toggle" | "" => !config.ghost,
        other => {
            eprintln!("Usage: traffic-status ghost [on|off|toggle] (got '{other}')");
            return;
        }
    };

    config.ghost = enabled;
    if let Err(e) = config.save() {
        eprintln!("⚠️ {e}");
    }
    send_config_update(None, Some(enabled));

    if enabled {
        println!("👻 Ghost mode enabled — clicks pass through. Hold Alt to drag.");
    } else {
        println!("🖱️ Ghost mode disabled — the capsule accepts clicks again.");
    }
}

fn set_scale(raw: Option<&str>) {
    let Some(raw) = raw else {
        let current = AppConfig::load().scale;
        println!("🔍 Current widget scale: {current:.2}x");
        println!("Usage: traffic-status scale <0.5-2.0>");
        return;
    };
    let Ok(value) = raw.parse::<f32>() else {
        eprintln!("Usage: traffic-status scale <0.5-2.0> (got '{raw}')");
        return;
    };
    let scale = clamp_scale(value);
    let mut config = AppConfig::load();
    config.scale = scale;
    if let Err(e) = config.save() {
        eprintln!("⚠️ {e}");
    }
    send_config_update(Some(scale), None);
    println!("🔍 Widget scale set to {scale:.2}x");
}

/// Read `--name value` or `--name=value` from the argument list.
fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let with_equals = format!("{name}=");
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == name {
            return iter.next().map(String::as_str);
        }
        if let Some(value) = arg.strip_prefix(with_equals.as_str()) {
            return Some(value);
        }
    }
    None
}

/// Unique, stable identifier for this terminal/workspace session.
///
/// Precedence: `TRAFFIC_STATUS_SESSION` > `--session ID` >
/// `cli-<fnv1a(workspace path)>`, so concurrent agents in different projects
/// never overwrite each other's light.
pub fn resolve_session_id(args: &[String]) -> String {
    if let Ok(id) = std::env::var("TRAFFIC_STATUS_SESSION") {
        if let Some(clean) = sanitize_session_id(&id) {
            return clean;
        }
    }
    if let Some(clean) = flag_value(args, "--session").and_then(sanitize_session_id) {
        return clean;
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = cwd.to_string_lossy().to_lowercase();
    format!("cli-{:08x}", fnv1a(path.as_bytes()))
}

fn sanitize_session_id(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .take(64)
        .collect();
    (!cleaned.is_empty()).then_some(cleaned)
}

fn fnv1a(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

pub fn stop_daemon_if_running(dry_run: bool) {
    let port = get_configured_port();
    let addr = socket_addr(port);
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(300)) {
        Ok(s) => s,
        Err(_) => return,
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    let ping_req = format!(
        "GET /ping HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        port
    );
    if stream.write_all(ping_req.as_bytes()).is_err() {
        return;
    }

    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    let resp_str = String::from_utf8_lossy(&response);

    // Verify it is our traffic-status app
    if !resp_str.contains("HTTP/1.1 200")
        || (!resp_str.contains("traffic-status") && !resp_str.contains("traffic-light"))
    {
        return;
    }

    if dry_run {
        println!("  [dry-run] Would stop running traffic-status daemon on port {port}");
        return;
    }

    // Send POST /shutdown
    let mut shutdown_stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
        Ok(s) => s,
        Err(_) => return,
    };

    let shutdown_req = format!(
        "POST /shutdown HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}",
        port
    );
    let _ = shutdown_stream.write_all(shutdown_req.as_bytes());
    let mut resp = Vec::new();
    let _ = shutdown_stream.read_to_end(&mut resp);

    // Wait up to 1.5s for process to exit
    for _ in 0..15 {
        std::thread::sleep(Duration::from_millis(100));
        if !is_daemon_alive(port) {
            println!("  🛑 Stopped running daemon on port {port}");
            return;
        }
    }
}

fn socket_addr(port: u16) -> SocketAddr {
    format!("127.0.0.1:{port}")
        .parse()
        .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT)))
}

pub fn get_configured_port() -> u16 {
    std::env::var("TRAFFIC_STATUS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

pub fn is_daemon_alive(port: u16) -> bool {
    let addr = socket_addr(port);
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(300)) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    let req = format!(
        "GET /ping HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        port
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }

    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    let resp_str = String::from_utf8_lossy(&response);
    resp_str.contains("HTTP/1.1 200")
        && (resp_str.contains("traffic-status") || resp_str.contains("traffic-light"))
}

pub fn ensure_daemon_running() -> bool {
    let port = get_configured_port();
    if is_daemon_alive(port) {
        return true;
    }

    // Locate candidate executables
    let mut candidates = Vec::new();
    if let Ok(cur) = std::env::current_exe() {
        candidates.push(cur);
    }
    if let Some(stable) = get_stable_bin_path() {
        candidates.push(stable);
    }
    if let Some(home) = get_home_dir() {
        let bin_name = if cfg!(windows) {
            "traffic-status.exe"
        } else {
            "traffic-status"
        };
        candidates.push(home.join(".cargo").join("bin").join(bin_name));
    }

    for exe in candidates {
        if !exe.exists() {
            continue;
        }

        let mut cmd = std::process::Command::new(&exe);
        cmd.arg("daemon");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        if cmd.spawn().is_ok() {
            // Poll for daemon readiness
            for _ in 0..15 {
                std::thread::sleep(Duration::from_millis(100));
                if is_daemon_alive(port) {
                    return true;
                }
            }
        }
    }

    false
}

/// Push a `/config` update to a running daemon (best effort, never spawns one).
pub fn send_config_update(scale: Option<f32>, ghost: Option<bool>) {
    let port = get_configured_port();
    let mut fields = Vec::new();
    if let Some(scale) = scale {
        fields.push(format!("\"scale\":{scale}"));
    }
    if let Some(ghost) = ghost {
        fields.push(format!("\"ghost\":{ghost}"));
    }
    if fields.is_empty() || !is_daemon_alive(port) {
        return;
    }
    let payload = format!("{{{}}}", fields.join(","));
    let _ = post_json(port, "/config", &payload);
}

pub fn send_quick_command(action: &str, msg: &str, args: &[String]) -> bool {
    let port = get_configured_port();
    let session_id = resolve_session_id(args);
    let (endpoint, payload) = match action {
        "on" => {
            let label = if msg.is_empty() { "CLI-Session" } else { msg };
            (
                "/session/on",
                format!(
                    r#"{{"session_id":"{}","label":"{}","source":"{}","state":"green"}}"#,
                    session_id,
                    json_escape(label),
                    json_escape(label)
                ),
            )
        }
        "off" => (
            "/session/off",
            format!(r#"{{"session_id":"{session_id}"}}"#),
        ),
        "clear" => ("/clear", "{}".to_string()),
        "green" | "yellow" | "red" => (
            "/state",
            format!(
                r#"{{"session_id":"{}","state":"{}","message":"{}"}}"#,
                session_id,
                action,
                json_escape(msg)
            ),
        ),
        _ => return false,
    };

    if !ensure_daemon_running() {
        eprintln!("⚠️ Unable to start or reach Traffic Status daemon on port {port}.");
        return false;
    }

    if post_json(port, endpoint, &payload) {
        println!("✨ Traffic Status [{action}] updated successfully.");
        true
    } else {
        eprintln!("⚠️ Daemon did not accept the [{action}] update.");
        false
    }
}

fn post_json(port: u16, endpoint: &str, payload: &str) -> bool {
    let addr = socket_addr(port);
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(1000)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to connect to daemon: {e}");
            return false;
        }
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(1000)));
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        endpoint,
        port,
        payload.len(),
        payload
    );

    if let Err(e) = stream.write_all(req.as_bytes()) {
        eprintln!("❌ Failed to send command: {e}");
        return false;
    }

    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    String::from_utf8_lossy(&response).contains("HTTP/1.1 200")
}
