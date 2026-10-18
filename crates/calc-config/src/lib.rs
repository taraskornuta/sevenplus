//! `settings.toml` and the per-user folders (ARC-004, ARC-005, THM-007).
//!
//! Settings are written atomically (temporary file, then rename). A settings file that
//! is corrupt, too large or from a newer version is renamed to `settings.toml.bak` and
//! defaults are used, so the app always starts.

#![forbid(unsafe_code)]

use calc_core::{Angle, Base, CharMode, Mode, WordSize};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Settings format version written by this build.
pub const SCHEMA: u32 = 1;
/// Settings files larger than this are treated as corrupt (NFR-031).
pub const MAX_FILE_SIZE: u64 = 1024 * 1024;
/// `theme` value that follows the OS light/dark setting (THM-002).
pub const SYSTEM_THEME: &str = "system";

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
    pub schema: u32,
    /// `"system"` to follow the OS light/dark setting, or a theme id.
    pub theme: String,
    /// Themes used by `"system"` in light and dark OS mode.
    pub theme_light: String,
    pub theme_dark: String,
    /// Switch to the High Contrast theme when the OS high-contrast mode is on (UI-024).
    pub follow_high_contrast: bool,
    pub mode: Mode,
    pub history: bool,
    pub digit_grouping: bool,
    pub extras: Extras,
    pub angle: Angle,
    pub base: Base,
    pub word: WordSize,
    /// UI scale, changed by resizing the window (1.0 = reference size).
    pub scale: f32,
    /// Outer window position in logical px, where the platform reports one.
    pub window_pos: Option<[f32; 2]>,
    /// Pixels per logical px of the monitor the position was saved on.
    pub window_ppp: Option<f32>,
    /// Decimal separator override; default follows the OS locale (NFR-020).
    pub decimal_separator: Option<char>,
    /// Digit-group separator override; default follows the OS locale.
    pub group_separator: Option<char>,
    /// Copy uses the locale decimal separator instead of `.` (KEY-006).
    pub copy_locale_decimal: bool,
    /// Copy includes digit-group separators when grouping is on (STD-010).
    pub copy_grouping: bool,
    /// Programmer-mode buttons show C operator signs (`<<`, `&`, `~`…) instead of words.
    pub programmer_c_signs: bool,
    /// Programmer mode shows the value as ASCII or Unicode characters.
    pub programmer_chars: CharMode,
    /// Glass themes (`backdrop = "system"`) make the window see-through and blurred.
    pub transparency: bool,
    /// Opacity of a glass theme's window tint, [`MIN_OPACITY`] to 1.
    pub opacity: f32,
}

/// Scale of a fresh install and of View ▸ Reset size. Menus, dialogs and tooltips
/// always use it, whatever the window's zoom, so at this scale they match the body.
/// 1.25 gives text the size of common desktop apps' menus.
pub const DEFAULT_SCALE: f32 = 1.25;

/// Lowest window opacity the Preferences slider allows.
pub const MIN_OPACITY: f32 = 0.2;

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema: SCHEMA,
            theme: SYSTEM_THEME.into(),
            theme_light: "light".into(),
            theme_dark: "dark".into(),
            follow_high_contrast: true,
            mode: Mode::Standard,
            history: false,
            digit_grouping: false,
            extras: Extras::None,
            angle: Angle::Degrees,
            base: Base::Dec,
            word: WordSize::Qword,
            scale: DEFAULT_SCALE,
            window_pos: None,
            window_ppp: None,
            decimal_separator: None,
            group_separator: None,
            copy_locale_decimal: false,
            copy_grouping: false,
            programmer_c_signs: true,
            programmer_chars: CharMode::Ascii,
            transparency: true,
            opacity: 0.85,
        }
    }
}

/// Settings plus a description of any problem found while loading them.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub settings: Settings,
    pub warning: Option<String>,
}

fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("", "", "sevenplus")
}

