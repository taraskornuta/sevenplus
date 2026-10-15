//! Layout tables (UI-001): the button grids of the four modes and where each part of
//! the window goes. Sizes come from the theme's spacing tokens, in logical px.

use calc_config::Extras;
use calc_core::{BinOp, Func, Key, Mode, StatFn};
use calc_theme::{Group as Kind, Theme};
use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};

/// One on-screen button.
#[derive(Debug, Clone, Copy)]
pub struct Button {
    /// Label markup: `^{..}` superscript, `~{..}` overline.
    pub label: &'static str,
    pub key: Key,
    /// Label and key while Inv is on.
    pub inv: Option<(&'static str, Key)>,
    pub kind: Kind,
    pub col: u8,
    pub row: u8,
    pub w: u8,
    pub h: u8,
}

impl Button {
    pub fn resolve(&self, inv: bool) -> (&'static str, Key) {
        match self.inv {
            Some(i) if inv => i,
            _ => (self.label, self.key),
        }
    }
}

const fn k(label: &'static str, key: Key, kind: Kind, col: u8, row: u8) -> Button {
    Button {
        label,
        key,
        inv: None,
        kind,
        col,
        row,
        w: 1,
        h: 1,
    }
}

const fn ki(label: &'static str, key: Key, inv: (&'static str, Key), col: u8, row: u8) -> Button {
    Button {
        label,
        key,
        inv: Some(inv),
        kind: Kind::Function,
        col,
        row,
        w: 1,
        h: 1,
    }
}

const fn span(mut b: Button, w: u8, h: u8) -> Button {
    b.w = w;
    b.h = h;
    b
}

use Kind::*;

const fn d(n: u8, col: u8, row: u8) -> Button {
    const LABELS: [&str; 16] = [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F",
    ];
    k(LABELS[n as usize], Key::Digit(n), Digit, col, row)
}

/// The 5×6 block shared by Standard, Scientific and Programmer.
pub const STANDARD: &[Button] = &[
    k("MC", Key::MemClear, Memory, 0, 0),
    k("MR", Key::MemRecall, Memory, 1, 0),
    k("MS", Key::MemStore, Memory, 2, 0),
    k("M+", Key::MemAdd, Memory, 3, 0),
    k("M-", Key::MemSub, Memory, 4, 0),
    k("←", Key::Back, Function, 0, 1),
    k("CE", Key::ClearEntry, Function, 1, 1),
    k("C", Key::Clear, Function, 2, 1),
    k("±", Key::Negate, Function, 3, 1),
    k("√", Key::Func(Func::Sqrt), Function, 4, 1),
    d(7, 0, 2),
    d(8, 1, 2),
    d(9, 2, 2),
    k("/", Key::Op(BinOp::Div), Operator, 3, 2),
    k("%", Key::Percent, Function, 4, 2),
    d(4, 0, 3),
    d(5, 1, 3),
    d(6, 2, 3),
    k("*", Key::Op(BinOp::Mul), Operator, 3, 3),
    k("1/x", Key::Func(Func::Reciprocal), Function, 4, 3),
    d(1, 0, 4),
    d(2, 1, 4),
    d(3, 2, 4),
    k("-", Key::Op(BinOp::Sub), Operator, 3, 4),
    span(k("=", Key::Equals, Equals, 4, 4), 1, 2),
    span(d(0, 0, 5), 2, 1),
    k(".", Key::Point, Digit, 2, 5),
    k("+", Key::Op(BinOp::Add), Operator, 3, 5),
];

