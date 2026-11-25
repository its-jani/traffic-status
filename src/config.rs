//! Persistent desktop configuration: window placement, widget scale, ghost mode.
//!
//! Stored as `config.json` beside the stable binary directory
//! (`%LOCALAPPDATA%\traffic-status\config.json` on Windows,
//! `~/.config/traffic-status/config.json` elsewhere). Only [`AppConfig::load`]
//! and [`AppConfig::save`] touch the filesystem; serialization and bounds
//! clamping are pure so they can be unit tested without a real user profile.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Smallest accepted scale multiplier (laptop / high-DPI friendly).
pub const MIN_SCALE: f32 = 0.5;
/// Largest accepted scale multiplier.
pub const MAX_SCALE: f32 = 2.0;
/// Scale used when the config is missing or invalid.
pub const DEFAULT_SCALE: f32 = 1.0;

/// A rectangle in desktop points covering the usable screen estate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ScreenBounds {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Bounds spanning every connected monitor (virtual desktop).
    pub fn virtual_desktop() -> Self {
        #[cfg(target_os = "windows")]
        {
            #[link(name = "user32")]
            extern "system" {
                fn GetSystemMetrics(n_index: i32) -> i32;
            }
            const SM_XVIRTUALSCREEN: i32 = 76;
            const SM_YVIRTUALSCREEN: i32 = 77;
            const SM_CXVIRTUALSCREEN: i32 = 78;
            const SM_CYVIRTUALSCREEN: i32 = 79;
            let (x, y, w, h) = unsafe {
                (
                    GetSystemMetrics(SM_XVIRTUALSCREEN),
                    GetSystemMetrics(SM_YVIRTUALSCREEN),
                    GetSystemMetrics(SM_CXVIRTUALSCREEN),
                    GetSystemMetrics(SM_CYVIRTUALSCREEN),
                )
            };
            if w > 0 && h > 0 {
                return Self::new(x as f32, y as f32, w as f32, h as f32);
            }
        }
        Self::new(0.0, 0.0, 1920.0, 1080.0)
    }
}

/// Remembered top-left corner of the floating capsule.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct WindowPlacement {
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub y: Option<f32>,
}

impl WindowPlacement {
    pub fn from_pos(x: f32, y: f32) -> Self {
        Self {
            x: Some(x),
            y: Some(y),
        }
    }

    /// Returns the stored point when both coordinates are present and finite.
    pub fn as_pos(&self) -> Option<(f32, f32)> {
        match (self.x, self.y) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Some((x, y)),
            _ => None,
        }
    }
}

/// Persisted user preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub window: WindowPlacement,
    pub scale: f32,
    pub ghost: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowPlacement::default(),
            scale: DEFAULT_SCALE,
            ghost: false,
        }
    }
}

impl AppConfig {
    /// Clamp every field into its accepted range.
    pub fn sanitized(mut self) -> Self {
        self.scale = clamp_scale(self.scale);
        self
    }

    /// `%LOCALAPPDATA%\traffic-status\config.json` on Windows,
    /// `$XDG_CONFIG_HOME/traffic-status/config.json` elsewhere.
    pub fn path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(dirs::data_local_dir);
        #[cfg(not(target_os = "windows"))]
        let base = dirs::config_dir();

        base.map(|dir| dir.join("traffic-status").join("config.json"))
    }

    /// Load the user config, falling back to defaults on any error.
    pub fn load() -> Self {
        Self::path()
            .map(|p| Self::load_from(&p))
            .unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .unwrap_or_default()
            .sanitized()
    }

    /// Persist next to the stable binary; failures are reported, never fatal.
    pub fn save(&self) -> Result<(), String> {
        match Self::path() {
            Some(path) => self.save_to(&path),
            None => Err("unable to resolve the traffic-status config directory".to_string()),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create '{}': {e}", parent.display()))?;
        }
        let mut body = serde_json::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize config: {e}"))?;
        body.push('\n');
        fs::write(path, body).map_err(|e| format!("failed to write '{}': {e}", path.display()))
    }
}

