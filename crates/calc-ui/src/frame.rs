//! The optional hand-drawn Aero glass window frame.

use crate::app::App;
use crate::i18n::tr;
use crate::paint::{self, c};
use calc_theme::{Color, TitlebarCfg};
use eframe::egui::{
    self, Align2, Color32, Id, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, ViewportCommand,
    pos2, vec2, viewport::ResizeDirection,
};

impl App {
    /// Draws the frame and returns the client area inside it.
    pub(crate) fn draw_aero_frame(&mut self, ui: &egui::Ui, win: Rect) -> Rect {
        let t = self.theme.titlebar.clone();
        let p = ui.painter().clone();
        let bw = t.border_width;
        paint::gradient_rect(&p, win, t.radius, t.background.stops());
        p.rect_stroke(
            win.shrink(1.0),
            paint::radius(t.radius - 1.0),
            Stroke::new(1.0, c(t.inner_border)),
            StrokeKind::Inside,
        );
        p.rect_stroke(
            win,
            paint::radius(t.radius),
            Stroke::new(1.0, c(t.border)),
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
            let hovered = resp.is_some_and(|r| r.hovered() || r.has_focus());
            let g = if hovered { hover } else { normal };
            paint::button_body(
                &p,
                rect,
                3.0,
                g.stops(),
                c(t.button_border),
                c(t.inner_border),
            );
        }
        let glyph = |rect: Rect, kind: u8, color: Color32| {
            let ctr = rect.center();
            let outline = Stroke::new(3.5, c(t.glyph_outline));
            let stroke = Stroke::new(1.6, color);
            let lines: Vec<[Pos2; 2]> = match kind {
                0 => vec![[ctr + vec2(-4.0, 3.0), ctr + vec2(4.0, 3.0)]],
                1 => vec![
                    [ctr + vec2(-5.0, -3.5), ctr + vec2(5.0, -3.5)],
                    [ctr + vec2(5.0, -3.5), ctr + vec2(5.0, 3.5)],
                    [ctr + vec2(5.0, 3.5), ctr + vec2(-5.0, 3.5)],
                    [ctr + vec2(-5.0, 3.5), ctr + vec2(-5.0, -3.5)],
                ],
                _ => vec![
                    [ctr + vec2(-4.0, -4.0), ctr + vec2(4.0, 4.0)],
                    [ctr + vec2(4.0, -4.0), ctr + vec2(-4.0, 4.0)],
                ],
            };
            if color != c(t.glyph_disabled) {
                for l in &lines {
                    p.line_segment(*l, outline);
                }
            }
            for l in &lines {
                p.line_segment(*l, stroke);
            }
        };
        glyph(min, 0, c(t.glyph));
        glyph(max, 1, c(t.glyph_disabled));
        glyph(close, 2, c(t.glyph));
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
            &tr("app-title"),
            paint::font(self.theme.font.caption),
            c(t.text),
            c(t.text_glow),
        );
        let drag = Rect::from_min_max(win.min, pos2(min.left(), win.top() + t.height));
        if ui
            .interact(drag, Id::new("caption"), Sense::DRAG)
            .drag_started()
        {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        resize_handles(ui, win, &t);
        p.rect_stroke(
            client.expand(1.0),
            0.0,
            Stroke::new(1.0, c(t.border).gamma_multiply(0.6)),
            StrokeKind::Inside,
        );
        client
    }
}

/// Borderless windows have no OS resize border: right/bottom edges and a corner grip.
fn resize_handles(ui: &egui::Ui, win: Rect, t: &TitlebarCfg) {
    let (bw, grip) = (t.border_width.max(4.0), 16.0);
    let corner = Rect::from_min_max(win.max - vec2(grip, grip), win.max);
    let handles = [
        (
            corner,
            ResizeDirection::SouthEast,
            egui::CursorIcon::ResizeSouthEast,
        ),
        (
            Rect::from_min_max(
                pos2(win.right() - bw, win.top() + t.height),
                pos2(win.right(), corner.top()),
            ),
            ResizeDirection::East,
            egui::CursorIcon::ResizeEast,
        ),
        (
            Rect::from_min_max(
                pos2(win.left(), win.bottom() - bw),
                pos2(corner.left(), win.bottom()),
            ),
            ResizeDirection::South,
            egui::CursorIcon::ResizeSouth,
        ),
    ];
    for (i, (rect, dir, cursor)) in handles.into_iter().enumerate() {
        let resp = ui
            .interact(rect, Id::new(("resize", i)), Sense::DRAG)
            .on_hover_cursor(cursor);
        if resp.drag_started() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::BeginResize(dir));
        }
    }
    // Grip: three diagonal rows of dots, like Win7 status-bar grips.
    let p = ui.painter();
    for row in 0..3 {
        for j in 0..=row {
            let ctr = win.max - vec2(4.0 + 3.0 * j as f32, 4.0 + 3.0 * (row - j) as f32);
            p.circle_filled(ctr, 0.9, c(t.glyph_outline));
        }
    }
}

/// A tiny vector calculator icon for the caption (original artwork, DPI-005).
fn draw_icon(p: &Painter, r: Rect) {
    let body = [Color::rgb(120, 140, 170), Color::rgb(70, 88, 118)];
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
