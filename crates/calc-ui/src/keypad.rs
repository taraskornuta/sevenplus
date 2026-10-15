//! Keypads, radio groups and the programmer bit panel.

use crate::app::{App, FLASH_SECS};
use crate::i18n::{num, tr, tr_args};
use crate::layout::{self, Button, Layout, bits_row_h};
use crate::paint::{self, RadioColors, c};
use calc_core::{Angle, Base, Key, Mode, WordSize};
use calc_theme::State;
use eframe::egui::{
    self, Align2, Color32, FontId, Id, Rect, Sense, WidgetInfo, WidgetType, pos2, vec2,
};

/// Plain-text form of label markup for screen readers.
fn plain(label: &str) -> String {
    label.replace(['^', '~', '{', '}'], "")
}

impl App {
    /// Draws the side block and the main keypad. Buttons are registered in reading
    /// order (row by row across both blocks) so Tab follows it (UI-022).
    pub(crate) fn draw_keypads(&mut self, ui: &egui::Ui, l: &Layout, off: egui::Vec2) {
        let ml = layout::for_mode(self.calc.mode);
        let sp = self.theme.spacing;
        let (bw, bh, g) = (sp.button_width, sp.button_height, sp.gap);
        let ppp = ui.ctx().pixels_per_point();
        let mut all: Vec<(Rect, &Button, &str)> = Vec::new();
        let mut place = |origin: egui::Pos2, keys: &'static [Button], block: &'static str| {
            for b in keys {
                let rect = Rect::from_min_size(
                    origin + off + vec2(b.col as f32 * (bw + g), b.row as f32 * (bh + g)),
                    vec2(
                        b.w as f32 * bw + (b.w - 1) as f32 * g,
                        b.h as f32 * bh + (b.h - 1) as f32 * g,
                    ),
                );
                all.push((paint::snap(rect, ppp), b, block));
            }
        };
        if let Some(side) = l.side {
            place(side, ml.side_keys, "side");
        }
        place(l.keypad, ml.keypad, "main");
        all.sort_by(|a, b| {
            (a.0.top().round() as i32, a.0.left().round() as i32)
                .cmp(&(b.0.top().round() as i32, b.0.left().round() as i32))
        });
        let mut clicked = None;
        for (rect, b, block) in all {
            if let Some(key) = self.draw_button(ui, rect, b, block) {
                clicked = Some(key);
            }
        }
        if let Some(key) = clicked {
            let now = ui.input(|i| i.time);
            self.press(key, now);
        }
    }

