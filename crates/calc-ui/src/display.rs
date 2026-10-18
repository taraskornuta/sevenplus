//! The display: history or statistics list, expression line and value line.

use crate::app::App;
use crate::i18n::{error_text, num, tr, tr_args};
use crate::layout::line_h;
use crate::paint::{self, c};
use calc_core::{Angle, Base, Mode, Num, WordSize};
use calc_format::{FormatSettings, Notation, Value, format};
use calc_theme::KeyLayout;
use eframe::egui::{
    self, Align2, FontId, Id, Painter, Rect, Sense, Stroke, StrokeKind, WidgetInfo, WidgetType,
    pos2, vec2,
};

/// Space kept free left of the value for the `M` indicator.
const INDICATOR_W: f32 = 16.0;

impl App {
    /// The unit the display is in: radix and word size, or the angle unit.
    fn unit_hint(&self) -> Option<String> {
        match self.calc.mode {
            Mode::Programmer => {
                let radix = match self.calc.base {
                    Base::Hex => "hint-hex",
                    Base::Dec => "hint-dec",
                    Base::Oct => "hint-oct",
                    Base::Bin => "hint-bin",
                };
                let word = match self.calc.word {
                    WordSize::Qword => "hint-qword",
                    WordSize::Dword => "hint-dword",
                    WordSize::Word => "hint-word",
                    WordSize::Byte => "hint-byte",
                };
                Some(format!("{} · {}", tr(radix), tr(word)))
            }
            Mode::Scientific => Some(tr(match self.calc.angle {
                Angle::Degrees => "hint-deg",
                Angle::Radians => "hint-rad",
                Angle::Gradians => "hint-grad",
            })),
            _ => None,
        }
    }

    /// A number formatted for display in the current mode and locale.
    pub(crate) fn display_num(&self, n: &Num) -> String {
        let s = FormatSettings {
            grouping: self.settings.digit_grouping,
            ..self.calc.format_settings()
        };
        format(&self.calc.to_value(n), &s, &self.locale())
    }