/// Scientific functions, 5×5, placed left of the standard block (rows 1–5).
pub const SCIENTIFIC: &[Button] = &[
    k("Inv", Key::Inv, Function, 1, 0),
    ki(
        "ln",
        Key::Func(Func::Ln),
        ("e^{x}", Key::Func(Func::ExpE)),
        2,
        0,
    ),
    k("(", Key::OpenParen, Function, 3, 0),
    k(")", Key::CloseParen, Function, 4, 0),
    ki(
        "Int",
        Key::Func(Func::Int),
        ("Frac", Key::Func(Func::Frac)),
        0,
        1,
    ),
    ki(
        "sinh",
        Key::Func(Func::Sinh),
        ("sinh^{-1}", Key::Func(Func::Asinh)),
        1,
        1,
    ),
    ki(
        "sin",
        Key::Func(Func::Sin),
        ("sin^{-1}", Key::Func(Func::Asin)),
        2,
        1,
    ),
    ki(
        "x^{2}",
        Key::Func(Func::Square),
        ("√x", Key::Func(Func::Sqrt)),
        3,
        1,
    ),
    k("n!", Key::Func(Func::Fact), Function, 4, 1),
    ki(
        "dms",
        Key::Func(Func::Dms),
        ("deg", Key::Func(Func::Degrees)),
        0,
        2,
    ),
    ki(
        "cosh",
        Key::Func(Func::Cosh),
        ("cosh^{-1}", Key::Func(Func::Acosh)),
        1,
        2,
    ),
    ki(
        "cos",
        Key::Func(Func::Cos),
        ("cos^{-1}", Key::Func(Func::Acos)),
        2,
        2,
    ),
    k("x^{y}", Key::Op(BinOp::Pow), Operator, 3, 2),
    k("^{y}√x", Key::Op(BinOp::Root), Operator, 4, 2),
    ki("π", Key::Pi, ("2*π", Key::Pi), 0, 3),
    ki(
        "tanh",
        Key::Func(Func::Tanh),
        ("tanh^{-1}", Key::Func(Func::Atanh)),
        1,
        3,
    ),
    ki(
        "tan",
        Key::Func(Func::Tan),
        ("tan^{-1}", Key::Func(Func::Atan)),
        2,
        3,
    ),
    ki(
        "x^{3}",
        Key::Func(Func::Cube),
        ("^{3}√x", Key::Func(Func::CubeRoot)),
        3,
        3,
    ),
    k("^{3}√x", Key::Func(Func::CubeRoot), Function, 4, 3),
    k("F-E", Key::FE, Function, 0, 4),
    k("Exp", Key::Exp, Function, 1, 4),
    k("Mod", Key::Op(BinOp::Mod), Operator, 2, 4),
    k("log", Key::Func(Func::Log), Function, 3, 4),
    k("10^{x}", Key::Func(Func::Pow10), Function, 4, 4),
];

/// Programmer operators and hex digits, 3×6, left of the standard block. Bitwise
/// operators are styled as functions so only `+ - * /` use the operator style.
pub const PROGRAMMER: &[Button] = &[
    k("Mod", Key::Op(BinOp::Mod), Function, 1, 0),
    d(10, 2, 0),
    k("(", Key::OpenParen, Function, 0, 1),
    k(")", Key::CloseParen, Function, 1, 1),
    d(11, 2, 1),
    k("RoL", Key::Func(Func::RoL), Function, 0, 2),
    k("RoR", Key::Func(Func::RoR), Function, 1, 2),
    d(12, 2, 2),
    k("Or", Key::Op(BinOp::Or), Function, 0, 3),
    k("Xor", Key::Op(BinOp::Xor), Function, 1, 3),
    d(13, 2, 3),
    k("Lsh", Key::Op(BinOp::Lsh), Function, 0, 4),
    k("Rsh", Key::Op(BinOp::Rsh), Function, 1, 4),
    d(14, 2, 4),
    k("Not", Key::Func(Func::Not), Function, 0, 5),
    k("And", Key::Op(BinOp::And), Function, 1, 5),
    d(15, 2, 5),
];

/// Programmer toolbar buttons, right of the text field (sized to their labels).
pub const TOOLS: &[Button] = &[
    k("Byte flip", Key::Func(Func::ByteFlip), Function, 0, 0),
    k("Word flip", Key::Func(Func::WordFlip), Function, 1, 0),
];

