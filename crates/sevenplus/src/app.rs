//! The calculator window.

use crate::config::{self, Extras, FrameStyle, Settings};
use crate::extras::{self, ExtrasState};
use crate::layout::{self, Key, Kind};
use crate::paint::{self, RadioColors};
use crate::theme::{ButtonState, Theme};
use calc_core::keys::cmd_for_char;
use calc_core::{Angle, Base, Calculator, Cmd, Mode, Num, WordSize};
use eframe::egui::{
    self, Align2, Button, Color32, Context, Event, FontData, FontDefinitions, FontFamily, FontId,
    Id, Key as EKey, Modifiers, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, TextStyle,
    UiBuilder, Vec2, ViewportCommand, pos2, vec2,
};
use std::sync::Arc;
use std::time::SystemTime;

const SELAWIK: &[u8] = include_bytes!("../../../assets/fonts/selawk.ttf");
const EXTRAS_WIDTH: f32 = 300.0;
const EXTRAS_MIN_HEIGHT: f32 = 300.0;
const FLASH_SECS: f64 = 0.12;

/// Positions of every part of the calculator body, relative to the body origin.
struct Layout {
    size: Vec2,
    menu: Rect,
    display: Rect,
    bits: Option<Rect>,
    /// Origin of the main keypad block (standard or statistics).
    keypad: Pos2,
    /// Origin of the secondary block (scientific or programmer).
    side: Option<Pos2>,
    /// Radio groups: scientific angle box, or programmer base and word boxes.
    radios: Vec<Rect>,
    extras: Option<Rect>,
}

fn block_w(t: &Theme, cols: u8) -> f32 {
    cols as f32 * t.button.width + (cols.max(1) - 1) as f32 * t.button.gap
}

fn block_h(t: &Theme, rows: u8) -> f32 {
    rows as f32 * t.button.height + (rows.max(1) - 1) as f32 * t.button.gap
}

fn line_h(t: &Theme) -> f32 {
    t.display.history_size + 5.0
}

fn bits_row_h(t: &Theme) -> f32 {
    t.bits.size * 2.0 + 6.0
}

fn compute_layout(t: &Theme, mode: Mode, history: bool, extras: Extras) -> Layout {
    let (pad, g) = (t.window.padding, t.button.gap);
    let radio_w = block_w(t, 2) + 8.0;
    let side_w = match mode {
        Mode::Scientific => block_w(t, 5) + g,
        Mode::Programmer => radio_w + g + block_w(t, 3) + g,
        _ => 0.0,
    };
    let content_w = side_w + block_w(t, 5);
    let list_h = t.display.history_lines as f32 * line_h(t);
    let display_h = t.display.height
        + match mode {
            Mode::Standard | Mode::Scientific if history => list_h + t.display.padding,
            Mode::Statistics => list_h + line_h(t) + t.display.padding,
            _ => 0.0,
        };
    let menu = Rect::from_min_size(Pos2::ZERO, vec2(0.0, t.menu.height));
    let mut y = t.menu.height + pad;
    let display = Rect::from_min_size(pos2(pad, y), vec2(content_w, display_h));
    y += display_h + pad;
    let bits = (mode == Mode::Programmer).then(|| {
        let r = Rect::from_min_size(pos2(pad, y), vec2(content_w, 2.0 * bits_row_h(t)));
        y += r.height() + pad;
        r
    });
    let rows = if mode == Mode::Statistics { 5 } else { 6 };
    let keypad_h = block_h(t, rows);
    let keypad = pos2(pad + side_w, y);
    let (side, radios) = match mode {
        Mode::Scientific => (
            Some(pos2(pad, y + t.button.height + g)),
            vec![Rect::from_min_size(
                pos2(pad, y),
                vec2(block_w(t, 5), t.button.height),
            )],
        ),
        Mode::Programmer => {
            let h = (keypad_h - g) / 2.0;
            (
                Some(pos2(pad + radio_w + g, y)),
                vec![
                    Rect::from_min_size(pos2(pad, y), vec2(radio_w, h)),
                    Rect::from_min_size(pos2(pad, y + h + g), vec2(radio_w, h)),
                ],
            )
        }
        _ => (None, vec![]),
    };
    let mut size = vec2(pad * 2.0 + content_w, y + keypad_h + pad);
    let extras = (extras != Extras::None).then(|| {
        let top = t.menu.height + pad;
        let h = (size.y - top - pad).max(EXTRAS_MIN_HEIGHT);
        let r = Rect::from_min_size(pos2(size.x, top), vec2(EXTRAS_WIDTH, h));
        size = vec2(r.right() + pad, size.y.max(r.bottom() + pad));
        r
    });
    let menu = menu.with_max_x(size.x);
    Layout {
        size,
        menu,
        display,
        bits,
        keypad,
        side,
        radios,
        extras,
    }
}

/// Window inner size for the given theme and settings.
pub fn window_size(t: &Theme, s: &Settings, frame: FrameStyle) -> Vec2 {
    let body = compute_layout(t, s.mode, s.history, s.extras).size;
    match frame {
        FrameStyle::Native => body,
        FrameStyle::Aero => {
            body + vec2(
                2.0 * t.titlebar.border_width,
                t.titlebar.height + t.titlebar.border_width,
            )
        }
    }
}

pub fn load_theme(s: &Settings) -> (Theme, Option<String>) {
    match Theme::load(&s.theme_path()) {
        Ok(t) => (t, None),
        Err(e) => (Theme::builtin(), Some(e)),
    }
}