/// Config folder; `SEVENPLUS_CONFIG_DIR` overrides the OS default.
pub fn config_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("SEVENPLUS_CONFIG_DIR") {
        return d.into();
    }
    project_dirs()
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".sevenplus"))
}

/// User themes folder (THM-007): `%APPDATA%\sevenplus\themes`,
/// `~/Library/Application Support/sevenplus/themes`, `$XDG_CONFIG_HOME/sevenplus/themes`.
pub fn themes_dir() -> PathBuf {
    if std::env::var_os("SEVENPLUS_CONFIG_DIR").is_none()
        && cfg!(windows)
        && let Some(root) =
            project_dirs().and_then(|p| p.config_dir().parent().map(Path::to_path_buf))
    {
        return root.join("themes");
    }
    config_dir().join("themes")
}

/// Log folder (ARC-007): `~/Library/Logs/sevenplus`, `$XDG_STATE_HOME/sevenplus`,
/// `%LOCALAPPDATA%\sevenplus\data\logs`.
pub fn log_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("SEVENPLUS_CONFIG_DIR") {
        return PathBuf::from(d).join("logs");
    }
    if cfg!(target_os = "macos")
        && let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())
    {
        return home.join("Library/Logs/sevenplus");
    }
    match project_dirs() {
        Some(p) => p
            .state_dir()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| p.data_local_dir().join("logs")),
        None => PathBuf::from(".sevenplus/logs"),
    }
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

/// Writes `contents` to `path` atomically: a temporary file in the same folder is
/// written, flushed and renamed over the target (ARC-004).
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Moves a bad settings file aside as `<name>.bak` (ARC-005).
fn back_up(path: &Path) -> String {
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    let bak = PathBuf::from(bak);
    match std::fs::rename(path, &bak) {
        Ok(()) => format!("saved the old file as {}", bak.display()),
        Err(e) => format!("could not rename it to {}: {e}", bak.display()),
    }
}

/// Theme ids used before schema 1.
fn migrate_theme(id: &str) -> String {
    match id {
        "win7" => "aero7".into(),
        "win7-dark" => "dark".into(),
        other => other.into(),
    }
}

impl Settings {
    /// Loads settings from the default location.
    pub fn load() -> Loaded {
        Self::load_from(&settings_path())
    }

