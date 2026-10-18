//! Themes (§6): TOML files of design tokens, loaded and validated at runtime.
//!
//! A theme names a `schema` version, may `extends` another theme and overrides only
//! the tokens it changes. Without `extends` a theme builds on the default theme
//! ([`DEFAULT_THEME`]). Built-in themes are embedded; user themes live in the
//! platform config folder and may shadow a built-in id. Files are untrusted input:
//! they are size-limited, never executed and never load external resources other
//! than optional font and SVG files next to them (NFR-031).

#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

/// The theme format version this build understands (THM-004).
pub const SCHEMA: u32 = 1;
/// Theme and font files larger than this are rejected (NFR-031).
pub const MAX_FILE_SIZE: u64 = 1024 * 1024;
/// The default theme and the implicit base of themes without `extends` (THM-001).
pub const DEFAULT_THEME: &str = "aero7";
/// Longest allowed `extends` chain.
const MAX_DEPTH: usize = 8;

/// Built-in themes as `(id, TOML source)`.
pub const BUILTIN: &[(&str, &str)] = &[
    ("aero7", include_str!("../../../assets/themes/aero7.toml")),
    (
        "classic",
        include_str!("../../../assets/themes/classic.toml"),
    ),
    ("light", include_str!("../../../assets/themes/light.toml")),
    ("dark", include_str!("../../../assets/themes/dark.toml")),
    ("macos", include_str!("../../../assets/themes/macos.toml")),
    ("glass", include_str!("../../../assets/themes/glass.toml")),
    (
        "glass-dark",
        include_str!("../../../assets/themes/glass-dark.toml"),
    ),
    (
        "high-contrast",
        include_str!("../../../assets/themes/high-contrast.toml"),
    ),
];

/// Button groups that can be styled separately (THM-005).
pub const GROUPS: [&str; 5] = ["digit", "operator", "memory", "function", "equals"];

// ---- colors ---------------------------------------------------------------------------

/// An sRGB color with straight (unmultiplied) alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(pub [u8; 4]);

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color([r, g, b, 255])
    }

    pub fn parse(s: &str) -> Option<Color> {
        let h = s.strip_prefix('#')?;
        let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
        match h.len() {
            6 => Some(Color([byte(0)?, byte(2)?, byte(4)?, 255])),
            8 => Some(Color([byte(0)?, byte(2)?, byte(4)?, byte(6)?])),
            _ => None,
        }
    }

    /// Alpha-composites this color over an opaque background.
    pub fn over(self, bg: Color) -> Color {
        let a = self.0[3] as f32 / 255.0;
        let mix = |f: u8, b: u8| (f as f32 * a + b as f32 * (1.0 - a)).round() as u8;
        Color([
            mix(self.0[0], bg.0[0]),
            mix(self.0[1], bg.0[1]),
            mix(self.0[2], bg.0[2]),
            255,
        ])
    }

    /// WCAG 2.1 relative luminance.
    pub fn luminance(self) -> f64 {
        let lin = |c: u8| {
            let c = c as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.0[0]) + 0.7152 * lin(self.0[1]) + 0.0722 * lin(self.0[2])
    }

    /// WCAG 2.1 contrast ratio between two opaque colors (1–21).
    pub fn contrast(self, other: Color) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Color::parse(&s).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "invalid color {s:?}, expected \"#rrggbb\" or \"#rrggbbaa\""
            ))
        })
    }
}

/// A vertical gradient: 1 color (solid) to 4 stops spread evenly top to bottom.
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient(pub Vec<Color>);

impl Gradient {
    pub fn stops(&self) -> &[Color] {
        &self.0
    }

    pub fn first(&self) -> Color {
        self.0.first().copied().unwrap_or(Color([0, 0, 0, 0]))
    }
}

impl<'de> Deserialize<'de> for Gradient {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            One(Color),
            Many(Vec<Color>),
        }
        let stops = match Raw::deserialize(d)? {
            Raw::One(c) => vec![c],
            Raw::Many(v) => v,
        };
        if !(1..=4).contains(&stops.len()) {
            return Err(serde::de::Error::custom(format!(
                "a gradient has 1 to 4 colors, found {}",
                stops.len()
            )));
        }
        Ok(Gradient(stops))
    }
}

