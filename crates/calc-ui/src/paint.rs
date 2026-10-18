//! Drawing helpers: gradients, Win7 buttons, rich labels, radio buttons.

use calc_theme::{ButtonState, Color, Gradient, Weight};
use eframe::egui::{
    Color32, CornerRadius, FontFamily, FontId, Galley, Mesh, Painter, Pos2, Rect, Shape, Stroke,
    StrokeKind,
    epaint::{RectShape, Tessellator},
    pos2,
    text::{LayoutJob, TextFormat},
    vec2,
};
use std::sync::Arc;

/// Font family name of the bold face.
pub const BOLD: &str = "bold";

/// A theme color as an egui color.
pub fn c(c: Color) -> Color32 {
    let [r, g, b, a] = c.0;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// The font for a theme text role.
pub fn font(role: calc_theme::FontRole) -> FontId {
    font_sized(role, role.size)
}

pub fn font_sized(role: calc_theme::FontRole, size: f32) -> FontId {
    match role.weight {
        Weight::Regular => FontId::proportional(size),
        Weight::Bold => FontId::new(size, FontFamily::Name(BOLD.into())),
    }
}

/// Snaps a rectangle to whole physical pixels so 1 px borders stay sharp (DPI-003).
pub fn snap(rect: Rect, pixels_per_point: f32) -> Rect {
    let r = |v: f32| (v * pixels_per_point).round() / pixels_per_point;
    Rect::from_min_max(
        pos2(r(rect.min.x), r(rect.min.y)),
        pos2(r(rect.max.x), r(rect.max.y)),
    )
}

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
        [one] => c(*one),
        _ => {
            let x = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
            let i = (x.floor() as usize).min(stops.len() - 2);
            lerp_color(c(stops[i]), c(stops[i + 1]), x - i as f32)
        }
    }
}

/// Fills a rounded rectangle with a vertical multi-stop gradient.
pub fn gradient_rect(p: &Painter, rect: Rect, radius: f32, stops: &[Color]) {
    if stops.is_empty() || rect.height() <= 0.0 {
        return;
    }
    let r = self::radius(
        radius
            .min(rect.width() / 2.0)
            .min(rect.height() / 2.0)
            .max(0.0),
    );
    if let [solid] = stops {
        p.rect_filled(rect, r, c(*solid));
        return;
    }
    // Let egui tessellate a white rounded rect, so the edges are anti-aliased exactly
    // like the border strokes, then tint each vertex by its height. The white's alpha
    // is the edge coverage and scales the gradient color.
    let options = p.ctx().tessellation_options(|o| *o);
    let mut tess = Tessellator::new(p.pixels_per_point(), options, [1, 1], Vec::new());
    let mut mesh = Mesh::default();
    tess.tessellate_rect(&RectShape::filled(rect, r, Color32::WHITE), &mut mesh);
    for v in &mut mesh.vertices {
        let coverage = v.color.a() as f32 / 255.0;
        let t = (v.pos.y - rect.top()) / rect.height();
        v.color = gradient_at(stops, t).gamma_multiply(coverage);
    }
    p.add(Shape::mesh(mesh));
}

/// A theme color between `a` and `b`, mixed with premultiplied alpha.
fn mix(a: Color32, b: Color32, t: f32) -> Color {
    Color(lerp_color(a, b, t).to_srgba_unmultiplied())
}

