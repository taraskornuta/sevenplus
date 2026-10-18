//! Menus (KEY-001). The same [`MenuCommand`]s come from the in-window menu bar and,
//! on macOS, from the global menu bar.

use crate::app::App;
use crate::i18n::tr;
use crate::paint::{self, c};
use calc_config::{Extras, SYSTEM_THEME};
use calc_core::Mode;
use eframe::egui::{
    self, Button, Rect, Stroke, UiBuilder, WidgetInfo, WidgetType, containers::menu::MenuButton,
    pos2, vec2,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuCommand {
    Mode(Mode),
    ToggleHistory,
    ToggleGrouping,
    Extras(Extras),
    /// A theme id, or `"system"` to follow the OS light/dark setting.
    Theme(String),
    ReloadThemes,
    OpenThemesFolder,
    OpenSettingsFolder,
    ZoomIn,
    ZoomOut,
    ResetSize,
    Copy,
    Paste,
    CopyHistory,
    ClearHistory,
    Preferences,
    Help,
    About,
    Licenses,
    Quit,
}

/// Check marks and enabled states a native menu mirrors.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuState {
    pub mode: Mode,
    pub history: bool,
    pub history_enabled: bool,
    pub grouping: bool,
    pub extras: Extras,
    pub theme: String,
    /// `(id, display name)` of every selectable theme.
    pub themes: Vec<(String, String)>,
}

impl MenuState {
    pub fn of(app: &App) -> MenuState {
        MenuState {
            mode: app.calc.mode,
            history: app.settings.history,
            history_enabled: matches!(app.calc.mode, Mode::Standard | Mode::Scientific),
            grouping: app.settings.digit_grouping,
            extras: app.settings.extras,
            theme: app.settings.theme.clone(),
            themes: app
                .store
                .list()
                .into_iter()
                .map(|t| (t.id, t.name))
                .collect(),
        }
    }
}

/// Mode menu entries: mode, Fluent id, shortcut digit (KEY-002).
pub const MODES: [(Mode, &str, char); 4] = [
    (Mode::Standard, "mode-standard", '1'),
    (Mode::Scientific, "mode-scientific", '2'),
    (Mode::Programmer, "mode-programmer", '3'),
    (Mode::Statistics, "mode-statistics", '4'),
];

pub const WORKSHEETS: [(Extras, &str); 4] = [
    (Extras::Mortgage, "ws-mortgage"),
    (Extras::VehicleLease, "ws-vehicle-lease"),
    (Extras::FuelMpg, "ws-fuel-mpg"),
    (Extras::FuelL100km, "ws-fuel-l100km"),
];

/// Shortcut text with the platform's modifier names.
pub fn shortcut(keys: &str) -> String {
    if cfg!(target_os = "macos") {
        keys.replace("Ctrl+", "⌘")
            .replace("Alt+", "⌘")
            .replace("Shift+", "⇧")
    } else {
        keys.to_string()
    }
}

fn help_menu(ui: &mut egui::Ui) -> Option<MenuCommand> {
    let mut out = None;
    if ui.button(tr("help-view")).clicked() {
        out = Some(MenuCommand::Help);
    }
    ui.separator();
    if ui.button(tr("help-licenses")).clicked() {
        out = Some(MenuCommand::Licenses);
    }
    if ui.button(tr("help-about")).clicked() {
        out = Some(MenuCommand::About);
    }
    out
}

fn item(ui: &mut egui::Ui, selected: bool, label: &str, keys: &str) -> bool {
    ui.add(Button::selectable(selected, label).shortcut_text(shortcut(keys)))
        .clicked()
}

