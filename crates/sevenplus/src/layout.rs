//! Button grids of the four Win7 calculator modes.

use calc_core::{BinOp, Calculator, Cmd, Func, Mode, StatFn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Digit,
    Operator,
    Memory,
    Function,
    Equals,
}

#[derive(Debug, Clone, Copy)]
pub struct Key {
    /// Label markup: `^{..}` superscript, `~{..}` overline.
    pub label: &'static str,
    pub cmd: Cmd,
    /// Label and command while Inv is on.
    pub inv: Option<(&'static str, Cmd)>,
    pub kind: Kind,
    pub col: u8,
    pub row: u8,
    pub w: u8,
    pub h: u8,
}

impl Key {
    pub fn resolve(&self, inv: bool) -> (&'static str, Cmd) {
        match self.inv {
            Some(i) if inv => i,
            _ => (self.label, self.cmd),
        }
    }
}

const fn k(label: &'static str, cmd: Cmd, kind: Kind, col: u8, row: u8) -> Key {
    Key {
        label,
        cmd,
        inv: None,
        kind,
        col,
        row,
        w: 1,
        h: 1,
    }
}

const fn ki(label: &'static str, cmd: Cmd, inv: (&'static str, Cmd), col: u8, row: u8) -> Key {
    Key {
        label,
        cmd,
        inv: Some(inv),
        kind: Kind::Function,
        col,
        row,
        w: 1,
        h: 1,
    }
}

const fn span(mut key: Key, w: u8, h: u8) -> Key {
    key.w = w;
    key.h = h;
    key
}

use Kind::*;

const fn d(n: u8, col: u8, row: u8) -> Key {
    const LABELS: [&str; 16] = [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F",
    ];
    k(LABELS[n as usize], Cmd::Digit(n), Digit, col, row)
}

/// The 5×6 block shared by Standard, Scientific and Programmer.
pub const STANDARD: &[Key] = &[
    k("MC", Cmd::MemClear, Memory, 0, 0),
    k("MR", Cmd::MemRecall, Memory, 1, 0),
    k("MS", Cmd::MemStore, Memory, 2, 0),
    k("M+", Cmd::MemAdd, Memory, 3, 0),
    k("M-", Cmd::MemSub, Memory, 4, 0),
    k("←", Cmd::Back, Function, 0, 1),
    k("CE", Cmd::ClearEntry, Function, 1, 1),
    k("C", Cmd::Clear, Function, 2, 1),
    k("±", Cmd::Negate, Function, 3, 1),
    k("√", Cmd::Func(Func::Sqrt), Function, 4, 1),
    d(7, 0, 2),
    d(8, 1, 2),
    d(9, 2, 2),
    k("/", Cmd::Op(BinOp::Div), Operator, 3, 2),
    k("%", Cmd::Percent, Function, 4, 2),
    d(4, 0, 3),
    d(5, 1, 3),
    d(6, 2, 3),
    k("*", Cmd::Op(BinOp::Mul), Operator, 3, 3),
    k("1/x", Cmd::Func(Func::Reciprocal), Function, 4, 3),
    d(1, 0, 4),
    d(2, 1, 4),
    d(3, 2, 4),
    k("-", Cmd::Op(BinOp::Sub), Operator, 3, 4),
    span(k("=", Cmd::Equals, Equals, 4, 4), 1, 2),
    span(d(0, 0, 5), 2, 1),
    k(".", Cmd::Point, Digit, 2, 5),
    k("+", Cmd::Op(BinOp::Add), Operator, 3, 5),
];