// ---- tokens ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub schema: u32,
    pub name: String,
    /// The theme this one was built on (informational after loading).
    #[serde(default)]
    pub extends: Option<String>,
    /// `system` asks for native backdrop materials where available (THM-010).
    #[serde(default)]
    pub backdrop: Backdrop,
    /// Which key arrangement the keypads use.
    #[serde(default)]
    pub layout: KeyLayout,
    pub font: Fonts,
    pub spacing: Spacing,
    pub window: WindowCfg,
    pub menu: MenuCfg,
    /// The `[titlebar]` table of the removed Aero glass frame; accepted and ignored so
    /// older theme files still load.
    #[serde(default, rename = "titlebar")]
    _titlebar: Option<serde::de::IgnoredAny>,
    pub display: DisplayCfg,
    pub button: Buttons,
    pub controls: ControlsCfg,
    pub bits: BitsCfg,
    #[serde(default)]
    pub imagery: Imagery,
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Backdrop {
    #[default]
    Solid,
    System,
}

/// Key arrangement: the Windows 7 calculator's, or the macOS Calculator's (one block
/// per mode, operators in the right column, segmented controls instead of radios).
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KeyLayout {
    #[default]
    Classic,
    Mac,
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Weight {
    #[default]
    Regular,
    Bold,
}

/// Size (logical px, THM-009) and weight of one text role.
#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(deny_unknown_fields)]
pub struct FontRole {
    pub size: f32,
    #[serde(default)]
    pub weight: Weight,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Fonts {
    /// Regular face: a .ttf/.otf path, absolute or relative to the themes folder.
    /// Empty = the bundled Inter (THM-011).
    pub file: String,
    /// Bold face; empty = bundled Inter Bold.
    pub bold_file: String,
    /// Menus, labels, radio buttons and the extras panel.
    pub ui: FontRole,
    pub button: FontRole,
    /// The main display number.
    pub display: FontRole,
    pub expression: FontRole,
    /// `M` and `(=n` indicators.
    pub indicator: FontRole,
    /// History and statistics list.
    pub history: FontRole,
    pub bits: FontRole,
    /// Aero frame caption.
    pub caption: FontRole,
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(deny_unknown_fields)]
pub struct Spacing {
    /// Space around the calculator body and between its sections.
    pub padding: f32,
    /// Space between buttons.
    pub gap: f32,
    pub button_width: f32,
    pub button_height: f32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WindowCfg {
    pub background: Gradient,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MenuCfg {
    pub height: f32,
    pub background: Gradient,
    pub text: Color,
    pub hover: Color,
    pub hover_border: Color,
    pub popup_background: Color,
    pub popup_border: Color,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct DisplayCfg {
    pub background: Gradient,
    pub border: Color,
    pub radius: f32,
    pub height: f32,
    pub padding: f32,
    pub text: Color,
    pub expression: Color,
    pub indicator: Color,
    pub history_text: Color,
    pub history_hover: Color,
    pub history_lines: usize,
    pub selection: Color,
    pub selection_text: Color,
}

/// Look of one button group in one state.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ButtonState {
    pub fill: Gradient,
    pub border: Color,
    pub text: Color,
    /// Inner 1 px highlight line (the Win7 gloss); transparent to disable.
    pub highlight: Color,
    /// Corner radius; defaults to `button.radius`.
    #[serde(default)]
    pub radius: Option<f32>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct States {
    pub normal: ButtonState,
    pub hover: ButtonState,
    pub pressed: ButtonState,
    pub disabled: ButtonState,
    /// Keyboard focus (UI-005).
    pub focus: ButtonState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Normal,
    Hover,
    Pressed,
    Disabled,
    Focus,
}

impl States {
    pub fn get(&self, s: State) -> &ButtonState {
        match s {
            State::Normal => &self.normal,
            State::Hover => &self.hover,
            State::Pressed => &self.pressed,
            State::Disabled => &self.disabled,
            State::Focus => &self.focus,
        }
    }
}

/// Button tokens. In the file, `[button.default.<state>]` applies to every group and
/// `[button.<group>.<state>]` overrides it for one group.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Buttons {
    pub radius: f32,
    pub digit: States,
    pub operator: States,
    pub memory: States,
    pub function: States,
    pub equals: States,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Digit,
    Operator,
    Memory,
    Function,
    Equals,
}

impl Buttons {
    pub fn group(&self, g: Group) -> &States {
        match g {
            Group::Digit => &self.digit,
            Group::Operator => &self.operator,
            Group::Memory => &self.memory,
            Group::Function => &self.function,
            Group::Equals => &self.equals,
        }
    }

    pub fn style(&self, g: Group, s: State) -> &ButtonState {
        self.group(g).get(s)
    }

    pub fn radius_of(&self, b: &ButtonState) -> f32 {
        b.radius.unwrap_or(self.radius)
    }
}

/// Radio buttons, group boxes and the extras panel.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ControlsCfg {
    pub text: Color,
    pub background: Color,
    pub border: Color,
    pub accent: Color,
    pub group_border: Color,
    pub hover: Color,
    /// Keyboard focus ring.
    pub focus: Color,
}

/// Programmer-mode bit panel.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BitsCfg {
    pub text: Color,
    pub dim: Color,
    pub index: Color,
}

/// Optional SVG imagery (DPI-005); paths are relative to the themes folder.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Imagery {
    /// Drawn over the window background, stretched to the window.
    #[serde(default)]
    pub window: Option<String>,
}

// ---- loading --------------------------------------------------------------------------

/// Why a theme could not be loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeError {
    pub theme: String,
    pub message: String,
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "theme {:?}: {}", self.theme, self.message)
    }
}