pub struct App {
    calc: Calculator,
    settings: Settings,
    saved: Settings,
    theme: Theme,
    theme_error: Option<String>,
    theme_mtime: Option<SystemTime>,
    last_check: f64,
    /// Frame style the window was created with (transparency can't change later).
    created_frame: FrameStyle,
    last_size: Option<Vec2>,
    extras: ExtrasState,
    about_open: bool,
    stat_selected: Option<usize>,
    flash: Option<(Cmd, f64)>,
    #[cfg(feature = "screenshot")]
    screenshot: Option<(String, u32)>,
}

impl App {
    pub fn new(
        ctx: &Context,
        settings: Settings,
        theme: Theme,
        theme_error: Option<String>,
    ) -> Self {
        let mut calc = Calculator::new();
        calc.set_mode(settings.mode);
        calc.angle = settings.angle;
        calc.base = settings.base;
        calc.set_word(settings.word);
        calc.grouping = settings.digit_grouping;
        let mut app = App {
            calc,
            saved: settings.clone(),
            created_frame: settings.frame,
            settings,
            theme,
            theme_error,
            theme_mtime: None,
            last_check: 0.0,
            last_size: None,
            extras: ExtrasState::default(),
            about_open: false,
            stat_selected: None,
            flash: None,
            #[cfg(feature = "screenshot")]
            screenshot: std::env::var("SEVENPLUS_SCREENSHOT").ok().map(|p| (p, 0)),
        };
        app.theme_mtime = mtime(&app.settings);
        app.apply_theme(ctx);
        app
    }

    fn apply_theme(&mut self, ctx: &Context) {
        let t = &self.theme;
        let mut fonts = FontDefinitions::default();
        let custom = t
            .font_path(&config::themes_dir())
            .and_then(|p| match std::fs::read(&p) {
                Ok(b) => Some(b),
                Err(e) => {
                    self.theme_error = Some(format!("font {}: {e}", p.display()));
                    None
                }
            });
        let data = match custom {
            Some(b) => FontData::from_owned(b),
            None => FontData::from_static(SELAWIK),
        };
        fonts.font_data.insert("ui".into(), Arc::new(data));
        for fam in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(fam)
                .or_default()
                .insert(0, "ui".into());
        }
        ctx.set_fonts(fonts);

