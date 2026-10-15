//! Linux: X11 monitor geometry via RandR, GNOME's high-contrast flag, blur behind the
//! window (Wayland `ext-background-effect`, KWin's X11 hint), and message boxes through
//! zenity or kdialog. Wayland hides window positions, so no monitors are
//! reported there and the compositor places the window.

use super::wayland_blur::WaylandBlur;
use calc_ui::{Platform, RawDisplayHandle, RawWindowHandle, Rect};
use eframe::egui::{Pos2, Vec2};
use std::cell::RefCell;
use std::process::Command;

pub struct Linux {
    monitors: Vec<Rect>,
    /// Wayland blur, set up on first use; `Some(None)` when the compositor lacks it.
    wayland_blur: RefCell<Option<Option<WaylandBlur>>>,
}

impl Linux {
    pub fn new() -> Linux {
        Linux {
            monitors: x11_monitors().unwrap_or_default(),
            wayland_blur: RefCell::new(None),
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

    #[allow(unsafe_code)]
    fn set_blur(&self, window: RawWindowHandle, display: RawDisplayHandle, on: bool) {
        let id = match (window, display) {
            (RawWindowHandle::Wayland(w), RawDisplayHandle::Wayland(d)) => {
                let mut blur = self.wayland_blur.borrow_mut();
                // SAFETY: both handles come from the live calculator window, which
                // lasts as long as the app (and so this platform object).
                let blur =
                    blur.get_or_insert_with(|| unsafe { WaylandBlur::new(d.display, w.surface) });
                match blur {
                    Some(b) => b.set(on),
                    None => tracing::debug!("the compositor offers no ext-background-effect"),
                }
                return;
            }
            (RawWindowHandle::Xlib(h), _) => h.window as u32,
            (RawWindowHandle::Xcb(h), _) => h.window.get(),
            _ => return,
        };
        if let Err(e) = x11_blur(id, on) {
            tracing::debug!("cannot set the KDE blur hint: {e}");
        }
    }
}

/// Sets or clears `_KDE_NET_WM_BLUR_BEHIND_REGION`; an empty region asks KWin to blur
/// behind the whole window. Other window managers ignore it.
fn x11_blur(window: u32, on: bool) -> Result<(), Box<dyn std::error::Error>> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
    use x11rb::wrapper::ConnectionExt as _;
    let (conn, _) = x11rb::connect(None)?;
    let atom = conn
        .intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION")?
        .reply()?
        .atom;
    if on {
        conn.change_property32(PropMode::REPLACE, window, atom, AtomEnum::CARDINAL, &[])?;
    } else {
        conn.delete_property(window, atom)?;
    }
    conn.flush()?;
    Ok(())
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
