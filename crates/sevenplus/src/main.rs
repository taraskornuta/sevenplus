#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod extras;
mod layout;
mod paint;
mod theme;

use config::{FrameStyle, Settings};
use eframe::egui;

const ICON: &[u8] = include_bytes!("../../../assets/icons/sevenplus.png");

fn icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(ICON).unwrap_or_default()
}

fn main() -> eframe::Result {
    let settings = Settings::load();
    let (theme, theme_error) = app::load_theme(&settings);
    let aero = settings.frame == FrameStyle::Aero;
    let natural = app::window_size(&theme, &settings, settings.frame);
    let scale = settings.scale.clamp(app::MIN_SCALE, app::MAX_SCALE);
    let viewport = egui::ViewportBuilder::default()
        .with_title("Calculator")
        .with_app_id("sevenplus")
        .with_icon(icon())
        .with_inner_size(natural * scale)
        .with_min_inner_size(natural * app::MIN_SCALE)
        .with_resizable(true)
        .with_maximize_button(false)
        .with_decorations(!aero)
        .with_transparent(aero);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "sevenplus",
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::App::new(
                &cc.egui_ctx,
                settings,
                theme,
                theme_error,
            )))
        }),
    )
}
