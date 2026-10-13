//! The SevenPlus user interface, built on egui (ARC-001).
//!
//! The binary (`calc-app`) creates the window and supplies a [`Platform`] for the few
//! things egui cannot do itself: monitor geometry, the OS high-contrast setting and the
//! macOS global menu bar.

mod app;
mod dialogs;
mod display;
mod extras;
mod frame;
pub mod i18n;
mod input;
mod keypad;
pub mod layout;
pub mod menu;
mod paint;

pub use app::{App, AppInit, MAX_SCALE, MIN_SCALE, load_theme, window_size};
pub use eframe::egui::Rect;
pub use menu::{MenuCommand, MenuState};

/// OS services the UI needs beyond egui. Every method has a do-nothing default.
pub trait Platform {
    /// Work areas of the connected monitors in physical pixels; empty when the
    /// platform does not expose them (Wayland).
    fn monitors(&self) -> Vec<Rect> {
        Vec::new()
    }

    /// Whether the OS high-contrast mode is on (UI-024).
    fn high_contrast(&self) -> bool {
        false
    }

    /// True when the menu lives in the OS menu bar instead of the window (macOS).
    fn native_menu(&self) -> bool {
        false
    }

    /// Commands chosen in the native menu since the last call.
    fn poll_menu(&mut self) -> Vec<MenuCommand> {
        Vec::new()
    }

    /// Updates check marks of the native menu.
    fn sync_menu(&mut self, _state: &MenuState) {}
}

/// A platform with none of the optional services.
pub struct NoPlatform;

impl Platform for NoPlatform {}