        let bg = t.window.background.first().map_or(Color32::WHITE, |c| c.0);
        let dark = (bg.r() as u32 + bg.g() as u32 + bg.b() as u32) < 384;
        let c = &t.controls;
        ctx.all_styles_mut(|style| {
            let mut v = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            v.override_text_color = Some(c.text.0);
            v.window_fill = t.menu.popup_background.0;
            v.panel_fill = bg;
            v.window_stroke = Stroke::new(1.0, t.menu.popup_border.0);
            v.extreme_bg_color = c.background.0;
            v.text_edit_bg_color = Some(c.background.0);
            v.selection.bg_fill = t.display.selection.0;
            v.selection.stroke = Stroke::new(1.0, t.display.selection_text.0);
            for (w, fill, stroke) in [
                (&mut v.widgets.inactive, c.background.0, c.border.0),
                (&mut v.widgets.hovered, c.hover.0, c.accent.0),
                (&mut v.widgets.active, c.hover.0, c.accent.0),
                (&mut v.widgets.open, c.hover.0, c.accent.0),
            ] {
                w.bg_fill = fill;
                w.weak_bg_fill = fill;
                w.bg_stroke = Stroke::new(1.0, stroke);
                w.fg_stroke = Stroke::new(1.0, c.text.0);
            }
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.group_border.0);
            v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, c.text.0);
            style.visuals = v;
            for ts in [TextStyle::Body, TextStyle::Button, TextStyle::Monospace] {
                style
                    .text_styles
                    .insert(ts, FontId::proportional(t.font.size));
            }
            style
                .text_styles
                .insert(TextStyle::Small, FontId::proportional(t.font.size * 0.85));
            style
                .text_styles
                .insert(TextStyle::Heading, FontId::proportional(t.font.size * 1.4));
        });
        self.last_size = None;
    }

    /// Reloads the active theme when it changes on disk (polled once a second).
    fn poll_theme(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        if now - self.last_check < 1.0 {
            return;
        }
        self.last_check = now;
        let m = mtime(&self.settings);
        if m != self.theme_mtime {
            self.theme_mtime = m;
            self.reload_theme(ctx);
        }
    }

    fn reload_theme(&mut self, ctx: &Context) {
        match Theme::load(&self.settings.theme_path()) {
            Ok(t) => {
                self.theme = t;
                self.theme_error = None;
                self.apply_theme(ctx);
            }
            Err(e) => self.theme_error = Some(e),
        }
    }

    fn press(&mut self, cmd: Cmd, now: f64) {
        self.calc.press(cmd);
        self.flash = Some((cmd, now));
    }

    fn set_mode(&mut self, mode: Mode) {
        self.calc.set_mode(mode);
        self.stat_selected = None;
    }

    fn paste(&mut self, text: &str) {
        let text = text.trim();
        if self.calc.mode == Mode::Programmer {
            let digits: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            if let Ok(v) = u128::from_str_radix(&digits, self.calc.base.radix()) {
                self.calc.load_value(Num::from_i128(v as i128));
                return;
            }
        } else if let Some(n) = Num::parse(text) {
            self.calc.load_value(n);
            return;
        }
        for ch in text.chars() {
            let ch = match ch {
                '×' => '*',
                '÷' => '/',
                c => c,
            };
            if let Some(cmd) = cmd_for_char(ch, self.calc.mode, self.calc.inv) {
                self.calc.press(cmd);
            }
        }
    }

    fn copy_text(&self) -> String {
        let s = self.calc.display();
        if self.calc.error().is_some() {
            return s;
        }
        s.chars().filter(|c| *c != ',' && *c != ' ').collect()
    }

    fn handle_keys(&mut self, ctx: &Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (events, now, alt) = ctx.input(|i| (i.events.clone(), i.time, i.modifiers.alt));
        for ev in events {
            match ev {
                Event::Copy => ctx.copy_text(self.copy_text()),
                Event::Paste(s) => self.paste(&s),
                // Alt+1..4 switch modes; don't also type the digit.
                Event::Text(t) if !alt => {
                    for ch in t.chars() {
                        if let Some(cmd) = cmd_for_char(ch, self.calc.mode, self.calc.inv) {
                            self.press(cmd, now);
                        }
                    }
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => self.key(key, modifiers, now),
                _ => {}
            }
        }
    }

    fn key(&mut self, key: EKey, m: Modifiers, now: f64) {
        let mode = self.calc.mode;
        if m.alt {
            match key {
                EKey::Num1 => self.set_mode(Mode::Standard),
                EKey::Num2 => self.set_mode(Mode::Scientific),
                EKey::Num3 => self.set_mode(Mode::Programmer),
                EKey::Num4 => self.set_mode(Mode::Statistics),
                _ => {}
            }
            return;
        }
        if m.command {
            match key {
                EKey::H => self.settings.history = !self.settings.history,
                EKey::U => self.settings.extras = Extras::Units,
                EKey::E => self.settings.extras = Extras::Date,
                EKey::F4 => self.settings.extras = Extras::None,
                EKey::D if m.shift => self.calc.clear_history(),
                EKey::M => self.press(Cmd::MemStore, now),
                EKey::R => self.press(Cmd::MemRecall, now),
                EKey::L => self.press(Cmd::MemClear, now),
                EKey::P => self.press(Cmd::MemAdd, now),
                EKey::Q => self.press(Cmd::MemSub, now),
                _ => {}
            }
            return;
        }
        let cmd = match key {
            EKey::Enter => Some(Cmd::Equals),
            EKey::Escape => Some(Cmd::Clear),
            EKey::Delete => Some(Cmd::ClearEntry),
            EKey::Backspace => Some(Cmd::Back),
            EKey::F9 => Some(Cmd::Negate),
            EKey::Insert if mode == Mode::Statistics => Some(Cmd::StatAdd),
            _ => None,
        };
        if let Some(c) = cmd {
            self.press(c, now);
            return;
        }
        match (mode, key) {
            (Mode::Scientific, EKey::F3) => self.calc.angle = Angle::Degrees,
            (Mode::Scientific, EKey::F4) => self.calc.angle = Angle::Radians,
            (Mode::Scientific, EKey::F5) => self.calc.angle = Angle::Gradians,
            (Mode::Programmer, EKey::F5) => self.calc.set_base(Base::Hex),
            (Mode::Programmer, EKey::F6) => self.calc.set_base(Base::Dec),
            (Mode::Programmer, EKey::F7) => self.calc.set_base(Base::Oct),
            (Mode::Programmer, EKey::F8) => self.calc.set_base(Base::Bin),
            (Mode::Programmer, EKey::F12) => self.calc.set_word(WordSize::Qword),
            (Mode::Programmer, EKey::F2) => self.calc.set_word(WordSize::Dword),
            (Mode::Programmer, EKey::F3) => self.calc.set_word(WordSize::Word),
            (Mode::Programmer, EKey::F4) => self.calc.set_word(WordSize::Byte),
            _ => {}
        }
    }

    fn sync_settings(&mut self) {
        let s = &mut self.settings;
        s.mode = self.calc.mode;
        s.angle = self.calc.angle;
        s.base = self.calc.base;
        s.word = self.calc.word;
        self.calc.grouping = s.digit_grouping;
        if *s != self.saved {
            s.save();
            self.saved = s.clone();
        }
    }

    // ---- drawing ---------------------------------------------------------------------

    fn key_state(
        &self,
        ui: &egui::Ui,
        id: Id,
        rect: Rect,
        enabled: bool,
        cmd: Cmd,
    ) -> (egui::Response, &ButtonState) {
        let b = &self.theme.button;
        let resp = ui.interact(
            rect,
            id,
            if enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let now = ui.input(|i| i.time);
        let flashing = self
            .flash
            .is_some_and(|(c, t)| c == cmd && now - t < FLASH_SECS);
        let state = if !enabled {
            &b.disabled
        } else if resp.is_pointer_button_down_on() || flashing {
            &b.pressed
        } else if resp.hovered() {
            &b.hover
        } else {
            &b.normal
        };
        (resp, state)
    }

    fn draw_keys(&mut self, ui: &egui::Ui, origin: Pos2, keys: &[Key], block: &str) {
        let t = &self.theme.button;
        let (bw, bh, g) = (t.width, t.height, t.gap);
        let mut clicked = None;
        for k in keys {
            let rect = Rect::from_min_size(
                origin + vec2(k.col as f32 * (bw + g), k.row as f32 * (bh + g)),
                vec2(
                    k.w as f32 * bw + (k.w - 1) as f32 * g,
                    k.h as f32 * bh + (k.h - 1) as f32 * g,
                ),
            );
            let (label, cmd) = k.resolve(self.calc.inv);
            let enabled = layout::enabled(&self.calc, cmd);
            let (resp, state) =
                self.key_state(ui, Id::new((block, k.col, k.row)), rect, enabled, cmd);
            let over = match k.kind {
                Kind::Digit => &t.digit,
                Kind::Operator => &t.operator,
                Kind::Memory => &t.memory,
                Kind::Function => &t.function,
                Kind::Equals => &t.equals,
            };
            let normal = std::ptr::eq(state, &t.normal);
            let gradient = over
                .gradient
                .as_ref()
                .filter(|_| normal)
                .unwrap_or(&state.gradient);
            let border = over.border.filter(|_| normal).unwrap_or(state.border);
            let text = over.text.filter(|_| enabled).unwrap_or(state.text);
            let p = ui.painter();
            paint::button_body(p, rect, t.radius, gradient, border.0, state.highlight.0);
            // Inv is a toggle: draw it pressed while active.
            if cmd == Cmd::Inv && self.calc.inv {
                paint::button(p, rect, t.radius, &t.pressed);
            }
            paint::rich_label(p, rect.center(), label, t.font_size, text.0);
            if resp.clicked() {
                clicked = Some(cmd);
            }
        }
        if let Some(cmd) = clicked {
            let now = ui.input(|i| i.time);
            self.press(cmd, now);
        }
    }

    fn radio_colors(&self) -> RadioColors {
        let c = &self.theme.controls;
        RadioColors {
            text: c.text.0,
            background: c.background.0,
            border: c.border.0,
            accent: c.accent.0,
            hover: c.hover.0,
        }
    }

    /// A group box of radio buttons laid out in `cols` columns; returns the clicked index.
    fn radio_group(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        labels: &[&str],
        selected: usize,
        cols: usize,
        id: &str,
    ) -> Option<usize> {
        let p = ui.painter();
        paint::group_box(p, rect, self.theme.controls.group_border.0);
        let rows = labels.len().div_ceil(cols);
        let inner = rect.shrink2(vec2(6.0, 3.0));
        let rh = inner.height() / rows as f32;
        let size = self.theme.font.size;
        // Items in a row get their natural width; leftover space is spread between them.
        let widths: Vec<f32> = labels
            .iter()
            .map(|l| {
                p.layout_no_wrap(l.to_string(), FontId::proportional(size), Color32::BLACK)
                    .size()
                    .x
                    + size
                    + 5.0
            })
            .collect();
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
            let pos = pos2(cell.left(), cell.center().y - (size + 1.0) / 2.0);
            paint::radio(p, pos, label, size, i == selected, resp.hovered(), &colors);
            if resp.clicked() {
                clicked = Some(i);
            }
        }
        clicked
    }

    fn draw_radios(&mut self, ui: &egui::Ui, radios: &[Rect]) {
        match self.calc.mode {
            Mode::Scientific => {
                let angles = [Angle::Degrees, Angle::Radians, Angle::Gradians];
                let sel = angles
                    .iter()
                    .position(|a| *a == self.calc.angle)
                    .unwrap_or(0);
                if let Some(i) = self.radio_group(
                    ui,
                    radios[0],
                    &["Degrees", "Radians", "Grads"],
                    sel,
                    3,
                    "angle",
                ) {
                    self.calc.angle = angles[i];
                }
            }
            Mode::Programmer => {
                let bases = [Base::Hex, Base::Dec, Base::Oct, Base::Bin];
                let sel = bases.iter().position(|b| *b == self.calc.base).unwrap_or(1);
                if let Some(i) =
                    self.radio_group(ui, radios[0], &["Hex", "Dec", "Oct", "Bin"], sel, 1, "base")
                {
                    self.calc.set_base(bases[i]);
                }
                let words = [
                    WordSize::Qword,
                    WordSize::Dword,
                    WordSize::Word,
                    WordSize::Byte,
                ];
                let sel = words.iter().position(|w| *w == self.calc.word).unwrap_or(0);
                if let Some(i) = self.radio_group(
                    ui,
                    radios[1],
                    &["Qword", "Dword", "Word", "Byte"],
                    sel,
                    1,
                    "word",
                ) {
                    self.calc.set_word(words[i]);
                }
            }
            _ => {}
        }
    }

    fn draw_display(&mut self, ui: &egui::Ui, rect: Rect) {
        let d = &self.theme.display;
        let p = ui.painter().clone();
        paint::gradient_rect(&p, rect, d.radius, &d.background);
        p.rect_stroke(
            rect,
            paint::radius(d.radius),
            Stroke::new(1.0, d.border.0),
            StrokeKind::Inside,
        );
        let inner = rect.shrink(d.padding);
        let mut top = inner.top();

        let lh = line_h(&self.theme);
        let list_h = d.history_lines as f32 * lh;
        let list_rect = Rect::from_min_size(inner.min, vec2(inner.width(), list_h));
        match self.calc.mode {
            Mode::Standard | Mode::Scientific if self.settings.history => {
                let items: Vec<(String, Num)> = self
                    .calc
                    .history()
                    .iter()
                    .map(|h| {
                        (
                            format!("{} = {}", h.expr, self.calc.format_num(&h.result)),
                            h.result.clone(),
                        )
                    })
                    .collect();
                let start = items.len().saturating_sub(d.history_lines);
                let mut load = None;
                for (row, (expr, result)) in items[start..].iter().enumerate() {
                    let r = Rect::from_min_size(
                        pos2(inner.left(), top + row as f32 * lh),
                        vec2(inner.width(), lh),
                    );
                    let resp = ui.interact(r, Id::new(("hist", start + row)), Sense::click());
                    if resp.hovered() {
                        p.rect_filled(r, 2.0, d.history_hover.0);
                    }
                    p.text(
                        r.right_center() - vec2(2.0, 0.0),
                        Align2::RIGHT_CENTER,
                        expr,
                        FontId::proportional(d.history_size),
                        d.history_text.0,
                    );
                    if resp.clicked() {
                        load = Some(result.clone());
                    }
                }
                if let Some(n) = load {
                    self.calc.load_value(n);
                }
                top += list_h + d.padding;
                p.line_segment(
                    [
                        pos2(inner.left(), top - d.padding / 2.0),
                        pos2(inner.right(), top - d.padding / 2.0),
                    ],
                    Stroke::new(1.0, d.border.0.gamma_multiply(0.5)),
                );
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
                    let selected = self.stat_selected == Some(i);
                    let color = if selected {
                        p.rect_filled(r, 2.0, d.selection.0);
                        d.selection_text.0
                    } else {
                        if resp.hovered() {
                            p.rect_filled(r, 2.0, d.history_hover.0);
                        }
                        d.history_text.0
                    };
                    let text = self.calc.format_num(&self.calc.stats()[i]);
                    p.text(
                        r.right_center() - vec2(2.0, 0.0),
                        Align2::RIGHT_CENTER,
                        text,
                        FontId::proportional(d.history_size),
                        color,
                    );
                    if resp.clicked() {
                        self.stat_selected = Some(i);
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Delete").clicked() {
                            delete = Some(i);
                        }
                    });
                }
                if let Some(i) = delete {
                    self.calc.stat_delete(i);
                    self.stat_selected = None;
                }
                let count = Rect::from_min_size(
                    pos2(inner.left(), list_rect.bottom()),
                    vec2(inner.width(), lh),
                );
                p.text(
                    count.right_center() - vec2(2.0, 0.0),
                    Align2::RIGHT_CENTER,
                    format!("Count = {n}"),
                    FontId::proportional(d.history_size),
                    d.expression_text.0,
                );
                top = count.bottom() + d.padding;
                p.line_segment(
                    [
                        pos2(inner.left(), top - d.padding / 2.0),
                        pos2(inner.right(), top - d.padding / 2.0),
                    ],
                    Stroke::new(1.0, d.border.0.gamma_multiply(0.5)),
                );
            }
            _ => {}
        }

        // Expression line.
        let expr_h = d.expression_size + 4.0;
        let expr_rect =
            Rect::from_min_max(pos2(inner.left(), top), pos2(inner.right(), top + expr_h));
        let expr = self.calc.expression();
        let expr_font = FontId::proportional(d.expression_size);
        let g = p.layout_no_wrap(expr, expr_font, d.expression_text.0);
        let indicator_w = 26.0;
        // Long expressions are clipped on the left, like Win7.
        let clip = Rect::from_min_max(
            pos2(inner.left() + indicator_w, expr_rect.top()),
            expr_rect.max,
        );
        p.with_clip_rect(clip).galley(
            pos2(
                expr_rect.right() - g.size().x,
                expr_rect.center().y - g.size().y / 2.0,
            ),
            g,
            d.expression_text.0,
        );
        let depth = self.calc.paren_depth();
        if depth > 0 {
            p.text(
                expr_rect.left_center(),
                Align2::LEFT_CENTER,
                format!("(={depth}"),
                FontId::proportional(d.indicator_size),
                d.indicator_text.0,
            );
        }

        // Value line, shrunk to fit.
        let value_rect = Rect::from_min_max(pos2(inner.left(), expr_rect.bottom()), inner.max);
        if self.calc.has_memory() {
            p.text(
                value_rect.left_center(),
                Align2::LEFT_CENTER,
                "M",
                FontId::proportional(d.indicator_size),
                d.indicator_text.0,
            );
        }
        let text = self.calc.display();
        let avail = value_rect.width() - 16.0;
        let mut size = d.value_size;
        let w = p
            .layout_no_wrap(text.clone(), FontId::proportional(size), d.text.0)
            .size()
            .x;
        if w > avail {
            size = (size * avail / w).max(8.0);
        }
        p.text(
            value_rect.right_center() - vec2(1.0, 0.0),
            Align2::RIGHT_CENTER,
            text,
            FontId::proportional(size),
            d.text.0,
        );
    }

    fn draw_bits(&mut self, ui: &egui::Ui, rect: Rect) {
        let b = &self.theme.bits;
        let p = ui.painter().clone();
        let bits = self.calc.bits();
        let word = self.calc.word.bits();
        let cell = (rect.width() / 39.0).min(b.size * 0.9);
        let row_h = bits_row_h(&self.theme);
        let font = FontId::proportional(b.size);
        let small = FontId::proportional(b.size * 0.85);
        let mut toggle = None;
        for row in 0..2 {
            let y = rect.top() + row as f32 * row_h;
            let x0 = rect.center().x - 39.0 * cell / 2.0;
            for j in 0..32 {
                let bit = 63 - (row * 32 + j);
                let x = x0 + (j + j / 4) as f32 * cell;
                let r = Rect::from_min_size(pos2(x, y), vec2(cell, b.size + 4.0));
                let active = (bit as u32) < word;
                let on = (bits >> bit) & 1 == 1;
                let color = if active { b.text.0 } else { b.dim.0 };
                if active {
                    let resp = ui.interact(r, Id::new(("bit", bit)), Sense::click());
                    if resp.hovered() {
                        p.rect_filled(r, 2.0, self.theme.controls.hover.0);
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
                        b.index.0,
                    );
                }
            }
        }
        if let Some(bit) = toggle {
            self.calc.press(Cmd::ToggleBit(bit));
        }
    }

    fn draw_menu(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let m = self.theme.menu.clone();
        paint::gradient_rect(ui.painter(), rect, 0.0, &m.background);
        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.visuals_mut().override_text_color = Some(m.text.0);
            ui.visuals_mut().widgets.hovered.weak_bg_fill = m.hover.0;
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(1.0, m.hover_border.0);
            ui.visuals_mut().widgets.open.weak_bg_fill = m.hover.0;
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("View", |ui| self.view_menu(ui));
                ui.menu_button("Edit", |ui| self.edit_menu(ui));
                ui.menu_button("Help", |ui| {
                    if ui.button("View Help").clicked() {
                        ui.ctx()
                            .open_url(egui::OpenUrl::new_tab(env!("CARGO_PKG_REPOSITORY")));
                    }
                    ui.separator();
                    if ui.button("About Calculator").clicked() {
                        self.about_open = true;
                    }
                });
            });
        });
    }

    fn item(ui: &mut egui::Ui, selected: bool, label: &str, shortcut: &str) -> bool {
        ui.add(Button::selectable(selected, label).shortcut_text(shortcut))
            .clicked()
    }

    fn view_menu(&mut self, ui: &mut egui::Ui) {
        let modes = [
            (Mode::Standard, "Standard", "Alt+1"),
            (Mode::Scientific, "Scientific", "Alt+2"),
            (Mode::Programmer, "Programmer", "Alt+3"),
            (Mode::Statistics, "Statistics", "Alt+4"),
        ];
        for (mode, label, key) in modes {
            if Self::item(ui, self.calc.mode == mode, label, key) {
                self.set_mode(mode);
            }
        }
        ui.separator();
        let history_ok = matches!(self.calc.mode, Mode::Standard | Mode::Scientific);
        if ui
            .add_enabled(
                history_ok,
                Button::selectable(self.settings.history, "History").shortcut_text("Ctrl+H"),
            )
            .clicked()
        {
            self.settings.history = !self.settings.history;
        }
        if Self::item(ui, self.settings.digit_grouping, "Digit grouping", "") {
            self.settings.digit_grouping = !self.settings.digit_grouping;
        }
        ui.separator();
        let ex = self.settings.extras;
        if Self::item(ui, ex == Extras::None, "Basic", "Ctrl+F4") {
            self.settings.extras = Extras::None;
        }
        if Self::item(ui, ex == Extras::Units, "Unit conversion", "Ctrl+U") {
            self.settings.extras = Extras::Units;
        }
        if Self::item(ui, ex == Extras::Date, "Date calculation", "Ctrl+E") {
            self.settings.extras = Extras::Date;
        }
        ui.menu_button("Worksheets", |ui| {
            for (e, label) in [
                (Extras::Mortgage, "Mortgage"),
                (Extras::VehicleLease, "Vehicle lease"),
                (Extras::FuelMpg, "Fuel economy (mpg)"),
                (Extras::FuelL100km, "Fuel economy (L/100 km)"),
            ] {
                if Self::item(ui, ex == e, label, "") {
                    self.settings.extras = e;
                }
            }
        });
        ui.separator();
        ui.menu_button("Theme", |ui| {
            ui.label(egui::RichText::new(format!("Active: {}", self.theme.name)).small());
            for name in config::list_themes() {
                if Self::item(ui, self.settings.theme == name, &name, "") {
                    self.settings.theme = name;
                    self.theme_mtime = mtime(&self.settings);
                    let ctx = ui.ctx().clone();
                    self.reload_theme(&ctx);
                }
            }
            ui.separator();
            if ui.button("Open themes folder").clicked() {
                open_folder(&config::themes_dir());
            }
        });
        ui.menu_button("Window frame", |ui| {
            for (f, label) in [
                (FrameStyle::Native, "Native"),
                (FrameStyle::Aero, "Aero glass"),
            ] {
                if Self::item(ui, self.settings.frame == f, label, "") {
                    self.settings.frame = f;
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::Decorations(f == FrameStyle::Native));
                    self.last_size = None;
                }
            }
            if self.settings.frame != self.created_frame {
                ui.label(egui::RichText::new("Restart for rounded corners").small());
            }
        });
    }

    fn edit_menu(&mut self, ui: &mut egui::Ui) {
        if Self::item(ui, false, "Copy", "Ctrl+C") {
            ui.ctx().copy_text(self.copy_text());
        }
        if Self::item(ui, false, "Paste", "Ctrl+V") {
            // Clipboard contents arrive as a paste event on the next frame.
            ui.ctx().send_viewport_cmd(ViewportCommand::RequestPaste);
        }
        ui.separator();
        ui.menu_button("History", |ui| {
            if Self::item(ui, false, "Copy history", "") {
                let text: Vec<String> = self
                    .calc
                    .history()
                    .iter()
                    .map(|h| format!("{} = {}", h.expr, self.calc.format_num(&h.result)))
                    .collect();
                ui.ctx().copy_text(text.join("\n"));
            }
            if Self::item(ui, false, "Clear", "Ctrl+Shift+D") {
                self.calc.clear_history();
            }
        });
    }

    fn draw_aero_frame(&mut self, ui: &egui::Ui, win: Rect) -> Rect {
        let t = self.theme.titlebar.clone();
        let p = ui.painter().clone();
        let bw = t.border_width;
        paint::gradient_rect(&p, win, t.radius, &t.background);
        p.rect_stroke(
            win.shrink(1.0),
            paint::radius(t.radius - 1.0),
            Stroke::new(1.0, t.inner_border.0),
            StrokeKind::Inside,
        );
        p.rect_stroke(
            win,
            paint::radius(t.radius),
            Stroke::new(1.0, t.border.0),
            StrokeKind::Inside,
        );
        let client = Rect::from_min_max(
            pos2(win.left() + bw, win.top() + t.height),
            pos2(win.right() - bw, win.bottom() - bw),
        );

        // Caption buttons: minimize, maximize (disabled, like Win7 calc), close.
        let bh = t.button_height;
        let close = Rect::from_min_size(
            pos2(win.right() - bw + 2.0 - t.close_width, win.top() + 1.0),
            vec2(t.close_width, bh),
        );
        let max = Rect::from_min_size(
            pos2(close.left() - t.max_width, close.top()),
            vec2(t.max_width, bh),
        );
        let min = Rect::from_min_size(
            pos2(max.left() - t.min_width, close.top()),
            vec2(t.min_width, bh),
        );
        let r_min = ui.interact(min, Id::new("cap_min"), Sense::click());
        let r_close = ui.interact(close, Id::new("cap_close"), Sense::click());
        for (rect, resp, normal, hover) in [
            (min, Some(&r_min), &t.button, &t.button_hover),
            (max, None, &t.button, &t.button),
            (close, Some(&r_close), &t.close, &t.close_hover),
        ] {
            let hovered = resp.is_some_and(|r| r.hovered());
            paint::button_body(
                &p,
                rect,
                3.0,
                if hovered { hover } else { normal },
                t.button_border.0,
                t.inner_border.0,
            );
        }
        let glyph = |rect: Rect, kind: u8, color: Color32| {
            let c = rect.center();
            let outline = Stroke::new(3.5, t.glyph_outline.0);
            let stroke = Stroke::new(1.6, color);
            let lines: Vec<[Pos2; 2]> = match kind {
                0 => vec![[c + vec2(-4.0, 3.0), c + vec2(4.0, 3.0)]],
                1 => vec![
                    [c + vec2(-5.0, -3.5), c + vec2(5.0, -3.5)],
                    [c + vec2(5.0, -3.5), c + vec2(5.0, 3.5)],
                    [c + vec2(5.0, 3.5), c + vec2(-5.0, 3.5)],
                    [c + vec2(-5.0, 3.5), c + vec2(-5.0, -3.5)],
                ],
                _ => vec![
                    [c + vec2(-4.0, -4.0), c + vec2(4.0, 4.0)],
                    [c + vec2(4.0, -4.0), c + vec2(-4.0, 4.0)],
                ],
            };
            if color != t.glyph_disabled.0 {
                for l in &lines {
                    p.line_segment(*l, outline);
                }
            }
            for l in &lines {
                p.line_segment(*l, stroke);
            }
        };
        glyph(min, 0, t.glyph.0);
        glyph(max, 1, t.glyph_disabled.0);
        glyph(close, 2, t.glyph.0);
        if r_min.clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
        }
        if r_close.clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }

        // Icon + caption; the rest of the title bar drags the window.
        let icon = Rect::from_min_size(
            pos2(win.left() + bw + 1.0, win.top() + (t.height - 16.0) / 2.0),
            vec2(16.0, 16.0),
        );
        draw_icon(&p, icon);
        paint::glow_text(
            &p,
            pos2(icon.right() + 6.0, icon.center().y),
            Align2::LEFT_CENTER,
            "Calculator",
            FontId::proportional(t.text_size),
            t.text.0,
            t.text_glow.0,
        );
        let drag = Rect::from_min_max(win.min, pos2(min.left(), win.top() + t.height));
        if ui
            .interact(drag, Id::new("caption"), Sense::click_and_drag())
            .drag_started()
        {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        p.rect_stroke(
            client.expand(1.0),
            0.0,
            Stroke::new(1.0, t.border.0.gamma_multiply(0.6)),
            StrokeKind::Inside,
        );
        client
    }

    fn draw_theme_error(&mut self, ui: &egui::Ui, win: Rect) {
        let Some(err) = self.theme_error.clone() else {
            return;
        };
        let p = ui.painter();
        let galley = p.layout(
            format!("Theme error (click to dismiss):\n{err}"),
            FontId::proportional(11.0),
            Color32::WHITE,
            win.width() - 16.0,
        );
        let r = Rect::from_min_size(
            pos2(win.left() + 4.0, win.bottom() - galley.size().y - 12.0),
            vec2(win.width() - 8.0, galley.size().y + 8.0),
        );
        p.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(150, 20, 20, 235));
        p.galley(r.min + vec2(4.0, 4.0), galley, Color32::WHITE);
        if ui
            .interact(r, Id::new("theme_err"), Sense::click())
            .clicked()
        {
            self.theme_error = None;
        }
    }

    fn about(&mut self, ctx: &Context) {
        egui::Window::new("About Calculator")
            .open(&mut self.about_open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading("SevenPlus Calculator");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label("A Windows 7 calculator clone written in Rust.");
                ui.label("Font: Selawik, SIL Open Font License 1.1.");
                ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));
            });
    }
}