/// The button style `t` of the way from `a` to `b`, for smooth state changes.
/// Gradients with different stop counts are resampled to the longer one.
pub fn blend(a: &ButtonState, b: &ButtonState, t: f32) -> ButtonState {
    if t <= 0.0 {
        return a.clone();
    }
    if t >= 1.0 {
        return b.clone();
    }
    let n = a.fill.stops().len().max(b.fill.stops().len());
    let at = |i: usize| i as f32 / (n.max(2) - 1) as f32;
    let fill = (0..n)
        .map(|i| {
            let (x, y) = (
                gradient_at(a.fill.stops(), at(i)),
                gradient_at(b.fill.stops(), at(i)),
            );
            mix(x, y, t)
        })
        .collect();
    ButtonState {
        fill: Gradient(fill),
        border: mix(c(a.border), c(b.border), t),
        text: mix(c(a.text), c(b.text), t),
        highlight: mix(c(a.highlight), c(b.highlight), t),
        radius: if t < 0.5 { a.radius } else { b.radius },
    }
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

/// Letter spacing for display-size text: large digits read too far apart, so they
/// get tightened; smaller text keeps the font's own spacing.
fn tracking(size: f32) -> f32 {
    if size >= 20.0 { -0.02 * size } else { 0.0 }
}

/// Lays out one line of text with size-aware tracking.
pub fn tracked(p: &Painter, text: &str, font: FontId, color: Color32) -> Arc<Galley> {
    let format = TextFormat {
        extra_letter_spacing: tracking(font.size),
        font_id: font,
        color,
        ..Default::default()
    };
    p.layout_job(LayoutJob::single_section(text.to_string(), format))
}

enum Seg<'a> {
    Normal(&'a str),
    Super(&'a str),
    Sub(&'a str),
    Over(&'a str),
}

/// Splits label markup: `^{..}` is superscript, `_{..}` subscript, `~{..}` overlined.
fn segments(text: &str) -> Vec<Seg<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let next = ["^{", "_{", "~{"].iter().filter_map(|m| rest.find(m)).min();
        match next {
            Some(i) => {
                if i > 0 {
                    out.push(Seg::Normal(&rest[..i]));
                }
                let close = rest[i..].find('}').map_or(rest.len(), |j| i + j);
                let inner = &rest[i + 2..close];
                out.push(match rest.as_bytes()[i] {
                    b'^' => Seg::Super(inner),
                    b'_' => Seg::Sub(inner),
                    _ => Seg::Over(inner),
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
pub fn rich_label(p: &Painter, center: Pos2, text: &str, font: FontId, color: Color32) {
    let size = font.size;
    if text == "←" {
        // Few fonts have the arrow glyph, so draw it.
        let (w, h) = (size * 0.45, size * 0.28);
        let s = Stroke::new(1.2, color);
        p.line_segment([center - vec2(w, 0.0), center + vec2(w, 0.0)], s);
        p.line_segment([center - vec2(w, 0.0), center + vec2(h - w, -h)], s);
        p.line_segment([center - vec2(w, 0.0), center + vec2(h - w, h)], s);
        return;
    }
    if text == "⌫" {
        // Delete-left: a tag pointing left with an × inside.
        let (w, h) = (size * 0.55, size * 0.34);
        let s = Stroke::new(1.3, color);
        let (l, r, t, b) = (center.x - w, center.x + w, center.y - h, center.y + h);
        let tip = l + h;
        let outline = vec![
            pos2(l, center.y),
            pos2(tip, t),
            pos2(r, t),
            pos2(r, b),
            pos2(tip, b),
        ];
        p.add(Shape::closed_line(outline, s));
        let (cx, x) = ((tip + r) / 2.0, h * 0.45);
        p.line_segment([pos2(cx - x, center.y - x), pos2(cx + x, center.y + x)], s);
        p.line_segment([pos2(cx - x, center.y + x), pos2(cx + x, center.y - x)], s);
        return;
    }
    let segs = segments(text);
    let galleys: Vec<_> = segs
        .iter()
        .map(|s| {
            let (t, sz) = match s {
                Seg::Super(t) | Seg::Sub(t) => (*t, size * 0.68),
                Seg::Normal(t) | Seg::Over(t) => (*t, size),
            };
            p.layout_no_wrap(t.to_string(), FontId::new(sz, font.family.clone()), color)
        })
        .collect();
    let total: f32 = galleys.iter().map(|g| g.size().x).sum();
    let mut x = center.x - total / 2.0;
    let base_h = p.layout_no_wrap("0".into(), font.clone(), color).size().y;
    let top = center.y - base_h / 2.0;
    for (s, g) in segs.iter().zip(galleys) {
        let y = match s {
            Seg::Super(_) => top - size * 0.12,
            Seg::Sub(_) => top + size * 0.42,
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

pub struct RadioColors {
    pub text: Color32,
    pub background: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub hover: Color32,
    pub focus: Color32,
}

/// Win7-style radio button; returns the full rect used (circle + label).
/// Interaction state of one radio button.
#[derive(Clone, Copy, Default)]
pub struct RadioState {
    pub selected: bool,
    pub hovered: bool,
    pub focused: bool,
}

pub fn radio(
    p: &Painter,
    pos: Pos2,
    label: &str,
    size: f32,
    state: RadioState,
    c: &RadioColors,
) -> Rect {
    let d = (size + 1.0).round();
    let circle = Rect::from_min_size(pos, vec2(d, d));
    let center = circle.center();
    p.circle_filled(
        center,
        d / 2.0,
        if state.hovered { c.hover } else { c.background },
    );
    p.circle_stroke(center, d / 2.0 - 0.5, Stroke::new(1.0, c.border));
    if state.selected {
        p.circle_filled(center, d / 4.0, c.accent);
    }
    let g = p.layout_no_wrap(label.to_string(), FontId::proportional(size), c.text);
    let text_pos = pos2(circle.right() + 4.0, center.y - g.size().y / 2.0);
    let r = Rect::from_min_size(text_pos, g.size()).union(circle);
    p.galley(text_pos, g, c.text);
    if state.focused {
        p.rect_stroke(
            r.expand(1.5),
            radius(2.0),
            Stroke::new(1.0, c.focus),
            StrokeKind::Outside,
        );
    }
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
        let s = segments("flip_{16}");
        assert!(matches!(s[..], [Seg::Normal("flip"), Seg::Sub("16")]));
    }
}
