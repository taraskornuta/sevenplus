//! Theme files: TOML merged over the built-in Windows 7 theme.

use eframe::egui::Color32;
use serde::{Deserialize, Deserializer};
use std::path::{Path, PathBuf};

pub const BUILTIN: &[(&str, &str)] = &[
    ("win7", include_str!("../../../assets/themes/win7.toml")),
    (
        "win7-dark",
        include_str!("../../../assets/themes/win7-dark.toml"),
    ),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color(pub Color32);

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        parse_color(&s).map(Color).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "invalid color {s:?}, expected \"#rrggbb\" or \"#rrggbbaa\""
            ))
        })
    }
}

fn parse_color(s: &str) -> Option<Color32> {
    let h = s.strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            byte(6)?,
        )),
        _ => None,
    }
}

/// Vertical gradient: colors spread evenly from top to bottom.
pub type Gradient = Vec<Color>;

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub name: String,
    pub font: FontCfg,
    pub window: WindowCfg,
    pub menu: MenuCfg,
    pub titlebar: TitlebarCfg,
    pub display: DisplayCfg,
    pub button: ButtonsCfg,
    pub controls: ControlsCfg,
    pub bits: BitsCfg,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FontCfg {
    pub file: String,
    pub size: f32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WindowCfg {
    pub background: Gradient,
    pub padding: f32,
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
pub struct TitlebarCfg {
    pub height: f32,
    pub border_width: f32,
    pub radius: f32,
    pub background: Gradient,
    pub border: Color,
    pub inner_border: Color,
    pub text: Color,
    pub text_glow: Color,
    pub text_size: f32,
    pub button_height: f32,
    pub min_width: f32,
    pub max_width: f32,
    pub close_width: f32,
    pub button: Gradient,
    pub button_hover: Gradient,
    pub button_border: Color,
    pub close: Gradient,
    pub close_hover: Gradient,
    pub glyph: Color,
    pub glyph_outline: Color,
    pub glyph_disabled: Color,
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
    pub expression_text: Color,
    pub indicator_text: Color,
    pub value_size: f32,
    pub expression_size: f32,
    pub indicator_size: f32,
    pub history_text: Color,
    pub history_size: f32,
    pub history_lines: usize,
    pub history_hover: Color,
    pub selection: Color,
    pub selection_text: Color,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ButtonState {
    pub gradient: Gradient,
    pub border: Color,
    pub highlight: Color,
    pub text: Color,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields, default)]
pub struct KindOverride {
    pub gradient: Option<Gradient>,
    pub border: Option<Color>,
    pub text: Option<Color>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ButtonsCfg {
    pub width: f32,
    pub height: f32,
    pub gap: f32,
    pub radius: f32,
    pub font_size: f32,
    pub normal: ButtonState,
    pub hover: ButtonState,
    pub pressed: ButtonState,
    pub disabled: ButtonState,
    #[serde(default)]
    pub digit: KindOverride,
    #[serde(default)]
    pub operator: KindOverride,
    #[serde(default)]
    pub memory: KindOverride,
    #[serde(default)]
    pub function: KindOverride,
    #[serde(default)]
    pub equals: KindOverride,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ControlsCfg {
    pub text: Color,
    pub background: Color,
    pub border: Color,
    pub accent: Color,
    pub group_border: Color,
    pub hover: Color,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BitsCfg {
    pub text: Color,
    pub dim: Color,
    pub index: Color,
    pub size: f32,
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

impl Theme {
    pub fn builtin() -> Theme {
        Self::from_str(BUILTIN[0].1).expect("built-in theme is valid")
    }

    /// Parses theme TOML on top of the built-in Windows 7 defaults.
    pub fn from_str(src: &str) -> Result<Theme, String> {
        let mut base: toml::Table = BUILTIN[0].1.parse().expect("built-in theme is valid TOML");
        let over: toml::Table = src.parse().map_err(|e: toml::de::Error| e.to_string())?;
        merge(&mut base, over);
        toml::Value::Table(base)
            .try_into()
            .map_err(|e: toml::de::Error| e.to_string())
    }

    pub fn load(path: &Path) -> Result<Theme, String> {
        let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_str(&src).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Resolves the configured font file, relative paths being relative to `themes_dir`.
    pub fn font_path(&self, themes_dir: &Path) -> Option<PathBuf> {
        let f = self.font.file.trim();
        (!f.is_empty()).then(|| themes_dir.join(f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_themes_parse() {
        for (name, src) in BUILTIN {
            Theme::from_str(src).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }

    #[test]
    fn partial_theme_inherits_defaults() {
        let t = Theme::from_str("[button.normal]\ntext = \"#ff0000\"").unwrap();
        assert_eq!(t.button.normal.text.0, Color32::from_rgb(255, 0, 0));
        assert_eq!(t.button.width, Theme::builtin().button.width);
    }

    #[test]
    fn errors_are_reported() {
        assert!(
            Theme::from_str("[button]\nwidht = 3.0")
                .unwrap_err()
                .contains("widht")
        );
        assert!(
            Theme::from_str("[display]\ntext = \"red\"")
                .unwrap_err()
                .contains("invalid color")
        );
    }
}
