//! The Programmer toolbar between the display and the bit panel: an ASCII/Unicode
//! switch, a text field that shows the value as characters and sets it when edited,
//! and the Byte flip / Word flip keys.

use crate::app::App;
use crate::i18n::tr;
use crate::layout::TOOLS;
use crate::paint;
use calc_core::{CharMode, Num, from_chars, to_chars};
use calc_theme::KeyLayout;
use eframe::egui::{
    self, Align, Color32, FontId, Id, Rect, Stroke, StrokeKind, TextEdit, pos2, vec2,
};

/// Horizontal padding inside the switch halves and buttons.
const PAD_X: f32 = 10.0;

/// The text field's id, so Ctrl+I can focus it.
pub(crate) fn chars_field_id() -> Id {
    Id::new("chars-field")
}

impl App {
    pub(crate) fn draw_tools(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let mac = self.theme.layout == KeyLayout::Mac;
        // The macOS toolbar is compact: UI-size text, tight segments.
        let (font, pad_x) = if mac {
            (paint::font(self.theme.font.ui), PAD_X * 0.8)
        } else {
            (paint::font(self.theme.font.button), PAD_X)
        };
        let width = |text: &str| {
            ui.painter()
                .layout_no_wrap(text.to_string(), font.clone(), Color32::BLACK)
                .size()
                .x
                + 2.0 * pad_x
        };
        let g = self.theme.spacing.gap.max(4.0);

        let modes = [
            (CharMode::Ascii, tr("chars-ascii")),
            (CharMode::Unicode, tr("chars-unicode")),
        ];
        // ASCII | Unicode switch, flush left: the same segmented control in every theme.
        let w: f32 = modes.iter().map(|(_, l)| width(l)).sum();
        let r = Rect::from_min_size(rect.min, vec2(w, rect.height()));
        let items: Vec<(&str, String)> =
            modes.iter().map(|(_, l)| (l.as_str(), l.clone())).collect();
        let sel = modes
            .iter()
            .position(|(m, _)| *m == self.settings.programmer_chars)
            .unwrap_or(0);
        if let Some(i) = self.segmented(ui, r, &items, sel, "chars-mode") {
            self.settings.programmer_chars = modes[i].0;
        }
        let x = r.right();
        let field_left = x + g;

        // Byte flip / Word flip, flush right. The macOS layout has them as keys.
        let mut right = rect.right();
        let mut keys = Vec::new();
        let flips = if mac { &[][..] } else { TOOLS };
        for b in flips.iter().rev() {
            let w = width(b.label);
            keys.push((
                Rect::from_min_size(pos2(right - w, rect.top()), vec2(w, rect.height())),
                b,
            ));
            right -= w + g;
        }
        let mut clicked = None;
        for (r, b) in keys.into_iter().rev() {
            if let Some(key) =
                self.draw_button(ui, paint::snap(r, ui.ctx().pixels_per_point()), b, "tools")
            {
                clicked = Some(key);
            }
        }

        // The text field fills the space between.
        let field = Rect::from_min_max(pos2(field_left, rect.top()), pos2(right, rect.bottom()));
        self.draw_chars_field(ui, field, font);

        if let Some(key) = clicked {
            let now = ui.input(|i| i.time);
            self.press(key, now);
        }
    }

    /// The value as text. While it is edited, each change that encodes to a value
    /// loads it; otherwise the field follows the value.
    fn draw_chars_field(&mut self, ui: &mut egui::Ui, rect: Rect, font: FontId) {
        let id = chars_field_id();
        let mode = self.settings.programmer_chars;
        let editing = ui.memory(|m| m.has_focus(id));
        if !editing {
            let text = to_chars(self.calc.bits(), self.calc.word, mode);
            self.chars_text = readable(ui.ctx(), &font, &text);
        }
        // Part of the calculator body: scales with the window, unlike dialogs.
        ui.set_style(self.body_style.clone());
        let resp = ui.put(
            rect,
            TextEdit::singleline(&mut self.chars_text)
                .id(id)
                .font(font)
                .vertical_align(Align::Center)
                .hint_text(tr("chars-hint")),
        );
        if resp.gained_focus() {
            // Edit the real characters, not their U+XXXX stand-ins.
            self.chars_text = to_chars(self.calc.bits(), self.calc.word, mode);
        }
        if !editing && !resp.has_focus() {
            return;
        }
        let value = if self.chars_text.is_empty() {
            Some(0)
        } else {
            from_chars(&self.chars_text, mode)
        };
        match value {
            Some(v) if resp.changed() => self.calc.load_value(Num::from_i128(v as i128)),
            Some(_) => {}
            None => {
                ui.painter().rect_stroke(
                    rect,
                    paint::radius(2.0),
                    Stroke::new(1.5, ui.visuals().error_fg_color),
                    StrokeKind::Outside,
                );
                resp.on_hover_text(tr("chars-invalid"));
            }
        }
    }
}

/// `text` with characters the font cannot draw shown as `U+XXXX`.
fn readable(ctx: &egui::Context, font: &FontId, text: &str) -> String {
    ctx.fonts_mut(|f| {
        text.chars()
            .map(|ch| {
                if f.has_glyph(font, ch) {
                    ch.to_string()
                } else {
                    format!("U+{:04X}", ch as u32)
                }
            })
            .collect()
    })
}