    pub(crate) fn draw_display(&mut self, ui: &egui::Ui, rect: Rect, history: bool) {
        let d = self.theme.display.clone();
        let fonts = self.theme.font.clone();
        let p = ui.painter().clone();
        paint::gradient_rect(&p, rect, d.radius, d.background.stops());
        p.rect_stroke(
            rect,
            paint::radius(d.radius),
            Stroke::new(1.0, c(d.border)),
            StrokeKind::Inside,
        );
        let inner = rect.shrink(d.padding);
        let mut top = inner.top();
        let lh = line_h(&self.theme);
        let list_h = d.history_lines as f32 * lh;
        let history_font = paint::font(fonts.history);
        let separator = |p: &Painter, y: f32| {
            p.line_segment(
                [pos2(inner.left(), y), pos2(inner.right(), y)],
                Stroke::new(1.0, c(d.border).gamma_multiply(0.5)),
            );
        };

        match self.calc.mode {
            Mode::Standard | Mode::Scientific if history => {
                let items: Vec<(String, Num)> = self
                    .calc
                    .history()
                    .iter()
                    .map(|h| {
                        let expr = calc_format::localize(&h.expr, &self.locale());
                        (
                            format!("{expr} = {}", self.display_num(&h.result)),
                            h.result.clone(),
                        )
                    })
                    .collect();
                let start = items.len().saturating_sub(d.history_lines);
                let mut load = None;
                for (row, (text, result)) in items[start..].iter().enumerate() {
                    let r = Rect::from_min_size(
                        pos2(inner.left(), top + row as f32 * lh),
                        vec2(inner.width(), lh),
                    );
                    let resp = ui.interact(r, Id::new(("hist", start + row)), Sense::click());
                    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, text));
                    if resp.hovered() || resp.has_focus() {
                        p.rect_filled(r, 2.0, c(d.history_hover));
                    }
                    p.text(
                        r.right_center() - vec2(2.0, 0.0),
                        Align2::RIGHT_CENTER,
                        text,
                        history_font.clone(),
                        c(d.history_text),
                    );
                    if resp.clicked() {
                        load = Some(result.clone());
                    }
                }
                // HIS-002: a selected entry's result becomes the current value.
                if let Some(n) = load {
                    self.calc.load_value(n);
                }
                top += list_h + d.padding;
                separator(&p, top - d.padding / 2.0);
            }
            Mode::Statistics => {
                let n = self.calc.stats().len();
                let start = n.saturating_sub(d.history_lines);
                let mut delete = None;
                for i in start..n {
                    let r = Rect::from_min_size(
                        pos2(inner.left(), top + (i - start) as f32 * lh),
                        vec2(inner.width(), lh),
                    );
                    let resp = ui.interact(r, Id::new(("stat", i)), Sense::click());
                    let text = self.display_num(&self.calc.stats()[i]);
                    let selected = self.stat_selected == Some(i);
                    resp.widget_info(|| {
                        WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &text)
                    });
                    let color = if selected {
                        p.rect_filled(r, 2.0, c(d.selection));
                        c(d.selection_text)
                    } else {
                        if resp.hovered() || resp.has_focus() {
                            p.rect_filled(r, 2.0, c(d.history_hover));
                        }
                        c(d.history_text)
                    };
                    p.text(
                        r.right_center() - vec2(2.0, 0.0),
                        Align2::RIGHT_CENTER,
                        text,
                        history_font.clone(),
                        color,
                    );
                    if resp.clicked() {
                        self.stat_selected = if selected { None } else { Some(i) };
                    }
                    resp.context_menu(|ui| {
                        if ui.button(tr("stat-delete")).clicked() {
                            delete = Some(i);
                        }
                    });
                }
                if let Some(i) = delete {
                    self.calc.stat_delete(i);
                    self.stat_selected = None;
                }
                let count = Rect::from_min_size(
                    pos2(inner.left(), inner.top() + list_h),
                    vec2(inner.width(), lh),
                );
                p.text(
                    count.right_center() - vec2(2.0, 0.0),
                    Align2::RIGHT_CENTER,
                    tr_args("stat-count", &[("count", num(n as f64))]),
                    history_font.clone(),
                    c(d.expression),
                );
                top = count.bottom() + d.padding;
                separator(&p, top - d.padding / 2.0);
            }
            _ => {}
        }

        let state = self.calc.state();
        let locale = self.locale();

        // Expression line; long expressions are clipped on the left, like Win7.
        let expr_font = paint::font(fonts.expression);
        let indicator_font = paint::font(fonts.indicator);
        let expr_h = fonts.expression.size + 4.0;
        let expr_rect =
            Rect::from_min_max(pos2(inner.left(), top), pos2(inner.right(), top + expr_h));
        let expr = calc_format::localize(&state.expression, &locale);
        let g = p.layout_no_wrap(expr, expr_font, c(d.expression));
        let clip = Rect::from_min_max(pos2(inner.left() + 26.0, expr_rect.top()), expr_rect.max);
        p.with_clip_rect(clip).galley(
            pos2(
                expr_rect.right() - g.size().x,
                expr_rect.center().y - g.size().y / 2.0,
            ),
            g,
            c(d.expression),
        );
        if state.paren_depth > 0 {
            p.text(
                expr_rect.left_center(),
                Align2::LEFT_CENTER,
                format!("(={}", state.paren_depth),
                indicator_font.clone(),
                c(d.indicator),
            );
        }

        // Value line.
        let value_rect = Rect::from_min_max(pos2(inner.left(), expr_rect.bottom()), inner.max);
        if state.memory {
            p.text(
                value_rect.left_center(),
                Align2::LEFT_CENTER,
                "M",
                indicator_font.clone(),
                c(d.indicator),
            );
        }
        // A subdued hint of the unit the number is in: radix and word size, or angle unit.
        let hint = self.unit_hint();
        let mut reserved = INDICATOR_W;
        if let Some(h) = &hint {
            let left = if state.memory { INDICATOR_W } else { 0.0 };
            let r = p.text(
                value_rect.left_center() + vec2(left, 0.0),
                Align2::LEFT_CENTER,
                h,
                indicator_font.clone(),
                c(d.expression),
            );
            reserved = reserved.max(r.right() - value_rect.left() + 8.0);
        }
        let grouping = self.settings.digit_grouping;
        let text = match state.error {
            Some(e) => error_text(e),
            None => state.text(grouping, &locale),
        };
        // macOS layout: the radix follows the value as a subscript, e.g. 1F308₁₆.
        let radix = (self.theme.layout == KeyLayout::Mac
            && self.calc.mode == Mode::Programmer
            && state.error.is_none())
        .then(|| {
            let size = (fonts.display.size * 0.4).max(9.0);
            let font = FontId::new(size, paint::font(fonts.display).family);
            p.layout_no_wrap(self.calc.base.radix().to_string(), font, c(d.text))
        });
        let suffix_w = radix.as_ref().map_or(0.0, |g| g.size().x + 2.0);
        let avail = value_rect.width() - reserved - suffix_w;
        let (text, font) = fit_value(&p, &state, text, fonts.display, avail, grouping, &locale);
        let g = paint::tracked(&p, &text, font, c(d.text));
        let right = value_rect.right() - 1.0 - suffix_w;
        let at = pos2(right - g.size().x, value_rect.center().y - g.size().y / 2.0);
        let value_bottom = at.y + g.size().y;
        p.with_clip_rect(value_rect).galley(at, g, c(d.text));
        if let Some(r) = radix {
            // Sits on the value's baseline, a little below it, but never below the
            // value line, where it would be clipped.
            let y = (value_bottom - r.size().y * 0.85).min(value_rect.bottom() - r.size().y);
            let pos = pos2(right + 2.0, y);
            p.with_clip_rect(value_rect).galley(pos, r, c(d.text));
        }

        // Double-click copies the value. CLICK without FOCUSABLE keeps the Tab order.
        let resp = ui
            .interact(value_rect, Id::new("display-value"), Sense::CLICK)
            .on_hover_text(tr("display-copy-hint"));
        if resp.double_clicked() {
            ui.ctx().copy_text(self.copy_text());
        }

        // Screen readers: the display is a polite live region (UI-020, UI-021).
        let spoken = text.clone();
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &spoken));
        let label = match &hint {
            Some(h) => format!("{}, {h}", tr("display-label")),
            None => tr("display-label"),
        };
        ui.ctx().accesskit_node_builder(resp.id, |node| {
            node.set_live(egui::accesskit::Live::Polite);
            node.set_label(label);
            node.set_value(text.clone());
        });
        self.announced = text;
    }
}

