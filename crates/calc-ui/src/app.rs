//! The calculator window: state, theme handling, window size and the frame loop.

use crate::extras::ExtrasState;
use crate::layout;
use crate::menu::MenuCommand;
use crate::paint;
use crate::{Platform, i18n::tr};
use calc_config::Settings;
use calc_core::{Calculator, Key, Mode};
use calc_format::Locale;
use calc_theme::{Backdrop, Color, Theme, ThemeStore};
use eframe::egui::{
    self, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect, Stroke,
    TextStyle, UiBuilder, Vec2, ViewportCommand, vec2,
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;
use std::time::SystemTime;

const INTER: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-Bold.ttf");
/// Custom font files larger than this are ignored.
const MAX_FONT_SIZE: u64 = 32 * 1024 * 1024;
/// Pressed keys flash their button at least this long (KEY-005).
pub(crate) const FLASH_SECS: f64 = 0.12;
/// Limits of the UI scale set by resizing the window or zooming.
pub const MIN_SCALE: f32 = 0.75;
pub const MAX_SCALE: f32 = 4.0;
/// How long to wait for the window manager to apply a requested size.
const RESIZE_WAIT_SECS: f64 = 1.0;
/// Settings are written at most this often while they keep changing.
const SAVE_INTERVAL_SECS: f64 = 1.0;

/// What the binary hands to the UI.
pub struct AppInit {
    pub settings: Settings,
    /// Problem found while loading settings, shown once.
    pub settings_warning: Option<String>,
    /// OS locale tag, e.g. `de-DE` (NFR-020).
    pub locale_tag: String,
    /// Third-party license texts (NFR-042).
    pub licenses: &'static str,
    pub platform: Box<dyn Platform>,
}

/// Window inner size for the given theme and settings, at scale 1.0.
pub fn window_size(t: &Theme, s: &Settings, native_menu: bool) -> Vec2 {
    let history = s.history && matches!(s.mode, Mode::Standard | Mode::Scientific);
    layout::compute(t, s.mode, history, s.extras, !native_menu).size
}

/// Loads the theme the settings ask for, falling back to the default (THM-004).
pub fn load_theme(store: &ThemeStore, id: &str) -> (Theme, Option<String>) {
    match store.load(id) {
        Ok(t) => (t, None),
        Err(e) => {
            tracing::warn!("{e}; using the default theme");
            (Theme::builtin(), Some(e.to_string()))
        }
    }
}

pub struct App {
    pub(crate) calc: Calculator,
    pub(crate) settings: Settings,
    saved: Settings,
    last_save: f64,
    pub(crate) store: ThemeStore,
    pub(crate) theme: Theme,
    pub(crate) theme_id: String,
    pub(crate) theme_error: Option<String>,
    pub(crate) settings_warning: Option<String>,
    theme_mtime: Option<SystemTime>,
    os_dark: bool,
    os_high_contrast: bool,
    /// The OS asks apps to reduce transparency.
    os_reduce_transparency: bool,
    /// Blur last requested for the window, to call the OS only on change.
    blur_applied: Option<bool>,
    focused: bool,
    last_size: Option<Vec2>,
    /// Requested physical window size and when it was requested; the scale is frozen
    /// until the window reaches it so a mode switch doesn't change the zoom.
    pending_size: Option<(Vec2, f64)>,
    pub(crate) extras: ExtrasState,
    pub(crate) about_open: bool,
    pub(crate) licenses_open: bool,
    pub(crate) prefs_open: bool,
    /// Programmer toolbar text field contents.
    pub(crate) chars_text: String,
    pub(crate) stat_selected: Option<usize>,
    pub(crate) flash: Option<(Key, f64)>,
    pub(crate) os_locale: Locale,
    pub(crate) licenses: &'static str,
    pub(crate) platform: Box<dyn Platform>,
    /// Last announced display text, for the screen-reader live region (UI-021).
    pub(crate) announced: String,
    position_checked: bool,
    #[cfg(feature = "screenshot")]
    screenshot: Option<(String, u32)>,
}

impl App {
    pub fn new(ctx: &Context, init: AppInit) -> Self {
        let AppInit {
            settings,
            settings_warning,
            locale_tag,
            licenses,
            platform,
        } = init;
        let mut calc = Calculator::new();
        calc.set_mode(settings.mode);
        calc.angle = settings.angle;
        calc.base = settings.base;
        calc.set_word(settings.word);
        let store = ThemeStore::new(Some(calc_config::themes_dir()));
        let os_dark = ctx.system_theme() == Some(egui::Theme::Dark);
        let os_high_contrast = platform.high_contrast();
        let theme_id = settings.theme_id(os_dark, os_high_contrast);
        let (theme, theme_error) = load_theme(&store, &theme_id);
        egui_extras::install_image_loaders(ctx);
        let mut app = App {
            calc,
            saved: settings.clone(),
            last_save: 0.0,
            settings,
            store,
            theme,
            theme_id,
            theme_error,
            settings_warning,
            theme_mtime: None,
            os_dark,
            os_high_contrast,
            os_reduce_transparency: platform.reduce_transparency(),
            blur_applied: None,
            focused: true,
            last_size: None,
            pending_size: None,
            extras: ExtrasState::default(),
            about_open: false,
            licenses_open: false,
            prefs_open: false,
            chars_text: String::new(),
            stat_selected: None,
            flash: None,
            os_locale: Locale::from_tag(&locale_tag),
            licenses,
            platform,
            announced: String::new(),
            position_checked: false,
            #[cfg(feature = "screenshot")]
            screenshot: std::env::var("SEVENPLUS_SCREENSHOT").ok().map(|p| (p, 0)),
        };
        app.theme_mtime = app.active_mtime();
        app.apply_theme(ctx);
        // The window size owns the zoom; Ctrl +/-/0 are handled by `zoom_by`.
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        ctx.set_zoom_factor(app.settings.scale.clamp(MIN_SCALE, MAX_SCALE));
        app
    }

    /// Separators in effect: the OS locale with the user's overrides (NFR-020).
    pub(crate) fn locale(&self) -> Locale {
        let decimal = self
            .settings
            .decimal_separator
            .unwrap_or(self.os_locale.decimal);
        let mut group = self
            .settings
            .group_separator
            .unwrap_or(self.os_locale.group);
        if group == decimal {
            // Identical separators would make numbers ambiguous.
            group = if decimal == ',' { '.' } else { ',' };
        }
        Locale { decimal, group }
    }

    fn active_mtime(&self) -> Option<SystemTime> {
        self.store
            .user_path(&self.theme_id)
            .and_then(|p| std::fs::metadata(p).ok())
            .and_then(|m| m.modified().ok())
    }

    fn read_font(&mut self, bold: bool) -> Option<Vec<u8>> {
        let p = self.theme.font_path(&calc_config::themes_dir(), bold)?;
        let result = std::fs::metadata(&p)
            .map_err(|e| e.to_string())
            .and_then(|m| {
                if m.len() > MAX_FONT_SIZE {
                    Err(format!("larger than {MAX_FONT_SIZE} bytes"))
                } else {
                    std::fs::read(&p).map_err(|e| e.to_string())
                }
            });
        match result {
            Ok(b) => Some(b),
            Err(e) => {
                let msg = format!("font {}: {e}", p.display());
                tracing::warn!("{msg}");
                self.theme_error = Some(msg);
                None
            }
        }
    }

    /// Installs the theme's fonts and maps its colors onto egui's widget styles.
    pub(crate) fn apply_theme(&mut self, ctx: &Context) {
        let mut fonts = FontDefinitions::default();
        // A theme font that is missing falls back to the bundled Inter (THM-011).
        let regular = match self.read_font(false) {
            Some(b) => FontData::from_owned(b),
            None => FontData::from_static(INTER),
        };
        let bold = match self.read_font(true) {
            Some(b) => FontData::from_owned(b),
            None => FontData::from_static(INTER_BOLD),
        };
        fonts.font_data.insert("ui".into(), Arc::new(regular));
        fonts.font_data.insert(paint::BOLD.into(), Arc::new(bold));
        for fam in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(fam)
                .or_default()
                .insert(0, "ui".into());
        }
        let fallback = fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        let mut bold_family = vec![paint::BOLD.to_string()];
        bold_family.extend(fallback);
        fonts
            .families
            .insert(FontFamily::Name(paint::BOLD.into()), bold_family);
        ctx.set_fonts(fonts);

        let t = &self.theme;
        let c = &t.controls;
        let col = paint::c;
        let bg = col(t.window.background.first());
        let dark = t.is_dark();
        let ui_size = t.font.ui.size;
        ctx.all_styles_mut(|style| {
            let mut v = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            v.override_text_color = Some(col(c.text));
            v.window_fill = col(t.menu.popup_background);
            v.panel_fill = bg;
            v.window_stroke = Stroke::new(1.0, col(t.menu.popup_border));
            v.extreme_bg_color = col(c.background);
            v.text_edit_bg_color = Some(col(c.background));
            v.selection.bg_fill = col(t.display.selection);
            v.selection.stroke = Stroke::new(1.0, col(t.display.selection_text));
            for (w, fill, stroke) in [
                (&mut v.widgets.inactive, c.background, c.border),
                (&mut v.widgets.hovered, c.hover, c.accent),
                (&mut v.widgets.active, c.hover, c.accent),
                (&mut v.widgets.open, c.hover, c.accent),
            ] {
                w.bg_fill = col(fill);
                w.weak_bg_fill = col(fill);
                w.bg_stroke = Stroke::new(1.0, col(stroke));
                w.fg_stroke = Stroke::new(1.0, col(c.text));
            }
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, col(c.group_border));
            v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, col(c.text));
            style.visuals = v;
            for ts in [TextStyle::Body, TextStyle::Button, TextStyle::Monospace] {
                style.text_styles.insert(ts, FontId::proportional(ui_size));
            }
            style
                .text_styles
                .insert(TextStyle::Small, FontId::proportional(ui_size * 0.85));
            style
                .text_styles
                .insert(TextStyle::Heading, FontId::proportional(ui_size * 1.4));
        });
        self.last_size = None;
    }

    /// Re-selects and reloads the theme when the selection, the OS light/dark or
    /// high-contrast state or the active file changed (THM-002, THM-003).
    pub(crate) fn refresh_theme(&mut self, ctx: &Context, force: bool) {
        let id = self.settings.theme_id(self.os_dark, self.os_high_contrast);
        let mtime = if id == self.theme_id {
            self.active_mtime()
        } else {
            None
        };
        if !force && id == self.theme_id && mtime == self.theme_mtime {
            return;
        }
        self.theme_id = id;
        let (theme, error) = load_theme(&self.store, &self.theme_id);
        self.theme = theme;
        self.theme_error = error;
        self.theme_mtime = self.active_mtime();
        self.apply_theme(ctx);
    }

    /// Watches OS light/dark, high contrast and window focus. Nothing here schedules
    /// repaints, so the app idles at 0% CPU (NFR-003).
    fn watch_environment(&mut self, ctx: &Context) {
        let dark = ctx
            .system_theme()
            .map_or(self.os_dark, |t| t == egui::Theme::Dark);
        let focused = ctx.input(|i| i.viewport().focused).unwrap_or(true);
        let regained = focused && !self.focused;
        self.focused = focused;
        let mut changed = dark != self.os_dark;
        self.os_dark = dark;
        if regained || !self.position_checked {
            let hc = self.platform.high_contrast();
            changed |= hc != self.os_high_contrast;
            self.os_high_contrast = hc;
            self.os_reduce_transparency = self.platform.reduce_transparency();
        }
        if changed || regained {
            // Theme files edited while the window was in the background apply now.
            self.refresh_theme(ctx, false);
        }
    }

    pub(crate) fn press(&mut self, key: Key, now: f64) {
        // STA-003: C with a selected statistics item deletes that item.
        if key == Key::Clear
            && self.calc.mode == Mode::Statistics
            && let Some(i) = self.stat_selected.take()
        {
            self.calc.stat_delete(i);
            self.flash = Some((key, now));
            return;
        }
        self.calc.apply_key(key);
        self.flash = Some((key, now));
    }

    /// Whether the window is see-through and blurred: a glass theme
    /// (`backdrop = "system"`) with transparency on and not reduced by the OS.
    pub(crate) fn glass(&self) -> bool {
        self.theme.backdrop == Backdrop::System
            && self.settings.transparency
            && !self.os_reduce_transparency
    }

    /// The window tint: its alpha scaled by the opacity setting for glass, otherwise
    /// opaque so a glass theme still works as a solid one.
    fn window_background(&self) -> Vec<Color> {
        let glass = self.glass();
        self.theme
            .window
            .background
            .stops()
            .iter()
            .map(|&Color([r, g, b, a])| {
                let a = if glass {
                    (a as f32 * self.settings.opacity).round() as u8
                } else {
                    255
                };
                Color([r, g, b, a])
            })
            .collect()
    }

    /// Asks the OS to blur what is behind the window while glass is on. winit covers
    /// macOS and older KWin; the platform covers the rest.
    fn sync_blur(&mut self, frame: &eframe::Frame) {
        let on = self.glass();
        if self.blur_applied == Some(on) {
            return;
        }
        let Some(window) = frame.winit_window() else {
            return;
        };
        window.set_blur(on);
        if let (Ok(w), Ok(d)) = (window.window_handle(), window.display_handle()) {
            self.platform.set_blur(w.as_raw(), d.as_raw(), on);
        }
        self.blur_applied = Some(on);
    }

    pub(crate) fn set_mode(&mut self, mode: Mode) {
        self.calc.set_mode(mode);
        self.stat_selected = None;
    }

    /// Applies a menu command from the in-window or native menu.
    pub(crate) fn command(&mut self, ctx: &Context, cmd: MenuCommand) {
        match cmd {
            MenuCommand::Mode(m) => self.set_mode(m),
            MenuCommand::ToggleHistory => {
                if matches!(self.calc.mode, Mode::Standard | Mode::Scientific) {
                    self.settings.history = !self.settings.history;
                }
            }
            MenuCommand::ToggleGrouping => {
                self.settings.digit_grouping = !self.settings.digit_grouping;
            }
            MenuCommand::Extras(e) => self.settings.extras = e,
            MenuCommand::Theme(id) => {
                self.settings.theme = id;
                self.refresh_theme(ctx, true);
            }
            MenuCommand::ReloadThemes => self.refresh_theme(ctx, true),
            MenuCommand::OpenThemesFolder => {
                let dir = calc_config::themes_dir();
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    tracing::warn!("cannot create {}: {e}", dir.display());
                }
                open_path(&dir);
            }
            MenuCommand::OpenSettingsFolder => open_path(&calc_config::config_dir()),
            MenuCommand::ZoomIn => self.zoom_by(ctx, 1.1),
            MenuCommand::ZoomOut => self.zoom_by(ctx, 1.0 / 1.1),
            MenuCommand::ResetSize => {
                ctx.set_zoom_factor(1.0);
                self.settings.scale = 1.0;
                self.last_size = None;
            }
            MenuCommand::Copy => ctx.copy_text(self.copy_text()),
            MenuCommand::Paste => ctx.send_viewport_cmd(ViewportCommand::RequestPaste),
            MenuCommand::CopyHistory => {
                let text: Vec<String> = self
                    .calc
                    .history()
                    .iter()
                    .map(|h| format!("{} = {}", h.expr, self.calc.format_num(&h.result)))
                    .collect();
                ctx.copy_text(text.join("\n"));
            }
            MenuCommand::ClearHistory => self.calc.clear_history(),
            MenuCommand::Preferences => self.prefs_open = true,
            MenuCommand::Help => ctx.open_url(egui::OpenUrl::new_tab(env!("CARGO_PKG_REPOSITORY"))),
            MenuCommand::About => self.about_open = true,
            MenuCommand::Licenses => self.licenses_open = true,
            MenuCommand::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
        }
    }

    /// Multiplies the UI scale (DPI-009): the window grows or shrinks to match.
    pub(crate) fn zoom_by(&mut self, ctx: &Context, factor: f32) {
        let k = (ctx.zoom_factor() * factor).clamp(MIN_SCALE, MAX_SCALE);
        ctx.set_zoom_factor(k);
        self.settings.scale = (k * 100.0).round() / 100.0;
        self.last_size = None;
    }

    /// Keeps the zoom factor in step with the window size, so resizing scales the whole UI
    /// uniformly. `natural` is the window size at scale 1.0.
    fn update_scale(&mut self, ctx: &Context, natural: Vec2) {
        let now = ctx.input(|i| i.time);
        if self.last_size != Some(natural) {
            self.request_size(ctx, natural, now);
        }
        // `viewport_rect` rather than `viewport().inner_rect`: the latter needs the window
        // position, which Wayland never reports.
        let (size, native) = ctx.input(|i| {
            (
                i.viewport_rect().size(),
                i.viewport().native_pixels_per_point,
            )
        });
        let Some(native) = native else {
            return;
        };
        let phys = size * ctx.pixels_per_point();
        if let Some((target, since)) = self.pending_size {
            if (phys - target).length() > 2.0 && now - since < RESIZE_WAIT_SECS {
                ctx.request_repaint();
                return;
            }
            self.pending_size = None;
            self.keep_on_monitor(ctx);
        }
        let k = (phys.x / (native * natural.x))
            .min(phys.y / (native * natural.y))
            .clamp(MIN_SCALE, MAX_SCALE);
        if (k - ctx.zoom_factor()).abs() > 0.001 {
            ctx.set_zoom_factor(k);
            self.settings.scale = (k * 100.0).round() / 100.0;
        }
    }

    /// Resizes the window to `natural` at the current scale (after a mode or theme
    /// change). Window managers keep the top-left corner in place (UI-003).
    fn request_size(&mut self, ctx: &Context, natural: Vec2, now: f64) {
        let k = ctx.zoom_factor();
        ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(natural * MIN_SCALE / k));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(natural));
        self.last_size = Some(natural);
        let native = ctx
            .input(|i| i.viewport().native_pixels_per_point)
            .unwrap_or(1.0);
        self.pending_size = Some((natural * k * native, now));
    }

    /// Moves the window back inside its monitor after it grew (UI-003). Only possible
    /// where the platform reports window positions (not Wayland).
    fn keep_on_monitor(&self, ctx: &Context) {
        let Some(outer) = ctx.input(|i| i.viewport().outer_rect) else {
            return;
        };
        let ppp = ctx.pixels_per_point();
        let phys = Rect::from_min_max(
            (outer.min.to_vec2() * ppp).to_pos2(),
            (outer.max.to_vec2() * ppp).to_pos2(),
        );
        let monitors = self.platform.monitors();
        let Some(m) = monitors.iter().find(|m| m.contains(phys.min)).or_else(|| {
            monitors
                .iter()
                .max_by_key(|m| m.intersect(phys).area() as i64)
        }) else {
            return;
        };
        let x = phys.min.x.min(m.max.x - phys.width()).max(m.min.x);
        let y = phys.min.y.min(m.max.y - phys.height()).max(m.min.y);
        if (x - phys.min.x).abs() > 0.5 || (y - phys.min.y).abs() > 0.5 {
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(x / ppp, y / ppp)));
        }
    }

    /// Records the window position for the next start (UI-006).
    fn track_position(&mut self, ctx: &Context) {
        let (outer, native) = ctx.input(|i| {
            (
                i.viewport().outer_rect,
                i.viewport().native_pixels_per_point,
            )
        });
        if let (Some(outer), Some(native)) = (outer, native) {
            let p = outer.min.to_vec2() * ctx.pixels_per_point();
            let pos = [p.x.round(), p.y.round()];
            if self.settings.window_pos != Some(pos) {
                self.settings.window_pos = Some(pos);
                self.settings.window_ppp = Some(native);
            }
        }
        self.position_checked = true;
    }

    fn sync_settings(&mut self, now: f64) {
        let s = &mut self.settings;
        s.mode = self.calc.mode;
        s.angle = self.calc.angle;
        s.base = self.calc.base;
        s.word = self.calc.word;
        if *s != self.saved && now - self.last_save >= SAVE_INTERVAL_SECS {
            s.save();
            self.saved = s.clone();
            self.last_save = now;
        }
    }

    pub(crate) fn copy_text(&self) -> String {
        let state = self.calc.state();
        if let Some(e) = state.error {
            return crate::i18n::error_text(e);
        }
        let grouping = self.settings.digit_grouping && self.settings.copy_grouping;
        let mut locale = self.locale();
        if !self.settings.copy_locale_decimal {
            locale.decimal = '.';
        }
        if locale.decimal == locale.group {
            locale.group = if locale.decimal == ',' { '.' } else { ',' };
        }
        state.text(grouping, &locale)
    }

    fn draw_notice(&mut self, ui: &egui::Ui, win: Rect) {
        let (title, text) = if let Some(e) = &self.theme_error {
            (tr("theme-error"), e.clone())
        } else if let Some(e) = &self.settings_warning {
            (tr("settings-error"), e.clone())
        } else {
            return;
        };
        let p = ui.painter();
        let galley = p.layout(
            format!("{title}\n{text}"),
            FontId::proportional(11.0),
            Color32::WHITE,
            win.width() - 16.0,
        );
        let r = Rect::from_min_size(
            egui::pos2(win.left() + 4.0, win.bottom() - galley.size().y - 12.0),
            vec2(win.width() - 8.0, galley.size().y + 8.0),
        );
        p.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(150, 20, 20, 235));
        p.galley(r.min + vec2(4.0, 4.0), galley, Color32::WHITE);
        if ui
            .interact(r, egui::Id::new("notice"), egui::Sense::click())
            .clicked()
        {
            if self.theme_error.is_some() {
                self.theme_error = None;
            } else {
                self.settings_warning = None;
            }
        }
    }
}

