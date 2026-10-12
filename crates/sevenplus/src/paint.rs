//! Drawing helpers: gradients, Win7 buttons, rich labels, radio buttons.

use crate::theme::{ButtonState, Color};
use eframe::egui::{
    Align2, Color32, CornerRadius, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind,
    pos2, vec2,
};

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

fn gradient_at(stops: &[Color], t: f32) -> Color32 {
    match stops {
        [] => Color32::TRANSPARENT,
        [c] => c.0,
        _ => {
            let x = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
            let i = (x.floor() as usize).min(stops.len() - 2);
            lerp_color(stops[i].0, stops[i + 1].0, x - i as f32)
        }
    }
}

/// Fills a rounded rectangle with a vertical multi-stop gradient.
pub fn gradient_rect(p: &Painter, rect: Rect, radius: f32, stops: &[Color]) {
    if stops.is_empty() || rect.height() <= 0.0 {
        return;
    }
    let r = radius
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);
    let (top, bottom, h) = (rect.top(), rect.bottom(), rect.height());
    let mut ys = vec![top, bottom];
    const STEPS: usize = 6;
    if r > 0.0 {
        for i in 1..=STEPS {
            let a = (i as f32 / STEPS as f32) * std::f32::consts::FRAC_PI_2;
            ys.push(top + r - r * a.cos());
            ys.push(bottom - r + r * a.cos());
        }
    }
    for i in 1..stops.len().saturating_sub(1) {
        ys.push(top + h * i as f32 / (stops.len() - 1) as f32);
    }
    ys.sort_by(f32::total_cmp);
    ys.dedup_by(|a, b| (*a - *b).abs() < 0.01);

    let inset = |y: f32| {
        let dy = if y < top + r {
            top + r - y
        } else if y > bottom - r {
            y - (bottom - r)
        } else {
            return 0.0;
        };
        r - (r * r - dy * dy).max(0.0).sqrt()
    };
    let mut mesh = Mesh::default();
    for (i, &y) in ys.iter().enumerate() {
        let c = gradient_at(stops, (y - top) / h);
        let d = inset(y);
        mesh.colored_vertex(pos2(rect.left() + d, y), c);
        mesh.colored_vertex(pos2(rect.right() - d, y), c);
        if i > 0 {
            let k = (i * 2) as u32;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k + 1, k);
        }
    }
    p.add(Shape::mesh(mesh));
}

pub fn radius(r: f32) -> CornerRadius {
    CornerRadius::same(r.clamp(0.0, 255.0) as u8)
}