/// Scientific functions, 5×5, placed left of the standard block (rows 1–5).
pub const SCIENTIFIC: &[Key] = &[
    k("Inv", Cmd::Inv, Function, 1, 0),
    ki(
        "ln",
        Cmd::Func(Func::Ln),
        ("e^{x}", Cmd::Func(Func::ExpE)),
        2,
        0,
    ),
    k("(", Cmd::OpenParen, Function, 3, 0),
    k(")", Cmd::CloseParen, Function, 4, 0),
    ki(
        "Int",
        Cmd::Func(Func::Int),
        ("Frac", Cmd::Func(Func::Frac)),
        0,
        1,
    ),
    ki(
        "sinh",
        Cmd::Func(Func::Sinh),
        ("sinh^{-1}", Cmd::Func(Func::Asinh)),
        1,
        1,
    ),
    ki(
        "sin",
        Cmd::Func(Func::Sin),
        ("sin^{-1}", Cmd::Func(Func::Asin)),
        2,
        1,
    ),
    ki(
        "x^{2}",
        Cmd::Func(Func::Square),
        ("√x", Cmd::Func(Func::Sqrt)),
        3,
        1,
    ),
    k("n!", Cmd::Func(Func::Fact), Function, 4, 1),
    ki(
        "dms",
        Cmd::Func(Func::Dms),
        ("deg", Cmd::Func(Func::Degrees)),
        0,
        2,
    ),
    ki(
        "cosh",
        Cmd::Func(Func::Cosh),
        ("cosh^{-1}", Cmd::Func(Func::Acosh)),
        1,
        2,
    ),
    ki(
        "cos",
        Cmd::Func(Func::Cos),
        ("cos^{-1}", Cmd::Func(Func::Acos)),
        2,
        2,
    ),
    k("x^{y}", Cmd::Op(BinOp::Pow), Operator, 3, 2),
    k("^{y}√x", Cmd::Op(BinOp::Root), Operator, 4, 2),
    ki("π", Cmd::Pi, ("2*π", Cmd::Pi), 0, 3),
    ki(
        "tanh",
        Cmd::Func(Func::Tanh),
        ("tanh^{-1}", Cmd::Func(Func::Atanh)),
        1,
        3,
    ),
    ki(
        "tan",
        Cmd::Func(Func::Tan),
        ("tan^{-1}", Cmd::Func(Func::Atan)),
        2,
        3,
    ),
    ki(
        "x^{3}",
        Cmd::Func(Func::Cube),
        ("^{3}√x", Cmd::Func(Func::CubeRoot)),
        3,
        3,
    ),
    k("^{3}√x", Cmd::Func(Func::CubeRoot), Function, 4, 3),
    k("F-E", Cmd::FE, Function, 0, 4),
    k("Exp", Cmd::Exp, Function, 1, 4),
    k("Mod", Cmd::Op(BinOp::Mod), Operator, 2, 4),
    k("log", Cmd::Func(Func::Log), Function, 3, 4),
    k("10^{x}", Cmd::Func(Func::Pow10), Function, 4, 4),
];

/// Programmer operators and hex digits, 3×6, left of the standard block.
pub const PROGRAMMER: &[Key] = &[
    k("Mod", Cmd::Op(BinOp::Mod), Operator, 1, 0),
    d(10, 2, 0),
    k("(", Cmd::OpenParen, Function, 0, 1),
    k(")", Cmd::CloseParen, Function, 1, 1),
    d(11, 2, 1),
    k("RoL", Cmd::Func(Func::RoL), Function, 0, 2),
    k("RoR", Cmd::Func(Func::RoR), Function, 1, 2),
    d(12, 2, 2),
    k("Or", Cmd::Op(BinOp::Or), Operator, 0, 3),
    k("Xor", Cmd::Op(BinOp::Xor), Operator, 1, 3),
    d(13, 2, 3),
    k("Lsh", Cmd::Op(BinOp::Lsh), Operator, 0, 4),
    k("Rsh", Cmd::Op(BinOp::Rsh), Operator, 1, 4),
    d(14, 2, 4),
    k("Not", Cmd::Func(Func::Not), Function, 0, 5),
    k("And", Cmd::Op(BinOp::And), Operator, 1, 5),
    d(15, 2, 5),
];

/// Statistics keypad, 5×5.
pub const STATISTICS: &[Key] = &[
    k("~{x}", Cmd::Stat(StatFn::Mean), Function, 0, 0),
    k("~{x}^{2}", Cmd::Stat(StatFn::MeanSq), Function, 1, 0),
    k("←", Cmd::Back, Function, 2, 0),
    k("CE", Cmd::ClearEntry, Function, 3, 0),
    k("C", Cmd::Clear, Function, 4, 0),
    k("Σx", Cmd::Stat(StatFn::Sum), Function, 0, 1),
    k("Σx^{2}", Cmd::Stat(StatFn::SumSq), Function, 1, 1),
    d(7, 2, 1),
    d(8, 3, 1),
    d(9, 4, 1),
    k("σn", Cmd::Stat(StatFn::StdDevN), Function, 0, 2),
    k("σn-1", Cmd::Stat(StatFn::StdDevN1), Function, 1, 2),
    d(4, 2, 2),
    d(5, 3, 2),
    d(6, 4, 2),
    k("CAD", Cmd::StatClear, Function, 0, 3),
    k("±", Cmd::Negate, Function, 1, 3),
    d(1, 2, 3),
    d(2, 3, 3),
    d(3, 4, 3),
    span(k("Add", Cmd::StatAdd, Equals, 0, 4), 2, 1),
    span(d(0, 2, 4), 2, 1),
    k(".", Cmd::Point, Digit, 4, 4),
];

/// Whether a key is clickable in the current calculator state.
pub fn enabled(calc: &Calculator, cmd: Cmd) -> bool {
    if calc.error().is_some() {
        return matches!(
            cmd,
            Cmd::Digit(_) | Cmd::Point | Cmd::Clear | Cmd::ClearEntry
        );
    }
    if calc.mode == Mode::Programmer {
        return match cmd {
            Cmd::Digit(n) => (n as u32) < calc.base.radix(),
            Cmd::Point | Cmd::Percent => false,
            Cmd::Func(f) => matches!(f, Func::Not | Func::RoL | Func::RoR),
            _ => true,
        };
    }
    match cmd {
        Cmd::CloseParen => calc.paren_depth() > 0,
        Cmd::Digit(n) => n < 10,
        _ => true,
    }
}
