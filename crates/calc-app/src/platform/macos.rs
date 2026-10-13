//! macOS: the global menu bar with an app menu (KEY-001, PKG-004), screen geometry,
//! the "Increase contrast" setting and alert dialogs.

use calc_config::{Extras, SYSTEM_THEME};
use calc_core::Mode;
use calc_ui::i18n::tr;
use calc_ui::menu::{MODES, WORKSHEETS};
use calc_ui::{MenuCommand, MenuState, Platform, Rect};
use eframe::egui::{Pos2, Vec2};
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{
    AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSScreen, NSWorkspace};
use std::cell::RefCell;
use std::collections::HashMap;

pub struct Mac {
    menu: Option<NativeMenu>,
}

struct NativeMenu {
    _bar: Menu,
    commands: HashMap<MenuId, MenuCommand>,
    modes: Vec<(Mode, CheckMenuItem)>,
    history: CheckMenuItem,
    grouping: CheckMenuItem,
    extras: Vec<(Extras, CheckMenuItem)>,
    themes_menu: Submenu,
    themes: Vec<(String, CheckMenuItem)>,
    last: Option<MenuState>,
}

impl Mac {
    pub fn new() -> Mac {
        Mac { menu: None }
    }

    /// Builds the menu bar on first use, once the app's event loop (and NSApp) exist.
    fn menu(&mut self) -> Option<&mut NativeMenu> {
        if self.menu.is_none() {
            self.menu = NativeMenu::build()
                .map_err(|e| tracing::warn!("menu bar: {e}"))
                .ok();
        }
        self.menu.as_mut()
    }
}

fn accel(mods: Modifiers, key: Code) -> Option<Accelerator> {
    Some(Accelerator::new(Some(mods), key))
}

impl NativeMenu {
    fn build() -> muda::Result<NativeMenu> {
        let commands = RefCell::new(HashMap::new());
        let item = |label: String, cmd: MenuCommand, a: Option<Accelerator>| {
            let i = MenuItem::new(label, true, a);
            commands.borrow_mut().insert(i.id().clone(), cmd);
            i
        };
        let cmd = Modifiers::META;
        let app = Submenu::with_items(
            tr("app-name"),
            true,
            &[
                &PredefinedMenuItem::about(
                    Some(&tr("help-about")),
                    Some(AboutMetadata {
                        name: Some(tr("app-name")),
                        version: Some(env!("CARGO_PKG_VERSION").into()),
                        ..Default::default()
                    }),
                ),
                &PredefinedMenuItem::separator(),
                &item(
                    tr("edit-preferences"),
                    MenuCommand::Preferences,
                    accel(cmd, Code::Comma),
                ),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ],
        )?;

        let check = |label: String, cmd: MenuCommand, a: Option<Accelerator>| {
            let i = CheckMenuItem::new(label, true, false, a);
            commands.borrow_mut().insert(i.id().clone(), cmd);
            i
        };
        let digits = [Code::Digit1, Code::Digit2, Code::Digit3, Code::Digit4];
        let modes: Vec<(Mode, CheckMenuItem)> = MODES
            .iter()
            .zip(digits)
            .map(|((m, id, _), d)| (*m, check(tr(id), MenuCommand::Mode(*m), accel(cmd, d))))
            .collect();
        // Cmd+H is the system "Hide" shortcut, so History uses Ctrl+H as on other systems.
        let history = check(
            tr("view-history"),
            MenuCommand::ToggleHistory,
            accel(Modifiers::CONTROL, Code::KeyH),
        );
        let grouping = check(tr("view-digit-grouping"), MenuCommand::ToggleGrouping, None);
        let mut extras = vec![
            (
                Extras::None,
                check(
                    tr("view-basic"),
                    MenuCommand::Extras(Extras::None),
                    accel(cmd, Code::F4),
                ),
            ),
            (
                Extras::Units,
                check(
                    tr("view-unit-conversion"),
                    MenuCommand::Extras(Extras::Units),
                    accel(cmd, Code::KeyU),
                ),
            ),
            (
                Extras::Date,
                check(
                    tr("view-date-calculation"),
                    MenuCommand::Extras(Extras::Date),
                    accel(cmd, Code::KeyE),
                ),
            ),
        ];
        let worksheets = Submenu::new(tr("view-worksheets"), true);
        for (e, id) in WORKSHEETS {
            let i = check(tr(id), MenuCommand::Extras(e), None);
            worksheets.append(&i)?;
            extras.push((e, i));
        }

        let themes_menu = Submenu::new(tr("view-themes"), true);
        let view = Submenu::new(tr("menu-view"), true);
        for (_, i) in &modes {
            view.append(i)?;
        }
        view.append_items(&[&PredefinedMenuItem::separator(), &history, &grouping])?;
        view.append(&PredefinedMenuItem::separator())?;
        for (_, i) in extras.iter().take(3) {
            view.append(i)?;
        }
        view.append_items(&[
            &worksheets,
            &PredefinedMenuItem::separator(),
            &themes_menu,
            &item(
                tr("view-zoom-in"),
                MenuCommand::ZoomIn,
                accel(cmd, Code::Equal),
            ),
            &item(
                tr("view-zoom-out"),
                MenuCommand::ZoomOut,
                accel(cmd, Code::Minus),
            ),
            &item(
                tr("view-reset-size"),
                MenuCommand::ResetSize,
                accel(cmd, Code::Digit0),
            ),
        ])?;

        let history_menu = Submenu::with_items(
            tr("edit-history"),
            true,
            &[
                &item(tr("history-copy"), MenuCommand::CopyHistory, None),
                &item(
                    tr("history-clear"),
                    MenuCommand::ClearHistory,
                    accel(cmd | Modifiers::SHIFT, Code::KeyD),
                ),
            ],
        )?;
        let edit = Submenu::with_items(
            tr("menu-edit"),
            true,
            &[
                &item(tr("edit-copy"), MenuCommand::Copy, accel(cmd, Code::KeyC)),
                &item(tr("edit-paste"), MenuCommand::Paste, accel(cmd, Code::KeyV)),
                &PredefinedMenuItem::separator(),
                &history_menu,
            ],
        )?;
        let help = Submenu::with_items(
            tr("menu-help"),
            true,
            &[
                &item(tr("help-view"), MenuCommand::Help, None),
                &item(tr("help-licenses"), MenuCommand::Licenses, None),
            ],
        )?;
        let bar = Menu::with_items(&[&app, &view, &edit, &help])?;
        bar.init_for_nsapp();
        Ok(NativeMenu {
            _bar: bar,
            commands: commands.into_inner(),
            modes,
            history,
            grouping,
            extras,
            themes_menu,
            themes: Vec::new(),
            last: None,
        })
    }