    /// One button; returns its key when clicked.
    pub(crate) fn draw_button(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        b: &Button,
        block: &str,
    ) -> Option<Key> {
        let t = &self.theme.button;
        let (label, key) = b.resolve(self.calc.inv);
        let enabled = self.calc.enabled(key);
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let resp = ui.interact(rect, Id::new((block, b.col, b.row)), sense);
        let name = match layout::a11y_id(key, label) {
            Some(id) => tr(id),
            None => plain(label),
        };
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &name));
        // Programmer operators may show their C sign; the word stays as name and tooltip.
        let c_sign = (self.calc.mode == Mode::Programmer && self.settings.programmer_c_signs)
            .then(|| layout::c_label(key))
            .flatten();
        let label = c_sign.unwrap_or(label);
        let resp = if c_sign.is_some() {
            resp.on_hover_text(&name)
        } else {
            resp
        };
        let now = ui.input(|i| i.time);
        let flashing = self
            .flash
            .is_some_and(|(k, t)| k == key && now - t < FLASH_SECS);
        // Inv is a toggle: drawn pressed while active.
        let toggled = key == Key::Inv && self.calc.inv;
        let state = if !enabled {
            State::Disabled
        } else if resp.is_pointer_button_down_on() || flashing || toggled {
            State::Pressed
        } else if resp.hovered() {
            State::Hover
        } else if resp.has_focus() {
            State::Focus
        } else {
            State::Normal
        };
        let s = t.style(b.kind, state);
        let p = ui.painter();
        paint::button_body(
            p,
            rect,
            t.radius_of(s),
            s.fill.stops(),
            c(s.border),
            c(s.highlight),
        );
        if resp.has_focus() && state != State::Focus {
            let f = t.style(b.kind, State::Focus);
            p.rect_stroke(
                rect.shrink(2.0),
                paint::radius(t.radius_of(f) - 1.0),
                egui::Stroke::new(1.0, c(f.border)),
                egui::StrokeKind::Inside,
            );
        }
        let font = paint::font(self.theme.font.button);
        paint::rich_label(p, rect.center(), label, font, c(s.text));
        resp.clicked().then_some(key)
    }

    fn radio_colors(&self) -> RadioColors {
        let k = &self.theme.controls;
        RadioColors {
            text: c(k.text),
            background: c(k.background),
            border: c(k.border),
            accent: c(k.accent),
            hover: c(k.hover),
            focus: c(k.focus),
        }
    }

    /// A group box of radio buttons laid out in `cols` columns; returns the clicked index.
    fn radio_group(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        labels: &[String],
        selected: usize,
        cols: usize,
        id: &str,
    ) -> Option<usize> {
        let p = ui.painter();
        paint::group_box(p, rect, c(self.theme.controls.group_border));
        let rows = labels.len().div_ceil(cols);
        let inner = rect.shrink2(vec2(6.0, 3.0));
        let rh = inner.height() / rows as f32;
        // Items in a row get their natural width; leftover space is spread between them.
        // Labels that don't fit with a minimum gap get a smaller font (long translations,
        // wide theme fonts).
        const MIN_GAP: f32 = 8.0;
        let measure = |size: f32| -> Vec<f32> {
            labels
                .iter()
                .map(|l| {
                    p.layout_no_wrap(l.clone(), FontId::proportional(size), Color32::BLACK)
                        .size()
                        .x
                        + size
                        + 5.0
                })
                .collect()
        };
        let mut size = self.theme.font.ui.size;
        let mut widths = measure(size);
        let fits = |w: &[f32]| {
            w.chunks(cols).all(|row| {
                row.iter().sum::<f32>() + MIN_GAP * (row.len() - 1) as f32 <= inner.width()
            })
        };
        while !fits(&widths) && size > 8.0 {
            size -= 0.5;
            widths = measure(size);
        }
        let colors = self.radio_colors();
        let mut clicked = None;
        for (i, label) in labels.iter().enumerate() {
            let (row, col) = (i / cols, i % cols);
            let row_items = &widths[row * cols..((row + 1) * cols).min(widths.len())];
            let spare =
                (inner.width() - row_items.iter().sum::<f32>()).max(0.0) / row_items.len() as f32;
            let x = inner.left() + row_items[..col].iter().map(|w| w + spare).sum::<f32>();
            let cell = Rect::from_min_size(
                pos2(x, inner.top() + row as f32 * rh),
                vec2(widths[i] + spare, rh),
            );
            let resp = ui.interact(cell, Id::new((id, i)), Sense::click());
            resp.widget_info(|| {
                WidgetInfo::selected(WidgetType::RadioButton, true, i == selected, label)
            });
            let pos = pos2(cell.left(), cell.center().y - (size + 1.0) / 2.0);
            let state = paint::RadioState {
                selected: i == selected,
                hovered: resp.hovered(),
                focused: resp.has_focus(),
            };
            paint::radio(p, pos, label, size, state, &colors);
            if resp.clicked() {
                clicked = Some(i);
            }
        }
        clicked
    }

    pub(crate) fn draw_radios(&mut self, ui: &egui::Ui, radios: &[Rect]) {
        let labels = |ids: &[&str]| ids.iter().map(|id| tr(id)).collect::<Vec<_>>();
        match self.calc.mode {
            Mode::Scientific => {
                let angles = [Angle::Degrees, Angle::Radians, Angle::Gradians];
                let sel = angles
                    .iter()
                    .position(|a| *a == self.calc.angle)
                    .unwrap_or(0);
                let names = labels(&["angle-degrees", "angle-radians", "angle-grads"]);
                if let Some(i) = self.radio_group(ui, radios[0], &names, sel, 3, "angle") {
                    self.calc.angle = angles[i];
                }
            }
            Mode::Programmer => {
                let bases = [Base::Hex, Base::Dec, Base::Oct, Base::Bin];
                let sel = bases.iter().position(|b| *b == self.calc.base).unwrap_or(1);
                let names = labels(&["base-hex", "base-dec", "base-oct", "base-bin"]);
                if let Some(i) = self.radio_group(ui, radios[0], &names, sel, 1, "base") {
                    self.calc.set_base(bases[i]);
                }
                let words = [
                    WordSize::Qword,
                    WordSize::Dword,
                    WordSize::Word,
                    WordSize::Byte,
                ];
                let sel = words.iter().position(|w| *w == self.calc.word).unwrap_or(0);
                let names = labels(&["word-qword", "word-dword", "word-word", "word-byte"]);
                if let Some(i) = self.radio_group(ui, radios[1], &names, sel, 1, "word") {
                    self.calc.set_word(words[i]);
                }
            }
            _ => {}
        }
    }

    /// The 64-bit panel in groups of four; clicking a bit toggles it (PRG-005).
    pub(crate) fn draw_bits(&mut self, ui: &egui::Ui, rect: Rect) {
        let b = self.theme.bits.clone();
        let size = self.theme.font.bits.size;
        let p = ui.painter().clone();
        let bits = self.calc.bits();
        let word = self.calc.word.bits();
        let cell = (rect.width() / 39.0).min(size * 0.9);
        let row_h = bits_row_h(&self.theme);
        let font = paint::font(self.theme.font.bits);
        let small = FontId::new(size * 0.85, font.family.clone());
        let mut toggle = None;
        for row in 0..2 {
            let y = rect.top() + row as f32 * row_h;
            let x0 = rect.center().x - 39.0 * cell / 2.0;
            for j in 0..32 {
                let bit = 63 - (row * 32 + j);
                let x = x0 + (j + j / 4) as f32 * cell;
                let r = Rect::from_min_size(pos2(x, y), vec2(cell, size + 4.0));
                let active = (bit as u32) < word;
                let on = (bits >> bit) & 1 == 1;
                let color = if active { c(b.text) } else { c(b.dim) };
                if active {
                    let resp = ui.interact(r, Id::new(("bit", bit)), Sense::click());
                    resp.widget_info(|| {
                        WidgetInfo::selected(
                            WidgetType::Checkbox,
                            true,
                            on,
                            tr_args("key-bit", &[("bit", num(bit as f64))]),
                        )
                    });
                    if resp.hovered() || resp.has_focus() {
                        p.rect_filled(r, 2.0, c(self.theme.controls.hover));
                    }
                    if resp.clicked() {
                        toggle = Some(bit as u32);
                    }
                }
                p.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    if on { "1" } else { "0" },
                    font.clone(),
                    color,
                );
                if [63, 47, 32, 31, 15, 0].contains(&bit) {
                    let align = if bit % 16 == 15 {
                        Align2::LEFT_TOP
                    } else {
                        Align2::RIGHT_TOP
                    };
                    let px = if bit % 16 == 15 { r.left() } else { r.right() };
                    p.text(
                        pos2(px, r.bottom()),
                        align,
                        bit.to_string(),
                        small.clone(),
                        c(b.index),
                    );
                }
            }
        }
        if let Some(bit) = toggle {
            self.calc.apply_key(Key::ToggleBit(bit));
        }
    }
}
