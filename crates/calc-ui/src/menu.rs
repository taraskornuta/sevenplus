//! Menus (KEY-001). The same [`MenuCommand`]s come from the in-window menu bar and,
//! on macOS, from the global menu bar.

use crate::app::App;
use crate::i18n::tr;
use crate::paint::{self, c};
use calc_config::{Extras, FrameStyle, SYSTEM_THEME};
use calc_core::Mode;
use eframe::egui::{self, Button, Rect, Stroke, UiBuilder};

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
    Frame(FrameStyle),
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
            ui.visuals_mut().override_text_color = Some(c(m.text));
            ui.visuals_mut().widgets.hovered.weak_bg_fill = c(m.hover);
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(1.0, c(m.hover_border));
            ui.visuals_mut().widgets.open.weak_bg_fill = c(m.hover);
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(tr("menu-view"), |ui| chosen = self.view_menu(ui));
                ui.menu_button(tr("menu-edit"), |ui| chosen = self.edit_menu(ui));
                ui.menu_button(tr("menu-help"), |ui| {
                    if ui.button(tr("help-view")).clicked() {
                        chosen = Some(MenuCommand::Help);
                    }
                    ui.separator();
                    if ui.button(tr("help-licenses")).clicked() {
                        chosen = Some(MenuCommand::Licenses);
                    }
                    if ui.button(tr("help-about")).clicked() {
                        chosen = Some(MenuCommand::About);
                    }
                });
            });
        });
        if let Some(cmd) = chosen {
            let ctx = ui.ctx().clone();
            self.command(&ctx, cmd);
        }
    }

    fn view_menu(&self, ui: &mut egui::Ui) -> Option<MenuCommand> {
        let mut out = None;
        let mut pick = |cond: bool, cmd: MenuCommand| {
            if cond {
                out = Some(cmd);
            }
        };
        for (mode, id, digit) in MODES {
            pick(
                item(ui, self.calc.mode == mode, &tr(id), &format!("Alt+{digit}")),
                MenuCommand::Mode(mode),
            );
        }
        ui.separator();
        let history_ok = matches!(self.calc.mode, Mode::Standard | Mode::Scientific);
        pick(
            ui.add_enabled(
                history_ok,
                Button::selectable(self.settings.history, tr("view-history"))
                    .shortcut_text(shortcut("Ctrl+H")),
            )
            .clicked(),
            MenuCommand::ToggleHistory,
        );
        pick(
            item(
                ui,
                self.settings.digit_grouping,
                &tr("view-digit-grouping"),
                "",
            ),
            MenuCommand::ToggleGrouping,
        );
        ui.separator();
        let ex = self.settings.extras;
        pick(
            item(ui, ex == Extras::None, &tr("view-basic"), "Ctrl+F4"),
            MenuCommand::Extras(Extras::None),
        );
        pick(
            item(
                ui,
                ex == Extras::Units,
                &tr("view-unit-conversion"),
                "Ctrl+U",
            ),
            MenuCommand::Extras(Extras::Units),
        );
        pick(
            item(
                ui,
                ex == Extras::Date,
                &tr("view-date-calculation"),
                "Ctrl+E",
            ),
            MenuCommand::Extras(Extras::Date),
        );
        ui.menu_button(tr("view-worksheets"), |ui| {
            for (e, id) in WORKSHEETS {
                if item(ui, ex == e, &tr(id), "") {
                    out = Some(MenuCommand::Extras(e));
                }
            }
        });
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
        let mut pick = |cond: bool, cmd: MenuCommand| {
            if cond {
                out = Some(cmd);
            }
        };
        pick(
            item(ui, false, &tr("view-zoom-in"), "Ctrl++"),
            MenuCommand::ZoomIn,
        );
        pick(
            item(ui, false, &tr("view-zoom-out"), "Ctrl+-"),
            MenuCommand::ZoomOut,
        );
        pick(
            item(ui, false, &tr("view-reset-size"), "Ctrl+0"),
            MenuCommand::ResetSize,
        );
        ui.menu_button(tr("view-window-frame"), |ui| {
            for (f, id) in [
                (FrameStyle::Native, "frame-native"),
                (FrameStyle::Aero, "frame-aero"),
            ] {
                if item(ui, self.settings.frame == f, &tr(id), "") {
                    out = Some(MenuCommand::Frame(f));
                }
            }
            if self.settings.frame != self.created_frame {
                ui.label(egui::RichText::new(tr("frame-restart")).small());
            }
        });
        out
    }

    fn edit_menu(&self, ui: &mut egui::Ui) -> Option<MenuCommand> {
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
        ui.separator();
        if item(ui, false, &tr("edit-preferences"), "Ctrl+,") {
            out = Some(MenuCommand::Preferences);
        }
        out
    }
}