/// Win7 glossy button body: gradient, inner highlight and outer border.
pub fn button_body(
    p: &Painter,
    rect: Rect,
    r: f32,
    gradient: &[Color],
    border: Color32,
    highlight: Color32,
) {
    gradient_rect(p, rect, r, gradient);
    p.rect_stroke(
        rect.shrink(1.0),
        radius(r - 1.0),
        Stroke::new(1.0, highlight),
        StrokeKind::Inside,
    );
    p.rect_stroke(
        rect,
        radius(r),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
}

pub fn button(p: &Painter, rect: Rect, r: f32, s: &ButtonState) {
    button_body(p, rect, r, &s.gradient, s.border.0, s.highlight.0);
}

enum Seg<'a> {
    Normal(&'a str),
    Super(&'a str),
    Over(&'a str),
}

/// Splits label markup: `^{..}` is superscript, `~{..}` is overlined.
fn segments(text: &str) -> Vec<Seg<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let next = rest.find("^{").into_iter().chain(rest.find("~{")).min();
        match next {
            Some(i) => {
                if i > 0 {
                    out.push(Seg::Normal(&rest[..i]));
                }
                let close = rest[i..].find('}').map_or(rest.len(), |j| i + j);
                let inner = &rest[i + 2..close];
                out.push(if rest[i..].starts_with('^') {
                    Seg::Super(inner)
                } else {
                    Seg::Over(inner)
                });
                rest = rest.get(close + 1..).unwrap_or("");
            }
            None => {
                out.push(Seg::Normal(rest));
                break;
            }
        }
    }
    out
}

/// Draws a label with markup centered at `center`.
pub fn rich_label(p: &Painter, center: Pos2, text: &str, size: f32, color: Color32) {
    if text == "←" {
        // Few fonts have the arrow glyph, so draw it.
        let (w, h) = (size * 0.45, size * 0.28);
        let s = Stroke::new(1.2, color);
        p.line_segment([center - vec2(w, 0.0), center + vec2(w, 0.0)], s);
        p.line_segment([center - vec2(w, 0.0), center + vec2(h - w, -h)], s);
        p.line_segment([center - vec2(w, 0.0), center + vec2(h - w, h)], s);
        return;
    }
    let segs = segments(text);
    let galleys: Vec<_> = segs
        .iter()
        .map(|s| {
            let (t, sz) = match s {
                Seg::Super(t) => (*t, size * 0.68),
                Seg::Normal(t) | Seg::Over(t) => (*t, size),
            };
            p.layout_no_wrap(t.to_string(), FontId::proportional(sz), color)
        })
        .collect();
    let total: f32 = galleys.iter().map(|g| g.size().x).sum();
    let mut x = center.x - total / 2.0;
    let base_h = p
        .layout_no_wrap("0".into(), FontId::proportional(size), color)
        .size()
        .y;
    let top = center.y - base_h / 2.0;
    for (s, g) in segs.iter().zip(galleys) {
        let y = match s {
            Seg::Super(_) => top - size * 0.12,
            _ => top,
        };
        let w = g.size().x;
        if let Seg::Over(_) = s {
            let ly = top + base_h * 0.28;
            p.line_segment(
                [pos2(x + 1.0, ly), pos2(x + w - 0.5, ly)],
                Stroke::new(1.0, color),
            );
        }
        p.galley(pos2(x, y), g, color);
        x += w;
    }
}

/// Text with a soft glow behind it (Aero caption).
pub fn glow_text(
    p: &Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    font: FontId,
    color: Color32,
    glow: Color32,
) {
    for (dx, dy) in [
        (-1.0, 0.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (-1.0, -1.0),
        (1.0, 1.0),
    ] {
        p.text(pos + vec2(dx, dy), align, text, font.clone(), glow);
    }
    p.text(pos, align, text, font, color);
}

pub struct RadioColors {
    pub text: Color32,
    pub background: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub hover: Color32,
}

/// Win7-style radio button; returns the full rect used (circle + label).
pub fn radio(
    p: &Painter,
    pos: Pos2,
    label: &str,
    size: f32,
    selected: bool,
    hovered: bool,
    c: &RadioColors,
) -> Rect {
    let d = (size + 1.0).round();
    let circle = Rect::from_min_size(pos, vec2(d, d));
    let center = circle.center();
    p.circle_filled(
        center,
        d / 2.0,
        if hovered { c.hover } else { c.background },
    );
    p.circle_stroke(center, d / 2.0 - 0.5, Stroke::new(1.0, c.border));
    if selected {
        p.circle_filled(center, d / 4.0, c.accent);
    }
    let g = p.layout_no_wrap(label.to_string(), FontId::proportional(size), c.text);
    let text_pos = pos2(circle.right() + 4.0, center.y - g.size().y / 2.0);
    let r = Rect::from_min_size(text_pos, g.size()).union(circle);
    p.galley(text_pos, g, c.text);
    r
}

/// Group box border (no caption, like the Win7 calculator's radio groups).
pub fn group_box(p: &Painter, rect: Rect, color: Color32) {
    p.rect_stroke(
        rect,
        radius(3.0),
        Stroke::new(1.0, color),
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_markup() {
        let s = segments("^{y}√x");
        assert!(matches!(s[..], [Seg::Super("y"), Seg::Normal("√x")]));
        let s = segments("~{x}^{2}");
        assert!(matches!(s[..], [Seg::Over("x"), Seg::Super("2")]));
        assert!(matches!(segments("sin")[..], [Seg::Normal("sin")]));
    }
}