/// Opens a folder in the platform file manager.
pub(crate) fn open_path(path: &std::path::Path) {
    let cmd = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(cmd).arg(path).spawn() {
        tracing::warn!("cannot open {}: {e}", path.display());
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        if self.glass() {
            return [0.0; 4];
        }
        egui::Rgba::from(paint::c(self.window_background()[0])).to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.watch_environment(&ctx);
        for cmd in self.platform.poll_menu() {
            self.command(&ctx, cmd);
        }
        self.handle_keys(&ctx);
        let native_menu = self.platform.native_menu();
        let history =
            self.settings.history && matches!(self.calc.mode, Mode::Standard | Mode::Scientific);
        let l = layout::compute(
            &self.theme,
            self.calc.mode,
            history,
            self.settings.extras,
            !native_menu,
        );
        let want = window_size(
            &self.theme,
            &Settings {
                mode: self.calc.mode,
                ..self.settings.clone()
            },
            native_menu,
        );
        self.update_scale(&ctx, want);

        let client = ui.max_rect();
        self.sync_blur(frame);
        paint::gradient_rect(ui.painter(), client, 0.0, &self.window_background());
        self.draw_imagery(ui, client);
        // When the window's aspect differs from the calculator's, center the body below a
        // full-width menu bar.
        let spare = ((client.size() - l.size) / 2.0).max(Vec2::ZERO);
        let off = client.min.to_vec2() + spare;
        let menu = Rect::from_min_size(client.min, vec2(client.width(), l.menu.height()));

        self.draw_display(ui, l.display.translate(off), history);
        let radios: Vec<Rect> = l.radios.iter().map(|r| r.translate(off)).collect();
        self.draw_radios(ui, &radios);
        if let Some(r) = l.tools {
            self.draw_tools(ui, r.translate(off));
        }
        if let Some(b) = l.bits {
            self.draw_bits(ui, b.translate(off));
        }
        self.draw_keypads(ui, &l, off);
        if let Some(ex) = l.extras {
            let r = ex.translate(off);
            let pad = self.theme.spacing.padding;
            ui.painter().line_segment(
                [
                    r.left_top() - vec2(pad / 2.0, 0.0),
                    r.left_bottom() - vec2(pad / 2.0, 0.0),
                ],
                Stroke::new(1.0, paint::c(self.theme.controls.group_border)),
            );
            let which = self.settings.extras;
            let grouping = self.settings.digit_grouping;
            let locale = self.locale();
            ui.scope_builder(UiBuilder::new().max_rect(r.shrink2(vec2(4.0, 0.0))), |ui| {
                crate::extras::show(ui, which, &mut self.extras, grouping, &locale);
            });
        }
        if !native_menu {
            self.draw_menu(ui, menu);
        }
        self.about(&ctx);
        self.licenses_window(&ctx);
        self.preferences(&ctx);
        self.draw_notice(ui, client);
        if self.flash.is_some_and(|(_, t)| now - t < FLASH_SECS) {
            ctx.request_repaint();
        }
        self.track_position(&ctx);
        self.platform.sync_menu(&crate::menu::MenuState::of(self));
        self.sync_settings(now);
        #[cfg(feature = "screenshot")]
        self.screenshot(&ctx);
    }

    fn on_exit(&mut self) {
        if self.settings != self.saved {
            self.settings.save();
        }
    }
}

