//! Linux: X11 monitor geometry via RandR, GNOME's high-contrast flag, and message
//! boxes through zenity or kdialog. Wayland hides window positions, so no monitors are
//! reported there and the compositor places the window.

use calc_ui::{Platform, Rect};
use eframe::egui::{Pos2, Vec2};
use std::process::Command;

pub struct Linux {
    monitors: Vec<Rect>,
}

impl Linux {
    pub fn new() -> Linux {
        Linux {
            monitors: x11_monitors().unwrap_or_default(),
        }
    }
}

fn wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var("WINIT_UNIX_BACKEND").map_or(true, |b| b != "x11")
}

/// Monitor rectangles from RandR, primary first.
fn x11_monitors() -> Option<Vec<Rect>> {
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::ConnectionExt;
    if wayland() || std::env::var_os("DISPLAY").is_none() {
        return None;
    }
    let (conn, screen) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen)?.root;
    let reply = conn.randr_get_monitors(root, true).ok()?.reply().ok()?;
    let mut monitors: Vec<(bool, Rect)> = reply
        .monitors
        .iter()
        .map(|m| {
            (
                m.primary,
                Rect::from_min_size(
                    Pos2::new(m.x as f32, m.y as f32),
                    Vec2::new(m.width as f32, m.height as f32),
                ),
            )
        })
        .collect();
    monitors.sort_by_key(|(primary, _)| !primary);
    Some(monitors.into_iter().map(|(_, r)| r).collect())
}

fn gnome_high_contrast() -> bool {
    if std::env::var("GTK_THEME").is_ok_and(|t| t.to_ascii_lowercase().contains("contrast")) {
        return true;
    }
    Command::new("gsettings")
        .args(["get", "org.gnome.desktop.a11y.interface", "high-contrast"])
        .output()
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "true")
}

impl Platform for Linux {
    fn monitors(&self) -> Vec<Rect> {
        self.monitors.clone()
    }

    /// Read at startup and when the window regains focus.
    fn high_contrast(&self) -> bool {
        gnome_high_contrast()
    }
}

/// Shows an error dialog with zenity or kdialog, whichever is installed.
pub fn message_box(title: &str, text: &str) {
    let shown = Command::new("zenity")
        .args(["--error", "--no-markup", "--title", title, "--text", text])
        .status()
        .is_ok()
        || Command::new("kdialog")
            .args(["--title", title, "--error", text])
            .status()
            .is_ok();
    if !shown {
        eprintln!("{title}: {text}");
    }
}