/// UI-004: the display font steps down 2 px at a time to 50% of its size; a result that
/// still doesn't fit switches to exponential notation.
fn fit_value(
    p: &Painter,
    state: &calc_core::DisplayState,
    text: String,
    role: calc_theme::FontRole,
    avail: f32,
    grouping: bool,
    locale: &calc_format::Locale,
) -> (String, FontId) {
    let width = |t: &str, size: f32| {
        paint::tracked(p, t, paint::font_sized(role, size), egui::Color32::WHITE)
            .size()
            .x
    };
    let min = role.size * 0.5;
    let mut size = role.size;
    while width(&text, size) > avail && size - 2.0 >= min {
        size -= 2.0;
    }
    if width(&text, size) <= avail {
        return (text, paint::font_sized(role, size));
    }
    // Typed entries and errors can't change notation: shrink them to fit instead.
    let sci = match (&state.entry, state.error, &state.value) {
        (None, None, Value::Dec(_)) => Some(format(
            &state.value,
            &FormatSettings {
                notation: Notation::Exponential,
                grouping,
                ..state.format
            },
            locale,
        )),
        _ => None,
    };
    let text = sci.unwrap_or(text);
    let mut size = role.size;
    while width(&text, size) > avail && size - 2.0 >= min {
        size -= 2.0;
    }
    let w = width(&text, size);
    if w > avail {
        size = (size * avail / w).max(6.0);
    }
    (text, paint::font_sized(role, size))
}