/// Statistics keypad, 5×5.
pub const STATISTICS: &[Button] = &[
    k("~{x}", Key::Stat(StatFn::Mean), Function, 0, 0),
    k("~{x}^{2}", Key::Stat(StatFn::MeanSq), Function, 1, 0),
    k("←", Key::Back, Function, 2, 0),
    k("CE", Key::ClearEntry, Function, 3, 0),
    k("C", Key::Clear, Function, 4, 0),
    k("Σx", Key::Stat(StatFn::Sum), Function, 0, 1),
    k("Σx^{2}", Key::Stat(StatFn::SumSq), Function, 1, 1),
    d(7, 2, 1),
    d(8, 3, 1),
    d(9, 4, 1),
    k("σn", Key::Stat(StatFn::StdDevN), Function, 0, 2),
    k("σn-1", Key::Stat(StatFn::StdDevN1), Function, 1, 2),
    d(4, 2, 2),
    d(5, 3, 2),
    d(6, 4, 2),
    k("CAD", Key::StatClear, Function, 0, 3),
    k("±", Key::Negate, Function, 1, 3),
    d(1, 2, 3),
    d(2, 3, 3),
    d(3, 4, 3),
    span(k("Add", Key::StatAdd, Equals, 0, 4), 2, 1),
    span(d(0, 2, 4), 2, 1),
    k(".", Key::Point, Digit, 4, 4),
];

/// Fluent id of a key's accessible name (UI-020); digits are read as themselves.
pub fn a11y_id(key: Key, label: &str) -> Option<&'static str> {
    use Func::*;
    Some(match key {
        Key::Digit(_) => return None,
        Key::Point => "key-point",
        Key::Negate => "key-negate",
        Key::Back => "key-back",
        Key::ClearEntry => "key-clear-entry",
        Key::Clear => "key-clear",
        Key::Op(op) => match op {
            BinOp::Add => "key-plus",
            BinOp::Sub => "key-minus",
            BinOp::Mul => "key-multiply",
            BinOp::Div => "key-divide",
            BinOp::Mod => "key-mod",
            BinOp::Pow => "key-power",
            BinOp::Root => "key-root",
            BinOp::And => "key-and",
            BinOp::Or => "key-or",
            BinOp::Xor => "key-xor",
            BinOp::Lsh => "key-lsh",
            BinOp::Rsh => "key-rsh",
        },
        Key::Equals => "key-equals",
        Key::Percent => "key-percent",
        Key::Func(f) => match f {
            Sqrt => "key-sqrt",
            Reciprocal => "key-reciprocal",
            Square => "key-square",
            Cube => "key-cube",
            CubeRoot => "key-cube-root",
            Pow10 => "key-pow10",
            ExpE => "key-exp-e",
            Ln => "key-ln",
            Log => "key-log",
            Sin => "key-sin",
            Cos => "key-cos",
            Tan => "key-tan",
            Asin => "key-asin",
            Acos => "key-acos",
            Atan => "key-atan",
            Sinh => "key-sinh",
            Cosh => "key-cosh",
            Tanh => "key-tanh",
            Asinh => "key-asinh",
            Acosh => "key-acosh",
            Atanh => "key-atanh",
            Fact => "key-factorial",
            Int => "key-int",
            Frac => "key-frac",
            Dms => "key-dms",
            Degrees => "key-deg",
            Not => "key-not",
            RoL => "key-rol",
            RoR => "key-ror",
            ByteFlip => "key-byte-flip",
            WordFlip => "key-word-flip",
        },
        Key::Pi if label.contains('2') => "key-two-pi",
        Key::Pi => "key-pi",
        Key::OpenParen => "key-open-paren",
        Key::CloseParen => "key-close-paren",
        Key::Exp => "key-exp",
        Key::FE => "key-fe",
        Key::Inv => "key-inv",
        Key::MemClear => "key-mem-clear",
        Key::MemRecall => "key-mem-recall",
        Key::MemStore => "key-mem-store",
        Key::MemAdd => "key-mem-add",
        Key::MemSub => "key-mem-sub",
        Key::StatAdd => "key-add",
        Key::StatClear => "key-cad",
        Key::Stat(f) => match f {
            StatFn::Mean => "key-mean",
            StatFn::MeanSq => "key-mean-sq",
            StatFn::Sum => "key-sum",
            StatFn::SumSq => "key-sum-sq",
            StatFn::StdDevN => "key-sd-n",
            StatFn::StdDevN1 => "key-sd-n1",
        },
        Key::ToggleBit(_) => "key-bit",
    })
}

/// The C-language sign shown instead of a Programmer key's word label.
pub fn c_label(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::Op(BinOp::Mod) => "%",
        Key::Op(BinOp::And) => "&",
        Key::Op(BinOp::Or) => "|",
        Key::Op(BinOp::Xor) => "^",
        Key::Op(BinOp::Lsh) => "<<",
        Key::Op(BinOp::Rsh) => ">>",
        Key::Func(Func::Not) => "~",
        _ => return None,
    })
}