impl std::error::Error for ThemeError {}

/// A theme available for selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub builtin: bool,
}

/// Finds and loads built-in and user themes.
#[derive(Clone, Debug, Default)]
pub struct ThemeStore {
    user_dir: Option<PathBuf>,
}

/// Recursively overlays `over` onto `base`.
fn merge(base: &mut toml::Table, over: toml::Table) {
    for (k, v) in over {
        match (base.get_mut(&k), v) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => merge(b, o),
            (_, v) => {
                base.insert(k, v);
            }
        }
    }
}

fn builtin_source(id: &str) -> Option<&'static str> {
    BUILTIN.iter().find(|(b, _)| *b == id).map(|(_, s)| *s)
}

/// Parses TOML and checks the schema version.
fn parse(src: &str) -> Result<toml::Table, String> {
    if src.len() as u64 > MAX_FILE_SIZE {
        return Err(format!("file is larger than {MAX_FILE_SIZE} bytes"));
    }
    let t: toml::Table = src.parse().map_err(|e: toml::de::Error| e.to_string())?;
    match t.get("schema") {
        Some(toml::Value::Integer(v)) if *v == SCHEMA as i64 => Ok(t),
        Some(toml::Value::Integer(v)) if *v > SCHEMA as i64 => Err(format!(
            "schema {v} is newer than this version supports ({SCHEMA})"
        )),
        Some(v) => Err(format!("unsupported schema {v}")),
        None => Err(format!(
            "missing `schema = {SCHEMA}` (themes from before SevenPlus 0.2 need updating)"
        )),
    }
}

/// Folds `button.default` into every button group.
fn resolve_buttons(t: &mut toml::Table) -> Result<(), String> {
    let Some(toml::Value::Table(button)) = t.get_mut("button") else {
        return Ok(());
    };
    let default = match button.remove("default") {
        Some(toml::Value::Table(d)) => d,
        Some(_) => return Err("`button.default` must be a table".into()),
        None => toml::Table::new(),
    };
    for g in GROUPS {
        let mut states = default.clone();
        match button.remove(g) {
            Some(toml::Value::Table(over)) => merge(&mut states, over),
            Some(_) => return Err(format!("`button.{g}` must be a table")),
            None => {}
        }
        button.insert(g.into(), toml::Value::Table(states));
    }
    Ok(())
}

