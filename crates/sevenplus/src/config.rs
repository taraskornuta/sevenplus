//! `settings.toml` and the per-user config/themes folders.

use calc_core::{Angle, Base, Mode, WordSize};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameStyle {
    #[default]
    Native,
    Aero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Extras {
    #[default]
    None,
    Units,
    Date,
    Mortgage,
    VehicleLease,
    FuelMpg,
    FuelL100km,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Theme file name (without `.toml`) in the themes folder.
    pub theme: String,
    /// "native" OS window frame or hand-drawn "aero" glass frame.
    pub frame: FrameStyle,
    pub mode: Mode,
    pub history: bool,
    pub digit_grouping: bool,
    pub extras: Extras,
    pub angle: Angle,
    pub base: Base,
    pub word: WordSize,
    /// UI scale, changed by resizing the window (1.0 = Win7 size).
    pub scale: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "win7".into(),
            frame: FrameStyle::Native,
            mode: Mode::Standard,
            history: false,
            digit_grouping: false,
            extras: Extras::None,
            angle: Angle::Degrees,
            base: Base::Dec,
            word: WordSize::Qword,
            scale: 1.0,
        }
    }
}

/// Config folder; `SEVENPLUS_CONFIG_DIR` overrides the OS default.
pub fn config_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("SEVENPLUS_CONFIG_DIR") {
        return d.into();
    }
    directories::ProjectDirs::from("", "", "sevenplus")
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".sevenplus"))
}

pub fn themes_dir() -> PathBuf {
    config_dir().join("themes")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

impl Settings {
    /// Loads settings (defaults on any problem) and writes out the built-in themes on
    /// first run so they can be copied and edited.
    pub fn load() -> Settings {
        let dir = themes_dir();
        if std::fs::create_dir_all(&dir).is_ok() {
            for (name, src) in crate::theme::BUILTIN {
                let p = dir.join(format!("{name}.toml"));
                if !p.exists() {
                    let _ = std::fs::write(p, src);
                }
            }
        }
        std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|s| {
                toml::from_str(&s)
                    .map_err(|e| eprintln!("settings.toml: {e}"))
                    .ok()
            })
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let header =
            "# SevenPlus settings. frame = \"native\" | \"aero\"; theme = file name in themes/\n";
        match toml::to_string(self) {
            Ok(s) => {
                if let Err(e) = std::fs::write(settings_path(), format!("{header}{s}")) {
                    eprintln!("cannot save settings: {e}");
                }
            }
            Err(e) => eprintln!("cannot serialize settings: {e}"),
        }
    }

    pub fn theme_path(&self) -> PathBuf {
        themes_dir().join(format!("{}.toml", self.theme))
    }
}

/// Theme names (file stems) available in the themes folder.
pub fn list_themes() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(themes_dir())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    v.sort();
    v
}