/// Clamp a scale multiplier into [`MIN_SCALE`, `MAX_SCALE`], rejecting NaN/inf.
pub fn clamp_scale(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.clamp(MIN_SCALE, MAX_SCALE)
    } else {
        DEFAULT_SCALE
    }
}

/// Keep a window of `size` inside `bounds`, so a disconnected monitor can never
/// leave the capsule unreachable off-screen.
pub fn clamp_position(pos: (f32, f32), size: (f32, f32), bounds: ScreenBounds) -> (f32, f32) {
    let max_x = (bounds.x + bounds.width - size.0).max(bounds.x);
    let max_y = (bounds.y + bounds.height - size.1).max(bounds.y);
    let x = if pos.0.is_finite() { pos.0 } else { bounds.x };
    let y = if pos.1.is_finite() { pos.1 } else { bounds.y };
    (x.clamp(bounds.x, max_x), y.clamp(bounds.y, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn coordinates_round_trip_through_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("config.json");
        let config = AppConfig {
            window: WindowPlacement::from_pos(1420.5, 36.0),
            scale: 0.75,
            ghost: true,
        };

        config.save_to(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("\"x\": 1420.5"),
            "x must be persisted: {text}"
        );
        assert!(text.contains("\"y\": 36.0"), "y must be persisted: {text}");

        assert_eq!(AppConfig::load_from(&path), config);
        assert_eq!(config.window.as_pos(), Some((1420.5, 36.0)));
    }

    #[test]
    fn missing_or_partial_files_fall_back_to_defaults() {
        let dir = tempdir().unwrap();

        let absent = AppConfig::load_from(&dir.path().join("does-not-exist.json"));
        assert_eq!(absent, AppConfig::default());

        let partial = dir.path().join("partial.json");
        fs::write(&partial, r#"{"window":{"x":10.0}}"#).unwrap();
        let loaded = AppConfig::load_from(&partial);
        assert_eq!(loaded.window.as_pos(), None);
        assert_eq!(loaded.scale, DEFAULT_SCALE);
        assert!(!loaded.ghost);
    }

    #[test]
    fn invalid_scale_is_sanitized_on_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, r#"{"scale": 9.5}"#).unwrap();
        assert_eq!(AppConfig::load_from(&path).scale, MAX_SCALE);

        fs::write(&path, r#"{"scale": -1}"#).unwrap();
        assert_eq!(AppConfig::load_from(&path).scale, MIN_SCALE);
    }

    #[test]
    fn scale_clamping_rejects_non_finite_values() {
        assert_eq!(clamp_scale(0.75), 0.75);
        assert_eq!(clamp_scale(0.1), MIN_SCALE);
        assert_eq!(clamp_scale(12.0), MAX_SCALE);
        assert_eq!(clamp_scale(f32::NAN), DEFAULT_SCALE);
        assert_eq!(clamp_scale(f32::INFINITY), DEFAULT_SCALE);
    }

    #[test]
    fn positions_stay_on_screen() {
        let bounds = ScreenBounds::new(0.0, 0.0, 1920.0, 1080.0);
        let size = (104.0, 280.0);

        assert_eq!(clamp_position((100.0, 50.0), size, bounds), (100.0, 50.0));
        assert_eq!(clamp_position((-500.0, -500.0), size, bounds), (0.0, 0.0));
        assert_eq!(
            clamp_position((5000.0, 5000.0), size, bounds),
            (1920.0 - 104.0, 1080.0 - 280.0)
        );
        // Corrupt values must not panic inside f32::clamp.
        assert_eq!(
            clamp_position((f32::NAN, f32::NAN), size, bounds),
            (0.0, 0.0)
        );
    }

    #[test]
    fn secondary_monitor_bounds_are_honoured() {
        // A monitor placed to the left of the primary screen starts at x = -1920.
        let bounds = ScreenBounds::new(-1920.0, 0.0, 3840.0, 1080.0);
        let size = (78.0, 210.0);
        assert_eq!(
            clamp_position((-1500.0, 20.0), size, bounds),
            (-1500.0, 20.0)
        );
        assert_eq!(
            clamp_position((-9000.0, 20.0), size, bounds),
            (-1920.0, 20.0)
        );
    }
}