impl Theme {
    /// The default built-in theme.
    pub fn builtin() -> Theme {
        ThemeStore::default()
            .load(DEFAULT_THEME)
            .expect("built-in default theme is valid")
    }

    /// Parses a theme from TOML, resolving `extends` against the built-in themes.
    pub fn parse(src: &str) -> Result<Theme, String> {
        let store = ThemeStore::default();
        let table = parse(src)?;
        let mut seen = HashSet::new();
        let table = store.with_base(table, None, 0, &mut seen)?;
        Self::from_table(table)
    }

    fn from_table(mut t: toml::Table) -> Result<Theme, String> {
        resolve_buttons(&mut t)?;
        let theme: Theme = toml::Value::Table(t)
            .try_into()
            .map_err(|e: toml::de::Error| e.to_string())?;
        theme.validate()?;
        Ok(theme)
    }

    /// Range checks that the type system cannot express.
    fn validate(&self) -> Result<(), String> {
        let f = &self.font;
        for (role, r) in [
            ("ui", f.ui),
            ("button", f.button),
            ("display", f.display),
            ("expression", f.expression),
            ("indicator", f.indicator),
            ("history", f.history),
            ("bits", f.bits),
            ("caption", f.caption),
        ] {
            if !(4.0..=200.0).contains(&r.size) {
                return Err(format!("font.{role}.size must be between 4 and 200"));
            }
        }
        let s = &self.spacing;
        for (name, v, min) in [
            ("spacing.padding", s.padding, 0.0),
            ("spacing.gap", s.gap, 0.0),
            ("spacing.button_width", s.button_width, 8.0),
            ("spacing.button_height", s.button_height, 8.0),
            ("display.height", self.display.height, 16.0),
            ("display.padding", self.display.padding, 0.0),
            ("menu.height", self.menu.height, 8.0),
        ] {
            if !(min..=500.0).contains(&v) {
                return Err(format!("{name} must be between {min} and 500"));
            }
        }
        if !(1..=20).contains(&self.display.history_lines) {
            return Err("display.history_lines must be between 1 and 20".into());
        }
        Ok(())
    }

    /// Resolves the configured font file, relative to `themes_dir`.
    pub fn font_path(&self, themes_dir: &Path, bold: bool) -> Option<PathBuf> {
        let f = if bold {
            &self.font.bold_file
        } else {
            &self.font.file
        };
        let f = f.trim();
        (!f.is_empty()).then(|| themes_dir.join(f))
    }

    /// True when the window background is dark (for the OS-level light/dark hint).
    pub fn is_dark(&self) -> bool {
        self.window.background.first().luminance() < 0.18
    }
}

impl ThemeStore {
    /// A store over the built-ins plus the `*.toml` files in `user_dir` (THM-007).
    pub fn new(user_dir: Option<PathBuf>) -> ThemeStore {
        ThemeStore { user_dir }
    }

    pub fn user_dir(&self) -> Option<&Path> {
        self.user_dir.as_deref()
    }