    fn sync(&mut self, s: &MenuState) {
        if self.last.as_ref() == Some(s) {
            return;
        }
        if self.last.as_ref().is_none_or(|l| l.themes != s.themes) {
            self.rebuild_themes(s);
        }
        for (m, i) in &self.modes {
            i.set_checked(*m == s.mode);
        }
        self.history.set_checked(s.history);
        self.history.set_enabled(s.history_enabled);
        self.grouping.set_checked(s.grouping);
        for (e, i) in &self.extras {
            i.set_checked(*e == s.extras);
        }
        for (id, i) in &self.themes {
            i.set_checked(*id == s.theme);
        }
        self.last = Some(s.clone());
    }

    fn rebuild_themes(&mut self, s: &MenuState) {
        while self.themes_menu.remove_at(0).is_some() {}
        self.themes.clear();
        let mut add_check = |id: String, label: String, menu: &Submenu| {
            let i = CheckMenuItem::new(label, true, false, None);
            self.commands
                .insert(i.id().clone(), MenuCommand::Theme(id.clone()));
            let _ = menu.append(&i);
            self.themes.push((id, i));
        };
        add_check(
            SYSTEM_THEME.into(),
            tr("themes-follow-system"),
            &self.themes_menu,
        );
        let _ = self.themes_menu.append(&PredefinedMenuItem::separator());
        for (id, name) in &s.themes {
            add_check(id.clone(), name.clone(), &self.themes_menu);
        }
        let _ = self.themes_menu.append(&PredefinedMenuItem::separator());
        for (label, cmd, a) in [
            (
                tr("themes-reload"),
                MenuCommand::ReloadThemes,
                accel(Modifiers::META | Modifiers::SHIFT, Code::KeyR),
            ),
            (
                tr("themes-open-folder"),
                MenuCommand::OpenThemesFolder,
                None,
            ),
        ] {
            let i = MenuItem::new(label, true, a);
            self.commands.insert(i.id().clone(), cmd);
            let _ = self.themes_menu.append(&i);
        }
    }
}

impl Platform for Mac {
    /// Screens in physical pixels with a top-left origin, main screen first.
    fn monitors(&self) -> Vec<Rect> {
        let Some(mtm) = MainThreadMarker::new() else {
            return Vec::new();
        };
        let screens = NSScreen::screens(mtm);
        let Some(main_height) = screens.iter().next().map(|s| s.frame().size.height) else {
            return Vec::new();
        };
        screens
            .iter()
            .map(|s| {
                let f = s.visibleFrame();
                let k = s.backingScaleFactor() as f32;
                let top = main_height - (f.origin.y + f.size.height);
                Rect::from_min_size(
                    Pos2::new(f.origin.x as f32 * k, top as f32 * k),
                    Vec2::new(f.size.width as f32 * k, f.size.height as f32 * k),
                )
            })
            .collect()
    }

    fn high_contrast(&self) -> bool {
        NSWorkspace::sharedWorkspace().accessibilityDisplayShouldIncreaseContrast()
    }

    fn native_menu(&self) -> bool {
        true
    }

    fn poll_menu(&mut self) -> Vec<MenuCommand> {
        let Some(menu) = self.menu() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Some(cmd) = menu.commands.get(&ev.id) {
                out.push(cmd.clone());
            }
        }
        out
    }

    fn sync_menu(&mut self, state: &MenuState) {
        if let Some(menu) = self.menu() {
            menu.sync(state);
        }
    }
}

/// Shows an alert through AppleScript, which works even while the app is unwinding.
pub fn message_box(title: &str, text: &str) {
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "display alert \"{}\" message \"{}\" as critical",
        esc(title),
        esc(text)
    );
    if std::process::Command::new("osascript")
        .args(["-e", &script])
        .status()
        .is_err()
    {
        eprintln!("{title}: {text}");
    }
}