impl App {
    pub(crate) fn draw_menu(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let m = self.theme.menu.clone();
        paint::gradient_rect(ui.painter(), rect, 0.0, m.background.stops());
        let mut chosen = None;
        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            // The shared style, sized like the menus it opens; the window's root Ui
            // still has the style it was created with.
            ui.set_style(ui.ctx().global_style());
            ui.visuals_mut().override_text_color = Some(c(m.text));
            ui.visuals_mut().widgets.hovered.weak_bg_fill = c(m.hover);
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(1.0, c(m.hover_border));
            ui.visuals_mut().widgets.open.weak_bg_fill = c(m.hover);
            egui::MenuBar::new().ui(ui, |ui| {
                // One ☰ button holds every menu. Inter has no ☰ glyph, so it is drawn.
                // The bar keeps one size on screen, like the menu it opens.
                let h = (rect.height() - 4.0).max(10.0);
                let button = Button::new("").min_size(vec2(h * 1.3, h));
                let (resp, _) = MenuButton::from_button(button).ui(ui, |ui| {
                    chosen = self.main_menu(ui);
                });
                let label = tr("menu-main");
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
                // About 16 px wide with 2 px lines, like other apps' menu icons.
                let (center, w) = (resp.rect.center(), h * 0.38);
                let stroke = Stroke::new(2.0 * crate::app::chrome_k(ui.ctx()), c(m.text));
                for dy in [-h * 0.24, 0.0, h * 0.24] {
                    let y = (center.y + dy).round();
                    ui.painter()
                        .line_segment([pos2(center.x - w, y), pos2(center.x + w, y)], stroke);
                }
                resp.on_hover_text(label);
            });
        });
        if let Some(cmd) = chosen {
            let ctx = ui.ctx().clone();
            self.command(&ctx, cmd);
        }
    }

    /// The ☰ menu: modes, then short submenus, so it fits even the smallest window.
    fn main_menu(&self, ui: &mut egui::Ui) -> Option<MenuCommand> {
        let mut out = None;
        for (mode, id, digit) in MODES {
            if item(ui, self.calc.mode == mode, &tr(id), &format!("Alt+{digit}")) {
                out = Some(MenuCommand::Mode(mode));
            }
        }
        ui.separator();
        ui.menu_button(tr("menu-tools"), |ui| out = self.tools_menu(ui));
        ui.menu_button(tr("menu-view"), |ui| out = self.view_menu(ui));
        ui.menu_button(tr("menu-edit"), |ui| out = edit_menu(ui));
        ui.separator();
        if item(ui, false, &tr("edit-preferences"), "Ctrl+,") {
            out = Some(MenuCommand::Preferences);
        }
        ui.menu_button(tr("menu-help"), |ui| out = help_menu(ui));
        out
    }

    /// Unit conversion, date calculation and worksheets beside the keypad.
    fn tools_menu(&self, ui: &mut egui::Ui) -> Option<MenuCommand> {
        let mut out = None;
        let ex = self.settings.extras;
        for (e, id, keys) in [
            (Extras::None, "view-basic", "Ctrl+F4"),
            (Extras::Units, "view-unit-conversion", "Ctrl+U"),
            (Extras::Date, "view-date-calculation", "Ctrl+E"),
        ] {
            if item(ui, ex == e, &tr(id), keys) {
                out = Some(MenuCommand::Extras(e));
            }
        }
        ui.menu_button(tr("view-worksheets"), |ui| {
            for (e, id) in WORKSHEETS {
                if item(ui, ex == e, &tr(id), "") {
                    out = Some(MenuCommand::Extras(e));
                }
            }
        });
        out
    }

    fn view_menu(&self, ui: &mut egui::Ui) -> Option<MenuCommand> {
        let mut out = None;
        let history_ok = matches!(self.calc.mode, Mode::Standard | Mode::Scientific);
        if ui
            .add_enabled(
                history_ok,
                Button::selectable(self.settings.history, tr("view-history"))
                    .shortcut_text(shortcut("Ctrl+H")),
            )
            .clicked()
        {
            out = Some(MenuCommand::ToggleHistory);
        }
        if item(
            ui,
            self.settings.digit_grouping,
            &tr("view-digit-grouping"),
            "",
        ) {
            out = Some(MenuCommand::ToggleGrouping);
        }
        ui.separator();
        ui.menu_button(tr("view-themes"), |ui| {
            if item(
                ui,
                self.settings.theme == SYSTEM_THEME,
                &tr("themes-follow-system"),
                "",
            ) {
                out = Some(MenuCommand::Theme(SYSTEM_THEME.into()));
            }
            ui.separator();
            for t in self.store.list() {
                if item(ui, self.settings.theme == t.id, &t.name, "") {
                    out = Some(MenuCommand::Theme(t.id));
                }
            }
            ui.separator();
            if item(ui, false, &tr("themes-reload"), "Ctrl+Shift+R") {
                out = Some(MenuCommand::ReloadThemes);
            }
            if item(ui, false, &tr("themes-open-folder"), "") {
                out = Some(MenuCommand::OpenThemesFolder);
            }
        });
        ui.separator();
        for (id, keys, cmd) in [
            ("view-zoom-in", "Ctrl++", MenuCommand::ZoomIn),
            ("view-zoom-out", "Ctrl+-", MenuCommand::ZoomOut),
            ("view-reset-size", "Ctrl+0", MenuCommand::ResetSize),
        ] {
            if item(ui, false, &tr(id), keys) {
                out = Some(cmd);
            }
        }
        out
    }
}

fn edit_menu(ui: &mut egui::Ui) -> Option<MenuCommand> {
    let mut out = None;
    if item(ui, false, &tr("edit-copy"), "Ctrl+C") {
        out = Some(MenuCommand::Copy);
    }
    if item(ui, false, &tr("edit-paste"), "Ctrl+V") {
        // Clipboard contents arrive as a paste event on the next frame.
        out = Some(MenuCommand::Paste);
    }
    ui.separator();
    ui.menu_button(tr("edit-history"), |ui| {
        if item(ui, false, &tr("history-copy"), "") {
            out = Some(MenuCommand::CopyHistory);
        }
        if item(ui, false, &tr("history-clear"), "Ctrl+Shift+D") {
            out = Some(MenuCommand::ClearHistory);
        }
    });
    out
}