    /// The user file for `id`, if one exists.
    pub fn user_path(&self, id: &str) -> Option<PathBuf> {
        let valid = !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '.'))
            && !id.starts_with('.');
        let p = self.user_dir.as_ref()?.join(format!("{id}.toml"));
        (valid && p.is_file()).then_some(p)
    }

    fn read_user(&self, id: &str) -> Option<Result<String, String>> {
        let p = self.user_path(id)?;
        Some((|| {
            let len = std::fs::metadata(&p).map_err(|e| e.to_string())?.len();
            if len > MAX_FILE_SIZE {
                return Err(format!("file is larger than {MAX_FILE_SIZE} bytes"));
            }
            std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
        })())
    }

    /// Built-in and user themes, built-ins first (THM-008).
    pub fn list(&self) -> Vec<ThemeInfo> {
        let mut out: Vec<ThemeInfo> = BUILTIN
            .iter()
            .filter(|(id, _)| self.user_path(id).is_none())
            .map(|(id, src)| ThemeInfo {
                id: id.to_string(),
                name: theme_name(src).unwrap_or_else(|| id.to_string()),
                builtin: true,
            })
            .collect();
        let mut user: Vec<ThemeInfo> = self
            .user_dir
            .as_ref()
            .and_then(|d| std::fs::read_dir(d).ok())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .filter_map(|p| {
                let id = p.file_stem()?.to_string_lossy().into_owned();
                self.user_path(&id)?;
                let name = self
                    .read_user(&id)
                    .and_then(Result::ok)
                    .and_then(|s| theme_name(&s))
                    .unwrap_or_else(|| id.clone());
                Some(ThemeInfo {
                    id,
                    name,
                    builtin: false,
                })
            })
            .collect();
        user.sort_by_key(|a| a.name.to_lowercase());
        out.extend(user);
        out
    }

    /// Loads a theme by id, resolving `extends` (THM-006) and validating it (THM-004).
    pub fn load(&self, id: &str) -> Result<Theme, ThemeError> {
        let err = |message: String| ThemeError {
            theme: id.to_string(),
            message,
        };
        let mut seen = HashSet::new();
        let table = self.resolve(id, true, 0, &mut seen).map_err(err)?;
        Theme::from_table(table).map_err(err)
    }

    /// The merged TOML table of `id` and everything it extends.
    fn resolve(
        &self,
        id: &str,
        allow_user: bool,
        depth: usize,
        seen: &mut HashSet<(String, bool)>,
    ) -> Result<toml::Table, String> {
        if depth > MAX_DEPTH {
            return Err(format!("`extends` chain is longer than {MAX_DEPTH}"));
        }
        let user = if allow_user { self.read_user(id) } else { None };
        let is_user = user.is_some();
        if !seen.insert((id.to_string(), is_user)) {
            return Err(format!("`extends` cycle through {id:?}"));
        }
        let src = match user {
            Some(r) => r?,
            None => builtin_source(id)
                .ok_or_else(|| format!("no theme named {id:?}"))?
                .to_string(),
        };
        let table = parse(&src).map_err(|e| if depth == 0 { e } else { format!("{id}: {e}") })?;
        let builtin_id = (!is_user).then_some(id);
        self.with_base(table, builtin_id, depth, seen)
    }

    /// Merges `table` over its base theme.
    fn with_base(
        &self,
        table: toml::Table,
        builtin_id: Option<&str>,
        depth: usize,
        seen: &mut HashSet<(String, bool)>,
    ) -> Result<toml::Table, String> {
        let extends = match table.get("extends") {
            Some(toml::Value::String(s)) => Some(s.clone()),
            Some(_) => return Err("`extends` must be a theme name".into()),
            None => None,
        };
        let base_id = match extends {
            Some(b) => Some(b),
            // The default built-in is the root of every chain.
            None if builtin_id == Some(DEFAULT_THEME) => None,
            None => Some(DEFAULT_THEME.to_string()),
        };
        let Some(base_id) = base_id else {
            return Ok(table);
        };
        // Extending one's own id (a user file shadowing a built-in) reaches the built-in.
        let allow_user =
            builtin_source(&base_id).is_none() || !seen.contains(&(base_id.clone(), true));
        let mut base = self.resolve(&base_id, allow_user, depth + 1, seen)?;
        merge(&mut base, table);
        Ok(base)
    }
}