    /// Loads settings; never fails (ARC-005).
    pub fn load_from(path: &Path) -> Loaded {
        let fallback = |problem: String| {
            let warning = format!("{}: {problem}; {}", path.display(), back_up(path));
            tracing::warn!("{warning}");
            Loaded {
                settings: Settings::default(),
                warning: Some(warning),
            }
        };
        let len = match std::fs::metadata(path) {
            Ok(m) => m.len(),
            Err(_) => {
                return Loaded {
                    settings: Settings::default(),
                    warning: None,
                };
            }
        };
        if len > MAX_FILE_SIZE {
            return fallback(format!("larger than {MAX_FILE_SIZE} bytes"));
        }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => return fallback(e.to_string()),
        };
        let table: toml::Table = match text.parse() {
            Ok(t) => t,
            Err(e) => return fallback(format!("not valid TOML: {e}")),
        };
        let schema = match table.get("schema") {
            None => 0,
            Some(toml::Value::Integer(v)) => *v,
            Some(_) => return fallback("`schema` is not a number".into()),
        };
        if schema > SCHEMA as i64 {
            return fallback(format!(
                "written by a newer version (schema {schema} > {SCHEMA})"
            ));
        }
        let mut settings: Settings = match toml::Value::Table(table).try_into() {
            Ok(s) => s,
            Err(e) => return fallback(e.to_string()),
        };
        if schema < SCHEMA as i64 {
            settings.theme = migrate_theme(&settings.theme);
            settings.schema = SCHEMA;
        }
        settings.opacity = if settings.opacity.is_finite() {
            settings.opacity.clamp(MIN_OPACITY, 1.0)
        } else {
            Settings::default().opacity
        };
        if !settings.scale.is_finite() {
            settings.scale = DEFAULT_SCALE;
        }
        Loaded {
            settings,
            warning: None,
        }
    }

    /// Saves to the default location.
    pub fn save(&self) {
        if let Err(e) = self.save_to(&settings_path()) {
            tracing::warn!("cannot save settings: {e}");
        }
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        let body = toml::to_string(self).map_err(std::io::Error::other)?;
        let header = "# SevenPlus settings, saved automatically.\n\
            # theme = \"system\" or a theme id.\n";
        write_atomic(path, &format!("{header}{body}"))
    }

    /// The theme id to use for the given OS state.
    pub fn theme_id(&self, os_dark: bool, os_high_contrast: bool) -> String {
        if os_high_contrast && self.follow_high_contrast {
            return "high-contrast".into();
        }
        if self.theme == SYSTEM_THEME {
            if os_dark {
                self.theme_dark.clone()
            } else {
                self.theme_light.clone()
            }
        } else {
            self.theme.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("calc-config-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("settings.toml")
    }

    #[test]
    fn round_trip() {
        let p = temp("round");
        let s = Settings {
            mode: Mode::Programmer,
            window_pos: Some([10.0, 20.0]),
            decimal_separator: Some(','),
            ..Settings::default()
        };
        s.save_to(&p).unwrap();
        let l = Settings::load_from(&p);
        assert!(l.warning.is_none());
        assert_eq!(l.settings, s);
        assert!(!p.with_extension("toml.tmp").exists());
    }

    #[test]
    fn missing_file_is_default() {
        let p = temp("missing");
        let l = Settings::load_from(&p);
        assert!(l.warning.is_none());
        assert_eq!(l.settings, Settings::default());
    }

    #[test]
    fn corrupt_file_is_backed_up() {
        let p = temp("corrupt");
        std::fs::write(&p, "mode = [[[").unwrap();
        let l = Settings::load_from(&p);
        assert!(l.warning.is_some());
        assert_eq!(l.settings, Settings::default());
        assert!(!p.exists());
        assert!(p.with_extension("toml.bak").exists());
    }

    #[test]
    fn wrong_types_and_newer_schema_are_backed_up() {
        for (name, body) in [
            ("types", "schema = 1\nmode = \"hexadecimal\""),
            ("newer", "schema = 99\nmode = \"scientific\""),
        ] {
            let p = temp(name);
            std::fs::write(&p, body).unwrap();
            let l = Settings::load_from(&p);
            assert!(l.warning.is_some(), "{name}");
            assert_eq!(l.settings, Settings::default());
            assert!(p.with_extension("toml.bak").exists(), "{name}");
        }
    }

    #[test]
    fn oversized_file_is_backed_up() {
        let p = temp("big");
        std::fs::write(&p, "#".repeat(MAX_FILE_SIZE as usize + 1)).unwrap();
        assert!(Settings::load_from(&p).warning.is_some());
    }

    #[test]
    fn old_settings_migrate() {
        let p = temp("old");
        std::fs::write(
            &p,
            "theme = \"win7-dark\"\nmode = \"scientific\"\nscale = 1.5\n",
        )
        .unwrap();
        let l = Settings::load_from(&p);
        assert!(l.warning.is_none());
        assert_eq!(l.settings.theme, "dark");
        assert_eq!(l.settings.mode, Mode::Scientific);
        assert_eq!(l.settings.scale, 1.5);
        assert_eq!(l.settings.schema, SCHEMA);
    }

    #[test]
    fn theme_selection() {
        let s = Settings::default();
        assert_eq!(s.theme_id(false, false), "light");
        assert_eq!(s.theme_id(true, false), "dark");
        assert_eq!(s.theme_id(true, true), "high-contrast");
        let s = Settings {
            theme: "classic".into(),
            ..Settings::default()
        };
        assert_eq!(s.theme_id(true, false), "classic");
    }
}