// ---- mode layout tables ---------------------------------------------------------------

/// What sits left of the main keypad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    None,
    /// Scientific: the angle radio row above a 5×5 function block.
    Scientific,
    /// Programmer: radix and word-size radio columns, then a 3×6 block.
    Programmer,
}

/// The list area inside the display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    /// History, when switched on (HIS-001).
    History,
    /// Statistics data list with a count line (STA-001).
    Stats,
}

/// One mode's layout (UI-001).
#[derive(Debug, Clone, Copy)]
pub struct ModeLayout {
    pub keypad: &'static [Button],
    pub keypad_rows: u8,
    pub side: Side,
    pub side_keys: &'static [Button],
    pub list: List,
    /// Programmer toolbar (text field, endian flips) and bit panel (PRG-005).
    pub bits: bool,
}

pub const STANDARD_LAYOUT: ModeLayout = ModeLayout {
    keypad: STANDARD,
    keypad_rows: 6,
    side: Side::None,
    side_keys: &[],
    list: List::History,
    bits: false,
};

pub const SCIENTIFIC_LAYOUT: ModeLayout = ModeLayout {
    keypad: STANDARD,
    keypad_rows: 6,
    side: Side::Scientific,
    side_keys: SCIENTIFIC,
    list: List::History,
    bits: false,
};

pub const PROGRAMMER_LAYOUT: ModeLayout = ModeLayout {
    keypad: STANDARD,
    keypad_rows: 6,
    side: Side::Programmer,
    side_keys: PROGRAMMER,
    list: List::History,
    bits: true,
};

pub const STATISTICS_LAYOUT: ModeLayout = ModeLayout {
    keypad: STATISTICS,
    keypad_rows: 5,
    side: Side::None,
    side_keys: &[],
    list: List::Stats,
    bits: false,
};

pub fn for_mode(mode: Mode) -> &'static ModeLayout {
    match mode {
        Mode::Standard => &STANDARD_LAYOUT,
        Mode::Scientific => &SCIENTIFIC_LAYOUT,
        Mode::Programmer => &PROGRAMMER_LAYOUT,
        Mode::Statistics => &STATISTICS_LAYOUT,
    }
}

/// Width of the extras panel and its minimum height (EXT-005).
pub const EXTRAS_WIDTH: f32 = 300.0;
pub const EXTRAS_MIN_HEIGHT: f32 = 300.0;

/// Positions of every part of the calculator body, relative to the body origin.
#[derive(Debug, Clone)]
pub struct Layout {
    pub size: Vec2,
    pub menu: Rect,
    pub display: Rect,
    /// Programmer toolbar: ASCII/Unicode switch, text field, Byte/Word flip.
    pub tools: Option<Rect>,
    pub bits: Option<Rect>,
    /// Origin of the main keypad block.
    pub keypad: Pos2,
    /// Origin of the side block (scientific or programmer).
    pub side: Option<Pos2>,
    /// Radio groups: scientific angle box, or programmer radix and word boxes.
    pub radios: Vec<Rect>,
    pub extras: Option<Rect>,
}

pub fn block_w(t: &Theme, cols: u8) -> f32 {
    cols as f32 * t.spacing.button_width + (cols.max(1) - 1) as f32 * t.spacing.gap
}

pub fn block_h(t: &Theme, rows: u8) -> f32 {
    rows as f32 * t.spacing.button_height + (rows.max(1) - 1) as f32 * t.spacing.gap
}

pub fn line_h(t: &Theme) -> f32 {
    t.font.history.size + 5.0
}

pub fn bits_row_h(t: &Theme) -> f32 {
    t.font.bits.size * 2.0 + 6.0
}