/// The `name` of a theme source, without validating the rest.
fn theme_name(src: &str) -> Option<String> {
    let t: toml::Table = src.parse().ok()?;
    t.get("name")?.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_dir(files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "calc-theme-test-{}-{}",
            std::process::id(),
            files.first().map_or("empty", |f| f.0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, src) in files {
            std::fs::write(dir.join(format!("{name}.toml")), src).unwrap();
        }
        dir
    }

    #[test]
    fn builtin_themes_load() {
        let store = ThemeStore::default();
        for (id, _) in BUILTIN {
            store.load(id).unwrap_or_else(|e| panic!("{e}"));
        }
        assert_eq!(store.list().len(), BUILTIN.len());
        assert_eq!(Theme::builtin().name, "Aero 7");
        // aero7 is glass; themes built on it opt out unless they are glass themes too.
        for (id, _) in BUILTIN {
            let glass = matches!(*id, "aero7" | "macos" | "glass" | "glass-dark");
            let backdrop = store.load(id).unwrap().backdrop;
            assert_eq!(backdrop == Backdrop::System, glass, "{id}");
            let layout = store.load(id).unwrap().layout;
            let mac = *id == "macos";
            assert_eq!(layout == KeyLayout::Mac, mac, "{id}");
        }
    }

    #[test]
    fn partial_theme_inherits_defaults() {
        let t = Theme::parse("schema = 1\nname = \"x\"\n[button.digit.normal]\ntext = \"#ff0000\"")
            .unwrap();
        assert_eq!(t.button.digit.normal.text, Color::rgb(255, 0, 0));
        assert_ne!(t.button.operator.normal.text, Color::rgb(255, 0, 0));
        assert_eq!(
            t.spacing.button_width,
            Theme::builtin().spacing.button_width
        );
    }

    #[test]
    fn default_group_applies_to_all() {
        let t =
            Theme::parse("schema = 1\nname = \"x\"\n[button.default.hover]\nborder = \"#123456\"")
                .unwrap();
        for g in [Group::Digit, Group::Operator, Group::Equals] {
            assert_eq!(
                t.button.style(g, State::Hover).border,
                Color::rgb(0x12, 0x34, 0x56)
            );
        }
    }

    #[test]
    fn extends_chain() {
        let t = Theme::parse("schema = 1\nname = \"x\"\nextends = \"dark\"").unwrap();
        let dark = ThemeStore::default().load("dark").unwrap();
        assert_eq!(t.display.text, dark.display.text);
        assert_eq!(t.name, "x");
    }

    #[test]
    fn errors_are_reported() {
        let e = |src: &str| Theme::parse(src).unwrap_err();
        assert!(e("schema = 1\nname = \"x\"\n[spacing]\nwidht = 3.0").contains("widht"));
        assert!(e("schema = 1\nname = \"x\"\n[display]\ntext = \"red\"").contains("invalid color"));
        assert!(e("name = \"x\"").contains("schema"));
        assert!(e("schema = 2\nname = \"x\"").contains("newer"));
        assert!(e("schema = 1\nname = \"x\"\nextends = \"nope\"").contains("nope"));
        assert!(
            e("schema = 1\nname = \"x\"\n[window]\nbackground = [\"#000000\", \"#000000\", \"#000000\", \"#000000\", \"#000000\"]")
                .contains("1 to 4")
        );
        assert!(e("schema = 1\nname = \"x\"\n[font.ui]\nsize = 0.5").contains("font.ui.size"));
        let big = format!(
            "schema = 1\nname = \"x\"\n#{}",
            "x".repeat(MAX_FILE_SIZE as usize)
        );
        assert!(e(&big).contains("larger"));
    }

    #[test]
    fn user_themes_shadow_and_extend() {
        let dir = user_dir(&[
            (
                "dark",
                "schema = 1\nname = \"My dark\"\nextends = \"dark\"\n[display]\ntext = \"#ff00ff\"",
            ),
            ("mine", "schema = 1\nname = \"Mine\"\nextends = \"dark\""),
            // The Aero frame is gone; its old `[titlebar]` table is ignored.
            (
                "legacy",
                "schema = 1\nname = \"Legacy\"\n[titlebar]\nheight = 30.0\nborder = \"#000000\"",
            ),
            ("loop-a", "schema = 1\nname = \"A\"\nextends = \"loop-b\""),
            ("loop-b", "schema = 1\nname = \"B\"\nextends = \"loop-a\""),
            (
                "broken",
                "schema = 1\nname = \"Broken\"\n[menu]\nheight = \"tall\"",
            ),
        ]);
        let store = ThemeStore::new(Some(dir.clone()));
        let dark = store.load("dark").unwrap();
        assert_eq!(dark.name, "My dark");
        assert_eq!(dark.display.text, Color::rgb(255, 0, 255));
        // "mine" extends the user's "dark", which extends the built-in "dark".
        assert_eq!(
            store.load("mine").unwrap().display.text,
            Color::rgb(255, 0, 255)
        );
        assert!(store.load("legacy").is_ok());
        assert!(store.load("loop-a").unwrap_err().message.contains("cycle"));
        assert!(store.load("broken").is_err());
        let ids: Vec<String> = store.list().into_iter().map(|t| t.id).collect();
        assert!(ids.contains(&"mine".to_string()) && ids.contains(&"broken".to_string()));
        assert_eq!(ids.iter().filter(|i| *i == "dark").count(), 1);
        assert!(store.user_path("../etc/passwd").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// UI-023: every built-in theme meets WCAG 2.1 AA (4.5:1) for button labels and
    /// display text in every enabled state.
    #[test]
    fn builtin_themes_meet_wcag_aa() {
        let store = ThemeStore::default();
        let mut failures = Vec::new();
        for (id, _) in BUILTIN {
            let t = store.load(id).unwrap();
            let window = t.window.background.first();
            let mut check = |what: String, fg: Color, bg: &[Color], under: Color| {
                for stop in bg {
                    let bg = stop.over(under);
                    let r = fg.over(bg).contrast(bg);
                    if r < 4.5 {
                        failures.push(format!("{id}: {what}: {r:.2}:1"));
                    }
                }
            };
            for g in [
                Group::Digit,
                Group::Operator,
                Group::Memory,
                Group::Function,
                Group::Equals,
            ] {
                for s in [State::Normal, State::Hover, State::Pressed, State::Focus] {
                    let b = t.button.style(g, s);
                    check(format!("{g:?}/{s:?} label"), b.text, b.fill.stops(), window);
                }
            }
            let d = &t.display;
            let disp = d.background.first().over(window);
            for (what, c) in [
                ("display text", d.text),
                ("expression", d.expression),
                ("indicator", d.indicator),
                ("history", d.history_text),
            ] {
                check(what.into(), c, d.background.stops(), window);
            }
            check(
                "history hover".into(),
                d.history_text,
                &[d.history_hover],
                disp,
            );
            check("selection".into(), d.selection_text, &[d.selection], disp);
            check(
                "menu".into(),
                t.menu.text,
                t.menu.background.stops(),
                window,
            );
            check(
                "menu popup".into(),
                t.menu.text,
                &[t.menu.popup_background],
                window,
            );
            check(
                "controls".into(),
                t.controls.text,
                &[t.controls.background],
                window,
            );
            check(
                "radio labels".into(),
                t.controls.text,
                t.window.background.stops(),
                window,
            );
            check(
                "bits".into(),
                t.bits.text,
                t.window.background.stops(),
                window,
            );
            check(
                "bit index".into(),
                t.bits.index,
                t.window.background.stops(),
                window,
            );
            // Glass: text on the see-through window must stay readable whatever is
            // behind it, at the default opacity setting (0.85).
            if t.backdrop == Backdrop::System {
                let surfaces = [
                    ("radio labels", t.controls.text, None),
                    ("bits", t.bits.text, None),
                    ("bit index", t.bits.index, None),
                    ("menu", t.menu.text, Some(t.menu.background.first())),
                    (
                        "display text",
                        t.display.text,
                        Some(t.display.background.first()),
                    ),
                ];
                for (behind_name, behind) in [
                    ("black", Color::rgb(0, 0, 0)),
                    ("white", Color::rgb(255, 255, 255)),
                ] {
                    for &Color([r, g, b, a]) in t.window.background.stops() {
                        let tint = Color([r, g, b, (a as f32 * 0.85).round() as u8]).over(behind);
                        for (what, fg, surface) in surfaces {
                            let bg = surface.map_or(tint, |s| s.over(tint));
                            let ratio = fg.over(bg).contrast(bg);
                            if ratio < 4.5 {
                                failures
                                    .push(format!("{id}: {what} over {behind_name}: {ratio:.2}:1"));
                            }
                        }
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "contrast below 4.5:1:\n{}",
            failures.join("\n")
        );
    }
}