fn mtime(s: &Settings) -> Option<SystemTime> {
    std::fs::metadata(s.theme_path())
        .and_then(|m| m.modified())
        .ok()
}

fn open_folder(path: &std::path::Path) {
    let cmd = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(cmd).arg(path).spawn() {
        eprintln!("cannot open {}: {e}", path.display());
    }
}

/// A tiny calculator icon for the Aero caption.
fn draw_icon(p: &Painter, r: Rect) {
    let body = [
        crate::theme::Color(Color32::from_rgb(120, 140, 170)),
        crate::theme::Color(Color32::from_rgb(70, 88, 118)),
    ];
    paint::gradient_rect(p, r, 2.0, &body);
    let screen = Rect::from_min_size(r.min + vec2(3.0, 2.5), vec2(r.width() - 6.0, 4.0));
    p.rect_filled(screen, 0.5, Color32::from_rgb(220, 236, 250));
    for i in 0..3 {
        for j in 0..3 {
            let k = Rect::from_min_size(
                r.min + vec2(3.0 + i as f32 * 3.6, 8.0 + j as f32 * 2.6),
                vec2(2.6, 1.8),
            );
            p.rect_filled(k, 0.0, Color32::from_rgb(235, 240, 248));
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        if self.created_frame == FrameStyle::Aero {
            [0.0; 4]
        } else {
            let c = self
                .theme
                .window
                .background
                .first()
                .map_or(Color32::WHITE, |c| c.0);
            egui::Rgba::from(c).to_array()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_theme(&ctx);
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        self.handle_keys(&ctx);
        let history =
            self.settings.history && matches!(self.calc.mode, Mode::Standard | Mode::Scientific);
        let l = compute_layout(&self.theme, self.calc.mode, history, self.settings.extras);
        let frame = self.settings.frame;
        let want = window_size(
            &self.theme,
            &Settings {
                mode: self.calc.mode,
                history,
                ..self.settings.clone()
            },
            frame,
        );
        if self.last_size != Some(want) {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(want));
            self.last_size = Some(want);
        }

        let win = ui.max_rect();
        let client = match frame {
            FrameStyle::Aero => self.draw_aero_frame(ui, win),
            FrameStyle::Native => win,
        };
        let off = client.min.to_vec2();
        paint::gradient_rect(ui.painter(), client, 0.0, &self.theme.window.background);

        self.draw_display(ui, l.display.translate(off));
        if let Some(b) = l.bits {
            self.draw_bits(ui, b.translate(off));
        }
        let radios: Vec<Rect> = l.radios.iter().map(|r| r.translate(off)).collect();
        self.draw_radios(ui, &radios);
        match self.calc.mode {
            Mode::Statistics => self.draw_keys(ui, l.keypad + off, layout::STATISTICS, "stat"),
            mode => {
                if let Some(side) = l.side {
                    let keys = if mode == Mode::Scientific {
                        layout::SCIENTIFIC
                    } else {
                        layout::PROGRAMMER
                    };
                    self.draw_keys(ui, side + off, keys, "side");
                }
                self.draw_keys(ui, l.keypad + off, layout::STANDARD, "std");
            }
        }
        if let Some(ex) = l.extras {
            let r = ex.translate(off);
            let c = &self.theme.controls;
            ui.painter().line_segment(
                [
                    r.left_top() - vec2(self.theme.window.padding / 2.0, 0.0),
                    r.left_bottom() - vec2(self.theme.window.padding / 2.0, 0.0),
                ],
                Stroke::new(1.0, c.group_border.0),
            );
            let which = self.settings.extras;
            let grouping = self.settings.digit_grouping;
            ui.scope_builder(UiBuilder::new().max_rect(r.shrink2(vec2(4.0, 0.0))), |ui| {
                extras::show(ui, which, &mut self.extras, grouping);
            });
        }
        self.draw_menu(ui, l.menu.translate(off));
        self.about(&ctx);
        self.draw_theme_error(ui, win);
        if self
            .flash
            .is_some_and(|(_, t)| ctx.input(|i| i.time) - t < FLASH_SECS)
        {
            ctx.request_repaint();
        }
        self.sync_settings();
        #[cfg(feature = "screenshot")]
        self.screenshot(&ctx);
    }
}

#[cfg(feature = "screenshot")]
impl App {
    /// Dev aid: saves a PNG of the window after a few frames, then exits.
    fn screenshot(&mut self, ctx: &Context) {
        let Some((_, frames)) = &mut self.screenshot else {
            return;
        };
        *frames += 1;
        if *frames == 3 {
            // Optional scripted input: SEVENPLUS_SCREENSHOT_MODE=programmer, SEVENPLUS_SCREENSHOT_INPUT="2+3=".
            if let Some(m) = std::env::var("SEVENPLUS_SCREENSHOT_MODE")
                .ok()
                .and_then(|m| toml::Value::String(m).try_into().ok())
            {
                self.set_mode(m);
            }
            for ch in std::env::var("SEVENPLUS_SCREENSHOT_INPUT")
                .unwrap_or_default()
                .chars()
            {
                if let Some(cmd) = cmd_for_char(ch, self.calc.mode, self.calc.inv) {
                    self.calc.press(cmd);
                }
            }
        }
        let Some((path, frames)) = &mut self.screenshot else {
            return;
        };
        if *frames == 20 {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let [w, h] = img.size;
            let bytes: Vec<u8> = img
                .pixels
                .iter()
                .flat_map(|c| c.to_srgba_unmultiplied())
                .collect();
            if let Err(e) =
                image::save_buffer(&*path, &bytes, w as u32, h as u32, image::ColorType::Rgba8)
            {
                eprintln!("screenshot: {e}");
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}