/// Lays out the body for a mode. `menu` is false when the menu lives in the OS menu
/// bar (macOS, KEY-001).
pub fn compute(t: &Theme, mode: Mode, history: bool, extras: Extras, menu: bool) -> Layout {
    let ml = for_mode(mode);
    let (pad, g) = (t.spacing.padding, t.spacing.gap);
    let radio_w = block_w(t, 2) + 8.0;
    let side_w = match ml.side {
        Side::Scientific => block_w(t, 5) + g,
        Side::Programmer => radio_w + g + block_w(t, 3) + g,
        Side::None => 0.0,
    };
    let content_w = side_w + block_w(t, 5);
    let list_h = t.display.history_lines as f32 * line_h(t);
    let display_h = t.display.height
        + match ml.list {
            List::History if history && mode != Mode::Programmer => list_h + t.display.padding,
            List::Stats => list_h + line_h(t) + t.display.padding,
            List::History => 0.0,
        };
    let menu_h = if menu { t.menu.height } else { 0.0 };
    let mut y = menu_h + pad;
    let display = Rect::from_min_size(pos2(pad, y), vec2(content_w, display_h));
    y += display_h + pad;
    let tools = ml.bits.then(|| {
        let r = Rect::from_min_size(pos2(pad, y), vec2(content_w, t.spacing.button_height));
        y += r.height() + pad;
        r
    });
    let bits = ml.bits.then(|| {
        let r = Rect::from_min_size(pos2(pad, y), vec2(content_w, 2.0 * bits_row_h(t)));
        y += r.height() + pad;
        r
    });
    let keypad_h = block_h(t, ml.keypad_rows);
    let keypad = pos2(pad + side_w, y);
    let (side, radios) = match ml.side {
        Side::Scientific => (
            Some(pos2(pad, y + t.spacing.button_height + g)),
            vec![Rect::from_min_size(
                pos2(pad, y),
                vec2(block_w(t, 5), t.spacing.button_height),
            )],
        ),
        Side::Programmer => {
            let h = (keypad_h - g) / 2.0;
            (
                Some(pos2(pad + radio_w + g, y)),
                vec![
                    Rect::from_min_size(pos2(pad, y), vec2(radio_w, h)),
                    Rect::from_min_size(pos2(pad, y + h + g), vec2(radio_w, h)),
                ],
            )
        }
        Side::None => (None, vec![]),
    };
    let mut size = vec2(pad * 2.0 + content_w, y + keypad_h + pad);
    let extras = (extras != Extras::None).then(|| {
        let top = menu_h + pad;
        let h = (size.y - top - pad).max(EXTRAS_MIN_HEIGHT);
        let r = Rect::from_min_size(pos2(size.x, top), vec2(EXTRAS_WIDTH, h));
        size = vec2(r.right() + pad, size.y.max(r.bottom() + pad));
        r
    });
    Layout {
        size,
        menu: Rect::from_min_size(Pos2::ZERO, vec2(size.x, menu_h)),
        display,
        tools,
        bits,
        keypad,
        side,
        radios,
        extras,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_sizes() {
        // §2.1 targets at 100%: Standard ~228×322, Scientific ~414×322,
        // Programmer ~414×380, Statistics ~228×400 (menu included, frame excluded).
        let t = Theme::builtin();
        let size = |m| compute(&t, m, false, Extras::None, true).size;
        let near = |v: Vec2, w: f32, h: f32| (v.x - w).abs() <= 24.0 && (v.y - h).abs() <= 60.0;
        assert!(
            near(size(Mode::Standard), 228.0, 300.0),
            "{:?}",
            size(Mode::Standard)
        );
        assert!(
            near(size(Mode::Scientific), 414.0, 300.0),
            "{:?}",
            size(Mode::Scientific)
        );
        assert!(
            near(size(Mode::Programmer), 414.0, 380.0),
            "{:?}",
            size(Mode::Programmer)
        );
        assert!(
            near(size(Mode::Statistics), 228.0, 400.0),
            "{:?}",
            size(Mode::Statistics)
        );
        // Extras panels add width beside the keypad (EXT-005).
        let with = compute(&t, Mode::Standard, false, Extras::Units, true).size;
        assert!(with.x > size(Mode::Standard).x + 250.0);
    }

    #[test]
    fn every_key_has_a_name() {
        for b in STANDARD
            .iter()
            .chain(SCIENTIFIC)
            .chain(PROGRAMMER)
            .chain(TOOLS)
            .chain(STATISTICS)
        {
            for (label, key) in [Some((b.label, b.key)), b.inv].into_iter().flatten() {
                if let Some(id) = a11y_id(key, label) {
                    assert_ne!(crate::i18n::tr(id), id, "missing translation {id}");
                }
            }
        }
    }
}