impl App {
    /// Optional theme SVG over the window background (THM-005, DPI-005).
    fn draw_imagery(&self, ui: &egui::Ui, rect: Rect) {
        let Some(file) = &self.theme.imagery.window else {
            return;
        };
        let path = calc_config::themes_dir().join(file);
        let ok = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
            && std::fs::metadata(&path).is_ok_and(|m| m.len() <= calc_theme::MAX_FILE_SIZE);
        if ok {
            egui::Image::new(format!("file://{}", path.display())).paint_at(ui, rect);
        }
    }
}

#[cfg(feature = "screenshot")]
impl App {
    /// Dev aid: saves a PNG of the window after a few frames, then exits.
    fn screenshot(&mut self, ctx: &Context) {
        use calc_core::keys::key_for_char;
        let Some((_, frames)) = &mut self.screenshot else {
            return;
        };
        *frames += 1;
        if *frames == 3 {
            // Optional scripted input: SEVENPLUS_SCREENSHOT_MODE=programmer,
            // SEVENPLUS_SCREENSHOT_INPUT="2+3=", SEVENPLUS_SCREENSHOT_THEME=dark.
            if let Some(m) = std::env::var("SEVENPLUS_SCREENSHOT_MODE")
                .ok()
                .and_then(|m| toml::Value::String(m).try_into().ok())
            {
                self.set_mode(m);
            }
            if let Ok(id) = std::env::var("SEVENPLUS_SCREENSHOT_THEME") {
                self.command(ctx, MenuCommand::Theme(id));
            }
            if let Ok(e) = std::env::var("SEVENPLUS_SCREENSHOT_EXTRAS")
                && let Ok(e) = toml::Value::String(e).try_into()
            {
                self.settings.extras = e;
            }
            if std::env::var("SEVENPLUS_SCREENSHOT_HISTORY").is_ok() {
                self.settings.history = true;
            }
            // SEVENPLUS_SCREENSHOT_RESIZE="WxH" simulates the user resizing the window.
            if let Some((w, h)) = std::env::var("SEVENPLUS_SCREENSHOT_RESIZE")
                .ok()
                .and_then(|s| s.split_once('x').map(|(w, h)| (w.parse(), h.parse())))
                .and_then(|(w, h)| Some((w.ok()?, h.ok()?)))
            {
                let ppp = ctx.pixels_per_point();
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(w, h) / ppp));
            }
            for ch in std::env::var("SEVENPLUS_SCREENSHOT_INPUT")
                .unwrap_or_default()
                .chars()
            {
                if let Some(key) = key_for_char(ch, self.calc.mode, self.calc.inv) {
                    self.calc.apply_key(key);
                }
            }
        }
        let Some((path, frames)) = &mut self.screenshot else {
            return;
        };
        if *frames == 20 {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let [w, h] = img.size;
            let bytes: Vec<u8> = img
                .pixels
                .iter()
                .flat_map(|c| c.to_srgba_unmultiplied())
                .collect();
            if let Err(e) =
                image::save_buffer(&*path, &bytes, w as u32, h as u32, image::ColorType::Rgba8)
            {
                tracing::error!("screenshot: {e}");
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}
